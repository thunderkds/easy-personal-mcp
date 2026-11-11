//! Saved investigate reports, read side (Python: `core/reports.py`, `list_reports` and `get_report`; FR-076..FR-078).
//!
//! The owner's own session saved the text; this module only reads it back, together with the server's rule snapshot taken
//! at save time. "Now" for the before / now line is worked out on every read, with the clock passed in, and never stored.
//! Read-only: the connection is never written to.
//!
//! The report text (`summary`, `fixes`) is shown as stored: it is the one text the store keeps. It is never logged here,
//! and no error of this module carries it, a row value or a path. Where Python dies on a stored value it cannot read (a
//! snapshot that is not JSON or lacks a key: a 500 with a traceback) this returns [`ReportsError::Corrupt`]; the list keeps
//! such a report with `fired_count` 0 instead of failing the whole list.

use crate::rules::{self, Rule, RulesError, ScopedRulesError};
use crate::scope::Scope;
use crate::store::StoreError;
use rusqlite::{Connection, params};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::fmt;

/// `LIST_LIMIT`: the newest reports the list shows.
const LIST_LIMIT: i64 = 50;
const NOT_MEASURED: &str = "not measured now";
const NOT_IN_USE_REASON: &str = "not in use";

#[derive(Debug)]
pub enum ReportsError {
    /// A stored snapshot or fixes value is not what `save` writes. Names nothing of it.
    Corrupt,
    Rules(RulesError),
    Store(StoreError),
}

impl fmt::Display for ReportsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReportsError::Corrupt => write!(f, "the saved report cannot be read"),
            ReportsError::Rules(e) => write!(f, "{e}"),
            ReportsError::Store(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ReportsError {}

impl From<rusqlite::Error> for ReportsError {
    fn from(e: rusqlite::Error) -> Self {
        ReportsError::Store(StoreError::from(e))
    }
}

impl From<RulesError> for ReportsError {
    fn from(e: RulesError) -> Self {
        ReportsError::Rules(e)
    }
}

type Result<T> = std::result::Result<T, ReportsError>;

/// `{"reports": [...]}`: the newest 50 reports, newest first, each `{id, saved_utc, workspace, account, fired_count,
/// summary_first_line}`.
pub fn list_reports(conn: &Connection) -> Result<Value> {
    let mut statement = conn.prepare(
        "SELECT id, saved_utc, workspace, account, summary, snapshot_json FROM investigate_reports \
         ORDER BY id DESC LIMIT ?",
    )?;
    let rows = statement.query_map(params![LIST_LIMIT], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
        ))
    })?;
    let mut reports = Vec::new();
    for row in rows {
        let (id, saved_utc, workspace, account, summary, snapshot) = row?;
        reports.push(json!({
            "id": id,
            "saved_utc": saved_utc,
            "workspace": workspace,
            "account": account,
            "fired_count": fired_count(&snapshot),
            "summary_first_line": first_line(&summary),
        }));
    }
    Ok(json!({ "reports": reports }))
}

/// Rules of the snapshot's `checked` list that fired; 0 for a snapshot that is not what `save` writes.
fn fired_count(snapshot: &str) -> usize {
    serde_json::from_str::<Value>(snapshot)
        .ok()
        .and_then(|s| s.get("checked").and_then(Value::as_array).cloned())
        .map_or(0, |checked| {
            checked
                .iter()
                .filter(|r| r.get("fired").and_then(Value::as_bool) == Some(true))
                .count()
        })
}

/// `(text.splitlines() or [""])[0]`: up to the first line boundary of Python's `str.splitlines`.
fn first_line(text: &str) -> &str {
    let end = text
        .find(|c: char| {
            matches!(
                c,
                '\n' | '\r'
                    | '\u{0b}'
                    | '\u{0c}'
                    | '\u{1c}'
                    | '\u{1d}'
                    | '\u{1e}'
                    | '\u{85}'
                    | '\u{2028}'
                    | '\u{2029}'
            )
        })
        .unwrap_or(text.len());
    &text[..end]
}

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value> {
    value.get(key).ok_or(ReportsError::Corrupt)
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    field(value, key)?.as_str().ok_or(ReportsError::Corrupt)
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    field(value, key)?.as_array().ok_or(ReportsError::Corrupt)
}

