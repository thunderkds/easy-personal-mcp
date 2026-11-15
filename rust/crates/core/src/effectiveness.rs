//! Effectiveness (FR-098): tokens per good result and tokens per task done, computed on read from the stored usage
//! events and the local git history. Nothing is stored.
//!
//! A *task* is a branch whose name holds a task id (`T123`, `HUN-123`, case as written; the first one when there are
//! two; not preceded by a letter or digit). A message goes to the task of its own branch; a sidechain (sub-agent)
//! message whose own branch holds no id goes to the task of its session's main-thread branch (the branch most of the
//! session's non-sidechain events carry; a tie is no branch). `main`, `HEAD`, `worktree-agent-*`, `fix-*` and every
//! other id-less branch, and sidechains of such sessions, are the one "unattributed" line. Tasks are keyed by
//! (owner project, task id), so the same id in two repos is two tasks.
//!
//! *Good result* = FR-061, as `kitperf` computes it per session; it counts for the line of the session's own branch.
//! *Done* = the first merge commit of one of the task's branches that holds its id on the first-parent history of the repo's `main`
//! (else `master`), read with `git log --merges --first-parent`; none = not done; no git history = unknown.
//!
//! Usage is counted once per `message.id`, as `usage_summary`. Read-only; only values are bound. The answer names
//! no path and no session id: a project is its folder name.

use crate::kitperf;
use crate::metrics::{
    MetricsError, bounds, folder, local_day, merge_branch, owner_map, parse_iso, period,
    period_json, round_py, run_git_within, stored_branches, text,
};
use crate::prices::PriceTable;
use crate::scope::{self, Clause, Scope};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use rusqlite::{Connection, params_from_iter};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::time::Duration;

type Result<T> = std::result::Result<T, MetricsError>;

const GIT_TIMEOUT: Duration = Duration::from_secs(5);
const UNATTRIBUTED_PREFIXES: [&str; 2] = ["worktree-agent-", "fix-"];

/// The first task id in a branch name, or `None` for an id-less (or excluded) branch.
pub fn task_id(branch: &str) -> Option<String> {
    if UNATTRIBUTED_PREFIXES.iter().any(|p| branch.starts_with(p)) {
        return None;
    }
    let bytes = branch.as_bytes();
    for start in 0..bytes.len() {
        // an id starts with an ASCII byte, so `start` is a char boundary whenever a prefix matches below
        if !branch.is_char_boundary(start)
            || branch[..start]
                .chars()
                .next_back()
                .is_some_and(char::is_alphanumeric)
        {
            continue;
        }
        for prefix in ["HUN-", "T"] {
            let Some(rest) = bytes[start..].strip_prefix(prefix.as_bytes()) else {
                continue;
            };
            let digits = rest.iter().take_while(|b| b.is_ascii_digit()).count();
            if digits > 0 {
                // every byte of the id is ASCII, so these are char boundaries
                return Some(branch[start..start + prefix.len() + digits].to_string());
            }
        }
    }
    None
}

#[derive(Default)]
struct Line {
    tokens: [i64; 4], // input, output, cache write (creation), cache read
    usd: f64,
    priced: i64,
    abstained: BTreeSet<String>,
    sessions: BTreeSet<String>,
    branches: BTreeSet<String>,
    good: i64,
}

impl Line {
    fn total(&self) -> i64 {
        self.tokens.iter().sum()
    }

    fn per_good(&self) -> Option<f64> {
        (self.good > 0).then(|| round_py(self.total() as f64 / self.good as f64, 1))
    }

    fn json(&self) -> Value {
        // as `usage_breakdown`: a sum only when something was priced or nothing was unknown; never $0 for unknown
        let usd = (self.priced > 0 || self.abstained.is_empty()).then(|| round_py(self.usd, 6));
        json!({
            "input_tokens": self.tokens[0],
            "output_tokens": self.tokens[1],
            "cache_creation_input_tokens": self.tokens[2],
            "cache_read_input_tokens": self.tokens[3],
            "tokens": self.total(),
            "api_equivalent_cost": {
                "usd": usd,
                "unknown_price": !self.abstained.is_empty(),
                "abstained_models": self.abstained,
            },
            "sessions": self.sessions.len(),
            "good_results": self.good,
            "tokens_per_good_result": self.per_good(),
        })
    }
}

