//! The `ui` source (P2b, T033): Python `sources/ui.py`. easy-ui `session-*.json` files become one overall row per
//! session in `ui_sessions`. Only `id`, `status`, `startedAt`, `endedAt` and the number of `actions` are read; `target`,
//! `issues` and the contents of `actions` never are (FR-013). A file that cannot be read is skipped and counted.
//!
//! Replace rules exactly as Python: a folder that is unset or not a folder deletes every row and reports `no data`;
//! otherwise rows are `INSERT OR REPLACE`d and nothing is deleted (a session whose file is gone stays), and a later
//! file (sorted) wins for one id. Rust differs where Python hangs or crashes: a file that is not a regular file
//! (a FIFO would block Python forever) and a time outside years 1-9999 (`OverflowError`) are skipped.

use super::{json, label, time};
use crate::writer::{Store, WriteError};
use rusqlite::{Transaction, TransactionBehavior, params};
use std::collections::HashMap;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

/// `MAX_BYTES`: a larger file is skipped, not decoded.
pub const MAX_BYTES: u64 = 20_000_000;

/// The counts of one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// `ok` or `no data`.
    pub status: &'static str,
    pub sessions: i64,
    pub skipped: i64,
}

type Row = (String, String, String, Option<String>, i64);

fn utc(value: &json::Value) -> Option<String> {
    value.as_str().and_then(time::utc_text)
}

/// `parse_session`: `(id, status, started_utc, ended_utc, action_count)`, or `None` when it cannot be read.
fn parse_session(raw: &[u8]) -> Option<Row> {
    let record = json::loads(raw)?;
    if !record.is_dict() {
        return None;
    }
    let id = label::label(record.get("id"))?;
    let status = label::label(record.get("status"))?;
    let started = utc(record.get("startedAt"))?;
    let ended = utc(record.get("endedAt"));
    let json::Value::Array(actions) = record.get("actions") else {
        return None;
    };
    if !matches!(record.get("endedAt"), json::Value::Null) && ended.is_none() {
        return None;
    }
    Some((id, status, started, ended, actions.len() as i64))
}

/// `folder.glob("session-*.json")`, sorted. `None` when the folder is not a folder; an unreadable folder is empty
/// (Python's `glob` swallows the permission error).
fn session_files(folder: &Path) -> Option<Vec<PathBuf>> {
    if !folder.is_dir() {
        return None;
    }
    let mut paths: Vec<PathBuf> = fs::read_dir(folder)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| {
                    let name = e.file_name();
                    let name = name.as_bytes();
                    name.len() >= 13 && name.starts_with(b"session-") && name.ends_with(b".json")
                })
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default();
    paths.sort();
    Some(paths)
}

fn begin(store: &Store) -> rusqlite::Result<Transaction<'_>> {
    Transaction::new_unchecked(store.conn(), TransactionBehavior::Immediate)
}

/// `ingest`: store every readable session of the folder.
pub fn ingest(store: &mut Store, folder: Option<&Path>) -> Result<Summary, WriteError> {
    let Some(paths) = folder.and_then(session_files) else {
        let tx = begin(store)?; // unset or missing folder: nothing to show
        tx.execute("DELETE FROM ui_sessions", [])?;
        tx.commit()?;
        return Ok(Summary {
            status: "no data",
            sessions: 0,
            skipped: 0,
        });
    };
    let (mut rows, mut index): (Vec<Row>, HashMap<String, usize>) = (Vec::new(), HashMap::new());
    let mut skipped = 0;
    for path in paths {
        // `path.stat()` follows a symlink; a folder, a FIFO or a broken link is not a session file.
        let row = fs::metadata(&path)
            .ok()
            .filter(|m| m.is_file() && m.len() <= MAX_BYTES)
            .and_then(|_| fs::read(&path).ok())
            .and_then(|raw| parse_session(&raw));
        match row {
            None => skipped += 1,
            Some(row) => match index.get(&row.0) {
                Some(&at) => rows[at] = row, // a dict keeps the first position of a key and the last value
                None => {
                    index.insert(row.0.clone(), rows.len());
                    rows.push(row);
                }
            },
        }
    }
    let tx = begin(store)?;
    {
        let mut insert = tx.prepare("INSERT OR REPLACE INTO ui_sessions VALUES (?, ?, ?, ?, ?)")?;
        for (id, status, started, ended, actions) in &rows {
            insert.execute(params![id, status, started, ended, actions])?;
        }
    }
    tx.commit()?;
    Ok(Summary {
        status: "ok",
        sessions: rows.len() as i64,
        skipped,
    })
}
