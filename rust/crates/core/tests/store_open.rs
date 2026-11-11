//! T013: `store::open_checked` and `clock::now_micros`.
//! Stores are temp copies of `rust/parity/store-v11.sqlite` (read-only source); `~/.easy-personal` is never opened.

use easy_personal_core::clock::now_micros;
use easy_personal_core::store::{KNOWN_MIGRATION, StoreError, open_checked};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t013-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    /// A folder holding a copy of the fixture store.
    fn with_fixture() -> Self {
        let dir = TempDir::new();
        fs::copy(fixture(), dir.store()).unwrap();
        dir
    }

    fn store(&self) -> PathBuf {
        self.0.join("store.sqlite")
    }

    fn path_text(&self) -> &str {
        self.0.to_str().unwrap()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/store-v11.sqlite")
}

/// Names of everything in the folder, sorted.
fn listing(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn a_missing_store_is_no_store_and_creates_nothing() {
    let dir = TempDir::new();
    let err = open_checked(&dir.store()).unwrap_err();
    assert!(matches!(err, StoreError::NoStore), "{err:?}");
    assert_eq!(
        err.to_string(),
        "no store yet: nothing has been ingested into the data folder"
    );
    assert_eq!(err.exit_code(), 1);
    assert!(!err.to_string().contains(dir.path_text()));
    assert!(
        listing(&dir.0).is_empty(),
        "no store, -wal or -shm was created"
    );
}

#[test]
fn a_directory_in_place_of_the_store_is_not_a_file() {
    let dir = TempDir::new();
    fs::create_dir(dir.store()).unwrap();
    let err = open_checked(&dir.store()).unwrap_err();
    assert!(matches!(err, StoreError::NotAFile), "{err:?}");
    assert_eq!(err.to_string(), "the store path is not a regular file");
    assert!(!err.to_string().contains(dir.path_text()));
}

#[test]
fn a_store_newer_than_the_build_is_refused_with_todays_text() {
    let dir = TempDir::with_fixture();
    {
        let conn = Connection::open(dir.store()).unwrap();
        conn.execute_batch("PRAGMA user_version = 99").unwrap();
    }
    let before = fs::read(dir.store()).unwrap();
    let err = open_checked(&dir.store()).unwrap_err();
    assert!(
        matches!(
            err,
            StoreError::Newer {
                found: 99,
                known: KNOWN_MIGRATION
            }
        ),
        "{err:?}"
    );
    assert_eq!(err.exit_code(), 2);
    assert_eq!(
        err.to_string(),
        "the store is at version 99, newer than the 11 this build knows; update easy-personal-rs (nothing was changed)"
    );
    assert!(!err.to_string().contains(dir.path_text()));
    assert_eq!(fs::read(dir.store()).unwrap(), before, "bytes unchanged");
}

#[test]
fn the_fixture_opens_read_only_and_its_bytes_stay_the_same() {
    let dir = TempDir::with_fixture();
    let before = fs::read(dir.store()).unwrap();
    let conn = open_checked(&dir.store()).unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, KNOWN_MIGRATION);
    let events: i64 = conn
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap();
    assert!(events > 0, "the fixture has events");
    assert!(
        conn.execute("DELETE FROM events", []).is_err(),
        "the connection cannot write"
    );
    drop(conn);
    assert_eq!(fs::read(dir.store()).unwrap(), before, "bytes unchanged");
}

#[cfg(unix)]
#[test]
fn a_symlink_to_a_regular_file_counts_as_a_file() {
    let dir = TempDir::with_fixture();
    let link_dir = TempDir::new();
    let link = link_dir.store();
    std::os::unix::fs::symlink(dir.store(), &link).unwrap();
    assert!(open_checked(&link).is_ok());
}

#[test]
fn a_file_that_is_not_a_database_errors_without_the_path() {
    let dir = TempDir::new();
    fs::write(dir.store(), b"this is not sqlite at all, just some text").unwrap();
    let err = open_checked(&dir.store()).unwrap_err();
    assert!(!err.to_string().contains(dir.path_text()), "{err}");
}

#[test]
fn now_micros_is_utc_microseconds_close_to_the_system_clock() {
    let before = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let got = now_micros();
    let after = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let (lo, hi) = (before.as_micros() as i64, after.as_micros() as i64);
    let slack = 5_000_000; // a few seconds
    assert!(
        got >= lo - slack && got <= hi + slack,
        "{got} not within {slack} us of [{lo}, {hi}]"
    );
}
