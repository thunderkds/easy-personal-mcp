//! Usage metrics over the store: `usage_summary` and `usage_breakdown`, ported from
//! `src/easy_personal/core/metrics.py` (plus the project grouping of `core/owners.py`).
//!
//! The SQL statements, grouping and ordering are Python's. Read-only: nothing here writes. The optional
//! `workspace` and `account` scope (T017, FR-035/FR-036) is the `*_scoped` variants, resolved by `scope.rs`; the
//! plain functions are those variants with no scope. Not ported: the `prices` argument as a file path (see
//! [`usage_breakdown_with_prices`]), and the 300 s owner cache.
//!
//! Time zone (T009): like Python's `astimezone()`, an offset-less `since`/`until` and the `day` key use the machine's
//! local zone, DST-aware: the `TZ` variable, else the OS zone (`/etc/localtime`), read through `jiff` (see
//! `rust/docs/spike-T009.md`). An unknown `TZ` reads as UTC, as libc does. A bound that names an offset (`Z`,
//! `+07:00`) ignores the zone.

use crate::prices::{PriceError, PriceTable};
use crate::scope::{self, Clause, Scope, ScopeError};
use crate::store::StoreError;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use rusqlite::types::Value as Bound;
use rusqlite::{Connection, params_from_iter};
use serde_json::{Map, Value, json};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::OsString;
use std::fmt;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub use crate::sessions::{
    session_outliers, session_outliers_scoped, session_timeline, session_timeline_scoped,
};

const DIMENSIONS: [&str; 4] = ["day", "model", "project", "tool"];
pub(crate) const SCRATCH: &str = "Scratch";
pub(crate) const DAY_MICROS: i64 = 86_400_000_000;

#[derive(Debug)]
pub enum MetricsError {
    /// The caller's `by`, `since` or `until` is not valid. The text equals Python's 400 message.
    Invalid(String),
    Store(StoreError),
    Prices(PriceError),
}

