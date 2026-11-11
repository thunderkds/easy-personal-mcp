//! T028 criterion 11 (and 2): the parse layer replayed against T026's ingest oracle. Each fixture file is parsed with
//! `ingest::claude_code::read_lines` from the offset the golden run started at, the lines are saved with T027's
//! `Store::save_file` into a temp store (which adds the session-entrypoint back-fill, as Python's `save_file` does),
//! and the `events`, `tool_signals` and `source_files` rows must equal the golden rows on every column but those T035
//! fills: `project_path` (git resolution), `agent_account_id`, `inode` (`id` and `machine_id` are not in the goldens).
//! The four inputs on which Python crashes (S06a-d) are asserted as "Rust differs": the stated row, the lines after it
//! read, no panic.

use easy_personal_core::ingest::claude_code::read_lines;
use easy_personal_core::writer::{Event, Line, Store, open_for_write};
use rusqlite::Connection;
use rusqlite::types::Value as Sql;
use serde_json::{Map, Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

const NOT_COMPARED: [&str; 3] = ["project_path", "agent_account_id", "inode"];
const PROJECTS: &str = "<AGENT_ROOT>/.claude/projects/";

struct Temp {
    dir: PathBuf,
    store: Store,
}

impl Temp {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t028-replay-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        let store = open_for_write(&dir.join("store.sqlite")).unwrap();
        Temp { dir, store }
    }

    /// Parse `bytes` from the file's stored offset (0 when new) and save the lines, as T035 will.
    fn ingest(&mut self, path: &str, bytes: &[u8]) -> Vec<Line> {
        let offset = self.offset(path).unwrap_or(0);
        let lines = read_lines(bytes, offset as usize);
        self.store
            .save_file("claude_code", path, 1, offset, &lines, None)
            .unwrap();
        lines
    }

    fn conn(&self) -> Connection {
        Connection::open(self.dir.join("store.sqlite")).unwrap()
    }

    fn offset(&self, path: &str) -> Option<i64> {
        self.conn()
            .query_row(
                "SELECT byte_offset FROM source_files WHERE path = ?",
                [path],
                |r| r.get(0),
            )
            .ok()
    }

    /// The rows of `table` as JSON objects keyed like the golden, in the golden's order.
    fn rows(&self, table: &str, columns: &[String], path_col: &str) -> Vec<Map<String, Value>> {
        let conn = self.conn();
        let sql = format!(
            "SELECT {} FROM {table} ORDER BY {path_col}, byte_offset{}",
            columns.join(", "),
            if table == "tool_signals" {
                ", kind, value"
            } else {
                ""
            }
        );
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map([], |r| {
            let mut m = Map::new();
            for (i, c) in columns.iter().enumerate() {
                let v = match r.get::<_, Sql>(i)? {
                    Sql::Null => Value::Null,
                    Sql::Integer(n) => json!(n),
                    Sql::Text(t) => json!(t),
                    other => panic!("unexpected {other:?}"),
                };
                m.insert(c.clone(), v);
            }
            Ok(m)
        })
        .unwrap()
        .map(Result::unwrap)
        .collect()
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn golden(name: &str) -> Value {
    let path = repo().join("rust/parity/ingest/golden").join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

/// The 240-character work root of the oracle (`repos.json` `work_path_length`): only its length matters here, it
/// keeps every byte offset of `synthetic/projects/repos.jsonl` equal to the golden's.
fn padded_work() -> String {
    let n = golden_fixture_json("repos.json")["work_path_length"]
        .as_u64()
        .unwrap() as usize;
    format!("/{}", "w".repeat(n - 1))
}

fn golden_fixture_json(name: &str) -> Value {
    let path = repo().join("rust/parity/ingest/fixtures").join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

/// A fixture file's bytes as the oracle copied it (`%WORK%` replaced by the work root).
fn fixture(dir: &str, rel: &str) -> Vec<u8> {
    let bytes = fs::read(repo().join(dir).join(rel)).unwrap();
    let work = padded_work();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"%WORK%") {
            out.extend_from_slice(work.as_bytes());
            i += 6;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

fn columns(table: &Value) -> Vec<String> {
    table["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap().to_owned())
        .filter(|c| !NOT_COMPARED.contains(&c.as_str()))
        .collect()
}

/// Every row of the three tables equal to the golden run's (minus the T035 columns).
fn assert_tables(t: &Temp, run: &Value, scenario: &str) -> usize {
    let tables = &run["tables_after"];
    let mut compared = 0;
    for (table, path_col) in [
        ("events", "source_path"),
        ("tool_signals", "source_path"),
        ("source_files", "path"),
    ] {
        let cols = columns(&tables[table]);
        let want: Vec<Map<String, Value>> = tables[table]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                cols.iter()
                    .map(|c| (c.clone(), r[c].clone()))
                    .collect::<Map<String, Value>>()
            })
            .collect();
        let got = t.rows(table, &cols, path_col);
        assert_eq!(got.len(), want.len(), "{scenario} {table}: row count");
        for (g, w) in got.iter().zip(&want) {
            assert_eq!(g, w, "{scenario} {table}");
        }
        compared += got.len();
    }
    compared
}

fn golden_paths(run: &Value) -> Vec<String> {
    run["tables_after"]["source_files"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["path"].as_str().unwrap().to_owned())
        .collect()
}

/// A cold ingest of every file the golden stored, from the fixture folder `dir`.
fn replay_cold(scenario: &str, dir: &str) -> usize {
    let g = golden(scenario);
    let run = &g["runs"][0];
    let mut t = Temp::new();
    for path in golden_paths(run) {
        let rel = path.strip_prefix(PROJECTS).unwrap();
        t.ingest(&path, &fixture(dir, rel));
    }
    assert_tables(&t, run, scenario)
}

#[test]
fn s01_committed_fixtures_equal_the_golden() {
    let n = replay_cold("S01_cold_committed.json", "tests/fixtures/claude_code");
    println!("S01: {n} rows equal");
}

#[test]
fn s06_synthetic_set_equals_the_golden() {
    let n = replay_cold(
        "S06_cold_synthetic.json",
        "rust/parity/ingest/fixtures/synthetic",
    );
    println!("S06: {n} rows equal");
}

/// Criterion 2: every committed fixture ends with a truncated line; it is never consumed and the stored offset
/// stops at its start.
#[test]
fn the_truncated_last_line_is_never_consumed() {
    let g = golden("S01_cold_committed.json");
    for path in golden_paths(&g["runs"][0]) {
        let bytes = fixture(
            "tests/fixtures/claude_code",
            path.strip_prefix(PROJECTS).unwrap(),
        );
        assert_ne!(bytes.last(), Some(&b'\n'), "{path} should end mid-line");
        let lines = read_lines(&bytes, 0);
        let end = lines.last().unwrap().next_offset as usize;
        assert_eq!(bytes[end - 1], b'\n');
        assert!(!bytes[end..].contains(&b'\n'));
        assert!(
            lines
                .windows(2)
                .all(|w| w[0].next_offset == w[1].line_offset)
        );
    }
}

#[test]
fn s03_an_appended_line_is_read_from_the_saved_offset() {
    let g = golden("S03_line_appended.json");
    let path = format!("{PROJECTS}s3/session.jsonl");
    let first = fixture("rust/parity/ingest/fixtures", "s3/session.jsonl");
    let mut t = Temp::new();
    t.ingest(&path, &first);
    assert_tables(&t, &g["runs"][0], "S03 run 1");
    let mut both = first.clone();
    both.extend(fixture("rust/parity/ingest/fixtures", "s3/append.jsonl"));
    let lines = t.ingest(&path, &both);
    assert_eq!(lines.len(), 1, "only the new line");
    assert_eq!(lines[0].line_offset as usize, first.len());
    assert_tables(&t, &g["runs"][1], "S03 run 2");
}

#[test]
fn s05_a_truncated_line_completed_later_is_read_once() {
    let g = golden("S05_truncated_then_completed.json");
    let path = format!("{PROJECTS}s5/session.jsonl");
    let first = fixture("rust/parity/ingest/fixtures", "s5/session.jsonl");
    let mut t = Temp::new();
    t.ingest(&path, &first);
    assert_tables(&t, &g["runs"][0], "S05 run 1");
    let mut both = first.clone();
    both.extend(fixture("rust/parity/ingest/fixtures", "s5/rest.part"));
    let lines = t.ingest(&path, &both);
    assert_eq!(lines.len(), 1, "only the completed line");
    assert_tables(&t, &g["runs"][1], "S05 run 2");
}

/// One crash scenario: the first file (stored by Python before its crash) equals the golden; the crashing file is
/// parsed whole, its line 2 is `want`, lines 1 and 3 are normal rows.
fn rust_differs(scenario: &str, dir: &str, file: &str, want: Event) {
    let g = golden(scenario);
    let run = &g["runs"][0];
    assert_eq!(run["exit_code"], 1, "{scenario}: Python crashes");
    let mut t = Temp::new();
    for path in golden_paths(run) {
        t.ingest(&path, &fixture(dir, path.strip_prefix(PROJECTS).unwrap()));
    }
    assert_tables(&t, run, scenario);
    let lines = read_lines(&fixture(dir, file), 0);
    assert_eq!(lines.len(), 3, "{scenario}: every line read");
    let events: Vec<&Event> = lines.iter().map(|l| l.event.as_ref().unwrap()).collect();
    assert_eq!(events[1], &want, "{scenario}: the line Python crashes on");
    for e in [events[0], events[2]] {
        assert!(
            !e.abstained && e.record_type.as_deref() == Some("user"),
            "{scenario}: {e:?}"
        );
    }
}

fn kept(record_type: Option<&str>, session: &str, ts: &str) -> Event {
    Event {
        abstained: true,
        record_type: record_type.map(str::to_owned),
        version: Some("2.1.400".into()),
        ts_utc: Some(ts.into()),
        session_id: Some(session.into()),
        ..Event::default()
    }
}

/// Rust differs (S06a): 100,000 nested `[` is an all-NULL abstained row (Python: `RecursionError`, exit 1).
#[test]
fn s06a_deep_nesting_abstains_and_the_run_goes_on() {
    let all_null = Event {
        abstained: true,
        ..Event::default()
    };
    rust_differs(
        "S06a_crash_deep_nesting.json",
        "rust/parity/ingest/fixtures/crash-deep",
        "p/m-deep.jsonl",
        all_null,
    );
}

/// Rust differs (S06b): `input_tokens` 2**63 abstains the record with type/version/ts/session (Python: `OverflowError`).
#[test]
fn s06b_int_beyond_int64_abstains_and_the_run_goes_on() {
    let want = kept(
        Some("assistant"),
        "00000000-0000-4000-8000-000000001024",
        "2026-01-10T14:00:01.000000Z",
    );
    rust_differs(
        "S06b_crash_int_beyond_int64.json",
        "rust/parity/ingest/fixtures/crash-bigint",
        "m-bigint.jsonl",
        want,
    );
}

/// Rust differs (S06d): `"type": ["user"]` abstains with `record_type` NULL (Python: `TypeError`, unhashable).
#[test]
fn s06d_list_type_abstains_and_the_run_goes_on() {
    let want = kept(
        None,
        "00000000-0000-4000-8000-000000001027",
        "2026-01-10T14:00:01.000000Z",
    );
    rust_differs(
        "S06d_crash_unhashable_type.json",
        "rust/parity/ingest/fixtures/crash-type",
        "m-type.jsonl",
        want,
    );
}
