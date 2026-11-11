//! The memory graph behind `GET /api/memory-graph` (T043), ported from `src/easy_personal/core/memgraph.py`.
//!
//! Sources: `<repo>/memory/**/*.md` (kit memory) of every stored project, and `<account>/projects/*/memory/**/*.md` (Claude
//! memory) of every confirmed Claude account whose folder exists, plus the easy-memory layer (`easymem.rs`). Only the
//! frontmatter `name` and `description` lines, the relative path and the `[[link]]` targets leave this module; a note body is
//! scanned for link targets and dropped. Read-only: nothing is written anywhere.
//!
//! Reads are bounded and non-following (the T045 rules): the repository folder and the account folder are opened by the
//! owner's own path, every part below them (`memory`, `projects`, each folder and note) with `O_NOFOLLOW` from the held
//! handle, a note is judged on its open handle and read through `take(cap + 1)`. Python follows symlinks that stay inside the
//! memory folder; Rust skips every symlink (decided for you, see `rust/README.md`).
//!
//! Privacy (NFR-002): the note names, descriptions and link targets are the owner's own text, shown as Python shows them
//! (a name or description is cut at 200 characters). No error of this module carries a path or a note text.

use crate::board::{is_space, open_regular, read_capped, strip};
use crate::easymem;
use crate::metrics::{MetricsError, folder, is_dir, repo_of};
use crate::report_files::sys;
use crate::scope::{self, Scope, casefold};
use jiff::tz::TimeZone;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

const MAX_NOTE_BYTES: u64 = 1024 * 1024;
const MAX_FILES: usize = 5000;
const MAX_TEXT: usize = 200;
/// Python's `os.walk` has no depth limit; a folder deeper than this is not entered (decided for you).
const MAX_DEPTH: usize = 64;
const CWD_SCAN_LINES: usize = 200;
const CWD_SCAN_FILES: usize = 3;
/// Python reads a transcript line by line with no limit; a transcript with no complete record in this many bytes is skipped.
const CWD_SCAN_BYTES: usize = 1024 * 1024;
const MEMORY: &str = "memory";

/// What a call needs from the machine: the easy-memory store folder, the zone and "now" for the 30-day window.
pub struct Env {
    pub em_home: PathBuf,
    pub zone: TimeZone,
    pub now_micros: i64,
}

impl Env {
    /// `$EASY_MEMORY_HOME` when set and not empty, else `$HOME/.easy-memory` (as easy-memory itself does).
    pub fn system(now_micros: i64) -> Env {
        let home = match std::env::var_os("EASY_MEMORY_HOME").filter(|v| !v.is_empty()) {
            Some(path) => PathBuf::from(path),
            None => {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".easy-memory")
            }
        };
        Env {
            em_home: home,
            zone: TimeZone::system(),
            now_micros,
        }
    }
}

/// One memory folder: `root` is opened by its own path, each of `parts` below it without following a symlink.
#[derive(Clone)]
struct Folder {
    root: String,
    parts: Vec<String>,
}

/// The directory handle of `root/parts...`, or `None` when a part is missing, is not a folder or is a symlink.
fn open_chain(root: &str, parts: &[String]) -> Option<File> {
    let mut dir = sys::open_root(root).ok()?;
    for part in parts {
        dir = sys::open_dir_at(&dir, part).ok()?;
    }
    Some(dir)
}

struct Source {
    kit: Option<Folder>,
    claude: Vec<(i64, String, Folder)>,
    em: Option<PathBuf>,
}

/// One `.md` file of a project.
struct Entry {
    kind: &'static str,
    account_id: Option<i64>,
    account: Option<String>,
    /// `memory/<path below the folder>`.
    rel: String,
    folder: Folder,
}

