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
//! Every text is cut at [`TEXT_MAX_CHARS`] characters (`cut: true`); hidden and control characters are visible escapes
//! (`live::clean_block`, the FR-115 rule); nothing is redacted. One page holds at most [`MAX_ENTRIES`] entries and
//! [`MAX_BYTES`] bytes of text, and a record is never split across pages.

use crate::ingest::json::Value as Raw;
use crate::live::{self, Next, call_label, clean_block, next_line};
use serde_json::{Map, Value, json};
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

pub const TEXT_MAX_CHARS: usize = 16_000;
pub const MAX_ENTRIES: usize = 500;
pub const MAX_BYTES: usize = 2 * 1024 * 1024;
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
    /// `false`: a cap cut the answer, more lies further in the asked direction (after `next`, or before the first entry).
    pub complete: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// A read failed, or a line is over [`live::MAX_LINE_BYTES`].
    Unreadable,
}

/// One change a record makes: nothing here touches earlier entries until it is applied.
enum Op {
    Skip,
    Entry(Value),
    Result {
        id: String,
        text: String,
        is_error: bool,
        cut: bool,
    },
}

impl Op {
    fn weight(&self) -> usize {
        match self {
            Op::Skip => 0,
            Op::Entry(e) => weight(e),
            Op::Result { text, .. } => text.len(),
        }
    }
}

fn weight(entry: &Value) -> usize {
    ["text", "input", "result"]
        .iter()
        .filter_map(|k| entry[k].as_str())
        .map(str::len)
        .sum()
}

/// `raw` cut at [`TEXT_MAX_CHARS`] and made visible: the text and whether the cut removed anything.
fn shown(raw: &str) -> (String, bool) {
    let prefix: String = raw.chars().take(TEXT_MAX_CHARS + 1).collect();
    let (text, complete) = clean_block(&prefix, TEXT_MAX_CHARS);
    (text, !complete)
}

fn text_entry(kind: &str, offset: u64, raw: &str) -> Option<Op> {
    if raw.trim().is_empty() {
        return None;
    }
    let (text, cut) = shown(raw);
    Some(Op::Entry(
        json!({"type": kind, "offset": offset, "text": text, "cut": cut}),
    ))
}

/// The text of a `tool_result`'s `content`: a string, or the `text` blocks of a list (an image is `[image]`).
fn result_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter_map(|b| match b["type"].as_str() {
                Some("text") => b["text"].as_str().map(str::to_string),
                Some("image") => Some("[image]".to_string()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
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

/// FR-115's rendering of an input: the raw `command` of `Bash`, else the pretty JSON of the whole input.
fn input_text(name: &str, input: &Value) -> String {
    match (name, input["command"].as_str()) {
        ("Bash", Some(command)) => command.to_string(),
        _ => serde_json::to_string_pretty(input).unwrap_or_default(),
    }
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
                let (shown_input, input_cut) = shown(&input_text(name, input));
                out.push(Op::Entry(json!({
                    "type": "tool", "offset": offset,
                    "id": block["id"].as_str().unwrap_or(""),
                    "name": clean_block(name, NAME_MAX_CHARS).0,
                    "label": label_of(name, input),
                    "input": shown_input, "result": Value::Null, "is_error": false, "cut": input_cut,
                })));
            }
            Some("tool_result") if user => {
                let (text, cut) = shown(&result_text(&block["content"]));
                out.push(Op::Result {
                    id: block["tool_use_id"].as_str().unwrap_or("").to_string(),
                    text,
                    is_error: block["is_error"] == true,
                    cut,
                });
            }
            _ => {} // thinking, images, anything new: not shown, not an error
        }
    }
    out
}

/// Entries in file order with the pairing of calls and results and the run of skipped records.
#[derive(Default)]
struct Builder {
    entries: VecDeque<Value>,
    /// The absolute index (count of entries ever popped from the front + position) of the front entry.
    base: usize,
    /// The tool entries still waiting for a result: id -> absolute index.
    open: HashMap<String, usize>,
    skipped: Option<(u64, u64)>,
    bytes: usize,
    dropped: bool,
}

impl Builder {
    fn pending(&self) -> usize {
        usize::from(self.skipped.is_some())
    }

    fn flush(&mut self) {
        if let Some((offset, count)) = self.skipped.take() {
            self.entries
                .push_back(json!({"type": "skipped", "offset": offset, "count": count}));
        }
    }

