//! `GET /api/kit/report-files` and `GET /api/kit/report-file`, ported from `src/easy_personal/core/kit.py`
//! (`report_files`, `report_file`, `_report_dir`, `_check_report_path`, `ReportNotFound`, `ReportTooLarge`; FR-090,
//! NFR-002). Read-only: nothing here writes a file or the store.
//!
//! **Nothing outside the report folder can be read.** The repository folder comes from the store row only (never from
//! the request). From it every level (the report folder, each sub-folder, the file) is opened *relative to the handle of
//! its parent* with `O_NOFOLLOW`, so no level can be a symlink, whatever happens between a check and the open: the
//! std library has no `openat`, and this crate takes no dependency for it, so a level is opened through
//! `/proc/self/fd/<parent handle>/<name>`, which the kernel resolves exactly as `openat(parent, name)`. The file is
//! opened with `O_NOFOLLOW | O_NONBLOCK` (a FIFO never blocks), then everything is judged on the **open handle**:
//! regular file, one link, size, and at most the cap + 1 bytes are read from it. A request path is refused before any
//! file system call when it has `..`, a leading `/`, a NUL, a backslash, an empty part or a part that starts with a dot.
//! Every miss is the same fixed value (`NotFound`, `TooLarge`): no error carries a path, a name or a byte of content.
//!
//! The flags are the Linux values of x86, x86-64, arm, aarch64 and riscv64 (`fcntl.h`; the two groups differ). Any other
//! target (macOS and Windows are later phases) compiles without them and answers `NotFound` to every call.
//!
//! The listing (`report_files`) is Python's: to `REPORT_DEPTH` levels, newest first (`-mtime`, then path), 200 at
//! most, 2000 files counted; but it walks by handle too, and a name that is not UTF-8 is left out (Python's JSON
//! encoder fails on it: a 500 with a traceback there).

use crate::kit::file_time;
use crate::metrics::MetricsError;
use crate::workspaces;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::fs::File;
use std::io::{self, Read};

const REPORT_WORD: &str = "report";
const FOLDERS_PER_REPO: usize = 8;
/// Files counted per report folder; past it the list says `truncated`.
const FILE_CAP: usize = 2000;
/// A file directly in the report folder is depth 1.
const DEPTH: usize = 3;
const LIST_CAP: usize = 200;
/// A file over this is listed, not openable, and not served.
pub const READ_CAP: u64 = 10 * 1024 * 1024;
const PATH_MAX_CHARS: usize = 512;
/// The content type comes from the extension, never from the content.
const TYPES: [(&str, &str); 9] = [
    ("html", "text/html; charset=utf-8"),
    ("htm", "text/html; charset=utf-8"),
    ("md", "text/markdown; charset=utf-8"),
    ("txt", "text/plain; charset=utf-8"),
    ("json", "application/json"),
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("pdf", "application/pdf"),
];

/// Every miss has a fixed text and never echoes the input.
#[derive(Debug)]
pub enum ReportError {
    /// 404 `not found`: any miss, any refusal.
    NotFound,
    /// 413 `too large`.
    TooLarge,
    Store(MetricsError),
}

impl From<MetricsError> for ReportError {
    fn from(error: MetricsError) -> Self {
        ReportError::Store(error)
    }
}

#[cfg(all(
    target_os = "linux",
    any(
        target_arch = "x86_64",
        target_arch = "x86",
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "riscv64"
    )
))]
pub(crate) mod sys {
    use std::fs::{File, OpenOptions};
    use std::io;
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

    const O_NONBLOCK: i32 = 0o4000;
    #[cfg(any(target_arch = "x86_64", target_arch = "x86", target_arch = "riscv64"))]
    const O_DIRECTORY: i32 = 0o200_000;
    #[cfg(any(target_arch = "x86_64", target_arch = "x86", target_arch = "riscv64"))]
    const O_NOFOLLOW: i32 = 0o400_000;
    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    const O_DIRECTORY: i32 = 0o40_000;
    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    const O_NOFOLLOW: i32 = 0o100_000;

