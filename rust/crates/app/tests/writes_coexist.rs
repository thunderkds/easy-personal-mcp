//! T048: a write route while `easy-personal-rs ingest` runs on the same WAL store (the T031 harness: real processes, an
//! environment built from nothing, every child in its own process group). Write requests (create, rename, delete a
//! workspace; account picks) are sent one after the other for as long as the ingest runs: each is answered with its
//! own status and no body ever carries a SQLite lock message. Afterwards the ingest has finished with every event
//! stored, the workspaces are exactly what the requests left, and `integrity_check` is `ok`.
//!
//! `cargo test -p easy-personal-rs --test writes_coexist -- --nocapture`

#[path = "../../core/tests/ingest_folders_common/mod.rs"]
mod common;
mod ingest_coexist_common;

use common::Case;
use easy_personal_core::writer::open_for_write;
use ingest_coexist_common::*;
use rusqlite::Connection;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

struct Api {
    port: u16,
    token: String,
}

impl Api {
    fn send(&self, method: &str, path: &str, body: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nx-dashboard-token: {}\r\ncontent-type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            self.port,
            self.token,
            body.len()
        )
        .unwrap();
        let mut bytes = Vec::new();
        let _ = stream.read_to_end(&mut bytes);
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
        let status = head
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        (status, body.to_string())
    }
}

/// The one designed answer of a busy store (`writes.rs` `BUSY`), after the 5 s busy wait.
const BUSY_TEXT: &str = "the store is busy (another process is writing); try again in a moment";
/// How many times one request is tried when it is answered with the designed busy answer.
const TRIES: u32 = 6;

impl Api {
    /// [`Api::send`], tried again (up to [`TRIES`] times in all) while the answer is the designed 503 with its fixed text;
    /// every retry is counted. Any other answer is returned as it is; the caller judges it.
    fn send_retrying(
        &self,
        method: &str,
        path: &str,
        body: &str,
        retries: &mut u32,
    ) -> (u16, String) {
        for attempt in 1..=TRIES {
            let (status, text) = self.send(method, path, body);
            let busy = status == 503 && text.contains(BUSY_TEXT);
            if !busy || attempt == TRIES {
                return (status, text);
            }
            *retries += 1;
        }
        unreachable!()
    }
}

/// Fails the test with the ingest child's state: it is stopped (its group killed) and its exit code, stdout and stderr
/// are printed, because a bare `{body}` can be an empty line.
fn explain(mut ingest: Proc, what: &str) -> ! {
    let was_running = ingest.running();
    ingest.terminate();
    let out = ingest.wait(Duration::from_secs(30));
    panic!(
        "{what}\ningest child: {} when the test failed; exit code {:?}\n--- ingest stdout ---\n{}\n--- ingest stderr ---\n{}",
        if was_running {
            "still running"
        } else {
            "already finished"
        },
        out.code,
        out.stdout,
        out.stderr
    );
}

#[test]
fn writes_and_an_ingest_share_the_wal_store_without_a_lock_error() {
    let case = Case::new();
    write_set_padded(&case.projects, "big", 600, 400, 48, 40);
    std::fs::create_dir_all(&case.home).unwrap();
    drop(open_for_write(&case.store_path()).unwrap());

    let mut serve = Spec::rust(&case, &["serve"]).spawn();
    let mut lines = BufReader::new(serve.child.stdout.take().unwrap());
    let mut first = String::new();
    lines.read_line(&mut first).unwrap();
    let (port, token) = first
        .trim_end()
        .strip_prefix("dashboard: http://127.0.0.1:")
        .expect("the URL line")
        .split_once("/#")
        .expect("a token fragment");
    let api = Api {
        port: port.parse().unwrap(),
        token: token.to_string(),
    };

    let mut ingest = Spec::rust(&case, &["ingest"]).spawn();
    let (mut requests, mut kept, mut retries) = (0, Vec::new(), 0u32);
    // Each request must be answered with its own status. The designed 503 (busy past the 5 s wait) is retried; a 500, a
    // lock-error text, an exhausted bound or any other status stops the test with the ingest child's state.
    macro_rules! write {
        ($want:expr, $method:expr, $path:expr, $body:expr) => {{
            let (status, body) = api.send_retrying($method, $path, $body, &mut retries);
            if status != $want || body.to_lowercase().contains("locked") {
                explain(
                    ingest,
                    &format!(
                        "{} {}: wanted {}, got {status} after {retries} retries so far: {body:?}",
                        $method, $path, $want
                    ),
                );
            }
            body
        }};
    }
    while ingest.running() {
        let name = format!("w{requests}");
        let body = write!(
            201,
            "POST",
            "/api/workspaces",
            &format!("{{\"name\":\"{name}\"}}")
        );
        let id = body
            .split("\"id\":")
            .nth(1)
            .and_then(|r| r.split([',', '}']).next())
            .unwrap()
            .to_string();
        write!(
            200,
            "PATCH",
            &format!("/api/workspaces/{id}"),
            "{\"description\":\"renamed while ingesting\"}"
        );
        if requests % 2 == 0 {
            write!(
                200,
                "DELETE",
                &format!("/api/workspaces/{id}"),
                "{\"confirm\":true}"
            );
        } else {
            kept.push(name);
        }
        requests += 1;
    }
    let out = ingest.wait(Duration::from_secs(120));
    // the exit status and both streams are judged before stdout is parsed: a failed child has no JSON to parse
    assert_eq!(
        out.code,
        Some(0),
        "the ingest child failed: stdout {:?}, stderr {:?}",
        out.stdout,
        out.stderr
    );
    println!(
        "writes while ingesting: {requests} rounds of 2-3 requests, {retries} designed 503 answers retried, ingest stored {} new events (exit {:?})",
        out.new_events(),
        out.code
    );
    assert!(
        requests >= 2,
        "the ingest ended before two rounds of writes ran: {requests}; ingest stdout {:?}, stderr {:?}",
        out.stdout,
        out.stderr
    );
    assert!(
        out.new_events() > 0,
        "the ingest stored nothing: stdout {:?}, stderr {:?}",
        out.stdout,
        out.stderr
    );
    // the last round may have landed after the ingest ended: one more write proves the store is free
    let (status, body) = api.send_retrying(
        "POST",
        "/api/workspaces",
        "{\"name\":\"after\"}",
        &mut retries,
    );
    assert_eq!(status, 201, "{body}");
    kept.push("after".into());

    let conn = Connection::open(case.store_path()).unwrap();
    let check: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(check, "ok");
    let mut names: Vec<String> = conn
        .prepare("SELECT name FROM workspaces WHERE is_orphan = 0 ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    kept.sort();
    names.sort();
    assert_eq!(names, kept);
    let events: i64 = conn
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap();
    assert!(events as u64 >= out.new_events() as u64);
    serve.kill();
}
