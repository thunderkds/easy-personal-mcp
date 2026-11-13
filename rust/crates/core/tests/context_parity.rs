//! T051: the `context` source against the Python oracle (`rust/parity/golden-context/golden.json`, made once by
//! `rust/parity/gen_context.py` from Python's `sources/context.py`). Each case has the projects a store knows, a script
//! for a fake of easy-context's MCP service and a URL. The fake here is Rust and follows the script as the Python fake
//! does; the golden keeps what Python stored in `context_index`, the tools and root headers the fake saw, and the
//! sessions it was asked to end. Rust must store the same rows and the fake must see the same.
//!
//! Every service is a thread on a free `127.0.0.1` port; nothing else is connected to. The temp folders are removed.

use easy_personal_core::ingest::context::{self, CallError};
use easy_personal_core::writer::open_for_write;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::ffi::OsStr;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

mod context_fake;
use context_fake::Fake;

const ROOT: &str = "{ROOT}";

fn golden() -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/golden-context/golden.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn temp_dir() -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "easy-personal-rs-t051-context-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn free_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.local_addr().unwrap().port()
}

/// `value` with `{ROOT}`, `{PORT}` and `{DEAD}` replaced, in every string of it.
fn expand(value: &Value, work: &Path, port: Option<u16>, dead: u16) -> Value {
    match value {
        Value::String(s) => Value::String(
            s.replace(ROOT, work.to_str().unwrap())
                .replace("{PORT}", &port.map_or(String::new(), |p| p.to_string()))
                .replace("{DEAD}", &dead.to_string()),
        ),
        Value::Array(items) => {
            Value::Array(items.iter().map(|v| expand(v, work, port, dead)).collect())
        }
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), expand(v, work, port, dead)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn rows(path: &Path) -> Vec<Value> {
    let conn = Connection::open(path).unwrap();
    let mut query = conn
        .prepare("SELECT project_path, last_index_at_utc FROM context_index ORDER BY 1")
        .unwrap();
    query
        .query_map([], |r| {
            Ok(json!([
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?
            ]))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn run_case(spec: &Value) {
    let name = spec["name"].as_str().unwrap();
    let work = temp_dir();
    let store_file = work.join("store.sqlite");
    drop(open_for_write(&store_file).unwrap());
    let projects = expand(&spec["projects"], &work, None, 0);
    {
        let conn = Connection::open(&store_file).unwrap();
        for (i, project) in projects.as_array().unwrap().iter().enumerate() {
            let path = project["path"].as_str().unwrap();
            if project["dir"] == true {
                std::fs::create_dir_all(path).unwrap();
            }
            conn.execute(
                "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, project_path, ts_utc) \
                 VALUES ('m', 'claude_code', 'p', 0, ?1, 0, ?2, '2026-10-01T00:00:00+00:00')",
                rusqlite::params![i as i64 + 1, path],
            )
            .unwrap();
        }
        for seed in spec["seed_rows"].as_array().unwrap() {
            let seed = expand(seed, &work, None, 0);
            conn.execute(
                "INSERT INTO context_index VALUES (?1, ?2)",
                rusqlite::params![seed[0].as_str().unwrap(), seed[1].as_str().unwrap()],
            )
            .unwrap();
        }
    }
    let fake =
        (!spec["server"].is_null()).then(|| Fake::start(expand(&spec["server"], &work, None, 0)));
    let url = expand(
        &spec["url"],
        &work,
        fake.as_ref().map(|f| f.port),
        free_port(),
    );
    let timeout = spec["timeout_s"]
        .as_f64()
        .map_or(context::TIMEOUT, Duration::from_secs_f64);
    let mut store = open_for_write(&store_file).unwrap();
    let began = Instant::now();
    let summary =
        context::ingest_with(&mut store, Some(OsStr::new(url.as_str().unwrap())), timeout).unwrap();
    // A slow service is a bounded wait (Python's test: under 2.5 s with a 0.3 s timeout).
    assert!(
        began.elapsed() < Duration::from_millis(2500),
        "{name}: took {:?}",
        began.elapsed()
    );
    drop(store);

    let want = &spec["result"];
    assert_eq!(
        summary.status,
        want["status"].as_str().unwrap(),
        "{name}: status"
    );
    if want["status"] == "ok" {
        assert_eq!(
            summary.indexes,
            want["indexes"].as_i64().unwrap(),
            "{name}: indexes"
        );
    }
    let got_rows = rows(&store_file);
    assert_eq!(
        Value::Array(got_rows),
        expand(&spec["rows"], &work, None, 0),
        "{name}: context_index rows"
    );
    if let Some(fake) = fake {
        // The golden has {ROOT} in the roots; the fake saw real paths.
        let seen = fake.seen.lock().unwrap();
        assert_eq!(
            json!(seen.calls),
            spec["calls"],
            "{name}: tools the service saw"
        );
        assert_eq!(
            json!(seen.roots),
            expand(&spec["roots"], &work, None, 0),
            "{name}: root headers"
        );
        assert_eq!(
            seen.deletes as u64,
            spec["deletes"].as_u64().unwrap(),
            "{name}: ended sessions"
        );
    }
    std::fs::remove_dir_all(&work).unwrap();
}

#[test]
fn every_oracle_case_stores_the_rows_python_stored() {
    let golden = golden();
    let cases = golden["cases"].as_array().unwrap();
    assert!(cases.len() >= 40, "the golden has {} cases", cases.len());
    for spec in cases {
        run_case(spec);
    }
}

#[test]
fn the_allow_list_matches_the_oracle() {
    let golden = golden();
    for spec in golden["tool_cases"].as_array().unwrap() {
        let name = spec["name"].as_str().unwrap();
        let fake = Fake::start(
            json!({"mode": "json", "session": false, "reply": "structured", "reject_roots": [],
            "delay_s": 0, "break": null, "indexes": []}),
        );
        let url = format!("http://127.0.0.1:{}/mcp", fake.port);
        let got = match context::call_tool(
            spec["tool"].as_str().unwrap(),
            Some("/x"),
            Some(OsStr::new(&url)),
            context::TIMEOUT,
        ) {
            Ok(_) => "ok",
            Err(CallError::Refused) => "Refused",
            Err(CallError::Tool) => "ToolError",
            Err(CallError::Unavailable) => "Unavailable",
        };
        assert_eq!(got, spec["outcome"].as_str().unwrap(), "{name}");
        let seen = fake.seen.lock().unwrap();
        assert_eq!(json!(seen.calls), spec["calls"], "{name}: calls");
        assert_eq!(json!(seen.roots), spec["roots"], "{name}: roots");
    }
}

/// Rust-only rules (Python has no cap and raises where Rust says "no data"): one more run of the same fake.
#[test]
fn a_reply_over_one_mebibyte_is_no_data_and_a_number_out_of_range_keeps_a_row_without_a_time() {
    let work = temp_dir();
    let store_file = work.join("store.sqlite");
    drop(open_for_write(&store_file).unwrap());
    let project = work.join("p");
    std::fs::create_dir_all(&project).unwrap();
    {
        let conn = Connection::open(&store_file).unwrap();
        conn.execute(
            "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, project_path, ts_utc) \
             VALUES ('m', 'claude_code', 'p', 0, 1, 0, ?1, '2026-10-01T00:00:00+00:00')",
            [project.to_str().unwrap()],
        )
        .unwrap();
    }
    let big = |filler: usize| {
        let mut entry = json!({"root": project.to_str().unwrap(), "root_exists": true, "last_index_at": 1_000_000_000_000_i64});
        entry["pad"] = Value::String("x".repeat(filler));
        Fake::start(
            json!({"mode": "json", "session": false, "reply": "structured", "reject_roots": [],
            "delay_s": 0, "break": null, "indexes": [entry]}),
        )
    };
    // A year-31690 number: Python raises (the run fails); here the project keeps a row with no time.
    let fake = big(10);
    let mut store = open_for_write(&store_file).unwrap();
    let url = format!("http://127.0.0.1:{}/mcp", fake.port);
    let summary = context::ingest(&mut store, Some(OsStr::new(&url))).unwrap();
    assert_eq!((summary.status, summary.indexes), ("ok", 1));
    assert_eq!(
        rows(&store_file),
        vec![json!([project.to_str().unwrap(), null])]
    );
    // The same reply grown past 1 MiB: no data, the old row is cleared.
    let fake = big(context::MAX_REPLY);
    let url = format!("http://127.0.0.1:{}/mcp", fake.port);
    let summary = context::ingest(&mut store, Some(OsStr::new(&url))).unwrap();
    assert_eq!(summary.status, "no data");
    assert!(rows(&store_file).is_empty());
    std::fs::remove_dir_all(&work).unwrap();
}

/// A reply that never ends (a header, then one byte every 50 ms for ever) is bounded by the whole-call limit (twice the
/// timeout), and the client closes its socket: the fake sees the connection end.
#[test]
fn a_reply_that_never_ends_is_no_data_within_the_call_limit_and_the_socket_is_closed() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let closed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let seen_closed = std::sync::Arc::clone(&closed);
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf);
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n");
        loop {
            std::thread::sleep(Duration::from_millis(50));
            if stream.write_all(b":").is_err() {
                seen_closed.store(true, std::sync::atomic::Ordering::SeqCst);
                return;
            }
        }
    });
    let began = Instant::now();
    let url = format!("http://127.0.0.1:{port}/mcp");
    let got = context::call_tool(
        "list_indexes",
        Some("/x"),
        Some(OsStr::new(&url)),
        Duration::from_millis(300),
    );
    assert_eq!(got, Err(CallError::Unavailable));
    assert!(
        began.elapsed() < Duration::from_millis(1500),
        "took {:?}",
        began.elapsed()
    );
    let until = Instant::now() + Duration::from_secs(3);
    while !closed.load(std::sync::atomic::Ordering::SeqCst) && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        closed.load(std::sync::atomic::Ordering::SeqCst),
        "the client left its socket open"
    );
}

