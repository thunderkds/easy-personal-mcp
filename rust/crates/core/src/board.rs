//! The kanban reader behind `GET /api/board` (T043), ported from `src/easy_personal/core/board.py`.
//!
//! Per project the answer holds only the folder name, the board tasks (id, title, board status), the board kind and the
//! last board pull (file time), the last two only when `.claude/board.json` names a board. Nothing is stored or written.
//! A missing, oversized, non-UTF-8, not-regular or board-less file is `{"status": "no data"}`; rows in an unknown `###`
//! subsection (or without an id) are counted as `abstained`.
//!
//! The parser is hand-written line code with the exact reading of Python's regular expressions (no regex crate: no new
//! dependency is allowed): `\s` and `str.strip()` are Python's whitespace, `\d` is a Unicode decimal digit (the `ND` table
//! is Python 3.14's `unicodedata`, Unicode 16.0), `str.splitlines()` splits on its eleven separators. The searches that a
//! regex would repeat from every start (`[` in a link, `\s|\s`) are linear here, so a 5 MB line of `[` cannot stall a read.
//!
//! Files are read as T045 reads report files: the project folder is opened by its own (owner's) path, every part below it
//! with `O_NOFOLLOW` from the held handle, the kind is judged on the open handle, and the read stops at the cap. Python
//! follows a symlinked `PROJECT_KANBAN.md` or `.claude/board.json`; Rust treats it as no file (decided for you).
//!
//! Privacy (NFR-002): the task ids and titles are the owner's own text, shown as Python shows them. They are never put in
//! an error, a log line or a `Debug` output: no error of this module carries a path or a title.

use crate::kit::file_time;
use crate::metrics::{MetricsError, folder, is_dir};
use crate::report_files::sys;
use crate::scope::{self, Scope, casefold};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::fs::{File, Metadata};
use std::io::Read;

const KANBAN: &str = "PROJECT_KANBAN.md";
const MAX_BYTES: u64 = 5_000_000;
const MAX_TITLE: usize = 300;
const UNKNOWN: &str = "?";
const BLOCKED: &str = "Blocked";

/// First code point of each run of ten Unicode decimal digits (category Nd), Python 3.14 / Unicode 16.0.
#[rustfmt::skip]
const ND: [u32; 76] = [
    0x30, 0x660, 0x6f0, 0x7c0, 0x966, 0x9e6, 0xa66, 0xae6, 0xb66, 0xbe6, 0xc66, 0xce6, 0xd66, 0xde6, 0xe50, 0xed0, 0xf20,
    0x1040, 0x1090, 0x17e0, 0x1810, 0x1946, 0x19d0, 0x1a80, 0x1a90, 0x1b50, 0x1bb0, 0x1c40, 0x1c50, 0xa620, 0xa8d0, 0xa900,
    0xa9d0, 0xa9f0, 0xaa50, 0xabf0, 0xff10, 0x104a0, 0x10d30, 0x10d40, 0x11066, 0x110f0, 0x11136, 0x111d0, 0x112f0, 0x11450,
    0x114d0, 0x11650, 0x116c0, 0x116d0, 0x116da, 0x11730, 0x118e0, 0x11950, 0x11bf0, 0x11c50, 0x11d50, 0x11da0, 0x11f50,
    0x16130, 0x16a60, 0x16ac0, 0x16b50, 0x16d70, 0x1ccf0, 0x1d7ce, 0x1d7d8, 0x1d7e2, 0x1d7ec, 0x1d7f6, 0x1e140, 0x1e2f0,
    0x1e4f0, 0x1e5f1, 0x1e950, 0x1fbf0,
];

/// Python's `\d` for a `str`.
pub(crate) fn is_digit(c: char) -> bool {
    let at = ND.partition_point(|&start| start <= c as u32);
    at > 0 && (c as u32) < ND[at - 1] + 10
}

/// Python's `str.isspace()` (= `\s` for a `str`): White_Space plus the separators `\x1c`..`\x1f`.
pub(crate) fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// Python's `str.strip()`.
pub(crate) fn strip(text: &str) -> &str {
    text.trim_matches(is_space)
}

