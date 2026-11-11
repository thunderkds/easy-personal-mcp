//! The v1 rule pack and the `warnings` answer: `rules::evaluate`, ported from `src/easy_personal/core/rules.py`
//! (plus the store reads of `core/metrics.py`, `core/kit.py` and `sources/context.py` that the rules use).
//!
//! The pack is read from the shipped `data/rules.yaml` by a hand-written line parser (the file is not YAML;
//! no YAML crate). Read-only: nothing here writes. No row text, path or secret is put into an error.
//!
//! The optional `workspace` and `account` scope (T017, FR-035/FR-036) is [`evaluate_scoped`], resolved by `scope.rs`;
//! [`evaluate`] is the same with no scope. Not ported (nothing in the goldens uses them, or the guide's cut list
//! excludes them): the `checked` list, the owner `config.yaml` threshold override, and the `plan` answer (it
//! is `plan.rs`). The clock is a parameter (`now_micros`, microseconds since the Unix epoch, UTC); nothing reads the
//! system clock. Python's `context_stale_indexes` reads the wall clock inside its freshness check; here that
//! check uses the same `now_micros` as every other window.
//!
//! Time parsing, project grouping and `git` calls are the helpers of `metrics.rs` (`pub(crate)` there).

use crate::metrics::{civil_from_days, parse_iso, project_map, round_py, run_git_within};
use crate::scope::{self, Clause, Resolved, Scope, ScopeError};
use crate::store::StoreError;
use rusqlite::types::Value as Bound;
use rusqlite::{Connection, params, params_from_iter};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::rc::Rc;
use std::time::Duration;

const SHIPPED: &str = include_str!("../../../../src/easy_personal/data/rules.yaml");
const DAY: i64 = 86_400_000_000;
const SECOND: f64 = 1_000_000.0;
const CAUSE_LIMIT: usize = 3;
const MIN_PROJECT_SHARE: f64 = 0.05; // a project needs this share of tokens to be named for low cache reuse
const BIGGER_SESSIONS_FROM: f64 = 0.10; // tokens per session up by more than this = bigger sessions
pub(crate) const IN_USE_DAYS: i64 = 7; // easy-context counts as in use with a tool call this recently (FR-080)
const SKEW_S: f64 = 60.0; // an index this close to a commit, or ahead of the clock, is not called stale
const TOOL_PREFIX: &str = "mcp__easy-context__";

// ---- errors ----

/// A rule-pack problem or a store problem. Names a line number or a reason, never the text of a line or a row.
#[derive(Debug)]
pub enum RulesError {
    Parse {
        line: usize,
        reason: &'static str,
    },
    Store(StoreError),
    /// A stored time that cannot be read as a date.
    Time,
}

impl fmt::Display for RulesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RulesError::Parse { line, reason } => write!(f, "rules.yaml line {line}: {reason}"),
            RulesError::Store(e) => write!(f, "{e}"),
            RulesError::Time => write!(f, "a stored time cannot be read"),
        }
    }
}

impl std::error::Error for RulesError {}

impl From<rusqlite::Error> for RulesError {
    fn from(e: rusqlite::Error) -> Self {
        RulesError::Store(StoreError::from(e))
    }
}

/// A scope that changed while the rules were read (it was resolved at the start): reported as a store problem.
impl From<ScopeError> for RulesError {
    fn from(e: ScopeError) -> Self {
        match e {
            ScopeError::Unknown(_) => RulesError::Store(StoreError::Unreadable(
                "the scope changed while the rules were read".into(),
            )),
            ScopeError::Store(e) => RulesError::Store(e),
        }
    }
}

/// What [`evaluate_scoped`] can refuse: a workspace or account the store does not know (Python's `ValueError`, which
/// the HTTP layer answers with 400; the text equals Python's and echoes no input), or any other [`RulesError`].
#[derive(Debug)]
pub enum ScopedRulesError {
    UnknownScope(String),
    Rules(RulesError),
}

impl fmt::Display for ScopedRulesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScopedRulesError::UnknownScope(text) => write!(f, "{text}"),
            ScopedRulesError::Rules(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ScopedRulesError {}

impl From<RulesError> for ScopedRulesError {
    fn from(e: RulesError) -> Self {
        ScopedRulesError::Rules(e)
    }
}

impl From<ScopeError> for ScopedRulesError {
    fn from(e: ScopeError) -> Self {
        match e {
            ScopeError::Unknown(text) => ScopedRulesError::UnknownScope(text),
            ScopeError::Store(e) => ScopedRulesError::Rules(RulesError::Store(e)),
        }
    }
}

type Result<T> = std::result::Result<T, RulesError>;

// ---- the rule pack ----

/// A number as Python keeps it: a whole value is an int (`150000`), anything else a float (`0.02`), so the
/// JSON text equals Python's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Num {
    Int(i64),
    Float(f64),
}

impl Num {
    pub fn as_f64(self) -> f64 {
        match self {
            Num::Int(i) => i as f64,
            Num::Float(f) => f,
        }
    }

    fn json(self) -> Value {
        match self {
            Num::Int(i) => json!(i),
            Num::Float(f) => json!(f),
        }
    }

