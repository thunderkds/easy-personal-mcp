//! T028 criterion 12 (NFR-005): the streaming reader on generated transcripts.
//! - `memory_12mb_under_8_mib`: a child process parses a 12 MB file with `LineReader`; its peak RSS (`VmHWM` of
//!   `/proc/self/status`) may grow less than 8 MiB above the baseline taken just before.
//! - `speed_200mb` (ignored, run with `--release -- --ignored`): parses a 200 MB file and needs >= 8 MB/s.
//!
//! The generated lines look like real transcripts: prompts, assistant messages with usage and tool calls, tool
//! results of 4-60 KB of escaped text, a few of 0.5-1 MB, metadata lines.

use easy_personal_core::ingest::claude_code::LineReader;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const MIB: u64 = 1024 * 1024;

struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// A transcript of about `size` bytes (deterministic).
fn generate(size: u64, name: &str) -> TempFile {
    let path = std::env::temp_dir().join(format!(
        "easy-personal-rs-t028-{name}-{}.jsonl",
        std::process::id()
    ));
    let mut out = BufWriter::new(File::create(&path).unwrap());
    let mut written = 0u64;
    let mut seed = 0x2028_u64;
    let mut rand = move |n: u64| {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 33) % n
    };
    let head = |i: u64, t: &str| {
        format!(
            r#"{{"type":"{t}","sessionId":"00000000-0000-4000-8000-{:012}","timestamp":"2026-01-10T10:{:02}:{:02}.123Z","version":"2.1.400","cwd":"/work/app","entrypoint":"cli","gitBranch":"main","isSidechain":false,"uuid":"u{i}""#,
            i / 500,
            i / 60 % 60,
            i % 60
        )
    };
    let text = |n: u64| {
        "line of output with \\\"quotes\\\" and a tab\\t and unicode \u{00e9}\\n"
            .repeat(n as usize / 60 + 1)
    };
    let mut i = 0u64;
    while written < size {
        let line = match i % 6 {
            0 => format!(
                r#"{},"message":{{"role":"user","content":"{}"}}}}"#,
                head(i, "user"),
                text(400 + rand(2000))
            ),
            1 | 3 => format!(
                r#"{},"message":{{"id":"msg_{i}","model":"claude-opus-5-5","role":"assistant","usage":{{"input_tokens":{},"output_tokens":{},"cache_creation_input_tokens":0,"cache_read_input_tokens":{}}},"content":[{{"type":"text","text":"{}"}},{{"type":"tool_use","id":"t{i}","name":"Bash","input":{{"command":"git status --short","description":"{}"}}}}]}}}}"#,
                head(i, "assistant"),
                rand(5000),
                rand(900),
                rand(100_000),
                text(200 + rand(3000)),
                text(80)
            ),
            2 | 4 => {
                let big = if rand(40) == 0 {
                    500_000 + rand(500_000)
                } else {
                    4_000 + rand(56_000)
                };
                format!(
                    r#"{},"message":{{"role":"user","content":[{{"type":"tool_result","tool_use_id":"t{i}","is_error":{},"content":"{}"}}]}},"toolUseResult":{{"stdout":"{}"}}}}"#,
                    head(i, "user"),
                    rand(10) == 0,
                    text(big),
                    text(big / 4)
                )
            }
            _ => format!(
                r#"{{"type":"file-history-snapshot","messageId":"m{i}","snapshot":{{"files":"{}"}}}}"#,
                text(500)
            ),
        };
        out.write_all(line.as_bytes()).unwrap();
        out.write_all(b"\n").unwrap();
        written += line.len() as u64 + 1;
        i += 1;
    }
    out.flush().unwrap();
    TempFile(path)
}

/// Lines, events and the bytes they cover, streaming.
fn parse(path: &Path) -> (usize, usize, u64) {
    let reader = LineReader::new(BufReader::new(File::open(path).unwrap()), 0);
    let (mut lines, mut events, mut end) = (0, 0, 0);
    for line in reader {
        let line = line.unwrap();
        lines += 1;
        events += usize::from(line.event.is_some_and(|e| !e.abstained));
        end = line.next_offset as u64;
    }
    (lines, events, end)
}

