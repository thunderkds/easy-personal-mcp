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
            .env("EASY_PERSONAL_CONTEXT_URL", dead_url())
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

/// A closed port on 127.0.0.1: no test of this file can reach a real easy-context service.
fn dead_url() -> String {
    let port = TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    format!("http://127.0.0.1:{port}/mcp")
}

/// `Command::output` with a limit: past it the child is killed and the test fails with `message`.
fn output_within(mut command: Command, limit: Duration, message: &str) -> std::process::Output {
    let mut child = command.spawn().unwrap();
    drop(child.stdin.take());
    let until = Instant::now() + limit;
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= until {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{message}");
        }
        sleep(Duration::from_millis(20));
    }
    child.wait_with_output().unwrap()
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
            let out = output_within(
                home.command(&["--transport", transport, "--host", host, "--port", &free]),
                Duration::from_secs(10),
                &format!(
                    "{transport} --host {host:?} did not exit within 10 s: the host refusal is missing"
                ),
            );
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

// -- fix round (Stage 4): bounded stop, read timeout, connection cap, duplicate Host and Origin

/// Ctrl-C with `stuck` already written to a connection: the process exits 0 within 5 s.
fn ctrl_c_with_a_stuck_client(stuck: impl Fn(u16) -> String) {
    let home = Home::new();
    let server = Http::start(&home);
    let mut client = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
    client.write_all(stuck(server.port).as_bytes()).unwrap();
    sleep(Duration::from_millis(200));
    let mut server = server;
    let kill = Command::new("kill")
        .args(["-INT", &server.child.id().to_string()])
        .status()
        .unwrap();
    assert!(kill.success());
    let until = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = server.child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= until {
            let _ = server.child.kill();
            let _ = server.child.wait();
            panic!("Ctrl-C did not stop the server within 5 s with a client stuck mid-request");
        }
        sleep(Duration::from_millis(50));
    };
    assert_eq!(status.code(), Some(0));
    drop(client);
}

#[test]
fn ctrl_c_stops_the_server_with_a_half_sent_header() {
    ctrl_c_with_a_stuck_client(|port| {
        format!("POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Ty")
    });
}

#[test]
fn ctrl_c_stops_the_server_with_a_half_sent_body() {
    ctrl_c_with_a_stuck_client(|port| {
        format!(
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nAccept: application/json\r\nContent-Length: 100\r\n\r\n{{\"jsonrpc\""
        )
    });
}

/// Reads until the server closes the connection; the time that took, or `None` if it was still open after `limit`.
fn closed_after(mut tcp: TcpStream, limit: Duration) -> Option<Duration> {
    let began = Instant::now();
    tcp.set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    let mut buf = [0u8; 256];
    while began.elapsed() < limit {
        match tcp.read(&mut buf) {
            Ok(0) => return Some(began.elapsed()),
            Ok(_) => {}
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(_) => return Some(began.elapsed()), // a reset is a close too
        }
    }
    None
}

#[test]
fn a_stalled_connection_is_closed_within_the_read_timeout() {
    let home = Home::new();
    let server = Http::start(&home);
    let port = server.port;
    let stalls: [(&str, String); 3] = [
        ("nothing sent", String::new()),
        (
            "half a header",
            format!("POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConten"),
        ),
        (
            "half a body",
            format!(
                "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nAccept: application/json\r\nContent-Length: 100\r\n\r\n{{\"a\""
            ),
        ),
    ];
    let threads: Vec<_> = stalls
        .iter()
        .map(|(label, bytes)| {
            let (label, bytes) = (*label, bytes.clone().into_bytes());
            std::thread::spawn(move || {
                let mut tcp = TcpStream::connect(("127.0.0.1", port)).unwrap();
                tcp.write_all(&bytes).unwrap();
                (label, closed_after(tcp, Duration::from_secs(20)))
            })
        })
        .collect();
    for t in threads {
        let (label, took) = t.join().unwrap();
        let took = took.unwrap_or_else(|| panic!("{label}: still open after 20 s"));
        assert!(
            took >= Duration::from_secs(8),
            "{label}: closed after only {took:?}, a slow client must get 10 s"
        );
        assert!(
            took <= Duration::from_secs(14),
            "{label}: closed after {took:?}, the limit is 10 s"
        );
    }
    server.stop();
}

#[test]
fn sixty_idle_sockets_do_not_stop_a_real_request_and_the_cap_closes_the_extra_ones() {
    let home = Home::new();
    let server = Http::start(&home);
    let idle: Vec<TcpStream> = (0..60)
        .map(|_| TcpStream::connect(("127.0.0.1", server.port)).unwrap())
        .collect();
    sleep(Duration::from_millis(300));
    let (status, body) = server.rpc(&json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}));
    assert_eq!(
        (status, &body["result"]),
        (200, &json!({})),
        "a legit request beside 60 idle sockets"
    );
    // Up to 64 open at once: four more fill the cap (one is the request's own, already closed), the next is closed at once.
    let more: Vec<TcpStream> = (0..4)
        .map(|_| TcpStream::connect(("127.0.0.1", server.port)).unwrap())
        .collect();
    sleep(Duration::from_millis(300));
    let extra = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
    let took = closed_after(extra, Duration::from_secs(3))
        .expect("a connection over the cap is closed at once");
    assert!(took < Duration::from_secs(2), "{took:?}");
    drop((idle, more));
    sleep(Duration::from_millis(300));
    let (status, _) = server.rpc(&json!({"jsonrpc": "2.0", "id": 2, "method": "ping"}));
    assert_eq!(status, 200, "the cap frees when the sockets close");
    server.stop();
}

#[test]
fn a_duplicate_host_or_origin_header_is_checked_in_every_value() {
    let home = Home::new();
    let server = Http::start(&home);
    let good_host = format!("127.0.0.1:{}", server.port);
    let ping = json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}).to_string();
    let base = [
        ("Content-Type", "application/json"),
        ("Accept", "application/json"),
    ];
    let code = |headers: &[(&str, &str)]| server.send("POST", "/mcp", headers, ping.as_bytes()).0;
    for (first, second) in [
        ("evil.example", good_host.as_str()),
        (good_host.as_str(), "evil.example"),
    ] {
        let headers = with(&[("Host", first), ("Host", second)], &base);
        assert_eq!(code(&headers), 421, "Host: {first}, Host: {second}");
    }
    let both_good = with(
        &[("Host", good_host.as_str()), ("Host", good_host.as_str())],
        &base,
    );
    assert_eq!(code(&both_good), 200, "two good values pass");
    for (first, second) in [
        ("http://evil.example", "http://localhost"),
        ("http://localhost", "http://evil.example"),
    ] {
        let headers = with(
            &[
                ("Host", good_host.as_str()),
                ("Origin", first),
                ("Origin", second),
            ],
            &base,
        );
        assert_eq!(code(&headers), 403, "Origin: {first}, Origin: {second}");
    }
    server.stop();
}
