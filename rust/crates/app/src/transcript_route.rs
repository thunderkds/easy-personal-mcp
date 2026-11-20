//! `GET /api/live/sessions/{id}/transcript?after=<offset>` or `?before=<offset>` (T115, FR-123, DDR-0007): the conversation
//! of one session, `{entries, next, start, complete}`, parsed by the core (`transcript.rs`). **The fifth exception of NFR-002**:
//! the answer's body is the only place the message text, tool input and tool output go. The file is opened anew for
//! every request with the live tail's open rules and nothing is kept, logged or counted (no log line here has any text).
//!
//! The file is the one the live tail read for the session (`session_origin`, a T105 resume included), else the newest
//! main-thread `source_path` the store holds for it (a past session, read only) when it lies under an account's `projects`
//! folder. The id never joins a path: it only keys the tail and the store. A session neither knows is a 404 `no such session`; a file that is gone, a symlink,
//! not regular, unreadable or with an oversized line is a 404 `the transcript cannot be read`, never a 500.
//! No `after` and no `before` is the first view: the newest entries. The guard in `server.rs` covers the route.
//!
//! T124 (FR-129, DDR-0008): `?agent=<id>` reads that session's sub-agent file `<main file stem>/subagents/agent-<id>.jsonl`
//! with the same parser, caps and paging (its `isSidechain` records kept); `GET /api/live/sessions/{id}/agents` lists the
//! folder. The agent id is checked by pattern and only ever joined under the session's own `subagents` folder, whose
//! real path must be exactly `<real main folder>/<stem>/subagents` (a symlink anywhere on it, or a file that is not regular,
//! is a 404 `the sub-agent log cannot be read`, never a 500). The meta file's `description` is never read.

use crate::api::{Api, error_body};
use crate::app_session::ResumeId;
use axum::extract::{Path, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use easy_personal_core::live::account_roots;
use easy_personal_core::store::{self, session_transcript_path};
use easy_personal_core::transcript::{self, Window};
use serde_json::json;
use std::path::PathBuf;

const UNKNOWN: &str = "no such session";
const UNREADABLE: &str = "the transcript cannot be read";
const AGENT_UNREADABLE: &str = "the sub-agent log cannot be read";
const BAD_OFFSET: &str = "after and before must be whole numbers, and only one of them";

/// `Some(window)`; `None` for a value that is not a whole number or when both are given.
fn window(query: &str) -> Option<Window> {
    let (mut after, mut before) = (None, None);
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match key {
            "after" => after = Some(value.parse::<u64>().ok()?),
            "before" => before = Some(value.parse::<u64>().ok()?),
            _ => {}
        }
    }
    match (after, before) {
        (Some(_), Some(_)) => None,
        (Some(n), None) => Some(Window::After(n)),
        (None, Some(n)) => Some(Window::Before(n)),
        (None, None) => Some(Window::Before(u64::MAX)),
    }
}

fn answer(status: StatusCode, body: &serde_json::Value) -> Response {
    (
        status,
        [
            (CONTENT_TYPE, "application/json"),
            (CACHE_CONTROL, "no-store"),
        ],
        body.to_string(),
    )
        .into_response()
}

fn fail(status: StatusCode, text: &str) -> Response {
    answer(status, &error_body(text))
}

/// A past session: the store's newest main-thread `source_path` for it, and whether it may be opened. A store row is data any
/// writer could have changed, so the path must be an absolute `*.jsonl` whose folder, once symlinks are resolved, lies under
/// the `projects` folder of a confirmed Claude account (the folders the live tail and the ingest read, with the ingest's
/// `EASY_PERSONAL_CLAUDE_PROJECTS` override). `None`: the store has no such session.
fn past(api: &Api, id: &str) -> Option<(String, bool)> {
    let conn = store::open_checked(&(api.store_path)().ok()?).ok()?;
    let path = session_transcript_path(&conn, id).ok().flatten()?;
    let projects = std::env::var_os("EASY_PERSONAL_CLAUDE_PROJECTS")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from);
    let roots = account_roots(&conn, projects.as_deref()).unwrap_or_default();
    let file = std::path::Path::new(&path);
    let inside = file.is_absolute()
        && file.extension().is_some_and(|e| e == "jsonl")
        && file
            .parent()
            .and_then(|dir| dir.canonicalize().ok())
            .is_some_and(|dir| {
                roots
                    .iter()
                    .filter_map(|root| root.projects.canonicalize().ok())
                    .any(|root| dir.starts_with(root))
            });
    Some((path, inside))
}

