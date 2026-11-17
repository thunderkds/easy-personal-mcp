//! The answer bridge's install (T106, FR-113, DDR-0006): `hooks install|remove|status` and `GET`/`PUT /api/hooks`.
//!
//! Off by default. For one confirmed Claude account, **on** adds exactly two entries to that account's user settings file
//! `<account folder>/settings.json`: `hooks.PreToolUse` (matcher `AskUserQuestion`) and `hooks.PermissionRequest`, each one
//! command hook `<absolute path of this program> hook ask`, timeout 130 s. **Off** removes only entries equal to the ones
//! the app adds. Never `~/.claude.json` (DDR-0001), never a project's settings.
//!
//! The write is add-only and safe: the file is read and parsed first (a file that is not a JSON object is refused and left
//! byte for byte as it is); every other key and hook is kept; the new text is written to a temp file in the same folder and
//! renamed over the file (a symlinked `settings.json` is written through, the link stays); a copy of the old file goes to
//! `<data folder>/backups/` first (owner-only). Nothing is written when nothing changes, so a second `install` adds nothing.
//! Object keys come back in alphabetical order and the indent is two spaces: the parser here keeps no key order.
//!
//! `status` is *on*, *off*, *changed by hand* (an entry that looks like ours differs from ours, or only one of the two is
//! there) or *binary moved* (our entries name a program that is gone). `remove` drops equal entries and entries that are ours
//! but name a gone program; a hand-edited entry is kept. `install` over a moved entry replaces it; over a changed one it
//! refuses. No answer or message names a path (NFR-002); the command text, which holds this program's path, is the one
//! thing shown on purpose (the confirm step needs it).

#![allow(clippy::result_large_err)] // a route's early answer is a `Response`, as in `agent_routes.rs`

use crate::api::{self, Api};
use crate::cli_cmds::{self, Run, Stop};
use crate::writes::{fail, parse, raw_body};
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::Response;
use clap::{Args, Subcommand};
use easy_personal_core::clock::now_micros;
use easy_personal_core::pyjson::sh_quote;
use easy_personal_core::scope::claude_account_id;
use easy_personal_core::store::{self, open_checked};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const PRE: &str = "PreToolUse";
const PERM: &str = "PermissionRequest";
const MATCHER: &str = "AskUserQuestion";
/// Seconds: the bridge waits 120 s for the owner, the hook may run a little longer (FR-113).
const TIMEOUT: u64 = 130;
const SETTINGS: &str = "settings.json";
/// The program's name in our commands: what tells an entry that looks like ours from the owner's own hooks.
const PROGRAM: &str = "easy-personal-rs";
const SUFFIX: &str = " hook ask";
const MAX_SETTINGS_BYTES: u64 = 8 << 20;

// ---- the entries ----

/// The command of both entries: the program's absolute path (quoted when it needs it) and `hook ask`.
pub(crate) fn command_text(exe: &Path) -> String {
    format!("{}{SUFFIX}", sh_quote(&exe.to_string_lossy()))
}

/// This program's command, or `None` when its path cannot be used (a replaced binary reads as `... (deleted)`).
fn own_command() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    exe.is_file().then(|| command_text(&exe))
}

fn entry(event: &str, command: &str) -> Value {
    let hook = json!({"type": "command", "command": command, "timeout": TIMEOUT});
    if event == PRE {
        json!({"matcher": MATCHER, "hooks": [hook]})
    } else {
        json!({"hooks": [hook]})
    }
}

/// The exact text the owner confirms: the `hooks` block that is merged in.
pub(crate) fn block_text(command: &str) -> String {
    let block = json!({"hooks": {PRE: [entry(PRE, command)], PERM: [entry(PERM, command)]}});
    serde_json::to_string_pretty(&block).unwrap_or_default()
}

/// The program of an entry that looks like ours (`<program> hook ask`, the program named `easy-personal-rs`).
fn program_of(command: &str) -> Option<PathBuf> {
    let program = command.strip_suffix(SUFFIX)?;
    let program = match program
        .strip_prefix('\'')
        .and_then(|p| p.strip_suffix('\''))
    {
        Some(quoted) => quoted.replace("'\"'\"'", "'"),
        None => program.to_string(),
    };
    let path = PathBuf::from(program);
    (path.file_name()? == PROGRAM).then_some(path)
}

