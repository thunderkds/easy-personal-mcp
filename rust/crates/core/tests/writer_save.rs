//! T027: `Store::save_file` (one transaction, Python's upsert and `added` count, entrypoint back-fill), the rollback,
//! foreign keys, `retire_files` and `prune`. Every case runs on a fresh temp store and, where it says so, on a temp copy
//! of `rust/parity/store-v11.sqlite`; `~/.easy-personal` is never opened.

use easy_personal_core::writer::{Event, Line, Signal, WriteError, open_for_write};
use rusqlite::Connection;
use rusqlite::types::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

const SRC: &str = "claude_code";

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t027-save-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn store(&self) -> PathBuf {
        self.0.join("store.sqlite")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The two kinds of store every table case runs on.
fn stores() -> Vec<(&'static str, TempDir)> {
    let fresh = TempDir::new();
    let copy = TempDir::new();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/store-v11.sqlite"),
        copy.store(),
    )
    .unwrap();
    vec![("fresh store", fresh), ("store-v11 copy", copy)]
}

/// A test-only `AFTER UPDATE` counter on `events`: how many rows a save rewrote (the upsert's `WHERE` keeps it 0
/// for a batch that changes nothing).
fn add_rewrite_counter(path: &Path) {
    drop(open_for_write(path).unwrap()); // a fresh store must exist first
    Connection::open(path)
        .unwrap()
        .execute_batch(
            "CREATE TABLE t027_rewrites (n INTEGER);
             CREATE TRIGGER t027_count AFTER UPDATE ON events BEGIN INSERT INTO t027_rewrites VALUES (1); END;",
        )
        .unwrap();
}

fn rewrites(path: &Path) -> i64 {
    let c = Connection::open(path).unwrap();
    let n: i64 = c
        .query_row("SELECT COUNT(*) FROM t027_rewrites", [], |r| r.get(0))
        .unwrap();
    c.execute("DELETE FROM t027_rewrites", []).unwrap();
    n
}

fn ev(session: Option<&str>, entry: Option<&str>, errors: Option<i64>) -> Event {
    Event {
        abstained: false,
        record_type: Some("user".into()),
        version: Some("2.1.300".into()),
        ts_utc: Some("2026-01-15T10:00:00.000000Z".into()),
        session_id: session.map(str::to_owned),
        project_path: Some("/work/invented".into()),
        git_branch: Some("main".into()),
        is_sidechain: Some(false),
        is_api_error: None,
        entrypoint: entry.map(str::to_owned),
        tool_error_count: errors,
        model: None,
        message_id: None,
        tool_names: None,
        input_tokens: None,
        output_tokens: None,
        cache_creation_input_tokens: None,
        cache_read_input_tokens: None,
        signals: Vec::new(),
    }
}

/// Lines 100 bytes apart starting at `start`; `None` is a line with no event (a blank line).
fn lines(start: i64, events: Vec<Option<Event>>) -> Vec<Line> {
    events
        .into_iter()
        .enumerate()
        .map(|(i, event)| Line {
            line_offset: start + 100 * i as i64,
            next_offset: start + 100 * (i as i64 + 1),
            event,
        })
        .collect()
}

/// Three events, one with two tool signals, then a blank line.
fn batch(entry: Option<&str>, errors: Option<i64>) -> Vec<Line> {
    let mut with_signals = ev(Some("s-1"), entry, errors);
    with_signals.signals = vec![
        Signal {
            kind: "bash_command".into(),
            value: "git".into(),
        },
        Signal {
            kind: "web_host".into(),
            value: "example.invalid".into(),
        },
    ];
    lines(
        0,
        vec![
            Some(ev(Some("s-1"), entry, None)),
            Some(with_signals),
            Some(ev(Some("s-1"), entry, errors)),
            None,
        ],
    )
}

