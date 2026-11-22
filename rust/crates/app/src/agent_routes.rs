//! The agent run routes (T091, FR-101, FR-103): `GET /api/agent-run`, `PUT /api/agent-run/setting`,
//! `POST /api/agent-run` and `POST /api/agent-run/stop`. They sit behind the guard in `server.rs` and use the body
//! rules of `writes.rs` (content type 415, size 413, JSON object 400).
//!
//! - The setting is the flag file of `agent_run.rs`, read at each request (nothing is cached): the page can turn it on,
//!   and `agent-run on|off` in a terminal can too. A flag that names no confirmed Claude account counts as off.
//! - A run starts on a thread of its own that lives until the run ends (`PR_SET_PDEATHSIG` follows the thread, so the
//!   `claude` child dies when the server stops), and the handler answers at once. One run at a time: this server's own
//!   (`busy`) or any other process's (the lock of `agent_run.rs`, so two dashboards, the tray and `agent-run now`
//!   share it). `stop` stops only this server's run; a run of another process is a 409.
//! - `last` is in memory only. No answer carries a path, an argument, an output line or an environment value: the
//!   run's output goes to `/dev/null` and is never read (NFR-002).
//! - The server's shutdown ([`AgentGuard`], also its drop) stops a going run and joins its thread.
//!
//! The one write feature of `serve`, which is read-only otherwise.

#![allow(clippy::result_large_err)]

use crate::agent_run::{LIMIT, Outcome, Refused, Runner, Setting, utc_now};
use crate::api::{Api, INTERNAL, respond};
use crate::app_session::Sessions;
use crate::term_session::Terms;
use crate::writes::{fail, parse, raw_body};
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::Response;
use easy_personal_core::store::open_checked;
use serde_json::{Map, Value, json};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const JOB: &str = "investigate";
/// The 403 text: it names no path.
const OFF: &str = "agent runs are off: choose an agent run account in Settings";
const FOREIGN: &str = "a run of another process is going; stop it there";
const NOT_CLAUDE: &str = "the account is not a confirmed Claude account";
const UNREADABLE: &str = "the store cannot be read right now; try again in a moment";
/// How long `stop` and `{account: null}` wait for this server's run to end (it polls every 20 ms).
const WAIT_STOPPED: Duration = Duration::from_secs(3);

#[derive(Default)]
struct Record {
    /// This server's run is going (set before its thread starts, cleared by the thread at its end).
    busy: bool,
    handle: Option<JoinHandle<()>>,
    last: Option<Value>,
}

/// What the routes of one server share: how to run `claude` and the record of this server's runs.
pub(crate) struct Agent {
    env: Vec<(OsString, OsString)>,
    exe: PathBuf,
    limit: Duration,
    stop: Arc<AtomicBool>,
    state: Arc<Mutex<Record>>,
    /// The app sessions of this server (T103); stopped with the agent run at shutdown.
    pub(crate) sessions: Arc<Sessions>,
    /// The terminal sessions of this server (T133); they share the app sessions' running cap; stopped at shutdown.
    pub(crate) terms: Arc<Terms>,
}