/// Python's `str.splitlines()`: breaks at `\n`, `\r`, `\r\n`, `\v`, `\f`, `\x1c`-`\x1e`, `\x85`, ` `, ` `.
fn splitlines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let (mut start, mut chars) = (0, text.char_indices().peekable());
    while let Some((at, c)) = chars.next() {
        if matches!(
            c,
            '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{1c}'
                ..='\u{1e}' | '\u{85}' | '\u{2028}' | '\u{2029}'
        ) {
            lines.push(&text[start..at]);
            start = at + c.len_utf8();
            if c == '\r' && chars.peek().is_some_and(|&(_, next)| next == '\n') {
                chars.next();
                start += 1;
            }
        }
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// The first `)` / `]` at or after `from`, remembering the last search so that a run of failed starts costs one scan.
fn next_of(
    c: &[char],
    from: usize,
    ch: char,
    cache: &mut Option<(usize, Option<usize>)>,
) -> Option<usize> {
    if let Some((searched_from, found)) = *cache
        && searched_from <= from
    {
        match found {
            None => return None,
            Some(at) if at >= from => return Some(at),
            Some(_) => {}
        }
    }
    let found = (from..c.len()).find(|&k| c[k] == ch);
    *cache = Some((from, found));
    found
}

/// `_LINK.sub(r"\1", text)`: `[label](target)` becomes `label`.
fn drop_links(text: &str) -> String {
    let c: Vec<char> = text.chars().collect();
    let (mut out, mut i) = (String::new(), 0);
    let (mut bracket, mut paren) = (None, None);
    while i < c.len() {
        if c[i] == '['
            && let Some(close) = next_of(&c, i + 1, ']', &mut bracket)
            && c.get(close + 1) == Some(&'(')
            && let Some(end) = next_of(&c, close + 2, ')', &mut paren)
        {
            out.extend(&c[i + 1..close]);
            i = end + 1;
            continue;
        }
        out.push(c[i]);
        i += 1;
    }
    out
}

/// `_ESCAPE.sub(r"\1", text)`: a backslash before one of ``\`*_{}[]()#+-.!|<>~`` is dropped and the escaped character is
/// kept (and consumed: a match does not overlap the next one, so `\\*` gives `\*`, not `*`).
fn drop_escapes(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\'
            && let Some(&next) = chars.peek()
            && "\\`*_{}[]()#+-.!|<>~".contains(next)
        {
            out.push(next);
            chars.next();
            continue;
        }
        out.push(c);
    }
    out
}

/// A title as plain text: links reduced to their label, backticks dropped, escapes removed (`_plain`).
fn plain(text: &str) -> String {
    let text = drop_escapes(&drop_links(text)).replace('`', "");
    strip(&text).chars().take(MAX_TITLE).collect()
}

/// `[A-Za-z]+-?\d+` from `i`: the end of the greedy match.
fn id_end(c: &[char], mut i: usize) -> Option<usize> {
    let letters = i;
    while i < c.len() && c[i].is_ascii_alphabetic() {
        i += 1;
    }
    if i == letters {
        return None;
    }
    if c.get(i) == Some(&'-') {
        i += 1;
    }
    let digits = i;
    while i < c.len() && is_digit(c[i]) {
        i += 1;
    }
    (i > digits).then_some(i)
}

fn clean_id(cell: &str) -> Option<String> {
    let cell = strip(strip(cell).trim_matches(['*', '`']));
    let c: Vec<char> = cell.chars().collect();
    (id_end(&c, 0) == Some(c.len())).then(|| cell.to_string())
}

/// The text before the first `\s|\s` (`_UNESCAPED_BAR_SPACED.split(rest, 1)[0]`).
fn before_spaced_bar(rest: &str) -> String {
    let c: Vec<char> = rest.chars().collect();
    let at = (0..c.len().saturating_sub(2))
        .find(|&i| is_space(c[i]) && c[i + 1] == '|' && is_space(c[i + 2]));
    c[..at.unwrap_or(c.len())].iter().collect()
}

