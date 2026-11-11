//! T018: `session_outliers` and `session_timeline` in the core (with the `workspace` / `account` scope of T017 and
//! `detail_sessions`) against the Python goldens of both oracles: `rust/parity/golden` over `store-v11.sqlite`
//! (5 entries) and `rust/parity/golden-rich` over `store-rich/store-v11.sqlite` (18 entries, the scoped ones among them).
//!
//! The entries come from the manifests: every `/api/usage/outliers` and `/api/usage/sessions` route and every MCP
//! `session_outliers` call, the `bad_since` errors included. Compared as parsed JSON (keys sorted in the files, list order
//! kept; `30` and `30.0` differ). The HTTP route asks for `detail_sessions=50` (`TIMELINE_DETAIL_SESSIONS` in `api.py`).
//!
//! The comparison runs in a child process with `TZ=UTC` like the other goldens; the zone is cached per process, and the
//! non-UTC cases (section 3) each run in a child with their own `TZ`. The owner's real `~/.easy-personal` is never
//! opened: only the invented stores and temp copies of them.

use easy_personal_core::metrics::{
    session_outliers, session_outliers_scoped, session_timeline, session_timeline_scoped,
};
use easy_personal_core::scope::Scope;
use easy_personal_core::store::open_read_only;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

/// `TIMELINE_DETAIL_SESSIONS` of `surfaces/api.py`: what the dashboard route passes as `detail_sessions`.
const DETAIL: usize = 50;

fn parity_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity")
}

#[derive(Clone, Copy)]
struct Oracle {
    name: &'static str,
    golden: &'static str,
    store: &'static str,
}

const DEFAULT: Oracle = Oracle {
    name: "default",
    golden: "golden",
    store: "store-v11.sqlite",
};
const RICH: Oracle = Oracle {
    name: "rich",
    golden: "golden-rich",
    store: "store-rich/store-v11.sqlite",
};

impl Oracle {
    fn store(&self) -> PathBuf {
        parity_dir().join(self.store)
    }

    fn read(&self, file: &str) -> Value {
        let path = parity_dir().join(self.golden).join(file);
        serde_json::from_str(&fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}")))
            .unwrap()
    }