/// Claude Code's folder name for a cwd: every non-alphanumeric character becomes `-`.
fn encoded(path: &str) -> String {
    path.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// The names in a folder handle (UTF-8 names only).
pub(crate) fn names(dir: &File) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(sys::handle_path(dir))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

/// `discovery.resolve_project`: a folder missing on disk is its own project, else its git repo, else the folder itself.
fn resolve_project(cwd: &str) -> String {
    if is_dir(cwd) {
        repo_of(cwd).unwrap_or_else(|| cwd.to_string())
    } else {
        cwd.to_string()
    }
}

/// The `cwd` of the first record of a transcript that has one, within its first 200 lines (`None`: none, or the file is
/// not UTF-8). Python's text reader decodes 8 KiB at a time, so a bad byte anywhere in the chunk that holds the first
/// line fails the whole file: the chunks are decoded here the same way.
fn first_cwd(mut file: File) -> Option<String> {
    let (mut pending, mut carry, mut total, mut seen) = (String::new(), Vec::new(), 0, 0);
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf).ok()?;
        total += n;
        let eof = n == 0;
        carry.extend_from_slice(&buf[..n]);
        match std::str::from_utf8(&carry) {
            Ok(text) => {
                pending.push_str(text);
                carry.clear();
            }
            Err(e) if e.error_len().is_none() && !eof => {
                let (done, rest) = carry.split_at(e.valid_up_to());
                pending.push_str(std::str::from_utf8(done).ok()?);
                carry = rest.to_vec();
            }
            Err(_) => return None,
        }
        while let Some(at) = pending.find(['\n', '\r']) {
            let mut next = at + 1;
            if pending.as_bytes()[at] == b'\r' {
                match pending.as_bytes().get(next) {
                    Some(b'\n') => next += 1,
                    None if !eof => break, // the `\n` of a `\r\n` may come with the next chunk
                    _ => {}
                }
            }
            let line: String = pending.drain(..next).collect();
            seen += 1;
            if let Some(cwd) = cwd_of(&line) {
                return Some(cwd);
            }
            if seen >= CWD_SCAN_LINES {
                return None;
            }
        }
        if eof {
            return cwd_of(&pending).filter(|_| seen < CWD_SCAN_LINES);
        }
        if total > CWD_SCAN_BYTES {
            return None;
        }
    }
}

fn cwd_of(line: &str) -> Option<String> {
    let record: Value = serde_json::from_str(line).ok()?;
    let cwd = record.as_object()?.get("cwd")?.as_str()?;
    (!cwd.is_empty()).then(|| resolve_project(cwd))
}

/// `_transcript_project`: the project of a Claude folder from the `cwd` of its first three transcripts.
fn transcript_project(dir: &File) -> Option<String> {
    let files = names(dir)
        .into_iter()
        .filter(|n| n.ends_with(".jsonl") && n != ".jsonl")
        .take(CWD_SCAN_FILES);
    for name in files {
        if let Some((file, _)) = open_regular(dir, &name)
            && let Some(cwd) = first_cwd(file)
        {
            return Some(cwd);
        }
    }
    None
}

