//! T043: `GET /api/board` and `GET /api/memory-graph` over the real `serve` binary: the guard (token, Host, Origin), the
//! write methods, the 400 texts and the goldens of `rust/parity/golden` (`api__board__all`, `api__memory_graph__list`).
//! The binary runs under `TZ=UTC` over a temp copy of the default parity store; `HOME`, `EASY_PERSONAL_HOME`,
//! `EASY_MEMORY_HOME` and `XDG_CONFIG_HOME` are temp folders, so no real data folder is reached. The richer answers (kanban
//! tasks, notes, links) are compared in `crates/core/tests/memgraph_parity.rs`.

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

struct Server {
    child: Child,
    _stdout: BufReader<ChildStdout>,
    port: u16,
    token: String,
    home: PathBuf,
}

impl Server {
    fn start() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let home = std::env::temp_dir().join(format!(
            "easy-personal-rs-t043-http-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&home).unwrap();
        std::fs::copy(parity("store-v11.sqlite"), home.join("store.sqlite")).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
            .args(["serve", "--no-ingest"])
            .env_clear()
            .env("HOME", &home)
            .env("EASY_PERSONAL_HOME", &home)
            .env("EASY_MEMORY_HOME", home.join("em-none"))
            .env("XDG_CONFIG_HOME", home.join("xdg"))
            .env("TZ", "UTC")
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
            home,
        }
    }

    /// `(status, body)` of one request; `headers` replace the good Host and token when their name matches.
    fn send(&self, method: &str, path: &str, host: Option<&str>, token: bool) -> (u16, String) {
        self.send_with_origin(method, path, host, token, None)
    }

    /// As [`Server::send`], with an `Origin` header when `origin` is given.
    fn send_with_origin(
        &self,
        method: &str,
        path: &str,
        host: Option<&str>,
        token: bool,
        origin: Option<&str>,
    ) -> (u16, String) {
        let host = host.map_or(format!("127.0.0.1:{}", self.port), str::to_string);
        let mut request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {host}\r\nContent-Length: 0\r\nConnection: close\r\n"
        );
        if token {
            request += &format!("x-dashboard-token: {}\r\n", self.token);
        }
        if let Some(origin) = origin {
            request += &format!("Origin: {origin}\r\n");
        }
        request += "\r\n";
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut bytes = Vec::new();
        let _ = stream.read_to_end(&mut bytes);
        let text = String::from_utf8_lossy(&bytes).to_string();
        let status = text
            .split(' ')
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let body = text
            .split_once("\r\n\r\n")
            .map_or(String::new(), |(_, b)| b.to_string());
        (status, body)
    }

    fn get(&self, path: &str) -> (u16, Value) {
        let (status, body) = self.send("GET", path, None, true);
        (status, serde_json::from_str(&body).unwrap_or(Value::Null))
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

fn golden(name: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(parity(&format!("golden/{name}"))).unwrap())
        .unwrap()
}

#[test]
fn both_routes_answer_the_default_goldens() {
    let server = Server::start();
    assert_eq!(
        server.get("/api/board"),
        (200, golden("api__board__all.json"))
    );
    assert_eq!(
        server.get("/api/memory-graph"),
        (200, golden("api__memory_graph__list.json"))
    );
    // an empty `project` is no project; an empty scope is no scope
    assert_eq!(server.get("/api/memory-graph?project=").0, 200);
    assert_eq!(server.get("/api/board?workspace=&account=").0, 200);
}

#[test]
fn caller_mistakes_are_400_with_fixed_texts_that_do_not_echo_the_input() {
    let server = Server::start();
    assert_eq!(
        server.get("/api/memory-graph?project=nope-secret"),
        (400, json!({"error": "unknown project"}))
    );
    let (status, body) = server.get("/api/board?workspace=nope-secret");
    assert_eq!(
        (status, body["error"].as_str()),
        (400, Some("unknown workspace"))
    );
    let (status, body) = server.get("/api/board?account=nope-secret");
    assert_eq!(status, 400);
    assert!(!body.to_string().contains("nope-secret"), "{body}");
    // the exact text (Python's `accounts.resolve`): the valid labels of the store, never the caller's word
    assert_eq!(
        body["error"].as_str(),
        Some("unknown account; valid labels: (none)")
    );
}

#[test]
fn the_guard_and_the_methods_are_the_same_as_for_the_other_routes() {
    let server = Server::start();
    for path in ["/api/board", "/api/memory-graph"] {
        assert_eq!(
            server.send("GET", path, None, false).0,
            403,
            "{path} without the token"
        );
        assert_eq!(
            server.send("GET", path, Some("localhost:1"), true).0,
            403,
            "{path} with a wrong Host"
        );
        // a page from another origin is refused, the page's own origin is let through
        for origin in ["http://evil.example", "http://127.0.0.1:1", "null"] {
            assert_eq!(
                server
                    .send_with_origin("GET", path, None, true, Some(origin))
                    .0,
                403,
                "{path} with Origin {origin}"
            );
        }
        let own = format!("http://127.0.0.1:{}", server.port);
        assert_eq!(
            server
                .send_with_origin("GET", path, None, true, Some(&own))
                .0,
            200,
            "{path} with its own Origin"
        );
        for method in ["POST", "PUT", "DELETE", "PATCH"] {
            assert_eq!(
                server.send(method, path, None, true).0,
                405,
                "{method} {path}"
            );
        }
    }
}
