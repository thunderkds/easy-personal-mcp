//! The read-only git diff of a repo (T136, FR-139, FR-140, DDR-0010): worktrees, refs, changed files and one file's patch.
//! **The seventh exception of NFR-002**: what these functions return goes to the dashboard page and nowhere else; nothing
//! here is stored, cached or logged, and no [`DiffError`] holds any of it.
//!
//! A repo's own config is not trusted to start a program (a plain `git diff` runs its clean filters and its external
//! diff): every call switches off fsmonitor, hooks and signature checks, every diff switches off external diff and
//! textconv and overrides each configured filter by name, submodule work trees are not entered, no call writes the index
//! (`diff.autoRefreshIndex=false`, no optional locks) and none may fetch. The filters are read in the folder every
//! later call runs in, the root of the work tree ([`toplevel`]): another repo's config would bring other filters. A ref
//! reaches `git diff` only as the commit id `rev-parse` gave for it; a path only as a `:(literal)` pathspec of a file the
//! list answer holds.

use crate::live::clean_block;
use crate::report_files::sys;
use serde_json::{Value, json};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// The most files one list answers.
pub const FILES_MAX: usize = 2000;
/// The most bytes and lines of one patch.
pub const PATCH_MAX_BYTES: usize = 1024 * 1024;
pub const PATCH_MAX_LINES: usize = 5000;
/// The most bytes read of a git answer that is not a patch, and of one part of a file list.
const OUTPUT_MAX: usize = 4 * 1024 * 1024;
const LIST_MAX: usize = 16 * 1024 * 1024;
/// The most bytes of untracked files one list reads to count their lines (each file at most [`PATCH_MAX_BYTES`]).
const COUNT_BUDGET: usize = 16 * 1024 * 1024;
/// The most paths one `hash-object` call is given.
const HASH_PATHS: usize = 500;
const GIT_TIMEOUT: Duration = Duration::from_secs(10);
const COMMITS_MAX: &str = "100";
const REFS_MAX: usize = 500;
const WORKTREES_MAX: usize = 100;
const REF_MAX_CHARS: usize = 200;
/// A file with a NUL byte in its first bytes is binary (git's own rule).
const BINARY_PROBE: usize = 8000;

/// What is compared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compare {
    Uncommitted,
    /// The merge base of `base` (else the default base) and `HEAD`, with `HEAD`.
    Branch {
        base: Option<String>,
    },
    /// One commit against its first parent.
    Commit {
        rev: String,
    },
    /// Two refs, directly.
    Range {
        base: String,
        head: String,
    },
}

impl Compare {
    /// From the query's `compare`, `base` and `head`; an empty value is absent.
    pub fn parse(
        compare: Option<&str>,
        base: Option<&str>,
        head: Option<&str>,
    ) -> Result<Compare, DiffError> {
        let value = |v: Option<&str>| v.filter(|v| !v.is_empty()).map(str::to_owned);
        match (compare, value(base), value(head)) {
            (Some("uncommitted"), _, _) => Ok(Compare::Uncommitted),
            (Some("branch"), base, _) => Ok(Compare::Branch { base }),
            (Some("commit"), _, Some(rev)) => Ok(Compare::Commit { rev }),
            (Some("range"), Some(base), Some(head)) => Ok(Compare::Range { base, head }),
            _ => Err(DiffError::BadCompare),
        }
    }
}

/// Why there is no answer. No variant holds a path, a ref or any text of the repo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffError {
    NotARepo,
    BadCompare,
    UnknownRef,
    NoBase,
    NoCommonHistory,
    NotListed,
    Filter,
    TooSlow,
    Failed,
}

impl DiffError {
    /// The one fixed line the page shows.
    pub fn text(self) -> &'static str {
        match self {
            DiffError::NotARepo => "this folder is not a git work tree",
            DiffError::BadCompare => {
                "compare must be uncommitted, branch, commit (with head) or range (with base and head)"
            }
            DiffError::UnknownRef => "no commit has that name in this repo",
            DiffError::NoBase => "no base branch was found; pick one",
            DiffError::NoCommonHistory => "these two have no common history",
            DiffError::NotListed => "that file is not in this compare; refresh the list",
            DiffError::Filter => {
                "this repo has a git filter that cannot be switched off, so its uncommitted changes are not read"
            }
            DiffError::TooSlow => "git took too long",
            DiffError::Failed => "git could not read this repo",
        }
    }
}

/// One work tree of a repo. `path` never leaves the app; the page gets `id` and `folder`.
#[derive(Debug, Clone)]
pub struct Worktree {
    pub id: String,
    pub path: PathBuf,
    pub folder: String,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub main: bool,
}

struct Output {
    ok: bool,
    code: Option<i32>,
    stdout: Vec<u8>,
    cut: bool,
}

