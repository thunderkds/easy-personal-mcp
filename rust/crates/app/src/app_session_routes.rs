//! The app session routes (T103, FR-109..FR-111, FR-115, FR-116): `GET /api/app-sessions`, `GET /api/app-sessions/{id}`,
//! `POST /api/app-sessions` (start), and `POST /api/app-sessions/{id}/message|interrupt|stop|answer|close`. They sit
//! behind the guard in `server.rs` (Host, Origin, token); every write needs `confirmed: true` in its body and takes the
//! body rules of `writes.rs` (content type 415, size 413, JSON object 400). The page names no program and no argument:
//! only the fields of the New session form, each checked against its fixed list or the store (`app_session.rs`).
//!
//! The permission tool is one more route of this module, `POST /app-session-mcp/{id}`: a minimal HTTP MCP endpoint of
//! the dashboard process for the one tool `permission_prompt` (T102 AC 5, option (a)). It is outside `/api/` on purpose:
//! `claude` cannot send the page token, it sends the per-session bearer secret from its temp `--mcp-config`; the Host and
//! Origin checks of the guard still apply. It is not part of the MCP tool surface of `mcp.rs` / `mcp_http.rs`.
//!
//! Text (FR-115): the answers of the app session routes carry the session's text and nothing else does; no body, error
//! or status line here names a prompt, a message or a path.

#![allow(clippy::result_large_err)]

use crate::agent_routes::{blocking, data_dir, object};
use crate::agent_run::{Runner, Setting};
use crate::api::{Api, INTERNAL, respond};
use crate::app_session::{
    self as app, Account, Answer, AnswerError, MODELS, MODES, Plan, Session, StartError, TOOL,
};
use crate::writes::{fail, parse};
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::{Path, Request, State};
use axum::http::header::{ALLOW, AUTHORIZATION, CONTENT_TYPE, HOST};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use easy_personal_core::store::open_checked;
use serde_json::{Map, Value, json};
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;

const OFF: &str = "app sessions are off: choose an agent run account in Settings";
const NO_SESSION: &str = "no such session";
const ENDED: &str = "the session has ended";
const UNREADABLE: &str = "the store cannot be read right now; try again in a moment";
const INCOMPLETE: &str =
    "the tool input is too long to show whole, so it cannot be allowed from here; deny it";
const MAX_PROMPT: usize = 100_000;
const MAX_NOTE: usize = 10_000;
/// The largest body of the permission tool's route.
const MAX_MCP_BODY: usize = 1 << 20;

pub(crate) fn routes() -> Router<Api> {
    Router::new()
        .route("/api/app-sessions", get(list).post(start))
        .route("/api/app-sessions/{id}", get(one))
        .route("/api/app-sessions/{id}/message", post(message))
        .route("/api/app-sessions/{id}/interrupt", post(interrupt))
        .route("/api/app-sessions/{id}/stop", post(stop))
        .route("/api/app-sessions/{id}/answer", post(answer))
        .route("/api/app-sessions/{id}/close", post(close))
        .route(
            "/app-session-mcp/{id}",
            post(mcp_post).get(mcp_other).delete(mcp_other),
        )
}

/// One session as the page gets it; a worktree's changed file names are read here, off the async workers.
fn view(session: &Session) -> Value {
    let changed = session
        .worktree_folder()
        .map(|folder| app::changed_files(&folder))
        .unwrap_or_default();
    session.json(&changed)
}

fn found(api: &Api, id: &str) -> Result<Arc<Session>, Response> {
    api.agent
        .sessions
        .get(id)
        .ok_or_else(|| fail(StatusCode::NOT_FOUND, NO_SESSION))
}

/// A write's body: JSON object, only `allowed` keys, `confirmed: true` (FR-116).
async fn write_body(req: Request, allowed: &[&str]) -> Result<Map<String, Value>, Response> {
    let body = object(req, false).await?;
    if body
        .keys()
        .any(|k| k != "confirmed" && !allowed.contains(&k.as_str()))
    {
        return Err(fail(StatusCode::BAD_REQUEST, "unknown field"));
    }
    if body.get("confirmed") != Some(&Value::Bool(true)) {
        return Err(fail(StatusCode::BAD_REQUEST, "confirmed must be true"));
    }
    Ok(body)
}

fn text_field(body: &Map<String, Value>, key: &str, max: usize) -> Result<String, Response> {
    match body.get(key) {
        Some(Value::String(text)) if !text.trim().is_empty() && text.len() <= max => {
            Ok(text.clone())
        }
        _ => Err(fail(
            StatusCode::BAD_REQUEST,
            &format!("{key} must be non-empty text of at most {max} bytes"),
        )),
    }
}

/// `GET /api/app-sessions` -> `{sessions, max}`.
async fn list(State(api): State<Api>) -> Response {
    blocking(move || {
        let sessions: Vec<Value> = api.agent.sessions.all().iter().map(|s| view(s)).collect();
        respond(
            StatusCode::OK,
            &json!({"sessions": sessions, "max": app::MAX_RUNNING}),
        )
    })
    .await
}

/// `GET /api/app-sessions/{id}`.
async fn one(State(api): State<Api>, Path(id): Path<String>) -> Response {
    blocking(move || match found(&api, &id) {
        Ok(session) => respond(StatusCode::OK, &view(&session)),
        Err(answer) => answer,
    })
    .await
}

/// The form of `POST /api/app-sessions`, each field checked against its fixed list.
struct Form {
    repo: String,
    workspace: String,
    account: Option<String>,
    model: &'static str,
    mode: &'static str,
    worktree: bool,
    prompt: String,
}

fn form(body: &Map<String, Value>) -> Result<Form, Response> {
    let name = |key: &str| match body.get(key) {
        Some(Value::String(text)) if !text.is_empty() && text.len() <= 4096 => Ok(text.clone()),
        _ => Err(fail(
            StatusCode::BAD_REQUEST,
            &format!("{key} must be a non-empty string"),
        )),
    };
    let listed = |key: &str, list: &'static [&'static str]| match body.get(key) {
        Some(Value::String(text)) => list
            .iter()
            .find(|item| **item == text)
            .copied()
            .ok_or_else(|| fail(StatusCode::BAD_REQUEST, &format!("unknown {key}"))),
        _ => Err(fail(
            StatusCode::BAD_REQUEST,
            &format!("{key} must be one of the listed values"),
        )),
    };
    let account = match body.get("account") {
        None | Some(Value::Null) => None,
        Some(Value::String(text)) if !text.is_empty() && text.len() <= 200 => Some(text.clone()),
        Some(_) => {
            return Err(fail(
                StatusCode::BAD_REQUEST,
                "account must be an account id or null",
            ));
        }
    };
    let worktree = match body.get("worktree") {
        None => false,
        Some(Value::Bool(flag)) => *flag,
        Some(_) => {
            return Err(fail(
                StatusCode::BAD_REQUEST,
                "worktree must be true or false",
            ));
        }
    };
    Ok(Form {
        repo: name("repo")?,
        workspace: name("workspace")?,
        account,
        model: listed("model", &MODELS)?,
        mode: listed("mode", &MODES)?,
        worktree,
        prompt: text_field(body, "prompt", MAX_PROMPT)?,
    })
}

/// `127.0.0.1:<port>` of the request's `Host` (the guard has checked it is this server's own): the port the child's
/// temp `--mcp-config` names.
fn port_of(headers: &HeaderMap) -> Option<u16> {
    headers
        .get(HOST)?
        .to_str()
        .ok()?
        .strip_prefix("127.0.0.1:")?
        .parse()
        .ok()
}

