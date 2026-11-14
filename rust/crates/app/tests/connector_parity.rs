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

use rusqlite::Connection;
use rusqlite::types::ValueRef;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

const SERVE: &str = env!("CARGO_BIN_EXE_easy-personal-rs");
const LAUNCH: &str = env!("CARGO_BIN_EXE_easy-personal-launch");
const NO_BUS: &str = "unix:path=/nonexistent/t050-no-bus";

fn parity(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity")
        .join(rel)
}

fn golden(file: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(parity(&format!("golden-connectors/{file}"))).unwrap(),
    )
    .unwrap()
}

fn temp(tag: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "easy-personal-rs-t050-{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::canonicalize(dir).unwrap()
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

// ---- normalising ----

/// Connector ids this run made (`c` + 8 lowercase hex digits, not in `known`), in order of first appearance.
fn new_ids(text: &str, known: &[String], into: &mut Vec<String>) {
    let b = text.as_bytes();
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'-';
    for i in 0..b.len().saturating_sub(8) {
        let token = &text[i..i + 9];
        let hex = token[1..]
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c));
        let bounded = (i == 0 || !word(b[i - 1])) && b.get(i + 9).is_none_or(|c| !word(*c));
        if b[i] == b'c'
            && hex
            && bounded
            && !known.iter().any(|k| k == token)
            && !into.iter().any(|k| k == token)
        {
            into.push(token.to_string());
        }
    }
}

struct Norm {
    work: String,
    launcher: String,
    ids: Vec<(String, String)>,
}

impl Norm {
    fn text(&self, s: &str) -> String {
        let mut s = s
            .replace(&self.work, "<WORK>")
            .replace(&self.launcher, "<LAUNCHER>");
        for (ours, theirs) in &self.ids {
            s = s.replace(ours, theirs);
        }
        s
    }

    fn value(&self, v: &Value) -> Value {
        match v {
            Value::String(s) => Value::String(self.text(s)),
            Value::Array(items) => Value::Array(items.iter().map(|i| self.value(i)).collect()),
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(k, v)| (self.text(k), self.value(v)))
                    .collect(),
            ),
            other => other.clone(),
        }
    }
}

fn dump(path: &Path) -> BTreeMap<String, Vec<Value>> {
    let conn = Connection::open(path).unwrap();
    let names: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let mut out = BTreeMap::new();
    for name in names {
        let mut stmt = conn.prepare(&format!("SELECT * FROM \"{name}\"")).unwrap();
        let width = stmt.column_count();
        let rows: Vec<Value> = stmt
            .query_map([], |r| {
                Ok(Value::Array(
                    (0..width)
                        .map(|i| match r.get_ref(i).unwrap() {
                            ValueRef::Null => Value::Null,
                            ValueRef::Integer(n) => json!(n),
                            ValueRef::Real(f) => json!(f),
                            ValueRef::Text(t) => json!(String::from_utf8_lossy(t)),
                            ValueRef::Blob(b) => {
                                json!(b.iter().map(|x| format!("{x:02x}")).collect::<String>())
                            }
                        })
                        .collect(),
                ))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        out.insert(name, rows);
    }
    out
}

/// Python's `json.dumps(value)` key used to sort rows (`sort_keys` off, default separators).
fn py_sort_key(v: &Value) -> String {
    match v {
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(py_sort_key).collect::<Vec<_>>().join(", ")
        ),
        Value::String(s) => easy_personal_core::pyjson::py_str(s),
        other => other.to_string(),
    }
}

/// The fake `claude`'s log as `[{"ccd", "argv"}]`, normalised, each run of `mcp get` calls sorted as the generator sorts
/// it (by Python's `json.dumps({"ccd": ..., "argv": [...]})`).
fn read_log(path: &Path, norm: &Norm) -> Vec<Value> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut calls: Vec<(String, Vec<String>)> = Vec::new();
    let mut cur: Option<(String, Vec<String>)> = None;
    for line in text.lines() {
        match cur.take() {
            None => {
                cur = Some((
                    norm.text(line.strip_prefix("CCD=").unwrap_or(line)),
                    Vec::new(),
                ))
            }
            Some(call) if line == "--" => calls.push(call),
            Some((ccd, mut argv)) => {
                argv.push(norm.text(line));
                cur = Some((ccd, argv));
            }
        }
    }
    let py = easy_personal_core::pyjson::py_str;
    let key = |(ccd, argv): &(String, Vec<String>)| {
        let argv: Vec<String> = argv.iter().map(|a| py(a)).collect();
        format!("{{\"ccd\": {}, \"argv\": [{}]}}", py(ccd), argv.join(", "))
    };
    let (mut out, mut run) = (Vec::new(), Vec::new());
    for call in calls.into_iter().map(Some).chain([None]) {
        if let Some(c) = call
            .as_ref()
            .filter(|(_, argv)| argv.len() > 1 && argv[0] == "mcp" && argv[1] == "get")
        {
            run.push(c.clone());
            continue;
        }
        run.sort_by_key(key);
        out.append(&mut run);
        out.extend(call);
    }
    out.into_iter()
        .map(|(ccd, argv)| json!({"ccd": ccd, "argv": argv}))
        .collect()
}

