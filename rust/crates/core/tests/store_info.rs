//! Fixture stores are built here from SQL in a temp folder; the owner's real `~/.easy-personal` is never opened.

use easy_personal_core::store::{
    KNOWN_MIGRATION, State, StoreError, open_read_only, read_info, store_path_from,
};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t002-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn store(&self) -> PathBuf {
        self.0.join("store.sqlite")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A store with two tables (3 and 0 rows) at the given `user_version`.
fn make_store(path: &Path, user_version: i64) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(&format!(
        "CREATE TABLE events (id INTEGER PRIMARY KEY, note TEXT);
         INSERT INTO events (note) VALUES ('secret-a'), ('secret-b'), ('secret-c');
         CREATE TABLE meta (key TEXT);
         PRAGMA user_version = {user_version};"
    ))
    .unwrap();
}

/// Every file in the folder with its bytes, sorted by name.
fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                fs::read(e.path()).unwrap(),
            )
        })
        .collect();
    files.sort();
    files
}

#[test]
fn known_migration_is_the_last_python_migration() {
    assert_eq!(KNOWN_MIGRATION, 11);
}

#[test]
fn version_11_reports_counts_and_no_pending() {
    let dir = TempDir::new();
    make_store(&dir.store(), 11);
    let info = read_info(&dir.store()).unwrap();
    assert_eq!(info.file, "store.sqlite");
    match info.state {
        State::Present {
            user_version,
            ref tables,
        } => {
            assert_eq!(user_version, 11);
            assert_eq!(
                tables,
                &vec![("events".to_string(), 3), ("meta".to_string(), 0)]
            );
        }
        State::Missing => panic!("store exists"),
    }
    let json = info.json();
    assert_eq!(
        json,
        r#"{"file":"store.sqlite","exists":true,"user_version":11,"known":11,"pending":false,"tables":{"events":3,"meta":0}}"#
    );
    let text = info.text();
    assert!(text.contains("pending: no"), "{text}");
    assert!(!text.contains("secret"), "no row content: {text}");
    assert!(
        !text.contains(dir.0.to_str().unwrap()),
        "no absolute path: {text}"
    );
}