    fn entries(&self) -> Vec<(Kind, Value)> {
        self.read("manifest.json")["entries"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|e| kind(e).map(|k| (k, e.clone())))
            .collect()
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Outliers,
    Timeline,
}

/// The function that answers an entry, by route (HTTP) or tool name (MCP).
fn kind(entry: &Value) -> Option<Kind> {
    match (entry["route"].as_str(), entry["tool"].as_str()) {
        (Some("/api/usage/outliers"), _) | (_, Some("session_outliers")) => Some(Kind::Outliers),
        (Some("/api/usage/sessions"), _) => Some(Kind::Timeline),
        _ => None,
    }
}

fn param(entry: &Value, key: &str) -> Option<String> {
    entry["params"]
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn status_of(entry: &Value) -> u64 {
    entry["status"].as_u64().unwrap_or_else(|| {
        if entry["status"] == json!("ok") {
            200
        } else {
            0
        }
    })
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t018-{}-{}",
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

/// A writable copy of an invented store with one SQL batch applied (the only way a test changes a value).
fn changed_store(dir: &TempDir, from: &Path, sql: &str) -> PathBuf {
    let path = dir.0.join(format!(
        "store-{}.sqlite",
        fs::read_dir(&dir.0).unwrap().count()
    ));
    fs::copy(from, &path).unwrap();
    Connection::open(&path).unwrap().execute_batch(sql).unwrap();
    path
}

/// One entry answered by the Rust core: `Ok(body)` or `Err(error text)`.
fn answer(conn: &Connection, kind: Kind, entry: &Value) -> Result<Value, String> {
    let (workspace, account) = (param(entry, "workspace"), param(entry, "account"));
    let scope = Scope {
        workspace: workspace.as_deref(),
        account: account.as_deref(),
    };
    let (since, until) = (param(entry, "since"), param(entry, "until"));
    match kind {
        Kind::Outliers => session_outliers_scoped(conn, since.as_deref(), until.as_deref(), scope),
        Kind::Timeline => session_timeline_scoped(
            conn,
            since.as_deref(),
            until.as_deref(),
            Some(DETAIL),
            scope,
        ),
    }
    .map_err(|e| e.to_string())
}

/// Compares every session entry of an oracle against `store`; returns (compared, the entries that differ).
fn compare(oracle: Oracle, store: &Path) -> (usize, Vec<String>) {
    let conn = open_read_only(store).unwrap();
    let (mut compared, mut wrong) = (0, Vec::new());
    for (kind, entry) in oracle.entries() {
        let file = entry["file"].as_str().unwrap().to_string();
        let want = oracle.read(&file);
        compared += 1;
        match (status_of(&entry), answer(&conn, kind, &entry)) {
            (200, Ok(got)) if got == want => {}
            // the manifest says: compare status and the `error` key only (the text is CPython's)
            (400, Err(_)) if entry.get("compare").is_some() && want.get("error").is_some() => {}
            (400, Err(text)) if json!(text) == want["error"] => {}
            (status, other) => wrong.push(format!(
                "{}/{file}: status {status}, Rust gave {other:?}",
                oracle.name
            )),
        }
    }
    (compared, wrong)
}

const DEFAULT_FILES: [&str; 5] = [
    "api__usage_outliers__all.json",
    "api__usage_sessions__all.json",
    "api__usage_sessions__bad_since.json",
    "api__usage_sessions__window.json",
    "mcp__session_outliers__all.json",
];

const RICH_FILES: [&str; 18] = [
    "api__usage_outliers__account_default.json",
    "api__usage_outliers__account_work.json",
    "api__usage_outliers__all.json",
    "api__usage_outliers__window.json",
    "api__usage_outliers__workspace_acme.json",
    "api__usage_outliers__workspace_acme_account_work.json",
    "api__usage_outliers__workspace_personal.json",
    "api__usage_sessions__account_default.json",
    "api__usage_sessions__account_work.json",
    "api__usage_sessions__all.json",
    "api__usage_sessions__bad_since.json",
    "api__usage_sessions__window.json",
    "api__usage_sessions__window_mid.json",
    "api__usage_sessions__workspace_acme.json",
    "api__usage_sessions__workspace_acme_account_work.json",
    "api__usage_sessions__workspace_personal.json",
    "mcp__session_outliers__all.json",
    "mcp__session_outliers__window.json",
];

// ---- 1. the goldens ----

/// Runs in the child, started with `TZ=UTC`. `T018_DEFAULT_STORE` / `T018_RICH_STORE` name a changed copy of a store
/// (the mutation tests).
#[test]
#[ignore = "run by session_entries_match_the_goldens, in a child process with TZ=UTC"]
fn sessions_child() {
    let mut wrong = Vec::new();
    let mut counts = Vec::new();
    for (oracle, var) in [(DEFAULT, "T018_DEFAULT_STORE"), (RICH, "T018_RICH_STORE")] {
        let store = std::env::var_os(var).map_or_else(|| oracle.store(), PathBuf::from);
        let (compared, bad) = compare(oracle, &store);
        counts.push(format!("{}={compared}", oracle.name));
        wrong.extend(bad);
    }
    println!("session entries compared: {}", counts.join(" "));
    assert!(
        wrong.is_empty(),
        "{} session entries differ: {wrong:#?}",
        wrong.len()
    );
}

fn run_child(test: &str, envs: &[(&str, &Path)], tz: &str) -> (bool, String) {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args([
        "--exact",
        test,
        "--ignored",
        "--nocapture",
        "--test-threads=1",
    ])
    .env("TZ", tz);
    for (key, value) in envs {
        cmd.env(key, value);
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
fn session_entries_match_the_goldens() {
    let (ok, text) = run_child("sessions_child", &[], "UTC");
    assert!(ok, "{text}");
    // the counts come from the manifests, not from a constant: a missing entry fails here
    let want = format!(
        "session entries compared: default={} rich={}",
        DEFAULT.entries().len(),
        RICH.entries().len()
    );
    assert!(text.contains(&want), "{want} not in {text}");
    assert!(text.contains("default=5 rich=18"), "{text}");
}

/// The 23 entries are the ones the task names, by file, kind and surface.
#[test]
fn the_manifests_hold_the_23_session_entries() {
    for (oracle, files) in [(DEFAULT, &DEFAULT_FILES[..]), (RICH, &RICH_FILES[..])] {
        let entries = oracle.entries();
        let found: BTreeSet<&str> = entries
            .iter()
            .map(|(_, e)| e["file"].as_str().unwrap())
            .collect();
        assert_eq!(found, files.iter().copied().collect(), "{}", oracle.name);
        assert_eq!(
            entries.len(),
            files.len(),
            "{}: duplicate entries",
            oracle.name
        );
    }
    let count = |o: Oracle, k: Kind, surface: &str| {
        o.entries()
            .iter()
            .filter(|(kind, e)| *kind == k && e["surface"] == json!(surface))
            .count()
    };
    assert_eq!(count(DEFAULT, Kind::Outliers, "http"), 1);
    assert_eq!(count(DEFAULT, Kind::Outliers, "mcp"), 1);
    assert_eq!(count(DEFAULT, Kind::Timeline, "http"), 3);
    assert_eq!(count(RICH, Kind::Outliers, "http"), 7);
    assert_eq!(count(RICH, Kind::Outliers, "mcp"), 2);
    assert_eq!(count(RICH, Kind::Timeline, "http"), 9);
    // the error entries: `since=bad`, one per oracle; the rich one is compared on status and the `error` key only
    for oracle in [DEFAULT, RICH] {
        let errors: Vec<Value> = oracle
            .entries()
            .into_iter()
            .map(|(_, e)| e)
            .filter(|e| status_of(e) == 400)
            .collect();
        assert_eq!(errors.len(), 1, "{}", oracle.name);
        assert_eq!(errors[0]["params"], json!({"since": "bad"}));
    }
    // ten of the rich entries are scoped: five outliers and five timelines, by workspace, account or both
    let scoped = RICH
        .entries()
        .iter()
        .filter(|(_, e)| {
            ["workspace", "account"]
                .iter()
                .any(|k| e["params"].get(k).is_some())
        })
        .count();
    assert_eq!(scoped, 10);
}

// ---- 2. errors, scope and the unscoped functions ----

fn conn_of(oracle: Oracle) -> Connection {
    open_read_only(&oracle.store()).unwrap()
}

fn scope<'a>(workspace: Option<&'a str>, account: Option<&'a str>) -> Scope<'a> {
    Scope { workspace, account }
}

#[test]
fn a_bad_since_or_until_is_invalid_with_pythons_text() {
    let conn = conn_of(DEFAULT);
    let want = DEFAULT.read("api__usage_sessions__bad_since.json")["error"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(want, "Invalid isoformat string: 'bad'");
    for result in [
        session_timeline(&conn, Some("bad"), None, Some(DETAIL)),
        session_outliers(&conn, Some("bad"), None),
        session_timeline(&conn, None, Some("bad"), None),
        session_outliers(&conn, None, Some("bad")),
    ] {
        assert_eq!(result.unwrap_err().to_string(), want);
    }
    // the period is read before the scope, as in Python (`_utc_bound` first)
    let both = session_timeline_scoped(&conn, Some("bad"), None, None, scope(Some("nope"), None));
    assert_eq!(both.unwrap_err().to_string(), want);
}

/// The text is Python's and echoes no input (NFR-002); the workspace is checked before the account.
#[test]
fn an_unknown_scope_answers_python_text_and_echoes_nothing() {
    let conn = conn_of(RICH);
    let secret = "/home/someone/secret-folder";
    for text in [
        session_timeline_scoped(&conn, None, None, None, scope(Some(secret), Some(secret))),
        session_outliers_scoped(&conn, None, None, scope(Some(secret), Some(secret))),
    ]
    .map(|r| r.unwrap_err().to_string())
    {
        assert_eq!(text, "unknown workspace");
    }
    for text in [
        session_timeline_scoped(&conn, None, None, None, scope(Some("acme"), Some(secret))),
        session_outliers_scoped(&conn, None, None, scope(Some("acme"), Some(secret))),
    ]
    .map(|r| r.unwrap_err().to_string())
    {
        assert_eq!(
            text,
            "unknown account; valid labels: Claude default, Claude work"
        );
        assert!(!text.contains("secret"));
    }
}

/// An empty string is no filter (Python's `or None`), and the plain functions are the scoped ones with no scope.
#[test]
fn the_plain_functions_and_an_empty_scope_equal_the_unscoped_golden() {
    let conn = conn_of(RICH);
    let empty = scope(Some(""), Some(""));
    let timeline = RICH.read("api__usage_sessions__all.json");
    let outliers = RICH.read("api__usage_outliers__all.json");
    assert_eq!(
        session_timeline_scoped(&conn, None, None, Some(DETAIL), empty).unwrap(),
        timeline
    );
    assert_eq!(
        session_timeline(&conn, None, None, Some(DETAIL)).unwrap(),
        timeline
    );
    assert_eq!(
        session_outliers_scoped(&conn, None, None, empty).unwrap(),
        outliers
    );
    assert_eq!(session_outliers(&conn, None, None).unwrap(), outliers);
    // the same answer as the MCP golden, which has no scope
    assert_eq!(outliers, RICH.read("mcp__session_outliers__all.json"));
}

/// A workspace with no repo (`orphan`) selects nothing, not everything (`0` clause); Python's answer for the same store.
#[test]
fn a_workspace_with_no_matching_project_selects_nothing() {
    let conn = conn_of(RICH);
    let only = scope(Some("orphan"), None);
    let timeline = session_timeline_scoped(&conn, None, None, Some(DETAIL), only).unwrap();
    assert_eq!(
        timeline,
        json!({"detail_sessions": 50, "gap_minutes": 30, "sessions": [],
               "period": {"since_utc": null, "until_utc": null}})
    );
    let outliers = session_outliers_scoped(&conn, None, None, only).unwrap();
    assert_eq!(outliers["sessions"], json!([]));
    assert_eq!(
        outliers["thresholds"],
        json!({"tool_calls": 90, "context_tokens": 200_000})
    );
}

// ---- 3. the local zone: offset-less bounds ----

/// A window written as local time in a zone, and what it must give: a golden (the same instants as a golden's `Z`
/// window) or an answer Python gave for exactly these bounds.
struct ZoneCase {
    zone: &'static str,
    oracle: Oracle,
    since: &'static str,
    until: &'static str,
    golden: Option<&'static str>,
}

/// How the expected values were taken: on a scratch copy of each store, `TZ=<zone>` and
/// `rust/parity/.venv/bin/python -B -I` calling `metrics.session_timeline(conn, since, until, detail_sessions=50)` and
/// `metrics.session_outliers(conn, since, until)` with the offset-less bounds below, `EASY_PERSONAL_HOME` a scratch
/// folder. Kiritimati (+14) and Pago_Pago (-11) gave the golden's answer for the instants it names; Kolkata (+5:30) has
/// no golden, its answer is `KOLKATA`.
const ZONE_CASES: [ZoneCase; 4] = [
    ZoneCase {
        zone: "Pacific/Kiritimati",
        oracle: DEFAULT,
        since: "2026-01-16T00:05:00",
        until: "2026-01-16T00:15:00",
        golden: Some("api__usage_sessions__window.json"),
    },
    ZoneCase {
        zone: "Pacific/Kiritimati",
        oracle: RICH,
        since: "2026-01-13T08:00:00",
        until: "2026-01-15T20:00:00",
        golden: Some("api__usage_sessions__window_mid.json"),
    },
    ZoneCase {
        zone: "Pacific/Pago_Pago",
        oracle: RICH,
        since: "2026-01-12T07:00:00",
        until: "2026-01-14T19:00:00",
        golden: Some("api__usage_sessions__window_mid.json"),
    },
    ZoneCase {
        zone: "Asia/Kolkata",
        oracle: DEFAULT,
        since: "2026-01-15T15:36:00",
        until: "2026-01-15T15:46:30",
        golden: None,
    },
];

const KOLKATA: &str = r#"{"outliers":{"period":{"since_utc":"2026-01-15T10:06:00.000000Z","until_utc":"2026-01-15T10:16:30.000000Z"},"sessions":[],"thresholds":{"context_tokens":200000,"tool_calls":90}},"timeline":{"detail_sessions":50,"gap_minutes":30,"period":{"since_utc":"2026-01-15T10:06:00.000000Z","until_utc":"2026-01-15T10:16:30.000000Z"},"sessions":[{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"end_utc":"2026-01-15T10:07:00.000000Z","gaps":[],"input_tokens":0,"messages":0,"models":[],"output_tokens":0,"points":[],"project":"demo-app","session_id":"00000000-0000-4000-8000-000000000001","start_utc":"2026-01-15T10:06:00.000000Z"},{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"end_utc":"2026-01-15T10:07:00.000000Z","gaps":[],"input_tokens":0,"messages":0,"models":[],"output_tokens":0,"points":[],"project":"demo-app","session_id":"00000000-0000-4000-8000-000000000002","start_utc":"2026-01-15T10:06:00.000000Z"}]}}"#;

#[test]
#[ignore = "run by offset_less_bounds_follow_the_local_zone, in a child process with its own TZ"]
fn zone_child() {
    let n: usize = std::env::var("T018_ZONE_CASE").unwrap().parse().unwrap();
    let case = &ZONE_CASES[n];
    let conn = conn_of(case.oracle);
    let timeline =
        session_timeline(&conn, Some(case.since), Some(case.until), Some(DETAIL)).unwrap();
    match case.golden {
        Some(file) => assert_eq!(timeline, case.oracle.read(file)),
        None => {
            let want: Value = serde_json::from_str(KOLKATA).unwrap();
            assert_eq!(timeline, want["timeline"]);
            let outliers = session_outliers(&conn, Some(case.since), Some(case.until)).unwrap();
            assert_eq!(outliers, want["outliers"]);
        }
    }
    println!("zone case {n} ok");
}

#[test]
fn offset_less_bounds_follow_the_local_zone() {
    for (n, case) in ZONE_CASES.iter().enumerate() {
        let (ok, text) = run_child(
            "zone_child",
            &[("T018_ZONE_CASE", Path::new(&n.to_string()))],
            case.zone,
        );
        assert!(ok, "{} case {n}: {text}", case.zone);
    }
}

/// Under `TZ=UTC` an offset-less bound is the `Z` bound of the same text, and the +14 h text of the Kiritimati case
/// reads as another window: the zone is really applied, not ignored.
#[test]
#[ignore = "run by under_utc_an_offset_less_bound_is_the_z_bound, in a child process with TZ=UTC"]
fn utc_child() {
    let conn = conn_of(DEFAULT);
    let timeline = session_timeline(
        &conn,
        Some("2026-01-15T10:05:00"),
        Some("2026-01-15T10:15:00"),
        Some(DETAIL),
    )
    .unwrap();
    assert_eq!(timeline, DEFAULT.read("api__usage_sessions__window.json"));
    let moved = session_timeline(
        &conn,
        Some("2026-01-16T00:05:00"),
        Some("2026-01-16T00:15:00"),
        Some(DETAIL),
    )
    .unwrap();
    assert_ne!(moved, timeline);
    println!("utc ok");
}

#[test]
fn under_utc_an_offset_less_bound_is_the_z_bound() {
    let (ok, text) = run_child("utc_child", &[], "UTC");
    assert!(ok, "{text}");
}

// ---- 4. the rules the goldens do not reach: a hand-made store, Python's answers ----

/// `store-v11.sqlite` with its events replaced by these (334 rows): exact and just-under gaps, gaps on a `.x5` edge,
/// calls with no priced message and the reverse, an empty `tool_names`, a context of 199,999, an event with no time, an
/// event with no session, same-time events, a non-ASCII session id. Python's answers for it are in the constants below;
/// they were taken from `metrics.session_timeline` / `session_outliers` over a scratch copy of the store with the same SQL,
/// `rust/parity/.venv/bin/python -B -I`, a scratch `EASY_PERSONAL_HOME`.
const EDGE_SQL: &str = r#"DELETE FROM events;
-- sess-a: gaps of exactly 30 min, just under it, and three that round on a .x5 edge; project path of the first event is empty
INSERT INTO events (id, machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id, project_path, model, message_id, tool_names, input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens) VALUES
 (1, 'm', 'claude_code', 'p', 1, 1, 0, '2026-02-01T10:00:00.000000Z', 'sess-a', '', NULL, NULL, NULL, NULL, NULL, NULL, NULL),
 (2, 'm', 'claude_code', 'p', 1, 2, 0, '2026-02-01T10:10:00.000000Z', 'sess-a', '/w/p1', 'model-x', 'm1', 'Read', 10, 5, 1, 2),
 (3, 'm', 'claude_code', 'p', 1, 3, 0, '2026-02-01T10:40:00.000000Z', 'sess-a', '/w/p1', 'model-x', 'm1', NULL, 10, 5, 1, 2),
 (4, 'm', 'claude_code', 'p', 1, 4, 0, '2026-02-01T11:09:59.999999Z', 'sess-a', '/w/p1', NULL, NULL, NULL, NULL, NULL, NULL, NULL),
 (5, 'm', 'claude_code', 'p', 1, 5, 0, '2026-02-01T11:40:02.999999Z', 'sess-a', '/w/p1', 'model-y', 'm2', 'Edit,Bash', 7, NULL, NULL, NULL),
 (6, 'm', 'claude_code', 'p', 1, 6, 0, '2026-02-01T12:10:11.999999Z', 'sess-a', '/w/p1', '', 'm3', NULL, 1, 1, 1, 1),
 (7, 'm', 'claude_code', 'p', 1, 7, 0, '2026-02-01T12:40:26.999999Z', 'sess-a', '/w/p1', NULL, NULL, NULL, NULL, NULL, NULL, NULL),
 (8, 'm', 'claude_code', 'p', 1, 8, 0, '2026-02-01T13:10:47.999999Z', 'sess-a', '/w/p1', NULL, NULL, NULL, NULL, NULL, NULL, NULL),
 (9, 'm', 'claude_code', 'p', 1, 9, 1, '2026-02-01T23:59:00.000000Z', 'sess-a', '/w/p1', NULL, NULL, 'Read', NULL, NULL, NULL, NULL);
-- sess-b: 90 calls and no priced message; ties with sess-f and sess-é on tool calls
INSERT INTO events (id, machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id, project_path, tool_names)
 WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 90)
 SELECT 100 + i, 'm', 'claude_code', 'p', 2, i, 0, '2026-02-01T09:00:00.000000Z', 'sess-b', '/w/p2/', 'Bash' FROM n;
-- sess-f: 45 events of two tools = 90 calls (the split is by comma); sess-é (non-ASCII id) 90 calls
INSERT INTO events (id, machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id, project_path, tool_names)
 WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 45)
 SELECT 300 + i, 'm', 'claude_code', 'p', 3, i, 0, '2026-02-01T09:00:00.000000Z', 'sess-f', '/w/p2', 'Read,Edit' FROM n;
INSERT INTO events (id, machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id, project_path, tool_names)
 WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 90)
 SELECT 400 + i, 'm', 'claude_code', 'p', 4, i, 0, '2026-02-01T09:00:00.000000Z', 'sess-é', NULL, 'Grep' FROM n;
-- sess-c: a big context and no tool call (in contexts, not in calls); sess-d: 89 calls and context 199999 (not an outlier)
INSERT INTO events (id, machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id, project_path, model, message_id, input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens) VALUES
 (500, 'm', 'claude_code', 'p', 5, 1, 0, '2026-02-02T08:00:00.000000Z', 'sess-c', '/w/p3', 'model-x', 'c1', 150000, 1, 50000, 10000),
 (501, 'm', 'claude_code', 'p', 5, 2, 0, '2026-02-02T08:05:00.000000Z', 'sess-c', '/w/p3', 'model-x', 'c2', 1, 1, NULL, NULL),
 (600, 'm', 'claude_code', 'p', 6, 1, 0, '2026-02-02T08:00:00.000000Z', 'sess-d', '/w/p3', 'model-x', 'd1', 199999, 1, NULL, NULL);
INSERT INTO events (id, machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id, project_path, tool_names)
 WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 89)
 SELECT 700 + i, 'm', 'claude_code', 'p', 7, i, 0, '2026-02-02T08:00:00.000000Z', 'sess-d', '/w/p3', 'Bash' FROM n;
-- sess-n: events with no time (the timeline skips them, a window excludes them), plus an abstained one and one with no session
INSERT INTO events (id, machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id, project_path, tool_names) VALUES
 (800, 'm', 'claude_code', 'p', 8, 1, 0, NULL, 'sess-n', '/w/p4', 'Read'),
 (801, 'm', 'claude_code', 'p', 8, 2, 0, '2026-02-02T09:00:00.000000Z', 'sess-n', '/w/p4', 'Read'),
 (802, 'm', 'claude_code', 'p', 8, 3, 0, '2026-02-02T09:00:00.000000Z', NULL, '/w/p4', 'Read');
-- sess-t: two messages, three events at the same time (the project comes from the first non-empty path), ties on the start with sess-u
INSERT INTO events (id, machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id, project_path, model, message_id, input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens) VALUES
 (900, 'm', 'claude_code', 'p', 9, 1, 0, '2026-02-03T10:00:00.000000Z', 'sess-u', '/w/p5', 'model-x', 'u1', 3, 3, 3, 3),
 (901, 'm', 'claude_code', 'p', 9, 2, 0, '2026-02-03T10:00:00.000000Z', 'sess-t', NULL, 'model-y', 't1', 4, 4, 4, 4),
 (902, 'm', 'claude_code', 'p', 9, 3, 0, '2026-02-03T10:00:00.000000Z', 'sess-t', '/w/p6', 'model-y', 't1', 4, 4, 4, 4),
 (903, 'm', 'claude_code', 'p', 9, 4, 0, '2026-02-03T10:00:00.000000Z', 'sess-t', '/w/p7', NULL, NULL, NULL, NULL, NULL, NULL),
 (904, 'm', 'claude_code', 'p', 9, 5, 0, '2026-02-03T10:45:00.000000Z', 'sess-t', '/w/p6', 'model-y', 't2', 5, 5, 5, 5);
"#;
const EDGE_TIMELINE_ALL: &str = r#"{"gap_minutes":30,"period":{"since_utc":null,"until_utc":null},"sessions":[{"cache_creation_input_tokens":9,"cache_read_input_tokens":9,"end_utc":"2026-02-03T10:45:00.000000Z","gaps":[{"end_utc":"2026-02-03T10:45:00.000000Z","minutes":45.0,"start_utc":"2026-02-03T10:00:00.000000Z"}],"input_tokens":9,"messages":2,"models":["model-y"],"output_tokens":9,"points":[{"cache_creation_input_tokens":4,"cache_read_input_tokens":4,"input_tokens":4,"model":"model-y","output_tokens":4,"ts_utc":"2026-02-03T10:00:00.000000Z"},{"cache_creation_input_tokens":5,"cache_read_input_tokens":5,"input_tokens":5,"model":"model-y","output_tokens":5,"ts_utc":"2026-02-03T10:45:00.000000Z"}],"project":"p7","session_id":"sess-t","start_utc":"2026-02-03T10:00:00.000000Z"},{"cache_creation_input_tokens":3,"cache_read_input_tokens":3,"end_utc":"2026-02-03T10:00:00.000000Z","gaps":[],"input_tokens":3,"messages":1,"models":["model-x"],"output_tokens":3,"points":[{"cache_creation_input_tokens":3,"cache_read_input_tokens":3,"input_tokens":3,"model":"model-x","output_tokens":3,"ts_utc":"2026-02-03T10:00:00.000000Z"}],"project":"p5","session_id":"sess-u","start_utc":"2026-02-03T10:00:00.000000Z"},{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"end_utc":"2026-02-02T09:00:00.000000Z","gaps":[],"input_tokens":0,"messages":0,"models":[],"output_tokens":0,"points":[],"project":"p4","session_id":"sess-n","start_utc":"2026-02-02T09:00:00.000000Z"},{"cache_creation_input_tokens":50000,"cache_read_input_tokens":10000,"end_utc":"2026-02-02T08:05:00.000000Z","gaps":[],"input_tokens":150001,"messages":2,"models":["model-x"],"output_tokens":2,"points":[{"cache_creation_input_tokens":50000,"cache_read_input_tokens":10000,"input_tokens":150000,"model":"model-x","output_tokens":1,"ts_utc":"2026-02-02T08:00:00.000000Z"},{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"input_tokens":1,"model":"model-x","output_tokens":1,"ts_utc":"2026-02-02T08:05:00.000000Z"}],"project":"p3","session_id":"sess-c","start_utc":"2026-02-02T08:00:00.000000Z"},{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"end_utc":"2026-02-02T08:00:00.000000Z","gaps":[],"input_tokens":199999,"messages":1,"models":["model-x"],"output_tokens":1,"points":[{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"input_tokens":199999,"model":"model-x","output_tokens":1,"ts_utc":"2026-02-02T08:00:00.000000Z"}],"project":"p3","session_id":"sess-d","start_utc":"2026-02-02T08:00:00.000000Z"},{"cache_creation_input_tokens":2,"cache_read_input_tokens":3,"end_utc":"2026-02-01T13:10:47.999999Z","gaps":[{"end_utc":"2026-02-01T10:40:00.000000Z","minutes":30.0,"start_utc":"2026-02-01T10:10:00.000000Z"},{"end_utc":"2026-02-01T11:40:02.999999Z","minutes":30.1,"start_utc":"2026-02-01T11:09:59.999999Z"},{"end_utc":"2026-02-01T12:10:11.999999Z","minutes":30.1,"start_utc":"2026-02-01T11:40:02.999999Z"},{"end_utc":"2026-02-01T12:40:26.999999Z","minutes":30.2,"start_utc":"2026-02-01T12:10:11.999999Z"},{"end_utc":"2026-02-01T13:10:47.999999Z","minutes":30.4,"start_utc":"2026-02-01T12:40:26.999999Z"}],"input_tokens":18,"messages":3,"models":["model-x","model-y"],"output_tokens":6,"points":[{"cache_creation_input_tokens":1,"cache_read_input_tokens":2,"input_tokens":10,"model":"model-x","output_tokens":5,"ts_utc":"2026-02-01T10:10:00.000000Z"},{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"input_tokens":7,"model":"model-y","output_tokens":0,"ts_utc":"2026-02-01T11:40:02.999999Z"},{"cache_creation_input_tokens":1,"cache_read_input_tokens":1,"input_tokens":1,"model":"","output_tokens":1,"ts_utc":"2026-02-01T12:10:11.999999Z"}],"project":"p1","session_id":"sess-a","start_utc":"2026-02-01T10:00:00.000000Z"},{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"end_utc":"2026-02-01T09:00:00.000000Z","gaps":[],"input_tokens":0,"messages":0,"models":[],"output_tokens":0,"points":[],"project":"p2","session_id":"sess-b","start_utc":"2026-02-01T09:00:00.000000Z"},{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"end_utc":"2026-02-01T09:00:00.000000Z","gaps":[],"input_tokens":0,"messages":0,"models":[],"output_tokens":0,"points":[],"project":"p2","session_id":"sess-f","start_utc":"2026-02-01T09:00:00.000000Z"},{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"end_utc":"2026-02-01T09:00:00.000000Z","gaps":[],"input_tokens":0,"messages":0,"models":[],"output_tokens":0,"points":[],"project":"(unknown)","session_id":"sess-é","start_utc":"2026-02-01T09:00:00.000000Z"}]}"#;
const EDGE_TIMELINE_WINDOW: &str = r#"{"detail_sessions":50,"gap_minutes":30,"period":{"since_utc":"2026-02-01T10:20:00.000000Z","until_utc":"2026-02-01T12:30:00.000000Z"},"sessions":[{"cache_creation_input_tokens":1,"cache_read_input_tokens":1,"end_utc":"2026-02-01T12:10:11.999999Z","gaps":[{"end_utc":"2026-02-01T11:40:02.999999Z","minutes":30.1,"start_utc":"2026-02-01T11:09:59.999999Z"},{"end_utc":"2026-02-01T12:10:11.999999Z","minutes":30.1,"start_utc":"2026-02-01T11:40:02.999999Z"}],"input_tokens":8,"messages":2,"models":["model-y"],"output_tokens":1,"points":[{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"input_tokens":7,"model":"model-y","output_tokens":0,"ts_utc":"2026-02-01T11:40:02.999999Z"},{"cache_creation_input_tokens":1,"cache_read_input_tokens":1,"input_tokens":1,"model":"","output_tokens":1,"ts_utc":"2026-02-01T12:10:11.999999Z"}],"project":"p1","session_id":"sess-a","start_utc":"2026-02-01T10:40:00.000000Z"}]}"#;
const EDGE_OUTLIERS_ALL: &str = r#"{"period":{"since_utc":null,"until_utc":null},"sessions":[{"max_context_tokens":0,"project":"p2","reasons":["tool_calls"],"session_id":"sess-b","tool_calls":90},{"max_context_tokens":0,"project":"p2","reasons":["tool_calls"],"session_id":"sess-f","tool_calls":90},{"max_context_tokens":0,"project":"(unknown)","reasons":["tool_calls"],"session_id":"sess-é","tool_calls":90},{"max_context_tokens":210000,"project":"p3","reasons":["context_tokens"],"session_id":"sess-c","tool_calls":0}],"thresholds":{"context_tokens":200000,"tool_calls":90}}"#;
const EDGE_OUTLIERS_WINDOW: &str = r#"{"period":{"since_utc":"2026-02-02T08:00:00.000000Z","until_utc":"2026-02-02T08:05:00.000000Z"},"sessions":[{"max_context_tokens":210000,"project":"p3","reasons":["context_tokens"],"session_id":"sess-c","tool_calls":0}],"thresholds":{"context_tokens":200000,"tool_calls":90}}"#;

fn edge() -> (TempDir, Connection) {
    let dir = TempDir::new();
    let path = changed_store(&dir, &DEFAULT.store(), EDGE_SQL);
    let conn = open_read_only(&path).unwrap();
    (dir, conn)
}

fn parsed(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

#[test]
fn the_edge_store_timeline_equals_pythons() {
    let (_dir, conn) = edge();
    let full = session_timeline(&conn, None, None, None).unwrap();
    assert_eq!(full, parsed(EDGE_TIMELINE_ALL));
    assert!(
        full.get("detail_sessions").is_none(),
        "only present when given"
    );
    // a window that starts inside a session: start/end from the events in the window, points from the inner window
    let window = session_timeline(
        &conn,
        Some("2026-02-01T10:20:00Z"),
        Some("2026-02-01T12:30:00Z"),
        Some(DETAIL),
    )
    .unwrap();
    assert_eq!(window, parsed(EDGE_TIMELINE_WINDOW));
}

/// Gaps: exactly 30 min is a gap, 29 min 59.999999 s is not; minutes are Python's `round(x, 1)` on the float
/// (half to even on the exact binary value): 30.05 -> 30.1, 30.15 -> 30.1 (the float is 30.1499...), 30.25 -> 30.2,
/// 30.35 -> 30.4.
#[test]
fn gaps_start_at_30_minutes_and_round_like_python() {
    let (_dir, conn) = edge();
    let full = session_timeline(&conn, None, None, None).unwrap();
    let a = full["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["session_id"] == "sess-a")
        .unwrap();
    let minutes: Vec<&Value> = a["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| &g["minutes"])
        .collect();
    assert_eq!(
        minutes,
        [
            &json!(30.0),
            &json!(30.1),
            &json!(30.1),
            &json!(30.2),
            &json!(30.4)
        ]
    );
    assert!(minutes.iter().all(|m| m.is_f64()), "minutes stay floats");
    assert_eq!(
        a["gaps"].as_array().unwrap().len(),
        5,
        "the 29:59.999999 gap is no gap"
    );
}

/// `detail_sessions` of 0, 1, more than the sessions, and the key present only when given: only the newest N carry
/// `points` and `gaps`, every total stays. (Python's answers for 0, 1, 3 and 100 were checked to be exactly this.)
#[test]
fn detail_sessions_blanks_points_and_gaps_after_the_newest_n() {
    let (_dir, conn) = edge();
    let full = session_timeline(&conn, None, None, None).unwrap();
    let count = full["sessions"].as_array().unwrap().len();
    assert_eq!(count, 9);
    for n in [0, 1, 3, 100] {
        let got = session_timeline(&conn, None, None, Some(n)).unwrap();
        let mut want = full.clone();
        want["detail_sessions"] = json!(n);
        for (rank, s) in want["sessions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            if rank >= n {
                s["points"] = json!([]);
                s["gaps"] = json!([]);
            }
        }
        assert_eq!(got, want, "detail_sessions {n}");
        assert_eq!(got["detail_sessions"], json!(n));
    }
    // sess-a carries gaps in the full answer, none once it is past the newest N
    let gaps = |v: &Value| {
        v["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["session_id"] == "sess-a")
            .unwrap()["gaps"]
            .as_array()
            .unwrap()
            .len()
    };
    assert_eq!(gaps(&full), 5);
    assert_eq!(
        gaps(&session_timeline(&conn, None, None, Some(5)).unwrap()),
        0
    );
    assert_eq!(
        gaps(&session_timeline(&conn, None, None, Some(6)).unwrap()),
        5
    );
}

/// Newest first by the session's first event; a tie keeps the ascending session id (Python sorts the ids, then does a
/// stable reversed sort on the start): `sess-t` before `sess-u`, and `sess-b`, `sess-f`, `sess-é` in id order with the
/// non-ASCII id last (UTF-8 byte order is code point order, the order Python compares `str` in).
#[test]
fn the_timeline_is_newest_first_and_ties_follow_the_session_id() {
    let (_dir, conn) = edge();
    let full = session_timeline(&conn, None, None, None).unwrap();
    let ids: Vec<&str> = full["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["session_id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        [
            "sess-t", "sess-u", "sess-n", "sess-c", "sess-d", "sess-a", "sess-b", "sess-f",
            "sess-é"
        ]
    );
    // an event with no time is skipped by the timeline (sess-n starts at its only timed event)
    let n = &full["sessions"][2];
    assert_eq!(n["start_utc"], "2026-02-02T09:00:00.000000Z");
}

#[test]
fn the_edge_store_outliers_equal_pythons() {
    let (_dir, conn) = edge();
    let all = session_outliers(&conn, None, None).unwrap();
    assert_eq!(all, parsed(EDGE_OUTLIERS_ALL));
    // tool_calls 90 (one tool name per event, comma pairs, a non-ASCII id) in id order, then the context-only session;
    // 89 calls with a context of 199,999 is no outlier
    let ids: Vec<&str> = all["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["session_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["sess-b", "sess-f", "sess-é", "sess-c"]);
    let window = session_outliers(
        &conn,
        Some("2026-02-02T08:00:00Z"),
        Some("2026-02-02T08:05:00Z"),
    )
    .unwrap();
    assert_eq!(window, parsed(EDGE_OUTLIERS_WINDOW));
}

/// The 90-call and 200,000-token thresholds are inclusive, an empty `tool_names` counts as one call, and a session
/// with calls but no priced message (or the reverse) is still listed.
#[test]
fn outlier_thresholds_are_inclusive_and_an_empty_tool_list_is_one_call() {
    let dir = TempDir::new();
    let path = changed_store(
        &dir,
        &DEFAULT.store(),
        "DELETE FROM events;
         INSERT INTO events (id, machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id,
                             project_path, tool_names)
           WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 90)
           SELECT i, 'm', 'claude_code', 'p', 1, i, 0, '2026-02-01T09:00:00.000000Z', 'empty-names', '/w/x', ''
           FROM n;
         INSERT INTO events (id, machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id,
                             project_path, model, message_id, input_tokens, cache_creation_input_tokens, cache_read_input_tokens)
           VALUES (200, 'm', 'claude_code', 'p', 2, 1, 0, '2026-02-01T09:00:00.000000Z', 'ctx-exact', '/w/y', 'x', 'a', 100000, 50000, 50000),
                  (201, 'm', 'claude_code', 'p', 2, 2, 0, '2026-02-01T09:00:00.000000Z', 'ctx-under', '/w/y', 'x', 'b', 100000, 50000, 49999);",
    );
    let conn = open_read_only(&path).unwrap();
    let got = session_outliers(&conn, None, None).unwrap();
    let rows: Vec<(String, i64, i64, Vec<String>)> = got["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            (
                s["session_id"].as_str().unwrap().to_string(),
                s["tool_calls"].as_i64().unwrap(),
                s["max_context_tokens"].as_i64().unwrap(),
                s["reasons"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| r.as_str().unwrap().to_string())
                    .collect(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            (
                "empty-names".to_string(),
                90,
                0,
                vec!["tool_calls".to_string()]
            ),
            (
                "ctx-exact".to_string(),
                0,
                200_000,
                vec!["context_tokens".to_string()]
            ),
        ]
    );
}

// ---- 5. the store is only read, no error leaks, and the tests are not constant ----

#[test]
fn the_invented_stores_are_unchanged_and_get_no_sidecars() {
    let before: Vec<Vec<u8>> = [DEFAULT, RICH]
        .iter()
        .map(|o| fs::read(o.store()).unwrap())
        .collect();
    for oracle in [DEFAULT, RICH] {
        compare(oracle, &oracle.store());
        let conn = conn_of(oracle);
        session_timeline(&conn, None, None, None).unwrap();
        session_outliers(&conn, None, None).unwrap();
        drop(conn);
    }
    let conn = conn_of(RICH);
    session_timeline_scoped(
        &conn,
        None,
        None,
        Some(DETAIL),
        scope(Some("acme"), Some("2")),
    )
    .unwrap();
    session_outliers_scoped(&conn, None, None, scope(Some("acme"), Some("2"))).unwrap();
    drop(conn);
    for (oracle, bytes) in [DEFAULT, RICH].iter().zip(before) {
        assert_eq!(fs::read(oracle.store()).unwrap(), bytes, "{}", oracle.name);
        for suffix in ["-journal", "-wal", "-shm"] {
            let mut name = oracle.store().into_os_string();
            name.push(suffix);
            assert!(
                !Path::new(&name).exists(),
                "{}: sidecar {suffix} appeared",
                oracle.name
            );
        }
    }
}

/// A store that is not a store: the error names no path, no SQL, no table (NFR-002).
#[test]
fn a_store_failure_carries_no_path_sql_or_row_text() {
    let dir = TempDir::new();
    let path = dir.0.join("empty-store.sqlite");
    Connection::open(&path)
        .unwrap()
        .execute_batch("CREATE TABLE other (x TEXT);")
        .unwrap();
    let conn = open_read_only(&path).unwrap();
    let dir_text = dir.0.to_string_lossy().into_owned();
    for text in [
        session_timeline(&conn, None, None, None)
            .unwrap_err()
            .to_string(),
        session_outliers(&conn, None, None).unwrap_err().to_string(),
    ] {
        assert!(text.starts_with("cannot read the store:"), "{text}");
        for leak in [dir_text.as_str(), "SELECT", "events", "session_id", "FROM"] {
            assert!(!text.contains(leak), "{leak:?} in {text}");
        }
    }
}

/// A caller's text only ever reaches SQLite as a bound value: a quote in a scope or a bound changes no statement.
#[test]
fn input_with_sql_text_is_a_bound_value_only() {
    let conn = conn_of(RICH);
    let hostile = "x' OR '1'='1";
    let err =
        session_timeline_scoped(&conn, None, None, None, scope(Some(hostile), None)).unwrap_err();
    assert_eq!(err.to_string(), "unknown workspace");
    let err = session_outliers_scoped(&conn, None, None, scope(None, Some(hostile))).unwrap_err();
    assert!(err.to_string().starts_with("unknown account;"), "{err}");
    let err = session_outliers(&conn, Some("2026-01-01' OR 1=1 --"), None).unwrap_err();
    assert!(
        err.to_string().starts_with("Invalid isoformat string:"),
        "{err}"
    );
    // the rich store still answers, so nothing was dropped or changed
    assert_eq!(
        session_outliers(&conn, None, None).unwrap(),
        RICH.read("api__usage_outliers__all.json")
    );
}

/// A changed `ts_utc` in a temp copy: the comparison fails and names the entries that differ.
#[test]
fn a_changed_ts_utc_breaks_the_comparison_and_names_the_entries() {
    let dir = TempDir::new();
    let path = changed_store(
        &dir,
        &RICH.store(),
        "UPDATE events SET ts_utc = '2026-01-05T00:00:00.000000Z' WHERE id = \
         (SELECT MIN(id) FROM events WHERE abstained = 0 AND session_id IS NOT NULL AND ts_utc IS NOT NULL)",
    );
    let (ok, text) = run_child("sessions_child", &[("T018_RICH_STORE", &path)], "UTC");
    assert!(!ok && text.contains("session entries differ"), "{text}");
    assert!(
        text.contains("rich/api__usage_sessions__all.json"),
        "{text}"
    );
    // the untouched default oracle is not among them
    assert!(!text.contains("default/"), "{text}");
}

/// A changed `tool_names`: one event of the 90-call session gets one more tool name, so it has 91 calls.
#[test]
fn a_changed_tool_names_breaks_the_outlier_comparisons() {
    let dir = TempDir::new();
    let path = changed_store(
        &dir,
        &RICH.store(),
        "UPDATE events SET tool_names = tool_names || ',Edit' WHERE id = \
         (SELECT MIN(id) FROM events WHERE session_id = '00000000-0000-4000-8000-000000000138' AND tool_names IS NOT NULL)",
    );
    let (ok, text) = run_child("sessions_child", &[("T018_RICH_STORE", &path)], "UTC");
    assert!(!ok, "{text}");
    for file in [
        "api__usage_outliers__all.json",
        "mcp__session_outliers__all.json",
    ] {
        assert!(
            text.contains(&format!("rich/{file}")),
            "{file} not named in {text}"
        );
    }
    // and the same change seen directly: 91 calls
    let conn = open_read_only(&path).unwrap();
    let got = session_outliers(&conn, None, None).unwrap();
    assert_eq!(got["sessions"][0]["tool_calls"], json!(91));
}

/// A changed `ts_utc` in the default store breaks an entry of that oracle too (both stores are compared).
#[test]
fn a_changed_event_in_the_default_store_is_named_too() {
    let dir = TempDir::new();
    let path = changed_store(
        &dir,
        &DEFAULT.store(),
        "UPDATE events SET ts_utc = '2026-01-15T10:30:00.000000Z' WHERE id = 5",
    );
    let (ok, text) = run_child("sessions_child", &[("T018_DEFAULT_STORE", &path)], "UTC");
    assert!(!ok, "{text}");
    assert!(
        text.contains("default/api__usage_sessions__all.json"),
        "{text}"
    );
}
