//! T059 (FR-091): `store_health::check` against Python's `core/store_health.check`.
//!
//! 1. The oracle: every case of `rust/parity/golden-store-health/` (made once by `gen_store_health.py`): the fixture
//!    folder is copied to a temp folder, the case's `setup` is repeated, the time budget is forced as Python forced it,
//!    and the JSON and the human lines must equal Python's.
//! 2. What the oracle cannot hold in git: a symlinked store, a live store with its `-shm`, a cancel flag, and the rule
//!    that a read never writes (file hash, mtime, `-wal` and the folder listing stay as they were).
//!
//! Every store is a temp copy; the owner's real `~/.easy-personal` is never opened.

use easy_personal_core::store_health::{Health, Options, check};
use rusqlite::Connection;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

fn golden() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/golden-store-health")
}

/// A temp folder removed on drop, also when an assertion fails.
struct Tmp(PathBuf);

impl Tmp {
    fn new(tag: &str) -> Tmp {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t059-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        Tmp(fs::canonicalize(dir).unwrap())
    }
}

impl std::ops::Deref for Tmp {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn cancel() -> Arc<AtomicBool> {
    Arc::default()
}

/// What `check` reads from a manifest entry: Python's `budget_s`, `full`, `full_check_max_bytes`.
fn options(case: &Value) -> Options {
    let mut options = Options::default();
    let given = &case["options"];
    if let Some(s) = given["budget_s"].as_f64() {
        options.budget = Duration::from_secs_f64(s);
    }
    if let Some(full) = given["full"].as_bool() {
        options.full = full;
    }
    if let Some(n) = given["full_check_max_bytes"].as_u64() {
        options.full_check_max_bytes = n;
    }
    options
}

fn listing(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

fn copy_dir(from: &Path, to: &Path) {
    // an empty fixture folder (`missing`) is not kept by git, so a fresh checkout has no such folder: it reads as empty
    let Ok(entries) = fs::read_dir(from) else {
        assert!(!from.exists(), "{from:?} cannot be read");
        return;
    };
    for entry in entries {
        let entry = entry.unwrap();
        fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
    }
}

/// The bytes that must not change in a read: every file of the folder with its mtime.
fn fingerprint(dir: &Path) -> Vec<(String, Vec<u8>, std::time::SystemTime)> {
    listing(dir)
        .into_iter()
        .map(|name| {
            let path = dir.join(&name);
            (
                name,
                fs::read(&path).unwrap(),
                fs::metadata(&path).unwrap().modified().unwrap(),
            )
        })
        .collect()
}

#[test]
fn every_golden_case_equals_python() {
    let dir = golden();
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(dir.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(
        manifest["latest_migration"],
        easy_personal_core::store::KNOWN_MIGRATION
    );
    let cases = manifest["cases"].as_array().unwrap();
    assert!(cases.len() >= 26, "the oracle holds {} cases", cases.len());
    let mut failures = Vec::new();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let home = Tmp::new(name);
        copy_dir(&dir.join(case["fixture"].as_str().unwrap()), &home);
        // a case `setup` is repeated here as Python repeated it
        let holder = (case["setup"] == "lock").then(|| {
            let conn = Connection::open(home.join("store.sqlite")).unwrap();
            conn.execute_batch("PRAGMA locking_mode=EXCLUSIVE").unwrap();
            conn.query_row("SELECT count(*) FROM meta", [], |r| r.get::<_, i64>(0))
                .unwrap();
            conn
        });
        let before = fingerprint(&home);
        let health = check(&home.join("store.sqlite"), &options(case), &cancel());
        let expected: Value =
            serde_json::from_str(&fs::read_to_string(dir.join(format!("{name}.json"))).unwrap())
                .unwrap();
        let expected_lines = fs::read_to_string(dir.join(format!("{name}.txt"))).unwrap();
        if health.json() != expected {
            failures.push(format!(
                "{name}: JSON differs\n  rust:   {}\n  python: {expected}",
                health.json()
            ));
        }
        if health.lines().join("\n") + "\n" != expected_lines {
            failures.push(format!(
                "{name}: lines differ\n  rust:\n{}\n  python:\n{expected_lines}",
                health.lines().join("\n")
            ));
        }
        drop(holder);
        let after = fingerprint(&home);
        // no write and no side file (the lock holder's own files are not the check's: only the locked case has them)
        if case["setup"].is_null() && before != after {
            failures.push(format!(
                "{name}: the check changed a file or added one: {:?} -> {:?}",
                listing(&home),
                before.iter().map(|b| b.0.clone()).collect::<Vec<_>>()
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} case(s) differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn exit_codes_follow_the_verdict() {
    let dir = golden();
    let code = |fixture: &str, o: Options| {
        let home = Tmp::new("exit");
        copy_dir(&dir.join(fixture), &home);
        check(&home.join("store.sqlite"), &o, &cancel()).exit_code()
    };
    assert_eq!(code("clean", Options::default()), 0);
    assert_eq!(code("behind", Options::default()), 1);
    assert_eq!(code("zero_byte", Options::default()), 1);
    assert_eq!(
        code(
            "big",
            Options {
                budget: Duration::ZERO,
                ..Options::default()
            }
        ),
        3
    );
    assert_eq!(
        code(
            "big",
            Options {
                full: true,
                budget: Duration::ZERO,
                ..Options::default()
            }
        ),
        0
    );
    // a problem found wins over a check that did not finish
    assert_eq!(
        code(
            "behind",
            Options {
                budget: Duration::ZERO,
                ..Options::default()
            }
        ),
        1
    );
}

#[test]
fn cancel_stops_every_running_read() {
    let home = Tmp::new("cancel");
    copy_dir(&golden().join("big"), &home);
    let stop = Arc::new(AtomicBool::new(true));
    let health = check(
        &home.join("store.sqlite"),
        &Options {
            full: true,
            ..Options::default()
        },
        &stop,
    );
    assert!(
        health.incomplete.contains(&"integrity_check"),
        "{:?}",
        health.incomplete
    );
    assert_eq!(health.healthy, None);
    assert_eq!(health.exit_code(), 3);
}

#[test]
fn a_cancel_set_while_the_check_runs_stops_it() {
    let home = Tmp::new("cancel-late");
    // 400k rows: integrity_check needs a second or more; the flag is set after 50 ms
    let store = home.join("store.sqlite");
    fs::copy(golden().join("clean").join("store.sqlite"), &store).unwrap();
    let conn = Connection::open(&store).unwrap();
    conn.execute_batch(
        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 400000)
         INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id, project_path, model, message_id, input_tokens, output_tokens)
         SELECT 'm', 'claude_code', '/big', 1, i, 0, '2026-01-01T00:00:00.000000Z', 's' || (i % 500), '/w/p', 'model-0', 'id' || i, 10, 5 FROM n;
         PRAGMA wal_checkpoint(TRUNCATE);",
    )
    .unwrap();
    drop(conn);
    let stop = cancel();
    let flag = Arc::clone(&stop);
    let timer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        flag.store(true, Ordering::SeqCst);
    });
    let started = std::time::Instant::now();
    let health = check(
        &store,
        &Options {
            full: true,
            ..Options::default()
        },
        &stop,
    );
    timer.join().unwrap();
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "stopped late: {:?}",
        started.elapsed()
    );
    assert!(
        health.incomplete.contains(&"integrity_check"),
        "{:?}",
        health.incomplete
    );
    assert_eq!(health.healthy, None);
}

#[test]
fn a_symlinked_store_reads_the_wal_beside_its_target_and_creates_nothing() {
    let home = Tmp::new("link");
    let real = home.join("real");
    fs::create_dir(&real).unwrap();
    fs::copy(
        golden().join("clean").join("store.sqlite"),
        real.join("store.sqlite"),
    )
    .unwrap();
    let writer = Connection::open(real.join("store.sqlite")).unwrap();
    writer.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; INSERT INTO meta (key, value) VALUES ('k', 'v')").unwrap();
    let links = home.join("links");
    fs::create_dir(&links).unwrap();
    std::os::unix::fs::symlink(real.join("store.sqlite"), links.join("store.sqlite")).unwrap();
    let health = check(&links.join("store.sqlite"), &Options::default(), &cancel());
    assert!(
        health.wal_bytes > 0,
        "the -wal beside the target is measured"
    );
    let meta = health
        .tables
        .as_ref()
        .unwrap()
        .iter()
        .find(|(n, _)| n == "meta")
        .unwrap()
        .1;
    assert_eq!(meta, Some(2), "the row that lives only in the WAL is seen");
    assert_eq!(
        listing(&links),
        vec!["store.sqlite"],
        "nothing is created beside the link"
    );
    drop(writer);
}

#[test]
fn a_live_store_is_read_with_its_wal_and_nothing_changes() {
    let home = Tmp::new("live");
    let store = home.join("store.sqlite");
    fs::copy(golden().join("clean").join("store.sqlite"), &store).unwrap();
    let writer = Connection::open(&store).unwrap();
    writer.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; INSERT INTO meta (key, value) VALUES ('k', 'v')").unwrap();
    assert_eq!(
        listing(&home),
        vec!["store.sqlite", "store.sqlite-shm", "store.sqlite-wal"]
    );
    let before = fingerprint(&home);
    let health: Health = check(&store, &Options::default(), &cancel());
    let meta = health
        .tables
        .as_ref()
        .unwrap()
        .iter()
        .find(|(n, _)| n == "meta")
        .unwrap()
        .1;
    assert_eq!(meta, Some(2));
    assert_eq!(health.healthy, Some(true));
    // the -shm of a live store is written by SQLite itself when a reader attaches; the store and the -wal are not
    let after = fingerprint(&home);
    for name in ["store.sqlite", "store.sqlite-wal"] {
        assert!(
            before.iter().find(|b| b.0 == name) == after.iter().find(|b| b.0 == name),
            "{name} changed"
        );
    }
    assert_eq!(
        listing(&home),
        vec!["store.sqlite", "store.sqlite-shm", "store.sqlite-wal"]
    );
    drop(writer);
}

#[test]
fn a_private_copy_is_removed_after_the_check() {
    let home = Tmp::new("copy");
    copy_dir(&golden().join("wal_no_shm"), &home);
    let mine = format!("easy-personal-check-{}-", std::process::id());
    let leftovers = || {
        fs::read_dir(std::env::temp_dir())
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(&mine)
            })
            .count()
    };
    assert_eq!(leftovers(), 0);
    let health = check(&home.join("store.sqlite"), &Options::default(), &cancel());
    assert!(health.readable);
    assert_eq!(
        leftovers(),
        0,
        "the temp folder with the copy of the store is gone"
    );
    assert_eq!(listing(&home), vec!["store.sqlite", "store.sqlite-wal"]);
}