/// One git call in `dir`, hardened as the module says. At most `cap` bytes of stdout are kept (`cut` says more came,
/// and git is then killed); past [`GIT_TIMEOUT`] it is killed and the answer is [`DiffError::TooSlow`].
fn run(dir: &Path, config: &[String], args: &[&str], cap: usize) -> Result<Output, DiffError> {
    let mut cmd = Command::new("git");
    cmd.arg("--no-pager").arg("-C").arg(dir);
    for pair in [
        "core.fsmonitor=",
        "core.untrackedCache=false",
        "core.hooksPath=/dev/null",
        "diff.orderFile=/dev/null", // git loads the whole file a repo names, whatever its size
        "core.quotePath=false",
        "log.showSignature=false",
        "diff.autoRefreshIndex=false",
        "diff.relative=false",
        "submodule.recurse=false",
    ] {
        cmd.args(["-c", pair]);
    }
    for pair in config {
        cmd.arg("-c").arg(pair);
    }
    // no `GIT_*` of the parent: they name other repos, indexes, config and diff programs
    for (key, _) in std::env::vars_os() {
        if key.as_encoded_bytes().starts_with(b"GIT_") {
            cmd.env_remove(key);
        }
    }
    let mut child = cmd
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| DiffError::Failed)?;
    let stdout = child.stdout.take().ok_or(DiffError::Failed)?;
    let (tx, rx) = mpsc::channel();
    // a reader thread, so a full pipe cannot block the time limit
    thread::spawn(move || {
        let mut buf = Vec::new();
        let read = stdout.take(cap as u64 + 1).read_to_end(&mut buf);
        let _ = tx.send(read.map(|_| buf));
    });
    let deadline = Instant::now() + GIT_TIMEOUT;
    let stop = |child: &mut std::process::Child| {
        let _ = child.kill();
        let _ = child.wait();
    };
    let mut buf = match rx.recv_timeout(GIT_TIMEOUT) {
        Ok(Ok(buf)) => buf,
        other => {
            stop(&mut child);
            return Err(match other {
                Err(mpsc::RecvTimeoutError::Timeout) => DiffError::TooSlow,
                _ => DiffError::Failed,
            });
        }
    };
    if buf.len() > cap {
        stop(&mut child);
        buf.truncate(cap);
        return Ok(Output {
            ok: true,
            code: None,
            stdout: buf,
            cut: true,
        });
    }
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ok(Output {
                    ok: status.success(),
                    code: status.code(),
                    stdout: buf,
                    cut: false,
                });
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            Ok(None) => {
                stop(&mut child);
                return Err(DiffError::TooSlow);
            }
            Err(_) => {
                stop(&mut child);
                return Err(DiffError::Failed);
            }
        }
    }
}

/// The trimmed first line of a call that succeeded, else `None`.
fn line(dir: &Path, args: &[&str]) -> Result<Option<String>, DiffError> {
    let out = run(dir, &[], args, OUTPUT_MAX)?;
    let text = String::from_utf8_lossy(&out.stdout);
    let first = text.lines().next().unwrap_or("").to_owned();
    Ok((out.ok && !first.is_empty()).then_some(first))
}

/// The root of the work tree `folder` is in. `None` for a folder that is in none, and for one whose repo's config puts
/// the work tree inside another repo's: asked there, git would answer with a different root and config.
pub fn toplevel(folder: &Path) -> Option<PathBuf> {
    let top = shown_toplevel(folder)?;
    (shown_toplevel(&top)? == top).then_some(top)
}

fn shown_toplevel(folder: &Path) -> Option<PathBuf> {
    let out = run(folder, &[], &["rev-parse", "--show-toplevel"], OUTPUT_MAX).ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    let top = text.strip_suffix('\n').unwrap_or(&text);
    (out.ok && !top.is_empty()).then(|| PathBuf::from(top))
}

fn work_tree(dir: &Path) -> Result<(), DiffError> {
    toplevel(dir).map(|_| ()).ok_or(DiffError::NotARepo)
}

/// A short stable name for a work tree's folder (FNV-1a 64): the page picks a tree by it, never by a path.
fn tree_id(path: &Path) -> String {
    let hash = path
        .as_os_str()
        .as_encoded_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
        });
    format!("{hash:016x}")
}

/// A name as it is shown: one line, every hidden or control character (line breaks and tabs too) as an escape.
fn visible(text: &str) -> String {
    clean_block(text, usize::MAX)
        .0
        .replace('\n', "\\u{a}")
        .replace('\t', "\\u{9}")
}

/// The work trees of the repo `dir` is in, the main one first; a bare, prunable or missing one is left out. At most
/// [`WORKTREES_MAX`]; the flag says more were left out.
pub fn worktrees(dir: &Path) -> Result<(Vec<Worktree>, bool), DiffError> {
    work_tree(dir)?;
    let out = run(
        dir,
        &[],
        &["worktree", "list", "--porcelain", "-z"],
        OUTPUT_MAX,
    )?;
    if !out.ok {
        return Err(DiffError::Failed);
    }
    let mut trees = Vec::new();
    let mut cut = out.cut;
    let mut fields: Vec<&[u8]> = Vec::new();
    let mut first = true;
    for field in out.stdout.split(|&b| b == 0) {
        if !field.is_empty() {
            fields.push(field);
            continue;
        }
        if fields.is_empty() {
            continue;
        }
        let record = std::mem::take(&mut fields);
        let main = std::mem::replace(&mut first, false);
        let value = |key: &str| {
            record
                .iter()
                .find_map(|f| f.strip_prefix(key.as_bytes()))
                .and_then(|v| std::str::from_utf8(v).ok())
        };
        let gone = record
            .iter()
            .any(|f| *f == b"bare" || f.starts_with(b"prunable"));
        let Some(path) = value("worktree ").map(PathBuf::from) else {
            continue;
        };
        if gone || !path.is_dir() {
            continue;
        }
        if trees.len() >= WORKTREES_MAX {
            cut = true;
            continue;
        }
        trees.push(Worktree {
            id: tree_id(&path),
            folder: visible(&path.file_name().unwrap_or_default().to_string_lossy()),
            branch: value("branch ").map(|b| visible(b.strip_prefix("refs/heads/").unwrap_or(b))),
            head: value("HEAD ").map(str::to_owned),
            main,
            path,
        });
    }
    Ok((trees, cut))
}

