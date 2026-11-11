//! T047: `reports::save` and the secret check against Python's answers in `rust/parity/golden-save-report/` (made once by
//! `rust/parity/gen_save_report.py`: Python's `reports.save` over temp copies of `store-rich`, and `SECRET.search` over a
//! corpus), plus what only the Rust side has: the write discipline (a refused call leaves the file's bytes untouched, a
//! store that is busy, read-only or not v11 gets a fixed text) and that no error carries the caller's text.
//!
//! Copy-store-then-compare: each case runs on its own temp copy of the rich store at the manifest's frozen clock; the new row
//! is compared column by column (`server_version` is the one normalised value: Python's package version against this
//! build's) with its `fixes_json` and `snapshot_json` **byte for byte**, and `get_report` over the copy must equal the
//! answer Python gave over its copy. Every store is a temp copy; the owner's `~/.easy-personal` is never opened.

use easy_personal_core::reports::{SaveError, get_report, save, validate};
use easy_personal_core::secret::looks_secret;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// `frozen_now` of the oracle (`2026-01-20T00:00:00+00:00`), in microseconds since the epoch.
const FROZEN_NOW: i64 = 1_768_867_200_000_000;

fn parity_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity")
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap_or_else(|e| panic!("{path:?}: {e}")))
        .unwrap()
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t047-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    /// A copy of the rich store with its account rows pointed at temp folders (as the generator did).
    fn store(&self) -> PathBuf {
        let path = self.0.join("store.sqlite");
        fs::copy(parity_dir().join("store-rich/store-v11.sqlite"), &path).unwrap();
        let conn = Connection::open(&path).unwrap();
        let ids: Vec<i64> = conn
            .prepare("SELECT id FROM agent_accounts")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        for id in ids {
            let folder = self.0.join(format!("account-{id}"));
            fs::create_dir_all(&folder).unwrap();
            conn.execute(
                "UPDATE agent_accounts SET folder = ?1 WHERE id = ?2",
                rusqlite::params![folder.to_str().unwrap(), id],
            )
            .unwrap();
        }
        // a checkpointed, single-file store: the byte comparisons below read the file itself
        conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
            .unwrap();
        path
    }
}

