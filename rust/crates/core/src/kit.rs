//! `kit_summary`, the by-workspace rollup and the report folders, ported from `src/easy_personal/core/kit.py`
//! (`kit_summary`, `workspace_rollup`, `report_folders`; FR-015, FR-087, FR-088, FR-089) with the parts of
//! `sources/git_local.py`, `sources/ui.py`, `sources/verifier.py` and `sources/context.py` they call. Served as
//! `GET /api/kit`, `GET /api/kit/workspaces`, `GET /api/kit/reports` and the MCP tool `kit_summary`. Read-only:
//! nothing here writes the store or a file, and no report file is opened.
//!
//! What each answer reads: the store (`events`, `kit_trace_events`, `context_index`, `verifier_scores`,
//! `ui_sessions`, the workspaces), the project folders (`is_dir`, and `git rev-list` for commit times only, in an
//! argument list with a 10 s limit, never a shell), and the names, sizes' count and modification times of the
//! report folders (`read_dir` and `lstat`: no symlink is followed, no file is opened).
//!
//! What can show repository text: the project folder name (last path component), the workspace name and colour,
//! the repo folder name and the report folder name. Never a full path, a file name, a commit message or an author.
//!
//! Where Rust differs from Python on hostile data (Python raises, so the HTTP answer is a 500): a score that is not a
//! finite number is shown as `null`, a stored time that is NULL counts as the empty string, a git line that is not
//! `<seconds> <hash>` is skipped, and a folder name that is not UTF-8 is not listed.

use crate::metrics::{
    DAY_MICROS, GitTimedOut, MetricsError, bounds, civil_from_days, folder, is_dir, local_day,
    owner_map, period, repo_of, round_py, run_git_timed, stored_branches, text,
};
use crate::rules::{IN_USE_DAYS, RulesError, easy_context_in_use, freshness};
use crate::scope::{self, Resolved, Scope, under};
use crate::store::StoreError;
use jiff::tz::TimeZone;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type Result<T> = std::result::Result<T, MetricsError>;

/// `git rev-list` runs in this many threads at once (Python's `GIT_WORKERS`); a big repository takes 100 ms or more.
const GIT_WORKERS: usize = 8;
/// How long `git rev-list` may run before the part says `abstained` (`git_local.TIMEOUT_S`).
const GIT_TIMEOUT: Duration = Duration::from_secs(10);
/// A report folder name contains this (lowercase compare).
const REPORT_WORD: &str = "report";
const REPORT_FOLDERS_PER_REPO: usize = 8;
/// Files counted per report folder; past it `files_capped` is true.
const REPORT_FILE_CAP: i64 = 2000;
/// A file directly in the report folder is depth 1.
const REPORT_DEPTH: usize = 3;
/// A call to this server's tools shows easy-ui use (FR-088, FR-060).
const UI_PREFIX: &str = "mcp__easy-ui__";
const UNKNOWN_SCHEMA: &str = "unknown";

fn no_data() -> Value {
    json!({"status": "no data"})
}

/// A rules-module failure as this module's error: a store problem stays one, anything else is an unreadable store.
fn rules_error(error: RulesError) -> MetricsError {
    match error {
        RulesError::Store(e) => MetricsError::Store(e),
        RulesError::Parse { .. } | RulesError::Time => MetricsError::Store(StoreError::Unreadable(
            "a stored time cannot be read".into(),
        )),
    }
}

/// Python's `Path(p).name or p`: the last component, or the path itself when it has none; empty stays empty.
fn name_of(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    folder(Some(path))
}

// ---- kit_summary ----

