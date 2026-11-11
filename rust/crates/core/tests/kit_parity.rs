//! T042: `kit::kit_summary`, `kit::workspace_rollup` and `kit::report_folders` against Python.
//!
//! 1. The 17 goldens of `rust/parity/golden-rich`: `api__kit__*` (7), `api__kit_workspaces__*` (3), `api__kit_reports__all`
//!    and `mcp__kit_summary__*` (6), read from the manifest, replayed at its `frozen_now`.
//! 2. A store and folders built here (a copy of the rich store plus `kit_extra/extra.sql`, four git repos, report
//!    folders) whose Python answers are `kit_extra/expected.json` (made by `kit_extra/make_expected.py` under `TZ=UTC`).
//!    It holds what the rich goldens cannot: scores, easy-ui checks and calls, a stale index, git history, a project
//!    with only abstained trace rows, report folders (symlink, depth, cap of 8, sub-second times).
//! 3. Errors as Python's, a read that changes nothing, no path or file name in an answer.
//!
//! The rich goldens were made over a work folder that held `kit/alpha` and `kit/beta`; the saved store says
//! `/work/rich/kit/...`. Here every text column of a temp copy is rewritten from `/work/rich/kit` to the temp work
//! folder, where the fixture folders are copied (`gen_golden.py`, `rich_work_tree`). The gone folders stay `/work/rich/...`.
//! The temp folder must sort before `/work/rich` (as the generator's did), because `repo_no` and the project order follow
//! the byte order of the stored paths. The owner's real `~/.easy-personal` is never opened.

use easy_personal_core::kit::{kit_summary, report_folders, workspace_rollup};
use easy_personal_core::metrics::MetricsError;
use easy_personal_core::scope::Scope;
use easy_personal_core::store::open_read_only;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, UNIX_EPOCH};

/// 2026-01-20T00:00:00Z in microseconds: the manifest's `frozen_now`.
const FROZEN_NOW: i64 = 1_768_867_200_000_000;
const RICH_MTIME: u64 = 1_768_694_400;

fn parity_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity")
}

fn extra_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/kit_extra")
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap_or_else(|e| panic!("{path:?}: {e}")))
        .unwrap()
}

fn set_mtime(path: &Path, secs: u64, nanos: u32) {
    fs::File::open(path)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::new(secs, nanos))
        .unwrap();
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

fn touch(path: &Path, secs: u64, nanos: u32) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "x").unwrap();
    set_mtime(path, secs, nanos);
}

/// `git` with an empty environment: no config of the owner's, fixed dates.
fn git(cwd: &Path, args: &[&str], date: Option<&str>) {
    let mut command = Command::new("git");
    command
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .env("HOME", cwd)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.invalid",
            "-C",
        ])
        .arg(cwd)
        .args(args);
    if let Some(date) = date {
        command
            .env("GIT_AUTHOR_DATE", date)
            .env("GIT_COMMITTER_DATE", date);
    }
    let out = command.output().unwrap();
    assert!(out.status.success(), "git {args:?} failed");
}

/// The commit times (UTC) of each test repo; `make_expected.py` has the same table.
const COMMITS: [(&str, &[&str]); 3] = [
    (
        "repo-a",
        &[
            "2026-01-05T10:00:00+0000",
            "2026-01-05T23:30:00+0000",
            "2026-01-12T01:00:00+0000",
        ],
    ),
    ("repo-b", &["2026-01-03T12:00:00+0000"]),
    ("repo-c", &[]),
];

/// A temp work folder with the store and folders of one scenario; removed on drop.
struct Work {
    dir: PathBuf,
    store: PathBuf,
}

