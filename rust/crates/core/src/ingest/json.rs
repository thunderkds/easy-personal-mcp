//! `json.loads(line)` on bytes, as CPython 3.14 does it (`json/__init__.py`, the C scanner `_json.c`), for one
//! transcript line. `None` means Python raises `ValueError` there (the parser then stores an abstained row).
//!
//! Where this matches Python, and the test that pins it (`tests/ingest_parse_json.rs` unless named):
//! - The bytes are decoded with `detect_encoding` and `'surrogatepass'`: a UTF-8 BOM is dropped, UTF-16/32 lines
//!   (BOM or zero-byte pattern) are decoded, UTF-8 may hold encoded surrogates (`ED A0..BF xx`), any other invalid
//!   byte fails (`decoding_*`). Strings are kept as the WTF-8 bytes of Python's `str` (a lone surrogate stays a
//!   3-byte sequence), so nothing Python can hold is lost (`time` needs a surrogate separator, `corpus-time.json`).
//! - Whitespace is ` \t\n\r` only; `NaN`, `Infinity`, `-Infinity` are numbers (`nan_and_infinity_are_floats`);
//!   a control character in a string fails (strict mode); `😀` joins into one character, a lone `\uD83D`
//!   stays lone; duplicate keys: the last wins (`duplicate_key_last_wins`).
//! - An integer is `Int` when it fits in `i64`, `BigInt` beyond (Python keeps it exact; the store cannot hold it),
//!   and a line fails when one has more than 4300 digits (`int_max_str_digits`, `int_digit_limit`); a fraction or an
//!   exponent makes a `Float` (never rounded into an integer).
//! - Nesting: Python's C scanner overflows its C stack past about 57,900 open containers and raises
//!   `RecursionError`, which crashes the ingest (S06a). Rust differs on purpose: past [`MAX_DEPTH`] the line fails
//!   like invalid JSON. The parser is iterative (no recursion), so no depth overflows the Rust stack
//!   (`hundred_thousand_brackets_abstain`, `python_depth_limit_parses`).

use std::borrow::Cow;

/// The deepest nesting Python parsed in **every** one of 12 runs (parity venv, CPython 3.14.4, `ulimit -s` 8192,
/// `json.loads` of `[`*n + `]`*n bytes called from a generator inside `sqlite3.executemany`, the ingest's call path):
/// 57,890 to 57,945 depending on the run (the C stack limit moves with ASLR), 57,898 to 57,952 when called bare.
/// One more level is an invalid line here; Python either parses it or crashes.
pub const MAX_DEPTH: usize = 57_890;

/// Containers nested deeper than this are checked but not kept: the record dispatch reads nothing below
/// `record.message.content[i].input` (level 5). Keeps memory and the drop of a value shallow.
const KEEP_DEPTH: usize = 5;

/// Python's `int_max_str_digits` default: `int()` of a longer digit string raises `ValueError`.
const MAX_INT_DIGITS: usize = 4300;

/// A decoded JSON value. Strings and keys are the WTF-8 bytes of the Python `str`.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    /// An integer outside `i64` (Python keeps it; `sqlite3` raises `OverflowError` when binding it, S06b).
    BigInt,
    /// A JSON number with a fraction or an exponent, or `NaN` / `Infinity` / `-Infinity`.
    Float,
    Str(Vec<u8>),
    Array(Vec<Value>),
    Object(Vec<(Vec<u8>, Value)>),
    /// A container nested deeper than the parser keeps (it is neither a list nor a dict to the dispatch).
    Deep,
}

static NULL: Value = Value::Null;

impl Value {
    /// `value.get(key)` of a dict (the last duplicate wins); `Null` when missing or when this is not an object,
    /// which is what every caller treats like Python's `None`.
    pub fn get(&self, key: &str) -> &Value {
        match self {
            Value::Object(pairs) => pairs
                .iter()
                .rev()
                .find(|(k, _)| k == key.as_bytes())
                .map_or(&NULL, |(_, v)| v),
            _ => &NULL,
        }
    }

