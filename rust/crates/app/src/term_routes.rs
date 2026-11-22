//! The terminal session routes (T133, FR-136, FR-137, DDR-0009): `GET /api/term-sessions`, `POST /api/term-sessions`
//! (start), `POST /api/term-sessions/{id}/stop`, and the WebSocket `GET /api/term/{id}`. They sit behind the guard in
//! `server.rs`. The writes take the body rules and the form of `POST /api/app-sessions` (`app_session_routes.rs`: the
//! same checks, codes and fixed texts; no prompt here), and need `confirmed: true`.
//!
//! The WebSocket: a browser cannot set a header on it, so the guard lets its upgrade through without the token header
//! (Host and Origin are still checked) and the token comes in the first frame, `{"t":"auth","token","cols","rows"}`,
//! within [`AUTH_WAIT`]; else the socket closes and no byte of the session was sent. The token is never read from the
//! URL. In: binary frames are typed bytes (FR-116's one exception), text `{"t":"resize","cols","rows"}`. Out: binary
//! PTY bytes, text `{"t":"exit","code"}`, `{"t":"pages","n"}` and `{"t":"notice","text"}`. Nothing here logs.

use crate::agent_routes::{blocking, data_dir};
use crate::agent_run::Setting;
use crate::api::{Api, INTERNAL, respond};
use crate::app_session::{self as app, StartError};
use crate::app_session_routes::{
    OFF, Target, busy, form, pick_account, still_allowed, stored_repo, write_body,
};
use crate::server::constant_time_eq;
use crate::term_session::{Out, Plan, Term};
use crate::writes::fail;
use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;

/// The first frame must come within this.
pub(crate) const AUTH_WAIT: Duration = Duration::from_secs(5);
/// The path prefix of the WebSocket; the guard's carve-out (`server.rs`) keys on it.
pub(crate) const TERM_PATH: &str = "/api/term/";
const NO_SESSION: &str = "no such session";
const NOTICE: &str = "typed input over 64 KiB a second was dropped";
/// The largest frame a page may send (the input cap drops more than 64 KiB a second anyway).
const MAX_FRAME: usize = 1 << 20;

pub(crate) fn routes(token: Arc<str>) -> Router<Api> {
    Router::new()
        .route("/api/term-sessions", get(list).post(start))
        .route("/api/term-sessions/{id}/stop", post(stop))
        .route(
            "/api/term/{id}",
            get(
                move |State(api): State<Api>, Path(id): Path<String>, ws: WebSocketUpgrade| {
                    let token = Arc::clone(&token);
                    async move {
                        ws.max_message_size(MAX_FRAME)
                            .max_frame_size(MAX_FRAME)
                            .on_upgrade(move |socket| attach(socket, api, id, token))
                    }
                },
            ),
        )
}

/// `GET /api/term-sessions` -> `{sessions, max}`: no screen byte.
async fn list(State(api): State<Api>) -> Response {
    blocking(move || {
        let sessions: Vec<Value> = api.agent.terms.all().iter().map(|t| t.json()).collect();
        respond(
            StatusCode::OK,
            &json!({"sessions": sessions, "max": app::MAX_RUNNING}),
        )
    })
    .await
}

