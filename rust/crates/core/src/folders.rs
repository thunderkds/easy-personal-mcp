//! Folder names for the dashboard's folder browser (HUN-203, FR-068). Read-only.
//!
//! Port of `src/easy_personal/core/folders.py`. Sub-folder names and a `repo` flag only: never a file name, size,
//! content or symlink target. The path asked for is never logged and never put in an error.

use crate::metrics::realpath;
use crate::scope::casefold;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

pub const MAX_DIRS: usize = 500;

/// Why a listing was refused; the texts are Python's and carry no path.
#[derive(Debug, PartialEq)]
pub enum DirsError {
    /// 400: `Path must be absolute` or `Not an existing folder`.
    Invalid(&'static str),
    /// 403: the folder cannot be read.
    Unreadable,
}

impl DirsError {
    pub fn text(&self) -> &'static str {
        match self {
            DirsError::Invalid(text) => text,
            DirsError::Unreadable => "Cannot read this folder",
        }
    }
}

/// `$HOME` (Python's `Path.home()`); `/` when it is not set (Python would ask the account database).
fn home() -> String {
    std::env::var("HOME")
        .ok()
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "/".to_string())
}

/// Sub-folders of `path` (default: the working folder, else home when that was deleted).
pub fn list_dirs(path: Option<&str>) -> Result<Value, DirsError> {
    let path = match path.filter(|p| !p.is_empty()) {
        Some(p) => p.to_string(),
        None => std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| home()),
    };
    if !path.starts_with('/') {
        return Err(DirsError::Invalid("Path must be absolute"));
    }
    let not_a_folder = DirsError::Invalid("Not an existing folder");
    if path.contains('\0') {
        return Err(not_a_folder);
    }
    let here = PathBuf::from(realpath(&path));
    if !here.is_dir() {
        return Err(not_a_folder);
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(&here).map_err(|_| DirsError::Unreadable)? {
        let entry = entry.map_err(|_| DirsError::Unreadable)?;
        // a name that is not UTF-8 cannot be sent as JSON: skipped
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let is_dir = match entry.file_type() {
            // a link is followed for this one test only; a broken link or a loop is not a folder
            Ok(kind) if kind.is_symlink() => fs::metadata(entry.path()).is_ok_and(|m| m.is_dir()),
            Ok(kind) => kind.is_dir(),
            Err(_) => false,
        };
        if is_dir {
            names.push(name);
        }
    }
    names.sort_by_cached_key(|n| (casefold(n), n.clone()));
    let cut = names.len() > MAX_DIRS;
    let dirs: Vec<Value> = names
        .iter()
        .take(MAX_DIRS)
        .map(
            |n| json!({"name": n, "repo": fs::symlink_metadata(here.join(n).join(".git")).is_ok()}),
        )
        .collect();
    let mut body = json!({
        "path": here.to_string_lossy(),
        "parent": parent_of(&here),
        "home": home(),
        "dirs": dirs,
    });
    if cut {
        body["truncated"] = Value::Bool(true);
    }
    Ok(body)
}

fn parent_of(here: &Path) -> Option<String> {
    here.parent().map(|p| p.to_string_lossy().into_owned())
}
