//! T042: a hung `git` must not hang `kit_summary`: Python's `git_part` kills it after 10 s and says `abstained`
//! (`git_local.TIMEOUT_S`). A missing `git` or a folder that is not a repo says `no data`. This file is its own test
//! binary because it changes `PATH`; it holds one test so nothing else reads the variable meanwhile.

use easy_personal_core::kit::kit_summary;
use easy_personal_core::scope::Scope;
use easy_personal_core::store::open_read_only;
use rusqlite::Connection;
use serde_json::json;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};

#[test]
fn a_hung_git_makes_the_git_part_abstained_after_ten_seconds_and_a_missing_git_is_no_data() {
    let dir = std::env::temp_dir().join(format!(
        "easy-personal-rs-t042-timeout-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    let bin = dir.join("bin");
    let project = dir.join("project");
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&project).unwrap();
    let git = bin.join("git");
    fs::write(&git, "#!/bin/sh\nexec sleep 60\n").unwrap();
    fs::set_permissions(&git, fs::Permissions::from_mode(0o755)).unwrap();

    let store = dir.join("store.sqlite");
    fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/store-v11.sqlite"),
        &store,
    )
    .unwrap();
    Connection::open(&store)
        .unwrap()
        .execute(
            "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id, \
             project_path, tool_names) VALUES ('m', 'claude_code', 'x', 1, 0, 0, '2026-01-15T10:00:00.000000Z', 's', ?1, 'Read')",
            [project.to_str().unwrap()],
        )
        .unwrap();
    let conn = open_read_only(&store).unwrap();
    let now = 1_768_867_200_000_000;
    let git_of = |conn: &Connection| {
        let body = kit_summary(conn, now, Scope::default(), false).unwrap();
        let list = body["projects"].as_array().unwrap().clone();
        list.into_iter()
            .find(|p| p["project"] == "project")
            .expect("the project is listed")["parts"]["git"]
            .clone()
    };

    // SAFETY: the only test of this binary, so no other thread reads or writes the environment meanwhile.
    unsafe { std::env::set_var("PATH", format!("{}:/usr/bin:/bin", bin.display())) };
    let started = Instant::now();
    assert_eq!(git_of(&conn), json!({"status": "abstained"}));
    let took = started.elapsed();
    assert!(
        took >= Duration::from_secs(10) && took < Duration::from_secs(30),
        "killed after the limit, not before and not never: {took:?}"
    );

    // no `git` on the PATH at all: Python's `OSError` is `None`, the part has no data
    fs::remove_file(&git).unwrap();
    assert_eq!(git_of(&conn), json!({"status": "no data"}));
    let _ = fs::remove_dir_all(&dir);
}
