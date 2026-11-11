//! T027 criterion 5, the real check (ignored: needs the parity venv `rust/parity/.venv`, see `rust/parity/README.md`):
//! the Python app opens a store the Rust writer created **without migrating it** (`store.opened` leaves
//! `user_version` 11 and writes no `.bak-v*` copy), reads Rust's `machine_id`, and saves a row that Rust then finds
//! under the same key. Temp folders only; `HOME` and `EASY_PERSONAL_HOME` point into the temp folder.
//!
//! Run: `cargo test -p easy-personal-core --test writer_python -- --ignored --nocapture`

use easy_personal_core::writer::{Event, Line, open_for_write};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SCRIPT: &str = r#"
import sys
from pathlib import Path
from easy_personal.core import store

path = Path(sys.argv[1])
with store.opened(path) as conn:
    print("user_version", conn.execute("PRAGMA user_version").fetchone()[0])
    print("machine_id", store.machine_id(conn))
    event = {c: None for c in store.EVENT_COLUMNS}
    event.update(abstained=0, record_type="user", version="2.1.400", ts_utc="2026-01-10T15:00:00.000000Z",
                 session_id="t027-session", is_sidechain=False, entrypoint="cli")
    added = store.save_file(conn, "claude_code", "t027/from-python.jsonl", 77, 0, iter([(0, 120, event)]))
    print("python_added", added)
    print("offsets", sorted(store.file_offsets(conn, "claude_code").items()))
"#;

#[test]
#[ignore = "needs rust/parity/.venv (the parity venv); run with --ignored"]
fn python_opens_a_rust_created_store_without_migrating() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let python = manifest.join("../../parity/.venv/bin/python");
    assert!(
        python.exists(),
        "no parity venv at rust/parity/.venv (see rust/parity/README.md)"
    );
    let dir = std::env::temp_dir().join(format!(
        "easy-personal-rs-t027-python-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("home")).unwrap();
    let path: PathBuf = dir.join("store.sqlite");

    let store = open_for_write(&path).unwrap();
    let rust_id = store.machine_id().to_owned();
    drop(store);

    let out = Command::new("timeout")
        .arg("60")
        .arg(&python)
        .args(["-I", "-c", SCRIPT])
        .arg(&path)
        .current_dir(&dir)
        .env("HOME", dir.join("home"))
        .env("EASY_PERSONAL_HOME", dir.join("home"))
        .env("TZ", "UTC")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    println!(
        "python exit {:?}\n{stdout}{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success());
    assert!(stdout.contains("user_version 11\n"), "{stdout}");
    assert!(
        stdout.contains(&format!("machine_id {rust_id}\n")),
        "Python read another machine id"
    );
    assert!(stdout.contains("python_added 1\n"), "{stdout}");
    assert!(
        stdout.contains("offsets [('t027/from-python.jsonl', (77, 120))]"),
        "{stdout}"
    );

    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    println!("folder after Python: {names:?}");
    assert!(
        !names.iter().any(|n| n.contains(".bak-v")),
        "Python migrated: {names:?}"
    );
    let version: i64 = Connection::open(&path)
        .unwrap()
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 11);

    // The row Python saved is the row Rust would save: the same key adds nothing.
    let mut store = open_for_write(&path).unwrap();
    assert_eq!(store.machine_id(), rust_id);
    let same = Event {
        record_type: Some("user".into()),
        version: Some("2.1.400".into()),
        ts_utc: Some("2026-01-10T15:00:00.000000Z".into()),
        session_id: Some("t027-session".into()),
        is_sidechain: Some(false),
        entrypoint: Some("cli".into()),
        ..Event::default()
    };
    let line = [Line {
        line_offset: 0,
        next_offset: 120,
        event: Some(same),
    }];
    let added = store
        .save_file("claude_code", "t027/from-python.jsonl", 77, 0, &line, None)
        .unwrap();
    println!("rust_added_for_the_python_row {added}");
    assert_eq!(added, 0);
    drop(store);
    let _ = fs::remove_dir_all(&dir);
}
