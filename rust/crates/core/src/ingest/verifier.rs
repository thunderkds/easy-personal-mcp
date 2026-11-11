//! The `verifier` source (P2b, T033): Python `sources/verifier.py`. For each project that still exists, the newest
//! `evidence-report-*.json` (by the UTC stamp in its name) from `<project>/reports/` and, when the project's folder name
//! is unique, from `<container>/<folder name>/`, becomes one row in `verifier_scores`. Only `schema_version` 1 is read
//! (another integer version makes the project `unknown`); only scope, `generated_at` and `overall` are kept. A
//! symlink, a non-regular file, a file over 1,000,000 bytes or one of another shape is skipped and counted.
//!
//! The table is replaced in one transaction (delete all, insert): a failure inside rolls back and leaves the old rows.
//! The result is then `{"status":"failed","reason":<kind>}` and never carries a path or content.
//!
//! Where Rust reads less than Python (decided for you): a score that is a fraction is read with `serde_json` from the
//! same bytes, so a report that is UTF-16/32 encoded, nested deeper than 128 levels, or holds `NaN` / `Infinity`
//! beside a fractional score is skipped here and read by Python; a stamp written with non-ASCII digits is not a stamp.

use super::json::{self, Value};
use super::{kit_trace, label, time};
use crate::writer::{Store, WriteError};
use rusqlite::{Transaction, TransactionBehavior, params};
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

/// `MAX_BYTES`.
pub const MAX_BYTES: u64 = 1_000_000;

// `os.O_NOFOLLOW | os.O_NONBLOCK` (std has no names for them; `libc` is not a dependency of this crate).
#[cfg(target_os = "linux")]
const O_NONBLOCK: i32 = 0o4000;
#[cfg(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64")))]
const O_NOFOLLOW: i32 = 0o100000;
#[cfg(all(
    target_os = "linux",
    not(any(target_arch = "arm", target_arch = "aarch64"))
))]
const O_NOFOLLOW: i32 = 0o400000;
#[cfg(not(target_os = "linux"))]
const O_NONBLOCK: i32 = 0x4;
#[cfg(not(target_os = "linux"))]
const O_NOFOLLOW: i32 = 0x100;

/// The counts of one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// `ok` or `no data`.
    pub status: &'static str,
    pub projects: i64,
    pub skipped: i64,
}

/// A readable report: a score, or a readable object of another schema version.
#[derive(Debug, Clone, PartialEq)]
enum Report {
    Unknown,
    Scored {
        scope: String,
        generated: String,
        value: Option<f64>,
        abstained: bool,
        reason: Option<String>,
    },
}

/// `_number`: an integer or a fraction in 0..=100 (a `bool` is not a number).
fn number(value: &Value, raw: &[u8]) -> Option<f64> {
    match value {
        Value::Int(n) if (0..=100).contains(n) => Some(*n as f64),
        Value::Float => {
            let raw = raw.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(raw);
            let doc: serde_json::Value = serde_json::from_slice(raw).ok()?;
            let f = doc.get("overall")?.get("value")?.as_f64()?;
            (0.0..=100.0).contains(&f).then_some(f)
        }
        _ => None,
    }
}

/// `parse_report`.
fn parse_report(raw: &[u8]) -> Option<Report> {
    let record = json::loads(raw)?;
    if !record.is_dict() {
        return None;
    }
    match record.get("schema_version") {
        Value::Int(1) => {}
        Value::Int(_) | Value::BigInt => return Some(Report::Unknown),
        _ => return None,
    }
    let scope = label::label(record.get("scope"))?;
    let generated = record
        .get("generated_at")
        .as_str()
        .and_then(time::utc_text)?;
    let overall = record.get("overall");
    if !overall.is_dict() {
        return None;
    }
    let Value::Bool(abstained) = overall.get("abstained") else {
        return None;
    };
    let reason = match overall.get("reason_code") {
        Value::Null => None,
        other => Some(label::label(other)?),
    };
    let value = number(overall.get("value"), raw);
    // abstained must carry a null value, a scored report a number in 0..=100
    if *abstained != value.is_none()
        || (!matches!(overall.get("value"), Value::Null) && value.is_none())
    {
        return None;
    }
    Some(Report::Scored {
        scope,
        generated,
        value,
        abstained: *abstained,
        reason,
    })
}

/// `_read`: the bytes, or `None` for a symlink, a file that is not regular or is over the cap, or one that cannot be read.
fn read(path: &Path, folder: &Path) -> Option<Vec<u8>> {
    if fs::canonicalize(path).ok()?.parent()? != fs::canonicalize(folder).ok()? {
        return None;
    }
    let file = File::options()
        .read(true)
        .custom_flags(O_NOFOLLOW | O_NONBLOCK)
        .open(path)
        .ok()?;
    let info = file.metadata().ok()?;
    if !info.is_file() || info.len() > MAX_BYTES {
        return None;
    }
    let mut raw = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut raw).ok()?;
    (raw.len() as u64 <= MAX_BYTES).then_some(raw)
}

