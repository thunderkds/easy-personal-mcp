//! The live tail engine (FR-043, T049): `src/easy_personal/core/live.py` without its thread and without its store
//! writes. Given each account's `projects/` tree, [`LiveTail::poll_once`] reads only complete new lines, turns each into
//! metadata events in a bounded buffer and keeps per-session state (totals, kit stage, open calls, failing, waiting,
//! needs attention) exactly as Python does. Time is an input (`now`, microseconds since the Unix epoch, UTC): nothing
//! here reads the clock, so the oracle (`rust/parity/gen_live.py`) can freeze it.
//!
//! **Publish-only** (the owner's Path 1, `BRAINSTORMING_LOG_RUSTLIVE.md`): Python stores a line from the store cursor on
//! under the ingest lock; this engine never writes the store and never takes `ingest.lock` (the scheduled `ingest` stays
//! the only writer). The store is read, read-only, for the cursor of each file and the account labels. Every line is
//! published from the tail's own offset, which is Python's FR-071 path (a line below the cursor is published only).
//!
//! Read-only on every source (NFR-003); an event holds names, ids, numbers and timestamps only (FR-002). Of a tool
//! call's input only `skill`, `subagent_type` and a Bash command's first word are read ([`call_label`], DDR-0004); of a
//! sub-agent meta file only `agentType` (FR-082). Labels, sub-agent types and the stage live in memory, never stored.
//!
//! Hostile files (wave-3 rules): every transcript and meta file is opened `O_NOFOLLOW | O_NONBLOCK`, judged on the open
//! handle (a regular file with the scanned inode), and read through a bound. ⚠ decided for you, where Python differs:
//! - a symlinked `*.jsonl` is not followed (Python's `entry.is_file()` follows it), and a file whose open fails for any
//!   reason is skipped for this step (Python's poll stops at a non-`FileNotFoundError` error);
//! - a line longer than [`MAX_LINE_BYTES`] is skipped once its newline arrives (no event; Python parses it), and while
//!   it has no newline the file waits, never holding more than the bound in memory;
//! - [`MAX_BATCH_LINES`] is the publish batch, as in Python, which reads every new line of a file in one poll.

use crate::board::is_space;
use crate::ingest::claude_code::parse_record;
use crate::ingest::folders::{SOURCE, resolve_project};
use crate::ingest::json::{self, Value};
use crate::ingest::label::label;
use crate::ingest::secret::is_secret;
use crate::jobs::command_name;
use crate::metrics::format_stored;
use crate::prices::PriceTable;
use crate::store;
use crate::writer::Event;
use rusqlite::{Connection, OptionalExtension};
use serde_json::{Value as Json, json};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};

pub const BUFFER_SIZE: usize = 2000;
/// FR-044: a session is active when it wrote a record in the last 5 minutes.
pub const ACTIVE_WINDOW_S: i64 = 300;
/// An idle session's running totals are dropped after an hour (bounded memory).
pub const FORGET_AFTER_S: i64 = 3600;
/// FR-086: the warm start replays at most this many trailing bytes of one file.
pub const WARM_MAX_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_BATCH_LINES: usize = 500;
/// An open call older than a day is dropped (the session ended without a result).
pub const OPEN_CALL_MAX_S: i64 = 24 * 3600;
/// Python's `rules.LIVE_DEFAULTS` (the `config.yaml` overrides are not ported: cut list).
pub const FAILING_STREAK: i64 = 3;
pub const OPEN_CALL_MINUTES: i64 = 10;
pub const COMMAND_READ_CHARS: usize = 4096;
pub const META_MAX_BYTES: u64 = 4096;
pub const META_WAIT_S: i64 = 30;
pub const SUB_AGENT: &str = "sub-agent";
pub const DEFAULT_LABEL: &str = "Claude default";
/// ⚠ decided for you: the bound on one transcript line held in memory (Python has none).
pub const MAX_LINE_BYTES: usize = 32 * 1024 * 1024;
const WAITING_TOOLS: [&str; 2] = ["AskUserQuestion", "ExitPlanMode"];
const SECOND: i64 = 1_000_000;

// `os.O_NOFOLLOW | os.O_NONBLOCK`: the one per-arch table of the crate (`report_files::sys`); a target it does not list
// fails to compile here instead of opening with a wrong flag.
use crate::report_files::sys::{O_NOFOLLOW, O_NONBLOCK};

/// FR-083: the kit stage of a skill (owner choice 2026-11-06); a skill not listed leaves the stage unchanged.
fn stage_of(skill: &str) -> Option<&'static str> {
    Some(match skill {
        "brainstorming" | "ideate" => "0.5",
        "git-guardrails-claude-code" | "map-codebase" => "1",
        "craft-agent" => "1.5",
        "grill-with-docs" | "to-issues" => "2",
        "tdd" | "bugfix" | "diagnose" | "craft-spawn-prompt" | "migration-safety" => "3",
        "code-review" | "security-review" | "blast-radius" | "html-report" => "4",
        "verify" | "ship" | "delivery-report" => "5",
        _ => return None,
    })
}

// ---- labels (FR-081) ------------------------------------------------------------------------------------------------

/// `_safe(value)` on the WTF-8 bytes of a Python `str`: `^[A-Za-z0-9][A-Za-z0-9:._-]{0,63}$` (a `fullmatch`, so no
/// trailing-newline quirk) and not secret-looking (`reports._SECRET`).
fn safe_bytes(s: &[u8]) -> Option<String> {
    let ok = matches!(s.first(), Some(b) if b.is_ascii_alphanumeric())
        && s.len() <= 64
        && s.iter()
            .all(|&b| b.is_ascii_alphanumeric() || b":._-".contains(&b));
    (ok && !is_secret(s)).then(|| String::from_utf8_lossy(s).into_owned())
}

