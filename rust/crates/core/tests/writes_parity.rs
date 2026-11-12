//! T048: the repo-path rules of `workspaces.py` (`normalise_repo`: `~`, absolute, `Path.resolve()`), replayed against the
//! answers Python gave in `rust/parity/golden-writes/paths.json` over a folder with real, relative, dangling, looping and
//! chained links. The workspace and account writes themselves are replayed over HTTP in `crates/app/tests/writes.rs`.

use easy_personal_core::workspaces::{WriteFail, normalise_repo};
use serde_json::Value;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

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
    work
}

#[test]
fn normalise_repo_equals_python_over_links_dots_and_bad_input() {
    let work = work_folder();
    let work_text = work.to_str().unwrap();
    let corpus = golden();
    let cases = corpus["cases"].as_array().unwrap();
    assert!(cases.len() >= 30);
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