/// `{"projects": [...], "ui": {...}}` (plus `"gone"` with `hide_gone`): every project seen in usage (in the
/// `workspace` / `account` scope, a gone worktree listed as its owner project), by folder name, each with the parts
/// `kernel`, `git`, `context` and `verifier`; `ui` is overall. `hide_gone` (the dashboard route) drops the projects
/// whose folder is missing and counts them; the MCP tool leaves it off and flags them `missing_on_disk`.
/// `now_micros` is the clock for the 7-day easy-context "in use" window and for index staleness.
pub fn kit_summary(
    conn: &Connection,
    now_micros: i64,
    scope: Scope,
    hide_gone: bool,
) -> Result<Value> {
    let sc = scope::resolve(conn, scope)?;
    let mut projects: Vec<String> = sc.scoped_projects(conn)?.into_iter().collect();
    let mut gone = None;
    if hide_gone {
        let total = projects.len();
        projects.retain(|p| is_dir(p));
        gone = Some(total - projects.len());
    }
    let context_used =
        easy_context_in_use(conn, &sc, now_micros - IN_USE_DAYS * DAY_MICROS, now_micros)
            .map_err(rules_error)?;
    let mut kernel = kernel_by_project(conn)?;
    let present: Vec<&str> = projects
        .iter()
        .map(String::as_str)
        .filter(|p| is_dir(p))
        .collect();
    let mut git: HashMap<&str, Value> = present.iter().copied().zip(git_parts(&present)).collect();
    let mut entries = Vec::with_capacity(projects.len());
    for project in &projects {
        let context = if context_used {
            match freshness(conn, project, now_micros).map_err(rules_error)? {
                None => no_data(),
                Some(found) => json!({
                    "status": "ok",
                    "last_index_at": found.last_index_at,
                    "latest_commit_utc": found.latest_commit_utc,
                    "stale": found.stale,
                }),
            }
        } else {
            json!({"status": "not in use"})
        };
        let git = git.remove(project.as_str());
        entries.push(json!({
            "project": name_of(project),
            "missing_on_disk": git.is_none(),
            "parts": {
                "kernel": kernel.remove(project).unwrap_or_else(no_data),
                "git": git.unwrap_or_else(no_data), // git on a missing folder says "no data" too
                "context": context,
                "verifier": verifier_part(conn, project)?,
            },
        }));
    }
    let mut summary = json!({"projects": entries, "ui": ui_part(conn)?});
    if let Some(gone) = gone {
        summary["gone"] = json!(gone);
    }
    Ok(summary)
}

/// The `kernel` part of every project that has trace rows, from one pass over the table (`_kernel_by_project`).
fn kernel_by_project(conn: &Connection) -> Result<HashMap<String, Value>> {
    let mut stmt = conn.prepare(
        "SELECT project_path, SUM(abstained = 0), COALESCE(SUM(CASE WHEN abstained = 0 THEN is_error END), 0), \
         COUNT(DISTINCT CASE WHEN abstained = 0 THEN kit_task_id END), \
         MIN(CASE WHEN abstained = 0 THEN ts_utc END), MAX(CASE WHEN abstained = 0 THEN ts_utc END), \
         SUM(abstained = 1) FROM kit_trace_events GROUP BY project_path",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, i64>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, Option<String>>(5)?,
            r.get::<_, i64>(6)?,
        ))
    })?;
    let mut found = HashMap::new();
    for row in rows {
        let (project, calls, errors, tasks, first, last, abstained) = row?;
        if calls == 0 && abstained == 0 {
            continue;
        }
        let rate = (calls != 0).then(|| round_py(errors as f64 / calls as f64, 4));
        found.insert(
            project,
            json!({
                "status": "ok",
                "tool_calls": calls,
                "tool_errors": errors,
                "error_rate": rate,
                "kit_tasks": tasks,
                "first_utc": first,
                "last_utc": last,
                "abstained": abstained,
            }),
        );
    }
    Ok(found)
}

/// The `git` part of each project, in order, `GIT_WORKERS` at a time.
fn git_parts(projects: &[&str]) -> Vec<Value> {
    let zone = TimeZone::system(); // read once per call: `TZ`, else the OS zone
    let next = AtomicUsize::new(0);
    let mut parts = vec![no_data(); projects.len()];
    thread::scope(|scope| {
        let workers: Vec<_> = (0..GIT_WORKERS.min(projects.len()))
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let at = next.fetch_add(1, Ordering::Relaxed);
                        let Some(project) = projects.get(at) else {
                            break done;
                        };
                        done.push((at, git_part(project, &zone)));
                    }
                })
            })
            .collect();
        for worker in workers {
            let done = worker
                .join()
                .unwrap_or_else(|p| std::panic::resume_unwind(p));
            for (at, part) in done {
                parts[at] = part;
            }
        }
    });
    parts
}