    fn from_text(text: &str) -> Option<Num> {
        let value: f64 = text.parse().ok()?;
        Some(
            if value.is_finite() && value.fract() == 0.0 && value.abs() < 9.0e15 {
                Num::Int(value as i64)
            } else {
                Num::Float(value)
            },
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    CacheHitRatio,
    MedianSessionContext,
    UnreadableShare,
    TopProjectShare,
    WeekOverWeekTokens,
    KitErrorRate,
    ContextStaleIndexes,
}

impl Metric {
    fn parse(name: &str) -> Option<Metric> {
        Some(match name {
            "cache_hit_ratio" => Metric::CacheHitRatio,
            "median_session_context" => Metric::MedianSessionContext,
            "unreadable_share" => Metric::UnreadableShare,
            "top_project_share" => Metric::TopProjectShare,
            "week_over_week_tokens" => Metric::WeekOverWeekTokens,
            "kit_error_rate" => Metric::KitErrorRate,
            "context_stale_indexes" => Metric::ContextStaleIndexes,
            _ => return None,
        })
    }

    /// The rules that count tokens also report `cache_read_share`.
    fn counts_tokens(self) -> bool {
        matches!(
            self,
            Metric::WeekOverWeekTokens
                | Metric::CacheHitRatio
                | Metric::MedianSessionContext
                | Metric::TopProjectShare
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Lt,
    Gt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub id: String,
    pub metric: Metric,
    pub op: Op,
    pub threshold: Num,
    pub label: String,
    pub priority: i64,
    pub window_days: i64,
    pub min_history_days: i64,
    pub title: String,
    pub why: String,
    pub steps: Vec<String>,
}

/// The pack shipped with the Python app (`src/easy_personal/data/rules.yaml`).
pub fn shipped_rules() -> Result<Vec<Rule>> {
    parse_rules(SHIPPED)
}

/// `rules:` followed by indented `id: key=value ... title=... why=... steps=a|b` lines. Blank lines and `#`
/// comments are skipped; lines outside `rules:` are ignored; a repeated id keeps its first position and its last
/// value (a Python dict). A bad line is an error naming its 1-based number, never its text. Unlike Python, a
/// `key=` that is not one of the seven known keys is also an error.
pub fn parse_rules(text: &str) -> Result<Vec<Rule>> {
    let mut found: Vec<(String, String, usize)> = Vec::new();
    let mut inside = false;
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if !line.starts_with(' ') {
            inside = line.split(':').next().unwrap_or("") == "rules";
        } else if inside {
            let (name, value) = line.trim().split_once(':').unwrap_or((line.trim(), ""));
            let entry = (name.to_string(), value.trim().to_string(), n + 1);
            match found.iter_mut().find(|f| f.0 == name) {
                Some(old) => *old = entry,
                None => found.push(entry),
            }
        }
    }
    found
        .iter()
        .map(|(id, value, line)| parse_rule(id, value, *line))
        .collect()
}

fn partition<'a>(text: &'a str, sep: &str) -> (&'a str, &'a str) {
    text.split_once(sep).unwrap_or((text, ""))
}

fn parse_rule(id: &str, text: &str, line: usize) -> Result<Rule> {
    let bad = |reason: &'static str| RulesError::Parse { line, reason };
    let (head, rest) = partition(text, " title=");
    let (title, rest) = partition(rest, " why=");
    let (why, steps) = partition(rest, " steps=");
    let steps: Vec<String> = steps
        .split('|')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    if title.trim().is_empty() {
        return Err(bad("missing title"));
    }
    if why.trim().is_empty() {
        return Err(bad("missing why"));
    }
    if steps.is_empty() {
        return Err(bad("missing steps"));
    }
    if steps.len() > 3 {
        return Err(bad("steps must be 1 to 3"));
    }
    let mut fields: HashMap<&str, &str> = HashMap::new();
    for item in head.split_whitespace() {
        let (key, value) = item.split_once('=').ok_or(bad("expected key=value"))?;
        if ![
            "metric",
            "op",
            "threshold",
            "label",
            "priority",
            "window_days",
            "min_history_days",
        ]
        .contains(&key)
        {
            return Err(bad("unknown key"));
        }
        fields.insert(key, value);
    }
    let field = |key: &str| fields.get(key).copied().ok_or(bad("missing key"));
    let integer = |key: &str| {
        field(key)?
            .parse::<i64>()
            .map_err(|_| bad("not an integer"))
    };
    let op = match field("op")? {
        "lt" => Op::Lt,
        "gt" => Op::Gt,
        _ => return Err(bad("bad op")),
    };
    let label = field("label")?;
    if !["improve", "aware", "warning"].contains(&label) {
        return Err(bad("bad label"));
    }
    Ok(Rule {
        id: id.to_string(),
        metric: Metric::parse(field("metric")?).ok_or(bad("bad metric"))?,
        op,
        threshold: Num::from_text(field("threshold")?).ok_or(bad("threshold is not a number"))?,
        label: label.to_string(),
        priority: integer("priority")?,
        window_days: integer("window_days")?,
        min_history_days: integer("min_history_days")?,
        title: title.trim().to_string(),
        why: why.trim().to_string(),
        steps,
    })
}

// ---- text helpers (Python's `format` specs `,.Nf` and `,g`, `_percent`, `_short`) ----

fn group_thousands(digits: &str) -> String {
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// `format(x, ",.{decimals}f")`.
fn fixed(x: f64, decimals: usize) -> String {
    let text = format!("{x:.decimals$}");
    let (sign, body) = text.strip_prefix('-').map_or(("", &text[..]), |b| ("-", b));
    let (int, frac) = body.split_once('.').unwrap_or((body, ""));
    let dot = if frac.is_empty() { "" } else { "." };
    format!("{sign}{}{dot}{frac}", group_thousands(int))
}

/// `format(x, ",g")`: six significant digits, no trailing zeros.
fn general(x: f64) -> String {
    if x == 0.0 {
        return "0".into();
    }
    let sci = format!("{x:.5e}"); // d.ddddde<exp>, rounded to six digits
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let strip = |s: &str| {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s.to_string()
        }
    };
    if (-4..6).contains(&exp) {
        let text = strip(&fixed(x, (5 - exp).max(0) as usize));
        return text;
    }
    format!(
        "{}e{}{:02}",
        strip(mantissa),
        if exp < 0 { '-' } else { '+' },
        exp.abs()
    )
}

fn percent(value: f64) -> String {
    let text = fixed(value * 100.0, 1);
    format!("{}%", text.trim_end_matches('0').trim_end_matches('.'))
}

fn short(n: f64) -> String {
    for (unit, size) in [("B", 1e9), ("M", 1e6), ("k", 1e3)] {
        if n >= size {
            return format!("{}{unit}", fixed(n / size, 1));
        }
    }
    fixed(n, 0)
}

fn r4(x: f64) -> f64 {
    round_py(x, 4)
}

/// Python's `Path(p).name`: the last component, `""` when there is none.
fn path_name(path: &str) -> &str {
    path.rsplit('/')
        .find(|c| !c.is_empty() && *c != ".")
        .unwrap_or("")
}

/// `_folder`: the last path component, else the path itself; `(unknown)` for no path.
fn folder_name(path: Option<&str>) -> String {
    match path.filter(|p| !p.is_empty()) {
        None => "(unknown)".into(),
        Some(p) => match path_name(p) {
            "" => p.to_string(),
            name => name.to_string(),
        },
    }
}

// ---- time ----

/// `YYYY-MM-DDTHH:MM:SS` of a UTC instant and its microsecond part.
fn civil_text(micros: i64) -> (String, i64) {
    let days = micros.div_euclid(DAY);
    let in_day = micros.rem_euclid(DAY);
    let (y, m, d) = civil_from_days(days);
    let secs = in_day / 1_000_000;
    (
        format!(
            "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}",
            secs / 3600,
            secs % 3600 / 60,
            secs % 60
        ),
        in_day % 1_000_000,
    )
}

/// The stored form `%Y-%m-%dT%H:%M:%S.%fZ` (what `ts_utc` holds, so a text comparison is a time comparison).
fn stored(micros: i64) -> String {
    let (text, frac) = civil_text(micros);
    format!("{text}.{frac:06}Z")
}

/// Python's `datetime.isoformat()` of a UTC time: microseconds only when not zero.
fn iso(micros: i64) -> String {
    let (text, frac) = civil_text(micros);
    if frac == 0 {
        format!("{text}+00:00")
    } else {
        format!("{text}.{frac:06}+00:00")
    }
}

// ---- store reads (SQL as in metrics.py / kit.py / context.py; the scope clause is `scope.rs`) ----

/// The scope clause for the events of [since, until) (microseconds), with its bound values (`metrics._scope`).
fn scope_clause(conn: &Connection, sc: &Resolved, since: i64, until: i64) -> Result<Clause> {
    Ok(sc.clause(conn, Some(&stored(since)), Some(&stored(until)))?)
}

/// The window bounds as bound values.
fn window(since: i64, until: i64) -> Vec<Bound> {
    vec![Bound::Text(stored(since)), Bound::Text(stored(until))]
}

/// UTC time of the earliest stored event; `None` for an empty store.
fn history_start(conn: &Connection) -> Result<Option<String>> {
    Ok(conn.query_row("SELECT MIN(ts_utc) FROM events", [], |r| r.get(0))?)
}

struct SessionTokens {
    session_id: Option<String>,
    project_path: Option<String>,
    input: i64,
    output: i64,
    cache_creation: i64,
    cache_read: i64,
    models: i64,
}

/// Token sums of the messages in [since, until) per (session, stored project path).
fn session_tokens(
    conn: &Connection,
    sc: &Resolved,
    since: i64,
    until: i64,
) -> Result<Vec<SessionTokens>> {
    let clause = scope_clause(conn, sc, since, until)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT sid, project_path, SUM(i), SUM(o), SUM(cc), SUM(cr), COUNT(DISTINCT model) FROM (
           SELECT MIN(ts_utc) AS ts, MAX(session_id) AS sid, MAX(project_path) AS project_path,
                  MAX(model) AS model, MAX(input_tokens) AS i, MAX(output_tokens) AS o,
                  MAX(cache_creation_input_tokens) AS cc, MAX(cache_read_input_tokens) AS cr
           FROM events WHERE abstained = 0 AND message_id IS NOT NULL
           AND input_tokens IS NOT NULL AND ts_utc >= ? AND ts_utc < ? AND {} GROUP BY message_id
         ) WHERE ts >= ? AND ts < ? GROUP BY sid, project_path",
        clause.sql
    ))?;
    let rows = stmt.query_map(
        params_from_iter(
            window(since - DAY, until + DAY)
                .into_iter()
                .chain(clause.params)
                .chain(window(since, until)),
        ),
        |r| {
            Ok(SessionTokens {
                session_id: r.get(0)?,
                project_path: r.get(1)?,
                input: r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                output: r.get::<_, Option<i64>>(3)?.unwrap_or(0),
                cache_creation: r.get::<_, Option<i64>>(4)?.unwrap_or(0),
                cache_read: r.get::<_, Option<i64>>(5)?.unwrap_or(0),
                models: r.get(6)?,
            })
        },
    )?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

struct SessionContext {
    session_id: String,
    peak: i64,
    compacted: bool,
}

/// Per session: peak context (prompt tokens of one message) and whether it was compacted (a later message below
/// half of the earlier peak).
fn session_contexts(
    conn: &Connection,
    sc: &Resolved,
    since: i64,
    until: i64,
) -> Result<Vec<SessionContext>> {
    let clause = scope_clause(conn, sc, since, until)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT session_id, ts, ctx FROM (
           SELECT MAX(session_id) AS session_id, MIN(ts_utc) AS ts,
                  MAX(COALESCE(input_tokens, 0) + COALESCE(cache_creation_input_tokens, 0)
                      + COALESCE(cache_read_input_tokens, 0)) AS ctx
           FROM events WHERE abstained = 0 AND session_id IS NOT NULL
           AND message_id IS NOT NULL AND input_tokens IS NOT NULL AND ts_utc >= ? AND ts_utc < ? AND {}
           GROUP BY message_id
         ) WHERE ts >= ? AND ts < ? ORDER BY session_id, ts",
        clause.sql
    ))?;
    let rows = stmt.query_map(
        params_from_iter(
            window(since - DAY, until + DAY)
                .into_iter()
                .chain(clause.params)
                .chain(window(since, until)),
        ),
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(2)?)),
    )?;
    let mut sessions: Vec<SessionContext> = Vec::new();
    for row in rows {
        let (session_id, context) = row?;
        if sessions.last().is_none_or(|s| s.session_id != session_id) {
            sessions.push(SessionContext {
                session_id,
                peak: 0,
                compacted: false,
            });
        }
        let state = sessions.last_mut().expect("just pushed");
        if context * 2 < state.peak {
            state.compacted = true;
        }
        state.peak = state.peak.max(context);
    }
    Ok(sessions)
}

