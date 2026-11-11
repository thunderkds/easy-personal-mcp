//! The `workspace` (FR-035) and agent `account` (FR-036) scope of the usage metrics and the rules, ported from
//! `src/easy_personal/core/metrics.py` (`_scope`, `_padded`), `core/workspaces.py` (`by_name`, `project_paths`,
//! `_under`), `core/accounts.py` (`resolve`, `default_id`), `core/discovery.py` (`resolve_project`) and the scoped
//! parts of `core/kit.py` (`_stored_projects`, `scoped_projects`).
//!
//! A scope becomes an AND-ed SQL clause plus bound values: the workspace's repo paths are resolved to the distinct
//! *stored* project paths first, so the big `events` table is never matched with a pattern, and the account (an id
//! or a label) becomes `agent_account_id = ?`. Only values are bound: no SQL is built from the workspace or account
//! text. Read-only. An unknown workspace or account is a [`ScopeError::Unknown`] whose text equals Python's and never
//! echoes the caller's input; a workspace with no matching project selects nothing (`0`).
//!
//! `git` is used by the shared helpers of `metrics.rs` (`repo_of`, `owner_map`), which call `run_git_within` with its
//! 5 s limit, as Python does.

use crate::metrics::{
    DAY_MICROS, format_stored, is_dir, owner_map, parse_iso, repo_of, stored_branches,
};
use crate::store::StoreError;
use rusqlite::types::Value;
use rusqlite::{Connection, params_from_iter};
use std::cell::OnceCell;
use std::collections::{BTreeSet, HashMap};
use std::fmt;

/// An optional workspace name and an optional account (id or label). An empty string is no filter (Python's
/// `or None` at the surfaces).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scope<'a> {
    pub workspace: Option<&'a str>,
    pub account: Option<&'a str>,
}

/// Why a scope could not be applied.
#[derive(Debug)]
pub enum ScopeError {
    /// The workspace or the account is not known. The text is Python's and holds none of the caller's input.
    Unknown(String),
    Store(StoreError),
}

impl fmt::Display for ScopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScopeError::Unknown(text) => write!(f, "{text}"),
            ScopeError::Store(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ScopeError {}

impl From<rusqlite::Error> for ScopeError {
    fn from(e: rusqlite::Error) -> Self {
        ScopeError::Store(StoreError::from(e))
    }
}

type Result<T> = std::result::Result<T, ScopeError>;

/// An AND-ed filter clause (`1` when there is no scope) and the values bound to its `?` placeholders.
pub(crate) struct Clause {
    pub sql: String,
    pub params: Vec<Value>,
}

/// A scope resolved against one store: the workspace's repo roots and the account's clause. Built once per call
/// (the rules reuse it for every window), so `git` runs once per repo, not once per query.
pub(crate) struct Resolved {
    /// The workspace's repo paths and what each resolves to; `None` = no workspace in the scope.
    roots: Option<Vec<String>>,
    /// The account's clause and its id; `None` = no account in the scope.
    account: Option<(&'static str, i64)>,
    /// Stored gone path -> owner project path (`owners.owner_map`), read when first needed.
    owner: OnceCell<HashMap<String, String>>,
}

/// Python's `str.strip()`: also the C0 separators `\x1c`..`\x1f`, which `char::is_whitespace` does not list.
fn strip(text: &str) -> &str {
    text.trim_matches(|c: char| c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c))
}

/// An approximation of Python's `str.casefold()`: each character lowercased on its own, plus the three full
/// folds that differ from lowercase in everyday text (`ß` -> `ss`, final `ς` -> `σ`, long `ſ` -> `s`).
pub(crate) fn casefold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .flat_map(|c| match c {
            'ß' => vec!['s', 's'],
            'ς' => vec!['σ'],
            'ſ' => vec!['s'],
            c => vec![c],
        })
        .collect()
}

