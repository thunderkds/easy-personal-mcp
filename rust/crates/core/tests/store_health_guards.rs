//! T059 fix round (Stage 4 review): the guards of `store_health::check` that the oracle cannot hold.
//!
//! - A cancel or a deadline is honoured between short statements (a store with very many tiny tables).
//! - A `-wal` or `-shm` that is a FIFO or a symlink is never opened or copied.
//! - The sweep of private copies that a killed process left behind.
//! - The caps on the listed tables, sources and backups.
//!
//! Every store is a temp copy of an oracle fixture; the owner's real `~/.easy-personal` is never opened.

use easy_personal_core::store_health::{LIST_CAP, Options, check, sweep_stale_copies};
use rusqlite::Connection;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

fn clean_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity/golden-store-health/clean/store.sqlite")
}

struct Tmp(PathBuf);

impl Tmp {
    fn new(tag: &str) -> Tmp {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t059g-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        Tmp(fs::canonicalize(dir).unwrap())
    }

    fn store(&self) -> PathBuf {
        let store = self.0.join("store.sqlite");
        fs::copy(clean_fixture(), &store).unwrap();
        store
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The tests that count or look at the temp folder's private copies run one at a time.
static COPIES: Mutex<()> = Mutex::new(());

fn no_cancel() -> Arc<AtomicBool> {
    Arc::default()
}

/// `count` tables that share the root page of one table with 40,000 rows: a `CREATE TABLE` per table would take minutes (the
/// schema is rewritten each time), and a `COUNT(*)` of each walks its 1,000 pages, so the counts are slow enough to see.
fn many_tables(store: &Path, count: usize) {
    let conn = Connection::open(store).unwrap();
    conn.execute_batch(
        "CREATE TABLE base (a);
         WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 40000)
         INSERT INTO base SELECT printf('%0100d', i) FROM n;",
    )
    .unwrap();
    let root: i64 = conn
        .query_row(
            "SELECT rootpage FROM sqlite_master WHERE name = 'base'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    conn.execute_batch(&format!(
        "PRAGMA writable_schema=ON;
         WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < {count})
         INSERT INTO sqlite_master (type, name, tbl_name, rootpage, sql)
         SELECT 'table', 't' || i, 't' || i, {root}, 'CREATE TABLE t' || i || ' (a)' FROM n;
         PRAGMA writable_schema=OFF;
         PRAGMA wal_checkpoint(TRUNCATE);"
    ))
    .unwrap();
}

#[test]
fn a_deadline_stops_the_table_counts_of_a_store_with_very_many_tables() {
    let home = Tmp::new("tables-deadline");
    let store = home.store();
    many_tables(&store, 20_000);
    let started = Instant::now();
    let health = check(
        &store,
        &Options {
            budget: Duration::from_millis(400),
            ..Options::default()
        },
        &no_cancel(),
    );
    let took = started.elapsed();
    assert!(took < Duration::from_secs(2), "stopped late: {took:?}");
    // which part the deadline cut first depends on the machine's load (the loops themselves are proved by the `loop` tests and
    // the unit tests of `store_health.rs`); something must be cut, and the check must not run on
    assert!(!health.incomplete.is_empty(), "nothing was cut off");
}

#[test]
fn a_cancel_stops_the_table_counts_of_a_store_with_very_many_tables() {
    let home = Tmp::new("tables-cancel");
    let store = home.store();
    many_tables(&store, 20_000);
    let stop = no_cancel();
    let flag = Arc::clone(&stop);
    let timer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        flag.store(true, Ordering::SeqCst);
    });
    let started = Instant::now();
    let health = check(
        &store,
        &Options {
            full: true,
            ..Options::default()
        },
        &stop,
    );
    timer.join().unwrap();
    let took = started.elapsed();
    assert!(took < Duration::from_secs(2), "stopped late: {took:?}");
    assert!(
        !health.incomplete.is_empty(),
        "nothing was cut off: {:?}",
        health.incomplete
    );
}

fn mkfifo(path: &Path) {
    assert!(
        std::process::Command::new("mkfifo")
            .arg(path)
            .status()
            .unwrap()
            .success()
    );
}

