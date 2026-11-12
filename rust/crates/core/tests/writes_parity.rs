//! T048: the repo-path rules of `workspaces.py` (`normalise_repo`: `~`, absolute, `Path.resolve()`), replayed against the
//! answers Python gave in `rust/parity/golden-writes/paths.json` over a folder with real, relative, dangling, looping and
//! chained links. The workspace and account writes themselves are replayed over HTTP in `crates/app/tests/writes.rs`.

use easy_personal_core::workspaces::{WriteFail, normalise_repo};
use serde_json::Value;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn golden() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/golden-writes/paths.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn work_folder() -> PathBuf {
    let work = std::env::temp_dir().join(format!(
        "easy-personal-rs-t048-paths-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(work.join("real")).unwrap();
    std::fs::create_dir(work.join("deep")).unwrap();
    std::fs::write(work.join("file"), "").unwrap();
    let work = std::fs::canonicalize(work).unwrap();
    symlink(work.join("real"), work.join("link")).unwrap();
    symlink("real", work.join("rel")).unwrap();
    symlink(work.join("nowhere"), work.join("dangle")).unwrap();
    symlink(work.join("loop-b"), work.join("loop-a")).unwrap();
    symlink(work.join("loop-a"), work.join("loop-b")).unwrap();
    symlink(work.join("link"), work.join("chain1")).unwrap();
    symlink(work.join("deep").join(".."), work.join("dir-link")).unwrap();
    // T054: a chain of 30 links that doubles at every hop, a loop of three links, a link to itself (as `gen_writes.py`)
    for i in 0..30 {
        symlink(
            format!("hop{}/hop{}", i + 1, i + 1),
            work.join(format!("hop{i}")),
        )
        .unwrap();
    }
    symlink(".", work.join("hop30")).unwrap();
    symlink("tri-b", work.join("tri-a")).unwrap();
    symlink("tri-c", work.join("tri-b")).unwrap();
    symlink("tri-a", work.join("tri-c")).unwrap();
    symlink("selfie", work.join("selfie")).unwrap();
    work
}

#[test]
fn normalise_repo_equals_python_over_links_dots_and_bad_input() {
    let work = work_folder();
    let work_text = work.to_str().unwrap();
    let corpus = golden();
    let cases = corpus["cases"].as_array().unwrap();
    // the committed file is the generator's output: 37 cases of T048 and the 7 of T054 (links doubling, a loop, a self-link)
    assert_eq!(cases.len(), 44);
    for case in cases {
        let input = match &case["input"] {
            Value::String(text) => Value::String(text.replace("@W@", work_text)),
            other => other.clone(),
        };
        let got = match normalise_repo(&input) {
            Ok(path) => serde_json::json!({ "ok": path.replace(work_text, "<WORK>") }),
            Err(WriteFail::Refused { status: 400, text }) => serde_json::json!({ "error": text }),
            Err(other) => panic!("{other:?}"),
        };
        assert_eq!(got["ok"], case["ok"], "{}", case["input"]);
        assert_eq!(got["error"], case["error"], "{}", case["input"]);
    }
    let _ = std::fs::remove_dir_all(&work);
}

/// A hostile chain `hop_i -> hop_{i+1}/hop_{i+1}`, `hop_n -> .` expands 2^n times without a cache of resolved links
/// (T048 held the write lock for it). Resolved once per link it is instant. The work runs on a thread so a regression is a
/// failed assertion after 10 s, not a test run that never ends.
#[test]
fn a_chain_of_links_that_doubles_at_every_hop_is_resolved_in_linear_time() {
    let work = std::env::temp_dir().join(format!(
        "easy-personal-rs-t054-chain-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    let work = std::fs::canonicalize(work).unwrap();
    const DEPTH: usize = 30;
    for i in 0..DEPTH {
        symlink(
            format!("hop{}/hop{}", i + 1, i + 1),
            work.join(format!("hop{i}")),
        )
        .unwrap();
    }
    symlink(".", work.join(format!("hop{DEPTH}"))).unwrap();
    let (done, answer) = mpsc::channel();
    let input = serde_json::json!(format!("{}/hop0/x", work.display()));
    let started = Instant::now();
    std::thread::spawn(move || {
        let _ = done.send(normalise_repo(&input).ok());
    });
    let got = answer.recv_timeout(Duration::from_secs(10));
    let elapsed = started.elapsed();
    let expected = format!("{}/x", work.display());
    let _ = std::fs::remove_dir_all(&work);
    assert_eq!(got, Ok(Some(expected)), "no answer in 10 s");
    assert!(elapsed < Duration::from_secs(1), "took {elapsed:?}");
}

/// A resolved path that is not UTF-8 would be stored as U+FFFD, a path that matches no folder (Python answers 500): refused.
#[test]
fn a_repo_that_resolves_to_a_path_that_is_not_utf8_is_refused() {
    use std::os::unix::ffi::OsStrExt;
    let work =
        std::env::temp_dir().join(format!("easy-personal-rs-t054-utf8-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    let work = std::fs::canonicalize(work).unwrap();
    let bad = work.join(std::ffi::OsStr::from_bytes(b"bad\xff"));
    std::fs::create_dir(&bad).unwrap();
    symlink(&bad, work.join("l")).unwrap();
    let got = normalise_repo(&serde_json::json!(format!("{}/l", work.display())));
    let _ = std::fs::remove_dir_all(&work);
    match got {
        Err(WriteFail::Refused { status: 400, text }) => {
            assert_eq!(text, "repo path must be a non-empty string");
        }
        other => panic!("{other:?}"),
    }
}