/// `_bullet`: `(id, title)` of a list line.
fn bullet(line: &str) -> Option<(String, String)> {
    let c: Vec<char> = line.chars().collect();
    let skip = |mut i: usize| {
        while i < c.len() && is_space(c[i]) {
            i += 1;
        }
        i
    };
    let mut i = skip(0);
    if !matches!(c.get(i), Some('-' | '*')) {
        return None;
    }
    i += 1;
    if !c.get(i).is_some_and(|&x| is_space(x)) {
        return None;
    }
    i = skip(i);
    if c.get(i) == Some(&'[')
        && matches!(c.get(i + 1), Some(' ' | 'x' | 'X'))
        && c.get(i + 2) == Some(&']')
        && c.get(i + 3).is_some_and(|&x| is_space(x))
    {
        i = skip(i + 3);
    }
    let id: String;
    if c.get(i) == Some(&'*') && c.get(i + 1) == Some(&'*') {
        let start = i + 2;
        let close = (start..c.len()).find(|&k| c[k] == '*')?;
        if close == start || c.get(close + 1) != Some(&'*') {
            return None;
        }
        id = c[start..close].iter().collect();
        i = close + 2;
    } else {
        let end = id_end(&c, i)?;
        if end < c.len() && !is_space(c[end]) {
            return None;
        }
        id = c[i..end].iter().collect();
        i = end;
    }
    i = skip(i);
    while i < c.len() && matches!(c[i], '—' | '–' | ':' | '-') {
        i += 1;
    }
    i = skip(i);
    let rest: String = c[i..].iter().collect();
    Some((clean_id(&id)?, plain(&before_spaced_bar(&rest))))
}

/// `_table_cells`: the stripped cells of a `|` row, split at a `|` that no backslash precedes.
fn table_cells(line: &str) -> Option<Vec<String>> {
    if !line.trim_start_matches(is_space).starts_with('|') {
        return None;
    }
    let inner: Vec<char> = strip(line).trim_matches('|').chars().collect();
    let (mut cells, mut cell) = (Vec::new(), String::new());
    for (i, &ch) in inner.iter().enumerate() {
        if ch == '|' && (i == 0 || inner[i - 1] != '\\') {
            cells.push(strip(&cell).to_string());
            cell.clear();
        } else {
            cell.push(ch);
        }
    }
    cells.push(strip(&cell).to_string());
    Some(cells)
}

fn table_row(cells: &[String]) -> Option<(String, String)> {
    for i in 0..2 {
        if let Some(id) = cells.get(i).and_then(|cell| clean_id(cell)) {
            return Some((id, cells.get(i + 1).map_or_else(String::new, |t| plain(t))));
        }
    }
    None
}

/// `_SEPARATOR.match`: a table separator row (`|---|:--:|`).
fn is_separator(text: &str) -> bool {
    let c: Vec<char> = text.chars().collect();
    let n = c.len();
    let ws = |i: &mut usize| {
        while *i < n && is_space(c[*i]) {
            *i += 1;
        }
    };
    let cell = |i: &mut usize| {
        if *i < n && c[*i] == ':' {
            *i += 1;
        }
        let dashes = *i;
        while *i < n && c[*i] == '-' {
            *i += 1;
        }
        if *i - dashes < 2 {
            return false;
        }
        if *i < n && c[*i] == ':' {
            *i += 1;
        }
        true
    };
    let mut i = 0;
    if i < n && c[i] == '|' {
        i += 1;
    }
    ws(&mut i);
    if !cell(&mut i) {
        return false;
    }
    ws(&mut i);
    while i < n && c[i] == '|' {
        let mut j = i + 1;
        ws(&mut j);
        if !cell(&mut j) {
            break;
        }
        ws(&mut j);
        i = j;
    }
    if i < n && c[i] == '|' {
        i += 1;
    }
    ws(&mut i);
    i == n
}

