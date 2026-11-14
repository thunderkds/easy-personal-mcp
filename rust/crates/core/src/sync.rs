//! The sync plan and its apply step, ported from `src/easy_personal/connectors/sync.py` (`Step`, `plan`, `plan_all`,
//! `_snippet`, `apply`) and the `claude` argument lists of `connectors/writer.py` (`add_json`, `_entry_exists`,
//! `entry_target`, `points_at_launcher`, `add_entry`, `remove_entry`); served as `GET /api/sync` (T041) and
//! `POST /api/sync` (T050).
//!
//! `plan` lists what applying the store's picks would change in each Claude account's user-scope MCP entries. When
//! `claude` is on `PATH` it asks `claude mcp get <alias>` per entry, with that account's `CLAUDE_CONFIG_DIR`, as Python
//! does; then a step can also be `record`, `forget` or `refused`. `apply` runs `claude mcp add-json|remove --scope user`
//! step by step and records each one in `account_registrations` right after it succeeds, so a failure can be re-run.
//!
//! This module starts no process itself: it builds the argument lists and reads the answers through [`Claude`], which
//! the app implements (`claude_cli.rs`) and a test fakes. Nothing here reads a keychain. No step line holds a secret:
//! a line is an account name, an alias and a connector id from the store. What `claude` prints on stderr is never read.

use crate::pyjson::{py_str, sh_quote};
use crate::store::StoreError;
use crate::workspaces::py_strip;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The launcher's file name (`writer.LAUNCHER`).
pub const LAUNCHER: &str = "easy-personal-launch";

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

/// What a `claude` call answered: its exit code (Python's 127 when it could not run or ran past its timeout) and what
/// it printed on stdout (capped by the runner). Its stderr is not kept. `not_run` is the runner's own signal that the
/// call did not start or was killed at its timeout (an exit code of 127 alone does not say so).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ran {
    pub code: i32,
    pub stdout: String,
    pub not_run: bool,
}

/// `claude mcp get` could not run: the answer is unknown, never "no entry" (Python takes it as no entry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotRun;

/// The `claude` CLI as `writer.py` runs it.
pub trait Claude {
    /// `shutil.which("claude") is not None`.
    fn on_path(&self) -> bool;
    /// `_claude(*args, env=account_env(config_dir_env))`: `None` is the default account (`CLAUDE_CONFIG_DIR` removed),
    /// else `CLAUDE_CONFIG_DIR` is set to it. Never a shell; `args` is the whole argument list after `claude`.
    fn run(&self, args: &[&str], config_dir_env: Option<&str>) -> Ran;
    /// `writer.launcher_path()`: the launcher's absolute path, or Python's `LaunchError` text.
    fn launcher(&self) -> std::result::Result<String, String>;
}

/// `claude` taken as absent (the T041 read of the plan; nothing is ever run).
pub struct NoClaude;

impl Claude for NoClaude {
    fn on_path(&self) -> bool {
        false
    }
    fn run(&self, _: &[&str], _: Option<&str>) -> Ran {
        Ran {
            code: 127,
            stdout: String::new(),
            not_run: true,
        }
    }
    fn launcher(&self) -> std::result::Result<String, String> {
        Err(format!(
            "cannot find {LAUNCHER}; reinstall easy-personal (uv tool install) so it is on disk"
        ))
    }
}

/// `add_json`: the JSON `claude mcp add-json` gets. It names the launcher and the connector id, never a secret.
pub fn add_json(connector_id: &str, launcher: &str) -> String {
    format!(
        "{{\"type\": \"stdio\", \"command\": {}, \"args\": [\"exec\", {}]}}",
        py_str(launcher),
        py_str(connector_id)
    )
}

/// `_entry_exists`: `claude mcp get <name>`'s answer when it exits 0, `None` when it exits non-zero, [`NotRun`] when
/// it could not run.
pub fn entry_exists(
    claude: &dyn Claude,
    name: &str,
    config_dir_env: Option<&str>,
) -> std::result::Result<Option<Ran>, NotRun> {
    let got = claude.run(&["mcp", "get", name], config_dir_env);
    if got.not_run {
        return Err(NotRun);
    }
    Ok((got.code == 0).then_some(got))
}