/// Source version -> (abstained records, all records) with a timestamp in [since, until).
fn record_readability(
    conn: &Connection,
    sc: &Resolved,
    since: i64,
    until: i64,
) -> Result<Vec<(String, i64, i64)>> {
    let clause = scope_clause(conn, sc, since, until)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT COALESCE(version, 'unknown'), SUM(abstained), COUNT(*) FROM events
         WHERE ts_utc >= ? AND ts_utc < ? AND {} GROUP BY 1 ORDER BY 1",
        clause.sql
    ))?;
    let rows = stmt.query_map(
        params_from_iter(window(since, until).into_iter().chain(clause.params)),
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// (record type, source version, abstained records) in [since, until), most records first.
fn unreadable_records(
    conn: &Connection,
    sc: &Resolved,
    since: i64,
    until: i64,
) -> Result<Vec<(Option<String>, String, i64)>> {
    let clause = scope_clause(conn, sc, since, until)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT record_type, COALESCE(version, 'unknown'), COUNT(*) FROM events
         WHERE abstained = 1 AND ts_utc >= ? AND ts_utc < ? AND {} GROUP BY 1, 2 ORDER BY 3 DESC, 1, 2",
        clause.sql
    ))?;
    let rows = stmt.query_map(
        params_from_iter(window(since, until).into_iter().chain(clause.params)),
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// `kit._trace_window`: the WHERE clause and values for the kernel-trace rows of [since, until) in scope; `None` = the
/// scope has no project. Trace rows carry no account and keep their own stored path, so a scope limits them to the
/// stored projects of the scope's usage events.
fn trace_window(
    conn: &Connection,
    sc: &Resolved,
    since: i64,
    until: i64,
) -> Result<Option<(String, Vec<Bound>)>> {
    let mut clause = "abstained = 0 AND ts_utc >= ? AND ts_utc < ?".to_string();
    let mut values = window(since, until);
    if !sc.is_empty() {
        let paths = sc.stored_projects(conn)?;
        if paths.is_empty() {
            return Ok(None);
        }
        clause.push_str(&format!(
            " AND project_path IN ({})",
            vec!["?"; paths.len()].join(", ")
        ));
        values.extend(paths.into_iter().map(Bound::Text));
    }
    Ok(Some((clause, values)))
}

/// (share of tool calls with `is_error`, number of calls) of the kernel traces in [since, until).
fn kernel_error_rate(
    conn: &Connection,
    sc: &Resolved,
    since: i64,
    until: i64,
) -> Result<(f64, i64)> {
    let Some((clause, values)) = trace_window(conn, sc, since, until)? else {
        return Ok((0.0, 0));
    };
    let (calls, errors): (i64, i64) = conn.query_row(
        &format!(
            "SELECT COUNT(*), COALESCE(SUM(is_error), 0) FROM kit_trace_events WHERE {clause}"
        ),
        params_from_iter(&values),
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok((
        if calls == 0 {
            0.0
        } else {
            errors as f64 / calls as f64
        },
        calls,
    ))
}

/// (tool name, stored project path, calls, failed calls).
type ToolErrors = (Option<String>, String, i64, i64);

/// The kernel-trace tool calls in [since, until) per tool and stored project path.
fn kernel_errors_by_tool(
    conn: &Connection,
    sc: &Resolved,
    since: i64,
    until: i64,
) -> Result<Vec<ToolErrors>> {
    let Some((clause, values)) = trace_window(conn, sc, since, until)? else {
        return Ok(Vec::new());
    };
    let mut stmt = conn.prepare(&format!(
        "SELECT tool_name, project_path, COUNT(*), COALESCE(SUM(is_error), 0)
         FROM kit_trace_events WHERE {clause} GROUP BY 1, 2"
    ))?;
    let rows = stmt.query_map(params_from_iter(&values), |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// True when the stored events in [since, until) hold a call to a tool of easy-context (FR-080).
pub(crate) fn easy_context_in_use(
    conn: &Connection,
    sc: &Resolved,
    since: i64,
    until: i64,
) -> Result<bool> {
    let clause = scope_clause(conn, sc, since, until)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT 1 FROM events WHERE abstained = 0 AND ts_utc >= ? AND ts_utc < ? AND {}
         AND (tool_names GLOB '{TOOL_PREFIX}*' OR tool_names GLOB '*,{TOOL_PREFIX}*') LIMIT 1",
        clause.sql
    ))?;
    Ok(stmt.exists(params_from_iter(
        window(since, until).into_iter().chain(clause.params),
    ))?)
}

pub(crate) struct Freshness {
    pub(crate) last_index_at: Option<String>,
    pub(crate) latest_commit_utc: Option<String>,
    pub(crate) stale: Option<bool>,
}

/// Epoch of the newest commit on any local ref; `None` for no repo, no commits or a slow git (10 s).
fn latest_commit(project: &str) -> Option<i64> {
    let out = run_git_within(
        project,
        &[
            "log",
            "--branches",
            "--tags",
            "--remotes",
            "-1",
            "--format=%ct",
        ],
        Duration::from_secs(10),
    )?;
    out.split_whitespace().next()?.parse().ok()
}

/// `None` = no index row. Else the index time, the latest commit time and whether the index is stale
/// (`None` when it cannot be judged).
pub(crate) fn freshness(
    conn: &Connection,
    project: &str,
    now_micros: i64,
) -> Result<Option<Freshness>> {
    let mut stmt =
        conn.prepare("SELECT last_index_at_utc FROM context_index WHERE project_path = ?1")?;
    let mut rows = stmt.query(params![project])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let last_index_at: Option<String> = row.get(0)?;
    let indexed = last_index_at
        .as_deref()
        .and_then(parse_iso)
        .map(|m| m as f64 / SECOND);
    let commit = latest_commit(project);
    let now = now_micros as f64 / SECOND;
    let stale = match (indexed, commit) {
        (Some(indexed), Some(commit)) if indexed <= now + SKEW_S => {
            Some(commit as f64 - indexed > SKEW_S)
        }
        _ => None,
    };
    Ok(Some(Freshness {
        last_index_at,
        latest_commit_utc: commit.map(|c| iso(c * 1_000_000)),
        stale,
    }))
}

// ---- token windows (rules.py `_Windows`) ----

#[derive(Default)]
struct Project {
    tokens: i64,
    prompt: i64,
    cache_read: i64,
    sessions: BTreeSet<String>,
}

struct Session {
    id: String,
    tokens: i64,
    prompt: i64,
    cache_read: i64,
    models: i64,
    /// The project folder that holds most of the session's tokens.
    project: String,
}

#[derive(Default)]
struct Window {
    projects: BTreeMap<String, Project>,
    sessions: BTreeMap<String, Session>,
    tokens: i64,
    prompt: i64,
    cache_read: i64,
}

/// Windows built once each per `evaluate` (one scan per window, shared by the rule and its causes).
struct Windows<'a> {
    conn: &'a Connection,
    sc: &'a Resolved,
    pmap: Option<HashMap<String, String>>,
    built: HashMap<(i64, i64), Rc<Window>>,
}

impl<'a> Windows<'a> {
    fn new(conn: &'a Connection, sc: &'a Resolved) -> Self {
        Windows {
            conn,
            sc,
            pmap: None,
            built: HashMap::new(),
        }
    }

    /// The project folder name of a stored project path (owner project or scratch first, then the last component).
    fn folder(&mut self, path: Option<&str>) -> Result<String> {
        let Some(path) = path else {
            return Ok(folder_name(None));
        };
        if self.pmap.is_none() {
            self.pmap = Some(project_map(self.conn)?);
        }
        let key = self
            .pmap
            .as_ref()
            .and_then(|m| m.get(path))
            .map_or(path, String::as_str);
        Ok(folder_name(Some(key)))
    }

    fn get(&mut self, since: i64, until: i64) -> Result<Rc<Window>> {
        if let Some(window) = self.built.get(&(since, until)) {
            return Ok(Rc::clone(window));
        }
        let mut window = Window::default();
        let mut tops: HashMap<String, (i64, String)> = HashMap::new();
        for row in session_tokens(self.conn, self.sc, since, until)? {
            let tokens = row.input + row.output + row.cache_creation + row.cache_read;
            let prompt = row.input + row.cache_creation + row.cache_read;
            let folder = self.folder(row.project_path.as_deref())?;
            window.tokens += tokens;
            window.prompt += prompt;
            window.cache_read += row.cache_read;
            let project = window.projects.entry(folder.clone()).or_default();
            project.tokens += tokens;
            project.prompt += prompt;
            project.cache_read += row.cache_read;
            let Some(id) = row.session_id else { continue };
            project.sessions.insert(id.clone());
            let session = window
                .sessions
                .entry(id.clone())
                .or_insert_with(|| Session {
                    id: id.clone(),
                    tokens: 0,
                    prompt: 0,
                    cache_read: 0,
                    models: 0,
                    project: String::new(),
                });
            session.tokens += tokens;
            session.prompt += prompt;
            session.cache_read += row.cache_read;
            session.models = session.models.max(row.models);
            let top = tops.entry(id).or_insert((-1, String::new()));
            *top = (*top).clone().max((tokens, folder));
        }
        for (id, (_, folder)) in tops {
            if let Some(session) = window.sessions.get_mut(&id) {
                session.project = folder;
            }
        }
        let window = Rc::new(window);
        self.built.insert((since, until), Rc::clone(&window));
        Ok(window)
    }
}

fn reuse(cache_read: i64, prompt: i64) -> Option<f64> {
    (prompt != 0).then(|| r4(cache_read as f64 / prompt as f64))
}

/// The three biggest sessions (tokens, then id), optionally only those of one project.
fn biggest_sessions<'w>(window: &'w Window, within: Option<&str>) -> Vec<&'w Session> {
    let mut found: Vec<&Session> = window
        .sessions
        .values()
        .filter(|s| within.is_none_or(|w| s.project == w))
        .collect();
    found.sort_by(|a, b| b.tokens.cmp(&a.tokens).then_with(|| a.id.cmp(&b.id)));
    found.truncate(CAUSE_LIMIT);
    found
}

