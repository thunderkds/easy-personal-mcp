//! T040: `reports::list_reports`, `reports::get_report` and `first_run::steps` against the Python goldens
//! `api__reports__all`, `api__reports_one__{found,missing,scoped}` and `api__first_run__all` of `rust/parity/golden-rich`
//! (over `store-rich/store-v11.sqlite`) and the ones `rust/parity/golden` holds (over `store-v11.sqlite`), under the
//! manifests' `frozen_now`, plus the edge cases of the read side (a saved scope that is gone, a snapshot older than FR-079,
//! a rule left out, a snapshot Python cannot read).
//!
//! The entries come from the manifests, so a missing or extra golden fails the count. Compared as parsed JSON: the files
//! hold sorted keys, `1` and `1.0` differ. Every store is a temp copy; the owner's real `~/.easy-personal` is never opened.
//! The first-run timer check reads a temp unit folder the test creates, never the real `~/.config`. The rich store's
//! confirmed accounts name `/work/rich/...` folders that do not exist here (Python's golden run had them), so the copy's
//! account rows are pointed at temp folders; a second test proves a missing folder is not "confirmed".

use easy_personal_core::first_run::{steps, unit_dir_from};
use easy_personal_core::reports::{ReportsError, get_report, list_reports};
use easy_personal_core::store::open_read_only;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// `frozen_now` of both manifests (`2026-01-20T00:00:00+00:00`), in microseconds since the epoch.
const FROZEN_NOW: i64 = 1_768_867_200_000_000;

fn parity_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity")
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap_or_else(|e| panic!("{path:?}: {e}")))
        .unwrap()
}

/// A temp folder removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t040-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    /// A copy of the parity store `fixture`, with `sql` run on it first.
    fn store(&self, fixture: &str, sql: &str) -> PathBuf {
        let path = self.0.join("store.sqlite");
        fs::copy(parity_dir().join(fixture), &path).unwrap();
        if !sql.is_empty() {
            Connection::open(&path).unwrap().execute_batch(sql).unwrap();
        }
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// `(manifest folder, store file)` of the two oracles.
const ORACLES: [(&str, &str); 2] = [
    ("golden", "store-v11.sqlite"),
    ("golden-rich", "store-rich/store-v11.sqlite"),
];

/// The id each `reports_one` golden asked for (`gen_golden.py`: `ROUTES` for the plain store, the rich list for the other).
fn report_id(dir: &str, file: &str) -> i64 {
    match (dir, file) {
        (_, "api__reports_one__found.json") => 1,
        (_, "api__reports_one__scoped.json") => 2,
        ("golden", "api__reports_one__missing.json") => 1,
        ("golden-rich", "api__reports_one__missing.json") => 99,
        other => panic!("{other:?}"),
    }
}

/// `(golden file, status)` of the slice's entries in one manifest.
fn slice_entries(dir: &str) -> Vec<(String, u16)> {
    let manifest = read_json(&parity_dir().join(dir).join("manifest.json"));
    manifest["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| {
            let file = e["file"].as_str().unwrap();
            ["api__reports__", "api__reports_one__", "api__first_run__"]
                .iter()
                .any(|p| file.starts_with(p))
        })
        .map(|e| {
            (
                e["file"].as_str().unwrap().to_string(),
                e["status"].as_u64().unwrap() as u16,
            )
        })
        .collect()
}

/// Points every account row of the copy at an existing temp folder (Python's golden run had its `agent-root` folders).
fn point_accounts_at(dir: &TempDir, path: &Path) {
    let conn = Connection::open(path).unwrap();
    let ids: Vec<i64> = conn
        .prepare("SELECT id FROM agent_accounts")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    for id in ids {
        let folder = dir.0.join(format!("account-{id}"));
        fs::create_dir_all(&folder).unwrap();
        conn.execute(
            "UPDATE agent_accounts SET folder = ? WHERE id = ?",
            params![folder.to_str().unwrap(), id],
        )
        .unwrap();
    }
}

/// The answer the route gives: `(status, body)`; an unknown id is Python's 404 `no such report`.
fn answer(conn: &Connection, dir: &str, file: &str, units: &Path) -> (u16, Value) {
    if file.starts_with("api__reports__") {
        return (200, list_reports(conn).unwrap());
    }
    if file.starts_with("api__first_run__") {
        return (200, steps(conn, Some(units)).unwrap());
    }
    match get_report(conn, report_id(dir, file), FROZEN_NOW).unwrap() {
        Some(body) => (200, body),
        None => (404, json!({"error": "no such report"})),
    }
}

