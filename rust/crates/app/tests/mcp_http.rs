//! T051: `mcp --transport http` through the real binary, over raw `TcpStream`s. Each server runs over a temp copy of
//! the parity store (`EASY_PERSONAL_HOME`, `HOME`, no agent root): the owner's folders are never reached. The tool list,
//! the prompt and tool answers over HTTP are compared with the same requests over stdio.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

struct Home(PathBuf);

impl Home {
    fn new() -> Home {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t051-mcphttp-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/store-v11.sqlite");
        std::fs::copy(store, dir.join("store.sqlite")).unwrap();
        Home(dir)
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"));
        c.arg("mcp")
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", &self.0)
            .env("EASY_PERSONAL_HOME", &self.0)
            .env("TZ", "UTC")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        c
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Http {
    child: Child,
    port: u16,
    stdout: std::process::ChildStdout,
    stderr: BufReader<std::process::ChildStderr>,
}

impl Http {
    fn start(home: &Home) -> Http {
        let mut child = home
            .command(&["--transport", "http", "--port", "0"])
            .spawn()
            .unwrap();
        let mut stderr = BufReader::new(child.stderr.take().unwrap());
        let mut line = String::new();
        stderr.read_line(&mut line).unwrap();
        let port = line
            .trim_end()
            .strip_prefix("MCP over HTTP: http://127.0.0.1:")
            .and_then(|r| r.strip_suffix("/mcp"))
            .unwrap_or_else(|| panic!("unexpected first line {line:?}"))
            .parse()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        Http {
            child,
            port,
            stdout,
            stderr,
        }
    }

    /// Ctrl-C, then the exit code, what stdout held and the rest of stderr.
    fn stop(mut self) -> (Option<i32>, String, String) {
        sleep(Duration::from_millis(200)); // the handler is installed once the line is printed
        let kill = Command::new("kill")
            .args(["-INT", &self.child.id().to_string()])
            .status()
            .unwrap();
        assert!(kill.success());
        let status = self.child.wait().unwrap();
        let (mut out, mut err) = (String::new(), String::new());
        self.stdout.read_to_string(&mut out).unwrap();
        self.stderr.read_to_string(&mut err).unwrap();
        (status.code(), out, err)
    }

    /// One request, `Connection: close`; returns (status, header block, body).
    fn send(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> (u16, String, String) {
        raw(self.port, method, path, headers, body)
    }

    fn rpc(&self, message: &Value) -> (u16, Value) {
        let host = format!("127.0.0.1:{}", self.port);
        let (status, _, body) = self.send(
            "POST",
            "/mcp",
            &[
                ("Host", &host),
                ("Content-Type", "application/json"),
                ("Accept", "application/json, text/event-stream"),
            ],
            message.to_string().as_bytes(),
        );
        (status, serde_json::from_str(&body).unwrap_or(Value::Null))
    }
}

fn with<'a>(extra: &[(&'a str, &'a str)], base: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    let mut all = base.to_vec();
    all.extend_from_slice(extra);
    all
}

fn raw(
    port: u16,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> (u16, String, String) {
    let mut tcp = TcpStream::connect(("127.0.0.1", port)).unwrap();
    tcp.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
    let mut req = format!("{method} {path} HTTP/1.1\r\n");
    for (k, v) in headers {
        req += &format!("{k}: {v}\r\n");
    }
    req += &format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    tcp.write_all(req.as_bytes()).unwrap();
    tcp.write_all(body).unwrap();
    let mut out = Vec::new();
    let _ = tcp.read_to_end(&mut out);
    let text = String::from_utf8_lossy(&out).to_string();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    (status, head.to_string(), body.to_string())
}

/// The same requests over stdio: initialize, initialized, then each request; the answers by id.
fn over_stdio(home: &Home, requests: &[Value]) -> Vec<Value> {
    let mut child = home.command(&[]).spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    let init = json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {"protocolVersion": "2025-06-18",
        "capabilities": {}, "clientInfo": {"name": "t", "version": "0"}}});
    writeln!(input, "{init}").unwrap();
    writeln!(
        input,
        "{}",
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
    )
    .unwrap();
    for r in requests {
        writeln!(input, "{r}").unwrap();
    }
    drop(input);
    let mut out = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    child.wait().unwrap();
    let all: Vec<Value> = out
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    requests
        .iter()
        .map(|r| {
            all.iter()
                .find(|a| a["id"] == r["id"])
                .cloned()
                .expect("an answer")
        })
        .collect()
}

/// The answer with the real clock's `as_of_utc` (`warnings`) blanked, in `structuredContent` and in the text copy of it.
fn unclocked(answer: &Value) -> String {
    let text = answer.to_string();
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(at) = rest.find("as_of_utc") {
        let (head, tail) = rest.split_at(at);
        out += head;
        let skip = tail.find("+00:00").map_or(tail.len(), |i| i + 6);
        out += "as_of_utc";
        rest = &tail[skip..];
    }
    out + rest
}

fn requests() -> Vec<Value> {
    let call = |id: i64, name: &str, args: Value| json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": name, "arguments": args}});
    vec![
        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "prompts/list"}),
        json!({"jsonrpc": "2.0", "id": 3, "method": "prompts/get", "params": {"name": "investigate", "arguments": {}}}),
        call(4, "ping", json!({})),
        call(5, "usage_summary", json!({})),
        call(6, "warnings", json!({})),
        call(
            7,
            "usage_summary",
            json!({"workspace": "no-such-workspace"}),
        ),
        json!({"jsonrpc": "2.0", "id": 8, "method": "ping"}),
        call(9, "no_such_tool", json!({})),
    ]
}