/// The command of the first hook of `entry` that looks like ours.
fn ours_command(entry: &Value) -> Option<&str> {
    entry
        .get("hooks")?
        .as_array()?
        .iter()
        .filter_map(|hook| hook.get("command")?.as_str())
        .find(|command| program_of(command).is_some())
}

/// How one entry stands to ours. Ordered by how much it needs the owner's eyes.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Kind {
    Absent,
    Equal,
    Moved,
    Changed,
}

/// `None`: the entry does not look like ours (the owner's own hook; never touched).
fn classify(entry_value: &Value, event: &str, command: &str) -> Option<Kind> {
    let found = ours_command(entry_value)?;
    Some(if *entry_value != entry(event, found) {
        Kind::Changed
    } else if !program_of(found).is_some_and(|p| p.exists()) {
        Kind::Moved
    } else if found == command {
        Kind::Equal
    } else {
        Kind::Changed
    })
}

/// `hooks.<event>` as a list: empty when absent, `None` when the file has another shape there.
fn event_list<'a>(settings: &'a Value, event: &str) -> Option<&'a [Value]> {
    match settings.get("hooks") {
        None => Some(&[]),
        Some(Value::Object(hooks)) => match hooks.get(event) {
            None => Some(&[]),
            Some(Value::Array(list)) => Some(list),
            Some(_) => None,
        },
        Some(_) => None,
    }
}

/// The kind of each event (worst entry wins; `Absent` with none of ours); `None` for an unexpected shape.
fn kinds(settings: &Value, command: &str) -> Option<[Kind; 2]> {
    let one = |event: &str| -> Option<Kind> {
        Some(
            event_list(settings, event)?
                .iter()
                .filter_map(|e| classify(e, event, command))
                .max()
                .unwrap_or(Kind::Absent),
        )
    };
    Some([one(PRE)?, one(PERM)?])
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Status {
    On,
    Off,
    ChangedByHand,
    BinaryMoved,
    /// The settings file is not a readable JSON object: nothing can be said about the entries.
    Unreadable,
}

impl Status {
    /// The words of the CLI.
    pub(crate) fn word(self) -> &'static str {
        match self {
            Status::On => "on",
            Status::Off => "off",
            Status::ChangedByHand => "changed by hand",
            Status::BinaryMoved => "binary moved",
            Status::Unreadable => "unreadable",
        }
    }

    /// The words of the route.
    fn code(self) -> &'static str {
        match self {
            Status::ChangedByHand => "changed_by_hand",
            Status::BinaryMoved => "binary_moved",
            other => other.word(),
        }
    }
}

pub(crate) fn status_of(settings: &Value, command: &str) -> Status {
    match kinds(settings, command) {
        None => Status::ChangedByHand,
        Some([Kind::Absent, Kind::Absent]) => Status::Off,
        Some(k) if k.contains(&Kind::Changed) || k.contains(&Kind::Absent) => Status::ChangedByHand,
        Some(k) if k.contains(&Kind::Moved) => Status::BinaryMoved,
        Some(_) => Status::On,
    }
}

/// Why a change was not made. The file is as it was.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Refused {
    Invalid,
    Shape,
    ChangedByHand,
    Io,
}

impl Refused {
    pub(crate) fn text(&self) -> &'static str {
        match self {
            Refused::Invalid => {
                "the account's settings.json is not a JSON object; it was left as it is"
            }
            Refused::Shape => {
                "the account's settings.json has an unexpected hooks shape; it was left as it is"
            }
            Refused::ChangedByHand => {
                "an entry of ours was changed by hand; fix or remove it yourself, then try again"
            }
            Refused::Io => "the account's settings.json cannot be read or written right now",
        }
    }
}

/// Adds what is missing. `Ok(false)`: both entries were there, nothing to do.
fn install_into(settings: &mut Value, command: &str) -> Result<bool, Refused> {
    let found = kinds(settings, command).ok_or(Refused::Shape)?;
    if found.contains(&Kind::Changed) {
        return Err(Refused::ChangedByHand);
    }
    if found == [Kind::Equal, Kind::Equal] {
        return Ok(false);
    }
    let hooks = settings
        .as_object_mut()
        .ok_or(Refused::Shape)?
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or(Refused::Shape)?;
    for (event, kind) in [PRE, PERM].into_iter().zip(found) {
        if kind == Kind::Equal {
            continue;
        }
        let list = hooks
            .entry(event)
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or(Refused::Shape)?;
        list.retain(|e| classify(e, event, command) != Some(Kind::Moved));
        list.push(entry(event, command));
    }
    Ok(true)
}