    /// The repository folder of a store row: its own path may hold symlinks (the owner put it there).
    pub fn open_root(path: &str) -> io::Result<File> {
        OpenOptions::new()
            .read(true)
            .custom_flags(O_DIRECTORY)
            .open(path)
    }

    /// `openat(parent, name, O_NOFOLLOW | O_DIRECTORY)`: `name` is one path part (no `/`, no NUL: checked by the callers).
    pub fn open_dir_at(parent: &File, name: &str) -> io::Result<File> {
        open_at(parent, name, O_NOFOLLOW | O_DIRECTORY)
    }

    /// `openat(parent, name, O_NOFOLLOW | O_NONBLOCK)`.
    pub fn open_file_at(parent: &File, name: &str) -> io::Result<File> {
        open_at(parent, name, O_NOFOLLOW | O_NONBLOCK)
    }

    /// The path of an open directory handle, to list it or to open a child of it.
    pub fn handle_path(dir: &File) -> String {
        format!("/proc/self/fd/{}", dir.as_raw_fd())
    }

    /// CPython's `st_mtime`: `sec + nsec * 1e-9`.
    pub fn mtime(meta: &std::fs::Metadata) -> f64 {
        meta.mtime() as f64 + meta.mtime_nsec() as f64 * 1e-9
    }

    pub fn links(meta: &std::fs::Metadata) -> u64 {
        meta.nlink()
    }

    fn open_at(parent: &File, name: &str, flags: i32) -> io::Result<File> {
        OpenOptions::new()
            .read(true)
            .custom_flags(flags)
            .open(format!("{}/{name}", handle_path(parent)))
    }
}

#[cfg(not(all(
    target_os = "linux",
    any(
        target_arch = "x86_64",
        target_arch = "x86",
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "riscv64"
    )
)))]
pub(crate) mod sys {
    use std::fs::File;
    use std::io;

    fn unsupported<T>() -> io::Result<T> {
        Err(io::ErrorKind::Unsupported.into())
    }
    pub fn open_root(_path: &str) -> io::Result<File> {
        unsupported()
    }
    pub fn open_dir_at(_parent: &File, _name: &str) -> io::Result<File> {
        unsupported()
    }
    pub fn open_file_at(_parent: &File, _name: &str) -> io::Result<File> {
        unsupported()
    }
    pub fn handle_path(_dir: &File) -> String {
        String::new()
    }
    pub fn mtime(_meta: &std::fs::Metadata) -> f64 {
        0.0
    }
    pub fn links(_meta: &std::fs::Metadata) -> u64 {
        0
    }
}

/// Python's `_report_args` for `repo_no`: plain ASCII digits, fewer than 9; anything else is a miss.
pub fn parse_repo_no(text: &str) -> Option<usize> {
    if text.is_empty() || text.len() >= 9 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// `os.path.splitext(name)[1][1:].lower()`: leading dots are not an extension separator.
fn extension(name: &str) -> String {
    let stripped = name.trim_start_matches('.');
    stripped
        .rfind('.')
        .map_or(String::new(), |i| stripped[i + 1..].to_lowercase())
}

fn content_type(name: &str) -> Option<&'static str> {
    let ext = extension(name);
    TYPES.iter().find(|(e, _)| *e == ext).map(|(_, t)| *t)
}