/// The commit id `rev` names. `rev` is the page's text: it goes after `--end-of-options` and only its id goes on.
fn resolve(dir: &Path, rev: &str) -> Result<String, DiffError> {
    if rev.is_empty() || rev.chars().count() > REF_MAX_CHARS || rev.chars().any(char::is_control) {
        return Err(DiffError::UnknownRef);
    }
    let spec = format!("{rev}^{{commit}}");
    line(
        dir,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            &spec,
        ],
    )?
    .filter(|id| id.len() >= 40 && id.bytes().all(|b| b.is_ascii_hexdigit()))
    .ok_or(DiffError::UnknownRef)
}

/// The full ref of `main`, else `master`, else the remote's default branch (as last fetched: nothing is fetched here).
/// Full, so a tag of the same name cannot take its place.
fn default_base(dir: &Path) -> Result<Option<String>, DiffError> {
    for name in ["main", "master"] {
        let full = format!("refs/heads/{name}");
        if resolve(dir, &full).is_ok() {
            return Ok(Some(full));
        }
    }
    line(
        dir,
        &["symbolic-ref", "--quiet", "refs/remotes/origin/HEAD"],
    )
}

/// `{branches, truncated, base, base_shown, commits}`: local and remote branches and tags (500 at most, `truncated` says
/// more were left out), the default base of a branch compare (`base` is the full ref, to be sent back as it is;
/// `base_shown` is its short name to show), and the newest 100 commits of `HEAD`.
pub fn overview(dir: &Path) -> Result<Value, DiffError> {
    work_tree(dir)?;
    let refs = run(
        dir,
        &[],
        &[
            "for-each-ref",
            "--format=%(refname)",
            "refs/heads",
            "refs/remotes",
            "refs/tags",
        ],
        OUTPUT_MAX,
    )?;
    if !refs.ok {
        return Err(DiffError::Failed);
    }
    let mut branches: Vec<Value> = String::from_utf8_lossy(&refs.stdout)
        .lines()
        .filter(|name| !(name.starts_with("refs/remotes/") && name.ends_with("/HEAD")))
        .filter_map(|full| {
            let (kind, name) = [
                ("branch", "refs/heads/"),
                ("remote", "refs/remotes/"),
                ("tag", "refs/tags/"),
            ]
            .iter()
            .find_map(|(kind, prefix)| Some((*kind, full.strip_prefix(prefix)?)))?;
            Some(json!({"name": name, "shown": visible(name), "kind": kind}))
        })
        .take(REFS_MAX + 1)
        .collect();
    let truncated = refs.cut || branches.len() > REFS_MAX;
    branches.truncate(REFS_MAX);
    let log = run(
        dir,
        &[],
        &[
            "log",
            "-z",
            "--no-show-signature",
            "--format=%H%x1f%ct%x1f%an%x1f%s",
            "-n",
            COMMITS_MAX,
            "--end-of-options",
            "HEAD",
        ],
        OUTPUT_MAX,
    )?;
    // a repo with no commit yet has no `HEAD`: no commits
    let commits: Vec<Value> = log
        .stdout
        .split(|&b| b == 0)
        .filter(|_| log.ok)
        .filter_map(|record| {
            let text = String::from_utf8_lossy(record);
            let mut parts = text.splitn(4, '\u{1f}');
            let (id, time, author, subject) =
                (parts.next()?, parts.next()?, parts.next()?, parts.next()?);
            Some(
                json!({"id": id, "time": time.parse::<i64>().ok()?, "author": visible(author),
                        "subject": visible(subject)}),
            )
        })
        .collect();
    let base = default_base(dir)?;
    let base_shown = base.as_deref().map(|full| {
        let short = ["refs/heads/", "refs/remotes/"]
            .iter()
            .find_map(|prefix| full.strip_prefix(prefix));
        visible(short.unwrap_or(full))
    });
    Ok(
        json!({"branches": branches, "truncated": truncated, "base": base, "base_shown": base_shown,
              "commits": commits}),
    )
}