/// `_sources`: project key -> where its memory is. Folder checks only: no note is opened here.
fn sources(
    conn: &Connection,
    stored: &[String],
    env: &Env,
) -> Result<BTreeMap<String, Source>, MetricsError> {
    let mut out: BTreeMap<String, Source> = BTreeMap::new();
    let memory = [MEMORY.to_string()];
    for path in stored {
        if open_chain(path, &memory).is_some() {
            out.entry(path.clone()).or_insert_with(|| Source {
                kit: Some(Folder {
                    root: path.clone(),
                    parts: memory.to_vec(),
                }),
                claude: Vec::new(),
                em: None,
            });
        }
    }
    let by_folder_name: HashMap<String, &String> =
        stored.iter().rev().map(|p| (encoded(p), p)).collect();
    let mut stmt = conn.prepare(
        "SELECT id, folder, label FROM agent_accounts \
         WHERE provider = 'claude' AND status = 'confirmed' ORDER BY id",
    )?;
    let accounts: Vec<(i64, String, Option<String>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    for (id, root, label) in accounts {
        // a confirmed account whose folder is gone is `missing` for Python: skipped
        let Some(projects) = open_chain(&root, &["projects".to_string()]) else {
            continue;
        };
        for name in names(&projects) {
            let Ok(dir) = sys::open_dir_at(&projects, &name) else {
                continue;
            };
            if sys::open_dir_at(&dir, MEMORY).is_err() {
                continue;
            }
            let key = match by_folder_name.get(&name) {
                Some(path) => (*path).clone(),
                None => transcript_project(&dir).unwrap_or_else(|| name.clone()),
            };
            out.entry(key)
                .or_insert_with(|| Source {
                    kit: None,
                    claude: Vec::new(),
                    em: None,
                })
                .claude
                .push((
                    id,
                    label.clone().unwrap_or_default(),
                    Folder {
                        root: root.clone(),
                        parts: vec!["projects".into(), name, MEMORY.into()],
                    },
                ));
        }
    }
    let store = easymem::load(&env.em_home);
    let keys: BTreeSet<String> = out.keys().cloned().chain(stored.iter().cloned()).collect();
    for key in keys {
        let Some(found) = key.starts_with('/').then(|| store.find(&key)).flatten() else {
            continue;
        };
        if let Some(source) = out.get_mut(&key) {
            source.em = Some(found);
        } else if easymem::has_notes(&found) {
            out.insert(
                key,
                Source {
                    kit: None,
                    claude: Vec::new(),
                    em: Some(found),
                },
            );
        }
    }
    Ok(out)
}

/// `_markdown`: the `.md` files of one memory folder in `os.walk` order (files of a folder, then its sub-folders, each
/// sorted), and whether the 5000-file cap stopped the walk. Only regular files are listed; a symlink is never followed.
fn markdown(dir: &File, prefix: &str, depth: usize, out: &mut Vec<(String, (u64, u64))>) -> bool {
    let (mut files, mut dirs) = (Vec::new(), Vec::new());
    for name in names(dir) {
        let Ok(meta) = std::fs::symlink_metadata(format!("{}/{name}", sys::handle_path(dir)))
        else {
            continue;
        };
        if meta.is_dir() {
            dirs.push(name);
        } else if meta.is_file() && name.ends_with(".md") {
            files.push((name, (meta.dev(), meta.ino())));
        }
    }
    for (name, id) in files {
        out.push((format!("{prefix}{name}"), id));
        if out.len() >= MAX_FILES {
            return true;
        }
    }
    if depth < MAX_DEPTH {
        for name in dirs {
            if let Ok(sub) = sys::open_dir_at(dir, &name)
                && markdown(&sub, &format!("{prefix}{name}/"), depth + 1, out)
            {
                return true;
            }
        }
    }
    false
}

/// `_files`: the memory files of one project, each read once even when a file has two names, in a stable order.
fn files(source: &Source) -> (Vec<Entry>, bool) {
    let mut entries = Vec::new();
    let (mut seen, mut capped) = (HashSet::new(), false);
    let mut claude: Vec<&(i64, String, Folder)> = source.claude.iter().collect();
    claude.sort_by_key(|c| c.0);
    let folders = source.kit.iter().map(|f| ("kit", None, None, f)).chain(
        claude
            .into_iter()
            .map(|(i, l, f)| ("claude", Some(*i), Some(l.clone()), f)),
    );
    for (kind, account_id, account, folder) in folders {
        let mut found = Vec::new();
        if let Some(dir) = open_chain(&folder.root, &folder.parts) {
            capped |= markdown(&dir, "", 0, &mut found);
        }
        for (rel, id) in found {
            if seen.insert(id) {
                entries.push(Entry {
                    kind,
                    account_id,
                    account: account.clone(),
                    rel: format!("{MEMORY}/{rel}"),
                    folder: folder.clone(),
                });
            }
        }
    }
    (entries, capped)
}

fn display_name(key: &str) -> String {
    if key.starts_with('/') {
        folder(Some(key))
    } else {
        key.to_string()
    }
}

/// `_clean`: one line, quotes removed, at most 200 characters; block scalars give nothing.
fn clean(value: &str) -> String {
    let value = strip(value);
    if matches!(value, "" | ">" | "|" | ">-" | "|-" | ">+" | "|+") {
        return String::new();
    }
    let chars: Vec<char> = value.chars().collect();
    let value =
        if chars.len() >= 2 && chars[0] == chars[chars.len() - 1] && "\"'".contains(chars[0]) {
            &value[chars[0].len_utf8()..value.len() - chars[0].len_utf8()]
        } else {
            value
        };
    value
        .split(is_space)
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_TEXT)
        .collect()
}

