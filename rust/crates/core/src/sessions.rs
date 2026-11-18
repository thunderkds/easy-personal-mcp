//! Per-session reads: `session_outliers` and `session_timeline`, ported from `src/easy_personal/core/metrics.py`
//! (FR-035, FR-036 scope; `detail_sessions` as the dashboard route sends it).
//!
//! The SQL statements, grouping and ordering are Python's, with the helpers `usage_summary` already uses (`bounds`, the
//! period clauses, `folder`, the scope clause of `scope.rs`). Read-only; only values are bound, no SQL is built from the
//! caller's text. The plain functions are the scoped ones with no scope; the four names are also reachable as
//! `metrics::session_outliers` and so on. An offset-less `since` / `until` is local time, as in `metrics.rs`.
//!
//! Strings are ordered by byte, which for UTF-8 is Python's code point order, so session ids sort as Python sorts them.

use crate::live::DEFAULT_LABEL;
use crate::metrics::{
    MetricsError, bounds, folder, format_stored, parse_iso, period, period_json, round_py, text,
};
use crate::scope::{self, Clause, Scope};
use crate::store::StoreError;
use jiff::tz::TimeZone;
use rusqlite::types::Value as Sql;
use rusqlite::{Connection, params_from_iter};
use serde_json::{Map, Value, json};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, MetricsError>;

/// A session is an outlier from this many tool calls (`OUTLIER_TOOL_CALLS`)...
const OUTLIER_TOOL_CALLS: i64 = 90;
/// ...or from a message whose prompt (input + cache creation + cache read tokens) is this big (`OUTLIER_CONTEXT_TOKENS`).
const OUTLIER_CONTEXT_TOKENS: i64 = 200_000;
/// An idle gap inside a session starts at this many minutes (`TIMELINE_GAP_MINUTES`).
const TIMELINE_GAP_MINUTES: i64 = 30;

/// Sessions with >= 90 tool calls or a context >= 200,000 tokens in [since, until), most calls first, ties by session id.
/// Context = input + cache creation + cache read tokens of one message (its prompt size).
pub fn session_outliers(
    conn: &Connection,
    since: Option<&str>,
    until: Option<&str>,
) -> Result<Value> {
    session_outliers_scoped(conn, since, until, Scope::default())
}

/// [`session_outliers`] limited to a workspace and/or an account, as `usage_summary_scoped`. A bad `since` / `until`
/// is checked first, then the scope; both are `MetricsError::Invalid` with Python's text.
pub fn session_outliers_scoped(
    conn: &Connection,
    since: Option<&str>,
    until: Option<&str>,
    scope: Scope,
) -> Result<Value> {
    let b = bounds(since, until, &TimeZone::system())?;
    let Clause {
        sql: scope_sql,
        params: scope_params,
    } = scope::resolve(conn, scope)?.clause(conn, b.since.as_deref(), b.until.as_deref())?;
    let (where_utc, params) = period("ts_utc", b.since.as_deref(), b.until.as_deref());
    let filter = format!("{where_utc} AND {scope_sql}");
    let mut params = text(params);
    params.extend(scope_params);

    // tool calls: the comma-separated names of every event, an empty list counting as one (`len(tools.split(","))`)
    let mut calls: BTreeMap<String, i64> = BTreeMap::new();
    let mut stmt = conn.prepare(&format!(
        "SELECT session_id, tool_names FROM events \
         WHERE abstained = 0 AND session_id IS NOT NULL AND tool_names IS NOT NULL AND {filter}"
    ))?;
    for row in stmt.query_map(params_from_iter(&params), |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })? {
        let (session, tools) = row?;
        *calls.entry(session).or_insert(0) += tools.split(',').count() as i64;
    }

    let mut contexts: BTreeMap<String, i64> = BTreeMap::new();
    let mut stmt = conn.prepare(&format!(
        "SELECT session_id, MAX(COALESCE(input_tokens, 0) + \
         COALESCE(cache_creation_input_tokens, 0) + COALESCE(cache_read_input_tokens, 0)) \
         FROM events WHERE abstained = 0 AND session_id IS NOT NULL \
         AND message_id IS NOT NULL AND {filter} GROUP BY session_id"
    ))?;
    for row in stmt.query_map(params_from_iter(&params), |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })? {
        let (session, context) = row?;
        contexts.insert(session, context);
    }

    let mut projects: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut stmt = conn.prepare(&format!(
        "SELECT session_id, MAX(project_path) FROM events \
         WHERE abstained = 0 AND session_id IS NOT NULL AND {filter} GROUP BY session_id"
    ))?;
    for row in stmt.query_map(params_from_iter(&params), |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
    })? {
        let (session, project) = row?;
        projects.insert(session, project);
    }

    let ids: BTreeSet<&String> = calls.keys().chain(contexts.keys()).collect();
    let mut sessions: Vec<(i64, i64, &String, Vec<&str>)> = Vec::new();
    for session in ids {
        let tool_calls = calls.get(session).copied().unwrap_or(0);
        let context = contexts.get(session).copied().unwrap_or(0);
        let mut reasons = Vec::new();
        if tool_calls >= OUTLIER_TOOL_CALLS {
            reasons.push("tool_calls");
        }
        if context >= OUTLIER_CONTEXT_TOKENS {
            reasons.push("context_tokens");
        }
        if !reasons.is_empty() {
            sessions.push((tool_calls, context, session, reasons));
        }
    }
    sessions.sort_by(|a, b| (Reverse(a.0), a.2).cmp(&(Reverse(b.0), b.2)));
    let sessions: Vec<Value> = sessions
        .into_iter()
        .map(|(tool_calls, context, session, reasons)| {
            json!({
                "session_id": session,
                "project": folder(projects.get(session).and_then(|p| p.as_deref())),
                "tool_calls": tool_calls,
                "max_context_tokens": context,
                "reasons": reasons,
            })
        })
        .collect();
    Ok(json!({
        "period": period_json(&b),
        "thresholds": {"tool_calls": OUTLIER_TOOL_CALLS, "context_tokens": OUTLIER_CONTEXT_TOKENS},
        "sessions": sessions,
    }))
}