/// `_STAMP.search(name)`: `\d{8}T\d{6}-\d+Z` right before `.json`, else empty (ASCII digits only).
fn stamp(name: &str) -> &str {
    let Some(stem) = name.strip_suffix(".json") else {
        return "";
    };
    let Some(body) = stem.strip_suffix('Z') else {
        return "";
    };
    let Some(dash) = body.rfind('-') else {
        return "";
    };
    let digits = |s: &[u8]| s.iter().all(u8::is_ascii_digit);
    let b = body.as_bytes();
    if dash < 15 || b.len() == dash + 1 || !digits(&b[dash + 1..]) {
        return "";
    }
    let head = &b[dash - 15..dash];
    if digits(&head[..8]) && head[8] == b'T' && digits(&head[9..]) {
        &stem[dash - 15..]
    } else {
        ""
    }
}

type Found = ((String, String), Report);

/// `_candidates`: the readable reports of one folder and the count of skipped files.
fn candidates(folder: &Path, root: &Path) -> (Vec<Found>, i64) {
    let (mut found, mut skipped) = (Vec::new(), 0);
    if !folder.is_dir() {
        return (found, skipped);
    }
    let (Ok(resolved), Ok(root)) = (fs::canonicalize(folder), fs::canonicalize(root)) else {
        return (found, skipped);
    };
    if resolved.parent() != Some(root.as_path()) {
        return (found, skipped);
    }
    let mut paths: Vec<PathBuf> = fs::read_dir(folder)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| {
                    let name = e.file_name();
                    let name = name.as_bytes();
                    name.len() >= 21
                        && name.starts_with(b"evidence-report-")
                        && name.ends_with(b".json")
                })
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default();
    paths.sort();
    for path in paths {
        let name = path.file_name().and_then(|n| n.to_str());
        let row = name
            .zip(read(&path, folder))
            .and_then(|(_, raw)| parse_report(&raw));
        match (row, name) {
            (Some(row), Some(name)) => found.push(((stamp(name).to_owned(), name.to_owned()), row)),
            _ => skipped += 1,
        }
    }
    (found, skipped)
}

/// `Path(p).name`.
fn folder_name(project: &str) -> String {
    Path::new(project)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The exception class name Python would report for a store failure.
fn kind(e: &WriteError) -> &'static str {
    match e {
        WriteError::Rejected => "IntegrityError",
        _ => "OperationalError",
    }
}

/// `ingest`: store the newest report per existing project. `container` is `None` only when no folder can be
/// worked out (no `HOME`): Python's `RuntimeError`. Never fails the run: the error is the exception class name.
pub fn ingest(store: &mut Store, container: Option<&Path>) -> Result<Summary, &'static str> {
    let container = container.ok_or("RuntimeError")?;
    let projects = kit_trace::projects(store).map_err(|e| kind(&e))?;
    let names: Vec<String> = projects.iter().map(|p| folder_name(p)).collect();
    let mut rows: Vec<(String, Report)> = Vec::new();
    let mut skipped = 0;
    for (project, name) in projects.iter().zip(&names) {
        let repo: PathBuf = Path::new(project).components().collect();
        let mut folders = vec![(repo.join("reports"), repo.clone())];
        if names.iter().filter(|n| *n == name).count() == 1 {
            // two projects with one folder name: the shared folder is ambiguous
            folders.push((container.join(name), container.to_path_buf()));
        }
        let mut best: Option<Found> = None;
        for (folder, root) in folders {
            let (items, bad) = candidates(&folder, &root);
            skipped += bad;
            for item in items {
                if best.as_ref().is_none_or(|b| item.0 > b.0) {
                    best = Some(item); // `max` keeps the first of equal keys
                }
            }
        }
        if let Some((_, report)) = best {
            rows.push((project.clone(), report));
        }
    }
    replace(store, &rows).map_err(|e| kind(&e))?;
    Ok(Summary {
        status: if rows.is_empty() { "no data" } else { "ok" },
        projects: rows.len() as i64,
        skipped,
    })
}

/// `DELETE FROM verifier_scores` and the inserts, in one transaction.
fn replace(store: &Store, rows: &[(String, Report)]) -> Result<(), WriteError> {
    let tx = Transaction::new_unchecked(store.conn(), TransactionBehavior::Immediate)?;
    tx.execute("DELETE FROM verifier_scores", [])?;
    {
        let mut insert = tx.prepare("INSERT INTO verifier_scores VALUES (?, ?, ?, ?, ?, ?, ?)")?;
        for (project, report) in rows {
            match report {
                Report::Unknown => insert.execute(params![
                    project,
                    "unknown",
                    None::<String>,
                    None::<String>,
                    None::<f64>,
                    0,
                    None::<String>
                ])?,
                Report::Scored {
                    scope,
                    generated,
                    value,
                    abstained,
                    reason,
                } => insert.execute(params![
                    project,
                    "ok",
                    scope,
                    generated,
                    value,
                    i64::from(*abstained),
                    reason
                ])?,
            };
        }
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::stamp;

    #[test]
    fn the_stamp_is_the_utc_time_in_the_name() {
        assert_eq!(
            stamp("evidence-report-20260102T030405-17Z.json"),
            "20260102T030405-17Z"
        );
        assert_eq!(stamp("evidence-report-latest.json"), "");
        assert_eq!(stamp("evidence-report-20260102T030405-Z.json"), "");
        assert_eq!(stamp("evidence-report-20260102T030405-1-2Z.json"), "");
        assert_eq!(stamp("evidence-report-2026010T030405-1Z.json"), "");
        assert_eq!(stamp("x20260102T030405-1Z.json"), "20260102T030405-1Z");
    }
}