impl fmt::Display for MetricsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetricsError::Invalid(text) => write!(f, "{text}"),
            MetricsError::Store(e) => write!(f, "{e}"),
            MetricsError::Prices(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for MetricsError {}

impl From<rusqlite::Error> for MetricsError {
    fn from(e: rusqlite::Error) -> Self {
        MetricsError::Store(StoreError::from(e))
    }
}

/// An unknown workspace or account is a caller error (`Invalid`, Python's 400 text); a store failure stays one.
impl From<ScopeError> for MetricsError {
    fn from(e: ScopeError) -> Self {
        match e {
            ScopeError::Unknown(text) => MetricsError::Invalid(text),
            ScopeError::Store(e) => MetricsError::Store(e),
        }
    }
}

type Result<T> = std::result::Result<T, MetricsError>;

/// Python's `round(x, digits)`: the exact binary value rounded half to even.
///
/// Rust formats a float exactly (no pre-multiplication) and rounds half to even on that exact decimal
/// expansion, the same as CPython's `round`, which calls `dtoa` in fixed-digits mode. Parsing the text back gives
/// the nearest f64, as Python's string-to-float step does. The naive `(x * 10^n).round() / 10^n`
/// rounds the product (already rounded to f64) half away from zero and differs on ties, e.g. `2.00005`.
pub fn round_py(x: f64, digits: usize) -> f64 {
    format!("{x:.digits$}").parse().unwrap_or(x)
}

// ---- time ----

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub(crate) fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// `n` ASCII digits at `s[at..]` as a number.
fn digits(s: &[u8], at: usize, n: usize) -> Option<i64> {
    let part = s.get(at..at + n)?;
    part.iter()
        .all(u8::is_ascii_digit)
        .then(|| part.iter().fold(0, |acc, b| acc * 10 + i64::from(b - b'0')))
}

/// What [`parse_iso_parts`] read: the clock fields as microseconds since the Unix epoch *as if they were UTC*, and the
/// offset in seconds the text named (`Z` is 0), `None` when it named none.
struct Iso {
    clock: i64,
    offset: Option<i64>,
}

/// The ISO 8601 forms Python's `datetime.fromisoformat` takes that the surfaces use: `YYYY-MM-DD`, then
/// optionally `T` or a space and `HH[:MM[:SS[.f...]]]`, then optionally `Z` or `+HH:MM` / `-HH:MM`.
/// `None` for anything else.
fn parse_iso_parts(value: &str) -> Option<Iso> {
    let s = value.as_bytes();
    if s.len() < 10 || s[4] != b'-' || s[7] != b'-' {
        return None;
    }
    let (y, mo, d) = (digits(s, 0, 4)?, digits(s, 5, 2)?, digits(s, 8, 2)?);
    if y < 1 || !(1..=12).contains(&mo) || d < 1 || d > days_in_month(y, mo) {
        return None;
    }
    let (mut h, mut mi, mut sec, mut micros, mut offset) = (0, 0, 0, 0, None);
    if s.len() > 10 {
        if s[10] != b'T' && s[10] != b' ' {
            return None;
        }
        h = digits(s, 11, 2)?;
        let mut at = 13;
        if s.get(at) == Some(&b':') {
            mi = digits(s, at + 1, 2)?;
            at += 3;
            if s.get(at) == Some(&b':') {
                sec = digits(s, at + 1, 2)?;
                at += 3;
                if s.get(at) == Some(&b'.') {
                    let frac: Vec<u8> = s[at + 1..]
                        .iter()
                        .take_while(|b| b.is_ascii_digit())
                        .copied()
                        .collect();
                    if frac.is_empty() {
                        return None;
                    }
                    for i in 0..6 {
                        micros = micros * 10 + frac.get(i).map_or(0, |b| i64::from(b - b'0'));
                    }
                    at += 1 + frac.len();
                }
            }
        }
        match &s[at..] {
            [] => {}
            [b'Z'] => offset = Some(0),
            [sign @ (b'+' | b'-'), rest @ ..] if rest.len() == 5 && rest[2] == b':' => {
                let (oh, om) = (digits(rest, 0, 2)?, digits(rest, 3, 2)?);
                if oh > 23 || om > 59 {
                    return None;
                }
                offset = Some((oh * 60 + om) * 60 * if *sign == b'-' { -1 } else { 1 });
            }
            _ => return None,
        }
        if h > 23 || mi > 59 || sec > 59 {
            return None;
        }
    }
    let seconds = days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + sec;
    Some(Iso {
        clock: seconds * 1_000_000 + micros,
        offset,
    })
}

/// A stored timestamp (always `...Z`) as microseconds since the Unix epoch in UTC; `None` for anything unreadable.
/// A value with no offset is read as UTC.
pub(crate) fn parse_iso(value: &str) -> Option<i64> {
    let iso = parse_iso_parts(value)?;
    Some(iso.clock - iso.offset.unwrap_or(0) * 1_000_000)
}

/// Wall-clock `clock` (microseconds, read as if UTC) in `zone` -> microseconds in UTC.
///
/// This is Python's `datetime.astimezone()` on a naive value: `fold=0`, so a repeated local time (the clocks go
/// back) is the first occurrence, and a skipped local time (the clocks go forward) is read with the offset in
/// force *before* the change, which lands it after the gap (02:30 on a spring-forward day is 03:30 summer
/// time). That is `jiff`'s `compatible` disambiguation. `None` when the result is outside `jiff`'s range.
fn local_to_utc(clock: i64, zone: &TimeZone) -> Option<i64> {
    let wall = Timestamp::from_microsecond(clock)
        .ok()?
        .to_zoned(TimeZone::UTC);
    let zoned = zone.to_ambiguous_zoned(wall.datetime()).compatible().ok()?;
    Some(zoned.timestamp().as_microsecond())
}

/// The calendar day (`YYYY-MM-DD`) of a UTC instant in `zone`.
pub(crate) fn local_day(micros: i64, zone: &TimeZone) -> Option<String> {
    let day = zone
        .to_datetime(Timestamp::from_microsecond(micros).ok()?)
        .date();
    Some(format!(
        "{:04}-{:02}-{:02}",
        day.year(),
        day.month(),
        day.day()
    ))
}

/// The stored form `%Y-%m-%dT%H:%M:%S.%fZ`.
pub(crate) fn format_stored(micros: i64) -> String {
    let days = micros.div_euclid(DAY_MICROS);
    let in_day = micros.rem_euclid(DAY_MICROS);
    let (y, m, d) = civil_from_days(days);
    let secs = in_day / 1_000_000;
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:06}Z",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60,
        in_day % 1_000_000
    )
}