/// The `-c` pairs that switch every configured filter off, and whether one has a name that cannot be overridden (a `=`
/// or a line break would end the key early).
fn filters(dir: &Path) -> Result<(Vec<String>, bool), DiffError> {
    let out = run(
        dir,
        &[],
        &[
            "config",
            "--null",
            "--name-only",
            "--get-regexp",
            "^filter\\.",
        ],
        OUTPUT_MAX,
    )?;
    // 1: no key matches
    if out.cut || !(out.ok || out.code == Some(1)) {
        return Err(DiffError::Failed);
    }
    let mut names: Vec<&[u8]> = out
        .stdout
        .split(|&b| b == 0)
        .filter_map(|key| {
            let rest = key.strip_prefix(b"filter.")?;
            Some(&rest[..rest.iter().rposition(|&b| b == b'.')?])
        })
        .collect();
    names.sort_unstable();
    names.dedup();
    let mut pairs = Vec::new();
    let mut stuck = false;
    for name in names {
        match std::str::from_utf8(name) {
            Ok(name) if !name.contains(['=', '\n']) => {
                for tail in ["clean=", "smudge=", "process=", "required=false"] {
                    pairs.push(format!("filter.{name}.{tail}"));
                }
            }
            _ => stuck = true,
        }
    }
    Ok((pairs, stuck))
}

/// The commit ids and `git diff` arguments of one part of a compare.
struct Part {
    area: &'static str,
    revs: Vec<String>,
}

struct Plan {
    /// The root of the work tree: every call of the plan runs here, where its filters were read.
    top: PathBuf,
    parts: Vec<Part>,
    base: Option<String>,
    head: Option<String>,
    config: Vec<String>,
}

fn plan(dir: &Path, compare: &Compare) -> Result<Plan, DiffError> {
    let top = toplevel(dir).ok_or(DiffError::NotARepo)?;
    let dir = top.as_path();
    let (config, stuck) = filters(dir)?;
    let committed = |base: String, head: String, config| Plan {
        top: top.clone(),
        parts: vec![Part {
            area: "committed",
            revs: vec![base.clone(), head.clone()],
        }],
        base: Some(base),
        head: Some(head),
        config,
    };
    Ok(match compare {
        Compare::Uncommitted => {
            // the one compare that reads the work tree, where a filter would run
            if stuck {
                return Err(DiffError::Filter);
            }
            Plan {
                top: top.clone(),
                parts: vec![
                    Part {
                        area: "staged",
                        revs: vec!["--cached".into()],
                    },
                    Part {
                        area: "unstaged",
                        revs: Vec::new(),
                    },
                ],
                base: None,
                head: None,
                config,
            }
        }
        Compare::Branch { base } => {
            let base = match base {
                Some(base) => base.clone(),
                None => default_base(dir)?.ok_or(DiffError::NoBase)?,
            };
            let base = resolve(dir, &base)?;
            let head = resolve(dir, "HEAD")?;
            let fork =
                line(dir, &["merge-base", &base, &head])?.ok_or(DiffError::NoCommonHistory)?;
            committed(fork, head, config)
        }
        Compare::Commit { rev } => {
            let head = resolve(dir, rev)?;
            let base = match resolve(dir, &format!("{head}^1")) {
                Ok(parent) => parent,
                // the root commit: against the empty tree (its id depends on the repo's hash)
                Err(DiffError::UnknownRef) => {
                    line(dir, &["hash-object", "-t", "tree", "/dev/null"])?
                        .ok_or(DiffError::Failed)?
                }
                Err(other) => return Err(other),
            };
            committed(base, head, config)
        }
        Compare::Range { base, head } => {
            committed(resolve(dir, base)?, resolve(dir, head)?, config)
        }
    })
}

const DIFF_ARGS: [&str; 9] = [
    "diff",
    "--no-ext-diff",
    "--no-textconv",
    "--no-color",
    "--ignore-submodules=dirty",
    "--submodule=short",
    "--find-renames",
    "--src-prefix=a/",
    "--dst-prefix=b/",
];

#[derive(Debug)]
struct Entry {
    area: &'static str,
    status: &'static str,
    path: Vec<u8>,
    old_path: Option<Vec<u8>>,
    added: Option<u64>,
    removed: Option<u64>,
    binary: bool,
    /// Either side is a submodule.
    submodule: bool,
    /// An unstaged binary file of unchanged mode: it differs only if its content's id is not this one, the index's.
    index_id: Option<String>,
}