/// Drops entries that are ours and equal or moved. `true` when something went.
fn remove_from(settings: &mut Value, command: &str) -> bool {
    let mut removed = false;
    let Some(hooks) = settings.get_mut("hooks").and_then(Value::as_object_mut) else {
        return false;
    };
    for event in [PRE, PERM] {
        let Some(list) = hooks.get_mut(event).and_then(Value::as_array_mut) else {
            continue;
        };
        let before = list.len();
        list.retain(|e| !matches!(classify(e, event, command), Some(Kind::Equal | Kind::Moved)));
        let went = list.len() != before;
        let empty = list.is_empty();
        removed |= went;
        if went && empty {
            hooks.remove(event); // the list was ours alone
        }
    }
    if hooks.is_empty() && removed {
        settings.as_object_mut().map(|root| root.remove("hooks"));
    }
    removed
}

// ---- the file ----

struct Loaded {
    /// The file written through any symlink (the temp file is made beside it).
    path: PathBuf,
    /// `None`: there is no file yet.
    old: Option<Vec<u8>>,
    value: Value,
}

fn load(dir: &Path) -> Result<Loaded, Refused> {
    if !dir.is_dir() {
        return Err(Refused::Io);
    }
    let link = dir.join(SETTINGS);
    if fs::symlink_metadata(&link).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
        return Ok(Loaded {
            path: link,
            old: None,
            value: json!({}),
        });
    }
    let path = fs::canonicalize(&link).map_err(|_| Refused::Io)?;
    let meta = fs::metadata(&path).map_err(|_| Refused::Io)?;
    if !meta.is_file() || meta.len() > MAX_SETTINGS_BYTES {
        return Err(Refused::Io);
    }
    let mut old = Vec::new();
    fs::File::open(&path)
        .and_then(|mut f| f.read_to_end(&mut old))
        .map_err(|_| Refused::Io)?;
    match serde_json::from_slice::<Value>(&old) {
        Ok(value) if value.is_object() => Ok(Loaded {
            path,
            old: Some(old),
            value,
        }),
        _ => Err(Refused::Invalid),
    }
}

/// The status of the account whose folder is `dir`.
pub(crate) fn status_in(dir: &Path, command: &str) -> Status {
    load(dir).map_or(Status::Unreadable, |l| status_of(&l.value, command))
}

/// A copy of the old file in `backups`, owner-only. Never overwrites a copy.
fn backup(backups: &Path, tag: &str, old: &[u8]) -> Result<(), Refused> {
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(backups)
        .map_err(|_| Refused::Io)?;
    let now = now_micros();
    let stamp = jiff::Timestamp::from_microsecond(now)
        .map_err(|_| Refused::Io)?
        .strftime("%Y%m%dT%H%M%S");
    let name = format!(
        "settings-{tag}-{stamp}-{:06}.json",
        now.rem_euclid(1_000_000)
    );
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(backups.join(name))
        .map_err(|_| Refused::Io)?;
    file.write_all(old)
        .and_then(|()| file.sync_all())
        .map_err(|_| Refused::Io)
}

/// A temp file beside `path` with `bytes`, renamed over it; the old file's permissions are kept (else 0600).
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Refused> {
    let mode = fs::metadata(path).map_or(0o600, |m| m.permissions().mode() & 0o777);
    let tmp = path.with_file_name(format!("{SETTINGS}.easy-personal.tmp"));
    let _ = fs::remove_file(&tmp);
    let written = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(&tmp)
        .and_then(|mut f| f.write_all(bytes).and_then(|()| f.sync_all()))
        .and_then(|()| fs::rename(&tmp, path));
    written.map_err(|_| {
        let _ = fs::remove_file(&tmp);
        Refused::Io
    })
}

/// Turns the bridge on or off for the account whose folder is `dir`. `Ok(true)`: the file was changed.
pub(crate) fn apply(
    dir: &Path,
    command: &str,
    on: bool,
    backups: &Path,
    tag: &str,
) -> Result<bool, Refused> {
    let loaded = load(dir)?;
    let mut value = loaded.value.clone();
    let changed = if on {
        install_into(&mut value, command)?
    } else {
        remove_from(&mut value, command)
    };
    if !changed {
        return Ok(false);
    }
    let mut text = serde_json::to_string_pretty(&value).map_err(|_| Refused::Io)?;
    text.push('\n');
    if let Some(old) = &loaded.old {
        backup(backups, tag, old)?;
    }
    write_atomic(&loaded.path, text.as_bytes())?;
    Ok(true)
}