/// Python's `repr` of a string, enough for the error text: single quotes unless the text holds one.
fn py_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::from(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// ISO date or datetime -> micros UTC. A value without an offset is local time in `zone` (see [`local_to_utc`]);
/// one with an offset ignores it. The error text is Python's `fromisoformat` message for a value
/// it cannot read; Python's separate messages for an out-of-range field (`month must be in 1..12`) are not
/// reproduced: those values get this same message. (Python also refuses a local date on the first and last day of
/// its range, `0001-01-01` and `9999-12-31`, whatever the zone; this port does not.)
fn utc_bound(value: Option<&str>, zone: &TimeZone) -> Result<Option<i64>> {
    value
        .map(|v| {
            let iso = parse_iso_parts(v).ok_or_else(|| {
                MetricsError::Invalid(format!("Invalid isoformat string: {}", py_repr(v)))
            })?;
            match iso.offset {
                Some(offset) => Ok(iso.clock - offset * 1_000_000),
                None => local_to_utc(iso.clock, zone)
                    .ok_or_else(|| MetricsError::Invalid("date value out of range".into())),
            }
        })
        .transpose()
}

pub(crate) fn period(
    column: &str,
    since: Option<&str>,
    until: Option<&str>,
) -> (String, Vec<String>) {
    let (mut clauses, mut params) = (Vec::new(), Vec::new());
    if let Some(s) = since {
        clauses.push(format!("{column} >= ?"));
        params.push(s.to_string());
    }
    if let Some(u) = until {
        clauses.push(format!("{column} < ?"));
        params.push(u.to_string());
    }
    (
        if clauses.is_empty() {
            "1".into()
        } else {
            clauses.join(" AND ")
        },
        params,
    )
}

/// The values of a `period` clause as bound values.
pub(crate) fn text(params: Vec<String>) -> Vec<Bound> {
    params.into_iter().map(Bound::Text).collect()
}

/// The stored-form bounds, and the same widened by one day on each side for the inner per-message GROUP BY.
pub(crate) struct Bounds {
    pub since: Option<String>,
    pub until: Option<String>,
    pub inner_since: Option<String>,
    pub inner_until: Option<String>,
}

pub(crate) fn bounds(since: Option<&str>, until: Option<&str>, zone: &TimeZone) -> Result<Bounds> {
    let (s, u) = (utc_bound(since, zone)?, utc_bound(until, zone)?);
    Ok(Bounds {
        since: s.map(format_stored),
        until: u.map(format_stored),
        inner_since: s.map(|v| format_stored(v - DAY_MICROS)),
        inner_until: u.map(|v| format_stored(v + DAY_MICROS)),
    })
}

pub(crate) fn period_json(b: &Bounds) -> Value {
    json!({"since_utc": b.since, "until_utc": b.until})
}

// ---- shared pieces ----

/// `typed` / `agent` / `unknown` work of one message (FR-039): sidechain -> agent, no entrypoint -> unknown,
/// an `sdk*` entrypoint -> agent, anything else typed.
fn work_kind(sidechain: Option<i64>, entrypoint: Option<&str>) -> usize {
    match (sidechain, entrypoint) {
        (Some(1), _) => 1,
        (_, None) => 2,
        (_, Some(ep)) if ep.starts_with("sdk") => 1,
        _ => 0,
    }
}

fn split_json(split: &[i64; 3]) -> Value {
    json!({"typed": split[0], "agent": split[1], "unknown": split[2]})
}

fn invalid_by() -> MetricsError {
    MetricsError::Invalid(format!("by must be one of {}", DIMENSIONS.join(", ")))
}

// ---- usage_summary ----

/// Sessions, tokens and cache-hit ratio for [since, until); no bound = all history. Usage is counted once
/// per `message.id`, at the message's earliest record time; abstained records feed no metric and are counted.
pub fn usage_summary(conn: &Connection, since: Option<&str>, until: Option<&str>) -> Result<Value> {
    usage_summary_scoped(conn, since, until, Scope::default())
}

/// [`usage_summary`] limited to a workspace (a name: events under its repo paths) and/or an agent account (an id or a
/// label); both given, both must match. An unknown workspace or account is `MetricsError::Invalid` with Python's text.
pub fn usage_summary_scoped(
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
    let mut params = text(params);
    params.extend(scope_params.iter().cloned());
    let sessions: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(DISTINCT session_id) FROM events \
             WHERE abstained = 0 AND session_id IS NOT NULL AND {where_utc} AND {scope_sql}"
        ),
        params_from_iter(&params),
        |r| r.get(0),
    )?;

    let (inner, params) = period("ts_utc", b.inner_since.as_deref(), b.inner_until.as_deref());
    let (where_ts, outer) = period("ts", b.since.as_deref(), b.until.as_deref());
    let mut params = text(params);
    params.extend(scope_params.iter().cloned());
    params.extend(text(outer));
    let (mut messages, mut inp, mut out, mut cache_create, mut cache_read) = (0, 0, 0, 0, 0);
    let mut split = [0_i64; 3];
    let mut entrypoints: BTreeMap<String, i64> = BTreeMap::new();
    let mut stmt = conn.prepare(&format!(
        "SELECT sc, ep, COUNT(*), COALESCE(SUM(i), 0), COALESCE(SUM(o), 0), COALESCE(SUM(cc), 0), \
         COALESCE(SUM(cr), 0) FROM ( \
           SELECT MIN(ts_utc) AS ts, MAX(input_tokens) AS i, MAX(output_tokens) AS o, \
                  MAX(cache_creation_input_tokens) AS cc, MAX(cache_read_input_tokens) AS cr, \
                  MAX(is_sidechain) AS sc, MAX(entrypoint) AS ep \
           FROM events WHERE abstained = 0 AND message_id IS NOT NULL \
           AND input_tokens IS NOT NULL AND {inner} AND {scope_sql} GROUP BY message_id \
         ) WHERE {where_ts} GROUP BY sc, ep"
    ))?;
    let rows = stmt.query_map(params_from_iter(&params), |r| {
        Ok((
            r.get::<_, Option<i64>>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, i64>(3)?,
            r.get::<_, i64>(4)?,
            r.get::<_, i64>(5)?,
            r.get::<_, i64>(6)?,
        ))
    })?;
    for row in rows {
        let (sidechain, entry, n, i, o, cc, cr) = row?;
        messages += n;
        inp += i;
        out += o;
        cache_create += cc;
        cache_read += cr;
        split[work_kind(sidechain, entry.as_deref())] += i + o + cc + cr;
        if let Some(entry) = entry {
            *entrypoints.entry(entry).or_insert(0) += i + o + cc + cr;
        }
    }

    let (where_utc, params) = period("ts_utc", b.since.as_deref(), b.until.as_deref());
    let mut params = text(params);
    params.extend(scope_params.iter().cloned());
    let mut stmt = conn.prepare(&format!(
        "SELECT COALESCE(version, 'unknown'), COUNT(*) FROM events \
         WHERE abstained = 1 AND {where_utc} AND {scope_sql} GROUP BY 1 ORDER BY 1"
    ))?;
    let by_version: BTreeMap<String, i64> = stmt
        .query_map(params_from_iter(&params), |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<std::result::Result<_, _>>()?;

    let prompt_tokens = inp + cache_create + cache_read;
    Ok(json!({
        "period": period_json(&b),
        "sessions": sessions,
        "messages": messages,
        "input_tokens": inp,
        "output_tokens": out,
        "cache_creation_input_tokens": cache_create,
        "cache_read_input_tokens": cache_read,
        "cache_hit_ratio": (prompt_tokens != 0).then(|| round_py(cache_read as f64 / prompt_tokens as f64, 4)),
        "split": split_json(&split),
        "entrypoints": entrypoints,
        "abstained": {"total": by_version.values().sum::<i64>(), "by_version": by_version},
    }))
}

