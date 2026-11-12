//! The store-only write routes of the dashboard API (T048): `POST /api/workspaces`, `PATCH` and `DELETE
//! /api/workspaces/{id}`, `POST /api/accounts`, `POST /api/accounts/{id}/confirm`, `POST /api/accounts/{id}/ignore` and
//! `PUT /api/accounts/{id}/workspaces`. They answer Python's status codes and texts (`surfaces/api.py`: `_object_body`,
//! `_json_body`, `_workspace_call`, `_account_write`, `_run_writer`); the oracle is `rust/parity/golden-writes/`.
//!
//! Order of a request: the guard in `server.rs` (token, Host, Origin), then the content type (415), the body (400, 413),
//! the id in the path (404) and the `confirm` flag (400), and only then the store: a bad request never opens it for
//! writing. The store is opened for writing inside the request (`workspaces::write_session`: it never creates a store,
//! needs schema v11, waits 5 s for a busy writer) and closed before the answer. One in-process lock lets one write run at
//! a time, as Python's `_WRITE_LOCK` does for the connector routes; there is no `ingest.lock` (Python takes none).
//!
//! A change that would need the `claude` CLI (an account registration to add or remove) is refused with a fixed 409 and
//! changes nothing, until T050. No answer names the caller's input: a body or store problem is a fixed text.

// The error of the body readers is the finished answer (a `Response`), which an early `return` would send as it is.
#![allow(clippy::result_large_err)]

use crate::api::{Api, INTERNAL, agent_root, error_body, report_id, respond};
use axum::body::to_bytes;
use axum::extract::{Request, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use easy_personal_core::accounts;
use easy_personal_core::store::StoreError;
use easy_personal_core::workspaces::{self, FIELDS, Outcome, WriteFail, py_strip};
use easy_personal_core::writer::WriteError;
use rusqlite::Connection;
use serde_json::{Map, Value, json};
use std::path::PathBuf;
use std::sync::Mutex;

/// Python has no cap on a body; this build refuses one over 1 MiB (a dashboard form is a few hundred bytes).
const MAX_BODY: usize = 1 << 20;
/// `_WRITE_LOCK`, widened to every write route here.
static WRITE_LOCK: Mutex<()> = Mutex::new(());

const NEEDS_CLAUDE: &str = "this change must update an agent account's Claude Code entries, which needs the claude CLI; \
                            this build cannot run it yet, so nothing was changed";
const BUSY: &str = "the store is busy (another process is writing); try again in a moment";
const NOT_WRITABLE: &str = "the store cannot be written right now; try again in a moment";

fn fail(status: StatusCode, text: &str) -> Response {
    respond(status, &error_body(text))
}

/// `request.headers.get("content-type", "").split(";")[0].strip().lower() == "application/json"` (a header read as
/// Latin-1, the first of a repeated one).
fn is_json_type(headers: &HeaderMap) -> bool {
    let raw: String = headers
        .get(CONTENT_TYPE)
        .map(|v| v.as_bytes().iter().map(|&b| b as char).collect())
        .unwrap_or_default();
    py_strip(raw.split(';').next().unwrap_or("")).eq_ignore_ascii_case("application/json")
}

/// The path and the body bytes, or the answer: 415 for another content type, 413 for a body over the cap.
async fn raw_body(req: Request) -> Result<(String, Vec<u8>), Response> {
    let (parts, body) = req.into_parts();
    if !is_json_type(&parts.headers) {
        return Err(fail(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "content-type must be application/json",
        ));
    }
    match to_bytes(body, MAX_BODY).await {
        Ok(bytes) => Ok((parts.uri.path().to_string(), bytes.to_vec())),
        Err(_) => Err(fail(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request body too large",
        )),
    }
}

/// `json.loads` of the bytes. A UTF-8 byte order mark is skipped, as Python's detection does. Where Python's parser
/// accepts text this one does not (`NaN`, `Infinity`, a number past a double, a lone surrogate escape, nesting past
/// 128, UTF-16 or UTF-32), the answer is the 400 for bad JSON.
fn parse(raw: &[u8]) -> Option<Value> {
    serde_json::from_slice(raw.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(raw)).ok()
}

/// A workspace route's body (`_object_body`): JSON, an object; `empty_ok` lets an empty body stand for `{}` (DELETE).
async fn object_body(
    req: Request,
    empty_ok: bool,
) -> Result<(String, Vec<u8>, Map<String, Value>), Response> {
    let (path, raw) = raw_body(req).await?;
    if empty_ok
        && raw
            .iter()
            .all(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c))
    {
        return Ok((path, raw, Map::new()));
    }
    match parse(&raw) {
        None => Err(fail(StatusCode::BAD_REQUEST, "body must be JSON")),
        Some(Value::Object(map)) => Ok((path, raw, map)),
        Some(_) => Err(fail(StatusCode::BAD_REQUEST, "body must be a JSON object")),
    }
}

