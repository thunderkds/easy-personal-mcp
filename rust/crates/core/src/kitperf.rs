//! Kit performance (FR-060..FR-063): sessions that used a kit MCP server, compared like with like.
//!
//! Port of `src/easy_personal/core/kitperf.py`. One row per session (with at least one tool call) is built from `events`
//! on every call (nothing is stored). For each kit server and each measure (tokens per call, cost per call, good-result
//! share), kit sessions are compared with non-kit sessions inside strata of (owner project, main model, tool-call band);
//! the strata are pooled and a pooled difference is shown only when each side has at least 10 sessions. A difference of
//! at least 5 % where the server side is better is a *finding* with a habit; the others are listed as `other`. All say
//! "linked to", never "caused". Metadata only: no id, path or text from an event leaves this module.
//!
//! Not read here (as in the rest of this build): `config.yaml`, so the good-result error threshold is Python's default.

use crate::kit::py_sum;
use crate::metrics::{
    MetricsError, bounds, merge_branch, owner_map, parse_iso, period, round_py, run_git_timed,
    stored_branches, text,
};
use crate::prices::PriceTable;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use rusqlite::{Connection, params_from_iter};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::time::{Duration, Instant};

const SERVERS: [&str; 4] = ["easy-ui", "easy-verifier", "easy-context", "easy-memory"];
const MIN_SIDE: usize = 10;
const MIN_GAP_PCT: f64 = 5.0;
pub const DEFAULT_LIMIT: i64 = 3;
const MAX_LIMIT: i64 = 10;
const WINDOW_AFTER_SECONDS: i64 = 30 * 60;
const GIT_BUDGET: Duration = Duration::from_secs(2);
/// Python's `KIT_ERROR_RATE_DEFAULT`.
const KIT_ERROR_RATE: f64 = 0.04;
const NOTE: &str = "linked to, not caused";
const IGNORED_BRANCHES: [&str; 4] = ["", "main", "master", "HEAD"];
const MEASURES: [(&str, bool, &str); 3] = [
    ("tokens_per_call", true, "tokens per tool call"),
    (
        "cost_per_call",
        true,
        "dollars of API-equivalent cost per tool call",
    ),
    ("good_result", false, "good results"),
];

fn habit(server: &str) -> &'static str {
    match server {
        "easy-ui" => "Check UI changes with easy-ui before you call the work done.",
        "easy-verifier" => "Run easy-verifier on your change before you hand it over.",
        "easy-context" => "Search with easy-context before you read files one by one.",
        _ => "Search easy-memory for earlier notes before you start the task.",
    }
}

type Result<T> = std::result::Result<T, MetricsError>;
type Candidate = (String, String);
type Stratum = (Option<String>, String, u8);
type Sides = (Vec<f64>, Vec<f64>);

#[derive(Default)]
struct Row {
    tokens: i64,
    cost: Option<f64>,
    assistants: i64,
    first: Option<String>,
    last: String,
    err_sum: i64,
    err_known: i64,
    calls: i64,
    servers: BTreeSet<&'static str>,
    errors: Option<i64>,
    tokens_per_call: f64,
    cost_per_call: Option<f64>,
    model: String,
    path: Option<String>,
    branch: Option<String>,
    tie: bool,
    band: u8,
    stratum: Stratum,
    candidate: Option<Candidate>,
    good_result: Option<i64>,
}

impl Row {
    fn error_rate(&self) -> Option<f64> {
        self.errors.map(|e| e as f64 / self.calls as f64)
    }

    fn window(&self) -> (i64, i64) {
        let secs = |s: &str| parse_iso(s).map_or(0, |m| m.div_euclid(1_000_000));
        (
            secs(self.first.as_deref().unwrap_or("")),
            secs(&self.last) + WINDOW_AFTER_SECONDS + 1,
        )
    }

    fn measure(&self, name: &str) -> Option<f64> {
        match name {
            "tokens_per_call" => Some(self.tokens_per_call),
            "cost_per_call" => self.cost_per_call,
            _ => self.good_result.map(|g| g as f64),
        }
    }
}

fn tool_servers(tool_names: &str) -> BTreeSet<&'static str> {
    SERVERS
        .into_iter()
        .filter(|s| {
            tool_names
                .split(',')
                .any(|n| n.starts_with(&format!("mcp__{s}__")))
        })
        .collect()
}

// ---- git -------------------------------------------------------------------------------------------------------