/// One token point: a message, once, at its earliest record time.
struct Point {
    ts: String,
    model: Option<String>,
    tokens: [i64; 4], // input, output, cache creation, cache read
}

impl Point {
    fn json(&self) -> Value {
        json!({
            "ts_utc": self.ts,
            "model": self.model,
            "input_tokens": self.tokens[0],
            "output_tokens": self.tokens[1],
            "cache_creation_input_tokens": self.tokens[2],
            "cache_read_input_tokens": self.tokens[3],
        })
    }
}

/// Per session in [since, until): start/end, models, token points per message and idle gaps. Newest first, ties by
/// session id. Start, end and gaps come from every non-abstained event of the session that has a time; points and totals
/// count each message once (the rule of `usage_summary`). Tokens and times only, no text.
///
/// `detail_sessions` (the dashboard sends 50): only the newest N sessions carry `points` and `gaps`; the others keep every
/// total but have both lists empty, and the answer says so in a top-level `detail_sessions`. `None` = every session in
/// full, and the key is absent.
pub fn session_timeline(
    conn: &Connection,
    since: Option<&str>,
    until: Option<&str>,
    detail_sessions: Option<usize>,
) -> Result<Value> {
    session_timeline_scoped(conn, since, until, detail_sessions, Scope::default())
}

