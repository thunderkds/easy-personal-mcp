//! The `kit_trace` source (P2b, T033): Python `sources/kit_trace.py`. Kernel traces
//! (`<project>/memory/event-trace/*.jsonl`) become metadata rows in `kit_trace_events`. Only `timestamp`, `tool_name`,
//! `is_error` and the kit task id (the file name; `_untagged` means none) are read; `summary` is never looked up
//! (FR-010). Only projects that still exist as folders are read (FR-009). Offsets, the replaced-inode and shrunk-file
//! restart, one transaction per file and the retire step copy `claude_code` (`folders.rs`).
//!
//! Rust differs from Python only where Python crashes and then stays stuck on every later run: a line nested deeper
//! than the JSON parser allows (`RecursionError`) or with a time outside years 1-9999 (`OverflowError`) is an
//! abstained row; an entry named `*.jsonl` that is not a regular file, one that cannot be stat'ed, opened or read, an
//! inode beyond `i64` and a path that is not UTF-8 are skipped and counted in [`Summary::skipped`]. No count or
//! error carries a path (NFR-002).

use super::{json, label, time};
use crate::writer::{Store, WriteError};
use rusqlite::{Transaction, TransactionBehavior, params};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub const SOURCE: &str = "kit_trace";
const UNTAGGED: &str = "_untagged";

/// The counts of one run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Summary {
    /// kit_trace.py `"files": len(seen)`.
    pub files: i64,
    /// kit_trace.py `new_rows`.
    pub new_events: i64,
    /// Not in Python (it crashes instead): entries skipped.
    pub skipped: i64,
}

/// `_projects`: the distinct project of the `claude_code` events that is a folder now, sorted by text.
pub(super) fn projects(store: &Store) -> Result<Vec<String>, WriteError> {
    let mut stmt = store.conn().prepare(
        "SELECT DISTINCT project_path FROM events \
         WHERE source = 'claude_code' AND project_path IS NOT NULL ORDER BY project_path",
    )?;
    let all: Vec<String> = stmt
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(all.into_iter().filter(|p| Path::new(p).is_dir()).collect())
}

/// `parse_line`: `(ts_utc, tool_name, is_error)`, or `None` to abstain.
fn parse_line(line: &[u8]) -> Option<(String, String, i64)> {
    let record = json::loads(line)?;
    if !record.is_dict() {
        return None;
    }
    let ts = record.get("timestamp").as_str().and_then(time::utc_text)?;
    let tool = label::label(record.get("tool_name"))?;
    match record.get("is_error") {
        json::Value::Bool(error) => Some((ts, tool, i64::from(*error))),
        _ => None,
    }
}

/// `line.strip()` on bytes is empty.
fn is_blank(line: &[u8]) -> bool {
    line.iter()
        .all(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C))
}

