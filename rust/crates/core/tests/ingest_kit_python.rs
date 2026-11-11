//! T033 criterion 6, the real run (ignored: needs `rust/parity/.venv`, see `rust/parity/README.md`): Rust and the Python app
//! share one store. For several `golden-kit` scenarios, up to their first `ingest` step: Rust ingests, then the real Python
//! `easy-personal ingest` over the same folders adds 0 `kit_trace` rows and leaves the three tables equal; then the reverse.
//!
//! Run: `cargo test -p easy-personal-core --test ingest_kit_python -- --ignored --nocapture`

mod ingest_kit_common;

use ingest_kit_common::common::repo;
use ingest_kit_common::{Kit, kit_golden};
use serde_json::{Value, json};
use std::process::Command;

const ENTRY: &str = "import sys; from easy_personal.surfaces.cli import main; sys.exit(main())";

/// The first `ingest` step of the scenario, after playing every step before it.
fn setup(name: &str) -> (Kit, Value) {
    let g = kit_golden(name);
    let kit = Kit::new();
    for step in g["steps"].as_array().unwrap() {
        if step["op"] == "ingest" {
            return (kit, step.clone());
        }
        kit.step(step);
    }
    panic!("no ingest step");
}

/// `easy-personal ingest` with the oracle's env plus the step's folders; the K objects it printed.
fn python(kit: &Kit, step: &Value) -> Value {
    let python = repo().join("rust/parity/.venv/bin/python");
    assert!(python.exists(), "no parity venv at rust/parity/.venv");
    let c = &kit.case;
    let mut cmd = Command::new("timeout");
    cmd.arg("120")
        .arg(&python)
        .args(["-P", "-c", ENTRY, "ingest"])
        .current_dir(&c.work)
        .env_clear()
        .env(
            "PATH",
            std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into()),
        )
        .env("HOME", &c.nowhere)
        .env("XDG_CONFIG_HOME", c.nowhere.join(".config"))
        .env("EASY_PERSONAL_HOME", &c.home)
        .env("EASY_PERSONAL_AGENT_ROOT", &c.agents)
        .env("EASY_PERSONAL_VERIFIER_REPORTS", kit.expand("<VERIFIER>"))
        .env("EASY_MEMORY_HOME", c.root.join("memory-none"))
        .env("EASY_PERSONAL_CONTEXT_URL", "http://127.0.0.1:9/mcp")
        .env("TZ", "UTC")
        .env("LC_ALL", "C.UTF-8")
        .env("LANG", "C.UTF-8")
        .env("PYTHONHASHSEED", "0")
        .env("PYTHONPATH", repo().join("src"))
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null");
    if let Some(env) = step.get("env").and_then(Value::as_object) {
        for (k, v) in env {
            match v {
                Value::String(v) => cmd.env(k, kit.expand(v)),
                _ => cmd.env_remove(k),
            };
        }
    }
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "Python ingest failed: {}",
        c.tok(&String::from_utf8_lossy(&out.stderr))
    );
    let printed: Value = serde_json::from_slice(&out.stdout).unwrap();
    println!(
        "python: {}",
        json!({"kit_trace": printed["kit_trace"], "ui": printed["ui"], "verifier": printed["verifier"]})
    );
    printed
}

fn check(name: &str) {
    // Rust first, Python second
    let (kit, step) = setup(name);
    let rust = kit.ingest(&step);
    let before = kit.dump();
    let py = python(&kit, &step);
    assert_eq!(
        py["kit_trace"]["new_events"], 0,
        "{name}: Python after Rust adds 0 kit_trace rows"
    );
    assert_eq!(py["kit_trace"]["files"], rust.kit_trace["files"], "{name}");
    assert_eq!(py["ui"], rust.ui, "{name}");
    assert_eq!(py["verifier"], rust.verifier, "{name}");
    assert_eq!(
        kit.dump(),
        before,
        "{name}: Python after Rust leaves the tables equal"
    );
    // Python first, Rust second
    let (kit, step) = setup(name);
    let py = python(&kit, &step);
    let after_python = kit.dump();
    let rust = kit.ingest(&step);
    assert_eq!(
        rust.kit_trace["new_events"], 0,
        "{name}: Rust after Python adds 0 kit_trace rows"
    );
    assert_eq!(rust.kit_trace["files"], py["kit_trace"]["files"], "{name}");
    assert_eq!(rust.ui, py["ui"], "{name}");
    assert_eq!(rust.verifier, py["verifier"], "{name}");
    assert_eq!(
        kit.dump(),
        after_python,
        "{name}: Rust after Python leaves the tables equal"
    );
}

#[test]
#[ignore = "needs rust/parity/.venv"]
fn rust_and_python_share_the_kit_tables() {
    for name in [
        "K01_kit_trace_lines.json",
        "K03_kit_trace_projects_gone.json",
        "K04_kit_trace_symlinks.json",
        "K05_ui_files.json",
        "K07_verifier_reports.json",
    ] {
        check(name);
    }
}