/// The entries of `git diff --raw --numstat -z`: first a `:modes ids status` field and its path (two for a rename or a
/// copy) per file, then `added<TAB>removed<TAB>path` (`-` for binary; an empty path and two more fields for a rename).
/// `cut`: the answer is only its beginning, so its last, unfinished field is dropped and a file with no counts stays.
fn entries(area: &'static str, out: &[u8], cut: bool) -> Vec<Entry> {
    let out = match out.iter().rposition(|&b| b == 0) {
        Some(end) if cut => &out[..=end],
        _ if cut => &[],
        _ => out,
    };
    let mut found: Vec<Entry> = Vec::new();
    // a path's place in `found`
    let mut places: HashMap<Vec<u8>, usize> = HashMap::new();
    // by place in `found`: the modes differ (or a side is missing), and a numstat field came
    let mut seen_as: Vec<(bool, bool)> = Vec::new();
    let mut fields = out.split(|&b| b == 0);
    while let Some(field) = fields.next() {
        if let Some(raw) = field.strip_prefix(b":") {
            let text = String::from_utf8_lossy(raw);
            let words: Vec<&str> = text.split(' ').collect();
            let letter = words.last().and_then(|w| w.chars().next()).unwrap_or('M');
            let status = match letter {
                'A' => "added",
                'D' => "deleted",
                'R' => "renamed",
                'C' => "copied",
                'T' => "type",
                'U' => "conflict",
                _ => "modified",
            };
            // a path is never empty: after a cut, the end of the answer stands where one should
            let Some(first) = fields.next().filter(|f| !f.is_empty()) else {
                break;
            };
            let (old_path, path) = if matches!(letter, 'R' | 'C') {
                let Some(second) = fields.next().filter(|f| !f.is_empty()) else {
                    break;
                };
                (Some(first.to_vec()), second.to_vec())
            } else {
                (None, first.to_vec())
            };
            // a conflict comes as `U` and again as `M`
            if let Some(&at) = places.get(&path) {
                if status == "conflict" {
                    found[at].status = "conflict";
                }
                continue;
            }
            places.insert(path.clone(), found.len());
            // `old mode, new mode, old id, new id, status`
            let mode_changed =
                words.first().map(|w| w.trim_start_matches(':')) != words.get(1).copied();
            seen_as.push((mode_changed, false));
            found.push(Entry {
                area,
                status,
                path,
                old_path,
                added: None,
                removed: None,
                binary: false,
                submodule: words.iter().take(2).any(|&mode| mode == "160000"),
                index_id: words.get(2).map(|&id| id.to_owned()),
            });
        } else {
            let mut parts = field.splitn(3, |&b| b == b'\t');
            let (Some(added), Some(removed), Some(path)) =
                (parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            let path = if path.is_empty() {
                let (_, Some(new)) = (fields.next(), fields.next()) else {
                    break;
                };
                new
            } else {
                path
            };
            let number = |b: &[u8]| std::str::from_utf8(b).ok()?.parse::<u64>().ok();
            if let Some(&at) = places.get(path) {
                seen_as[at].1 = true;
                let entry = &mut found[at];
                entry.binary = added == b"-";
                entry.added = number(added);
                entry.removed = number(removed);
            }
        }
    }
    // with the index not refreshed, a file whose time changed but not its content is listed with nothing to show
    if area == "unstaged" {
        let mut seen = seen_as.into_iter();
        found.retain_mut(|e| {
            let (mode_changed, counted) = seen.next().unwrap_or((true, true));
            let unproven = !mode_changed && e.status == "modified";
            if !(unproven && e.binary) {
                e.index_id = None;
            }
            let empty = if counted {
                !e.binary && (e.added, e.removed) == (Some(0), Some(0))
            } else {
                !cut
            };
            !(unproven && empty)
        });
    } else {
        found.iter_mut().for_each(|e| e.index_id = None);
    }
    // a conflict is shown once, from the work tree (the side with the markers)
    if area == "staged" {
        found.retain(|e| e.status != "conflict");
    }
    found
}

/// Drops each unstaged binary file whose content is what the index holds (git says nothing more for a binary file whose
/// time alone changed); with `only`, only that path is asked about. `hash-object` reads the file as it is, no filter, and
/// writes nothing. A file it cannot answer for, or not within [`GIT_TIMEOUT`] for them all, stays listed.
fn drop_unchanged_binaries(plan: &Plan, only: Option<&[u8]>, all: &mut Vec<Entry>) {
    let asked: Vec<&str> = all
        .iter()
        .filter(|e| e.index_id.is_some() && only.is_none_or(|path| e.path == path))
        .filter_map(|e| std::str::from_utf8(&e.path).ok())
        .collect();
    let mut same: HashMap<Vec<u8>, String> = HashMap::new();
    let deadline = Instant::now() + GIT_TIMEOUT;
    for paths in asked.chunks(HASH_PATHS) {
        if Instant::now() > deadline {
            break;
        }
        let mut args = vec!["hash-object", "--no-filters", "--"];
        args.extend(paths);
        let Ok(out) = run(&plan.top, &plan.config, &args, OUTPUT_MAX) else {
            break; // git is too slow here: the rest stay listed
        };
        if !out.ok || out.cut {
            continue;
        }
        let ids = String::from_utf8_lossy(&out.stdout);
        same.extend(
            paths
                .iter()
                .zip(ids.lines())
                .map(|(path, id)| (path.as_bytes().to_vec(), id.to_owned())),
        );
    }
    all.retain(|e| e.index_id.is_none() || same.get(&e.path) != e.index_id.as_ref());
}

/// The listed files of the plan's compare, and whether git's answer was cut. `only` is the one file the caller opens.
fn list(plan: &Plan, only: Option<&[u8]>) -> Result<(Vec<Entry>, bool), DiffError> {
    let dir = plan.top.as_path();
    let mut all = Vec::new();
    let mut cut = false;
    for part in &plan.parts {
        let mut args: Vec<&str> = DIFF_ARGS.to_vec();
        args.extend(["--raw", "--no-abbrev", "--numstat", "-z"]);
        args.extend(part.revs.iter().map(String::as_str));
        args.push("--");
        let out = run(dir, &plan.config, &args, LIST_MAX)?;
        if !out.ok {
            return Err(DiffError::Failed);
        }
        cut |= out.cut;
        all.extend(entries(part.area, &out.stdout, out.cut));
    }
    drop_unchanged_binaries(plan, only, &mut all);
    if plan.parts.iter().any(|p| p.area == "unstaged") {
        let out = run(
            dir,
            &plan.config,
            &["ls-files", "--others", "--exclude-standard", "-z"],
            LIST_MAX,
        )?;
        if !out.ok {
            return Err(DiffError::Failed);
        }
        cut |= out.cut;
        let mut names: Vec<&[u8]> = out.stdout.split(|&b| b == 0).collect();
        // the last field of a cut answer may be half a name; of a whole one it is empty
        names.pop();
        all.extend(names.into_iter().map(|name| Entry {
            area: "untracked",
            status: "untracked",
            path: name.to_vec(),
            old_path: None,
            added: None,
            removed: None,
            binary: false,
            submodule: false,
            index_id: None,
        }));
    }
    Ok((all, cut))
}

/// `{files, total, truncated, base, head, filters_off}`. A file is `{area, status, path, old_path, shown, shown_old,
/// added, removed, binary, openable}`; `path` is what [`file`] takes, `shown` has hidden characters as escapes, and a
/// path that is not UTF-8 cannot be opened.
pub fn files(dir: &Path, compare: &Compare) -> Result<Value, DiffError> {
    let plan = plan(dir, compare)?;
    let (all, cut) = list(&plan, None)?;
    let total = all.len();
    let mut budget = COUNT_BUDGET;
    let rows: Vec<Value> = all
        .into_iter()
        .take(FILES_MAX)
        .map(|mut e| {
            let openable = std::str::from_utf8(&e.path).is_ok();
            let path = String::from_utf8_lossy(&e.path).into_owned();
            if e.area == "untracked" && openable {
                (e.binary, e.added) = peek(&plan.top, &path, &mut budget);
                e.removed = e.added.map(|_| 0);
            }
            let old = e
                .old_path
                .map(|p| String::from_utf8_lossy(&p).into_owned());
            json!({"area": e.area, "status": e.status, "shown": visible(&path),
                   "shown_old": old.as_deref().map(visible), "path": path, "old_path": old,
                   "added": e.added, "removed": e.removed, "binary": e.binary, "openable": openable})
        })
        .collect();
    // filters apply only where the work tree is read
    let reads_tree = plan.parts.iter().any(|p| p.area == "unstaged");
    Ok(
        json!({"files": rows, "total": total, "truncated": cut || total > FILES_MAX,
              "base": plan.base, "head": plan.head,
              "filters_off": reads_tree && !plan.config.is_empty()}),
    )
}

/// The patch without the carriage return of each Windows line ending, when every ended line of its hunks has one. Where
/// only some do, the endings are what differs (or the file mixes them): all are kept, and shown as escapes.
fn without_windows_ends(text: &str) -> Cow<'_, str> {
    let mut plain = String::with_capacity(text.len());
    let (mut in_hunk, mut any) = (false, false);
    let mut lines = text.split_inclusive('\n').peekable();
    while let Some(line) = lines.next() {
        // a file's last line may have no ending: git says so on the next line
        let unended = lines.peek().is_some_and(|next| next.starts_with('\\'));
        if line.starts_with("@@") {
            in_hunk = true;
        } else if line.starts_with("diff ") {
            in_hunk = false;
        } else if in_hunk && !unended && !line.starts_with('\\') {
            if let Some(body) = line.strip_suffix("\r\n") {
                plain.push_str(body);
                plain.push('\n');
                any = true;
                continue;
            }
            if line.ends_with('\n') {
                return Cow::Borrowed(text);
            }
        }
        plain.push_str(line);
    }
    if any {
        Cow::Owned(plain)
    } else {
        Cow::Borrowed(text)
    }
}

