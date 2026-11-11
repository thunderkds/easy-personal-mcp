//! T010: `plan::improvement_plan` against the four Python plan goldens: `api__plan__all.json` and
//! `mcp__improvement_plan__all.json` in `rust/parity/golden` (store `store-v11.sqlite`) and in `rust/parity/golden-rich`
//! (store `store-rich/store-v11.sqlite`). The clock is the manifests' `frozen_now`. Goldens are compared as parsed JSON
//! (keys sorted in the files, list order kept; `1000` and `1000.0` differ).
//! The `workspace` and `account` goldens are compared in `scope_parity.rs`. The plan reads no OS-local time (the seven rules use the
//! UTC windows only), so no `TZ` pin is needed. Variations are built on temp copies of the stores; the owner's real
//! `~/.easy-personal` is never opened.

use easy_personal_core::plan::improvement_plan;
use easy_personal_core::rules::{evaluate, shipped_rules};
use easy_personal_core::store::open_read_only;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// The keys Python's `plan.improvement_plan` copies from every hit.
const COPIED: [&str; 11] = [
    "rule_id",
    "label",
    "measured",
    "threshold",
    "fix",
    "title",
    "why",
    "steps",
    "summary",
    "causes",
    "diagnosis",
];
/// The keys it copies only when the hit has them.
const OPTIONAL: [&str; 2] = ["detail", "cache_read_share"];

const PLAN_FILES: [&str; 2] = ["api__plan__all.json", "mcp__improvement_plan__all.json"];

fn parity_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity")
}

fn default_store() -> PathBuf {
    parity_dir().join("store-v11.sqlite")
}

fn rich_store() -> PathBuf {
    parity_dir().join("store-rich").join("store-v11.sqlite")
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap_or_else(|e| panic!("{path:?}: {e}")))
        .unwrap()
}

/// `rich` picks `golden-rich`, else `golden`.
fn golden_dir(rich: bool) -> PathBuf {
    parity_dir().join(if rich { "golden-rich" } else { "golden" })
}

fn golden(rich: bool, file: &str) -> Value {
    read_json(&golden_dir(rich).join(file))
}

fn store(rich: bool) -> PathBuf {
    if rich { rich_store() } else { default_store() }
}

