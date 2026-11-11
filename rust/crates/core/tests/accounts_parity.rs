//! T041: `accounts::grouped` and `sync::plan_json` against Python's answers.
//!
//! 1. The 8 goldens of the two default oracles: `api__accounts__{by_account,by_provider,bad_by}` and
//!    `api__sync_plan__all` of `rust/parity/golden` (a store with no account) and of `rust/parity/golden-rich`, read from
//!    the manifests and replayed at their `frozen_now`, in UTC. The rich store names `/work/rich/...` folders that the
//!    golden run had on disk: a temp copy points its two accounts at temp folders that exist, and `<WORK>` in the golden
//!    is that temp folder.
//! 2. `rust/parity/golden-accounts/` (made by `gen_accounts.py`): a hand-made store with a missing folder, an ignored
//!    account that keeps its picks and registrations, a Codex account, a detected one, a label with unicode, quotes and a
//!    newline, a limits file, an identity file beside the default folder, events in two accounts; asked in three zones.
//! 3. Nothing is written (a read-only connection; a folder that appears in the agent root adds no row), and the small
//!    file cases Python has: a file too big, a folder where a file should be, wrong types.
//!
//! Every store is a temp copy of a parity file opened read-only; the owner's real `~/.easy-personal` is never opened.

use easy_personal_core::accounts::grouped_in;
use easy_personal_core::metrics::MetricsError;
use easy_personal_core::store::open_read_only;
use easy_personal_core::sync::{SyncError, account_arg, plan_json};
use jiff::tz::TimeZone;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// 2026-01-20T00:00:00Z in microseconds: the `frozen_now` of every oracle.
const FROZEN_NOW: i64 = 1_768_867_200_000_000;

fn parity_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity")
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap_or_else(|e| panic!("{path:?}: {e}")))
        .unwrap()
}

fn temp_dir(tag: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "easy-personal-rs-t041-{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir_all(&dir).unwrap();
    fs::canonicalize(dir).unwrap()
}