/// `workspaces.by_name(...)["repos"]`, then `{repo, discovery.resolve_project(repo)}` for each: the roots a stored
/// project path must be equal to or under.
fn workspace_roots(conn: &Connection, name: &str) -> Result<Vec<String>> {
    let id: Option<i64> = conn
        .prepare("SELECT id FROM workspaces WHERE name = ?1")?
        .query_map([name], |r| r.get(0))?
        .next()
        .transpose()?;
    let Some(id) = id else {
        return Err(ScopeError::Unknown("unknown workspace".into()));
    };
    let mut stmt = conn.prepare(
        "SELECT repo_path FROM workspace_repos WHERE workspace_id = ?1 ORDER BY repo_path",
    )?;
    let repos: Vec<String> = stmt
        .query_map([id], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    let mut roots = BTreeSet::new();
    for repo in repos {
        // `discovery.resolve_project`: a folder missing on disk is its own project, else its git repo (a worktree
        // gives its main repo, a subfolder its top level), else the folder itself.
        let resolved = if is_dir(&repo) {
            repo_of(&repo).unwrap_or_else(|| repo.clone())
        } else {
            repo.clone()
        };
        roots.insert(repo);
        roots.insert(resolved);
    }
    Ok(roots.into_iter().collect())
}

/// `accounts.resolve`: the confirmed account an id or a label (case-insensitive, stripped) names. The default
/// account (`CLAUDE_CONFIG_DIR` unset) also owns events stored with no account.
fn account_clause(conn: &Connection, reference: &str) -> Result<(&'static str, i64)> {
    let reference = strip(reference);
    let mut stmt = conn
        .prepare("SELECT id, label FROM agent_accounts WHERE status = 'confirmed' ORDER BY id")?;
    let rows: Vec<(i64, Option<String>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<std::result::Result<_, _>>()?;
    let wanted = casefold(reference);
    let Some((id, _)) = rows.iter().find(|(id, label)| {
        id.to_string() == reference || label.as_deref().is_some_and(|l| casefold(l) == wanted)
    }) else {
        let labels: Vec<&str> = rows.iter().filter_map(|(_, l)| l.as_deref()).collect();
        let labels = if labels.is_empty() {
            "(none)".to_string()
        } else {
            labels.join(", ")
        };
        return Err(ScopeError::Unknown(format!(
            "unknown account; valid labels: {labels}"
        )));
    };
    let default: Option<i64> = conn
        .prepare(
            "SELECT id FROM agent_accounts WHERE provider = 'claude' AND config_dir_env IS NULL ORDER BY id",
        )?
        .query_map([], |r| r.get(0))?
        .next()
        .transpose()?;
    Ok(if default == Some(*id) {
        ("(agent_account_id = ? OR agent_account_id IS NULL)", *id)
    } else {
        ("agent_account_id = ?", *id)
    })
}

/// `path` equals `root`, or lies under it (`workspaces._under`): `/a/b` is never under `/a/bc`.
pub(crate) fn under(path: &str, root: &str) -> bool {
    path == root
        || root == "/"
        || path
            .strip_prefix(root)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// A stored bound moved by `delta` microseconds, in the stored form (`metrics._padded`).
fn shifted(stored: Option<&str>, delta: i64) -> Result<Option<String>> {
    stored
        .map(|text| {
            parse_iso(text)
                .map(|micros| format_stored(micros + delta))
                .ok_or_else(|| {
                    ScopeError::Store(StoreError::Unreadable(
                        "a bound is not a stored time".into(),
                    ))
                })
        })
        .transpose()
}

/// Resolves a scope against the store: the workspace first (an unknown one is an error), then the account.
/// Nothing is written.
pub(crate) fn resolve(conn: &Connection, scope: Scope) -> Result<Resolved> {
    let roots = match scope.workspace.filter(|w| !w.is_empty()) {
        Some(name) => Some(workspace_roots(conn, name)?),
        None => None,
    };
    let account = match scope.account.filter(|a| !a.is_empty()) {
        Some(reference) => Some(account_clause(conn, reference)?),
        None => None,
    };
    Ok(Resolved {
        roots,
        account,
        owner: OnceCell::new(),
    })
}

impl Resolved {
    /// True when neither a workspace nor an account is in the scope.
    pub(crate) fn is_empty(&self) -> bool {
        self.roots.is_none() && self.account.is_none()
    }

    fn owner(&self, conn: &Connection) -> Result<&HashMap<String, String>> {
        if self.owner.get().is_none() {
            let map = owner_map(&stored_branches(conn)?);
            let _ = self.owner.set(map);
        }
        Ok(self.owner.get().expect("just set"))
    }

    /// `workspaces.project_paths`: the distinct stored project paths (inside the window padded by one day, for
    /// speed) equal to or under a root, a gone worktree counting for its owner project (FR-057), sorted.
    fn project_paths(
        &self,
        conn: &Connection,
        roots: &[String],
        since_utc: Option<&str>,
        until_utc: Option<&str>,
    ) -> Result<Vec<String>> {
        if roots.is_empty() {
            return Ok(Vec::new());
        }
        let (mut clauses, mut params) = (vec!["project_path IS NOT NULL"], Vec::new());
        if let Some(since) = shifted(since_utc, -DAY_MICROS)? {
            clauses.push("ts_utc >= ?");
            params.push(since);
        }
        if let Some(until) = shifted(until_utc, DAY_MICROS)? {
            clauses.push("ts_utc < ?");
            params.push(until);
        }
        let mut stmt = conn.prepare(&format!(
            "SELECT DISTINCT project_path FROM events WHERE {}",
            clauses.join(" AND ")
        ))?;
        let stored: Vec<String> = stmt
            .query_map(params_from_iter(&params), |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        let owner = self.owner(conn)?;
        let mut found: Vec<String> = stored
            .into_iter()
            .filter(|p| {
                roots.iter().any(|root| {
                    under(owner.get(p).map_or(p.as_str(), String::as_str), root) || under(p, root)
                })
            })
            .collect();
        found.sort();
        Ok(found)
    }

    /// `metrics._scope`: the clause for events in [`since_utc`, `until_utc`) (stored form; `None` = no bound) and its
    /// values. The window only narrows the workspace's path lookup; the caller still applies its own period.
    pub(crate) fn clause(
        &self,
        conn: &Connection,
        since_utc: Option<&str>,
        until_utc: Option<&str>,
    ) -> Result<Clause> {
        let (mut sql, mut params) = (Vec::new(), Vec::new());
        if let Some(roots) = &self.roots {
            let paths = self.project_paths(conn, roots, since_utc, until_utc)?;
            if paths.is_empty() {
                sql.push("0".to_string());
            } else {
                sql.push(format!(
                    "project_path IN ({})",
                    vec!["?"; paths.len()].join(", ")
                ));
                params.extend(paths.into_iter().map(Value::Text));
            }
        }
        if let Some((clause, id)) = self.account {
            sql.push(clause.to_string());
            params.push(Value::Integer(id));
        }
        Ok(Clause {
            sql: if sql.is_empty() {
                "1".into()
            } else {
                sql.join(" AND ")
            },
            params,
        })
    }

    /// `kit._stored_projects`: the distinct stored `claude_code` project paths in scope over all time, sorted.
    /// Kernel-trace rows carry no account, so the kit rules follow this list.
    pub(crate) fn stored_projects(&self, conn: &Connection) -> Result<Vec<String>> {
        let clause = self.clause(conn, None, None)?;
        let mut stmt = conn.prepare(&format!(
            "SELECT DISTINCT project_path FROM events WHERE project_path IS NOT NULL \
             AND source = 'claude_code' AND {} ORDER BY project_path",
            clause.sql
        ))?;
        Ok(stmt
            .query_map(params_from_iter(&clause.params), |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?)
    }

    /// `kit.scoped_projects`: [`Resolved::stored_projects`] with a gone worktree listed as its owner project, once.
    pub(crate) fn scoped_projects(&self, conn: &Connection) -> Result<BTreeSet<String>> {
        let paths = self.stored_projects(conn)?;
        let owner = self.owner(conn)?;
        Ok(paths
            .into_iter()
            .map(|p| owner.get(&p).cloned().unwrap_or(p))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{casefold, strip, under};

    #[test]
    fn under_is_path_aware() {
        assert!(under("/a/b", "/a/b"));
        assert!(under("/a/b/c", "/a/b"));
        assert!(!under("/a/bc", "/a/b"));
        assert!(!under("/a", "/a/b"));
        assert!(under("/anything", "/"));
    }

    #[test]
    fn strip_and_casefold_follow_python_for_everyday_text() {
        assert_eq!(strip(" \t Claude work\n\u{1f}"), "Claude work");
        assert_eq!(casefold("Claude WORK"), "claude work");
        assert_eq!(casefold("Straße"), casefold("STRASSE"));
        assert_eq!(casefold("ΟΔΟΣ"), casefold("οδος"));
        assert_eq!(casefold("ΟΔΟΣ"), casefold("οδοσ"));
    }
}
