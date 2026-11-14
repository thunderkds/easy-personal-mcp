//! T050 AC8: the connector oracle (`rust/parity/golden-connectors/`, made once by `gen_connectors.py` from Python's own
//! handlers) replayed over the real binaries. Per case: one `serve` over a temp copy of `store-rich` with the case's
//! folders and setup SQL, the debug-build fake keychain file (`EASY_PERSONAL_TEST_KEYCHAIN`) holding the case's items,
//! and the same fake `claude` (`golden-connectors/fake-claude.sh`) first on PATH with the same per-account state. Each
//! answer (status, JSON), the keychain items after, every `claude` call (account folder + argv), the fake's state after
//! and every table must equal what Python left. Normalised on both sides: the work folder is `<WORK>`, the launcher path
//! `<LAUNCHER>`, and a connector id the run made is mapped, in order of first appearance, to the golden's.
//! Then `launcher.json`: the items Python's `keychain.store` wrote and what `launch.main` printed, on `easy-personal-launch`.
//!
//! The environment is built from nothing: no real `claude` (PATH is the fake's folder, `/usr/bin` and `/bin`), no real
//! keychain (the fake file, and a session bus address that leads nowhere), temp HOME and data folders.

mod connector_world;

use connector_world::*;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::time::Duration;

fn golden(file: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(parity(&format!("golden-connectors/{file}"))).unwrap(),
    )
    .unwrap()
}

// ---- the server ----

struct Server {
    child: Child,
    _stdout: BufReader<ChildStdout>,
    port: u16,
    token: String,
}

