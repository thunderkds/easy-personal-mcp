//! T035 criterion 10, the real run (ignored: needs `rust/parity/.venv`, see `rust/parity/README.md`): Rust and the
//! Python app share one store. Rust ingests the S01 inputs into a fresh store, then the real Python
//! `easy-personal ingest` (the CLI entry the console script runs) reads the same folder into the same store and adds
//! 0 events; then the reverse order: Python first, Rust second adds 0. The child gets only the oracle's fixed env
//! (`gen_ingest.py` `Case.env`), every folder in the temp case.
//!
//! Run: `cargo test -p easy-personal-core --test ingest_folders_python -- --ignored --nocapture`

mod ingest_folders_common;

use ingest_folders_common::{Case, committed, repo};
use serde_json::Value;
use std::process::Command;

const ENTRY: &str = "import sys; from easy_personal.surfaces.cli import main; sys.exit(main())";

/// `easy-personal ingest` in the case (cwd <WORK>); its `files` and `new_events`.
fn python_ingest(case: &Case) -> (i64, i64) {
    let python = repo().join("rust/parity/.venv/bin/python");
    assert!(python.exists(), "no parity venv at rust/parity/.venv");
    let out = Command::new("timeout")
        .arg("120")
        .arg(&python)
        .args(["-P", "-c", ENTRY, "ingest"])
        .current_dir(&case.work)
        .env_clear()
        .env(
            "PATH",
            std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into()),
        )
        .env("HOME", &case.nowhere)
        .env("XDG_CONFIG_HOME", case.nowhere.join(".config"))
        .env("EASY_PERSONAL_HOME", &case.home)
        .env("EASY_PERSONAL_AGENT_ROOT", &case.agents)
        .env(
            "EASY_PERSONAL_VERIFIER_REPORTS",
            case.root.join("verifier-none"),
        )
        .env("EASY_MEMORY_HOME", case.root.join("memory-none"))
        .env("EASY_PERSONAL_CONTEXT_URL", "http://127.0.0.1:9/mcp")
        .env("TZ", "UTC")
        .env("LC_ALL", "C.UTF-8")
        .env("LANG", "C.UTF-8")
        .env("PYTHONHASHSEED", "0")
        .env("PYTHONPATH", repo().join("src"))
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "Python ingest failed: {}",
        case.tok(&stderr)
    );
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    println!(
        "python ingest: exit {} files {} new_events {}",
        out.status.code().unwrap(),
        json["files"],
        json["new_events"]
    );
    (
        json["files"].as_i64().unwrap(),
        json["new_events"].as_i64().unwrap(),
    )
}

#[test]
#[ignore = "needs rust/parity/.venv (the parity venv); run with --ignored"]
fn rust_then_python_adds_nothing() {
    let case = Case::new();
    case.copy(&committed(), &case.projects);
    let rust = case.ingest(None);
    println!(
        "rust ingest: files {} new_events {}",
        rust.files, rust.new_events
    );
    assert_eq!((rust.files, rust.new_events), (6, 39));
    assert_eq!(python_ingest(&case), (6, 0), "Python after Rust adds 0");
}

#[test]
#[ignore = "needs rust/parity/.venv (the parity venv); run with --ignored"]
fn python_then_rust_adds_nothing() {
    let case = Case::new();
    case.copy(&committed(), &case.projects);
    assert_eq!(python_ingest(&case), (6, 39));
    let rust = case.ingest(None);
    println!(
        "rust ingest: files {} new_events {}",
        rust.files, rust.new_events
    );
    assert_eq!(
        (rust.files, rust.new_events),
        (6, 0),
        "Rust after Python adds 0"
    );
}
