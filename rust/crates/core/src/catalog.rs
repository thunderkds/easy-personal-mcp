//! The provider catalog, ported from `src/easy_personal/connectors/catalog.py` (PRD FR-016): the five entries of
//! `src/easy_personal/catalog/*.yaml`, read by a hand-written parser of Python's restricted line format
//! (`key: value`, `field:` repeats; not YAML, so no YAML crate). The files are embedded copies under
//! `rust/crates/core/catalog/`; `tests/connectors_parity.rs` compares them byte for byte with Python's.
//!
//! The parser accepts what `catalog.parse` accepts and refuses with Python's `CatalogError` texts. Where Python
//! raises something else (a `ValueError` from `shlex`, a `UnicodeDecodeError`) this one returns a `CatalogError` with a
//! fixed text. Three details of Python are copied on purpose: lines split like `str.splitlines()`, `strip()` removes
//! what `str.isspace()` calls whitespace (including U+001C to U+001F), and `command` is split by a port of POSIX
//! `shlex.split`. The one difference: `\d` of the pinned-version rule matches only ASCII digits here (Python's also
//! matches other Unicode decimal digits). The shipped files hold nothing else, and a test names the case.
//!
//! Nothing here reads a keychain, starts a process or touches the network; the catalog holds public names and URLs.

use crate::workspaces::{py_space, py_strip};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub env: String,
    pub label: String,
    pub secret: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub category: String,
    pub provider: String,
    pub claude_ai_connector: bool,
    pub claude_ai_link: Option<String>,
    pub command: Vec<String>,
    pub fields: Vec<Field>,
}

/// A bad catalog entry; the text is Python's (it names the entry id and, for an unknown key, the key).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogError(pub String);

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CatalogError {}

type Result<T> = std::result::Result<T, CatalogError>;

fn fail<T>(text: String) -> Result<T> {
    Err(CatalogError(text))
}

/// The embedded files, in the order Python's `ids()` returns them (sorted by file stem).
const FILES: [(&str, &[u8]); 5] = [
    ("discord", include_bytes!("../catalog/discord.yaml")),
    ("gmail", include_bytes!("../catalog/gmail.yaml")),
    ("sendgrid", include_bytes!("../catalog/sendgrid.yaml")),
    ("slack", include_bytes!("../catalog/slack.yaml")),
    ("teams", include_bytes!("../catalog/teams.yaml")),
];

/// Catalog ids, sorted (Python's `ids()`).
pub fn ids() -> Vec<&'static str> {
    FILES.iter().map(|(id, _)| *id).collect()
}

/// The embedded bytes of one entry, as Python's `PACKAGED / f"{id}.yaml"` file holds them.
pub fn file(id: &str) -> Option<&'static [u8]> {
    FILES.iter().find(|(i, _)| *i == id).map(|(_, b)| *b)
}

/// Every entry in `ids()` order (Python's `[load(i) for i in ids()]`).
pub fn entries() -> Result<Vec<Entry>> {
    FILES
        .iter()
        .map(|(id, bytes)| parse_bytes(id, bytes))
        .collect()
}

/// `parse` of file bytes. Python's `read_text()` crashes on a byte that is not UTF-8; this refuses with a fixed text.
pub fn parse_bytes(entry_id: &str, bytes: &[u8]) -> Result<Entry> {
    match std::str::from_utf8(bytes) {
        Ok(text) => parse(entry_id, text),
        Err(_) => fail(format!("{entry_id}: the catalog file is not valid UTF-8")),
    }
}

const REQUIRED: [&str; 4] = ["category", "provider", "claude_ai_connector", "command"];
const SINGLE: [&str; 5] = [
    "category",
    "provider",
    "claude_ai_connector",
    "command",
    "claude_ai_link",
];

