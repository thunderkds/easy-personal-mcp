//! T075: a `git log` that hangs or fails makes the done time unknown (never "not done"), and one deadline bounds the
//! git work of a whole request. Its own test binary because it changes PATH.

use easy_personal_core::effectiveness::effectiveness_in;
use easy_personal_core::scope::Scope;
use easy_personal_core::store::open_read_only;
use jiff::tz::TimeZone;
use rusqlite::Connection;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

fn real_git() -> String {
    let out = Command::new("sh")
        .args(["-c", "command -v git"])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn git(git: &str, repo: &Path, args: &[&str]) {
    let ok = Command::new(git)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(["-c", "init.defaultBranch=main", "-C"])
        .arg(repo)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?}");
}

#[test]
fn a_hung_or_failed_git_log_is_unknown_and_one_deadline_bounds_the_request() {
    let dir = std::env::temp_dir().join(format!(
        "easy-personal-rs-t075-gitto-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    let real = real_git();
    // four repos, each with main and no merge
    let names = ["hang-a", "hang-b", "hang-c", "fails"];
    for name in names {
        let repo = dir.join(name);
        fs::create_dir_all(&repo).unwrap();
        git(&real, &repo, &["init", "-q"]);
        fs::write(repo.join("f"), "x").unwrap();
        git(&real, &repo, &["add", "f"]);
        git(&real, &repo, &["commit", "-q", "-m", "base"]);
    }
    // the fake git: `log` hangs in the hang-* repos and fails in `fails`; everything else is the real git
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let fake = bin.join("git");
    fs::write(
        &fake,
        format!(
            "#!/bin/sh\ncase \"$*\" in\n  *fails*log*) exit 128 ;;\n  *hang-*log*) exec sleep 60 ;;\nesac\nexec {real} \"$@\"\n"
        ),
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();

    let store = dir.join("store.sqlite");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/store-v11.sqlite"),
        &store,
    )
    .unwrap();
    let c = Connection::open(&store).unwrap();
    c.execute_batch("DELETE FROM events").unwrap();
    for (i, name) in names.iter().enumerate() {
        c.execute(
            "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, record_type, ts_utc,
             session_id, project_path, git_branch, is_sidechain, model, message_id, input_tokens, output_tokens,
             cache_creation_input_tokens, cache_read_input_tokens)
             VALUES ('m', 'claude_code', 'f', 1, ?1, 0, 'assistant', '2026-01-10T10:00:00.000000Z', ?2, ?3, 'T1', 0,
             'claude-sonnet-5-5', ?2, 10, 1, 0, 0)",
            rusqlite::params![i as i64, format!("s{i}"), dir.join(name).to_str().unwrap()],
        )
        .unwrap();
    }
    drop(c);

    let old_path = std::env::var("PATH").unwrap_or_default();
    // SAFETY: this binary has one test, so no other thread reads the environment.
    unsafe { std::env::set_var("PATH", format!("{}:{old_path}", bin.display())) };
    let started = Instant::now();
    let body = effectiveness_in(
        &open_read_only(&store).unwrap(),
        None,
        None,
        Scope::default(),
        &TimeZone::UTC,
    )
    .unwrap();
    let took = started.elapsed();
    unsafe { std::env::set_var("PATH", old_path) };
    let _ = fs::remove_dir_all(&dir);

    for task in body["tasks"].as_array().unwrap() {
        assert_eq!(task["done_state"], "unknown", "{task}");
    }
    assert_eq!(body["tasks"].as_array().unwrap().len(), 4);
    assert!(
        body["notes"][0]
            .as_str()
            .unwrap()
            .contains("done time unknown"),
        "{body:#}"
    );
    // three hung repos at 5 s each would be 15 s; the request's git deadline stops it near 10 s
    assert!(
        took < Duration::from_secs(13),
        "the deadline did not hold: {took:?}"
    );
}
