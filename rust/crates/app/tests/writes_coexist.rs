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
    let (mut requests, mut kept) = (0, Vec::new());
    while ingest.running() {
        let name = format!("w{requests}");
        let (status, body) = api.send(
            "POST",
            "/api/workspaces",
            &format!("{{\"name\":\"{name}\"}}"),
        );
        assert_eq!(status, 201, "{body}");
        let id = body
            .split("\"id\":")
            .nth(1)
            .and_then(|r| r.split([',', '}']).next())
            .unwrap()
            .to_string();
        let (status, body) = api.send(
            "PATCH",
            &format!("/api/workspaces/{id}"),
            "{\"description\":\"renamed while ingesting\"}",
        );
        assert_eq!(status, 200, "{body}");
        if requests % 2 == 0 {
            let (status, body) = api.send(
                "DELETE",
                &format!("/api/workspaces/{id}"),
                "{\"confirm\":true}",
            );
            assert_eq!(status, 200, "{body}");
        } else {
            kept.push(name);
        }
        assert!(!body.to_lowercase().contains("locked"), "{body}");
        requests += 1;
    }
    let out = ingest.wait(Duration::from_secs(120));
    println!(
        "writes while ingesting: {requests} rounds of 2-3 requests, ingest stored {} new events",
        out.new_events()
    );
    assert!(
        requests >= 2,
        "the ingest ended before two rounds of writes ran: {requests}"
    );
    assert!(out.new_events() > 0, "the ingest stored nothing");
    // the last round may have landed after the ingest ended: one more write proves the store is free
    let (status, body) = api.send("POST", "/api/workspaces", "{\"name\":\"after\"}");
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