// ---- usage_breakdown ----

struct Group {
    messages: i64,
    tokens: [i64; 4], // input, output, cache creation, cache read
    split: [i64; 3],
    usd: f64,
    priced: i64,
    abstained: BTreeSet<String>,
}

impl Group {
    fn new() -> Self {
        Group {
            messages: 0,
            tokens: [0; 4],
            split: [0; 3],
            usd: 0.0,
            priced: 0,
            abstained: BTreeSet::new(),
        }
    }

    fn add(&mut self, model: Option<&str>, tokens: [i64; 4], usd: Option<f64>, kind: usize) {
        self.messages += 1;
        self.split[kind] += tokens.iter().sum::<i64>();
        for (sum, t) in self.tokens.iter_mut().zip(tokens) {
            *sum += t;
        }
        match usd {
            None => {
                self.abstained.insert(
                    model
                        .filter(|m| !m.is_empty())
                        .unwrap_or("(unknown)")
                        .to_string(),
                );
            }
            Some(usd) => {
                self.usd += usd;
                self.priced += 1;
            }
        }
    }

    fn total_tokens(&self) -> i64 {
        self.tokens.iter().sum()
    }

    fn finish(&self, as_of: &str) -> Map<String, Value> {
        let usd = (self.priced > 0 || self.abstained.is_empty()).then(|| round_py(self.usd, 6));
        let Value::Object(map) = json!({
            "messages": self.messages,
            "input_tokens": self.tokens[0],
            "output_tokens": self.tokens[1],
            "cache_creation_input_tokens": self.tokens[2],
            "cache_read_input_tokens": self.tokens[3],
            "split": split_json(&self.split),
            "api_equivalent_cost": {
                "usd": usd,
                "price_table_date": as_of,
                "abstained_models": self.abstained,
            },
        }) else {
            unreachable!()
        };
        map
    }
}

struct Message {
    ts: Option<String>,
    model: Option<String>,
    project_path: Option<String>,
    tools: Option<String>,
    tokens: [i64; 4],
    sidechain: Option<i64>,
    entrypoint: Option<String>,
}