#[test]
fn http_answers_like_stdio_for_the_tool_list_the_prompt_and_the_tools() {
    let home = Home::new();
    let server = Http::start(&home);
    let want = over_stdio(&home, &requests());
    for (request, expected) in requests().iter().zip(&want) {
        // No `initialize` first: the transport is stateless, a request carries its own handshake.
        let (status, got) = server.rpc(request);
        assert_eq!(status, 200, "{request}");
        assert_eq!(unclocked(&got), unclocked(expected), "{request}");
    }
    assert_eq!(
        want[0]["result"]["tools"].as_array().unwrap().len(),
        12,
        "the 12 tools of tools/list"
    );
    let (code, out, err) = server.stop();
    assert_eq!(code, Some(0));
    assert_eq!(out, "", "stdout stays empty over HTTP");
    assert_eq!(
        err, "Ctrl-C to stop.\n",
        "nothing is logged but the start line (already read)"
    );
}

#[test]
fn initialize_over_http_gives_the_server_info_and_a_notification_is_accepted() {
    let home = Home::new();
    let server = Http::start(&home);
    let init = json!({"jsonrpc": "2.0", "id": "a", "method": "initialize", "params": {"protocolVersion": "2025-06-18",
        "capabilities": {}, "clientInfo": {"name": "t", "version": "0"}}});
    let (status, got) = server.rpc(&init);
    assert_eq!(status, 200);
    assert_eq!(got["id"], "a");
    assert_eq!(got["result"]["serverInfo"]["name"], "easy-personal-rs");
    assert!(got["result"]["capabilities"]["tools"].is_object());
    let host = format!("127.0.0.1:{}", server.port);
    let note = json!({"jsonrpc": "2.0", "method": "notifications/initialized"}).to_string();
    let (status, head, body) = server.send(
        "POST",
        "/mcp",
        &[
            ("Host", &host),
            ("Content-Type", "application/json"),
            ("Accept", "application/json"),
        ],
        note.as_bytes(),
    );
    assert_eq!((status, body.as_str()), (202, ""), "{head}");
    server.stop();
}