impl Server {
    fn start(cmd: &mut Command) -> Server {
        let mut child = cmd
            .args(["serve", "--no-ingest"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        let rest = line
            .trim_end()
            .strip_prefix("dashboard: http://127.0.0.1:")
            .unwrap_or_else(|| panic!("{line:?}"));
        let (port, token) = rest.split_once("/#").unwrap();
        Server {
            port: port.parse().unwrap(),
            token: token.to_string(),
            child,
            _stdout: stdout,
        }
    }

    /// (status, body) of one request with the good Host and token; `ctype` `None` sends no content type.
    fn send(&self, method: &str, path: &str, ctype: Option<&str>, body: &[u8]) -> (u16, String) {
        let mut head = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nx-dashboard-token: {}\r\n",
            self.port, self.token
        );
        if let Some(ctype) = ctype {
            head += &format!("content-type: {ctype}\r\n");
        }
        head += &format!(
            "Content-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(60)))
            .unwrap();
        let mut request = head.into_bytes();
        request.extend_from_slice(body);
        stream.write_all(&request).unwrap();
        let mut bytes = Vec::new();
        let _ = stream.read_to_end(&mut bytes);
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
        let status = head
            .split(' ')
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        (status, body.to_string())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ---- one case ----

fn replay(case: &Value) -> Vec<String> {
    let name = case["name"].as_str().unwrap();
    let world = World::build(case);
    let (work, store, known) = (world.work.clone(), world.store.clone(), world.known.clone());
    let mut cmd = Command::new(SERVE);
    world.env(&mut cmd, case, "UTC");
    let before = dump(&store);
    let server = Server::start(&mut cmd);
    let mut answers = Vec::new();
    for request in case["requests"].as_array().unwrap() {
        let (status, body) = server.send(
            request["method"].as_str().unwrap(),
            request["path"].as_str().unwrap(),
            request["type"].as_str(),
            &unhex(request["body_hex"].as_str().unwrap()),
        );
        answers.push((status, serde_json::from_str::<Value>(&body).ok()));
    }
    drop(server);
    // the ids this run made, in order of first appearance, against the golden's
    let (mut ours, mut theirs) = (Vec::new(), Vec::new());
    let keychain_after = world.keychain_after();
    for (_, body) in &answers {
        new_ids(
            &body.as_ref().map(Value::to_string).unwrap_or_default(),
            &known,
            &mut ours,
        );
    }
    new_ids(&keychain_after.to_string(), &known, &mut ours);
    for answer in case["answers"].as_array().unwrap() {
        new_ids(&answer["json"].to_string(), &known, &mut theirs);
    }
    new_ids(&case["keychain_after"].to_string(), &known, &mut theirs);
    let mut problems = Vec::new();
    if ours.len() != theirs.len() {
        problems.push(format!("{name}: new ids {ours:?} vs {theirs:?}"));
    }
    let norm = Norm {
        work: work.clone(),
        launcher: LAUNCH.to_string(),
        ids: ours.into_iter().zip(theirs).collect(),
    };
    for (i, ((status, body), want)) in answers
        .iter()
        .zip(case["answers"].as_array().unwrap())
        .enumerate()
    {
        let label = format!(
            "{name} #{i} {} {}",
            case["requests"][i]["method"], case["requests"][i]["path"]
        );
        if u64::from(*status) != want["status"].as_u64().unwrap() {
            problems.push(format!(
                "{label}: status {status} want {}: {body:?}",
                want["status"]
            ));
        } else if body.as_ref().map(|b| norm.value(b))
            != Some(want["json"].clone()).filter(|j| !j.is_null())
        {
            problems.push(format!(
                "{label}: body {:?}\n  want {}",
                body.as_ref().map(|b| norm.value(b)),
                want["json"]
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
    let calls = Value::Array(read_log(&world.log, &norm));
    if calls != case["calls"] {
        problems.push(format!(
            "{name}: claude calls {calls}\n  want {}",
            case["calls"]
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
    let after = dump(&store);
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
fn every_connector_case_equals_what_python_answered_wrote_and_ran() {
    let golden = golden("cases.json");
    let cases = golden["cases"].as_array().unwrap();
    // the committed file is the generator's output: a dropped case must fail here
    assert_eq!(cases.len(), 40, "cases");
    let requests: usize = cases
        .iter()
        .map(|c| c["requests"].as_array().unwrap().len())
        .sum();
    let calls: usize = cases
        .iter()
        .map(|c| c["calls"].as_array().unwrap().len())
        .sum();
    assert_eq!((requests, calls), (102, 175));
    let problems: Vec<String> = cases.iter().flat_map(replay).collect();
    println!(
        "connector oracle: {} cases, {requests} requests, {calls} claude calls, {} different",
        cases.len(),
        problems.len()
    );
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_launcher_reads_pythons_items_and_answers_as_launch_main() {
    let golden = golden("launcher.json");
    let dir = temp("launcher-oracle");
    let file = dir.join("keychain.json");
    std::fs::write(&file, golden["items"].to_string()).unwrap();
    let runs = golden["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 14);
    for run in runs {
        let args: Vec<&str> = run["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap())
            .collect();
        let keychain = if run["keychain"] == "none" {
            Path::new("unusable")
        } else {
            file.as_path()
        };
        let out = Command::new(LAUNCH)
            .args(&args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("EASY_PERSONAL_TEST_KEYCHAIN", keychain)
            .env("DBUS_SESSION_BUS_ADDRESS", NO_BUS)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let got = json!({
            "code": out.status.code(),
            "stdout": String::from_utf8_lossy(&out.stdout),
            "stderr": String::from_utf8_lossy(&out.stderr),
        });
        let want = json!({"code": run["code"], "stdout": run["stdout"], "stderr": run["stderr"]});
        assert_eq!(got, want, "{args:?} ({})", run["keychain"]);
    }
    // the bytes Rust writes for the same item are Python's (the server's `keychain::store` uses this text)
    let slack = golden["items"]["easy-personal/slack"].as_str().unwrap();
    assert!(
        slack.starts_with("{\"command\": [\"npx\", \"-y\", \"slack-mcp-server@1.3.0\""),
        "{slack}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