impl Work {
    /// `extra`: also `extra.sql`, the repos and the report folders. `sql` runs last (a mutation for a red control).
    fn new(extra: bool, sql: Option<&str>) -> Work {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t042-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        assert!(
            dir.to_str().unwrap() < "/work/rich",
            "the temp folder must sort before /work/rich (the generator's work folder did): set TMPDIR to a folder that does"
        );
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let kit = dir.join("kit");
        copy_tree(&parity_dir().join("fixtures-rich/kit"), &kit);
        let mut stack = vec![kit.clone()];
        while let Some(folder) = stack.pop() {
            for entry in fs::read_dir(folder).unwrap() {
                let path = entry.unwrap().path();
                set_mtime(&path, RICH_MTIME, 0);
                if path.is_dir() {
                    stack.push(path);
                }
            }
        }
        let store = dir.join("store.sqlite");
        fs::copy(parity_dir().join("store-rich/store-v11.sqlite"), &store).unwrap();
        let conn = Connection::open(&store).unwrap();
        let tables: Vec<String> = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
            )
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        for table in tables {
            let columns: Vec<String> = conn
                .prepare(&format!("PRAGMA table_info(\"{table}\")"))
                .unwrap()
                .query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, String>(2)?)))
                .unwrap()
                .map(Result::unwrap)
                .filter(|(_, kind)| kind.eq_ignore_ascii_case("TEXT"))
                .map(|(name, _)| name)
                .collect();
            for column in columns {
                conn.execute(
                    &format!(
                        "UPDATE \"{table}\" SET \"{column}\" = replace(\"{column}\", ?1, ?2) WHERE instr(\"{column}\", ?1) > 0"
                    ),
                    [
                        "/work/rich/kit",
                        &format!("{}/kit", dir.to_str().unwrap()),
                    ],
                )
                .unwrap();
            }
        }
        if extra {
            for (name, dates) in COMMITS {
                fs::create_dir(dir.join(name)).unwrap();
                git(&dir.join(name), &["init", "-q"], None);
                for date in dates {
                    git(
                        &dir.join(name),
                        &["commit", "-q", "--allow-empty", "-m", "c"],
                        Some(date),
                    );
                }
            }
            fs::create_dir(dir.join("plain")).unwrap();
            let a = dir.join("repo-a");
            touch(
                &a.join("test-reports/session-001.json"),
                RICH_MTIME,
                999_999_700,
            );
            touch(&a.join("test-reports/notes.txt"), RICH_MTIME, 0);
            touch(
                &a.join("test-reports/sub/deep/x.json"),
                RICH_MTIME - 86_400,
                0,
            );
            touch(
                &a.join("test-reports/sub/deep/y/too-deep.json"),
                RICH_MTIME,
                0,
            );
            touch(&a.join("Reports-Archive/a.json"), RICH_MTIME - 5, 0);
            touch(&a.join("reports.txt"), RICH_MTIME, 0);
            std::os::unix::fs::symlink("test-reports", a.join("ref-reports")).unwrap();
            fs::create_dir(dir.join("repo-b/reports")).unwrap();
            for n in 1..=8 {
                fs::create_dir(dir.join(format!("repo-b/report-{n}"))).unwrap();
            }
            let sql = fs::read_to_string(extra_dir().join("extra.sql"))
                .unwrap()
                .replace("@T@", dir.to_str().unwrap());
            conn.execute_batch(&sql).unwrap();
        }
        if let Some(sql) = sql {
            let changed = conn
                .execute(&sql.replace("@T@", dir.to_str().unwrap()), [])
                .unwrap();
            assert!(changed >= 1, "the change touched no row: {sql}");
        }
        drop(conn);
        Work { dir, store }
    }

    fn conn(&self) -> Connection {
        open_read_only(&self.store).unwrap()
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// `(status, body)` as the route answers: an `Invalid` error is a 400 with `{"error": text}`.
fn outcome(result: Result<Value, MetricsError>) -> (u16, Value) {
    match result {
        Ok(body) => (200, body),
        Err(MetricsError::Invalid(text)) => (400, json!({ "error": text })),
        Err(other) => panic!("not a caller error: {other}"),
    }
}

fn param<'a>(params: &'a Value, key: &str) -> Option<&'a str> {
    params[key].as_str().filter(|v| !v.is_empty())
}

/// The answer of one manifest entry (HTTP route or MCP tool).
fn answer(conn: &Connection, entry: &Value) -> (u16, Value) {
    let params = &entry["params"];
    let scope = Scope {
        workspace: param(params, "workspace"),
        account: param(params, "account"),
    };
    if entry["surface"] == "mcp" {
        return outcome(kit_summary(conn, FROZEN_NOW, scope, false));
    }
    match entry["route"].as_str().unwrap() {
        "/api/kit" => outcome(kit_summary(conn, FROZEN_NOW, scope, true)),
        "/api/kit/workspaces" => outcome(workspace_rollup(
            conn,
            params["since"].as_str(),
            params["until"].as_str(),
            scope.account,
        )),
        "/api/kit/reports" => outcome(report_folders(conn)),
        other => panic!("{other}"),
    }
}

