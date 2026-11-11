//! T017: the `workspace` and `account` scope in the core (`usage_summary_scoped`, `usage_breakdown_scoped`,
//! `rules::evaluate_scoped`, `plan::improvement_plan_scoped`) against the scoped RICH Python goldens in
//! `rust/parity/golden-rich`, made by `gen_golden.py --rich` over `rust/parity/store-rich/store-v11.sqlite`.
//!
//! The 41 entries come from the manifest: every `usage_summary`, `usage_breakdown`, `warnings` and plan entry (HTTP or
//! MCP) whose parameters name a `workspace` or an `account`, the unknown ones included. Compared as parsed JSON (keys
//! sorted in the files, list order kept; `1000` and `1000.0` differ). An error entry (HTTP 400) must be an error in
//! Rust with the same text.
//!
//! The usage comparison runs in a child process with `TZ=UTC` (the goldens were made under UTC and the `day` key is a
//! local day; the zone is cached per process, as in `usage_tz_parity.rs`). The rules read no local time. The owner's real
//! `~/.easy-personal` is never opened: only the invented store and temp copies of it.

use easy_personal_core::metrics::{usage_breakdown_scoped, usage_summary, usage_summary_scoped};
use easy_personal_core::plan::{improvement_plan, improvement_plan_scoped};
use easy_personal_core::rules::{evaluate, evaluate_scoped, shipped_rules};
use easy_personal_core::scope::Scope;
use easy_personal_core::store::open_read_only;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

/// 2026-01-20T00:00:00Z in microseconds: `frozen_now` of the manifest.
const FROZEN_NOW: i64 = 1_768_867_200_000_000;

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

fn entries() -> Vec<Value> {
    manifest()["entries"].as_array().unwrap().clone()
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

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Summary,
    Breakdown,
    Warnings,
    Plan,
}

/// The slice that answers an entry, by route (HTTP) or tool name (MCP).
fn kind(entry: &Value) -> Option<Kind> {
    let name = entry["route"]
        .as_str()
        .map(|r| match r {
            "/api/usage/summary" => "usage_summary",
            "/api/usage/breakdown" => "usage_breakdown",
            "/api/warnings" => "warnings",
            "/api/plan" => "improvement_plan",
            _ => "",
        })
        .or_else(|| entry["tool"].as_str())?;
    match name {
        "usage_summary" => Some(Kind::Summary),
        "usage_breakdown" => Some(Kind::Breakdown),
        "warnings" => Some(Kind::Warnings),
        "improvement_plan" => Some(Kind::Plan),
        _ => None,
    }
}

fn is_scoped(entry: &Value) -> bool {
    ["workspace", "account"]
        .iter()
        .any(|k| entry["params"].get(k).is_some())
}

/// The scoped entries of the four slices; no entry with request headers (a guard entry) can be among them.
fn scoped_entries(wanted: &[Kind]) -> Vec<(Kind, Value)> {
    entries()
        .into_iter()
        .filter(is_scoped)
        .filter_map(|e| kind(&e).filter(|k| wanted.contains(k)).map(|k| (k, e)))
        .collect()
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t017-{}-{}",
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

fn changed_store(dir: &TempDir, sql: &str) -> PathBuf {
    let path = copy_store(dir);
    let n = Connection::open(&path).unwrap().execute(sql, []).unwrap();
    assert!(n >= 1, "the change touched no row: {sql}");
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
        Kind::Summary => usage_summary_scoped(conn, since.as_deref(), until.as_deref(), scope)
            .map_err(|e| e.to_string()),
        Kind::Breakdown => {
            let by = param(entry, "by").unwrap();
            usage_breakdown_scoped(conn, &by, since.as_deref(), until.as_deref(), scope)
                .map_err(|e| e.to_string())
        }
        Kind::Warnings => evaluate_scoped(conn, FROZEN_NOW, &shipped_rules().unwrap(), scope)
            .map_err(|e| e.to_string()),
        Kind::Plan => improvement_plan_scoped(conn, FROZEN_NOW, &shipped_rules().unwrap(), scope)
            .map_err(|e| e.to_string()),
    }
}

