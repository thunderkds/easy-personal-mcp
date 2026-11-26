//! The read-only git diff of a repo (T136, FR-139, FR-140, DDR-0010): worktrees, refs, changed files and one file's patch.
//! **The seventh exception of NFR-002**: what these functions return goes to the dashboard page and nowhere else; nothing
//! here is stored, cached or logged, and no [`DiffError`] holds any of it.
//!
//! A repo's own config is not trusted to start a program (a plain `git diff` runs its clean filters and its external
//! diff): every call switches off fsmonitor, hooks and signature checks, every diff switches off external diff and
//! textconv and overrides each configured filter by name, submodule work trees are not entered, no call writes the index
//! (`diff.autoRefreshIndex=false`, no optional locks) and none may fetch. A ref reaches `git diff` only as the commit id
//! `rev-parse` gave for it; a path only as a `:(literal)` pathspec of a file the list answer holds.

use crate::live::clean_block;
use crate::report_files::sys;
use serde_json::{Value, json};
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
/// The most bytes read of a git answer that is not a patch.
const OUTPUT_MAX: usize = 4 * 1024 * 1024;
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

/// The root of the work tree `folder` is in; `None` for a folder that is in none.
pub fn toplevel(folder: &Path) -> Option<PathBuf> {
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

fn visible(text: &str) -> String {
    clean_block(text, usize::MAX).0
}

/// The work trees of the repo `dir` is in, the main one first; a bare, prunable or missing one is left out.
pub fn worktrees(dir: &Path) -> Result<Vec<Worktree>, DiffError> {
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
        if gone || !path.is_dir() || trees.len() >= WORKTREES_MAX {
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
    Ok(trees)
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

/// `main`, else `master`, else the remote's default branch (as last fetched: nothing is fetched here).
fn default_base(dir: &Path) -> Result<Option<String>, DiffError> {
    for name in ["main", "master"] {
        if resolve(dir, &format!("refs/heads/{name}")).is_ok() {
            return Ok(Some(name.to_owned()));
        }
    }
    line(
        dir,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    )
}

/// `{branches, base, commits}`: local and remote branches and tags (500 at most), the default base of a branch compare,
/// and the newest 100 commits of `HEAD`.
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
    let branches: Vec<Value> = String::from_utf8_lossy(&refs.stdout)
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
        .take(REFS_MAX)
        .collect();
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
    Ok(json!({"branches": branches, "base": default_base(dir)?, "commits": commits}))
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
    parts: Vec<Part>,
    base: Option<String>,
    head: Option<String>,
    config: Vec<String>,
}

fn plan(dir: &Path, compare: &Compare) -> Result<Plan, DiffError> {
    work_tree(dir)?;
    let (config, stuck) = filters(dir)?;
    let committed = |base: String, head: String, config| Plan {
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
}

/// The entries of `git diff --raw --numstat -z`: first a `:modes ids status` field and its path (two for a rename or a
/// copy) per file, then `added<TAB>removed<TAB>path` (`-` for binary; an empty path and two more fields for a rename).
fn entries(area: &'static str, out: &[u8]) -> Vec<Entry> {
    let mut found: Vec<Entry> = Vec::new();
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
            let Some(first) = fields.next() else { break };
            let (old_path, path) = if matches!(letter, 'R' | 'C') {
                let Some(second) = fields.next() else { break };
                (Some(first.to_vec()), second.to_vec())
            } else {
                (None, first.to_vec())
            };
            // a conflict comes as `U` and again as `M`
            if let Some(seen) = found.iter_mut().rev().find(|e| e.path == path) {
                if status == "conflict" {
                    seen.status = "conflict";
                }
                continue;
            }
            seen_as.push((words.first() != words.get(1), false));
            found.push(Entry {
                area,
                status,
                path,
                old_path,
                added: None,
                removed: None,
                binary: false,
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
            if let Some(at) = found.iter().position(|e| e.path == path) {
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
        found.retain(|e| {
            let (mode_changed, counted) = seen.next().unwrap_or((true, true));
            let empty = !counted || (!e.binary && (e.added, e.removed) == (Some(0), Some(0)));
            mode_changed || e.status != "modified" || !empty
        });
    }
    // a conflict is shown once, from the work tree (the side with the markers)
    if area == "staged" {
        found.retain(|e| e.status != "conflict");
    }
    found
}

fn list(dir: &Path, plan: &Plan) -> Result<(Vec<Entry>, bool), DiffError> {
    let mut all = Vec::new();
    let mut cut = false;
    for part in &plan.parts {
        let mut args: Vec<&str> = DIFF_ARGS.to_vec();
        args.extend(["--raw", "--numstat", "-z"]);
        args.extend(part.revs.iter().map(String::as_str));
        args.push("--");
        let out = run(dir, &plan.config, &args, OUTPUT_MAX)?;
        if !out.ok {
            return Err(DiffError::Failed);
        }
        cut |= out.cut;
        all.extend(entries(part.area, &out.stdout));
    }
    if plan.parts.iter().any(|p| p.area == "unstaged") {
        let out = run(
            dir,
            &plan.config,
            &["ls-files", "--others", "--exclude-standard", "-z"],
            OUTPUT_MAX,
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
        }));
    }
    Ok((all, cut))
}

/// `{files, total, truncated, base, head, filters_off}`. A file is `{area, status, path, old_path, shown, shown_old,
/// added, removed, binary, openable}`; `path` is what [`file`] takes, `shown` has hidden characters as escapes, and a
/// path that is not UTF-8 cannot be opened.
pub fn files(dir: &Path, compare: &Compare) -> Result<Value, DiffError> {
    let plan = plan(dir, compare)?;
    let (all, cut) = list(dir, &plan)?;
    let total = all.len();
    let rows: Vec<Value> = all
        .into_iter()
        .take(FILES_MAX)
        .map(|e| {
            let openable = std::str::from_utf8(&e.path).is_ok();
            let path = String::from_utf8_lossy(&e.path).into_owned();
            let old = e
                .old_path
                .map(|p| String::from_utf8_lossy(&p).into_owned());
            json!({"area": e.area, "status": e.status, "shown": visible(&path),
                   "shown_old": old.as_deref().map(visible), "path": path, "old_path": old,
                   "added": e.added, "removed": e.removed, "binary": e.binary, "openable": openable})
        })
        .collect();
    Ok(
        json!({"files": rows, "total": total, "truncated": cut || total > FILES_MAX,
              "base": plan.base, "head": plan.head, "filters_off": !plan.config.is_empty()}),
    )
}

fn patch_answer(bytes: &[u8], cut: bool, binary: bool, note: Option<&str>) -> Value {
    let text = String::from_utf8_lossy(bytes);
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

/// `{patch, cut, binary, note}` of one file the list of this compare holds in `area`.
pub fn file(dir: &Path, compare: &Compare, area: &str, path: &str) -> Result<Value, DiffError> {
    let plan = plan(dir, compare)?;
    let (all, _) = list(dir, &plan)?;
    let entry = all
        .iter()
        .find(|e| e.area == area && e.path == path.as_bytes())
        .ok_or(DiffError::NotListed)?;
    let Some(part) = plan.parts.iter().find(|p| p.area == entry.area) else {
        return untracked(dir, path);
    };
    let mut specs = vec![format!(":(literal){path}")];
    if let Some(old) = &entry.old_path {
        let old = std::str::from_utf8(old).map_err(|_| DiffError::NotListed)?;
        specs.push(format!(":(literal){old}"));
    }
    let mut args: Vec<&str> = DIFF_ARGS.to_vec();
    args.push("--patch");
    args.extend(part.revs.iter().map(String::as_str));
    args.push("--");
    args.extend(specs.iter().map(String::as_str));
    let out = run(dir, &plan.config, &args, PATCH_MAX_BYTES)?;
    if !out.ok {
        return Err(DiffError::Failed);
    }
    Ok(patch_answer(&out.stdout, out.cut, entry.binary, None))
}