    fn push(&mut self, entry: Value) {
        self.flush();
        self.bytes += weight(&entry);
        if entry["type"] == "tool" && !entry["orphan"].as_bool().unwrap_or(false) {
            if let Some(id) = entry["id"].as_str() {
                self.open
                    .insert(id.to_string(), self.base + self.entries.len());
            }
        }
        self.entries.push_back(entry);
    }

    fn apply(&mut self, offset: u64, op: Op) {
        match op {
            Op::Skip => match &mut self.skipped {
                Some((_, count)) => *count += 1,
                None => self.skipped = Some((offset, 1)),
            },
            Op::Entry(entry) => self.push(entry),
            Op::Result {
                id,
                text,
                is_error,
                cut,
            } => {
                let slot = self
                    .open
                    .remove(&id)
                    .and_then(|abs| abs.checked_sub(self.base))
                    .and_then(|i| self.entries.get_mut(i));
                match slot {
                    Some(entry) => {
                        self.bytes += text.len();
                        entry["result"] = Value::String(text);
                        entry["is_error"] = Value::Bool(is_error);
                        entry["cut"] = Value::Bool(entry["cut"] == true || cut);
                    }
                    None => {
                        let mut orphan = Map::new();
                        for (k, v) in [
                            ("type", json!("tool")),
                            ("offset", json!(offset)),
                            ("id", json!(id)),
                            ("name", Value::Null),
                            ("label", Value::Null),
                            ("input", Value::Null),
                            ("result", json!(text)),
                            ("is_error", json!(is_error)),
                            ("cut", json!(cut)),
                            ("orphan", json!(true)),
                        ] {
                            orphan.insert(k.to_string(), v);
                        }
                        self.push(Value::Object(orphan));
                    }
                }
            }
        }
    }

    /// Drops the oldest records (never half of one, never the newest) until the caps hold.
    fn trim(&mut self) {
        while self.entries.len() > MAX_ENTRIES || self.bytes > MAX_BYTES {
            let front = self.entries[0]["offset"].as_u64();
            if front == self.entries.back().and_then(|e| e["offset"].as_u64()) {
                return;
            }
            while self
                .entries
                .front()
                .is_some_and(|e| e["offset"].as_u64() == front)
            {
                let gone = self.entries.pop_front().unwrap_or(Value::Null);
                self.bytes = self.bytes.saturating_sub(weight(&gone));
                if let Some(id) = gone["id"].as_str() {
                    if self.open.get(id) == Some(&self.base) {
                        self.open.remove(id);
                    }
                }
                self.base += 1;
                self.dropped = true;
            }
        }
    }
}

/// Answers `window` from `reader`, a transcript positioned anywhere (it is rewound). A line over the bound or a failed read
/// is [`Error::Unreadable`]; a last line without its newline is not complete and is left for the next poll.
pub fn parse<R: BufRead + Seek>(reader: &mut R, window: Window) -> Result<Page, Error> {
    let len = reader
        .seek(SeekFrom::End(0))
        .map_err(|_| Error::Unreadable)?;
    let (start, after, to) = match window {
        Window::After(n) if n <= len => (n, true, u64::MAX),
        Window::After(_) => (0, false, u64::MAX), // the file is not the one the page followed: its first view again
        Window::Before(to) => (0, false, to),
    };
    reader
        .seek(SeekFrom::Start(start))
        .map_err(|_| Error::Unreadable)?;
    let mut b = Builder::default();
    let mut at = start;
    let mut complete = true;
    while at < to {
        let line = match next_line(reader).map_err(|_| Error::Unreadable)? {
            Next::Line(line) => line,
            Next::Skipped(_) => return Err(Error::Unreadable),
            Next::End => break,
        };
        let mut todo = ops(at, &line);
        let (mut count, mut bytes) = (0, 0);
        todo.retain(|op| {
            count += 1;
            bytes += op.weight();
            count <= MAX_ENTRIES && bytes <= MAX_BYTES
        });
        if after {
            let more = todo.iter().filter(|op| !matches!(op, Op::Skip)).count();
            let added: usize = todo.iter().map(Op::weight).sum();
            let held = b.entries.len() + b.pending();
            let new_run =
                usize::from(b.pending() == 0 && todo.iter().any(|op| matches!(op, Op::Skip)));
            if held > 0 && (held + more + new_run > MAX_ENTRIES || b.bytes + added > MAX_BYTES) {
                complete = false;
                break; // this record starts the next page
            }
        }
        for op in todo {
            b.apply(at, op);
        }
        at += line.len() as u64;
        if !after {
            b.trim();
        }
    }
    b.flush();
    if !after {
        b.trim();
        complete = !b.dropped;
    }
    Ok(Page {
        entries: b.entries.into(),
        next: at,
        complete,
    })
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
}