fn lock(state: &Mutex<Record>) -> MutexGuard<'_, Record> {
    state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Agent {
    /// This process's environment and program, the 10 minute limit.
    pub(crate) fn system() -> Agent {
        Agent::new(
            std::env::vars_os().collect(),
            std::env::current_exe().unwrap_or_default(),
            LIMIT,
        )
    }

    pub(crate) fn new(env: Vec<(OsString, OsString)>, exe: PathBuf, limit: Duration) -> Agent {
        let sessions = Arc::new(Sessions::new(env.clone()));
        Agent {
            terms: Arc::new(Terms::new(env.clone(), Arc::clone(&sessions))),
            sessions,
            env,
            exe,
            limit,
            stop: Arc::default(),
            state: Arc::default(),
        }
    }

    pub(crate) fn runner(&self, dir: &Path) -> Runner {
        Runner::over(
            dir.to_path_buf(),
            self.env.clone(),
            self.exe.clone(),
            self.limit,
            Arc::clone(&self.stop),
        )
    }

    fn busy(&self) -> bool {
        lock(&self.state).busy
    }

    /// The `GET` shape.
    fn json(&self, dir: &Path) -> Value {
        let runner = self.runner(dir);
        let (enabled, account) = match runner.setting() {
            Setting::Off => (false, Value::Null),
            Setting::Default => (true, Value::Null),
            Setting::Account(chosen) => (
                true,
                json!({
                    "id": chosen.id,
                    "label": chosen.label,
                    "status": if chosen.missing { "missing" } else { "confirmed" },
                }),
            ),
        };
        let state = lock(&self.state);
        json!({
            "enabled": enabled,
            "account": account,
            "claude_found": runner.claude().is_some(),
            "running": state.busy || runner.running(),
            "last": state.last.clone().unwrap_or(Value::Null),
        })
    }

    /// Starts the job on a thread that lives until it ends. `Err(true)`: a run is going (409); `Err(false)`: no thread.
    fn start(&self, dir: &Path, workspace: Option<String>) -> Result<(), bool> {
        let runner = self.runner(dir);
        let mut state = lock(&self.state);
        if state.busy || runner.running() {
            return Err(true);
        }
        self.stop.store(false, Ordering::Relaxed);
        let shared = Arc::clone(&self.state);
        let handle = std::thread::Builder::new()
            .name("agent-run".into())
            .spawn(move || {
                let started = utc_now();
                let result = runner.run(workspace.as_deref(), None);
                let mut state = lock(&shared);
                match result {
                    Ok(outcome) => state.last = Some(last(&outcome)),
                    // another process took the lock in the instant between the check and the start: nothing ran
                    Err(Refused::Going) => {}
                    Err(_) => {
                        state.last = Some(json!({
                            "job": JOB, "started": started, "ended": utc_now(),
                            "outcome": "failed", "report_id": Value::Null,
                        }));
                    }
                }
                state.busy = false;
            })
            .map_err(|_| false)?;
        state.busy = true;
        state.handle = Some(handle);
        Ok(())
    }

    /// Waits (a few seconds at most) for this server's run to end.
    fn wait_idle(&self) {
        let deadline = Instant::now() + WAIT_STOPPED;
        while self.busy() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Stops this server's run and joins its thread. After this no `claude` of this server lives.
    pub(crate) fn shutdown(&self) {
        self.sessions.shutdown();
        self.terms.shutdown();
        self.stop.store(true, Ordering::Relaxed);
        let handle = lock(&self.state).handle.take();
        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }
}

fn last(outcome: &Outcome) -> Value {
    json!({
        "job": JOB,
        "started": outcome.start_utc,
        "ended": outcome.end_utc,
        "outcome": outcome.ended.word(),
        "report_id": outcome.report,
    })
}

/// Stops the going run of this server when the server stops or is dropped.
pub(crate) struct AgentGuard(pub(crate) Arc<Agent>);

impl Drop for AgentGuard {
    fn drop(&mut self) {
        self.0.shutdown();
    }
}

/// The store's folder, absolute (the child runs in its own folder).
pub(crate) fn data_dir(api: &Api) -> Result<PathBuf, Response> {
    let path = (api.store_path)().map_err(|_| fail(StatusCode::SERVICE_UNAVAILABLE, UNREADABLE))?;
    path.parent()
        .and_then(|dir| std::path::absolute(dir).ok())
        .ok_or_else(|| fail(StatusCode::SERVICE_UNAVAILABLE, UNREADABLE))
}

/// Runs the blocking part off the async workers; a panic says nothing about why.
pub(crate) async fn blocking(work: impl FnOnce() -> Response + Send + 'static) -> Response {
    tokio::task::spawn_blocking(work)
        .await
        .unwrap_or_else(|_| fail(StatusCode::INTERNAL_SERVER_ERROR, INTERNAL))
}

/// A JSON object body: `{}` or an empty body when `empty_ok`.
pub(crate) async fn object(req: Request, empty_ok: bool) -> Result<Map<String, Value>, Response> {
    let (_, raw) = raw_body(req).await?;
    if empty_ok && raw.iter().all(u8::is_ascii_whitespace) {
        return Ok(Map::new());
    }
    match parse(&raw) {
        None => Err(fail(StatusCode::BAD_REQUEST, "body must be JSON")),
        Some(Value::Object(map)) => Ok(map),
        Some(_) => Err(fail(StatusCode::BAD_REQUEST, "body must be a JSON object")),
    }
}

/// `GET /api/agent-run` -> `{enabled, account, claude_found, running, last}`.
pub(crate) async fn get(State(api): State<Api>) -> Response {
    let dir = match data_dir(&api) {
        Ok(dir) => dir,
        Err(answer) => return answer,
    };
    blocking(move || respond(StatusCode::OK, &api.agent.json(&dir))).await
}

/// `PUT /api/agent-run/setting` `{account: "<id>", confirmed: true}` or `{account: null}`.
pub(crate) async fn setting(State(api): State<Api>, req: Request) -> Response {
    let body = match object(req, false).await {
        Ok(body) => body,
        Err(answer) => return answer,
    };
    if body.keys().any(|k| k != "account" && k != "confirmed") {
        return fail(StatusCode::BAD_REQUEST, "unknown field");
    }
    let account = match body.get("account") {
        Some(Value::Null) => None,
        Some(Value::String(reference)) => Some(reference.clone()),
        _ => {
            return fail(
                StatusCode::BAD_REQUEST,
                "account must be an account id or null",
            );
        }
    };
    if account.is_some() && body.get("confirmed") != Some(&Value::Bool(true)) {
        return fail(
            StatusCode::BAD_REQUEST,
            "confirmed must be true: agent runs use the Claude subscription login of the account and count against its plan limits",
        );
    }
    let dir = match data_dir(&api) {
        Ok(dir) => dir,
        Err(answer) => return answer,
    };
    blocking(move || {
        let agent = &api.agent;
        let runner = agent.runner(&dir);
        match account {
            Some(reference) => match runner.resolve(&reference) {
                Ok(id) => {
                    if runner.choose(&id).is_err() {
                        return fail(StatusCode::INTERNAL_SERVER_ERROR, INTERNAL);
                    }
                }
                Err(_) => return fail(StatusCode::BAD_REQUEST, NOT_CLAUDE),
            },
            None => {
                if runner.turn(false).is_err() {
                    return fail(StatusCode::INTERNAL_SERVER_ERROR, INTERNAL);
                }
                // the run sees the flag gone at its next poll
                agent.wait_idle();
            }
        }
        respond(StatusCode::OK, &agent.json(&dir))
    })
    .await
}

/// `POST /api/agent-run` `{job: "investigate", workspace?}` -> 202 the state, the run going on its own thread.
pub(crate) async fn start(State(api): State<Api>, req: Request) -> Response {
    let body = match object(req, false).await {
        Ok(body) => body,
        Err(answer) => return answer,
    };
    if body.keys().any(|k| k != "job" && k != "workspace") {
        return fail(StatusCode::BAD_REQUEST, "unknown field");
    }
    if body.get("job") != Some(&Value::String(JOB.into())) {
        return fail(StatusCode::BAD_REQUEST, "job must be \"investigate\"");
    }
    let workspace = match body.get("workspace") {
        None => None,
        Some(Value::String(name)) => Some(name.clone()).filter(|name| !name.is_empty()),
        Some(_) => return fail(StatusCode::BAD_REQUEST, "workspace must be a string"),
    };
    let dir = match data_dir(&api) {
        Ok(dir) => dir,
        Err(answer) => return answer,
    };
    blocking(move || {
        let agent = &api.agent;
        // read now, at this request: the flag may have been written or removed since the last one
        if agent.runner(&dir).setting() == Setting::Off {
            return fail(StatusCode::FORBIDDEN, OFF);
        }
        if let Some(name) = &workspace {
            match workspace_known(&dir, name) {
                Ok(true) => {}
                Ok(false) => return fail(StatusCode::BAD_REQUEST, "unknown workspace"),
                Err(()) => return fail(StatusCode::SERVICE_UNAVAILABLE, UNREADABLE),
            }
        }
        match agent.start(&dir, workspace) {
            Ok(()) => respond(StatusCode::ACCEPTED, &agent.json(&dir)),
            Err(true) => fail(StatusCode::CONFLICT, "a run is going"),
            Err(false) => fail(StatusCode::INTERNAL_SERVER_ERROR, INTERNAL),
        }
    })
    .await
}

fn workspace_known(dir: &Path, name: &str) -> Result<bool, ()> {
    let conn = open_checked(&dir.join("store.sqlite")).map_err(|_| ())?;
    conn.query_row("SELECT 1 FROM workspaces WHERE name = ?1", [name], |_| {
        Ok(())
    })
    .map(|()| true)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(false),
        _ => Err(()),
    })
}