/// `_safe(value)`: the string when it is a safe label, else `None` (also for a non-string).
pub fn safe(value: &Value) -> Option<String> {
    safe_bytes(value.as_str()?)
}

/// `_CD_PREFIX.match` on code points: `\s*cd\s+(?:"[^"]*"|'[^']*'|[^\s;&|]+)\s*(?:&&|;)\s*`, the end of the match.
fn cd_prefix(c: &[u32]) -> Option<usize> {
    let sp = |i: usize| {
        c.get(i)
            .and_then(|&x| char::from_u32(x))
            .is_some_and(is_space)
    };
    let is = |i: usize, ch: char| c.get(i) == Some(&(ch as u32));
    let spaces = |mut i: usize| {
        while sp(i) {
            i += 1;
        }
        i
    };
    let tail = |j: usize| {
        let j = spaces(j);
        let j = if is(j, '&') && is(j + 1, '&') {
            j + 2
        } else if is(j, ';') {
            j + 1
        } else {
            return None;
        };
        Some(spaces(j))
    };
    let i = spaces(0);
    if !(is(i, 'c') && is(i + 1, 'd')) {
        return None;
    }
    let i = spaces(i + 2);
    if !sp(i - 1) {
        return None; // `\s+`: at least one space after `cd`
    }
    for q in ['"', '\''] {
        if is(i, q)
            && let Some(k) = c[i + 1..].iter().position(|&x| x == q as u32)
            && let Some(end) = tail(i + 1 + k + 1)
        {
            return Some(end);
        }
    }
    let mut j = i;
    while c
        .get(j)
        .is_some_and(|&x| !(sp(j) || [';' as u32, '&' as u32, '|' as u32].contains(&x)))
    {
        j += 1;
    }
    if j == i { None } else { tail(j) }
}

/// `_bash_label(command)`: the first word of a Bash command after any leading `cd <dir> &&` / `cd <dir>;`.
pub fn bash_label(command: &Value) -> Option<String> {
    let s = command.as_str()?;
    let points: Vec<u32> = json::code_points(s).take(COMMAND_READ_CHARS).collect();
    let mut rest: &[u32] = &points;
    while let Some(end) = cd_prefix(rest) {
        rest = &rest[end..];
    }
    let text: String = rest
        .iter()
        .map(|&x| char::from_u32(x).unwrap_or('\u{FFFD}'))
        .collect();
    safe_bytes(command_name(&text)?.as_bytes())
}

/// A call label: a name, or an MCP tool's `{server, tool}`.
#[derive(Debug, Clone, PartialEq)]
pub enum Label {
    Name(String),
    Mcp { server: String, tool: String },
}

impl Label {
    pub fn to_json(&self) -> Json {
        match self {
            Label::Name(n) => json!(n),
            Label::Mcp { server, tool } => json!({"server": server, "tool": tool}),
        }
    }
}

/// `_call_label(name, tool_input)`: reads `skill` (Skill), `subagent_type` (Agent, Task) or the first word of
/// `command` (Bash); an MCP tool is split from its name and its input is not read. The only place an input is read.
pub fn call_label(name: &str, input: &Value) -> Option<Label> {
    if let Some(rest) = name.strip_prefix("mcp__") {
        let (server, tool) = rest.split_once("__").unwrap_or((rest, ""));
        let (server, tool) = (safe_bytes(server.as_bytes())?, safe_bytes(tool.as_bytes())?);
        return Some(Label::Mcp { server, tool });
    }
    let field = match name {
        "Skill" => "skill",
        "Agent" | "Task" => "subagent_type",
        "Bash" => "command",
        _ => return None,
    };
    if !input.is_dict() {
        return None;
    }
    let value = input.get(field);
    if name == "Bash" {
        bash_label(value).map(Label::Name)
    } else {
        safe(value).map(Label::Name)
    }
}

// ---- items ----------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    ToolUse,
    ToolResult,
    Usage,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::ToolUse => "tool_use",
            Kind::ToolResult => "tool_result",
            Kind::Usage => "usage",
        }
    }
}

#[derive(Debug, Clone)]
struct ItemEvent {
    kind: Kind,
    tool: Option<String>,
    is_error: bool,
    block_id: Option<String>,
    label: Option<Label>,
}

#[derive(Debug, Clone)]
struct Usage {
    message_id: String,
    model: Option<String>,
    counts: [i64; 4],
    api_error: bool,
}

/// `_item(record, parsed)`: the metadata of one decoded record (memory only; `block_id` never reaches the buffer).
#[derive(Debug, Clone)]
pub struct Item {
    session_id: String,
    project: Option<String>,
    ts: String,
    is_sidechain: bool,
    message_id: Option<String>,
    events: Vec<ItemEvent>,
    usage: Option<Usage>,
}

/// `Path(path).name or path`.
fn project_name(path: &str) -> String {
    let name = match Path::new(path).components().next_back() {
        Some(Component::Normal(s)) => s.to_string_lossy().into_owned(),
        Some(Component::ParentDir) => "..".to_owned(),
        _ => String::new(),
    };
    if name.is_empty() {
        path.to_owned()
    } else {
        name
    }
}