fn patch_answer(bytes: &[u8], cut: bool, binary: bool, note: Option<&str>) -> Value {
    let text = String::from_utf8_lossy(bytes);
    let text = without_windows_ends(&text);
    // hidden characters become longer escapes: cap what is sent, at a line end where one fits
    let (mut shown, whole) = clean_block(&text, PATCH_MAX_BYTES);
    let mut cut = cut || !whole;
    if let Some((end, _)) = shown.match_indices('\n').nth(PATCH_MAX_LINES - 1)
        && end + 1 < shown.len()
    {
        shown.truncate(end + 1);
        cut = true;
    }
    if shown.len() > PATCH_MAX_BYTES {
        let mut end = PATCH_MAX_BYTES;
        while !shown.is_char_boundary(end) {
            end -= 1;
        }
        shown.truncate(end);
        cut = true;
    }
    json!({"patch": shown, "cut": cut, "binary": binary, "note": note})
}

/// The work tree's file `path` (a listed untracked name), every folder and the file opened with no symlink followed.
fn open_inside(dir: &Path, path: &str) -> Option<File> {
    let mut parts: Vec<&str> = path.split('/').collect();
    let name = parts.pop()?;
    if parts
        .iter()
        .chain([&name])
        .any(|p| matches!(*p, "" | "." | ".."))
    {
        return None;
    }
    let mut folder = sys::open_root(dir.to_str()?).ok()?;
    for part in parts {
        folder = sys::open_dir_at(&folder, part).ok()?;
    }
    sys::open_file_at(&folder, name).ok()
}

