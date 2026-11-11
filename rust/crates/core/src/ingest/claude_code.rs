//! The `claude_code` source's parse step (`src/easy_personal/sources/claude_code.py`): `read_complete_lines` +
//! `_file_events` + `parse_record`, bytes in, [`Line`] values out. No file, folder, git or store access: the caller
//! (T035) opens the file, resolves `project_path` (here the record's raw `cwd`) and saves the lines with
//! [`crate::writer::Store::save_file`], which also back-fills a session's entrypoint.
//!
//! Only the FR-002 fields are read; message text, thinking, tool inputs and outputs, `toolUseResult`, attachments,
//! prompts, titles and `totalCostUSD` never reach an [`Event`]: every string kept passes `_LABEL`, a Bash first word
//! or a WebFetch host (`signals.rs`), or is the `cwd`.
//!
//! Rust differs from Python only where Python crashes (`TASK_REVIEW_T026.md`, findings): nesting past
//! [`super::json::MAX_DEPTH`] (S06a), an integer beyond `i64` in a usage field (S06b), a `type` that is a list or an
//! object (S06d), a timestamp whose UTC value leaves years 1-9999 (`time.rs`), and a `cwd` holding a lone surrogate
//! (`UnicodeEncodeError` binding `project_path`). Each is an abstained or timestamp-less row here, and the lines after
//! it are read.

use super::json::{self, Value};
use super::label::label;
use super::signals::signals;
use super::time::utc_text;
use crate::writer::{Event, Line};
use std::io::{self, BufRead};

/// `METADATA_TYPES`: known lines without `version`/`cwd`; only type, version, time and session are kept.
const METADATA_TYPES: [&str; 19] = [
    "agent-name",
    "custom-title",
    "last-prompt",
    "atis-latch",
    "queue-operation",
    "cost-state",
    "file-history-snapshot",
    "file-history-delta",
    "artifact-comment-monitor",
    "artifact-autoreact-ledger",
    "frame-link",
    "api-request",
    "api-request-blob",
    "api-request-shape",
    "tools-used",
    "ai-title",
    "mode",
    "permission-mode",
    "bridge-session",
];
/// `VERSIONED_TYPES`.
const VERSIONED_TYPES: [&str; 4] = ["user", "assistant", "attachment", "system"];

/// Every complete line of `bytes` from `start_offset` (a byte offset into `bytes`, e.g. the stored offset): Python's
/// `(line_offset, next_offset, event or None)` per line. A last line without `\n` is still being written: it is not
/// returned and no offset moves into it. A start past the end yields nothing (Python's `seek` + `readline`).
pub fn read_lines(bytes: &[u8], start_offset: usize) -> Vec<Line> {
    let Some(rest) = bytes.get(start_offset..) else {
        return Vec::new();
    };
    let mut offset = start_offset as i64;
    let mut out = Vec::new();
    for line in rest.split_inclusive(|&b| b == b'\n') {
        if line.last() != Some(&b'\n') {
            break;
        }
        out.push(parse_line(line, offset));
        offset += line.len() as i64;
    }
    out
}

/// The streaming form of [`read_lines`]: reads one line at a time from `reader` (already positioned at
/// `start_offset`), so memory is bounded by the longest line, not the file. Yields `Err` on a read error and stops.
pub struct LineReader<R> {
    reader: R,
    offset: i64,
    buf: Vec<u8>,
    done: bool,
}

impl<R: BufRead> LineReader<R> {
    pub fn new(reader: R, start_offset: i64) -> Self {
        LineReader {
            reader,
            offset: start_offset,
            buf: Vec::new(),
            done: false,
        }
    }
}

impl<R: BufRead> Iterator for LineReader<R> {
    type Item = io::Result<Line>;

    fn next(&mut self) -> Option<io::Result<Line>> {
        if self.done {
            return None;
        }
        self.buf.clear();
        if let Err(e) = self.reader.read_until(b'\n', &mut self.buf) {
            self.done = true;
            return Some(Err(e));
        }
        if self.buf.last() != Some(&b'\n') {
            self.done = true; // end of file, or a partial last line: not consumed
            return None;
        }
        let line = parse_line(&self.buf, self.offset);
        self.offset += self.buf.len() as i64;
        Some(Ok(line))
    }
}

/// `_file_events` for one complete line (with its `\n`): blank -> no event; else `json.loads` (an error is `None`)
/// and `parse_record`.
fn parse_line(line: &[u8], line_offset: i64) -> Line {
    // `line.strip()` on bytes: ASCII whitespace, including \x0b and \x0c.
    let blank = line
        .iter()
        .all(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C));
    let event = if blank {
        None
    } else {
        Some(match json::loads(line) {
            Some(record) => parse_record(&record),
            None => abstain(&Value::Null),
        })
    };
    Line {
        line_offset,
        next_offset: line_offset + line.len() as i64,
        event,
    }
}

/// `_abstain(record)`: abstained, with type, version, time and session when the record is a dict.
fn abstain(record: &Value) -> Event {
    Event {
        abstained: true,
        record_type: label(record.get("type")),
        version: label(record.get("version")),
        ts_utc: utc(record.get("timestamp")),
        session_id: label(record.get("sessionId")),
        ..Event::default()
    }
}