fn status_of(name: &str) -> &'static str {
    match casefold(name).as_str() {
        "todo" => "Todo",
        "in progress" => "In Progress",
        "ready for review" => "Ready for Review",
        "done" => "Done",
        "closed (will not do)" => "Closed (will not do)",
        _ => UNKNOWN,
    }
}

/// `_parse`: `{"tasks": [...], "abstained": n}`, or `None` when the text has no `## Board` section.
pub fn parse_text(text: &str) -> Option<Value> {
    let lines = splitlines(text);
    let (mut tasks, mut abstained) = (Vec::new(), 0);
    let (mut section, mut status, mut found): (Option<&str>, Option<&str>, bool) =
        (None, None, false);
    for (i, line) in lines.iter().enumerate() {
        let stripped = strip(line);
        if let Some(name) = stripped.strip_prefix("## ") {
            let name = casefold(strip(name));
            section = match name.as_str() {
                "board" => Some("board"),
                "blocked" => Some("blocked"),
                _ => None,
            };
            found = found || name == "board";
            status = None;
            continue;
        }
        let Some(section) = section else { continue };
        if section == "board"
            && let Some(name) = stripped.strip_prefix("### ")
        {
            status = Some(status_of(strip(name)));
            continue;
        }
        let Some(current) = (if section == "blocked" {
            Some(BLOCKED)
        } else {
            status
        }) else {
            continue;
        };
        let (row, is_row) = if let Some(cells) = table_cells(line) {
            if is_separator(stripped) || lines.get(i + 1).is_some_and(|n| is_separator(strip(n))) {
                continue;
            }
            if cells[0].starts_with("~~") {
                continue;
            }
            (table_row(&cells), cells.iter().any(|c| !c.is_empty()))
        } else {
            let c: Vec<char> = line.chars().collect();
            let lead = c.iter().take_while(|&&x| is_space(x)).count();
            let is_row = matches!(c.get(lead), Some('-' | '*'))
                && c.get(lead + 1).is_some_and(|&x| is_space(x));
            (bullet(line), is_row)
        };
        if !is_row {
            continue;
        }
        match row {
            Some((id, title)) if current != UNKNOWN => {
                tasks.push(json!({"id": id, "title": title, "status": current}));
            }
            _ => abstained += 1,
        }
    }
    found.then(|| json!({"tasks": tasks, "abstained": abstained}))
}

/// Opens `name` below a held directory without following a symlink; `Some` only for a regular file.
pub(crate) fn open_regular(dir: &File, name: &str) -> Option<(File, Metadata)> {
    let file = sys::open_file_at(dir, name).ok()?;
    let meta = file.metadata().ok()?;
    meta.is_file().then_some((file, meta))
}