fn session_cause(session: &Session, window: &Window) -> Value {
    json!({
        "name": session.id,
        "kind": "session",
        "project": session.project,
        "tokens": session.tokens,
        "token_share": if window.tokens != 0 { r4(session.tokens as f64 / window.tokens as f64) } else { 0.0 },
        "text": format!("{} in {} used {} tokens", session.id, session.project, short(session.tokens as f64)),
    })
}

// ---- measuring ----

type Detail = Option<Map<String, Value>>;

enum Outcome {
    Measured(Num, Detail),
    /// The metric cannot be measured; the text is the reason shown to the owner.
    Abstain(String),
}

fn abstain(reason: &str) -> Result<Outcome> {
    Ok(Outcome::Abstain(reason.to_string()))
}

fn detail(key: &str, value: Value) -> Detail {
    let mut map = Map::new();
    map.insert(key.to_string(), value);
    Some(map)
}

/// `statistics.median` of ints: an int for an odd count, a float (the mean of the middle two) for an even one.
fn median(mut values: Vec<i64>) -> Num {
    values.sort_unstable();
    let n = values.len();
    if n % 2 == 1 {
        Num::Int(values[n / 2])
    } else {
        Num::Float((values[n / 2 - 1] + values[n / 2]) as f64 / 2.0)
    }
}