/// The report folder of a request, opened: non-orphan workspace, repo number, a folder the FR-089 rule names
/// (depth 1 under the repo, `report` in the lowercase name, a real directory, 8 at most, by name).
fn open_report_dir(
    conn: &Connection,
    workspace: Option<&str>,
    repo_no: Option<usize>,
    folder: Option<&str>,
) -> Result<File, ReportError> {
    let (Some(workspace), Some(repo_no), Some(folder)) = (workspace, repo_no, folder) else {
        return Err(ReportError::NotFound);
    };
    let listed = workspaces::list_all(conn)?;
    let repo = listed["workspaces"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|w| w["name"].as_str() == Some(workspace) && w["is_orphan"] != true)
        .find_map(|w| w["repos"].get(repo_no).and_then(Value::as_str))
        .ok_or(ReportError::NotFound)?;
    let root = sys::open_root(repo).map_err(|_| ReportError::NotFound)?;
    if !report_names(&root).iter().any(|name| name == folder) {
        return Err(ReportError::NotFound);
    }
    sys::open_dir_at(&root, folder).map_err(|_| ReportError::NotFound)
}

/// The report folder names of a repo handle (a symlink is not one); sorted, 8 at most.
fn report_names(root: &File) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(sys::handle_path(root)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| name.to_lowercase().contains(REPORT_WORD))
        .collect();
    names.sort();
    names.truncate(FOLDERS_PER_REPO);
    names
}

/// The regular files of one report folder (FR-090), to `DEPTH` levels, newest first, 200 at most. `path` is relative to
/// the report folder, `/` separated. No symlink is followed; no file is opened.
pub fn report_files(
    conn: &Connection,
    workspace: Option<&str>,
    repo_no: Option<usize>,
    folder: Option<&str>,
) -> Result<Value, ReportError> {
    let top = open_report_dir(conn, workspace, repo_no, folder)?;
    let mut found: Vec<(f64, String, Option<String>, u64)> = Vec::new(); // (mtime, path, mtime_utc, size)
    let mut capped = false;
    let mut pending: Vec<(Vec<String>, String)> = vec![(Vec::new(), String::new())];
    while let Some((chain, prefix)) = pending.pop() {
        let depth = chain.len() + 1;
        let Some(dir) = open_chain(&top, &chain) else {
            if depth == 1 {
                return Err(ReportError::NotFound);
            }
            continue;
        };
        let Ok(entries) = std::fs::read_dir(sys::handle_path(&dir)) else {
            if depth == 1 {
                return Err(ReportError::NotFound);
            }
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let Ok(name) = entry.file_name().into_string() else {
                continue; // not UTF-8: cannot be shown or asked for
            };
            if kind.is_file() {
                if found.len() == FILE_CAP {
                    capped = true;
                    break;
                }
                let Ok(meta) = entry.metadata() else {
                    continue;
                };
                let mtime = sys::mtime(&meta);
                let stamp = meta.modified().ok().and_then(file_time).map(|t| t.1);
                found.push((mtime, format!("{prefix}{name}"), stamp, meta.len()));
            } else if depth < DEPTH && kind.is_dir() {
                let mut next = chain.clone();
                next.push(name.clone());
                pending.push((next, format!("{prefix}{name}/")));
            }
        }
        if capped {
            break;
        }
    }
    found.sort_by(|(ma, pa, ..), (mb, pb, ..)| {
        mb.partial_cmp(ma)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| pa.cmp(pb))
    });
    let truncated = capped || found.len() > LIST_CAP;
    let files: Vec<Value> = found
        .into_iter()
        .take(LIST_CAP)
        .map(|(_, path, stamp, size)| {
            json!({
                "path": path,
                "size": size,
                "openable": content_type(&path).is_some() && size <= READ_CAP,
                "mtime_utc": stamp,
            })
        })
        .collect();
    Ok(json!({"files": files, "truncated": truncated}))
}

/// The directory `chain` below `top`, each level opened from the handle of the one before with `O_NOFOLLOW`.
fn open_chain(top: &File, chain: &[String]) -> Option<File> {
    let mut dir = top.try_clone().ok()?;
    for part in chain {
        dir = sys::open_dir_at(&dir, part).ok()?;
    }
    Some(dir)
}

