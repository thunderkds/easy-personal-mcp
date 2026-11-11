//! T027: `writer::open_for_write` — open rules, refusals (file untouched), created-store equality, `machine_id`.
//! Stores are fresh temp files or temp copies of `rust/parity/store-v11.sqlite`; `~/.easy-personal` is never opened.

use easy_personal_core::store::{KNOWN_MIGRATION, StoreError};
use easy_personal_core::writer::{SCHEMA_V11, WriteError, open_for_write};
use rusqlite::Connection;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// A named setup step for a table case.
type Case<F> = (&'static str, F);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t027-open-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn with_fixture() -> Self {
        let dir = TempDir::new();
        fs::copy(parity().join("store-v11.sqlite"), dir.store()).unwrap();
        dir
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

fn parity() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity")
}

fn listing(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn snapshot_json() -> Value {
    serde_json::from_str(
        &fs::read_to_string(parity().join("ingest/golden/schema-v11.json")).unwrap(),
    )
    .unwrap()
}

/// `sqlite_master` as `[type, name, tbl_name, sql]` sorted by name, like `schema-v11.json`.
fn master(conn: &Connection) -> Vec<Value> {
    conn.prepare("SELECT type, name, tbl_name, sql FROM sqlite_master ORDER BY name")
        .unwrap()
        .query_map([], |r| {
            Ok(serde_json::json!([
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?
            ]))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn user_version(path: &Path) -> i64 {
    Connection::open(path)
        .unwrap()
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap()
}

fn is_uuid4(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.iter().map(|p| p.len()).collect::<Vec<_>>() == [8, 4, 4, 4, 12]
        && s.chars()
            .all(|c| c == '-' || c.is_ascii_digit() || ('a'..='f').contains(&c))
        && parts[2].starts_with('4')
        && matches!(parts[3].as_bytes()[0], b'8' | b'9' | b'a' | b'b')
}

/// The text of every error a refused open printed must not name the folder, SQL or a value.
fn assert_clean(err: &WriteError, dir: &TempDir) {
    let text = err.to_string();
    let folder = dir.0.to_string_lossy();
    assert!(!text.contains(folder.as_ref()), "path in {text:?}");
    assert!(!text.contains("store.sqlite"), "file name in {text:?}");
    for word in ["SELECT", "INSERT", "PRAGMA", "CREATE"] {
        assert!(!text.contains(word), "SQL in {text:?}");
    }
}

#[test]
fn embedded_schema_is_byte_equal_to_the_t026_golden() {
    let golden = fs::read(parity().join("ingest/golden/schema-v11.sql")).unwrap();
    assert_eq!(SCHEMA_V11.as_bytes(), golden.as_slice());
}

/// Criterion 2 + 5: no file, an empty file, a SQLite file with no table -> a new v11 store equal to the snapshot.
#[test]
fn a_new_store_equals_the_python_snapshot() {
    let snap = snapshot_json();
    let make: [Case<fn(&Path)>; 3] = [
        ("missing", |_| {}),
        ("empty file", |p| fs::write(p, b"").unwrap()),
        ("sqlite file without tables", |p| {
            let c = Connection::open(p).unwrap();
            c.execute_batch("CREATE TABLE gone (x); DROP TABLE gone;")
                .unwrap();
        }),
    ];
    for (case, setup) in make {
        let dir = TempDir::new();
        setup(&dir.store());
        let store = open_for_write(&dir.store()).unwrap_or_else(|e| panic!("{case}: {e}"));
        assert!(
            is_uuid4(store.machine_id()),
            "{case}: {}",
            store.machine_id()
        );
        let id = store.machine_id().to_owned();
        drop(store);

        let conn = Connection::open(dir.store()).unwrap();
        assert_eq!(
            Value::Array(master(&conn)),
            snap["sqlite_master"],
            "{case}: sqlite_master"
        );
        let seed: Vec<Value> = conn
            .prepare("SELECT id, name, color, icon, description, email, is_orphan FROM workspaces")
            .unwrap()
            .query_map([], |r| {
                Ok(serde_json::json!([
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, i64>(6)?
                ]))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            Value::Array(seed),
            snap["seed_rows"]["workspaces"]["rows"],
            "{case}"
        );
        let meta: Vec<(String, String)> = conn
            .prepare("SELECT key, value FROM meta")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(meta, vec![("machine_id".to_owned(), id)], "{case}");
        let integrity: String = conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .unwrap();
        assert_eq!(integrity, "ok", "{case}");
        let journal: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(journal, snap["pragmas"]["journal_mode"], "{case}");
        assert_eq!(user_version(&dir.store()), 11, "{case}");
        drop(conn);
        // A second open keeps the store as it is.
        let again = open_for_write(&dir.store()).unwrap();
        drop(again);
        let conn = Connection::open(dir.store()).unwrap();
        assert_eq!(Value::Array(master(&conn)), snap["sqlite_master"], "{case}");
    }
}

/// Criterion 2: an up-to-date store opens; nothing in its schema or rows changes.
#[test]
fn a_v11_store_opens_without_a_schema_change() {
    let dir = TempDir::with_fixture();
    let before = {
        let c = Connection::open(dir.store()).unwrap();
        let rows: i64 = c
            .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
            .unwrap();
        (master(&c), rows)
    };
    let store = open_for_write(&dir.store()).unwrap();
    drop(store);
    let c = Connection::open(dir.store()).unwrap();
    let rows: i64 = c
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap();
    assert_eq!((master(&c), rows), before);
    assert_eq!(user_version(&dir.store()), KNOWN_MIGRATION);
}

/// Criterion 2: newer and older stores are refused before any write: same bytes, no `-wal`/`-shm` left.
#[test]
fn other_versions_are_refused_and_the_file_is_untouched() {
    let reader_newer = StoreError::Newer {
        found: 12,
        known: 11,
    }
    .to_string();
    let cases: [(&str, i64, bool, Option<&str>); 7] = [
        ("v12", 12, false, Some(reader_newer.as_str())),
        ("v12 in WAL mode", 12, true, Some(reader_newer.as_str())),
        ("v99", 99, false, None),
        (
            "v10",
            10,
            false,
            Some("store is schema v10: run the Python app once to migrate it"),
        ),
        (
            "v8 in WAL mode",
            8,
            true,
            Some("store is schema v8: run the Python app once to migrate it"),
        ),
        (
            "v1",
            1,
            false,
            Some("store is schema v1: run the Python app once to migrate it"),
        ),
        (
            "v0 with tables",
            0,
            false,
            Some("store is schema v0: run the Python app once to migrate it"),
        ),
    ];
    for (case, version, wal, text) in cases {
        let dir = TempDir::with_fixture();
        {
            let c = Connection::open(dir.store()).unwrap();
            if wal {
                c.query_row("PRAGMA journal_mode=WAL", [], |_| Ok(()))
                    .unwrap();
            }
            c.execute_batch(&format!("PRAGMA user_version = {version}"))
                .unwrap();
        }
        let files = listing(&dir.0);
        assert_eq!(files, ["store.sqlite"], "{case}: setup left side files");
        let bytes = fs::read(dir.store()).unwrap();

        let err = open_for_write(&dir.store()).unwrap_err();
        assert_eq!(err.exit_code(), 2, "{case}");
        match (&err, version > 11) {
            (WriteError::Newer { found, known }, true) => {
                assert_eq!((*found, *known), (version, 11), "{case}")
            }
            (WriteError::Older { found }, false) => assert_eq!(*found, version, "{case}"),
            other => panic!("{case}: {other:?}"),
        }
        if let Some(text) = text {
            assert_eq!(err.to_string(), text, "{case}");
        }
        assert_clean(&err, &dir);
        assert_eq!(
            fs::read(dir.store()).unwrap(),
            bytes,
            "{case}: bytes changed"
        );
        assert_eq!(listing(&dir.0), files, "{case}: side files left");
    }
}

/// Criterion 2: a directory, a symlink loop or a file that is not SQLite -> one fixed text, no path, never a panic.
#[test]
fn a_path_that_is_not_a_store_gets_one_fixed_error() {
    let cases: [Case<fn(&Path)>; 3] = [
        ("directory", |p| fs::create_dir(p).unwrap()),
        ("symlink loop", |p| {
            let other = p.with_file_name("loop-b");
            std::os::unix::fs::symlink(&other, p).unwrap();
            std::os::unix::fs::symlink(p, &other).unwrap();
        }),
        ("not SQLite", |p| {
            fs::write(
                p,
                b"this is not a SQLite file at all, just invented text\n".repeat(20),
            )
            .unwrap()
        }),
    ];
    let mut texts = Vec::new();
    for (case, setup) in cases {
        let dir = TempDir::new();
        setup(&dir.store());
        let before = fs::read(dir.store()).ok();
        let files = listing(&dir.0);
        let err = open_for_write(&dir.store()).unwrap_err();
        assert!(matches!(err, WriteError::NotAStore), "{case}: {err:?}");
        assert_eq!(err.exit_code(), 1, "{case}");
        assert_clean(&err, &dir);
        assert_eq!(fs::read(dir.store()).ok(), before, "{case}: changed");
        assert_eq!(listing(&dir.0), files, "{case}: side files");
        texts.push(err.to_string());
    }
    texts.dedup();
    assert_eq!(texts, ["the store path is not a SQLite store file"]);
}

/// Criterion 4: an existing `machine_id` is read, never replaced or doubled.
#[test]
fn an_existing_machine_id_is_kept() {
    let dir = TempDir::with_fixture();
    let before: String = Connection::open(dir.store())
        .unwrap()
        .query_row("SELECT value FROM meta WHERE key = 'machine_id'", [], |r| {
            r.get(0)
        })
        .unwrap();
    for _ in 0..2 {
        let store = open_for_write(&dir.store()).unwrap();
        assert_eq!(store.machine_id(), before);
    }
    let rows: i64 = Connection::open(dir.store())
        .unwrap()
        .query_row("SELECT COUNT(*) FROM meta", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 1);
}

/// Criterion 4: opening an up-to-date store takes no write lock — it opens at once while another connection holds a
/// read transaction, and also while another holds the write lock (a writer open would wait 5 s, then fail busy).
#[test]
fn opening_an_up_to_date_store_takes_no_write_lock() {
    let dir = TempDir::with_fixture();
    drop(open_for_write(&dir.store()).unwrap()); // the fixture is in rollback-journal mode: the first open sets WAL

    let other = Connection::open(dir.store()).unwrap();
    other
        .execute_batch("BEGIN; SELECT COUNT(*) FROM events;")
        .unwrap();
    let started = Instant::now();
    drop(open_for_write(&dir.store()).unwrap());
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "read txn: {:?}",
        started.elapsed()
    );
    other.execute_batch("COMMIT").unwrap();

    other.execute_batch("BEGIN IMMEDIATE").unwrap();
    let started = Instant::now();
    let store = open_for_write(&dir.store());
    let took = started.elapsed();
    other.execute_batch("ROLLBACK").unwrap();
    store.unwrap();
    assert!(took < Duration::from_secs(1), "write lock held: {took:?}");
}