struct Message {
    session: Option<String>,
    project: Option<String>,
    branch: Option<String>,
    sidechain: bool,
    model: Option<String>,
    tokens: [i64; 4],
}

fn messages(
    conn: &Connection,
    since: Option<&str>,
    until: Option<&str>,
    inner: (Option<&str>, Option<&str>),
    scope: Clause,
) -> Result<Vec<Message>> {
    let (inner_sql, params) = period("ts_utc", inner.0, inner.1);
    let (where_ts, outer) = period("ts", since, until);
    let mut params = text(params);
    params.extend(scope.params);
    params.extend(text(outer));
    let mut stmt = conn.prepare(&format!(
        "SELECT s, p, b, sc, model, i, o, cc, cr FROM ( \
           SELECT MIN(ts_utc) AS ts, MAX(session_id) AS s, MAX(project_path) AS p, MAX(git_branch) AS b, \
                  MAX(is_sidechain) AS sc, MAX(model) AS model, MAX(input_tokens) AS i, MAX(output_tokens) AS o, \
                  MAX(cache_creation_input_tokens) AS cc, MAX(cache_read_input_tokens) AS cr \
           FROM events WHERE abstained = 0 AND message_id IS NOT NULL \
           AND input_tokens IS NOT NULL AND {inner_sql} AND {} GROUP BY message_id \
         ) WHERE {where_ts}",
        scope.sql
    ))?;
    let rows = stmt.query_map(params_from_iter(&params), |r| {
        let n = |i| r.get::<_, Option<i64>>(i).map(Option::unwrap_or_default);
        Ok(Message {
            session: r.get(0)?,
            project: r.get(1)?,
            branch: r.get(2)?,
            sidechain: r.get::<_, Option<i64>>(3)? == Some(1),
            model: r.get(4)?,
            tokens: [n(5)?, n(6)?, n(7)?, n(8)?],
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// Session -> the branch most of its non-sidechain events carry (a tie: no branch).
fn main_threads(conn: &Connection) -> Result<HashMap<String, String>> {
    let mut stmt = conn.prepare(
        "SELECT session_id, git_branch, COUNT(*) FROM events WHERE abstained = 0 AND session_id IS NOT NULL \
         AND COALESCE(is_sidechain, 0) = 0 AND git_branch IS NOT NULL AND git_branch != '' GROUP BY 1, 2",
    )?;
    let mut counts: HashMap<String, Vec<(String, i64)>> = HashMap::new();
    for row in stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })? {
        let (session, branch, n) = row?;
        counts.entry(session).or_default().push((branch, n));
    }
    Ok(counts
        .into_iter()
        .filter_map(|(session, branches)| {
            let top = branches.iter().map(|(_, n)| *n).max()?;
            let mut leaders = branches.into_iter().filter(|(_, n)| *n == top);
            let (first, _) = leaders.next()?;
            leaders.next().is_none().then_some((session, first))
        })
        .collect())
}

/// Branch -> its first merge time (unix seconds) on the first-parent history of `main` (else `master`);
/// `None` when the folder has no git history or neither branch. Only an absolute path is read: an empty or relative
/// project would otherwise be resolved against the process's working folder.
fn first_merges(repo: &str) -> Option<HashMap<String, i64>> {
    if !Path::new(repo).is_absolute() || !Path::new(repo).join(".git").exists() {
        return None;
    }
    let main = ["main", "master"].into_iter().find(|b| {
        run_git_within(
            repo,
            &[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("refs/heads/{b}"),
            ],
            GIT_TIMEOUT,
        )
        .is_some()
    })?;
    let log = run_git_within(
        repo,
        &[
            "log",
            "--merges",
            "--first-parent",
            "--reverse",
            "--format=%ct%x09%s",
            &format!("refs/heads/{main}"),
        ],
        GIT_TIMEOUT,
    )
    .unwrap_or_default(); // empty output: no merge yet
    let mut first = HashMap::new();
    for line in log.lines() {
        let Some((secs, subject)) = line.split_once('\t') else {
            continue;
        };
        if let (Ok(secs), Some(branch)) = (secs.parse::<i64>(), merge_branch(subject)) {
            first.entry(branch).or_insert(secs); // oldest first: the first merge wins
        }
    }
    Some(first)
}

fn utc(secs: i64) -> Option<String> {
    Timestamp::from_second(secs)
        .ok()
        .map(|t| t.strftime("%Y-%m-%dT%H:%M:%SZ").to_string())
}

/// [`effectiveness_in`] in the machine's local zone (`TZ`, else the OS zone).
pub fn effectiveness(
    conn: &Connection,
    since: Option<&str>,
    until: Option<&str>,
    scope: Scope,
) -> Result<Value> {
    effectiveness_in(conn, since, until, scope, &TimeZone::system())
}

/// Per task (tokens, API-equivalent cost, sessions, good results, tokens per good result, done time, tokens per
/// task done), the unattributed line and the totals for [since, until) in a workspace / account scope, plus the
/// oldest stored day (`zone`'s calendar day). An offset-less bound is local time in `zone`.
pub fn effectiveness_in(
    conn: &Connection,
    since: Option<&str>,
    until: Option<&str>,
    scope: Scope,
    zone: &TimeZone,
) -> Result<Value> {
    let b = bounds(since, until, zone)?;
    let clause =
        scope::resolve(conn, scope)?.clause(conn, b.since.as_deref(), b.until.as_deref())?;
    let prices = PriceTable::shipped().map_err(MetricsError::Prices)?;
    let owner = owner_map(&stored_branches(conn)?);
    let project_of =
        |p: Option<&str>| p.map(|p| owner.get(p).cloned().unwrap_or_else(|| p.to_string()));
    let mains = main_threads(conn)?;

    let mut tasks: BTreeMap<(String, String), Line> = BTreeMap::new(); // (project, task id)
    let mut unattributed = Line::default();
    let mut total = Line::default();
    let msgs = messages(
        conn,
        b.since.as_deref(),
        b.until.as_deref(),
        (b.inner_since.as_deref(), b.inner_until.as_deref()),
        clause,
    )?;
    for m in msgs {
        let own = m.branch.as_deref().filter(|b| !b.is_empty());
        let parent = m.session.as_ref().and_then(|s| mains.get(s));
        let target = match (own.and_then(task_id), m.sidechain) {
            (Some(id), _) => Some((id, own.unwrap_or_default())),
            (None, true) => parent.and_then(|b| task_id(b).map(|id| (id, b.as_str()))),
            (None, false) => None,
        };
        let usd = prices.cost(
            m.model.as_deref(),
            m.tokens[0],
            m.tokens[1],
            m.tokens[2],
            m.tokens[3],
        );
        let mut lines: Vec<&mut Line> = vec![&mut total];
        let line = match target {
            Some((id, branch)) => {
                let key = (project_of(m.project.as_deref()).unwrap_or_default(), id);
                let line = tasks.entry(key).or_default();
                line.branches.insert(branch.to_string());
                if let Some(own) = own {
                    line.branches.insert(own.to_string());
                }
                line
            }
            None => &mut unattributed,
        };
        lines.push(line);
        for line in lines {
            for (sum, t) in line.tokens.iter_mut().zip(m.tokens) {
                *sum += t;
            }
            match usd {
                Some(usd) => {
                    line.usd += usd;
                    line.priced += 1;
                }
                None => {
                    line.abstained.insert(
                        m.model
                            .clone()
                            .filter(|m| !m.is_empty())
                            .unwrap_or_else(|| "(unknown)".into()),
                    );
                }
            }
            if let Some(s) = &m.session {
                line.sessions.insert(s.clone());
            }
        }
    }

    // good results (FR-061), on the line of the session's own branch, among the sessions of that line
    for (session, path, branch, good) in kitperf::session_results(conn, b.since.as_deref())? {
        if good != Some(1) {
            continue;
        }
        let key = branch
            .as_deref()
            .and_then(task_id)
            .map(|id| (project_of(path.as_deref()).unwrap_or_default(), id));
        let line = match key {
            Some(key) => tasks.get_mut(&key),
            None => Some(&mut unattributed),
        };
        if let Some(line) = line.filter(|l| l.sessions.contains(&session)) {
            line.good += 1;
            total.good += 1;
        }
    }

    // done time from local git, once per project
    let mut merges: HashMap<String, Option<HashMap<String, i64>>> = HashMap::new();
    let mut no_git: BTreeSet<String> = BTreeSet::new();
    let (mut done_count, mut done_tokens) = (0_i64, 0_i64);
    let mut rows: Vec<(i64, Value)> = Vec::new();
    for ((project, id), line) in &tasks {
        let known = merges
            .entry(project.clone())
            .or_insert_with(|| first_merges(project));
        // only a branch that holds this task's id makes it done (not a sub-agent's shared `worktree-agent-*`)
        let done = known.as_ref().map(|m| {
            line.branches
                .iter()
                .filter(|b| task_id(b).as_deref() == Some(id.as_str()))
                .filter_map(|b| m.get(b))
                .min()
                .copied()
        });
        let (done_utc, state) = match done {
            None => {
                no_git.insert(folder(Some(project)));
                (None, "unknown")
            }
            Some(None) => (None, "not done"),
            Some(Some(secs)) => {
                done_count += 1;
                done_tokens += line.total();
                (utc(secs), "done")
            }
        };
        let mut row = line.json();
        let map = row.as_object_mut().expect("an object");
        map.insert("task_id".into(), json!(id));
        map.insert("project".into(), json!(folder(Some(project))));
        map.insert("branches".into(), json!(line.branches));
        map.insert("done_utc".into(), json!(done_utc));
        map.insert("done_state".into(), json!(state));
        map.insert(
            "tokens_per_task_done".into(),
            json!((state == "done").then(|| line.total())),
        );
        rows.push((line.total(), row));
    }
    rows.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1["task_id"].as_str().cmp(&b.1["task_id"].as_str()))
    });

    let mut totals = total.json();
    let map = totals.as_object_mut().expect("an object");
    map.insert("tasks".into(), json!(tasks.len()));
    map.insert("tasks_done".into(), json!(done_count));
    map.insert(
        "tokens_per_task_done".into(),
        json!((done_count > 0).then(|| round_py(done_tokens as f64 / done_count as f64, 1))),
    );

    let oldest: Option<String> = conn.query_row(
        "SELECT MIN(ts_utc) FROM events WHERE abstained = 0 AND ts_utc IS NOT NULL",
        [],
        |r| r.get(0),
    )?;
    let oldest_day = oldest
        .as_deref()
        .and_then(parse_iso)
        .and_then(|m| local_day(m, zone));
    let mut notes = Vec::new();
    if !no_git.is_empty() {
        notes.push(format!(
            "no local git history for {}: done time unknown",
            no_git.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    Ok(json!({
        "period": period_json(&b),
        "price_table_date": prices.as_of,
        "oldest_stored_day": oldest_day,
        "retention_note": oldest_day.as_ref().map(|d| format!(
            "events before {d} are not in the store (pruned with --retention-days, or never recorded)"
        )),
        "tasks": rows.into_iter().map(|(_, r)| r).collect::<Vec<_>>(),
        "unattributed": unattributed.json(),
        "totals": totals,
        "notes": notes,
    }))
}

#[cfg(test)]
mod tests {
    use super::task_id;

    #[test]
    fn task_ids_are_read_as_written_first_one_wins() {
        for (branch, id) in [
            ("T075", Some("T075")),
            ("feat/T12-x", Some("T12")),
            ("HUN-245", Some("HUN-245")),
            ("feat/HUN-12-and-T13", Some("HUN-12")),
            ("T", None),
            ("t075", None),
            ("LAST5", None),
            ("main", None),
            ("HEAD", None),
            ("fix-T075", None),
            ("worktree-agent-T1", None),
            ("hun-5", None),
            ("é-T7", Some("T7")),
            ("éT7", None),
        ] {
            assert_eq!(task_id(branch).as_deref(), id, "{branch}");
        }
    }
}