#[test]
fn the_guard_and_the_http_rules() {
    let home = Home::new();
    let server = Http::start(&home);
    let host = format!("127.0.0.1:{}", server.port);
    let good = [
        ("Host", host.as_str()),
        ("Content-Type", "application/json"),
        ("Accept", "application/json, text/event-stream"),
    ];
    let ping = json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}).to_string();
    let code = |method: &str, path: &str, headers: &[(&str, &str)], body: &[u8]| {
        server.send(method, path, headers, body).0
    };

    // Host: a name that is not ours (DNS rebinding), the right name with a wrong port, none
    for bad in [
        "evil.example",
        "evil.example:80",
        "127.0.0.1",
        "127.0.0.1:1",
        "localhost",
        "0.0.0.0:9",
        "[::1]:1",
    ] {
        let headers = [
            ("Host", bad),
            ("Content-Type", "application/json"),
            ("Accept", "application/json"),
        ];
        assert_eq!(
            code("POST", "/mcp", &headers, ping.as_bytes()),
            421,
            "Host {bad}"
        );
    }
    let localhost = format!("localhost:{}", server.port);
    let headers = [
        ("Host", localhost.as_str()),
        ("Content-Type", "application/json"),
        ("Accept", "application/json"),
    ];
    assert_eq!(
        code("POST", "/mcp", &headers, ping.as_bytes()),
        200,
        "localhost is our name too"
    );
    // Origin: a web page of another site is refused, a local one passes
    for bad in [
        "http://evil.example",
        "https://127.0.0.1",
        "null",
        "http://127.0.0.1.evil.example",
        "http://localhost.evil.example:1",
        "http://127.0.0.1:x",
    ] {
        assert_eq!(
            code(
                "POST",
                "/mcp",
                &with(&[("Origin", bad)], &good),
                ping.as_bytes()
            ),
            403,
            "Origin {bad}"
        );
    }
    for ok in [
        "http://127.0.0.1:3000",
        "http://localhost",
        "http://localhost:6274",
    ] {
        assert_eq!(
            code(
                "POST",
                "/mcp",
                &with(&[("Origin", ok)], &good),
                ping.as_bytes()
            ),
            200,
            "Origin {ok}"
        );
    }
    // the route, the method, the types
    assert_eq!(code("POST", "/", &good, ping.as_bytes()), 404);
    assert_eq!(code("POST", "/mcp/", &good, ping.as_bytes()), 404);
    assert_eq!(code("POST", "/api/version", &good, ping.as_bytes()), 404);
    for method in ["GET", "DELETE", "PUT"] {
        let (status, head, _) = server.send(method, "/mcp", &good, b"");
        assert_eq!(status, 405, "{method}");
        assert!(head.to_ascii_lowercase().contains("allow: post"), "{head}");
    }
    assert_eq!(
        code(
            "POST",
            "/mcp",
            &[
                ("Host", &host),
                ("Content-Type", "text/plain"),
                ("Accept", "application/json")
            ],
            ping.as_bytes()
        ),
        415
    );
    assert_eq!(
        code(
            "POST",
            "/mcp",
            &[("Host", &host), ("Accept", "application/json")],
            ping.as_bytes()
        ),
        415
    );
    assert_eq!(
        code(
            "POST",
            "/mcp",
            &[
                ("Host", &host),
                ("Content-Type", "application/json"),
                ("Accept", "text/html")
            ],
            ping.as_bytes()
        ),
        406
    );
    assert_eq!(
        code(
            "POST",
            "/mcp",
            &[("Host", &host), ("Content-Type", "application/json")],
            ping.as_bytes()
        ),
        406
    );
    // the body
    let (status, _, body) = server.send("POST", "/mcp", &good, b"{not json");
    let parsed: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        (status, parsed["error"]["code"].as_i64(), &parsed["id"]),
        (400, Some(-32700), &Value::Null)
    );
    for body in ["[]", "5", "\"x\"", "{}", "{\"jsonrpc\":\"2.0\",\"id\":3}"] {
        let (status, _, answer) = server.send("POST", "/mcp", &good, body.as_bytes());
        let parsed: Value = serde_json::from_str(&answer).unwrap();
        assert_eq!(
            (status, parsed["error"]["code"].as_i64()),
            (400, Some(-32600)),
            "{body}"
        );
    }
    // an unknown method is the JSON-RPC answer stdio gives (HTTP 200, code -32601)
    let (status, _, answer) = server.send(
        "POST",
        "/mcp",
        &good,
        br#"{"jsonrpc":"2.0","id":3,"method":"nope/never"}"#,
    );
    let parsed: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(
        (status, parsed["error"]["code"].as_i64()),
        (200, Some(-32601))
    );
    let big = vec![b' '; (1 << 20) + 1];
    assert_eq!(code("POST", "/mcp", &good, &big), 413);
    // after all that the server still answers
    assert_eq!(code("POST", "/mcp", &good, ping.as_bytes()), 200);
    let (exit, out, _) = server.stop();
    assert_eq!((exit, out.as_str()), (Some(0), ""));
}

