//! T042: `GET /api/kit`, `/api/kit/workspaces`, `/api/kit/reports` over the real `serve` binary and the MCP tool
//! `kit_summary` over the real `mcp` binary (stdio), against the Python goldens of `rust/parity/golden` (the default
//! store: one gone project, no kit data) and its `mcp__tools_list.json`. The rich goldens need the fixture folders and
//! the injected clock: `crates/core/tests/kit_parity.rs` replays those. The binaries run under `TZ=UTC` over a temp copy
//! of the store; `HOME` and `EASY_PERSONAL_HOME` are temp folders, so the owner's real data folder is never reached. The
//! HTTP helpers are a trimmed copy of those of `tests/workspaces.rs` (a test target cannot import another).

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

fn parity(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity")
        .join(rel)
}

fn read_json(rel: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(parity(rel)).unwrap()).unwrap()
}

/// A temp data folder holding a copy of the default parity store; removed on drop.
struct Home(PathBuf);

impl Home {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t042-app-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::copy(parity("store-v11.sqlite"), dir.join("store.sqlite")).unwrap();
        Home(dir)
    }

    fn command(&self, subcommand: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"));
        command
            .arg(subcommand)
            .env("HOME", &self.0)
            .env("EASY_PERSONAL_HOME", &self.0)
            .env("TZ", "UTC");
        command
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// ---- HTTP ----

struct Server {
    child: Child,
    _stdout: BufReader<ChildStdout>,
    port: u16,
    token: String,
    _home: Home,
}

struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Server {
    fn start() -> Self {
        let home = Home::new();
        let mut child = home
            .command("serve")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        let url = line
            .trim_end()
            .strip_prefix("dashboard: http://127.0.0.1:")
            .unwrap_or_else(|| panic!("unexpected first line {line:?}"))
            .to_string();
        let (port, token) = url.split_once("/#").expect("URL fragment");
        Server {
            child,
            _stdout: stdout,
            port: port.parse().unwrap(),
            token: token.to_string(),
            _home: home,
        }
    }

    fn raw(&self, request: &str) -> Response {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut bytes = Vec::new();
        let _ = stream.read_to_end(&mut bytes);
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((text.as_str(), ""));
        let mut lines = head.split("\r\n");
        let status = lines
            .next()
            .and_then(|l| l.split(' ').nth(1))
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| panic!("no status line in {text:?}"));
        Response {
            status,
            headers: lines
                .filter_map(|l| l.split_once(':'))
                .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
                .collect(),
            body: body.to_string(),
        }
    }

    /// `method path` with a good Host and token.
    fn send(&self, method: &str, path: &str) -> Response {
        self.raw(&format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nx-dashboard-token: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            self.port, self.token
        ))
    }

    fn get(&self, path: &str) -> Response {
        self.send("GET", path)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Response {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or_else(|_| panic!("not JSON: {:?}", self.body))
    }

    fn content_type(&self) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(k, _)| k == "content-type")
            .map(|(_, v)| v.as_str())
            .collect()
    }
}

const ROUTES: [(&str, &str); 3] = [
    ("/api/kit", "golden/api__kit__all.json"),
    (
        "/api/kit/workspaces",
        "golden/api__kit_workspaces__all.json",
    ),
    ("/api/kit/reports", "golden/api__kit_reports__all.json"),
];

#[test]
fn the_three_routes_equal_the_default_goldens_over_real_http() {
    let server = Server::start();
    for (route, golden) in ROUTES {
        let response = server.get(route);
        assert_eq!(response.status, 200, "{route}: {}", response.body);
        assert_eq!(response.content_type(), vec!["application/json"], "{route}");
        assert_eq!(response.json(), read_json(golden), "{route}");
    }
}

#[test]
fn the_routes_are_guarded_and_read_only() {
    let server = Server::start();
    for (route, _) in ROUTES {
        let no_token = server.raw(&format!(
            "GET {route} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
            server.port
        ));
        assert_eq!(no_token.status, 403, "{route} without the token");
        assert_eq!(
            no_token.json(),
            read_json("golden/api__guard__no_token.json")
        );
        let bad_host = server.raw(&format!(
            "GET {route} HTTP/1.1\r\nHost: evil.example:{}\r\nx-dashboard-token: {}\r\nConnection: close\r\n\r\n",
            server.port, server.token
        ));
        assert_eq!(bad_host.status, 403, "{route} with a bad Host");
        assert_eq!(
            bad_host.json(),
            read_json("golden/api__guard__bad_host.json")
        );
        for method in ["POST", "PUT", "DELETE", "PATCH"] {
            assert_eq!(server.send(method, route).status, 405, "{method} {route}");
        }
    }
}