/// Compares the scoped entries of `wanted` slices; returns (compared, the entries that differ).
fn compare(store: &Path, wanted: &[Kind]) -> (usize, Vec<String>) {
    let conn = open_read_only(store).unwrap();
    let (mut compared, mut wrong) = (0, Vec::new());
    for (kind, entry) in scoped_entries(wanted) {
        let file = entry["file"].as_str().unwrap().to_string();
        let want = golden(&file);
        compared += 1;
        match (status_of(&entry), answer(&conn, kind, &entry)) {
            (200, Ok(v)) if v == want => {}
            (400, Err(text)) if json!(text) == want["error"] => {}
            (status, other) => wrong.push(format!("{file}: status {status}, Rust gave {other:?}")),
        }
    }
    (compared, wrong)
}

// ---- 1. the 41 goldens ----

/// Runs in the child, started with `TZ=UTC`. `T017_STORE` names a changed copy of the store (the mutation tests).
#[test]
#[ignore = "run by scoped_usage_entries_match_the_goldens, in a child process with TZ=UTC"]
fn scoped_usage_child() {
    let path = std::env::var_os("T017_STORE").map_or_else(store, PathBuf::from);
    let (compared, wrong) = compare(&path, &[Kind::Summary, Kind::Breakdown]);
    assert!(
        wrong.is_empty(),
        "{} scoped usage entries differ: {wrong:#?}",
        wrong.len()
    );
    println!("scoped usage entries compared: {compared}");
}

fn run_usage_child(store: Option<&Path>) -> (bool, String) {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args([
        "--exact",
        "scoped_usage_child",
        "--ignored",
        "--nocapture",
        "--test-threads=1",
    ])
    .env("TZ", "UTC");
    if let Some(path) = store {
        cmd.env("T017_STORE", path);
    }
    let out = cmd.output().unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success() && text.contains("1 passed"), text)
}