/// `POST /api/agent-run/stop`: stops this server's run and answers the state; with no run going, 200 and no change.
pub(crate) async fn stop(State(api): State<Api>, req: Request) -> Response {
    match object(req, true).await {
        Ok(body) if body.is_empty() => {}
        Ok(_) => return fail(StatusCode::BAD_REQUEST, "unknown field"),
        Err(answer) => return answer,
    }
    let dir = match data_dir(&api) {
        Ok(dir) => dir,
        Err(answer) => return answer,
    };
    blocking(move || {
        let agent = &api.agent;
        if agent.busy() {
            agent.stop.store(true, Ordering::Relaxed);
            agent.wait_idle();
        } else if agent.runner(&dir).running() {
            return fail(StatusCode::CONFLICT, FOREIGN);
        }
        respond(StatusCode::OK, &agent.json(&dir))
    })
    .await
}

/// An account was ignored through the account routes: when it is the agent run account, the setting turns off.
pub(crate) fn account_gone(dir: &Path, id: &str) {
    let runner = Runner::over(
        dir.to_path_buf(),
        Vec::new(),
        PathBuf::new(),
        LIMIT,
        Arc::default(),
    );
    let _ = runner.forget(id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_run::tests::{Fake, GROUP, insert_report, runs, serial};
    use crate::server::router;
    use axum::Router;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use http_body_util::BodyExt;
    use rusqlite::Connection;
    use std::cell::RefCell;
    use std::future::Future;
    use std::os::unix::fs::PermissionsExt;
    use tower::ServiceExt;

    const TOKEN: &str = "test-token";
    const PORT: u16 = 4242;
    /// Planted in the environment the runner gives the child, and in the fake's output (AC 9).
    const SECRET: &str = "sk-ant-T091-planted-0123456789abcdefABCDEF0123456789";
    const WAIT: &str = "while [ ! -e \"$L/go\" ]; do sleep 0.02; done; exit 0";
    const JSON: Option<&str> = Some("application/json");

    /// The tests of this module take the `claude` lock (like the runner's), so each holds `serial()` outside its
    /// runtime: a std guard held across an `.await` is what clippy refuses.
    fn block<F: Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    }

    struct World {
        fake: Fake,
        conn: Connection,
        api: Api,
        app: Router,
        /// Every body any call got, to look for what must not be in one.
        seen: RefCell<String>,
    }

    fn world(tag: &str, tail: &str) -> World {
        let fake = Fake::new(&format!("t091-{tag}"), tail);
        let conn = fake.store();
        let path = fake.dir().join("store.sqlite");
        let agent = Agent::new(
            fake.env(&[("ANTHROPIC_API_KEY", SECRET)]),
            PathBuf::from("/opt/t091/easy-personal-rs"),
            Duration::from_secs(30),
        );
        let api = Api::new(move || Ok(path.clone()), || 0).with_agent(agent);
        World {
            app: router(TOKEN.into(), PORT, api.clone()),
            fake,
            conn,
            api,
            seen: RefCell::default(),
        }
    }

    impl World {
        fn flag(&self) -> std::path::PathBuf {
            self.fake.dir().join("agent-run.on")
        }

        fn started(&self) -> bool {
            self.fake.root.join("log/pid").exists()
        }

        /// A call with a good Host and token; `content_type` is sent when given.
        async fn call(
            &self,
            method: &str,
            path: &str,
            content_type: Option<&str>,
            body: &str,
        ) -> (StatusCode, Value) {
            let mut request = HttpRequest::builder()
                .method(method)
                .uri(path)
                .header("host", format!("127.0.0.1:{PORT}"))
                .header("x-dashboard-token", TOKEN);
            if let Some(content_type) = content_type {
                request = request.header("content-type", content_type);
            }
            self.send(request.body(Body::from(body.to_string())).unwrap())
                .await
        }

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

        async fn state(&self) -> Value {
            let (status, body) = self.call("GET", "/api/agent-run", None, "").await;
            assert_eq!(status, 200, "{body}");
            body
        }

        async fn put(&self, body: &str) -> (StatusCode, Value) {
            self.call("PUT", "/api/agent-run/setting", JSON, body).await
        }

        async fn post(&self, body: &str) -> (StatusCode, Value) {
            self.call("POST", "/api/agent-run", JSON, body).await
        }

        /// Waits until no run is going.
        async fn settle(&self) -> Value {
            let deadline = Instant::now() + Duration::from_secs(20);
            loop {
                let state = self.state().await;
                if state["running"] == false {
                    return state;
                }
                assert!(Instant::now() < deadline, "the run never ended: {state}");
                std::thread::sleep(Duration::from_millis(20));
            }
        }

        fn go(&self) {
            std::fs::write(self.fake.root.join("log/go"), "").unwrap();
        }
    }

    const SHAPE: [&str; 5] = ["account", "claude_found", "enabled", "last", "running"];

    fn keys(state: &Value) -> Vec<&str> {
        let mut keys: Vec<&str> = state
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        keys
    }

    // ---- AC 1: the guard ----

    #[test]
    fn each_route_without_the_token_with_a_wrong_host_or_a_foreign_origin_is_refused_and_does_nothing()
     {
        let _serial = serial();
        block(async {
            let w = world("guard", WAIT);
            let routes = [
                ("GET", "/api/agent-run", ""),
                (
                    "PUT",
                    "/api/agent-run/setting",
                    r#"{"account":"2","confirmed":true}"#,
                ),
                ("POST", "/api/agent-run", r#"{"job":"investigate"}"#),
                ("POST", "/api/agent-run/stop", "{}"),
            ];
            for (method, path, body) in routes {
                for refusal in ["no token", "wrong token", "wrong host", "foreign origin"] {
                    let mut request = HttpRequest::builder()
                        .method(method)
                        .uri(path)
                        .header("content-type", "application/json");
                    request = match refusal {
                        "wrong host" => request
                            .header("host", "evil.example:80")
                            .header("x-dashboard-token", TOKEN),
                        _ => request.header("host", format!("127.0.0.1:{PORT}")),
                    };
                    request = match refusal {
                        "wrong token" => request.header("x-dashboard-token", "nope"),
                        "foreign origin" => request
                            .header("x-dashboard-token", TOKEN)
                            .header("origin", "http://evil.example"),
                        _ => request,
                    };
                    let (status, answer) = w
                        .send(request.body(Body::from(body.to_string())).unwrap())
                        .await;
                    assert_eq!(status, 403, "{method} {path} {refusal}");
                    assert_eq!(answer, json!({"error": "forbidden"}));
                }
            }
            // a token in the query string is for `/api/live` only
            let (status, _) = w
                .send(
                    HttpRequest::builder()
                        .uri(format!("/api/agent-run?token={TOKEN}"))
                        .header("host", format!("127.0.0.1:{PORT}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await;
            assert_eq!(status, 403);
            assert!(!w.flag().exists(), "a refused PUT stores nothing");
            w.api.agent.runner(&w.fake.dir()).turn(true).unwrap();
            // with the setting on, a refused POST starts nothing
            let (status, _) = w
                .send(
                    HttpRequest::builder()
                        .method("POST")
                        .uri("/api/agent-run")
                        .header("host", format!("127.0.0.1:{PORT}"))
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"job":"investigate"}"#))
                        .unwrap(),
                )
                .await;
            assert_eq!(status, 403);
            assert!(!w.started());
            assert_eq!(w.state().await["running"], false);
        });
    }

    // ---- AC 2: the setting ----

    #[test]
    fn the_setting_is_off_by_default_put_stores_the_confirmed_claude_account_and_nothing_else() {
        let _serial = serial();
        block(async {
            let w = world("setting", WAIT);
            let state = w.state().await;
            assert_eq!(keys(&state), SHAPE);
            assert_eq!(
                state,
                json!({"enabled": false, "account": null, "claude_found": true, "running": false, "last": null})
            );
            assert!(
                !w.fake.dir().join("agent-run.on").exists(),
                "GET makes no flag"
            );

            let (status, state) = w.put(r#"{"account":"2","confirmed":true}"#).await;
            assert_eq!(status, 200, "{state}");
            assert_eq!(keys(&state), SHAPE);
            assert_eq!(state["enabled"], true);
            assert_eq!(state["account"]["id"], "2");
            assert_eq!(state["account"]["label"], "work");
            assert_eq!(std::fs::read_to_string(w.flag()).unwrap(), "2\n");
            assert_eq!(
                std::fs::metadata(w.flag()).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(w.state().await, state, "GET shows the same");

            // the account is chosen by label too, and the flag holds its id
            let (status, state) = w.put(r#"{"account":"HOME","confirmed":true}"#).await;
            assert_eq!(
                (status, state["account"]["id"].as_str()),
                (StatusCode::OK, Some("1"))
            );
            assert_eq!(std::fs::read_to_string(w.flag()).unwrap(), "1\n");
            w.put(r#"{"account":"2","confirmed":true}"#).await;

            // refused: nothing changes
            w.conn
                .execute(
                    "UPDATE agent_accounts SET status = 'ignored' WHERE id = 3",
                    [],
                )
                .unwrap();
            for bad in [
                r#"{"account":"3","confirmed":true}"#,  // an ignored account
                r#"{"account":"4","confirmed":true}"#,  // a Codex account
                r#"{"account":"99","confirmed":true}"#, // an unknown one
                r#"{"account":"nope","confirmed":true}"#,
                r#"{"account":"","confirmed":true}"#,
                r#"{"account":"1"}"#, // not confirmed
                r#"{"account":"1","confirmed":false}"#,
                r#"{"account":"1","confirmed":"true"}"#,
                r#"{"account":"1","confirmed":1}"#,
                r#"{"account":1,"confirmed":true}"#, // wrong type
                r#"{"confirmed":true}"#,             // no account
                r#"{"account":"1","confirmed":true,"x":1}"#,
            ] {
                let (status, body) = w.put(bad).await;
                assert_eq!(status, 400, "{bad}: {body}");
                assert!(body["error"].is_string());
                assert_eq!(std::fs::read_to_string(w.flag()).unwrap(), "2\n", "{bad}");
            }
            assert!(
                !w.seen.borrow().contains("valid labels"),
                "no label list in an error"
            );

            // off
            let (status, state) = w.put(r#"{"account":null}"#).await;
            assert_eq!(status, 200);
            assert_eq!(
                (state["enabled"].clone(), state["account"].clone()),
                (json!(false), Value::Null)
            );
            assert!(!w.flag().exists());
            let (status, _) = w.put(r#"{"account":null}"#).await;
            assert_eq!(status, 200, "off twice is no error");

            // T090's empty flag: on with the default account
            w.api.agent.runner(&w.fake.dir()).turn(true).unwrap();
            let state = w.state().await;
            assert_eq!(
                (state["enabled"].clone(), state["account"].clone()),
                (json!(true), Value::Null)
            );
        });
    }

    #[test]
    fn the_account_shows_whether_its_folder_is_there() {
        let _serial = serial();
        block(async {
            let w = world("missing", WAIT);
            w.put(r#"{"account":"2","confirmed":true}"#).await;
            assert_eq!(w.state().await["account"]["status"], "missing");
            let folder = w.fake.root.join("claude-work");
            std::fs::create_dir_all(&folder).unwrap();
            w.conn
                .execute(
                    "UPDATE agent_accounts SET folder = ?1 WHERE id = 2",
                    [folder.to_str().unwrap()],
                )
                .unwrap();
            assert_eq!(w.state().await["account"]["status"], "confirmed");
        });
    }

    // ---- AC 3: off, and the flag is read per request ----

    #[test]
    fn with_no_account_chosen_a_post_is_403_and_the_flag_is_read_at_each_request() {
        let _serial = serial();
        block(async {
            let w = world("off", WAIT);
            let (status, body) = w.post(r#"{"job":"investigate"}"#).await;
            assert_eq!(status, 403);
            assert_eq!(body, json!({"error": OFF}));
            assert!(!OFF.contains('/'), "the text names no path");
            assert!(!w.started());
            // the flag is written while the server runs (as a terminal's `agent-run on --account` does)
            let runner = w.api.agent.runner(&w.fake.dir());
            runner.choose("2").unwrap();
            let (status, state) = w.post(r#"{"job":"investigate"}"#).await;
            assert_eq!(status, 202, "{state}");
            assert_eq!(state["running"], true);
            w.fake.wait_for("pid");
            w.go();
            assert_eq!(w.settle().await["last"]["outcome"], "ok");
            // and removed again: the next POST is 403
            runner.turn(false).unwrap();
            assert_eq!(w.post(r#"{"job":"investigate"}"#).await.0, 403);
        });
    }

    // ---- AC 4: the body ----

    #[test]
    fn the_body_rules_of_the_other_write_routes_hold() {
        let _serial = serial();
        block(async {
            let w = world("body", WAIT);
            w.api.agent.runner(&w.fake.dir()).choose("2").unwrap();
            for (path, method, ok_body) in [
                ("/api/agent-run", "POST", r#"{"job":"investigate"}"#),
                ("/api/agent-run/setting", "PUT", r#"{"account":null}"#),
                ("/api/agent-run/stop", "POST", "{}"),
            ] {
                for content_type in [
                    None,
                    Some("text/plain"),
                    Some("application/x-www-form-urlencoded"),
                ] {
                    assert_eq!(
                        w.call(method, path, content_type, ok_body).await.0,
                        415,
                        "{path}"
                    );
                }
                assert_eq!(
                    w.call(method, path, Some("Application/JSON; charset=utf-8"), "[]")
                        .await
                        .0,
                    400,
                    "{path}: the content type is read like the other routes"
                );
                let big = format!(r#"{{"job":"{}"}}"#, "x".repeat((1 << 20) + 1));
                assert_eq!(w.call(method, path, JSON, &big).await.0, 413, "{path}");
                for bad in ["", "nope", "[]", "null", "\"investigate\"", "1"] {
                    if path.ends_with("stop") && bad.is_empty() {
                        continue; // an empty body stands for `{}` there
                    }
                    assert_eq!(
                        w.call(method, path, JSON, bad).await.0,
                        400,
                        "{path} {bad:?}"
                    );
                }
            }
            for bad in [
                "{}",
                r#"{"job":"other"}"#,
                r#"{"job":"Investigate"}"#,
                r#"{"job":1}"#,
                r#"{"job":null}"#,
                r#"{"workspace":"x"}"#,
                r#"{"job":"investigate","account":"1"}"#, // the run's account is the setting's
                r#"{"job":"investigate","account":null}"#,
                r#"{"job":"investigate","x":1}"#,
                r#"{"job":"investigate","workspace":1}"#,
                r#"{"job":"investigate","workspace":null}"#,
                r#"{"job":"investigate","workspace":["a"]}"#,
                r#"{"job":"investigate","workspace":"nope"}"#, // the store does not hold it
            ] {
                let (status, body) = w.post(bad).await;
                assert_eq!(status, 400, "{bad}: {body}");
                assert!(
                    !body.to_string().contains("nope"),
                    "the text names no input"
                );
            }
            assert!(!w.started(), "no bad body starts a run");
            // stop takes `{}` or nothing, no key
            assert_eq!(
                w.call("POST", "/api/agent-run/stop", JSON, r#"{"x":1}"#)
                    .await
                    .0,
                400
            );
            assert_eq!(w.call("POST", "/api/agent-run/stop", JSON, "").await.0, 200);
            // other methods
            for (method, path) in [
                ("DELETE", "/api/agent-run"),
                ("PUT", "/api/agent-run"),
                ("GET", "/api/agent-run/setting"),
                ("POST", "/api/agent-run/setting"),
                ("GET", "/api/agent-run/stop"),
            ] {
                assert_eq!(
                    w.call(method, path, JSON, "{}").await.0,
                    405,
                    "{method} {path}"
                );
            }
            // a workspace the store holds starts a run
            w.conn
                .execute("INSERT INTO workspaces (name) VALUES ('shop')", [])
                .unwrap();
            let (status, _) = w.post(r#"{"job":"investigate","workspace":"shop"}"#).await;
            assert_eq!(status, 202);
            w.fake.wait_for("pid");
            w.go();
            w.settle().await;
            assert!(
                w.fake.argv()[1].contains("shop"),
                "the workspace is the prompt's scope"
            );
        });
    }

    // ---- AC 5: a run ----

    #[test]
    fn a_run_is_202_then_running_a_second_post_is_409_and_the_end_shows_the_outcome_and_the_report()
    {
        let _serial = serial();
        block(async {
            let w = world("run", WAIT);
            w.put(r#"{"account":"2","confirmed":true}"#).await;
            let (status, state) = w.post(r#"{"job":"investigate"}"#).await;
            assert_eq!(status, 202, "{state}");
            assert_eq!(keys(&state), SHAPE);
            assert_eq!(state["running"], true);
            w.fake.wait_for("pid");
            assert_eq!(w.state().await["running"], true);
            assert!(
                w.fake
                    .child_env()
                    .iter()
                    .any(|(k, v)| k == "CLAUDE_CONFIG_DIR" && v == "/acc/.claude-work"),
                "the chosen account's folder"
            );
            assert!(
                !w.fake
                    .child_env()
                    .iter()
                    .any(|(k, _)| k == "ANTHROPIC_API_KEY")
            );
            let (status, body) = w.post(r#"{"job":"investigate"}"#).await;
            assert_eq!(
                (status, body),
                (StatusCode::CONFLICT, json!({"error": "a run is going"}))
            );
            assert_eq!(w.fake.log("pid").lines().count(), 1, "one process started");
            // the run saves a report through the real `save_report` code path
            let store = w.fake.dir().join("store.sqlite");
            let saved = easy_personal_core::reports::save(
                &store,
                &json!("found it"),
                &json!([]),
                None,
                Some("2"),
                easy_personal_core::clock::now_micros(),
            )
            .unwrap();
            w.go();
            let state = w.settle().await;
            let last = &state["last"];
            assert_eq!(last["job"], "investigate");
            assert_eq!(last["outcome"], "ok");
            assert_eq!(last["report_id"], saved.id);
            assert_eq!(
                keys(last),
                ["ended", "job", "outcome", "report_id", "started"]
            );
            assert!(last["started"].as_str().unwrap().ends_with('Z'));

            // the default account: the variable is removed; a run that saves nothing has no report id
            w.put(r#"{"account":"1","confirmed":true}"#).await;
            let _ = std::fs::remove_file(w.fake.root.join("log/env"));
            assert_eq!(w.post(r#"{"job":"investigate"}"#).await.0, 202);
            w.settle().await;
            assert!(
                !w.fake
                    .child_env()
                    .iter()
                    .any(|(k, _)| k == "CLAUDE_CONFIG_DIR")
            );
            let last = w.state().await["last"].clone();
            assert_eq!(
                (last["outcome"].clone(), last["report_id"].clone()),
                (json!("ok"), Value::Null)
            );
            // a failed run, and no `claude`
            std::fs::write(w.fake.root.join("bin/claude"), "#!/bin/sh\nexit 7\n").unwrap();
            assert_eq!(w.post(r#"{"job":"investigate"}"#).await.0, 202);
            assert_eq!(w.settle().await["last"]["outcome"], "failed");
            std::fs::remove_file(w.fake.root.join("bin/claude")).unwrap();
            assert_eq!(w.state().await["claude_found"], false);
            assert_eq!(w.post(r#"{"job":"investigate"}"#).await.0, 202);
            assert_eq!(w.settle().await["last"]["outcome"], "claude not found");
        });
    }

    /// T114: the app's `PATH` lacks `~/.local/bin` (the .deb tray, started by systemd); `claude` is found in `HOME`.
    #[test]
    fn claude_found_is_true_when_claude_is_only_in_the_homes_local_bin() {
        let _guard = serial();
        block(async {
            let w = world("t114", WAIT);
            std::fs::remove_file(w.fake.root.join("bin/claude")).unwrap();
            assert_eq!(
                w.state().await["claude_found"],
                false,
                "no PATH hit, no fallback"
            );
            let local = w.fake.root.join("home/.local/bin");
            std::fs::create_dir_all(&local).unwrap();
            let exe = local.join("claude");
            std::fs::write(&exe, "#!/bin/sh\nexit 0\n").unwrap();
            std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert_eq!(w.state().await["claude_found"], false, "not executable");
            std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
            assert_eq!(w.state().await["claude_found"], true);
        });
    }

    // ---- AC 6: stop and shutdown ----

    #[test]
    fn stop_ends_the_run_as_stopped_with_no_process_left_and_with_no_run_it_changes_nothing() {
        let _serial = serial();
        block(async {
            let w = world("stop", GROUP);
            w.put(r#"{"account":"2","confirmed":true}"#).await;
            // no run going: 200, nothing changes
            let before = w.state().await;
            let (status, after) = w.call("POST", "/api/agent-run/stop", JSON, "{}").await;
            assert_eq!((status, &after), (StatusCode::OK, &before));
            assert_eq!(w.state().await, before);
            assert!(!w.started());

            assert_eq!(w.post(r#"{"job":"investigate"}"#).await.0, 202);
            w.fake.wait_for("grandchild");
            let (status, state) = w.call("POST", "/api/agent-run/stop", JSON, "{}").await;
            assert_eq!(status, 200);
            assert_eq!(state["running"], false, "{state}");
            assert_eq!(state["last"]["outcome"], "stopped");
            assert_eq!(state["enabled"], true, "stop leaves the setting on");
            for file in ["pid", "grandchild"] {
                assert!(!runs(&w.fake.log(file)), "{file} still runs");
            }
            // a second stop: nothing to stop
            assert_eq!(
                w.call("POST", "/api/agent-run/stop", JSON, "{}").await.1["last"]["outcome"],
                "stopped"
            );

            // `{account: null}` stops a going run too
            let _ = std::fs::remove_file(w.fake.root.join("log/grandchild"));
            assert_eq!(w.post(r#"{"job":"investigate"}"#).await.0, 202);
            w.fake.wait_for("grandchild");
            let (status, state) = w.put(r#"{"account":null}"#).await;
            assert_eq!(status, 200);
            assert_eq!(
                (state["running"].clone(), state["enabled"].clone()),
                (json!(false), json!(false))
            );
            assert_eq!(state["last"]["outcome"], "stopped");
            assert!(!runs(&w.fake.log("grandchild")));
        });
    }

    #[test]
    fn a_run_of_another_process_shows_as_running_and_stop_does_not_touch_it() {
        let _serial = serial();
        block(async {
            let w = world("foreign", WAIT);
            let runner = w.api.agent.runner(&w.fake.dir());
            runner.choose("2").unwrap();
            let other = w.api.agent.runner(&w.fake.dir());
            let run = std::thread::spawn(move || other.run(None, None));
            w.fake.wait_for("pid");
            assert_eq!(w.state().await["running"], true);
            let (status, _) = w.post(r#"{"job":"investigate"}"#).await;
            assert_eq!(status, 409);
            let (status, body) = w.call("POST", "/api/agent-run/stop", JSON, "{}").await;
            assert_eq!(
                (status, body),
                (StatusCode::CONFLICT, json!({"error": FOREIGN}))
            );
            assert_eq!(w.state().await["running"], true, "still going");
            w.go();
            assert!(run.join().unwrap().is_ok());
            assert_eq!(w.state().await["running"], false);
            assert_eq!(
                w.state().await["last"],
                Value::Null,
                "last is this server's runs only"
            );
        });
    }

    #[test]
    fn the_server_stopping_during_a_run_leaves_no_process_of_its_group() {
        let _serial = serial();
        block(async {
            let w = world("shutdown", GROUP);
            w.put(r#"{"account":"2","confirmed":true}"#).await;
            assert_eq!(w.post(r#"{"job":"investigate"}"#).await.0, 202);
            w.fake.wait_for("grandchild");
            // what `Running` holds and drops when the server stops
            drop(AgentGuard(Arc::clone(&w.api.agent)));
            for file in ["pid", "grandchild"] {
                assert!(!runs(&w.fake.log(file)), "{file} still runs");
            }
            assert!(!w.api.agent.busy(), "its thread is joined");
        });
    }

    // ---- AC 7: the account goes ----

    #[test]
    fn an_account_that_is_ignored_or_gone_turns_the_setting_off_and_a_dead_flag_starts_nothing() {
        let _serial = serial();
        block(async {
            let w = world("gone", WAIT);
            for sql in [
                "UPDATE agent_accounts SET status = 'ignored' WHERE id = 2",
                "DELETE FROM agent_accounts WHERE id = 2",
                "UPDATE agent_accounts SET provider = 'codex' WHERE id = 2",
            ] {
                w.conn
                    .execute(
                        "INSERT OR REPLACE INTO agent_accounts (id, provider, folder, label, status, config_dir_env) \
                         VALUES (2, 'claude', '/acc/.claude-work', 'work', 'confirmed', '/acc/.claude-work')",
                        [],
                    )
                    .unwrap();
                assert_eq!(w.put(r#"{"account":"2","confirmed":true}"#).await.0, 200);
                assert_eq!(w.state().await["enabled"], true);
                w.conn.execute(sql, []).unwrap();
                let state = w.state().await;
                assert_eq!(
                    (state["enabled"].clone(), state["account"].clone()),
                    (json!(false), Value::Null),
                    "{sql}"
                );
                assert_eq!(w.post(r#"{"job":"investigate"}"#).await.0, 403, "{sql}");
                assert!(!w.started(), "{sql}");
            }
        });
    }

    #[test]
    fn the_ignore_hook_removes_the_flag_of_that_account_only() {
        let dir =
            std::env::temp_dir().join(format!("easy-personal-rs-t091-hook-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let runner = Runner::over(
            dir.clone(),
            Vec::new(),
            PathBuf::new(),
            LIMIT,
            Arc::default(),
        );
        runner.choose("2").unwrap();
        account_gone(&dir, "3");
        assert!(dir.join("agent-run.on").exists(), "another account's");
        account_gone(&dir, "2");
        assert!(!dir.join("agent-run.on").exists());
        runner.turn(true).unwrap();
        account_gone(&dir, "2");
        assert!(
            dir.join("agent-run.on").exists(),
            "an empty flag is the default account's"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- AC 9: nothing of the run reaches a response ----

    #[test]
    fn a_planted_secret_in_the_output_and_the_environment_is_in_no_response_body() {
        let _serial = serial();
        block(async {
            let w = world(
                "leak",
                &format!(
                    "echo '{SECRET}'; echo '{SECRET}' >&2; echo 'the model says hello'; {WAIT}"
                ),
            );
            insert_report(&w.conn, None);
            w.put(r#"{"account":"2","confirmed":true}"#).await;
            w.post(r#"{"job":"investigate"}"#).await;
            w.fake.wait_for("pid");
            w.state().await;
            w.call("POST", "/api/agent-run/stop", JSON, "{}").await;
            w.post(r#"{"job":"investigate"}"#).await;
            w.fake.wait_for("pid");
            w.go();
            w.settle().await;
            w.post(r#"{"job":"other"}"#).await;
            let seen = w.seen.borrow();
            for hidden in [
                SECRET,
                "the model says hello",
                "sk-ant",
                "/opt/t091",
                "--mcp-config",
                "mcp__easy-personal",
                "ANTHROPIC",
                "CLAUDE_CONFIG_DIR",
                "/acc/",
                w.fake.root.to_str().unwrap(),
            ] {
                assert!(!seen.contains(hidden), "{hidden} is in a response: {seen}");
            }
            assert!(seen.contains("investigate"), "the bodies were collected");
        });
    }
}