fn item(record: &Value, p: &Event, project_of: &mut dyn FnMut(&str) -> String) -> Option<Item> {
    if p.abstained {
        return None;
    }
    let (session_id, ts) = (p.session_id.clone()?, p.ts_utc.clone()?);
    let rtype = p.record_type.as_deref();
    let content = match rtype {
        Some("assistant" | "user") => record.get("message").get("content"),
        _ => &Value::Null,
    };
    let mut events = Vec::new();
    if let Value::Array(blocks) = content {
        for block in blocks.iter().filter(|b| b.is_dict()) {
            let btype = block.get("type").as_str();
            if btype == Some(b"tool_use") && rtype == Some("assistant") {
                if let Some(name) = label(block.get("name")) {
                    events.push(ItemEvent {
                        kind: Kind::ToolUse,
                        label: call_label(&name, block.get("input")),
                        tool: Some(name),
                        is_error: false,
                        block_id: label(block.get("id")),
                    });
                }
            } else if btype == Some(b"tool_result") && rtype == Some("user") {
                events.push(ItemEvent {
                    kind: Kind::ToolResult,
                    tool: None,
                    is_error: block.get("is_error").is_true(),
                    block_id: label(block.get("tool_use_id")),
                    label: None,
                });
            }
        }
    }
    let usage = match (&p.message_id, p.input_tokens) {
        (Some(id), Some(input)) => Some(Usage {
            message_id: id.clone(),
            model: p.model.clone(),
            counts: [
                input,
                p.output_tokens.unwrap_or(0),
                p.cache_creation_input_tokens.unwrap_or(0),
                p.cache_read_input_tokens.unwrap_or(0),
            ],
            api_error: p.is_api_error == Some(true),
        }),
        _ => None,
    };
    Some(Item {
        session_id,
        project: p
            .project_path
            .as_deref()
            .map(|cwd| project_name(&project_of(cwd))),
        ts,
        is_sidechain: p.is_sidechain == Some(true),
        message_id: p.message_id.clone(),
        events,
        usage,
    })
}

impl Item {
    /// Python's `_item` dict, for the parity table (`usage` is the tuple as a list).
    pub fn to_json(&self) -> Json {
        let events: Vec<Json> = self
            .events
            .iter()
            .map(|e| {
                let mut v = json!({"kind": e.kind.name(), "tool": e.tool, "is_error": e.is_error,
                                   "block_id": e.block_id});
                if e.kind == Kind::ToolUse {
                    v["label"] = e.label.as_ref().map_or(Json::Null, Label::to_json);
                }
                v
            })
            .collect();
        let usage = self.usage.as_ref().map(|u| {
            let [i, o, c, r] = u.counts;
            json!([u.message_id, u.model, i, o, c, r, u.api_error])
        });
        json!({"session_id": self.session_id, "project": self.project, "ts": self.ts,
               "is_sidechain": self.is_sidechain, "message_id": self.message_id,
               "events": events, "usage": usage})
    }
}

/// One line (with its newline) to its item: `json.loads`, a dict, `parse_record`, `_item`. `project_of` resolves a
/// `cwd` (the tail passes `discovery.resolve_project`, cached).
pub fn line_item(line: &[u8], project_of: &mut dyn FnMut(&str) -> String) -> Option<Item> {
    let record = json::loads(line)?;
    if !record.is_dict() {
        return None;
    }
    item(&record, &parse_record(&record), project_of)
}

// ---- hostile-safe reading -------------------------------------------------------------------------------------------

/// Opens a file without following a symlink and without blocking (a FIFO); `Some` only for a regular file.
fn open_regular(path: &Path) -> Option<(File, fs::Metadata)> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW | O_NONBLOCK)
        .open(path)
        .ok()?;
    let meta = file.metadata().ok()?;
    meta.is_file().then_some((file, meta))
}

enum Next {
    Line(Vec<u8>),
    /// A line over [`MAX_LINE_BYTES`], its newline seen: its length.
    Skipped(u64),
    End,
}

/// The next complete line, bounded: at most `MAX_LINE_BYTES + 1` bytes are held; a longer line is skipped to its
/// newline with a small buffer. A last line without `\n` is not complete: `End`, and nothing moves into it.
fn next_line<R: BufRead>(reader: &mut R) -> io::Result<Next> {
    let mut buf = Vec::new();
    reader
        .by_ref()
        .take(MAX_LINE_BYTES as u64 + 1)
        .read_until(b'\n', &mut buf)?;
    if buf.last() == Some(&b'\n') {
        return Ok(Next::Line(buf));
    }
    if buf.len() <= MAX_LINE_BYTES {
        return Ok(Next::End);
    }
    let mut skipped = buf.len() as u64;
    drop(buf);
    loop {
        let chunk = reader.fill_buf()?;
        if chunk.is_empty() {
            return Ok(Next::End);
        }
        if let Some(i) = chunk.iter().position(|&b| b == b'\n') {
            reader.consume(i + 1);
            return Ok(Next::Skipped(skipped + i as u64 + 1));
        }
        let n = chunk.len();
        skipped += n as u64;
        reader.consume(n);
    }
}

/// `_meta_agent_type(stream_path)`: `agentType` of `<stream>.meta.json`, read once through a no-follow, non-blocking
/// open judged on the handle, at most [`META_MAX_BYTES`]; only that one field is taken.
fn meta_agent_type(stream: &str) -> Option<String> {
    let meta = format!("{}.meta.json", stream.strip_suffix(".jsonl")?);
    let (file, info) = open_regular(Path::new(&meta))?;
    if info.len() > META_MAX_BYTES {
        return None;
    }
    let mut raw = Vec::new();
    file.take(META_MAX_BYTES + 1).read_to_end(&mut raw).ok()?;
    if raw.len() as u64 > META_MAX_BYTES {
        return None;
    }
    let data = json::loads(&raw)?;
    if !data.is_dict() {
        return None;
    }
    safe(data.get("agentType"))
}

/// `_jsonl_files(root)`: `(path, inode, size, mtime)` of every regular `*.jsonl` under `root` in Python's walk order
/// (a stack of folders, entries in directory order). A symlink is neither entered nor read.
fn jsonl_files(root: &Path) -> Vec<(String, u64, u64, f64)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_dir() {
                stack.push(path);
            } else if kind.is_file()
                && entry.file_name().as_encoded_bytes().ends_with(b".jsonl")
                && let (Ok(meta), Some(text)) = (entry.metadata(), path.to_str())
            {
                let mtime = meta.mtime() as f64 + meta.mtime_nsec() as f64 * 1e-9;
                out.push((text.to_owned(), meta.ino(), meta.len(), mtime));
            }
        }
    }
    out
}