/// An untracked file for the list: whether it is binary, and its line count when it is text that fits both caps.
fn peek(dir: &Path, path: &str, budget: &mut usize) -> (bool, Option<u64>) {
    let unknown = (false, None);
    let Some(file) = open_inside(dir, path) else {
        return unknown;
    };
    let Ok(meta) = file.metadata() else {
        return unknown;
    };
    if !meta.is_file() {
        return unknown;
    }
    sniff(file, meta.len(), budget)
}

/// [`peek`] over an open file of `len` bytes. Only the start of a binary file is read; the bytes of a counted one come
/// off the budget.
fn sniff(mut file: impl Read, len: u64, budget: &mut usize) -> (bool, Option<u64>) {
    let unknown = (false, None);
    let whole = len <= PATCH_MAX_BYTES.min(*budget) as u64;
    let mut data = Vec::new();
    let probe = file.by_ref().take(BINARY_PROBE as u64);
    if { probe }.read_to_end(&mut data).is_err() {
        return unknown;
    }
    if data.contains(&0) {
        return (true, None);
    }
    let rest = len.saturating_sub(data.len() as u64);
    if !whole || file.take(rest).read_to_end(&mut data).is_err() {
        return unknown;
    }
    *budget = budget.saturating_sub(data.len());
    let breaks = data.iter().filter(|&&b| b == b'\n').count();
    let open_end = data.last().is_some_and(|&b| b != b'\n');
    (false, Some((breaks + usize::from(open_end)) as u64))
}

/// A new file's patch, made here: `git diff` has none for a file it does not track.
fn untracked(dir: &Path, path: &str) -> Result<Value, DiffError> {
    let full = dir.join(path);
    if std::fs::symlink_metadata(&full).is_ok_and(|m| m.file_type().is_symlink()) {
        return Ok(patch_answer(
            b"",
            false,
            false,
            Some("a symbolic link: its content is not read"),
        ));
    }
    let file = open_inside(dir, path).ok_or(DiffError::NotListed)?;
    if !file.metadata().is_ok_and(|m| m.is_file()) {
        return Err(DiffError::NotListed);
    }
    let mut data = Vec::new();
    file.take(PATCH_MAX_BYTES as u64 + 1)
        .read_to_end(&mut data)
        .map_err(|_| DiffError::Failed)?;
    if data[..data.len().min(BINARY_PROBE)].contains(&0) {
        return Ok(patch_answer(b"", false, true, None));
    }
    let cut = data.len() > PATCH_MAX_BYTES;
    data.truncate(PATCH_MAX_BYTES);
    if data.is_empty() {
        return Ok(patch_answer(b"", false, false, Some("an empty file")));
    }
    let body = data.strip_suffix(b"\n").unwrap_or(&data);
    let lines: Vec<&[u8]> = body.split(|&b| b == b'\n').collect();
    let mut patch = format!("@@ -0,0 +1,{} @@\n", lines.len()).into_bytes();
    for line in lines {
        patch.push(b'+');
        patch.extend_from_slice(line);
        patch.push(b'\n');
    }
    if !cut && !data.ends_with(b"\n") {
        patch.extend_from_slice(b"\\ No newline at end of file\n");
    }
    Ok(patch_answer(&patch, cut, false, None))
}

/// The pathspecs of the patch of the listed `entry`, whose path is `path`.
fn pathspecs(entry: &Entry, path: &str, all: &[Entry]) -> Result<Vec<String>, DiffError> {
    let mut specs = vec![format!(":(literal){path}")];
    if let Some(old) = &entry.old_path {
        let old = std::str::from_utf8(old).map_err(|_| DiffError::NotListed)?;
        specs.push(format!(":(literal){old}"));
    }
    // a literal path also matches what is under a folder of that name: other listed files there are left out
    let own: Vec<&[u8]> = [Some(&entry.path), entry.old_path.as_ref()]
        .into_iter()
        .flatten()
        .map(Vec::as_slice)
        .collect();
    let others: Vec<&[u8]> = all
        .iter()
        .filter(|o| o.area == entry.area)
        .flat_map(|o| [Some(&o.path), o.old_path.as_ref()])
        .flatten()
        .map(Vec::as_slice)
        .filter(|name| !own.contains(name))
        .collect();
    for mine in &own {
        let folder = [mine, b"/".as_slice()].concat();
        let under: Vec<&&[u8]> = others.iter().filter(|o| o.starts_with(&folder)).collect();
        if under.is_empty() {
            continue;
        }
        let spec = |name: &[u8]| {
            let name = std::str::from_utf8(name).map_err(|_| DiffError::NotListed)?;
            Ok(format!(":(exclude,literal){name}"))
        };
        if entry.submodule || own.iter().any(|o| o.starts_with(&folder)) {
            // leaving the folder out would also leave out a submodule of that name, or the file's own other name in it:
            // leave out the others one by one
            for name in under {
                specs.push(spec(name)?);
            }
        } else {
            specs.push(spec(&folder)?);
        }
    }
    Ok(specs)
}

