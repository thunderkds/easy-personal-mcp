//! T028 criteria 3, 4, 5: `ingest::json::loads` is Python's `json.loads` on bytes. Each expectation below was taken
//! from the parity venv (CPython 3.14.4, `json.loads(<bytes>)`), noted per case; "Error" is a `ValueError` there.

use easy_personal_core::ingest::claude_code::read_lines;
use easy_personal_core::ingest::json::{MAX_DEPTH, Value, loads};
use easy_personal_core::writer::Event;

fn s(text: &str) -> Value {
    Value::Str(text.as_bytes().to_vec())
}

fn obj_a1() -> Value {
    Value::Object(vec![(b"a".to_vec(), Value::Int(1))])
}

/// Python: `{'a': 1}` for each.
#[test]
fn decoding_utf16_and_utf32_lines() {
    let le = |t: &str| {
        t.encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<u8>>()
    };
    let be = |t: &str| {
        t.encode_utf16()
            .flat_map(u16::to_be_bytes)
            .collect::<Vec<u8>>()
    };
    let mut bom = vec![0xFF, 0xFE];
    bom.extend(le(r#"{"a":1}"#));
    assert_eq!(loads(&bom), Some(obj_a1()), "utf-16 BOM");
    assert_eq!(
        loads(&be(r#"{"a":1}"#)),
        Some(obj_a1()),
        "utf-16-be by zero bytes"
    );
    assert_eq!(
        loads(&le(r#"{"a":1}"#)),
        Some(obj_a1()),
        "utf-16-le by zero bytes"
    );
    let mut odd = le(r#"{"a":1}"#);
    odd.push(b'\n');
    assert_eq!(
        loads(&odd),
        None,
        "utf-16-le + 0x0a (the oracle's line): UnicodeDecodeError"
    );
    let u32le: Vec<u8> = r#"{"a":1}"#.chars().flat_map(|c| (c as u32).to_le_bytes()).collect();
    assert_eq!(loads(&u32le), Some(obj_a1()), "utf-32-le by zero bytes");
    let mut u32be = vec![0, 0, 0xFE, 0xFF];
    u32be.extend(r#"{"a":1}"#.chars().flat_map(|c| (c as u32).to_be_bytes()));
    assert_eq!(loads(&u32be), Some(obj_a1()), "utf-32 BOM");
    assert_eq!(
        loads(&[0x22, 0, 0, 0, 0, 0, 0x11, 0, 0x22, 0, 0, 0]),
        None,
        "utf-32 above U+10FFFF"
    );
    assert_eq!(
        loads(b"1\x00"),
        Some(Value::Int(1)),
        "two bytes, XX 00: utf-16-le"
    );
}

/// `surrogatepass`: a lone surrogate is kept as its 3-byte form; Python: `'\ud800'`.
#[test]
fn decoding_keeps_lone_surrogates() {
    let lone = Value::Str(vec![0xED, 0xA0, 0x80]);
    assert_eq!(
        loads(b"\"\xed\xa0\x80\""),
        Some(lone.clone()),
        "utf-8 encoded surrogate"
    );
    assert_eq!(
        loads(b"\xff\xfe\"\x00\x00\xd8\"\x00"),
        Some(lone.clone()),
        "utf-16 lone high"
    );
    assert_eq!(
        loads(b"\"\x00\x00\x00\x00\xd8\x00\x00\"\x00\x00\x00"),
        Some(lone),
        "utf-32 surrogate"
    );
    assert_eq!(loads(b"\"\xc0\x80\""), None, "overlong: UnicodeDecodeError");
    assert_eq!(loads(b"\"\xff\""), None, "0xff: UnicodeDecodeError");
}

/// A UTF-8 BOM is dropped once (Python: `{}`; two BOMs: JSONDecodeError).
#[test]
fn decoding_utf8_bom() {
    assert_eq!(loads(b"\xef\xbb\xbf  {}\n"), Some(Value::Object(vec![])));
    assert_eq!(loads(b"\xef\xbb\xbf\xef\xbb\xbf{}"), None);
}

/// Python: `[inf, -inf, nan]`; `-NaN` is an error.
#[test]
fn nan_and_infinity_are_floats() {
    assert_eq!(
        loads(b"[Infinity,-Infinity,NaN]"),
        Some(Value::Array(vec![Value::Float; 3]))
    );
    assert_eq!(loads(b"-NaN"), None);
}

/// Python: `{'type': 'user'}` for `{"type":"x","type":"user"}` (the last wins; the oracle's line 16 too).
#[test]
fn duplicate_key_last_wins() {
    let v = loads(br#"{"type":"brand-new-type","type":"user"}"#).unwrap();
    assert_eq!(v.get("type"), &s("user"));
}

/// Strings: escapes, pairs joined, lone surrogates kept, control characters refused, DEL kept.
#[test]
fn strings_like_the_c_scanner() {
    assert_eq!(loads(b"\"\\ud83d\\ude00\""), Some(s("\u{1F600}")), "pair");
    assert_eq!(
        loads(b"\"\\ud83dx\""),
        Some(Value::Str(vec![0xED, 0xA0, 0xBD, b'x'])),
        "lone high"
    );
    assert_eq!(
        loads(b"\"\\ud83d\\ud83d\""),
        Some(Value::Str([0xED, 0xA0, 0xBD].repeat(2))),
        "two highs"
    );
    assert_eq!(loads(b"\"a\x01\""), None, "control character");
    assert_eq!(loads(b"\"a\x7f\""), Some(s("a\x7f")), "DEL");
    assert_eq!(loads(b"\"\\x\""), None, "bad escape");
    assert_eq!(loads(b"\"\\u12g4\""), None, "bad hex");
    assert_eq!(
        loads(br#""\"\\\/\b\f\n\r\t""#),
        Some(s("\"\\/\u{8}\u{c}\n\r\t"))
    );
}

/// Numbers and the grammar around them (each: Python's answer).
#[test]
fn numbers_and_grammar() {
    assert_eq!(loads(b"-0"), Some(Value::Int(0)));
    assert_eq!(loads(b"9223372036854775807"), Some(Value::Int(i64::MAX)));
    assert_eq!(loads(b"-9223372036854775808"), Some(Value::Int(i64::MIN)));
    assert_eq!(
        loads(b"9223372036854775808"),
        Some(Value::BigInt),
        "2**63: an int Python keeps"
    );
    assert_eq!(
        loads(b"123456789012345678901234567890"),
        Some(Value::BigInt)
    );
    assert_eq!(loads(b"1E+2"), Some(Value::Float));
    assert_eq!(loads(b"1.0"), Some(Value::Float), "a float stays a float");
    for bad in [
        &b"01"[..],
        b"1.",
        b"1e",
        b"[1,]",
        br#"{"a":1,}"#,
        b"\x0b{}",
        b"{} \x0c",
        b"",
        b"  ",
    ] {
        assert_eq!(loads(bad), None, "{:?}", String::from_utf8_lossy(bad));
    }
}

/// `int_max_str_digits` = 4300: Python parses 4300 digits (sign not counted) and raises ValueError at 4301.
#[test]
fn int_digit_limit() {
    let digits = |n: usize| "1".repeat(n);
    assert_eq!(loads(digits(4300).as_bytes()), Some(Value::BigInt));
    assert_eq!(
        loads(format!("-{}", digits(4300)).as_bytes()),
        Some(Value::BigInt)
    );
    assert_eq!(loads(digits(4301).as_bytes()), None);
}

const USER: &str = r#"{"type":"user","sessionId":"s1","timestamp":"2026-01-10T10:00:00Z","version":"2.1.400","cwd":"/w","#;

fn one(line: &str) -> Event {
    let mut bytes = line.as_bytes().to_vec();
    bytes.push(b'\n');
    read_lines(&bytes, 0).remove(0).event.unwrap()
}

/// Criterion 3 + 4: a bad value in a non-usage field does not spoil the line; past the digit limit it does.
#[test]
fn int_max_str_digits_in_a_record() {
    let ok = one(&format!(r#"{USER}"x":{}}}"#, "9".repeat(4300)));
    assert!(!ok.abstained && ok.record_type.as_deref() == Some("user"));
    let bad = one(&format!(r#"{USER}"x":{}}}"#, "9".repeat(4301)));
    assert_eq!(
        bad,
        Event {
            abstained: true,
            ..Event::default()
        }
    );
}

/// Criterion 5: a valid record nested exactly [`MAX_DEPTH`] deep (Python parses it in every measured run) is a
/// normal row; one level more is an all-NULL abstained row.
#[test]
fn python_depth_limit_parses() {
    let nested = |levels: usize| {
        format!(
            r#"{USER}"x":{}{}}}"#,
            "[".repeat(levels),
            "]".repeat(levels)
        )
    };
    let ok = one(&nested(MAX_DEPTH - 1)); // the record object is one level
    assert!(
        !ok.abstained && ok.session_id.as_deref() == Some("s1"),
        "{ok:?}"
    );
    let deep = one(&nested(MAX_DEPTH));
    assert_eq!(
        deep,
        Event {
            abstained: true,
            ..Event::default()
        }
    );
    assert_eq!(
        MAX_DEPTH, 57_890,
        "pinned: measured with the parity venv, see json.rs"
    );
}

/// Criterion 5: 100,000 nested `[` (the S06a line, here closed and unclosed) never touches the Rust stack.
#[test]
fn hundred_thousand_brackets_abstain() {
    let open = "[".repeat(100_000);
    assert_eq!(loads(open.as_bytes()), None);
    assert_eq!(
        loads(format!("{open}{}", "]".repeat(100_000)).as_bytes()),
        None
    );
    assert_eq!(
        one(&open),
        Event {
            abstained: true,
            ..Event::default()
        }
    );
    // Deep but inside the limit, and objects as well as arrays.
    let objects = format!("{}0{}", r#"{"a":"#.repeat(50_000), "}".repeat(50_000));
    assert!(matches!(loads(objects.as_bytes()), Some(Value::Object(_))));
}