/// Commit times per (repo, branch) with one time budget for the whole call; what it cannot check is unknown.
struct Git {
    deadline: Instant,
    want: HashMap<Candidate, (i64, i64)>,
    times: HashMap<Candidate, Option<Vec<i64>>>,
    merges: HashMap<String, HashMap<String, Vec<String>>>,
}

/// `[\s~^:?*\[\\\x00-\x1f\x7f]|\.\.|@\{|^-`: a branch text that could act as a git option or range.
fn bad_ref(branch: &str) -> bool {
    branch.starts_with('-')
        || branch.contains("..")
        || branch.contains("@{")
        || branch
            .chars()
            .any(|c| c.is_whitespace() || "~^:?*[\\".contains(c) || c < ' ' || c == '\x7f')
}

fn is_sha(text: &str) -> bool {
    (40..=64).contains(&text.len())
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl Git {
    fn rev_list(&self, repo: &str, args: &[&str]) -> Option<Vec<String>> {
        let left = self
            .deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())?;
        let mut argv = vec!["rev-list"];
        argv.extend_from_slice(args);
        let out = run_git_timed(repo, &argv, left).ok().flatten()?;
        Some(out.lines().map(str::to_string).collect())
    }

    fn rev_list_owned(&self, repo: &str, args: &[String]) -> Option<Vec<String>> {
        self.rev_list(repo, &args.iter().map(String::as_str).collect::<Vec<_>>())
    }

    fn merge_shas(&mut self, repo: &str, branch: &str) -> Vec<String> {
        if !self.merges.contains_key(repo) {
            let mut found: HashMap<String, Vec<String>> = HashMap::new();
            let lines = self
                .rev_list(repo, &["--all", "--merges", "--format=%s"])
                .unwrap_or_default();
            let mut sha: Option<String> = None;
            for line in lines {
                if let Some(rest) = line.strip_prefix("commit ") {
                    sha = Some(rest.trim().to_string());
                } else if let Some(s) = sha.as_ref().filter(|s| is_sha(s))
                    && let Some(named) = merge_branch(&line)
                {
                    found.entry(named).or_default().push(s.clone());
                }
            }
            self.merges.insert(repo.to_string(), found);
        }
        self.merges[repo].get(branch).cloned().unwrap_or_default()
    }

    /// Sorted committer times of the commits on `branch` (through its merge commit when the ref is gone); `None` = unknown.
    fn commit_times(&mut self, repo: &str, branch: &str) -> Option<Vec<i64>> {
        let key = (repo.to_string(), branch.to_string());
        if !self.times.contains_key(&key) {
            let read = self.read(repo, branch);
            self.times.insert(key.clone(), read);
        }
        self.times[&key].clone()
    }

    fn read(&mut self, repo: &str, branch: &str) -> Option<Vec<i64>> {
        if bad_ref(branch) {
            return None;
        }
        let (low, high) = self.want[&(repo.to_string(), branch.to_string())];
        let (max_age, min_age) = (format!("--max-age={low}"), format!("--min-age={high}"));
        let with = |extra: &'_ str| -> Vec<String> {
            vec![
                max_age.clone(),
                min_age.clone(),
                "--timestamp".to_string(),
                extra.to_string(),
            ]
        };
        let lines = match self.rev_list_owned(repo, &with(&format!("refs/heads/{branch}"))) {
            Some(lines) => lines,
            None => {
                // the ref is gone, or it has no commit in the window: ask git whether the ref exists
                if self
                    .rev_list(repo, &["--max-count=1", &format!("refs/heads/{branch}")])
                    .is_some()
                {
                    return Some(Vec::new());
                }
                let shas = self.merge_shas(repo, branch);
                if shas.is_empty() {
                    return None;
                }
                let mut lines = Vec::new();
                for sha in shas {
                    lines.extend(
                        self.rev_list_owned(repo, &with(&format!("{sha}^1..{sha}^2")))
                            .unwrap_or_default(),
                    );
                }
                lines
            }
        };
        let mut times: Vec<i64> = lines
            .iter()
            .filter_map(|l| l.split_whitespace().next())
            .filter(|w| !w.is_empty() && w.bytes().all(|b| b.is_ascii_digit()))
            .filter_map(|w| w.parse().ok())
            .collect();
        times.sort_unstable();
        Some(times)
    }
}

/// A commit time in [low, high): `high` is last event + 30 min + 1 s, so 30 min after the last event still counts.
fn has_commit(times: &[i64], low: i64, high: i64) -> bool {
    let i = times.partition_point(|t| *t < low);
    i < times.len() && times[i] < high
}

// ---- sessions --------------------------------------------------------------------------------------------------

