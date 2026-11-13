//! T059 (FR-091): `GET /api/store/health` of the real `serve` binary over a raw `TcpStream`, on temp copies of the oracle's
//! stores (`rust/parity/golden-store-health/`). The oracle is Python's `store_health.check` (the goldens) and Python's
//! `tests/surfaces/test_api_store_health.py` (the guard, the methods, `HEAD`). The cache (5 s per store) and the single
//! flight are tested in-process, where a check can be counted: `crates/app/src/api_tests.rs`.
//! `HOME` and `EASY_PERSONAL_HOME` point into a temp folder, so the owner's `~/.easy-personal` is never reached.

use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

fn golden() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/golden-store-health")
}

struct Server {
    child: Child,
    _stdout: BufReader<ChildStdout>,
    port: u16,
    token: String,
    home: PathBuf,
}

impl Server {
    /// `serve --no-ingest` over a data folder that holds a copy of the oracle's `fixture` folder.
    fn start(fixture: Option<&str>) -> Server {
        static N: AtomicU32 = AtomicU32::new(0);
        let home = std::env::temp_dir().join(format!(
            "easy-personal-rs-t059-route-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&home).unwrap();
        let home = fs::canonicalize(home).unwrap();
        if let Some(fixture) = fixture {
            for entry in fs::read_dir(golden().join(fixture)).unwrap() {
                let entry = entry.unwrap();
                fs::copy(entry.path(), home.join(entry.file_name())).unwrap();
            }
        }
        let mut child = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
            .args(["serve", "--no-ingest"])
            .env("HOME", &home)
            .env("EASY_PERSONAL_HOME", &home)
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
            .unwrap()
            .to_string();
        let (port, token) = url.split_once("/#").unwrap();
        Server {
            child,
            _stdout: stdout,
            port: port.parse().unwrap(),
            token: token.to_string(),
            home,
        }
    }

    fn raw(&self, request: &str) -> Response {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut bytes = Vec::new();
        let _ = stream.read_to_end(&mut bytes);
        Response::parse(&bytes)
    }

    /// `method path` with the good Host and token, plus `extra` header lines (a `Host` or token in `extra` replaces it).
    fn call(&self, method: &str, path: &str, token: bool, extra: &[(&str, &str)]) -> Response {
        let mut headers: Vec<(String, String)> =
            vec![("Host".into(), format!("127.0.0.1:{}", self.port))];
        if token {
            headers.push(("x-dashboard-token".into(), self.token.clone()));
        }
        for (k, v) in extra {
            headers.retain(|(h, _)| !h.eq_ignore_ascii_case(k));
            headers.push((k.to_string(), v.to_string()));
        }
        let mut request = format!("{method} {path} HTTP/1.1\r\n");
        for (k, v) in headers {
            request += &format!("{k}: {v}\r\n");
        }
        request += "Content-Length: 0\r\nConnection: close\r\n\r\n";
        self.raw(&request)
    }

    fn get(&self, path: &str) -> Response {
        self.call("GET", path, true, &[])
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.home);
    }
}

struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Response {
    fn parse(bytes: &[u8]) -> Response {
        let at = bytes
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .expect("a complete response");
        let head = String::from_utf8_lossy(&bytes[..at]).into_owned();
        let mut lines = head.split("\r\n");
        let status = lines
            .next()
            .unwrap()
            .split(' ')
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        let headers = lines
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
            .collect();
        Response {
            status,
            headers,
            body: bytes[at + 4..].to_vec(),
        }
    }

    fn header(&self, name: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or_else(|_| panic!("not JSON: {}", self.text()))
    }
}

const URL: &str = "/api/store/health";

fn golden_json(case: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(golden().join(format!("{case}.json"))).unwrap())
        .unwrap()
}

#[test]
fn the_route_answers_what_python_answers_for_every_plain_case() {
    for case in [
        "clean",
        "empty",
        "behind",
        "newer",
        "hostile",
        "blob_time",
        "backups",
        "later_table",
        "wal_no_shm",
        "corrupt_page",
    ] {
        let server = Server::start(Some(case));
        let response = server.get(URL);
        assert_eq!(response.status, 200, "{case}: {}", response.text());
        assert_eq!(response.json(), golden_json(case), "{case}");
        assert_eq!(response.header("content-type"), vec!["application/json"]);
        assert_eq!(response.header("cache-control"), vec!["no-store"]);
    }
}