/// A store in WAL mode, as the Python app leaves every store it has opened (the fixture is in `delete` mode).
fn wal(path: &Path) {
    let conn = Connection::open(path).unwrap();
    conn.query_row("PRAGMA journal_mode=WAL", [], |_| Ok(()))
        .unwrap();
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn row_count(path: &Path) -> i64 {
    Connection::open(path)
        .unwrap()
        .query_row("SELECT count(*) FROM investigate_reports", [], |r| r.get(0))
        .unwrap()
}

/// Every column of the row `id`, by name.
fn row(path: &Path, id: i64) -> Value {
    let conn = Connection::open(path).unwrap();
    conn.query_row(
        "SELECT id, saved_utc, workspace, account, summary, fixes_json, snapshot_json FROM investigate_reports WHERE id = ?",
        [id],
        |r| {
            Ok(json!({
                "id": r.get::<_, i64>(0)?,
                "saved_utc": r.get::<_, String>(1)?,
                "workspace": r.get::<_, Option<String>>(2)?,
                "account": r.get::<_, Option<String>>(3)?,
                "summary": r.get::<_, String>(4)?,
                "fixes_json": r.get::<_, String>(5)?,
                "snapshot_json": r.get::<_, String>(6)?,
            }))
        },
    )
    .unwrap()
}

fn text<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

/// The oracle's cases: each one saved or refused as Python did, on a copy of the store.
#[test]
fn every_case_stores_or_refuses_what_python_did() {
    let golden = read_json(&parity_dir().join("golden-save-report/cases.json"));
    assert_eq!(golden["frozen_now"], "2026-01-20T00:00:00+00:00");
    let cases = golden["cases"].as_array().unwrap();
    assert!(cases.len() >= 40, "the oracle has {} cases", cases.len());
    let (mut equal, mut saved, mut refused) = (0, 0, 0);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let (dir, args, python) = (TempDir::new(), &case["args"], &case["python"]);
        let path = dir.store();
        let before = fs::read(&path).unwrap();
        assert_eq!(
            row_count(&path),
            python["rows_before"].as_i64().unwrap(),
            "{name}"
        );
        let got = save(
            &path,
            &args["summary"],
            &args["fixes"],
            text(args, "workspace"),
            text(args, "account"),
            FROZEN_NOW,
        );
        match (got, python.get("error")) {
            (Err(SaveError::Refused(got)), Some(want)) => {
                assert_eq!(&Value::String(got), want, "{name}");
                assert_eq!(
                    fs::read(&path).unwrap(),
                    before,
                    "{name}: a refusal touched the file"
                );
                refused += 1;
            }
            (Ok(done), None) => {
                let want_row = &python["row"];
                assert_eq!(done.id, want_row["id"].as_i64().unwrap(), "{name}");
                assert_eq!(json!(done.saved_utc), python["ok"]["saved_utc"], "{name}");
                let mut want_row = want_row.clone();
                want_row.as_object_mut().unwrap().remove("server_version");
                assert_eq!(row(&path, done.id), want_row, "{name}: the stored row");
                let conn = Connection::open(&path).unwrap();
                let version: String = conn
                    .query_row(
                        "SELECT server_version FROM investigate_reports WHERE id = ?",
                        [done.id],
                        |r| r.get(0),
                    )
                    .unwrap();
                assert_eq!(version, env!("CARGO_PKG_VERSION"), "{name}");
                let mut report = get_report(&conn, done.id, FROZEN_NOW).unwrap().unwrap();
                let mut want = python["report"].clone();
                for body in [&mut report, &mut want] {
                    body["server_version"] = Value::Null; // the one normalised field
                }
                assert_eq!(report, want, "{name}: GET /api/reports/<id>");
                saved += 1;
            }
            (got, want) => panic!("{name}: Rust {got:?}, Python {want:?}"),
        }
        assert_eq!(
            row_count(&path),
            python["rows_after"].as_i64().unwrap(),
            "{name}"
        );
        equal += 1;
    }
    println!(
        "save_report oracle: {equal}/{} cases equal ({saved} saved, {refused} refused)",
        cases.len()
    );
    assert_eq!(equal, cases.len());
}

#[test]
fn the_secret_check_matches_pythons_on_the_corpus() {
    let corpus = read_json(&parity_dir().join("golden-save-report/secret_corpus.json"));
    let corpus = corpus.as_array().unwrap();
    assert!(corpus.len() >= 40);
    let mut equal = 0;
    for entry in corpus {
        let text = entry["text"].as_str().unwrap();
        assert_eq!(
            looks_secret(text),
            entry["secret"].as_bool().unwrap(),
            "{text:?}"
        );
        equal += 1;
    }
    println!("secret corpus: {equal}/{} equal", corpus.len());
}

/// A wrong type reaches `validate` only from a direct call (the MCP layer checks types first), but the texts are Python's.
#[test]
fn validate_names_the_field_and_never_the_value() {
    let canary = "CANARY-text-with-spaces";
    let refusals = [
        validate(&json!(canary), &json!("nope")).unwrap_err(),
        validate(&json!(canary), &json!([{"text": canary, "extra": 1}])).unwrap_err(),
        validate(&json!(canary), &json!([{"text": canary, "number": true}])).unwrap_err(),
        validate(
            &json!(canary),
            &json!([{"text": canary, "number": [canary]}]),
        )
        .unwrap_err(),
        validate(&json!(canary), &json!([{"text": canary, "tool": 5}])).unwrap_err(),
        validate(&json!(canary), &json!([{"text": 5}])).unwrap_err(),
        validate(&json!(5), &json!([])).unwrap_err(),
        validate(&json!("sk-abc CANARY"), &json!([])).unwrap_err(),
    ];
    // the first one is fine for the summary: it fails on `fixes`
    for text in refusals {
        assert!(!text.contains("CANARY"), "{text}");
        assert!(text.ends_with("nothing was saved"), "{text}");
    }
    assert!(validate(&json!(canary), &json!([])).is_ok());
}