/// `_frontmatter`: `name` and `description` from a leading `---` block (the first 200 lines).
fn frontmatter(text: &str) -> (String, String) {
    let lines: Vec<&str> = text.splitn(201, '\n').take(200).collect();
    let (mut name, mut description) = (String::new(), String::new());
    if lines.first().is_none_or(|l| strip(l) != "---") {
        return (name, description);
    }
    for line in &lines[1..] {
        if strip(line) == "---" {
            return (name, description);
        }
        if let Some((key, value)) = line.split_once(':') {
            match key {
                "name" => name = clean(value),
                "description" => description = clean(value),
                _ => {}
            }
        }
    }
    (String::new(), String::new()) // no closing fence: not frontmatter
}

/// `_LINK.findall`: the targets of `[[name]]`, `[[name|alias]]` and `[[name#anchor]]`, each stripped.
fn links(text: &str) -> Vec<String> {
    let c: Vec<char> = text.chars().collect();
    let (mut found, mut i) = (Vec::new(), 0);
    while i + 1 < c.len() {
        if c[i] != '[' || c[i + 1] != '[' {
            i += 1;
            continue;
        }
        let start = i + 2;
        let mut k = start;
        while k < c.len() && !matches!(c[k], '[' | ']' | '|' | '#' | '\n') {
            k += 1;
        }
        let mut end = k;
        if k > start && matches!(c.get(k), Some('|' | '#')) {
            end = k + 1;
            while end < c.len() && !matches!(c[end], '[' | ']' | '\n') {
                end += 1;
            }
        }
        if k > start && c.get(end) == Some(&']') && c.get(end + 1) == Some(&']') {
            let target: String = c[start..k].iter().collect();
            found.push(strip(&target).to_string());
            i = end + 2;
        } else {
            i += 1;
        }
    }
    found
}

struct Note {
    name: String,
    description: String,
    links: Vec<String>,
}

/// `_read_note`: `None` when the note is abstained (too big, unreadable, not UTF-8, not a regular file).
fn read_note(entry: &Entry) -> Option<Note> {
    let mut parts = entry.folder.parts.clone();
    let below: Vec<&str> = entry.rel[MEMORY.len() + 1..].split('/').collect();
    parts.extend(below[..below.len() - 1].iter().map(|p| p.to_string()));
    let dir = open_chain(&entry.folder.root, &parts)?;
    let (file, meta) = open_regular(&dir, below[below.len() - 1])?;
    let text = String::from_utf8(read_capped(file, &meta, MAX_NOTE_BYTES)?).ok()?;
    let (name, description) = frontmatter(&text);
    Some(Note {
        name,
        description,
        links: links(&text),
    })
}

fn ring(entry: &Entry) -> i64 {
    let name = entry.rel.rsplit('/').next().unwrap_or("");
    if entry.kind == "kit" {
        if entry.rel == "memory/MEMORY.md" {
            0
        } else if entry.rel.matches('/').count() == 1 {
            1
        } else {
            2
        }
    } else if name == "MEMORY.md" {
        1
    } else {
        2
    }
}

/// `Path(name).stem`: the name without its last suffix (a leading dot is not a suffix).
fn stem(name: &str) -> &str {
    match name.rfind('.') {
        Some(i) if i > 0 && i < name.len() - 1 => &name[..i],
        _ => name,
    }
}

