//! `GET /api/live/sessions/{id}/transcript?after=<offset>` or `?before=<offset>` (T115, FR-123, DDR-0007): the conversation
//! of one session, `{entries, next, complete}`, parsed by the core (`transcript.rs`). **The fifth exception of NFR-002**:
//! the answer's body is the only place the message text, tool input and tool output go. The file is opened anew for
//! every request with the live tail's open rules and nothing is kept, logged or counted (no log line here has any text).
//!
//! The file is the one the live tail read for the session (`session_origin`, a T105 resume included), else the newest
//! main-thread `source_path` the store holds for it (a past session, read only). The id never joins a path: it only
//! keys the tail and the store. A session neither knows is a 404 `no such session`; a file that is gone, a symlink,
//! not regular, unreadable or with an oversized line is a 404 `the transcript cannot be read`, never a 500.
//! No `after` and no `before` is the first view: the newest entries. The guard in `server.rs` covers the route.

use crate::api::{Api, error_body};
use crate::app_session::ResumeId;
use axum::extract::{Path, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use easy_personal_core::store::{self, session_transcript_path};
use easy_personal_core::transcript::{self, Window};
use serde_json::json;
use std::path::PathBuf;

pub(crate) const PATH: &str = "/api/live/sessions/{id}/transcript";
const UNKNOWN: &str = "no such session";
const UNREADABLE: &str = "the transcript cannot be read";
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

pub(crate) async fn get(State(api): State<Api>, Path(id): Path<String>, uri: Uri) -> Response {
    let Some(id) = ResumeId::new(&id) else {
        return fail(StatusCode::NOT_FOUND, UNKNOWN);
    };
    let Some(window) = window(uri.query().unwrap_or("")) else {
        return fail(StatusCode::BAD_REQUEST, BAD_OFFSET);
    };
    let from_tail = api.live_session_origin(id.as_str()).map(|(path, _)| path);
    let outcome = tokio::task::spawn_blocking(move || {
        let path = match from_tail {
            Some(path) => Some(path),
            None => (api.store_path)()
                .ok()
                .and_then(|file| store::open_checked(&file).ok())
                .and_then(|conn| session_transcript_path(&conn, id.as_str()).ok().flatten()),
        };
        let Some(path) = path.map(PathBuf::from) else {
            return fail(StatusCode::NOT_FOUND, UNKNOWN);
        };
        // a path of the store is a row any writer could have changed: only an absolute `*.jsonl` is opened
        if !path.is_absolute() || path.extension().is_none_or(|e| e != "jsonl") {
            return fail(StatusCode::NOT_FOUND, UNREADABLE);
        }
        match transcript::read(&path, window) {
            Ok(page) => answer(
                StatusCode::OK,
                &json!({"entries": page.entries, "next": page.next, "complete": page.complete}),
            ),
            Err(_) => fail(StatusCode::NOT_FOUND, UNREADABLE),
        }
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

    /// A temp copy of the parity store whose first Claude session id points at `file`.
    fn past_store(temp: &Temp, file: &std::path::Path) -> (PathBuf, String) {
        let path = temp.0.join("store.sqlite");
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/store-v11.sqlite"),
            &path,
        )
        .unwrap();
        let conn = rusqlite::Connection::open(&path).unwrap();
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
        let file = temp.0.join("past.jsonl");
        std::fs::write(&file, line("user", "from the past")).unwrap();
        let (store, id) = past_store(&temp, &file);
        let (app, _) = app(store, None);
        let (status, body) = get(&app, &url(&id, ""), &OK).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["entries"][0]["text"], "from the past");
    }

    #[tokio::test]
    async fn a_symlink_a_non_regular_file_an_oversized_line_and_a_foreign_path_are_404_never_500() {
        let temp = Temp::new();
        let real = temp.0.join("real.jsonl");
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
        let fifo = temp.0.join("fifo.jsonl");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        let link = temp.0.join("link.jsonl");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let folder = temp.0.join("dir.jsonl");
        std::fs::create_dir(&folder).unwrap();
        let huge = temp.0.join("huge.jsonl");
        let mut big = vec![b'x'; easy_personal_core::live::MAX_LINE_BYTES + 5];
        big.push(b'\n');
        std::fs::write(&huge, big).unwrap();
        for target in [
            link.display().to_string(),
            fifo.display().to_string(),
            folder.display().to_string(),
            huge.display().to_string(),
            temp.0.join("gone.jsonl").display().to_string(),
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
        point(&real.display().to_string());
        assert_eq!(get(&app, &url(&id, ""), &OK).await.0, 200);
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