/// `_cursor(state, inode, size)`: the store cursor of this file, 0 when unknown, replaced or shorter.
fn cursor(state: Option<&(i64, i64)>, inode: u64, size: u64) -> u64 {
    match state {
        Some(&(i, off)) if i == inode as i64 && off >= 0 && off as u64 <= size => off as u64,
        _ => 0,
    }
}

// ---- store (read-only) ----------------------------------------------------------------------------------------------

/// One account's transcript tree: `(account id, <folder>/projects)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Root {
    pub account: Option<i64>,
    pub projects: PathBuf,
}

/// `claude_code.account_roots(conn, override)`: every confirmed Claude account in id order; `projects_override`
/// replaces the default account's folder, and with no default account it is read untagged.
pub fn account_roots(
    conn: &Connection,
    projects_override: Option<&Path>,
) -> rusqlite::Result<Vec<Root>> {
    let mut stmt = conn.prepare(
        "SELECT id, folder, config_dir_env FROM agent_accounts \
         WHERE provider = 'claude' AND status = 'confirmed' ORDER BY id",
    )?;
    let rows: Vec<(i64, String, Option<String>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let mut roots: Vec<Root> = rows
        .iter()
        .map(|(id, folder, env)| Root {
            account: Some(*id),
            projects: match projects_override {
                Some(o) if env.is_none() => o.to_path_buf(),
                _ => Path::new(folder).join("projects"),
            },
        })
        .collect();
    if let Some(o) = projects_override
        && !rows.iter().any(|(_, _, env)| env.is_none())
    {
        roots.push(Root {
            account: None,
            projects: o.to_path_buf(),
        });
    }
    Ok(roots)
}

/// `store.file_offsets(conn, SOURCE)` read-only: a store without a `machine_id` has no cursor on this machine.
fn file_offsets(conn: &Connection) -> rusqlite::Result<HashMap<String, (i64, i64)>> {
    let machine: Option<String> = conn
        .query_row("SELECT value FROM meta WHERE key = 'machine_id'", [], |r| {
            r.get(0)
        })
        .optional()?;
    let Some(machine) = machine else {
        return Ok(HashMap::new());
    };
    let mut stmt = conn.prepare(
        "SELECT path, inode, byte_offset FROM source_files WHERE machine_id = ? AND source = ?",
    )?;
    let rows = stmt.query_map(rusqlite::params![machine, SOURCE], |r| {
        Ok((r.get(0)?, (r.get(1)?, r.get(2)?)))
    })?;
    rows.collect()
}

/// `_refresh_roots`'s labels: every account's label (NULL for a detected or ignored row), and `None` -> the default
/// account's label, else [`DEFAULT_LABEL`] (Python's `labels.get(default_id) or DEFAULT_LABEL`).
type Labels = HashMap<Option<i64>, Option<String>>;

fn account_labels(conn: &Connection) -> rusqlite::Result<Labels> {
    let mut stmt = conn.prepare("SELECT id, label FROM agent_accounts")?;
    let mut labels: Labels = stmt
        .query_map([], |r| Ok((Some(r.get(0)?), r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let default: Option<i64> = conn
        .query_row(
            "SELECT id FROM agent_accounts WHERE provider = 'claude' AND config_dir_env IS NULL ORDER BY id",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let label = default
        .and_then(|d| labels.get(&Some(d)).cloned().flatten())
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| DEFAULT_LABEL.to_owned());
    labels.insert(None, Some(label));
    Ok(labels)
}

// ---- state ----------------------------------------------------------------------------------------------------------

/// An insertion-ordered map (Python's `dict`): a re-set key keeps its place, iteration is in insertion order. A removed
/// key leaves a tombstone (`pop` is O(1)); the slots are compacted once tombstones outnumber the live keys.
#[derive(Debug)]
struct Ordered<V> {
    index: HashMap<String, usize>,
    items: Vec<Option<(String, V)>>,
}

impl<V> Default for Ordered<V> {
    fn default() -> Self {
        Ordered {
            index: HashMap::new(),
            items: Vec::new(),
        }
    }
}

impl<V> Ordered<V> {
    fn slot(&self, i: usize) -> &V {
        &self.items[i].as_ref().expect("indexed slot is live").1
    }
    fn get(&self, key: &str) -> Option<&V> {
        self.index.get(key).map(|&i| self.slot(i))
    }
    fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        let i = *self.index.get(key)?;
        self.items[i].as_mut().map(|(_, v)| v)
    }
    fn entry(&mut self, key: &str, make: impl FnOnce() -> V) -> &mut V {
        if !self.index.contains_key(key) {
            self.items.push(Some((key.to_owned(), make())));
            self.index.insert(key.to_owned(), self.items.len() - 1);
        }
        self.get_mut(key).expect("just inserted")
    }
    fn set_value(&mut self, key: &str, value: V) {
        match self.get_mut(key) {
            Some(slot) => *slot = value,
            None => {
                self.items.push(Some((key.to_owned(), value)));
                self.index.insert(key.to_owned(), self.items.len() - 1);
            }
        }
    }
    fn retain(&mut self, mut keep: impl FnMut(&str, &mut V) -> bool) {
        self.items
            .retain_mut(|slot| slot.as_mut().is_some_and(|(k, v)| keep(k, v)));
        self.index = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.as_ref().map(|(k, _)| (k.clone(), i)))
            .collect();
    }
    fn iter(&self) -> impl Iterator<Item = (&String, &V)> {
        self.items.iter().flatten().map(|(k, v)| (k, v))
    }
    fn values(&self) -> impl Iterator<Item = &V> {
        self.items.iter().flatten().map(|(_, v)| v)
    }
    fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.items.iter_mut().flatten().map(|(_, v)| v)
    }
    fn len(&self) -> usize {
        self.index.len()
    }
    fn is_empty(&self) -> bool {
        self.index.is_empty()
    }
    fn pop(&mut self, key: &str) -> Option<V> {
        let i = self.index.remove(key)?;
        let (_, v) = self.items[i].take()?;
        if self.items.len() > 2 * self.index.len() + 16 {
            self.retain(|_, _| true); // compaction: amortised O(1) per pop
        }
        Some(v)
    }
}

#[derive(Debug, Clone)]
struct Call {
    tool: String,
    message_id: Option<String>,
    ts: String,
}

#[derive(Debug, Default)]
struct Stream {
    is_sidechain: bool,
    sub_agent: bool,
    agent: Option<String>,
    meta_since: Option<i64>,
    open: Ordered<Call>,
    streak: i64,
    streak_tool: Option<String>,
    streak_since: String,
}

type Figures = ([i64; 4], Option<f64>);

#[derive(Debug, Clone, PartialEq)]
struct Attention {
    state: &'static str,
    tool: Option<String>,
    count: i64,
    is_sidechain: bool,
    since: String,
}

#[derive(Debug)]
struct Session {
    account: Option<String>,
    project: Option<String>,
    last_ts: String,
    tool_calls: i64,
    messages: Ordered<Figures>,
    streams: Ordered<Stream>,
    attention: Vec<Attention>,
    stage: Option<&'static str>,
    stage_skill: Option<String>,
    stage_ts: String,
}

impl Session {
    fn has_open_call(&self) -> bool {
        self.streams.values().any(|s| !s.open.is_empty())
    }
}

/// One buffered event (`_append`'s dict).
#[derive(Debug, Clone)]
struct Buffered {
    id: i64,
    session_id: String,
    account: Option<String>,
    project: Option<String>,
    ts: String,
    kind: Kind,
    tool: Option<String>,
    is_error: bool,
    is_sidechain: bool,
    label: Option<Label>,
    agent: Option<String>,
    tokens: Option<[i64; 4]>,
    cost: Option<f64>,
}

impl Buffered {
    fn to_json(&self) -> Json {
        let t = |i: usize| self.tokens.map(|t| t[i]);
        json!({
            "id": self.id, "session_id": self.session_id, "account": self.account,
            "project": self.project.as_deref().unwrap_or("(unknown)"), "ts": self.ts,
            "kind": self.kind.name(), "tool": self.tool, "is_error": self.is_error,
            "is_sidechain": self.is_sidechain,
            "label": self.label.as_ref().map_or(Json::Null, Label::to_json), "agent": self.agent,
            "tokens_in": t(0), "tokens_out": t(1), "cache_creation": t(2), "cache_read": t(3),
            "cost_usd": self.cost,
        })
    }
}

/// The tail: per-file offsets, per-session state and the bounded buffer of recent events. No thread, no clock.
pub struct LiveTail {
    prices: PriceTable,
    state_version: i64,
    buffer: VecDeque<Buffered>,
    last_id: i64,
    sessions: Ordered<Session>,
    projects: HashMap<String, String>,
    offsets: HashMap<String, (u64, u64)>,
    primed: bool,
    replaying: bool,
}

/// Counts only: no path, session id or label reaches a `Debug` line.
impl fmt::Debug for LiveTail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LiveTail")
            .field("sessions", &self.sessions.len())
            .field("files", &self.offsets.len())
            .field("buffered", &self.buffer.len())
            .field("last_id", &self.last_id)
            .field("state_version", &self.state_version)
            .finish()
    }
}