fn measure(
    rule: &Rule,
    conn: &Connection,
    sc: &Resolved,
    now: i64,
    win: &mut Windows,
) -> Result<Outcome> {
    let Some(start) = history_start(conn)? else {
        return abstain("no stored history");
    };
    if parse_iso(&start).ok_or(RulesError::Time)? > now - rule.min_history_days * DAY {
        return abstain(&format!(
            "less than {} days of stored history",
            rule.min_history_days
        ));
    }
    let (since, until) = (now - rule.window_days * DAY, now);
    match rule.metric {
        Metric::CacheHitRatio => {
            let w = win.get(since, until)?;
            match reuse(w.cache_read, w.prompt) {
                Some(ratio) => Ok(Outcome::Measured(Num::Float(ratio), None)),
                None => abstain("no prompt tokens in the window"),
            }
        }
        Metric::MedianSessionContext => {
            let sessions = session_contexts(conn, sc, since, until)?;
            if sessions.is_empty() {
                return abstain("no sessions in the window");
            }
            let open: Vec<i64> = sessions
                .iter()
                .filter(|s| !s.compacted)
                .map(|s| s.peak)
                .collect();
            let value = if open.is_empty() {
                Num::Int(0)
            } else {
                median(open)
            };
            Ok(Outcome::Measured(value, None))
        }
        Metric::UnreadableShare => {
            let mut worst: Option<(String, f64)> = None; // versions come sorted; ties keep the first
            for (version, bad, total) in record_readability(conn, sc, since, until)? {
                if total == 0 {
                    continue;
                }
                let share = r4(bad as f64 / total as f64);
                if worst.as_ref().is_none_or(|(_, best)| share > *best) {
                    worst = Some((version, share));
                }
            }
            match worst {
                None => abstain("no records in the window"),
                Some((version, share)) => Ok(Outcome::Measured(
                    Num::Float(share),
                    detail("version", json!(version)),
                )),
            }
        }
        Metric::TopProjectShare => {
            let w = win.get(since, until)?;
            let whole: i64 = w.projects.values().map(|p| p.tokens).sum();
            if w.projects.len() < 2 || whole == 0 {
                return abstain("fewer than two projects with tokens in the window");
            }
            // most tokens; ties: first folder name
            let (top, project) = w
                .projects
                .iter()
                .min_by(|a, b| b.1.tokens.cmp(&a.1.tokens).then_with(|| a.0.cmp(b.0)))
                .expect("two or more projects");
            Ok(Outcome::Measured(
                Num::Float(r4(project.tokens as f64 / whole as f64)),
                detail("project", json!(top)),
            ))
        }
        Metric::WeekOverWeekTokens => {
            let previous = win.get(since - (until - since), since)?.tokens;
            if previous == 0 {
                return abstain("no tokens in the previous window");
            }
            let current = win.get(since, until)?.tokens;
            Ok(Outcome::Measured(
                Num::Float(r4(current as f64 / previous as f64 - 1.0)),
                None,
            ))
        }
        Metric::KitErrorRate => {
            let (rate, calls) = kernel_error_rate(conn, sc, since, until)?;
            if calls == 0 {
                return abstain("no kernel-trace tool calls in the window");
            }
            Ok(Outcome::Measured(Num::Float(r4(rate)), None))
        }
        Metric::ContextStaleIndexes => {
            // `kit.scoped_projects`: with a scope only the indexes of the scope's projects count (`stale_projects`)
            let mut paths = context_paths(conn)?;
            if !sc.is_empty() {
                let within = sc.scoped_projects(conn)?;
                paths.retain(|p| within.contains(p));
            }
            if paths.is_empty() {
                return abstain(
                    "no easy-context data (service down or no index for a known project)",
                );
            }
            let mut names = Vec::new();
            for path in &paths {
                let found = freshness(conn, path, now)?;
                if found.is_some_and(|f| f.stale == Some(true)) {
                    names.push(name_or_path(path));
                }
            }
            Ok(Outcome::Measured(
                Num::Int(names.len() as i64),
                detail("projects", json!(names)),
            ))
        }
    }
}

fn context_paths(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT project_path FROM context_index ORDER BY 1")?;
    let rows = stmt.query_map([], |r| r.get(0))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

fn name_or_path(path: &str) -> String {
    match path_name(path) {
        "" => path.to_string(),
        name => name.to_string(),
    }
}

// ---- the one-sentence summary ----

fn summary(rule: &Rule, measured: f64, detail: &Detail) -> String {
    let days = rule.window_days;
    let threshold = rule.threshold.as_f64();
    let mut target = format!(
        "{} {}",
        percent(threshold),
        if rule.op == Op::Lt {
            "or more"
        } else {
            "or less"
        }
    );
    let text = |key: &str| {
        detail
            .as_ref()
            .and_then(|d| d.get(key))
            .and_then(Value::as_str)
    };
    let sentence = match rule.metric {
        Metric::CacheHitRatio => format!("{} of prompt tokens came from cache", percent(measured)),
        Metric::MedianSessionContext => {
            target = format!("{} or fewer", fixed(threshold, 0));
            format!(
                "The median open session reached {} tokens of context",
                fixed(measured, 0)
            )
        }
        Metric::UnreadableShare => {
            let version = text("version").map_or(String::new(), |v| format!(" {v}"));
            format!(
                "{} of records from Claude Code{version} could not be read",
                percent(measured)
            )
        }
        Metric::WeekOverWeekTokens => {
            let way = if measured >= 0.0 { "rose" } else { "fell" };
            return format!(
                "Token use {way} {} in the last {days} days against the {days} days before; target {target}",
                percent(measured.abs())
            );
        }
        Metric::TopProjectShare => format!(
            "One project ({}) used {} of tokens",
            text("project").unwrap_or("?"),
            percent(measured)
        ),
        Metric::KitErrorRate => format!("{} of kernel-trace tool calls failed", percent(measured)),
        Metric::ContextStaleIndexes => {
            let n = measured as i64;
            target = if threshold == 0.0 {
                "none".into()
            } else {
                format!("{} or fewer", fixed(threshold, 0))
            };
            format!(
                "{n} project{} a stale easy-context index",
                if n == 1 { " has" } else { "s have" }
            )
        }
    };
    format!("{sentence}; target {target}, last {days} days")
}

// ---- causes and diagnosis (FR-073) ----

/// What a hit reports beyond its measure: the `diagnosis` that picks its fix lines, at most three `causes`, and
/// the `facts` that fill the fix lines. Ids, folder names, tool names and numbers only; never message content.
struct Explained {
    diagnosis: &'static str,
    causes: Vec<Value>,
    facts: HashMap<&'static str, String>,
}

fn explain_week_over_week(since: i64, until: i64, win: &mut Windows) -> Result<Explained> {
    let before = win.get(since - (until - since), since)?;
    let now = win.get(since, until)?;
    let (n_before, n_now) = (before.sessions.len(), now.sessions.len());
    let per_session = |w: &Window, n: usize| {
        if n == 0 {
            0.0
        } else {
            w.tokens as f64 / n as f64
        }
    };
    let (tps_before, tps_now) = (per_session(&before, n_before), per_session(&now, n_now));
    let mut facts = HashMap::from([
        ("sessions_before", n_before.to_string()),
        ("sessions_now", n_now.to_string()),
        ("tps_before", short(tps_before)),
        ("tps_now", short(tps_now)),
    ]);
    if tps_before == 0.0 || tps_now > tps_before * (1.0 + BIGGER_SESSIONS_FROM) {
        let biggest = biggest_sessions(&now, None);
        if let Some(top) = biggest.first() {
            facts.insert("top_session", top.id.clone());
            facts.insert("top_session_project", top.project.clone());
            facts.insert("top_session_tokens", short(top.tokens as f64));
        }
        return Ok(Explained {
            diagnosis: "bigger_sessions",
            causes: biggest.iter().map(|s| session_cause(s, &now)).collect(),
            facts,
        });
    }
    let mut rises: Vec<(i64, &String, Option<&Project>, &Project)> = Vec::new();
    for (name, is_) in &now.projects {
        let was = before.projects.get(name);
        let rise = is_.tokens - was.map_or(0, |p| p.tokens);
        if rise > 0 {
            // a project that fell, or left, is never a cause of the rise
            rises.push((rise, name, was, is_));
        }
    }
    rises.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    let mut causes = Vec::new();
    let mut new_names = Vec::new();
    for (_, name, was, is_) in rises.iter().take(CAUSE_LIMIT) {
        let tokens_before = was.map_or(0, |p| p.tokens);
        let sessions_before = was.map_or(0, |p| p.sessions.len());
        let sessions_now = is_.sessions.len();
        let new = tokens_before == 0;
        if new {
            new_names.push(name.as_str());
        }
        causes.push(json!({
            "name": name,
            "kind": "project",
            "tokens_before": tokens_before,
            "tokens_now": is_.tokens,
            "sessions_before": sessions_before,
            "sessions_now": sessions_now,
            "new": new,
            "text": format!(
                "{name}: {} \u{2192} {} tokens, {sessions_before} \u{2192} {sessions_now} sessions{}",
                short(tokens_before as f64),
                short(is_.tokens as f64),
                if new { " (new)" } else { "" }
            ),
        }));
    }
    if let Some((rise, name, _, _)) = rises.first() {
        facts.insert("top", (*name).clone());
        facts.insert("top_rise", short(*rise as f64));
    }
    if !new_names.is_empty() {
        facts.insert("new", new_names.join(", "));
    }
    Ok(Explained {
        diagnosis: "more_sessions",
        causes,
        facts,
    })
}

fn explain_cache_hit(since: i64, until: i64, win: &mut Windows) -> Result<Explained> {
    let now = win.get(since, until)?;
    let mut low: Vec<(f64, &String, &Project)> = now
        .projects
        .iter()
        .filter(|(_, p)| p.prompt != 0 && p.tokens as f64 >= now.tokens as f64 * MIN_PROJECT_SHARE)
        .filter_map(|(name, p)| reuse(p.cache_read, p.prompt).map(|r| (r, name, p)))
        .collect();
    low.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(b.1)));
    let project_causes: Vec<Value> = low
        .iter()
        .map(|(reuse, name, p)| {
            let share = p.tokens as f64 / now.tokens as f64;
            json!({
                "name": name,
                "kind": "project",
                "cache_reuse": reuse,
                "tokens": p.tokens,
                "token_share": r4(share),
                "text": format!(
                    "{name} read {} of its prompt tokens from cache ({} of all tokens)",
                    percent(*reuse),
                    percent(share)
                ),
            })
        })
        .collect();
    let mut facts = HashMap::new();
    if let Some((reuse, name, _)) = low.first() {
        facts.insert("low_project", (*name).clone());
        facts.insert("low_reuse", percent(*reuse));
    }
    let sessions: Vec<&Session> = now.sessions.values().filter(|s| s.prompt != 0).collect();
    let multi: Vec<&Session> = sessions.iter().copied().filter(|s| s.models >= 2).collect();
    let single: Vec<&Session> = sessions.iter().copied().filter(|s| s.models < 2).collect();
    let sum_reuse = |group: &[&Session]| {
        reuse(
            group.iter().map(|s| s.cache_read).sum(),
            group.iter().map(|s| s.prompt).sum(),
        )
    };
    let (multi_reuse, single_reuse) = (sum_reuse(&multi), sum_reuse(&single));
    if let (Some(multi_reuse), Some(single_reuse)) = (multi_reuse, single_reuse)
        && multi_reuse < single_reuse
    {
        // the sessions with the most prompt tokens that did not come from cache
        let mut worst = multi.clone();
        worst.sort_by(|a, b| {
            (a.cache_read - a.prompt)
                .cmp(&(b.cache_read - b.prompt))
                .then_with(|| a.id.cmp(&b.id))
        });
        worst.truncate(2);
        let mut causes: Vec<Value> = worst
            .iter()
            .map(|s| {
                let own = reuse(s.cache_read, s.prompt).unwrap_or(0.0);
                json!({
                    "name": s.id,
                    "kind": "session",
                    "project": s.project,
                    "models": s.models,
                    "cache_reuse": own,
                    "tokens": s.tokens,
                    "text": format!(
                        "{} in {} used {} models and read {} of its prompt tokens from cache",
                        s.id, s.project, s.models, percent(own)
                    ),
                })
            })
            .collect();
        causes.extend(project_causes);
        causes.truncate(CAUSE_LIMIT);
        facts.insert("multi_count", multi.len().to_string());
        facts.insert("multi_reuse", percent(multi_reuse));
        facts.insert("single_reuse", percent(single_reuse));
        facts.insert("top_session", worst[0].id.clone());
        facts.insert("top_session_project", worst[0].project.clone());
        return Ok(Explained {
            diagnosis: "model_switching",
            causes,
            facts,
        });
    }
    Ok(Explained {
        diagnosis: "unstable_prefix",
        causes: project_causes.into_iter().take(CAUSE_LIMIT).collect(),
        facts,
    })
}