#[test]
fn many_requests_at_once_each_get_their_own_answer() {
    let home = Home::new();
    let server = Http::start(&home);
    let port = server.port;
    let threads: Vec<_> = (0..16)
        .map(|i| {
            std::thread::spawn(move || {
                let host = format!("127.0.0.1:{port}");
                let msg = json!({"jsonrpc": "2.0", "id": i, "method": "tools/call", "params": {"name": "ping", "arguments": {}}});
                let (status, _, body) = raw(
                    port,
                    "POST",
                    "/mcp",
                    &[("Host", &host), ("Content-Type", "application/json"), ("Accept", "application/json")],
                    msg.to_string().as_bytes(),
                );
                let answer: Value = serde_json::from_str(&body).unwrap();
                (i, status, answer["id"].clone(), answer["result"]["structuredContent"]["result"].clone())
            })
        })
        .collect();
    for t in threads {
        let (i, status, id, text) = t.join().unwrap();
        assert_eq!(
            (status, id, text),
            (200, json!(i), json!("easy-personal core ok")),
            "request {i}"
        );
    }
    server.stop();
}

#[test]
fn another_host_is_refused_with_exit_2_before_anything_is_bound() {
    let home = Home::new();
    let free = TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
        .to_string();
    for host in [
        "0.0.0.0",
        "localhost",
        "::1",
        "example.com",
        "127.0.0.2",
        "",
        "127.0.0.1 ",
        "LOCALHOST",
    ] {
        for transport in ["http", "stdio"] {
            let began = Instant::now();
            let out = home
                .command(&["--transport", transport, "--host", host, "--port", &free])
                .output()
                .unwrap();
            let err = String::from_utf8(out.stderr).unwrap();
            assert_eq!(
                out.status.code(),
                Some(2),
                "{transport} --host {host:?}: {err}"
            );
            assert!(out.stdout.is_empty(), "{host:?}");
            assert_eq!(err.lines().count(), 1, "{err}");
            assert!(err.contains("HTTP must bind 127.0.0.1 only"), "{err}");
            assert!(
                began.elapsed() < Duration::from_secs(5),
                "it must not wait for a client"
            );
        }
    }
    // The refusal does not bind: the same port is still free for the next listener.
    TcpListener::bind(("127.0.0.1", free.parse().unwrap())).unwrap();
}

#[test]
fn a_bad_port_or_transport_is_a_clap_error_with_exit_2() {
    let home = Home::new();
    for args in [
        vec!["--port", "abc"],
        vec!["--port", "65536"],
        vec!["--port", "-1"],
        vec!["--port", ""],
        vec!["--transport", "ftp"],
        vec!["--transport", "HTTP"],
        vec!["--transport"],
        vec!["--host"],
    ] {
        let out = home.command(&args).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty());
        assert!(
            String::from_utf8(out.stderr).unwrap().starts_with("error:"),
            "{args:?}"
        );
    }
}

#[test]
fn a_port_in_use_is_one_line_with_no_path_and_exit_1() {
    let home = Home::new();
    let busy = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = busy.local_addr().unwrap().port().to_string();
    let out = home
        .command(&["--transport", "http", "--port", &port])
        .output()
        .unwrap();
    let err = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert_eq!(err.lines().count(), 1, "{err}");
    assert!(
        err.starts_with("easy-personal-rs: cannot bind 127.0.0.1:"),
        "{err}"
    );
    assert!(
        !err.contains(home.0.to_str().unwrap()) && !err.contains('/'),
        "no path: {err}"
    );
}

#[test]
fn the_server_listens_on_127_0_0_1_only() {
    let home = Home::new();
    let server = Http::start(&home);
    let listening = std::fs::read_to_string("/proc/net/tcp").unwrap();
    let hex = format!(":{:04X} ", server.port);
    let lines: Vec<&str> = listening
        .lines()
        .filter(|l| l.contains(&hex) && l.split_whitespace().nth(3) == Some("0A"))
        .collect();
    assert!(!lines.is_empty(), "the port is listening");
    for line in lines {
        assert!(
            line.split_whitespace()
                .nth(1)
                .unwrap()
                .starts_with("0100007F:"),
            "only 127.0.0.1: {line}"
        );
    }
    server.stop();
}