/// A stored repo of a stored workspace, as an existing folder.
fn stored_repo(dir: &FsPath, workspace: &str, repo: &str) -> Result<PathBuf, Response> {
    let bad = |text: &str| fail(StatusCode::BAD_REQUEST, text);
    let conn = open_checked(&dir.join("store.sqlite"))
        .map_err(|_| fail(StatusCode::SERVICE_UNAVAILABLE, UNREADABLE))?;
    let known = conn
        .query_row(
            "SELECT 1 FROM workspace_repos r JOIN workspaces w ON w.id = r.workspace_id \
             WHERE w.name = ?1 AND r.repo_path = ?2",
            [workspace, repo],
            |_| Ok(()),
        )
        .is_ok();
    if !known {
        return Err(bad("repo must be a repo of the workspace"));
    }
    let path = PathBuf::from(repo);
    if !path.is_absolute() || !path.is_dir() {
        return Err(bad("the repo folder is missing"));
    }
    Ok(path)
}

/// The account the session uses: the one asked for, else the agent run account (`None`: the default account).
fn pick_account(
    runner: &Runner,
    dir: &FsPath,
    asked: Option<&str>,
) -> Result<Option<Account>, Response> {
    let Some(reference) = asked else {
        return Ok(match runner.setting() {
            Setting::Account(chosen) => Some(Account {
                id: chosen.id.clone(),
                label: chosen.label.clone(),
                config_dir: chosen.config_dir_env.clone(),
            }),
            _ => None,
        });
    };
    let bad = || {
        fail(
            StatusCode::BAD_REQUEST,
            "the account is not a confirmed Claude account",
        )
    };
    let id = runner.resolve(reference).map_err(|_| bad())?;
    let conn = open_checked(&dir.join("store.sqlite"))
        .map_err(|_| fail(StatusCode::SERVICE_UNAVAILABLE, UNREADABLE))?;
    conn.query_row(
        "SELECT label, config_dir_env FROM agent_accounts WHERE id = ?1 AND provider = 'claude'",
        [&id],
        |r| {
            Ok(Account {
                id: id.clone(),
                label: r.get(0)?,
                config_dir: r.get(1)?,
            })
        },
    )
    .map(Some)
    .map_err(|_| bad())
}

/// `POST /api/app-sessions` `{repo, workspace, account?, model, mode, worktree, prompt, confirmed: true}` -> 202 `{id}`.
async fn start(State(api): State<Api>, req: Request) -> Response {
    let port = port_of(req.headers());
    let body = match write_body(
        req,
        &[
            "repo",
            "workspace",
            "account",
            "model",
            "mode",
            "worktree",
            "prompt",
        ],
    )
    .await
    {
        Ok(body) => body,
        Err(answer) => return answer,
    };
    let form = match form(&body) {
        Ok(form) => form,
        Err(answer) => return answer,
    };
    let Some(port) = port else {
        return fail(StatusCode::INTERNAL_SERVER_ERROR, INTERNAL);
    };
    let dir = match data_dir(&api) {
        Ok(dir) => dir,
        Err(answer) => return answer,
    };
    blocking(move || {
        let agent = &api.agent;
        let runner = agent.runner(&dir);
        // read now, at this request: the flag may have been written or removed since the last one
        if runner.setting() == Setting::Off {
            return fail(StatusCode::FORBIDDEN, OFF);
        }
        let repo = match stored_repo(&dir, &form.workspace, &form.repo) {
            Ok(repo) => repo,
            Err(answer) => return answer,
        };
        let account = match pick_account(&runner, &dir, form.account.as_deref()) {
            Ok(account) => account,
            Err(answer) => return answer,
        };
        let pinned = account.as_ref().map(|a| a.id.clone());
        let allowed_runner = agent.runner(&dir);
        // the setting is the flag file (removed by `off` and when the agent run account is ignored); a named account
        // that cannot be found three checks in a row (one a second) counts as gone: a busy store is not "gone"
        let misses = std::sync::atomic::AtomicU8::new(0);
        let allowed = Box::new(move || {
            if !allowed_runner.enabled() {
                return false;
            }
            let Some(id) = pinned.as_deref() else {
                return true;
            };
            if allowed_runner.resolve(id).is_ok() {
                misses.store(0, std::sync::atomic::Ordering::Relaxed);
                return true;
            }
            misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1 < 3
        });
        let plan = Plan {
            repo,
            account,
            model: form.model,
            mode: form.mode,
            worktree: form.worktree,
            prompt: form.prompt,
        };
        match agent.sessions.start(plan, &dir, port, allowed) {
            Ok(id) => respond(StatusCode::ACCEPTED, &json!({"id": id})),
            Err(StartError::Busy) => fail(
                StatusCode::CONFLICT,
                &format!(
                    "{} app sessions are going; stop one first",
                    app::MAX_RUNNING
                ),
            ),
            Err(StartError::Refused(text)) => fail(StatusCode::BAD_REQUEST, text),
            Err(StartError::Internal) => fail(StatusCode::INTERNAL_SERVER_ERROR, INTERNAL),
        }
    })
    .await
}

/// `POST /api/app-sessions/{id}/message` `{text, confirmed: true}`: one more user message (sent as typed).
async fn message(State(api): State<Api>, Path(id): Path<String>, req: Request) -> Response {
    let body = match write_body(req, &["text"]).await {
        Ok(body) => body,
        Err(answer) => return answer,
    };
    let text = match text_field(&body, "text", MAX_PROMPT) {
        Ok(text) => text,
        Err(answer) => return answer,
    };
    blocking(move || {
        let session = match found(&api, &id) {
            Ok(session) => session,
            Err(answer) => return answer,
        };
        match session.message(&text) {
            Ok(()) => respond(StatusCode::OK, &view(&session)),
            Err(()) => fail(StatusCode::CONFLICT, ENDED),
        }
    })
    .await
}

/// `POST /api/app-sessions/{id}/interrupt` `{confirmed: true}`: the stream-json interrupt (the process stays alive).
async fn interrupt(State(api): State<Api>, Path(id): Path<String>, req: Request) -> Response {
    if let Err(answer) = write_body(req, &[]).await {
        return answer;
    }
    blocking(move || {
        let session = match found(&api, &id) {
            Ok(session) => session,
            Err(answer) => return answer,
        };
        match session.interrupt() {
            Ok(()) => respond(StatusCode::OK, &view(&session)),
            Err(()) => fail(StatusCode::CONFLICT, ENDED),
        }
    })
    .await
}

/// `POST /api/app-sessions/{id}/stop` `{confirmed: true}`: SIGTERM to the process group, SIGKILL after 5 s. A session
/// that has ended is 200 and no change.
async fn stop(State(api): State<Api>, Path(id): Path<String>, req: Request) -> Response {
    if let Err(answer) = write_body(req, &[]).await {
        return answer;
    }
    blocking(move || {
        let session = match found(&api, &id) {
            Ok(session) => session,
            Err(answer) => return answer,
        };
        if !session.ended() {
            session.request_stop("stopped");
        }
        respond(StatusCode::OK, &view(&session))
    })
    .await
}

/// `POST /api/app-sessions/{id}/close` `{confirmed: true}`: the panel is closed; an ended session's text is dropped.
async fn close(State(api): State<Api>, Path(id): Path<String>, req: Request) -> Response {
    if let Err(answer) = write_body(req, &[]).await {
        return answer;
    }
    blocking(move || match api.agent.sessions.close(&id) {
        Ok(()) => respond(StatusCode::OK, &json!({"closed": true})),
        Err(true) => fail(StatusCode::CONFLICT, "stop the session first"),
        Err(false) => fail(StatusCode::NOT_FOUND, NO_SESSION),
    })
    .await
}