/// The temp folders of private copies that this process made (their names carry its pid).
fn copies_left() -> usize {
    let mine = format!("easy-personal-check-{}-", std::process::id());
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
}

#[test]
fn a_fifo_wal_is_not_opened_and_the_check_returns() {
    let _one_at_a_time = COPIES.lock().unwrap_or_else(|e| e.into_inner());
    let home = Tmp::new("fifo-wal");
    let store = home.store();
    mkfifo(&home.0.join("store.sqlite-wal"));
    let started = Instant::now();
    let health = check(&store, &Options::default(), &no_cancel());
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(health.unreadable, vec!["file"]);
    assert_eq!(health.healthy, Some(false));
    assert_eq!(health.exit_code(), 1);
    assert_eq!(
        health.wal_bytes, 0,
        "no size is read from a side file that is not a regular file"
    );
    assert!(
        health
            .lines()
            .join("\n")
            .contains("side file is not a regular file")
    );
    assert_eq!(copies_left(), 0);
}

#[test]
fn a_fifo_shm_is_not_opened_either() {
    let home = Tmp::new("fifo-shm");
    let store = home.store();
    fs::write(home.0.join("store.sqlite-wal"), b"").unwrap();
    mkfifo(&home.0.join("store.sqlite-shm"));
    let started = Instant::now();
    let health = check(&store, &Options::default(), &no_cancel());
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(health.unreadable, vec!["file"]);
}

#[test]
fn a_symlinked_wal_is_not_copied_and_its_size_is_not_read() {
    let _one_at_a_time = COPIES.lock().unwrap_or_else(|e| e.into_inner());
    let home = Tmp::new("link-wal");
    let store = home.store();
    let elsewhere = home.0.join("elsewhere");
    fs::write(&elsewhere, vec![7u8; 5000]).unwrap();
    std::os::unix::fs::symlink(&elsewhere, home.0.join("store.sqlite-wal")).unwrap();
    let before = fs::read_dir(&home.0).unwrap().count();
    let health = check(&store, &Options::default(), &no_cancel());
    assert_eq!(health.unreadable, vec!["file"]);
    assert_eq!(health.wal_bytes, 0, "not the 5000 bytes behind the link");
    assert_eq!(health.healthy, Some(false));
    assert_eq!(fs::read_dir(&home.0).unwrap().count(), before);
    assert_eq!(copies_left(), 0);
    let text = health.json().to_string() + &health.lines().join("\n");
    assert!(!text.contains("elsewhere") && !text.contains(home.0.to_str().unwrap()));
}

#[test]
fn a_private_copy_is_made_with_mode_0600_in_a_0700_folder() {
    let _one_at_a_time = COPIES.lock().unwrap_or_else(|e| e.into_inner());
    // the copy is removed before `check` returns, so look at it from inside: a wal-no-shm store, many tables (slow enough is not
    // needed): the permission bits are checked on the folder the sweep recognises, which `check` itself creates as 0700
    let home = Tmp::new("modes");
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/golden-store-health/wal_no_shm");
    for name in ["store.sqlite", "store.sqlite-wal"] {
        fs::copy(fixture.join(name), home.0.join(name)).unwrap();
    }
    let seen = Arc::new(AtomicBool::new(false));
    let watcher = {
        let seen = Arc::clone(&seen);
        std::thread::spawn(move || {
            let mine = format!("easy-personal-check-{}-", std::process::id());
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline && !seen.load(Ordering::SeqCst) {
                for entry in fs::read_dir(std::env::temp_dir()).unwrap().flatten() {
                    if !entry.file_name().to_string_lossy().starts_with(&mine) {
                        continue;
                    }
                    let folder =
                        fs::metadata(entry.path()).map(|m| m.permissions().mode() & 0o7777);
                    let files: Vec<u32> = fs::read_dir(entry.path())
                        .map(|d| {
                            d.flatten()
                                .filter(|f| {
                                    f.file_name().to_string_lossy() == "store.sqlite"
                                        || f.file_name().to_string_lossy() == "store.sqlite-wal"
                                })
                                .filter_map(|f| fs::metadata(f.path()).ok())
                                .map(|m| m.permissions().mode() & 0o7777)
                                .collect()
                        })
                        .unwrap_or_default();
                    if let (Ok(folder), 2) = (folder, files.len()) {
                        assert_eq!(folder, 0o700);
                        assert!(
                            files.iter().all(|m| *m == 0o600),
                            "{:?}",
                            files.iter().map(|m| format!("{m:o}")).collect::<Vec<_>>()
                        );
                        seen.store(true, Ordering::SeqCst);
                    }
                }
            }
        })
    };
    // 30 runs: one of them is caught while the copy exists
    for _ in 0..30 {
        if seen.load(Ordering::SeqCst) {
            break;
        }
        let health = check(
            &home.0.join("store.sqlite"),
            &Options::default(),
            &no_cancel(),
        );
        assert!(health.readable);
    }
    watcher.join().unwrap();
    assert!(
        seen.load(Ordering::SeqCst),
        "the private copy was never seen"
    );
}

