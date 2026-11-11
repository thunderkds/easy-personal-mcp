//! T028 criteria 6, 7, 8: the differential corpora of `parity/gen_ingest_corpus.py` (Python's own `_utc`, `_label`,
//! `_WORD`, `SECRET`, `_bash_word`, `_host` on generated strings). Every entry must give Python's answer; an entry
//! where Python crashes (`crash:*`) must give `None`. The files are read with the crate's JSON parser: they hold lone
//! surrogates (`\udc80`), which serde_json refuses.

use easy_personal_core::ingest::json::{Value, code_points, loads};
use easy_personal_core::ingest::label::{is_label, is_word};
use easy_personal_core::ingest::secret::is_secret;
use easy_personal_core::ingest::signals::{
    LOWER_TO_ASCII, NFKC_NETLOC_FORBIDDEN, bash_word, host, is_py_space,
};
use easy_personal_core::ingest::time::utc_text;
use std::fs;
use std::path::Path;

fn corpus(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity/ingest/golden")
        .join(name);
    loads(&fs::read(path).unwrap()).expect("corpus parses")
}

fn list<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    match v.get(key) {
        Value::Array(items) => items,
        _ => panic!("no list {key}"),
    }
}

fn text(v: &Value) -> Option<String> {
    v.as_str().map(|s| String::from_utf8(s.to_vec()).unwrap())
}

fn show(s: &[u8]) -> String {
    format!(
        "{:?}",
        code_points(s)
            .map(|c| char::from_u32(c).unwrap_or('\u{FFFD}'))
            .collect::<String>()
    )
}

fn int(v: &Value) -> i64 {
    match v {
        Value::Int(n) => *n,
        _ => panic!("not an int"),
    }
}

/// Compare every entry of `section` with `f`; report all mismatches at once.
fn check<T: PartialEq + std::fmt::Debug>(
    section: &[Value],
    f: impl Fn(&[u8]) -> T,
    want: impl Fn(&Value) -> T,
) -> usize {
    let mut bad = Vec::new();
    for e in section {
        let input = e.get("in").as_str().unwrap();
        let (got, exp) = (f(input), want(e.get("out")));
        if got != exp {
            bad.push(format!("{} -> {got:?}, Python {exp:?}", show(input)));
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {} differ:\n{}",
        bad.len(),
        section.len(),
        bad.join("\n")
    );
    section.len()
}

#[test]
fn time_corpus_equals_python() {
    let c = corpus("corpus-time.json");
    let entries = list(&c, "entries");
    assert!(entries.len() >= 1500, "{}", entries.len());
    let n = check(entries, utc_text, text);
    let crashes = entries
        .iter()
        .filter(|e| {
            e.get("why")
                .as_str()
                .is_some_and(|w| w.starts_with(b"crash:"))
        })
        .count();
    println!("time corpus: {n} entries equal ({crashes} where Python crashes give None)");
}

#[test]
fn label_corpus_equals_python() {
    let c = corpus("corpus-label.json");
    let flag = |v: &Value| v.is_true();
    let mut total = 0;
    total += check(
        list(&c, "label"),
        |s| is_label(s).then(|| String::from_utf8_lossy(s).into_owned()),
        text,
    );
    total += check(list(&c, "word"), is_word, flag);
    total += check(list(&c, "secret"), is_secret, flag);
    total += check(list(&c, "bash"), bash_word, text);
    total += check(list(&c, "host"), host, text);
    assert!(total >= 800, "{total}");
    println!("label corpus: {total} entries equal");
}

/// The Python runtime tables the Rust code embeds or relies on (Unicode 16.0 in Python 3.14).
#[test]
fn runtime_tables_equal_python() {
    let c = corpus("corpus-label.json");
    let tables = c.get("tables");
    let ints = |key: &str| list(tables, key).iter().map(int).collect::<Vec<_>>();
    let nfkc: Vec<i64> = NFKC_NETLOC_FORBIDDEN
        .iter()
        .map(|&c| i64::from(c))
        .collect();
    assert_eq!(nfkc, ints("nfkc_netloc_forbidden"));
    let lower: Vec<i64> = LOWER_TO_ASCII.iter().map(|&(c, _)| i64::from(c)).collect();
    assert_eq!(lower, ints("lower_to_ascii"));
    let spaces: Vec<i64> = (0..0x11_0000u32)
        .filter(|&c| is_py_space(c))
        .map(i64::from)
        .collect();
    assert_eq!(spaces, ints("split_whitespace"));
}