pub fn parse(entry_id: &str, text: &str) -> Result<Entry> {
    if !valid_id(entry_id) {
        return fail("invalid catalog entry name".to_string());
    }
    let mut single: Vec<(&str, String)> = Vec::new();
    let mut fields: Vec<Field> = Vec::new();
    for line in splitlines(text) {
        if py_strip(line).is_empty() || line.trim_start_matches(py_space).starts_with('#') {
            continue;
        }
        let (key, value) = line.split_once(':').unwrap_or((line, ""));
        let (key, value) = (py_strip(key), py_strip(value));
        if key == "field" {
            fields.push(parse_field(entry_id, value)?);
        } else if let Some(name) = SINGLE.iter().find(|k| **k == key)
            && !single.iter().any(|(k, _)| k == name)
        {
            single.push((name, value.to_string()));
        } else {
            return fail(format!("{entry_id}: unknown or repeated key '{key}'"));
        }
    }
    let get = |key: &str| {
        single
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.as_str())
    };
    let missing: Vec<&str> = REQUIRED
        .iter()
        .copied()
        .filter(|k| get(k).is_none_or(str::is_empty))
        .collect();
    if !missing.is_empty() || fields.is_empty() {
        let what = if missing.is_empty() {
            "field".to_string()
        } else {
            missing.join(", ")
        };
        return fail(format!("{entry_id}: missing {what}"));
    }
    let flag = get("claude_ai_connector").unwrap_or("");
    if flag != "yes" && flag != "no" {
        return fail(format!("{entry_id}: claude_ai_connector must be yes or no"));
    }
    let link = get("claude_ai_link");
    if (flag == "yes") != link.is_some() {
        return fail(format!(
            "{entry_id}: claude_ai_link is required with claude_ai_connector: yes, refused with no"
        ));
    }
    if let Some(link) = link
        && !valid_link(link)
    {
        return fail(format!("{entry_id}: claude_ai_link must be an https URL"));
    }
    for (i, field) in fields.iter().enumerate() {
        if fields[..i].iter().any(|f| f.env == field.env) {
            return fail(format!("{entry_id}: duplicate field"));
        }
    }
    let Some(command) = shlex_split(get("command").unwrap_or("")) else {
        // Python: an uncaught `ValueError` ("No closing quotation" / "No escaped character")
        return fail(format!(
            "{entry_id}: command has an unclosed quote or a trailing backslash"
        ));
    };
    check_pinned(entry_id, &command)?;
    Ok(Entry {
        id: entry_id.to_string(),
        category: get("category").unwrap_or("").to_string(),
        provider: get("provider").unwrap_or("").to_string(),
        claude_ai_connector: flag == "yes",
        claude_ai_link: link.map(str::to_string),
        command,
        fields,
    })
}

/// `[a-z0-9][a-z0-9_-]{0,63}` (Python's `_ID`, also the file stem rule).
fn valid_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    (1..=64).contains(&bytes.len())
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes[1..]
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_' || *b == b'-')
}

/// `ENV_NAME | label | secret|plain`: three parts, `ENV_NAME` is `[A-Z_][A-Z0-9_]*`, the label is not empty.
fn parse_field(entry_id: &str, text: &str) -> Result<Field> {
    let parts: Vec<&str> = text.split('|').map(py_strip).collect();
    if let [env, label, kind] = parts[..]
        && valid_env(env)
        && !label.is_empty()
        && (kind == "secret" || kind == "plain")
    {
        return Ok(Field {
            env: env.to_string(),
            label: label.to_string(),
            secret: kind == "secret",
        });
    }
    fail(format!(
        "{entry_id}: bad field line (want ENV_NAME | label | secret|plain)"
    ))
}

fn valid_env(env: &str) -> bool {
    let bytes = env.as_bytes();
    !bytes.is_empty()
        && (bytes[0].is_ascii_uppercase() || bytes[0] == b'_')
        && bytes
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'_')
}

/// `https://[A-Za-z0-9.-]+(/[A-Za-z0-9._~/%-]*)?` as a whole (Python's `_LINK.fullmatch`).
fn valid_link(link: &str) -> bool {
    let Some(rest) = link.strip_prefix("https://") else {
        return false;
    };
    let host_len = rest
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'.' || *b == b'-')
        .count();
    let path = &rest[host_len..];
    host_len > 0
        && (path.is_empty()
            || path.strip_prefix('/').is_some_and(|p| {
                p.bytes().all(|b| {
                    b.is_ascii_alphanumeric()
                        || matches!(b, b'.' | b'_' | b'~' | b'/' | b'%' | b'-')
                })
            }))
}

/// A package runner (`npx`, `uvx`, `pipx run`) must name its package at an exact version.
fn check_pinned(entry_id: &str, command: &[String]) -> Result<()> {
    let rest = match command {
        [first, rest @ ..] if first == "npx" || first == "uvx" => rest,
        [first, second, rest @ ..] if first == "pipx" && second == "run" => rest,
        _ => return Ok(()),
    };
    let package = rest
        .iter()
        .find(|t| !t.starts_with('-'))
        .map_or("", String::as_str);
    if valid_pinned(package) {
        return Ok(());
    }
    fail(format!(
        "{entry_id}: package '{package}' must be pinned (name@1.2.3)"
    ))
}