fn plant_check_dir(temp: &Path, name: &str, mode: u32, age: Duration) -> PathBuf {
    let dir = temp.join(name);
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("store.sqlite"), b"x").unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(mode)).unwrap();
    fs::File::open(&dir)
        .unwrap()
        .set_modified(SystemTime::now() - age)
        .unwrap();
    dir
}

#[test]
fn the_sweep_removes_old_private_copies_of_this_user_and_nothing_else() {
    let temp = Tmp::new("sweep");
    let old = plant_check_dir(
        &temp.0,
        "easy-personal-check-1-2-3",
        0o700,
        Duration::from_secs(3600),
    );
    let young = plant_check_dir(
        &temp.0,
        "easy-personal-check-4-5-6",
        0o700,
        Duration::from_secs(60),
    );
    let wrong_mode = plant_check_dir(
        &temp.0,
        "easy-personal-check-7-8-9",
        0o755,
        Duration::from_secs(3600),
    );
    let wrong_name = plant_check_dir(
        &temp.0,
        "easy-personal-check-evil",
        0o700,
        Duration::from_secs(3600),
    );
    let other_tool = plant_check_dir(
        &temp.0,
        "other-tool-1-2-3",
        0o700,
        Duration::from_secs(3600),
    );
    // a symlink with the right name, to a folder with the right look: never followed, never removed
    let target = plant_check_dir(&temp.0, "not-a-copy", 0o700, Duration::from_secs(3600));
    std::os::unix::fs::symlink(&target, temp.0.join("easy-personal-check-10-11-12")).unwrap();
    let removed = sweep_stale_copies(&temp.0, Duration::from_secs(600));
    assert_eq!(removed, 1);
    assert!(!old.exists(), "an old copy of this user is removed");
    for kept in [&young, &wrong_mode, &wrong_name, &other_tool, &target] {
        assert!(kept.exists(), "{kept:?} must stay");
    }
    assert!(temp.0.join("easy-personal-check-10-11-12").is_symlink());
    assert!(target.join("store.sqlite").exists());
}

#[test]
fn a_check_sweeps_the_old_copies_in_the_temp_folder() {
    // the real temp folder: only a folder with this test's own unlikely pid-free name pattern is planted and removed
    let old = plant_check_dir(
        &std::env::temp_dir(),
        &format!("easy-personal-check-{}-1-1", std::process::id() + 1_000_000),
        0o700,
        Duration::from_secs(7200),
    );
    let home = Tmp::new("sweep-check");
    let store = home.store();
    check(&store, &Options::default(), &no_cancel());
    assert!(!old.exists(), "check() sweeps the stale copies");
}

