//! `_LABEL` and `_WORD` of `claude_code.py`, over the WTF-8 bytes of a Python `str` (both are ASCII classes, so a
//! byte test is exact). Proved against `corpus-label.json`.

use super::json::Value;

/// `_label(value)`: the string when it matches `^[A-Za-z0-9_.:@/+-]{1,200}$`, else `None`. Python's `$` also matches
/// just before one trailing `\n`, so `"feature\n"` is a label (stored with its newline) and `"feature\n\n"` is not.
pub fn label(value: &Value) -> Option<String> {
    let s = value.as_str()?;
    is_label(s).then(|| String::from_utf8_lossy(s).into_owned())
}

/// The `_LABEL` match on a string.
pub fn is_label(s: &[u8]) -> bool {
    let body = s.strip_suffix(b"\n").unwrap_or(s);
    (1..=200).contains(&body.len())
        && body
            .iter()
            .all(|&b| b.is_ascii_alphanumeric() || b"_.:@/+-".contains(&b))
}

/// `_WORD.fullmatch(s)`: `[A-Za-z0-9][A-Za-z0-9_.+-]{0,63}` (no `$` quirk: `fullmatch`).
pub fn is_word(s: &[u8]) -> bool {
    matches!(s.first(), Some(b) if b.is_ascii_alphanumeric())
        && s.len() <= 64
        && s.iter()
            .all(|&b| b.is_ascii_alphanumeric() || b"_.+-".contains(&b))
}