/// One row per API message in the period (same selection as `usage_summary`), oldest first.
fn messages(conn: &Connection, b: &Bounds, scope: Clause) -> Result<Vec<Message>> {
    let Clause {
        sql: scope_sql,
        params: scope_params,
    } = scope;
    let (inner, params) = period("ts_utc", b.inner_since.as_deref(), b.inner_until.as_deref());
    let (where_ts, outer) = period("ts", b.since.as_deref(), b.until.as_deref());
    let mut params = text(params);
    params.extend(scope_params);
    params.extend(text(outer));
    let mut stmt = conn.prepare(&format!(
        "SELECT ts, model, project_path, tools, i, o, cc, cr, sc, ep FROM ( \
           SELECT MIN(ts_utc) AS ts, MAX(model) AS model, MAX(project_path) AS project_path, \
                  GROUP_CONCAT(tool_names) AS tools, MAX(input_tokens) AS i, \
                  MAX(output_tokens) AS o, MAX(cache_creation_input_tokens) AS cc, \
                  MAX(cache_read_input_tokens) AS cr, MAX(is_sidechain) AS sc, \
                  MAX(entrypoint) AS ep \
           FROM events WHERE abstained = 0 AND message_id IS NOT NULL \
           AND input_tokens IS NOT NULL AND {inner} AND {scope_sql} GROUP BY message_id \
         ) WHERE {where_ts} ORDER BY ts"
    ))?;
    let rows = stmt.query_map(params_from_iter(&params), |r| {
        let n = |i| r.get::<_, Option<i64>>(i).map(Option::unwrap_or_default);
        Ok(Message {
            ts: r.get(0)?,
            model: r.get(1)?,
            project_path: r.get(2)?,
            tools: r.get(3)?,
            tokens: [n(4)?, n(5)?, n(6)?, n(7)?],
            sidechain: r.get(8)?,
            entrypoint: r.get(9)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// The last path component, or the path itself when it has none; `(unknown)` for no path (`_folder`).
pub(crate) fn folder(project_path: Option<&str>) -> String {
    let Some(path) = project_path.filter(|p| !p.is_empty()) else {
        return "(unknown)".into();
    };
    path.rsplit('/')
        .find(|c| !c.is_empty() && *c != ".")
        .unwrap_or(path)
        .to_string()
}

/// A message's group keys for one dimension (`_keys`; `project` is handled by the caller). The `day` key is the
/// calendar day in `zone`, the machine's local zone.
fn keys(by: &str, m: &Message, project: Option<&str>, zone: &TimeZone) -> Result<Vec<String>> {
    Ok(match by {
        "day" => {
            let day =
                m.ts.as_deref()
                    .and_then(parse_iso)
                    .and_then(|ts| local_day(ts, zone))
                    .ok_or_else(|| {
                        MetricsError::Store(StoreError::Unreadable(
                            "a stored timestamp is not ISO 8601".into(),
                        ))
                    })?;
            vec![day]
        }
        "model" => vec![
            m.model
                .as_deref()
                .filter(|v| !v.is_empty())
                .unwrap_or("(unknown)")
                .to_string(),
        ],
        "project" => vec![folder(project)],
        _ => match m.tools.as_deref().filter(|t| !t.is_empty()) {
            Some(tools) => tools
                .split(',')
                .map(str::to_string)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            None => vec!["(none)".into()],
        },
    })
}

/// Most tokens first, ties by key.
fn by_tokens(rows: &mut [(String, Group)]) {
    rows.sort_by(|a, b| {
        (Reverse(a.1.total_tokens()), &a.0).cmp(&(Reverse(b.1.total_tokens()), &b.0))
    });
}

fn row(key: &str, group: &Group, as_of: &str) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("key".into(), json!(key));
    map.extend(group.finish(as_of));
    map
}

/// Tokens per day (local) / model / project (folder name of the owner project, FR-057) / tool for [since, until).
/// Scratch folders (FR-069) are one `project` group, key `Scratch`, `scratch: true`, with `folders`. `total`
/// counts each message once, so it equals `usage_summary`; tool groups overlap. Cost is API-equivalent and
/// uses the shipped price table; a model missing from it abstains.
pub fn usage_breakdown(
    conn: &Connection,
    by: &str,
    since: Option<&str>,
    until: Option<&str>,
) -> Result<Value> {
    usage_breakdown_scoped(conn, by, since, until, Scope::default())
}

/// [`usage_breakdown`] limited to a workspace and/or an account, as [`usage_summary_scoped`]. `by` is checked first.
pub fn usage_breakdown_scoped(
    conn: &Connection,
    by: &str,
    since: Option<&str>,
    until: Option<&str>,
    scope: Scope,
) -> Result<Value> {
    if !DIMENSIONS.contains(&by) {
        return Err(invalid_by());
    }
    let prices = PriceTable::shipped().map_err(MetricsError::Prices)?;
    breakdown(conn, by, since, until, &prices, scope)
}

/// [`usage_breakdown`] with a given price table (Python's `prices=` argument, as a parsed table).
pub fn usage_breakdown_with_prices(
    conn: &Connection,
    by: &str,
    since: Option<&str>,
    until: Option<&str>,
    prices: &PriceTable,
) -> Result<Value> {
    if !DIMENSIONS.contains(&by) {
        return Err(invalid_by());
    }
    breakdown(conn, by, since, until, prices, Scope::default())
}

fn breakdown(
    conn: &Connection,
    by: &str,
    since: Option<&str>,
    until: Option<&str>,
    prices: &PriceTable,
    scope: Scope,
) -> Result<Value> {
    let zone = TimeZone::system(); // read once per call: `TZ`, else the OS zone (T009)
    let b = bounds(since, until, &zone)?;
    let clause =
        scope::resolve(conn, scope)?.clause(conn, b.since.as_deref(), b.until.as_deref())?;
    let pmap = if by == "project" {
        project_map(conn)?
    } else {
        HashMap::new()
    };
    let mut groups: HashMap<String, Group> = HashMap::new();
    let mut folders: HashMap<String, Group> = HashMap::new();
    let mut total = Group::new();
    for m in messages(conn, &b, clause)? {
        let usd = prices.cost(
            m.model.as_deref(),
            m.tokens[0],
            m.tokens[1],
            m.tokens[2],
            m.tokens[3],
        );
        let kind = work_kind(m.sidechain, m.entrypoint.as_deref());
        total.add(m.model.as_deref(), m.tokens, usd, kind);
        let mapped = m
            .project_path
            .as_deref()
            .and_then(|p| pmap.get(p))
            .map(String::as_str);
        let project = if mapped == Some(SCRATCH) {
            folders
                .entry(folder(m.project_path.as_deref()))
                .or_insert_with(Group::new)
                .add(m.model.as_deref(), m.tokens, usd, kind);
            Some(SCRATCH)
        } else {
            mapped.or(m.project_path.as_deref())
        };
        for key in keys(by, &m, project, &zone)? {
            groups.entry(key).or_insert_with(Group::new).add(
                m.model.as_deref(),
                m.tokens,
                usd,
                kind,
            );
        }
    }

    let mut groups: Vec<(String, Group)> = groups.into_iter().collect();
    if by == "day" {
        groups.sort_by(|a, b| a.0.cmp(&b.0));
    } else {
        by_tokens(&mut groups);
    }
    let rows: Vec<Value> = groups
        .iter()
        .map(|(key, group)| {
            let mut map = row(key, group, &prices.as_of);
            if key == SCRATCH && !folders.is_empty() {
                let mut inner: Vec<(String, Group)> =
                    std::mem::take(&mut folders).into_iter().collect();
                by_tokens(&mut inner);
                map.insert("scratch".into(), json!(true));
                map.insert(
                    "folders".into(),
                    Value::Array(
                        inner
                            .iter()
                            .map(|(k, g)| Value::Object(row(k, g, &prices.as_of)))
                            .collect(),
                    ),
                );
            }
            Value::Object(map)
        })
        .collect();
    Ok(json!({
        "by": by,
        "period": period_json(&b),
        "groups": rows,
        "total": total.finish(&prices.as_of),
    }))
}

// ---- owner project (FR-057) and scratch (FR-069), read from `core/owners.py` ----

const IGNORED_BRANCHES: [&str; 4] = ["", "main", "master", "HEAD"];

/// How long a git call may run (Python's `run_git` default `timeout=5`).
const GIT_TIMEOUT: Duration = Duration::from_secs(5);

/// [`run_git_within`] with the default limit, [`GIT_TIMEOUT`].
fn run_git(cwd: &str, args: &[&str]) -> Option<String> {
    run_git_within(cwd, args, GIT_TIMEOUT)
}

/// A read-only git command in `cwd`; its trimmed stdout, or `None` on failure, empty output or timeout.
/// Hardened as Python's `run_git`: no fsmonitor daemon, no optional index lock. After `timeout` the
/// process is killed and the answer is `None`; Python's callers (`owners._lines`, `discovery._git`) treat
/// `TimeoutExpired` the same way, as "no answer". `rules::latest_commit` waits 10 s, everything else 5 s.
pub(crate) fn run_git_within(cwd: &str, args: &[&str], timeout: Duration) -> Option<String> {
    run_git_timed(cwd, args, timeout).ok().flatten()
}

/// A git call that ran past its limit and was killed: Python's `subprocess.TimeoutExpired`.
#[derive(Debug)]
pub(crate) struct GitTimedOut;

/// [`run_git_within`] that tells a timeout (`Err`) from any other failure or empty output (`Ok(None)`), for the
/// callers Python lets see `TimeoutExpired` (`git_local.git_part` answers `abstained`).
pub(crate) fn run_git_timed(
    cwd: &str,
    args: &[&str],
    timeout: Duration,
) -> std::result::Result<Option<String>, GitTimedOut> {
    let Ok(mut child) = Command::new("git")
        .args(["-c", "core.fsmonitor=false", "-C", cwd])
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_LAZY_FETCH", "1") // a partial clone must never make a read fetch over the network (Rust-only hardening)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Ok(None);
    };
    let Some(mut stdout) = child.stdout.take() else {
        return Ok(None);
    };
    let (tx, rx) = mpsc::channel();
    // A reader thread, so a full pipe cannot block git; it is left to finish on its own after a timeout.
    thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        let _ = tx.send(buf);
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            other => {
                let _ = child.kill();
                let _ = child.wait();
                return if matches!(other, Ok(None)) {
                    Err(GitTimedOut)
                } else {
                    Ok(None)
                };
            }
        }
    };
    let buf = match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(buf) => buf,
        Err(mpsc::RecvTimeoutError::Timeout) => return Err(GitTimedOut),
        Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(None),
    };
    let text = String::from_utf8_lossy(&buf).trim().to_string();
    Ok((status.success() && !text.is_empty()).then_some(text))
}