/// [`session_timeline`] limited to a workspace and/or an account, as `usage_summary_scoped`.
pub fn session_timeline_scoped(
    conn: &Connection,
    since: Option<&str>,
    until: Option<&str>,
    detail_sessions: Option<usize>,
    scope: Scope,
) -> Result<Value> {
    let b = bounds(since, until, &TimeZone::system())?;
    let Clause {
        sql: scope_sql,
        params: scope_params,
    } = scope::resolve(conn, scope)?.clause(conn, b.since.as_deref(), b.until.as_deref())?;

    // every timed event: the stamps (oldest first) and the first non-empty project path of each session
    let (where_utc, params) = period("ts_utc", b.since.as_deref(), b.until.as_deref());
    let mut params = text(params);
    params.extend(scope_params.iter().cloned());
    let mut sessions: BTreeMap<String, (Vec<String>, Option<String>)> = BTreeMap::new();
    let mut stmt = conn.prepare(&format!(
        "SELECT session_id, ts_utc, project_path FROM events WHERE abstained = 0 \
         AND session_id IS NOT NULL AND ts_utc IS NOT NULL AND {where_utc} AND {scope_sql} ORDER BY ts_utc"
    ))?;
    for row in stmt.query_map(params_from_iter(&params), |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
        ))
    })? {
        let (session, ts, project_path) = row?;
        let (stamps, path) = sessions.entry(session).or_default();
        stamps.push(ts);
        if path.as_deref().is_none_or(str::is_empty) {
            *path = project_path;
        }
    }

    // one point per message, grouped like `usage_summary`: the inner window is padded by a day
    let (inner, params) = period("ts_utc", b.inner_since.as_deref(), b.inner_until.as_deref());
    let (where_ts, outer) = period("ts", b.since.as_deref(), b.until.as_deref());
    let mut params = text(params);
    params.extend(scope_params);
    params.extend(text(outer));
    let mut points: BTreeMap<String, Vec<Point>> = BTreeMap::new();
    let mut stmt = conn.prepare(&format!(
        "SELECT session_id, ts, model, i, o, cc, cr FROM ( \
           SELECT MAX(session_id) AS session_id, MIN(ts_utc) AS ts, MAX(model) AS model, \
                  message_id, MAX(input_tokens) AS i, \
                  COALESCE(MAX(output_tokens), 0) AS o, \
                  COALESCE(MAX(cache_creation_input_tokens), 0) AS cc, \
                  COALESCE(MAX(cache_read_input_tokens), 0) AS cr \
           FROM events WHERE abstained = 0 AND message_id IS NOT NULL \
           AND input_tokens IS NOT NULL AND session_id IS NOT NULL AND {inner} AND {scope_sql} \
           GROUP BY message_id \
         ) WHERE ts IS NOT NULL AND {where_ts} ORDER BY ts, message_id"
    ))?;
    for row in stmt.query_map(params_from_iter(&params), |r| {
        Ok((
            r.get::<_, String>(0)?,
            Point {
                ts: r.get(1)?,
                model: r.get(2)?,
                tokens: [r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?],
            },
        ))
    })? {
        let (session, point) = row?;
        points.entry(session).or_default().push(point);
    }

    // newest first by start; the sort is stable over the ascending ids, so a tie keeps the session id order
    let mut order: Vec<&String> = sessions.keys().collect();
    order.sort_by(|a, c| sessions[*c].0[0].cmp(&sessions[*a].0[0]));
    let mut result = Vec::with_capacity(order.len());
    for (rank, session) in order.into_iter().enumerate() {
        let (stamps, path) = &sessions[session];
        let pts = points.get(session).map_or(&[][..], Vec::as_slice);
        let detailed = detail_sessions.is_none_or(|n| rank < n);
        let mut gaps = Vec::new();
        if detailed {
            for pair in stamps.windows(2) {
                let unreadable = || {
                    MetricsError::Store(StoreError::Unreadable(
                        "a stored timestamp is not ISO 8601".into(),
                    ))
                };
                let before = parse_iso(&pair[0]).ok_or_else(unreadable)?;
                let after = parse_iso(&pair[1]).ok_or_else(unreadable)?;
                // Python: `timedelta.total_seconds() / 60` (microseconds / 1e6, then / 60), as floats
                let minutes = (after - before) as f64 / 1e6 / 60.0;
                if minutes >= TIMELINE_GAP_MINUTES as f64 {
                    gaps.push(json!({
                        "start_utc": pair[0],
                        "end_utc": pair[1],
                        "minutes": round_py(minutes, 1),
                    }));
                }
            }
        }
        let models: BTreeSet<&str> = pts
            .iter()
            .filter_map(|p| p.model.as_deref())
            .filter(|m| !m.is_empty())
            .collect();
        let total = |i: usize| pts.iter().map(|p| p.tokens[i]).sum::<i64>();
        result.push(json!({
            "session_id": session,
            "project": folder(path.as_deref()),
            "start_utc": stamps[0],
            "end_utc": stamps[stamps.len() - 1],
            "models": models,
            "messages": pts.len(),
            "input_tokens": total(0),
            "output_tokens": total(1),
            "cache_creation_input_tokens": total(2),
            "cache_read_input_tokens": total(3),
            "points": if detailed { pts.iter().map(Point::json).collect() } else { Vec::new() },
            "gaps": gaps,
        }));
    }
    let mut answer = Map::new();
    answer.insert("period".into(), period_json(&b));
    answer.insert("gap_minutes".into(), json!(TIMELINE_GAP_MINUTES));
    answer.insert("sessions".into(), Value::Array(result));
    if let Some(n) = detail_sessions {
        answer.insert("detail_sessions".into(), json!(n));
    }
    Ok(Value::Object(answer))
}