fn scoped(conn: &Connection) -> Result<Vec<String>, MetricsError> {
    Ok(scope::resolve(conn, Scope::default())?
        .scoped_projects(conn)?
        .into_iter()
        .collect())
}

/// `project_list`: the projects that have at least one memory note, with the note counts per origin.
pub fn project_list(conn: &Connection, env: &Env) -> Result<Value, MetricsError> {
    let stored = scoped(conn)?;
    let stored_set: HashSet<&String> = stored.iter().collect();
    let (mut rows, mut gone) = (Vec::new(), 0);
    for (key, source) in sources(conn, &stored, env)? {
        let (entries, _) = files(&source);
        let kit_notes = entries.iter().filter(|e| e.kind == "kit").count();
        let em_notes = source.em.as_deref().map_or(0, easymem::note_count);
        if entries.is_empty() && em_notes == 0 {
            continue;
        }
        if stored_set.contains(&key) && !Path::new(&key).is_dir() {
            gone += 1;
            continue;
        }
        rows.push(json!({
            "project": key, "name": display_name(&key), "kit_notes": kit_notes,
            "claude_notes": entries.len() - kit_notes, "easy_memory_notes": em_notes,
        }));
    }
    Ok(json!({"projects": rows, "gone": gone}))
}

struct Node {
    id: String,
    kind: &'static str,
    ring: i64,
    name: String,
    stem: String,
    description: String,
    path: Option<String>,
    account: Option<String>,
    abstained: bool,
    links_in: u64,
    links_out: u64,
    cited: Option<Value>,
}

impl Node {
    fn json(&self) -> Value {
        let mut node = json!({
            "id": self.id, "kind": self.kind, "ring": self.ring, "name": self.name, "stem": self.stem,
            "description": self.description, "path": self.path, "account": self.account,
            "abstained": self.abstained, "links_in": self.links_in, "links_out": self.links_out,
        });
        if let Some(fields) = &self.cited {
            for (key, value) in fields.as_object().into_iter().flatten() {
                node[key] = value.clone();
            }
        }
        node
    }

    fn special(id: String, kind: &'static str, ring: i64, name: &str, stem: &str) -> Node {
        Node {
            id,
            kind,
            ring,
            name: name.into(),
            stem: stem.into(),
            description: String::new(),
            path: None,
            account: None,
            abstained: false,
            links_in: 0,
            links_out: 0,
            cited: None,
        }
    }
}