/// `git_local.git_part`: commits per OS-local day over all local branches, tags and remote-tracking refs; "no data"
/// for a folder that is not a repo or has no commit; "abstained" when git is too slow. Commit times only.
fn git_part(project: &str, zone: &TimeZone) -> Value {
    let args = [
        "rev-list",
        "--branches",
        "--tags",
        "--remotes",
        "--timestamp",
    ];
    let out = match run_git_timed(project, &args, GIT_TIMEOUT) {
        Err(GitTimedOut) => return json!({"status": "abstained"}),
        Ok(None) => return no_data(),
        Ok(Some(out)) => out,
    };
    let mut days: BTreeMap<String, i64> = BTreeMap::new();
    for line in out.lines() {
        // "<committer time> <hash>"; a line of any other shape is skipped (Python would raise)
        let day = line
            .split_whitespace()
            .next()
            .and_then(|first| first.parse::<i64>().ok())
            .and_then(|secs| secs.checked_mul(1_000_000))
            .and_then(|micros| local_day(micros, zone));
        if let Some(day) = day {
            *days.entry(day).or_insert(0) += 1;
        }
    }
    json!({"status": "ok", "commits": days.values().sum::<i64>(), "commits_by_day": days})
}

/// `verifier.verifier_part`: the newest score read for one project; "no data" when none was read.
fn verifier_part(conn: &Connection, project: &str) -> Result<Value> {
    let row = conn
        .query_row(
            "SELECT status, scope, generated_at_utc, overall_value, overall_abstained, reason_code \
             FROM verifier_scores WHERE project_path = ?1",
            params![project],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<f64>>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, Option<String>>(5)?,
                ))
            },
        )
        .optional()?;
    let Some((status, scope, generated, value, abstained, reason)) = row else {
        return Ok(no_data());
    };
    if status == UNKNOWN_SCHEMA {
        return Ok(json!({"status": "abstained", "reason": "unknown_schema_version"}));
    }
    // a whole score is an int (`int(value)` when `value == int(value)`), anything else stays a float
    let value = value.map(|v| match v {
        v if v.is_finite() && v == v.trunc() && v.abs() < 9.0e18 => json!(v as i64),
        v => json!(v),
    });
    Ok(json!({
        "status": "ok",
        "scope": scope,
        "generated_at": generated,
        "overall": {"value": value, "abstained": abstained != 0, "reason_code": reason},
    }))
}

/// `ui.ui_part`: the overall easy-ui check entry; "no data" when nothing was ingested.
fn ui_part(conn: &Connection) -> Result<Value> {
    let (count, passed, failed, actions, first, last): (i64, i64, i64, i64, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(status = 'passed'), 0), COALESCE(SUM(status = 'failed'), 0), \
             COALESCE(SUM(action_count), 0), MIN(started_at_utc), MAX(started_at_utc) FROM ui_sessions",
            [],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )?;
    if count == 0 {
        return Ok(no_data());
    }
    Ok(json!({
        "status": "ok",
        "sessions": count,
        "passed": passed,
        "failed": failed,
        "other": count - passed - failed,
        "actions": actions,
        "first_utc": first,
        "last_utc": last,
    }))
}

// ---- workspace rollup ----

/// Stored project path -> (session ids with an easy-ui call, the calls, the newest call time). Python's `set` keeps a
/// NULL session id as one member of its own.
type UiCalls = HashMap<String, (HashSet<Option<String>>, i64, String)>;

/// `_ui_calls`: the stored Claude events of the period and account that call an easy-ui tool, per stored project path.
fn ui_calls(
    conn: &Connection,
    since: Option<&str>,
    until: Option<&str>,
    sc: &Resolved,
) -> Result<UiCalls> {
    let clause = sc.clause(conn, since, until)?;
    let (period_sql, period_params) = period("ts_utc", since, until);
    let mut stmt = conn.prepare(&format!(
        "SELECT project_path, session_id, tool_names, ts_utc FROM events WHERE abstained = 0 \
         AND source = 'claude_code' AND project_path IS NOT NULL AND {period_sql} AND {} \
         AND (tool_names GLOB '{UI_PREFIX}*' OR tool_names GLOB '*,{UI_PREFIX}*')",
        clause.sql
    ))?;
    let mut values = text(period_params);
    values.extend(clause.params);
    let rows = stmt.query_map(params_from_iter(&values), |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, Option<String>>(3)?,
        ))
    })?;
    let mut found = UiCalls::new();
    for row in rows {
        let (path, session, tool_names, ts) = row?;
        let calls = tool_names
            .split(',')
            .filter(|name| name.starts_with(UI_PREFIX))
            .count() as i64;
        let (sessions, total, last) = found.entry(path).or_default();
        sessions.insert(session);
        *total += calls;
        *last = std::mem::take(last).max(ts.unwrap_or_default());
    }
    Ok(found)
}

