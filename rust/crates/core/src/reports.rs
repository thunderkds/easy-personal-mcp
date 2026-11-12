//! Saved investigate reports (Python: `core/reports.py`; FR-076..FR-078). The read side is `list_reports` and `get_report`;
//! the one write is [`save`] (T047), which opens its own write handle for that one call (see its doc).
//!
//! Read side:
//! The owner's own session saved the text; this module only reads it back, together with the server's rule snapshot taken
//! at save time. "Now" for the before / now line is worked out on every read, with the clock passed in, and never stored.
//! Read-only: the connection is never written to.
//!
//! The report text (`summary`, `fixes`) is shown as stored: it is the one text the store keeps. It is never logged here,
//! and no error of this module carries it, a row value or a path. Where Python dies on a stored value it cannot read (a
//! snapshot that is not JSON or lacks a key: a 500 with a traceback) this returns [`ReportsError::Corrupt`]; the list keeps
//! such a report with `fired_count` 0 instead of failing the whole list.

use crate::rules::{self, Rule, RulesError, ScopedRulesError};
use crate::scope::{self, Scope, ScopeError};
use crate::secret::looks_secret;
use crate::store::{KNOWN_MIGRATION, StoreError, open_checked};
use crate::writer::{WriteError, open_existing_for_write};
use rusqlite::{Connection, TransactionBehavior, params};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

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

/// Rules of the snapshot's `checked` list that fired; 0 for a snapshot that is not what `save` writes, and that includes
/// one where any `checked` entry has no boolean `fired` (the whole report counts 0, never a partial number).
fn fired_count(snapshot: &str) -> usize {
    let Some(checked) = serde_json::from_str::<Value>(snapshot)
        .ok()
        .and_then(|s| s.get("checked").and_then(Value::as_array).cloned())
    else {
        return 0;
    };
    let mut fired = 0;
    for rule in &checked {
        match rule.get("fired").and_then(Value::as_bool) {
            Some(true) => fired += 1,
            Some(false) => {}
            None => return 0,
        }
    }
    fired
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

// ---- the write side (T047) ----

const SUMMARY_MAX: usize = 2000;
const FIXES_MAX: usize = 5;
const FIX_TEXT_MAX: usize = 300;
/// `CITE_MAX`: `tool` and `number` of a fix are short names, not prose.
const CITE_MAX: usize = 100;
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Why a report was not saved. Both texts are fixed: they name a field or a state, never a value, SQL or a path (NFR-002).
#[derive(Debug, PartialEq)]
pub enum SaveError {
    /// Python's `ValueError` text, byte for byte (ends `; nothing was saved`): a limit, a secret, a type, an unknown scope.
    Refused(String),
    /// The store could not be read or written: busy, read-only, not v11, missing.
    Failed(String),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::Refused(text) | SaveError::Failed(text) => write!(f, "{text}"),
        }
    }
}

impl std::error::Error for SaveError {}

/// What a saved report is called by: its row id and the second it was saved (UTC, `%Y-%m-%dT%H:%M:%SZ`).
#[derive(Debug, PartialEq)]
pub struct SavedReport {
    pub id: i64,
    pub saved_utc: String,
}

/// `_check_text`: a text of 1 to `limit` characters (Python's `len`) with something other than white space, no secret.
fn check_text(field: &str, value: &Value, limit: usize) -> std::result::Result<String, String> {
    let Some(text) = value.as_str() else {
        return Err(format!("{field}: must be text; nothing was saved"));
    };
    // Python's `str.strip()`: also the C0 separators `\x1c`..`\x1f`
    if text
        .trim_matches(|c: char| c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c))
        .is_empty()
    {
        return Err(format!("{field}: must not be empty; nothing was saved"));
    }
    let length = text.chars().count();
    if length > limit {
        return Err(format!(
            "{field}: {length} characters, the limit is {limit}; nothing was saved"
        ));
    }
    if looks_secret(text) {
        return Err(format!(
            "{field}: looks like a secret (a key, token or private-key header); nothing was saved"
        ));
    }
    Ok(text.to_string())
}