/// `value` with every `from` in a string replaced by `to`.
fn replaced(value: &Value, from: &str, to: &str) -> Value {
    match value {
        Value::String(s) => Value::String(s.replace(from, to)),
        Value::Array(items) => Value::Array(items.iter().map(|v| replaced(v, from, to)).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), replaced(v, from, to)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// The route's answer as `(status, body)`: what `api.rs` maps the core's errors to.
fn ask(
    conn: &Connection,
    route: &str,
    params: &Value,
    agent_root: &Path,
    zone: &TimeZone,
) -> (u16, Value) {
    let param = |key: &str| params.get(key).and_then(Value::as_str);
    if route == "/api/accounts" {
        return match grouped_in(
            conn,
            param("by").unwrap_or("account"),
            FROZEN_NOW,
            agent_root,
            zone,
        ) {
            Ok(body) => (200, body),
            Err(MetricsError::Invalid(text)) => (400, json!({ "error": text })),
            Err(e) => panic!("{e}"),
        };
    }
    assert_eq!(route, "/api/sync");
    let answer = account_arg(param("account")).and_then(|account| plan_json(conn, account));
    match answer {
        Ok(body) => (200, body),
        Err(SyncError::Invalid(text)) => (400, json!({ "error": text })),
        Err(SyncError::NotFound(text)) => (404, json!({ "error": text })),
        Err(SyncError::Store(e)) => panic!("{e}"),
    }
}

/// Compares one oracle entry; the error names it.
fn check(
    file: &str,
    want_status: u64,
    want: &Value,
    got: (u16, Value),
    work: &str,
) -> Result<(), String> {
    let want = replaced(want, "<WORK>", work);
    if u64::from(got.0) != want_status || got.1 != want {
        return Err(format!(
            "{file}: status {} (want {want_status})\n  got  {}\n  want {want}",
            got.0, got.1
        ));
    }
    Ok(())
}

/// The accounts and sync-plan entries of a default oracle's manifest: `(file, route, params, status)`.
fn default_entries(dir: &str) -> Vec<(String, String, Value, u64)> {
    let manifest = read_json(&parity_dir().join(dir).join("manifest.json"));
    assert_eq!(manifest["frozen_now"], "2026-01-20T00:00:00+00:00");
    manifest["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| {
            let file = e["file"].as_str().unwrap();
            file.starts_with("api__accounts__") || file.starts_with("api__sync_plan__")
        })
        .map(|e| {
            (
                e["file"].as_str().unwrap().to_string(),
                e["route"].as_str().unwrap().to_string(),
                e["params"].clone(),
                e["status"].as_u64().unwrap(),
            )
        })
        .collect()
}

/// Rewrites the folder columns of the store copy `<work>/store.sqlite` (through a writable handle, before it is read).
fn rewrite_folders(work: &Path, from: &str, to: &str) {
    let writable = Connection::open(work.join("store.sqlite")).unwrap();
    writable
        .execute(
            "UPDATE agent_accounts SET folder = replace(folder, ?1, ?2), config_dir_env = replace(config_dir_env, ?1, ?2)",
            [from, to],
        )
        .unwrap();
}

#[test]
fn default_oracles_are_equal() {
    let (mut equal, mut total) = (0, 0);
    let mut failures = Vec::new();
    for (dir, store) in [
        ("golden", "store-v11.sqlite"),
        ("golden-rich", "store-rich/store-v11.sqlite"),
    ] {
        let work = temp_dir(dir);
        fs::copy(parity_dir().join(store), work.join("store.sqlite")).unwrap();
        // the rich store's accounts name `/work/rich/...`; the golden run had those folders on disk
        let root = work.join("agent-root");
        for folder in [".claude", ".claude-work"] {
            fs::create_dir_all(root.join(folder)).unwrap();
        }
        rewrite_folders(&work, "/work/rich", work.to_str().unwrap());
        let conn = open_read_only(&work.join("store.sqlite")).unwrap();
        for (file, route, params, status) in default_entries(dir) {
            let want = read_json(&parity_dir().join(dir).join(&file));
            total += 1;
            let got = ask(&conn, &route, &params, &root, &TimeZone::UTC);
            match check(&file, status, &want, got, work.to_str().unwrap()) {
                Ok(()) => equal += 1,
                Err(e) => failures.push(e),
            }
        }
    }
    eprintln!("default oracles: {equal} equal / {total} total");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(total, 8, "3 accounts + 1 sync entry in each of two oracles");
}

#[test]
fn hand_made_store_is_equal_in_three_zones() {
    let dir = parity_dir().join("golden-accounts");
    let manifest = read_json(&dir.join("manifest.json"));
    let (mut equal, mut total) = (0, 0);
    let mut failures = Vec::new();
    for (zone, info) in manifest["zones"].as_object().unwrap() {
        let work = temp_dir("hand");
        for d in manifest["tree"]["dirs"].as_array().unwrap() {
            fs::create_dir_all(work.join(d.as_str().unwrap())).unwrap();
        }
        for (rel, content) in manifest["tree"]["files"].as_object().unwrap() {
            let path = work.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content.as_str().unwrap()).unwrap();
        }
        fs::copy(dir.join("store.sqlite"), work.join("store.sqlite")).unwrap();
        rewrite_folders(&work, "@W@", work.to_str().unwrap());
        let conn = open_read_only(&work.join("store.sqlite")).unwrap();
        let zone_tz = TimeZone::get(zone).unwrap();
        for entry in info["entries"].as_array().unwrap() {
            let file = entry["file"].as_str().unwrap();
            let want = read_json(&dir.join(info["dir"].as_str().unwrap()).join(file));
            total += 1;
            let got = ask(
                &conn,
                entry["route"].as_str().unwrap(),
                &entry["params"],
                &work.join("agent-root"),
                &zone_tz,
            );
            match check(
                &format!("{zone}/{file}"),
                entry["status"].as_u64().unwrap(),
                &want,
                got,
                work.to_str().unwrap(),
            ) {
                Ok(()) => equal += 1,
                Err(e) => failures.push(e),
            }
        }
    }
    eprintln!("golden-accounts: {equal} equal / {total} total");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(total, 3 * 19);
}

/// The hand-made store in a temp work folder, for the cases below.
fn hand_made() -> (PathBuf, Connection) {
    let dir = parity_dir().join("golden-accounts");
    let work = temp_dir("edge");
    fs::create_dir_all(work.join("agent-root/.claude-work")).unwrap();
    fs::copy(dir.join("store.sqlite"), work.join("store.sqlite")).unwrap();
    rewrite_folders(&work, "@W@", work.to_str().unwrap());
    let conn = open_read_only(&work.join("store.sqlite")).unwrap();
    (work, conn)
}

