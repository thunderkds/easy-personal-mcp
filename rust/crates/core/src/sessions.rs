//! Per-session reads: `session_outliers` and `session_timeline`, ported from `src/easy_personal/core/metrics.py`
//! (FR-035, FR-036 scope; `detail_sessions` as the dashboard route sends it).
//!
//! The SQL statements, grouping and ordering are Python's, with the helpers `usage_summary` already uses (`bounds`, the
//! period clauses, `folder`, the scope clause of `scope.rs`). Read-only; only values are bound, no SQL is built from the
//! caller's text. The plain functions are the scoped ones with no scope; the four names are also reachable as
//! `metrics::session_outliers` and so on. An offset-less `since` / `until` is local time, as in `metrics.rs`.
//!
//! Strings are ordered by byte, which for UTF-8 is Python's code point order, so session ids sort as Python sorts them.

use crate::metrics::{
    MetricsError, bounds, folder, parse_iso, period, period_json, round_py, text,
};
use crate::scope::{self, Clause, Scope};
use crate::store::StoreError;
use jiff::tz::TimeZone;
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
