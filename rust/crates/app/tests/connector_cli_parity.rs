//! T060 AC1-AC4: the `connector ...` and `doctor` commands against Python's CLI. `rust/parity/golden-connector-cli/` (made
//! once by `gen_connector_cli.py` from Python's own `main()`) holds each case: the T050 world (store setup, agent folders,
//! keychain items, the fake `claude` and its state, a failing `claude` call, a launcher or none) and its steps (arguments,
//! stdin). For every step the real binary must give Python's exit code, stdout and stderr (Python's `easy-personal: ` is
//! `easy-personal-rs: ` here; argparse's usage text is not compared, only its exit code 2 and the empty stdout). After the
//! case the keychain items, the `claude` writes (`add-json`, `remove`, in order), the fake's state and every table must be
//! what Python left.
//!
//! `claude mcp get` calls are not compared one by one: the CLI works the plan out again after the question (`cli_cmds::change`),
//! so the same reads repeat. They change nothing.
//!
//! The binary runs with no controlling terminal (`setsid`), as the generator does, so a secret is read as Python's
//! `getpass` fallback reads it: from stdin, with its warning and the prompt on stderr. `TZ` is the test's own (`UTC` when
//! unset), so the three-zone run of the workspace covers it.

mod connector_world;

use connector_world::*;
use serde_json::{Value, json};
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

unsafe extern "C" {
    fn setsid() -> i32;
}

fn golden() -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(parity("golden-connector-cli/cases.json")).unwrap(),
    )
    .unwrap()
}

