//! T035 criterion 11 (NFR-002): after `ingest_accounts` over a transcript holding a `CANARY-` text in every free-text
//! field, no canary is in the store file, its WAL or its SHM (a second connection stays open so the WAL is kept).

mod ingest_folders_common;

use ingest_folders_common::Case;
use rusqlite::Connection;
use std::fs;

const LINES: &[&str] = &[
    r#"{"type":"user","sessionId":"t035-p","timestamp":"2026-01-10T10:00:00Z","version":"2.1.400","cwd":"/nonexistent-t035/app","entrypoint":"cli","message":{"role":"user","content":"CANARY-prompt"},"toolUseResult":"CANARY-result","attachment":{"text":"CANARY-attachment"}}"#,
    r#"{"type":"user","sessionId":"t035-p","timestamp":"2026-01-10T10:00:01Z","version":"2.1.400","cwd":"/nonexistent-t035/app","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"CANARY-tooluse","is_error":true,"content":"CANARY-tool-output"},{"type":"text","text":"CANARY-user-text"}]}}"#,
    r#"{"type":"assistant","sessionId":"t035-p","timestamp":"2026-01-10T10:00:02Z","version":"2.1.400","cwd":"/nonexistent-t035/app","message":{"id":"msg_t035","model":"claude-fixture-model-1","usage":{"input_tokens":1,"output_tokens":2,"cache_creation_input_tokens":0,"cache_read_input_tokens":0},"content":[{"type":"text","text":"CANARY-answer"},{"type":"thinking","thinking":"CANARY-thinking"},{"type":"tool_use","name":"Bash","input":{"command":"ls CANARY-bash-argument"}},{"type":"tool_use","name":"WebFetch","input":{"url":"https://example.com/CANARY-path?q=CANARY-query","prompt":"CANARY-fetch-prompt"}},{"type":"tool_use","name":"Read","input":{"file_path":"/CANARY-file"}}]}}"#,
    r#"{"type":"last-prompt","sessionId":"t035-p","lastPrompt":"CANARY-last-prompt"}"#,
    r#"{"type":"custom-title","sessionId":"t035-p","customTitle":"CANARY-title"}"#,
    r#"{"type":"ai-title","sessionId":"t035-p","aiTitle":"CANARY-ai-title"}"#,
    r#"{"type":"summary","summary":"CANARY-summary","leafUuid":"CANARY-leaf"}"#,
    r#"{"type":"system","sessionId":"t035-p","timestamp":"2026-01-10T10:00:03Z","version":"2.1.400","cwd":"/nonexistent-t035/app","content":"CANARY-system"}"#,
    r#"not json CANARY-broken"#,
];

#[test]
fn no_canary_reaches_the_store_wal_or_shm() {
    let case = Case::new();
    let path = case.projects.join("p/session.jsonl");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, LINES.join("\n") + "\n").unwrap();
    fs::create_dir_all(&case.home).unwrap();
    drop(easy_personal_core::writer::open_for_write(&case.store_path()).unwrap());
    let keep = Connection::open(case.store_path()).unwrap(); // keeps the WAL file after the ingest's connection closes
    let summary = case.ingest(None);
    assert_eq!((summary.files, summary.new_events), (1, LINES.len() as i64));
    let signals: i64 = keep
        .query_row("SELECT COUNT(*) FROM tool_signals", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        signals, 2,
        "the Bash word and the WebFetch host are kept, nothing else"
    );
    let mut read = 0;
    let mut stored = false; // control: the stored session id is visible in these bytes, so a canary would be too
    for suffix in ["", "-wal", "-shm"] {
        let file = format!("{}{suffix}", case.store_path().to_str().unwrap());
        let Ok(bytes) = fs::read(&file) else { continue };
        read += 1;
        stored |= bytes.windows(6).any(|w| w == b"t035-p");
        assert!(
            !bytes.windows(7).any(|w| w == b"CANARY-"),
            "a canary in the store file{suffix}"
        );
    }
    assert_eq!(read, 3, "store, WAL and SHM all checked");
    assert!(stored, "the stored rows are in the checked bytes");
    drop(keep);
}