/// The clean `(summary, fixes)`, or the refusal text naming the failing field; the value is never echoed. Python's
/// `validate`: every limit, type and key rule, in the same order.
pub fn validate(
    summary: &Value,
    fixes: &Value,
) -> std::result::Result<(String, Vec<Value>), String> {
    let summary = check_text("summary", summary, SUMMARY_MAX)?;
    let Some(fixes) = fixes.as_array() else {
        return Err("fixes: must be a list; nothing was saved".into());
    };
    if fixes.len() > FIXES_MAX {
        return Err(format!(
            "fixes: {} items, the limit is {FIXES_MAX}; nothing was saved",
            fixes.len()
        ));
    }
    let mut clean = Vec::new();
    for (n, fix) in fixes.iter().enumerate() {
        let name = format!("fixes[{n}]");
        let object = fix.as_object().filter(|o| {
            o.contains_key("text")
                && o.keys()
                    .all(|k| matches!(k.as_str(), "text" | "tool" | "number"))
        });
        let Some(object) = object else {
            return Err(format!(
                "{name}: must be an object with `text` and optional `tool`, `number`; nothing was saved"
            ));
        };
        let mut item = Map::new();
        let text = check_text(&format!("{name}.text"), &object["text"], FIX_TEXT_MAX)?;
        item.insert("text".into(), json!(text));
        // an optional field sent as "" (models often fill optional arguments that way) counts as absent
        match object.get("tool") {
            None | Some(Value::Null) => {}
            Some(Value::String(s)) if s.is_empty() => {}
            Some(tool) => {
                let tool = check_text(&format!("{name}.tool"), tool, CITE_MAX)?;
                item.insert("tool".into(), json!(tool));
            }
        }
        match object.get("number") {
            None | Some(Value::Null) => {}
            Some(Value::String(s)) if s.is_empty() => {}
            Some(number @ (Value::String(_) | Value::Number(_))) => {
                if number.is_string() {
                    check_text(&format!("{name}.number"), number, CITE_MAX)?;
                }
                item.insert("number".into(), number.clone());
            }
            Some(_) => {
                return Err(format!(
                    "{name}.number: must be a number or text; nothing was saved"
                ));
            }
        }
        clean.push(Value::Object(item));
    }
    Ok((summary, clean))
}

/// Python's `json.dumps(value, sort_keys=True)` (default separators, `ensure_ascii`), so a row written here is the row
/// Python writes, byte for byte.
fn dumps(value: &Value) -> String {
    let mut out = String::new();
    write_value(value, &mut out);
    out
}

fn write_value(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) if n.is_f64() => out.push_str(&float_repr(n.as_f64().unwrap_or(0.0))),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => write_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_value(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            // Python sorts keys by code point; UTF-16 order differs only above U+FFFF, so compare code points
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (i, key) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_string(key, out);
                out.push_str(": ");
                write_value(&map[key], out);
            }
            out.push('}');
        }
    }
}