fn vm_hwm_kib() -> u64 {
    let status = fs::read_to_string("/proc/self/status").unwrap();
    let line = status.lines().find(|l| l.starts_with("VmHWM:")).unwrap();
    line.split_whitespace().nth(1).unwrap().parse().unwrap()
}

/// The child half of `memory_12mb_under_8_mib`: only does work when the parent sets the variable.
#[test]
fn memory_child() {
    let Ok(path) = std::env::var("EASY_PERSONAL_T028_MEM_FILE") else {
        return;
    };
    let baseline = vm_hwm_kib();
    let (lines, events, end) = parse(Path::new(&path));
    let peak = vm_hwm_kib();
    println!(
        "T028-MEM baseline_kib={baseline} peak_kib={peak} lines={lines} events={events} bytes={end}"
    );
}

#[test]
fn memory_12mb_under_8_mib() {
    let file = generate(12 * 1_000_000, "mem");
    let size = fs::metadata(&file.0).unwrap().len();
    let out = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "memory_child", "--nocapture", "--test-threads=1"])
        .env("EASY_PERSONAL_T028_MEM_FILE", &file.0)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    // libtest prints `test memory_child ... ` on the same line, before the report.
    let Some(report) = stdout
        .lines()
        .find_map(|l| l.find("T028-MEM").map(|i| &l[i..]))
    else {
        panic!(
            "no child report:\n{stdout}\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    let field = |k: &str| -> u64 {
        report
            .split_whitespace()
            .find_map(|f| f.strip_prefix(&format!("{k}=")))
            .unwrap()
            .parse()
            .unwrap()
    };
    let delta = field("peak_kib") - field("baseline_kib");
    println!("{report} file_bytes={size} delta_kib={delta}");
    assert_eq!(field("bytes"), size, "every line read");
    assert!(delta * 1024 < 8 * MIB, "peak grew {delta} KiB");
}

/// About `size` bytes of the committed fixtures' complete lines, repeated: small records with many short fields,
/// the slow case for a parser (the generated file above is dominated by long strings).
fn repeat_fixtures(size: u64, name: &str) -> TempFile {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/claude_code");
    let mut lines = Vec::new();
    for sub in [
        "2.1.293/session.jsonl",
        "2.1.295/session.jsonl",
        "2.1.302/session.jsonl",
        "2.1.316/meta.jsonl",
    ] {
        let bytes = fs::read(dir.join(sub)).unwrap();
        let end = bytes.iter().rposition(|&b| b == b'\n').unwrap() + 1; // complete lines only
        lines.extend_from_slice(&bytes[..end]);
    }
    let path = std::env::temp_dir().join(format!(
        "easy-personal-rs-t028-{name}-{}.jsonl",
        std::process::id()
    ));
    let mut out = BufWriter::new(File::create(&path).unwrap());
    let mut written = 0;
    while written < size {
        out.write_all(&lines).unwrap();
        written += lines.len() as u64;
    }
    out.flush().unwrap();
    TempFile(path)
}

fn timed(label: &str, file: &TempFile) -> f64 {
    let size = fs::metadata(&file.0).unwrap().len();
    let start = Instant::now();
    let (lines, events, end) = parse(&file.0);
    let secs = start.elapsed().as_secs_f64();
    let rate = size as f64 / 1e6 / secs;
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    println!(
        "T028-SPEED {profile} {label}: {size} bytes, {lines} lines, {events} events in {secs:.2} s = {rate:.1} MB/s"
    );
    assert_eq!(end, size);
    rate
}

#[test]
#[ignore = "200 MB; run with: cargo test --release -p easy-personal-core --test ingest_parse_perf -- --ignored --nocapture"]
fn speed_200mb() {
    let generated = timed("generated", &generate(200 * 1_000_000, "speed"));
    let fixtures = timed(
        "fixture lines",
        &repeat_fixtures(200 * 1_000_000, "speed-fixtures"),
    );
    assert!(
        generated >= 8.0 && fixtures >= 8.0,
        "{generated:.1} / {fixtures:.1} MB/s"
    );
}