/// The `agent` value of the query as written (never decoded: a `%`, `.` or `/` fails the pattern): `Ok(None)` without the key,
/// `Err` for a value that is not an agent id.
fn agent_param(query: &str) -> Result<Option<&str>, ()> {
    let mut found = None;
    for pair in query.split('&') {
        if let Some(("agent", value)) = pair.split_once('=') {
            found = Some(
                transcript::valid_agent_id(value)
                    .then_some(value)
                    .ok_or(())?,
            );
        } else if pair == "agent" {
            return Err(());
        }
    }
    Ok(found)
}

/// The main file of the session: the one the live tail read, else the store's (a past session, root-checked).
/// `Err`: the one line of the 404.
fn main_file(api: &Api, id: &str, from_tail: Option<String>) -> Result<PathBuf, &'static str> {
    let (path, trusted) = match from_tail {
        Some(path) => (path, true), // the file the live tail itself read
        None => past(api, id).ok_or(UNKNOWN)?,
    };
    if !trusted {
        return Err(UNREADABLE);
    }
    Ok(PathBuf::from(path))
}

/// `<main folder>/<stem>/subagents` when its real path is exactly that under the real main folder (no symlink on the way,
/// so it lies where the main file lies: inside the confirmed account's `projects` root). `None` otherwise or when absent.
fn agents_dir(main: &std::path::Path) -> Option<PathBuf> {
    let parent = main.parent()?.canonicalize().ok()?;
    let want = parent.join(main.file_stem()?).join("subagents");
    (want.canonicalize().ok()? == want).then_some(want)
}

pub(crate) async fn get(State(api): State<Api>, Path(id): Path<String>, uri: Uri) -> Response {
    let Some(id) = ResumeId::new(&id) else {
        return fail(StatusCode::NOT_FOUND, UNKNOWN);
    };
    let query = uri.query().unwrap_or("").to_owned();
    let Some(window) = window(&query) else {
        return fail(StatusCode::BAD_REQUEST, BAD_OFFSET);
    };
    let Ok(agent) = agent_param(&query).map(|a| a.map(str::to_owned)) else {
        return fail(StatusCode::NOT_FOUND, AGENT_UNREADABLE);
    };
    let from_tail = api.live_session_origin(id.as_str()).map(|(path, _)| path);
    let outcome = tokio::task::spawn_blocking(move || {
        let main = match main_file(&api, id.as_str(), from_tail) {
            Ok(main) => main,
            Err(line) => return fail(StatusCode::NOT_FOUND, line),
        };
        let (path, unreadable) = match &agent {
            None => (main, UNREADABLE),
            Some(agent) => match agents_dir(&main) {
                Some(dir) => (dir.join(format!("agent-{agent}.jsonl")), AGENT_UNREADABLE),
                None => return fail(StatusCode::NOT_FOUND, AGENT_UNREADABLE),
            },
        };
        match transcript::read(&path, window, agent.is_some()) {
            Ok(page) => answer(
                StatusCode::OK,
                &json!({"entries": page.entries, "next": page.next, "start": page.start,
                        "complete": page.complete}),
            ),
            Err(_) => fail(StatusCode::NOT_FOUND, unreadable),
        }
    })
    .await;
    outcome.unwrap_or_else(|_| fail(StatusCode::INTERNAL_SERVER_ERROR, crate::api::INTERNAL))
}

