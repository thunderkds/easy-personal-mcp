//! The sync plan, read part of `src/easy_personal/connectors/sync.py` (`Step`, `plan`, `plan_all`) and of the
//! `_sync_account` / `sync_plan` handler of `surfaces/api.py`, served as `GET /api/sync` (T041).
//!
//! `plan` lists what applying the store's picks would change in each Claude account's user-scope MCP entries. This port
//! **never starts a process and never asks the keychain**: Python asks `shutil.which("claude")` and, when it is there,
//! runs `claude mcp get <alias>` per entry. Rust answers as Python does when `claude` is not on `PATH` (the state of the
//! goldens): an entry the store wants and the account has no registration for is an `add`; a registration the store no
//! longer wants is a `remove`; a registration the store still wants is left alone (Python would ask `claude` whether
//! the entry still exists and still points at the launcher, and could say `add`, `remove`, `forget`, `record` or
//! `refused`). Nothing is written, no account is detected, and no step line holds a secret: a line is the account name,
//! an alias and a connector id taken from the store, shown as data.

use crate::store::StoreError;
use rusqlite::{Connection, OptionalExtension};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

/// Why the plan could not be made. The texts are Python's and name none of the caller's input.
#[derive(Debug)]
pub enum SyncError {
    /// `account` is not an integer id (400).
    Invalid(String),
    /// No account has that id (404).
    NotFound(String),
    Store(StoreError),
}

impl From<rusqlite::Error> for SyncError {
    fn from(e: rusqlite::Error) -> Self {
        SyncError::Store(StoreError::from(e))
    }
}

type Result<T> = std::result::Result<T, SyncError>;

/// What a step does. Python has three more (`record`, `forget`, `refused`): each needs an answer from `claude`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Add,
    Remove,
    Skipped,
}

/// One step of a plan, as `Step.as_dict()`.
#[derive(Debug)]
pub struct Step {
    account_id: i64,
    account: String,
    action: Action,
    alias: String,
    connector_id: String,
    note: &'static str,
}

impl Step {
    /// `Step.line()`.
    fn line(&self) -> String {
        let target = format!("exec {}", self.connector_id);
        match self.action {
            Action::Add => format!("{}: add {} -> {target}", self.account, self.alias),
            Action::Remove => format!("{}: remove {} ({target})", self.account, self.alias),
            Action::Skipped => format!("{}: skipped ({})", self.account, self.note),
        }
    }

    fn as_dict(&self) -> Value {
        let action = match self.action {
            Action::Add => "add",
            Action::Remove => "remove",
            Action::Skipped => "skipped",
        };
        json!({
            "account_id": self.account_id, "account": self.account, "action": action,
            "alias": self.alias, "connector_id": self.connector_id, "line": self.line(),
        })
    }
}

/// `_sync_account`: no `account` (or an empty one) is every confirmed Claude account; else plain ASCII digits.
pub fn account_arg(raw: Option<&str>) -> Result<Option<&str>> {
    match raw {
        None | Some("") => Ok(None),
        Some(id) if id.bytes().all(|b| b.is_ascii_digit()) => Ok(Some(id)),
        Some(_) => Err(SyncError::Invalid("account must be an account id".into())),
    }
}

/// `{"steps": [...]}` for one account id (digits, from [`account_arg`]) or for every account that counts.
pub fn plan_json(conn: &Connection, account: Option<&str>) -> Result<Value> {
    let steps: Vec<Value> = plan_all(conn, account)?.iter().map(Step::as_dict).collect();
    Ok(json!({ "steps": steps }))
}

/// `plan_all`: the plan of one account, or of every confirmed Claude account plus every ignored Claude account that
/// still holds registrations (they are removed), by id.
pub fn plan_all(conn: &Connection, account: Option<&str>) -> Result<Vec<Step>> {
    let ids: Vec<i64> = match account {
        // an id past the 64-bit range names no account (Python's sqlite3 raises OverflowError there: a 500)
        Some(id) => vec![
            id.parse::<i64>()
                .ok()
                .and_then(|id| exists(conn, id).transpose())
                .transpose()?
                .ok_or_else(|| SyncError::NotFound("no such account".into()))?,
        ],
        None => {
            let mut stmt = conn.prepare(
                "SELECT id FROM agent_accounts WHERE provider = 'claude' AND (status = 'confirmed' OR (status = 'ignored' \
                 AND id IN (SELECT account_id FROM account_registrations))) ORDER BY id",
            )?;
            let rows = stmt.query_map([], |r| r.get(0))?;
            rows.collect::<std::result::Result<_, _>>()?
        }
    };
    let mut steps = Vec::new();
    for id in ids {
        steps.extend(plan(conn, id)?);
    }
    Ok(steps)
}

fn exists(conn: &Connection, id: i64) -> Result<Option<i64>> {
    Ok(conn
        .query_row("SELECT id FROM agent_accounts WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .optional()?)
}

/// `Path(folder).name`.
fn folder_name(folder: &str) -> String {
    let path = Path::new(folder);
    match path.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None if path.ends_with("..") => "..".to_string(),
        None => String::new(),
    }
}

fn pairs(conn: &Connection, sql: &str, account_id: i64) -> Result<BTreeMap<String, String>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([account_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    let mut map = BTreeMap::new();
    for row in rows {
        let (alias, connector): (String, String) = row?;
        map.insert(alias, connector); // a repeated alias keeps the last, as `dict(...)`
    }
    Ok(map)
}

/// The steps that make one account match its picks, with `claude` taken as absent (see the module text).
fn plan(conn: &Connection, account_id: i64) -> Result<Vec<Step>> {
    let (provider, folder, label, status): (String, String, Option<String>, String) = conn
        .query_row(
            "SELECT provider, folder, label, status FROM agent_accounts WHERE id = ?1",
            [account_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
    let name = label
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| folder_name(&folder));
    let skipped = |note| Step {
        account_id,
        account: name.clone(),
        action: Action::Skipped,
        alias: String::new(),
        connector_id: String::new(),
        note,
    };
    if provider != "claude" || (status != "confirmed" && status != "ignored") {
        return Ok(vec![skipped("not a confirmed Claude account")]);
    }
    let registered = pairs(
        conn,
        "SELECT alias, connector_id FROM account_registrations WHERE account_id = ?1",
        account_id,
    )?;
    if status == "ignored" && registered.is_empty() {
        return Ok(Vec::new());
    }
    if !crate::metrics::is_dir(&folder) {
        return Ok(vec![skipped("folder missing")]);
    }
    let desired = if status == "ignored" {
        BTreeMap::new() // its picks no longer count: every entry we added there is removed
    } else {
        pairs(
            conn,
            "SELECT c.alias, c.id FROM connectors c JOIN account_workspaces p \
             ON p.workspace_id = c.workspace_id WHERE p.account_id = ?1",
            account_id,
        )?
    };
    let step = |action, alias: &String, connector: &String| Step {
        account_id,
        account: name.clone(),
        action,
        alias: alias.clone(),
        connector_id: connector.clone(),
        note: "",
    };
    // removals before additions, so a re-used alias is free when it is added
    let mut steps: Vec<Step> = registered
        .iter()
        .filter(|(alias, connector)| desired.get(*alias) != Some(*connector))
        .map(|(alias, connector)| step(Action::Remove, alias, connector))
        .collect();
    steps.extend(
        desired
            .iter()
            .filter(|(alias, connector)| registered.get(*alias) != Some(*connector))
            .map(|(alias, connector)| step(Action::Add, alias, connector)),
    );
    Ok(steps)
}