    pub fn as_str(&self) -> Option<&[u8]> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// `value is True`.
    pub fn is_true(&self) -> bool {
        matches!(self, Value::Bool(true))
    }

    pub fn is_dict(&self) -> bool {
        matches!(self, Value::Object(_))
    }
}

/// `json.loads(line)`; `None` where Python raises `ValueError` (or, past [`MAX_DEPTH`], `RecursionError`).
pub fn loads(line: &[u8]) -> Option<Value> {
    let text = decode(line)?;
    Parser { s: &text, i: 0 }.document()
}

// ---- decoding: `s.decode(detect_encoding(s), 'surrogatepass')` ----

#[derive(Clone, Copy)]
enum Wide {
    U16 { big: bool },
    U32 { big: bool },
}

/// `detect_encoding` (`json/__init__.py`), then the decode. A BOM picks the codec and is dropped.
fn decode(b: &[u8]) -> Option<Cow<'_, [u8]>> {
    let (rest, wide) = if let Some(rest) = b.strip_prefix(&[0, 0, 0xFE, 0xFF]) {
        (rest, Some(Wide::U32 { big: true }))
    } else if let Some(rest) = b.strip_prefix(&[0xFF, 0xFE, 0, 0]) {
        (rest, Some(Wide::U32 { big: false }))
    } else if let Some(rest) = b.strip_prefix(&[0xFE, 0xFF]) {
        (rest, Some(Wide::U16 { big: true }))
    } else if let Some(rest) = b.strip_prefix(&[0xFF, 0xFE]) {
        (rest, Some(Wide::U16 { big: false }))
    } else if let Some(rest) = b.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        (rest, None)
    } else if b.len() >= 4 && b[0] == 0 {
        (
            b,
            Some(if b[1] != 0 {
                Wide::U16 { big: true }
            } else {
                Wide::U32 { big: true }
            }),
        )
    } else if b.len() >= 4 && b[1] == 0 {
        (
            b,
            Some(if b[2] != 0 || b[3] != 0 {
                Wide::U16 { big: false }
            } else {
                Wide::U32 { big: false }
            }),
        )
    } else if b.len() == 2 && b[0] == 0 {
        (b, Some(Wide::U16 { big: true }))
    } else if b.len() == 2 && b[1] == 0 {
        (b, Some(Wide::U16 { big: false }))
    } else {
        (b, None)
    };
    match wide {
        Some(w) => wide_decode(rest, w).map(Cow::Owned),
        None => utf8_surrogatepass(rest).then_some(Cow::Borrowed(rest)),
    }
}

/// UTF-16 or UTF-32 with `surrogatepass`: a lone surrogate unit is kept, a valid pair joins, a cut unit fails.
fn wide_decode(b: &[u8], w: Wide) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(b.len());
    match w {
        Wide::U16 { big } => {
            let (pairs, []) = b.as_chunks::<2>() else {
                return None; // an odd byte: "truncated data"
            };
            let units: Vec<u32> = pairs
                .iter()
                .map(|&c| {
                    u32::from(if big {
                        u16::from_be_bytes(c)
                    } else {
                        u16::from_le_bytes(c)
                    })
                })
                .collect();
            let mut i = 0;
            while i < units.len() {
                let u = units[i];
                if (0xD800..0xDC00).contains(&u)
                    && units
                        .get(i + 1)
                        .is_some_and(|n| (0xDC00..0xE000).contains(n))
                {
                    push_cp(
                        &mut out,
                        0x10000 + ((u - 0xD800) << 10) + (units[i + 1] - 0xDC00),
                    );
                    i += 2;
                } else {
                    push_cp(&mut out, u);
                    i += 1;
                }
            }
        }
        Wide::U32 { big } => {
            let (quads, []) = b.as_chunks::<4>() else {
                return None;
            };
            for &c in quads {
                let u = if big {
                    u32::from_be_bytes(c)
                } else {
                    u32::from_le_bytes(c)
                };
                if u > 0x10FFFF {
                    return None;
                }
                push_cp(&mut out, u);
            }
        }
    }
    Some(out)
}