/// `POST /api/term-sessions` `{repo, workspace, account?, model, mode, worktree?, confirmed: true}` -> 201 `{id}`.
async fn start(State(api): State<Api>, req: Request) -> Response {
    let body = match write_body(
        req,
        &["repo", "workspace", "account", "model", "mode", "worktree"],
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
    let dir = match data_dir(&api) {
        Ok(dir) => dir,
        Err(answer) => return answer,
    };
    blocking(move || {
        let agent = &api.agent;
        let runner = agent.runner(&dir);
        if runner.setting() == Setting::Off {
            return fail(StatusCode::FORBIDDEN, OFF);
        }
        // `resume` and `continue` are not fields of this form (`write_body` refused them)
        let Target::Repo {
            repo,
            workspace,
            worktree,
            ..
        } = form.target
        else {
            return fail(StatusCode::BAD_REQUEST, "unknown field");
        };
        let repo = match stored_repo(&dir, &workspace, &repo) {
            Ok(path) => path,
            Err(answer) => return answer,
        };
        let account = match pick_account(&runner, &dir, form.account.as_deref()) {
            Ok(account) => account,
            Err(answer) => return answer,
        };
        let allowed = still_allowed(agent.runner(&dir), account.as_ref());
        let plan = Plan {
            repo,
            account,
            model: form.model,
            mode: form.mode,
            worktree,
        };
        match agent.terms.start(plan, &dir, allowed) {
            Ok(id) => respond(StatusCode::CREATED, &json!({"id": id})),
            Err(StartError::Busy) => busy(),
            Err(StartError::Refused(text)) => fail(StatusCode::BAD_REQUEST, text),
            Err(StartError::Running | StartError::Internal) => {
                fail(StatusCode::INTERNAL_SERVER_ERROR, INTERNAL)
            }
        }
    })
    .await
}

/// `POST /api/term-sessions/{id}/stop` `{confirmed: true}`: the master is closed, SIGKILL to the group after 5 s. An
/// entry that has exited is 200 and no change.
async fn stop(State(api): State<Api>, Path(id): Path<String>, req: Request) -> Response {
    if let Err(answer) = write_body(req, &[]).await {
        return answer;
    }
    blocking(move || {
        let Some(term) = api.agent.terms.get(&id) else {
            return fail(StatusCode::NOT_FOUND, NO_SESSION);
        };
        if !term.ended() {
            term.request_stop("stopped");
        }
        respond(StatusCode::OK, &term.json())
    })
    .await
}

/// The first frame's size, when it is a size at all.
fn size_of(frame: &Value) -> Option<(u64, u64)> {
    Some((frame.get("cols")?.as_u64()?, frame.get("rows")?.as_u64()?))
}

/// The first frame: `{"t":"auth","token":<the page token>,...}` within [`AUTH_WAIT`].
async fn authenticate(socket: &mut WebSocket, token: &str) -> Option<Value> {
    let first = tokio::time::timeout(AUTH_WAIT, socket.recv())
        .await
        .ok()??;
    let Ok(Message::Text(text)) = first else {
        return None;
    };
    let frame: Value = serde_json::from_str(text.as_str()).ok()?;
    let ok = frame.get("t") == Some(&json!("auth"))
        && frame
            .get("token")
            .and_then(Value::as_str)
            .is_some_and(|t| constant_time_eq(t.as_bytes(), token.as_bytes()));
    ok.then_some(frame)
}

fn text(socket_frame: Value) -> Message {
    Message::Text(socket_frame.to_string().into())
}

async fn attach(mut socket: WebSocket, api: Api, id: String, token: Arc<str>) {
    let Some(frame) = authenticate(&mut socket, &token).await else {
        let _ = socket.send(Message::Close(None)).await;
        return;
    };
    let Some(term) = api.agent.terms.get(&id) else {
        let _ = socket.send(Message::Close(None)).await;
        return;
    };
    let (replay, rx, exit) = term.attach();
    talk(&mut socket, &term, frame, replay, rx, exit).await;
    term.detach();
    let _ = socket.send(Message::Close(None)).await;
}

async fn talk(
    socket: &mut WebSocket,
    term: &Arc<Term>,
    frame: Value,
    replay: Vec<u8>,
    mut rx: tokio::sync::broadcast::Receiver<Out>,
    exit: Option<Option<i32>>,
) {
    if !replay.is_empty() && socket.send(Message::Binary(replay.into())).await.is_err() {
        return;
    }
    if let Some((cols, rows)) = size_of(&frame) {
        term.resize(cols, rows);
    }
    term.nudge();
    if let Some(code) = exit
        && socket
            .send(text(json!({"t": "exit", "code": code})))
            .await
            .is_err()
    {
        return;
    }
    loop {
        tokio::select! {
            got = socket.recv() => match got {
                Some(Ok(Message::Binary(bytes))) => {
                    if term.input(bytes.to_vec()) == Err(true)
                        && socket.send(text(json!({"t": "notice", "text": NOTICE}))).await.is_err()
                    {
                        return;
                    }
                }
                Some(Ok(Message::Text(raw))) => {
                    if let Ok(frame) = serde_json::from_str::<Value>(raw.as_str())
                        && frame.get("t") == Some(&json!("resize"))
                        && let Some((cols, rows)) = size_of(&frame)
                    {
                        term.resize(cols, rows);
                    }
                }
                Some(Ok(Message::Close(_)) | Err(_)) | None => return,
                Some(Ok(_)) => {} // ping, pong
            },
            out = rx.recv() => {
                let message = match out {
                    Ok(Out::Bytes(bytes)) => Message::Binary(bytes.to_vec().into()),
                    Ok(Out::Exit(code)) => text(json!({"t": "exit", "code": code})),
                    Ok(Out::Pages(n)) => text(json!({"t": "pages", "n": n})),
                    // too far behind: the page attaches again and gets the replay
                    Err(RecvError::Lagged(_) | RecvError::Closed) => return,
                };
                if socket.send(message).await.is_err() {
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_routes::Agent;
    use crate::agent_run::tests::{Fake, serial};
    use crate::server::router;
    use crate::term_session::tests::{DEAF, STUB};
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use futures_util::{SinkExt, StreamExt};
    use http_body_util::BodyExt;
    use rusqlite::Connection;
    use std::path::PathBuf;
    use std::time::Instant;
    use tokio::net::TcpStream;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::tungstenite::{Error as WsError, Message as WsMessage};
    use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
    use tower::ServiceExt;

    const TOKEN: &str = "test-token-t133";
    type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

    fn block<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
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
        port: u16,
        repo: PathBuf,
    }

    impl Drop for World {
        fn drop(&mut self) {
            self.api.agent.shutdown();
        }
    }

    /// A server on a real loopback port (the WebSocket needs one) over a fake data folder and a stub `claude`.
    async fn world(tag: &str, tail: &str) -> World {
        let fake = Fake::new(&format!("t133r-{tag}"), tail);
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
                ("ANTHROPIC_API_KEY", "sk-ant-T133-planted"),
                ("CLAUDECODE", "1"),
                ("CLAUDE_CODE_ENTRYPOINT", "cli"),
                ("CLAUDE_CONFIG_DIR", "/parent/.claude"),
            ]),
            PathBuf::from("/opt/t133/easy-personal-rs"),
            Duration::from_secs(30),
        );
        let api = Api::new(move || Ok(path.clone()), || 0).with_agent(agent);
        api.agent.runner(&fake.dir()).turn(true).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let app = router(TOKEN.into(), port, api.clone());
        tokio::spawn(axum::serve(listener, app.clone()).into_future());
        World {
            fake,
            _conn: conn,
            api,
            app,
            port,
            repo,
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
        async fn call(&self, method: &str, path: &str, body: &str) -> (StatusCode, Value) {
            self.call_with(method, path, body, &[]).await
        }

        async fn call_with(
            &self,
            method: &str,
            path: &str,
            body: &str,
            headers: &[(&str, &str)],
        ) -> (StatusCode, Value) {
            let mut request = HttpRequest::builder()
                .method(method)
                .uri(path)
                .header("host", format!("127.0.0.1:{}", self.port))
                .header("x-dashboard-token", TOKEN)
                .header("content-type", "application/json");
            for (k, v) in headers {
                request = request.header(*k, *v);
            }
            let response = self
                .app
                .clone()
                .oneshot(request.body(Body::from(body.to_string())).unwrap())
                .await
                .unwrap();
            let status = response.status();
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            (
                status,
                serde_json::from_slice(&bytes).unwrap_or(Value::Null),
            )
        }

        async fn post(&self, path: &str, body: &Value) -> (StatusCode, Value) {
            self.call("POST", path, &body.to_string()).await
        }

        fn form(&self, extra: Value) -> Value {
            let mut form = json!({
                "repo": self.repo.display().to_string(), "workspace": "acme", "model": "default",
                "mode": "default", "confirmed": true,
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
            let (status, body) = self.post("/api/term-sessions", &self.form(extra)).await;
            assert_eq!(status, 201, "{body}");
            body["id"].as_str().unwrap().to_string()
        }

        fn term(&self, id: &str) -> Arc<Term> {
            self.api.agent.terms.get(id).unwrap()
        }

        fn url(&self, path: &str) -> String {
            format!("ws://127.0.0.1:{}{path}", self.port)
        }

        async fn connect_with(&self, path: &str, headers: &[(&str, &str)]) -> Result<Ws, WsError> {
            let mut request = self.url(path).into_client_request().unwrap();
            for (k, v) in headers {
                request.headers_mut().insert(
                    axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                    v.parse().unwrap(),
                );
            }
            tokio_tungstenite::connect_async(request)
                .await
                .map(|(ws, _)| ws)
        }

        /// Attached with the token and a size.
        async fn attach(&self, id: &str, cols: u32, rows: u32) -> Ws {
            let origin = format!("http://127.0.0.1:{}", self.port);
            let mut ws = self
                .connect_with(&format!("/api/term/{id}"), &[("origin", &origin)])
                .await
                .unwrap();
            let auth = json!({"t": "auth", "token": TOKEN, "cols": cols, "rows": rows});
            ws.send(WsMessage::Text(auth.to_string().into()))
                .await
                .unwrap();
            ws
        }
    }

    /// What a page saw: the bytes, and the text frames as JSON.
    #[derive(Default)]
    struct Seen {
        bytes: Vec<u8>,
        texts: Vec<Value>,
        closed: bool,
    }

    impl Seen {
        fn text(&self) -> String {
            String::from_utf8_lossy(&self.bytes).into_owned()
        }
    }

    /// Reads until `done` holds, the socket closes, or 15 s pass.
    async fn read_until(ws: &mut Ws, seen: &mut Seen, mut done: impl FnMut(&Seen) -> bool) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        while !done(seen) && !seen.closed {
            match tokio::time::timeout_at(deadline, ws.next()).await {
                Err(_) => break,
                Ok(Some(Ok(WsMessage::Binary(b)))) => seen.bytes.extend_from_slice(&b),
                Ok(Some(Ok(WsMessage::Text(t)))) => {
                    seen.texts.push(serde_json::from_str(t.as_str()).unwrap())
                }
                Ok(Some(Ok(WsMessage::Close(_))) | Some(Err(_)) | None) => seen.closed = true,
                Ok(Some(Ok(_))) => {}
            }
        }
    }

    async fn saw(ws: &mut Ws, seen: &mut Seen, want: &str) {
        read_until(ws, seen, |s| s.text().contains(want)).await;
        assert!(
            seen.text().contains(want),
            "never saw {want:?}: {:?}",
            seen.text()
        );
    }

    // ---- AC 1, AC 2, AC 4: start, the form, the list ----

    #[test]
    fn start_is_201_and_runs_the_stub_in_a_pty_with_the_scrubbed_env_and_the_terminal_type() {
        let _serial = serial();
        block(async {
            let w = world("start", STUB).await;
            let id = w
                .begin(json!({"model": "opus", "mode": "plan", "account": "2"}))
                .await;
            let term = w.term(&id);
            until("the stub logged its env", || {
                w.fake.root.join("log/pid").exists()
            });
            let argv = w.fake.argv();
            assert_eq!(argv, ["--model", "opus", "--permission-mode", "plan"]);
            let env = std::fs::read(w.fake.root.join("log/env")).unwrap();
            let env: Vec<String> = env
                .split(|b| *b == 0)
                .map(|e| String::from_utf8_lossy(e).into_owned())
                .collect();
            assert!(env.contains(&"TERM=xterm-256color".to_string()));
            assert!(env.contains(&"COLORTERM=truecolor".to_string()));
            let config: Vec<&String> = env
                .iter()
                .filter(|e| e.starts_with("CLAUDE_CONFIG_DIR="))
                .collect();
            assert_eq!(
                config,
                ["CLAUDE_CONFIG_DIR=/acc/.claude-work"],
                "the chosen account's folder, not the parent's"
            );
            for gone in [
                "ANTHROPIC_API_KEY=",
                "CLAUDECODE=",
                "CLAUDE_CODE_",
                "MCP_TOOL_TIMEOUT=",
            ] {
                assert!(!env.iter().any(|e| e.starts_with(gone)), "{gone}");
            }
            let cwd = std::fs::read_to_string(w.fake.root.join("log/cwd")).unwrap();
            assert_eq!(
                cwd.trim(),
                w.repo.canonicalize().unwrap().display().to_string()
            );
            assert!(!term.ended());
            // AC 4: the list holds the entry and no screen byte
            let mut ws = w.attach(&id, 80, 24).await;
            let mut seen = Seen::default();
            saw(&mut ws, &mut seen, "ready").await;
            let (status, list) = w.call("GET", "/api/term-sessions", "").await;
            assert_eq!(status, 200);
            assert_eq!(list["max"], 4);
            let entry = &list["sessions"][0];
            assert_eq!(entry["id"], id);
            assert_eq!(entry["repo"], w.repo.display().to_string());
            assert_eq!(entry["state"], "running");
            assert_eq!(entry["code"], Value::Null);
            assert_eq!(entry["pages"], 1);
            assert_eq!(entry["account"], json!({"id": "2", "label": "work"}));
            assert!(entry["started"].as_str().unwrap().ends_with('Z'));
            let raw = list.to_string();
            assert!(
                !raw.contains("ready") && !raw.contains("/dev/pts"),
                "no screen byte: {raw}"
            );
        });
    }

    /// AC 2: every refusal has the code and the fixed text of `POST /api/app-sessions`.
    #[test]
    fn refusals_have_the_same_codes_and_texts_as_the_app_session_start_and_nothing_starts() {
        let _serial = serial();
        block(async {
            let w = world("refuse", STUB).await;
            let cases = [
                json!({"confirmed": null}),
                json!({"confirmed": false}),
                json!({"confirmed": "true"}),
                json!({"model": "gpt-9"}),
                json!({"mode": "yolo"}),
                json!({"model": 3}),
                json!({"repo": "/etc"}),
                json!({"repo": "a; touch /tmp/x"}),
                json!({"workspace": "nope"}),
                json!({"worktree": "yes"}),
                json!({"account": "4"}),
                json!({"account": "999"}),
                json!({"repo": null}),
                json!({"extra": 1}),
            ];
            for case in cases {
                let term = w.form(case.clone());
                let mut chat = term.clone();
                chat["prompt"] = json!("x");
                let got = w.post("/api/term-sessions", &term).await;
                let want = w.post("/api/app-sessions", &chat).await;
                assert_eq!(got, want, "{case}");
                assert!(got.0.is_client_error(), "{case}: {got:?}");
            }
            // fields of a Chat start only: refused as unknown here
            for extra in [
                json!({"prompt": "hi"}),
                json!({"resume": "abc"}),
                json!({"continue": true}),
                json!({"effort": "high"}),
            ] {
                let (status, body) = w.post("/api/term-sessions", &w.form(extra.clone())).await;
                assert_eq!(
                    (status, body["error"].as_str()),
                    (StatusCode::BAD_REQUEST, Some("unknown field")),
                    "{extra}"
                );
            }
            for (ct, body) in [
                ("text/plain", "{}"),
                ("application/json", "[]"),
                ("application/json", "not json"),
            ] {
                let got = w
                    .call_with("POST", "/api/term-sessions", body, &[("content-type", ct)])
                    .await;
                let want = w
                    .call_with("POST", "/api/app-sessions", body, &[("content-type", ct)])
                    .await;
                assert_eq!(got, want, "{ct} {body}");
            }
            // the guard: a foreign origin, a rebinding host
            for headers in [
                [("origin", "http://evil.example")],
                [("host", "evil.example:80")],
            ] {
                let body = w.form(json!({})).to_string();
                let got = w
                    .call_with("POST", "/api/term-sessions", &body, &headers)
                    .await;
                assert_eq!(got.0, StatusCode::FORBIDDEN);
                let want = w
                    .call_with("POST", "/api/app-sessions", &body, &headers)
                    .await;
                assert_eq!(got, want);
            }
            assert!(!w.fake.root.join("log/pid").exists(), "nothing was started");
            assert!(w.api.agent.terms.all().is_empty());
            // the setting off
            w.api.agent.runner(&w.fake.dir()).turn(false).unwrap();
            let got = w.post("/api/term-sessions", &w.form(json!({}))).await;
            let mut chat = w.form(json!({}));
            chat["prompt"] = json!("x");
            assert_eq!(got, w.post("/api/app-sessions", &chat).await);
            assert_eq!(got.0, StatusCode::FORBIDDEN);
        });
    }

    /// AC 3: one cap across both kinds: two Chat and two Terminal sessions fill it; a fifth of either kind is 409 with
    /// the same fixed text.
    #[test]
    fn the_fifth_start_of_either_kind_is_409_with_the_fixed_text() {
        let _serial = serial();
        block(async {
            let w = world("cap", STUB).await;
            let mut chat = w.form(json!({}));
            chat["prompt"] = json!("hello");
            for _ in 0..2 {
                assert_eq!(w.post("/api/app-sessions", &chat).await.0, 202);
            }
            let first = w.begin(json!({})).await;
            w.begin(json!({})).await;
            let term = w.post("/api/term-sessions", &w.form(json!({}))).await;
            let app = w.post("/api/app-sessions", &chat).await;
            assert_eq!(term.0, StatusCode::CONFLICT);
            assert_eq!(term, app);
            assert_eq!(term.1["error"], "4 app sessions are going; stop one first");
            // a terminal that ends frees a slot for either kind
            let (status, _) = w
                .post(
                    &format!("/api/term-sessions/{first}/stop"),
                    &json!({"confirmed": true}),
                )
                .await;
            assert_eq!(status, 200);
            let ended = w.term(&first);
            until("stopped", || ended.ended());
            assert_eq!(w.post("/api/app-sessions", &chat).await.0, 202);
            assert_eq!(
                w.post("/api/term-sessions", &w.form(json!({}))).await.0,
                409
            );
        });
    }

    /// AC 5: stop needs `confirmed`; unknown id is 404; the master closes and a child deaf to SIGHUP is killed after 5 s.
    #[test]
    fn stop_closes_the_master_then_kills_the_group_after_5_s_and_an_unknown_id_is_404() {
        let _serial = serial();
        block(async {
            let w = world("stop", DEAF).await;
            let (status, _) = w
                .post(
                    "/api/term-sessions/0123abcd/stop",
                    &json!({"confirmed": true}),
                )
                .await;
            assert_eq!(status, 404);
            let id = w.begin(json!({})).await;
            let term = w.term(&id);
            until("pid", || w.fake.root.join("log/pid").exists());
            let (status, _) = w
                .post(&format!("/api/term-sessions/{id}/stop"), &json!({}))
                .await;
            assert_eq!(status, 400, "confirmed is needed");
            assert!(!term.ended());
            let asked = Instant::now();
            let (status, view) = w
                .post(
                    &format!("/api/term-sessions/{id}/stop"),
                    &json!({"confirmed": true}),
                )
                .await;
            assert_eq!(status, 200, "{view}");
            until("killed", || term.ended());
            let took = asked.elapsed();
            assert!(
                took >= Duration::from_secs(5) && took < Duration::from_secs(8),
                "{took:?}"
            );
            let (_, list) = w.call("GET", "/api/term-sessions", "").await;
            assert_eq!(list["sessions"][0]["state"], "exited");
            assert_eq!(list["sessions"][0]["reason"], "stopped");
            let pid = std::fs::read_to_string(w.fake.root.join("log/pid")).unwrap();
            assert!(
                !std::path::Path::new(&format!("/proc/{}", pid.trim())).exists() || {
                    let stat = std::fs::read_to_string(format!("/proc/{}/stat", pid.trim()))
                        .unwrap_or_default();
                    stat.rsplit(')')
                        .next()
                        .unwrap_or("")
                        .trim_start()
                        .starts_with('Z')
                }
            );
        });
    }

    // ---- AC 6: the guard and the first frame ----

    #[test]
    fn the_socket_needs_a_good_host_and_origin_and_a_first_frame_token_and_never_reads_the_url() {
        let _serial = serial();
        block(async {
            let w = world("guard", STUB).await;
            let id = w.begin(json!({})).await;
            let path = format!("/api/term/{id}");
            let status = |r: Result<Ws, WsError>| match r {
                Err(WsError::Http(response)) => response.status().as_u16(),
                Err(e) => panic!("{e}"),
                Ok(_) => 101,
            };
            assert_eq!(
                status(
                    w.connect_with(&path, &[("origin", "http://evil.example")])
                        .await
                ),
                403
            );
            let rebind = format!("evil.example:{}", w.port);
            assert_eq!(
                status(w.connect_with(&path, &[("host", rebind.as_str())]).await),
                403
            );
            // a token header that is wrong is refused by the guard
            assert_eq!(
                status(
                    w.connect_with(&path, &[("x-dashboard-token", "nope")])
                        .await
                ),
                403
            );
            // only a plain hex id passes the guard without the header
            for other in [
                "/api/term/ABC",
                "/api/term/",
                "/api/term/a/b",
                "/api/term-sessions",
            ] {
                assert_eq!(status(w.connect_with(other, &[]).await), 403, "{other}");
            }
            // wrong token, a binary first frame, a token only in the URL with no frame (5 s), no frame at all: closed, no data
            let started = Instant::now();
            let mut wrong = w.connect_with(&path, &[]).await.unwrap();
            wrong
                .send(WsMessage::Text(
                    json!({"t": "auth", "token": "nope", "cols": 80, "rows": 24})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
            let mut binary = w.connect_with(&path, &[]).await.unwrap();
            binary
                .send(WsMessage::Binary(TOKEN.as_bytes().to_vec().into()))
                .await
                .unwrap();
            let mut url = w
                .connect_with(&format!("{path}?token={TOKEN}"), &[])
                .await
                .unwrap();
            let mut silent = w.connect_with(&path, &[]).await.unwrap();
            for ws in [&mut wrong, &mut binary, &mut url, &mut silent] {
                let mut seen = Seen::default();
                read_until(ws, &mut seen, |_| false).await;
                assert!(seen.closed, "closed");
                assert!(seen.bytes.is_empty() && seen.texts.is_empty(), "no data");
            }
            assert!(
                started.elapsed() >= AUTH_WAIT,
                "the silent ones waited for the first frame"
            );
            assert!(started.elapsed() < AUTH_WAIT + Duration::from_secs(5));
            // the right token works
            let mut ws = w.attach(&id, 80, 24).await;
            let mut seen = Seen::default();
            saw(&mut ws, &mut seen, "ready").await;
            // an unknown id with the right token: closed, no data
            let mut ws = w.attach("0123abcd", 80, 24).await;
            let mut seen = Seen::default();
            read_until(&mut ws, &mut seen, |_| false).await;
            assert!(seen.closed && seen.bytes.is_empty() && seen.texts.is_empty());
        });
    }

    // ---- AC 7, 8, 9, 10: in, out, two pages, replay, exit ----

    #[test]
    fn keys_arrive_in_order_resize_is_clamped_and_input_over_the_cap_gets_one_notice() {
        let _serial = serial();
        block(async {
            let w = world("io", STUB).await;
            let id = w.begin(json!({})).await;
            let term = w.term(&id);
            let mut ws = w.attach(&id, 80, 24).await;
            let mut seen = Seen::default();
            saw(&mut ws, &mut seen, "ready").await;
            // 186 single-key frames, then Enter
            let keys: Vec<u8> = (0..186)
                .map(|i| b"abcdefghijklmnopqrstuvwxyz0123456789"[i % 36])
                .collect();
            for key in &keys {
                ws.send(WsMessage::Binary(vec![*key].into())).await.unwrap();
            }
            ws.send(WsMessage::Binary(b"\r".to_vec().into()))
                .await
                .unwrap();
            let want = format!("got:{}", String::from_utf8_lossy(&keys));
            saw(&mut ws, &mut seen, &want).await;
            // resize: the stub prints its size on SIGWINCH
            ws.send(WsMessage::Text(
                json!({"t": "resize", "cols": 100, "rows": 30})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
            saw(&mut ws, &mut seen, "winch 30 100").await;
            ws.send(WsMessage::Text(
                json!({"t": "resize", "cols": 5000, "rows": 1})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
            saw(&mut ws, &mut seen, "winch 5 500").await;
            for zero in [
                json!({"t": "resize", "cols": 0, "rows": 30}),
                json!({"t": "resize", "cols": 90, "rows": 0}),
            ] {
                ws.send(WsMessage::Text(zero.to_string().into()))
                    .await
                    .unwrap();
            }
            ws.send(WsMessage::Text(json!({"t": "other"}).to_string().into()))
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_millis(200)).await;
            assert_eq!(term.size(), (500, 5), "zero ignored");
            // over 64 KiB in a second: dropped, one notice
            let line = {
                let mut l = vec![b'z'; 1023];
                l.push(b'\r');
                l
            };
            for _ in 0..80 {
                ws.send(WsMessage::Binary(line.clone().into()))
                    .await
                    .unwrap();
            }
            read_until(&mut ws, &mut seen, |s| {
                s.texts.iter().any(|t| t["t"] == "notice")
            })
            .await;
            tokio::time::sleep(Duration::from_millis(300)).await;
            read_until(&mut ws, &mut seen, |s| {
                s.texts.iter().filter(|t| t["t"] == "notice").count() > 1
            })
            .await;
            let notices = seen.texts.iter().filter(|t| t["t"] == "notice").count();
            assert_eq!(notices, 1, "{:?}", seen.texts);
            // out: the page count came on attach
            assert!(
                seen.texts
                    .iter()
                    .any(|t| *t == json!({"t": "pages", "n": 1}))
            );
        });
    }

    #[test]
    fn two_pages_both_see_the_output_both_type_the_last_resize_wins_and_both_get_the_exit() {
        let _serial = serial();
        block(async {
            let w = world("two", STUB).await;
            let id = w.begin(json!({})).await;
            let term = w.term(&id);
            let (mut a, mut b) = (w.attach(&id, 80, 24).await, w.attach(&id, 80, 24).await);
            let (mut sa, mut sb) = (Seen::default(), Seen::default());
            saw(&mut a, &mut sa, "ready").await;
            saw(&mut b, &mut sb, "ready").await;
            read_until(&mut a, &mut sa, |s| {
                s.texts.contains(&json!({"t": "pages", "n": 2}))
            })
            .await;
            assert!(
                sa.texts.contains(&json!({"t": "pages", "n": 2})),
                "{:?}",
                sa.texts
            );
            a.send(WsMessage::Binary(b"from-a\r".to_vec().into()))
                .await
                .unwrap();
            saw(&mut a, &mut sa, "got:from-a").await;
            saw(&mut b, &mut sb, "got:from-a").await;
            b.send(WsMessage::Binary(b"from-b\r".to_vec().into()))
                .await
                .unwrap();
            saw(&mut a, &mut sa, "got:from-b").await;
            saw(&mut b, &mut sb, "got:from-b").await;
            a.send(WsMessage::Text(
                json!({"t": "resize", "cols": 100, "rows": 30})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
            saw(&mut b, &mut sb, "winch 30 100").await;
            b.send(WsMessage::Text(
                json!({"t": "resize", "cols": 90, "rows": 20})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
            saw(&mut a, &mut sa, "winch 20 90").await;
            assert_eq!(term.size(), (90, 20), "the last resize wins");
            // the program exits by itself: both pages get the exit
            a.send(WsMessage::Binary(b"quit\r".to_vec().into()))
                .await
                .unwrap();
            let exit = json!({"t": "exit", "code": 7});
            read_until(&mut a, &mut sa, |s| s.texts.contains(&exit)).await;
            read_until(&mut b, &mut sb, |s| s.texts.contains(&exit)).await;
            assert!(sa.texts.contains(&exit) && sb.texts.contains(&exit));
            until("exited", || term.ended());
            let (_, list) = w.call("GET", "/api/term-sessions", "").await;
            assert_eq!(list["sessions"][0]["state"], "exited");
            assert_eq!(list["sessions"][0]["code"], 7);
            assert!(term.held() > 0, "kept while pages are attached");
            a.close(None).await.unwrap();
            read_until(&mut b, &mut sb, |s| {
                s.texts.contains(&json!({"t": "pages", "n": 1}))
            })
            .await;
            assert!(term.held() > 0);
            b.close(None).await.unwrap();
            until("dropped at the last detach", || term.held() == 0);
            assert_eq!(w.api.agent.terms.all().len(), 1, "the entry stays");
            let marker = w.fake.root.join("bin/claude").display().to_string();
            let left = std::process::Command::new("pgrep")
                .args(["-f", &marker])
                .output()
                .unwrap();
            assert!(left.stdout.is_empty(), "no stub left");
        });
    }

    #[test]
    fn a_page_that_attaches_later_gets_the_first_bytes_and_the_ring_then_a_resize_nudge() {
        let _serial = serial();
        block(async {
            let w = world("replay", STUB).await;
            let id = w.begin(json!({})).await;
            let term = w.term(&id);
            let mut first = w.attach(&id, 80, 24).await;
            let mut seen = Seen::default();
            saw(&mut first, &mut seen, "ready").await;
            first
                .send(WsMessage::Binary(b"before-reload\r".to_vec().into()))
                .await
                .unwrap();
            saw(&mut first, &mut seen, "got:before-reload").await;
            first.close(None).await.unwrap();
            until("detached", || term.json()["pages"] == 0);
            // a reload: the replay holds the start of the screen and what came after, then the nudge redraws (SIGWINCH)
            let mut again = w.attach(&id, 80, 24).await;
            let mut replay = Seen::default();
            read_until(&mut again, &mut replay, |s| !s.bytes.is_empty()).await;
            let text = replay.text();
            assert!(
                text.contains("tty /dev/pts/") && text.contains("got:before-reload"),
                "{text}"
            );
            saw(&mut again, &mut replay, "winch 24 80").await;
        });
    }

    /// AC 10: at the server's stop every PTY child ends, also one deaf to SIGHUP; no stub is left.
    #[test]
    fn the_servers_shutdown_ends_every_terminal_child() {
        let _serial = serial();
        block(async {
            let w = world("down", DEAF).await;
            let ids = [w.begin(json!({})).await, w.begin(json!({})).await];
            for id in &ids {
                let term = w.term(id);
                until("ready", || {
                    let (bytes, _, _) = term.attach();
                    term.detach();
                    String::from_utf8_lossy(&bytes).contains("ready")
                });
            }
            let agent = Arc::clone(&w.api.agent);
            tokio::task::spawn_blocking(move || agent.shutdown())
                .await
                .unwrap();
            assert!(w.api.agent.terms.all().is_empty());
            let marker = w.fake.root.join("bin/claude").display().to_string();
            let left = std::process::Command::new("pgrep")
                .args(["-f", &marker])
                .output()
                .unwrap();
            assert!(left.stdout.is_empty(), "no stub left");
        });
    }
}
