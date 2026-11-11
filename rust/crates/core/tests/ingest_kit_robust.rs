//! T033: the inputs where Python crashes or hangs and Rust does not (criterion 5, each one a test), the rollback of the
//! `verifier` replace, and the fractional score read from a file with a BOM. The goldens cannot hold the crash cases.

mod ingest_kit_common;

use ingest_kit_common::{Kit, common::Case};
use rusqlite::Connection;
use serde_json::json;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const TRACE: &str = "<WORK>/proj/memory/event-trace";

fn good(ts: &str) -> String {
    format!(r#"{{"timestamp":"{ts}","tool_name":"Bash","is_error":false,"summary":"%CANARY%"}}"#)
}

fn write(kit: &Kit, path: &str, text: &str) {
    kit.step(&json!({"op": "write", "path": path, "text": text}));
}

fn ingest(kit: &Kit) -> ingest_kit_common::Printed {
    kit.ingest(&json!({"op": "ingest", "env": {"EASY_PERSONAL_UI_REPORTS": "<UI>"}}))
}

fn rows(kit: &Kit, table: &str) -> Vec<serde_json::Value> {
    kit.dump()[table].as_array().unwrap().clone()
}

/// Python: `RecursionError` (a line nested past ~57,900 levels) and `OverflowError` (a UTC time before year 1) are not
/// `ValueError`, so the ingest crashes and every later run crashes on the same line. Rust stores an abstained row.
#[test]
fn a_deep_line_and_an_out_of_range_time_are_abstained_rows_not_a_crash() {
    let kit = Kit::new();
    kit.step(&json!({"op": "transcript", "cwds": ["<WORK>/proj"]}));
    kit.step(&json!({"op": "mkdir", "path": "<WORK>/proj"}));
    let deep = format!("{}{}", "[".repeat(60_000), "]".repeat(60_000));
    let text = format!(
        "{deep}\n{}\n{}\n",
        good("0001-01-01T00:00:00+01:00"),
        good("2026-01-10T15:00:00Z")
    );
    write(&kit, &format!("{TRACE}/T001.jsonl"), &text);
    let printed = ingest(&kit);
    assert_eq!(printed.kit_trace, json!({"files": 1, "new_events": 3}));
    let rows = rows(&kit, "kit_trace_events");
    let abstained: Vec<_> = rows.iter().filter(|r| r["abstained"] == 1).collect();
    assert_eq!(abstained.len(), 2);
    assert!(
        abstained
            .iter()
            .all(|r| r["ts_utc"].is_null() && r["tool_name"].is_null() && r["is_error"].is_null())
    );
    assert_eq!(
        ingest(&kit).kit_trace["new_events"],
        0,
        "the second run is not stuck and adds nothing"
    );
}

/// Python reads a FIFO and blocks for ever; Rust skips every entry that is not a regular file and counts it.
#[test]
fn a_fifo_is_skipped_and_never_blocks_the_run() {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let kit = Kit::new();
        kit.step(&json!({"op": "transcript", "cwds": ["<WORK>/proj"]}));
        kit.step(&json!({"op": "mkfifo", "path": format!("{TRACE}/x.jsonl")}));
        write(
            &kit,
            &format!("{TRACE}/y.jsonl"),
            &(good("2026-01-10T15:00:00Z") + "\n"),
        );
        kit.step(&json!({"op": "mkdir", "path": "<WORK>/proj"}));
        kit.step(&json!({"op": "mkfifo", "path": "<UI>/session-f.json"}));
        let printed = ingest(&kit);
        tx.send((printed.kit_trace, printed.kit_skipped, printed.ui))
            .unwrap();
    });
    let (kit_trace, skipped, ui) = rx
        .recv_timeout(Duration::from_secs(30))
        .expect("the ingest blocked on a FIFO");
    assert_eq!(kit_trace, json!({"files": 1, "new_events": 1}));
    assert_eq!(skipped, 1);
    assert_eq!(ui, json!({"status": "ok", "sessions": 0, "skipped": 1}));
}

/// Python: `OverflowError` from `_utc` is not caught in `ui.parse_session`, so the ingest crashes on the file.
#[test]
fn a_ui_session_with_an_out_of_range_time_is_skipped() {
    let kit = Kit::new();
    write(
        &kit,
        "<UI>/session-a.json",
        r#"{"id":"a","status":"passed","startedAt":"0001-01-01T00:00:00+01:00","actions":[]}"#,
    );
    write(
        &kit,
        "<UI>/session-b.json",
        r#"{"id":"b","status":"passed","startedAt":"2026-01-10T15:00:00Z","actions":[]}"#,
    );
    assert_eq!(
        ingest(&kit).ui,
        json!({"status": "ok", "sessions": 1, "skipped": 1})
    );
}

/// A fractional score is read from the file's own bytes; a UTF-8 BOM does not hide it.
#[test]
fn a_fractional_score_in_a_file_with_a_bom_is_read() {
    let kit = Kit::new();
    kit.step(&json!({"op": "transcript", "cwds": ["<WORK>/proj"]}));
    let report = "\u{feff}".to_owned()
        + r#"{"schema_version":1,"scope":"changes","generated_at":"2026-01-10T15:00:00Z","overall":{"abstained":false,"value":86.5,"reason_code":null}}"#;
    write(
        &kit,
        "<WORK>/proj/reports/evidence-report-20260110T150000-1Z.json",
        &report,
    );
    assert_eq!(
        ingest(&kit).verifier,
        json!({"status": "ok", "projects": 1, "skipped": 0})
    );
    assert_eq!(
        rows(&kit, "verifier_scores")[0]["overall_value"],
        json!(86.5)
    );
}

/// A failure while the table is replaced rolls back: the old rows stay, the answer is `failed` + the error kind.
#[test]
fn a_failed_verifier_replace_leaves_the_old_rows() {
    let kit = Kit::new();
    kit.step(&json!({"op": "transcript", "cwds": ["<WORK>/proj"]}));
    write(
        &kit,
        "<WORK>/proj/reports/evidence-report-20260110T150000-1Z.json",
        r#"{"schema_version":1,"scope":"changes","generated_at":"2026-01-10T15:00:00Z","overall":{"abstained":false,"value":50,"reason_code":null}}"#,
    );
    ingest(&kit);
    let before = rows(&kit, "verifier_scores");
    assert_eq!(before.len(), 1);
    Connection::open(kit.case.store_path())
        .unwrap()
        .execute_batch("CREATE TRIGGER stop BEFORE INSERT ON verifier_scores BEGIN SELECT RAISE(ABORT, 'no'); END;")
        .unwrap();
    let mut store = easy_personal_core::writer::open_for_write(&kit.case.store_path()).unwrap();
    let reason = easy_personal_core::ingest::verifier::ingest(
        &mut store,
        Some(&kit.case.root.join("verifier-none")),
    )
    .expect_err("the insert is refused");
    assert_eq!(reason, "IntegrityError");
    drop(store);
    assert_eq!(
        rows(&kit, "verifier_scores"),
        before,
        "the old rows are still there"
    );
    // no folder can be worked out (no HOME): Python's RuntimeError, nothing deleted
    let mut store = easy_personal_core::writer::open_for_write(&kit.case.store_path()).unwrap();
    assert_eq!(
        easy_personal_core::ingest::verifier::ingest(&mut store, None).unwrap_err(),
        "RuntimeError"
    );
}

#[test]
fn the_case_helper_is_short_enough() {
    let _ = Case::new();
}