/// `(@[a-z0-9._-]+/)?[a-z0-9._-]+@\d+\.\d+\.\d+([-+][0-9A-Za-z.-]+)?` as a whole (`\d` is ASCII only here).
fn valid_pinned(package: &str) -> bool {
    let name_char =
        |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-');
    let name_run = |s: &str| s.bytes().all(name_char) && !s.is_empty();
    let rest = match package.strip_prefix('@') {
        Some(scoped) => match scoped.split_once('/') {
            Some((scope, rest)) if name_run(scope) => rest,
            _ => return false,
        },
        None => package,
    };
    let Some((name, version)) = rest.split_once('@') else {
        return false;
    };
    if !name_run(name) {
        return false;
    }
    let mut numbers = version.splitn(3, '.');
    let (Some(major), Some(minor), Some(tail)) = (numbers.next(), numbers.next(), numbers.next())
    else {
        return false;
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let patch_len = tail.bytes().take_while(u8::is_ascii_digit).count();
    let suffix = &tail[patch_len..];
    digits(major)
        && digits(minor)
        && patch_len > 0
        && (suffix.is_empty()
            || suffix.strip_prefix(['-', '+']).is_some_and(|s| {
                !s.is_empty()
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
            }))
}

/// Python's `str.splitlines()`: breaks at `\n \r \r\n \v \f \x1c \x1d \x1e \x85    `, no empty last line.
fn splitlines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let (mut start, mut chars) = (0, text.char_indices().peekable());
    while let Some((i, c)) = chars.next() {
        if matches!(
            c,
            '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{1c}'
                ..='\u{1e}' | '\u{85}' | '\u{2028}' | '\u{2029}'
        ) {
            lines.push(&text[start..i]);
            start = i + c.len_utf8();
            if c == '\r' && chars.peek().is_some_and(|(_, n)| *n == '\n') {
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

/// Python's `shlex.split(s)` (POSIX mode, no comments): whitespace is space, tab, CR, LF; single quotes keep
/// everything; inside double quotes a backslash escapes only `"` and `\`; outside quotes it escapes any character.
/// `None` where Python raises `ValueError` (an unclosed quote, or a backslash at the end).
fn shlex_split(text: &str) -> Option<Vec<String>> {
    #[derive(Clone, Copy)]
    enum State {
        Space,
        Word,
        Quote(char),
        /// A backslash was read; the state to return to (`Word` or `Quote`).
        Escape(Option<char>),
    }
    let mut out = Vec::new();
    let (mut token, mut state) = (String::new(), State::Space);
    for c in text.chars() {
        state = match state {
            State::Space | State::Word => match c {
                ' ' | '\t' | '\r' | '\n' => {
                    if matches!(state, State::Word) {
                        out.push(std::mem::take(&mut token));
                    }
                    State::Space
                }
                '\'' | '"' => State::Quote(c),
                '\\' => State::Escape(None),
                _ => {
                    token.push(c);
                    State::Word
                }
            },
            State::Quote(q) if c == q => State::Word,
            State::Quote('"') if c == '\\' => State::Escape(Some('"')),
            State::Quote(q) => {
                token.push(c);
                State::Quote(q)
            }
            State::Escape(back) => {
                if let Some(q) = back
                    && c != '\\'
                    && c != q
                {
                    token.push('\\');
                }
                token.push(c);
                back.map_or(State::Word, State::Quote)
            }
        };
    }
    match state {
        State::Quote(_) | State::Escape(_) => None,
        State::Word => {
            out.push(token);
            Some(out)
        }
        State::Space => Some(out),
    }
}

#[cfg(test)]
mod tests {
    use super::shlex_split;
    use serde_json::Value;

    /// 3,000 strings over `a b x`, blanks, quotes, backslash, `-`, `@`, `1`, `$`, `#` and `é`, each with the tokens (or
    /// the `ValueError`) of Python's `shlex.split` (`rust/parity/gen_connectors_cases.py`).
    #[test]
    fn shlex_split_equals_pythons_on_every_case() {
        let cases: Value =
            serde_json::from_str(include_str!("../tests/connectors_cases.json")).unwrap();
        let cases = cases["shlex"].as_array().unwrap();
        assert_eq!(cases.len(), 3000);
        for case in cases {
            let text = case["text"].as_str().unwrap();
            let want: Option<Vec<String>> = case["want"]
                .as_array()
                .map(|a| a.iter().map(|t| t.as_str().unwrap().to_string()).collect());
            assert_eq!(shlex_split(text), want, "{text:?}");
        }
    }

    #[test]
    fn shlex_split_hand_cases() {
        let some = |t: &[&str]| Some(t.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(shlex_split(""), some(&[]));
        assert_eq!(shlex_split("  \t\n "), some(&[]));
        assert_eq!(shlex_split("''"), some(&[""]));
        assert_eq!(shlex_split("a '' b"), some(&["a", "", "b"]));
        assert_eq!(shlex_split(r#"a"b c"d"#), some(&["ab cd"]));
        assert_eq!(shlex_split(r#""a\"b\\c\d""#), some(&[r#"a"b\c\d"#]));
        assert_eq!(shlex_split(r"a\ b"), some(&["a b"]));
        assert_eq!(shlex_split("'a\\'"), some(&["a\\"]));
        assert_eq!(shlex_split("'a"), None);
        assert_eq!(shlex_split("\"a\\"), None);
        assert_eq!(shlex_split("a\\"), None);
    }
}
