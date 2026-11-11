//! T028 criteria 2, 9, 10 and the fuzz of the Edge Case Checklist: line framing, the record dispatch, privacy
//! (canaries, the planted fake secrets, a compile-level guard on `Event`), and no panic on any byte sequence.

use easy_personal_core::ingest::claude_code::{LineReader, read_lines};
use easy_personal_core::ingest::signals::bash_word;
use easy_personal_core::writer::{Event, Line, Signal};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// Every `.jsonl` file of the committed fixtures and of the T026 oracle inputs (`%WORK%` left as is).
fn fixture_files() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let mut entries: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|e| e == "jsonl") {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    walk(&repo().join("tests/fixtures/claude_code"), &mut out);
    walk(&repo().join("rust/parity/ingest/fixtures"), &mut out);
    out
}

/// The event of one line (newlines inside `line` are only layout: they become spaces).
fn event(line: &str) -> Event {
    let bytes = format!("{}\n", line.replace('\n', " "));
    read_lines(bytes.as_bytes(), 0).remove(0).event.unwrap()
}

// ---- criterion 2: framing ----

#[test]
fn framing_blank_whitespace_crlf_and_the_end() {
    let rec = r#"{"type":"mode","sessionId":"s1"}"#;
    let text = format!("{rec}\n\n \t\x0b\x0c\r\n{rec}\r\n{rec}");
    let lines = read_lines(text.as_bytes(), 0);
    let at = |l: &Line| (l.line_offset, l.next_offset, l.event.is_some());
    let a = rec.len() as i64 + 1;
    assert_eq!(
        lines.iter().map(at).collect::<Vec<_>>(),
        vec![
            (0, a, true),
            (a, a + 1, false),
            (a + 1, a + 7, false),
            (a + 7, 2 * a + 8, true)
        ],
        "blank and ASCII-whitespace lines step the offset with no event; the cut last line is not read"
    );
    assert_eq!(
        lines[3].event.as_ref().unwrap().record_type.as_deref(),
        Some("mode"),
        "CRLF parses"
    );
    // A file that ends exactly at `\n`: no phantom line.
    let exact = format!("{rec}\n");
    assert_eq!(read_lines(exact.as_bytes(), 0).len(), 1);
    assert_eq!(read_lines(exact.as_bytes(), exact.len()), vec![]);
    assert_eq!(
        read_lines(exact.as_bytes(), exact.len() + 10),
        vec![],
        "a start past the end"
    );
    assert_eq!(read_lines(b"", 0), vec![]);
}

/// The streaming reader gives exactly the lines of `read_lines`, from any start offset, on every fixture.
#[test]
fn line_reader_equals_read_lines() {
    for path in fixture_files() {
        let bytes = fs::read(&path).unwrap();
        for start in [
            0,
            bytes.iter().position(|&b| b == b'\n').map_or(0, |i| i + 1),
        ] {
            let streamed: Vec<Line> = LineReader::new(Cursor::new(&bytes[start..]), start as i64)
                .map(Result::unwrap)
                .collect();
            assert_eq!(streamed, read_lines(&bytes, start), "{}", path.display());
        }
    }
}

// ---- criterion 9: the dispatch, beyond what the goldens hold ----

const HEAD: &str =
    r#""sessionId":"s1","timestamp":"2026-01-10T10:00:00Z","version":"2.1.400","cwd":"/w/app""#;