fn write_string(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || (c as u32) >= 0x7f => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Python's `float.__repr__`: the shortest text that reads back, `1e-07` and `1e+16` with a signed two-digit exponent.
fn float_repr(x: f64) -> String {
    let text = format!("{x:?}");
    match text.split_once('e') {
        Some((mantissa, exponent)) => {
            let exponent: i32 = exponent.parse().unwrap_or(0);
            let sign = if exponent < 0 { '-' } else { '+' };
            format!("{mantissa}e{sign}{:02}", exponent.abs())
        }
        None => text,
    }
}

fn failed(error: impl fmt::Display) -> SaveError {
    SaveError::Failed(error.to_string())
}

/// Refuse an unknown workspace or account with the field named (the value is not echoed), as `_check_scope`.
fn check_scope(
    conn: &Connection,
    workspace: Option<&str>,
    account: Option<&str>,
) -> std::result::Result<(), SaveError> {
    let one = |field: &str, scope: Scope| match scope::resolve(conn, scope) {
        Ok(_) => Ok(()),
        Err(ScopeError::Unknown(_)) => Err(SaveError::Refused(format!(
            "{field}: unknown {field}; nothing was saved"
        ))),
        Err(ScopeError::Store(e)) => Err(failed(e)),
    };
    if let Some(w) = workspace {
        let scope = Scope {
            workspace: Some(w),
            account: None,
        };
        one("workspace", scope)?;
    }
    if let Some(a) = account {
        let scope = Scope {
            workspace: None,
            account: Some(a),
        };
        one("account", scope)?;
    }
    Ok(())
}

/// Store one report with the server's rule snapshot for its scope; return its id and save time.
///
/// The order is fixed and nothing is written before the last step: (1) `validate`; (2) the store is opened **read-only**,
/// must be at schema v11, and the scope is checked; (3) every rule is measured now for the scope (the shipped pack, as
/// `/api/warnings`; the `config.yaml` override is not ported); (4) only then is a write handle opened, for the one `INSERT`
/// in one transaction (busy timeout 5 s, the pragmas of `store.connect`; no `ingest.lock`: Python takes none), and closed
/// before this returns. Any refusal leaves the file untouched. `now_micros` is the injected clock (UTC).
pub fn save(
    path: &Path,
    summary: &Value,
    fixes: &Value,
    workspace: Option<&str>,
    account: Option<&str>,
    now_micros: i64,
) -> std::result::Result<SavedReport, SaveError> {
    let (summary, fixes) = validate(summary, fixes).map_err(SaveError::Refused)?;
    let workspace = workspace.filter(|s| !s.is_empty());
    let account = account.filter(|s| !s.is_empty());
    let snapshot = {
        let conn = open_checked(path).map_err(failed)?;
        let found: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(|e| failed(StoreError::from(e)))?;
        if found != KNOWN_MIGRATION {
            return Err(failed(WriteError::Older { found }));
        }
        check_scope(&conn, workspace, account)?;
        let pack = rules::shipped_rules().map_err(failed)?;
        let scope = Scope { workspace, account };
        let found =
            rules::evaluate_checked_scoped(&conn, now_micros, &pack, scope).map_err(failed)?;
        json!({
            "as_of_utc": found["as_of_utc"],
            "checked": found["checked"],
            "fired": found["hits"], // the whole warning hit: the page draws the Warnings & plan card from it (FR-079)
            "abstained": found["abstained"],
        })
    };
    let saved_utc = jiff::Timestamp::from_second(now_micros.div_euclid(1_000_000))
        .map_err(|_| SaveError::Failed("the clock is out of range".into()))?
        .strftime("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let store = open_existing_for_write(path).map_err(failed_write)?; // never creates: a store removed since the check stays gone
    let id = {
        let tx = rusqlite::Transaction::new_unchecked(store.conn(), TransactionBehavior::Immediate)
            .map_err(|e| failed_write(e.into()))?;
        tx.execute(
            "INSERT INTO investigate_reports (saved_utc, server_version, workspace, account, summary, fixes_json, \
             snapshot_json) VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                saved_utc,
                SERVER_VERSION,
                workspace,
                account,
                summary,
                dumps(&Value::Array(fixes)),
                dumps(&snapshot)
            ],
        )
        .map_err(|e| failed_write(e.into()))?;
        let id = tx.last_insert_rowid();
        tx.commit().map_err(|e| failed_write(e.into()))?;
        id
    };
    drop(store); // closed before the answer is built
    Ok(SavedReport { id, saved_utc })
}

/// A store the OS will not let this process write is said so; every other failure keeps its fixed text.
fn failed_write(error: WriteError) -> SaveError {
    match error {
        WriteError::Sqlite(code) if code == "ReadOnly" || code == "CannotOpen" => {
            SaveError::Failed("the store is read-only; nothing was saved".into())
        }
        other => failed(other),
    }
}

#[cfg(test)]
mod tests {
    use super::{fired_count, first_line};

    /// One `checked` entry with no boolean `fired` makes the whole snapshot "not what `save` writes": 0, never a partial count.
    #[test]
    fn a_malformed_checked_entry_makes_the_whole_count_zero() {
        let count = fired_count;
        assert_eq!(
            count(r#"{"checked":[{"fired":true},{"fired":false},{"fired":true}]}"#),
            2
        );
        assert_eq!(count(r#"{"checked":[]}"#), 0);
        assert_eq!(count(r#"{"checked":[{"fired":true},{}]}"#), 0);
        assert_eq!(count(r#"{"checked":[{},{"fired":true}]}"#), 0);
        assert_eq!(count(r#"{"checked":[{"fired":true},{"fired":1}]}"#), 0);
        assert_eq!(count(r#"{"checked":[{"fired":true},"x"]}"#), 0);
        assert_eq!(count(r#"{"checked":{"fired":true}}"#), 0);
        assert_eq!(count(r#"{}"#), 0);
        assert_eq!(count("not json"), 0);
    }

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