fn explain_median_context(
    conn: &Connection,
    sc: &Resolved,
    since: i64,
    until: i64,
    win: &mut Windows,
) -> Result<Explained> {
    let now = win.get(since, until)?;
    let mut open: Vec<SessionContext> = session_contexts(conn, sc, since, until)?
        .into_iter()
        .filter(|s| !s.compacted)
        .collect();
    open.sort_by(|a, b| {
        b.peak
            .cmp(&a.peak)
            .then_with(|| a.session_id.cmp(&b.session_id))
    });
    open.truncate(CAUSE_LIMIT);
    let mut causes = Vec::new();
    let mut facts = HashMap::new();
    for s in &open {
        let project = now
            .sessions
            .get(&s.session_id)
            .map_or("(unknown)", |found| found.project.as_str());
        if causes.is_empty() {
            facts.insert("top_session", s.session_id.clone());
            facts.insert("top_session_project", project.to_string());
            facts.insert("top_context", short(s.peak as f64));
        }
        causes.push(json!({
            "name": s.session_id,
            "kind": "session",
            "project": project,
            "context_tokens": s.peak,
            "text": format!(
                "{} in {project} reached {} tokens of context without compacting",
                s.session_id,
                short(s.peak as f64)
            ),
        }));
    }
    Ok(Explained {
        diagnosis: "huge_sessions",
        causes,
        facts,
    })
}

fn explain_top_project(
    since: i64,
    until: i64,
    measured: f64,
    detail: &Detail,
    win: &mut Windows,
) -> Result<Explained> {
    let now = win.get(since, until)?;
    let project = detail
        .as_ref()
        .and_then(|d| d.get("project"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let biggest = biggest_sessions(&now, Some(project));
    let mut facts = HashMap::from([
        ("project", project.to_string()),
        ("share", percent(measured)),
    ]);
    if let Some(top) = biggest.first() {
        facts.insert("top_session", top.id.clone());
        facts.insert(
            "top_session_share",
            percent(top.tokens as f64 / now.tokens as f64),
        );
    }
    Ok(Explained {
        diagnosis: "dominant_project",
        causes: biggest.iter().map(|s| session_cause(s, &now)).collect(),
        facts,
    })
}

fn explain_kit_errors(
    conn: &Connection,
    sc: &Resolved,
    since: i64,
    until: i64,
    win: &mut Windows,
) -> Result<Explained> {
    let mut by_tool: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    let mut by_project: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    for (tool, path, calls, errors) in kernel_errors_by_tool(conn, sc, since, until)? {
        let tool = tool
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "(unknown)".into());
        let project = win.folder(Some(&path))?;
        for (table, key) in [(&mut by_tool, tool), (&mut by_project, project)] {
            let totals = table.entry(key).or_default();
            totals.0 += calls;
            totals.1 += errors;
        }
    }
    let total_errors: i64 = by_tool.values().map(|t| t.1).sum();
    let ranked = |table: &BTreeMap<String, (i64, i64)>| {
        let mut rows: Vec<(String, i64, i64)> = table
            .iter()
            .filter(|(_, t)| t.1 != 0)
            .map(|(n, t)| (n.clone(), t.0, t.1))
            .collect();
        rows.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
        rows
    };
    let tools = ranked(&by_tool);
    let one_tool = tools.first().is_some_and(|t| t.2 * 2 > total_errors);
    let (kind, rows) = if one_tool {
        ("tool", tools)
    } else {
        ("project", ranked(&by_project))
    };
    let causes: Vec<Value> = rows
        .iter()
        .take(CAUSE_LIMIT)
        .map(|(name, calls, errors)| {
            let rate = *errors as f64 / *calls as f64;
            json!({
                "name": name,
                "kind": kind,
                "calls": calls,
                "errors": errors,
                "error_rate": r4(rate),
                "text": format!("{name}: {errors} of {calls} calls failed ({})", percent(rate)),
            })
        })
        .collect();
    let mut facts = HashMap::from([("total_errors", total_errors.to_string())]);
    if let Some((name, _, errors)) = rows.first() {
        facts.insert("top", name.clone());
        facts.insert("top_errors", errors.to_string());
        facts.insert("top_share", percent(*errors as f64 / total_errors as f64));
    }
    Ok(Explained {
        diagnosis: if one_tool {
            "one_tool_fails"
        } else {
            "errors_spread"
        },
        causes,
        facts,
    })
}

fn explain_unreadable(
    conn: &Connection,
    sc: &Resolved,
    since: i64,
    until: i64,
) -> Result<Explained> {
    let causes: Vec<(String, String, i64)> = unreadable_records(conn, sc, since, until)?
        .into_iter()
        .take(CAUSE_LIMIT)
        .map(|(record_type, version, count)| {
            (
                record_type
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| "(unknown)".into()),
                version,
                count,
            )
        })
        .collect();
    let mut facts = HashMap::new();
    if let Some((name, version, count)) = causes.first() {
        facts.insert("top", name.clone());
        facts.insert("top_version", version.clone());
        facts.insert("top_count", count.to_string());
    }
    Ok(Explained {
        diagnosis: "unreadable_records",
        causes: causes
            .iter()
            .map(|(name, version, count)| {
                json!({
                    "name": name,
                    "kind": "record_type",
                    "version": version,
                    "unreadable": count,
                    "text": format!(
                        "{name} records from Claude Code {version}: {count} could not be read"
                    ),
                })
            })
            .collect(),
        facts,
    })
}

