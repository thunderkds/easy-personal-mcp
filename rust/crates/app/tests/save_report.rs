//! T047: the `save_report` tool and the `investigate` prompt over the real `mcp` binary (stdio), and the saved report
//! seen through the real `serve` binary's `/api/reports`. The Python goldens: `mcp__tools_list.json` (the tool),
//! `mcp__prompt_investigate__{all,scoped}.json` (the prompt text). The oracle for what is stored is in
//! `crates/core/tests/save_report_parity.rs`; this file proves the surface: listing, argument errors, that no answer or
//! stderr echoes the caller's text, that every other tool leaves the store file untouched, and that a save while an
//! `ingest` runs on the same store leaves both consistent.
//!
//! Every store is a temp copy of a parity store; `HOME` and `EASY_PERSONAL_HOME` are temp folders (env built from nothing),
//! so the owner's real data folder is never reached. The helpers are trimmed copies of those of `tests/connectors.rs`.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

fn parity(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity")
        .join(rel)
}

fn read_json(rel: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(parity(rel)).unwrap()).unwrap()
}

/// A temp data folder (`HOME` and `EASY_PERSONAL_HOME`) holding a copy of a parity store.
struct Home {
    base: PathBuf,
    dir: PathBuf,
}

impl Home {
    fn new(store: Option<&str>) -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let base = std::env::temp_dir().join(format!(
            "easy-personal-rs-t047app-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let dir = base.join("home");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(base.join("agents")).unwrap();
        if let Some(store) = store {
            std::fs::copy(parity(store), dir.join("store.sqlite")).unwrap();
        }
        Home { base, dir }
    }

    fn rich() -> Self {
        Home::new(Some("store-rich/store-v11.sqlite"))
    }

    fn store(&self) -> PathBuf {
        self.dir.join("store.sqlite")
    }

    fn command(&self, subcommand: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"));
        command
            .arg(subcommand)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", &self.dir)
            .env("EASY_PERSONAL_HOME", &self.dir)
            .env("EASY_PERSONAL_AGENT_ROOT", self.base.join("agents"))
            .env("TZ", std::env::var("TZ").unwrap_or_else(|_| "UTC".into()));
        command
    }

    fn rows(&self) -> i64 {
        rusqlite::Connection::open(self.store())
            .unwrap()
            .query_row("SELECT count(*) FROM investigate_reports", [], |r| r.get(0))
            .unwrap()
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

struct Mcp {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    next_id: u64,
    stderr: Receiver<String>,
    home: Home,
    /// The `result` of `initialize`.
    init: Value,
}

impl Mcp {
    fn started(home: Home) -> Self {
        let mut child = home
            .command("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let mut stderr_pipe = child.stderr.take().unwrap();
        let (tx, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let (etx, stderr) = channel();
        std::thread::spawn(move || {
            let mut text = String::new();
            let _ = stderr_pipe.read_to_string(&mut text);
            let _ = etx.send(text);
        });
        let mut mcp = Mcp {
            child,
            stdin,
            lines,
            next_id: 1,
            stderr,
            home,
            init: Value::Null,
        };
        let answer = mcp.request(
            "initialize",
            json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t047-test", "version": "0"}}),
        );
        assert!(answer["result"].is_object(), "initialize failed: {answer}");
        mcp.init = answer["result"].clone();
        mcp.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        mcp
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        let line = self
            .lines
            .recv_timeout(Duration::from_secs(20))
            .expect("no answer within 20 s");
        let answer: Value =
            serde_json::from_str(&line).unwrap_or_else(|_| panic!("not JSON: {line:?}"));
        assert_eq!(answer["id"], id, "answer for another request: {answer}");
        answer
    }

    fn call(&mut self, name: &str, arguments: Value) -> Value {
        self.request("tools/call", json!({"name": name, "arguments": arguments}))
    }

    /// Ends the server (stdin closed) and returns everything it wrote to stderr.
    fn finish(&mut self) -> String {
        drop(self.stdin.take());
        let text = self
            .stderr
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_default();
        let _ = self.child.wait();
        text
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn error_text(answer: &Value) -> String {
    assert_eq!(
        answer["result"]["isError"], true,
        "expected a tool error: {answer}"
    );
    answer["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn the_tool_is_listed_with_pythons_description_and_schemas_and_the_prompt_capability_is_advertised()
{
    let mut mcp = Mcp::started(Home::rich());
    let tools = mcp.request("tools/list", json!({}));
    let tools = tools["result"]["tools"].as_array().unwrap();
    let got = tools
        .iter()
        .find(|t| t["name"] == "save_report")
        .expect("save_report is not listed");
    let golden = read_json("golden-rich/mcp__tools_list.json");
    let want = golden
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "save_report")
        .unwrap();
    for key in ["description", "inputSchema", "outputSchema"] {
        assert_eq!(got[key], want[key], "{key}");
    }
    let listed: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(
        listed.contains(&"save_report") && listed.len() == 12,
        "{listed:?}"
    );
    assert!(
        mcp.init["capabilities"]["prompts"].is_object(),
        "{}",
        mcp.init
    );
    let prompts = mcp.request("prompts/list", json!({}));
    assert_eq!(
        prompts["result"]["prompts"],
        json!([{
            "name": "investigate",
            "description": "Investigate your Claude Code usage: causes of each warning and up to 5 ranked fixes.",
            "arguments": [{"name": "workspace", "required": false}, {"name": "account", "required": false}]
        }])
    );
}

fn prompt_text(mcp: &mut Mcp, arguments: Value) -> Value {
    let answer = mcp.request(
        "prompts/get",
        json!({"name": "investigate", "arguments": arguments}),
    );
    assert!(answer["result"].is_object(), "{answer}");
    answer
}

#[test]
fn the_investigate_prompt_equals_the_two_goldens_and_quotes_the_scope_like_json_dumps() {
    let mut mcp = Mcp::started(Home::rich());
    let mut equal = 0;
    for (file, arguments) in [("all", json!({})), ("scoped", json!({"workspace": "acme"}))] {
        let want = read_json(&format!("golden-rich/mcp__prompt_investigate__{file}.json"));
        let answer = prompt_text(&mut mcp, arguments);
        let message = &answer["result"]["messages"][0];
        assert_eq!(message["role"], "user");
        assert_eq!(message["content"]["type"], "text");
        assert_eq!(message["content"]["text"], want["text"], "{file}");
        assert_eq!(
            answer["result"]["description"],
            "Investigate your Claude Code usage: causes of each warning and up to 5 ranked fixes."
        );
        equal += 1;
    }
    assert_eq!(equal, 2, "two goldens");
    // empty and null arguments are none; both together, quoted as `json.dumps(ensure_ascii=False)` does
    let all = read_json("golden-rich/mcp__prompt_investigate__all.json");
    for arguments in [json!({"workspace": "", "account": null}), json!({})] {
        assert_eq!(
            prompt_text(&mut mcp, arguments)["result"]["messages"][0]["content"]["text"],
            all["text"]
        );
    }
    let answer = prompt_text(
        &mut mcp,
        json!({"workspace": "a\"b \u{e9}\u{4e2d}\n", "account": "x\\y"}),
    );
    let text = answer["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap();
    assert!(
        text.contains("\nScope: pass workspace=\"a\\\"b \u{e9}\u{4e2d}\\n\" and account=\"x\\\\y\" to every tool call that takes a scope (`warnings`, `improvement_plan`, `save_report`).\n\n7. Save the report"),
        "{text}"
    );
    // a wrong type and an unknown prompt are errors, not text
    let bad = mcp.request(
        "prompts/get",
        json!({"name": "investigate", "arguments": {"workspace": 5}}),
    );
    assert!(bad["error"].is_object(), "{bad}");
    let unknown = mcp.request("prompts/get", json!({"name": "nope"}));
    assert_eq!(unknown["error"]["message"], "Unknown prompt: nope");
}

#[test]
fn a_call_stores_the_report_and_both_dashboards_routes_show_it() {
    let mut mcp = Mcp::started(Home::rich());
    let answer = mcp.call(
        "save_report",
        json!({"summary": "Cache reuse is low on the big project.", "fixes": [{"text": "Keep the prefix stable.", "tool": "warnings", "number": 0.19}], "workspace": "acme"}),
    );
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    let body = &answer["result"]["structuredContent"];
    assert_eq!(body["id"], 3);
    let saved = body["saved_utc"].as_str().unwrap();
    assert!(
        saved.len() == 20 && saved.ends_with('Z') && saved.as_bytes()[10] == b'T',
        "{saved}"
    );
    let text: Value =
        serde_json::from_str(answer["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(&text, body);
    assert_eq!(mcp.home.rows(), 3);
    let home_dir = mcp.home.dir.clone();
    let stderr = mcp.finish();
    assert_eq!(stderr, "", "the MCP server logs nothing");
    // the dashboard's own server reads it back
    let mut server = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
        .arg("serve")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", &home_dir)
        .env("EASY_PERSONAL_HOME", &home_dir)
        .env("TZ", "UTC")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(server.stdout.take().unwrap());
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let url = line
        .trim_end()
        .strip_prefix("dashboard: http://127.0.0.1:")
        .unwrap()
        .to_string();
    let (port, token) = url.split_once("/#").unwrap();
    let get = |path: &str| -> Value {
        let mut stream = TcpStream::connect(("127.0.0.1", port.parse::<u16>().unwrap())).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(stream, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nx-dashboard-token: {token}\r\nConnection: close\r\n\r\n").unwrap();
        let mut bytes = String::new();
        let _ = stream.read_to_string(&mut bytes);
        serde_json::from_str(bytes.split_once("\r\n\r\n").unwrap().1).unwrap()
    };
    let list = get("/api/reports");
    let one = get("/api/reports/3");
    let _ = server.kill();
    let _ = server.wait();
    assert_eq!(list["reports"][0]["id"], 3, "{list}");
    assert_eq!(
        list["reports"][0]["summary_first_line"],
        "Cache reuse is low on the big project."
    );
    assert_eq!(list["reports"][0]["workspace"], "acme");
    assert_eq!(one["summary"], "Cache reuse is low on the big project.");
    assert_eq!(
        one["fixes"],
        json!([{"text": "Keep the prefix stable.", "tool": "warnings", "number": 0.19}])
    );
    assert_eq!(
        one["rules"].as_array().unwrap().len(),
        6,
        "every measured rule of the snapshot: {one}"
    );
}

#[test]
fn argument_errors_and_refusals_store_nothing_and_never_echo_the_text() {
    let canary = "CANARYxyz sk-ant-CANARY0123";
    let mut mcp = Mcp::started(Home::rich());
    let before = std::fs::read(mcp.home.store()).unwrap();
    let cases: Vec<(Value, &str)> = vec![
        (
            json!({"summary": 5, "fixes": []}),
            "Error executing tool save_report: argument `summary` is required and must be a string",
        ),
        (
            json!({"fixes": []}),
            "Error executing tool save_report: argument `summary` is required and must be a string",
        ),
        (
            json!({"summary": "ok"}),
            "Error executing tool save_report: argument `fixes` is required and must be a list of objects",
        ),
        (
            json!({"summary": "ok", "fixes": "CANARYxyz"}),
            "Error executing tool save_report: argument `fixes` is required and must be a list of objects",
        ),
        (
            json!({"summary": "ok", "fixes": ["CANARYxyz"]}),
            "Error executing tool save_report: argument `fixes` is required and must be a list of objects",
        ),
        (
            json!({"summary": "ok", "fixes": [], "workspace": 3}),
            "Error executing tool save_report: argument `workspace` must be a string or null",
        ),
        (
            json!({"summary": canary, "fixes": []}),
            "Error executing tool save_report: summary: looks like a secret (a key, token or private-key header); nothing was saved",
        ),
        (
            json!({"summary": "ok", "fixes": [{"text": canary}]}),
            "Error executing tool save_report: fixes[0].text: looks like a secret (a key, token or private-key header); nothing was saved",
        ),
        (
            json!({"summary": "ok", "fixes": [{"text": "ok", "number": canary}]}),
            "Error executing tool save_report: fixes[0].number: looks like a secret (a key, token or private-key header); nothing was saved",
        ),
        (
            json!({"summary": "ok", "fixes": [{"text": "ok", "extra": canary}]}),
            "Error executing tool save_report: fixes[0]: must be an object with `text` and optional `tool`, `number`; nothing was saved",
        ),
        (
            json!({"summary": "ok", "fixes": [{"text": "ok", "number": true}]}),
            "Error executing tool save_report: fixes[0].number: must be a number or text; nothing was saved",
        ),
        (
            json!({"summary": "ok", "fixes": [{"text": "CANARY ".repeat(50)}]}),
            "Error executing tool save_report: fixes[0].text: 350 characters, the limit is 300; nothing was saved",
        ),
        (
            json!({"summary": "ok", "fixes": [], "workspace": "nope"}),
            "Error executing tool save_report: workspace: unknown workspace; nothing was saved",
        ),
        (
            json!({"summary": canary, "fixes": [], "account": "nope"}),
            "Error executing tool save_report: summary: looks like a secret (a key, token or private-key header); nothing was saved",
        ),
        (
            json!({"summary": "ok", "fixes": [], "account": "nope"}),
            "Error executing tool save_report: account: unknown account; nothing was saved",
        ),
    ];
    let mut refused = 0;
    for (arguments, want) in &cases {
        let answer = mcp.call("save_report", arguments.clone());
        assert_eq!(&error_text(&answer), want, "{arguments}");
        assert!(!answer.to_string().contains("CANARY"), "{answer}");
        refused += 1;
    }
    assert_eq!(
        std::fs::read(mcp.home.store()).unwrap(),
        before,
        "a refusal touched the store file"
    );
    let stderr = mcp.finish();
    assert!(!stderr.contains("CANARY"), "{stderr}");
    println!(
        "save_report refusals over the binary: {refused}/{} as expected",
        cases.len()
    );
}

#[test]
fn a_missing_store_is_a_tool_error_without_a_path_and_creates_nothing() {
    let home = Home::new(None);
    let dir = home.dir.clone();
    let mut mcp = Mcp::started(home);
    let text = error_text(&mcp.call("save_report", json!({"summary": "ok", "fixes": []})));
    assert!(
        text.starts_with("Error executing tool save_report: no store yet"),
        "{text}"
    );
    assert!(!text.contains(dir.to_str().unwrap()), "{text}");
    assert!(
        !dir.join("store.sqlite").exists(),
        "a missing store is never created"
    );
}

#[test]
fn every_other_tool_leaves_the_store_file_byte_for_byte_alone() {
    let mut mcp = Mcp::started(Home::rich());
    let before = std::fs::read(mcp.home.store()).unwrap();
    let mtime = std::fs::metadata(mcp.home.store())
        .unwrap()
        .modified()
        .unwrap();
    for (tool, arguments) in [
        ("ping", json!({})),
        ("usage_summary", json!({})),
        ("usage_breakdown", json!({"by": "project"})),
        ("session_outliers", json!({})),
        ("warnings", json!({})),
        ("improvement_plan", json!({"workspace": "acme"})),
        ("kit_summary", json!({})),
        ("connector_catalog", json!({})),
        ("connector_status", json!({})),
        ("connector_suggestions", json!({})),
    ] {
        let answer = mcp.call(tool, arguments);
        assert_eq!(answer["result"]["isError"], false, "{tool}: {answer}");
    }
    let _ = mcp.request("prompts/list", json!({}));
    let _ = mcp.request("prompts/get", json!({"name": "investigate"}));
    assert_eq!(std::fs::read(mcp.home.store()).unwrap(), before);
    assert_eq!(
        std::fs::metadata(mcp.home.store())
            .unwrap()
            .modified()
            .unwrap(),
        mtime
    );
    // starting the server opened nothing for writing either: the data folder holds the store and nothing else
    let names: Vec<String> = std::fs::read_dir(&mcp.home.dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        ["store.sqlite"],
        "no -wal, -shm, journal or lock file"
    );
}

/// Saves go on while an `ingest` of the same store runs: both finish, the store is intact, every save is a row.
#[test]
fn saves_during_an_ingest_leave_both_consistent() {
    let home = Home::new(Some("store-v11.sqlite"));
    let store: &Path = &home.store();
    // the Python app leaves every store in WAL mode; the fixture is in `delete` mode
    rusqlite::Connection::open(store)
        .unwrap()
        .query_row("PRAGMA journal_mode=WAL", [], |_| Ok(()))
        .unwrap();
    let events = |path: &Path| -> i64 {
        rusqlite::Connection::open(path)
            .unwrap()
            .query_row("SELECT count(*) FROM events", [], |r| r.get(0))
            .unwrap()
    };
    let (events_before, rows_before) = (events(store), home.rows());
    let mut ingest = home
        .command("ingest")
        .env(
            "EASY_PERSONAL_CLAUDE_PROJECTS",
            parity("fixtures-rich/claude/projects"),
        )
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let path = store.to_path_buf();
    let saver = std::thread::spawn(move || {
        let mut saved = 0;
        for n in 0..25 {
            let summary = json!(format!("report {n}"));
            match easy_personal_core::reports::save(
                &path,
                &summary,
                &json!([]),
                None,
                None,
                easy_personal_core::clock::now_micros(),
            ) {
                Ok(_) => saved += 1,
                Err(e) => panic!("save {n}: {e}"),
            }
        }
        saved
    });
    let code = ingest.wait().unwrap().code();
    let saved = saver.join().unwrap();
    assert_eq!(code, Some(0), "the ingest failed while saves ran");
    assert_eq!(saved, 25);
    assert_eq!(home.rows(), rows_before + 25);
    assert!(events(store) > events_before, "the ingest stored nothing");
    let check: String = rusqlite::Connection::open(store)
        .unwrap()
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(check, "ok");
}