#[test]
fn a_store_path_with_odd_characters_is_opened() {
    // `?`, `#`, `%` and a space would end or change a file URI if they were not encoded
    let home = Tmp::new("odd");
    let dir = home.join("a b?c#d%e");
    fs::create_dir(&dir).unwrap();
    copy_dir(&golden().join("clean"), &dir);
    let health = check(&dir.join("store.sqlite"), &Options::default(), &cancel());
    assert!(health.readable);
    assert_eq!(health.healthy, Some(true));
}

#[test]
fn a_directory_where_the_store_should_be_is_not_a_store() {
    let home = Tmp::new("dir");
    fs::create_dir(home.join("store.sqlite")).unwrap();
    let health = check(&home.join("store.sqlite"), &Options::default(), &cancel());
    assert!(!health.exists);
    assert_eq!(health.healthy, Some(false));
    assert_eq!(listing(&home), vec!["store.sqlite"]);
}

#[test]
fn nothing_in_the_answer_names_a_folder() {
    let home = Tmp::new("secret-folder-name");
    copy_dir(&golden().join("backups"), &home);
    let health = check(&home.join("store.sqlite"), &Options::default(), &cancel());
    let text = health.json().to_string() + &health.lines().join("\n");
    assert!(
        !text.contains("secret-folder-name") && !text.contains(home.to_str().unwrap()),
        "{text}"
    );
    assert!(
        !text.contains("CANARY"),
        "a lookalike backup name is not listed"
    );
}