#[test]
fn every_golden_of_the_slice_equals_the_core_answer() {
    let (mut equal, mut total) = (0, 0);
    for (dir, fixture) in ORACLES {
        let entries = slice_entries(dir);
        let want: usize = if dir == "golden" { 3 } else { 5 };
        assert_eq!(entries.len(), want, "{dir}: {entries:?}");
        let temp = TempDir::new();
        let path = temp.store(fixture, "");
        if dir == "golden-rich" {
            point_accounts_at(&temp, &path);
        }
        let units = temp.0.join("xdg/systemd/user"); // no timer in it
        fs::create_dir_all(&units).unwrap();
        let conn = open_read_only(&path).unwrap();
        for (file, status) in entries {
            total += 1;
            let golden = read_json(&parity_dir().join(dir).join(&file));
            let (got_status, got) = answer(&conn, dir, &file, &units);
            assert_eq!(got_status, status, "{dir}/{file}");
            assert_eq!(got, golden, "{dir}/{file}");
            equal += 1;
        }
    }
    assert_eq!((equal, total), (8, 8));
    println!("goldens equal / total: {equal} / {total}");
}

#[test]
fn the_timer_step_follows_the_unit_file_and_the_two_variables() {
    let temp = TempDir::new();
    let path = temp.store("store-v11.sqlite", "");
    let conn = open_read_only(&path).unwrap();
    let done = |units: Option<&Path>| steps(&conn, units).unwrap()["steps"][1]["done"].clone();
    let base = temp.0.join("xdg");
    let units = unit_dir_from(Some(base.clone().into_os_string()), None).unwrap();
    assert_eq!(units, base.join("systemd").join("user"));
    assert_eq!(done(Some(&units)), false);
    fs::create_dir_all(&units).unwrap();
    // the service file alone is not the timer
    fs::write(units.join("easy-personal-ingest.service"), "").unwrap();
    assert_eq!(done(Some(&units)), false);
    fs::write(units.join("easy-personal-ingest.timer"), "").unwrap();
    assert_eq!(done(Some(&units)), true);
    assert_eq!(done(None), false);
    // the hint does not change with the timer (Python: the install command on a machine with systemd)
    assert_eq!(
        steps(&conn, Some(&units)).unwrap()["steps"][1]["hint"],
        "easy-personal schedule install"
    );

    let os = |s: &str| Some(OsString::from(s));
    let tail = Path::new("systemd").join("user");
    assert_eq!(
        unit_dir_from(os("/x"), os("/h")),
        Some(Path::new("/x").join(&tail))
    );
    // an empty XDG_CONFIG_HOME is unset (Python's `or`)
    assert_eq!(
        unit_dir_from(os(""), os("/h")),
        Some(Path::new("/h/.config").join(&tail))
    );
    assert_eq!(
        unit_dir_from(None, os("/h")),
        Some(Path::new("/h/.config").join(&tail))
    );
    assert_eq!(
        unit_dir_from(os("./rel"), None),
        Some(Path::new("rel").join(&tail))
    );
    assert_eq!(unit_dir_from(None, None), None);
    assert_eq!(unit_dir_from(None, os("")), None);
}

#[test]
fn the_account_step_needs_a_confirmed_account_whose_folder_exists() {
    let temp = TempDir::new();
    let path = temp.store("store-rich/store-v11.sqlite", "");
    let units = temp.0.join("none");
    let accounts = |path: &Path| {
        let conn = open_read_only(path).unwrap();
        steps(&conn, Some(&units)).unwrap()["steps"][2]["done"].clone()
    };
    // `/work/rich/agent-root/...` is not on this machine: Python reports such an account as `missing`
    assert_eq!(accounts(&path), false);
    point_accounts_at(&temp, &path);
    assert_eq!(accounts(&path), true);
    // detected and ignored accounts never count, whatever their folder
    Connection::open(&path)
        .unwrap()
        .execute_batch(
            "UPDATE agent_accounts SET status = 'detected' WHERE id = 1; \
             UPDATE agent_accounts SET status = 'ignored' WHERE id = 2",
        )
        .unwrap();
    assert_eq!(accounts(&path), false);
}