impl LiveTail {
    pub fn new(prices: PriceTable) -> Self {
        LiveTail {
            prices,
            state_version: 0,
            buffer: VecDeque::with_capacity(BUFFER_SIZE),
            last_id: 0,
            sessions: Ordered::default(),
            projects: HashMap::new(),
            offsets: HashMap::new(),
            primed: false,
            replaying: false,
        }
    }

    // -- buffer API ---------------------------------------------------------------------------------------------------

    /// Id of the newest event so far (0: none yet).
    pub fn last_id(&self) -> i64 {
        self.last_id
    }

    /// Counts changes of any session's attention states or kit stage (and each warm replay).
    pub fn state_version(&self) -> i64 {
        self.state_version
    }

    /// Buffered events with `id > last_id`, oldest first.
    pub fn events_since(&self, last_id: i64) -> Vec<Json> {
        self.buffer
            .iter()
            .filter(|e| e.id > last_id)
            .map(Buffered::to_json)
            .collect()
    }

    /// True when events after `last_id` were already dropped from the buffer (send a fresh snapshot).
    pub fn rolled_past(&self, last_id: i64) -> bool {
        self.buffer
            .front()
            .is_some_and(|e| last_id.saturating_add(1) < e.id)
    }

    /// Active sessions with running totals and the `last_id` they include. A session with an open call is listed
    /// even when it wrote nothing for 5 minutes (FR-044).
    pub fn snapshot(&self, now: i64) -> Json {
        let cutoff = format_stored(now - ACTIVE_WINDOW_S * SECOND);
        let mut sessions: Vec<(&String, Json)> = Vec::new();
        for (id, s) in self.sessions.iter() {
            if s.last_ts < cutoff && !s.has_open_call() {
                continue;
            }
            let figures: Vec<&Figures> = s.messages.values().collect();
            let sum = |i: usize| figures.iter().fold(0i64, |a, f| a.saturating_add(f.0[i]));
            let cost = if figures.is_empty() {
                json!(0) // Python's `sum([])` is the int 0
            } else if figures.iter().any(|f| f.1.is_none()) {
                Json::Null
            } else {
                json!(figures.iter().fold(0.0, |a, f| a + f.1.unwrap_or(0.0)))
            };
            let attention: Vec<Json> = s.attention.iter().map(attention_json).collect();
            sessions.push((
                &s.last_ts,
                json!({
                    "session_id": id, "account": s.account,
                    "project": s.project.as_deref().unwrap_or("(unknown)"), "last_ts": s.last_ts,
                    "tool_calls": s.tool_calls, "tokens_in": sum(0), "tokens_out": sum(1),
                    "cache_creation": sum(2), "cache_read": sum(3), "cost_usd": cost,
                    "attention": attention, "stage": s.stage, "stage_skill": s.stage_skill,
                }),
            ));
        }
        sessions.sort_by(|a, b| b.0.cmp(a.0)); // stable: Python's `sort(reverse=True)` keeps ties in order
        json!({
            "active_window_s": ACTIVE_WINDOW_S,
            "last_id": self.last_id,
            "state_version": self.state_version,
            "open_call_minutes": OPEN_CALL_MINUTES,
            "sessions": sessions.into_iter().map(|(_, s)| s).collect::<Vec<_>>(),
        })
    }