// ---- past sessions (T117, FR-126) --------------------------------------------------------------------------------------

/// Sessions on one page of [`past_sessions`].
pub const PAST_PAGE: usize = 50;
/// The longest `project` / `branch` filter, in bytes.
const FILTER_MAX: usize = 255;
/// The longest session id a cursor or a name route accepts.
const ID_MAX: usize = 128;

/// The filters of `GET /api/sessions/past`; an empty value is no filter. `from` / `to` are ISO 8601 dates or datetimes
/// (an offset-less one is local time, as `since` / `until`); `kind` is `terminal`, `app` or `headless`; `account` is an
/// account id or label; `before` is the cursor the last page gave (`next`).
#[derive(Debug, Default, Clone, Copy)]
pub struct PastFilter<'a> {
    pub project: Option<&'a str>,
    pub branch: Option<&'a str>,
    pub from: Option<&'a str>,
    pub to: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub account: Option<&'a str>,
    pub before: Option<&'a str>,
}

/// A session id the app handles as a name key or a cursor part: 1..=128 characters of `A-Z a-z 0-9 _ -`.
pub fn valid_session_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= ID_MAX
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// `terminal` for any entrypoint that is not `sdk*` (cli, IDE, desktop), `app` for `sdk-cli` (what this app's own
/// `claude -p` session writes), `headless` for any other `sdk*`; NULL when the session has no entrypoint yet.
/// The store cannot tell an app session from a scheduled `claude -p` run: both write `sdk-cli`.
const KIND_SQL: &str = "CASE WHEN MAX(entrypoint) IS NULL THEN NULL WHEN MAX(entrypoint) = 'sdk-cli' THEN 'app' \
                        WHEN MAX(entrypoint) LIKE 'sdk%' THEN 'headless' ELSE 'terminal' END";

fn filter_text<'a>(name: &str, value: Option<&'a str>) -> Result<Option<&'a str>> {
    match value.filter(|v| !v.is_empty()) {
        Some(v) if v.len() > FILTER_MAX || v.chars().any(char::is_control) => {
            Err(MetricsError::Invalid(format!(
                "{name} must be one line of at most {FILTER_MAX} bytes"
            )))
        }
        other => Ok(other),
    }
}

/// `from` / `to` as stored-form UTC bounds; the error text is fixed and holds none of the input.
fn past_bound(name: &str, value: Option<&str>) -> Result<Option<String>> {
    let Some(value) = value.filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    let bad = || MetricsError::Invalid(format!("{name} must be an ISO 8601 date or datetime"));
    let b = bounds(Some(value), None, &TimeZone::system()).map_err(|_| bad())?;
    b.since.ok_or_else(bad).map(Some)
}

/// The cursor: a stored-form time, then `|` and a session id (`next` of the page before); the id may be left out.
fn past_cursor(value: Option<&str>) -> Result<Option<(String, Option<String>)>> {
    let Some(value) = value.filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    let bad = || MetricsError::Invalid("before must be the next value of the last page".into());
    let (stamp, id) = match value.split_once('|') {
        Some((stamp, id)) => (stamp, Some(id)),
        None => (value, None),
    };
    if id.is_some_and(|id| !valid_session_id(id)) {
        return Err(bad());
    }
    let micros = parse_iso(stamp).ok_or_else(bad)?;
    Ok(Some((format_stored(micros), id.map(str::to_owned))))
}