// ---- the accounts ----

pub(crate) struct Account {
    pub(crate) id: String,
    pub(crate) label: String,
    folder: PathBuf,
}

/// The confirmed Claude accounts, or the one `reference` (an id or a label) names.
fn accounts(conn: &Connection, reference: Option<&str>) -> Result<Vec<Account>, &'static str> {
    const NOT_CLAUDE: &str = "the account is not a confirmed Claude account";
    let one = match reference {
        Some(reference) => Some(claude_account_id(conn, reference).map_err(|_| NOT_CLAUDE)?),
        None => None,
    };
    let mut statement = conn
        .prepare(
            "SELECT id, label, folder FROM agent_accounts \
             WHERE provider = 'claude' AND status = 'confirmed' AND (?1 IS NULL OR id = ?1) ORDER BY id",
        )
        .map_err(|_| "the store cannot be read right now; try again in a moment")?;
    let rows = statement
        .query_map([one], |r| {
            Ok(Account {
                id: r.get::<_, i64>(0)?.to_string(),
                label: r.get(1)?,
                folder: PathBuf::from(r.get::<_, String>(2)?),
            })
        })
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        .map_err(|_| "the store cannot be read right now; try again in a moment")?;
    if reference.is_some() && rows.is_empty() {
        return Err(NOT_CLAUDE);
    }
    Ok(rows)
}

/// The folder `backups/` lives in: the store's folder.
fn backups_dir() -> Result<PathBuf, &'static str> {
    store::store_path()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("backups")))
        .ok_or("no data folder")
}

// ---- the CLI ----

#[derive(Subcommand)]
pub enum Action {
    /// Show the two entries, ask, then add them to an account's user settings.json (add-only; the old file is copied to backups/ first)
    Install(Target),
    /// Remove the entries `install` added (an entry changed by hand is kept)
    Remove(Target),
    /// Print on, off, changed by hand or binary moved for each confirmed Claude account (or one)
    Status {
        /// The account (id or label of a confirmed Claude account)
        #[arg(long)]
        account: Option<String>,
    },
}

#[derive(Args)]
pub struct Target {
    /// The account (id or label of a confirmed Claude account); without it, the default account
    #[arg(long)]
    account: Option<String>,
    /// Skip the confirmation question
    #[arg(long)]
    yes: bool,
}

pub fn run(action: Action) -> ExitCode {
    cli_cmds::finish(command(action))
}

fn command(action: Action) -> Run {
    let exe = own_command().ok_or_else(|| Stop::failed(1, "cannot find this program's path"))?;
    let conn = cli_cmds::open_read()?;
    match action {
        Action::Status { account } => {
            for a in accounts(&conn, account.as_deref()).map_err(Stop::refused)? {
                cli_cmds::put(&format!(
                    "{} {:?}: {}\n",
                    a.id,
                    a.label,
                    status_in(&a.folder, &exe).word()
                ))?;
            }
            Ok(())
        }
        Action::Install(t) => change(&conn, &exe, true, t),
        Action::Remove(t) => change(&conn, &exe, false, t),
    }
}

/// The default account is the one whose folder is the agent root's `.claude`.
fn target_account(conn: &Connection, reference: Option<&str>) -> Result<Account, Stop> {
    let mut found = accounts(conn, reference).map_err(Stop::refused)?;
    if reference.is_some() {
        return Ok(found.remove(0));
    }
    let default = easy_personal_core::metrics::resolve_path(&api::agent_root().join(".claude"));
    found
        .into_iter()
        .find(|a| easy_personal_core::metrics::resolve_path(&a.folder) == default)
        .ok_or_else(|| Stop::refused("no confirmed default Claude account; pass --account"))
}