/// Python's `sum()` of floats (3.12+): Neumaier compensated, so the mean rounds as Python's does.
fn py_sum(values: &[f64]) -> f64 {
    let (mut total, mut carry) = (0.0_f64, 0.0_f64);
    for &x in values {
        let t = total + x;
        carry += if total.abs() >= x.abs() {
            (total - t) + x
        } else {
            (x - t) + total
        };
        total = t;
    }
    if carry != 0.0 && carry.is_finite() {
        total += carry;
    }
    total
}

/// `_verifier_part`: the verifier entry of one workspace; each existing repo takes its newest scored row, the others
/// are no score. `scored` is `(project path, value, generated time)`.
fn workspace_verifier(
    repos: &[(String, BTreeSet<String>)],
    covers: &dyn Fn(&str, &BTreeSet<String>) -> bool,
    scored: &[(String, f64, Option<String>)],
) -> Value {
    let total = repos.iter().filter(|(repo, _)| is_dir(repo)).count();
    let mut per_repo: Vec<(f64, String, Option<String>)> = Vec::new();
    for (repo, roots) in repos {
        if !is_dir(repo) {
            continue;
        }
        let newest = scored
            .iter()
            .filter(|row| covers(&row.0, roots))
            .max_by(|a, b| (&a.2, &a.0).cmp(&(&b.2, &b.0)));
        if let Some((_, value, generated)) = newest {
            let name = name_of(repo);
            per_repo.push((
                *value,
                if name.is_empty() { "/".into() } else { name },
                generated.clone(),
            ));
        }
    }
    if per_repo.is_empty() {
        return json!({"status": "no data", "repos_scored": 0, "repos_total": total, "mean": null,
                      "lowest": null, "newest_utc": null});
    }
    let lowest = per_repo
        .iter()
        .min_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.cmp(&b.1))
        })
        .expect("not empty");
    let values: Vec<f64> = per_repo.iter().map(|item| item.0).collect();
    let mean = py_sum(&values) / per_repo.len() as f64;
    json!({
        "status": "ok",
        "repos_scored": per_repo.len(),
        "repos_total": total,
        "mean": mean.is_finite().then(|| round_py(mean, 1)), // Python would answer `Infinity`, which is not JSON
        "lowest": {"project": lowest.1, "value": lowest.0},
        "newest_utc": per_repo.iter().map(|item| &item.2).max().expect("not empty"),
    })
}

/// `discovery.resolve_project`: a folder missing on disk is its own project, else its git repo, else itself.
fn resolve_project(path: &str) -> String {
    if is_dir(path) {
        repo_of(path).unwrap_or_else(|| path.to_string())
    } else {
        path.to_string()
    }
}