fn session_rows(
    conn: &Connection,
    since_utc: Option<&str>,
    prices: &PriceTable,
) -> rusqlite::Result<Vec<Row>> {
    let (where_utc, params) = period("ts_utc", since_utc, None);
    let params = text(params);
    let mut order: Vec<String> = Vec::new();
    let mut rows: HashMap<String, Row> = HashMap::new();
    let mut models: HashMap<String, BTreeMap<String, i64>> = HashMap::new();
    let mut stmt = conn.prepare(&format!(
        "SELECT session_id, MAX(model), MAX(input_tokens), MAX(output_tokens), MAX(cache_creation_input_tokens), \
         MAX(cache_read_input_tokens) FROM events WHERE abstained = 0 AND session_id IS NOT NULL \
         AND message_id IS NOT NULL AND input_tokens IS NOT NULL AND {where_utc} GROUP BY session_id, message_id"
    ))?;
    let mut query = stmt.query(params_from_iter(&params))?;
    while let Some(r) = query.next()? {
        let session: String = r.get(0)?;
        let model: Option<String> = r.get(1)?;
        let n = |i| r.get::<_, Option<i64>>(i).map(Option::unwrap_or_default);
        let (i, o, cc, cr) = (n(2)?, n(3)?, n(4)?, n(5)?);
        if !rows.contains_key(&session) {
            order.push(session.clone());
            rows.insert(
                session.clone(),
                Row {
                    cost: Some(0.0),
                    ..Row::default()
                },
            );
        }
        let row = rows.get_mut(&session).expect("inserted above");
        row.tokens += i + o + cc + cr;
        let usd = prices.cost(model.as_deref(), i, o, cc, cr);
        row.cost = match (usd, row.cost) {
            (Some(usd), Some(sum)) => Some(sum + usd),
            _ => None,
        };
        if let Some(model) = model.filter(|m| !m.is_empty()) {
            *models.entry(session).or_default().entry(model).or_insert(0) += o;
        }
    }
    drop(query);
    drop(stmt);
    let mut paths: HashMap<String, BTreeMap<String, i64>> = HashMap::new();
    let mut branches: HashMap<String, BTreeMap<String, i64>> = HashMap::new();
    let mut stmt = conn.prepare(&format!(
        "SELECT session_id, project_path, git_branch, COUNT(*), SUM(record_type = 'assistant'), MIN(ts_utc), \
         MAX(ts_utc), SUM(tool_error_count), COUNT(tool_error_count), GROUP_CONCAT(tool_names)\
         FROM events WHERE abstained = 0 AND session_id IS NOT NULL AND ts_utc IS NOT NULL \
         AND {where_utc} GROUP BY session_id, project_path, git_branch"
    ))?;
    let mut query = stmt.query(params_from_iter(&params))?;
    while let Some(r) = query.next()? {
        let session: String = r.get(0)?;
        let Some(row) = rows
            .get_mut(&session)
            .filter(|_| models.contains_key(&session))
        else {
            continue;
        };
        let (path, branch): (Option<String>, Option<String>) = (r.get(1)?, r.get(2)?);
        let n: i64 = r.get(3)?;
        let (first, last): (String, String) = (r.get(5)?, r.get(6)?);
        let tools: Option<String> = r.get(9)?;
        row.assistants += r.get::<_, Option<i64>>(4)?.unwrap_or(0);
        row.first = Some(match row.first.take() {
            Some(old) if old <= first => old,
            _ => first,
        });
        if last > row.last {
            row.last = last;
        }
        row.err_sum += r.get::<_, Option<i64>>(7)?.unwrap_or(0);
        row.err_known += r.get::<_, i64>(8)?;
        if let Some(tools) = tools.filter(|t| !t.is_empty()) {
            row.calls += tools.split(',').count() as i64;
            row.servers.extend(tool_servers(&tools));
        }
        if let Some(path) = path.filter(|p| !p.is_empty()) {
            *paths
                .entry(session.clone())
                .or_default()
                .entry(path)
                .or_insert(0) += n;
        }
        if let Some(branch) = branch.filter(|b| !b.is_empty()) {
            *branches
                .entry(session)
                .or_default()
                .entry(branch)
                .or_insert(0) += n;
        }
    }
    let mut result = Vec::new();
    for session in order {
        let mut row = rows.remove(&session).expect("ordered from rows");
        let Some(counts) = models.get(&session) else {
            continue;
        };
        if row.assistants == 0 || row.calls == 0 {
            continue; // no assistant event, no model, or no tool call: not compared
        }
        row.errors = (row.err_known > 0).then_some(row.err_sum); // no counted record = unknown
        row.tokens_per_call = row.tokens as f64 / row.calls as f64;
        row.cost_per_call = row.cost.map(|c| c / row.calls as f64);
        // the most output tokens, a tie going to the smaller name
        row.model = counts
            .iter()
            .min_by_key(|(name, n)| (-**n, (*name).clone()))
            .map(|(m, _)| m.clone())
            .unwrap_or_default();
        row.path = paths.get(&session).and_then(|c| {
            c.iter()
                .min_by_key(|(p, n)| (-**n, (*p).clone()))
                .map(|(p, _)| p.clone())
        });
        if let Some(counts) = branches.get(&session) {
            let top = counts.values().copied().max().unwrap_or(0);
            let leaders: Vec<&String> = counts
                .iter()
                .filter(|(_, n)| **n == top)
                .map(|(b, _)| b)
                .collect();
            row.tie = leaders.len() > 1;
            row.branch = (leaders.len() == 1).then(|| leaders[0].clone());
        }
        result.push(row);
    }
    Ok(result)
}