fn events_of(path: &Path, source_path: &str) -> Vec<Vec<Value>> {
    let c = Connection::open(path).unwrap();
    c.prepare(
        "SELECT source, source_path, inode, byte_offset, agent_account_id, abstained, record_type, version, ts_utc, \
         session_id, project_path, git_branch, is_sidechain, is_api_error, entrypoint, tool_error_count, model, \
         message_id, tool_names, input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens, \
         machine_id = (SELECT value FROM meta WHERE key = 'machine_id') \
         FROM events WHERE source_path = ? ORDER BY inode, byte_offset",
    )
    .unwrap()
    .query_map([source_path], |r| (0..24).map(|i| r.get(i)).collect())
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

fn column(path: &Path, source_path: &str, col: &str) -> Vec<Value> {
    let c = Connection::open(path).unwrap();
    c.prepare(&format!(
        "SELECT {col} FROM events WHERE source_path = ? ORDER BY inode, byte_offset"
    ))
    .unwrap()
    .query_map([source_path], |r| r.get(0))
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

fn offsets(path: &Path) -> Vec<(String, String, i64, i64)> {
    let c = Connection::open(path).unwrap();
    c.prepare("SELECT source, path, inode, byte_offset FROM source_files ORDER BY source, path")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn count(path: &Path, sql: &str) -> i64 {
    Connection::open(path)
        .unwrap()
        .query_row(sql, [], |r| r.get(0))
        .unwrap()
}

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

/// Criterion 8: the same batch twice adds nothing and rewrites nothing; rows, signals and offsets are equal.
#[test]
fn the_same_batch_twice_adds_and_rewrites_nothing() {
    for (case, dir) in stores() {
        let p = dir.store();
        add_rewrite_counter(&p);
        let mut store = open_for_write(&p).unwrap();
        let file = "t027/same.jsonl";
        let signals_before = count(&p, "SELECT COUNT(*) FROM tool_signals");
        assert_eq!(
            store
                .save_file(SRC, file, 7, 0, &batch(Some("cli"), Some(1)), None)
                .unwrap(),
            3,
            "{case}"
        );
        let (rows, sf, signals) = (
            events_of(&p, file),
            offsets(&p),
            count(&p, "SELECT COUNT(*) FROM tool_signals"),
        );
        assert_eq!(signals - signals_before, 2, "{case}");
        assert_eq!(rewrites(&p), 0, "{case}: first save");
        assert_eq!(
            store
                .save_file(SRC, file, 7, 0, &batch(Some("cli"), Some(1)), None)
                .unwrap(),
            0,
            "{case}"
        );
        assert_eq!(rewrites(&p), 0, "{case}: the second save rewrote rows");
        assert_eq!(events_of(&p, file), rows, "{case}");
        assert_eq!(offsets(&p), sf, "{case}");
        assert_eq!(
            count(&p, "SELECT COUNT(*) FROM tool_signals"),
            signals,
            "{case}"
        );
        assert!(
            sf.contains(&(SRC.into(), file.into(), 7, 400)),
            "{case}: {sf:?}"
        );
    }
}

/// Criterion 8: the fixture store's own rows, saved again from offset 0 (a re-read), add 0 rows and rewrite none.
#[test]
fn re_reading_the_fixture_files_adds_nothing() {
    let (_, dir) = stores().pop().unwrap();
    let p = dir.store();
    add_rewrite_counter(&p);
    let c = Connection::open(&p).unwrap();
    let files = offsets(&p);
    let mut store = open_for_write(&p).unwrap();
    let mut total = 0;
    for (source, file, inode, end) in &files {
        let mut stmt = c
            .prepare(
                "SELECT byte_offset, abstained, record_type, version, ts_utc, session_id, project_path, git_branch, \
                 is_sidechain, is_api_error, entrypoint, tool_error_count, model, message_id, tool_names, input_tokens, \
                 output_tokens, cache_creation_input_tokens, cache_read_input_tokens \
                 FROM events WHERE source_path = ? AND inode = ? ORDER BY byte_offset",
            )
            .unwrap();
        let mut batch: Vec<Line> = stmt
            .query_map(rusqlite::params![file, inode], |r| {
                Ok(Line {
                    line_offset: r.get(0)?,
                    next_offset: r.get(0)?,
                    event: Some(Event {
                        abstained: r.get(1)?,
                        record_type: r.get(2)?,
                        version: r.get(3)?,
                        ts_utc: r.get(4)?,
                        session_id: r.get(5)?,
                        project_path: r.get(6)?,
                        git_branch: r.get(7)?,
                        is_sidechain: r.get(8)?,
                        is_api_error: r.get(9)?,
                        entrypoint: r.get(10)?,
                        tool_error_count: r.get(11)?,
                        model: r.get(12)?,
                        message_id: r.get(13)?,
                        tool_names: r.get(14)?,
                        input_tokens: r.get(15)?,
                        output_tokens: r.get(16)?,
                        cache_creation_input_tokens: r.get(17)?,
                        cache_read_input_tokens: r.get(18)?,
                        signals: Vec::new(),
                    }),
                })
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        batch.push(Line {
            line_offset: *end,
            next_offset: *end,
            event: None,
        });
        total += store
            .save_file(source, file, *inode, 0, &batch, None)
            .unwrap();
    }
    assert_eq!(total, 0);
    assert_eq!(rewrites(&p), 0);
    assert_eq!(offsets(&p), files);
}

/// Criterion 8: a re-save with `entrypoint` / `tool_error_count` newly known fills them and adds no row.
#[test]
fn a_re_save_fills_newly_known_values() {
    for (case, dir) in stores() {
        let p = dir.store();
        add_rewrite_counter(&p);
        let mut store = open_for_write(&p).unwrap();
        let file = "t027/fill.jsonl";
        assert_eq!(
            store
                .save_file(SRC, file, 7, 0, &batch(None, None), None)
                .unwrap(),
            3,
            "{case}"
        );
        assert_eq!(
            column(&p, file, "entrypoint"),
            [Value::Null, Value::Null, Value::Null],
            "{case}"
        );
        rewrites(&p);
        assert_eq!(
            store
                .save_file(SRC, file, 7, 0, &batch(Some("sdk-cli"), Some(2)), None)
                .unwrap(),
            0,
            "{case}"
        );
        assert_eq!(
            column(&p, file, "entrypoint"),
            vec![text("sdk-cli"); 3],
            "{case}"
        );
        assert_eq!(
            column(&p, file, "tool_error_count"),
            [Value::Null, Value::Integer(2), Value::Integer(2)],
            "{case}"
        );
        assert_eq!(rewrites(&p), 3, "{case}: one rewrite per changed row");
        assert_eq!(
            count(
                &p,
                "SELECT COUNT(*) FROM events WHERE source_path = 't027/fill.jsonl'"
            ),
            3
        );
    }
}

/// Criterion 8: a re-save with NULL never erases a known value.
#[test]
fn a_re_save_with_null_never_erases() {
    for (case, dir) in stores() {
        let p = dir.store();
        add_rewrite_counter(&p);
        let mut store = open_for_write(&p).unwrap();
        let file = "t027/keep.jsonl";
        store
            .save_file(SRC, file, 7, 0, &batch(Some("cli"), Some(4)), None)
            .unwrap();
        let rows = events_of(&p, file);
        rewrites(&p);
        assert_eq!(
            store
                .save_file(SRC, file, 7, 0, &batch(None, None), None)
                .unwrap(),
            0,
            "{case}"
        );
        assert_eq!(events_of(&p, file), rows, "{case}");
        assert_eq!(rewrites(&p), 0, "{case}");
    }
}

/// Criterion 8: the inode is part of the key — the same path and offsets under a new inode are new rows.
#[test]
fn a_new_inode_makes_new_rows() {
    for (case, dir) in stores() {
        let p = dir.store();
        let mut store = open_for_write(&p).unwrap();
        let file = "t027/inode.jsonl";
        assert_eq!(
            store
                .save_file(SRC, file, 7, 0, &batch(Some("cli"), None), None)
                .unwrap(),
            3,
            "{case}"
        );
        assert_eq!(
            store
                .save_file(SRC, file, 8, 0, &batch(Some("cli"), None), None)
                .unwrap(),
            3,
            "{case}"
        );
        assert_eq!(events_of(&p, file).len(), 6, "{case}");
        assert!(
            offsets(&p).contains(&(SRC.into(), file.into(), 8, 400)),
            "{case}"
        );
    }
}

/// Criterion 7: the session-entrypoint back-fill of store.py:192-207, case by case.
#[test]
fn the_session_entrypoint_back_fill_matches_python() {
    for (case, dir) in stores() {
        let p = dir.store();
        let mut store = open_for_write(&p).unwrap();
        // In one batch: the first entrypoint seen fills the session's NULL rows; other sessions keep theirs.
        let a = "t027/backfill-a.jsonl";
        let first = lines(
            0,
            vec![
                Some(ev(Some("A"), None, None)),
                Some(ev(Some("A"), Some("cli"), None)),
                Some(ev(Some("A"), Some("sdk-cli"), None)),
                Some(ev(Some("A"), None, None)),
                Some(ev(Some("B"), None, None)),
                Some(ev(Some("C"), Some("sdk-ts"), None)),
                Some(ev(None, None, None)),
            ],
        );
        assert_eq!(store.save_file(SRC, a, 1, 0, &first, None).unwrap(), 7);
        assert_eq!(
            column(&p, a, "entrypoint"),
            [
                text("cli"),
                text("cli"),
                text("sdk-cli"),
                text("cli"),
                Value::Null,
                text("sdk-ts"),
                Value::Null
            ],
            "{case}"
        );
        // A later batch with no entrypoint takes one already stored for that session in this file and inode.
        let later = lines(
            700,
            vec![
                Some(ev(Some("A"), None, None)),
                Some(ev(Some("B"), None, None)),
            ],
        );
        assert_eq!(store.save_file(SRC, a, 1, 700, &later, None).unwrap(), 2);
        assert_eq!(
            column(&p, a, "entrypoint")[7..],
            [text("cli"), Value::Null],
            "{case}"
        );
        // A later batch with its own first entrypoint fills the earlier NULL rows of that session.
        let b_entry = lines(900, vec![Some(ev(Some("B"), Some("cli"), None))]);
        store.save_file(SRC, a, 1, 900, &b_entry, None).unwrap();
        let entries = column(&p, a, "entrypoint");
        assert_eq!(
            (&entries[4], &entries[8], &entries[9]),
            (&text("cli"), &text("cli"), &text("cli")),
            "{case}"
        );
        // Another file, or another inode of the same path, never lends its entrypoint.
        let other = "t027/backfill-b.jsonl";
        store
            .save_file(
                SRC,
                other,
                1,
                0,
                &lines(0, vec![Some(ev(Some("A"), None, None))]),
                None,
            )
            .unwrap();
        store
            .save_file(
                SRC,
                a,
                2,
                0,
                &lines(0, vec![Some(ev(Some("A"), None, None))]),
                None,
            )
            .unwrap();
        assert_eq!(column(&p, other, "entrypoint"), [Value::Null], "{case}");
        let by_inode = column(&p, a, "inode");
        let a_entries = column(&p, a, "entrypoint");
        let inode2: Vec<&Value> = a_entries
            .iter()
            .zip(&by_inode)
            .filter(|(_, i)| **i == Value::Integer(2))
            .map(|(e, _)| e)
            .collect();
        assert_eq!(inode2, [&Value::Null], "{case}");
    }
}

/// Edge case: the offset stored is what the caller passes (the last `next_offset`, else `offset`); an event-less line
/// advances it; values above `i32` round-trip.
#[test]
fn offsets_are_stored_as_given_including_large_values() {
    let dir = TempDir::new();
    let p = dir.store();
    let mut store = open_for_write(&p).unwrap();
    store
        .save_file(SRC, "t027/empty.jsonl", 3, 1234, &[], None)
        .unwrap();
    let blank = [Line {
        line_offset: 1234,
        next_offset: 1240,
        event: None,
    }];
    store
        .save_file(SRC, "t027/blank.jsonl", 3, 1234, &blank, None)
        .unwrap();
    let big = [Line {
        line_offset: 3_000_000_000,
        next_offset: 4_000_000_001,
        event: Some(ev(Some("s"), None, Some(5_000_000_000))),
    }];
    store
        .save_file(
            SRC,
            "t027/big.jsonl",
            5_000_000_000,
            3_000_000_000,
            &big,
            None,
        )
        .unwrap();
    assert_eq!(
        offsets(&p),
        [
            (
                SRC.into(),
                "t027/big.jsonl".into(),
                5_000_000_000,
                4_000_000_001
            ),
            (SRC.into(), "t027/blank.jsonl".into(), 3, 1240),
            (SRC.into(), "t027/empty.jsonl".into(), 3, 1234),
        ]
    );
    let row = &events_of(&p, "t027/big.jsonl")[0];
    assert_eq!(
        (&row[2], &row[3], &row[15]),
        (
            &Value::Integer(5_000_000_000),
            &Value::Integer(3_000_000_000),
            &Value::Integer(5_000_000_000)
        )
    );
}

/// Criterion 7: a failure on the last step (the `source_files` upsert) rolls back the events, the back-fill and the
/// signals of that save; the earlier offset stays.
#[test]
fn a_failure_on_the_offset_rolls_back_the_whole_file() {
    for (case, dir) in stores() {
        let p = dir.store();
        let mut store = open_for_write(&p).unwrap();
        let file = "t027/rollback.jsonl";
        store
            .save_file(
                SRC,
                file,
                7,
                0,
                &lines(0, vec![Some(ev(Some("A"), None, None))]),
                None,
            )
            .unwrap();
        let (events, signals, sf) = (
            count(&p, "SELECT COUNT(*) FROM events"),
            count(&p, "SELECT COUNT(*) FROM tool_signals"),
            offsets(&p),
        );
        Connection::open(&p)
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER t027_fail_insert BEFORE INSERT ON source_files BEGIN SELECT RAISE(ABORT, 'invented'); END;
                 CREATE TRIGGER t027_fail_update BEFORE UPDATE ON source_files BEGIN SELECT RAISE(ABORT, 'invented'); END;",
            )
            .unwrap();
        // A new event of session A with an entrypoint (it would back-fill row 0 of `file`) and two signals, then a blank.
        let mut fresh = ev(Some("A"), Some("cli"), Some(1));
        fresh.signals = vec![Signal {
            kind: "bash_command".into(),
            value: "git".into(),
        }];
        // The same file (the offset upsert's UPDATE path) and a new file (its INSERT path).
        for (target, start) in [(file, 100), ("t027/rollback-new.jsonl", 0)] {
            let failing = lines(start, vec![Some(fresh.clone()), None]);
            let err = store
                .save_file(SRC, target, 7, start, &failing, None)
                .unwrap_err();
            assert!(matches!(err, WriteError::Rejected), "{case}: {err:?}");
            assert!(!err.to_string().contains("invented"), "{case}: {err}");
            assert_eq!(
                count(&p, "SELECT COUNT(*) FROM events"),
                events,
                "{case} {target}: events kept"
            );
            assert_eq!(
                count(&p, "SELECT COUNT(*) FROM tool_signals"),
                signals,
                "{case} {target}: signals kept"
            );
            assert_eq!(offsets(&p), sf, "{case} {target}: offset changed");
            assert_eq!(
                column(&p, file, "entrypoint"),
                [Value::Null],
                "{case}: back-fill kept"
            );
        }
    }
}

/// Criterion 10: an unknown agent account is refused by the foreign key; nothing of that file is written.
#[test]
fn an_unknown_account_is_rejected_by_the_foreign_key() {
    for (case, dir) in stores() {
        let p = dir.store();
        let mut store = open_for_write(&p).unwrap();
        let before = (count(&p, "SELECT COUNT(*) FROM events"), offsets(&p));
        let err = store
            .save_file(SRC, "t027/fk.jsonl", 7, 0, &batch(None, None), Some(999))
            .unwrap_err();
        assert!(matches!(err, WriteError::Rejected), "{case}: {err:?}");
        assert_eq!(err.exit_code(), 1);
        assert!(
            !err.to_string().contains("999") && !err.to_string().contains("t027"),
            "{case}: {err}"
        );
        assert_eq!(
            (count(&p, "SELECT COUNT(*) FROM events"), offsets(&p)),
            before,
            "{case}"
        );

        let c = Connection::open(&p).unwrap();
        c.execute(
            "INSERT INTO agent_accounts (provider, folder, label, status) VALUES ('claude', '/invented/.claude-t027', 'T027', 'confirmed')",
            [],
        )
        .unwrap();
        let id = c.last_insert_rowid();
        assert_eq!(
            store
                .save_file(SRC, "t027/fk.jsonl", 7, 0, &batch(None, None), Some(id))
                .unwrap(),
            3
        );
        assert_eq!(
            column(&p, "t027/fk.jsonl", "agent_account_id"),
            vec![Value::Integer(id); 3],
            "{case}"
        );
    }
}

/// Edge case: a writer that finds the database locked waits the 5 s busy timeout, then a clean `Busy`, not a hang.
#[test]
fn a_locked_database_is_busy_after_the_wait() {
    let dir = TempDir::new();
    let p = dir.store();
    let mut store = open_for_write(&p).unwrap();
    let other = Connection::open(&p).unwrap();
    other.execute_batch("BEGIN IMMEDIATE").unwrap();
    let started = Instant::now();
    let err = store
        .save_file(SRC, "t027/busy.jsonl", 1, 0, &batch(None, None), None)
        .unwrap_err();
    let waited = started.elapsed();
    other.execute_batch("ROLLBACK").unwrap();
    assert!(matches!(err, WriteError::Busy), "{err:?}");
    assert!(
        waited >= Duration::from_millis(4500) && waited < Duration::from_secs(9),
        "{waited:?}"
    );
    assert_eq!(count(&p, "SELECT COUNT(*) FROM events"), 0);
}

/// Criterion 9: `retire_files` deletes only the named offset rows of that source; events stay.
#[test]
fn retire_files_forgets_only_the_named_offsets() {
    for (case, dir) in stores() {
        let p = dir.store();
        let mut store = open_for_write(&p).unwrap();
        for file in ["t027/a.jsonl", "t027/b.jsonl", "t027/c.jsonl"] {
            store
                .save_file(SRC, file, 1, 0, &batch(None, None), None)
                .unwrap();
        }
        store
            .save_file("other", "t027/a.jsonl", 1, 0, &[], None)
            .unwrap();
        let events = count(&p, "SELECT COUNT(*) FROM events");
        let before = offsets(&p);
        store
            .retire_files(
                SRC,
                &["t027/a.jsonl", "t027/b.jsonl", "t027/never-seen.jsonl"],
            )
            .unwrap();
        let after = offsets(&p);
        let gone: Vec<_> = before.iter().filter(|r| !after.contains(r)).collect();
        assert_eq!(
            gone,
            [
                &(SRC.into(), "t027/a.jsonl".into(), 1, 400),
                &(SRC.into(), "t027/b.jsonl".into(), 1, 400)
            ],
            "{case}"
        );
        assert!(
            after.contains(&("other".into(), "t027/a.jsonl".into(), 1, 0)),
            "{case}"
        );
        assert_eq!(
            count(&p, "SELECT COUNT(*) FROM events"),
            events,
            "{case}: events stay"
        );
        store.retire_files::<&str>(SRC, &[]).unwrap();
    }
}

fn micros(text: &str) -> i64 {
    text.parse::<jiff::Timestamp>().unwrap().as_microsecond()
}

/// Criterion 9: `prune` deletes timestamped rows before the cutoff from the three tables by **text** comparison, keeps
/// NULL timestamps and `source_files`, and returns the summed count.
#[test]
fn prune_compares_text_and_keeps_null_timestamps() {
    let dir = TempDir::new();
    let p = dir.store();
    let mut store = open_for_write(&p).unwrap();
    // now 2026-01-20T00:00:00Z, 10 days -> cutoff text "2026-01-10T00:00:00.000000Z".
    let cases: [(Option<&str>, bool); 8] = [
        (Some("2026-01-01T00:00:00.000000Z"), true),
        (Some("2026-01-09T23:59:59.999999Z"), true),
        (Some("2026-01-10T00:00:00.000000Z"), false), // equal: `<` keeps it
        (Some("2026-01-10T00:00:00.000001Z"), false),
        (None, false),
        // Not the stored form: compared as text exactly like Python. 'Z' > '0', so 3 digits sort after 6 digits;
        // '+' < 'Z', so "+00:00" sorts before "Z" at the same instant.
        (Some("2026-01-10T00:00:00.000Z"), false),
        (Some("2026-01-10T00:00:00.000000+00:00"), true),
        (Some("2025-12-31"), true),
    ];
    let batch: Vec<Line> = lines(
        0,
        cases
            .iter()
            .map(|(ts, _)| {
                let mut e = ev(Some("s"), None, None);
                e.ts_utc = ts.map(str::to_owned);
                e.signals = vec![Signal {
                    kind: "k".into(),
                    value: "v".into(),
                }];
                Some(e)
            })
            .collect(),
    );
    store
        .save_file(SRC, "t027/prune.jsonl", 1, 0, &batch, None)
        .unwrap();
    let c = Connection::open(&p).unwrap();
    for (i, (ts, _)) in cases.iter().enumerate() {
        c.execute(
            "INSERT INTO kit_trace_events (machine_id, source_path, inode, byte_offset, project_path, abstained, ts_utc) \
             VALUES ('m', 'trace', 1, ?, '/work/invented', 0, ?)",
            rusqlite::params![i as i64, ts],
        )
        .unwrap();
    }
    let sf = offsets(&p);
    assert_eq!(
        store.prune(None, micros("2026-01-20T00:00:00Z")).unwrap(),
        0
    );
    let deleted = store
        .prune(Some(10), micros("2026-01-20T00:00:00Z"))
        .unwrap();
    let expected_gone = cases.iter().filter(|(_, gone)| *gone).count() as i64;
    assert_eq!(deleted, 3 * expected_gone);
    let kept: Vec<Value> = cases
        .iter()
        .filter(|(_, gone)| !gone)
        .map(|(ts, _)| ts.map_or(Value::Null, text))
        .collect();
    assert_eq!(column(&p, "t027/prune.jsonl", "ts_utc"), kept);
    for table in ["kit_trace_events", "tool_signals"] {
        let left: Vec<Value> = c
            .prepare(&format!("SELECT ts_utc FROM {table} ORDER BY byte_offset"))
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(left, kept, "{table}");
    }
    assert_eq!(offsets(&p), sf, "source_files is never pruned");
    assert!(matches!(
        store.prune(Some(i64::MAX), 0),
        Err(WriteError::OutOfRange)
    ));
    assert!(matches!(
        store.prune(Some(800_000), 0),
        Err(WriteError::OutOfRange)
    ));
    assert!(matches!(
        store.prune(Some(-3_000_000), 0),
        Err(WriteError::OutOfRange)
    ));
}