/// Python's header encoder raises on a non-ASCII root and the whole ingest is "no data" (the oracle's
/// `root_with_non_ascii_is_no_data`, one project). Rust skips that root only and tries the next one.
#[test]
fn a_root_that_is_not_a_header_value_is_skipped_and_the_next_root_is_tried() {
    let work = temp_dir();
    let store_file = work.join("store.sqlite");
    drop(open_for_write(&store_file).unwrap());
    let (odd, plain) = (work.join("p-0\u{e9}"), work.join("p-a"));
    {
        let conn = Connection::open(&store_file).unwrap();
        for (i, dir) in [&odd, &plain].into_iter().enumerate() {
            std::fs::create_dir_all(dir).unwrap();
            conn.execute(
                "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, project_path, ts_utc) \
                 VALUES ('m', 'claude_code', 'p', 0, ?1, 0, ?2, '2026-10-01T00:00:00+00:00')",
                rusqlite::params![i as i64 + 1, dir.to_str().unwrap()],
            )
            .unwrap();
        }
    }
    let indexes =
        json!([{"root": plain, "root_exists": true, "last_index_at": "2026-10-21T10:00:00+00:00"}]);
    let fake = Fake::start(
        json!({"mode": "json", "session": false, "reply": "structured", "reject_roots": [],
        "delay_s": 0, "break": null, "indexes": indexes}),
    );
    let mut store = open_for_write(&store_file).unwrap();
    let url = format!("http://127.0.0.1:{}/mcp", fake.port);
    let summary = context::ingest(&mut store, Some(OsStr::new(&url))).unwrap();
    assert_eq!((summary.status, summary.indexes), ("ok", 1));
    assert_eq!(
        fake.seen.lock().unwrap().roots,
        vec![Some(plain.to_str().unwrap().to_string())]
    );
    std::fs::remove_dir_all(&work).unwrap();
}