/// An account route's body (`_json_body`): JSON, an object whose `confirm`, when present, is a boolean.
async fn account_body(req: Request) -> Result<(String, Map<String, Value>), Response> {
    let (path, raw) = raw_body(req).await?;
    match parse(&raw) {
        None => Err(fail(StatusCode::BAD_REQUEST, "body must be JSON")),
        Some(Value::Object(map)) if map.get("confirm").is_none_or(Value::is_boolean) => {
            Ok((path, map))
        }
        Some(_) => Err(fail(
            StatusCode::BAD_REQUEST,
            "body must be an object with boolean confirm",
        )),
    }
}

/// The keys of the top-level object in the order they first appear (a repeated key keeps its first place, as Python's
/// `dict` does): the order of the checks of a `PATCH` follows it. The text is valid JSON already.
fn key_order(raw: &[u8]) -> Vec<String> {
    let raw = raw.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(raw);
    let (mut keys, mut depth, mut want_key, mut i) = (Vec::new(), 0usize, false, 0usize);
    while i < raw.len() {
        match raw[i] {
            b'{' | b'[' => {
                depth += 1;
                want_key = depth == 1 && raw[i] == b'{';
                i += 1;
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                i += 1;
            }
            b',' => {
                want_key = depth == 1;
                i += 1;
            }
            b'"' => {
                let start = i;
                i += 1;
                while i < raw.len() && raw[i] != b'"' {
                    i += if raw[i] == b'\\' { 2 } else { 1 };
                }
                i += 1;
                if depth == 1 && want_key {
                    want_key = false;
                    if let Some(key) = raw
                        .get(start..i)
                        .and_then(|t| serde_json::from_slice::<String>(t).ok())
                        && !keys.contains(&key)
                    {
                        keys.push(key);
                    }
                }
            }
            _ => i += 1,
        }
    }
    keys
}

/// `_confirmed`: absent is false; anything but a boolean is a 400.
fn confirmed(body: &mut Map<String, Value>) -> Result<bool, Response> {
    match body.remove("confirm") {
        None => Ok(false),
        Some(Value::Bool(yes)) => Ok(yes),
        Some(_) => Err(fail(
            StatusCode::BAD_REQUEST,
            "confirm must be true or false",
        )),
    }
}

/// `_workspace_id` on the path `/api/workspaces/{id}`: plain digits (at most 18) or none.
fn workspace_id(path: &str) -> Option<i64> {
    report_id(path.strip_prefix("/api/workspaces/")?)
}

/// The `{id}` of `/api/accounts/{id}/...` as the digits Python's `_find` reads, or none (`no such account`).
fn account_id(path: &str) -> Option<String> {
    let segment = path.strip_prefix("/api/accounts/")?.split('/').next()?;
    report_id(segment).map(|id| id.to_string())
}