/// The Kit view's by-workspace block: one entry per non-orphan workspace, in `list_all` order, and the overall
/// `ui_checks`. `verifier` reads the stored scores only and ignores `since`, `until` and `account`; `ui` counts the
/// stored Claude events in [`since`, `until`) of the account that call an easy-ui tool, for the projects a workspace
/// repo equals or lies above. An unknown `account` or a `since` / `until` that is not a date is `MetricsError::Invalid`.
pub fn workspace_rollup(
    conn: &Connection,
    since: Option<&str>,
    until: Option<&str>,
    account: Option<&str>,
) -> Result<Value> {
    let b = bounds(since, until, &TimeZone::system())?;
    let sc = scope::resolve(
        conn,
        Scope {
            workspace: None,
            account,
        },
    )?;
    let ui_found = ui_calls(conn, b.since.as_deref(), b.until.as_deref(), &sc)?;
    let mut stmt = conn.prepare(
        "SELECT project_path, overall_value, generated_at_utc FROM verifier_scores \
         WHERE status = 'ok' AND overall_abstained = 0 AND overall_value IS NOT NULL",
    )?;
    let scored: Vec<(String, f64, Option<String>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<std::result::Result<_, _>>()?;
    let owner = owner_map(&stored_branches(conn)?);
    let covers = |path: &str, roots: &BTreeSet<String>| {
        let owned = owner.get(path).map_or(path, String::as_str);
        roots
            .iter()
            .any(|root| under(owned, root) || under(path, root))
    };
    let mut entries = Vec::new();
    for workspace in workspace_list(conn)? {
        if workspace.orphan {
            continue;
        }
        let repos: Vec<(String, BTreeSet<String>)> = workspace
            .repos
            .iter()
            .map(|repo| {
                let roots = BTreeSet::from([repo.clone(), resolve_project(repo)]);
                (repo.clone(), roots)
            })
            .collect();
        let every_root: BTreeSet<String> =
            repos.iter().flat_map(|(_, r)| r.iter().cloned()).collect();
        let (mut sessions, mut calls, mut last) = (HashSet::new(), 0, String::new());
        for (path, (path_sessions, path_calls, path_last)) in &ui_found {
            if covers(path, &every_root) {
                sessions.extend(path_sessions.iter().cloned());
                calls += path_calls;
                last = last.max(path_last.clone());
            }
        }
        let ui = if calls > 0 {
            json!({"status": "ok", "sessions": sessions.len(), "calls": calls, "last_utc": last})
        } else {
            json!({"status": "not used", "sessions": 0, "calls": 0, "last_utc": null})
        };
        entries.push(json!({
            "name": workspace.name,
            "color": workspace.color,
            "repos": workspace.repos.len(),
            "verifier": workspace_verifier(&repos, &covers, &scored),
            "ui": ui,
        }));
    }
    Ok(json!({"workspaces": entries, "ui_checks": ui_part(conn)?}))
}

struct Workspace {
    name: String,
    color: Option<String>,
    orphan: bool,
    repos: Vec<String>,
}

/// `workspaces.list_all` as far as the rollup and the report folders need it: the same order and the same repo order.
fn workspace_list(conn: &Connection) -> Result<Vec<Workspace>> {
    let listed = crate::workspaces::list_all(conn)?;
    let string = |v: &Value| v.as_str().map(str::to_string);
    Ok(listed["workspaces"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|w| Workspace {
            name: string(&w["name"]).unwrap_or_default(),
            color: string(&w["color"]),
            orphan: w["is_orphan"].as_bool().unwrap_or(false),
            repos: w["repos"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(string)
                .collect(),
        })
        .collect())
}

// ---- report folders ----

/// The Kit view's report folders (FR-089): per non-orphan workspace, `list_all` order, for reference only. A report
/// folder sits directly inside a workspace repo and has `report` in its lowercase name; at most 8 per repo, by name.
/// Only the repo folder name, the report folder name, counts and a time are returned: `repo_no` is the place of the
/// repo in the workspace's repo list. Never a path, a file name or a sub-folder name. Nothing is opened or written.
pub fn report_folders(conn: &Connection) -> Result<Value> {
    let mut workspaces = Vec::new();
    for workspace in workspace_list(conn)? {
        if workspace.orphan {
            continue;
        }
        let mut folders = Vec::new();
        for (repo_no, repo) in workspace.repos.iter().enumerate() {
            for name in report_names(repo) {
                let Some(stats) = report_stats(&Path::new(repo).join(&name)) else {
                    continue;
                };
                folders.push(json!({
                    "repo": repo.rsplit('/').next().unwrap_or_default(), // `os.path.basename`
                    "repo_no": repo_no,
                    "folder": name,
                    "files": stats.files,
                    "files_capped": stats.capped,
                    "easy_ui_sessions": stats.sessions,
                    "newest_utc": stats.newest_utc,
                }));
            }
        }
        workspaces.push(json!({"name": workspace.name, "folders": folders}));
    }
    Ok(json!({"workspaces": workspaces}))
}

/// The report folder names of a repo: depth 1, `report` in the lowercase name, a real directory (a symlink is not);
/// sorted, 8 at most. A repo folder that is gone or unreadable has none.
fn report_names(repo: &str) -> Vec<String> {
    let Ok(entries) = fs::read_dir(repo) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| name.to_lowercase().contains(REPORT_WORD))
        .collect();
    names.sort();
    names.truncate(REPORT_FOLDERS_PER_REPO);
    names
}

struct Stats {
    files: i64,
    capped: bool,
    sessions: i64,
    newest_utc: Option<String>,
}

/// `_report_stats`: counts the regular files of a report folder to `REPORT_DEPTH` levels, `REPORT_FILE_CAP` files at
/// most. Names and `lstat` only: no file is opened and no symlink is followed. A sub-folder that cannot be listed is
/// skipped; a top folder that cannot be listed gives `None`. Walked depth first from a stack, in the order the file
/// system lists the entries, as Python does (which files make the cap depends on that order).
fn report_stats(top: &Path) -> Option<Stats> {
    fs::read_dir(top).ok()?;
    let (mut files, mut sessions, mut capped) = (0, 0, false);
    let mut newest: Option<(f64, String)> = None;
    let mut pending = vec![(top.to_path_buf(), 1)];
    while let Some((path, depth)) = pending.pop() {
        let Ok(entries) = fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.filter_map(std::result::Result::ok) {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_file() {
                if files == REPORT_FILE_CAP {
                    capped = true;
                    break;
                }
                let Ok(meta) = entry.metadata() else {
                    continue;
                };
                files += 1;
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with("session-") && name.ends_with(".json") {
                    sessions += 1;
                }
                let stamp = meta.modified().ok().and_then(file_time);
                if let Some((mtime, stamp)) = stamp
                    && newest.as_ref().is_none_or(|(best, _)| mtime > *best)
                {
                    newest = Some((mtime, stamp)); // an unreadable time is counted, never the newest
                }
            } else if depth < REPORT_DEPTH && kind.is_dir() {
                pending.push((entry.path(), depth + 1));
            }
        }
        if capped {
            break;
        }
    }
    Some(Stats {
        files,
        capped,
        sessions,
        newest_utc: newest.map(|(_, stamp)| stamp),
    })
}

/// `(st_mtime as Python's float, "%Y-%m-%dT%H:%M:%SZ")` of a file time, or `None` when Python's `datetime` cannot
/// represent it (`_file_utc`): `fromtimestamp` rounds the float to microseconds half to even, then the second shown.
pub(crate) fn file_time(time: SystemTime) -> Option<(f64, String)> {
    let (secs, nanos) = match time.duration_since(UNIX_EPOCH) {
        Ok(d) => (
            i64::try_from(d.as_secs()).ok()?,
            i64::from(d.subsec_nanos()),
        ),
        Err(e) => {
            let d = e.duration();
            let (s, n) = (
                i64::try_from(d.as_secs()).ok()?,
                i64::from(d.subsec_nanos()),
            );
            if n == 0 {
                (-s, 0)
            } else {
                (-s - 1, 1_000_000_000 - n)
            }
        }
    };
    let mtime = secs as f64 + nanos as f64 * 1e-9; // CPython: `sec + nsec * 1e-9`
    let whole = mtime.trunc();
    let micros = ((mtime - whole) * 1e6).round_ties_even();
    let shown = if micros >= 1e6 {
        whole + 1.0
    } else if micros < 0.0 {
        whole - 1.0
    } else {
        whole
    };
    // years 1..=9999, the range of Python's `datetime`
    if !(-62_135_596_800.0..=253_402_300_799.0).contains(&shown) {
        return None;
    }
    let shown = shown as i64;
    let (year, month, day) = civil_from_days(shown.div_euclid(86_400));
    let in_day = shown.rem_euclid(86_400);
    Some((
        mtime,
        format!(
            "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
            in_day / 3600,
            in_day % 3600 / 60,
            in_day % 60
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::{file_time, name_of, py_sum};
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn name_of_is_pythons_path_name_or_the_path() {
        for (path, want) in [
            ("/a/b", "b"),
            ("/", "/"),
            ("", ""),
            ("/a/b/", "b"),
            ("..", ".."),
        ] {
            assert_eq!(name_of(path), want, "{path:?}");
        }
    }

    #[test]
    fn py_sum_is_compensated_like_python() {
        // Python 3.14: sum([0.1] * 10) == 1.0 (a plain loop gives 0.9999999999999999)
        assert_eq!(py_sum(&[0.1; 10]), 1.0);
        assert_eq!(py_sum(&[]), 0.0);
    }

    #[test]
    fn file_time_rounds_to_the_microsecond_then_shows_the_second() {
        let at = |s: u64, n: u32| file_time(UNIX_EPOCH + Duration::new(s, n)).map(|t| t.1);
        assert_eq!(
            at(1_768_694_400, 0).as_deref(),
            Some("2026-01-18T00:00:00Z")
        );
        assert_eq!(
            at(1_768_694_400, 999_999_700).as_deref(),
            Some("2026-01-18T00:00:01Z")
        );
        assert_eq!(
            at(1_768_694_400, 999_999_000).as_deref(),
            Some("2026-01-18T00:00:00Z")
        );
        assert_eq!(at(0, 0).as_deref(), Some("1970-01-01T00:00:00Z"));
        // before the epoch and past year 9999
        let before = file_time(UNIX_EPOCH - Duration::new(1, 500_000_000)).map(|t| t.1);
        assert_eq!(before.as_deref(), Some("1969-12-31T23:59:58Z"));
        assert_eq!(at(253_402_300_800, 0), None);
        // Python 3.14 pads the year to four digits: `_file_utc(-61000000000)`
        let early = file_time(UNIX_EPOCH - Duration::new(61_000_000_000, 0)).map(|t| t.1);
        assert_eq!(early.as_deref(), Some("0036-12-26T11:33:20Z"));
    }
}