fn candidate(row: &Row, owner: &HashMap<String, String>) -> Option<Candidate> {
    let path = row.path.as_ref().map(|p| owner.get(p).unwrap_or(p).clone());
    let branch = row
        .branch
        .as_deref()
        .filter(|b| !IGNORED_BRANCHES.contains(b))?;
    if row.error_rate().is_none() || row.tie {
        return None;
    }
    let path = path.filter(|p| !p.is_empty())?;
    Path::new(&path)
        .join(".git")
        .exists()
        .then(|| (path, branch.to_string()))
}

fn prepare(
    conn: &Connection,
    since_utc: Option<&str>,
    prices: &PriceTable,
    threshold: f64,
) -> rusqlite::Result<Vec<Row>> {
    let mut sessions = session_rows(conn, since_utc, prices)?;
    let mut calls: Vec<i64> = sessions.iter().map(|r| r.calls).collect();
    calls.sort_unstable();
    let (low, high) = if calls.is_empty() {
        (0, 0)
    } else {
        (calls[calls.len() / 3], calls[2 * calls.len() / 3])
    };
    let owner = owner_map(&stored_branches(conn)?);
    let mut want: HashMap<Candidate, (i64, i64)> = HashMap::new();
    for row in &mut sessions {
        row.band = if row.calls < low {
            0
        } else if row.calls < high {
            1
        } else {
            2
        };
        let key = row.path.as_ref().map(|p| owner.get(p).unwrap_or(p).clone());
        row.stratum = (key, row.model.clone(), row.band);
        row.candidate = candidate(row, &owner);
        if let Some(c) = &row.candidate {
            let (low, high) = row.window();
            let old = want.get(c).copied().unwrap_or((low, high));
            want.insert(c.clone(), (old.0.min(low), old.1.max(high)));
        }
    }
    let mut git = Git {
        deadline: Instant::now() + GIT_BUDGET,
        want,
        times: HashMap::new(),
        merges: HashMap::new(),
    };
    let mut newest_first: Vec<usize> = (0..sessions.len()).collect();
    newest_first.sort_by(|a, b| sessions[*b].last.cmp(&sessions[*a].last)); // stable: ties keep their order
    for i in newest_first {
        let times = sessions[i]
            .candidate
            .clone()
            .and_then(|(repo, branch)| git.commit_times(&repo, &branch));
        sessions[i].good_result = times.map(|times| {
            let (low, high) = sessions[i].window();
            i64::from(
                has_commit(&times, low, high)
                    && sessions[i].error_rate().is_some_and(|r| r <= threshold),
            )
        });
    }
    Ok(sessions)
}

// ---- comparison ------------------------------------------------------------------------------------------------

/// (kit sessions, other sessions, pooled delta %) over the strata that have both sides.
fn pool(sessions: &[Row], server: &str, measure: &str) -> (usize, usize, Option<f64>) {
    let mut strata: Vec<(Stratum, Sides)> = Vec::new();
    let mut index: HashMap<Stratum, usize> = HashMap::new();
    for row in sessions {
        let Some(value) = row.measure(measure) else {
            continue;
        };
        let at = *index.entry(row.stratum.clone()).or_insert_with(|| {
            strata.push((row.stratum.clone(), (Vec::new(), Vec::new())));
            strata.len() - 1
        });
        let sides = &mut strata[at].1;
        if row.servers.contains(server) {
            sides.0.push(value)
        } else {
            sides.1.push(value)
        }
    }
    let (mut with_n, mut without_n) = (0, 0);
    let (mut num, mut den) = (0.0_f64, 0.0_f64);
    for (_, (kit, other)) in &strata {
        if kit.is_empty() || other.is_empty() {
            continue;
        }
        let other_mean = py_sum(other) / other.len() as f64;
        if other_mean == 0.0 {
            continue;
        }
        with_n += kit.len();
        without_n += other.len();
        num += kit.len() as f64 * (py_sum(kit) / kit.len() as f64 - other_mean);
        den += kit.len() as f64 * other_mean;
    }
    (with_n, without_n, (den != 0.0).then(|| num / den * 100.0))
}