/// `_save`: one file's new rows and its offset in one transaction; the rows added. A read error rolls the file back and
/// comes out as `Ok(Err(e))` (the store is unchanged).
fn save(
    store: &Store,
    path: &str,
    inode: i64,
    offset: i64,
    project: &str,
    task_id: Option<&str>,
    reader: &mut impl BufRead,
) -> Result<io::Result<i64>, WriteError> {
    let mid = store.machine_id();
    let tx = Transaction::new_unchecked(store.conn(), TransactionBehavior::Immediate)?;
    let (mut added, mut line_offset) = (0_i64, offset);
    let mut buf = Vec::new();
    {
        let mut insert = tx.prepare(
            "INSERT OR IGNORE INTO kit_trace_events (machine_id, source_path, inode, byte_offset, \
             project_path, kit_task_id, abstained, ts_utc, tool_name, is_error) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?;
        loop {
            buf.clear();
            if let Err(e) = reader.read_until(b'\n', &mut buf) {
                return Ok(Err(e));
            }
            if buf.last() != Some(&b'\n') {
                break; // end of file, or a last line still being written: never consumed
            }
            if !is_blank(&buf) {
                let row = parse_line(&buf);
                let (ts, tool, error) = match &row {
                    Some((ts, tool, error)) => (Some(ts), Some(tool), Some(*error)),
                    None => (None, None, None),
                };
                added += insert.execute(params![
                    mid,
                    path,
                    inode,
                    line_offset,
                    project,
                    task_id,
                    i64::from(row.is_none()),
                    ts,
                    tool,
                    error
                ])? as i64;
            }
            line_offset += buf.len() as i64;
        }
    }
    tx.execute(
        "INSERT INTO source_files (machine_id, source, path, inode, byte_offset) \
         VALUES (?, ?, ?, ?, ?) ON CONFLICT (machine_id, source, path) \
         DO UPDATE SET inode = excluded.inode, byte_offset = excluded.byte_offset",
        params![mid, SOURCE, path, inode, line_offset],
    )?;
    tx.commit()?;
    Ok(Ok(added))
}

/// `ingest`: incrementally ingest the trace files of every existing project.
pub fn ingest(store: &mut Store) -> Result<Summary, WriteError> {
    let known = store.file_offsets(SOURCE)?;
    let mut seen: HashSet<String> = HashSet::new();
    let mut kept: HashSet<String> = HashSet::new(); // present but skipped: not retired
    let mut summary = Summary::default();
    for project in projects(store)? {
        // `Path(project)` drops repeated and trailing separators.
        let root: PathBuf = Path::new(&project).components().collect();
        let trace_dir = root.join("memory").join("event-trace");
        let Ok(entries) = fs::read_dir(&trace_dir) else {
            continue; // not a folder, or not readable: Python's glob yields nothing
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .filter(|e| e.file_name().as_bytes().ends_with(b".jsonl"))
            .map(|e| e.path())
            .collect();
        paths.sort();
        for path in paths {
            let Some(key) = path.to_str() else {
                summary.skipped += 1;
                continue;
            };
            // `path.stat()` follows a symlink; a broken one is `FileNotFoundError`, which Python skips (and retires).
            let info = match fs::metadata(&path) {
                Ok(info) => info,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(_) => {
                    summary.skipped += 1;
                    kept.insert(key.to_owned());
                    continue;
                }
            };
            if !info.is_file() {
                summary.skipped += 1; // Python: IsADirectoryError, for good; a FIFO would hang it
                continue;
            }
            let Ok(inode) = i64::try_from(info.ino()) else {
                summary.skipped += 1;
                kept.insert(key.to_owned());
                continue;
            };
            let (known_inode, mut offset) = known.get(key).copied().unwrap_or((inode, 0));
            if known_inode != inode || offset > info.len() as i64 {
                offset = 0; // replaced or shrunk file: start over
            }
            let task_id = path
                .file_stem()
                .map(|stem| stem.as_bytes())
                .filter(|stem| *stem != UNTAGGED.as_bytes())
                .and_then(|stem| label::label(&json::Value::Str(stem.to_vec())));
            let opened = File::open(&path).and_then(|mut file| {
                file.seek(SeekFrom::Start(u64::try_from(offset).unwrap_or(0)))?;
                Ok(BufReader::new(file))
            });
            let mut reader = match opened {
                Ok(reader) => reader,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(_) => {
                    summary.skipped += 1;
                    kept.insert(key.to_owned());
                    continue;
                }
            };
            match save(
                store,
                key,
                inode,
                offset,
                &project,
                task_id.as_deref(),
                &mut reader,
            )? {
                Ok(added) => summary.new_events += added,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(_) => {
                    summary.skipped += 1;
                    kept.insert(key.to_owned());
                    continue;
                }
            }
            seen.insert(key.to_owned());
        }
    }
    let gone: Vec<&String> = known
        .keys()
        .filter(|p| !seen.contains(*p) && !kept.contains(*p))
        .collect();
    store.retire_files(SOURCE, &gone)?;
    summary.files = seen.len() as i64;
    Ok(summary)
}
