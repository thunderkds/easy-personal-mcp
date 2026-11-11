//! T029: `easy-personal-rs ingest` through the real binary. The inputs are the T026 oracle's (`rust/parity/ingest`):
//! each scenario is rebuilt with the T035 test helpers (`core/tests/ingest_folders_common`, included below by path:
//! one crate's test module cannot be imported by another), the binary runs in the case's work root with an env built
//! from nothing (`HOME`, `EASY_PERSONAL_HOME` and `EASY_PERSONAL_AGENT_ROOT` all inside the case, so the owner's real
//! folders are never reached), and its stdout, exit code, stderr and the four tables are compared to the goldens.
//!
//! The real Python run (ignored: needs `rust/parity/.venv`, see `rust/parity/README.md`):
//! `cargo test -p easy-personal-rs --test ingest_cli -- --ignored --nocapture`

#[path = "../../core/tests/ingest_folders_common/mod.rs"]
mod common;

use common::{Case, committed, fixtures, golden, repo, repos};
use easy_personal_core::writer::open_for_write;
use rusqlite::Connection;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const BUSY: &str = "another ingest is running; try again later\n";

struct Out {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// The binary, in the case's work root, under `timeout`, with an env built from nothing.
fn command(case: &Case) -> Command {
    let mut c = Command::new("timeout");
    c.arg("120")
        .arg(env!("CARGO_BIN_EXE_easy-personal-rs"))
        .current_dir(&case.work)
        .env_clear()
        .env(
            "PATH",
            std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into()),
        )
        .env("HOME", &case.nowhere)
        .env("EASY_PERSONAL_HOME", &case.home)
        .env("EASY_PERSONAL_AGENT_ROOT", &case.agents)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null");
    if let Ok(tz) = std::env::var("TZ") {
        c.env("TZ", tz); // the TZ runs of criterion 12 reach the binary
    }
    c
}

