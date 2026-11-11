//! T007: the Rust slices that exist (`usage_summary`, `usage_breakdown`, `rules::evaluate`) against the RICH Python
//! goldens in `rust/parity/golden-rich`, made by `gen_golden.py --rich` over the invented store
//! `rust/parity/store-rich/store-v11.sqlite` (18 days of history, 3 models, 2 accounts, every rule but one firing).
//! The calls to make come from the manifest (route or tool + params), so a new golden entry needs no change here.
//! Goldens are compared as parsed JSON (keys sorted in the files, list order kept; `1000` and `1000.0` differ).
//!
//! Entries with a `workspace` or `account` parameter are compared in `scope_parity.rs` (the 41 of `usage_summary`,
//! `usage_breakdown`, `warnings` and the plan); `scoped_entries_are_listed_not_compared` only counts them here. Nothing
//! compares the entries of a route the core does not serve yet (kit, board, accounts, workspaces, reports, memory graph,
//! sessions, outliers) or the scoped ones of those routes.
//!
//! The usage comparison runs in a child process with `TZ=UTC` (the `day` keys are local days and the goldens were made
//! under UTC; the zone is cached per process, as in `usage_tz_parity.rs`). The owner's real `~/.easy-personal` is never
//! opened: only the invented store and temp copies of it.

use easy_personal_core::metrics::{usage_breakdown, usage_summary};
use easy_personal_core::rules::{evaluate, shipped_rules};
use easy_personal_core::store::open_read_only;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

/// 2026-01-20T00:00:00Z in microseconds: `frozen_now` of the manifest (checked in `manifest_describes_the_rich_oracle`).
const FROZEN_NOW: i64 = 1_768_867_200_000_000;
/// A rule that no entry of the rich oracle can make fire; the manifest and the README say why (`cannot_fire`).
const NEVER_FIRES: &str = "context_index_stale";

fn parity_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity")
}

fn golden_dir() -> PathBuf {
    parity_dir().join("golden-rich")
}

fn store() -> PathBuf {
    parity_dir().join("store-rich").join("store-v11.sqlite")
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap_or_else(|e| panic!("{path:?}: {e}")))
        .unwrap()
}

fn manifest() -> Value {
    read_json(&golden_dir().join("manifest.json"))
}

fn golden(file: &str) -> Value {
    read_json(&golden_dir().join(file))
}

fn entries(manifest: &Value) -> Vec<Value> {
    manifest["entries"].as_array().unwrap().clone()
}