/// Python's strict UTF-8 decoder plus `surrogatepass` (an encoded surrogate `ED A0..BF 80..BF` is accepted).
fn utf8_surrogatepass(b: &[u8]) -> bool {
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c < 0x80 {
            i += 1;
            continue;
        }
        let (n, lo, hi) = match c {
            0xC2..=0xDF => (1, 0x80, 0xBF),
            0xE0 => (2, 0xA0, 0xBF),
            0xE1..=0xEF => (2, 0x80, 0xBF), // 0xED: surrogates allowed (surrogatepass)
            0xF0 => (3, 0x90, 0xBF),
            0xF1..=0xF3 => (3, 0x80, 0xBF),
            0xF4 => (3, 0x80, 0x8F),
            _ => return false,
        };
        let Some(next) = b.get(i + 1..i + 1 + n) else {
            return false;
        };
        if !(lo..=hi).contains(&next[0]) || next[1..].iter().any(|x| !(0x80..=0xBF).contains(x)) {
            return false;
        }
        i += 1 + n;
    }
    true
}

/// Append one code point as WTF-8 (a surrogate as its 3-byte form, like CESU / `surrogatepass`).
fn push_cp(out: &mut Vec<u8>, cp: u32) {
    match cp {
        0..=0x7F => out.push(cp as u8),
        0x80..=0x7FF => out.extend_from_slice(&[0xC0 | (cp >> 6) as u8, 0x80 | (cp & 0x3F) as u8]),
        0x800..=0xFFFF => out.extend_from_slice(&[
            0xE0 | (cp >> 12) as u8,
            0x80 | ((cp >> 6) & 0x3F) as u8,
            0x80 | (cp & 0x3F) as u8,
        ]),
        _ => out.extend_from_slice(&[
            0xF0 | (cp >> 18) as u8,
            0x80 | ((cp >> 12) & 0x3F) as u8,
            0x80 | ((cp >> 6) & 0x3F) as u8,
            0x80 | (cp & 0x3F) as u8,
        ]),
    }
}

/// The code point at byte `i` of WTF-8 text (surrogates included) and the number of bytes it takes. Never panics, on
/// any byte slice: a truncated sequence is clamped to the bytes left, and a stray continuation or invalid lead byte
/// is one byte wide (valid input never meets either). `None` only when `i` is past the end.
pub fn char_at(s: &[u8], i: usize) -> Option<(u32, usize)> {
    let c = *s.get(i)?;
    let (n, init) = match c {
        0..=0x7F => (0, u32::from(c)),
        0xC0..=0xDF => (1, u32::from(c & 0x1F)),
        0xE0..=0xEF => (2, u32::from(c & 0x0F)),
        0xF0..=0xF7 => (3, u32::from(c & 0x07)),
        _ => return Some((u32::from(c), 1)),
    };
    let tail = s.get(i + 1..).unwrap_or(&[]);
    let tail = &tail[..n.min(tail.len())];
    let cp = tail
        .iter()
        .fold(init, |acc, b| (acc << 6) | u32::from(b & 0x3F));
    Some((cp, 1 + tail.len()))
}

/// The code points of WTF-8 text (surrogates included). The input is valid by construction (decoded above).
pub fn code_points(s: &[u8]) -> impl Iterator<Item = u32> + '_ {
    let mut i = 0;
    std::iter::from_fn(move || {
        let (cp, width) = char_at(s, i)?;
        i += width;
        Some(cp)
    })
}

/// Whether WTF-8 text holds a lone surrogate (`ED A0..BF xx`), a character Python's `str` can hold but cannot encode.
/// `0xED` is never a continuation byte, so a plain byte scan cannot misread the middle of another character.
pub fn has_lone_surrogate(s: &[u8]) -> bool {
    s.windows(2).any(|w| w[0] == 0xED && w[1] >= 0xA0)
}

// ---- the scanner ----