/// The bytes of an open regular file, or `None` when it is over `cap` (by its size, and by what the read finds).
pub(crate) fn read_capped(file: File, meta: &Metadata, cap: u64) -> Option<Vec<u8>> {
    if meta.len() > cap {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(cap + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= cap).then_some(bytes)
}

/// `_board_kind`: the board named in `.claude/board.json`, at most 40 characters.
fn board_kind(root: &File) -> Option<String> {
    let dir = sys::open_dir_at(root, ".claude").ok()?;
    let (file, meta) = open_regular(&dir, "board.json")?;
    let text = String::from_utf8(read_capped(file, &meta, MAX_BYTES)?).ok()?;
    let data: Value = serde_json::from_str(&text).ok()?;
    let kind = strip(data.as_object()?.get("board")?.as_str()?);
    (!kind.is_empty()).then(|| kind.chars().take(40).collect())
}

/// `read_project`: the board entry of one project folder; any problem with the file is `no data`.
pub fn read_project(root: &str) -> Value {
    let mut entry = json!({
        "project": folder(Some(root)), "status": "no data", "board": null, "last_pull": null,
        "abstained": 0, "tasks": [],
    });
    let Ok(dir) = sys::open_root(root) else {
        return entry;
    };
    let Some((file, meta)) = open_regular(&dir, KANBAN) else {
        return entry;
    };
    let Some(parsed) = read_capped(file, &meta, MAX_BYTES)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .and_then(|text| parse_text(&text))
    else {
        return entry;
    };
    entry["status"] = json!("ok");
    entry["tasks"] = parsed["tasks"].clone();
    entry["abstained"] = parsed["abstained"].clone();
    if let Some(kind) = board_kind(&dir) {
        entry["board"] = json!(kind);
        entry["last_pull"] = meta
            .modified()
            .ok()
            .and_then(file_time)
            .map_or(Value::Null, |(_, text)| json!(text));
    }
    entry
}

/// `_is_worktree`: the path has a `.claude/worktrees` pair of neighbouring parts.
fn is_worktree(path: &str) -> bool {
    let parts: Vec<&str> = path
        .split('/')
        .filter(|p| !p.is_empty() && *p != ".")
        .collect();
    parts
        .windows(2)
        .any(|w| w[0] == ".claude" && w[1] == "worktrees")
}

/// `board`: every project seen in usage (in scope) whose folder exists, with its board tasks; a gone project is left out
/// and counted in `gone`. An unknown workspace or account is [`MetricsError::Invalid`] with the core's fixed text.
pub fn board(conn: &Connection, scope: Scope) -> Result<Value, MetricsError> {
    let sc = scope::resolve(conn, scope)?;
    let stored: Vec<String> = sc
        .scoped_projects(conn)?
        .into_iter()
        .filter(|p| !is_worktree(p))
        .collect();
    let alive: Vec<&String> = stored.iter().filter(|p| is_dir(p)).collect();
    Ok(json!({
        "projects": alive.iter().map(|p| read_project(p)).collect::<Vec<_>>(),
        "gone": stored.len() - alive.len(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitlines_matches_python() {
        assert_eq!(splitlines("a\r\nb\rc\u{2028}d\n"), ["a", "b", "c", "d"]);
        assert_eq!(splitlines("a\n\nb"), ["a", "", "b"]);
        assert!(splitlines("").is_empty());
    }

    #[test]
    fn unicode_digits_are_python_digits() {
        assert!(is_digit('7') && is_digit('\u{663}') && is_digit('\u{ff17}'));
        assert!(!is_digit('\u{b2}') && !is_digit('a') && !is_digit('\u{2167}'));
    }

    /// Every code point Python's `re.match(r"\\d", c)` accepts (dumped by `gen_memgraph.py` into `cases-t053.json`).
    #[test]
    fn the_digit_table_is_pythons_over_every_code_point() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../parity/golden-memgraph/cases-t053.json");
        let golden: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let runs: Vec<(u32, u32)> = golden["digits"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| (r[0].as_u64().unwrap() as u32, r[1].as_u64().unwrap() as u32))
            .collect();
        let mut wrong = Vec::new();
        for point in 0..=0x10FFFF_u32 {
            let Some(c) = char::from_u32(point) else {
                continue;
            };
            let want = runs.iter().any(|&(a, b)| (a..=b).contains(&point));
            if is_digit(c) != want {
                wrong.push(format!("{point:#x}"));
            }
        }
        assert!(wrong.is_empty(), "differs from Python at {wrong:?}");
        assert!(runs.len() > 60, "the dump holds the digit runs");
    }

    #[test]
    fn an_escaped_backslash_keeps_one_backslash() {
        // Python's `_ESCAPE.sub(r"\1", ..)`: the escaped character stays, and a match does not overlap the next
        assert_eq!(drop_escapes("\\\\* hi"), "\\* hi");
        assert_eq!(drop_escapes("\\\\\\\\y"), "\\\\y");
        assert_eq!(drop_escapes("\\*star\\*"), "*star*");
        assert_eq!(drop_escapes("a\\b \\"), "a\\b \\"); // a backslash before a plain character or at the end stays
        assert_eq!(
            parse_text("## Board\n### Todo\n- T1 \\\\* hi\n").unwrap()["tasks"][0]["title"],
            "\\* hi"
        );
    }

    #[test]
    fn a_line_of_open_brackets_is_read_in_one_pass() {
        let line = "[".repeat(2_000_000);
        assert_eq!(plain(&line).chars().count(), MAX_TITLE);
    }
}