/// What a refusal looks like on the wire: Python's status and text, or a fixed one.
fn refusal(failure: WriteFail) -> Response {
    match failure {
        WriteFail::Refused { status, text } => fail(
            StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_REQUEST),
            &text,
        ),
        WriteFail::NeedsClaude => fail(StatusCode::CONFLICT, NEEDS_CLAUDE),
        WriteFail::Store(error) => {
            let text = match &error {
                StoreError::NoStore | StoreError::Newer { .. } | StoreError::NoHome => {
                    error.to_string()
                }
                StoreError::Busy => BUSY.to_string(),
                StoreError::NotAFile | StoreError::Unreadable(_) => NOT_WRITABLE.to_string(),
            };
            fail(StatusCode::SERVICE_UNAVAILABLE, &text)
        }
        WriteFail::Write(error) => {
            let text = match &error {
                WriteError::Newer { .. } | WriteError::Older { .. } => error.to_string(),
                WriteError::Busy => BUSY.to_string(),
                _ => NOT_WRITABLE.to_string(),
            };
            fail(StatusCode::SERVICE_UNAVAILABLE, &text)
        }
    }
}

/// Runs `work` in a write session off the async workers, under the write lock. `detect` is the agent root whose
/// folders are detected first (the account routes: Python's `accounts.detect`).
async fn run<F>(api: Api, detect: Option<PathBuf>, status: StatusCode, work: F) -> Response
where
    F: FnOnce(&Connection) -> Result<Value, WriteFail> + Send + 'static,
{
    let outcome = tokio::task::spawn_blocking(move || {
        let path = (api.store_path)().map_err(WriteFail::Store)?;
        let _one_at_a_time = WRITE_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        workspaces::write_session(&path, detect.as_deref(), work)
    })
    .await;
    match outcome {
        Ok(Ok(body)) => respond(status, &body),
        Ok(Err(failure)) => refusal(failure),
        // the task panicked: say nothing about why
        Err(_) => fail(StatusCode::INTERNAL_SERVER_ERROR, INTERNAL),
    }
}

fn answer(outcome: &Outcome) -> Value {
    json!({ "status": outcome.status, "lines": outcome.lines, "pending": outcome.pending })
}

/// `POST /api/workspaces` -> 201 `{"workspace": ...}`.
pub(crate) async fn workspace_add(State(api): State<Api>, req: Request) -> Response {
    let body = match object_body(req, false).await {
        Ok((_, _, body)) => body,
        Err(response) => return response,
    };
    if body.keys().any(|k| !FIELDS.contains(&k.as_str())) {
        return fail(StatusCode::BAD_REQUEST, "unknown field");
    }
    run(api, None, StatusCode::CREATED, move |conn| {
        Ok(json!({ "workspace": workspaces::add(conn, &body)? }))
    })
    .await
}

/// `PATCH /api/workspaces/{id}`. A new name of a workspace that holds connectors shows the plan; `confirm: true`
/// applies it, or is refused (409) when the plan needs the `claude` CLI.
pub(crate) async fn workspace_edit(State(api): State<Api>, req: Request) -> Response {
    let (path, raw, mut body) = match object_body(req, false).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let Some(id) = workspace_id(&path) else {
        return fail(StatusCode::NOT_FOUND, "unknown workspace");
    };
    let yes = match confirmed(&mut body) {
        Ok(yes) => yes,
        Err(response) => return response,
    };
    let changes: Vec<(String, Value)> = key_order(&raw)
        .into_iter()
        .filter_map(|key| body.remove(&key).map(|value| (key, value)))
        .collect();
    run(api, None, StatusCode::OK, move |conn| {
        let outcome = workspaces::patch(conn, id, changes, yes)?;
        let mut body = answer(&outcome);
        body["workspace"] = workspaces::get(conn, id)?;
        Ok(body)
    })
    .await
}