/// The fake's folder name for an account (`fake-claude.sh`): every byte outside `[A-Za-z0-9._-]` as `_`.
fn ccd_key(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"._-".contains(&b) {
                b as char
            } else {
                '_'
            }
        })
        .collect()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn read_state(dir: &Path) -> Value {
    let mut out = Map::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Value::Object(out);
    };
    let mut folders: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    folders.sort();
    for folder in folders {
        let ccd = std::fs::read_to_string(folder.join(".ccd")).unwrap();
        let mut files: Vec<PathBuf> = std::fs::read_dir(&folder)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.file_name().is_some_and(|n| n != ".ccd"))
            .collect();
        files.sort();
        let entries: Map<String, Value> = files
            .iter()
            .map(|f| {
                (
                    f.file_name().unwrap().to_string_lossy().into_owned(),
                    json!(std::fs::read_to_string(f).unwrap()),
                )
            })
            .collect();
        out.insert(ccd, Value::Object(entries));
    }
    Value::Object(out)
}

// ---- one case ----

fn replay(case: &Value) -> Vec<String> {
    let name = case["name"].as_str().unwrap();
    let dir = temp("case");
    let work = dir.to_str().unwrap().to_string();
    for d in case["dirs"].as_array().unwrap() {
        std::fs::create_dir_all(dir.join(d.as_str().unwrap())).unwrap();
    }
    for sub in ["home", "nowhere", "bin"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    let store = dir.join("home/store.sqlite");
    std::fs::copy(parity("store-rich/store-v11.sqlite"), &store).unwrap();
    let mut known = Vec::new();
    {
        let conn = Connection::open(&store).unwrap();
        conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))
            .unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        conn.execute(
            "UPDATE agent_accounts SET folder = replace(folder, '/work/rich', ?1), config_dir_env = replace(config_dir_env, '/work/rich', ?1)",
            [&work],
        )
        .unwrap();
        for sql in case["setup"].as_array().unwrap() {
            conn.execute(&sql.as_str().unwrap().replace("@W@", &work), [])
                .unwrap();
        }
        let mut stmt = conn.prepare("SELECT id FROM connectors").unwrap();
        known.extend(
            stmt.query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .flatten(),
        );
    }
    let keychain = dir.join("keychain.json");
    let keychain_env: PathBuf = match &case["keychain"] {
        Value::String(none) if none == "none" => PathBuf::from("unusable"),
        Value::Object(items) => {
            std::fs::write(&keychain, Value::Object(items.clone()).to_string()).unwrap();
            keychain.clone()
        }
        other => panic!("{name}: keychain {other}"),
    };
    let (state, log) = (dir.join("claude-state"), dir.join("claude.log"));
    if let Value::Object(accounts) = &case["claude"] {
        std::fs::copy(
            parity("golden-connectors/fake-claude.sh"),
            dir.join("bin/claude"),
        )
        .unwrap();
        for (ccd, entries) in accounts {
            let value = ccd.replace("@W@", &work);
            let folder = state.join(ccd_key(&value));
            std::fs::create_dir_all(&folder).unwrap();
            std::fs::write(folder.join(".ccd"), &value).unwrap();
            for (alias, entry) in entries.as_object().unwrap() {
                let command = entry[0].as_str().unwrap().replace("@LAUNCHER@", LAUNCH);
                std::fs::write(
                    folder.join(alias),
                    format!("Command: {command}\nArgs: {}\n", entry[1].as_str().unwrap()),
                )
                .unwrap();
            }
        }
    }
    let mut cmd = Command::new(SERVE);
    cmd.env_clear()
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", dir.join("bin").display()),
        )
        .env("HOME", dir.join("nowhere"))
        .env("XDG_CONFIG_HOME", dir.join("nowhere/.config"))
        .env("EASY_PERSONAL_HOME", dir.join("home"))
        .env("EASY_PERSONAL_AGENT_ROOT", dir.join("agent-root"))
        .env("EASY_PERSONAL_TEST_KEYCHAIN", &keychain_env)
        .env("DBUS_SESSION_BUS_ADDRESS", NO_BUS)
        .env("FAKE_CLAUDE_LOG", &log)
        .env("FAKE_CLAUDE_STATE", &state)
        .env("FAKE_CLAUDE_STDERR", "")
        .env("CLAUDE_CONFIG_DIR", "/runner/dir")
        .env("TZ", "UTC")
        .current_dir(&dir);
    if let Some(fail) = case["fail"].as_str() {
        cmd.env("FAKE_CLAUDE_FAIL", fail);
    }
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
    let keychain_after: Value = if keychain_env == Path::new("unusable") {
        json!("none")
    } else {
        serde_json::from_str(&std::fs::read_to_string(&keychain).unwrap_or_else(|_| "{}".into()))
            .unwrap()
    };
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
    let calls = Value::Array(read_log(&log, &norm));
    if calls != case["calls"] {
        problems.push(format!(
            "{name}: claude calls {calls}\n  want {}",
            case["calls"]
        ));
    }
    if !case["claude"].is_null() {
        let after = norm.value(&read_state(&state));
        if after != case["claude_after"] {
            problems.push(format!(
                "{name}: claude state {after} want {}",
                case["claude_after"]
            ));
        }
    }
    let after = dump(&store);
    for (table, rows) in &after {
        let mut rows: Vec<Value> = rows.iter().map(|r| norm.value(r)).collect();
        rows.sort_by_key(py_sort_key);
        let want: Vec<Value> = match case["changed"].get(table) {
            Some(golden) => golden.as_array().unwrap().clone(),
            None => {
                let mut b: Vec<Value> = before[table].iter().map(|r| norm.value(r)).collect();
                b.sort_by_key(py_sort_key);
                b
            }
        };
        if rows != want {
            problems.push(format!("{name}: table {table} {rows:?}\n  want {want:?}"));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
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