/// The git repo an existing folder is in (a worktree gives its main repo), else `None`.
pub(crate) fn repo_of(folder: &str) -> Option<String> {
    if let Some(common) = run_git(
        folder,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    ) {
        let common = Path::new(&common);
        if common.file_name().is_some_and(|n| n == ".git") {
            return common.parent().map(|p| p.to_string_lossy().into_owned());
        }
    }
    run_git(folder, &["rev-parse", "--show-toplevel"])
}

/// Python's `Path(p).is_dir()`: an empty path is the current folder.
pub(crate) fn is_dir(path: &str) -> bool {
    path.is_empty() || Path::new(path).is_dir()
}

/// Local and remote branch names of a repo, plus branches named in merge commit subjects.
fn repo_branches(repo: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let refs = run_git(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname)",
            "refs/heads",
            "refs/remotes",
        ],
    )
    .unwrap_or_default();
    for name in refs.lines() {
        let parts: Vec<&str> = name.splitn(4, '/').collect();
        if parts.get(1) == Some(&"heads") && parts.len() > 2 {
            names.insert(name.splitn(3, '/').nth(2).unwrap_or_default().to_string());
        } else if parts.len() == 4 {
            names.insert(parts[3].to_string());
        }
    }
    let subjects = run_git(repo, &["log", "--all", "--merges", "--format=%s"]).unwrap_or_default();
    names.extend(subjects.lines().filter_map(merge_branch));
    names
}