/// `entry_target`: `(command, args)` from the first `Command:` and `Args:` lines (for our own checks, never shown).
pub fn entry_target(stdout: &str) -> (String, String) {
    let (mut command, mut args) = (None, None);
    for line in crate::catalog::splitlines(stdout) {
        let Some((key, value)) = py_strip(line).split_once(':') else {
            continue;
        };
        let slot = match key {
            "Command" => &mut command,
            "Args" => &mut args,
            _ => continue,
        };
        if slot.is_none() {
            *slot = Some(py_strip(value).to_string());
        }
    }
    (command.unwrap_or_default(), args.unwrap_or_default())
}

/// `points_at_launcher`: the entry runs `.../easy-personal-launch exec <connector_id>`.
pub fn points_at_launcher(got: &Ran, connector_id: &str) -> bool {
    let (command, args) = entry_target(&got.stdout);
    Path::new(&command)
        .file_name()
        .is_some_and(|n| n == LAUNCHER)
        && args == format!("exec {connector_id}")
}

/// `add_entry`.
pub fn add_entry(
    claude: &dyn Claude,
    alias: &str,
    connector_id: &str,
    launcher: &str,
    cde: Option<&str>,
) -> Ran {
    let json = add_json(connector_id, launcher);
    claude.run(&["mcp", "add-json", alias, &json, "--scope", "user"], cde)
}

/// `remove_entry`.
pub fn remove_entry(claude: &dyn Claude, alias: &str, cde: Option<&str>) -> Ran {
    claude.run(&["mcp", "remove", alias, "--scope", "user"], cde)
}

/// What a step does (`Step.action`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Add,
    Remove,
    Record,
    Forget,
    Refused,
    Skipped,
}

/// One step of a plan.
#[derive(Debug, Clone)]
pub struct Step {
    account_id: i64,
    account: String,
    config_dir_env: Option<String>,
    action: Action,
    alias: String,
    connector_id: String,
    note: &'static str,
}

impl Step {
    /// `Step.line()`.
    pub fn line(&self) -> String {
        let note = if self.note.is_empty() {
            String::new()
        } else {
            format!(" ({})", self.note)
        };
        let target = format!("exec {}", self.connector_id);
        let (account, alias) = (&self.account, &self.alias);
        match self.action {
            Action::Add => format!("{account}: add {alias} -> {target}{note}"),
            Action::Remove => format!("{account}: remove {alias} ({target}){note}"),
            Action::Record => {
                format!("{account}: record {alias} -> {target} (already in Claude Code)")
            }
            Action::Forget => {
                format!("{account}: forget {alias} ({target}) (already gone from Claude Code)")
            }
            Action::Refused => format!(
                "{account}: refused {alias} -> {target}: an MCP server with this name exists that easy-personal did not \
                 add; nothing is overwritten"
            ),
            Action::Skipped => format!("{account}: skipped{note}"),
        }
    }

    /// The connector a step is about (`Step.connector_id`; empty for a `skipped` step).
    pub fn connector_id(&self) -> &str {
        &self.connector_id
    }

    /// Whether `apply` runs it (Python's `APPLIED`: `remove`, `forget`, `add`, `record`).
    pub fn is_applied(&self) -> bool {
        matches!(
            self.action,
            Action::Add | Action::Remove | Action::Record | Action::Forget
        )
    }

    pub fn is_skipped(&self) -> bool {
        self.action == Action::Skipped
    }

