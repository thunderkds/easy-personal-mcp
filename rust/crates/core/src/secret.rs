//! The one secret check of saved reports (FR-076): Python's `SECRET` of `core/secretlike.py`, written by hand (the `regex`
//! crate is in `Cargo.lock` but is not a dependency of any crate here, and this task adds none).
//!
//! ```text
//! \bsk-[A-Za-z0-9]  |  \bgh[pousr]_[A-Za-z0-9]  |  \bgithub_pat_  |  \bxox[A-Za-z]?[-_]?[A-Za-z0-9]{4,}
//! |  \b(?:AKIA|ASIA)[A-Z0-9]{4,}  |  -----BEGIN  |  [A-Za-z0-9]{32,}
//! ```
//!
//! `search` answers whether the pattern matches anywhere in the text. `\b` before a word character means the character
//! before is not a word character (or there is none); a word character is Python's `\w` for text: a letter or digit of
//! any script, or `_`. The one known difference: Rust counts a combining mark that is part of a letter (a few scripts'
//! vowel signs) as a letter and Python does not, so `xsk-abc` after such a mark is judged differently. The corpus in
//! `rust/parity/golden-save-report/secret_corpus.json` (Python's answers) is compared in `tests/save_report_parity.rs`.

fn word(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

fn alnum(c: char) -> bool {
    c.is_ascii_alphanumeric()
}

/// `true` when `text` holds secret-looking text.
pub fn looks_secret(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    long_run(&chars) || (0..chars.len()).any(|i| at(&chars, i))
}

/// `[A-Za-z0-9]{32,}` anywhere.
fn long_run(chars: &[char]) -> bool {
    let mut run = 0;
    for &c in chars {
        run = if alnum(c) { run + 1 } else { 0 };
        if run >= 32 {
            return true;
        }
    }
    false
}

/// Does one of the anchored alternatives match with its first character at `i`?
fn at(chars: &[char], i: usize) -> bool {
    let boundary = i == 0 || !word(chars[i - 1]);
    let rest = &chars[i..];
    let starts = |prefix: &str| {
        let mut p = prefix.chars();
        rest.iter().zip(&mut p).all(|(a, b)| *a == b) && p.next().is_none()
    };
    let alnum_at = |n: usize| rest.get(n).is_some_and(|c| alnum(*c));
    if rest.starts_with(&['-', '-', '-', '-', '-', 'B', 'E', 'G', 'I', 'N']) {
        return true;
    }
    if !boundary {
        return false;
    }
    (starts("sk-") && alnum_at(3))
        || (starts_gh(rest) && alnum_at(4))
        || starts("github_pat_")
        || (starts("xox") && xox_tail(&rest[3..]))
        || ((starts("AKIA") || starts("ASIA")) && upper_run(&rest[4..]) >= 4)
}

/// `gh[pousr]_` (the character after the underscore is checked by the caller).
fn starts_gh(rest: &[char]) -> bool {
    matches!(rest, ['g', 'h', 'p' | 'o' | 'u' | 's' | 'r', '_', ..])
}

/// `[A-Z0-9]` characters in a row from the start.
fn upper_run(rest: &[char]) -> usize {
    rest.iter()
        .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        .count()
}

/// `[A-Za-z]?[-_]?[A-Za-z0-9]{4,}` at the start of `rest`, trying every way the optional parts can be taken.
fn xox_tail(rest: &[char]) -> bool {
    let run = |from: usize| {
        rest[from.min(rest.len())..]
            .iter()
            .take_while(|c| alnum(**c))
            .count()
            >= 4
    };
    let letter = rest.first().is_some_and(|c| c.is_ascii_alphabetic());
    let sep = |at: usize| rest.get(at).is_some_and(|c| matches!(c, '-' | '_'));
    run(0) || (letter && run(1)) || (sep(0) && run(1)) || (letter && sep(1) && run(2))
}

#[cfg(test)]
mod tests {
    use super::looks_secret;

    #[test]
    fn the_alternatives_and_their_near_misses() {
        for yes in [
            "sk-a",
            "ghp_a",
            "ghr_9",
            "github_pat_",
            "xoxb-1234",
            "xox_abcd5",
            "xox1234",
            "AKIAABCD",
            "ASIA1234",
            "-----BEGIN",
            "x-----BEGIN",
            &"a".repeat(32),
            "é sk-abc",
        ] {
            assert!(looks_secret(yes), "{yes}");
        }
        for no in [
            "",
            "sk-",
            "xsk-abc",
            "ésk-abc",
            "ghx_a",
            "ghp-a",
            "xoxb",
            "AKIAABC",
            "akiaabcd",
            &"a".repeat(31),
            "550e8400-e29b-41d4-a716-446655440000",
            "/home/a/b",
        ] {
            assert!(!looks_secret(no), "{no}");
        }
    }
}