#[test]
fn a_missing_store_is_a_200_answer_and_creates_nothing() {
    let server = Server::start(None);
    let response = server.get(URL);
    assert_eq!(response.status, 200);
    assert_eq!(response.json(), golden_json("missing"));
    assert_eq!(fs::read_dir(&server.home).unwrap().count(), 0);
}

#[test]
fn the_guard_refuses_no_token_a_wrong_token_a_wrong_host_and_a_foreign_origin() {
    let server = Server::start(Some("clean"));
    assert_eq!(server.call("GET", URL, false, &[]).status, 403);
    assert_eq!(
        server
            .call("GET", URL, true, &[("x-dashboard-token", "nope")])
            .status,
        403
    );
    assert_eq!(
        server
            .call("GET", URL, true, &[("Host", "evil.example")])
            .status,
        403
    );
    assert_eq!(
        server
            .call("GET", URL, true, &[("Origin", "http://evil.example")])
            .status,
        403
    );
    let own = format!("http://127.0.0.1:{}", server.port);
    assert_eq!(
        server.call("GET", URL, true, &[("Origin", &own)]).status,
        200
    );
    // the token is not accepted from the query
    let query = format!("{URL}?token={}", server.token);
    assert_eq!(server.call("GET", &query, false, &[]).status, 403);
    for refused in [
        server.call("GET", URL, false, &[]),
        server.call("GET", URL, true, &[("Host", "evil.example")]),
    ] {
        assert!(
            !refused.text().contains("user_version") && !refused.text().contains("tables"),
            "{}",
            refused.text()
        );
    }
}

#[test]
fn only_get_and_head_are_allowed() {
    let server = Server::start(Some("clean"));
    for method in ["POST", "PUT", "PATCH", "DELETE"] {
        let response = server.call(method, URL, true, &[]);
        assert_eq!(response.status, 405, "{method}: {}", response.text());
    }
}

#[test]
fn head_answers_headers_only() {
    let server = Server::start(Some("clean"));
    let response = server.call("HEAD", URL, true, &[]);
    assert_eq!(response.status, 200);
    assert!(response.body.is_empty(), "{}", response.text());
    assert_eq!(response.header("cache-control"), vec!["no-store"]);
    assert_eq!(
        server.call("HEAD", URL, false, &[]).status,
        403,
        "the guard still applies"
    );
}

#[test]
fn the_route_writes_nothing_and_names_no_folder() {
    let server = Server::start(Some("wal_no_shm"));
    let snapshot = |home: &Path| {
        let mut names: Vec<String> = fs::read_dir(home)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
            .into_iter()
            .map(|n| {
                (
                    fs::read(home.join(&n)).unwrap(),
                    fs::metadata(home.join(&n)).unwrap().modified().unwrap(),
                    n,
                )
            })
            .collect::<Vec<_>>()
    };
    let before = snapshot(&server.home);
    let mut texts = String::new();
    for _ in 0..2 {
        let response = server.get(URL);
        assert_eq!(response.status, 200);
        texts += &response.text();
    }
    assert!(
        before == snapshot(&server.home),
        "a file changed or appeared"
    );
    assert!(
        !texts.contains(server.home.to_str().unwrap()) && !texts.contains("t059-route"),
        "{texts}"
    );
}

#[test]
fn the_route_does_not_migrate_a_store_one_migration_behind() {
    let server = Server::start(Some("behind"));
    let body = server.get(URL).json();
    assert_eq!(
        (body["migration_pending"].clone(), body["healthy"].clone()),
        (Value::Bool(true), Value::Bool(false))
    );
    assert_eq!(
        body["user_version"],
        body["latest_migration"].as_i64().unwrap() - 1
    );
    let listing: Vec<String> = fs::read_dir(&server.home)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(
        listing,
        vec!["store.sqlite"],
        "a migration would have left a backup"
    );
}

#[test]
fn the_route_answer_equals_the_cli_json() {
    let server = Server::start(Some("hostile"));
    let cli = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
        .args(["store-check", "--json"])
        .env("HOME", &server.home)
        .env("EASY_PERSONAL_HOME", &server.home)
        .output()
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&cli.stdout).unwrap(),
        server.get(URL).json()
    );
}

#[test]
fn concurrent_requests_all_get_the_same_answer() {
    let server = Server::start(Some("clean"));
    let answers: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..12).map(|_| scope.spawn(|| server.get(URL))).collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .map(|r| format!("{} {}", r.status, r.text()))
            .collect()
    });
    assert!(
        answers
            .iter()
            .all(|a| a == &answers[0] && a.starts_with("200 ")),
        "{answers:?}"
    );
}
