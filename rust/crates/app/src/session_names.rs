//! The owner's own session names (T117, FR-127): `PUT /api/sessions/{id}/name {name|null, confirmed: true}`, and the
//! one owner-only file they live in, `session_names.json` in the data folder (`EASY_PERSONAL_HOME`, else
//! `~/.easy-personal`). The route sits behind the guard in `server.rs` (Host, Origin, token) like every `/api/` route.
//!
//! - **File**: `{"names": [{"id": "<session id>", "name": "<name>"}, ...]}`, oldest first, at most [`MAX_NAMES`]; one
//!   past that drops the oldest. A write goes to a new owner-only (0600) temp file in the same folder (`O_NOFOLLOW`,
//!   `O_EXCL`), is synced, then renamed over the old one, so a reader never sees half a file.
//! - **Read**: never an error. A missing file, a symlink, a file that is not the app user's or that others may write, one
//!   over [`MAX_FILE`] bytes, one that is not valid JSON of this shape, or one holding a bad entry means *no names*.
//! - **Name**: 1..=[`MAX_NAME_CHARS`] characters after cleaning: line breaks and tabs become spaces, the hidden and
//!   control characters of FR-092 are removed (`live::clean_line`), the ends are trimmed. A longer one is refused, not cut.
//!   `null` removes the session's name. The text is the owner's own typing, not transcript text; it is shown in Live
//!   and Past and goes to no log, MCP tool, CLI output, tray row or popup.
//! - **Live**: [`stamp`] (inode, time and size of the file) changes on every write, so an open `/api/live` stream sends a
//!   fresh snapshot (`live_feed.rs`).

use crate::agent_routes::{blocking, data_dir, object};
use crate::api::{Api, respond};
use crate::writes::fail;
use axum::Router;
use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::put;
use easy_personal_core::live::clean_line;
use easy_personal_core::sessions::valid_session_id;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path as FsPath, PathBuf};
use std::sync::Mutex;

/// The file in the data folder.
pub(crate) const FILE: &str = "session_names.json";
/// At most this many names are kept; the oldest is dropped for a new one.
pub(crate) const MAX_NAMES: usize = 2_000;
/// A name is 1 to this many characters after cleaning.
pub(crate) const MAX_NAME_CHARS: usize = 80;
/// A file bigger than this is not read (2,000 entries of the longest id and name are under 1 MB).
const MAX_FILE: u64 = 2 << 20;

/// One writer at a time: a write reads the file, changes it and renames a new one over it.
static WRITE: Mutex<()> = Mutex::new(());

pub(crate) fn routes() -> Router<Api> {
    Router::new().route("/api/sessions/{id}/name", put(set_name))
}

/// What a stream compares to tell its snapshot is stale: the names file's inode, modification time and size (`None`: no
/// file). Every write renames a new file over the old one, so every write changes it.
pub(crate) type Stamp = Option<(u64, i64, u64)>;

pub(crate) fn stamp(dir: &FsPath) -> Stamp {
    let meta = fs::symlink_metadata(dir.join(FILE)).ok()?;
    Some((meta.ino(), meta.mtime_nsec() + meta.mtime() * 1_000_000_000, meta.len()))
}

/// The data folder the names live in (the store's folder), or `None` when the store path cannot be told.
pub(crate) fn dir_of(api: &Api) -> Option<PathBuf> {
    (api.store_path)()
        .ok()
        .and_then(|path| path.parent().and_then(|d| std::path::absolute(d).ok()))
}

/// The names, oldest first; empty for a file that is missing or not usable (see the module text).
pub(crate) fn load(dir: &FsPath) -> Vec<(String, String)> {
    read(dir).unwrap_or_default()
}

/// The names as a map from session id to name.
pub(crate) fn load_map(dir: &FsPath) -> HashMap<String, String> {
    load(dir).into_iter().collect()
}

fn read(dir: &FsPath) -> Option<Vec<(String, String)>> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(dir.join(FILE))
        .ok()?;
    let meta = file.metadata().ok()?;
    // SAFETY: `geteuid` has no preconditions and cannot fail.
    let me = unsafe { libc::geteuid() };
    if !meta.is_file() || meta.len() > MAX_FILE || meta.uid() != me || meta.mode() & 0o022 != 0 {
        return None;
    }
    let mut raw = Vec::new();
    file.take(MAX_FILE).read_to_end(&mut raw).ok()?;
    let Value::Object(top) = serde_json::from_slice(&raw).ok()? else {
        return None;
    };
    let list = top.get("names")?.as_array()?;
    if list.len() > MAX_NAMES {
        return None;
    }
    let mut names = Vec::with_capacity(list.len());
    for entry in list {
        let id = entry.get("id")?.as_str()?;
        let name = entry.get("name")?.as_str()?;
        if !valid_session_id(id) || clean(name).as_deref() != Some(name) {
            return None;
        }
        names.push((id.to_owned(), name.to_owned()));
    }
    Some(names)
}