/// `GET /api/live/sessions/{id}/agents` (FR-129): `{agents, total}`, the session's sub-agent streams, newest 200. A session
/// with no `subagents` folder (or one that is not where it must be) lists nothing.
pub(crate) async fn agents(State(api): State<Api>, Path(id): Path<String>) -> Response {
    let Some(id) = ResumeId::new(&id) else {
        return fail(StatusCode::NOT_FOUND, UNKNOWN);
    };
    let from_tail = api.live_session_origin(id.as_str()).map(|(path, _)| path);
    let outcome = tokio::task::spawn_blocking(move || {
        let main = match main_file(&api, id.as_str(), from_tail) {
            Ok(main) => main,
            Err(line) => return fail(StatusCode::NOT_FOUND, line),
        };
        let listing = agents_dir(&main).map(|dir| transcript::list_agents(&dir));
        let (agents, total) = listing.map_or((Vec::new(), 0), |l| (l.agents, l.total));
        answer(StatusCode::OK, &json!({"agents": agents, "total": total}))
    })
    .await;
    outcome.unwrap_or_else(|_| fail(StatusCode::INTERNAL_SERVER_ERROR, crate::api::INTERNAL))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::router;
    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use easy_personal_core::live::{LiveTail, Root};
    use easy_personal_core::prices::PriceTable;
    use http_body_util::BodyExt;
    use serde_json::Value;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    const TOKEN: &str = "test-token";
    const PORT: u16 = 4243;
    const SID: &str = "5e55c0de-0000-4000-8000-000000000115";
    const NOW: i64 = 1_768_867_200_000_000;

    #[test]
    fn the_query_picks_one_window_and_refuses_a_bad_number_or_both() {
        let tail = Window::Before(u64::MAX);
        assert_eq!(window(""), Some(tail));
        assert_eq!(window("after=12"), Some(Window::After(12)));
        assert_eq!(window("before=7&x=1"), Some(Window::Before(7)));
        for bad in [
            "after=-1",
            "after=",
            "after=1.5",
            "before=x",
            "after=1&before=2",
        ] {
            assert_eq!(window(bad), None, "{bad}");
        }
    }

    /// A temp folder; removed on drop.
    struct Temp(PathBuf);

    impl Temp {
        fn new() -> Temp {
            static N: AtomicUsize = AtomicUsize::new(0);
            let dir = std::env::temp_dir().join(format!(
                "easy-personal-rs-t115-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::SeqCst)
            ));
            std::fs::create_dir_all(dir.join("p")).unwrap();
            Temp(dir)
        }

        fn file(&self) -> PathBuf {
            self.0.join(format!("p/{SID}.jsonl"))
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn line(kind: &str, text: &str) -> String {
        let content = if kind == "user" {
            json!(text)
        } else {
            json!([{"type": "text", "text": text}])
        };
        format!(
            "{}\n",
            json!({"type": kind, "sessionId": SID, "cwd": "/w", "timestamp": "2026-01-19T23:00:00.000Z",
                   "message": {"id": "m1", "model": "x", "content": content}})
        )
    }

    /// The router over a store at `store` and, when `temp` is given, a live tail that has read its transcript.
    fn app(store: PathBuf, tail: Option<&Temp>) -> (Router, Option<crate::live_feed::SharedTail>) {
        let api = Api::new(move || Ok(store.clone()), || NOW);
        let shared = tail.map(|t| {
            let mut tail = LiveTail::new(PriceTable::shipped().unwrap());
            tail.poll_once(
                &[Root {
                    account: None,
                    projects: t.0.clone(),
                }],
                NOW,
                None,
            );
            Arc::new(Mutex::new(tail))
        });
        let api = match &shared {
            Some(tail) => api.with_live(Some(tail.clone()), Arc::new(AtomicBool::new(false))),
            None => api,
        };
        (router(TOKEN.into(), PORT, api), shared)
    }

    async fn get(app: &Router, path: &str, headers: &[(&str, &str)]) -> (StatusCode, Value) {
        let mut request = Request::builder().uri(path);
        for (k, v) in headers {
            request = request.header(*k, *v);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    const OK: [(&str, &str); 2] = [("host", "127.0.0.1:4243"), ("x-dashboard-token", TOKEN)];

    fn url(id: &str, query: &str) -> String {
        format!("/api/live/sessions/{id}/transcript{query}")
    }

    fn no_store() -> PathBuf {
        PathBuf::from("/nonexistent/easy-personal-t115/store.sqlite")
    }

    #[tokio::test]
    async fn a_live_session_is_read_from_the_tails_file_anew_at_every_call() {
        let temp = Temp::new();
        std::fs::write(temp.file(), line("user", "first")).unwrap();
        let (app, tail) = app(no_store(), Some(&temp));
        assert!(tail.unwrap().lock().unwrap().session_origin(SID).is_some());
        let (status, body) = get(&app, &url(SID, ""), &OK).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(
            (body["entries"][0]["text"].clone(), body["complete"].clone()),
            (json!("first"), json!(true))
        );
        let next = body["next"].as_u64().unwrap();
        // nothing is cached: the file changed between two calls and the second call sees it
        let mut more = line("assistant", "second");
        more.push_str(&line("user", "third"));
        let mut text = std::fs::read_to_string(temp.file()).unwrap();
        text.push_str(&more);
        std::fs::write(temp.file(), text).unwrap();
        let (status, body) = get(&app, &url(SID, &format!("?after={next}")), &OK).await;
        assert_eq!(status, 200);
        let texts: Vec<&Value> = body["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| &e["text"])
            .collect();
        assert_eq!(texts, [&json!("second"), &json!("third")]);
        // a file rewritten under the same name is read again, not remembered
        std::fs::write(temp.file(), line("user", "rewritten")).unwrap();
        let (_, body) = get(&app, &url(SID, ""), &OK).await;
        assert_eq!(body["entries"][0]["text"], "rewritten");
    }

    #[tokio::test]
    async fn a_gone_file_and_an_unknown_or_bad_id_are_404_with_one_line() {
        let temp = Temp::new();
        std::fs::write(temp.file(), line("user", "x")).unwrap();
        let (app, _tail) = app(no_store(), Some(&temp));
        std::fs::remove_file(temp.file()).unwrap();
        let (status, body) = get(&app, &url(SID, ""), &OK).await;
        assert_eq!((status.as_u16(), body), (404, json!({"error": UNREADABLE})));
        for id in [
            "5e55c0de-0000-4000-8000-0000000000f9",
            "..%2F..%2Fetc",
            "not-hex",
        ] {
            let (status, body) = get(&app, &url(id, ""), &OK).await;
            assert_eq!(
                (status.as_u16(), body),
                (404, json!({"error": UNKNOWN})),
                "{id}"
            );
        }
        let (status, _) = get(&app, &url(SID, "?after=x"), &OK).await;
        assert_eq!(status, 400);
    }

    /// The `projects/p` folder of the confirmed account `past_store` adds (its folder is the temp folder).
    fn projects(temp: &Temp) -> PathBuf {
        let dir = temp.0.join("projects/p");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A temp copy of the parity store with one confirmed Claude account (folder: the temp folder, so its `projects` folder
    /// is `projects(temp)`), whose first Claude session id points at `file`.
    fn past_store(temp: &Temp, file: &std::path::Path) -> (PathBuf, String) {
        let path = temp.0.join("store.sqlite");
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/store-v11.sqlite"),
            &path,
        )
        .unwrap();
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute(
            "INSERT INTO agent_accounts (provider, folder, label, status) VALUES ('claude', ?1, 'T115', 'confirmed')",
            [temp.0.display().to_string()],
        )
        .unwrap();
        let id: String = conn
            .query_row(
                "SELECT session_id FROM events WHERE source = 'claude_code' AND COALESCE(is_sidechain,0) = 0 LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        conn.execute(
            "UPDATE events SET source_path = ?1 WHERE session_id = ?2",
            rusqlite::params![file.display().to_string(), id],
        )
        .unwrap();
        (path, id)
    }

    #[tokio::test]
    async fn a_past_session_is_read_from_the_stores_newest_main_thread_path() {
        let temp = Temp::new();
        let file = projects(&temp).join("past.jsonl");
        std::fs::write(&file, line("user", "from the past")).unwrap();
        let (store, id) = past_store(&temp, &file);
        let (app, _) = app(store, None);
        let (status, body) = get(&app, &url(&id, ""), &OK).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["entries"][0]["text"], "from the past");
    }

    #[tokio::test]
    async fn a_symlink_a_non_regular_file_an_oversized_line_and_a_path_outside_the_account_folders_are_404_never_500()
     {
        let temp = Temp::new();
        let dir = projects(&temp);
        let real = dir.join("real.jsonl");
        std::fs::write(&real, line("user", "x")).unwrap();
        let (store, id) = past_store(&temp, &real);
        let conn = rusqlite::Connection::open(&store).unwrap();
        let point = |to: &str| {
            conn.execute(
                "UPDATE events SET source_path = ?1 WHERE session_id = ?2",
                rusqlite::params![to, id],
            )
            .unwrap();
        };
        let (app, _) = app(store.clone(), None);
        let fifo = dir.join("fifo.jsonl");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        let link = dir.join("link.jsonl");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let folder = dir.join("dir.jsonl");
        std::fs::create_dir(&folder).unwrap();
        let huge = dir.join("huge.jsonl");
        let mut big = vec![b'x'; easy_personal_core::live::MAX_LINE_BYTES + 5];
        big.push(b'\n');
        std::fs::write(&huge, big).unwrap();
        for target in [
            link.display().to_string(),
            fifo.display().to_string(),
            folder.display().to_string(),
            huge.display().to_string(),
            dir.join("gone.jsonl").display().to_string(),
            "relative/x.jsonl".to_string(),
            "/etc/passwd".to_string(),
        ] {
            point(&target);
            let (status, body) = get(&app, &url(&id, ""), &OK).await;
            assert_eq!(
                (status.as_u16(), body),
                (404, json!({"error": UNREADABLE})),
                "{target}"
            );
        }
        // a `.jsonl` outside every account folder (a row any writer could have changed), a folder symlinked out, and a file
        // that is not `*.jsonl` inside
        let outside = temp.0.join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(outside.join("x.jsonl"), line("user", "OUTSIDE")).unwrap();
        std::os::unix::fs::symlink(&outside, dir.join("out")).unwrap();
        std::fs::write(dir.join("notes.txt"), line("user", "TXT")).unwrap();
        for target in [
            outside.join("x.jsonl"),
            dir.join("out/x.jsonl"),
            dir.join("notes.txt"),
            dir.join("../../outside/x.jsonl"),
        ] {
            point(&target.display().to_string());
            let (status, body) = get(&app, &url(&id, ""), &OK).await;
            assert_eq!(
                (status.as_u16(), body),
                (404, json!({"error": UNREADABLE})),
                "{target:?}"
            );
        }
        point(&real.display().to_string());
        let (status, body) = get(&app, &url(&id, ""), &OK).await;
        assert_eq!(status, 200);
        assert_eq!(body["start"], 0);
    }

    // ---- T124 (FR-129): sub-agent logs ----

    const AGENT: &str = "a1addd7ec56e8f010";

    fn agent_line(kind: &str, text: &str) -> String {
        let content = if kind == "user" {
            json!(text)
        } else {
            json!([{"type": "text", "text": text}])
        };
        format!(
            "{}\n",
            json!({"type": kind, "isSidechain": true, "agentId": AGENT, "sessionId": SID, "cwd": "/w",
                   "timestamp": "2026-01-19T23:00:00.000Z", "message": {"id": "m2", "content": content}})
        )
    }

    /// `<dir of the main file>/<stem>/subagents`, made.
    fn agents_folder(main: &std::path::Path) -> PathBuf {
        let dir = main.with_extension("").join("subagents");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn agents_url(id: &str) -> String {
        format!("/api/live/sessions/{id}/agents")
    }

    fn agent_url(id: &str, agent: &str, rest: &str) -> String {
        format!("/api/live/sessions/{id}/transcript?agent={agent}{rest}")
    }

    #[tokio::test]
    async fn a_live_sessions_sub_agent_is_listed_and_read_with_its_sidechain_records() {
        let temp = Temp::new();
        let mut main = line("user", "main text");
        main.push_str(&format!(
            "{}\n",
            json!({"type":"assistant","isSidechain":true,"message":{"content":[{"type":"text","text":"SIDE-IN-MAIN"}]}})
        ));
        std::fs::write(temp.file(), main).unwrap();
        let folder = agents_folder(&temp.file());
        std::fs::write(
            folder.join(format!("agent-{AGENT}.jsonl")),
            agent_line("user", "agent prompt") + &agent_line("assistant", "agent says"),
        )
        .unwrap();
        std::fs::write(
            folder.join(format!("agent-{AGENT}.meta.json")),
            json!({"agentType": "Explore", "description": "DESC-MARKER", "toolUseId": "toolu_01X", "spawnDepth": 1, "finished": true})
                .to_string(),
        )
        .unwrap();
        let (app, _tail) = app(no_store(), Some(&temp));
        let (status, body) = get(&app, &agents_url(SID), &OK).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["total"], 1);
        let a = &body["agents"][0];
        assert_eq!(
            (
                &a["id"],
                &a["type"],
                &a["parent_call"],
                &a["depth"],
                &a["finished"]
            ),
            (
                &json!(AGENT),
                &json!("Explore"),
                &json!("toolu_01X"),
                &json!(1),
                &json!(true)
            )
        );
        assert!(!body.to_string().contains("DESC-MARKER"));
        let (status, body) = get(&app, &agent_url(SID, AGENT, ""), &OK).await;
        assert_eq!(status, 200, "{body}");
        let texts: Vec<&Value> = body["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| &e["text"])
            .collect();
        assert_eq!(texts, [&json!("agent prompt"), &json!("agent says")]);
        // T129: the page reads the kind of an entry from `kind` (never `type`), in a sub-agent's log and in the main thread
        let kinds = |body: &Value| -> Vec<(Value, Value)> {
            body["entries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| (e["kind"].clone(), e["type"].clone()))
                .collect()
        };
        assert_eq!(
            kinds(&body),
            [
                (json!("user"), Value::Null),
                (json!("assistant"), Value::Null)
            ]
        );
        let (_, main_body) = get(&app, &url(SID, ""), &OK).await;
        assert!(kinds(&main_body).contains(&(json!("user"), Value::Null)));
        // paging works as for the main file
        let next = body["next"].as_u64().unwrap();
        let (_, body) = get(&app, &agent_url(SID, AGENT, &format!("&after={next}")), &OK).await;
        assert_eq!(body["entries"], json!([]));
        let (_, body) = get(&app, &agent_url(SID, AGENT, "&before=0"), &OK).await;
        assert_eq!(body["entries"], json!([]));
        // the main file still drops its sidechain records
        let (_, body) = get(&app, &url(SID, ""), &OK).await;
        assert!(!body.to_string().contains("SIDE-IN-MAIN"));
    }

    #[tokio::test]
    async fn a_past_sessions_sub_agent_is_listed_and_read_from_the_store_path() {
        let temp = Temp::new();
        let file = projects(&temp).join("past.jsonl");
        std::fs::write(&file, line("user", "from the past")).unwrap();
        let folder = agents_folder(&file);
        std::fs::write(
            folder.join("agent-bbb2.jsonl"),
            agent_line("user", "past agent"),
        )
        .unwrap();
        let (store, id) = past_store(&temp, &file);
        let (app, _) = app(store, None);
        let (status, body) = get(&app, &agents_url(&id), &OK).await;
        assert_eq!(
            (status.as_u16(), &body["total"]),
            (200, &json!(1)),
            "{body}"
        );
        assert_eq!(
            (
                &body["agents"][0]["id"],
                &body["agents"][0]["type"],
                &body["agents"][0]["parent_call"]
            ),
            (&json!("bbb2"), &json!("sub-agent"), &Value::Null)
        );
        let (status, body) = get(&app, &agent_url(&id, "bbb2", ""), &OK).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["entries"][0]["text"], "past agent");
    }

    #[tokio::test]
    async fn a_session_without_a_subagents_folder_lists_nothing() {
        let temp = Temp::new();
        std::fs::write(temp.file(), line("user", "x")).unwrap();
        let (app, _tail) = app(no_store(), Some(&temp));
        let (status, body) = get(&app, &agents_url(SID), &OK).await;
        assert_eq!(
            (status.as_u16(), body),
            (200, json!({"agents": [], "total": 0}))
        );
        let (status, body) = get(&app, &agent_url(SID, AGENT, ""), &OK).await;
        assert_eq!(
            (status.as_u16(), body),
            (404, json!({"error": AGENT_UNREADABLE}))
        );
        // an unknown or bad session id is the session 404
        for id in ["5e55c0de-0000-4000-8000-0000000000f9", "not-hex"] {
            let (status, body) = get(&app, &agents_url(id), &OK).await;
            assert_eq!(
                (status.as_u16(), body),
                (404, json!({"error": UNKNOWN})),
                "{id}"
            );
        }
    }

    #[tokio::test]
    async fn a_bad_gone_or_hostile_agent_file_is_404_with_one_line_never_500() {
        let temp = Temp::new();
        std::fs::write(temp.file(), line("user", "x")).unwrap();
        let folder = agents_folder(&temp.file());
        std::fs::write(folder.join("agent-ok1.jsonl"), agent_line("user", "fine")).unwrap();
        let outside = temp.0.join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(
            outside.join("agent-sec.jsonl"),
            agent_line("user", "OUTSIDE"),
        )
        .unwrap();
        std::os::unix::fs::symlink(
            outside.join("agent-sec.jsonl"),
            folder.join("agent-lnk.jsonl"),
        )
        .unwrap();
        std::fs::create_dir(folder.join("agent-dir.jsonl")).unwrap();
        let fifo = folder.join("agent-fifo.jsonl");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        let mut big = vec![b'x'; easy_personal_core::live::MAX_LINE_BYTES + 5];
        big.push(b'\n');
        std::fs::write(folder.join("agent-huge.jsonl"), big).unwrap();
        let (app, _tail) = app(no_store(), Some(&temp));
        let (status, _) = get(&app, &agent_url(SID, "ok1", ""), &OK).await;
        assert_eq!(status, 200);
        let long = "x".repeat(65);
        for agent in [
            "..", "../x", "..%2Fx", "a%2Fb", "a/b", "a.b", "a-b", "", "gone", "lnk", "dir", "fifo",
            "huge", &long,
        ] {
            let (status, body) = get(&app, &agent_url(SID, agent, ""), &OK).await;
            assert_eq!(
                (status.as_u16(), body),
                (404, json!({"error": AGENT_UNREADABLE})),
                "{agent:?}"
            );
        }
        // `agent` with no `=` is no id
        let (status, _) = get(
            &app,
            &format!("/api/live/sessions/{SID}/transcript?agent"),
            &OK,
        )
        .await;
        assert_eq!(status, 404);
        // a bad window is still the 400
        let (status, _) = get(&app, &agent_url(SID, "ok1", "&after=x"), &OK).await;
        assert_eq!(status, 400);
        // the listing leaves the symlink, the folder and the foreign names out
        let (_, body) = get(&app, &agents_url(SID), &OK).await;
        let ids: std::collections::BTreeSet<String> = body["agents"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["id"].as_str().unwrap().to_owned())
            .collect();
        assert!(
            ids.contains("ok1") && !ids.contains("lnk") && !ids.contains("dir"),
            "{ids:?}"
        );
        assert!(!body.to_string().contains("OUTSIDE"));
    }

    #[tokio::test]
    async fn a_symlinked_subagents_folder_or_stem_folder_is_not_followed() {
        let temp = Temp::new();
        std::fs::write(temp.file(), line("user", "x")).unwrap();
        let outside = temp.0.join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(
            outside.join("agent-out1.jsonl"),
            agent_line("user", "OUTSIDE"),
        )
        .unwrap();
        let stem = temp.file().with_extension("");
        // `subagents` itself a symlink to a folder elsewhere
        std::fs::create_dir(&stem).unwrap();
        std::os::unix::fs::symlink(&outside, stem.join("subagents")).unwrap();
        let (app, _tail) = app(no_store(), Some(&temp));
        let (status, body) = get(&app, &agent_url(SID, "out1", ""), &OK).await;
        assert_eq!(
            (status.as_u16(), body),
            (404, json!({"error": AGENT_UNREADABLE}))
        );
        let (status, body) = get(&app, &agents_url(SID), &OK).await;
        assert_eq!(
            (status.as_u16(), body),
            (200, json!({"agents": [], "total": 0}))
        );
        // the stem folder a symlink
        std::fs::remove_dir_all(&stem).unwrap();
        std::fs::create_dir(outside.join("subagents")).unwrap();
        std::fs::write(
            outside.join("subagents/agent-out1.jsonl"),
            agent_line("user", "OUTSIDE"),
        )
        .unwrap();
        std::os::unix::fs::symlink(&outside, &stem).unwrap();
        let (status, _) = get(&app, &agent_url(SID, "out1", ""), &OK).await;
        assert_eq!(status, 404);
        let (_, body) = get(&app, &agents_url(SID), &OK).await;
        assert_eq!(body, json!({"agents": [], "total": 0}));
    }

    #[tokio::test]
    async fn a_hostile_past_path_gives_the_same_404_for_the_agent_routes() {
        let temp = Temp::new();
        let dir = projects(&temp);
        let real = dir.join("real.jsonl");
        std::fs::write(&real, line("user", "x")).unwrap();
        // a sub-agent folder of a file OUTSIDE every account folder, which the store row points at
        let outside = temp.0.join("outside");
        std::fs::create_dir(&outside).unwrap();
        let stray = outside.join("stray.jsonl");
        std::fs::write(&stray, line("user", "x")).unwrap();
        std::fs::create_dir_all(outside.join("stray/subagents")).unwrap();
        std::fs::write(
            outside.join("stray/subagents/agent-s1.jsonl"),
            agent_line("user", "OUTSIDE"),
        )
        .unwrap();
        let (store, id) = past_store(&temp, &stray);
        let (app, _) = app(store, None);
        let (status, body) = get(&app, &agents_url(&id), &OK).await;
        assert_eq!((status.as_u16(), body), (404, json!({"error": UNREADABLE})));
        let (status, body) = get(&app, &agent_url(&id, "s1", ""), &OK).await;
        assert_eq!((status.as_u16(), body), (404, json!({"error": UNREADABLE})));
    }

    #[tokio::test]
    async fn the_agent_routes_read_anew_and_leave_the_tail_alone() {
        let temp = Temp::new();
        std::fs::write(temp.file(), line("user", "x")).unwrap();
        let folder = agents_folder(&temp.file());
        let file = folder.join(format!("agent-{AGENT}.jsonl"));
        std::fs::write(&file, agent_line("user", "one")).unwrap();
        let (app, tail) = app(no_store(), Some(&temp));
        let tail = tail.unwrap();
        let before = tail.lock().unwrap().session_origin(SID);
        let (_, body) = get(&app, &agent_url(SID, AGENT, ""), &OK).await;
        assert_eq!(body["entries"][0]["text"], "one");
        std::fs::write(&file, agent_line("user", "two")).unwrap();
        let (_, body) = get(&app, &agent_url(SID, AGENT, ""), &OK).await;
        assert_eq!(body["entries"][0]["text"], "two");
        let _ = get(&app, &agents_url(SID), &OK).await;
        assert_eq!(tail.lock().unwrap().session_origin(SID), before);
    }

    #[tokio::test]
    async fn the_guard_covers_the_agent_routes() {
        let temp = Temp::new();
        std::fs::write(temp.file(), line("user", "guarded")).unwrap();
        let folder = agents_folder(&temp.file());
        std::fs::write(
            folder.join(format!("agent-{AGENT}.jsonl")),
            agent_line("user", "g"),
        )
        .unwrap();
        let (app, _tail) = app(no_store(), Some(&temp));
        for path in [agents_url(SID), agent_url(SID, AGENT, "")] {
            for headers in [
                vec![("host", "127.0.0.1:4243")],
                vec![("host", "127.0.0.1:4243"), ("x-dashboard-token", "wrong")],
                vec![("host", "evil.example"), ("x-dashboard-token", TOKEN)],
                vec![
                    ("host", "127.0.0.1:4243"),
                    ("x-dashboard-token", TOKEN),
                    ("origin", "http://evil.invalid"),
                ],
            ] {
                let (status, body) = get(&app, &path, &headers).await;
                assert_eq!(
                    (status.as_u16(), body),
                    (403, json!({"error": "forbidden"})),
                    "{path} {headers:?}"
                );
            }
        }
        let (status, _) = get(
            &app,
            &format!("{}?token={TOKEN}", agents_url(SID)),
            &[("host", "127.0.0.1:4243")],
        )
        .await;
        assert_eq!(status, 403);
    }

    #[tokio::test]
    async fn the_guard_covers_the_route() {
        let temp = Temp::new();
        std::fs::write(temp.file(), line("user", "guarded")).unwrap();
        let (app, _tail) = app(no_store(), Some(&temp));
        let path = url(SID, "");
        for headers in [
            vec![("host", "127.0.0.1:4243")],
            vec![("host", "127.0.0.1:4243"), ("x-dashboard-token", "wrong")],
            vec![("host", "evil.example"), ("x-dashboard-token", TOKEN)],
            vec![
                ("host", "127.0.0.1:4243"),
                ("x-dashboard-token", TOKEN),
                ("origin", "http://evil.example"),
            ],
        ] {
            let (status, body) = get(&app, &path, &headers).await;
            assert_eq!(
                (status.as_u16(), body),
                (403, json!({"error": "forbidden"})),
                "{headers:?}"
            );
        }
        // the query token is for the live stream only
        let (status, _) = get(
            &app,
            &format!("{path}?token={TOKEN}"),
            &[("host", "127.0.0.1:4243")],
        )
        .await;
        assert_eq!(status, 403);
    }
}