fn is_of_the_slice(entry: &Value) -> bool {
    match entry["surface"].as_str().unwrap() {
        "mcp" => entry["tool"] == "kit_summary",
        _ => matches!(
            entry["route"].as_str().unwrap(),
            "/api/kit" | "/api/kit/workspaces" | "/api/kit/reports"
        ),
    }
}

/// Replays every golden of the slice in `golden_dir` over `conn`; the error names the first that differs.
fn compare_dir(golden_dir: &str, conn: &Connection) -> Result<usize, String> {
    let dir = parity_dir().join(golden_dir);
    let manifest = read_json(&dir.join("manifest.json"));
    assert_eq!(manifest["frozen_now"], "2026-01-20T00:00:00+00:00");
    let mut compared = 0;
    for entry in manifest["entries"].as_array().unwrap() {
        if !is_of_the_slice(entry) {
            continue;
        }
        let file = entry["file"].as_str().unwrap();
        let (status, body) = answer(conn, entry);
        let want_status = entry["status"].as_u64().map_or(200, |s| s as u16);
        if status != want_status {
            return Err(format!("{file}: status {status}, want {want_status}"));
        }
        let golden = read_json(&dir.join(file));
        if body != golden {
            return Err(format!("{file}: differs\n  got  {body}\n  want {golden}"));
        }
        compared += 1;
    }
    Ok(compared)
}

fn compare_goldens(work: &Work) -> Result<usize, String> {
    compare_dir("golden-rich", &work.conn())
}

// ---- 1. the goldens ----

#[test]
fn the_seventeen_goldens_of_the_slice_equal_the_answers() {
    let manifest = read_json(&parity_dir().join("golden-rich/manifest.json"));
    let count = |prefix: &str| {
        manifest["entries"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["file"].as_str().unwrap().starts_with(prefix))
            .count()
    };
    assert_eq!(count("api__kit__"), 7);
    assert_eq!(count("api__kit_workspaces__"), 3);
    assert_eq!(count("api__kit_reports__"), 1);
    assert_eq!(count("mcp__kit_summary__"), 6);
    let work = Work::new(false, None);
    assert_eq!(compare_goldens(&work).unwrap(), 17);
}

#[test]
fn the_four_default_goldens_equal_the_answers_over_the_default_store() {
    // `golden/`: one project whose folder does not exist, no kit data, no workspace.
    let conn = open_read_only(&parity_dir().join("store-v11.sqlite")).unwrap();
    assert_eq!(compare_dir("golden", &conn).unwrap(), 4);
}

#[test]
fn a_changed_row_breaks_the_golden_comparison_and_names_the_entry() {
    for (sql, entry) in [
        (
            "UPDATE kit_trace_events SET is_error = 1 - is_error WHERE project_path LIKE '%/kit/beta' AND abstained = 0 AND id = (SELECT MIN(id) FROM kit_trace_events WHERE project_path LIKE '%/kit/beta')",
            "kit",
        ),
        (
            "UPDATE context_index SET last_index_at_utc = '2026-01-11T00:00:00+00:00' WHERE project_path LIKE '%/kit/alpha'",
            "kit",
        ),
        (
            "DELETE FROM workspace_repos WHERE repo_path LIKE '%/kit/alpha'",
            "api__kit__workspace_acme",
        ),
    ] {
        let work = Work::new(false, Some(sql));
        let error = compare_goldens(&work).unwrap_err();
        assert!(error.contains(entry), "{sql}: {error}");
    }
}

// ---- 2. the hand-made store against Python ----

