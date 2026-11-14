//! T059 (FR-091): the real `store-check` command over temp copies of the oracle's stores
//! (`rust/parity/golden-store-health/`, made once by Python's `core/store_health.check`).
//!
//! The command's stdout is compared byte for byte with the golden of the same case: `--json` with `<case>.json` (Python's
//! `json.dumps(indent=2, sort_keys=True)`), the default form with `<case>.txt`. Exit codes: 0 healthy, 1 a problem found,
//! 3 not fully checked, 130 for Ctrl-C. `HOME` and `EASY_PERSONAL_HOME` point into a temp folder, so the owner's
//! `~/.easy-personal` is never reached.

use rusqlite::Connection;
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

fn golden() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/golden-store-health")
}

struct Home(PathBuf);

impl Home {
    fn new() -> Home {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t059-cli-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        Home(fs::canonicalize(dir).unwrap())
    }

    /// A home that holds a copy of the oracle's fixture folder.
    fn with(fixture: &str) -> Home {
        let home = Home::new();
        // an empty fixture folder (`missing`) is not kept by git, so a fresh checkout has no such folder: it reads as empty
        let folder = golden().join(fixture);
        if let Ok(entries) = fs::read_dir(&folder) {
            for entry in entries {
                let entry = entry.unwrap();
                fs::copy(entry.path(), home.0.join(entry.file_name())).unwrap();
            }
        } else {
            assert!(!folder.exists(), "{folder:?} cannot be read");
        }
        home
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"));
        command
            .arg("store-check")
            .args(args)
            .env("HOME", &self.0)
            .env("EASY_PERSONAL_HOME", &self.0);
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }

    fn listing(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn exit(o: &Output) -> i32 {
    o.status.code().unwrap_or(-1)
}

/// The cases the command can run as they are: no forced option, no setup.
fn plain_cases() -> Vec<(String, String)> {
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(golden().join("manifest.json")).unwrap()).unwrap();
    manifest["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["options"].as_object().unwrap().is_empty() && c["setup"].is_null())
        .map(|c| {
            (
                c["name"].as_str().unwrap().to_string(),
                c["fixture"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
fn json_and_text_equal_python_byte_for_byte_for_every_plain_case() {
    let cases = plain_cases();
    assert!(cases.len() >= 18, "{} plain cases", cases.len());
    for (case, fixture) in cases {
        let home = Home::with(&fixture);
        let json = home.run(&["--json"]);
        assert_eq!(
            text(&json.stdout),
            fs::read_to_string(golden().join(format!("{case}.json"))).unwrap(),
            "{case} --json"
        );
        let human = home.run(&[]);
        assert_eq!(
            text(&human.stdout),
            fs::read_to_string(golden().join(format!("{case}.txt"))).unwrap(),
            "{case} text"
        );
        let expected = match serde_json::from_slice::<Value>(&json.stdout).unwrap()["healthy"] {
            Value::Bool(true) => 0,
            Value::Bool(false) => 1,
            _ => 3,
        };
        assert_eq!(
            (exit(&json), exit(&human)),
            (expected, expected),
            "{case} exit code"
        );
        assert!(
            json.stderr.is_empty() && human.stderr.is_empty(),
            "{case}: {}",
            text(&human.stderr)
        );
        // read only: the folder is what the fixture was (no side file, no migration backup)
        let mut fixture_files: Vec<String> = fs::read_dir(golden().join(&fixture))
            .into_iter()
            .flatten()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        fixture_files.sort();
        assert_eq!(home.listing(), fixture_files, "{case}: files");
    }
}

#[test]
fn exit_codes_are_0_1_3() {
    assert_eq!(exit(&Home::with("clean").run(&[])), 0);
    assert_eq!(exit(&Home::with("behind").run(&[])), 1);
    assert_eq!(exit(&Home::with("corrupt_page").run(&[])), 1);
    assert_eq!(exit(&Home::new().run(&[])), 1, "no store at all");
    assert_eq!(exit(&Home::with("zero_byte").run(&[])), 1);
    assert_eq!(exit(&Home::with("not_a_database").run(&["--full"])), 1);
}

#[test]
fn a_locked_store_is_exit_3_never_one_problem() {
    let home = Home::with("locked");
    let holder = Connection::open(home.0.join("store.sqlite")).unwrap();
    holder
        .execute_batch("PRAGMA locking_mode=EXCLUSIVE")
        .unwrap();
    holder
        .query_row("SELECT count(*) FROM meta", [], |r| r.get::<_, i64>(0))
        .unwrap();
    let started = Instant::now();
    let out = home.run(&[]);
    assert_eq!(exit(&out), 3, "{}", text(&out.stdout));
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "waits at most the budget: {:?}",
        started.elapsed()
    );
    let shown = text(&out.stdout);
    assert!(
        shown.contains("unreadable (locked)") && !shown.contains("problem"),
        "{shown}"
    );
    let json: Value = serde_json::from_slice(&home.run(&["--json"]).stdout).unwrap();
    assert_eq!(
        (json["locked"].clone(), json["healthy"].clone()),
        (Value::Bool(true), Value::Null)
    );
    drop(holder);
}

#[test]
fn full_says_so_on_stderr_and_stdout_stays_pure_json() {
    let home = Home::with("clean");
    let out = home.run(&["--full", "--json"]);
    assert_eq!(exit(&out), 0);
    assert_eq!(
        text(&out.stderr),
        "full check: this can take minutes; Ctrl-C stops it\n"
    );
    let json: Value = serde_json::from_slice(&out.stdout).expect("stdout is only JSON");
    assert_eq!(json["integrity_check"]["mode"], "full");
    // a default run says nothing on stderr
    assert!(home.run(&["--json"]).stderr.is_empty());
}

#[test]
fn full_finds_a_corrupt_page_and_names_no_row() {
    let home = Home::with("corrupt_page");
    let out = home.run(&["--full"]);
    assert_eq!(exit(&out), 1);
    let shown = text(&out.stdout);
    assert!(
        shown.contains("integrity_check: 1 problem(s)") && shown.contains("verdict: NOT healthy"),
        "{shown}"
    );
    assert!(
        !shown.contains("CANARY") && !shown.contains("/work/"),
        "{shown}"
    );
}

#[test]
fn a_store_over_the_size_limit_gets_the_quick_check_and_full_overrides_it() {
    // 33 MB of events: past `FULL_CHECK_MAX_BYTES`, so a default run is a quick check unless it runs out of time
    let home = Home::with("clean");
    let conn = Connection::open(home.0.join("store.sqlite")).unwrap();
    conn.execute_batch(
        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 160000)
         INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id, project_path, model, message_id, input_tokens, output_tokens)
         SELECT 'm', 'claude_code', '/big', 1, i, 0, '2026-01-01T00:00:00.000000Z', 's' || (i % 500), '/w/p', 'model-0', 'id' || i, 10, 5 FROM n;
         PRAGMA wal_checkpoint(TRUNCATE);",
    )
    .unwrap();
    drop(conn);
    assert!(fs::metadata(home.0.join("store.sqlite")).unwrap().len() > 32_000_000);
    let default: Value = serde_json::from_slice(&home.run(&["--json"]).stdout).unwrap();
    match default["integrity_check"]["mode"].as_str() {
        Some(mode) => assert_eq!(mode, "quick"),
        None => assert_eq!(default["healthy"], Value::Null, "{default}"),
    }
    let full: Value = serde_json::from_slice(&home.run(&["--full", "--json"]).stdout).unwrap();
    assert_eq!(
        full["integrity_check"],
        serde_json::json!({"ok": true, "problems": 0, "mode": "full"})
    );
    assert_eq!(full["healthy"], Value::Bool(true));
}

#[test]
fn nothing_is_written_the_hash_the_mtime_and_the_wal_stay() {
    let home = Home::with("wal_no_shm");
    let snapshot = |home: &Home| {
        home.listing()
            .into_iter()
            .map(|n| {
                (
                    fs::read(home.0.join(&n)).unwrap(),
                    fs::metadata(home.0.join(&n)).unwrap().modified().unwrap(),
                    n,
                )
            })
            .collect::<Vec<_>>()
    };
    let before = snapshot(&home);
    for args in [&[][..], &["--json"], &["--full"], &["--full", "--json"]] {
        assert_eq!(exit(&home.run(args)), 0, "{args:?}");
    }
    assert!(before == snapshot(&home), "a file changed");
    assert_eq!(
        home.listing(),
        vec!["store.sqlite", "store.sqlite-wal"],
        "no -shm and no copy left beside the store"
    );
    // a missing store is not created
    let empty = Home::new();
    assert_eq!(exit(&empty.run(&[])), 1);
    assert!(empty.listing().is_empty());
}

#[test]
fn a_hostile_name_is_cleaned_in_every_output() {
    let home = Home::with("hostile");
    for args in [&[][..], &["--json"], &["--full"]] {
        let out = home.run(args);
        assert_eq!(exit(&out), 0, "{args:?}");
        assert!(out.stdout.len() > 100, "{args:?}: an answer");
        let shown = text(&out.stdout) + &text(&out.stderr);
        assert!(
            !shown.contains('\x1b') && !shown.contains('\x07'),
            "{args:?}: control character"
        );
        assert!(
            shown.chars().all(|c| c == '\n' || !c.is_control()),
            "{args:?}"
        );
    }
    let json = text(&home.run(&["--json"]).stdout);
    assert!(
        json.is_ascii(),
        "Python's JSON escapes every non-ASCII character"
    );
}

#[test]
fn no_panic_text_for_a_store_that_is_not_a_database() {
    for fixture in ["not_a_database", "zero_byte", "bad_table_name", "blob_time"] {
        let out = Home::with(fixture).run(&["--json"]);
        let err = text(&out.stderr);
        assert!(
            err.is_empty() && !text(&out.stdout).contains("panicked"),
            "{fixture}: {err}"
        );
    }
}

/// 500k events: `integrity_check` of this takes seconds even in a release build.
fn big_store(home: &Home) {
    let conn = Connection::open(home.0.join("store.sqlite")).unwrap();
    conn.execute_batch(
        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 600000)
         INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id, project_path, model, message_id, input_tokens, output_tokens)
         SELECT 'm', 'claude_code', '/big', 1, i, 0, '2026-01-01T00:00:00.000000Z', 's' || (i % 500), '/w/p', 'model-0', 'id' || i, 10, 5 FROM n;
         PRAGMA wal_checkpoint(TRUNCATE);",
    )
    .unwrap();
}

#[test]
fn ctrl_c_during_full_stops_with_130_no_panic_text_and_no_process_left() {
    let home = Home::with("clean");
    big_store(&home);
    let mut child = home
        .command(&["--full", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    let mut first = String::new();
    stderr.read_line(&mut first).unwrap();
    assert_eq!(
        first,
        "full check: this can take minutes; Ctrl-C stops it\n"
    );
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        child.try_wait().unwrap().is_none(),
        "the check ended before Ctrl-C: the store is too small for this machine"
    );
    let started = Instant::now();
    assert!(
        Command::new("kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "did not stop within 5 s of Ctrl-C"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut rest = String::new();
    stderr.read_to_string(&mut rest).unwrap();
    let mut out = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    assert_eq!(status.code(), Some(130));
    assert!(
        !rest.contains("panicked")
            && !rest.contains("RUST_BACKTRACE")
            && !rest.contains("Traceback"),
        "{rest}"
    );
    assert_eq!(rest, "store-check: stopped\n");
    assert_eq!(out, "", "no partial answer on stdout");
}

/// Starts `store-check --full --json` over a store of 600k events, with an empty `-wal` and no `-shm` (so the store is copied
/// to a private temp folder first), with `TMPDIR` in the home's own folder; returns the child once its notice is on stderr.
fn running_full_check_with_a_private_copy(
    home: &Home,
) -> (
    std::process::Child,
    BufReader<std::process::ChildStderr>,
    PathBuf,
) {
    big_store(home);
    fs::write(home.0.join("store.sqlite-wal"), b"").unwrap();
    let tmp = home.0.join("tmp");
    fs::create_dir(&tmp).unwrap();
    let mut child = home
        .command(&["--full", "--json"])
        .env("TMPDIR", &tmp)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    let mut first = String::new();
    stderr.read_line(&mut first).unwrap();
    std::thread::sleep(Duration::from_millis(600)); // the copy is made and the reads are running
    assert!(
        child.try_wait().unwrap().is_none(),
        "the check ended before the signal: the store is too small for this machine"
    );
    (child, stderr, tmp)
}

fn wait_for(child: &mut std::process::Child, secs: u64) -> std::process::ExitStatus {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        assert!(
            started.elapsed() < Duration::from_secs(secs),
            "did not stop within {secs} s"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn sigterm_stops_like_ctrl_c_and_the_private_copy_is_removed() {
    let home = Home::with("clean");
    let (mut child, mut stderr, tmp) = running_full_check_with_a_private_copy(&home);
    assert!(
        fs::read_dir(&tmp).unwrap().count() >= 1,
        "the private copy exists while the check runs"
    );
    assert!(
        Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let status = wait_for(&mut child, 5);
    let mut rest = String::new();
    stderr.read_to_string(&mut rest).unwrap();
    assert_eq!(status.code(), Some(130));
    assert_eq!(rest, "store-check: stopped\n");
    assert_eq!(
        fs::read_dir(&tmp).unwrap().count(),
        0,
        "no private copy is left"
    );
}

#[test]
fn a_second_ctrl_c_ends_the_process_at_once_with_130() {
    let home = Home::with("clean");
    let (mut child, _stderr, _tmp) = running_full_check_with_a_private_copy(&home);
    let pid = child.id().to_string();
    for _ in 0..2 {
        assert!(
            Command::new("kill")
                .args(["-INT", &pid])
                .status()
                .unwrap()
                .success()
        );
    }
    let status = wait_for(&mut child, 5);
    assert_eq!(status.code(), Some(130));
}

#[test]
#[ignore = "writes a store of about 360 MB; run: cargo test --release -p easy-personal-rs --test store_check -- --ignored --nocapture"]
fn a_store_of_a_million_events_answers_in_budget_or_says_unknown() {
    let home = Home::with("clean");
    let conn = Connection::open(home.0.join("store.sqlite")).unwrap();
    conn.execute_batch(
        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 1000000)
         INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id, project_path, model, message_id, input_tokens, output_tokens)
         SELECT 'm', 'claude_code', '/big', 1, i, 0,
                '2026-01-01T00:' || printf('%02d', i / 60 % 60) || ':' || printf('%02d', i % 60) || '.000000Z',
                's' || (i % 500), '/w/p', 'model-0', 'id' || i, 10, 5 FROM n;
         PRAGMA wal_checkpoint(TRUNCATE);",
    )
    .unwrap();
    drop(conn);
    let size = fs::metadata(home.0.join("store.sqlite")).unwrap().len();
    let mut timings = Vec::new();
    for _ in 0..3 {
        let started = Instant::now();
        let out = home.run(&["--json"]);
        timings.push(started.elapsed().as_secs_f64());
        let json: Value = serde_json::from_slice(&out.stdout).unwrap();
        let events = json["tables"]["events"].as_i64();
        println!(
            "store {size} bytes: {:.2} s, healthy {}, incomplete {}, mode {}, events {events:?}",
            timings.last().unwrap(),
            json["healthy"],
            json["incomplete"],
            json["integrity_check"]["mode"]
        );
        // never "healthy" with an unfinished part
        if json["healthy"] == Value::Bool(true) {
            assert!(
                json["incomplete"].as_array().unwrap().is_empty()
                    && json["integrity_check"]["mode"] == "quick"
            );
        } else {
            assert_eq!(json["healthy"], Value::Null);
            assert!(!json["incomplete"].as_array().unwrap().is_empty());
        }
        assert_eq!(
            events,
            Some(1_000_005),
            "the clean fixture holds 5 events of its own"
        );
    }
    assert!(timings.iter().all(|t| *t < 2.5), "{timings:?}");
}