/// The branch a merge commit subject names (`Merge branch 'b'`, `Merge pull request #N from user/b`,
/// `Merge <b>: <what it does>`), else `None`. The three forms are tried in that order, as in Python.
pub(crate) fn merge_branch(subject: &str) -> Option<String> {
    merge_branch_quoted(subject)
        .or_else(|| merge_branch_pull_request(subject))
        .or_else(|| merge_branch_bare(subject))
}

fn merge_branch_quoted(subject: &str) -> Option<String> {
    let rest = subject.strip_prefix("Merge branch '")?;
    let end = rest.find('\'').filter(|e| *e > 0)?;
    Some(rest[..end].to_string())
}

fn merge_branch_pull_request(subject: &str) -> Option<String> {
    let rest = subject.strip_prefix("Merge pull request #")?;
    let after_digits = rest.trim_start_matches(|c: char| c.is_ascii_digit());
    if after_digits.len() == rest.len() {
        return None;
    }
    let from = after_digits.strip_prefix(" from ")?;
    let user_len = from.find(|c: char| c == '/' || c.is_whitespace())?;
    let branch = from[user_len..]
        .strip_prefix('/')
        .filter(|_| user_len > 0)?;
    let branch: String = branch.chars().take_while(|c| !c.is_whitespace()).collect();
    (!branch.is_empty()).then_some(branch)
}

fn merge_branch_bare(subject: &str) -> Option<String> {
    let rest = subject.strip_prefix("Merge ")?;
    if ["branch ", "pull request ", "remote-tracking "]
        .iter()
        .any(|p| rest.starts_with(p))
    {
        return None;
    }
    let branch: String = rest
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != ':' && *c != '\'')
        .collect();
    (!branch.is_empty()).then_some(branch)
}

/// Stored project path -> the git branches of its events (`""` for none), in path order.
pub(crate) fn stored_branches(
    conn: &Connection,
) -> rusqlite::Result<BTreeMap<String, BTreeSet<String>>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT project_path, git_branch FROM events WHERE project_path IS NOT NULL",
    )?;
    let mut map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for row in stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
    })? {
        let (path, branch) = row?;
        map.entry(path)
            .or_default()
            .insert(branch.unwrap_or_default());
    }
    Ok(map)
}

/// Rule 1: the repo of the nearest existing parent folder of a gone path.
fn repo_of_parent(path: &str, memo: &mut HashMap<String, Option<String>>) -> Option<String> {
    if !Path::new(path).is_absolute() {
        return None;
    }
    let parent = Path::new(path)
        .ancestors()
        .skip(1)
        .find(|p| p.is_dir())?
        .to_string_lossy()
        .into_owned();
    let owner = memo
        .entry(parent.clone())
        .or_insert_with(|| repo_of(&parent))
        .clone();
    owner.filter(|o| Path::new(o).is_dir())
}

/// Stored gone project path -> its owner project path; only the paths that have an owner.
pub(crate) fn owner_map(branches: &BTreeMap<String, BTreeSet<String>>) -> HashMap<String, String> {
    let mut mapping = HashMap::new();
    let (mut parents, mut repo_names): (
        HashMap<String, Option<String>>,
        HashMap<String, BTreeSet<String>>,
    ) = (HashMap::new(), HashMap::new());
    let mut candidates: Option<Vec<&String>> = None;
    for (path, stored) in branches.iter().filter(|(p, _)| !is_dir(p)) {
        let mut owner = repo_of_parent(path, &mut parents);
        let wanted: Vec<&String> = stored
            .iter()
            .filter(|b| !IGNORED_BRANCHES.contains(&b.as_str()))
            .collect();
        if owner.is_none() && !wanted.is_empty() {
            let repos = candidates.get_or_insert_with(|| {
                branches
                    .keys()
                    .filter(|p| Path::new(p).join(".git").exists())
                    .collect()
            });
            let mut matches: Vec<&String> = repos
                .iter()
                .copied()
                .filter(|repo| {
                    let names = repo_names
                        .entry((*repo).clone())
                        .or_insert_with(|| repo_branches(repo));
                    wanted.iter().any(|w| names.contains(*w))
                })
                .collect();
            if matches.len() > 1 {
                // a tie: a repo inside a hidden folder (a clone) loses to a visible one
                matches.retain(|r| {
                    !r.split('/')
                        .any(|part| part.starts_with('.') && part != ".")
                });
            }
            owner = (matches.len() == 1).then(|| matches[0].clone());
        }
        if let Some(owner) = owner {
            mapping.insert(path.clone(), owner);
        }
    }
    mapping
}