/// The answers `make_expected.py` records, by the names it gives them.
fn extra_answers(work: &Work) -> BTreeMap<String, (u16, Value)> {
    let conn = work.conn();
    let mut got = BTreeMap::new();
    let who = [
        ("all", Scope::default()),
        (
            "acme",
            Scope {
                workspace: Some("acme"),
                account: None,
            },
        ),
        (
            "personal",
            Scope {
                workspace: Some("personal"),
                account: None,
            },
        ),
        (
            "work",
            Scope {
                workspace: None,
                account: Some("Claude work"),
            },
        ),
        (
            "default",
            Scope {
                workspace: None,
                account: Some("1"),
            },
        ),
        (
            "acme_work",
            Scope {
                workspace: Some("acme"),
                account: Some("Claude work"),
            },
        ),
    ];
    for (name, scope) in who {
        got.insert(
            format!("mcp_{name}"),
            outcome(kit_summary(&conn, FROZEN_NOW, scope, false)),
        );
        got.insert(
            format!("api_{name}"),
            outcome(kit_summary(&conn, FROZEN_NOW, scope, true)),
        );
    }
    got.insert(
        "rollup_all".into(),
        outcome(workspace_rollup(&conn, None, None, None)),
    );
    got.insert(
        "rollup_work".into(),
        outcome(workspace_rollup(&conn, None, None, Some("Claude work"))),
    );
    got.insert(
        "rollup_window".into(),
        outcome(workspace_rollup(
            &conn,
            Some("2026-01-16T00:00:00Z"),
            Some("2026-01-20T00:00:00Z"),
            None,
        )),
    );
    got.insert("reports".into(), outcome(report_folders(&conn)));
    got
}

/// Drops `commits_by_day` (OS-local-day text, checked under each `TZ` in `commits_by_day_follow_the_os_zone`).
fn without_days(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.remove("commits_by_day");
            map.values_mut().for_each(without_days);
        }
        Value::Array(items) => items.iter_mut().for_each(without_days),
        _ => {}
    }
}

#[test]
fn the_hand_made_store_equals_pythons_answers() {
    let expected = read_json(&extra_dir().join("expected.json"));
    let work = Work::new(true, None);
    let got = extra_answers(&work);
    assert_eq!(got.len(), expected.as_object().unwrap().len());
    for (name, (status, mut body)) in got {
        assert_eq!(status, 200, "{name}");
        let mut want = expected[&name].clone();
        without_days(&mut body);
        without_days(&mut want);
        assert_eq!(body, want, "{name}");
    }
}

#[test]
fn the_hand_made_store_has_what_the_goldens_lack() {
    // Guards the comparison: the parts the rich goldens cannot show are in the answers.
    let work = Work::new(true, None);
    let got = extra_answers(&work);
    let all = &got["mcp_all"].1;
    let text = all.to_string();
    for needle in [
        "\"stale\":true",
        "\"stale\":false",
        "unknown_schema_version",
        "\"commits\":3",
        "\"error_rate\":null",
        "\"status\":\"abstained\"",
    ] {
        assert!(text.contains(needle), "{needle} is not in the answer");
    }
    let rollup = &got["rollup_all"].1["workspaces"][0];
    assert_eq!(rollup["verifier"]["status"], "ok");
    assert_eq!(rollup["verifier"]["repos_scored"], 3);
    assert_eq!(rollup["ui"]["status"], "ok");
}

