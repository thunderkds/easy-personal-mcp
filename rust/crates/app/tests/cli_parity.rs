//! T058: the CLI commands `ping`, `usage`, `outliers`, `kit-findings`, `workspace` and `account` against the Python CLI.
//! `rust/parity/golden-cli/cases.json` (made once by `rust/parity/gen_cli.py` from Python's own `main()`) holds every case:
//! the store it starts from, the SQL and the folder tree it needs, the steps (arguments and stdin) and, for each zone, the
//! exit code, stdout and stderr Python gave. This test builds the same work folder, runs the real binary with the same
//! steps and `TZ`, and requires the same exit code and stdout, and the same stderr where the record says so
//! (`exact`; `prefix`: Python's `easy-personal: ` line with `easy-personal-rs: ` in its place; `none`: argparse's usage).
//! `HOME`, `EASY_PERSONAL_HOME` and `EASY_PERSONAL_AGENT_ROOT` are all inside one temp folder (NFR-002): no real home
//! folder, `~/.claude*` or `~/.easy-personal` is read. The work folder is `<WORK>` in the record; `as_of` is `<NOW>`.

use easy_personal_core::writer::open_for_write;
use rusqlite::Connection;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

fn parity(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity")
        .join(rel)
}

struct Work(PathBuf);

impl Work {
    fn new() -> Work {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t058-parity-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        Work(dir.canonicalize().unwrap())
    }