/// The frozen clock the manifest records, in microseconds, after checking that the manifest names the plan entry with
/// no parameters (so the goldens are the unscoped plan) and that the plan goldens exist.
fn frozen_now(rich: bool) -> i64 {
    let manifest = read_json(&golden_dir(rich).join("manifest.json"));
    for file in PLAN_FILES {
        let entry = manifest["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["file"] == json!(file))
            .unwrap_or_else(|| panic!("the manifest has no entry {file}"));
        assert_eq!(entry["params"], json!({}), "{file} has parameters");
        assert!(golden_dir(rich).join(file).is_file(), "{file} is missing");
    }
    let now: jiff::Timestamp = manifest["frozen_now"].as_str().unwrap().parse().unwrap();
    now.as_microsecond()
}

fn plan_over(path: &Path, rich: bool) -> Value {
    let conn = open_read_only(path).unwrap();
    improvement_plan(&conn, frozen_now(rich), &shipped_rules().unwrap()).unwrap()
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t010-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    /// A writable copy of a store (the only way a test changes a value).
    fn copy_of(&self, from: &Path) -> PathBuf {
        let path = self.0.join("store.sqlite");
        fs::copy(from, &path).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// ---- 1. the four goldens (AC1) ----

#[test]
fn plan_matches_the_default_api_golden() {
    assert_eq!(
        plan_over(&default_store(), false),
        golden(false, "api__plan__all.json")
    );
}

#[test]
fn plan_matches_the_default_mcp_golden() {
    assert_eq!(
        plan_over(&default_store(), false),
        golden(false, "mcp__improvement_plan__all.json")
    );
}

#[test]
fn plan_matches_the_rich_api_golden() {
    assert_eq!(
        plan_over(&rich_store(), true),
        golden(true, "api__plan__all.json")
    );
}

#[test]
fn plan_matches_the_rich_mcp_golden() {
    assert_eq!(
        plan_over(&rich_store(), true),
        golden(true, "mcp__improvement_plan__all.json")
    );
}

// ---- 2. groups, order and keys (AC2) ----

fn hits_of(rich: bool) -> Vec<Value> {
    let conn = open_read_only(&store(rich)).unwrap();
    let warnings = evaluate(&conn, frozen_now(rich), &shipped_rules().unwrap()).unwrap();
    warnings["hits"].as_array().unwrap().clone()
}

fn keys_of(value: &Value) -> Vec<String> {
    let mut keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    keys
}

#[test]
fn groups_ascend_by_priority_and_items_keep_the_evaluate_order() {
    let plan = plan_over(&rich_store(), true);
    let groups = plan["groups"].as_array().unwrap();
    let priorities: Vec<i64> = groups
        .iter()
        .map(|g| g["priority"].as_i64().unwrap())
        .collect();
    assert_eq!(
        priorities,
        [1, 2, 3],
        "ascending and one group per priority"
    );
    let hits = hits_of(true);
    for group in groups {
        let priority = group["priority"].as_i64().unwrap();
        // Python: `groups.setdefault(priority, []).append(item)` while walking the hits in order
        let want: Vec<&str> = hits
            .iter()
            .filter(|h| h["priority"].as_i64() == Some(priority))
            .map(|h| h["rule_id"].as_str().unwrap())
            .collect();
        let got: Vec<&str> = group["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["rule_id"].as_str().unwrap())
            .collect();
        assert_eq!(got, want, "items of priority {priority}");
        assert_eq!(keys_of(group), ["items", "priority"]);
    }
    // two rules of one priority share one group (edge case), in evaluate order
    assert_eq!(
        groups[1]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["rule_id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "records_unreadable",
            "session_huge_no_compact",
            "tool_errors_high"
        ]
    );
}

#[test]
fn each_item_has_the_copied_keys_and_detail_or_cache_read_share_only_when_the_hit_has_them() {
    for rich in [false, true] {
        let plan = plan_over(&store(rich), rich);
        let hits = hits_of(rich);
        let mut seen_extra = [false, false];
        let mut seen_neither = false;
        let mut count = 0;
        for item in plan["groups"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|g| g["items"].as_array().unwrap())
        {
            count += 1;
            let hit = hits
                .iter()
                .find(|h| h["rule_id"] == item["rule_id"])
                .unwrap();
            let mut want: Vec<String> = COPIED.iter().map(|k| k.to_string()).collect();
            for (n, key) in OPTIONAL.iter().enumerate() {
                if hit.get(key).is_some() {
                    want.push(key.to_string());
                    seen_extra[n] = true;
                }
            }
            want.sort();
            assert_eq!(keys_of(item), want, "{}", item["rule_id"]);
            for key in COPIED.iter().chain(OPTIONAL.iter()) {
                assert_eq!(item.get(key), hit.get(key), "{} {key}", item["rule_id"]);
            }
            for dropped in ["priority", "window_days"] {
                assert!(item.get(dropped).is_none(), "{dropped} leaked into an item");
            }
            seen_neither |= OPTIONAL.iter().all(|k| item.get(k).is_none());
        }
        assert_eq!(count, hits.len(), "every hit is in the plan once");
        if rich {
            assert!(
                seen_extra[0] && seen_extra[1],
                "rich plan lacks an extra key"
            );
            assert!(seen_neither, "no rich item shows a rule with neither key");
            let neither = plan["groups"][1]["items"][2].clone();
            assert_eq!(neither["rule_id"], json!("tool_errors_high"));
            assert!(neither.get("detail").is_none() && neither.get("cache_read_share").is_none());
        }
    }
}

// ---- 3. no fired rule (AC3) ----

/// Python gave the same: with `rules.evaluate` over this very temp store (`UPDATE events SET abstained = 0`),
/// `plan.improvement_plan(conn, now=datetime(2026, 1, 20, tzinfo=UTC), rules=rules.load_rules())` is `{"groups": []}`
/// (run with `rust/parity/.venv/bin/python`; the answer is recorded in the T010 report).
#[test]
fn no_fired_rule_gives_an_empty_groups_list() {
    let dir = TempDir::new();
    let path = dir.copy_of(&default_store());
    // the only hit of the fixture is `records_unreadable` (1 of 4 records of 2.1.302): make every record readable
    Connection::open(&path)
        .unwrap()
        .execute("UPDATE events SET abstained = 0 WHERE abstained = 1", [])
        .unwrap();
    let conn = open_read_only(&path).unwrap();
    let rules = shipped_rules().unwrap();
    let warnings = evaluate(&conn, frozen_now(false), &rules).unwrap();
    assert_eq!(warnings["hits"], json!([]), "the copy still has a hit");
    assert_eq!(
        improvement_plan(&conn, frozen_now(false), &rules).unwrap(),
        json!({"groups": []})
    );
}

// ---- 4. the plan's `cache_read_share` is not the warning's `measured` (AC4) ----

/// `rules._explain` (rules.py): for a rule in `TOKEN_METRICS` it adds `cache_read_share = cache_read / tokens`, where
/// `tokens` is input + output + cache creation + cache read of the window. The `cache_hit_low` rule's `measured` is
/// `_reuse(cache_read, prompt)` = `cache_read / prompt` with `prompt` = input + cache creation + cache read (no
/// output). Same numerator, a larger denominator, so the share is lower: 0.1895 against 0.1929 in the rich store.
/// The plan only copies `cache_read_share` from the hit, so the port equals the golden by construction; this test
/// recomputes both numbers from the store with SQL, independent of `rules.rs`.
#[test]
fn plan_cache_read_share_differs_from_the_warning_measured_for_a_stated_reason() {
    let plan = plan_over(&rich_store(), true);
    let item = plan["groups"][0]["items"][0].clone();
    assert_eq!(item["rule_id"], json!("cache_hit_low"));
    assert_eq!(item["measured"], json!(0.1929), "cache_read / prompt");
    assert_eq!(
        item["cache_read_share"],
        json!(0.1895),
        "cache_read / tokens"
    );
    assert_eq!(
        item,
        golden(true, "api__plan__all.json")["groups"][0]["items"][0]
    );

    // the same window the rule reads: [frozen_now - 7 days, frozen_now), one row per message id
    let conn =
        Connection::open_with_flags(rich_store(), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (cache_read, tokens, prompt): (i64, i64, i64) = conn
        .query_row(
            "SELECT SUM(cr), SUM(i + o + cc + cr), SUM(i + cc + cr) FROM (
               SELECT MIN(ts_utc) AS ts, COALESCE(MAX(input_tokens), 0) AS i, COALESCE(MAX(output_tokens), 0) AS o,
                      COALESCE(MAX(cache_creation_input_tokens), 0) AS cc,
                      COALESCE(MAX(cache_read_input_tokens), 0) AS cr
               FROM events WHERE abstained = 0 AND message_id IS NOT NULL AND input_tokens IS NOT NULL
               AND ts_utc >= '2026-01-12T00:00:00.000000Z' AND ts_utc < '2026-01-21T00:00:00.000000Z'
               GROUP BY message_id
             ) WHERE ts >= '2026-01-13T00:00:00.000000Z' AND ts < '2026-01-20T00:00:00.000000Z'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert!(
        tokens > prompt && prompt > cache_read,
        "{cache_read} {prompt} {tokens}"
    );
    let round4 = |x: f64| format!("{x:.4}").parse::<f64>().unwrap();
    assert_eq!(round4(cache_read as f64 / prompt as f64), 0.1929);
    assert_eq!(round4(cache_read as f64 / tokens as f64), 0.1895);

    // every rule that counts tokens shows the same window share, whatever its own `measured` is
    for rule in [
        "session_huge_no_compact",
        "project_dominates",
        "tokens_jump",
    ] {
        let found = plan["groups"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|g| g["items"].as_array().unwrap())
            .find(|i| i["rule_id"] == json!(rule))
            .unwrap();
        assert_eq!(found["cache_read_share"], json!(0.1895), "{rule}");
    }
}

// ---- 5. the stores are read-only (AC5) ----

#[test]
fn the_stores_are_unchanged_and_get_no_sidecars() {
    for path in [default_store(), rich_store()] {
        let before = fs::read(&path).unwrap();
        let _ = plan_over(&path, path == rich_store());
        assert_eq!(fs::read(&path).unwrap(), before);
        for suffix in ["-journal", "-wal", "-shm"] {
            let mut name = path.clone().into_os_string();
            name.push(suffix);
            assert!(!Path::new(&name).exists(), "sidecar {suffix} appeared");
        }
    }
}

// ---- 6. the tests are not constant (mutation check) ----

#[test]
fn changing_one_fixture_value_breaks_the_golden_comparison() {
    // default store: the one record of 2.1.302 that cannot be read becomes readable, so `records_unreadable` moves
    let dir = TempDir::new();
    let path = dir.copy_of(&default_store());
    let changed = Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE events SET abstained = 0 WHERE version = '2.1.302' AND abstained = 1",
            [],
        )
        .unwrap();
    assert_eq!(changed, 1);
    let plan = plan_over(&path, false);
    for file in PLAN_FILES {
        assert_ne!(plan, golden(false, file), "{file}");
    }

    // rich store: every unreadable record becomes readable
    let dir = TempDir::new();
    let path = dir.copy_of(&rich_store());
    Connection::open(&path)
        .unwrap()
        .execute("UPDATE events SET abstained = 0 WHERE abstained = 1", [])
        .unwrap();
    let plan = plan_over(&path, true);
    for file in PLAN_FILES {
        assert_ne!(plan, golden(true, file), "{file}");
    }
}