    // -- polling ------------------------------------------------------------------------------------------------------

    /// One step over the store at `store_path`: opened read-only (`store::open_checked`) for this step and closed.
    /// A store that is missing, busy, newer or broken (or whose account rows cannot be read) skips the scan of this
    /// step: no offset is dropped, the first scan is not used up (warm start stays), only the time-based states move.
    pub fn poll_store(&mut self, store_path: &Path, projects_override: Option<&Path>, now: i64) {
        let Ok(conn) = store::open_checked(store_path) else {
            return self.evaluate(now);
        };
        match account_roots(&conn, projects_override) {
            Ok(roots) => self.poll_once(&roots, now, Some(&conn)),
            Err(_) => self.evaluate(now),
        }
    }

    /// One scan and read pass over every root (Python's `poll_once`), then the time-based states move. With a store
    /// whose cursors or labels cannot be read, the scan is skipped (as `poll_store` does for a store it cannot open).
    pub fn poll_once(&mut self, roots: &[Root], now: i64, store: Option<&Connection>) {
        // Each query runs once: a BUSY on a second read cannot turn into an empty map.
        let (known, labels) = match store {
            Some(c) => match (file_offsets(c), account_labels(c)) {
                (Ok(known), Ok(labels)) => (known, labels),
                _ => return self.evaluate(now),
            },
            None => (HashMap::new(), HashMap::new()),
        };
        self.scan(roots, now, &known, &labels);
        self.evaluate(now);
    }

    fn scan(
        &mut self,
        roots: &[Root],
        now: i64,
        known: &HashMap<String, (i64, i64)>,
        labels: &Labels,
    ) {
        let mut seen: HashSet<String> = HashSet::new();
        for root in roots {
            let label = labels
                .get(&root.account)
                .cloned()
                .unwrap_or_else(|| Some(DEFAULT_LABEL.to_owned()));
            for (path, inode, size, mtime) in jsonl_files(&root.projects) {
                seen.insert(path.clone());
                let mut own = self.offsets.get(&path).copied();
                if own.is_none_or(|(i, off)| i != inode || size < off) {
                    let fresh = self.primed && own.is_none_or(|(i, _)| i != inode);
                    let warm = !self.primed && own.is_none();
                    let at = if fresh {
                        0
                    } else {
                        cursor(known.get(&path), inode, size)
                    };
                    own = Some((inode, at));
                    self.offsets.insert(path.clone(), (inode, at));
                    if warm
                        && at > 0
                        && now as f64 / 1e6 - mtime <= FORGET_AFTER_S as f64
                        && self
                            .warm(label.as_deref(), &path, inode, size, at, now)
                            .is_err()
                    {
                        continue;
                    }
                }
                let (_, at) = own.unwrap_or((inode, 0));
                if size <= at {
                    continue;
                }
                let _ = self.follow(label.as_deref(), &path, inode, now);
            }
        }
        self.offsets.retain(|path, _| seen.contains(path));
        self.primed = true;
    }

    /// One line to its item, parsed as soon as it is read: a batch holds items, never raw lines (bounded memory).
    fn parse_one(&mut self, line: &[u8]) -> Option<Item> {
        let projects = &mut self.projects;
        line_item(line, &mut |cwd: &str| {
            projects
                .entry(cwd.to_owned())
                .or_insert_with(|| resolve_project(cwd, Path::new("")))
                .clone()
        })
    }

    /// FR-086: replay the complete lines below the store cursor into session state only (no event, no id).
    fn warm(
        &mut self,
        account: Option<&str>,
        path: &str,
        inode: u64,
        size: u64,
        cursor: u64,
        now: i64,
    ) -> io::Result<()> {
        let result = (|| {
            let (file, meta) = open_regular(Path::new(path)).ok_or(io::ErrorKind::NotFound)?;
            if meta.ino() != inode {
                return Err(io::ErrorKind::NotFound.into());
            }
            let mut reader = BufReader::new(file);
            let mut start = size.saturating_sub(WARM_MAX_BYTES);
            if start > 0 {
                reader.seek(SeekFrom::Start(start - 1))?;
                start += match next_line(&mut reader)? {
                    Next::Line(l) => l.len() as u64,
                    Next::Skipped(n) => n,
                    Next::End => 1, // no complete line follows: nothing below the cursor is replayed
                } - 1;
            }
            reader.seek(SeekFrom::Start(start))?;
            let mut offset = start;
            self.replaying = true;
            let mut done = false;
            while !done {
                let (mut rows, mut batch) = (0, Vec::new());
                while rows < MAX_BATCH_LINES {
                    if offset >= cursor {
                        done = true;
                        break;
                    }
                    match next_line(&mut reader)? {
                        Next::Line(l) => {
                            offset += l.len() as u64;
                            batch.extend(self.parse_one(&l));
                        }
                        Next::Skipped(n) => offset += n,
                        Next::End => {
                            done = true;
                            break;
                        }
                    }
                    rows += 1;
                }
                if rows == 0 {
                    break;
                }
                let items = batch;
                self.publish(account, path, items, now);
            }
            Ok(())
        })();
        self.replaying = false;
        self.state_version += 1;
        result
    }