fn compared_count(text: &str) -> usize {
    let marker = "scoped usage entries compared: ";
    let at = text
        .find(marker)
        .unwrap_or_else(|| panic!("the child printed no count: {text}"));
    text[at + marker.len()..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap()
}

#[test]
fn scoped_usage_entries_match_the_goldens() {
    let (ok, text) = run_usage_child(None);
    assert!(ok, "{text}");
    // 8 summary + 11 breakdown entries, counted from the manifest
    assert_eq!(compared_count(&text), 19, "{text}");
}

#[test]
fn scoped_warnings_and_plan_entries_match_the_goldens() {
    let (compared, wrong) = compare(&store(), &[Kind::Warnings, Kind::Plan]);
    // 6 + 5 warnings, 6 + 5 plan entries
    assert_eq!(compared, 22);
    assert!(wrong.is_empty(), "scoped entries differ: {wrong:#?}");
}

/// The 41 are counted from the manifest, by slice and surface, so a missing or extra entry fails here.
#[test]
fn the_manifest_holds_41_scoped_entries_of_the_four_slices() {
    let all = scoped_entries(&[Kind::Summary, Kind::Breakdown, Kind::Warnings, Kind::Plan]);
    let count = |k: Kind, surface: &str| {
        all.iter()
            .filter(|(kind, e)| *kind == k && e["surface"] == json!(surface))
            .count()
    };
    assert_eq!(count(Kind::Summary, "http"), 8);
    assert_eq!(count(Kind::Breakdown, "http"), 11);
    assert_eq!(count(Kind::Warnings, "http"), 6);
    assert_eq!(count(Kind::Warnings, "mcp"), 5);
    assert_eq!(count(Kind::Plan, "http"), 6);
    assert_eq!(count(Kind::Plan, "mcp"), 5);
    assert_eq!(all.len(), 41);
    // the error entries: four unknown scopes over HTTP, no guard entry among them
    let errors: Vec<&str> = all
        .iter()
        .filter(|(_, e)| status_of(e) == 400)
        .map(|(_, e)| e["file"].as_str().unwrap())
        .collect();
    assert_eq!(errors.len(), 4, "{errors:?}");
    assert!(all.iter().all(|(_, e)| {
        !e["request_headers"]
            .as_array()
            .is_some_and(|h| !h.is_empty())
    }));
}

// ---- 2. the scope rules the goldens do not name ----

fn rich() -> Connection {
    open_read_only(&store()).unwrap()
}

fn scope<'a>(workspace: Option<&'a str>, account: Option<&'a str>) -> Scope<'a> {
    Scope { workspace, account }
}

/// An empty string is no filter (Python's `or None`): the answer equals the unscoped one, which equals its golden.
#[test]
fn an_empty_workspace_or_account_is_no_filter() {
    let conn = rich();
    let all = golden("api__usage_summary__all.json");
    let empty = scope(Some(""), Some(""));
    assert_eq!(usage_summary_scoped(&conn, None, None, empty).unwrap(), all);
    assert_eq!(usage_summary(&conn, None, None).unwrap(), all);
    assert_eq!(
        usage_breakdown_scoped(&conn, "model", None, None, empty).unwrap(),
        golden("api__usage_breakdown__model.json")
    );
    let rules = shipped_rules().unwrap();
    assert_eq!(
        evaluate_scoped(&conn, FROZEN_NOW, &rules, empty).unwrap(),
        golden("api__warnings__all.json")
    );
    assert_eq!(
        improvement_plan_scoped(&conn, FROZEN_NOW, &rules, empty).unwrap(),
        golden("api__plan__all.json")
    );
    // and the unscoped functions are the scoped ones with no scope
    assert_eq!(
        evaluate(&conn, FROZEN_NOW, &rules).unwrap(),
        evaluate_scoped(&conn, FROZEN_NOW, &rules, Scope::default()).unwrap()
    );
    assert_eq!(
        improvement_plan(&conn, FROZEN_NOW, &rules).unwrap(),
        improvement_plan_scoped(&conn, FROZEN_NOW, &rules, Scope::default()).unwrap()
    );
}

/// An account is an id or a label; the label is compared case-insensitively and the text is stripped
/// (`accounts.resolve`). The default account (id 1) also owns events stored with no account.
#[test]
fn an_account_is_an_id_or_a_label_in_any_case() {
    let conn = rich();
    let work = golden("api__usage_summary__account_work.json");
    let default = golden("api__usage_summary__account_default.json");
    for reference in ["2", " 2 ", "Claude work", "claude WORK", "  CLAUDE work\t"] {
        assert_eq!(
            usage_summary_scoped(&conn, None, None, scope(None, Some(reference))).unwrap(),
            work,
            "{reference:?}"
        );
    }
    for reference in ["1", "Claude default", "claude default"] {
        assert_eq!(
            usage_summary_scoped(&conn, None, None, scope(None, Some(reference))).unwrap(),
            default,
            "{reference:?}"
        );
    }
}

/// Python's `str(id) == ref`: `02`, `+2`, `2.0` are not the id 2; none is a label either.
#[test]
fn only_the_exact_digits_of_an_id_name_an_account() {
    let conn = rich();
    for reference in ["02", "+2", "2.0", "0", "3", "", "Claude", "nope"] {
        let result = usage_summary_scoped(&conn, None, None, scope(None, Some(reference)));
        if reference.is_empty() {
            assert!(result.is_ok(), "an empty account is no filter");
        } else {
            assert_eq!(
                result.unwrap_err().to_string(),
                "unknown account; valid labels: Claude default, Claude work",
                "{reference:?}"
            );
        }
    }
}

/// The error text names no input (NFR-002) and the workspace is checked before the account.
#[test]
fn an_unknown_scope_answers_python_text_and_echoes_nothing() {
    let conn = rich();
    let secret = "/home/someone/secret-folder";
    let both = usage_summary_scoped(&conn, None, None, scope(Some(secret), Some(secret)));
    let text = both.unwrap_err().to_string();
    assert_eq!(text, "unknown workspace");
    assert!(!text.contains("secret"));
    let account = usage_summary_scoped(&conn, None, None, scope(Some("acme"), Some(secret)));
    let text = account.unwrap_err().to_string();
    assert!(text.starts_with("unknown account; valid labels:"), "{text}");
    assert!(!text.contains("secret"));
    // the same for the rules and the plan, even before any history check
    let rules = shipped_rules().unwrap();
    for text in [
        evaluate_scoped(&conn, 0, &rules, scope(Some(secret), None))
            .unwrap_err()
            .to_string(),
        improvement_plan_scoped(&conn, 0, &rules, scope(Some(secret), None))
            .unwrap_err()
            .to_string(),
    ] {
        assert_eq!(text, "unknown workspace");
    }
    // `by` is checked before the scope, as in Python
    let by = usage_breakdown_scoped(&conn, "bad", None, None, scope(Some(secret), None));
    assert_eq!(
        by.unwrap_err().to_string(),
        "by must be one of day, model, project, tool"
    );
}

/// Events stored before migration 5 have no account and belong to the default account (id 1, `CLAUDE_CONFIG_DIR` unset),
/// never to another one (`accounts.resolve`). The rich store has none, so half of account 1's events lose their tag:
/// the answers for account 1 and for account 2 must not change.
#[test]
fn events_with_no_account_belong_to_the_default_account() {
    let dir = TempDir::new();
    let path = changed_store(
        &dir,
        "UPDATE events SET agent_account_id = NULL WHERE agent_account_id = 1 AND id % 2 = 0",
    );
    let conn = open_read_only(&path).unwrap();
    let rules = shipped_rules().unwrap();
    for (reference, summary, warnings) in [
        (
            "1",
            "api__usage_summary__account_default.json",
            "api__warnings__account_default.json",
        ),
        (
            "Claude default",
            "api__usage_summary__account_default.json",
            "api__warnings__account_default.json",
        ),
        (
            "2",
            "api__usage_summary__account_work.json",
            "api__warnings__account_work.json",
        ),
    ] {
        let only = scope(None, Some(reference));
        assert_eq!(
            usage_summary_scoped(&conn, None, None, only).unwrap(),
            golden(summary),
            "{reference}"
        );
        assert_eq!(
            evaluate_scoped(&conn, FROZEN_NOW, &rules, only).unwrap(),
            golden(warnings),
            "{reference}"
        );
    }
}

/// A workspace with no repo (`orphan`) or none that matches a stored project selects nothing (`0`), not everything.
/// Expected values are Python's: `metrics.usage_summary`, `usage_breakdown("model")` and `rules.evaluate` over the same
/// store with `workspace="orphan"` (and the same three answers for a workspace whose one repo no event is under).
#[test]
fn a_workspace_with_no_matching_project_selects_nothing() {
    let zero_summary = json!({
        "abstained": {"by_version": {}, "total": 0}, "cache_creation_input_tokens": 0, "cache_hit_ratio": null,
        "cache_read_input_tokens": 0, "entrypoints": {}, "input_tokens": 0, "messages": 0, "output_tokens": 0,
        "period": {"since_utc": null, "until_utc": null}, "sessions": 0,
        "split": {"agent": 0, "typed": 0, "unknown": 0}});
    let zero_breakdown = json!({
        "by": "model", "groups": [], "period": {"since_utc": null, "until_utc": null},
        "total": {"api_equivalent_cost": {"abstained_models": [], "price_table_date": "2026-10-21", "usd": 0.0},
                  "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0, "input_tokens": 0, "messages": 0,
                  "output_tokens": 0, "split": {"agent": 0, "typed": 0, "unknown": 0}}});
    let abstained = json!([
        {"reason": "no prompt tokens in the window", "rule_id": "cache_hit_low", "title": "Low cache reuse"},
        {"reason": "fewer than two projects with tokens in the window", "rule_id": "project_dominates",
         "title": "One project uses most of your tokens"},
        {"reason": "no records in the window", "rule_id": "records_unreadable",
         "title": "Some Claude Code records are unreadable"},
        {"reason": "no sessions in the window", "rule_id": "session_huge_no_compact", "title": "Sessions grow too large"},
        {"reason": "no tokens in the previous window", "rule_id": "tokens_jump", "title": "Token use jumped"},
        {"reason": "no kernel-trace tool calls in the window", "rule_id": "tool_errors_high", "title": "Many tool calls fail"}]);
    let dir = TempDir::new();
    let path = copy_store(&dir);
    // a workspace whose one repo no stored event is under
    Connection::open(&path)
        .unwrap()
        .execute_batch(
            "INSERT INTO workspaces (id, name, is_orphan) VALUES (9, 'lonely', 0);
             INSERT INTO workspace_repos VALUES (9, '/nowhere/at/all');",
        )
        .unwrap();
    let conn = open_read_only(&path).unwrap();
    let rules = shipped_rules().unwrap();
    for name in ["orphan", "lonely"] {
        let only = scope(Some(name), None);
        assert_eq!(
            usage_summary_scoped(&conn, None, None, only).unwrap(),
            zero_summary,
            "{name}"
        );
        assert_eq!(
            usage_breakdown_scoped(&conn, "model", None, None, only).unwrap(),
            zero_breakdown,
            "{name}"
        );
        let out = evaluate_scoped(&conn, FROZEN_NOW, &rules, only).unwrap();
        assert_eq!(out["hits"], json!([]), "{name}");
        assert_eq!(out["abstained"], abstained, "{name}");
        // with an account too the answer is still empty
        let both = scope(Some(name), Some("Claude work"));
        assert_eq!(
            usage_summary_scoped(&conn, None, None, both).unwrap(),
            zero_summary
        );
    }
}

// ---- 3. the store is only read, and the tests are not constant ----

#[test]
fn the_invented_store_is_unchanged_and_gets_no_sidecars() {
    let before = fs::read(store()).unwrap();
    compare(&store(), &[Kind::Warnings, Kind::Plan]);
    let conn = rich();
    usage_summary_scoped(&conn, None, None, scope(Some("acme"), Some("2"))).unwrap();
    drop(conn);
    assert_eq!(fs::read(store()).unwrap(), before);
    for suffix in ["-journal", "-wal", "-shm"] {
        let mut name = store().into_os_string();
        name.push(suffix);
        assert!(!Path::new(&name).exists(), "sidecar {suffix} appeared");
    }
}

#[test]
fn moving_one_event_to_the_other_account_breaks_the_scoped_comparisons() {
    // the first priced message of account 2 now belongs to account 1
    let dir = TempDir::new();
    let path = changed_store(
        &dir,
        "UPDATE events SET agent_account_id = 1 WHERE id = \
         (SELECT MIN(id) FROM events WHERE agent_account_id = 2 AND input_tokens IS NOT NULL)",
    );
    let (ok, text) = run_usage_child(Some(&path));
    assert!(
        !ok && text.contains("scoped usage entries differ"),
        "{text}"
    );
    assert!(
        text.contains("account_work") && text.contains("account_default"),
        "{text}"
    );
}

#[test]
fn removing_a_workspace_repo_breaks_the_scoped_comparisons() {
    let dir = TempDir::new();
    let path = changed_store(
        &dir,
        "DELETE FROM workspace_repos WHERE repo_path = '/work/rich/kit/beta'",
    );
    let (ok, text) = run_usage_child(Some(&path));
    assert!(!ok && text.contains("workspace_personal"), "{text}");
    let (compared, wrong) = compare(&path, &[Kind::Warnings, Kind::Plan]);
    assert_eq!(compared, 22);
    assert!(
        wrong.iter().any(|w| w.contains("workspace_personal")),
        "{wrong:?}"
    );
}

#[test]
fn a_changed_account_tag_breaks_the_scoped_rules() {
    // every event of account 2 moves to account 1: the account entries of warnings and plan differ
    let dir = TempDir::new();
    let path = changed_store(
        &dir,
        "UPDATE events SET agent_account_id = 1 WHERE agent_account_id = 2",
    );
    let (compared, wrong) = compare(&path, &[Kind::Warnings, Kind::Plan]);
    assert_eq!(compared, 22);
    assert!(
        wrong.iter().any(|w| w.contains("account_work")),
        "{wrong:?}"
    );
}

// ---- 4. a store built for the kit rules and the owner project, with real git repos ----
//
// The rich store cannot show `context_index_stale` (it needs a git repo that exists on the machine), the easy-context
// "in use" check, or a gone path that belongs to a workspace through its owner project (FR-057). This store has two
// real git repos, `alpha` (branch `feat-x`) and `beta`, kernel traces and context indexes for both, a gone folder
// outside `alpha` whose branch `feat-x` makes `alpha` its owner, and the invented accounts 1 (alpha) and 2 (beta).
// The expected values are Python's: the same store (kept with `T017_KEEP=1`) was run through
//
//   TZ=UTC rust/parity/.venv/bin/python -c '
//   import json, sqlite3, sys
//   from datetime import UTC, datetime
//   from easy_personal.core import metrics, rules
//   conn = sqlite3.connect(sys.argv[1]); now = datetime(2026, 1, 20, tzinfo=UTC)
//   for ws, ac in [(None, None), ("acme", None), ("personal", None), (None, "Claude work"), ("acme", "Claude work")]:
//       out = rules.evaluate(conn, now, rules.load_rules(), ws, ac)
//       print(ws, ac, json.dumps({"hits": {h["rule_id"]: [h["measured"], h.get("detail")] for h in out["hits"]},
//             "abstained": [a["rule_id"] for a in out["abstained"]],
//             "sessions": metrics.usage_summary(conn, None, None, ws, ac)["sessions"]}, sort_keys=True))' <store>
//
// with `EASY_PERSONAL_HOME` set to an empty scratch folder.

fn git(dir: &Path, args: &[&str], date: Option<&str>) {
    let mut cmd = Command::new("git");
    cmd.args([
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "-c",
        "init.defaultBranch=main",
    ])
    .arg("-C")
    .arg(dir)
    .args(args)
    .env("GIT_CONFIG_GLOBAL", "/dev/null")
    .env("GIT_CONFIG_NOSYSTEM", "1");
    if let Some(date) = date {
        cmd.env("GIT_AUTHOR_DATE", date)
            .env("GIT_COMMITTER_DATE", date);
    }
    assert!(
        cmd.output().unwrap().status.success(),
        "git {args:?} failed"
    );
}

struct Kit {
    dir: TempDir,
    conn: Connection,
    alpha: String,
    beta: String,
    elsewhere: String,
    n: i64,
}

impl Kit {
    fn new() -> Kit {
        let dir = TempDir::new();
        let path = copy_store(&dir);
        let conn = Connection::open(&path).unwrap();
        for table in [
            "events",
            "kit_trace_events",
            "context_index",
            "workspace_repos",
        ] {
            conn.execute(&format!("DELETE FROM {table}"), []).unwrap();
        }
        let mut repos = Vec::new();
        for name in ["alpha", "beta"] {
            let repo = dir.0.join(name);
            fs::create_dir_all(&repo).unwrap();
            git(&repo, &["init", "-q"], None);
            git(
                &repo,
                &["commit", "-q", "--allow-empty", "-m", "c"],
                Some("2026-01-14T00:00:00Z"),
            );
            repos.push(repo.to_str().unwrap().to_string());
        }
        git(Path::new(&repos[0]), &["branch", "feat-x"], None);
        let mut kit = Kit {
            conn,
            alpha: repos[0].clone(),
            beta: repos[1].clone(),
            elsewhere: dir.0.join("elsewhere-gone").to_str().unwrap().to_string(),
            dir,
            n: 0,
        };
        let (alpha, beta) = (kit.alpha.clone(), kit.beta.clone());
        for (ws, repo) in [(2, alpha.as_str()), (3, beta.as_str())] {
            kit.conn
                .execute(
                    "INSERT INTO workspace_repos VALUES (?1, ?2)",
                    params![ws, repo],
                )
                .unwrap();
        }
        // history from 2025-12-01, then two windows of usage
        kit.ev("2025-12-01T00:00:00.000000Z", None, None, None, 1, None);
        let (elsewhere, alpha_s, beta_s) = (kit.elsewhere.clone(), alpha.clone(), beta.clone());
        kit.ev(
            "2026-01-10T10:00:00.000000Z",
            Some("sA0"),
            Some(&alpha_s),
            Some([100, 100, 0, 0]),
            1,
            None,
        );
        kit.ev(
            "2026-01-15T10:00:00.000000Z",
            Some("sA1"),
            Some(&alpha_s),
            Some([1000, 500, 0, 200]),
            1,
            None,
        );
        kit.ev(
            "2026-01-16T10:00:00.000000Z",
            Some("sA1"),
            Some(&alpha_s),
            Some([1000, 500, 0, 200]),
            1,
            Some("mcp__easy-context__search,Bash"),
        );
        kit.ev(
            "2026-01-17T10:00:00.000000Z",
            Some("sA2"),
            Some(&elsewhere),
            Some([800, 100, 0, 0]),
            1,
            None,
        );
        kit.ev(
            "2026-01-10T10:00:00.000000Z",
            Some("sB0"),
            Some(&beta_s),
            Some([100, 100, 0, 0]),
            2,
            None,
        );
        kit.ev(
            "2026-01-15T10:00:00.000000Z",
            Some("sB1"),
            Some(&beta_s),
            Some([3000, 900, 0, 100]),
            2,
            Some("Bash"),
        );
        kit.ev(
            "2026-01-16T10:00:00.000000Z",
            Some("sB1"),
            Some(&beta_s),
            Some([3000, 900, 0, 100]),
            2,
            None,
        );
        for (project, tool, errors) in
            [(&alpha, "Bash", vec![0, 1]), (&beta, "Read", vec![0, 0, 0])]
        {
            for e in errors {
                kit.trace(project, tool, e);
            }
        }
        for project in [&alpha, &beta] {
            kit.conn
                .execute(
                    "INSERT INTO context_index (project_path, last_index_at_utc) VALUES (?1, '2026-01-01T00:00:00.000000Z')",
                    params![project],
                )
                .unwrap();
        }
        kit
    }

    fn ev(
        &mut self,
        ts: &str,
        session: Option<&str>,
        project: Option<&str>,
        tokens: Option<[i64; 4]>,
        account: i64,
        tools: Option<&str>,
    ) {
        self.n += 1;
        let t = tokens;
        let branch = project.filter(|p| *p == self.elsewhere).map(|_| "feat-x");
        self.conn
            .execute(
                "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id,
                    project_path, is_sidechain, model, message_id, tool_names, input_tokens, output_tokens,
                    cache_creation_input_tokens, cache_read_input_tokens, entrypoint, git_branch, agent_account_id)
                 VALUES ('m', 'claude_code', 'p', 1, ?1, 0, ?2, ?3, ?4, 0, 'claude-sonnet-4-5', ?5, ?6, ?7, ?8, ?9, ?10, 'cli', ?11, ?12)",
                params![
                    self.n,
                    ts,
                    session,
                    project,
                    t.map(|_| format!("msg{}", self.n)),
                    tools,
                    t.map(|t| t[0]),
                    t.map(|t| t[1]),
                    t.map(|t| t[2]),
                    t.map(|t| t[3]),
                    branch,
                    account
                ],
            )
            .unwrap();
    }

    fn trace(&mut self, project: &str, tool: &str, is_error: i64) {
        self.n += 1;
        self.conn
            .execute(
                "INSERT INTO kit_trace_events (machine_id, source_path, inode, byte_offset, project_path, kit_task_id,
                    abstained, ts_utc, tool_name, is_error)
                 VALUES ('m', 'p', 1, ?1, ?2, 'T001', 0, '2026-01-15T10:00:00.000000Z', ?3, ?4)",
                params![self.n, project, tool, is_error],
            )
            .unwrap();
    }

    fn path(&self) -> PathBuf {
        self.dir.0.join("store.sqlite")
    }
}