#[test]
fn the_workspace_and_ingest_steps_ignore_the_orphan_and_need_an_event() {
    let temp = TempDir::new();
    let units = temp.0.join("none");
    let path = temp.store(
        "store-rich/store-v11.sqlite",
        "DROP TRIGGER workspaces_orphan_no_delete; DELETE FROM workspace_repos; \
         DELETE FROM account_workspaces; DELETE FROM workspaces WHERE is_orphan = 0",
    );
    let conn = open_read_only(&path).unwrap();
    assert_eq!(
        steps(&conn, Some(&units)).unwrap()["steps"][3]["done"],
        false
    );
    drop(conn);
    let path = temp.store("store-rich/store-v11.sqlite", "DELETE FROM events");
    let conn = open_read_only(&path).unwrap();
    let found = steps(&conn, Some(&units)).unwrap();
    assert_eq!(found["steps"][0]["done"], false);
    assert_eq!(found["steps"][3]["done"], true);
}

fn rich_report(temp: &TempDir, sql: &str) -> Connection {
    open_read_only(&temp.store("store-rich/store-v11.sqlite", sql)).unwrap()
}

#[test]
fn a_saved_scope_that_is_gone_says_not_measured_now_for_every_rule() {
    let temp = TempDir::new();
    let conn = rich_report(
        &temp,
        "DROP TRIGGER workspaces_orphan_no_delete; DELETE FROM workspace_repos; \
         DELETE FROM account_workspaces; DELETE FROM workspaces WHERE name = 'acme'",
    );
    let report = get_report(&conn, 2, FROZEN_NOW).unwrap().unwrap();
    assert_eq!(report["now_as_of_utc"], Value::Null);
    let rules = report["rules"].as_array().unwrap();
    assert!(!rules.is_empty());
    for rule in rules {
        assert_eq!(
            rule["now"],
            json!({"state": "not_measured", "reason": "not measured now"}),
            "{rule}"
        );
        assert_ne!(rule["before"], Value::Null);
    }
    // the unscoped report is not affected
    let report = get_report(&conn, 1, FROZEN_NOW).unwrap().unwrap();
    assert_ne!(report["now_as_of_utc"], Value::Null);
}

#[test]
fn a_clock_a_year_later_changes_only_the_now_side() {
    let temp = TempDir::new();
    let conn = rich_report(&temp, "");
    let year = 366 * 86_400 * 1_000_000;
    let now = get_report(&conn, 1, FROZEN_NOW).unwrap().unwrap();
    let later = get_report(&conn, 1, FROZEN_NOW + year).unwrap().unwrap();
    assert_ne!(now["rules"], later["rules"]);
    assert_eq!(now["summary"], later["summary"]);
    for (a, b) in now["rules"]
        .as_array()
        .unwrap()
        .iter()
        .zip(later["rules"].as_array().unwrap())
    {
        assert_eq!(a["before"], b["before"]);
    }
    assert_eq!(later["now_as_of_utc"], "2027-01-21T00:00:00+00:00");
}