fn param(entry: &Value, key: &str) -> Option<String> {
    entry["params"]
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// The HTTP status of an entry; an MCP result has `"ok"`, which stands for 200.
fn status_of(entry: &Value) -> u64 {
    entry["status"].as_u64().unwrap_or_else(|| {
        if entry["status"] == json!("ok") {
            200
        } else {
            0
        }
    })
}

/// A guard entry (403 for a missing token, a wrong host or origin): the HTTP guard, not the core, answers it.
fn is_guard(entry: &Value) -> bool {
    entry["request_headers"]
        .as_array()
        .is_some_and(|h| !h.is_empty())
}

/// An entry whose parameters name a workspace or an account, or a report folder: not served by the Rust core yet.
fn is_scoped(entry: &Value) -> bool {
    ["workspace", "account"]
        .iter()
        .any(|k| entry["params"].get(k).is_some())
}

/// Which Rust slice answers an entry: `usage_summary`, `usage_breakdown`, `warnings`, or `None` (not ported).
fn slice(entry: &Value) -> Option<&'static str> {
    let name = entry["route"]
        .as_str()
        .map(|r| match r {
            "/api/usage/summary" => "usage_summary",
            "/api/usage/breakdown" => "usage_breakdown",
            "/api/warnings" => "warnings",
            _ => "",
        })
        .or_else(|| entry["tool"].as_str())?;
    ["usage_summary", "usage_breakdown", "warnings"]
        .into_iter()
        .find(|s| *s == name)
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t007-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A writable copy of the invented store (the only way a test changes a value).
fn copy_store(dir: &TempDir) -> PathBuf {
    let path = dir.0.join("store.sqlite");
    fs::copy(store(), &path).unwrap();
    path
}

// ---- the comparisons: each returns the entries that differ, so one run names every difference ----

/// `usage_summary` and `usage_breakdown` entries without a scope. Returns (compared, differing entry files).
fn compare_usage(store: &Path) -> (usize, Vec<String>) {
    let conn = open_read_only(store).unwrap();
    let (mut compared, mut wrong) = (0, Vec::new());
    for entry in entries(&manifest()) {
        let file = entry["file"].as_str().unwrap().to_string();
        let which = slice(&entry);
        if is_guard(&entry)
            || is_scoped(&entry)
            || !matches!(which, Some("usage_summary" | "usage_breakdown"))
        {
            continue;
        }
        let (since, until) = (param(&entry, "since"), param(&entry, "until"));
        let actual = if which == Some("usage_summary") {
            usage_summary(&conn, since.as_deref(), until.as_deref())
        } else {
            let by = param(&entry, "by").unwrap();
            usage_breakdown(&conn, &by, since.as_deref(), until.as_deref())
        };
        let want = golden(&file);
        compared += 1;
        match (status_of(&entry), actual) {
            (200, Ok(v)) if v == want => {}
            (400, Err(_)) if entry.get("compare").is_some() => {
                // the message is CPython-specific: only the status and the `error` key are compared
                let keys: Vec<&String> = want.as_object().unwrap().keys().collect();
                if keys != [&"error".to_string()] {
                    wrong.push(format!(
                        "{file}: the golden holds more than the `error` key"
                    ));
                }
            }
            (400, Err(e)) if json!(e.to_string()) == want["error"] => {}
            (status, other) => wrong.push(format!("{file}: status {status}, Rust gave {other:?}")),
        }
    }
    (compared, wrong)
}

/// `rules::evaluate` against the unscoped warnings entries (API and MCP).
fn compare_warnings(store: &Path) -> (usize, Vec<String>) {
    let conn = open_read_only(store).unwrap();
    let rules = shipped_rules().unwrap();
    let (mut compared, mut wrong) = (0, Vec::new());
    for entry in entries(&manifest()) {
        if is_guard(&entry) || is_scoped(&entry) || slice(&entry) != Some("warnings") {
            continue;
        }
        let file = entry["file"].as_str().unwrap();
        compared += 1;
        let actual = evaluate(&conn, FROZEN_NOW, &rules).unwrap();
        if actual != golden(file) {
            wrong.push(file.to_string());
        }
    }
    (compared, wrong)
}

// ---- 1. the oracle describes itself ----

#[test]
fn manifest_describes_the_rich_oracle() {
    let m = manifest();
    assert_eq!(m["store"]["user_version"], json!(11));
    assert_eq!(m["timezone"], json!("UTC"));
    let now: jiff::Timestamp = m["frozen_now"].as_str().unwrap().parse().unwrap();
    assert_eq!(now.as_microsecond(), FROZEN_NOW);
    assert!(m["fixture_sha256"].as_str().unwrap().len() == 64);
    assert!(
        store().is_file(),
        "the manifest names a store that is missing"
    );
    // key order: every HTTP and MCP entry but the CPython-specific one carries the raw body hash
    let with_hash = entries(&m)
        .iter()
        .filter(|e| e["raw_sha256"].as_str().is_some_and(|h| h.len() == 64))
        .count();
    let without: Vec<_> = entries(&m)
        .iter()
        .filter(|e| e["raw_sha256"].is_null())
        .map(|e| e["file"].as_str().unwrap().to_string())
        .collect();
    assert!(with_hash > 100, "{with_hash}");
    assert!(
        entries(&m)
            .iter()
            .filter(|e| e["raw_sha256"].is_null())
            .all(|e| e.get("compare").is_some()),
        "an entry without raw_sha256 must say how it is compared: {without:?}"
    );
    assert!(
        entries(&m)
            .iter()
            .filter(|e| e["file"].as_str().unwrap().ends_with("bad_since.json"))
            .all(|e| e["compare"]
                .as_str()
                .is_some_and(|c| c.contains("`error` key"))),
        "the bad_since entry must say it keeps only status and the error key"
    );
}

// ---- 2. usage (in a child process with TZ=UTC) ----

/// Runs in the child, started with `TZ=UTC`. `T007_STORE` names a changed copy of the store (the mutation test).
#[test]
#[ignore = "run by usage_entries_match_the_goldens, in a child process with TZ=UTC"]
fn usage_child() {
    let path = std::env::var_os("T007_STORE").map_or_else(store, PathBuf::from);
    let (compared, wrong) = compare_usage(&path);
    assert!(
        compared >= 20,
        "only {compared} usage entries were compared"
    );
    assert!(
        wrong.is_empty(),
        "{} usage entries differ: {wrong:#?}",
        wrong.len()
    );
    println!("usage entries compared: {compared}");
}

fn run_usage_child(store: Option<&Path>) -> (bool, String) {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args([
        "--exact",
        "usage_child",
        "--ignored",
        "--nocapture",
        "--test-threads=1",
    ])
    .env("TZ", "UTC");
    if let Some(path) = store {
        cmd.env("T007_STORE", path);
    }
    let out = cmd.output().unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success() && text.contains("1 passed"), text)
}

#[test]
fn usage_entries_match_the_goldens() {
    let (ok, text) = run_usage_child(None);
    assert!(ok, "{text}");
}

/// The `bad_since` golden holds a CPython message: only the status (400) and the `error` key are compared, and the
/// manifest says so; Rust must refuse the same value.
#[test]
fn bad_since_keeps_only_the_status_and_the_error_key() {
    let m = manifest();
    let found: Vec<Value> = entries(&m)
        .into_iter()
        .filter(|e| e["file"] == json!("api__usage_sessions__bad_since.json"))
        .collect();
    assert_eq!(found.len(), 1);
    assert_eq!(status_of(&found[0]), 400);
    assert!(found[0]["raw_sha256"].is_null());
    assert!(m["compare_notes"]["api__usage_sessions__bad_since.json"].is_string());
    let body = golden("api__usage_sessions__bad_since.json");
    assert_eq!(body.as_object().unwrap().len(), 1);
    assert!(body["error"].is_string());
    let conn = open_read_only(&store()).unwrap();
    assert!(usage_summary(&conn, Some("bad"), None).is_err());
}

