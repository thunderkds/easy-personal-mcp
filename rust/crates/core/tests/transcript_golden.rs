//! T115 (FR-123) goldens: a transcript of every record kind of claude 2.1.327 (text replaced by `<...>`, probes of
//! `tasks/evidence-T110/SPIKE.md`) goes through the parser and must give `session.golden.json` exactly. Set
//! `BLESS_TRANSCRIPT=1` to write the golden after an intended change, and read the diff.

use easy_personal_core::transcript::{Window, parse};
use serde_json::{Value, json};
use std::io::Cursor;
use std::path::PathBuf;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/transcript_golden")
}

fn answer(window: Window) -> Value {
    let bytes = std::fs::read(dir().join("session.jsonl")).unwrap();
    let page = parse(&mut Cursor::new(bytes), window, false).unwrap();
    json!({"entries": page.entries, "next": page.next, "start": page.start, "complete": page.complete})
}

#[test]
fn every_record_kind_gives_the_golden_entries() {
    let got = answer(Window::Before(u64::MAX));
    let path = dir().join("session.golden.json");
    if std::env::var_os("BLESS_TRANSCRIPT").is_some() {
        std::fs::write(&path, serde_json::to_string_pretty(&got).unwrap() + "\n").unwrap();
    }
    let want: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(got, want);
}

#[test]
fn the_first_view_is_the_whole_file_and_a_later_poll_gets_orphans() {
    let whole = answer(Window::After(0));
    assert_eq!(whole, answer(Window::Before(u64::MAX)));
    let kinds: Vec<&str> = whole["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["type"].as_str().unwrap())
        .collect();
    for kind in ["user", "reply", "assistant", "tool", "skipped"] {
        assert!(kinds.contains(&kind), "{kind} missing in {kinds:?}");
    }
    // the poll that starts after the first call sees its result as an orphan with the same id
    let calls = whole["entries"].as_array().unwrap();
    let call = calls.iter().find(|e| e["id"] == "toolu_01").unwrap();
    assert_eq!(call["result"], "<output>\nline two");
    let from = calls.iter().find(|e| e["id"] == "toolu_02").unwrap()["offset"]
        .as_u64()
        .unwrap();
    let late = answer(Window::After(from));
    let orphan = late["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == "toolu_01");
    let orphan = orphan.expect("the result of a call before the offset comes as an orphan");
    assert_eq!(
        (&orphan["orphan"], &orphan["name"], &orphan["result"]),
        (&json!(true), &Value::Null, &json!("<output>\nline two"))
    );
}

fn agent_answer(window: Window, sidechain: bool) -> Value {
    let bytes = std::fs::read(dir().join("agent.jsonl")).unwrap();
    let page = parse(&mut Cursor::new(bytes), window, sidechain).unwrap();
    json!({"entries": page.entries, "next": page.next, "start": page.start, "complete": page.complete})
}

/// FR-129: a sub-agent's own file (every record `isSidechain: true`, an `agentId`, a nested `Agent` call) read with the
/// flag gives its entries; the same bytes read as a main file give nothing but one skipped run.
#[test]
fn a_sub_agent_file_gives_the_golden_entries_and_the_main_flag_still_drops_its_records() {
    let got = agent_answer(Window::Before(u64::MAX), true);
    let path = dir().join("agent.golden.json");
    if std::env::var_os("BLESS_TRANSCRIPT").is_some() {
        std::fs::write(&path, serde_json::to_string_pretty(&got).unwrap() + "\n").unwrap();
    }
    let want: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(got, want);
    assert_eq!(agent_answer(Window::After(0), true), got);
    let nested = got["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == "toolu_a02")
        .unwrap();
    assert_eq!(
        (&nested["name"], &nested["result"], &nested["is_error"]),
        (&json!("Agent"), &json!("<nested answer>"), &json!(false))
    );
    let dropped = agent_answer(Window::Before(u64::MAX), false);
    let kinds: Vec<&str> = dropped["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["type"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["skipped"]);
    assert_eq!(dropped["entries"][0]["count"], 8);
}