fn explain_stale_indexes(
    conn: &Connection,
    now: i64,
    measured: f64,
    detail: &Detail,
) -> Result<Explained> {
    let stale: Vec<&str> = detail
        .as_ref()
        .and_then(|d| d.get("projects"))
        .and_then(Value::as_array)
        .map(|names| names.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let mut causes = Vec::new();
    let mut names = Vec::new();
    for path in context_paths(conn)? {
        if causes.len() == CAUSE_LIMIT {
            break;
        }
        let name = name_or_path(&path);
        if !stale.contains(&name.as_str()) {
            continue;
        }
        let Some(found) = freshness(conn, &path, now)? else {
            continue;
        };
        if found.stale != Some(true) {
            continue;
        }
        let (indexed, commit) = (found.last_index_at, found.latest_commit_utc);
        causes.push(json!({
            "name": name,
            "kind": "project",
            "last_index_at": indexed,
            "latest_commit_utc": commit,
            "text": format!(
                "{name}: indexed {}, latest commit {}",
                indexed.as_deref().unwrap_or("None"),
                commit.as_deref().unwrap_or("None")
            ),
        }));
        names.push(name);
    }
    Ok(Explained {
        diagnosis: "stale_index",
        causes,
        facts: HashMap::from([
            ("names", names.join(", ")),
            ("count", (measured as i64).to_string()),
        ]),
    })
}

/// The fix lines of one (rule, diagnosis); a placeholder in `{braces}` is filled from the facts.
fn fix_lines(rule_id: &str, diagnosis: &str) -> &'static [&'static str] {
    match (rule_id, diagnosis) {
        ("tokens_jump", "more_sessions") => &[
            "The rise comes from more sessions ({sessions_before} \u{2192} {sessions_now}), not bigger ones: tokens per session went from {tps_before} to {tps_now}.",
            "New since the period before: {new}. Check that it should run this often.",
            "Biggest rise: {top} (+{top_rise} tokens). Decide if that pace is intended before it becomes your baseline.",
        ],
        ("tokens_jump", "bigger_sessions") => &[
            "Each session got bigger: tokens per session went from {tps_before} to {tps_now} ({sessions_before} \u{2192} {sessions_now} sessions).",
            "The largest session, {top_session} in {top_session_project}, used {top_session_tokens} tokens. Run /compact in long sessions or start a new one.",
            "Decide if the rise is intended before it becomes your baseline.",
        ],
        ("cache_hit_low", "model_switching") => &[
            "{multi_count} sessions used 2 or more models; their cache reuse is {multi_reuse} against {single_reuse} in the others.",
            "Use one model for a whole session: switching model in the middle restarts the cache. Biggest case: {top_session} in {top_session_project}.",
            "Lowest reuse by project: {low_project} ({low_reuse}).",
        ],
        ("cache_hit_low", "unstable_prefix") => &[
            "Lowest cache reuse by project: {low_project} ({low_reuse} of its prompt tokens came from cache).",
            "Keep the start of your prompts the same from one turn to the next.",
            "Avoid editing CLAUDE.md or switching models in the middle of a session.",
        ],
        ("session_huge_no_compact", "huge_sessions") => &[
            "The largest open session is {top_session} in {top_session_project}: {top_context} tokens of context.",
            "Run /compact when a session grows long.",
            "Start a new session before the context passes {threshold} tokens.",
        ],
        ("project_dominates", "dominant_project") => &[
            "{project} used {share} of tokens; its largest session, {top_session}, alone used {top_session_share} of all tokens.",
            "Confirm that this project should take most of your effort.",
            "Split long-running work into shorter sessions.",
        ],
        ("tool_errors_high", "one_tool_fails") => &[
            "{top} caused {top_errors} of the {total_errors} failed calls ({top_share}).",
            "Read the failing {top} calls in memory/event-trace/ and fix the command or permission that keeps failing.",
        ],
        ("tool_errors_high", "errors_spread") => &[
            "Failures are spread over many tools; {top} has the most failed calls: {top_errors} of {total_errors}.",
            "Read the failing tool calls of {top} in memory/event-trace/.",
            "Fix the command or permission that keeps failing.",
        ],
        ("records_unreadable", "unreadable_records") => &[
            "Most unreadable records are {top} records from Claude Code {top_version} ({top_count} records).",
            "Update easy-personal to the latest version.",
            "If it stays high, tell the easy-personal maintainer about {top} records from Claude Code {top_version}.",
        ],
        ("context_index_stale", "stale_index") => &[
            "The index is older than the latest commit in: {names} ({count} in all).",
            "Call `index_path` in easy-context for the listed projects.",
            "You run it yourself; easy-personal never does.",
        ],
        _ => &[],
    }
}

/// `line.format_map(facts)`, or `None` when a placeholder has no fact (a line about something the data lacks is left out).
fn fill(line: &str, facts: &HashMap<&'static str, String>) -> Option<String> {
    let mut out = String::new();
    let mut rest = line;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let close = rest[open..].find('}')? + open;
        out.push_str(facts.get(&rest[open + 1..close])?);
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    Some(out)
}

/// `causes`, `diagnosis`, the fix lines (`steps`, `fix`) and, for a rule that counts tokens, `cache_read_share`.
fn explain(
    rule: &Rule,
    conn: &Connection,
    sc: &Resolved,
    now: i64,
    measured: f64,
    detail: &Detail,
    win: &mut Windows,
) -> Result<Map<String, Value>> {
    let (since, until) = (now - rule.window_days * DAY, now);
    let mut explained = match rule.metric {
        Metric::WeekOverWeekTokens => explain_week_over_week(since, until, win)?,
        Metric::CacheHitRatio => explain_cache_hit(since, until, win)?,
        Metric::MedianSessionContext => explain_median_context(conn, sc, since, until, win)?,
        Metric::TopProjectShare => explain_top_project(since, until, measured, detail, win)?,
        Metric::KitErrorRate => explain_kit_errors(conn, sc, since, until, win)?,
        Metric::UnreadableShare => explain_unreadable(conn, sc, since, until)?,
        Metric::ContextStaleIndexes => explain_stale_indexes(conn, now, measured, detail)?,
    };
    explained
        .facts
        .insert("threshold", general(rule.threshold.as_f64()));
    let mut steps: Vec<String> = fix_lines(&rule.id, explained.diagnosis)
        .iter()
        .filter_map(|line| fill(line, &explained.facts))
        .collect();
    if steps.is_empty() {
        steps = rule.steps.clone();
    }
    let mut found = Map::new();
    found.insert("causes".into(), Value::Array(explained.causes));
    found.insert("diagnosis".into(), json!(explained.diagnosis));
    found.insert("fix".into(), json!(steps.join(" ")));
    found.insert("steps".into(), json!(steps));
    if rule.metric.counts_tokens() {
        let window = win.get(since, until)?;
        found.insert(
            "cache_read_share".into(),
            if window.tokens != 0 {
                json!(r4(window.cache_read as f64 / window.tokens as f64))
            } else {
                Value::Null
            },
        );
    }
    Ok(found)
}