fn work_status_line(work: &Path, conn: &Connection) -> Value {
    let body = grouped_in(
        conn,
        "account",
        FROZEN_NOW,
        &work.join("agent-root"),
        &TimeZone::UTC,
    )
    .unwrap();
    body["accounts"][1]["status_line"].clone()
}

#[test]
fn a_new_agent_folder_inserts_nothing() {
    let (work, conn) = hand_made();
    let count = |c: &Connection| -> i64 {
        c.query_row("SELECT COUNT(*) FROM agent_accounts", [], |r| r.get(0))
            .unwrap()
    };
    let before = count(&conn);
    fs::create_dir_all(work.join("agent-root/.claude-extra/projects")).unwrap();
    for by in ["account", "provider"] {
        let body = grouped_in(
            &conn,
            by,
            FROZEN_NOW,
            &work.join("agent-root"),
            &TimeZone::UTC,
        )
        .unwrap();
        assert!(
            !body.to_string().contains("claude-extra"),
            "a folder the store does not know is not listed"
        );
    }
    assert_eq!(count(&conn), before);
    assert!(
        !plan_json(&conn, None).unwrap()["steps"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn the_two_status_files_are_read_as_python_does() {
    let (work, conn) = hand_made();
    let folder = work.join("agent-root/.claude-work");
    // too big: neither file is read
    fs::write(folder.join(".claude.json"), vec![b' '; 33 * 1024 * 1024]).unwrap();
    fs::write(
        folder.join("easy-personal-limits.json"),
        vec![b' '; 65 * 1024],
    )
    .unwrap();
    let line = work_status_line(&work, &conn);
    assert_eq!(line["email"], Value::Null);
    assert_eq!(line["limits"], Value::Null);
    // a folder where a file should be, and invalid JSON: null fields, no error
    fs::remove_file(folder.join(".claude.json")).unwrap();
    fs::remove_file(folder.join("easy-personal-limits.json")).unwrap();
    fs::create_dir(folder.join(".claude.json")).unwrap();
    fs::create_dir(folder.join("easy-personal-limits.json")).unwrap();
    let line = work_status_line(&work, &conn);
    assert_eq!(
        (&line["email"], &line["limits"]),
        (&Value::Null, &Value::Null)
    );
    // wrong types: a bool is no number, a string is no epoch, a non-text email is no email
    fs::remove_dir(folder.join(".claude.json")).unwrap();
    fs::remove_dir(folder.join("easy-personal-limits.json")).unwrap();
    fs::write(
        folder.join(".claude.json"),
        r#"{"oauthAccount": {"emailAddress": 5, "billingType": null, "organizationName": "O"}}"#,
    )
    .unwrap();
    fs::write(
        folder.join("easy-personal-limits.json"),
        r#"{"five_hour": {"used_percentage": true}, "seven_day": {"used_percentage": 7, "resets_at": "soon"}, "written_at": "x"}"#,
    )
    .unwrap();
    let line = work_status_line(&work, &conn);
    assert_eq!(
        (&line["email"], &line["plan"], &line["organization"]),
        (&Value::Null, &Value::Null, &json!("O"))
    );
    assert_eq!(
        line["limits"],
        json!({"five_hour": null, "seven_day": {"used_percentage": 7, "resets_at": null, "stale": false},
               "written_at": null, "stale": false})
    );
}

#[test]
fn errors_name_none_of_the_input() {
    let (_work, conn) = hand_made();
    for raw in ["99", "00099", "99999999999999999999", "0"] {
        let error = plan_json(&conn, account_arg(Some(raw)).unwrap()).unwrap_err();
        assert!(
            matches!(&error, SyncError::NotFound(t) if t == "no such account"),
            "{raw}"
        );
    }
    for raw in [
        "abc",
        "-1",
        "1.5",
        "true",
        " 2",
        "\u{b2}",
        "\u{663}",
        "1;DROP TABLE agent_accounts",
    ] {
        let error = account_arg(Some(raw)).unwrap_err();
        assert!(
            matches!(&error, SyncError::Invalid(t) if t == "account must be an account id"),
            "{raw}"
        );
    }
    assert!(matches!(account_arg(Some("")), Ok(None)));
    let error =
        grouped_in(&conn, "x'; --", FROZEN_NOW, Path::new("/"), &TimeZone::UTC).unwrap_err();
    assert!(matches!(&error, MetricsError::Invalid(t) if t == "by must be account or provider"));
}