/// `project_graph`: the nodes and edges of one project's memory notes. An unknown project is `Invalid("unknown project")`.
pub fn project_graph(conn: &Connection, env: &Env, project: &str) -> Result<Value, MetricsError> {
    let stored = scoped(conn)?;
    let all = sources(conn, &stored, env)?;
    let Some(source) = all.get(project) else {
        return Err(MetricsError::Invalid("unknown project".into()));
    };
    let (entries, capped) = files(source);
    let (mut nodes, mut targets) = (Vec::new(), Vec::new());
    for entry in &entries {
        let note = read_note(entry);
        let file_stem = stem(entry.rel.rsplit('/').next().unwrap_or(""));
        let prefix = match entry.account_id {
            None => "kit".to_string(),
            Some(id) => format!("claude:{id}"),
        };
        nodes.push(Node {
            id: format!("{prefix}:{}", entry.rel),
            kind: entry.kind,
            ring: ring(entry),
            name: note
                .as_ref()
                .map(|n| n.name.clone())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| file_stem.to_string()),
            stem: file_stem.to_string(),
            description: note
                .as_ref()
                .map_or(String::new(), |n| n.description.clone()),
            path: Some(entry.rel.clone()),
            account: entry.account.clone(),
            abstained: note.is_none(),
            links_in: 0,
            links_out: 0,
            cited: None,
        });
        targets.push(note.map_or_else(Vec::new, |n| n.links));
    }

    let (mut by_name, mut by_stem): (HashMap<String, String>, HashMap<String, String>) =
        (HashMap::new(), HashMap::new());
    let mut duplicates = 0;
    for node in nodes.iter().filter(|n| !n.abstained) {
        let key = casefold(&node.name);
        match by_name.entry(key) {
            std::collections::hash_map::Entry::Occupied(_) => duplicates += 1,
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(node.id.clone());
            }
        }
        by_stem
            .entry(casefold(&node.stem))
            .or_insert_with(|| node.id.clone());
    }

    let mut dangling: BTreeMap<String, Node> = BTreeMap::new();
    let mut edges: BTreeSet<(String, String)> = BTreeSet::new();
    for (node, wanted) in nodes.iter().zip(&targets) {
        for target in wanted {
            let key = casefold(target);
            let found = match by_name.get(&key).or_else(|| by_stem.get(&key)) {
                Some(id) => id.clone(),
                None => {
                    let id = format!("dangling:{key}");
                    dangling.entry(id.clone()).or_insert_with(|| {
                        Node::special(id.clone(), "dangling", 3, target, target)
                    });
                    id
                }
            };
            if found != node.id {
                edges.insert((node.id.clone(), found));
            }
        }
    }
    for (from, to) in &edges {
        if let Some(node) = nodes.iter_mut().find(|n| &n.id == from) {
            node.links_out += 1;
        }
        if let Some(node) = nodes.iter_mut().find(|n| &n.id == to) {
            node.links_in += 1;
        } else if let Some(node) = dangling.get_mut(to) {
            node.links_in += 1;
        }
    }
    let (mut panel, mut cited_nodes) = (Value::Null, Vec::new());
    if let Some(folder) = &source.em {
        let today = env
            .zone
            .to_datetime(jiff::Timestamp::from_microsecond(env.now_micros).unwrap_or_default())
            .date();
        let (mut summary, cited) = easymem::summary(folder, project, today, &env.zone);
        summary["report"] = easymem::report(folder);
        panel = summary;
        for item in cited {
            let fields = json!({"cited": item.cited, "newest": item.newest, "stale": item.stale});
            let mut hit = false;
            for node in nodes
                .iter_mut()
                .filter(|n| n.kind == "kit" && n.path.as_deref() == Some(&item.path))
            {
                node.cited = Some(fields.clone());
                hit = true;
            }
            if !hit {
                let name = item.path.rsplit('/').next().unwrap_or("");
                let mut node = Node::special(
                    format!("easymem:{}", item.path),
                    "easy_memory",
                    4,
                    name,
                    stem(name),
                );
                node.path = Some(item.path.clone());
                node.cited = Some(fields);
                cited_nodes.push(node);
            }
        }
    }
    let notes = nodes.len();
    let abstained = nodes.iter().filter(|n| n.abstained).count();
    let all_nodes: Vec<Value> = nodes
        .iter()
        .chain(dangling.values())
        .chain(&cited_nodes)
        .map(Node::json)
        .collect();
    Ok(json!({
        "project": project,
        "name": display_name(project),
        "nodes": all_nodes,
        "edges": edges.iter().map(|(s, t)| json!({"source": s, "target": t})).collect::<Vec<_>>(),
        "counts": {
            "notes": notes, "links": edges.len(), "dangling": dangling.len(), "abstained": abstained,
            "duplicates": duplicates, "capped": capped,
        },
        "easy_memory": panel,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_follow_the_python_pattern() {
        assert_eq!(
            links("[[a]] [[b|x]] [[c#y]] [[ d ]] [[]] [[e\nf]] [[[g]] [[h|i\n]] [[j"),
            ["a", "b", "c", "d", "g"]
        );
    }

    #[test]
    fn stems_follow_pathlib() {
        assert_eq!(stem("a.md"), "a");
        assert_eq!(stem("a.b.md"), "a.b");
        assert_eq!(stem(".md"), ".md");
        assert_eq!(stem("..md"), ".");
    }

    #[test]
    fn a_description_is_one_clean_line() {
        assert_eq!(clean("  \"two\n words\"  "), "two words");
        assert_eq!(clean(">"), "");
        assert_eq!(clean("'"), "'");
    }
}