/// `{"state": ..., "measured": ..., "threshold": ..., "fired": ...}` of one `checked` entry.
fn checked_state(rule: &Value) -> Result<Map<String, Value>> {
    let fired = field(rule, "fired")?;
    let mut state = Map::new();
    state.insert(
        "state".into(),
        json!(if truthy(fired) { "fired" } else { "clear" }),
    );
    for key in ["measured", "threshold", "fired"] {
        state.insert(key.into(), field(rule, key)?.clone());
    }
    Ok(state)
}

/// Python truthiness of a value read from JSON.
fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// A fired entry saved before FR-079 has no `label`: fill what the rule pack says now, never `steps`.
fn fill_from_pack(entry: &Map<String, Value>, current: &[Rule]) -> Map<String, Value> {
    let mut entry = entry.clone();
    if entry.contains_key("label") {
        return entry;
    }
    let id = entry.get("rule_id").and_then(Value::as_str);
    if let Some(rule) = current.iter().find(|r| Some(r.id.as_str()) == id) {
        entry.insert("label".into(), json!(rule.label));
        entry.insert("priority".into(), json!(rule.priority));
        entry.insert("why".into(), json!(rule.why));
        entry.insert("fix".into(), json!(rule.steps.join(" ")));
        entry.insert("window_days".into(), json!(rule.window_days));
    }
    entry
}

/// A rule's title and its state on one side of the before / now line.
type Saved = (String, Map<String, Value>);

/// The saved side: one `(title, state)` per rule id of the snapshot (`checked`, then `abstained` over it).
fn before(snapshot: &Value, current: &[Rule]) -> Result<BTreeMap<String, Saved>> {
    let mut fired: BTreeMap<&str, &Map<String, Value>> = BTreeMap::new();
    for entry in array(snapshot, "fired")? {
        let object = entry.as_object().ok_or(ReportsError::Corrupt)?;
        fired.insert(text(entry, "rule_id")?, object);
    }
    // like Python, the pack is consulted only when some saved entry predates FR-079 (no `label`)
    let pack: &[Rule] = if fired.values().any(|f| !f.contains_key("label")) {
        current
    } else {
        &[]
    };
    let mut out = BTreeMap::new();
    for rule in array(snapshot, "checked")? {
        let id = text(rule, "rule_id")?;
        let mut state = checked_state(rule)?;
        if truthy(field(rule, "fired")?)
            && let Some(entry) = fired.get(id)
        {
            state.extend(fill_from_pack(entry, pack));
        }
        out.insert(id.to_string(), (text(rule, "title")?.to_string(), state));
    }
    for rule in array(snapshot, "abstained")? {
        let mut state = Map::new();
        state.insert("state".into(), json!("abstained"));
        state.insert("reason".into(), field(rule, "reason")?.clone());
        out.insert(
            text(rule, "rule_id")?.to_string(),
            (text(rule, "title")?.to_string(), state),
        );
    }
    Ok(out)
}

/// The measured-now side, one state per rule id of the fresh evaluation.
fn now_states(found: &Value) -> Result<BTreeMap<String, Value>> {
    let mut summaries: BTreeMap<&str, &Value> = BTreeMap::new();
    for hit in array(found, "hits")? {
        summaries.insert(text(hit, "rule_id")?, field(hit, "summary")?);
    }
    let mut out = BTreeMap::new();
    for rule in array(found, "checked")? {
        let id = text(rule, "rule_id")?;
        let mut state = checked_state(rule)?;
        if let Some(summary) = summaries.get(id) {
            state.insert("summary".into(), (*summary).clone());
        }
        out.insert(id.to_string(), Value::Object(state));
    }
    for rule in array(found, "abstained")? {
        out.insert(
            text(rule, "rule_id")?.to_string(),
            json!({"state": "abstained", "reason": field(rule, "reason")?}),
        );
    }
    Ok(out)
}