#[test]
fn the_listed_tables_sources_and_backups_are_capped_with_a_count_of_the_rest() {
    let home = Tmp::new("caps");
    let store = home.store();
    {
        let conn = Connection::open(&store).unwrap();
        let mut sql = String::from("BEGIN;");
        for n in 0..250 {
            sql += &format!("CREATE TABLE t{n:04} (a);");
            sql += &format!(
                "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc) VALUES ('m', 's{n:04}', '/y', 1, {n}, 0, '2026-01-01T00:00:00.000000Z');"
            );
        }
        sql += "COMMIT; PRAGMA wal_checkpoint(TRUNCATE);";
        conn.execute_batch(&sql).unwrap();
    }
    for n in 0..250 {
        fs::write(home.0.join(format!("store.sqlite.bak-v{n}")), b"x").unwrap();
    }
    assert_eq!(LIST_CAP, 200);
    let health = check(&store, &Options::default(), &no_cancel());
    assert_eq!(health.tables.as_ref().unwrap().len(), LIST_CAP);
    assert_eq!(health.sources.as_ref().unwrap().len(), LIST_CAP);
    assert_eq!(health.backups.len(), LIST_CAP);
    // 15 schema tables + 250 added = 265 tables: 65 more; 5 fixed + 250 + the fixture's claude_code = 255 sources: 55 more
    let json = health.json();
    assert_eq!(json["tables_more"], 65);
    assert_eq!(json["sources_more"], 55);
    assert_eq!(json["backups_more"], 50);
    let text = health.lines().join("\n");
    assert!(
        text.contains("(65 more not shown)")
            && text.contains("(55 more not shown)")
            && text.contains("(50 more not shown)"),
        "{text}"
    );
    // the lowest backup versions are the ones listed
    assert_eq!(health.backups[0], "store.sqlite.bak-v0");
    // under the cap nothing is added to the JSON (the oracle's goldens have none of these keys)
    let small = Tmp::new("caps-small");
    let small_store = small.store();
    let json = check(&small_store, &Options::default(), &no_cancel()).json();
    assert!(
        json.get("tables_more").is_none()
            && json.get("sources_more").is_none()
            && json.get("backups_more").is_none()
    );
}

/// 70 real tables (85 with the schema's own): more than the 64 rounds between two looks at the deadline, and few enough that
/// SQLite loads the schema in under 1000 VM steps, so the progress handler does not stop the read before the loop.
fn seventy_tables(store: &Path) {
    let conn = Connection::open(store).unwrap();
    let mut sql = String::from("BEGIN;");
    for n in 0..70 {
        sql += &format!("CREATE TABLE t{n:03} (a);");
    }
    sql += "COMMIT; PRAGMA wal_checkpoint(TRUNCATE);";
    conn.execute_batch(&sql).unwrap();
}

#[test]
fn a_deadline_that_has_passed_stops_the_loop_of_short_table_counts() {
    // each COUNT is one short statement: SQLite's progress handler never sees them, so the loop must look itself
    let home = Tmp::new("loop-deadline");
    let store = home.store();
    seventy_tables(&store);
    let health = check(
        &store,
        &Options {
            budget: Duration::ZERO,
            ..Options::default()
        },
        &no_cancel(),
    );
    assert!(
        health.incomplete.contains(&"tables"),
        "{:?}",
        health.incomplete
    );
    assert_eq!(health.tables, None);
    assert_eq!(health.healthy, None);
}

#[test]
fn a_cancel_stops_the_loop_of_short_table_counts_at_once() {
    let home = Tmp::new("loop-cancel");
    let store = home.store();
    seventy_tables(&store);
    let stop = Arc::new(AtomicBool::new(true));
    let health = check(
        &store,
        &Options {
            full: true,
            ..Options::default()
        },
        &stop,
    );
    assert!(
        health.incomplete.contains(&"tables"),
        "{:?}",
        health.incomplete
    );
    assert!(
        health.incomplete.contains(&"sources"),
        "{:?}",
        health.incomplete
    );
    assert_eq!(health.tables, None);
}

#[test]
fn below_the_round_limit_a_passed_deadline_changes_nothing_as_python() {
    // the oracle's `budget_zero` case: a store with few tables still gets its counts (Python's answer)
    let home = Tmp::new("loop-small");
    let store = home.store();
    let health = check(
        &store,
        &Options {
            budget: Duration::ZERO,
            ..Options::default()
        },
        &no_cancel(),
    );
    assert!(health.tables.is_some());
}