fn plain_save(path: &Path) -> Result<easy_personal_core::reports::SavedReport, SaveError> {
    save(
        path,
        &json!("a summary"),
        &json!([]),
        None,
        None,
        FROZEN_NOW,
    )
}

/// Two saves at once: two rows, two ids, both answers whole.
#[test]
fn two_saves_at_once_make_two_rows() {
    let dir = TempDir::new();
    let path = dir.store();
    wal(&path);
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let path = path.clone();
            std::thread::spawn(move || plain_save(&path))
        })
        .collect();
    let mut ids: Vec<i64> = handles
        .into_iter()
        .map(|h| h.join().unwrap().unwrap().id)
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, vec![3, 4]);
    assert_eq!(row_count(&path), 4);
}

/// A writer holding the store for under 5 s only delays a save; one holding it longer gets a fixed text without SQL.
#[test]
fn a_busy_store_waits_five_seconds_then_says_so_in_fixed_words() {
    let dir = TempDir::new();
    let path = dir.store();
    wal(&path);
    let hold = |seconds: u64| {
        let path = path.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch("BEGIN IMMEDIATE").unwrap();
            tx.send(()).unwrap();
            std::thread::sleep(Duration::from_secs(seconds));
            conn.execute_batch("COMMIT").unwrap();
        });
        rx.recv().unwrap();
        handle
    };
    let short = hold(1);
    let started = Instant::now();
    let saved = plain_save(&path).expect("a 1 s hold only delays the save");
    assert!(started.elapsed() >= Duration::from_millis(500));
    short.join().unwrap();
    assert_eq!(saved.id, 3);
    let long = hold(7);
    let started = Instant::now();
    let refused = plain_save(&path).unwrap_err();
    let waited = started.elapsed();
    long.join().unwrap();
    assert!(
        (Duration::from_secs(4)..Duration::from_secs(7)).contains(&waited),
        "{waited:?}"
    );
    assert_eq!(
        refused,
        SaveError::Failed(
            "the store is busy (another process is writing); try again in a moment".into()
        )
    );
    assert_eq!(row_count(&path), 3, "the refused call stored nothing");
}

/// A store that is read-only on disk: a fixed text, nothing stored, no path.
#[test]
fn a_read_only_store_gets_a_fixed_text() {
    let dir = TempDir::new();
    let path = dir.store();
    wal(&path);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
    if fs::OpenOptions::new().append(true).open(&path).is_ok() {
        return; // running as a user the mode does not bind (root): nothing to show
    }
    let refused = plain_save(&path).unwrap_err();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let SaveError::Failed(text) = refused else {
        panic!("{refused:?}")
    };
    assert_eq!(text, "the store is read-only; nothing was saved");
    assert!(!text.contains(dir.0.to_str().unwrap()));
    assert_eq!(row_count(&path), 2);
}

/// A store that is missing, not v11 or newer is refused before any write, with the texts the other surfaces use.
#[test]
fn a_store_that_is_not_v11_is_refused_untouched() {
    let dir = TempDir::new();
    let path = dir.store();
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("PRAGMA user_version = 10").unwrap();
    drop(conn);
    let bytes = fs::read(&path).unwrap();
    assert_eq!(
        plain_save(&path).unwrap_err(),
        SaveError::Failed("store is schema v10: run the Python app once to migrate it".into())
    );
    Connection::open(&path)
        .unwrap()
        .execute_batch("PRAGMA user_version = 12")
        .unwrap();
    let bytes12 = fs::read(&path).unwrap();
    let SaveError::Failed(newer) = plain_save(&path).unwrap_err() else {
        panic!()
    };
    assert!(
        newer.contains("newer than the 11 this build knows"),
        "{newer}"
    );
    assert_eq!(fs::read(&path).unwrap(), bytes12);
    assert_ne!(bytes, bytes12);
    let missing = dir.0.join("nothing.sqlite");
    let SaveError::Failed(text) = plain_save(&missing).unwrap_err() else {
        panic!()
    };
    assert!(text.starts_with("no store yet"), "{text}");
    assert!(!missing.exists(), "a missing store is never created");
}