/// `ok keychain: <class or name>` -> `ok keychain: <BACKEND>` (Python prints its backend's class name).
fn backend(text: &str) -> String {
    text.lines()
        .map(|l| {
            if l.starts_with("ok keychain: ") {
                "ok keychain: <BACKEND>"
            } else {
                l
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + if text.ends_with('\n') { "\n" } else { "" }
}

/// Runs one step with no controlling terminal; (exit code, stdout, stderr).
pub fn run_step(
    world: &World,
    binary: &str,
    case: &Value,
    args: &[String],
    stdin: &str,
) -> (i32, String, String) {
    let mut cmd = Command::new(binary);
    world.env(
        &mut cmd,
        case,
        &std::env::var("TZ").unwrap_or_else(|_| "UTC".into()),
    );
    cmd.args(args.iter().map(|a| a.replace("@W@", &world.work)))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // SAFETY: `setsid` is async-signal-safe; it only detaches the child from this terminal (no `/dev/tty` for it)
    unsafe {
        cmd.pre_exec(|| {
            setsid();
            Ok(())
        });
    }
    let mut child = cmd.spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    // the oracle's stdin is `io.StringIO(stdin, newline=None)`: universal newlines, so `\r\n` and a lone `\r` reach
    // Python as `\n`. The real stdin keeps the `\r` (T066; `connector_cli.rs` proves that case), so the same
    // translation is applied here to send what the oracle's Python read.
    let stdin = stdin.replace("\r\n", "\n").replace('\r', "\n");
    let _ = input.write_all(stdin.as_bytes()); // a command that never reads stdin may be gone already
    drop(input);
    let out = child.wait_with_output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The binary a case runs: the build's own, or (no launcher) a copy in a folder where no `easy-personal-launch` is beside it.
fn binary(world: &World, case: &Value) -> String {
    if case["launcher"] == true {
        return SERVE.to_string();
    }
    let dir: PathBuf = world.dir.join("nolauncher");
    std::fs::create_dir_all(&dir).unwrap();
    let copy = dir.join("easy-personal-rs");
    std::fs::copy(SERVE, &copy).unwrap();
    copy.to_str().unwrap().to_string()
}

fn writes(calls: &Value) -> Vec<Value> {
    calls
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["argv"][1] != "get")
        .cloned()
        .collect()
}

fn replay(case: &Value) -> Vec<String> {
    let name = case["name"].as_str().unwrap();
    let world = World::build(case);
    let binary = binary(&world, case);
    let before = dump(&world.store);
    let steps = case["steps"].as_array().unwrap();
    let mut got = Vec::new();
    for step in steps {
        let args: Vec<String> = step["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap().to_string())
            .collect();
        got.push((
            args.clone(),
            run_step(
                &world,
                &binary,
                case,
                &args,
                step["stdin"].as_str().unwrap(),
            ),
        ));
    }
    // the ids this run made, in order of first appearance, against the golden's
    let keychain_after = world.keychain_after();
    let (mut ours, mut theirs) = (Vec::new(), Vec::new());
    for (_, (_, stdout, _)) in &got {
        new_ids(stdout, &world.known, &mut ours);
    }
    new_ids(&keychain_after.to_string(), &world.known, &mut ours);
    for result in case["results"].as_array().unwrap() {
        new_ids(
            result["stdout"].as_str().unwrap(),
            &world.known,
            &mut theirs,
        );
    }
    new_ids(
        &case["keychain_after"].to_string(),
        &world.known,
        &mut theirs,
    );
    let mut problems = Vec::new();
    if ours.len() != theirs.len() {
        problems.push(format!("{name}: new ids {ours:?} vs {theirs:?}"));
    }
    let norm = Norm {
        work: world.work.clone(),
        launcher: LAUNCH.to_string(),
        ids: ours.into_iter().zip(theirs).collect(),
    };
    let nolauncher = format!("{}/nolauncher/easy-personal-rs", world.work);
    for (i, ((args, (code, stdout, stderr)), want)) in got
        .iter()
        .zip(case["results"].as_array().unwrap())
        .enumerate()
    {
        let at = format!("{name} step {i} {args:?}");
        let (stdout, stderr) = (
            backend(&norm.text(stdout)),
            norm.text(&stderr.replace(&nolauncher, "<BIN>"))
                .replace("easy-personal-rs: ", "easy-personal: "),
        );
        if i64::from(*code) != want["code"].as_i64().unwrap() {
            problems.push(format!(
                "{at}: exit {code}, Python {}; stdout {stdout:?} stderr {stderr:?}",
                want["code"]
            ));
            continue;
        }
        if stdout != want["stdout"].as_str().unwrap() {
            problems.push(format!(
                "{at}: stdout differs\n--- Rust\n{stdout}\n--- Python\n{}",
                want["stdout"].as_str().unwrap()
            ));
        }
        if want["stderr_compare"] == "exact" && stderr != want["stderr"].as_str().unwrap() {
            problems.push(format!(
                "{at}: stderr {stderr:?}, Python {:?}",
                want["stderr"]
            ));
        }
    }
    if norm.value(&keychain_after) != case["keychain_after"] {
        problems.push(format!(
            "{name}: keychain {} want {}",
            norm.value(&keychain_after),
            case["keychain_after"]
        ));
    }
    let calls = writes(&Value::Array(read_log(&world.log, &norm)));
    if calls != writes(&case["calls"]) {
        problems.push(format!(
            "{name}: claude writes {calls:?}\n  want {:?}",
            writes(&case["calls"])
        ));
    }
    if !case["claude"].is_null() {
        let after = norm.value(&read_state(&world.state));
        if after != case["claude_after"] {
            problems.push(format!(
                "{name}: claude state {after} want {}",
                case["claude_after"]
            ));
        }
    }
    let after = dump(&world.store);
    problems.extend(table_problems(
        name,
        &before,
        &after,
        &case["changed"],
        &norm,
    ));
    problems
}

#[test]
fn every_connector_and_doctor_step_equals_the_python_cli() {
    let golden = golden();
    assert_eq!(golden["generator"], "rust/parity/gen_connector_cli.py");
    let cases = golden["cases"].as_array().unwrap();
    // the committed file is the generator's output: a dropped case must fail here
    let steps: usize = cases
        .iter()
        .map(|c| c["steps"].as_array().unwrap().len())
        .sum();
    assert_eq!((cases.len(), steps), (95, 116), "cases, steps");
    let problems: Vec<String> = cases.iter().flat_map(replay).collect();
    println!(
        "connector cli oracle: {} cases, {steps} steps, {} different",
        cases.len(),
        problems.len()
    );
    assert!(
        problems.is_empty(),
        "{} problems:\n{}",
        problems.len(),
        problems.join("\n=====\n")
    );
}

/// Every action of Python's `connector` parser and `doctor` is in the record, each with a done and a refused step.
#[test]
fn the_record_covers_every_action_with_an_answer_and_a_refusal() {
    let golden = golden();
    let mut seen = std::collections::BTreeMap::<String, (u32, u32)>::new();
    for case in golden["cases"].as_array().unwrap() {
        for (step, result) in case["steps"]
            .as_array()
            .unwrap()
            .iter()
            .zip(case["results"].as_array().unwrap())
        {
            let args = step["args"].as_array().unwrap();
            let key = if args[0] == "doctor" {
                "doctor".to_string()
            } else {
                format!(
                    "connector {}",
                    args.get(1).and_then(Value::as_str).unwrap_or("")
                )
            };
            let slot = seen.entry(key).or_default();
            if result["code"] == json!(0) {
                slot.0 += 1;
            } else {
                slot.1 += 1;
            }
        }
    }
    for action in [
        "connector list",
        "connector add",
        "connector remove",
        "connector sync",
        "connector move",
        "connector alias",
        "connector adopt-legacy",
        "doctor",
    ] {
        let (ok, refused) = seen.get(action).copied().unwrap_or_default();
        assert!(
            ok > 0 && refused > 0,
            "{action}: {ok} ok, {refused} refused"
        );
    }
}
