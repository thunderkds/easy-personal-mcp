//! The transcript of one session (T115, FR-123, DDR-0007): a pure parser from the lines of Claude Code's own JSONL file to
//! the entries the Live panel shows. **The fifth exception of NFR-002**: this is the one place message text, tool input
//! and tool output are read, and they are held only while one answer is built. Nothing here stores, logs or caches
//! them; the caller (`transcript_route.rs`) opens the file anew for every request.
//!
//! Entries, in file order, each a JSON object with a `type` and the byte `offset` of the line it came from:
//! - `user` `{text, cut}`: typed text (a string, or a `text` block of a block list);
//! - `reply` `{text, cut}`: an `isMeta` record whose string starts with `Stop hook feedback:` (what the app sent, T110);
//! - `assistant` `{text, cut}`: one `text` block; `thinking` blocks are not shown;
//! - `tool` `{id, name, label, input, result, is_error, cut}`: a `tool_use` block, its `tool_result` (matched by id, even
//!   from a later record) filled in; `result` is `null` until it comes. A result whose call is outside the window
//!   (an `after` poll that starts after the call, or a *Load earlier* page that ends before the result) comes as a
//!   `tool` with the same `id`, `orphan: true`, `name`, `label` and `input` `null`: the page merges by `id`;
//! - `skipped` `{count}`: one entry for a run of other or unknown records (summary, system, attachment, a line that is not
//!   JSON, a sub-agent record, an `isMeta` record that is not a hook reply). Never an error.
//!
//! Every text is cut at [`TEXT_MAX_CHARS`] characters (`cut: true`), while it is built (a 5 MB tool input is never
//! pretty-printed whole); hidden and control characters are visible escapes (`live::clean_block`, the FR-115 rule); nothing
//! is redacted. One page holds at most [`MAX_ENTRIES`] entries, and its JSON body at most [`MAX_BYTES`] bytes (each entry
//! weighs its serialized length plus [`ENTRY_OVERHEAD`]); a record is never split across pages.
//!
//! Reading: `After` reads forward from the offset. `Before` (and the first view) first scans the bytes once for the line
//! starts below the offset (no JSON), then parses lines newest-first until a cap is met, so the cost of the first view is
//! one byte scan plus the newest few hundred lines, not a parse of the whole file.

use crate::ingest::json::Value as Raw;
use crate::live::{self, Next, call_label, clean_block, next_line};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::Path;

pub const TEXT_MAX_CHARS: usize = 16_000;
pub const MAX_ENTRIES: usize = 500;
pub const MAX_BYTES: usize = 2 * 1024 * 1024;
/// What an entry weighs beyond its serialized length: the comma, a `cut` flag that turns `true`.
pub const ENTRY_OVERHEAD: usize = 8;
/// Room kept for the answer's own keys (`entries`, `next`, `start`, `complete`).
const ENVELOPE: usize = 1024;
const BUDGET: usize = MAX_BYTES - ENVELOPE;
/// A `skipped` entry: `{"count":N,"offset":N,"type":"skipped"}` and its overhead.
const SKIP_WEIGHT: usize = 64;
const NAME_MAX_CHARS: usize = 100;
const REPLY_LABEL: &str = "Stop hook feedback:";

/// Which part of the file to answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    /// Records that start at or after the byte offset (the poll), oldest first, up to the caps.
    After(u64),
    /// The newest entries of the records that start before the byte offset (*Load earlier*; `u64::MAX` is the end of the
    /// file, the panel's first view).
    Before(u64),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Page {
    pub entries: Vec<Value>,
    /// The offset to poll from: the end of the last complete line the answer covers.
    pub next: u64,
    /// The `offset` of the first entry (what the page sends as `before` for *Load earlier*), `None` with no entry.
    pub start: Option<u64>,
    /// `false`: a cap cut the answer, more lies further in the asked direction (after `next`, or before the first entry).
    pub complete: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// A read failed, or a line is over [`live::MAX_LINE_BYTES`].
    Unreadable,
}

/// One change a record makes, with the weight of its entry.
enum Op {
    Skip,
    Entry(Value, usize),
}

fn entry(value: Value) -> Op {
    let weight = weight(&value);
    Op::Entry(value, weight)
}

fn weight(entry: &Value) -> usize {
    entry.to_string().len() + ENTRY_OVERHEAD
}

/// Collects at most [`TEXT_MAX_CHARS`] + 1 characters: enough to know whether a cut removes anything.
#[derive(Default)]
struct Cap {
    text: String,
    chars: usize,
}

impl Cap {
    fn full(&self) -> bool {
        self.chars > TEXT_MAX_CHARS
    }