/// `{patch, cut, binary, note}` of one file the list of this compare holds in `area`.
pub fn file(dir: &Path, compare: &Compare, area: &str, path: &str) -> Result<Value, DiffError> {
    let plan = plan(dir, compare)?;
    let (all, _) = list(&plan, Some(path.as_bytes()))?;
    let entry = all
        .iter()
        .find(|e| e.area == area && e.path == path.as_bytes())
        .ok_or(DiffError::NotListed)?;
    let Some(part) = plan.parts.iter().find(|p| p.area == entry.area) else {
        // git names a folder with its own repository by the folder, with a slash
        if path.ends_with('/') {
            let note = "a folder with its own git repository: its content is not read";
            return Ok(patch_answer(b"", false, false, Some(note)));
        }
        return untracked(&plan.top, path);
    };
    let specs = pathspecs(entry, path, &all)?;
    let mut args: Vec<&str> = DIFF_ARGS.to_vec();
    args.push("--patch");
    args.extend(part.revs.iter().map(String::as_str));
    args.push("--");
    args.extend(specs.iter().map(String::as_str));
    let out = run(&plan.top, &plan.config, &args, PATCH_MAX_BYTES)?;
    if !out.ok {
        return Err(DiffError::Failed);
    }
    Ok(patch_answer(&out.stdout, out.cut, entry.binary, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cut_list_keeps_the_files_whose_counts_did_not_come_and_drops_half_a_name() {
        let raw = |path: &str| {
            format!(
                ":100644 100644 {} {} M\0{path}\0",
                "a".repeat(40),
                "0".repeat(40)
            )
        };
        let out = raw("one") + &raw("two") + &raw("three-cut-in-the-mid");
        let out = &out.as_bytes()[..out.len() - 4];
        let names: Vec<Vec<u8>> = entries("unstaged", out, true)
            .into_iter()
            .map(|e| e.path)
            .collect();
        assert_eq!(names, [b"one".to_vec(), b"two".to_vec()]);
        // a whole answer: a listed file with no counts only had its time changed
        let whole = raw("one");
        assert!(entries("unstaged", whole.as_bytes(), false).is_empty());
    }

    /// Counts the bytes taken from it.
    struct Counted<'a>(&'a [u8], &'a mut usize);

    impl Read for Counted<'_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let n = self.0.read(buf)?;
            *self.1 += n;
            Ok(n)
        }
    }

    #[test]
    fn only_the_start_of_a_binary_file_is_read_and_text_is_charged_to_the_budget() {
        let binary = vec![0u8; PATCH_MAX_BYTES];
        let (mut read, mut budget) = (0, COUNT_BUDGET);
        let answer = sniff(
            Counted(&binary, &mut read),
            binary.len() as u64,
            &mut budget,
        );
        assert_eq!(answer, (true, None));
        assert_eq!((read, budget), (BINARY_PROBE, COUNT_BUDGET));

        let text = b"line\n".repeat(4000);
        let (mut read, mut budget) = (0, COUNT_BUDGET);
        let answer = sniff(Counted(&text, &mut read), text.len() as u64, &mut budget);
        assert_eq!(answer, (false, Some(4000)));
        assert_eq!((read, budget), (text.len(), COUNT_BUDGET - text.len()));
        // too big for what is left: only the start is read
        let (mut read, mut budget) = (0, 100);
        let answer = sniff(Counted(&text, &mut read), text.len() as u64, &mut budget);
        assert_eq!(answer, (false, None));
        assert_eq!((read, budget), (BINARY_PROBE, 100));
    }

    fn listed(path: &str, old_path: Option<&str>) -> Entry {
        Entry {
            area: "staged",
            status: "modified",
            path: path.into(),
            old_path: old_path.map(Into::into),
            added: None,
            removed: None,
            binary: false,
            submodule: false,
            index_id: None,
        }
    }

    #[test]
    fn a_folder_in_a_file_s_place_is_left_out_by_one_pathspec() {
        let all = [
            listed("a", None),
            listed("a/one", None),
            listed("a/two", None),
            listed("ab", None),
        ];
        assert_eq!(
            pathspecs(&all[0], "a", &all).unwrap(),
            [":(literal)a", ":(exclude,literal)a/"]
        );
        assert_eq!(pathspecs(&all[3], "ab", &all).unwrap(), [":(literal)ab"]);
        // a file moved into a folder of its old name: that folder cannot be left out whole
        let moved = [listed("a/b", Some("a")), listed("a/c", None)];
        assert_eq!(
            pathspecs(&moved[0], "a/b", &moved).unwrap(),
            [":(literal)a/b", ":(literal)a", ":(exclude,literal)a/c"]
        );
    }
}
