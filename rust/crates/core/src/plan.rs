//! The improvement plan (FR-007): the fired rules grouped by priority, each with its fix line. Ported from
//! `src/easy_personal/core/plan.py`; it runs `rules::evaluate` and reshapes the hits, so it reads the store only
//! through that function (read-only, clock as a parameter).
//!
//! The optional `workspace` / `account` scope (T017) is [`improvement_plan_scoped`], which runs
//! `rules::evaluate_scoped`; [`improvement_plan`] is the same with no scope.

use crate::rules::{Rule, RulesError, ScopedRulesError, evaluate, evaluate_scoped};
use crate::scope::Scope;
use rusqlite::Connection;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

/// The keys Python copies from every hit.
const COPIED: [&str; 11] = [
    "rule_id",
    "label",
    "measured",
    "threshold",
    "fix",
    "title",
    "why",
    "steps",
    "summary",
    "causes",
    "diagnosis",
];

/// The keys copied only when the hit has them.
const OPTIONAL: [&str; 2] = ["detail", "cache_read_share"];

/// `{"groups": [{"priority": p, "items": [...]}]}`: groups in ascending priority, items in the order
/// `rules::evaluate` returns the hits. No fired rule gives `{"groups": []}`.
pub fn improvement_plan(
    conn: &Connection,
    now_micros: i64,
    rules: &[Rule],
) -> Result<Value, RulesError> {
    Ok(group_by_priority(&evaluate(conn, now_micros, rules)?))
}

/// [`improvement_plan`] limited to a workspace and/or an account, as [`evaluate_scoped`]; an unknown one is
/// `ScopedRulesError::UnknownScope` with Python's text.
pub fn improvement_plan_scoped(
    conn: &Connection,
    now_micros: i64,
    rules: &[Rule],
    scope: Scope,
) -> Result<Value, ScopedRulesError> {
    Ok(group_by_priority(&evaluate_scoped(
        conn, now_micros, rules, scope,
    )?))
}

/// The hits of a warnings answer, grouped by priority.
fn group_by_priority(warnings: &Value) -> Value {
    let mut groups: BTreeMap<i64, Vec<Value>> = BTreeMap::new();
    for hit in warnings["hits"].as_array().into_iter().flatten() {
        let mut item = Map::new();
        for key in COPIED.iter().chain(OPTIONAL.iter()) {
            if let Some(value) = hit.get(*key) {
                item.insert((*key).to_string(), value.clone());
            }
        }
        groups
            .entry(hit["priority"].as_i64().unwrap_or(0))
            .or_default()
            .push(Value::Object(item));
    }
    json!({
        "groups": groups
            .into_iter()
            .map(|(priority, items)| json!({"priority": priority, "items": items}))
            .collect::<Vec<_>>()
    })
}