#[test]
fn a_snapshot_older_than_fr_079_gets_its_label_filled_from_the_rule_pack() {
    let temp = TempDir::new();
    let conn = rich_report(&temp, "");
    let before = get_report(&conn, 1, FROZEN_NOW).unwrap().unwrap();
    let cache = |report: &Value| {
        report["rules"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["rule_id"] == "cache_hit_low")
            .unwrap()["before"]
            .clone()
    };
    assert_eq!(cache(&before)["state"], "fired");
    drop(conn);
    // the entry as an older version saved it: no label, priority, why, fix, window_days
    let temp = TempDir::new();
    let path = temp.store("store-rich/store-v11.sqlite", "");
    let c = Connection::open(&path).unwrap();
    let snapshot: String = c
        .query_row(
            "SELECT snapshot_json FROM investigate_reports WHERE id = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let mut snapshot: Value = serde_json::from_str(&snapshot).unwrap();
    for entry in snapshot["fired"].as_array_mut().unwrap() {
        for key in ["label", "priority", "why", "fix", "window_days"] {
            entry.as_object_mut().unwrap().remove(key);
        }
    }
    c.execute(
        "UPDATE investigate_reports SET snapshot_json = ? WHERE id = 1",
        params![snapshot.to_string()],
    )
    .unwrap();
    drop(c);
    let after = get_report(&open_read_only(&path).unwrap(), 1, FROZEN_NOW)
        .unwrap()
        .unwrap();
    // label, priority, why and window_days are the pack's; `fix` is the pack's steps joined (Python's `Rule.fix`, without the
    // cause line the saved one had); `steps` stay as saved
    let mut want = cache(&before);
    want["fix"] = json!(
        "Keep the start of your prompts the same from one turn to the next. Avoid editing CLAUDE.md or switching models in the middle of a session."
    );
    assert_eq!(cache(&after), want);
}

#[test]
fn a_rule_left_out_is_not_in_use_now_and_hidden_when_it_was_abstained() {
    // `context_index_stale` is left out in the rich store's `personal` scope (no easy-context call there), see rule_coverage
    let temp = TempDir::new();
    let path = temp.store("store-rich/store-v11.sqlite", "");
    let c = Connection::open(&path).unwrap();
    c.execute(
        "UPDATE investigate_reports SET workspace = 'personal' WHERE id = 2",
        [],
    )
    .unwrap();
    drop(c);
    let conn = open_read_only(&path).unwrap();
    let report = get_report(&conn, 2, FROZEN_NOW).unwrap().unwrap();
    let rule = report["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule_id"] == "context_index_stale")
        .unwrap();
    assert_eq!(
        rule["now"],
        json!({"state": "not_in_use", "reason": "not in use"})
    );
    assert_eq!(rule["before"]["state"], "clear");
    // saved as abstained in that scope: not shown at all
    let c = Connection::open(&path).unwrap();
    let snapshot: String = c
        .query_row(
            "SELECT snapshot_json FROM investigate_reports WHERE id = 2",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let mut snapshot: Value = serde_json::from_str(&snapshot).unwrap();
    let checked = snapshot["checked"].as_array_mut().unwrap();
    checked.retain(|r| r["rule_id"] != "context_index_stale");
    snapshot["abstained"]
        .as_array_mut()
        .unwrap()
        .push(json!({"rule_id": "context_index_stale", "title": "Code index is out of date", "reason": "no data"}));
    c.execute(
        "UPDATE investigate_reports SET snapshot_json = ? WHERE id = 2",
        params![snapshot.to_string()],
    )
    .unwrap();
    drop(c);
    let report = get_report(&open_read_only(&path).unwrap(), 2, FROZEN_NOW)
        .unwrap()
        .unwrap();
    assert!(
        report["rules"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["rule_id"] != "context_index_stale")
    );
}

#[test]
fn a_snapshot_python_cannot_read_is_refused_for_one_report_and_counted_as_zero_in_the_list() {
    let temp = TempDir::new();
    let path = temp.store("store-rich/store-v11.sqlite", "");
    let c = Connection::open(&path).unwrap();
    for (id, bad) in [(1, "not json"), (2, r#"{"as_of_utc": "x"}"#)] {
        c.execute(
            "UPDATE investigate_reports SET snapshot_json = ? WHERE id = ?",
            params![bad, id],
        )
        .unwrap();
    }
    c.execute("INSERT INTO investigate_reports (saved_utc, server_version, summary, fixes_json, snapshot_json) VALUES ('2026-01-21T00:00:00Z', '0', 'third', 'oops', '{}')", []).unwrap();
    drop(c);
    let conn = open_read_only(&path).unwrap();
    for id in [1, 2, 3] {
        assert!(
            matches!(
                get_report(&conn, id, FROZEN_NOW),
                Err(ReportsError::Corrupt)
            ),
            "{id}"
        );
    }
    let list = list_reports(&conn).unwrap();
    let rows = list["reports"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().all(|r| r["fired_count"] == 0));
    assert_eq!(rows[0]["summary_first_line"], "third");
    assert_eq!(
        ReportsError::Corrupt.to_string(),
        "the saved report cannot be read"
    );
}

#[test]
fn the_list_shows_the_newest_fifty_with_the_first_line_of_the_summary() {
    let temp = TempDir::new();
    let path = temp.store("store-v11.sqlite", "");
    let c = Connection::open(&path).unwrap();
    for n in 1..=55 {
        c.execute(
            "INSERT INTO investigate_reports (saved_utc, server_version, summary, fixes_json, snapshot_json) VALUES (?, '0', ?, '[]', ?)",
            params![
                format!("2026-01-20T00:00:{:02}Z", n % 60),
                format!("line {n}\u{2028}hidden second line"),
                r#"{"checked": [{"fired": true}, {"fired": false}, {"fired": true}]}"#
            ],
        )
        .unwrap();
    }
    drop(c);
    let conn = open_read_only(&path).unwrap();
    let list = list_reports(&conn).unwrap();
    let rows = list["reports"].as_array().unwrap();
    assert_eq!(rows.len(), 50);
    assert_eq!(rows[0]["id"], 55);
    assert_eq!(rows[49]["id"], 6);
    assert_eq!(rows[0]["summary_first_line"], "line 55");
    assert_eq!(rows[0]["fired_count"], 2);
    assert_eq!(get_report(&conn, 56, FROZEN_NOW).unwrap(), None);
}
