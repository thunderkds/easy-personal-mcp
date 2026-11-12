//! T052: every `git` this crate starts runs with `GIT_NO_LAZY_FETCH=1`, so a partial-clone repository can never make a
//! `rev-list` / `rev-parse` fetch a missing object over the network (a Rust-only hardening: Python has the same gap). The
//! fake `git` on a scratch `PATH` writes the variable it was started with and fails, so nothing real is run. This file is
//! its own test binary because it changes PATH.

use easy_personal_core::metrics::usage_breakdown;
use easy_personal_core::store::open_read_only;
use rusqlite::Connection;
use std::fs;
use std::os::unix::fs::PermissionsExt;

#[test]
fn every_git_run_says_never_to_fetch_lazily() {
    let dir = std::env::temp_dir().join(format!(
        "easy-personal-rs-t052-gitenv-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let seen = dir.join("seen.txt");
    let git = bin.join("git");
    fs::write(
        &git,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"${{GIT_NO_LAZY_FETCH-unset}}\" >> '{}'\nexit 1\n",
            seen.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&git, fs::Permissions::from_mode(0o755)).unwrap();

    // A gone folder whose nearest existing parent exists: Python asks git about that parent (rule 1).
    let gone = dir.join("missing").join("wt");
    let store = dir.join("store.sqlite");
    let c = Connection::open(&store).unwrap();
    c.execute_batch(
        "CREATE TABLE events (id INTEGER PRIMARY KEY, abstained INTEGER NOT NULL, version TEXT, ts_utc TEXT,
            session_id TEXT, project_path TEXT, git_branch TEXT, is_sidechain INTEGER, model TEXT,
            message_id TEXT, tool_names TEXT, input_tokens INTEGER, output_tokens INTEGER,
            cache_creation_input_tokens INTEGER, cache_read_input_tokens INTEGER, entrypoint TEXT);",
    )
    .unwrap();
    c.execute(
        "INSERT INTO events (abstained, ts_utc, project_path, git_branch, message_id, model, input_tokens,
            output_tokens, cache_creation_input_tokens, cache_read_input_tokens, entrypoint, is_sidechain)
         VALUES (0, '2026-01-15T10:00:00.000000Z', ?1, 'feat', 'm1', 'm', 100, 0, 0, 0, 'cli', 0)",
        [gone.to_str().unwrap()],
    )
    .unwrap();
    drop(c);

    let old_path = std::env::var("PATH").unwrap_or_default();
    // SAFETY: this binary has one test, so no other thread reads the environment.
    unsafe { std::env::set_var("PATH", format!("{}:{old_path}", bin.display())) };
    let _ = usage_breakdown(&open_read_only(&store).unwrap(), "project", None, None).unwrap();
    unsafe { std::env::set_var("PATH", old_path) };

    let log = fs::read_to_string(&seen).unwrap_or_default();
    let _ = fs::remove_dir_all(&dir);
    let runs: Vec<&str> = log.lines().collect();
    assert!(!runs.is_empty(), "the fake git was never started");
    assert!(
        runs.iter().all(|v| *v == "1"),
        "GIT_NO_LAZY_FETCH as each git run saw it: {runs:?}"
    );
}