#[test]
fn the_query_is_read_as_python_reads_it() {
    let server = Server::start();
    // an empty workspace or account is no filter
    assert_eq!(
        server.get("/api/kit?workspace=&account=").json(),
        read_json(ROUTES[0].1)
    );
    // an unknown workspace is a 400 on `/api/kit`, with Python's text and none of the input
    let unknown = server.get("/api/kit?workspace=SECRET-name");
    assert_eq!(unknown.status, 400);
    assert_eq!(unknown.json(), json!({"error": "unknown workspace"}));
    assert!(!unknown.body.contains("SECRET"));
    let account = server.get("/api/kit?account=SECRET-name");
    assert_eq!(account.status, 400);
    assert!(
        account.json()["error"]
            .as_str()
            .unwrap()
            .starts_with("unknown account")
    );
    assert!(!account.body.contains("SECRET"));
    // the rollup takes no workspace (it is ignored) but takes `account`, `since` and `until`
    assert_eq!(server.get("/api/kit/workspaces?workspace=nope").status, 200);
    assert_eq!(
        server.get("/api/kit/workspaces?account=SECRET-name").status,
        400
    );
    let bad = server.get("/api/kit/workspaces?since=SECRET-day");
    assert_eq!(bad.status, 400);
    assert!(bad.json()["error"].is_string());
    let window =
        server.get("/api/kit/workspaces?since=2026-01-13T00:00:00Z&until=2026-01-20T00:00:00Z");
    assert_eq!(window.json(), read_json(ROUTES[1].1));
    // the report folders take no parameter
    assert_eq!(
        server.get("/api/kit/reports?workspace=nope").json(),
        read_json(ROUTES[2].1)
    );
}

// ---- MCP ----

/// Sends the handshake and `requests` (JSON-RPC, ids from 2 on), closes stdin and returns the answers by id.
fn mcp(home: &Home, requests: &[Value]) -> Vec<Value> {
    let mut child = home
        .command("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut lines = vec![
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t042", "version": "0"}}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    ];
    lines.extend(requests.iter().cloned());
    let mut stdin = child.stdin.take().unwrap();
    for line in &lines {
        writeln!(stdin, "{line}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|_| panic!("not JSON-RPC: {l:?}")))
        .collect()
}

fn answer_of(answers: &[Value], id: u64) -> &Value {
    answers
        .iter()
        .find(|a| a["id"] == id)
        .unwrap_or_else(|| panic!("no answer {id}"))
}

fn call(id: u64, arguments: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": "kit_summary", "arguments": arguments}})
}

#[test]
fn kit_summary_is_listed_with_pythons_text_and_returns_the_golden() {
    let home = Home::new();
    let answers = mcp(
        &home,
        &[
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
            call(3, json!({})),
            call(4, json!({"workspace": null, "account": ""})),
        ],
    );
    let tools = answer_of(&answers, 2)["result"]["tools"]
        .as_array()
        .unwrap()
        .clone();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    let after_plan = names.iter().position(|n| *n == "improvement_plan").unwrap() + 1;
    assert_eq!(
        names[after_plan], "kit_summary",
        "Python registers it right after improvement_plan"
    );
    let tool = tools.iter().find(|t| t["name"] == "kit_summary").unwrap();
    let golden = read_json("golden/mcp__tools_list.json");
    let want = golden
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["name"] == "kit_summary")
        .unwrap();
    for key in ["description", "inputSchema", "outputSchema"] {
        assert_eq!(tool[key], want[key], "kit_summary {key}");
    }
    let expected = read_json("golden/mcp__kit_summary__all.json");
    for id in [3, 4] {
        let result = &answer_of(&answers, id)["result"];
        assert_eq!(result["isError"], false, "{result}");
        assert_eq!(result["structuredContent"], expected, "call {id}");
        let text: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(text, expected, "text block of call {id}");
    }
}

#[test]
fn kit_summary_errors_are_tool_errors_with_pythons_text_and_none_of_the_input() {
    let home = Home::new();
    let answers = mcp(
        &home,
        &[
            call(2, json!({"workspace": "SECRET-name"})),
            call(3, json!({"account": "SECRET-name"})),
            call(4, json!({"workspace": 5})),
        ],
    );
    let text = |id| {
        let result = &answer_of(&answers, id)["result"];
        assert_eq!(result["isError"], true, "{result}");
        result["content"][0]["text"].as_str().unwrap().to_string()
    };
    assert_eq!(
        text(2),
        "Error executing tool kit_summary: unknown workspace"
    );
    assert!(
        text(3).starts_with("Error executing tool kit_summary: unknown account; valid labels: ")
    );
    assert!(text(4).starts_with("Error executing tool kit_summary: "));
    for id in [2, 3] {
        assert!(!text(id).contains("SECRET"));
    }
}