struct Finding {
    server: &'static str,
    measure: &'static str,
    delta: f64,
    with: usize,
    without: usize,
    habit: Option<&'static str>,
}

fn finding(
    server: &'static str,
    measure: (&'static str, bool, &str),
    delta: f64,
    with: usize,
    without: usize,
) -> (Finding, String) {
    let (name, lower_is_better, words) = measure;
    let delta = round_py(delta, 1);
    let text = format!(
        "Sessions that used {server} are linked to {:.1}% {} {words} than similar sessions without it ({with} vs {without} sessions).",
        delta.abs(),
        if delta < 0.0 { "fewer" } else { "more" },
    );
    let better = (if lower_is_better {
        delta < 0.0
    } else {
        delta > 0.0
    }) && delta.abs() >= MIN_GAP_PCT;
    let finding = Finding {
        server,
        measure: name,
        delta,
        with,
        without,
        habit: better.then(|| habit(server)),
    };
    (finding, text)
}

fn as_json(f: &Finding, text: &str) -> Value {
    json!({
        "server": f.server, "measure": f.measure, "delta_pct": f.delta, "with": f.with, "without": f.without,
        "text": text, "habit": f.habit,
    })
}

/// [`kit_findings`] with `since` read in the machine's local zone (Python's `astimezone()`).
pub fn kit_findings_local(
    conn: &Connection,
    limit: i64,
    since: Option<&str>,
    now_micros: i64,
) -> Result<Value> {
    kit_findings(conn, limit, since, now_micros, &TimeZone::system())
}

/// `findings` (server side better, habit, at most `limit`, capped at 10) and `other` (every other passing comparison).
/// `since` is an ISO date or datetime (without an offset: local time in `zone`).
pub fn kit_findings(
    conn: &Connection,
    limit: i64,
    since: Option<&str>,
    now_micros: i64,
    zone: &TimeZone,
) -> Result<Value> {
    let limit = limit.clamp(1, MAX_LIMIT) as usize;
    let b = bounds(since, None, zone)?;
    let prices = PriceTable::shipped().map_err(MetricsError::Prices)?;
    let sessions = prepare(conn, b.since.as_deref(), &prices, KIT_ERROR_RATE)?;
    let (mut found, mut not_enough): (Vec<(Finding, String)>, Vec<Value>) =
        (Vec::new(), Vec::new());
    for server in SERVERS {
        let (with_n, without_n, _) = pool(&sessions, server, "tokens_per_call");
        if with_n < MIN_SIDE || without_n < MIN_SIDE {
            not_enough.push(json!({"server": server, "with": with_n, "without": without_n}));
            continue;
        }
        for measure in MEASURES {
            let (with_n, without_n, delta) = pool(&sessions, server, measure.0);
            if let Some(delta) = delta.filter(|_| with_n >= MIN_SIDE && without_n >= MIN_SIDE) {
                found.push(finding(server, measure, delta, with_n, without_n));
            }
        }
    }
    found.sort_by(|(a, _), (b, _)| {
        b.delta
            .abs()
            .total_cmp(&a.delta.abs())
            .then(a.server.cmp(b.server))
            .then(a.measure.cmp(b.measure))
    });
    let (with_habit, other): (Vec<_>, Vec<_>) = found.iter().partition(|(f, _)| f.habit.is_some());
    let as_of = Timestamp::from_microsecond(now_micros).map_or_else(
        |_| String::new(),
        |t| t.strftime("%Y-%m-%dT%H:%M:%SZ").to_string(),
    );
    Ok(json!({
        "as_of": as_of,
        "sessions": sessions.len(),
        "findings": with_habit.iter().take(limit).map(|(f, t)| as_json(f, t)).collect::<Vec<_>>(),
        "other": other.iter().map(|(f, t)| as_json(f, t)).collect::<Vec<_>>(),
        "not_enough": not_enough,
        "unknown_results": sessions.iter().filter(|r| r.good_result.is_none()).count(),
        "note": NOTE,
    }))
}
