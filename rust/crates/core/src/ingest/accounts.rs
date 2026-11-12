//! Agent accounts for the ingest (T035): Python `core/accounts.py` `detect`, `_insert`, `_provider_of`, `default_id`.
//!
//! Detection reads folder names and whether marker entries exist, nothing else. Rows already in the store (any
//! status) are never changed: `INSERT OR IGNORE` on the `folder` key, as Python.

use crate::writer::{Store, WriteError};
use rusqlite::{Connection, OptionalExtension, params};
use std::fs;
use std::path::{Path, PathBuf};

/// `DEFAULT_LABEL`.
pub const DEFAULT_LABEL: &str = "Claude default";

/// `_provider_of`: Claude (`projects/` or `settings.json`), Codex (`config.toml`), else none.
pub(crate) fn provider_of(folder: &Path) -> Option<&'static str> {
    if folder.join("projects").is_dir() || folder.join("settings.json").is_file() {
        Some("claude")
    } else if folder.join("config.toml").is_file() {
        Some("codex")
    } else {
        None
    }
}

/// `_insert`: the default account is `confirmed` with its label and no `CLAUDE_CONFIG_DIR`; any other is `detected`,
/// and a Claude one names its folder as `config_dir_env`.
pub(crate) fn insert(
    conn: &Connection,
    provider: &str,
    folder: &str,
    default: bool,
) -> rusqlite::Result<()> {
    let (label, status) = if default {
        (Some(DEFAULT_LABEL), "confirmed")
    } else {
        (None, "detected")
    };
    let env = (provider == "claude" && !default).then_some(folder);
    conn.execute(
        "INSERT OR IGNORE INTO agent_accounts (provider, folder, label, status, config_dir_env) \
         VALUES (?, ?, ?, ?, ?)",
        params![provider, folder, label, status, env],
    )?;
    Ok(())
}

/// `accounts.detect`: insert the `.claude*` / `.codex*` folders of `agent_root` not yet known, in one transaction.
/// An unreadable `agent_root` inserts nothing (Python: `except OSError: return`). Entries are sorted by name, files
/// are skipped (`~/.claude.json`), symlinks are resolved. Rust differs only where Python would crash: a name that is
/// not UTF-8 (Python cannot store it) is skipped.
pub fn detect(store: &mut Store, agent_root: &Path) -> Result<(), WriteError> {
    let Ok(dir) = fs::read_dir(agent_root) else {
        return Ok(());
    };
    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    for entry in dir {
        let Ok(entry) = entry else {
            return Ok(()); // an error while listing: Python's `iterdir` raises OSError -> return
        };
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if name.starts_with(".claude") || name.starts_with(".codex") {
            entries.push((name, entry.path()));
        }
    }
    entries.sort();
    let tx = store.conn().unchecked_transaction()?;
    for (name, entry) in entries {
        if !entry.is_dir() {
            continue;
        }
        let Ok(folder) = fs::canonicalize(&entry) else {
            continue;
        };
        let Some(text) = folder.to_str() else {
            continue;
        };
        if name == ".claude" {
            insert(&tx, "claude", text, true)?;
        } else if name.starts_with(".claude") && provider_of(&folder) == Some("claude") {
            insert(&tx, "claude", text, false)?;
        } else if name.starts_with(".codex") && provider_of(&folder) == Some("codex") {
            insert(&tx, "codex", text, false)?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// `accounts.default_id`: the first Claude account with no `CLAUDE_CONFIG_DIR`, by id; found by its columns, not by id 1.
pub fn default_id(store: &Store) -> Result<Option<i64>, WriteError> {
    Ok(store
        .conn()
        .query_row(
            "SELECT id FROM agent_accounts WHERE provider = 'claude' AND config_dir_env IS NULL ORDER BY id",
            [],
            |r| r.get(0),
        )
        .optional()?)
}