/// What is left of a path to resolve: a name to look at, or the end of a link's target (that link is now resolved).
enum Step {
    Name(OsString),
    Resolved(PathBuf),
}

/// The one port of Python's non-strict `Path.resolve()` / `os.path.realpath` (CPython 3.14's `posixpath.realpath`; used by
/// the metrics' scratch rule, the easy-memory and memory-graph reads, the folder list, the account paths and the workspace
/// repos): the path is walked name by name; `.` and `..` are folded (`..` goes up from where the walk is, after any link
/// before it has been replaced by its target); a symlink is followed, a dangling one by its text too; a name that does not
/// exist is kept as typed. A relative path starts at the working folder. There is no cap on links. `seen` maps a link to its
/// fully resolved target (`None` while it is being resolved), so a link met again is read from there, not expanded again (a
/// chain that doubles at every link costs one expansion per link), and a link met while it is being resolved is a loop: it is
/// kept as it is, with the rest of the path as typed.
pub fn resolve_path(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };
    let names = |p: &Path| -> Vec<OsString> {
        p.components()
            .filter_map(|c| match c {
                Component::Normal(n) => Some(n.to_os_string()),
                Component::ParentDir => Some("..".into()),
                _ => None,
            })
            .collect()
    };
    // the next name to look at is the last one
    let mut rest: Vec<Step> = names(&absolute).into_iter().rev().map(Step::Name).collect();
    let mut resolved = PathBuf::from("/");
    let mut seen: HashMap<PathBuf, Option<PathBuf>> = HashMap::new();
    while let Some(step) = rest.pop() {
        let name = match step {
            Step::Resolved(link) => {
                seen.insert(link, Some(resolved.clone()));
                continue;
            }
            Step::Name(name) => name,
        };
        if name == ".." {
            resolved.pop();
            continue;
        }
        let next = resolved.join(&name);
        if !std::fs::symlink_metadata(&next).is_ok_and(|m| m.file_type().is_symlink()) {
            resolved = next;
            continue;
        }
        if let Some(done) = seen.get(&next) {
            // resolved before: its result; still being resolved: a loop, kept as it is
            resolved = done.clone().unwrap_or(next);
            continue;
        }
        let Ok(target) = std::fs::read_link(&next) else {
            resolved = next;
            continue;
        };
        if target.is_absolute() {
            resolved = PathBuf::from("/");
        }
        seen.insert(next.clone(), None);
        rest.push(Step::Resolved(next));
        rest.extend(names(&target).into_iter().rev().map(Step::Name));
    }
    resolved
}

/// [`resolve_path`] over text: a name that is not UTF-8 comes back with U+FFFD.
pub fn realpath(path: &str) -> String {
    resolve_path(Path::new(path)).to_string_lossy().into_owned()
}

/// FR-069 (a)-(d) for a path with no owner project; a path under a temp folder is decided before any git call.
fn is_scratch(path: &str) -> bool {
    if path.is_empty() || !Path::new(path).is_absolute() {
        return true;
    }
    let real = realpath(path);
    let roots = [
        realpath(&std::env::temp_dir().to_string_lossy()),
        realpath("/tmp"),
    ];
    if roots
        .iter()
        .any(|root| real == *root || real.starts_with(&format!("{}/", root.trim_end_matches('/'))))
    {
        return true; // (a)
    }
    if path
        .split('/')
        .any(|part| part.starts_with('.') && part != ".")
    {
        return true; // (d)
    }
    if !is_dir(path) {
        return true; // (b)
    }
    repo_of(path).is_none() // (c)
}

/// Every stored project path -> its project key: the owner repo, else `Scratch`, else the path.
pub(crate) fn project_map(conn: &Connection) -> rusqlite::Result<HashMap<String, String>> {
    let branches = stored_branches(conn)?;
    let owners = owner_map(&branches);
    Ok(branches
        .keys()
        .map(|p| {
            let key = match owners.get(p) {
                Some(owner) => owner.clone(),
                None if is_scratch(p) => SCRATCH.to_string(),
                None => p.clone(),
            };
            (p.clone(), key)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::merge_branch;

    /// Expected values are the output of Python's `owners.merge_branch` for the same subjects.
    #[test]
    fn merge_branch_matches_python() {
        let cases = [
            ("Merge branch 'feat-a' into main", Some("feat-a")),
            ("Merge branch ''", None),
            ("Merge branch 'x", None),
            ("Merge pull request #12 from user/feat-b", Some("feat-b")),
            ("Merge pull request #12 from user/", None),
            ("Merge pull request # from u/b", None),
            ("Merge pull request #7 from user", None),
            ("Merge HUN-5: adds thing", Some("HUN-5")),
            ("Merge HUN-5", Some("HUN-5")),
            ("Merge remote-tracking branch 'o/x'", None),
            ("Merge  x", None),
            ("Merge :x", None),
            ("Merge a'b", Some("a")),
            ("Merge branch", Some("branch")),
            ("Fix thing", None),
        ];
        for (subject, want) in cases {
            assert_eq!(merge_branch(subject).as_deref(), want, "{subject:?}");
        }
    }
}