/// Python's `_check_report_path`, plus the hardening the guide asks: a part that starts with a dot is refused.
fn check_path(path: Option<&str>) -> Result<Vec<&str>, ReportError> {
    let path = path.ok_or(ReportError::NotFound)?;
    if path.is_empty()
        || path.chars().count() > PATH_MAX_CHARS
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains('\0')
    {
        return Err(ReportError::NotFound);
    }
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() > DEPTH || parts.iter().any(|p| p.is_empty() || p.starts_with('.')) {
        return Err(ReportError::NotFound);
    }
    if content_type(parts[parts.len() - 1]).is_none() {
        return Err(ReportError::NotFound);
    }
    Ok(parts)
}

/// At most `cap` + 1 bytes of `source`: one more than the cap tells the caller it is too large, and an endless source
/// still stops.
fn read_capped(source: impl Read, cap: u64) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    source.take(cap + 1).read_to_end(&mut data)?;
    Ok(data)
}

/// `(content type, bytes)` of one file of a report folder. Judged on the open handle; any read error is `NotFound`.
pub fn report_file(
    conn: &Connection,
    workspace: Option<&str>,
    repo_no: Option<usize>,
    folder: Option<&str>,
    path: Option<&str>,
) -> Result<(&'static str, Vec<u8>), ReportError> {
    report_file_capped(conn, workspace, repo_no, folder, path, READ_CAP)
}

/// `report_file` with the cap as a parameter (a test asks for a small one).
pub fn report_file_capped(
    conn: &Connection,
    workspace: Option<&str>,
    repo_no: Option<usize>,
    folder: Option<&str>,
    path: Option<&str>,
    cap: u64,
) -> Result<(&'static str, Vec<u8>), ReportError> {
    let parts = check_path(path)?;
    let (last, dirs) = parts.split_last().ok_or(ReportError::NotFound)?;
    let kind = content_type(last).ok_or(ReportError::NotFound)?;
    let mut dir = open_report_dir(conn, workspace, repo_no, folder)?;
    for part in dirs {
        dir = sys::open_dir_at(&dir, part).map_err(|_| ReportError::NotFound)?;
    }
    let file = sys::open_file_at(&dir, last).map_err(|_| ReportError::NotFound)?;
    let meta = file.metadata().map_err(|_| ReportError::NotFound)?;
    // a FIFO, a device, a folder; a second link may be a name for a file kept elsewhere
    if !meta.file_type().is_file() || sys::links(&meta) != 1 {
        return Err(ReportError::NotFound);
    }
    if meta.len() > cap {
        return Err(ReportError::TooLarge);
    }
    let data = read_capped(&file, cap).map_err(|_| ReportError::NotFound)?;
    if data.len() as u64 > cap {
        return Err(ReportError::TooLarge); // it grew after the size check
    }
    Ok((kind, data))
}

#[cfg(test)]
mod tests {
    use super::{extension, parse_repo_no, read_capped};

    #[test]
    fn extension_is_pythons_splitext() {
        for (name, want) in [
            ("a.html", "html"),
            ("a.HTML", "html"),
            (".html", ""),
            (".a.html", "html"),
            ("a.", ""),
            ("a", ""),
            ("a.b.json", "json"),
            ("..html", ""),
        ] {
            assert_eq!(extension(name), want, "{name:?}");
        }
    }

    #[test]
    fn repo_no_is_plain_ascii_digits_under_nine() {
        assert_eq!(parse_repo_no("0"), Some(0));
        assert_eq!(parse_repo_no("007"), Some(7));
        assert_eq!(parse_repo_no("99999999"), Some(99_999_999));
        for bad in ["", "-1", "1e3", "99999999999", "\u{661}", "+1", " 1", "1 "] {
            assert_eq!(parse_repo_no(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn an_endless_source_stops_at_the_cap_plus_one() {
        let data = read_capped(std::io::repeat(b'x'), 10).unwrap();
        assert_eq!(data.len(), 11);
    }
}