    /// `_follow`, publish-only: every new complete line from the tail's own offset, in batches of
    /// [`MAX_BATCH_LINES`].
    fn follow(
        &mut self,
        account: Option<&str>,
        path: &str,
        inode: u64,
        now: i64,
    ) -> io::Result<()> {
        let Some((file, meta)) = open_regular(Path::new(path)) else {
            return Ok(()); // gone, a symlink or not a file now: skipped this step
        };
        if meta.ino() != inode {
            return Ok(());
        }
        let mut own = self.offsets.get(path).map_or(0, |o| o.1);
        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::Start(own))?;
        loop {
            let (mut rows, mut lines, mut next) = (0, Vec::new(), own);
            while rows < MAX_BATCH_LINES {
                match next_line(&mut reader)? {
                    Next::Line(l) => {
                        next += l.len() as u64;
                        lines.extend(self.parse_one(&l));
                    }
                    Next::Skipped(n) => next += n,
                    Next::End => break,
                }
                rows += 1;
            }
            if rows == 0 {
                return Ok(());
            }
            let items = lines;
            self.publish(account, path, items, now);
            own = next;
            self.offsets.insert(path.to_owned(), (inode, own));
            if rows < MAX_BATCH_LINES {
                return Ok(());
            }
        }
    }

    // -- state updates ------------------------------------------------------------------------------------------------

    fn publish(&mut self, account: Option<&str>, path: &str, items: Vec<Item>, now: i64) {
        let mut tried = false; // the stream's meta file was looked for in this step (one path per call)
        let sub_agent = Path::new(path)
            .components()
            .any(|c| c == Component::Normal("subagents".as_ref()));
        for item in items {
            let mut appended: Vec<Pending> = Vec::new();
            let session = self.sessions.entry(&item.session_id, || Session {
                account: account.map(str::to_owned),
                project: None,
                last_ts: String::new(),
                tool_calls: 0,
                messages: Ordered::default(),
                streams: Ordered::default(),
                attention: Vec::new(),
                stage: None,
                stage_skill: None,
                stage_ts: String::new(),
            });
            if item.ts > session.last_ts {
                session.last_ts = item.ts.clone();
            }
            if item.project.as_deref().is_some_and(|p| !p.is_empty()) {
                session.project = item.project.clone();
            }
            let stream = session.streams.entry(path, Stream::default);
            stream.sub_agent = sub_agent;
            stream.is_sidechain = item.is_sidechain || sub_agent;
            let agent = agent_of(stream, path, &mut tried, now);
            if let Some(mid) = &item.message_id {
                // A later assistant message: the results of earlier ones were read (or skipped by the ingest).
                stream
                    .open
                    .retain(|_, call| call.message_id.as_deref() == Some(mid));
            }
            for event in &item.events {
                if event.kind == Kind::ToolUse {
                    session.tool_calls += 1;
                    if event.tool.as_deref() == Some("Skill") {
                        set_stage(
                            session,
                            event.label.as_ref(),
                            &item.ts,
                            &mut self.state_version,
                        );
                    }
                }
                let stream = session.streams.get_mut(path).expect("set above");
                let tool = pair(stream, &item, event);
                appended.push((
                    event.kind,
                    tool,
                    event.is_error,
                    event.label.clone().filter(|_| event.kind == Kind::ToolUse),
                    None,
                ));
            }
            if let Some(u) = &item.usage
                && let Some(figs) = count_usage(&self.prices, session, u)
            {
                appended.push((Kind::Usage, None, u.api_error, None, Some(figs)));
            }
            if self.replaying {
                continue; // warm start: state only, no id consumed (FR-086)
            }
            for (kind, tool, is_error, label, figs) in appended {
                self.last_id += 1;
                if self.buffer.len() == BUFFER_SIZE {
                    self.buffer.pop_front();
                }
                self.buffer.push_back(Buffered {
                    id: self.last_id,
                    session_id: item.session_id.clone(),
                    account: account.map(str::to_owned),
                    project: item.project.clone().filter(|p| !p.is_empty()),
                    ts: item.ts.clone(),
                    kind,
                    tool,
                    is_error,
                    is_sidechain: item.is_sidechain,
                    label,
                    agent: agent.clone(),
                    tokens: figs.map(|f| f.0),
                    cost: figs.and_then(|f| f.1),
                });
            }
        }
        if !self.replaying {
            self.evaluate(now);
        }
    }

    /// Drop expired calls and idle sessions, then work out each session's attention states.
    fn evaluate(&mut self, now: i64) {
        let before = |secs: i64| format_stored(now - secs * SECOND);
        let (call_horizon, forget) = (before(OPEN_CALL_MAX_S), before(FORGET_AFTER_S));
        let quiet = before(OPEN_CALL_MINUTES * 60);
        let mut moved = 0;
        self.sessions.retain(|_, session| {
            for stream in session.streams.values_mut() {
                stream.open.retain(|_, call| call.ts >= call_horizon);
            }
            if session.last_ts < forget && !session.has_open_call() {
                return false;
            }
            let states = attention(session, &quiet);
            if states != session.attention {
                session.attention = states;
                moved += 1;
            }
            true
        });
        self.state_version += moved;
    }
}

type Figs = Option<Figures>;
/// An event waiting for its id: kind, tool, `is_error`, label, token figures.
type Pending = (Kind, Option<String>, bool, Option<Label>, Figs);