/// True when the rule is left out because the kit part it checks is not in use (FR-080): only
/// `context_stale_indexes` can be, with no easy-context tool call in the last 7 days.
fn left_out(rule: &Rule, conn: &Connection, sc: &Resolved, now: i64) -> Result<bool> {
    if rule.metric != Metric::ContextStaleIndexes {
        return Ok(false);
    }
    Ok(!easy_context_in_use(
        conn,
        sc,
        now - IN_USE_DAYS * DAY,
        now,
    )?)
}

/// Runs every rule over the store as of `now_micros` (microseconds since the Unix epoch, UTC).
///
/// `hits` are the fired rules, ordered by priority then rule id; `abstained` are the rules that could not be
/// measured, ordered by rule id; a rule about a kit part that is not in use is in neither list.
pub fn evaluate(conn: &Connection, now_micros: i64, rules: &[Rule]) -> Result<Value> {
    evaluate_in(
        conn,
        now_micros,
        rules,
        &scope::resolve(conn, Scope::default())?,
        false,
    )
}

/// [`evaluate`] limited to a workspace (a name) and/or an agent account (an id or a label): every metric, and the
/// kit rules (through the scope's projects), see only that scope. An unknown workspace or account is refused up
/// front, even when the store has too little history to measure anything.
pub fn evaluate_scoped(
    conn: &Connection,
    now_micros: i64,
    rules: &[Rule],
    scope: Scope,
) -> std::result::Result<Value, ScopedRulesError> {
    let resolved = scope::resolve(conn, scope)?;
    Ok(evaluate_in(conn, now_micros, rules, &resolved, false)?)
}

/// [`evaluate_scoped`] plus Python's `checked=True`: a `checked` list (by rule id) of every measured rule with its
/// `title`, `measured`, `threshold` and `fired`, so a rule that did not fire still has its value (saved reports, FR-076).
pub fn evaluate_checked_scoped(
    conn: &Connection,
    now_micros: i64,
    rules: &[Rule],
    scope: Scope,
) -> std::result::Result<Value, ScopedRulesError> {
    let resolved = scope::resolve(conn, scope)?;
    Ok(evaluate_in(conn, now_micros, rules, &resolved, true)?)
}

/// The ids of the rules [`evaluate`] leaves out in this scope because the kit part they check is not in use (FR-080;
/// Python's `rules.left_out`), in rule order. An unknown workspace or account is refused as for [`evaluate_scoped`].
pub fn left_out_ids(
    conn: &Connection,
    now_micros: i64,
    rules: &[Rule],
    scope: Scope,
) -> std::result::Result<Vec<String>, ScopedRulesError> {
    let resolved = scope::resolve(conn, scope)?;
    let mut ids = Vec::new();
    for rule in rules {
        if left_out(rule, conn, &resolved, now_micros)? {
            ids.push(rule.id.clone());
        }
    }
    Ok(ids)
}

fn evaluate_in(
    conn: &Connection,
    now_micros: i64,
    rules: &[Rule],
    sc: &Resolved,
    checked: bool,
) -> Result<Value> {
    let mut hits: Vec<Map<String, Value>> = Vec::new();
    let mut abstained: Vec<(String, Value)> = Vec::new();
    let mut measured_rules: Vec<(String, Value)> = Vec::new();
    let mut win = Windows::new(conn, sc);
    for rule in rules {
        if left_out(rule, conn, sc, now_micros)? {
            continue;
        }
        let (measured, detail) = match measure(rule, conn, sc, now_micros, &mut win)? {
            Outcome::Abstain(reason) => {
                abstained.push((
                    rule.id.clone(),
                    json!({"rule_id": rule.id, "title": rule.title, "reason": reason}),
                ));
                continue;
            }
            Outcome::Measured(measured, detail) => (measured, detail),
        };
        let (m, t) = (measured.as_f64(), rule.threshold.as_f64());
        let fired = match rule.op {
            Op::Lt => m < t,
            Op::Gt => m > t,
        };
        if checked {
            measured_rules.push((
                rule.id.clone(),
                json!({
                    "rule_id": rule.id, "title": rule.title, "measured": measured.json(),
                    "threshold": rule.threshold.json(), "fired": fired,
                }),
            ));
        }
        if !fired {
            continue;
        }
        let mut hit = Map::new();
        hit.insert("rule_id".into(), json!(rule.id));
        hit.insert("label".into(), json!(rule.label));
        hit.insert("priority".into(), json!(rule.priority));
        hit.insert("measured".into(), measured.json());
        hit.insert("threshold".into(), rule.threshold.json());
        hit.insert("window_days".into(), json!(rule.window_days));
        hit.insert("fix".into(), json!(rule.steps.join(" ")));
        hit.insert("title".into(), json!(rule.title));
        hit.insert("why".into(), json!(rule.why));
        hit.insert("steps".into(), json!(rule.steps));
        hit.insert("summary".into(), json!(summary(rule, m, &detail)));
        if let Some(detail) = &detail {
            hit.insert("detail".into(), Value::Object(detail.clone()));
        }
        hit.extend(explain(rule, conn, sc, now_micros, m, &detail, &mut win)?);
        hits.push(hit);
    }
    let rank = |hit: &Map<String, Value>| {
        (
            hit["priority"].as_i64().unwrap_or(0),
            hit["rule_id"].as_str().unwrap_or("").to_string(),
        )
    };
    hits.sort_by_key(rank);
    abstained.sort_by(|a, b| a.0.cmp(&b.0));
    let mut found = json!({
        "as_of_utc": iso(now_micros),
        "hits": hits,
        "abstained": abstained.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
    });
    if checked {
        measured_rules.sort_by(|a, b| a.0.cmp(&b.0));
        found["checked"] = Value::Array(measured_rules.into_iter().map(|(_, v)| v).collect());
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::{fixed, general, percent, short};

    /// Expected values are Python's: `format(x, ",g")`, `format(x, ",.0f")`, `rules._percent`, `rules._short`.
    #[test]
    fn number_text_matches_python() {
        for (x, want) in [
            (150000.0, "150,000"),
            (0.5, "0.5"),
            (0.02, "0.02"),
            (0.0, "0"),
            (1e6, "1e+06"),
            (123456.7, "123,457"),
            (1234567.0, "1.23457e+06"),
            (0.00001, "1e-05"),
            (0.0001234, "0.0001234"),
            (999999.5, "1e+06"),
        ] {
            assert_eq!(general(x), want, "{x}");
        }
        for (x, want) in [
            (0.5, "0"),
            (1.5, "2"),
            (2.5, "2"),
            (150001.5, "150,002"),
            (1234.5, "1,234"),
            (-0.04, "-0"),
            (1e9, "1,000,000,000"),
        ] {
            assert_eq!(fixed(x, 0), want, "{x}");
        }
        assert_eq!(fixed(1e9, 1), "1,000,000,000.0");
        assert_eq!(fixed(-0.04, 1), "-0.0");
        for (v, want) in [
            (0.25, "25%"),
            (0.02, "2%"),
            (1.0, "100%"),
            (12.5, "1,250%"),
            (0.12345, "12.3%"),
            (0.0004, "0%"),
            (0.999, "99.9%"),
            (-0.5, "-50%"),
            (10.0, "1,000%"),
        ] {
            assert_eq!(percent(v), want, "{v}");
        }
        for (n, want) in [
            (0.0, "0"),
            (999.0, "999"),
            (999.5, "1,000"),
            (1000.0, "1.0k"),
            (1050.0, "1.1k"),
            (2500.0, "2.5k"),
            (1.5e6, "1.5M"),
            (2.25e9, "2.2B"),
            (12345678.0, "12.3M"),
        ] {
            assert_eq!(short(n), want, "{n}");
        }
    }
}