/// One report with a before / now entry per rule of its snapshot, or `None` for an unknown id.
///
/// Before = the saved snapshot; now = the same rule measured on this call for the same scope (never stored). A rule missing
/// from the rule pack says `not measured now`; a workspace or account deleted since the save makes every rule
/// `not measured now`. Each side keeps its own threshold.
pub fn get_report(conn: &Connection, report_id: i64, now_micros: i64) -> Result<Option<Value>> {
    let row = conn
        .query_row(
            "SELECT id, saved_utc, server_version, workspace, account, summary, fixes_json, snapshot_json \
             FROM investigate_reports WHERE id = ?",
            params![report_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        )
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?;
    let Some((id, saved_utc, server_version, workspace, account, summary, fixes, snapshot)) = row
    else {
        return Ok(None);
    };
    let snapshot: Value = serde_json::from_str(&snapshot).map_err(|_| ReportsError::Corrupt)?;
    let fixes: Value = serde_json::from_str(&fixes).map_err(|_| ReportsError::Corrupt)?;
    let pack = rules::shipped_rules()?;
    let scope = Scope {
        workspace: workspace.as_deref(),
        account: account.as_deref(),
    };
    // the saved workspace or account no longer exists: nothing is measured now
    let found = match rules::evaluate_checked_scoped(conn, now_micros, &pack, scope) {
        Ok(found) => Some(found),
        Err(ScopedRulesError::UnknownScope(_)) => None,
        Err(ScopedRulesError::Rules(e)) => return Err(e.into()),
    };
    let mut after = match &found {
        Some(found) => now_states(found)?,
        None => BTreeMap::new(),
    };
    // a rule `evaluate` left out because its kit part is not in use: `not in use` now (FR-080)
    let mut not_in_use = Vec::new();
    if found.is_some() {
        match rules::left_out_ids(conn, now_micros, &pack, scope) {
            Ok(ids) => not_in_use = ids,
            Err(ScopedRulesError::UnknownScope(_)) => {}
            Err(ScopedRulesError::Rules(e)) => return Err(e.into()),
        }
    }
    for id in &not_in_use {
        after.insert(
            id.clone(),
            json!({"state": "not_in_use", "reason": NOT_IN_USE_REASON}),
        );
    }
    let missing = json!({"state": "not_measured", "reason": NOT_MEASURED});
    let mut rows = Vec::new();
    for (rule_id, (title, saved)) in before(&snapshot, &pack)? {
        let abstained = saved.get("state").and_then(Value::as_str) == Some("abstained");
        if abstained && not_in_use.contains(&rule_id) {
            continue; // nothing worth showing (FR-080)
        }
        let now = after.get(&rule_id).unwrap_or(&missing).clone();
        rows.push(json!({"rule_id": rule_id, "title": title, "before": saved, "now": now}));
    }
    Ok(Some(json!({
        "id": id,
        "saved_utc": saved_utc,
        "server_version": server_version,
        "workspace": workspace,
        "account": account,
        "summary": summary,
        "fixes": fixes,
        "as_of_utc": field(&snapshot, "as_of_utc")?,
        "now_as_of_utc": found.as_ref().map(|f| f["as_of_utc"].clone()),
        "rules": rows,
    })))
}

#[cfg(test)]
mod tests {
    use super::first_line;

    /// `(text.splitlines() or [""])[0]` for every boundary Python knows.
    #[test]
    fn the_first_line_ends_at_pythons_line_boundaries() {
        assert_eq!(first_line(""), "");
        assert_eq!(first_line("one"), "one");
        assert_eq!(first_line("one\ntwo"), "one");
        assert_eq!(first_line("one\r\ntwo"), "one");
        assert_eq!(first_line("\nrest"), "");
        for sep in [
            "\r", "\u{0b}", "\u{0c}", "\u{1c}", "\u{1d}", "\u{1e}", "\u{85}", "\u{2028}",
            "\u{2029}",
        ] {
            assert_eq!(first_line(&format!("a{sep}b")), "a", "{sep:?}");
        }
        // not boundaries: a tab, a no-break space, a zero-width space
        assert_eq!(first_line("a\tb\u{a0}c\u{200b}d"), "a\tb\u{a0}c\u{200b}d");
    }
}
