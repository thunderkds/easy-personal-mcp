//! T048: the repo-path rules of `workspaces.py` (`normalise_repo`: `~`, absolute, `Path.resolve()`), replayed against the
//! answers Python gave in `rust/parity/golden-writes/paths.json` over a folder with real, relative, dangling, looping and
//! chained links. The workspace and account writes themselves are replayed over HTTP in `crates/app/tests/writes.rs`.

use easy_personal_core::workspaces::{WriteFail, normalise_repo};
use paths_common::{Temp, links_folder};
use serde_json::Value;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[path = "paths_common/mod.rs"]
mod paths_common;

fn golden() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/golden-writes/paths.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn normalise_repo_equals_python_over_links_dots_and_bad_input() {
    let work = links_folder("paths");
    let work_text = work.text();
    let corpus = golden();
    let cases = corpus["cases"].as_array().unwrap();
    // the committed file is the generator's output: 37 cases of T048, the 7 of T054 (links doubling, a loop, a self-link)
    // and the 9 of T055 (chains of 41 and 60 links, loops with a tail or a `..`, a `..` after a dangling link)
    assert_eq!(cases.len(), 53);
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
}

/// A hostile chain `hop_i -> hop_{i+1}/hop_{i+1}`, `hop_n -> .` expands 2^n times without a cache of resolved links
/// (T048 held the write lock for it). Resolved once per link it is instant. The work runs on a thread so a regression is a
/// failed assertion after 10 s, not a test run that never ends.
#[test]
fn a_chain_of_links_that_doubles_at_every_hop_is_resolved_in_linear_time() {
    let work = Temp::new("chain");
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
    assert_eq!(got, Ok(Some(expected)), "no answer in 10 s");
    assert!(elapsed < Duration::from_secs(1), "took {elapsed:?}");
}

/// A resolved path that is not UTF-8 would be stored as U+FFFD, a path that matches no folder (Python answers 500): refused.
#[test]
fn a_repo_that_resolves_to_a_path_that_is_not_utf8_is_refused() {
    use std::os::unix::ffi::OsStrExt;
    let work = Temp::new("utf8");
    let bad = work.join(std::ffi::OsStr::from_bytes(b"bad\xff"));
    std::fs::create_dir(&bad).unwrap();
    symlink(&bad, work.join("l")).unwrap();
    let got = normalise_repo(&serde_json::json!(format!("{}/l", work.display())));
    match got {
        Err(WriteFail::Refused { status: 400, text }) => {
            assert_eq!(text, "repo path must be a non-empty string");
        }
        other => panic!("{other:?}"),
    }
}