/// `date` is the C library's `localtime`, as Python's `time.localtime` is: an oracle that is not our zone code.
fn date(args: &[&str]) -> String {
    let out = Command::new("date").args(args).output().unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

#[test]
fn commits_by_day_follow_the_os_zone() {
    let work = Work::new(true, None);
    let conn = work.conn();
    let body = kit_summary(&conn, FROZEN_NOW, Scope::default(), false).unwrap();
    for (repo, dates) in COMMITS {
        let mut want: BTreeMap<String, i64> = BTreeMap::new();
        for stamp in dates {
            let utc = format!("{}Z", &stamp[..19]);
            let secs = date(&["-u", "-d", &utc, "+%s"]);
            *want
                .entry(date(&["-d", &format!("@{secs}"), "+%F"]))
                .or_default() += 1;
        }
        let project = body["projects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["project"] == repo)
            .unwrap();
        let git = &project["parts"]["git"];
        if dates.is_empty() {
            assert_eq!(git, &json!({"status": "no data"}), "{repo}");
        } else {
            assert_eq!(git["commits_by_day"], json!(want), "{repo}");
        }
    }
}

// ---- 3. errors, privacy, a read that changes nothing ----

#[test]
fn unknown_scope_and_bad_period_are_pythons_400_with_no_input_in_the_text() {
    let work = Work::new(false, None);
    let conn = work.conn();
    let text = |result: Result<Value, MetricsError>| match result {
        Err(MetricsError::Invalid(text)) => text,
        other => panic!("{other:?}"),
    };
    let scope = |workspace, account| Scope { workspace, account };
    assert_eq!(
        text(kit_summary(
            &conn,
            FROZEN_NOW,
            scope(Some("SECRET-ws"), None),
            true
        )),
        "unknown workspace"
    );
    assert_eq!(
        text(kit_summary(
            &conn,
            FROZEN_NOW,
            scope(None, Some("SECRET-acct")),
            false
        )),
        "unknown account; valid labels: Claude default, Claude work"
    );
    assert_eq!(
        text(workspace_rollup(&conn, None, None, Some("SECRET-acct"))),
        "unknown account; valid labels: Claude default, Claude work"
    );
    assert_eq!(
        text(workspace_rollup(&conn, Some("SECRET-day"), None, None)),
        "Invalid isoformat string: 'SECRET-day'"
    );
    assert!(workspace_rollup(&conn, None, Some(""), None).is_err());
    // the workspace parameter does not exist on the rollup route: nothing to refuse
    assert!(workspace_rollup(&conn, None, None, None).is_ok());
}

#[test]
fn an_answer_holds_no_path_and_no_file_name() {
    let work = Work::new(true, None);
    let dir = work.dir.to_str().unwrap().to_string();
    for (name, (_, body)) in extra_answers(&work) {
        let text = body.to_string();
        assert!(!text.contains(&dir), "{name}: a path");
        assert!(!text.contains("/work/rich"), "{name}: a path");
        for file in [
            "session-001.json",
            "notes.txt",
            "x.json",
            "a.json",
            "reports.txt",
            "PROJECT_KANBAN",
        ] {
            assert!(!text.contains(file), "{name}: file name {file}");
        }
    }
}

#[test]
fn reading_leaves_the_store_bytes_and_the_folder_as_they_were() {
    let work = Work::new(true, None);
    let before = fs::read(&work.store).unwrap();
    let names = |dir: &Path| -> Vec<String> {
        let mut found: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        found.sort();
        found
    };
    let folder_before = names(&work.dir);
    let _ = extra_answers(&work);
    assert_eq!(fs::read(&work.store).unwrap(), before);
    assert_eq!(names(&work.dir), folder_before, "no -wal, -shm or journal");
}

#[test]
fn a_report_folder_caps_at_2000_files_and_says_so_only_past_it() {
    let work = Work::new(true, None);
    let top = work.dir.join("plain/reports-big");
    fs::create_dir_all(&top).unwrap();
    for n in 0..2001 {
        fs::write(top.join(format!("f{n}.txt")), "").unwrap();
    }
    let exact = work.dir.join("plain/reports-exact");
    fs::create_dir_all(&exact).unwrap();
    for n in 0..2000 {
        fs::write(exact.join(format!("f{n}.txt")), "").unwrap();
    }
    let conn = work.conn();
    let body = report_folders(&conn).unwrap();
    let folders = body["workspaces"][0]["folders"].as_array().unwrap();
    let find = |name: &str| {
        folders
            .iter()
            .find(|f| f["folder"] == name)
            .unwrap()
            .clone()
    };
    assert_eq!(find("reports-big")["files"], 2000);
    assert_eq!(find("reports-big")["files_capped"], true);
    assert_eq!(find("reports-exact")["files"], 2000);
    assert_eq!(find("reports-exact")["files_capped"], false);
    // a symlink to a folder, and a regular file with `report` in its name, are not report folders
    assert!(
        folders
            .iter()
            .all(|f| f["folder"] != "ref-reports" && f["folder"] != "reports.txt")
    );
}

#[test]
fn a_hostile_store_does_not_stop_the_answer() {
    // Python raises (500) for each of these; Rust answers: a NULL time, a score that is not finite, a NULL session.
    let work = Work::new(
        true,
        Some("UPDATE events SET ts_utc = NULL WHERE source_path = 'x1' AND byte_offset = 1"),
    );
    let conn = Connection::open(&work.store).unwrap();
    conn.execute(
        "UPDATE verifier_scores SET overall_value = 1e999 WHERE project_path LIKE '%/repo-a'",
        [],
    )
    .unwrap();
    drop(conn);
    let conn = work.conn();
    assert!(kit_summary(&conn, FROZEN_NOW, Scope::default(), false).is_ok());
    assert!(workspace_rollup(&conn, None, None, None).is_ok());
}