/// `POST /api/app-sessions/{id}/answer` `{prompt_id, behavior: "allow"|"deny", updatedInput?, message?, confirmed: true}`.
/// For a question, `updatedInput.answers` is the answer per question text (free text accepted); no other part of
/// `updatedInput` reaches the child.
async fn answer(State(api): State<Api>, Path(id): Path<String>, req: Request) -> Response {
    let body = match write_body(req, &["prompt_id", "behavior", "updatedInput", "message"]).await {
        Ok(body) => body,
        Err(answer) => return answer,
    };
    let prompt_id = match body.get("prompt_id") {
        Some(Value::String(text)) if text.len() <= 64 => text.clone(),
        _ => return fail(StatusCode::BAD_REQUEST, "prompt_id must be text"),
    };
    let allow = match body.get("behavior").and_then(Value::as_str) {
        Some("allow") => true,
        Some("deny") => false,
        _ => {
            return fail(
                StatusCode::BAD_REQUEST,
                "behavior must be \"allow\" or \"deny\"",
            );
        }
    };
    let message = match body.get("message") {
        None | Some(Value::Null) => None,
        Some(Value::String(text)) if text.len() <= MAX_NOTE => Some(text.clone()),
        Some(_) => return fail(StatusCode::BAD_REQUEST, "message must be text"),
    };
    let answers = match body.get("updatedInput") {
        None | Some(Value::Null) => None,
        Some(Value::Object(input)) => match input.get("answers") {
            None => None,
            Some(Value::Object(answers)) => Some(answers.clone()),
            Some(_) => return fail(StatusCode::BAD_REQUEST, "answers must be an object"),
        },
        Some(_) => return fail(StatusCode::BAD_REQUEST, "updatedInput must be an object"),
    };
    let answer = Answer {
        allow,
        answers,
        message,
    };
    blocking(move || {
        let session = match found(&api, &id) {
            Ok(session) => session,
            Err(answer) => return answer,
        };
        match session.answer(&prompt_id, &answer) {
            Ok(()) => respond(StatusCode::OK, &view(&session)),
            Err(AnswerError::Unknown) => fail(StatusCode::NOT_FOUND, "no such prompt"),
            Err(AnswerError::Settled) => fail(
                StatusCode::CONFLICT,
                "that prompt was answered or closed already",
            ),
            Err(AnswerError::Bad(text)) => fail(StatusCode::BAD_REQUEST, text),
            Err(AnswerError::Incomplete) => fail(StatusCode::CONFLICT, INCOMPLETE),
        }
    })
    .await
}

// ---- the permission tool: a minimal HTTP MCP endpoint -----------------------------------------------------------------

fn plain(status: StatusCode, text: &'static str) -> Response {
    let mut response = Response::new(Body::from(text));
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, "text/plain".parse().expect("a fixed value"));
    response
}

fn rpc_reply(id: &Value, result: Value) -> Response {
    respond(
        StatusCode::OK,
        &json!({"jsonrpc": "2.0", "id": id, "result": result}),
    )
}

fn rpc_error(id: &Value, code: i64, text: &str) -> Response {
    respond(
        StatusCode::OK,
        &json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": text}}),
    )
}

/// `GET` and `DELETE`: no sessions and no server-initiated stream (as `mcp_http.rs`).
async fn mcp_other() -> Response {
    let mut response = plain(StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed");
    response
        .headers_mut()
        .insert(ALLOW, "POST".parse().expect("a fixed value"));
    response
}

/// `POST /app-session-mcp/{id}`: one JSON-RPC message. The bearer secret is the session's own; an unknown session and a
/// wrong secret are the same 401.
async fn mcp_post(State(api): State<Api>, Path(id): Path<String>, req: Request) -> Response {
    let (parts, body) = req.into_parts();
    let bearer = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let session = api
        .agent
        .sessions
        .get(&id)
        .filter(|s| bearer.is_some_and(|b| s.secret_ok(b)));
    let Some(session) = session else {
        return plain(StatusCode::UNAUTHORIZED, "Unauthorized");
    };
    let json_type = parts
        .headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|t| {
            t.trim_start()
                .to_ascii_lowercase()
                .starts_with("application/json")
        });
    if !json_type {
        return plain(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Content-Type must be application/json",
        );
    }
    let Ok(bytes) = to_bytes(body, MAX_MCP_BODY).await else {
        return plain(StatusCode::PAYLOAD_TOO_LARGE, "Request body too large");
    };
    let Some(Value::Object(message)) = parse(&bytes) else {
        return rpc_error(&Value::Null, -32700, "Parse error");
    };
    let Some(request_id) = message.get("id").cloned() else {
        // a notification: no answer body. `notifications/cancelled` is Claude Code giving up a `tools/call`: its prompt
        // (when that call was the last one waiting for it) is settled with a deny (FR-111); `initialized` changes nothing
        if message.get("method").and_then(Value::as_str) == Some("notifications/cancelled")
            && let Some(call) = message.get("params").and_then(|p| p.get("requestId"))
        {
            session.cancel_call(call);
        }
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::ACCEPTED;
        return response;
    };
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    match message.get("method").and_then(Value::as_str) {
        Some("initialize") => rpc_reply(
            &request_id,
            json!({
                "protocolVersion": params["protocolVersion"].as_str().unwrap_or("2025-06-18"),
                "capabilities": {"tools": {}},
                "serverInfo": {"name": app::SERVER, "version": env!("CARGO_PKG_VERSION")},
            }),
        ),
        Some("ping") => rpc_reply(&request_id, json!({})),
        Some("tools/list") => rpc_reply(
            &request_id,
            json!({"tools": [{
                "name": TOOL,
                "description": "Asks the owner in the app to allow a tool call, answer a question or approve a plan.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "tool_name": {"type": "string"},
                        "input": {"type": "object"},
                        "tool_use_id": {"type": "string"},
                    },
                    "required": ["tool_name", "input"],
                },
            }]}),
        ),
        Some("tools/call") if params["name"] == TOOL => {
            let meta = params["_meta"]["claudecode/toolUseId"]
                .as_str()
                .map(str::to_string);
            let text =
                prompt_result(&session, &params["arguments"], meta.as_deref(), &request_id).await;
            rpc_reply(
                &request_id,
                json!({"content": [{"type": "text", "text": text}]}),
            )
        }
        Some("tools/call") => rpc_error(&request_id, -32602, "Unknown tool"),
        _ => rpc_error(&request_id, -32601, "Method not found"),
    }
}