// ---- 3. warnings ----

#[test]
fn warnings_entries_match_the_goldens() {
    let (compared, wrong) = compare_warnings(&store());
    assert!(
        compared >= 2,
        "only {compared} warnings entries were compared"
    );
    assert!(wrong.is_empty(), "warnings differ from: {wrong:#?}");
}

/// Python's own count, read from the goldens: each rule but `context_index_stale` has a hit in some warnings golden,
/// the unscoped one has several, and every rule is measured and clear in some entry (the manifest's `rule_coverage`
/// says the same; this test recounts it from the files).
#[test]
fn every_rule_but_one_fires_in_some_warnings_golden() {
    let m = manifest();
    let rules: Vec<String> = shipped_rules()
        .unwrap()
        .iter()
        .map(|r| r.id.clone())
        .collect();
    assert_eq!(rules.len(), 7);
    let unscoped = golden("api__warnings__all.json");
    for rule in &rules {
        let mut fired_in = Vec::new();
        for e in entries(&m)
            .iter()
            .filter(|e| e["route"] == json!("/api/warnings") && e["status"] == json!(200))
        {
            let body = golden(e["file"].as_str().unwrap());
            if body["hits"]
                .as_array()
                .unwrap()
                .iter()
                .any(|h| h["rule_id"] == json!(rule))
            {
                fired_in.push(e["file"].as_str().unwrap().to_string());
            }
        }
        if rule == NEVER_FIRES {
            assert!(fired_in.is_empty(), "{rule} fired in {fired_in:?}");
            assert!(
                m["cannot_fire"].get(rule).is_some(),
                "the manifest must say why {rule} cannot fire"
            );
        } else {
            assert!(!fired_in.is_empty(), "{rule} fires in no warnings golden");
            let in_unscoped = unscoped["hits"]
                .as_array()
                .unwrap()
                .iter()
                .any(|h| h["rule_id"] == json!(rule));
            assert!(
                in_unscoped,
                "{rule} does not fire in the unscoped golden, the one the Rust core is compared on"
            );
        }
        let cov = &m["rule_coverage"][rule];
        assert!(
            !cov["clear"].as_array().unwrap().is_empty(),
            "{rule} is never measured and clear"
        );
    }
}

// ---- 4. what is not compared yet ----

#[test]
fn scoped_entries_are_listed_not_compared() {
    let m = manifest();
    let scoped: Vec<String> = entries(&m)
        .iter()
        .filter(|e| is_scoped(e))
        .map(|e| e["file"].as_str().unwrap().to_string())
        .collect();
    let report_params: Vec<String> = entries(&m)
        .iter()
        .filter(|e| e["params"].get("folder").is_some() || e["params"].get("repo_no").is_some())
        .map(|e| e["file"].as_str().unwrap().to_string())
        .collect();
    assert!(scoped.len() >= 30, "{}", scoped.len());
    assert!(report_params.len() >= 5, "{}", report_params.len());
    println!(
        "listed, not compared here: {} workspace/account entries (the 41 of the four slices are compared in scope_parity.rs) and {} report-folder entries",
        scoped.len(),
        report_params.len()
    );
}

// ---- 5. the tests are not constant (mutation check) ----

#[test]
fn changing_one_fixture_value_breaks_the_comparisons() {
    // usage: one more output token on the first message changes the totals and the breakdowns
    let dir = TempDir::new();
    let path = copy_store(&dir);
    let n = Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE events SET output_tokens = output_tokens + 1 \
             WHERE id = (SELECT MIN(id) FROM events WHERE output_tokens IS NOT NULL)",
            [],
        )
        .unwrap();
    assert_eq!(n, 1);
    let (ok, text) = run_usage_child(Some(&path));
    assert!(!ok && text.contains("usage entries differ"), "{text}");

    // warnings: making the unreadable records readable moves `records_unreadable` and the causes
    let dir = TempDir::new();
    let path = copy_store(&dir);
    Connection::open(&path)
        .unwrap()
        .execute("UPDATE events SET abstained = 0 WHERE abstained = 1", [])
        .unwrap();
    let (compared, wrong) = compare_warnings(&path);
    assert!(
        compared >= 2 && wrong.len() == compared,
        "{compared} compared, {wrong:?} differ"
    );
}

#[test]
fn the_invented_store_is_unchanged_and_gets_no_sidecars() {
    let before = fs::read(store()).unwrap();
    compare_warnings(&store());
    assert_eq!(fs::read(store()).unwrap(), before);
    for suffix in ["-journal", "-wal", "-shm"] {
        let mut name = store().into_os_string();
        name.push(suffix);
        assert!(!Path::new(&name).exists(), "sidecar {suffix} appeared");
    }
}