/// `{hits: {rule: [measured, detail]}, abstained: [rule], sessions}` of one scope.
fn kit_view(conn: &Connection, workspace: Option<&str>, account: Option<&str>) -> Value {
    let out = evaluate_scoped(
        conn,
        FROZEN_NOW,
        &shipped_rules().unwrap(),
        scope(workspace, account),
    )
    .unwrap();
    let hits: serde_json::Map<String, Value> = out["hits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| {
            (
                h["rule_id"].as_str().unwrap().to_string(),
                json!([
                    h["measured"],
                    h.get("detail").cloned().unwrap_or(Value::Null)
                ]),
            )
        })
        .collect();
    let abstained: Vec<&Value> = out["abstained"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| &a["rule_id"])
        .collect();
    let sessions =
        usage_summary_scoped(conn, None, None, scope(workspace, account)).unwrap()["sessions"]
            .clone();
    json!({"hits": hits, "abstained": abstained, "sessions": sessions})
}

#[test]
fn a_scope_reaches_the_kit_rules_the_easy_context_check_and_the_owner_project() {
    let kit = Kit::new();
    if std::env::var_os("T017_KEEP").is_some() {
        // keep the store (and its git repos) so the command above can be run over it
        let keep = std::env::temp_dir().join("t017-kit-kept.sqlite");
        fs::copy(kit.path(), &keep).unwrap();
        std::mem::forget(kit.dir);
        return;
    }
    let conn = open_read_only(&kit.path()).unwrap();
    // acme = alpha: its events, its gone folder `elsewhere-gone` (owner project by branch `feat-x`), its traces and
    // its context index; personal = beta (account 2); the unscoped answer sees both.
    let cases = [
        (
            (None, None),
            json!({"abstained": [], "sessions": 5, "hits": {
                "cache_hit_low": [0.0638, null],
                "context_index_stale": [2, {"projects": ["alpha", "beta"]}],
                "project_dominates": [0.9268, {"project": "Scratch"}],
                "tokens_jump": [29.75, null],
                "tool_errors_high": [0.2, null]}}),
        ),
        (
            (Some("acme"), None),
            json!({"abstained": [], "sessions": 3, "hits": {
                "cache_hit_low": [0.125, null],
                "context_index_stale": [1, {"projects": ["alpha"]}],
                "project_dominates": [0.7907, {"project": "Scratch"}],
                "tokens_jump": [20.5, null],
                "tool_errors_high": [0.5, null]}}),
        ),
        // no easy-context call in this scope: `context_index_stale` is left out, neither a hit nor abstained
        (
            (Some("personal"), None),
            json!({"abstained": ["project_dominates"], "sessions": 2, "hits": {
                "cache_hit_low": [0.0323, null],
                "tokens_jump": [39.0, null]}}),
        ),
        (
            (None, Some("Claude work")),
            json!({"abstained": ["project_dominates"], "sessions": 2, "hits": {
                "cache_hit_low": [0.0323, null],
                "tokens_jump": [39.0, null]}}),
        ),
        // alpha's events are all account 1: nothing is left in the scope, every rule abstains
        (
            (Some("acme"), Some("Claude work")),
            json!({"sessions": 0, "hits": {}, "abstained": [
                "cache_hit_low", "project_dominates", "records_unreadable",
                "session_huge_no_compact", "tokens_jump", "tool_errors_high"]}),
        ),
    ];
    for ((workspace, account), want) in cases {
        assert_eq!(
            kit_view(&conn, workspace, account),
            want,
            "{workspace:?} {account:?}"
        );
    }
}