/// Opens (or re-joins) the prompt and waits for its result text: the owner's answer, or a deny when the call was
/// cancelled, the owner's interrupt took effect or the session ended. Nothing answers it by itself.
async fn prompt_result(
    session: &Session,
    arguments: &Value,
    meta_id: Option<&str>,
    call: &Value,
) -> String {
    let Some(mut rx) = session.open_prompt(arguments, meta_id, call) else {
        return json!({"behavior": "deny", "message": "The session is not taking prompts."})
            .to_string();
    };
    match rx.wait_for(Option::is_some).await {
        Ok(result) => result.clone().unwrap_or_default(),
        Err(_) => json!({"behavior": "deny", "message": "The session ended."}).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_routes::Agent;
    use crate::agent_run::tests::{Fake, runs, serial};
    use crate::app_session::tests::CHAT;
    use crate::server::router;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use http_body_util::BodyExt;
    use rusqlite::Connection;
    use std::cell::RefCell;
    use std::future::Future;
    use std::time::{Duration, Instant};
    use tower::ServiceExt;

    const TOKEN: &str = "test-token";
    const PORT: u16 = 4242;
    const JSON: Option<&str> = Some("application/json");
    const KEEP: &str = "sleep 60 & wait";

    fn block<F: Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    }

    struct World {
        fake: Fake,
        _conn: Connection,
        api: Api,
        app: Router,
        repo: PathBuf,
        seen: RefCell<String>,
    }

    fn world(tag: &str, tail: &str) -> World {
        let fake = Fake::new(&format!("t103r-{tag}"), tail);
        let conn = fake.store();
        let repo = fake.root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        conn.execute_batch(&format!(
            "INSERT INTO workspaces (id, name) VALUES (50, 'acme'); \
             INSERT INTO workspace_repos VALUES (50, '{}');",
            repo.display()
        ))
        .unwrap();
        let path = fake.dir().join("store.sqlite");
        let agent = Agent::new(
            fake.env(&[
                ("ANTHROPIC_API_KEY", "sk-ant-T103-planted"),
                ("CLAUDECODE", "1"),
                ("CLAUDE_CODE_ENTRYPOINT", "cli"),
                ("CLAUDE_CONFIG_DIR", "/parent/.claude"),
            ]),
            PathBuf::from("/opt/t103/easy-personal-rs"),
            Duration::from_secs(30),
        );
        let api = Api::new(move || Ok(path.clone()), || 0).with_agent(agent);
        api.agent.runner(&fake.dir()).turn(true).unwrap();
        World {
            app: router(TOKEN.into(), PORT, api.clone()),
            fake,
            _conn: conn,
            api,
            repo,
            seen: RefCell::default(),
        }
    }

    impl Drop for World {
        fn drop(&mut self) {
            self.api.agent.shutdown();
        }
    }

    fn until(what: &str, mut ok: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(20);
        while !ok() {
            assert!(Instant::now() < deadline, "never happened: {what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    impl World {
        async fn send(&self, request: HttpRequest<Body>) -> (StatusCode, Value) {
            let response = self.app.clone().oneshot(request).await.unwrap();
            let status = response.status();
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            self.seen
                .borrow_mut()
                .push_str(&String::from_utf8_lossy(&bytes));
            (
                status,
                serde_json::from_slice(&bytes).unwrap_or(Value::Null),
            )
        }

        async fn call(
            &self,
            method: &str,
            path: &str,
            ct: Option<&str>,
            body: &str,
        ) -> (StatusCode, Value) {
            let mut request = HttpRequest::builder()
                .method(method)
                .uri(path)
                .header("host", format!("127.0.0.1:{PORT}"))
                .header("x-dashboard-token", TOKEN);
            if let Some(ct) = ct {
                request = request.header("content-type", ct);
            }
            self.send(request.body(Body::from(body.to_string())).unwrap())
                .await
        }

        async fn post(&self, path: &str, body: &Value) -> (StatusCode, Value) {
            self.call("POST", path, JSON, &body.to_string()).await
        }

        fn form(&self, extra: Value) -> Value {
            let mut form = json!({
                "repo": self.repo.display().to_string(), "workspace": "acme", "model": "default",
                "mode": "default", "worktree": false, "prompt": "hello", "confirmed": true,
            });
            for (key, value) in extra.as_object().unwrap() {
                if value.is_null() {
                    form.as_object_mut().unwrap().remove(key);
                } else {
                    form[key] = value.clone();
                }
            }
            form
        }

        async fn begin(&self, extra: Value) -> String {
            let (status, body) = self.post("/api/app-sessions", &self.form(extra)).await;
            assert_eq!(status, 202, "{body}");
            body["id"].as_str().unwrap().to_string()
        }

        async fn session(&self, id: &str) -> Value {
            let (status, body) = self
                .call("GET", &format!("/api/app-sessions/{id}"), None, "")
                .await;
            assert_eq!(status, 200, "{body}");
            body
        }

        async fn until_state(&self, id: &str, state: &str) -> Value {
            let deadline = Instant::now() + Duration::from_secs(20);
            loop {
                let body = self.session(id).await;
                if body["state"] == state {
                    return body;
                }
                assert!(Instant::now() < deadline, "never {state}: {body}");
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }

        /// The URL path and the bearer secret the child would read from its temp config.
        fn config(&self, id: &str) -> String {
            let path = self.fake.dir().join(format!("app-sessions/{id}.mcp.json"));
            let body: Value =
                serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            let server = &body["mcpServers"][app::SERVER];
            assert_eq!(
                server["url"],
                format!("http://127.0.0.1:{PORT}/app-session-mcp/{id}")
            );
            server["headers"]["Authorization"]
                .as_str()
                .unwrap()
                .to_string()
        }

        async fn mcp(&self, id: &str, bearer: Option<&str>, body: &Value) -> (StatusCode, Value) {
            let mut request = HttpRequest::builder()
                .method("POST")
                .uri(format!("/app-session-mcp/{id}"))
                .header("host", format!("127.0.0.1:{PORT}"))
                .header("content-type", "application/json");
            if let Some(bearer) = bearer {
                request = request.header("authorization", bearer);
            }
            self.send(request.body(Body::from(body.to_string())).unwrap())
                .await
        }

        async fn tool_call(&self, id: &str, bearer: &str, args: Value) -> Value {
            let body = json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call",
                              "params": {"name": TOOL, "arguments": args}});
            let (status, answer) = self.mcp(id, Some(bearer), &body).await;
            assert_eq!(status, 200, "{answer}");
            serde_json::from_str(answer["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
        }
    }

    // ---- AC 1: start and the fixed lists ----

    #[test]
    fn start_answers_202_with_an_id_and_runs_the_fake_with_the_chosen_argv() {
        let _serial = serial();
        block(async {
            let w = world("start", CHAT);
            let id = w.begin(json!({"model": "opus", "mode": "plan"})).await;
            let view = w.until_state(&id, "idle").await;
            assert_eq!(
                (view["model"].as_str(), view["mode"].as_str()),
                (Some("opus"), Some("plan"))
            );
            assert_eq!(view["folder"], "repo");
            assert_eq!(view["worktree"], false);
            assert_eq!(view["branch"], Value::Null);
            let argv = w.fake.argv();
            assert!(argv.windows(2).any(|p| p == ["--model", "opus"]));
            assert!(argv.windows(2).any(|p| p == ["--permission-mode", "plan"]));
            assert!(argv.contains(&"--verbose".to_string()));
            let (status, list) = w.call("GET", "/api/app-sessions", None, "").await;
            assert_eq!(status, 200);
            assert_eq!(list["max"], 4);
            assert_eq!(list["sessions"][0]["id"], id);
            let mode = std::fs::metadata(w.fake.dir().join("app-sessions")).unwrap();
            assert_eq!(
                std::os::unix::fs::PermissionsExt::mode(&mode.permissions()) & 0o777,
                0o700
            );
        });
    }

    #[test]
    fn a_shell_text_an_option_an_unknown_model_or_mode_and_a_path_outside_the_store_are_400_and_nothing_starts()
     {
        let _serial = serial();
        block(async {
            let w = world("bad", KEEP);
            let bad_texts = [
                "a; touch /tmp/x",
                "--dangerously-skip-permissions",
                "$(id)",
                "`id`",
                "../repo",
                "",
            ];
            for text in bad_texts {
                for field in ["repo", "workspace", "model", "mode", "account"] {
                    let (status, body) = w
                        .post("/api/app-sessions", &w.form(json!({ field: text })))
                        .await;
                    assert_eq!(status, 400, "{field} {text:?}: {body}");
                }
            }
            for extra in [
                json!({"model": "gpt-9"}),
                json!({"mode": "yolo"}),
                json!({"model": 3}),
                json!({"mode": ["plan"]}),
                json!({"repo": "/etc"}),
                json!({"repo": "/"}),
                json!({"workspace": "nope"}),
                json!({"worktree": "yes"}),
                json!({"prompt": ""}),
                json!({"prompt": "   "}),
                json!({"prompt": 5}),
                json!({"prompt": null}),
                json!({"repo": null}),
                json!({"model": null}),
                json!({"extra": 1}),
                json!({"account": "4"}),
                json!({"account": "999"}),
            ] {
                let (status, body) = w.post("/api/app-sessions", &w.form(extra.clone())).await;
                assert_eq!(status, 400, "{extra}: {body}");
            }
            assert!(!w.fake.root.join("log/pid").exists(), "nothing was started");
            assert!(w.api.agent.sessions.all().is_empty());
            let (status, _) = w.call("POST", "/api/app-sessions", JSON, "[]").await;
            assert_eq!(status, 400);
            let (status, _) = w
                .call("POST", "/api/app-sessions", Some("text/plain"), "{}")
                .await;
            assert_eq!(status, 415);
            // a stored repo whose folder is gone
            std::fs::remove_dir_all(&w.repo).unwrap();
            let (status, _) = w.post("/api/app-sessions", &w.form(json!({}))).await;
            assert_eq!(status, 400);
            assert!(!w.fake.root.join("log/pid").exists());
        });
    }

    #[test]
    fn the_setting_off_is_403_and_the_chosen_account_or_a_named_one_sets_the_config_folder() {
        let _serial = serial();
        block(async {
            let w = world("account", CHAT);
            w.api.agent.runner(&w.fake.dir()).turn(false).unwrap();
            let (status, _) = w.post("/api/app-sessions", &w.form(json!({}))).await;
            assert_eq!(status, 403);
            assert!(!w.fake.root.join("log/pid").exists());
            w.api.agent.runner(&w.fake.dir()).choose("2").unwrap();
            let id = w.begin(json!({})).await;
            let view = w.until_state(&id, "idle").await;
            assert_eq!(view["account"], json!({"id": "2", "label": "work"}));
            let env = w.fake.child_env();
            assert!(
                env.iter()
                    .any(|(k, v)| k == "CLAUDE_CONFIG_DIR" && v == "/acc/.claude-work")
            );
            // the account asked for wins over the agent run account
            let _ = std::fs::remove_file(w.fake.root.join("log/env"));
            let other = w.begin(json!({"account": "home"})).await;
            let view = w.until_state(&other, "idle").await;
            assert_eq!(view["account"], json!({"id": "1", "label": "home"}));
            until("env", || w.fake.root.join("log/env").exists());
            assert!(
                !w.fake
                    .child_env()
                    .iter()
                    .any(|(k, _)| k == "CLAUDE_CONFIG_DIR"),
                "the default account has none"
            );
        });
    }

    // ---- AC 2: the child's environment, through the route ----

    #[test]
    fn the_child_gets_no_billing_variable_no_parent_marker_and_the_prompt_tool() {
        let _serial = serial();
        block(async {
            let w = world("env", CHAT);
            let id = w.begin(json!({"mode": "bypassPermissions"})).await;
            w.until_state(&id, "idle").await;
            let env = w.fake.child_env();
            for gone in [
                "ANTHROPIC_API_KEY",
                "CLAUDECODE",
                "CLAUDE_CODE_ENTRYPOINT",
                "CLAUDE_CONFIG_DIR",
            ] {
                assert!(!env.iter().any(|(k, _)| k == gone), "{gone}");
            }
            assert!(
                env.iter()
                    .any(|(k, v)| k == "MCP_TOOL_TIMEOUT" && v == "3600000")
            );
            let argv = w.fake.argv();
            assert!(argv.windows(2).any(|p| p
                == [
                    "--permission-prompt-tool",
                    "mcp__easy-personal-app__permission_prompt"
                ]));
            assert!(argv.contains(&"--dangerously-skip-permissions".to_string()));
            for banned in [
                "--allowedTools",
                "--disallowedTools",
                "--tools",
                "--strict-mcp-config",
            ] {
                assert!(!argv.contains(&banned.to_string()), "{banned}");
            }
        });
    }

    // ---- AC 3: the worktree ----

    #[test]
    fn a_worktree_session_runs_on_its_own_branch_and_lists_the_changed_file_names() {
        let _serial = serial();
        block(async {
            let w = world("worktree", CHAT);
            for args in [
                vec!["init", "-q"],
                vec![
                    "-c",
                    "user.name=t",
                    "-c",
                    "user.email=t@t",
                    "commit",
                    "-q",
                    "--allow-empty",
                    "-m",
                    "i",
                ],
            ] {
                assert!(
                    std::process::Command::new("git")
                        .arg("-C")
                        .arg(&w.repo)
                        .args(args)
                        .env("GIT_CONFIG_GLOBAL", "/dev/null")
                        .status()
                        .unwrap()
                        .success()
                );
            }
            let id = w.begin(json!({"worktree": true})).await;
            let view = w.until_state(&id, "idle").await;
            let branch = view["branch"].as_str().unwrap();
            let rest = branch.strip_prefix("easy/").unwrap();
            let (stamp, hex) = rest.rsplit_once('-').unwrap();
            assert!(
                stamp.len() == 13
                    && stamp.as_bytes()[8] == b'-'
                    && stamp.bytes().all(|b| b.is_ascii_digit() || b == b'-')
            );
            assert!(hex.len() == 4 && hex.bytes().all(|b| b.is_ascii_hexdigit()));
            let folder = view["folder"].as_str().unwrap();
            assert_eq!(folder, format!("repo-{rest}"));
            let cwd = std::fs::canonicalize(w.fake.dir().join("worktrees").join(folder)).unwrap();
            assert_eq!(w.fake.log("cwd").trim(), cwd.display().to_string());
            assert_eq!(view["changed_files"], json!([]));
            std::fs::write(cwd.join("edited.txt"), "x").unwrap();
            assert_eq!(w.session(&id).await["changed_files"], json!(["edited.txt"]));
            // not a git repository: a fixed 400 and no session
            let plain = w.fake.root.join("plain");
            std::fs::create_dir_all(&plain).unwrap();
            w._conn
                .execute(
                    "INSERT INTO workspace_repos VALUES (50, ?1)",
                    [plain.display().to_string()],
                )
                .unwrap();
            let (status, body) = w
                .post(
                    "/api/app-sessions",
                    &w.form(json!({"repo": plain.display().to_string(), "worktree": true})),
                )
                .await;
            assert_eq!(
                (status, body["error"].as_str()),
                (
                    StatusCode::BAD_REQUEST,
                    Some("the folder is not a git repository")
                )
            );
            assert_eq!(w.api.agent.sessions.all().len(), 1);
        });
    }

    // ---- AC 4: talk, interrupt, stop, the fifth ----

    #[test]
    fn message_interrupt_and_stop_reach_the_child_and_an_ended_session_is_409() {
        let _serial = serial();
        block(async {
            let w = world("talk", CHAT);
            let id = w.begin(json!({})).await;
            w.until_state(&id, "idle").await;
            let (status, view) = w
                .post(
                    &format!("/api/app-sessions/{id}/message"),
                    &json!({"text": "/ping now", "confirmed": true}),
                )
                .await;
            assert_eq!(status, 200, "{view}");
            until("the message", || w.fake.log("stdin").lines().count() == 2);
            let sent: Value =
                serde_json::from_str(w.fake.log("stdin").lines().nth(1).unwrap()).unwrap();
            assert_eq!(sent["message"]["content"], "/ping now");
            w.until_state(&id, "idle").await;
            let (status, _) = w
                .post(
                    &format!("/api/app-sessions/{id}/interrupt"),
                    &json!({"confirmed": true}),
                )
                .await;
            assert_eq!(status, 200);
            until("the interrupt", || {
                w.fake.log("stdin").contains("\"subtype\":\"interrupt\"")
            });
            assert!(
                runs(&w.fake.log("pid")),
                "the process is alive after an interrupt"
            );
            let (status, view) = w
                .post(
                    &format!("/api/app-sessions/{id}/stop"),
                    &json!({"confirmed": true}),
                )
                .await;
            assert_eq!(status, 200);
            assert!(matches!(view["state"].as_str(), Some("stopping" | "ended")));
            let ended = w.until_state(&id, "ended").await;
            assert_eq!(ended["reason"], "stopped");
            for action in ["message", "interrupt"] {
                let (status, _) = w
                    .post(
                        &format!("/api/app-sessions/{id}/{action}"),
                        &json!({"text": "x", "confirmed": true}),
                    )
                    .await;
                assert!(status == 409 || status == 400, "{action}: {status}");
            }
            let (status, _) = w
                .post(
                    &format!("/api/app-sessions/{id}/message"),
                    &json!({"text": "x", "confirmed": true}),
                )
                .await;
            assert_eq!(status, 409);
            let (status, _) = w
                .post(
                    &format!("/api/app-sessions/{id}/interrupt"),
                    &json!({"confirmed": true}),
                )
                .await;
            assert_eq!(status, 409);
            let (status, _) = w
                .post(
                    &format!("/api/app-sessions/{id}/stop"),
                    &json!({"confirmed": true}),
                )
                .await;
            assert_eq!(status, 200, "stop of an ended session changes nothing");
            let (status, _) = w
                .post("/api/app-sessions/nope/stop", &json!({"confirmed": true}))
                .await;
            assert_eq!(status, 404);
            let (status, _) = w.call("GET", "/api/app-sessions/nope", None, "").await;
            assert_eq!(status, 404);
        });
    }

    #[test]
    fn a_fifth_concurrent_session_is_409_with_a_fixed_message() {
        let _serial = serial();
        block(async {
            let w = world("five", KEEP);
            let mut ids = Vec::new();
            for _ in 0..4 {
                ids.push(w.begin(json!({})).await);
            }
            let (status, body) = w.post("/api/app-sessions", &w.form(json!({}))).await;
            assert_eq!(status, 409);
            assert_eq!(body["error"], "4 app sessions are going; stop one first");
            let (_, _) = w
                .post(
                    &format!("/api/app-sessions/{}/stop", ids[0]),
                    &json!({"confirmed": true}),
                )
                .await;
            w.until_state(&ids[0], "ended").await;
            let _ = w.begin(json!({})).await;
        });
    }

    // ---- AC 5: the conversation ----

    #[test]
    fn the_session_shows_text_labels_and_outcomes_and_never_a_tool_input_or_output() {
        let _serial = serial();
        block(async {
            let w = world("view", CHAT);
            let id = w.begin(json!({"prompt": "OWNER-PROMPT"})).await;
            let view = w.until_state(&id, "idle").await;
            let mut keys: Vec<&str> = view
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            keys.sort_unstable();
            assert_eq!(
                keys,
                [
                    "account",
                    "branch",
                    "changed_files",
                    "folder",
                    "id",
                    "items",
                    "mode",
                    "model",
                    "reason",
                    "session_id",
                    "state",
                    "worktree"
                ]
            );
            assert_eq!(
                view["items"][0],
                json!({"kind": "user", "text": "OWNER-PROMPT"})
            );
            assert_eq!(view["items"][1]["text"], "ANSWER-TEXT");
            assert_eq!(
                view["items"][2],
                json!({"kind": "tool", "name": "Bash", "label": "ls", "outcome": "ok"})
            );
            assert_eq!(view["account"], Value::Null);
            let all = w.seen.borrow().clone();
            for hidden in [
                "TOOL-SECRET",
                "TOOL-OUTPUT",
                "ls -la",
                "sk-ant-T103-planted",
                "/opt/t103",
            ] {
                assert!(!all.contains(hidden), "{hidden}");
            }
        });
    }

    // ---- AC 6: permission prompts over the HTTP route ----

    fn bash_call(use_id: &str) -> Value {
        json!({"tool_name": "Bash", "input": {"command": "touch x", "description": "d"}, "tool_use_id": use_id})
    }

    #[test]
    fn a_prompt_call_waits_two_seconds_with_no_answer_then_the_retry_gets_the_one_answer() {
        let _serial = serial();
        block(async {
            let w = world("prompt", KEEP);
            let id = w.begin(json!({})).await;
            let bearer = w.config(&id);
            // the handshake Claude Code does
            let (status, init) = w.mcp(&id, Some(&bearer), &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}})).await;
            assert_eq!(
                (status, init["result"]["protocolVersion"].as_str()),
                (StatusCode::OK, Some("2025-06-18"))
            );
            let (status, _) = w
                .mcp(
                    &id,
                    Some(&bearer),
                    &json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
                )
                .await;
            assert_eq!(status, 202);
            let (_, tools) = w
                .mcp(
                    &id,
                    Some(&bearer),
                    &json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
                )
                .await;
            assert_eq!(tools["result"]["tools"][0]["name"], TOOL);
            let (_, unknown) = w
                .mcp(
                    &id,
                    Some(&bearer),
                    &json!({"jsonrpc":"2.0","id":3,"method":"server/discover"}),
                )
                .await;
            assert_eq!(unknown["error"]["code"], -32601);

            // no auto-allow: after 2 s the call is still waiting
            let first = w.tool_call(&id, &bearer, bash_call("toolu_a"));
            let waited = tokio::time::timeout(Duration::from_secs(2), first).await;
            assert!(waited.is_err(), "the call must wait for the owner");
            let view = w.until_state(&id, "waiting").await;
            let card = view["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|i| i["kind"] == "prompt")
                .unwrap()
                .clone();
            assert_eq!(
                (
                    card["status"].as_str(),
                    card["input"].as_str(),
                    card["complete"].as_bool()
                ),
                (Some("pending"), Some("touch x"), Some(true))
            );
            let prompt_id = card["prompt_id"].as_str().unwrap().to_string();

            // Claude Code retried the cut call: the same prompt, one answer for both
            let retried = w.tool_call(&id, &bearer, bash_call("toolu_a"));
            let answering = async {
                tokio::time::sleep(Duration::from_millis(200)).await;
                w.post(
                    &format!("/api/app-sessions/{id}/answer"),
                    &json!({"prompt_id": prompt_id, "behavior": "allow", "confirmed": true}),
                )
                .await
            };
            let (result, (status, after)) = tokio::join!(retried, answering);
            assert_eq!(status, 200, "{after}");
            assert_eq!(
                result,
                json!({"behavior": "allow", "updatedInput": {"command": "touch x", "description": "d"}})
            );
            assert_eq!(
                w.session(&id).await["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|i| i["kind"] == "prompt")
                    .count(),
                1
            );
            // a second answer is 409, an unknown prompt 404, a bad shape 400
            let again = json!({"prompt_id": prompt_id, "behavior": "deny", "confirmed": true});
            assert_eq!(
                w.post(&format!("/api/app-sessions/{id}/answer"), &again)
                    .await
                    .0,
                409
            );
            let unknown = json!({"prompt_id": "ffffffff", "behavior": "allow", "confirmed": true});
            assert_eq!(
                w.post(&format!("/api/app-sessions/{id}/answer"), &unknown)
                    .await
                    .0,
                404
            );
            let bad = json!({"prompt_id": prompt_id, "behavior": "maybe", "confirmed": true});
            assert_eq!(
                w.post(&format!("/api/app-sessions/{id}/answer"), &bad)
                    .await
                    .0,
                400
            );
            assert_eq!(
                w.post(
                    &format!("/api/app-sessions/{id}/answer"),
                    &json!({"prompt_id": prompt_id, "behavior": "deny"})
                )
                .await
                .0,
                400
            );
        });
    }

    #[test]
    fn a_deny_note_and_a_question_answer_go_back_as_the_tools_documented_result() {
        let _serial = serial();
        block(async {
            let w = world("answers", KEEP);
            let id = w.begin(json!({})).await;
            let bearer = w.config(&id);
            let deny = w.tool_call(&id, &bearer, bash_call("toolu_d"));
            let ask_args = json!({"tool_name": "AskUserQuestion", "tool_use_id": "toolu_q", "input": {"questions": [
                {"question": "Which colour?", "header": "Colour", "multiSelect": false, "options": [{"label": "Red", "description": "r"}]}]}});
            let ask = w.tool_call(&id, &bearer, ask_args);
            let owner = async {
                let mut ids = Vec::new();
                while ids.len() < 2 {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    ids = w.session(&id).await["items"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|i| i["kind"] == "prompt")
                        .map(|i| {
                            (
                                i["tool"].as_str().unwrap().to_string(),
                                i["prompt_id"].as_str().unwrap().to_string(),
                            )
                        })
                        .collect();
                }
                for (tool, prompt_id) in ids {
                    let body = if tool == "Bash" {
                        json!({"prompt_id": prompt_id, "behavior": "deny", "message": "no thanks", "confirmed": true})
                    } else {
                        json!({"prompt_id": prompt_id, "behavior": "allow", "updatedInput": {"answers": {"Which colour?": "Green"}}, "confirmed": true})
                    };
                    assert_eq!(
                        w.post(&format!("/api/app-sessions/{id}/answer"), &body)
                            .await
                            .0,
                        200
                    );
                }
            };
            let (deny, ask, ()) = tokio::join!(deny, ask, owner);
            assert_eq!(deny, json!({"behavior": "deny", "message": "no thanks"}));
            assert_eq!(
                ask["updatedInput"]["answers"],
                json!({"Which colour?": "Green"})
            );
        });
    }

    #[test]
    fn a_tool_card_shows_the_whole_input_and_an_allow_of_a_cut_input_is_409_while_a_deny_is_200() {
        let _serial = serial();
        block(async {
            let w = world("overcap", KEEP);
            let id = w.begin(json!({})).await;
            let bearer = w.config(&id);
            let long = format!("echo {}", "x".repeat(16_000));
            let multi = w.tool_call(
                &id,
                &bearer,
                json!({"tool_name": "Bash", "tool_use_id": "toolu_m",
                       "input": {"command": "git status\nrm -rf x\u{202e}", "description": "d"}}),
            );
            let over = w.tool_call(
                &id,
                &bearer,
                json!({"tool_name": "Bash", "tool_use_id": "toolu_o", "input": {"command": long}}),
            );
            let owner = async {
                let mut cards = Vec::new();
                while cards.len() < 2 {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    cards = w.session(&id).await["items"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|i| i["kind"] == "prompt")
                        .cloned()
                        .collect();
                }
                let find = |text: &str| {
                    cards
                        .iter()
                        .find(|c| c["input"].as_str().unwrap().starts_with(text))
                        .unwrap()
                        .clone()
                };
                let (multi, over) = (find("git status"), find("echo"));
                assert_eq!(multi["input"], "git status\nrm -rf x\\u{202e}");
                assert_eq!(multi["complete"], true);
                assert_eq!(multi["tool"], "Bash");
                assert!(multi.get("summary").is_none());
                assert_eq!(over["complete"], false);
                let path = format!("/api/app-sessions/{id}/answer");
                let allow =
                    json!({"prompt_id": over["prompt_id"], "behavior": "allow", "confirmed": true});
                let (status, body) = w.post(&path, &allow).await;
                assert_eq!(status, 409, "{body}");
                assert!(
                    body["error"]
                        .as_str()
                        .unwrap()
                        .contains("too long to show whole")
                );
                assert_eq!(w.session(&id).await["state"], "waiting", "the card stays");
                let deny =
                    json!({"prompt_id": over["prompt_id"], "behavior": "deny", "confirmed": true});
                assert_eq!(w.post(&path, &deny).await.0, 200);
                let allow = json!({"prompt_id": multi["prompt_id"], "behavior": "allow", "confirmed": true});
                assert_eq!(w.post(&path, &allow).await.0, 200);
            };
            let (multi, over, ()) = tokio::join!(multi, over, owner);
            assert_eq!(multi["behavior"], "allow");
            assert_eq!(over["behavior"], "deny");
        });
    }

    #[test]
    fn a_cancelled_notification_settles_the_prompt_of_that_call_and_no_other() {
        let _serial = serial();
        block(async {
            let w = world("cancel", KEEP);
            let id = w.begin(json!({})).await;
            let bearer = w.config(&id);
            let body = json!({"jsonrpc": "2.0", "id": 41, "method": "tools/call",
                              "params": {"name": TOOL, "arguments": bash_call("toolu_c")}});
            let call = w.mcp(&id, Some(&bearer), &body);
            let cancelling = async {
                w.until_state(&id, "waiting").await;
                let cancel = |request_id: i64| {
                    json!({"jsonrpc": "2.0", "method": "notifications/cancelled",
                           "params": {"requestId": request_id, "reason": "timeout"}})
                };
                let (status, _) = w.mcp(&id, Some(&bearer), &cancel(99)).await;
                assert_eq!(status, 202);
                tokio::time::sleep(Duration::from_millis(300)).await;
                assert_eq!(
                    w.session(&id).await["state"],
                    "waiting",
                    "another call's id changes nothing"
                );
                let (status, _) = w.mcp(&id, Some(&bearer), &cancel(41)).await;
                assert_eq!(status, 202);
            };
            let ((status, answer), ()) = tokio::join!(call, cancelling);
            assert_eq!(status, 200);
            let text = answer["result"]["content"][0]["text"].as_str().unwrap();
            assert_eq!(
                serde_json::from_str::<Value>(text).unwrap()["behavior"],
                "deny"
            );
            let view = w.session(&id).await;
            let card = view["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|i| i["kind"] == "prompt")
                .unwrap();
            assert_eq!(card["status"], "cancelled");
            assert_eq!(view["state"], "working", "the session itself goes on");
        });
    }

    #[test]
    fn the_prompt_route_needs_the_sessions_own_bearer_and_keeps_the_guard_and_is_post_only() {
        let _serial = serial();
        block(async {
            let w = world("route", KEEP);
            let id = w.begin(json!({})).await;
            let bearer = w.config(&id);
            let body = json!({"jsonrpc":"2.0","id":1,"method":"ping"});
            for wrong in [None, Some("Bearer nope"), Some("nope"), Some("Bearer ")] {
                assert_eq!(w.mcp(&id, wrong, &body).await.0, 401, "{wrong:?}");
            }
            assert_eq!(
                w.mcp("deadbeef", Some(&bearer), &body).await.0,
                401,
                "an unknown session is the same 401"
            );
            assert_eq!(w.mcp(&id, Some(&bearer), &body).await.0, 200);
            // the page token is not what opens it, and it is not needed
            let mut request = HttpRequest::builder()
                .method("POST")
                .uri(format!("/app-session-mcp/{id}"))
                .header("host", format!("127.0.0.1:{PORT}"))
                .header("content-type", "application/json")
                .header("x-dashboard-token", TOKEN);
            assert_eq!(
                w.send(request.body(Body::from(body.to_string())).unwrap())
                    .await
                    .0,
                401
            );
            // the guard's Host and Origin checks stay
            request = HttpRequest::builder()
                .method("POST")
                .uri(format!("/app-session-mcp/{id}"))
                .header("host", "evil.example:80")
                .header("content-type", "application/json")
                .header("authorization", &bearer);
            assert_eq!(
                w.send(request.body(Body::from(body.to_string())).unwrap())
                    .await
                    .0,
                403
            );
            request = HttpRequest::builder()
                .method("POST")
                .uri(format!("/app-session-mcp/{id}"))
                .header("host", format!("127.0.0.1:{PORT}"))
                .header("origin", "http://evil.example")
                .header("content-type", "application/json")
                .header("authorization", &bearer);
            assert_eq!(
                w.send(request.body(Body::from(body.to_string())).unwrap())
                    .await
                    .0,
                403
            );
            for method in ["GET", "DELETE", "PUT"] {
                let request = HttpRequest::builder()
                    .method(method)
                    .uri(format!("/app-session-mcp/{id}"))
                    .header("host", format!("127.0.0.1:{PORT}"))
                    .header("authorization", &bearer);
                assert_eq!(
                    w.send(request.body(Body::empty()).unwrap()).await.0,
                    405,
                    "{method}"
                );
            }
            // another session's secret does not open this one
            let other = w.begin(json!({})).await;
            assert_eq!(w.mcp(&id, Some(&w.config(&other)), &body).await.0, 401);
            let (status, answer) = w.mcp(&id, Some(&bearer), &json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"other","arguments":{}}})).await;
            assert_eq!(
                (status, answer["error"]["code"].as_i64()),
                (StatusCode::OK, Some(-32602))
            );
        });
    }

    // ---- AC 7: the guard and `confirmed` ----

    #[test]
    fn every_route_needs_the_token_a_good_host_and_origin_and_each_write_needs_confirmed_true() {
        let _serial = serial();
        block(async {
            let w = world("guard", KEEP);
            let id = w.begin(json!({})).await;
            let routes: Vec<(&str, String, String)> = vec![
                ("GET", "/api/app-sessions".into(), String::new()),
                ("GET", format!("/api/app-sessions/{id}"), String::new()),
                (
                    "POST",
                    "/api/app-sessions".into(),
                    w.form(json!({})).to_string(),
                ),
                (
                    "POST",
                    format!("/api/app-sessions/{id}/message"),
                    r#"{"text":"x","confirmed":true}"#.into(),
                ),
                (
                    "POST",
                    format!("/api/app-sessions/{id}/interrupt"),
                    r#"{"confirmed":true}"#.into(),
                ),
                (
                    "POST",
                    format!("/api/app-sessions/{id}/stop"),
                    r#"{"confirmed":true}"#.into(),
                ),
                (
                    "POST",
                    format!("/api/app-sessions/{id}/answer"),
                    r#"{"prompt_id":"a","behavior":"allow","confirmed":true}"#.into(),
                ),
                (
                    "POST",
                    format!("/api/app-sessions/{id}/close"),
                    r#"{"confirmed":true}"#.into(),
                ),
            ];
            for (method, path, body) in &routes {
                for refusal in [
                    "no token",
                    "wrong token",
                    "wrong host",
                    "foreign origin",
                    "query token",
                ] {
                    let mut request = HttpRequest::builder()
                        .method(*method)
                        .uri(path.as_str())
                        .header("content-type", "application/json");
                    request = match refusal {
                        "wrong host" => request
                            .header("host", "evil.example:80")
                            .header("x-dashboard-token", TOKEN),
                        "no token" => request.header("host", format!("127.0.0.1:{PORT}")),
                        "wrong token" => request
                            .header("host", format!("127.0.0.1:{PORT}"))
                            .header("x-dashboard-token", "nope"),
                        "foreign origin" => request
                            .header("host", format!("127.0.0.1:{PORT}"))
                            .header("x-dashboard-token", TOKEN)
                            .header("origin", "http://evil.example"),
                        _ => request.header("host", format!("127.0.0.1:{PORT}")), // the token in the query is for /api/live only
                    };
                    let target = if refusal == "query token" {
                        format!("{path}?token={TOKEN}")
                    } else {
                        path.clone()
                    };
                    let request = request.uri(target);
                    let (status, answer) = w
                        .send(request.body(Body::from(body.clone())).unwrap())
                        .await;
                    assert_eq!(status, 403, "{method} {path} {refusal}");
                    assert_eq!(answer, json!({"error": "forbidden"}));
                }
            }
            assert_eq!(
                w.api.agent.sessions.all().len(),
                1,
                "a refused start starts nothing"
            );
            let stopped = w.session(&id).await;
            assert_ne!(stopped["state"], "stopping", "a refused stop stops nothing");
            // confirmed: missing, false, or not a boolean
            for confirmed in [json!(null), json!(false), json!("true"), json!(1)] {
                let mut start = w.form(json!({}));
                let mut write = json!({});
                if !confirmed.is_null() {
                    start["confirmed"] = confirmed.clone();
                    write["confirmed"] = confirmed.clone();
                } else {
                    start.as_object_mut().unwrap().remove("confirmed");
                }
                assert_eq!(
                    w.post("/api/app-sessions", &start).await.0,
                    400,
                    "{confirmed}"
                );
                for action in ["interrupt", "stop", "close"] {
                    assert_eq!(
                        w.post(&format!("/api/app-sessions/{id}/{action}"), &write)
                            .await
                            .0,
                        400,
                        "{action} {confirmed}"
                    );
                }
                let mut message = write.clone();
                message["text"] = json!("x");
                assert_eq!(
                    w.post(&format!("/api/app-sessions/{id}/message"), &message)
                        .await
                        .0,
                    400
                );
            }
            assert_eq!(w.api.agent.sessions.all().len(), 1);
            assert_eq!(w.session(&id).await["state"], "working");
            // an unknown field, and a wrong method
            let (status, _) = w
                .post(
                    &format!("/api/app-sessions/{id}/stop"),
                    &json!({"confirmed": true, "x": 1}),
                )
                .await;
            assert_eq!(status, 400);
            let (status, _) = w
                .call("DELETE", &format!("/api/app-sessions/{id}"), None, "")
                .await;
            assert_eq!(status, 405);
            // the token is in no answer
            assert!(!w.seen.borrow().contains(TOKEN));
        });
    }

    #[test]
    fn turning_the_setting_off_ends_a_going_session() {
        let _serial = serial();
        block(async {
            let w = world("off", KEEP);
            let id = w.begin(json!({})).await;
            w.until_state(&id, "working").await;
            w.api.agent.runner(&w.fake.dir()).turn(false).unwrap();
            let ended = w.until_state(&id, "ended").await;
            assert_eq!(ended["reason"], "account turned off");
            let (status, _) = w.post("/api/app-sessions", &w.form(json!({}))).await;
            assert_eq!(status, 403, "a new session needs the setting on");
        });
    }

    #[test]
    fn an_account_that_is_no_longer_confirmed_ends_a_going_session() {
        let _serial = serial();
        block(async {
            let w = world("gone", KEEP);
            let id = w.begin(json!({"account": "home"})).await;
            w.until_state(&id, "working").await;
            w._conn
                .execute(
                    "UPDATE agent_accounts SET status = 'ignored' WHERE id = 1",
                    [],
                )
                .unwrap();
            let ended = w.until_state(&id, "ended").await;
            assert_eq!(ended["reason"], "account turned off");
            let (status, _) = w
                .post("/api/app-sessions", &w.form(json!({"account": "home"})))
                .await;
            assert_eq!(status, 400, "the account is not a confirmed one any more");
        });
    }

    // ---- AC 9: close drops the text ----

    #[test]
    fn close_drops_an_ended_sessions_text_and_refuses_a_going_one() {
        let _serial = serial();
        block(async {
            let w = world("close", CHAT);
            let id = w.begin(json!({"prompt": "OWNER-PROMPT"})).await;
            w.until_state(&id, "idle").await;
            let held = w.api.agent.sessions.get(&id).unwrap();
            let path = format!("/api/app-sessions/{id}/close");
            assert_eq!(w.post(&path, &json!({"confirmed": true})).await.0, 409);
            w.post(
                &format!("/api/app-sessions/{id}/stop"),
                &json!({"confirmed": true}),
            )
            .await;
            w.until_state(&id, "ended").await;
            assert!(w.session(&id).await.to_string().contains("ANSWER-TEXT"));
            assert_eq!(
                w.post(&path, &json!({"confirmed": true})).await,
                (StatusCode::OK, json!({"closed": true}))
            );
            assert_eq!(
                w.call("GET", &format!("/api/app-sessions/{id}"), None, "")
                    .await
                    .0,
                404
            );
            assert_eq!(
                w.call("GET", "/api/app-sessions", None, "").await.1["sessions"],
                json!([])
            );
            let left = held.json(&[]).to_string();
            assert!(
                !left.contains("ANSWER-TEXT") && !left.contains("OWNER-PROMPT"),
                "{left}"
            );
            assert_eq!(w.post(&path, &json!({"confirmed": true})).await.0, 404);
        });
    }
}
