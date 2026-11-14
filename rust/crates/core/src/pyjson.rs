//! Text written as Python writes it (T050): `json.dumps` of a string or of string pairs with the default
//! `ensure_ascii=True` and separators, and `shlex.quote`. Used for the keychain item, the `claude mcp add-json` JSON
//! and the `claude` commands shown when `claude` is not on `PATH`, so each byte equals Python's.

/// `json.dumps(str)`: `"` and `\` escaped, `\n \r \t \b \f` short, every other character outside `' '..='~'` as
/// `\uXXXX` (a surrogate pair above U+FFFF).
pub fn py_str(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            ' '..='~' => out.push(c),
            _ => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out.push('"');
    out
}

/// `json.dumps(dict)` of string pairs, in the given order.
pub fn py_object(pairs: &[(String, String)]) -> String {
    let parts: Vec<String> = pairs
        .iter()
        .map(|(k, v)| format!("{}: {}", py_str(k), py_str(v)))
        .collect();
    format!("{{{}}}", parts.join(", "))
}

/// `json.dumps(list_of_str)`.
pub fn py_list(items: &[String]) -> String {
    let parts: Vec<String> = items.iter().map(|s| py_str(s)).collect();
    format!("[{}]", parts.join(", "))
}

/// `shlex.quote`: as it is when every character is ASCII `[A-Za-z0-9_@%+=:,./-]`, `''` when empty, else single-quoted
/// with each `'` as `'"'"'`.
pub fn sh_quote(text: &str) -> String {
    if text.is_empty() {
        return "''".into();
    }
    let safe = |c: char| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c);
    if text.chars().all(safe) {
        return text.into();
    }
    format!("'{}'", text.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Expected values are Python's `json.dumps` / `shlex.quote` for the same input.
    #[test]
    fn equals_python() {
        assert_eq!(py_str("a\"b\\c"), r#""a\"b\\c""#);
        assert_eq!(py_str("\u{7f}\u{1}"), "\"\\u007f\\u0001\"");
        assert_eq!(py_str("\u{e9}\u{1f600}"), "\"\\u00e9\\ud83d\\ude00\"");
        assert_eq!(py_str("\n\r\t\u{8}\u{c}"), r#""\n\r\t\b\f""#);
        assert_eq!(py_list(&["exec".into(), "c1".into()]), r#"["exec", "c1"]"#);
        assert_eq!(py_list(&[]), "[]");
        assert_eq!(py_object(&[]), "{}");
        for (raw, want) in [
            ("", "''"),
            ("slack", "slack"),
            ("/a/.claude-work", "/a/.claude-work"),
            ("a b", "'a b'"),
            ("it's", "'it'\"'\"'s'"),
            ("\u{e9}", "'\u{e9}'"),
            ("$HOME", "'$HOME'"),
            ("a@b%c+d=e:f,g", "a@b%c+d=e:f,g"),
        ] {
            assert_eq!(sh_quote(raw), want, "{raw:?}");
        }
    }
}