/// The name as it is kept, or `None` when cleaning leaves nothing or more than [`MAX_NAME_CHARS`] characters.
pub(crate) fn clean(raw: &str) -> Option<String> {
    let text = clean_line(raw, usize::MAX);
    let text = text.trim();
    let chars = text.chars().count();
    (1..=MAX_NAME_CHARS).contains(&chars).then(|| text.to_owned())
}

/// The names after setting (`Some`) or removing (`None`) one: the session's old entry goes, a new one is the newest, and
/// the oldest are dropped past [`MAX_NAMES`].
fn with(mut names: Vec<(String, String)>, id: &str, name: Option<String>) -> Vec<(String, String)> {
    names.retain(|(kept, _)| kept != id);
    if let Some(name) = name {
        names.push((id.to_owned(), name));
    }
    let extra = names.len().saturating_sub(MAX_NAMES);
    names.drain(..extra);
    names
}

/// Writes the names: the data folder is made 0700 when missing; a new 0600 file is renamed over the old one.
fn save(dir: &FsPath, names: &[(String, String)]) -> io::Result<()> {
    match DirBuilder::new().recursive(true).mode(0o700).create(dir) {
        Err(e) if e.kind() != io::ErrorKind::AlreadyExists => return Err(e),
        _ => {}
    }
    let list: Vec<Value> = names
        .iter()
        .map(|(id, name)| json!({"id": id, "name": name}))
        .collect();
    let body = json!({"names": list}).to_string();
    let new = dir.join(format!("{FILE}.new"));
    let _ = fs::remove_file(&new);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&new)?;
    let written = file.write_all(body.as_bytes()).and_then(|()| file.sync_all());
    drop(file);
    written
        .and_then(|()| fs::rename(&new, dir.join(FILE)))
        .inspect_err(|_| {
            let _ = fs::remove_file(&new);
        })
}

/// Sets or removes one name in the file in `dir`. A file that cannot be read starts again from no names.
pub(crate) fn store_name(dir: &FsPath, id: &str, name: Option<String>) -> io::Result<()> {
    let _one = WRITE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    save(dir, &with(load(dir), id, name))
}

/// Adds `name` (the owner's name, else `null`) to each session of a past-sessions answer.
pub(crate) fn attach(body: &mut Value, dir: Option<&FsPath>) {
    let names = dir.map(load_map).unwrap_or_default();
    for session in body["sessions"].as_array_mut().into_iter().flatten() {
        let name = session["id"].as_str().and_then(|id| names.get(id)).cloned();
        session["name"] = name.map_or(Value::Null, Value::String);
    }
}

const BAD_ID: &str = "session id must be 1 to 128 letters, digits, - or _";
const BAD_BODY: &str = "body must be {name: text or null, confirmed: true}";
const BAD_NAME: &str = "name must be 1 to 80 characters after cleaning";
const NOT_WRITTEN: &str = "the name could not be saved right now; try again in a moment";

/// `PUT /api/sessions/{id}/name` `{name: "<text>"|null, confirmed: true}` -> `{id, name}`.
async fn set_name(State(api): State<Api>, Path(id): Path<String>, req: Request) -> Response {
    let body = match object(req, false).await {
        Ok(body) => body,
        Err(answer) => return answer,
    };
    if body.keys().any(|k| k != "name" && k != "confirmed") {
        return fail(StatusCode::BAD_REQUEST, "unknown field");
    }
    if body.get("confirmed") != Some(&Value::Bool(true)) {
        return fail(StatusCode::BAD_REQUEST, "confirmed must be true");
    }
    if !valid_session_id(&id) {
        return fail(StatusCode::BAD_REQUEST, BAD_ID);
    }
    let name = match body.get("name") {
        Some(Value::Null) => None,
        Some(Value::String(raw)) => match clean(raw) {
            Some(name) => Some(name),
            None => return fail(StatusCode::BAD_REQUEST, BAD_NAME),
        },
        _ => return fail(StatusCode::BAD_REQUEST, BAD_BODY),
    };
    let dir = match data_dir(&api) {
        Ok(dir) => dir,
        Err(answer) => return answer,
    };
    blocking(move || match store_name(&dir, &id, name.clone()) {
        Ok(()) => respond(StatusCode::OK, &json!({"id": id, "name": name})),
        Err(_) => fail(StatusCode::INTERNAL_SERVER_ERROR, NOT_WRITTEN),
    })
    .await
}

#[cfg(test)]
#[path = "session_names_tests.rs"]
mod tests;