#[test]
fn older_store_is_pending() {
    let dir = TempDir::new();
    make_store(&dir.store(), 9);
    let info = read_info(&dir.store()).unwrap();
    assert!(
        info.json()
            .contains(r#""user_version":9,"known":11,"pending":true"#)
    );
    assert!(info.text().contains("pending: yes"));
}

#[test]
fn version_12_is_refused_and_nothing_is_written() {
    let dir = TempDir::new();
    make_store(&dir.store(), 12);
    let before = snapshot(&dir.0);
    let err = read_info(&dir.store()).unwrap_err();
    assert!(
        matches!(
            err,
            StoreError::Newer {
                found: 12,
                known: 11
            }
        ),
        "{err:?}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("12") && msg.contains("11"),
        "message names both numbers: {msg}"
    );
    assert!(
        !msg.contains(dir.0.to_str().unwrap()),
        "no absolute path: {msg}"
    );
    assert_eq!(
        snapshot(&dir.0),
        before,
        "store bytes and folder listing unchanged"
    );
}

#[test]
fn missing_store_is_no_data_and_creates_nothing() {
    let dir = TempDir::new();
    let info = read_info(&dir.store()).unwrap();
    assert!(matches!(info.state, State::Missing));
    assert!(info.text().contains("no data"));
    assert_eq!(info.json(), r#"{"file":"store.sqlite","exists":false}"#);
    assert!(!dir.store().exists());
    assert!(snapshot(&dir.0).is_empty(), "no file created");
}

#[test]
fn directory_in_place_of_the_store_is_an_error() {
    let dir = TempDir::new();
    fs::create_dir(dir.store()).unwrap();
    let err = read_info(&dir.store()).unwrap_err();
    assert!(matches!(err, StoreError::NotAFile), "{err:?}");
    assert!(!err.to_string().contains(dir.0.to_str().unwrap()));
}

#[test]
fn file_that_is_not_a_database_is_an_error_not_a_panic() {
    let dir = TempDir::new();
    fs::write(
        dir.store(),
        b"this is not sqlite at all, just some text to fill a page",
    )
    .unwrap();
    let before = snapshot(&dir.0);
    assert!(read_info(&dir.store()).is_err());
    assert_eq!(snapshot(&dir.0), before);
}

#[test]
fn wal_files_next_to_the_store_are_left_alone() {
    let dir = TempDir::new();
    let path = dir.store();
    // Python's settings: WAL. Keep the writer open so -wal and -shm stay on disk, with an
    // uncheckpointed row only the WAL holds.
    let writer = Connection::open(&path).unwrap();
    writer
        .execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;")
        .unwrap();
    writer
        .execute_batch(
            "CREATE TABLE events (id INTEGER PRIMARY KEY, note TEXT);
             INSERT INTO events (note) VALUES ('x'), ('y');
             PRAGMA user_version = 11;",
        )
        .unwrap();
    let wal = dir.0.join("store.sqlite-wal");
    assert!(
        wal.exists() && fs::metadata(&wal).unwrap().len() > 0,
        "fixture has a live WAL"
    );
    let (db_before, wal_before) = (fs::read(&path).unwrap(), fs::read(&wal).unwrap());

    let info = read_info(&path).unwrap();
    assert!(
        info.json().contains(r#""tables":{"events":2}"#),
        "reads the WAL content: {}",
        info.json()
    );

    assert_eq!(
        fs::read(&path).unwrap(),
        db_before,
        "no checkpoint into the store file"
    );
    assert_eq!(
        fs::read(&wal).unwrap(),
        wal_before,
        "WAL not truncated or changed"
    );
    drop(writer);
}

#[test]
fn clean_wal_store_keeps_its_main_file_and_leaves_only_empty_sidecars() {
    // SQLite itself must create `-shm` (and an empty `-wal`) to read a WAL store, even read-only, and a
    // read-only connection cannot delete them on close. They hold no data; the next writer reuses them.
    let dir = TempDir::new();
    let path = dir.store();
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; CREATE TABLE events (id INTEGER); PRAGMA user_version = 11;",
        )
        .unwrap();
    }
    let before = snapshot(&dir.0);
    assert_eq!(before.len(), 1, "a cleanly closed WAL store is one file");
    read_info(&path).unwrap();
    let after = snapshot(&dir.0);
    assert_eq!(after[0], before[0], "main store file bytes unchanged");
    for (name, bytes) in &after[1..] {
        assert!(
            name == "store.sqlite-shm" || name == "store.sqlite-wal",
            "unexpected file {name}"
        );
        if name.ends_with("-wal") {
            assert!(bytes.is_empty(), "the WAL stays empty");
        }
    }
}

#[test]
fn connection_is_read_only_and_a_write_attempt_fails() {
    let dir = TempDir::new();
    make_store(&dir.store(), 11);
    let before = snapshot(&dir.0);
    let conn = open_read_only(&dir.store()).unwrap();
    assert!(
        conn.execute("INSERT INTO events (note) VALUES ('nope')", [])
            .is_err()
    );
    assert!(conn.execute("CREATE TABLE sneaky (x)", []).is_err());
    assert!(conn.execute("DELETE FROM events", []).is_err());
    drop(conn);
    assert_eq!(snapshot(&dir.0), before, "file unchanged");
    let rows: i64 = Connection::open(dir.store())
        .unwrap()
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 3);
}

#[test]
fn store_path_follows_the_python_override_variable() {
    // Pure function over the two inputs, so no test touches the process environment.
    assert_eq!(
        store_path_from(Some("/x/custom".into()), Some("/home/u".into())).unwrap(),
        PathBuf::from("/x/custom/store.sqlite")
    );
    assert_eq!(
        store_path_from(Some("".into()), Some("/home/u".into())).unwrap(),
        PathBuf::from("/home/u/.easy-personal/store.sqlite"),
        "an empty value falls through, like Python's `or`"
    );
    assert_eq!(
        store_path_from(None, Some("/home/u".into())).unwrap(),
        PathBuf::from("/home/u/.easy-personal/store.sqlite")
    );
    assert!(store_path_from(None, None).is_err());
}

#[test]
fn a_non_sqlite_failure_error_names_only_its_variant() {
    let dir = TempDir::new();
    let conn = Connection::open(dir.store()).unwrap();
    // Reading text as an integer is an `InvalidColumnType` error, whose Display names the column.
    let err = conn
        .query_row("SELECT 'secret-value' AS leaked_column", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap_err();
    let text = StoreError::from(err).to_string();
    assert!(text.contains("InvalidColumnType"), "{text}");
    assert!(
        !text.contains("secret-value") && !text.contains("leaked_column"),
        "{text}"
    );
}
