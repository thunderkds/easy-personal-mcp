//! The connector-suggestion signals of `claude_code.py` (`_signals`, `_bash_word`, `_host`): only a Bash command's
//! first word (its program name, never a secret-like one) and a WebFetch URL's host. Nothing else of a tool input is
//! read. Proved against the `bash` and `host` entries of `corpus-label.json`.

use super::json::{Value, char_at, code_points};
use super::label::is_word;
use super::secret::is_secret;
use crate::writer::Signal;

/// The non-ASCII characters whose NFKC form holds one of `/?#@:` (`urlsplit`'s `_checknetloc` refuses a netloc with
/// one). Python 3.14 / Unicode 16.0; a test compares it with the `nfkc_netloc_forbidden` table of the corpus.
pub const NFKC_NETLOC_FORBIDDEN: [u32; 19] = [
    0x2047, 0x2048, 0x2049, 0x2100, 0x2101, 0x2105, 0x2106, 0x2A74, 0xFE13, 0xFE16, 0xFE55, 0xFE56,
    0xFE5F, 0xFE6B, 0xFF03, 0xFF0F, 0xFF1A, 0xFF1F, 0xFF20,
];

/// The one non-ASCII character `str.lower` maps to ASCII: KELVIN SIGN -> `k` (corpus table `lower_to_ascii`).
pub const LOWER_TO_ASCII: [(u32, u8); 1] = [(0x212A, b'k')];

/// `_signals(blocks)`: `("bash", word)` / `("domain", host)` per `tool_use` block, in order, repeats kept (the store
/// de-duplicates them).
pub fn signals(blocks: &[Value]) -> Vec<Signal> {
    let mut found = Vec::new();
    for b in blocks {
        if !b.is_dict() || b.get("type").as_str() != Some(b"tool_use") {
            continue;
        }
        let input = b.get("input");
        let name = b.get("name").as_str();
        let (kind, value) = if name == Some(b"Bash") {
            ("bash", input.get("command").as_str().and_then(bash_word))
        } else if name == Some(b"WebFetch") {
            ("domain", input.get("url").as_str().and_then(host))
        } else {
            continue;
        };
        if let Some(value) = value {
            found.push(Signal {
                kind: kind.to_owned(),
                value,
            });
        }
    }
    found
}

/// `str.split()` whitespace: Unicode `White_Space` plus U+001C..U+001F (Python counts those, Rust does not).
pub fn is_py_space(c: u32) -> bool {
    (0x1C..=0x1F).contains(&c) || char::from_u32(c).is_some_and(char::is_whitespace)
}

/// `_bash_word(command)`: the first whitespace-separated word, after its last `/`, if it matches `_WORD` and is not
/// secret-like.
pub fn bash_word(command: &[u8]) -> Option<String> {
    // Byte offsets of the first word: skip leading whitespace, stop at the next. `char_at` gives each width with its
    // code point, so no byte slice can index out of range.
    let mut start = None;
    let mut end = command.len();
    let mut pos = 0;
    while let Some((cp, width)) = char_at(command, pos) {
        match (start, is_py_space(cp)) {
            (None, false) => start = Some(pos),
            (Some(_), true) => {
                end = pos;
                break;
            }
            _ => {}
        }
        pos += width;
    }
    let word = &command[start?..end];
    let word = word.rsplit(|&b| b == b'/').next().unwrap_or(word);
    (is_word(word) && !is_secret(word)).then(|| String::from_utf8_lossy(word).into_owned())
}