fn utc(v: &Value) -> Option<String> {
    v.as_str().and_then(utc_text)
}

/// The four `usage` fields with `type(x) is int`: a JSON integer, never a bool or a float. `None` for anything else,
/// and for an integer beyond `i64` (Python raises `OverflowError` binding it and the ingest crashes, S06b; here the
/// record abstains like any other bad usage value).
fn usage_ints(usage: &Value) -> Option<[i64; 4]> {
    let field = |name| match usage.get(name) {
        Value::Int(n) => Some(*n),
        _ => None,
    };
    Some([
        field("input_tokens")?,
        field("output_tokens")?,
        field("cache_creation_input_tokens")?,
        field("cache_read_input_tokens")?,
    ])
}

/// `_tool_errors(message)`: failed `tool_result` blocks (`is_error is True`); `None` when there is no such block.
/// Only `type` and `is_error` of a block are read.
fn tool_errors(message: &Value) -> Option<i64> {
    let Value::Array(content) = message.get("content") else {
        return None;
    };
    let results: Vec<&Value> = content
        .iter()
        .filter(|b| b.is_dict() && b.get("type").as_str() == Some(b"tool_result"))
        .collect();
    (!results.is_empty()).then(|| {
        results
            .iter()
            .filter(|b| b.get("is_error").is_true())
            .count() as i64
    })
}

/// `parse_record(record, project_of)` with `project_of` = identity: one decoded line to its event.
pub fn parse_record(record: &Value) -> Event {
    if !record.is_dict() {
        return abstain(record);
    }
    // `rtype in METADATA_TYPES`: only a string can be in the set (a list or a dict makes Python raise `TypeError`,
    // S06d; here it falls through to `_abstain`, whose `_label` gives NULL).
    let rtype = record.get("type").as_str();
    let is = |names: &[&str]| rtype.is_some_and(|t| names.iter().any(|n| n.as_bytes() == t));
    if is(&METADATA_TYPES) {
        return Event {
            record_type: rtype.map(|t| String::from_utf8_lossy(t).into_owned()),
            version: label(record.get("version")),
            ts_utc: utc(record.get("timestamp")),
            session_id: label(record.get("sessionId")),
            ..Event::default()
        };
    }
    if !is(&VERSIONED_TYPES) {
        return abstain(record);
    }
    let rtype = rtype.map(|t| String::from_utf8_lossy(t).into_owned());
    let ts_utc = utc(record.get("timestamp"));
    let session_id = label(record.get("sessionId"));
    let cwd = record.get("cwd");
    // `cwd` absent or a string Python can store. ⚠ decided for you: a lone surrogate (`"\udc80"`) is a `str` Python
    // cannot encode, so binding it as `project_path` raises `UnicodeEncodeError` and the ingest crashes; here the
    // record abstains like a non-string `cwd` (it would otherwise be stored with U+FFFD, silently changed data).
    let cwd_storable = match cwd {
        Value::Null => true,
        Value::Str(s) => !json::has_lone_surrogate(s),
        _ => false,
    };
    if ts_utc.is_none() || session_id.is_none() || !cwd_storable {
        return abstain(record);
    }
    let mut event = Event {
        record_type: rtype.clone(),
        version: label(record.get("version")),
        ts_utc,
        session_id,
        // `project_of(cwd) if cwd else None`: the raw cwd; T035 resolves it through git.
        project_path: cwd
            .as_str()
            .filter(|c| !c.is_empty())
            .map(|c| String::from_utf8_lossy(c).into_owned()),
        git_branch: label(record.get("gitBranch")),
        is_sidechain: Some(record.get("isSidechain").is_true()),
        entrypoint: label(record.get("entrypoint")),
        ..Event::default()
    };
    match rtype.as_deref() {
        Some("user") => {
            event.tool_error_count = tool_errors(record.get("message"));
            return event;
        }
        Some("assistant") => {}
        _ => return event,
    }
    let message = record.get("message");
    if !message.is_dict() {
        return abstain(record);
    }
    let usage = message.get("usage");
    let message_id = label(message.get("id"));
    if !matches!(usage, Value::Null) {
        // Usage is deduped on message.id; without one it cannot be counted correctly.
        let Some([input, output, creation, read]) = message_id.as_ref().and(usage_ints(usage))
        else {
            return abstain(record);
        };
        event.input_tokens = Some(input);
        event.output_tokens = Some(output);
        event.cache_creation_input_tokens = Some(creation);
        event.cache_read_input_tokens = Some(read);
    }
    let blocks: &[Value] = match message.get("content") {
        Value::Array(items) => items,
        _ => &[],
    };
    let tools: Vec<String> = blocks
        .iter()
        .filter(|b| b.is_dict() && b.get("type").as_str() == Some(b"tool_use"))
        .filter_map(|b| label(b.get("name")))
        .collect();
    event.model = label(message.get("model"));
    event.message_id = message_id;
    event.is_api_error = Some(record.get("isApiErrorMessage").is_true());
    event.tool_names = (!tools.is_empty()).then(|| tools.join(","));
    event.signals = signals(blocks);
    event
}
