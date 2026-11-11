//! `secretlike.SECRET.search(text)` (`src/easy_personal/core/secretlike.py`), without a regex crate:
//!
//! ```text
//! \bsk-[A-Za-z0-9] | \bgh[pousr]_[A-Za-z0-9] | \bgithub_pat_ | \bxox[A-Za-z]?[-_]?[A-Za-z0-9]{4,}
//! | \b(?:AKIA|ASIA)[A-Z0-9]{4,} | -----BEGIN | [A-Za-z0-9]{32,}
//! ```
//!
//! Every branch is ASCII; only `\b` looks at a non-ASCII character (the one before a key prefix). Python's `\b` on a
//! `str` uses Unicode `\w` (letters, numbers, `_`); this uses `char::is_alphanumeric`, which agrees on every code
//! point but 6,167 marks (`Other_Alphabetic`, e.g. U+0345) that Rust counts as letters and Python does not: before
//! such a mark Python finds a boundary, this does not. Unreachable from the parser: the transcript reader only checks
//! a Bash first word that already matched the ASCII `_WORD` (`claude_code._bash_word`). Proved against the `secret`
//! entries of `corpus-label.json` (ASCII and non-ASCII neighbours, surrogates).

use super::json::code_points;

/// `SECRET.search(s) is not None` on the WTF-8 bytes of a Python `str`.
pub fn is_secret(s: &[u8]) -> bool {
    if s.windows(10).any(|w| w == b"-----BEGIN") {
        return true;
    }
    let mut run = 0;
    for &b in s {
        run = if b.is_ascii_alphanumeric() {
            run + 1
        } else {
            0
        };
        if run >= 32 {
            return true;
        }
    }
    (0..s.len()).any(|i| boundary_before(s, i) && prefixed(&s[i..]))
}

/// A `\b`-guarded branch matching at the start of `t`.
fn prefixed(t: &[u8]) -> bool {
    let alnum = |i: usize| t.get(i).is_some_and(u8::is_ascii_alphanumeric);
    let upper_or_digit = |i: usize| {
        t.get(i)
            .is_some_and(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    };
    if t.starts_with(b"sk-") {
        return alnum(3);
    }
    if t.starts_with(b"gh")
        && t.get(2).is_some_and(|b| b"pousr".contains(b))
        && t.get(3) == Some(&b'_')
    {
        return alnum(4);
    }
    if t.starts_with(b"github_pat_") {
        return true;
    }
    if t.starts_with(b"AKIA") || t.starts_with(b"ASIA") {
        return (4..8).all(upper_or_digit);
    }
    if t.starts_with(b"xox") {
        // `[A-Za-z]?[-_]?[A-Za-z0-9]{4,}`: try each way the two optional parts can be taken.
        let letter = usize::from(t.get(3).is_some_and(u8::is_ascii_alphabetic));
        return (0..=letter).any(|l| {
            let sep = usize::from(t.get(3 + l).is_some_and(|b| *b == b'-' || *b == b'_'));
            (0..=sep).any(|s| (3 + l + s..3 + l + s + 4).all(alnum))
        });
    }
    false
}

/// `\b` before `s[i]`, which starts a key prefix (an ASCII letter, a word character): a boundary when nothing or a
/// non-word character comes before it.
fn boundary_before(s: &[u8], i: usize) -> bool {
    if i == 0 {
        return true;
    }
    let prev = s[i - 1];
    if prev.is_ascii() {
        return !(prev.is_ascii_alphanumeric() || prev == b'_');
    }
    // `i` must start a character for a prefix to match there (prefixes are ASCII), so the previous character ends
    // at `i - 1`: step back over continuation bytes to its first byte.
    let mut start = i - 1;
    while start > 0 && s[start] & 0xC0 == 0x80 {
        start -= 1;
    }
    let cp = code_points(&s[start..i]).next().unwrap_or(0);
    !char::from_u32(cp).is_some_and(|c| c == '_' || c.is_alphanumeric())
}