fn collect(out: std::process::Output) -> Out {
    Out {
        code: out.status.code(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

fn run(case: &Case, args: &[&str]) -> Out {
    collect(command(case).args(["ingest"]).args(args).output().unwrap())
}

fn run_with(case: &Case, args: &[&str], tweak: impl FnOnce(&mut Command)) -> Out {
    let mut c = command(case);
    c.arg("ingest").args(args);
    tweak(&mut c);
    collect(c.output().unwrap())
}

/// What `json.dumps({...}, indent=2, sort_keys=True)` prints for the events source's result, written out by hand.
fn expected(files: i64, new_events: i64, skipped: i64, pruned: Option<i64>) -> String {
    let pruned = pruned.map_or(String::new(), |n| format!("  \"pruned\": {n},\n"));
    format!(
        "{{\n  \"files\": {files},\n  \"new_events\": {new_events},\n  \"not_ingested\": [\n    \"context\",\n    \
         \"kit_trace\",\n    \"ui\",\n    \"verifier\"\n  ],\n{pruned}  \"skipped\": {skipped}\n}}\n"
    )
}

fn runs(g: &Value) -> &Vec<Value> {
    g["runs"].as_array().unwrap()
}

/// Run the binary once and compare it to golden run `i`: exit 0, nothing on stderr, the numbers, the exact text,
/// the four tables.
fn replay(case: &Case, g: &Value, i: usize, args: &[&str]) {
    let run_g = &runs(g)[i];
    let what = format!("{} run {i}", g["scenario"].as_str().unwrap());
    assert_eq!(run_g["exit_code"], 0, "{what}: Python succeeded");
    let out = run(case, args);
    assert_eq!(out.code, Some(0), "{what}: {}", case.tok(&out.stderr));
    assert_eq!(out.stderr, "", "{what}: stderr");
    let j = &run_g["stdout_json"];
    assert_eq!(
        out.stdout,
        expected(
            j["files"].as_i64().unwrap(),
            j["new_events"].as_i64().unwrap(),
            0,
            j.get("pruned").and_then(Value::as_i64)
        ),
        "{what}: stdout"
    );
    case.assert_tables(run_g, &what);
}

fn append(path: &Path, bytes: &[u8]) {
    fs::OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(bytes)
        .unwrap();
}

// -- the oracle replay through the binary (criterion 8)

#[test]
fn s01_cold_ingest_prints_the_golden_numbers_and_fills_the_four_tables() {
    let g = golden("S01_cold_committed.json");
    let case = Case::new();
    case.copy(&committed(), &case.projects);
    replay(&case, &g, 0, &[]);
    // The data folder as Python leaves it: the lock file and the store, mode 0700.
    let mut names: Vec<String> = fs::read_dir(&case.home)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(
        names,
        runs(&g)[0]["home_files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        fs::metadata(&case.home).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(runs(&g)[0]["home_mode"], "0o700");
}

#[test]
fn s02_a_second_run_adds_nothing_and_counts_the_same_files() {
    let g = golden("S02_rerun_unchanged.json");
    let case = Case::new();
    case.copy(&committed(), &case.projects);
    replay(&case, &g, 0, &[]);
    replay(&case, &g, 1, &[]);
}

#[test]
fn s03_an_appended_line_is_read_incrementally() {
    let g = golden("S03_line_appended.json");
    let case = Case::new();
    let target = case.projects.join("s3/session.jsonl");
    case.copy(&fixtures().join("s3/session.jsonl"), &target);
    replay(&case, &g, 0, &[]);
    append(
        &target,
        &fs::read(fixtures().join("s3/append.jsonl")).unwrap(),
    );
    replay(&case, &g, 1, &[]);
}

#[test]
fn s04_offsets_reset_back_fill_without_new_rows() {
    let g = golden("S04_offsets_reset_backfill.json");
    let case = Case::new();
    case.copy(&committed(), &case.projects);
    replay(&case, &g, 0, &[]);
    case.sql(
        "UPDATE events SET entrypoint = NULL, tool_error_count = NULL; UPDATE source_files SET byte_offset = 0;",
    );
    case.assert_tables(&runs(&g)[1], "S04 after the reset");
    replay(&case, &g, 2, &[]);
}

#[test]
fn s05_a_truncated_last_line_is_read_once_completed() {
    let g = golden("S05_truncated_then_completed.json");
    let case = Case::new();
    let target = case.projects.join("s5/session.jsonl");
    case.copy(&fixtures().join("s5/session.jsonl"), &target);
    replay(&case, &g, 0, &[]);
    append(&target, &fs::read(fixtures().join("s5/rest.part")).unwrap());
    replay(&case, &g, 1, &[]);
}

#[test]
fn s06_the_synthetic_set_through_git_resolves_the_same_projects() {
    let g = golden("S06_cold_synthetic.json");
    let layout = repos();
    let case = Case::new();
    case.apply(&layout["work"]);
    case.copy(&fixtures().join("synthetic"), &case.projects);
    case.copy(&fixtures().join("outside"), &case.agents.join("outside"));
    case.apply(&layout["synthetic"]);
    replay(&case, &g, 0, &[]);
}

/// The oracle confirmed `.claude-work` through the real CLI (`account confirm`, which runs `detect` first); the Rust
/// binary has no such command, so the test runs `detect` and applies the same final row state by SQL.
#[test]
fn s07_two_confirmed_accounts() {
    let g = golden("S07_two_accounts.json");
    let case = Case::new();
    for entry in fs::read_dir(fixtures().join("s7/agents")).unwrap() {
        let entry = entry.unwrap();
        let name = format!(".{}", entry.file_name().to_str().unwrap());
        case.copy(&entry.path(), &case.agents.join(name));
    }
    fs::create_dir_all(&case.home).unwrap();
    let mut store = open_for_write(&case.store_path()).unwrap();
    easy_personal_core::ingest::accounts::detect(&mut store, &case.agents).unwrap();
    drop(store);
    case.sql(&format!(
        "UPDATE agent_accounts SET status = 'confirmed', label = 'Claude work' WHERE folder = '{}'",
        case.agents.join(".claude-work").to_str().unwrap()
    ));
    case.assert_tables(&runs(&g)[0], "S07 after confirm");
    replay(&case, &g, 1, &[]);
}

#[test]
fn s08a_a_retention_that_prunes_nothing_prints_pruned_0() {
    let g = golden("S08a_retention_36500.json");
    let case = Case::new();
    case.copy(&committed(), &case.projects);
    replay(&case, &g, 0, &["--retention-days", "36500"]);
}

/// Every timestamped row goes (26), the NULL-timestamp rows stay, `source_files` is untouched: the tables equal S08b.
#[test]
fn s08b_retention_1_prunes_every_timestamped_row_and_keeps_the_rest() {
    let g = golden("S08b_retention_1.json");
    let case = Case::new();
    case.copy(&committed(), &case.projects);
    replay(&case, &g, 0, &["--retention-days", "1"]);
    let kept: i64 = Connection::open(case.store_path())
        .unwrap()
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(kept, 39 - 26);
}

#[test]
fn s10a_source_replaces_the_default_accounts_folder() {
    let g = golden("S10a_source_override_default_account.json");
    let case = Case::new();
    case.copy(
        &fixtures().join("s3/session.jsonl"),
        &case.projects.join("own/session.jsonl"),
    );
    let over = case.work.join("override");
    case.copy(&committed(), &over);
    replay(&case, &g, 0, &["--source", over.to_str().unwrap()]);
}

#[test]
fn s10b_source_with_no_account_is_read_untagged() {
    let g = golden("S10b_source_override_no_account.json");
    let case = Case::new();
    let over = case.work.join("override");
    case.copy(&committed(), &over);
    replay(&case, &g, 0, &["--source", over.to_str().unwrap()]);
}

// -- the source option and the environment (criterion 2)

/// A relative `--source` is `Path(...)`-normalised before it is stored: no leading `./`, no repeated or trailing
/// separator; the flag wins over `EASY_PERSONAL_CLAUDE_PROJECTS`, which alone does the same.
#[test]
fn a_relative_source_is_normalised_like_pythons_path_and_the_flag_wins_over_the_variable() {
    let case = Case::new();
    case.copy(&committed(), &case.work.join("override"));
    let stored = |case: &Case| -> Vec<String> {
        Connection::open(case.store_path())
            .unwrap()
            .prepare("SELECT path FROM source_files ORDER BY path")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    let out = run_with(&case, &["--source", "./override//"], |c| {
        c.env("EASY_PERSONAL_CLAUDE_PROJECTS", "./nowhere-at-all");
    });
    assert_eq!(out.stdout, expected(6, 39, 0, None), "{}", out.stderr);
    let paths = stored(&case);
    assert_eq!(paths.len(), 6);
    assert!(
        paths.iter().all(|p| p.starts_with("override/")),
        "{paths:?}"
    );
    // The variable alone, in a second store: the same normalisation, the same keys.
    let again = Case::new();
    again.copy(&committed(), &again.work.join("override"));
    let out = run_with(&again, &[], |c| {
        c.env("EASY_PERSONAL_CLAUDE_PROJECTS", "./override/");
    });
    assert_eq!(out.stdout, expected(6, 39, 0, None), "{}", out.stderr);
    assert_eq!(stored(&again), paths);
    // An empty variable is unset: the default account's own (empty) folder is read.
    let empty = run_with(&again, &[], |c| {
        c.env("EASY_PERSONAL_CLAUDE_PROJECTS", "");
    });
    assert_eq!(empty.stdout, expected(0, 0, 0, None), "{}", empty.stderr);
}

/// S10 reads a folder that exists; a `--source` that does not exist reads nothing and is not an error (Python:
/// `root.is_dir()` is false, nothing is scanned).
#[test]
fn a_missing_source_folder_is_zero_files_not_an_error() {
    let case = Case::new();
    let missing = case.nowhere.join("missing");
    let out = run(&case, &["--source", missing.to_str().unwrap()]);
    assert_eq!(out.code, Some(0), "{}", out.stderr);
    assert_eq!(out.stdout, expected(0, 0, 0, None));
    assert_eq!(out.stderr, "");
    assert!(case.store_path().is_file(), "the store is still created");
}

#[test]
fn the_data_dir_falls_back_to_home_dot_easy_personal_and_an_existing_dir_keeps_its_mode() {
    let case = Case::new();
    let out = run_with(&case, &[], |c| {
        c.env_remove("EASY_PERSONAL_HOME");
    });
    assert_eq!(out.code, Some(0), "{}", out.stderr);
    let made = case.nowhere.join(".easy-personal");
    assert!(made.join("store.sqlite").is_file());
    assert_eq!(
        fs::metadata(&made).unwrap().permissions().mode() & 0o777,
        0o700
    );
    // An existing folder is used as it is: its mode is not ours to change.
    let kept = case.root.join("open-home");
    fs::DirBuilder::new().mode(0o755).create(&kept).unwrap();
    fs::set_permissions(&kept, fs::Permissions::from_mode(0o755)).unwrap();
    let out = run_with(&case, &[], |c| {
        c.env("EASY_PERSONAL_HOME", &kept);
    });
    assert_eq!(out.code, Some(0), "{}", out.stderr);
    assert_eq!(
        fs::metadata(&kept).unwrap().permissions().mode() & 0o777,
        0o755
    );
}

#[test]
fn a_malformed_lock_timeout_is_a_clean_error_before_anything_is_made() {
    let case = Case::new();
    for bad in ["soon", "", "3 s"] {
        let out = run_with(&case, &[], |c| {
            c.env("EASY_PERSONAL_LOCK_TIMEOUT", bad);
        });
        assert_eq!(out.code, Some(1), "{bad:?}");
        assert_eq!(out.stdout, "");
        assert_eq!(
            out.stderr,
            "easy-personal-rs: EASY_PERSONAL_LOCK_TIMEOUT is not a number of seconds\n"
        );
        assert!(!case.home.exists(), "the data folder is not made");
    }
}

#[test]
fn a_data_dir_that_cannot_be_used_is_one_line_exit_1_without_the_path() {
    let case = Case::new();
    let file = case.root.join("a-file");
    fs::write(&file, "x").unwrap();
    let dangling = case.root.join("dangling");
    std::os::unix::fs::symlink(case.root.join("not-there"), &dangling).unwrap();
    for home in [file, dangling, case.root.join("a-file/inside")] {
        let out = run_with(&case, &[], |c| {
            c.env("EASY_PERSONAL_HOME", &home);
        });
        assert_eq!(out.code, Some(1), "{}", case.tok(&out.stderr));
        assert_eq!(out.stdout, "");
        assert_eq!(out.stderr.lines().count(), 1, "{}", out.stderr);
        assert!(
            out.stderr.starts_with("easy-personal-rs: "),
            "{}",
            out.stderr
        );
        assert!(
            !out.stderr.contains(case.root.to_str().unwrap()),
            "no path in the message: {}",
            out.stderr
        );
        assert!(!out.stderr.contains("panicked"));
    }
}

/// A result that cannot be written (`/dev/full`) is exit 1 with one line, never a panic.
#[test]
fn a_stdout_that_cannot_be_written_is_a_clean_failure_not_a_panic() {
    let case = Case::new();
    let full = fs::OpenOptions::new().write(true).open("/dev/full");
    let Ok(full) = full else { return };
    let mut c = command(&case);
    c.arg("ingest").stdout(Stdio::from(full));
    let out = collect(c.output().unwrap());
    assert_eq!(out.code, Some(1), "{}", out.stderr);
    assert_eq!(
        out.stderr,
        "easy-personal-rs: cannot write the result to stdout\n"
    );
}

// -- usage errors, busy lock, refused stores (criteria 1, 4, 5, 7)

#[test]
fn a_retention_of_zero_or_less_is_a_usage_error_exit_2_and_touches_nothing() {
    let g = golden("S08c_retention_0_refused.json");
    assert_eq!(runs(&g)[0]["exit_code"], 2, "the oracle's S08c");
    let case = Case::new();
    for bad in ["0", "-1", "abc", "1.5"] {
        let out = run(&case, &["--retention-days", bad]);
        assert_eq!(out.code, Some(2), "{bad}");
        assert_eq!(out.stdout, "");
        assert!(out.stderr.contains("--retention-days"), "{}", out.stderr);
        assert!(!case.home.exists(), "a usage error makes no data folder");
    }
}

#[test]
fn ingest_is_listed_in_help_with_its_two_options() {
    let case = Case::new();
    let top = collect(command(&case).arg("--help").output().unwrap());
    assert!(
        top.stdout
            .lines()
            .any(|l| l.trim_start().starts_with("ingest")),
        "{}",
        top.stdout
    );
    let help = collect(command(&case).args(["ingest", "--help"]).output().unwrap());
    assert!(help.stdout.contains("--source <PATH>"), "{}", help.stdout);
    assert!(
        help.stdout.contains("--retention-days <N>"),
        "{}",
        help.stdout
    );
}

/// Holds `<home>/ingest.lock` with `flock` from this process (like the generator for S09), until told to let go.
struct Holder {
    release: mpsc::Sender<()>,
    thread: thread::JoinHandle<()>,
}

impl Holder {
    fn take(case: &Case, hold_for: Option<Duration>) -> Self {
        fs::create_dir_all(&case.home).unwrap();
        let file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(case.home.join("ingest.lock"))
            .unwrap();
        let (release, wait) = mpsc::channel::<()>();
        let (held, held_rx) = mpsc::channel::<()>();
        let thread = thread::spawn(move || {
            file.lock().unwrap();
            held.send(()).unwrap();
            match hold_for {
                Some(span) => thread::sleep(span),
                None => {
                    let _ = wait.recv();
                }
            }
            file.unlock().unwrap();
        });
        held_rx.recv().unwrap();
        Holder { release, thread }
    }

    fn release(self) {
        let _ = self.release.send(());
        self.thread.join().unwrap();
    }
}

/// S09: the exact stderr line, empty stdout, exit 75, `ingest.lock` still there, the store never touched.
#[test]
fn s09_a_busy_lock_is_exit_75_with_pythons_line_and_never_touches_the_store() {
    let g = golden("S09_lock_busy.json");
    assert_eq!(runs(&g)[0]["exit_code"], 75);
    assert_eq!(runs(&g)[0]["stderr_text_tokenised"], BUSY);
    let case = Case::new();
    case.copy(&committed(), &case.projects);
    let holder = Holder::take(&case, None);
    let started = Instant::now();
    let out = run_with(&case, &[], |c| {
        c.env("EASY_PERSONAL_LOCK_TIMEOUT", "0.5");
    });
    let waited = started.elapsed();
    holder.release();
    assert_eq!(out.code, Some(75), "{}", out.stderr);
    assert_eq!(out.stderr, BUSY);
    assert_eq!(out.stdout, "");
    assert!(waited >= Duration::from_millis(400), "it waited {waited:?}");
    assert!(
        case.home.join("ingest.lock").is_file(),
        "the lock file stays"
    );
    assert!(
        !case.store_path().exists(),
        "a busy lock never opens or creates the store"
    );
}

/// Criterion 10, two processes: another holder lets go after 1 s; with a 3 s wait the run waits, then succeeds; with a
/// 0.2 s wait on a holder that stays, it is 75.
#[test]
fn two_processes_the_second_waits_for_the_lock_or_gives_up() {
    let case = Case::new();
    case.copy(&committed(), &case.projects);
    let holder = Holder::take(&case, Some(Duration::from_secs(1)));
    let started = Instant::now();
    let out = run_with(&case, &[], |c| {
        c.env("EASY_PERSONAL_LOCK_TIMEOUT", "3");
    });
    let waited = started.elapsed();
    holder.release();
    assert_eq!(out.code, Some(0), "{}", out.stderr);
    assert_eq!(out.stdout, expected(6, 39, 0, None));
    assert!(waited >= Duration::from_millis(500), "it waited {waited:?}");

    let holder = Holder::take(&case, None);
    let out = run_with(&case, &[], |c| {
        c.env("EASY_PERSONAL_LOCK_TIMEOUT", "0.2");
    });
    holder.release();
    assert_eq!(out.code, Some(75), "{}", out.stderr);
    assert_eq!(out.stderr, BUSY);
    // The lock is free again: the next run is clean.
    let out = run(&case, &[]);
    assert_eq!(out.stdout, expected(6, 0, 0, None));
}

/// A store of another schema version, made as the writer makes one (`open_for_write`) and then marked.
fn marked_store(case: &Case, user_version: i64) -> Vec<u8> {
    fs::create_dir_all(&case.home).unwrap();
    drop(open_for_write(&case.store_path()).unwrap());
    case.sql(&format!("PRAGMA user_version = {user_version}"));
    assert_eq!(listing(&case.home), ["store.sqlite"], "no WAL left behind");
    fs::read(case.store_path()).unwrap()
}

fn listing(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

/// S11 (Python: a traceback, exit 1; decided in T026/T027: Rust exits 2 with the clean message) and S12 (Python backs
/// the store up and migrates it; Rust never migrates): exit 2, stdout empty, the file byte for byte as it was, no WAL,
/// SHM or backup beside it.
#[test]
fn s11_s12_a_newer_or_older_store_is_refused_with_a_clean_message_and_not_touched() {
    let newer = golden("S11_store_newer_v12.json");
    assert_eq!(runs(&newer)[1]["exit_code"], 1, "Python: traceback, exit 1");
    let older = golden("S12_store_older_v10.json");
    assert_eq!(
        runs(&older).last().unwrap()["exit_code"],
        0,
        "Python migrates"
    );
    for (version, text) in [
        (
            12,
            "easy-personal-rs: the store is at version 12, newer than the 11 this build knows; update easy-personal-rs (nothing was changed)\n",
        ),
        (
            10,
            "easy-personal-rs: store is schema v10: run the Python app once to migrate it\n",
        ),
    ] {
        let case = Case::new();
        case.copy(&committed(), &case.projects);
        let before = marked_store(&case, version);
        let out = run(&case, &[]);
        assert_eq!(out.code, Some(2), "v{version}: {}", out.stderr);
        assert_eq!(out.stdout, "", "v{version}");
        assert_eq!(out.stderr, text, "v{version}");
        assert_eq!(
            fs::read(case.store_path()).unwrap(),
            before,
            "v{version}: byte for byte"
        );
        assert_eq!(
            listing(&case.home),
            ["ingest.lock", "store.sqlite"],
            "v{version}: no -wal, -shm or backup"
        );
    }
}

// -- privacy (criterion 11)

const CANARY_LINES: &[&str] = &[
    r#"{"type":"user","sessionId":"t029-p","timestamp":"2026-01-10T10:00:00Z","version":"2.1.400","cwd":"/nonexistent-t029/app","entrypoint":"cli","message":{"role":"user","content":"CANARY-prompt"},"toolUseResult":"CANARY-result","attachment":{"text":"CANARY-attachment"}}"#,
    r#"{"type":"user","sessionId":"t029-p","timestamp":"2026-01-10T10:00:01Z","version":"2.1.400","cwd":"/nonexistent-t029/app","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"CANARY-tooluse","is_error":true,"content":"CANARY-tool-output"},{"type":"text","text":"CANARY-user-text"}]}}"#,
    r#"{"type":"assistant","sessionId":"t029-p","timestamp":"2026-01-10T10:00:02Z","version":"2.1.400","cwd":"/nonexistent-t029/app","message":{"id":"msg_t029","model":"claude-fixture-model-1","usage":{"input_tokens":1,"output_tokens":2,"cache_creation_input_tokens":0,"cache_read_input_tokens":0},"content":[{"type":"text","text":"CANARY-answer"},{"type":"thinking","thinking":"CANARY-thinking"},{"type":"tool_use","name":"Bash","input":{"command":"ls CANARY-bash-argument"}},{"type":"tool_use","name":"WebFetch","input":{"url":"https://example.com/CANARY-path?q=CANARY-query","prompt":"CANARY-fetch-prompt"}},{"type":"tool_use","name":"Read","input":{"file_path":"/CANARY-file"}}]}}"#,
    r#"{"type":"last-prompt","sessionId":"t029-p","lastPrompt":"CANARY-last-prompt"}"#,
    r#"{"type":"custom-title","sessionId":"t029-p","customTitle":"CANARY-title"}"#,
    r#"{"type":"ai-title","sessionId":"t029-p","aiTitle":"CANARY-ai-title"}"#,
    r#"{"type":"summary","summary":"CANARY-summary","leafUuid":"CANARY-leaf"}"#,
    r#"{"type":"system","sessionId":"t029-p","timestamp":"2026-01-10T10:00:03Z","version":"2.1.400","cwd":"/nonexistent-t029/app","content":"CANARY-system"}"#,
    r#"not json CANARY-broken"#,
];

/// A transcript with a `CANARY-` text in every free-text field: after the binary ingested it, stdout, stderr, the store
/// file, its WAL and its SHM hold none (a second connection stays open, so the WAL is kept). T030 widens this.
#[test]
fn no_canary_reaches_stdout_stderr_the_store_its_wal_or_its_shm() {
    let case = Case::new();
    let path = case.projects.join("p/session.jsonl");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, CANARY_LINES.join("\n") + "\n").unwrap();
    fs::create_dir_all(&case.home).unwrap();
    drop(open_for_write(&case.store_path()).unwrap());
    let keep = Connection::open(case.store_path()).unwrap();
    // One read now attaches this connection to the WAL index, so the binary's exit is not the last close and the WAL
    // (its content) stays on disk to be scanned.
    let _: i64 = keep
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap();
    let out = run(&case, &[]);
    assert_eq!(out.code, Some(0), "{}", out.stderr);
    assert_eq!(out.stdout, expected(1, CANARY_LINES.len() as i64, 0, None));
    assert_eq!(out.stderr, "");
    for (name, text) in [("stdout", &out.stdout), ("stderr", &out.stderr)] {
        assert!(!text.contains("CANARY-"), "a canary in {name}");
    }
    let mut read = 0;
    let mut stored = false; // control: the stored session id is visible in these bytes, so a canary would be too
    for suffix in ["", "-wal", "-shm"] {
        let file = format!("{}{suffix}", case.store_path().to_str().unwrap());
        let Ok(bytes) = fs::read(&file) else { continue };
        read += 1;
        stored |= bytes.windows(6).any(|w| w == b"t029-p");
        assert!(
            !bytes.windows(7).any(|w| w == b"CANARY-"),
            "a canary in the store file{suffix}"
        );
    }
    assert_eq!(read, 3, "store, WAL and SHM all checked");
    assert!(stored, "the stored rows are in the checked bytes");
    drop(keep);
}

// -- the real Python app on the same data folder (criterion 10; ignored: needs rust/parity/.venv)

const ENTRY: &str = "import sys; from easy_personal.surfaces.cli import main; sys.exit(main())";

fn python() -> std::path::PathBuf {
    let python = repo().join("rust/parity/.venv/bin/python");
    assert!(python.exists(), "no parity venv at rust/parity/.venv");
    python
}

/// `easy-personal ingest` in the case (cwd <WORK>, the oracle's fixed env, every folder in the case): its JSON.
fn python_ingest(case: &Case) -> Value {
    let out = Command::new("timeout")
        .arg("120")
        .arg(python())
        .args(["-P", "-c", ENTRY, "ingest"])
        .current_dir(&case.work)
        .env_clear()
        .env(
            "PATH",
            std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into()),
        )
        .env("HOME", &case.nowhere)
        .env("XDG_CONFIG_HOME", case.nowhere.join(".config"))
        .env("EASY_PERSONAL_HOME", &case.home)
        .env("EASY_PERSONAL_AGENT_ROOT", &case.agents)
        .env(
            "EASY_PERSONAL_VERIFIER_REPORTS",
            case.root.join("verifier-none"),
        )
        .env("EASY_MEMORY_HOME", case.root.join("memory-none"))
        .env("EASY_PERSONAL_CONTEXT_URL", "http://127.0.0.1:9/mcp")
        .env("TZ", "UTC")
        .env("LC_ALL", "C.UTF-8")
        .env("LANG", "C.UTF-8")
        .env("PYTHONHASHSEED", "0")
        .env("PYTHONPATH", repo().join("src"))
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    let out = collect(out);
    assert_eq!(
        out.code,
        Some(0),
        "Python ingest: {}",
        case.tok(&out.stderr)
    );
    serde_json::from_str(&out.stdout).unwrap()
}

/// The bytes `json.dumps(json.loads(text), indent=2, sort_keys=True) + "\n"` gives, by the real Python.
fn python_formats(text: &str) -> String {
    let mut child = Command::new("timeout")
        .arg("60")
        .arg(python())
        .args([
            "-I",
            "-c",
            "import sys, json; print(json.dumps(json.load(sys.stdin), indent=2, sort_keys=True))",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(text.as_bytes())
        .unwrap();
    String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap()
}

#[test]
#[ignore = "needs the parity venv: rust/parity/.venv (see rust/parity/README.md)"]
fn rust_then_python_and_python_then_rust_share_one_store_and_the_second_adds_nothing() {
    // Rust first (with a retention that keeps everything, so `pruned` is in the text), then Python.
    let case = Case::new();
    case.copy(&committed(), &case.projects);
    let rust = run(&case, &["--retention-days", "36500"]);
    assert_eq!(rust.code, Some(0), "{}", rust.stderr);
    assert_eq!(rust.stdout, expected(6, 39, 0, Some(0)));
    assert_eq!(
        python_formats(&rust.stdout),
        rust.stdout,
        "the text is what Python's json.dumps(indent=2, sort_keys=True) prints"
    );
    let py = python_ingest(&case);
    println!(
        "rust then python: rust exit {:?}, python files {} new_events {}",
        rust.code, py["files"], py["new_events"]
    );
    assert_eq!(
        (py["files"].as_i64(), py["new_events"].as_i64()),
        (Some(6), Some(0))
    );
    let rows = |case: &Case| -> i64 {
        Connection::open(case.store_path())
            .unwrap()
            .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
            .unwrap()
    };
    assert_eq!(rows(&case), 39);

    // Python first, then Rust.
    let case = Case::new();
    case.copy(&committed(), &case.projects);
    let py = python_ingest(&case);
    assert_eq!(
        (py["files"].as_i64(), py["new_events"].as_i64()),
        (Some(6), Some(39))
    );
    let rust = run(&case, &[]);
    println!(
        "python then rust: python files {} new_events {}, rust exit {:?}, stdout {:?}",
        py["files"], py["new_events"], rust.code, rust.stdout
    );
    assert_eq!(rust.code, Some(0), "{}", rust.stderr);
    assert_eq!(rust.stdout, expected(6, 0, 0, None));
    assert_eq!(rows(&case), 39);
    assert_eq!(python_formats(&rust.stdout), rust.stdout);
    // And Python once more after Rust ran: still nothing new.
    let py = python_ingest(&case);
    assert_eq!(py["new_events"].as_i64(), Some(0));
}