    fn as_dict(&self) -> Value {
        let action = match self.action {
            Action::Add => "add",
            Action::Remove => "remove",
            Action::Record => "record",
            Action::Forget => "forget",
            Action::Refused => "refused",
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

/// `{"steps": [...]}` with `claude` taken as absent (T041's read; the oracle's state).
pub fn plan_json(conn: &Connection, account: Option<&str>) -> Result<Value> {
    plan_json_with(conn, account, &NoClaude)
}

/// `{"steps": [...]}` for one account id (digits, from [`account_arg`]) or for every account that counts.
pub fn plan_json_with(
    conn: &Connection,
    account: Option<&str>,
    claude: &dyn Claude,
) -> Result<Value> {
    let steps: Vec<Value> = plan_all(conn, account, claude)?
        .iter()
        .map(Step::as_dict)
        .collect();
    Ok(json!({ "steps": steps }))
}

/// `plan_all`: the plan of one account, or of every confirmed Claude account plus every ignored Claude account that
/// still holds registrations (they are removed), by id.
pub fn plan_all(
    conn: &Connection,
    account: Option<&str>,
    claude: &dyn Claude,
) -> Result<Vec<Step>> {
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
        steps.extend(plan(conn, id, &BTreeSet::new(), claude)?);
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
pub fn folder_name(folder: &str) -> String {
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

/// `plan`: the steps that make one account match its picks; `drop` holds connector ids treated as not desired.
pub fn plan(
    conn: &Connection,
    account_id: i64,
    drop: &BTreeSet<String>,
    claude: &dyn Claude,
) -> Result<Vec<Step>> {
    let (provider, folder, label, status, config_dir_env): (
        String,
        String,
        Option<String>,
        String,
        Option<String>,
    ) = conn.query_row(
        "SELECT provider, folder, label, status, config_dir_env FROM agent_accounts WHERE id = ?1",
        [account_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    )?;
    let name = label
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| folder_name(&folder));
    let step = |action, alias: &str, connector: &str, note| Step {
        account_id,
        account: name.clone(),
        config_dir_env: config_dir_env.clone(),
        action,
        alias: alias.to_string(),
        connector_id: connector.to_string(),
        note,
    };
    if provider != "claude" || (status != "confirmed" && status != "ignored") {
        return Ok(vec![step(
            Action::Skipped,
            "",
            "",
            "not a confirmed Claude account",
        )]);
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
        return Ok(vec![step(Action::Skipped, "", "", "folder missing")]);
    }
    let mut desired = if status == "ignored" {
        BTreeMap::new() // its picks no longer count: every entry we added there is removed
    } else {
        pairs(
            conn,
            "SELECT c.alias, c.id FROM connectors c JOIN account_workspaces p \
             ON p.workspace_id = c.workspace_id WHERE p.account_id = ?1",
            account_id,
        )?
    };
    desired.retain(|_, connector| !drop.contains(connector));
    let has_claude = claude.on_path();
    let cde = config_dir_env.as_deref();
    let get = |alias: &str| {
        if has_claude {
            entry_exists(claude, alias, cde)
        } else {
            Ok(None)
        }
    };
    let steps = (|| -> std::result::Result<Vec<Step>, NotRun> {
        // removals before additions, so a re-used alias is free when it is added
        let (mut first, mut then) = (Vec::new(), Vec::new());
        for (alias, connector) in &registered {
            if desired.get(alias) == Some(connector) {
                if !has_claude {
                    continue;
                }
                match get(alias)? {
                    None => then.push(step(
                        Action::Add,
                        alias,
                        connector,
                        "missing from Claude Code: re-add",
                    )),
                    Some(got) if !points_at_launcher(&got, connector) => {
                        first.push(step(
                            Action::Remove,
                            alias,
                            connector,
                            "does not point at the launcher",
                        ));
                        then.push(step(Action::Add, alias, connector, "re-add"));
                    }
                    Some(_) => {}
                }
            } else if has_claude && get(alias)?.is_none() {
                first.push(step(Action::Forget, alias, connector, ""));
            } else {
                first.push(step(Action::Remove, alias, connector, ""));
            }
        }
        for (alias, connector) in &desired {
            if registered.get(alias) == Some(connector) {
                continue;
            }
            // a registered alias is ours: it was removed above
            let got = if registered.contains_key(alias) {
                None
            } else {
                get(alias)?
            };
            then.push(match got {
                None => step(Action::Add, alias, connector, ""),
                Some(got) if points_at_launcher(&got, connector) => {
                    step(Action::Record, alias, connector, "")
                }
                Some(_) => step(Action::Refused, alias, connector, ""),
            });
        }
        first.extend(then);
        Ok(first)
    })();
    // `claude` could not run: this account is not changed now (Python would take every entry as absent)
    Ok(
        steps
            .unwrap_or_else(|NotRun| vec![step(Action::Skipped, "", "", "`claude` could not run")]),
    )
}

/// `_snippet`: the `claude` command an owner runs by hand when `claude` is not on `PATH`.
fn snippet(step: &Step, launcher: &str) -> String {
    let prefix = match &step.config_dir_env {
        Some(dir) => format!("CLAUDE_CONFIG_DIR={}", sh_quote(dir)),
        None => "env -u CLAUDE_CONFIG_DIR".into(),
    };
    let command = if step.action == Action::Add {
        format!(
            "mcp add-json {} {}",
            sh_quote(&step.alias),
            sh_quote(&add_json(&step.connector_id, launcher))
        )
    } else {
        format!("mcp remove {}", sh_quote(&step.alias))
    };
    format!("{prefix} claude {command} --scope user")
}

/// `apply`: runs the steps in order, records each one right after it succeeds, stops at the first failure. `Ok(status)`
/// is Python's return value; `Err(text)` is a `LaunchError` (no launcher for an `add`), which the caller shows as the
/// last line with status 1. A failed `claude` shows its exit code only: its output is never shown (Python also prints
/// its stderr).
pub fn apply(
    conn: &Connection,
    steps: &[Step],
    claude: &dyn Claude,
    out: &mut Vec<String>,
) -> std::result::Result<std::result::Result<i64, String>, StoreError> {
    let work: Vec<&Step> = steps.iter().filter(|s| s.is_applied()).collect();
    if work.is_empty() {
        out.push("nothing to change.".into());
        return Ok(Ok(0));
    }
    let launcher = if work.iter().any(|s| s.action == Action::Add) {
        match claude.launcher() {
            Ok(path) => path,
            Err(text) => return Ok(Err(text)),
        }
    } else {
        String::new()
    };
    if !claude.on_path() {
        out.push(
            "the `claude` CLI is not on PATH; nothing was written. Run these once it is installed:"
                .into(),
        );
        for s in &work {
            if matches!(s.action, Action::Add | Action::Remove) {
                out.push(format!("  {}", snippet(s, &launcher)));
            }
        }
        return Ok(Ok(0));
    }
    for s in work {
        let cde = s.config_dir_env.as_deref();
        let done = match s.action {
            Action::Add => Some(add_entry(claude, &s.alias, &s.connector_id, &launcher, cde)),
            Action::Remove => Some(remove_entry(claude, &s.alias, cde)),
            _ => None,
        };
        if let Some(done) = done.filter(|d| d.code != 0) {
            out.push(format!(
                "failed: {} (`claude` exit {}); stopped here.",
                s.line(),
                done.code
            ));
            out.push("run `easy-personal connector sync` again to finish.".into());
            return Ok(Ok(1));
        }
        // one small transaction per step: registrations match what succeeded
        if matches!(s.action, Action::Add | Action::Record) {
            conn.execute(
                "INSERT OR REPLACE INTO account_registrations VALUES (?1, ?2, ?3)",
                params![s.account_id, s.alias, s.connector_id],
            )
        } else {
            conn.execute(
                "DELETE FROM account_registrations WHERE account_id = ?1 AND alias = ?2",
                params![s.account_id, s.alias],
            )
        }
        .map_err(StoreError::from)?;
        out.push(format!("done: {}", s.line()));
    }
    out.push("restart Claude Code in the changed accounts to use the change.".into());
    Ok(Ok(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_json_and_the_entry_check_equal_python() {
        // Python: json.dumps({"type": "stdio", "command": "/opt/x/easy-personal-launch", "args": ["exec", "c1"]})
        assert_eq!(
            add_json("c1", "/opt/x/easy-personal-launch"),
            r#"{"type": "stdio", "command": "/opt/x/easy-personal-launch", "args": ["exec", "c1"]}"#
        );
        let got = |out: &str| Ran {
            code: 0,
            stdout: out.into(),
            not_run: false,
        };
        let ours = "slack:\n  Scope: User config\n  Type: stdio\n  Command: /x/easy-personal-launch\n  Args: exec slack\n";
        assert!(points_at_launcher(&got(ours), "slack"));
        assert!(!points_at_launcher(&got(ours), "slack2"));
        assert!(!points_at_launcher(
            &got("  Command: /x/other\n  Args: exec slack\n"),
            "slack"
        ));
        // the first `Command:` line counts, a later one does not
        assert!(points_at_launcher(
            &got("Command: easy-personal-launch\nArgs: exec a\nCommand: /evil\n"),
            "a"
        ));
        assert!(!points_at_launcher(&got(""), "a"));
    }
}
