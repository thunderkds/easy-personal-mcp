//! T032 (NFR-005): the ingest gates of the real binary that cargo can repeat; the 200 MB speed run stays in
//! `rust/parity/bench_ingest.py` (it also runs Python beside Rust). Both tests are ignored (they write about 30 MB):
//!
//! `cargo test --release -p easy-personal-rs --test ingest_perf -- --ignored --nocapture` (release: the numbers of the
//! shipped binary; a debug build passes too, with a thin memory margin, see below).
//!
//! - `memory_12mb_under_8_mib`: one 12 MB transcript of small assistant lines, the shape of the Python gate
//!   `tests/ops/test_ingest_memory.py` (one event per 300 bytes: the worst case for memory that grows with the events
//!   of a file). The peak RSS of the `ingest` child may grow less than 8 MiB above the same binary running `--version`.
//!   This pins the T032 bound: before the lines were streamed into `save_file_streamed` (T032), they were collected
//!   per file and this test failed (see `tasks/TASK_REVIEW_T032.md`).
//! - `unchanged_rerun_under_2s`: 200 files ingested once, then again unchanged: the second run must take < 2 s.
//!
//! The peak is the child's `VmHWM` at exit as `/usr/bin/time -f %M` reports it (its `ru_maxrss`). Not our own wait:
//! a spawned child starts with its parent's high-water mark and Linux keeps it across `execve`; `time` is small.
//! CI margin (measured 2026-11-11, Ryzen 7 5700X, ext4, 27,053 events): release +5.1 MiB of the 8 MiB and a 0.012 s
//! re-run of the 2 s; debug +7.4 MiB (thin) and 0.03 s. Memory does not depend on CPU speed, so
//! a slower runner keeps the same margin; the re-run has a 60x margin. The thresholds are NFR-005's, not tuned here.

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const MIB: u64 = 1024 * 1024;
const TIME: &str = "/usr/bin/time";

/// A fresh folder under cargo's target tmp dir (on the build disk, not a RAM-backed `/tmp`); removed on drop.
struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Dir {
        let path = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("t032-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Dir(path)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Lines like `tests/ops/test_ingest_memory.py`'s (one assistant event each, a distinct `message.id`), until `size`.
fn small_lines(path: &Path, size: u64, tag: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut out = BufWriter::new(File::create(path).unwrap());
    let (mut written, mut i) = (0u64, 0u64);
    while written < size {
        let line = format!(
            r#"{{"type": "assistant", "sessionId": "s-{tag}", "timestamp": "2026-01-01T00:{:02}:{:02}Z", "cwd": "/work/none", "message": {{"id": "m{tag}{i}", "model": "claude-x", "content": [{{"type": "text", "text": "{}"}}], "usage": {{"input_tokens": 1, "output_tokens": 1}}}}}}"#,
            i / 60 % 60,
            i % 60,
            "x".repeat(200)
        );
        writeln!(out, "{line}").unwrap();
        written += line.len() as u64 + 1;
        i += 1;
    }
    out.flush().unwrap();
}

/// Run the binary under `time` with an env built from nothing; (seconds, peak KiB, stdout).
fn run(dir: &Path, args: &[&str]) -> (f64, u64, String) {
    assert!(
        Path::new(TIME).exists(),
        "{TIME} (GNU time) measures the peak RSS"
    );
    let (nowhere, agents, rss) = (dir.join("nowhere"), dir.join("agents"), dir.join("rss"));
    fs::create_dir_all(&nowhere).unwrap();
    fs::create_dir_all(&agents).unwrap(); // empty: no account row, the source is read untagged
    let start = Instant::now();
    let out = Command::new(TIME)
        .args(["-f", "%M", "-o"])
        .arg(&rss)
        .arg(env!("CARGO_BIN_EXE_easy-personal-rs"))
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", &nowhere)
        .env("EASY_PERSONAL_HOME", dir.join("data"))
        .env("EASY_PERSONAL_AGENT_ROOT", &agents)
        .env("TZ", "UTC")
        .current_dir(&nowhere)
        .output()
        .unwrap();
    let secs = start.elapsed().as_secs_f64();
    assert!(
        out.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let kib = fs::read_to_string(&rss).unwrap();
    let kib = kib.split_whitespace().last().unwrap().parse().unwrap();
    (secs, kib, String::from_utf8(out.stdout).unwrap())
}

fn new_events(stdout: &str) -> i64 {
    let v: serde_json::Value = serde_json::from_str(stdout).unwrap();
    v["new_events"].as_i64().unwrap()
}

fn profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

#[test]
#[ignore = "writes 12 MB; cargo test --release -p easy-personal-rs --test ingest_perf -- --ignored --nocapture"]
fn memory_12mb_under_8_mib() {
    let dir = Dir::new("mem");
    let src = dir.0.join("src");
    small_lines(&src.join("p").join("big.jsonl"), 12 * 1_000_000, "m");
    let (_, baseline, _) = run(&dir.0, &["--version"]);
    let (secs, peak, out) = run(&dir.0, &["ingest", "--source", src.to_str().unwrap()]);
    let events = new_events(&out);
    let delta = peak.saturating_sub(baseline);
    println!(
        "T032-MEM {} baseline_kib={baseline} peak_kib={peak} delta_kib={delta} events={events} secs={secs:.2}",
        profile()
    );
    assert!(events > 25_000, "{events} events");
    assert!(delta * 1024 < 8 * MIB, "peak grew {delta} KiB");
}

#[test]
#[ignore = "writes 20 MB; cargo test --release -p easy-personal-rs --test ingest_perf -- --ignored --nocapture"]
fn unchanged_rerun_under_2s() {
    let dir = Dir::new("rerun");
    let src = dir.0.join("src");
    for i in 0..200 {
        let path = src.join(format!("p{}", i % 10)).join(format!("f{i}.jsonl"));
        small_lines(&path, 100_000, &format!("r{i}-"));
    }
    let src = src.to_str().unwrap();
    let (cold, _, first) = run(&dir.0, &["ingest", "--source", src]);
    let (rerun, peak, again) = run(&dir.0, &["ingest", "--source", src]);
    println!(
        "T032-RERUN {} files=200 cold_secs={cold:.2} new_events={} rerun_secs={rerun:.3} rerun_new_events={} rerun_peak_kib={peak}",
        profile(),
        new_events(&first),
        new_events(&again)
    );
    assert!(new_events(&first) > 0);
    assert_eq!(new_events(&again), 0, "an unchanged re-run adds nothing");
    assert!(rerun < 2.0, "unchanged re-run took {rerun:.2} s");
}