/// `_host(url)`: `urlsplit(url)` (`urllib/parse.py`, Python 3.14), then `hostname`, `rstrip(".")` and `_HOST`;
/// `None` for any `ValueError` of `urlsplit`, a scheme other than http(s), or a host `_HOST` refuses.
pub fn host(url: &[u8]) -> Option<String> {
    // `_urlsplit`: lstrip C0 controls and space, drop every tab, CR and LF.
    let first = url.iter().position(|&b| b > b' ').unwrap_or(url.len());
    let url: Vec<u8> = url[first..]
        .iter()
        .copied()
        .filter(|b| !matches!(b, b'\t' | b'\r' | b'\n'))
        .collect();
    // The scheme: text before the first `:`, starting with an ASCII letter, all of `scheme_chars`.
    let colon = url.iter().position(|&b| b == b':')?;
    let scheme = &url[..colon];
    if colon == 0
        || !url[0].is_ascii_alphabetic()
        || !scheme
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"+-.".contains(b))
        || !(scheme.eq_ignore_ascii_case(b"http") || scheme.eq_ignore_ascii_case(b"https"))
    {
        return None;
    }
    let rest = &url[colon + 1..];
    let netloc = match rest.strip_prefix(b"//") {
        Some(after) => {
            &after[..after
                .iter()
                .position(|b| b"/?#".contains(b))
                .unwrap_or(after.len())]
        }
        None => b"",
    };
    // `_hostinfo`: after the last `@`; inside `[...]` when there is a `[`, else before the first `:`.
    let hostinfo = netloc.rsplit(|&b| b == b'@').next().unwrap_or(netloc);
    let bracket = hostinfo.iter().position(|&b| b == b'[');
    let hostname = match bracket {
        Some(open) => {
            let inner = &hostinfo[open + 1..];
            &inner[..inner.iter().position(|&b| b == b']').unwrap_or(inner.len())]
        }
        None => {
            &hostinfo[..hostinfo
                .iter()
                .position(|&b| b == b':')
                .unwrap_or(hostinfo.len())]
        }
    };
    let (has_open, has_close) = (netloc.contains(&b'['), netloc.contains(&b']'));
    if has_open != has_close {
        return None; // "Invalid IPv6 URL"
    }
    if has_open {
        // `_check_bracketed_netloc`: nothing before `[`, only `:port` after `]`; then `_check_bracketed_host`. An IP
        // address in brackets always holds a `:` (IPv6) or is refused (IPv4), and `_HOST` refuses a `:`, so only an
        // IPvFuture `v<hex>.<anything but a newline>` can become a host.
        if let Some(open) = bracket {
            let after = &hostinfo[open + 1..];
            let port = after
                .iter()
                .position(|&b| b == b']')
                .map_or(&b""[..], |c| &after[c + 1..]);
            if open > 0 || (!port.is_empty() && port[0] != b':') {
                return None;
            }
        }
        let future = hostname.strip_prefix(b"v").and_then(|h| {
            let dot = h.iter().position(|&b| b == b'.')?;
            (dot > 0
                && h[..dot].iter().all(u8::is_ascii_hexdigit)
                && dot + 1 < h.len()
                && !h[dot + 1..].contains(&b'\n'))
            .then_some(())
        });
        future?;
    }
    // `_checknetloc`: a non-ASCII netloc whose NFKC form gains one of `/?#@:` is refused.
    if !netloc.is_ascii() && code_points(netloc).any(|c| NFKC_NETLOC_FORBIDDEN.contains(&c)) {
        return None;
    }
    // `hostname.lower()` (a non-ASCII result never passes `_HOST`), then `rstrip(".")`.
    let mut lower = Vec::with_capacity(hostname.len());
    for cp in code_points(hostname) {
        match u8::try_from(cp) {
            Ok(b) if b.is_ascii() => lower.push(b.to_ascii_lowercase()),
            _ => lower.push(LOWER_TO_ASCII.iter().find(|(c, _)| *c == cp)?.1),
        }
    }
    while lower.last() == Some(&b'.') {
        lower.pop();
    }
    is_host(&lower).then(|| String::from_utf8_lossy(&lower).into_owned())
}

/// `_HOST.fullmatch`: two or more dot-separated labels of 1-64 `[a-z0-9-]`, not starting or ending with `-`.
fn is_host(h: &[u8]) -> bool {
    let mut labels = 0;
    for part in h.split(|&b| b == b'.') {
        let ok = (1..=64).contains(&part.len())
            && part
                .iter()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
            && part[0] != b'-'
            && part[part.len() - 1] != b'-';
        if !ok {
            return false;
        }
        labels += 1;
    }
    labels >= 2
}