enum Frame {
    Array(Vec<Value>, bool),
    Object(Vec<(Vec<u8>, Value)>, Vec<u8>, bool),
}

impl Frame {
    fn keep(&self) -> bool {
        match self {
            Frame::Array(_, k) | Frame::Object(_, _, k) => *k,
        }
    }
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn at(&self, i: usize) -> u8 {
        self.s.get(i).copied().unwrap_or(0)
    }

    fn ws(&mut self) {
        while matches!(self.at(self.i), b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn eat(&mut self, lit: &[u8]) -> bool {
        if self.s[self.i..].starts_with(lit) {
            self.i += lit.len();
            true
        } else {
            false
        }
    }

    /// `decode`: whitespace, one value, whitespace, the end.
    fn document(mut self) -> Option<Value> {
        let mut stack: Vec<Frame> = Vec::new();
        self.ws();
        loop {
            let keep = stack.last().is_none_or(Frame::keep);
            // One value at `i`: a scalar, or the opening of a container (pushed, then its first value).
            let mut value = match self.at(self.i) {
                c @ (b'{' | b'[') => {
                    if stack.len() >= MAX_DEPTH {
                        return None;
                    }
                    self.i += 1;
                    self.ws();
                    let inner = stack.len() < KEEP_DEPTH;
                    let close = if c == b'{' { b'}' } else { b']' };
                    if self.at(self.i) == close {
                        self.i += 1;
                        match (inner, c) {
                            (false, _) => Value::Deep,
                            (true, b'{') => Value::Object(Vec::new()),
                            (true, _) => Value::Array(Vec::new()),
                        }
                    } else {
                        if c == b'{' {
                            let key = self.key(inner)?;
                            stack.push(Frame::Object(Vec::new(), key, inner));
                        } else {
                            stack.push(Frame::Array(Vec::new(), inner));
                        }
                        continue;
                    }
                }
                _ => self.scalar(keep)?,
            };
            // Attach the finished value to its container; close containers that end here.
            loop {
                let Some(frame) = stack.last_mut() else {
                    self.ws();
                    return (self.i == self.s.len()).then_some(value);
                };
                let is_object = matches!(frame, Frame::Object(..));
                match frame {
                    Frame::Array(items, true) => items.push(value),
                    Frame::Object(pairs, key, true) => pairs.push((std::mem::take(key), value)),
                    _ => {}
                }
                self.ws();
                let c = self.at(self.i);
                if c == b',' {
                    self.i += 1;
                    self.ws();
                    if is_object {
                        let inner = frame_keep(&stack);
                        let key = self.key(inner)?;
                        if let Some(Frame::Object(_, k, _)) = stack.last_mut() {
                            *k = key;
                        }
                    }
                    break;
                }
                if c != if is_object { b'}' } else { b']' } {
                    return None;
                }
                self.i += 1;
                value = match stack.pop() {
                    Some(Frame::Array(items, true)) => Value::Array(items),
                    Some(Frame::Object(pairs, _, true)) => Value::Object(pairs),
                    _ => Value::Deep,
                };
            }
        }
    }

    /// `"key"`, whitespace, `:`, whitespace (`_parse_object_unicode`).
    fn key(&mut self, keep: bool) -> Option<Vec<u8>> {
        if self.at(self.i) != b'"' {
            return None;
        }
        let key = self.string(keep)?;
        self.ws();
        if self.at(self.i) != b':' {
            return None;
        }
        self.i += 1;
        self.ws();
        Some(key)
    }

    fn scalar(&mut self, keep: bool) -> Option<Value> {
        if self.i >= self.s.len() {
            return None;
        }
        let c = self.s[self.i];
        match c {
            b'"' => {
                let s = self.string(keep)?;
                Some(if keep { Value::Str(s) } else { Value::Null })
            }
            b'n' if self.eat(b"null") => Some(Value::Null),
            b't' if self.eat(b"true") => Some(Value::Bool(true)),
            b'f' if self.eat(b"false") => Some(Value::Bool(false)),
            b'N' if self.eat(b"NaN") => Some(Value::Float),
            b'I' if self.eat(b"Infinity") => Some(Value::Float),
            b'-' if self.eat(b"-Infinity") => Some(Value::Float),
            _ => self.number(),
        }
    }

    /// `_match_number_unicode`: `-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][-+]?[0-9]+)?`. What it leaves unread makes the
    /// caller fail, as in Python (only whitespace, `,`, `]`, `}` or the end may follow a value).
    fn number(&mut self) -> Option<Value> {
        let start = self.i;
        let neg = self.at(self.i) == b'-';
        if neg {
            self.i += 1;
        }
        match self.at(self.i) {
            b'0' => self.i += 1,
            b'1'..=b'9' => self.digits(),
            _ => return None,
        }
        let int_end = self.i;
        let mut float = false;
        if self.at(self.i) == b'.' && self.at(self.i + 1).is_ascii_digit() {
            self.i += 1;
            self.digits();
            float = true;
        }
        if matches!(self.at(self.i), b'e' | b'E') {
            let mut j = self.i + 1;
            if matches!(self.at(j), b'+' | b'-') {
                j += 1;
            }
            if self.at(j).is_ascii_digit() {
                self.i = j;
                self.digits();
                float = true;
            }
        }
        if float {
            return Some(Value::Float);
        }
        if int_end - start - usize::from(neg) > MAX_INT_DIGITS {
            return None;
        }
        // ASCII digits with an optional sign: valid UTF-8, and `parse` fails only on overflow.
        let text = std::str::from_utf8(&self.s[start..int_end]).ok()?;
        Some(text.parse::<i64>().map_or(Value::BigInt, Value::Int))
    }

    fn digits(&mut self) {
        while self.at(self.i).is_ascii_digit() {
            self.i += 1;
        }
    }

    /// `scanstring_unicode` (strict): `i` is at the opening quote. Returns the WTF-8 bytes (empty when not kept).
    fn string(&mut self, keep: bool) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        self.i += 1;
        loop {
            let run = self.i;
            while self.i < self.s.len() && !matches!(self.s[self.i], b'"' | b'\\' | 0..=0x1F) {
                self.i += 1;
            }
            if keep {
                out.extend_from_slice(&self.s[run..self.i]);
            }
            match self.s.get(self.i) {
                Some(b'"') => {
                    self.i += 1;
                    return Some(out);
                }
                Some(b'\\') => {
                    let c = self.at(self.i + 1);
                    self.i += 2;
                    let unit = match c {
                        b'"' => u32::from(b'"'),
                        b'\\' => u32::from(b'\\'),
                        b'/' => u32::from(b'/'),
                        b'b' => 0x08,
                        b'f' => 0x0C,
                        b'n' => u32::from(b'\n'),
                        b'r' => u32::from(b'\r'),
                        b't' => u32::from(b'\t'),
                        b'u' => {
                            let mut cp = self.hex4(self.i)?;
                            self.i += 4;
                            if (0xD800..0xDC00).contains(&cp)
                                && self.at(self.i) == b'\\'
                                && self.at(self.i + 1) == b'u'
                            {
                                let low = self.hex4(self.i + 2)?;
                                if (0xDC00..0xE000).contains(&low) {
                                    cp = 0x10000 + ((cp - 0xD800) << 10) + (low - 0xDC00);
                                    self.i += 6;
                                }
                            }
                            cp
                        }
                        _ => return None,
                    };
                    if keep {
                        push_cp(&mut out, unit);
                    }
                }
                // A control character (strict mode) or the end of the text.
                _ => return None,
            }
        }
    }

    fn hex4(&self, at: usize) -> Option<u32> {
        let part = self.s.get(at..at + 4)?;
        part.iter()
            .try_fold(0u32, |acc, b| Some(acc * 16 + char::from(*b).to_digit(16)?))
    }
}

fn frame_keep(stack: &[Frame]) -> bool {
    stack.last().is_some_and(Frame::keep)
}