    fn push_str(&mut self, raw: &str) {
        for c in raw.chars() {
            if self.full() {
                return;
            }
            self.text.push(c);
            self.chars += 1;
        }
    }

    /// The text made visible and cut, and whether the cut removed anything.
    fn shown(&self) -> (String, bool) {
        let (text, complete) = clean_block(&self.text, TEXT_MAX_CHARS);
        (text, !complete)
    }
}

fn shown(raw: &str) -> (String, bool) {
    let mut cap = Cap::default();
    cap.push_str(raw);
    cap.shown()
}

/// A writer that keeps its first `max` bytes and then fails, so a huge value is never serialized whole.
struct Limited {
    buf: Vec<u8>,
    max: usize,
}

impl Write for Limited {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        let room = self.max - self.buf.len();
        if room == 0 {
            return Err(io::Error::other("full"));
        }
        let n = data.len().min(room);
        self.buf.extend_from_slice(&data[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn text_entry(kind: &str, offset: u64, raw: &str) -> Option<Op> {
    if raw.trim().is_empty() {
        return None;
    }
    let (text, cut) = shown(raw);
    Some(entry(
        json!({"type": kind, "offset": offset, "text": text, "cut": cut}),
    ))
}

/// The text of a `tool_result`'s `content`: a string, or the `text` blocks of a list (an image is `[image]`), cut as built.
fn result_cap(content: &Value) -> Cap {
    let mut cap = Cap::default();
    match content {
        Value::String(s) => cap.push_str(s),
        Value::Array(blocks) => {
            let mut first = true;
            for block in blocks {
                let part = match block["type"].as_str() {
                    Some("text") => block["text"].as_str(),
                    Some("image") => Some("[image]"),
                    _ => None,
                };
                if let Some(part) = part {
                    if !first {
                        cap.push_str("\n");
                    }
                    first = false;
                    cap.push_str(part);
                }
                if cap.full() {
                    break;
                }
            }
        }
        _ => {}
    }
    cap
}

/// The label the live tail gives a call (DDR-0004), read from the one input field it reads.
fn label_of(name: &str, input: &Value) -> Value {
    let field = match name {
        "Skill" => "skill",
        "Agent" | "Task" => "subagent_type",
        "Bash" => "command",
        _ => "",
    };
    let raw = match input[field].as_str() {
        Some(s) if !field.is_empty() => Raw::Object(vec![(
            field.as_bytes().to_vec(),
            Raw::Str(s.as_bytes().to_vec()),
        )]),
        _ => Raw::Object(Vec::new()),
    };
    call_label(name, &raw).map_or(Value::Null, |l| l.to_json())
}

/// FR-115's rendering of an input, cut as built: the raw `command` of `Bash`, else the pretty JSON of the whole input.
fn input_cap(name: &str, input: &Value) -> Cap {
    let mut cap = Cap::default();
    match (name, input["command"].as_str()) {
        ("Bash", Some(command)) => cap.push_str(command),
        _ => {
            let mut out = Limited {
                buf: Vec::new(),
                max: (TEXT_MAX_CHARS + 1) * 4, // a character is at most 4 bytes: the first N + 1 are whole
            };
            let _ = serde_json::to_writer_pretty(&mut out, input);
            cap.push_str(&String::from_utf8_lossy(&out.buf));
        }
    }
    cap
}

/// The changes one line makes. `offset` is where the line starts.
fn ops(offset: u64, line: &[u8]) -> Vec<Op> {
    let Ok(record) = serde_json::from_slice::<Value>(line) else {
        return vec![Op::Skip];
    };
    let (kind, content) = (record["type"].as_str(), &record["message"]["content"]);
    if record["isSidechain"] == true || !matches!(kind, Some("user" | "assistant")) {
        return vec![Op::Skip];
    }
    let user = kind == Some("user");
    if user && record["isMeta"] == true {
        let reply = content.as_str().and_then(|s| s.strip_prefix(REPLY_LABEL));
        return match reply {
            Some(rest) => text_entry("reply", offset, rest.strip_prefix('\n').unwrap_or(rest))
                .into_iter()
                .collect(),
            None => vec![Op::Skip],
        };
    }
    let text_kind = if user { "user" } else { "assistant" };
    let blocks = match content {
        Value::String(s) => return text_entry(text_kind, offset, s).into_iter().collect(),
        Value::Array(blocks) => blocks,
        _ => return vec![Op::Skip],
    };
    let mut out = Vec::new();
    for block in blocks {
        match block["type"].as_str() {
            Some("text") => {
                out.extend(text_entry(
                    text_kind,
                    offset,
                    block["text"].as_str().unwrap_or(""),
                ));
            }
            Some("tool_use") if !user => {
                let name = block["name"].as_str().unwrap_or("");
                let input = &block["input"];
                let (shown_input, input_cut) = input_cap(name, input).shown();
                out.push(entry(json!({
                    "type": "tool", "offset": offset,
                    "id": block["id"].as_str().unwrap_or(""),
                    "name": clean_block(name, NAME_MAX_CHARS).0,
                    "label": label_of(name, input),
                    "input": shown_input, "result": Value::Null, "is_error": false, "cut": input_cut,
                })));
            }
            Some("tool_result") if user => {
                let (text, cut) = result_cap(&block["content"]).shown();
                out.push(entry(json!({
                    "type": "tool", "offset": offset,
                    "id": block["tool_use_id"].as_str().unwrap_or(""),
                    "name": Value::Null, "label": Value::Null, "input": Value::Null,
                    "result": text, "is_error": block["is_error"] == true, "cut": cut, "orphan": true,
                })));
            }
            _ => {} // thinking, images, anything new: not shown, not an error
        }
    }
    out
}

/// Keeps the ops of one record that fit the caps alone (a record is never split, but one that is over a cap by itself is
/// cut at its end).
fn fit_alone(ops: &mut Vec<Op>) {
    let (mut count, mut bytes) = (0, 0);
    ops.retain(|op| {
        count += 1;
        bytes += match op {
            Op::Skip => SKIP_WEIGHT,
            Op::Entry(_, w) => *w,
        };
        count <= MAX_ENTRIES && bytes <= BUDGET
    });
}

fn is_orphan(entry: &Value) -> bool {
    entry["orphan"] == true
}

fn is_call(entry: &Value) -> bool {
    entry["type"] == "tool" && !is_orphan(entry)
}

/// Puts the result of the `orphan` entry into the `call`; returns the bytes the call grows by.
fn fill(call: &mut Value, orphan: &Value) -> usize {
    call["result"] = orphan["result"].clone();
    call["is_error"] = orphan["is_error"].clone();
    call["cut"] = Value::Bool(call["cut"] == true || orphan["cut"] == true);
    orphan["result"]
        .as_str()
        .map_or(0, |r| Value::String(r.to_string()).to_string().len())
        .saturating_sub(4)
}

fn skipped(offset: u64, count: u64) -> Value {
    json!({"type": "skipped", "offset": offset, "count": count})
}

/// Forward reading: entries in file order, calls and results paired, runs of skipped records merged.
#[derive(Default)]
struct Forward {
    entries: Vec<Value>,
    /// The tool entries still waiting for a result: id -> position.
    open: HashMap<String, usize>,
    skipped: Option<(u64, u64)>,
    bytes: usize,
}

impl Forward {
    fn flush(&mut self) {
        if let Some((offset, count)) = self.skipped.take() {
            self.entries.push(skipped(offset, count));
            self.bytes += SKIP_WEIGHT;
        }
    }

    fn apply(&mut self, offset: u64, op: Op) {
        match op {
            Op::Skip => match &mut self.skipped {
                Some((_, count)) => *count += 1,
                None => self.skipped = Some((offset, 1)),
            },
            Op::Entry(mut entry, w) => {
                let id = entry["id"].as_str().filter(|id| !id.is_empty());
                if is_orphan(&entry)
                    && let Some(i) = id.and_then(|id| self.open.remove(id))
                {
                    self.bytes += fill(&mut self.entries[i], &entry);
                    return; // paired: no entry of its own, a run of skipped records goes on
                }
                self.flush();
                if let (true, Some(id)) = (is_call(&entry), id) {
                    self.open.insert(id.to_string(), self.entries.len());
                }
                self.bytes += w;
                self.entries.push(std::mem::take(&mut entry));
            }
        }
    }
}

/// `After`: reads forward from `start` until the end or a cap (the record that would pass a cap starts the next page).
fn read_after<R: BufRead + Seek>(reader: &mut R, start: u64) -> Result<Page, Error> {
    reader
        .seek(SeekFrom::Start(start))
        .map_err(|_| Error::Unreadable)?;
    let mut f = Forward::default();
    let (mut at, mut complete) = (start, true);
    loop {
        let line = match next_line(reader).map_err(|_| Error::Unreadable)? {
            Next::Line(line) => line,
            Next::Skipped(_) => return Err(Error::Unreadable),
            Next::End => break,
        };
        let mut todo = ops(at, &line);
        fit_alone(&mut todo);
        let pending = usize::from(f.skipped.is_some());
        let held = f.entries.len() + pending;
        let new_run = usize::from(pending == 0 && todo.iter().any(|op| matches!(op, Op::Skip)));
        let more = todo.iter().filter(|op| matches!(op, Op::Entry(..))).count();
        let added: usize = todo
            .iter()
            .map(|op| match op {
                Op::Skip => 0,
                Op::Entry(_, w) => *w,
            })
            .sum::<usize>()
            + new_run * SKIP_WEIGHT;
        let bytes = f.bytes + pending * SKIP_WEIGHT;
        if held > 0 && (held + more + new_run > MAX_ENTRIES || bytes + added > BUDGET) {
            complete = false;
            break; // this record starts the next page
        }
        for op in todo {
            f.apply(at, op);
        }
        at += line.len() as u64;
    }
    f.flush();
    let start = f.entries.first().and_then(|e| e["offset"].as_u64());
    Ok(Page {
        entries: f.entries,
        next: at,
        start,
        complete,
    })
}

/// The start offsets of the complete lines that start below `to` (a byte scan, no JSON), and the end of the last of them.
/// A complete line over the bound fails like [`next_line`] would.
fn line_starts<R: BufRead + Seek>(reader: &mut R, to: u64) -> Result<(Vec<u64>, u64), Error> {
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|_| Error::Unreadable)?;
    let (mut starts, mut line_start, mut base) = (Vec::new(), 0u64, 0u64);
    if to == 0 {
        return Ok((starts, 0));
    }
    loop {
        let chunk = reader.fill_buf().map_err(|_| Error::Unreadable)?;
        if chunk.is_empty() {
            return Ok((starts, line_start));
        }
        let n = chunk.len();
        let mut from = 0;
        while let Some(i) = chunk[from..].iter().position(|&b| b == b'\n') {
            let next = base + (from + i) as u64 + 1;
            if next - line_start > live::MAX_LINE_BYTES as u64 + 1 {
                return Err(Error::Unreadable);
            }
            starts.push(line_start);
            line_start = next;
            from += i + 1;
            if line_start >= to {
                return Ok((starts, line_start));
            }
        }
        base += n as u64;
        reader.consume(n);
    }
}

/// Newest-first reading: items from the newest record back; a result seen before its call waits as an `orphan` entry and is
/// folded into the call when the call comes.
#[derive(Default)]
struct Reverse {
    /// Newest first; a record's own entries are in reverse too.
    items: Vec<Item>,
    live: usize,
    bytes: usize,
    /// Orphan results waiting for their call: id -> position in `items`.
    waiting: HashMap<String, usize>,
}

enum Item {
    Entry(Value),
    Skip(u64, u64),
    /// An orphan that found its call.
    Gone,
}

impl Reverse {
    fn last_is_skip(&self) -> bool {
        matches!(
            self.items.iter().rev().find(|i| !matches!(i, Item::Gone)),
            Some(Item::Skip(..))
        )
    }

    /// Would the record's ops pass a cap? (Always false for the first record.)
    fn full(&self, ops: &[Op]) -> bool {
        if self.items.is_empty() {
            return false;
        }
        let new_run =
            usize::from(ops.iter().any(|op| matches!(op, Op::Skip)) && !self.last_is_skip());
        let more = ops.iter().filter(|op| matches!(op, Op::Entry(..))).count();
        let added: usize = ops
            .iter()
            .map(|op| match op {
                Op::Skip => 0,
                Op::Entry(_, w) => *w,
            })
            .sum::<usize>()
            + new_run * SKIP_WEIGHT;
        self.live + more + new_run > MAX_ENTRIES || self.bytes + added > BUDGET
    }

    fn add(&mut self, offset: u64, ops: Vec<Op>) {
        for op in ops.into_iter().rev() {
            match op {
                Op::Skip => {
                    if self.last_is_skip() {
                        if let Some(Item::Skip(o, c)) = self
                            .items
                            .iter_mut()
                            .rev()
                            .find(|i| !matches!(i, Item::Gone))
                        {
                            (*o, *c) = (offset, *c + 1);
                        }
                    } else {
                        self.items.push(Item::Skip(offset, 1));
                        self.live += 1;
                        self.bytes += SKIP_WEIGHT;
                    }
                }
                Op::Entry(mut entry, mut w) => {
                    let id = entry["id"]
                        .as_str()
                        .filter(|id| !id.is_empty())
                        .map(str::to_string);
                    if is_call(&entry)
                        && let Some(at) = id.as_ref().and_then(|id| self.waiting.remove(id))
                        && let Item::Entry(orphan) =
                            std::mem::replace(&mut self.items[at], Item::Gone)
                    {
                        self.live -= 1;
                        self.bytes -= weight(&orphan);
                        w += fill(&mut entry, &orphan);
                    } else if let (true, Some(id)) = (is_orphan(&entry), id) {
                        self.waiting.insert(id, self.items.len());
                    }
                    self.items.push(Item::Entry(entry));
                    self.live += 1;
                    self.bytes += w;
                }
            }
        }
    }

    /// The entries in file order, adjacent runs of skipped records merged (a paired orphan between two runs is gone).
    fn finish(self) -> Vec<Value> {
        let mut out = Vec::with_capacity(self.live);
        let mut run: Option<(u64, u64)> = None;
        for item in self.items.into_iter().rev() {
            match item {
                Item::Gone => {}
                Item::Skip(o, c) => run = Some(run.map_or((o, c), |(o0, c0)| (o0, c0 + c))),
                Item::Entry(e) => {
                    if let Some((o, c)) = run.take() {
                        out.push(skipped(o, c));
                    }
                    out.push(e);
                }
            }
        }
        if let Some((o, c)) = run {
            out.push(skipped(o, c));
        }
        out
    }
}

/// `Before` and the first view: a byte scan for the line starts below `to`, then a newest-first parse until a cap.
fn read_before<R: BufRead + Seek>(reader: &mut R, to: u64) -> Result<Page, Error> {
    let (starts, next) = line_starts(reader, to)?;
    let mut rev = Reverse::default();
    let mut complete = true;
    for &at in starts.iter().rev() {
        reader
            .seek(SeekFrom::Start(at))
            .map_err(|_| Error::Unreadable)?;
        let Next::Line(line) = next_line(reader).map_err(|_| Error::Unreadable)? else {
            return Err(Error::Unreadable); // the file shrank or changed under the read
        };
        let mut todo = ops(at, &line);
        fit_alone(&mut todo);
        if rev.full(&todo) {
            complete = false;
            break; // this record and the older ones stay for *Load earlier*
        }
        rev.add(at, todo);
    }
    let entries = rev.finish();
    let start = entries.first().and_then(|e| e["offset"].as_u64());
    Ok(Page {
        entries,
        next,
        start,
        complete,
    })
}

/// Answers `window` from `reader`, a transcript positioned anywhere (it is rewound). A line over the bound or a failed read
/// is [`Error::Unreadable`]; a last line without its newline is not complete and is left for the next poll. An `After`
/// that starts inside a line reads that line's tail as one skipped record.
pub fn parse<R: BufRead + Seek>(reader: &mut R, window: Window) -> Result<Page, Error> {
    let len = reader
        .seek(SeekFrom::End(0))
        .map_err(|_| Error::Unreadable)?;
    match window {
        Window::After(n) if n <= len => read_after(reader, n),
        // the file is not the one the page followed: its first view again
        Window::After(_) => read_before(reader, u64::MAX),
        Window::Before(to) => read_before(reader, to),
    }
}

/// Opens the file with the live tail's rules (no symlink at the end, no blocking, a regular file judged on the open
/// handle) and answers `window`. A file that is gone, unreadable, not regular or has an oversized line is
/// [`Error::Unreadable`]. The file is read anew by every call: nothing is kept between calls.
pub fn read(path: &Path, window: Window) -> Result<Page, Error> {
    let (file, _) = live::open_regular(path).ok_or(Error::Unreadable)?;
    parse(&mut BufReader::new(file), window)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn page(text: &str, window: Window) -> Page {
        parse(&mut Cursor::new(text.as_bytes().to_vec()), window).unwrap()
    }

    fn types(p: &Page) -> Vec<String> {
        p.entries
            .iter()
            .map(|e| e["type"].as_str().unwrap().to_string())
            .collect()
    }

    fn user(text: &str) -> String {
        format!(
            "{}\n",
            json!({"type":"user","message":{"role":"user","content":text}})
        )
    }

    fn assistant_text(text: &str) -> String {
        format!(
            "{}\n",
            json!({"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":text}]}})
        )
    }

    fn call(id: &str, name: &str, input: Value) -> String {
        format!(
            "{}\n",
            json!({"type":"assistant","message":{"content":[{"type":"tool_use","id":id,"name":name,"input":input}]}})
        )
    }

    fn result(id: &str, content: Value, is_error: bool) -> String {
        format!(
            "{}\n",
            json!({"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":id,"content":content,"is_error":is_error}]}})
        )
    }

    #[test]
    fn a_result_is_matched_to_its_call_by_id_even_from_a_later_record() {
        let text = [
            call("a", "Read", json!({"file_path": "/x"})),
            call("b", "Bash", json!({"command": "cd /w && cargo test"})),
            assistant_text("between"),
            result("b", json!("ok"), false),
            result(
                "a",
                json!([{"type":"text","text":"one"},{"type":"image"}]),
                true,
            ),
        ]
        .concat();
        let p = page(&text, Window::Before(u64::MAX));
        assert_eq!(types(&p), ["tool", "tool", "assistant"]);
        let [a, b, _] = &p.entries[..] else { panic!() };
        assert_eq!(
            (
                a["name"].clone(),
                a["result"].clone(),
                a["is_error"].clone()
            ),
            (json!("Read"), json!("one\n[image]"), json!(true))
        );
        assert_eq!(
            (b["input"].clone(), b["label"].clone(), b["result"].clone()),
            (json!("cd /w && cargo test"), json!("cargo"), json!("ok"))
        );
        assert!(p.complete);
    }

    #[test]
    fn user_reply_assistant_thinking_and_the_other_records() {
        let text = [
            user("typed"),
            format!("{}\n", json!({"type":"user","message":{"content":[{"type":"text","text":"blocks"}]}})),
            format!("{}\n", json!({"type":"user","isMeta":true,"message":{"content":"Stop hook feedback:\nfrom the app"}})),
            format!("{}\n", json!({"type":"user","isMeta":true,"message":{"content":"other meta"}})),
            format!("{}\n", json!({"type":"assistant","message":{"content":[{"type":"thinking","thinking":"secret thoughts"},{"type":"text","text":"said"}]}})),
            "{\"type\":\"summary\"}\n".into(),
            "{\"type\":\"never-seen\",\"x\":1}\n".into(),
            "not json\n".into(),
            format!("{}\n", json!({"type":"assistant","isSidechain":true,"message":{"content":[{"type":"text","text":"sub"}]}})),
            assistant_text("last"),
        ]
        .concat();
        let p = page(&text, Window::Before(u64::MAX));
        assert_eq!(
            types(&p),
            [
                "user",
                "user",
                "reply",
                "skipped",
                "assistant",
                "skipped",
                "assistant"
            ]
        );
        assert_eq!(p.entries[2]["text"], "from the app");
        assert_eq!(
            (p.entries[3]["count"].clone(), p.entries[5]["count"].clone()),
            (json!(1), json!(4))
        );
        assert!(!text_of(&p).contains("secret thoughts") && !text_of(&p).contains("sub"));
    }

    fn text_of(p: &Page) -> String {
        serde_json::to_string(&p.entries).unwrap()
    }

    #[test]
    fn a_text_is_cut_at_16000_characters_and_hidden_characters_are_escaped() {
        let p = page(&user(&"a".repeat(16_000)), Window::Before(u64::MAX));
        assert_eq!(p.entries[0]["cut"], false);
        let long = format!("{}\u{202e}\r", "é".repeat(16_001));
        let p = page(&user(&long), Window::Before(u64::MAX));
        assert_eq!(
            (
                p.entries[0]["cut"].clone(),
                p.entries[0]["text"].as_str().unwrap().chars().count()
            ),
            (json!(true), 16_000)
        );
        let p = page(&user("a\u{202e}b\u{7}\tc\nd"), Window::Before(u64::MAX));
        assert_eq!(p.entries[0]["text"], "a\\u{202e}b\\u{7}\tc\nd");
        let p = page(
            &call("x", "Edit", json!({"p": "\u{200b}"})),
            Window::Before(u64::MAX),
        );
        assert!(
            p.entries[0]["input"]
                .as_str()
                .unwrap()
                .contains("\\u{200b}")
        );
    }

    #[test]
    fn after_returns_the_records_from_the_offset_and_a_partial_last_line_waits() {
        let (one, two) = (user("one"), user("two"));
        let text = format!("{one}{two}{{\"type\":\"user\"");
        let p = page(&text, Window::After(0));
        assert_eq!(
            (types(&p), p.next, p.complete),
            (
                vec!["user".into(), "user".into()],
                (one.len() + two.len()) as u64,
                true
            )
        );
        let p = page(&text, Window::After(one.len() as u64));
        assert_eq!(
            (
                p.entries.len(),
                p.entries[0]["text"].clone(),
                p.entries[0]["offset"].clone()
            ),
            (1, json!("two"), json!(one.len()))
        );
        let p = page(&text, Window::After((one.len() + two.len()) as u64));
        assert_eq!(
            (p.entries.len(), p.next),
            (0, (one.len() + two.len()) as u64)
        );
        // an offset past the end is a file that is not the one followed: the first view again
        assert_eq!(page(&text, Window::After(9_999)).entries.len(), 2);
    }

    #[test]
    fn a_result_after_the_offset_of_its_call_comes_as_an_orphan_with_the_same_id() {
        let first = call("t1", "Read", json!({}));
        let text = format!("{first}{}", result("t1", json!("late"), false));
        let p = page(&text, Window::After(first.len() as u64));
        let e = &p.entries[0];
        assert_eq!(
            (
                e["id"].clone(),
                e["orphan"].clone(),
                e["name"].clone(),
                e["result"].clone()
            ),
            (json!("t1"), json!(true), Value::Null, json!("late"))
        );
    }

    #[test]
    fn a_page_holds_at_most_500_entries_and_before_pages_back_without_splitting_a_record() {
        let text: String = (0..1200).map(|i| user(&format!("m{i}"))).collect();
        let p = page(&text, Window::After(0));
        assert_eq!((p.entries.len(), p.complete), (500, false));
        let again = page(&text, Window::After(p.next));
        assert_eq!(again.entries[0]["text"], "m500");
        let tail = page(&text, Window::Before(u64::MAX));
        assert_eq!((tail.entries.len(), tail.complete), (500, false));
        assert_eq!(tail.entries[0]["text"], "m700");
        let earlier = page(
            &text,
            Window::Before(tail.entries[0]["offset"].as_u64().unwrap()),
        );
        assert_eq!(
            (
                earlier.entries.len(),
                earlier.entries[0]["text"].clone(),
                earlier.entries[499]["text"].clone()
            ),
            (500, json!("m200"), json!("m699"))
        );
        let first = page(
            &text,
            Window::Before(earlier.entries[0]["offset"].as_u64().unwrap()),
        );
        assert_eq!((first.entries.len(), first.complete), (200, true));
    }

    #[test]
    fn a_page_holds_at_most_2_mb_of_text() {
        let big = "x".repeat(15_000);
        let text: String = (0..300).map(|_| user(&big)).collect();
        let p = page(&text, Window::After(0));
        assert!(p.entries.len() < 300 && !p.complete);
        assert!(p.entries.iter().map(weight).sum::<usize>() <= MAX_BYTES);
        let tail = page(&text, Window::Before(u64::MAX));
        assert!(tail.entries.iter().map(weight).sum::<usize>() <= MAX_BYTES && !tail.complete);
        assert_eq!(tail.entries.last().unwrap()["offset"], p_last_offset(&text));
    }

    fn p_last_offset(text: &str) -> Value {
        json!(text.len() - user(&"x".repeat(15_000)).len())
    }

    #[test]
    fn a_line_over_the_bound_is_unreadable() {
        let mut huge = Cursor::new(vec![b'x'; live::MAX_LINE_BYTES + 10]);
        huge.get_mut().push(b'\n');
        assert_eq!(parse(&mut huge, Window::After(0)), Err(Error::Unreadable));
    }

    fn body(p: &Page) -> usize {
        json!({"entries": p.entries, "next": p.next, "start": p.start, "complete": p.complete})
            .to_string()
            .len()
    }

    #[test]
    fn a_skipped_run_counts_as_an_entry_so_after_never_passes_500() {
        // 499 records, 3 other records (one skipped entry), then 5 more: the run takes the 500th place
        let text: String = (0..499).map(|i| user(&format!("m{i}"))).collect::<String>()
            + "{\"type\":\"summary\"}\n{\"type\":\"x\"}\nnot json\n"
            + &(0..5).map(|i| user(&format!("t{i}"))).collect::<String>();
        let p = page(&text, Window::After(0));
        assert_eq!((p.entries.len(), p.complete), (500, false));
        assert_eq!(
            (
                types(&p).last().unwrap().as_str(),
                p.entries[499]["count"].clone()
            ),
            ("skipped", json!(3))
        );
        let rest = page(&text, Window::After(p.next));
        assert_eq!(
            (
                rest.entries.len(),
                rest.entries[0]["text"].clone(),
                rest.complete
            ),
            (5, json!("t0"), true)
        );
    }

    #[test]
    fn a_result_far_after_its_call_is_an_orphan_when_the_window_starts_after_the_call() {
        let text = [call("t0", "Read", json!({}))].concat()
            + &(0..598).map(|i| user(&format!("m{i}"))).collect::<String>()
            + &result("t0", json!("late"), false);
        let p = page(&text, Window::Before(u64::MAX));
        assert_eq!((p.entries.len(), p.complete), (500, false));
        let last = p.entries.last().unwrap();
        assert_eq!(
            (
                last["id"].clone(),
                last["orphan"].clone(),
                last["result"].clone()
            ),
            (json!("t0"), json!(true), json!("late"))
        );
        assert!(
            p.entries
                .iter()
                .all(|e| e["id"] != "t0" || e["orphan"] == true),
            "the call is outside the window"
        );
        // the earlier page holds the call, without the result: the page merges the two by id
        let earlier = page(&text, Window::Before(p.start.unwrap()));
        assert_eq!(earlier.entries[0]["id"], "t0");
        assert_eq!(earlier.entries[0]["result"], Value::Null);
        assert!(earlier.complete);
    }

    #[test]
    fn a_window_inside_a_line_gives_one_skipped_entry_not_an_error() {
        let p = page(&user("one"), Window::After(1));
        assert_eq!(types(&p), ["skipped"]);
        assert_eq!(p.entries[0]["count"], 1);
        let two = user("one") + &user("two");
        let p = page(&two, Window::After(1));
        assert_eq!(
            (types(&p), p.entries[1]["text"].clone()),
            (vec!["skipped".to_string(), "user".into()], json!("two"))
        );
    }

    #[test]
    fn start_is_the_offset_of_the_first_entry_or_null() {
        let (one, two) = (user("one"), user("two"));
        let text = format!("{one}{two}");
        let p = page(&text, Window::Before(u64::MAX));
        assert_eq!(p.start, Some(0));
        let p = page(&text, Window::After(one.len() as u64));
        assert_eq!(p.start, Some(one.len() as u64));
        let p = page(&text, Window::Before(0));
        assert_eq!(
            (p.entries.len(), p.start, p.complete, p.next),
            (0, None, true, 0)
        );
        assert_eq!(page("", Window::After(0)).start, None);
    }

    #[test]
    fn the_answer_body_stays_within_2_mb_in_both_directions() {
        // quotes double when serialized: the weight is the serialized length, not the text length
        let big = "\"".repeat(15_000);
        let text: String = (0..300).map(|_| user(&big)).collect();
        for window in [Window::After(0), Window::Before(u64::MAX)] {
            let p = page(&text, window);
            assert!(!p.complete && p.entries.len() > 10, "{window:?}");
            assert!(body(&p) <= MAX_BYTES, "{window:?}: {} bytes", body(&p));
        }
        // a record of many blocks alone is cut at its end, not split in the middle of a block and not over the cap
        let blocks: Vec<Value> = (0..200)
            .map(|_| json!({"type":"text","text":big}))
            .collect();
        let one = format!(
            "{}\n",
            json!({"type":"assistant","message":{"content":blocks}})
        );
        let p = page(&one, Window::Before(u64::MAX));
        assert!(body(&p) <= MAX_BYTES && !p.entries.is_empty());
    }

    #[test]
    fn a_huge_input_and_a_huge_result_are_cut_while_they_are_built() {
        let huge = "é".repeat(5_000_000);
        let text = call("a", "Write", json!({"content": huge}))
            + &call("b", "Bash", json!({"command": huge}))
            + &result(
                "a",
                json!([{"type":"text","text":huge},{"type":"text","text":huge}]),
                false,
            );
        let p = page(&text, Window::Before(u64::MAX));
        let (a, b) = (&p.entries[0], &p.entries[1]);
        for e in [a, b] {
            assert_eq!(e["cut"], true);
        }
        assert!(a["input"].as_str().unwrap().chars().count() <= TEXT_MAX_CHARS);
        assert_eq!(b["input"].as_str().unwrap().chars().count(), TEXT_MAX_CHARS);
        assert_eq!(
            a["result"].as_str().unwrap().chars().count(),
            TEXT_MAX_CHARS
        );
        assert!(body(&p) < 100_000);
        assert_eq!(page(&text, Window::After(0)), p);
    }

    #[test]
    fn the_first_view_of_a_long_file_equals_the_tail_of_a_forward_read() {
        let text: String = (0..3000)
            .map(|i| match i % 7 {
                0 => call(&format!("c{i}"), "Read", json!({"i": i})),
                1 => result(&format!("c{}", i - 1), json!(format!("r{i}")), false),
                2 => "{\"type\":\"system\"}\n".to_string(),
                3 => "{\"type\":\"system\"}\n".to_string(),
                _ => user(&format!("m{i}")),
            })
            .collect();
        let first = page(&text, Window::Before(u64::MAX));
        assert!(!first.complete && first.entries.len() <= MAX_ENTRIES);
        // the same records read forward from the first entry's record give the same entries
        let forward = page(&text, Window::After(first.start.unwrap()));
        assert_eq!(
            forward.entries[..first.entries.len().min(forward.entries.len())],
            first.entries[..first.entries.len().min(forward.entries.len())]
        );
    }
}