fn change(conn: &Connection, exe: &str, on: bool, t: Target) -> Run {
    let account = target_account(conn, t.account.as_deref())?;
    let dir = account.folder.clone();
    if on {
        cli_cmds::put(&format!(
            "Adds to {SETTINGS} of account {} {:?}:\n{}\n",
            account.id,
            account.label,
            block_text(exe)
        ))?;
        cli_cmds::put("Sessions that were already running are not covered; start a new one.\n")?;
    } else {
        cli_cmds::put(&format!(
            "Removes only the entries equal to the ones `install` adds, from {SETTINGS} of account {} {:?}.\n",
            account.id, account.label
        ))?;
    }
    if !t.yes && !cli_cmds::ask()? {
        cli_cmds::put(&format!("{}\n", cli_cmds::ABORTED))?;
        return Err(Stop::silent(1));
    }
    let backups = backups_dir().map_err(|e| Stop::failed(1, e))?;
    let changed =
        apply(&dir, exe, on, &backups, &account.id).map_err(|e| Stop::refused(e.text()))?;
    cli_cmds::put(&format!(
        "{}: {}\n",
        if changed { "done" } else { "nothing to change" },
        status_in(&dir, exe).word()
    ))
}

/// Is the bridge on (or changed, or moved: entries are there) for any confirmed Claude account of the store at `store`?
/// `serve` opens the answer socket only then. Reads the store read-only and each account's settings file; writes nothing.
pub(crate) fn any_on(store: &Path) -> bool {
    let (Some(command), Ok(conn)) = (own_command(), open_checked(store)) else {
        return false;
    };
    accounts(&conn, None).is_ok_and(|rows| {
        rows.iter().any(|a| {
            !matches!(
                status_in(&a.folder, &command),
                Status::Off | Status::Unreadable
            )
        })
    })
}

// ---- the routes ----

const OFFLINE: &str = "this program's path cannot be used; start the app from its installed copy";

fn account_json(a: &Account, command: &str) -> Value {
    json!({"id": a.id, "label": a.label, "status": status_in(&a.folder, command).code()})
}

/// `GET /api/hooks` -> `{command, entries_text, accounts: [{id, label, status}]}`. `entries_text` is the exact block that
/// *Turn on* merges in, for the confirm step.
pub(crate) async fn get(State(api): State<Api>) -> Response {
    blocking(move || {
        let Some(command) = own_command() else {
            return fail(StatusCode::SERVICE_UNAVAILABLE, OFFLINE);
        };
        let rows = match open_accounts(&api, None) {
            Ok(rows) => rows,
            Err(answer) => return answer,
        };
        api::respond(
            StatusCode::OK,
            &json!({
                "command": command,
                "entries_text": block_text(&command),
                "accounts": rows.iter().map(|a| account_json(a, &command)).collect::<Vec<_>>(),
            }),
        )
    })
    .await
}

fn open_accounts(api: &Api, reference: Option<&str>) -> Result<Vec<Account>, Response> {
    let path = (api.store_path)().map_err(|e| {
        let (status, text) = api::store_failure(e);
        fail(status, &text)
    })?;
    let conn = open_checked(&path).map_err(|e| {
        let (status, text) = api::store_failure(e);
        fail(status, &text)
    })?;
    accounts(&conn, reference).map_err(|text| {
        let status = if reference.is_some() {
            StatusCode::BAD_REQUEST
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        };
        fail(status, text)
    })
}

async fn blocking(work: impl FnOnce() -> Response + Send + 'static) -> Response {
    tokio::task::spawn_blocking(work)
        .await
        .unwrap_or_else(|_| fail(StatusCode::INTERNAL_SERVER_ERROR, api::INTERNAL))
}