/// One page of past sessions from `events` metadata only (FR-002), newest `last` first, [`PAST_PAGE`] a page, as
/// `{sessions: [{id, project, branch, first, last, kind, account, model}], next}`; `next` is the `before` of the following
/// page, `null` on the last. One query: filters on project, branch and kind apply to the session's own values (HAVING);
/// `from` / `to` and the account window the events that are counted (WHERE, so the time index serves them), which makes
/// `first` / `last` the first / last event inside the window. A session's branch, project and model are the greatest
/// stored value (`MAX`), its account the account of its events (none = the default account). No text is read.
pub fn past_sessions(conn: &Connection, filter: PastFilter) -> Result<Value> {
    let (sql, params) = past_query(conn, filter)?;
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt
        .query_map(params_from_iter(&params), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, Option<String>>(7)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let more = rows.len() > PAST_PAGE;
    rows.truncate(PAST_PAGE);
    let next = if more {
        rows.last().map(|r| format!("{}|{}", r.2, r.0))
    } else {
        None
    };
    let sessions: Vec<Value> = rows
        .into_iter()
        .map(|(id, first, last, project, branch, kind, model, account)| {
            json!({
                "id": id, "project": folder(project.as_deref()), "branch": branch.filter(|b| !b.is_empty()),
                "first": first, "last": last, "kind": kind, "account": account, "model": model,
            })
        })
        .collect();
    Ok(json!({"sessions": sessions, "next": next}))
}

/// The one statement of [`past_sessions`] and its values, after the filters are checked (a test asks SQLite for its plan).
#[doc(hidden)]
pub fn past_query(conn: &Connection, filter: PastFilter) -> Result<(String, Vec<Sql>)> {
    let project = filter_text("project", filter.project)?;
    let branch = filter_text("branch", filter.branch)?;
    let kind = match filter.kind.filter(|k| !k.is_empty()) {
        None => None,
        Some(k @ ("terminal" | "app" | "headless")) => Some(k),
        Some(_) => {
            return Err(MetricsError::Invalid(
                "kind must be one of terminal, app, headless".into(),
            ));
        }
    };
    let (from, to) = (
        past_bound("from", filter.from)?,
        past_bound("to", filter.to)?,
    );
    let before = past_cursor(filter.before)?;
    let Clause {
        sql: scope_sql,
        params: scope_params,
    } = scope::resolve(
        conn,
        Scope {
            workspace: None,
            account: filter.account.filter(|a| !a.is_empty()),
        },
    )?
    .clause(conn, from.as_deref(), to.as_deref())?;

    let (where_utc, window) = period("ts_utc", from.as_deref(), to.as_deref());
    let mut params: Vec<Sql> = text(window);
    params.extend(scope_params);
    let mut having: Vec<&str> = Vec::new();
    let text_param = |v: &str| Sql::Text(v.to_owned());
    if let Some(p) = project {
        having.push(
            "(MAX(project_path) = ? OR substr(MAX(project_path), -length(?) - 1) = '/' || ?)",
        );
        params.extend([text_param(p), text_param(p), text_param(p)]);
    }
    if let Some(b) = branch {
        having.push("MAX(git_branch) = ?");
        params.push(text_param(b));
    }
    let kind_having = format!("{KIND_SQL} = ?");
    if let Some(k) = kind {
        having.push(&kind_having);
        params.push(text_param(k));
    }
    if let Some((stamp, id)) = &before {
        match id {
            Some(id) => {
                having.push("(MAX(ts_utc) < ? OR (MAX(ts_utc) = ? AND session_id < ?))");
                params.extend([text_param(stamp), text_param(stamp), text_param(id)]);
            }
            None => {
                having.push("MAX(ts_utc) < ?");
                params.push(text_param(stamp));
            }
        }
    }
    let having = if having.is_empty() {
        String::new()
    } else {
        format!(" HAVING {}", having.join(" AND "))
    };
    let sql = format!(
        "SELECT s.session_id, s.t_first, s.t_last, s.project, s.branch, s.kind, s.model, \
                COALESCE((SELECT label FROM agent_accounts WHERE id = s.account), \
                         (SELECT label FROM agent_accounts WHERE provider = 'claude' AND config_dir_env IS NULL \
                          ORDER BY id LIMIT 1), ?) \
         FROM (SELECT session_id, MIN(ts_utc) AS t_first, MAX(ts_utc) AS t_last, MAX(project_path) AS project, \
                      MAX(git_branch) AS branch, {KIND_SQL} AS kind, MAX(model) AS model, \
                      MAX(agent_account_id) AS account \
               FROM events WHERE abstained = 0 AND session_id IS NOT NULL AND ts_utc IS NOT NULL \
               AND {where_utc} AND {scope_sql} GROUP BY session_id{having} \
               ORDER BY MAX(ts_utc) DESC, session_id DESC LIMIT {limit}) AS s \
         ORDER BY s.t_last DESC, s.session_id DESC",
        limit = PAST_PAGE + 1
    );
    // the label for a default account with no row comes first in the statement text, so it is bound first
    params.insert(0, Sql::Text(DEFAULT_LABEL.to_owned()));
    Ok((sql, params))
}