/// FR-082: `None` on the main thread, else the stream's agent type (`SUB_AGENT` while it is not known). The meta file
/// is looked for at most once per stream per step, until it gives a type or `META_WAIT_S` passed since the first look.
fn agent_of(stream: &mut Stream, path: &str, tried: &mut bool, now: i64) -> Option<String> {
    if !stream.sub_agent {
        return None;
    }
    if stream.agent.is_none() && !*tried {
        *tried = true;
        let since = *stream.meta_since.get_or_insert(now);
        if let Some(agent) = meta_agent_type(path) {
            stream.agent = Some(agent);
        } else if now - since >= META_WAIT_S * SECOND {
            stream.agent = Some(SUB_AGENT.to_owned());
        }
    }
    Some(stream.agent.clone().unwrap_or_else(|| SUB_AGENT.to_owned()))
}

/// FR-083: a mapped Skill call not older than the current one sets the stage; a change moves `state_version`.
fn set_stage(session: &mut Session, skill: Option<&Label>, ts: &str, version: &mut i64) {
    let Some(Label::Name(skill)) = skill else {
        return;
    };
    let bare = skill.rsplit(':').next().unwrap_or(skill);
    if let Some(stage) = stage_of(bare)
        && ts >= session.stage_ts.as_str()
        && (Some(stage), Some(bare)) != (session.stage, session.stage_skill.as_deref())
    {
        session.stage = Some(stage);
        session.stage_skill = Some(bare.to_owned());
        session.stage_ts = ts.to_owned();
        *version += 1;
    }
}

/// `_pair`: open a call or close it by block id; keep the failure streak. Returns the event's tool for the buffer.
fn pair(stream: &mut Stream, item: &Item, event: &ItemEvent) -> Option<String> {
    let Some(block_id) = &event.block_id else {
        return event.tool.clone();
    };
    if event.kind == Kind::ToolUse {
        let tool = event.tool.clone().unwrap_or_default();
        stream.open.set_value(
            block_id,
            Call {
                tool,
                message_id: item.message_id.clone(),
                ts: item.ts.clone(),
            },
        );
        return event.tool.clone();
    }
    let tool = stream.open.pop(block_id).map(|c| c.tool);
    if event.is_error {
        if stream.streak == 0 {
            stream.streak_since = item.ts.clone();
            stream.streak_tool = tool.clone();
        } else if stream.streak_tool != tool {
            stream.streak_tool = None; // mixed tools: the page must not name just the last one
        }
        stream.streak += 1;
    } else {
        stream.streak = 0;
    }
    tool
}

fn attention(session: &Session, quiet_cutoff: &str) -> Vec<Attention> {
    let (mut failing, mut waiting, mut others) = (Vec::new(), Vec::new(), Vec::new());
    for stream in session.streams.values() {
        if stream.streak >= FAILING_STREAK.max(1) {
            failing.push(Attention {
                state: "failing",
                tool: stream.streak_tool.clone(),
                count: stream.streak,
                is_sidechain: stream.is_sidechain,
                since: stream.streak_since.clone(),
            });
        }
        let mut calls: Vec<&Call> = stream.open.values().collect();
        calls.sort_by(|a, b| a.ts.cmp(&b.ts));
        let asking: Vec<&&Call> = calls
            .iter()
            .filter(|c| WAITING_TOOLS.contains(&c.tool.as_str()))
            .collect();
        if let Some(first) = asking.first() {
            waiting.push(Attention {
                state: "waiting",
                tool: Some(first.tool.clone()),
                count: asking.len() as i64,
                is_sidechain: stream.is_sidechain,
                since: first.ts.clone(),
            });
        }
        others.extend(
            calls
                .iter()
                .filter(|c| !WAITING_TOOLS.contains(&c.tool.as_str()))
                .map(|c| (*c, stream.is_sidechain)),
        );
    }
    failing.sort_by(|a, b| a.since.cmp(&b.since));
    waiting.sort_by(|a, b| a.since.cmp(&b.since));
    let mut states = failing;
    states.extend(waiting);
    if !others.is_empty() && session.last_ts.as_str() <= quiet_cutoff {
        let (oldest, is_sidechain) = others
            .iter()
            .fold(None::<&(&Call, bool)>, |best, c| match best {
                Some(b) if b.0.ts <= c.0.ts => Some(b),
                _ => Some(c),
            })
            .expect("not empty");
        states.push(Attention {
            state: "needs_attention",
            tool: Some(oldest.tool.clone()),
            count: others.len() as i64,
            is_sidechain: *is_sidechain,
            since: oldest.ts.clone(),
        });
    }
    states
}

fn attention_json(a: &Attention) -> Json {
    json!({"state": a.state, "tool": a.tool, "count": a.count, "is_sidechain": a.is_sidechain,
           "since": a.since})
}

/// `_count_usage`: a message counts once (its largest figures); growth is an event carrying the difference.
fn count_usage(prices: &PriceTable, session: &mut Session, u: &Usage) -> Option<Figures> {
    let model = u.model.as_deref();
    let [i, o, c, r] = u.counts;
    let Some(&(before, before_cost)) = session.messages.get(&u.message_id) else {
        let cost = prices.cost(model, i, o, c, r);
        session.messages.set_value(&u.message_id, (u.counts, cost));
        return Some((u.counts, cost));
    };
    let largest: [i64; 4] = std::array::from_fn(|k| u.counts[k].max(before[k]));
    if largest == before {
        return None; // a repeat that grew nothing: no event
    }
    let [i, o, c, r] = largest;
    let cost = prices.cost(model, i, o, c, r);
    let delta: [i64; 4] = std::array::from_fn(|k| largest[k].saturating_sub(before[k]));
    let delta_cost = match (cost, before_cost) {
        (Some(a), Some(b)) => Some(a - b),
        _ => None,
    };
    session.messages.set_value(&u.message_id, (largest, cost));
    Some((delta, delta_cost))
}
