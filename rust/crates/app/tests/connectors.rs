//! T046: `GET /api/connectors` over the real `serve` binary and `connector_catalog`, `connector_status` and
//! `connector_suggestions` over the real `mcp` binary, against the Python goldens of `rust/parity/golden` and
//! `golden-rich`, plus the guard, method, store-state and no-process cases. Both binaries run under `TZ=UTC` over a
//! temp copy of a parity store; `HOME` and `EASY_PERSONAL_HOME` are temp folders, so the owner's real data folder is
//! never reached. The helpers are trimmed copies of those of `tests/workspaces.rs` and `tests/mcp.rs` (a test target
//! cannot import another).
//!
//! `PATH` of every spawned binary is a temp folder holding a fake `claude` that writes a marker file when it runs: the
//! build must never start it (it asks neither the keychain nor Claude Code: presence is reported as "not configured").

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

/// Same string as `CSP` in `src/easy_personal/surfaces/api.py`.
const CSP: &str = "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'";

const TOOLS: [&str; 3] = [
    "connector_catalog",
    "connector_status",
    "connector_suggestions",
];

fn parity(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity")
        .join(rel)
}

fn read_json(rel: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(parity(rel)).unwrap()).unwrap()
}

/// A temp data folder (`HOME` and `EASY_PERSONAL_HOME`) and a `PATH` folder with a fake `claude` that leaves a marker.
struct Home {
    dir: PathBuf,
    bin: PathBuf,
}