#[test]
fn dispatch_cases() {
    // A `type` that is an object (Python: `TypeError`, unhashable, like S06d's list): abstained, record_type NULL.
    let e = event(&format!(r#"{{"type":{{"a":1}},{HEAD}}}"#));
    assert!(e.abstained && e.record_type.is_none() && e.session_id.as_deref() == Some("s1"));
    // A number or `true` as type is hashable in Python and unknown: abstained, record_type NULL.
    assert!(event(&format!(r#"{{"type":true,{HEAD}}}"#)).abstained);
    // `project_path` is the raw cwd; an empty cwd is NULL; a non-string cwd abstains.
    assert_eq!(
        event(&format!(r#"{{"type":"user",{HEAD}}}"#))
            .project_path
            .as_deref(),
        Some("/w/app")
    );
    let empty =
        event(r#"{"type":"user","sessionId":"s1","timestamp":"2026-01-10T10:00:00Z","cwd":""}"#);
    assert!(!empty.abstained && empty.project_path.is_none());
    assert!(
        event(r#"{"type":"user","sessionId":"s1","timestamp":"2026-01-10T10:00:00Z","cwd":1}"#)
            .abstained
    );
    // A normal non-ASCII `cwd` is stored as is.
    assert_eq!(
        event(r#"{"type":"user","sessionId":"s1","timestamp":"2026-01-10T10:00:00Z","cwd":"/work/ünï"}"#)
            .project_path
            .as_deref(),
        Some("/work/ünï")
    );
    // ⚠ decided for you (Rust differs): a lone-surrogate `cwd` makes Python crash (`UnicodeEncodeError` binding
    // `project_path`); here it counts as "present but not storable" and the record abstains, like a non-string `cwd`.
    // A valid surrogate pair (U+1F600) is a real character and is stored.
    let lone =
        r#"{"type":"user","sessionId":"s1","timestamp":"2026-01-10T10:00:00Z","cwd":"/w/\udc80x"}"#;
    let e = event(lone);
    assert!(e.abstained && e.project_path.is_none() && e.session_id.as_deref() == Some("s1"));
    assert!(event(&lone.replace("udc80", "ud83d")).abstained);
    assert_eq!(
        event(&lone.replace(r"\udc80", r"\ud83d\ude00"))
            .project_path
            .as_deref(),
        Some("/w/\u{1F600}x")
    );
    // The line after an abstained one is still read.
    let text = format!("{lone}\n{{\"type\":\"mode\",\"sessionId\":\"s2\"}}\n");
    let lines = read_lines(text.as_bytes(), 0);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].event.as_ref().unwrap().abstained);
    assert_eq!(
        lines[1].event.as_ref().unwrap().record_type.as_deref(),
        Some("mode")
    );
    // A naive timestamp abstains a versioned type, not a metadata type.
    assert!(
        event(r#"{"type":"user","sessionId":"s1","timestamp":"2026-01-10T10:00:00"}"#).abstained
    );
    let meta = event(r#"{"type":"mode","sessionId":"s1","timestamp":"2026-01-10T10:00:00"}"#);
    assert!(!meta.abstained && meta.ts_utc.is_none() && meta.is_sidechain.is_none());
    // A timestamp whose UTC value is before year 1 (Python: OverflowError, crash): no timestamp, abstained here.
    assert!(
        event(r#"{"type":"user","sessionId":"s1","timestamp":"0001-01-01T00:00:00+01:00"}"#)
            .abstained
    );
}

/// `type(x) is int`: a bool, a float, a string, null, a list or an object in a usage field abstains the record
/// with its type, version, ts and session kept; `NaN` too (not the whole line).
#[test]
fn usage_needs_four_exact_ints() {
    let usage = |input: &str| {
        event(&format!(
            r#"{{"type":"assistant",{HEAD},"message":{{"id":"m1","model":"x","usage":{{"input_tokens":{input},"output_tokens":2,"cache_creation_input_tokens":3,"cache_read_input_tokens":4}}}}}}"#
        ))
    };
    let ok = usage("1");
    assert!(!ok.abstained);
    assert_eq!(
        (
            ok.input_tokens,
            ok.output_tokens,
            ok.cache_creation_input_tokens,
            ok.cache_read_input_tokens
        ),
        (Some(1), Some(2), Some(3), Some(4))
    );
    assert_eq!(usage("-9223372036854775808").input_tokens, Some(i64::MIN));
    for bad in [
        "true",
        "false",
        "1.0",
        "1e0",
        "\"1\"",
        "null",
        "[]",
        "{}",
        "NaN",
        "Infinity",
        "9223372036854775808",
    ] {
        let e = usage(bad);
        assert!(e.abstained, "{bad}");
        assert_eq!(
            (
                e.record_type.as_deref(),
                e.session_id.as_deref(),
                e.version.as_deref()
            ),
            (Some("assistant"), Some("s1"), Some("2.1.400")),
            "{bad}"
        );
        assert!(
            e.ts_utc.is_some() && e.input_tokens.is_none() && e.model.is_none(),
            "{bad}"
        );
    }
}

#[test]
fn tool_names_signals_and_errors() {
    let e = event(&format!(
        r#"{{"type":"assistant",{HEAD},"isApiErrorMessage":true,"isSidechain":1,"message":{{"id":"m1","content":[
        {{"type":"tool_use","name":"Bash","input":{{"command":"git status"}}}},
        {{"type":"tool_use","name":"Bash","input":{{"command":"git log"}}}},
        {{"type":"tool_use","name":"bad name","input":{{}}}},
        {{"type":"tool_use","name":"WebFetch","input":{{"url":"https://u:p@Docs.Example.com.:8443/x"}}}},
        {{"type":"text","text":"x"}}, 7]}}}}"#
    ));
    assert_eq!(
        e.tool_names.as_deref(),
        Some("Bash,Bash,WebFetch"),
        "repeats kept, a non-label dropped"
    );
    let sig = |k: &str, v: &str| Signal {
        kind: k.into(),
        value: v.into(),
    };
    assert_eq!(
        e.signals,
        vec![
            sig("bash", "git"),
            sig("bash", "git"),
            sig("domain", "docs.example.com")
        ]
    );
    assert_eq!(
        (e.is_api_error, e.is_sidechain),
        (Some(true), Some(false)),
        "`is True`: 1 is not True"
    );
    let user = |content: &str| {
        event(&format!(
            r#"{{"type":"user",{HEAD},"message":{{"content":{content}}}}}"#
        ))
        .tool_error_count
    };
    assert_eq!(
        user(
            r#"[{"type":"tool_result","is_error":true},{"type":"tool_result","is_error":"true"},{"type":"tool_result"}]"#
        ),
        Some(1)
    );
    assert_eq!(
        user(r#"[{"type":"tool_result","is_error":false}]"#),
        Some(0)
    );
    assert_eq!(user(r#"[{"type":"text"}]"#), None);
    assert_eq!(user(r#""text""#), None);
}

/// `bash_word` is `pub` and reads bytes: no byte slice may make it panic (a stray continuation byte, a cut multi-byte
/// lead, empty, whitespace only, a long run of continuation bytes). Valid input is covered by the `bash` corpus.
#[test]
fn bash_word_never_panics_on_any_bytes() {
    let long = vec![0x80u8; 10_000];
    let cases: [&[u8]; 15] = [
        b"\x80",
        b"\x80ls",
        b"ls\x80 x",
        b"\xe2\x82",
        b"ls \xe2\x82",
        b"\xf0\x9f",
        b"\xf0",
        b"\xff\xfe",
        b"",
        b"  \t\n",
        b"\xed\xa0",
        b"/bin/\xc3",
        b"\xc3\xa9",
        b"\xc3\xa9\xc3\xa9 \xe2\x82\xac",
        &long,
    ];
    for c in cases {
        let r = std::panic::catch_unwind(|| bash_word(c));
        assert!(r.is_ok(), "bash_word panicked on {c:?}");
    }
    assert_eq!(bash_word(b"  /usr/bin/git status").as_deref(), Some("git"));
    assert_eq!(bash_word(b""), None);
    assert_eq!(bash_word(b"   "), None);
}

// ---- criterion 10: privacy ----

/// Compile-level guard: adding a field to `Event` breaks this destructuring, which forces a look at what the new
/// field may hold. The only strings are labels (`_LABEL`), the stored UTC text, the raw `cwd` (`project_path`,
/// resolved by T035, as Python stores it) and the signals (a Bash first word, a host).
#[test]
fn event_has_no_free_text_field() {
    let Event {
        abstained: _,
        record_type: _,
        version: _,
        ts_utc: _,
        session_id: _,
        project_path: _,
        git_branch: _,
        is_sidechain: _,
        is_api_error: _,
        entrypoint: _,
        tool_error_count: _,
        model: _,
        message_id: _,
        tool_names: _,
        input_tokens: _,
        output_tokens: _,
        cache_creation_input_tokens: _,
        cache_read_input_tokens: _,
        signals: _,
    } = Event::default();
    let Signal { kind: _, value: _ } = Signal {
        kind: String::new(),
        value: String::new(),
    };
}

/// Every text-bearing field Python never reads holds a unique canary; none reaches any `Event` field or signal. A
/// Bash first word *is* kept by design, so the canaries there are secret-shaped (`sk-`, a 32-character run).
#[test]
fn canaries_never_reach_an_event() {
    let lines = [
        format!(
            r#"{{"type":"assistant",{HEAD},"gitBranch":"main","entrypoint":"cli","toolUseResult":"CANARY-1","summary":"CANARY-2","requestId":"CANARY-3",
            "message":{{"id":"msg_1","model":"claude-x","role":"CANARY-4","stop_reason":"CANARY-5","usage":{{"input_tokens":1,"output_tokens":2,"cache_creation_input_tokens":3,"cache_read_input_tokens":4,"service_tier":"CANARY-6"}},
            "content":[{{"type":"text","text":"CANARY-7"}},{{"type":"thinking","thinking":"CANARY-8","signature":"CANARY-9"}},
            {{"type":"tool_use","id":"CANARY-10","name":"Bash","input":{{"command":"ls CANARY-11 --x=CANARY-12","description":"CANARY-13"}}}},
            {{"type":"tool_use","id":"CANARY-14","name":"WebFetch","input":{{"url":"https://CANARY-15:CANARY-16@example.com:1/CANARY-17?q=CANARY-18#CANARY-19","prompt":"CANARY-20"}}}},
            {{"type":"tool_use","name":"Edit","input":{{"file_path":"/CANARY-21","old_string":"CANARY-22","new_string":"CANARY-23"}}}},
            {{"type":"tool_use","name":"Bash","input":{{"command":"sk-CANARY24 --x"}}}},
            {{"type":"tool_use","name":"Bash","input":{{"command":"/bin/CANARY24AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA x"}}}}]}}}}"#
        ),
        format!(
            r#"{{"type":"user",{HEAD},"toolUseResult":{{"stdout":"CANARY-25","stderr":"CANARY-26"}},"message":{{"role":"user","content":[
            {{"type":"tool_result","tool_use_id":"CANARY-27","is_error":true,"content":"CANARY-28"}},
            {{"type":"tool_result","content":[{{"type":"text","text":"CANARY-29"}}]}},{{"type":"text","text":"CANARY-30"}}]}}}}"#
        ),
        format!(r#"{{"type":"user",{HEAD},"message":{{"role":"user","content":"CANARY-31"}}}}"#),
        format!(r#"{{"type":"attachment",{HEAD},"attachment":{{"type":"file","content":"CANARY-32","filename":"CANARY-33"}}}}"#),
        format!(r#"{{"type":"system",{HEAD},"content":"CANARY-34","subtype":"CANARY-35"}}"#),
        r#"{"type":"last-prompt","sessionId":"s1","lastPrompt":"CANARY-36"}"#.to_owned(),
        r#"{"type":"custom-title","sessionId":"s1","customTitle":"CANARY-37"}"#.to_owned(),
        r#"{"type":"cost-state","sessionId":"s1","totalCostUSD":"CANARY-38","costs":{"x":"CANARY-39"}}"#.to_owned(),
        r#"{"type":"ai-title","sessionId":"s1","aiTitle":"CANARY-40"}"#.to_owned(),
        r#"{"type":"queue-operation","sessionId":"s1","operation":"enqueue","content":"CANARY-41"}"#.to_owned(),
        r#"{"type":"file-history-snapshot","messageId":"CANARY-42","snapshot":{"trackedFileBackups":{"CANARY-43":{}}}}"#
            .to_owned(),
        r#"{"type":"brand-new","sessionId":"s1","text":"CANARY-44"}"#.to_owned(),
    ];
    let text: String = lines.iter().map(|l| l.replace('\n', " ") + "\n").collect();
    let events: Vec<Event> = read_lines(text.as_bytes(), 0)
        .into_iter()
        .map(|l| l.event.unwrap())
        .collect();
    assert_eq!(events.len(), lines.len());
    let dump = format!("{events:?}");
    assert!(
        !dump.contains("CANARY"),
        "a canary reached an event: {}",
        dump.len()
    );
    // Anti-vacuity: the lines were read, not abstained, and the safe parts did come through.
    let a = &events[0];
    assert!(!a.abstained && a.tool_names.as_deref() == Some("Bash,WebFetch,Edit,Bash,Bash"));
    let sig = |k: &str, v: &str| Signal {
        kind: k.into(),
        value: v.into(),
    };
    assert_eq!(
        a.signals,
        vec![sig("bash", "ls"), sig("domain", "example.com")]
    );
    assert_eq!(events[1].tool_error_count, Some(1));
    assert!(events[..11].iter().all(|e| !e.abstained) && events[11].abstained);
}

/// The planted fake secrets of the fixtures (`sk-FAKE...`, `AKIAFAKE...`, every `FAKE`/`INVENTED` canary) never
/// appear in an `Event` of any fixture line.
#[test]
fn planted_fake_secrets_never_reach_an_event() {
    let mut events = 0;
    for path in fixture_files() {
        let bytes = fs::read(&path).unwrap();
        for line in read_lines(&bytes, 0) {
            let dump = format!("{:?}", line.event);
            assert!(
                !dump.contains("FAKE") && !dump.contains("INVENTED"),
                "{}",
                path.display()
            );
            events += usize::from(line.event.is_some());
        }
    }
    assert!(events > 200, "{events}");
}

// ---- fuzz: no panic on any byte sequence ----

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

#[test]
fn fuzz_ten_thousand_lines_never_panic() {
    let seeds: Vec<Vec<u8>> = fixture_files()
        .iter()
        .flat_map(|p| {
            fs::read(p)
                .unwrap()
                .split(|&b| b == b'\n')
                .map(<[u8]>::to_vec)
                .collect::<Vec<_>>()
        })
        .filter(|l| !l.is_empty() && l.len() < 200_000)
        .collect();
    let alphabet =
        b"{}[]\":,\\u0123456789eE+-.ntfrueal sNaIy\x00\x0b\xff\xfe\xed\xa0\x80\xc0\xef\xbb\xbfZT";
    let mut rng = Rng(0x7028_2026_1111);
    let mut lines = 0;
    let mut abstained = 0;
    let mut panics = 0;
    for i in 0..10_000 {
        let mut line = if i % 5 == 0 {
            (0..rng.below(300))
                .map(|_| alphabet[rng.below(alphabet.len())])
                .collect()
        } else {
            seeds[rng.below(seeds.len())].clone()
        };
        for _ in 0..rng.below(4) {
            let at = rng.below(line.len() + 1);
            match rng.below(6) {
                0 if !line.is_empty() => {
                    line.remove(at.min(line.len() - 1));
                }
                1 => line.insert(at, alphabet[rng.below(alphabet.len())]),
                2 if at < line.len() => line[at] = rng.next() as u8,
                3 => line.truncate(at),
                4 => {
                    let open = if rng.below(2) == 0 { b'[' } else { b'{' };
                    line.splice(at..at, std::iter::repeat_n(open, rng.below(70_000)));
                }
                _ => {
                    line.splice(at..at, b"99999999999999999999".iter().copied());
                }
            }
        }
        for b in &mut line {
            if *b == b'\n' {
                *b = b' ';
            }
        }
        line.push(b'\n');
        match std::panic::catch_unwind(|| read_lines(&line, 0)) {
            Ok(out) => {
                assert_eq!(out.len(), 1);
                lines += 1;
                abstained += usize::from(out[0].event.as_ref().is_some_and(|e| e.abstained));
            }
            Err(_) => panics += 1,
        }
    }
    println!("fuzz: {lines} lines parsed, {abstained} abstained, {panics} panicked");
    assert_eq!((lines, panics), (10_000, 0));
}