    fn text(&self) -> &str {
        self.0.to_str().unwrap()
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// `"as_of": "<time>"` -> `"as_of": "<NOW>"` (the one value of an answer that is the clock's).
fn now_token(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find("\"as_of\": \"") {
        let value_start = at + "\"as_of\": \"".len();
        out.push_str(&rest[..value_start]);
        let tail = &rest[value_start..];
        let end = tail.find('"').unwrap_or(tail.len());
        out.push_str("<NOW>");
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

fn build(case: &Value, work: &Work) {
    let text = work.text();
    let sub = |s: &str| s.replace("@W@", text);
    let tree = &case["tree"];
    let mut dirs = vec![
        "agent-root/.claude".to_string(),
        "agent-root/.claude-work".into(),
        "nowhere".into(),
        "home".into(),
    ];
    dirs.extend(
        tree["dirs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d.as_str().unwrap().to_string()),
    );
    for dir in dirs {
        fs::create_dir_all(work.0.join(dir)).unwrap();
    }
    for (rel, content) in tree["files"].as_object().unwrap() {
        let path = work.0.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content.as_str().unwrap()).unwrap();
    }
    if let Some(links) = tree.get("symlinks").and_then(Value::as_object) {
        for (rel, target) in links {
            std::os::unix::fs::symlink(sub(target.as_str().unwrap()), work.0.join(rel)).unwrap();
        }
    }
    let store = work.0.join("home/store.sqlite");
    match case["store"].as_str().unwrap() {
        "rich" => {
            fs::copy(parity("store-rich/store-v11.sqlite"), &store).unwrap();
            let conn = Connection::open(&store).unwrap();
            conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
            conn.execute(
                "UPDATE agent_accounts SET folder = replace(folder, '/work/rich', ?1), \
                 config_dir_env = replace(config_dir_env, '/work/rich', ?1)",
                [text],
            )
            .unwrap();
            for sql in case["setup"].as_array().unwrap() {
                conn.execute(&sub(sql.as_str().unwrap()), []).unwrap();
            }
        }
        "empty" => drop(open_for_write(&store).unwrap()),
        "none" => {}
        other => panic!("unknown store kind {other}"),
    }
}

/// Runs one step; the exit code, stdout and stderr with the work folder as `<WORK>`.
fn run_step(work: &Work, zone: &str, args: &[String], stdin: &str) -> (i32, String, String) {
    let text = work.text();
    let mut child = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
        .args(args.iter().map(|a| a.replace("@W@", text)))
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", work.0.join("nowhere"))
        .env("EASY_PERSONAL_HOME", work.0.join("home"))
        .env("EASY_PERSONAL_AGENT_ROOT", work.0.join("agent-root"))
        .env("TZ", zone)
        .current_dir(&work.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let _ = input.write_all(stdin.as_bytes()); // a command that never reads stdin may be gone already
    drop(input);
    let out = child.wait_with_output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).replace(text, "<WORK>"),
        String::from_utf8_lossy(&out.stderr).replace(text, "<WORK>"),
    )
}

fn check_zone(zone: &str) {
    let manifest: Value =
        serde_json::from_slice(&fs::read(parity("golden-cli/cases.json")).unwrap()).unwrap();
    let (mut ran, mut failures) = (0, Vec::new());
    for case in manifest["cases"].as_array().unwrap() {
        if !case["zones"].as_array().unwrap().iter().any(|z| z == zone) {
            continue;
        }
        let name = case["name"].as_str().unwrap();
        let work = Work::new();
        build(case, &work);
        let steps = case["steps"].as_array().unwrap();
        let wanted = case["results"][zone].as_array().unwrap();
        for (number, (step, want)) in steps.iter().zip(wanted).enumerate() {
            let args: Vec<String> = step["args"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a.as_str().unwrap().to_string())
                .collect();
            let (code, stdout, stderr) =
                run_step(&work, zone, &args, step["stdin"].as_str().unwrap());
            ran += 1;
            let stdout = now_token(&stdout);
            let at = format!("{name} step {number} {args:?} under {zone}");
            if i64::from(code) != want["code"].as_i64().unwrap() {
                failures.push(format!(
                    "{at}: exit {code}, Python {}; stderr {stderr:?}",
                    want["code"]
                ));
                continue;
            }
            if stdout != want["stdout"].as_str().unwrap() {
                failures.push(format!(
                    "{at}: stdout differs\n--- Rust\n{stdout}\n--- Python\n{}",
                    want["stdout"].as_str().unwrap()
                ));
                continue;
            }
            let wanted_err = want["stderr"].as_str().unwrap();
            match want["stderr_compare"].as_str().unwrap() {
                "none" => {}
                "prefix" => {
                    let ours = stderr
                        .strip_prefix("easy-personal-rs: ")
                        .map(|rest| format!("easy-personal: {rest}"));
                    if ours.as_deref() != Some(wanted_err) {
                        failures.push(format!("{at}: stderr {stderr:?}, Python {wanted_err:?}"));
                    }
                }
                _ => {
                    if stderr != wanted_err {
                        failures.push(format!("{at}: stderr {stderr:?}, Python {wanted_err:?}"));
                    }
                }
            }
        }
    }
    assert!(ran > 50, "only {ran} step runs for {zone}");
    assert!(
        failures.is_empty(),
        "{} of {ran} step runs differ from Python under {zone}:\n{}",
        failures.len(),
        failures
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n=====\n")
    );
}

#[test]
fn every_case_equals_the_python_cli_under_utc() {
    check_zone("UTC");
}

#[test]
fn every_case_equals_the_python_cli_under_kiritimati() {
    check_zone("Pacific/Kiritimati");
}

#[test]
fn every_case_equals_the_python_cli_under_pago_pago() {
    check_zone("Pacific/Pago_Pago");
}

#[test]
fn the_record_is_whole() {
    let manifest: Value =
        serde_json::from_slice(&fs::read(parity("golden-cli/cases.json")).unwrap()).unwrap();
    assert_eq!(manifest["generator"], "rust/parity/gen_cli.py");
    let cases = manifest["cases"].as_array().unwrap();
    assert!(cases.len() > 250, "{} cases", cases.len());
    let zones: Vec<&str> = manifest["zones"]
        .as_array()
        .unwrap()
        .iter()
        .map(|z| z.as_str().unwrap())
        .collect();
    assert_eq!(zones, ["UTC", "Pacific/Kiritimati", "Pacific/Pago_Pago"]);
    for case in cases {
        for zone in case["zones"].as_array().unwrap() {
            let results = case["results"][zone.as_str().unwrap()].as_array().unwrap();
            assert_eq!(
                results.len(),
                case["steps"].as_array().unwrap().len(),
                "{}",
                case["name"]
            );
        }
    }
    // every command of the guide is in the record, each with an answer and with a refusal
    for command in ["ping", "usage", "outliers", "kit-findings"] {
        assert!(
            cases.iter().any(|c| c["steps"][0]["args"][0] == command),
            "{command}"
        );
    }
    for (command, action) in [
        ("workspace", "list"),
        ("workspace", "add"),
        ("workspace", "edit"),
        ("workspace", "delete"),
        ("account", "list"),
        ("account", "confirm"),
        ("account", "ignore"),
        ("account", "add"),
        ("account", "use"),
    ] {
        let steps = cases
            .iter()
            .flat_map(|c| {
                c["steps"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .zip(c["results"]["UTC"].as_array().unwrap())
            })
            .filter(|(s, _)| s["args"][0] == command && s["args"][1] == action);
        let (ok, refused) = steps.fold((0, 0), |(ok, refused), (_, r)| match r["code"].as_i64() {
            Some(0) => (ok + 1, refused),
            _ => (ok, refused + 1),
        });
        assert!(
            ok > 0 && refused > 0,
            "{command} {action}: {ok} ok, {refused} refused"
        );
    }
}