impl Home {
    fn new(setup: impl FnOnce(&Path)) -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let base = std::env::temp_dir().join(format!(
            "easy-personal-rs-t046-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let (dir, bin) = (base.join("home"), base.join("bin"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(&bin).unwrap();
        let fake = bin.join("claude");
        std::fs::write(
            &fake,
            format!(
                "#!/bin/sh\necho ran > '{}'\n",
                base.join("claude-ran").display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();
        setup(&dir);
        Home { dir, bin }
    }

    fn with_store(fixture: &str) -> Self {
        Home::new(|dir| {
            std::fs::copy(parity(fixture), dir.join("store.sqlite")).unwrap();
        })
    }

    fn store(&self) -> PathBuf {
        self.dir.join("store.sqlite")
    }

    fn command(&self, subcommand: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"));
        command
            .arg(subcommand)
            .env("HOME", &self.dir)
            .env("EASY_PERSONAL_HOME", &self.dir)
            .env("PATH", &self.bin)
            .env("TZ", "UTC")
            .stderr(Stdio::null());
        command
    }

    /// True when the fake `claude` was started by anything.
    fn claude_ran(&self) -> bool {
        self.bin.parent().unwrap().join("claude-ran").exists()
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.bin.parent().unwrap());
    }
}

// ---- HTTP ----

struct Server {
    child: Child,
    _stdout: BufReader<ChildStdout>,
    port: u16,
    token: String,
    home: Home,
}

impl Server {
    fn start(home: Home) -> Self {
        let mut child = home
            .command("serve")
            .stdout(Stdio::piped())
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
            home,
        }
    }

    fn with_store(fixture: &str) -> Self {
        Server::start(Home::with_store(fixture))
    }

    fn host(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    fn raw(&self, request: &str) -> Response {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut bytes = Vec::new();
        let _ = stream.read_to_end(&mut bytes);
        Response::parse(&bytes)
    }

    /// `method path` with a good Host and token and the extra header lines.
    fn send(&self, method: &str, path: &str, extra: &[(&str, &str)]) -> Response {
        let mut req = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nx-dashboard-token: {}\r\nContent-Length: 0\r\n",
            self.host(),
            self.token
        );
        for (k, v) in extra {
            req += &format!("{k}: {v}\r\n");
        }
        self.raw(&(req + "Connection: close\r\n\r\n"))
    }

    fn authed(&self, path: &str) -> Response {
        self.send("GET", path, &[])
    }

    /// `GET path` with the given Host (`None`: no header) and extra header lines, and no token unless given.
    fn get(&self, path: &str, host: Option<&str>, extra: &[(&str, &str)]) -> Response {
        let mut req = format!("GET {path} HTTP/1.1\r\n");
        if let Some(h) = host {
            req += &format!("Host: {h}\r\n");
        }
        for (k, v) in extra {
            req += &format!("{k}: {v}\r\n");
        }
        self.raw(&(req + "Connection: close\r\n\r\n"))
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Response {
    fn parse(bytes: &[u8]) -> Self {
        let text = String::from_utf8_lossy(bytes).into_owned();
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((text.as_str(), ""));
        let mut lines = head.split("\r\n");
        let status = lines
            .next()
            .and_then(|l| l.split(' ').nth(1))
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| panic!("no status line in {text:?}"));
        let headers = lines
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
            .collect();
        Response {
            status,
            headers,
            body: body.to_string(),
        }
    }

    fn header(&self, name: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or_else(|_| panic!("not JSON: {:?}", self.body))
    }

    fn assert_csp(&self) {
        assert_eq!(
            self.header("content-security-policy"),
            vec![CSP],
            "status {}",
            self.status
        );
    }

    fn assert_json(&self, status: u16) {
        assert_eq!(self.status, status, "{}", self.body);
        assert_eq!(self.header("content-type"), vec!["application/json"]);
        self.assert_csp();
    }

    fn assert_forbidden(&self, golden: &str) {
        assert_eq!(self.status, 403, "{}", self.body);
        assert_eq!(self.json(), read_json(&format!("golden/{golden}.json")));
        self.assert_csp();
    }
}

#[test]
fn the_route_equals_the_two_goldens_over_the_binary_and_starts_no_process() {
    let mut compared = 0;
    for (dir, store) in [
        ("golden", "store-v11.sqlite"),
        ("golden-rich", "store-rich/store-v11.sqlite"),
    ] {
        let manifest = read_json(&format!("{dir}/manifest.json"));
        let entry = manifest["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["route"] == "/api/connectors" && e["method"] == "GET")
            .unwrap();
        assert_eq!(entry["status"], 200);
        assert_eq!(entry["params"], json!({}));
        let server = Server::with_store(store);
        let r = server.authed("/api/connectors");
        r.assert_json(200);
        assert_eq!(
            r.json(),
            read_json(&format!("{dir}/{}", entry["file"].as_str().unwrap())),
            "{dir}"
        );
        assert!(!server.home.claude_ran(), "{dir}: `claude` was started");
        compared += 1;
    }
    assert_eq!(compared, 2, "two oracles x one route");
}

#[test]
fn the_query_string_is_ignored() {
    let s = Server::with_store("store-rich/store-v11.sqlite");
    let plain = s.authed("/api/connectors");
    for query in [
        "workspace=nope",
        "account=nope&since=garbage",
        "by=x&workspace=",
    ] {
        let r = s.authed(&format!("/api/connectors?{query}"));
        r.assert_json(200);
        assert_eq!(r.body, plain.body, "?{query}");
    }
}

#[test]
fn stored_connector_rows_show_only_id_alias_provider_and_workspace_in_name_order() {
    let s = Server::start(Home::new(|dir| {
        let store = dir.join("store.sqlite");
        std::fs::copy(parity("store-rich/store-v11.sqlite"), &store).unwrap();
        let conn = rusqlite::Connection::open(&store).unwrap();
        let id = |name: &str| -> i64 {
            conn.query_row("SELECT id FROM workspaces WHERE name = ?1", [name], |r| {
                r.get(0)
            })
            .unwrap()
        };
        let (acme, personal) = (id("acme"), id("personal"));
        conn.execute_batch(&format!(
            "INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES \
             ('c0000002', 'my-discord', 1, 'discord', {personal}), \
             ('c0000001', 'acme-slack', 0, 'slack', {acme}), \
             ('c0000003', 'acme-gmail', 0, 'gmail', {acme})"
        ))
        .unwrap();
    }));
    let body = s.authed("/api/connectors").json();
    assert_eq!(
        body["workspace_connectors"],
        json!([
            {"id": "c0000003", "alias": "acme-gmail", "provider": "gmail", "workspace": "acme"},
            {"id": "c0000001", "alias": "acme-slack", "provider": "slack", "workspace": "acme"},
            {"id": "c0000002", "alias": "my-discord", "provider": "discord", "workspace": "personal"}
        ])
    );
    // a stored row does not make a connector "configured": presence is never asked
    assert!(
        body["connectors"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["configured"] == false)
    );
    assert!(!s.home.claude_ran());
}

#[test]
fn writes_are_405_with_the_csp_and_head_is_answered_like_get() {
    let s = Server::with_store("store-rich/store-v11.sqlite");
    let before = std::fs::read(s.home.store()).unwrap();
    for method in ["POST", "PATCH", "PUT", "DELETE"] {
        let r = s.send(method, "/api/connectors", &[]);
        assert_eq!(r.status, 405, "{method}");
        r.assert_csp();
        assert!(r.body.is_empty(), "{method}: {:?}", r.body);
    }
    let head = s.send("HEAD", "/api/connectors", &[]);
    assert_eq!(head.status, 200);
    assert!(head.body.is_empty());
    head.assert_csp();
    // Python's other connector routes are not served by this build
    for path in ["/api/connectors/legacy", "/api/connectors/slack"] {
        for method in ["GET", "DELETE", "PUT"] {
            let r = s.send(method, path, &[]);
            assert_eq!(r.status, 404, "{method} {path}");
        }
    }
    assert_eq!(
        std::fs::read(s.home.store()).unwrap(),
        before,
        "no write reached the store"
    );
}

#[test]
fn the_guard_covers_the_route() {
    let s = Server::with_store("store-v11.sqlite");
    let host = s.host();
    let route = "/api/connectors";
    s.get(route, Some(&host), &[])
        .assert_forbidden("api__guard__no_token");
    s.get(route, Some(&host), &[("x-dashboard-token", "wrong")])
        .assert_forbidden("api__guard__no_token");
    s.get(&format!("{route}?token={}", s.token), Some(&host), &[])
        .assert_forbidden("api__guard__no_token");
    s.get(
        route,
        Some(&format!("localhost:{}", s.port)),
        &[("x-dashboard-token", &s.token)],
    )
    .assert_forbidden("api__guard__bad_host");
    s.send("GET", route, &[("Origin", "http://evil.example")])
        .assert_forbidden("api__guard__bad_origin");
    s.get(route, None, &[("x-dashboard-token", &s.token)])
        .assert_forbidden("api__guard__bad_host");
    assert_eq!(s.authed(route).status, 200);
    // a write without the token is a 403 before it is a 405
    s.raw(&format!(
        "POST {route} HTTP/1.1\r\nHost: {host}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    ))
    .assert_forbidden("api__guard__no_token");
}

#[test]
fn encoded_or_doubled_paths_are_never_served() {
    let s = Server::with_store("store-rich/store-v11.sqlite");
    for path in [
        "/%61pi/connectors",
        "//api/connectors",
        "/api%2fconnectors",
        "/api/./connectors",
        "/api/connectors/",
        "/api/%63onnectors",
    ] {
        for with_token in [true, false] {
            let r = if with_token {
                s.authed(path)
            } else {
                s.get(path, Some(&s.host()), &[])
            };
            assert!(r.status == 404 || r.status == 403, "{path}: {}", r.status);
            r.assert_csp();
            assert!(!r.body.contains("connectors"), "{path}: {}", r.body);
        }
    }
}

#[test]
fn a_missing_store_is_503_and_a_bad_one_is_a_fixed_text_without_path_or_row_text() {
    let missing = Server::start(Home::new(|_| {}));
    let r = missing.authed("/api/connectors");
    r.assert_json(503);
    assert_eq!(
        r.json(),
        json!({"error": "no store yet: nothing has been ingested into the data folder"})
    );
    assert_eq!(
        std::fs::read_dir(&missing.home.dir).unwrap().count(),
        0,
        "nothing created"
    );

    let newer = Server::start(Home::new(|dir| {
        let store = dir.join("store.sqlite");
        std::fs::copy(parity("store-v11.sqlite"), &store).unwrap();
        rusqlite::Connection::open(&store)
            .unwrap()
            .execute_batch("PRAGMA user_version = 12")
            .unwrap();
    }));
    let r = newer.authed("/api/connectors");
    r.assert_json(503);
    assert_eq!(
        r.json(),
        json!({"error": "the store is at version 12, newer than the 11 this build knows; update easy-personal-rs (nothing was changed)"})
    );

    let garbage = Server::start(Home::new(|dir| {
        std::fs::write(dir.join("store.sqlite"), "not a database, PRIVATE-ROW-TEXT").unwrap()
    }));
    // right version, no `connectors` table: the SQL error must not reach the body
    let no_tables = Server::start(Home::new(|dir| {
        rusqlite::Connection::open(dir.join("store.sqlite"))
            .unwrap()
            .execute_batch(
                "CREATE TABLE note (x TEXT); INSERT INTO note VALUES ('PRIVATE-ROW-TEXT')",
            )
            .unwrap()
    }));
    let dropped = Server::with_store("store-v11.sqlite");
    rusqlite::Connection::open(dropped.home.store())
        .unwrap()
        .execute_batch("DROP TABLE connectors")
        .unwrap();
    let fixed = "the store cannot be read right now; try again in a moment";
    for server in [&garbage, &no_tables, &dropped] {
        let r = server.authed("/api/connectors");
        r.assert_json(503);
        assert_eq!(r.json(), json!({ "error": fixed }));
        for banned in [
            server.home.dir.to_str().unwrap(),
            "PRIVATE-ROW-TEXT",
            "SELECT",
            "connectors",
            "no such table",
        ] {
            assert!(!r.body.contains(banned), "{}", r.body);
        }
    }
}

#[test]
fn serving_leaves_the_store_bytes_and_the_folder_as_they_were() {
    for store in ["store-v11.sqlite", "store-rich/store-v11.sqlite"] {
        let s = Server::with_store(store);
        let before = std::fs::read(s.home.store()).unwrap();
        s.authed("/api/connectors").assert_json(200);
        assert_eq!(std::fs::read(s.home.store()).unwrap(), before, "{store}");
        let names: Vec<String> = std::fs::read_dir(&s.home.dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["store.sqlite"], "{store}: no -wal, -shm or journal");
    }
}

// ---- MCP ----

struct Mcp {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    next_id: u64,
    _home: Home,
}

impl Mcp {
    fn started(home: Home) -> Self {
        let mut child = home
            .command("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (tx, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let mut mcp = Mcp {
            child,
            stdin,
            lines,
            next_id: 1,
            _home: home,
        };
        let answer = mcp.request(
            "initialize",
            json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t046-test", "version": "0"}}),
        );
        assert!(answer["result"].is_object(), "initialize failed: {answer}");
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
            .recv_timeout(Duration::from_secs(10))
            .expect("no answer within 10 s");
        let answer: Value =
            serde_json::from_str(&line).unwrap_or_else(|_| panic!("not JSON: {line:?}"));
        assert_eq!(answer["id"], id, "answer for another request: {answer}");
        answer
    }

    fn call(&mut self, name: &str, arguments: Value) -> Value {
        self.request("tools/call", json!({"name": name, "arguments": arguments}))
    }

    fn claude_ran(&self) -> bool {
        self._home.claude_ran()
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn structured(answer: &Value) -> &Value {
    assert_eq!(answer["result"]["isError"], false, "tool failed: {answer}");
    &answer["result"]["structuredContent"]
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
fn the_three_tools_are_listed_with_pythons_description_and_schemas() {
    let mut mcp = Mcp::started(Home::with_store("store-v11.sqlite"));
    let answer = mcp.request("tools/list", json!({}));
    let tools = answer["result"]["tools"].as_array().unwrap();
    let golden = read_json("golden/mcp__tools_list.json");
    for name in TOOLS {
        let got = tools
            .iter()
            .find(|t| t["name"] == name)
            .unwrap_or_else(|| panic!("{name} not listed"));
        let want = golden
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["name"] == name)
            .unwrap();
        for key in ["description", "inputSchema", "outputSchema"] {
            assert_eq!(got[key], want[key], "{name} {key}");
        }
    }
}

#[test]
fn the_three_tools_equal_the_goldens_of_both_oracles_and_start_no_process() {
    let mut compared = 0;
    for (dir, store) in [
        ("golden", "store-v11.sqlite"),
        ("golden-rich", "store-rich/store-v11.sqlite"),
    ] {
        let mut mcp = Mcp::started(Home::with_store(store));
        for tool in TOOLS {
            let want = read_json(&format!("{dir}/mcp__{tool}__all.json"));
            let answer = mcp.call(tool, json!({}));
            assert_eq!(structured(&answer), &want, "{dir} {tool}");
            // the text block carries the same body as the structured one
            let text: Value =
                serde_json::from_str(answer["result"]["content"][0]["text"].as_str().unwrap())
                    .unwrap();
            assert_eq!(text, want, "{dir} {tool} text");
            compared += 1;
        }
        assert!(!mcp.claude_ran(), "{dir}: `claude` was started");
    }
    assert_eq!(compared, 6, "two oracles x three tools");
}

#[test]
fn arguments_are_dropped_and_catalog_and_status_need_no_store() {
    let mut mcp = Mcp::started(Home::new(|_| {}));
    let want = read_json("golden-rich/mcp__connector_catalog__all.json");
    assert_eq!(
        structured(&mcp.call("connector_catalog", json!({"workspace": "x", "extra": 1}))),
        &want
    );
    let want = read_json("golden-rich/mcp__connector_status__all.json");
    assert_eq!(
        structured(&mcp.call("connector_status", json!({"account": 5}))),
        &want
    );
    // suggestions read the store: no store is a tool error with no path in it
    let text = error_text(&mcp.call("connector_suggestions", json!({})));
    assert!(
        text.starts_with("Error executing tool connector_suggestions: "),
        "{text}"
    );
    assert!(!text.contains(mcp._home.dir.to_str().unwrap()), "{text}");
    assert!(
        !mcp._home.store().exists(),
        "the server must not create the store"
    );
    assert!(!mcp.claude_ran());
}

/// Rows for the suggestions check. The expected answer below is what Python's `suggestions.suggestions(conn, [])`
/// returned on a copy of the rich store with these rows (`rust/parity/.venv`, T046). Three-way tie-breaking, a
/// case-sensitive host, a look-alike domain, a `claude_ai_` server (excluded) and a NULL are all in it.
const SIGNALS: &str = "
INSERT INTO tool_signals (machine_id, source_path, inode, byte_offset, kind, value) VALUES
 ('m', 'p', 1, 1, 'domain', 'api.slack.com'), ('m', 'p', 1, 2, 'domain', 'api.slack.com'), ('m', 'p', 1, 3, 'domain', 'api.slack.com'),
 ('m', 'p', 1, 4, 'domain', 'slack.com'), ('m', 'p', 1, 5, 'domain', 'slack.com'),
 ('m', 'p', 1, 6, 'domain', 'discord.gg'),
 ('m', 'p', 1, 7, 'domain', 'notslack.com'), ('m', 'p', 1, 8, 'domain', 'notslack.com'),
 ('m', 'p', 1, 9, 'domain', 'x.sendgrid.com'),
 ('m', 'p', 1, 10, 'domain', 'graph.microsoft.com'), ('m', 'p', 1, 11, 'domain', 'graph.microsoft.com'),
 ('m', 'p', 1, 12, 'domain', 'SLACK.COM'),
 ('m', 'p', 1, 13, 'domain', 'mail.google.com'), ('m', 'p', 1, 14, 'domain', 'mail.google.com'),
 ('m', 'p', 1, 15, 'domain', '.slack.com'),
 ('m', 'p', 1, 16, 'host', 'slack.com'), ('m', 'p', 1, 17, 'host', 'slack.com');
INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, tool_names) VALUES
 ('m', 's', 'p', 1, 101, 0, 'Bash,mcp__slack__post,mcp__slack__read'),
 ('m', 's', 'p', 1, 102, 0, 'mcp__claude_ai_Gmail__send'),
 ('m', 's', 'p', 1, 103, 0, 'mcp__Discord_Server__x,mcp__a'),
 ('m', 's', 'p', 1, 104, 0, 'MCP__slack__z'),
 ('m', 's', 'p', 1, 105, 0, 'mcp__teams_x__a'),
 ('m', 's', 'p', 1, 106, 0, 'mcp__discordslack__a'),
 ('m', 's', 'p', 1, 107, 0, NULL),
 ('m', 's', 'p', 1, 108, 0, 'mcp__sendgrid__a,mcp__sendgrid__b,mcp__sendgrid__c,mcp__sendgrid__d');
";

#[test]
fn suggestions_equal_pythons_answer_on_a_store_with_usage() {
    let mut mcp = Mcp::started(Home::new(|dir| {
        let store = dir.join("store.sqlite");
        std::fs::copy(parity("store-rich/store-v11.sqlite"), &store).unwrap();
        rusqlite::Connection::open(&store)
            .unwrap()
            .execute_batch(SIGNALS)
            .unwrap();
    }));
    let answer = mcp.call("connector_suggestions", json!({}));
    assert_eq!(
        structured(&answer),
        &json!({
            "note": "Counts cover sessions ingested since this version; run `easy-personal ingest` to refresh. Bash first words are counted but map to no provider yet.",
            "suggestions": [
                {"id": "slack", "mcp_calls": 3, "provider": "Slack", "webfetch_calls": 6},
                {"id": "sendgrid", "mcp_calls": 4, "provider": "SendGrid", "webfetch_calls": 1},
                {"id": "discord", "mcp_calls": 2, "provider": "Discord", "webfetch_calls": 1},
                {"id": "teams", "mcp_calls": 1, "provider": "Teams", "webfetch_calls": 2}
            ]
        })
    );
    assert!(!mcp.claude_ran());
    // status is unmoved by that usage
    let status = mcp.call("connector_status", json!({}));
    assert_eq!(
        structured(&status),
        &read_json("golden-rich/mcp__connector_status__all.json")
    );
}

#[test]
fn a_bad_store_is_a_tool_error_without_path_or_row_text_for_suggestions() {
    let mut mcp = Mcp::started(Home::new(|dir| {
        std::fs::write(dir.join("store.sqlite"), "not a database, PRIVATE-ROW-TEXT").unwrap()
    }));
    let text = error_text(&mcp.call("connector_suggestions", json!({})));
    assert!(
        text.starts_with("Error executing tool connector_suggestions: "),
        "{text}"
    );
    for banned in [
        mcp._home.dir.to_str().unwrap(),
        "PRIVATE-ROW-TEXT",
        "SELECT",
    ] {
        assert!(!text.contains(banned), "{text}");
    }
    // the next call still works
    let status = mcp.call("connector_status", json!({}));
    assert_eq!(
        structured(&status),
        &read_json("golden-rich/mcp__connector_status__all.json")
    );
}
