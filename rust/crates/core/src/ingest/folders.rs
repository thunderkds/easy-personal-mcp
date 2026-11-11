//! The `claude_code` source over real folders (P2a, T035): Python `sources/claude_code.py` `ingest_accounts`,
//! `account_roots` and `_scan`, plus `core/discovery.py` `resolve_project`. Finds every transcript, picks its account,
//! resolves each `cwd` to its project through `git`, resumes from the stored offset, and saves each file through
//! [`Store::save_file`] (one transaction per file).
//!
//! Rust differs from Python only where Python crashes and then stays stuck on every later run (`TASK_REVIEW_T026.md`,
//! findings): an entry named `*.jsonl` that is not a regular file (S06c, `IsADirectoryError`), an entry it cannot
//! stat, open or read (`PermissionError`, `OSError`), a path that is not UTF-8 (Python cannot store it), and an inode
//! beyond `i64`. Each is skipped and counted in [`Summary::skipped`]; the files after it are read; an existing file
//! skipped this way keeps its stored offset (it is not retired). No error or count carries a path.

use super::accounts;
use super::claude_code::LineReader;
use crate::metrics::repo_of;
use crate::writer::{Line, Store, WriteError};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufReader, Seek, SeekFrom};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// `SOURCE`.
pub const SOURCE: &str = "claude_code";

/// What the caller (T029) read from the environment and the process; nothing here reads env vars.
#[derive(Debug, Clone)]
pub struct Options {
    /// `config.agent_root()`: the folder [`accounts::detect`] scans for `.claude*` / `.codex*`.
    pub agent_root: PathBuf,
    /// `--source PATH` / `EASY_PERSONAL_CLAUDE_PROJECTS`: replaces the default account's projects folder.
    pub projects_override: Option<PathBuf>,
    /// The Python process's cwd: a relative transcript `cwd` is looked up from here (`Path(cwd).is_dir()`, `git -C`).
    pub cwd_base: PathBuf,
}

/// The counts of one run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Summary {
    /// claude_code.py:270 `"files": len(seen)`: distinct files read this run (a file with nothing new counts).
    pub files: i64,
    /// claude_code.py:264,270 `new_events += store.save_file(...)`: rows added this run.
    pub new_events: i64,
    /// Not in Python (it crashes instead): entries skipped because they could not be read as a transcript.
    pub skipped: i64,
}

/// The fixed, path-free text for a run with skipped entries (NFR-002).
pub const SKIPPED_NOTE: &str = "some transcript entries could not be read and were skipped";

/// `ingest_accounts` as the CLI runs it (cli.py:166-167): `accounts.detect`, then the `NULL -> default account`
/// update (claude_code.py:299-302), then the scan of every confirmed Claude account's folder.
pub fn ingest_accounts(store: &mut Store, opts: &Options) -> Result<Summary, WriteError> {
    accounts::detect(store, &opts.agent_root)?;
    if let Some(default) = accounts::default_id(store)? {
        // Rows stored before migration 5 carry no account: they are the default's.
        store.conn().execute(
            "UPDATE events SET agent_account_id = ? WHERE agent_account_id IS NULL",
            [default],
        )?;
    }
    let roots = account_roots(store, opts.projects_override.as_deref())?;
    scan(store, &roots, &opts.cwd_base)
}

/// `account_roots`: `(id, <folder>/projects)` of every confirmed Claude account in id order; the override replaces
/// the default account's folder, and with no default account it is read untagged.
fn account_roots(
    store: &Store,
    projects_override: Option<&Path>,
) -> Result<Vec<(Option<i64>, PathBuf)>, WriteError> {
    let mut stmt = store.conn().prepare(
        "SELECT id, folder, config_dir_env FROM agent_accounts \
         WHERE provider = 'claude' AND status = 'confirmed' ORDER BY id",
    )?;
    let rows: Vec<(i64, String, Option<String>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let mut roots: Vec<(Option<i64>, PathBuf)> = rows
        .iter()
        .map(|(id, folder, env)| {
            let root = match projects_override {
                Some(o) if env.is_none() => o.to_path_buf(),
                _ => Path::new(folder).join("projects"),
            };
            (Some(*id), root)
        })
        .collect();
    if let Some(o) = projects_override
        && !rows.iter().any(|(_, _, env)| env.is_none())
    {
        roots.push((None, o.to_path_buf()));
    }
    // `Path(...)` drops repeated and trailing separators, so the keys and the retire prefixes below match Python's.
    Ok(roots
        .into_iter()
        .map(|(id, root)| (id, root.components().collect()))
        .collect())
}

/// `sorted(root.rglob("*.jsonl"))`: every entry named `*.jsonl` under `root` (any kind; the caller checks), never
/// descending into a symlinked folder, sorted by path components (`p/` before `p-q/`). Unreadable folders are
/// skipped and counted.
fn discover(root: &Path, skipped: &mut i64) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            *skipped += 1;
            continue;
        };
        for entry in entries {
            let Ok(entry) = entry else {
                *skipped += 1;
                continue;
            };
            let path = entry.path();
            if entry.file_name().as_bytes().ends_with(b".jsonl") {
                found.push(path.clone());
            }
            // `DirEntry::file_type` does not follow symlinks: a symlinked folder is not entered.
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                pending.push(path);
            }
        }
    }
    found.sort(); // `Path` orders by components, as `PurePath.__lt__`
    found
}