/// `PUT /api/hooks` `{account, on, confirmed: true}` -> 200 the account's row after the change; 400 a bad body; 409 the
/// file was refused (not JSON, odd shape, an entry changed by hand), left as it was.
pub(crate) async fn put(State(api): State<Api>, req: Request) -> Response {
    let (_, raw) = match raw_body(req).await {
        Ok(body) => body,
        Err(answer) => return answer,
    };
    let Some(Value::Object(body)) = parse(&raw) else {
        return fail(StatusCode::BAD_REQUEST, "body must be a JSON object");
    };
    if body
        .keys()
        .any(|k| !["account", "on", "confirmed"].contains(&k.as_str()))
    {
        return fail(StatusCode::BAD_REQUEST, "unknown field");
    }
    let (Some(Value::String(account)), Some(Value::Bool(on))) =
        (body.get("account"), body.get("on"))
    else {
        return fail(
            StatusCode::BAD_REQUEST,
            "account must be an account id and on a boolean",
        );
    };
    if body.get("confirmed") != Some(&Value::Bool(true)) {
        return fail(
            StatusCode::BAD_REQUEST,
            "confirmed must be true: this changes the account's Claude Code settings file",
        );
    }
    let (account, on) = (account.clone(), *on);
    blocking(move || {
        let Some(command) = own_command() else {
            return fail(StatusCode::SERVICE_UNAVAILABLE, OFFLINE);
        };
        let found = match open_accounts(&api, Some(&account)) {
            Ok(mut rows) => rows.remove(0),
            Err(answer) => return answer,
        };
        let Ok(backups) = backups_dir() else {
            return fail(StatusCode::SERVICE_UNAVAILABLE, "no data folder");
        };
        match apply(&found.folder, &command, on, &backups, &found.id) {
            Ok(_) => {
                if on {
                    crate::answer_bridge::open_socket(&api.socket, &api.answers);
                }
                api::respond(StatusCode::OK, &account_json(&found, &command))
            }
            Err(Refused::Io) => fail(StatusCode::SERVICE_UNAVAILABLE, Refused::Io.text()),
            Err(refused) => fail(StatusCode::CONFLICT, refused.text()),
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh temp folder with an account folder `acc/` (and room for `data/`).
    struct Temp(PathBuf);

    impl Temp {
        fn new(tag: &str) -> Temp {
            let dir = std::env::temp_dir().join(format!(
                "easy-personal-rs-t106-hooks-{tag}-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(dir.join("acc")).unwrap();
            Temp(dir)
        }
        fn acc(&self) -> PathBuf {
            self.0.join("acc")
        }
        fn settings(&self) -> PathBuf {
            self.acc().join(SETTINGS)
        }
        fn backups(&self) -> PathBuf {
            self.0.join("data").join("backups")
        }
        /// A program that exists, and its command.
        fn command(&self) -> String {
            let exe = self.0.join("bin").join(PROGRAM);
            fs::create_dir_all(exe.parent().unwrap()).unwrap();
            fs::write(&exe, "").unwrap();
            command_text(&exe)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A settings file with the owner's other hooks, permissions and keys of every kind.
    const OWNER: &str = r#"{
  "permissions": {"allow": ["Bash(ls:*)"], "deny": []},
  "env": {"FOO": "bar"},
  "unknownKey": {"nested": [1, 2.5, null, true]},
  "hooks": {
    "PreToolUse": [
      {"matcher": "Bash", "hooks": [{"type": "command", "command": "/usr/bin/audit", "timeout": 5}]},
      {"matcher": "AskUserQuestion", "hooks": [{"type": "command", "command": "/opt/mine/ask-hook"}]}
    ],
    "PostToolUse": [{"hooks": [{"type": "command", "command": "echo done"}]}]
  }
}"#;

    fn read(path: &Path) -> Value {
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
    }

    /// AC 1: golden. Exactly the two entries are added; every other key and hook equals what was there.
    #[test]
    fn install_adds_exactly_two_entries_and_keeps_everything_else() {
        let t = Temp::new("golden");
        let command = t.command();
        fs::write(t.settings(), OWNER).unwrap();
        assert_eq!(apply(&t.acc(), &command, true, &t.backups(), "1"), Ok(true));
        let mut want: Value = serde_json::from_str(OWNER).unwrap();
        want["hooks"]["PreToolUse"]
            .as_array_mut()
            .unwrap()
            .push(json!({"matcher": "AskUserQuestion", "hooks": [
                {"type": "command", "command": command, "timeout": 130}]}));
        want["hooks"]["PermissionRequest"] =
            json!([{"hooks": [{"type": "command", "command": command, "timeout": 130}]}]);
        assert_eq!(read(&t.settings()), want);
        assert_eq!(status_in(&t.acc(), &command), Status::On);
        assert!(fs::read_to_string(t.settings()).unwrap().ends_with("}\n"));
        // temp + rename: no temp file is left
        assert!(!t.acc().join("settings.json.easy-personal.tmp").exists());
        // the old file is in backups/, byte for byte, owner-only
        let copies: Vec<_> = fs::read_dir(t.backups()).unwrap().collect();
        assert_eq!(copies.len(), 1);
        let copy = copies[0].as_ref().unwrap().path();
        assert_eq!(fs::read_to_string(&copy).unwrap(), OWNER);
        assert_eq!(
            fs::metadata(&copy).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(t.backups()).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }

    /// The text the confirm step shows is the text that is merged in.
    #[test]
    fn the_block_shown_is_the_block_merged() {
        let t = Temp::new("block");
        let command = t.command();
        apply(&t.acc(), &command, true, &t.backups(), "1").unwrap();
        let shown: Value = serde_json::from_str(&block_text(&command)).unwrap();
        assert_eq!(
            read(&t.settings()),
            shown,
            "a new file holds just the block"
        );
        assert!(block_text(&command).contains("\"timeout\": 130"));
        assert!(block_text(&command).contains("\"matcher\": \"AskUserQuestion\""));
        assert!(fs::read_dir(t.backups()).is_err(), "no old file, no backup");
    }

    /// AC 1: an invalid file, a file that is not an object and an odd `hooks` shape are refused byte for byte.
    #[test]
    fn a_file_that_cannot_be_merged_is_refused_and_left_as_it_is() {
        for (tag, text, want) in [
            ("bad", "{\"hooks\": ", Refused::Invalid),
            ("array", "[1, 2]", Refused::Invalid),
            ("empty", "", Refused::Invalid),
            ("shape", "{\"hooks\": []}", Refused::Shape),
            (
                "shape2",
                "{\"hooks\": {\"PreToolUse\": {}}}",
                Refused::Shape,
            ),
        ] {
            let t = Temp::new(tag);
            let command = t.command();
            fs::write(t.settings(), text).unwrap();
            assert_eq!(
                apply(&t.acc(), &command, true, &t.backups(), "1"),
                Err(want),
                "{tag}"
            );
            assert_eq!(fs::read_to_string(t.settings()).unwrap(), text, "{tag}");
            assert!(!t.backups().exists(), "{tag}: no backup of a refused file");
        }
    }

    /// AC 2: a second install writes nothing at all (no new backup, same bytes).
    #[test]
    fn installing_twice_adds_nothing() {
        let t = Temp::new("twice");
        let command = t.command();
        fs::write(t.settings(), OWNER).unwrap();
        apply(&t.acc(), &command, true, &t.backups(), "1").unwrap();
        let once = fs::read(t.settings()).unwrap();
        assert_eq!(
            apply(&t.acc(), &command, true, &t.backups(), "1"),
            Ok(false)
        );
        assert_eq!(fs::read(t.settings()).unwrap(), once);
        assert_eq!(fs::read_dir(t.backups()).unwrap().count(), 1);
    }

    /// AC 2: remove puts the owner's file back (as a value); a hand-edited entry stays and `status` says so.
    #[test]
    fn remove_drops_only_equal_entries_and_a_hand_edited_one_stays() {
        let t = Temp::new("remove");
        let command = t.command();
        fs::write(t.settings(), OWNER).unwrap();
        apply(&t.acc(), &command, true, &t.backups(), "1").unwrap();
        assert_eq!(
            apply(&t.acc(), &command, false, &t.backups(), "1"),
            Ok(true)
        );
        assert_eq!(
            read(&t.settings()),
            serde_json::from_str::<Value>(OWNER).unwrap()
        );
        assert_eq!(status_in(&t.acc(), &command), Status::Off);
        assert_eq!(
            apply(&t.acc(), &command, false, &t.backups(), "1"),
            Ok(false)
        );

        // hand-edited: the timeout of the permission entry is changed
        apply(&t.acc(), &command, true, &t.backups(), "1").unwrap();
        let mut value = read(&t.settings());
        value["hooks"]["PermissionRequest"][0]["hooks"][0]["timeout"] = json!(7);
        fs::write(t.settings(), serde_json::to_string(&value).unwrap()).unwrap();
        assert_eq!(status_in(&t.acc(), &command), Status::ChangedByHand);
        assert_eq!(
            apply(&t.acc(), &command, false, &t.backups(), "1"),
            Ok(true)
        );
        let after = read(&t.settings());
        assert_eq!(
            after["hooks"]["PermissionRequest"][0]["hooks"][0]["timeout"], 7,
            "kept"
        );
        assert_eq!(
            after["hooks"]["PreToolUse"].as_array().unwrap().len(),
            2,
            "the equal one went"
        );
        // and install refuses over it, naming nothing of the file
        assert_eq!(
            apply(&t.acc(), &command, true, &t.backups(), "1"),
            Err(Refused::ChangedByHand)
        );
    }

    /// AC 2: our entries naming a program that is gone read *binary moved*; install replaces them, remove drops them.
    #[test]
    fn a_gone_binary_reads_as_moved_and_install_replaces_the_entries() {
        let t = Temp::new("moved");
        let old = t.0.join("old").join(PROGRAM);
        let old_command = command_text(&old); // never created
        fs::write(t.settings(), "{}").unwrap();
        // write the entries by hand with the dead path
        let block: Value = serde_json::from_str(&block_text(&old_command)).unwrap();
        fs::write(t.settings(), block.to_string()).unwrap();
        let command = t.command();
        assert_eq!(status_in(&t.acc(), &command), Status::BinaryMoved);
        // the same path that exists would be `on`
        assert_eq!(
            status_in(&t.acc(), &old_command),
            Status::BinaryMoved,
            "even for itself"
        );
        assert_eq!(apply(&t.acc(), &command, true, &t.backups(), "1"), Ok(true));
        assert_eq!(status_in(&t.acc(), &command), Status::On);
        let value = read(&t.settings());
        assert_eq!(value["hooks"]["PreToolUse"].as_array().unwrap().len(), 1);
        assert_eq!(
            value["hooks"]["PreToolUse"][0]["hooks"][0]["command"],
            json!(command)
        );
        // and remove drops moved ones
        fs::write(t.settings(), block.to_string()).unwrap();
        assert_eq!(
            apply(&t.acc(), &command, false, &t.backups(), "1"),
            Ok(true)
        );
        assert_eq!(read(&t.settings()), json!({}));
    }

    /// One of the two entries alone is *changed by hand* (someone removed the other); install adds the missing one.
    #[test]
    fn half_installed_is_changed_by_hand_and_install_completes_it() {
        let t = Temp::new("half");
        let command = t.command();
        apply(&t.acc(), &command, true, &t.backups(), "1").unwrap();
        let mut value = read(&t.settings());
        value["hooks"].as_object_mut().unwrap().remove(PERM);
        fs::write(t.settings(), value.to_string()).unwrap();
        assert_eq!(status_in(&t.acc(), &command), Status::ChangedByHand);
        assert_eq!(apply(&t.acc(), &command, true, &t.backups(), "1"), Ok(true));
        assert_eq!(status_in(&t.acc(), &command), Status::On);
        assert_eq!(
            read(&t.settings())["hooks"]["PreToolUse"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    /// The owner's own hook that merely mentions `hook ask` is not ours, and no file at all is *off*.
    #[test]
    fn the_owners_hooks_are_never_taken_for_ours() {
        let t = Temp::new("own");
        let command = t.command();
        assert_eq!(status_in(&t.acc(), &command), Status::Off, "no file");
        fs::write(
            t.settings(),
            r#"{"hooks": {"PreToolUse": [{"hooks": [{"type": "command", "command": "/x/other hook ask"}]}]}}"#,
        )
        .unwrap();
        assert_eq!(status_in(&t.acc(), &command), Status::Off);
        assert_eq!(
            apply(&t.acc(), &command, false, &t.backups(), "1"),
            Ok(false)
        );
        // a missing account folder is not created
        let none = t.0.join("nope");
        assert_eq!(
            apply(&none, &command, true, &t.backups(), "1"),
            Err(Refused::Io)
        );
        assert!(!none.exists());
    }

    /// A symlinked settings.json is written through; the link stays a link. A file's mode is kept.
    #[test]
    fn a_symlinked_file_is_written_through_and_its_mode_kept() {
        let t = Temp::new("link");
        let command = t.command();
        let real = t.0.join("dotfiles-settings.json");
        fs::write(&real, "{\"a\": 1}").unwrap();
        fs::set_permissions(&real, fs::Permissions::from_mode(0o640)).unwrap();
        std::os::unix::fs::symlink(&real, t.settings()).unwrap();
        apply(&t.acc(), &command, true, &t.backups(), "1").unwrap();
        assert!(
            fs::symlink_metadata(t.settings())
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(read(&real)["a"], 1);
        assert_eq!(status_in(&t.acc(), &command), Status::On);
        assert_eq!(
            fs::metadata(&real).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    /// A program path with a space or a quote is quoted, and read back as the same program.
    #[test]
    fn a_path_with_a_space_or_quote_is_quoted_and_read_back() {
        for path in [
            "/opt/my apps/easy-personal-rs",
            "/opt/it's/easy-personal-rs",
            "/opt/a/easy-personal-rs",
        ] {
            let command = command_text(Path::new(path));
            assert!(command.ends_with(" hook ask"), "{command}");
            assert_eq!(program_of(&command), Some(PathBuf::from(path)), "{command}");
        }
        assert_eq!(program_of("/opt/x/other hook ask"), None);
        assert_eq!(program_of("/opt/x/easy-personal-rs mcp"), None);
    }
}