/// `DELETE /api/workspaces/{id}`: the plan; `confirm: true` deletes it and moves its connectors to orphan (or is
/// refused with 409 when the plan needs the `claude` CLI). An empty body stands for `{}`.
pub(crate) async fn workspace_delete(State(api): State<Api>, req: Request) -> Response {
    let (path, _, mut body) = match object_body(req, true).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let Some(id) = workspace_id(&path) else {
        return fail(StatusCode::NOT_FOUND, "unknown workspace");
    };
    let yes = match confirmed(&mut body) {
        Ok(yes) => yes,
        Err(response) => return response,
    };
    run(api, None, StatusCode::OK, move |conn| {
        let outcome = workspaces::delete(conn, id, yes)?;
        let mut body = answer(&outcome);
        body["deleted"] = if outcome.status == 0 {
            json!(id)
        } else {
            Value::Null
        };
        Ok(body)
    })
    .await
}

/// Parses an account write (`_account_write`): body, then the id of the path, then `work(conn, id, body, root)`.
async fn account_write<F>(
    api: Api,
    req: Request,
    with_id: bool,
    status: StatusCode,
    work: F,
) -> Response
where
    F: FnOnce(&Connection, &str, &Map<String, Value>, &std::path::Path) -> Result<Value, WriteFail>
        + Send
        + 'static,
{
    let (path, body) = match account_body(req).await {
        Ok(parsed) => parsed,
        Err(response) => return response,
    };
    let id = if with_id {
        match account_id(&path) {
            Some(id) => id,
            None => return fail(StatusCode::NOT_FOUND, "no such account"),
        }
    } else {
        String::new()
    };
    let root = agent_root();
    let detect = Some(root.clone());
    run(api, detect, status, move |conn| {
        work(conn, &id, &body, &root)
    })
    .await
}

/// `POST /api/accounts` -> 201, the account row.
pub(crate) async fn account_add(State(api): State<Api>, req: Request) -> Response {
    account_write(
        api,
        req,
        false,
        StatusCode::CREATED,
        |conn, _, body, root| accounts::add(conn, body.get("path").unwrap_or(&Value::Null), root),
    )
    .await
}

/// `POST /api/accounts/{id}/confirm` with `{"label": ...}`.
pub(crate) async fn account_confirm(State(api): State<Api>, req: Request) -> Response {
    account_write(api, req, true, StatusCode::OK, |conn, id, body, root| {
        accounts::confirm(conn, id, body.get("label").unwrap_or(&Value::Null), root)
    })
    .await
}

/// `POST /api/accounts/{id}/ignore`.
pub(crate) async fn account_ignore(State(api): State<Api>, req: Request) -> Response {
    account_write(api, req, true, StatusCode::OK, |conn, id, _, root| {
        accounts::ignore(conn, id, root)
    })
    .await
}

/// `PUT /api/accounts/{id}/workspaces` with `{"workspaces": [names]}`. Store only: it is not synced to Claude Code.
pub(crate) async fn account_workspaces(State(api): State<Api>, req: Request) -> Response {
    account_write(api, req, true, StatusCode::OK, |conn, id, body, root| {
        accounts::use_workspaces(
            conn,
            id,
            body.get("workspaces").unwrap_or(&Value::Null),
            root,
        )
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::key_order;

    #[test]
    fn key_order_is_the_order_of_first_appearance() {
        let order = |text: &str| key_order(text.as_bytes());
        assert_eq!(order(r#"{"b":1,"a":2}"#), ["b", "a"]);
        assert_eq!(
            order(r#"{"b":1,"a":{"z":1,"y":[1,{"q":2}]},"c":"x,\"y\""}"#),
            ["b", "a", "c"]
        );
        assert_eq!(
            order(r#"{"a":1,"b":2,"a":3}"#),
            ["a", "b"],
            "a repeated key keeps its first place"
        );
        assert_eq!(
            order("\u{feff}{\"\\u0062\":1,\"a\":2}"),
            ["b", "a"],
            "an escaped key is decoded"
        );
        assert_eq!(order(r#"{"k, v":"a,b","x":"}"}"#), ["k, v", "x"]);
        assert_eq!(order("{}"), Vec::<String>::new());
    }
}