/// `discovery.resolve_project`: a `cwd` that is not an existing folder is its own project; else its git repo (a
/// worktree gives its main repo, `metrics::repo_of`, 5 s per git call), else the `cwd` itself. A relative `cwd` is
/// looked up from `base`, the returned text is the `cwd` as written when git gives no answer.
fn resolve_project(cwd: &str, base: &Path) -> String {
    let folder = base.join(cwd);
    if !folder.is_dir() {
        return cwd.to_owned();
    }
    folder
        .to_str()
        .and_then(repo_of)
        .unwrap_or_else(|| cwd.to_owned())
}

/// `read_complete_lines` + `_file_events` from `offset`, streamed (memory bounded by the longest line): the lines go
/// straight into [`Store::save_file_streamed`], never collected (T032).
fn read_from(path: &Path, offset: u64) -> io::Result<LineReader<BufReader<File>>> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    Ok(LineReader::new(BufReader::new(file), offset as i64))
}

/// `_scan`: ingest every transcript of each `(account, root)`; retire the files that vanished under a root that exists
/// now.
fn scan(
    store: &mut Store,
    roots: &[(Option<i64>, PathBuf)],
    base: &Path,
) -> Result<Summary, WriteError> {
    let known = store.file_offsets(SOURCE)?;
    let mut projects: HashMap<String, String> = HashMap::new(); // cwd -> project, once per run
    let mut seen: HashSet<String> = HashSet::new();
    let mut kept: HashSet<String> = HashSet::new(); // present but skipped: not retired
    let mut summary = Summary::default();
    // claude_code.py:254 `scanned = [str(root) + os.sep for _, root in roots if root.is_dir()]`
    let scanned: Vec<String> = roots
        .iter()
        .filter(|(_, root)| root.is_dir())
        .filter_map(|(_, root)| root.to_str().map(|r| format!("{r}/")))
        .collect();
    for (account, root) in roots {
        if !root.is_dir() {
            continue;
        }
        for path in discover(root, &mut summary.skipped) {
            let Some(key) = path.to_str() else {
                summary.skipped += 1;
                continue;
            };
            // `path.stat()` follows a file symlink.
            let info = match fs::metadata(&path) {
                Ok(info) => info,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue, // removed mid-run
                Err(_) => {
                    summary.skipped += 1;
                    kept.insert(key.to_owned());
                    continue;
                }
            };
            if !info.is_file() {
                summary.skipped += 1; // S06c: a folder named `x.jsonl` (Python: IsADirectoryError)
                continue;
            }
            let Ok(inode) = i64::try_from(info.ino()) else {
                summary.skipped += 1;
                kept.insert(key.to_owned());
                continue;
            };
            // claude_code.py:260-262: a replaced (new inode) or shrunk file starts over. No mtime, no hash.
            let (known_inode, mut offset) = known.get(key).copied().unwrap_or((inode, 0));
            if known_inode != inode || offset > info.len() as i64 {
                offset = 0;
            }
            let lines = match u64::try_from(offset).map(|o| read_from(&path, o)) {
                Ok(Ok(lines)) => lines,
                Ok(Err(e)) if e.kind() == io::ErrorKind::NotFound => continue,
                _ => {
                    summary.skipped += 1;
                    kept.insert(key.to_owned());
                    continue;
                }
            };
            let lines = lines.map(|line| {
                line.map(|mut line: Line| {
                    if let Some(event) = line.event.as_mut()
                        && let Some(cwd) = event.project_path.take()
                    {
                        let project = projects
                            .entry(cwd)
                            .or_insert_with_key(|cwd| resolve_project(cwd, base));
                        event.project_path = Some(project.clone());
                    }
                    line
                })
            });
            // A read error part-way rolls the file back: skipped exactly as an error before the first line was.
            match store.save_file_streamed(SOURCE, key, inode, offset, lines, *account)? {
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
    // claude_code.py:268-269
    let gone: Vec<&String> = known
        .keys()
        .filter(|p| !seen.contains(*p) && !kept.contains(*p))
        .filter(|p| scanned.iter().any(|s| p.starts_with(s.as_str())))
        .collect();
    store.retire_files(SOURCE, &gone)?;
    summary.files = seen.len() as i64;
    Ok(summary)
}
