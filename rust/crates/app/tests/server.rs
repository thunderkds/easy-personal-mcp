//! Runs the real `serve` binary and talks to it over a raw `TcpStream` (no HTTP client crate). The oracle is the
//! Python guard: the three `api__guard__*` goldens and `CSP` in `src/easy_personal/surfaces/api.py`.
//! `HOME` and `EASY_PERSONAL_HOME` point into a temp folder, so the owner's `~/.easy-personal` is never reached.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// Same string as `CSP` in `src/easy_personal/surfaces/api.py`.
const CSP: &str = "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'";

fn golden(name: &str) -> serde_json::Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../parity/golden/{name}.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

struct Server {
    child: Child,
    stdout: BufReader<ChildStdout>,
    port: u16,
    token: String,
    first_lines: Vec<String>,
    home: PathBuf,
}

impl Server {
    fn start() -> Self {
        Self::launch(|_| {}, &[])
    }

    /// A server whose data folder holds a copy of `fixture` as `store.sqlite`, run under `TZ=UTC` like the goldens.
    fn start_with_store(fixture: &Path) -> Self {
        Self::launch(
            |home| {
                std::fs::copy(fixture, home.join("store.sqlite")).unwrap();
            },
            &[("TZ", "UTC")],
        )
    }

    fn launch(setup: impl FnOnce(&Path), env: &[(&str, &str)]) -> Self {
        Self::launch_args(setup, env, &[], None)
    }

    /// `serve` with extra arguments (`--static-dir <dir>`) and, if given, a working directory.
    fn launch_args(
        setup: impl FnOnce(&Path),
        env: &[(&str, &str)],
        args: &[&std::ffi::OsStr],
        cwd: Option<&Path>,
    ) -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let home = std::env::temp_dir().join(format!(
            "easy-personal-rs-t004-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&home).unwrap();
        setup(&home);
        let mut command = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"));
        command
            .arg("serve")
            .args(args)
            .env("HOME", &home)
            .env("EASY_PERSONAL_HOME", &home)
            .envs(env.iter().copied())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        let mut child = command.spawn().unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut first_lines = Vec::new();
        for _ in 0..2 {
            let mut line = String::new();
            stdout.read_line(&mut line).unwrap();
            first_lines.push(line.trim_end().to_string());
        }
        let url = first_lines[0]
            .strip_prefix("dashboard: http://127.0.0.1:")
            .unwrap_or_else(|| panic!("unexpected first line {:?}", first_lines[0]));
        let (port, token) = url.split_once("/#").expect("URL fragment");
        let (port, token) = (port.parse().unwrap(), token.to_string());
        Server {
            child,
            stdout,
            port,
            token,
            first_lines,
            home,
        }
    }

    fn host(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    /// Sends `request` as is and reads the whole answer (the requests carry `Connection: close`).
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

    /// `GET path` with extra header lines; `Host` and `Connection: close` are added unless `host` is `None`.
    fn get(&self, path: &str, host: Option<&str>, extra: &[(&str, &str)]) -> Response {
        let mut req = format!("GET {path} HTTP/1.1\r\n");
        if let Some(h) = host {
            req += &format!("Host: {h}\r\n");
        }
        for (k, v) in extra {
            req += &format!("{k}: {v}\r\n");
        }
        req += "Connection: close\r\n\r\n";
        self.raw(&req)
    }

    /// Good Host, good token.
    fn authed(&self, path: &str, extra: &[(&str, &str)]) -> Response {
        let mut headers = vec![("x-dashboard-token", self.token.as_str())];
        headers.extend_from_slice(extra);
        self.get(path, Some(&self.host()), &headers)
    }

    fn stop_with_sigint(&mut self) -> (std::process::ExitStatus, String, String) {
        let kill = Command::new("kill")
            .args(["-INT", &self.child.id().to_string()])
            .status()
            .unwrap();
        assert!(kill.success());
        let deadline = Instant::now() + Duration::from_secs(5);
        let status = loop {
            if let Some(s) = self.child.try_wait().unwrap() {
                break s;
            }
            assert!(Instant::now() < deadline, "server did not stop on SIGINT");
            std::thread::sleep(Duration::from_millis(20));
        };
        let mut rest = String::new();
        self.stdout.read_to_string(&mut rest).unwrap();
        let mut err = String::new();
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut err)
            .unwrap();
        (status, rest, err)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
    /// The body as received (`body` is lossy UTF-8).
    bytes: Vec<u8>,
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
        let at = bytes.windows(4).position(|w| w == b"\r\n\r\n");
        Response {
            status,
            headers,
            body: body.to_string(),
            bytes: at.map_or_else(Vec::new, |i| bytes[i + 4..].to_vec()),
        }
    }

    fn header(&self, name: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    fn json(&self) -> serde_json::Value {
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

    fn assert_forbidden(&self, golden_name: &str) {
        assert_eq!(self.status, 403, "{}", self.body);
        assert_eq!(self.json(), golden(golden_name));
        self.assert_csp();
    }
}

#[test]
fn good_host_and_token_get_the_version() {
    let s = Server::start();
    let r = s.authed("/api/version", &[]);
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(
        r.json(),
        serde_json::json!({"version": env!("CARGO_PKG_VERSION")})
    );
    assert_eq!(r.header("content-type"), vec!["application/json"]);
    r.assert_csp();
}

#[test]
fn prints_exactly_the_two_python_lines() {
    let s = Server::start();
    let line = &s.first_lines[0];
    let token = line.rsplit_once("/#").unwrap().1;
    assert_eq!(
        line,
        &format!("dashboard: http://127.0.0.1:{}/#{token}", s.port)
    );
    assert_eq!(token.len(), 43, "32 bytes, base64url, no padding");
    assert!(
        token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    );
    assert_eq!(s.first_lines[1], "Ctrl-C to stop.");
    assert_eq!(token, s.token);
    assert_ne!(Server::start().token, s.token, "a new token per run");
}

#[test]
fn bad_host_is_forbidden() {
    let s = Server::start();
    // DNS rebinding: another name that points at 127.0.0.1
    for host in [
        format!("localhost:{}", s.port),
        format!("evil.example:{}", s.port),
        "127.0.0.1".to_string(),
        format!("127.0.0.1:{}", s.port + 1),
        format!("127.0.0.1:{}.", s.port),
        format!("[::1]:{}", s.port),
        String::new(),
    ] {
        let r = s.get(
            "/api/version",
            Some(&host),
            &[("x-dashboard-token", &s.token)],
        );
        r.assert_forbidden("api__guard__bad_host");
    }
}

#[test]
fn missing_host_header_is_forbidden() {
    let s = Server::start();
    let req = format!(
        "GET /api/version HTTP/1.0\r\nx-dashboard-token: {}\r\n\r\n",
        s.token
    );
    let r = s.raw(&req);
    assert_ne!(r.status, 200);
    assert!(r.status == 403 || r.status == 400, "{}", r.status);
    assert!(!r.body.contains(&s.token));
}

#[test]
fn two_host_headers_never_get_through() {
    let s = Server::start();
    let (good, bad) = (s.host(), format!("localhost:{}", s.port));
    for pair in [[&good, &bad], [&bad, &good]] {
        let req = format!(
            "GET /api/version HTTP/1.1\r\nHost: {}\r\nHost: {}\r\nx-dashboard-token: {}\r\nConnection: close\r\n\r\n",
            pair[0], pair[1], s.token
        );
        let r = s.raw(&req);
        assert!(
            r.status == 403 || r.status == 400,
            "got {} for {pair:?}",
            r.status
        );
    }
}

#[test]
fn absolute_form_request_line_is_judged_by_the_host_header() {
    let s = Server::start();
    // Handled like Python: the Host header decides, the authority in the request line does not.
    let req = format!(
        "GET http://{}/api/version HTTP/1.1\r\nHost: localhost:{}\r\nx-dashboard-token: {}\r\nConnection: close\r\n\r\n",
        s.host(),
        s.port,
        s.token
    );
    s.raw(&req).assert_forbidden("api__guard__bad_host");
}

#[test]
fn bad_origin_is_forbidden_and_no_origin_is_fine() {
    let s = Server::start();
    for origin in [
        "http://evil.example".to_string(),
        "null".to_string(),
        format!("http://localhost:{}", s.port),
        format!("https://127.0.0.1:{}", s.port),
        format!("http://127.0.0.1:{}/", s.port),
        String::new(),
    ] {
        let r = s.authed("/api/version", &[("Origin", &origin)]);
        r.assert_forbidden("api__guard__bad_origin");
    }
    assert_eq!(s.authed("/api/version", &[]).status, 200);
    let same = format!("http://127.0.0.1:{}", s.port);
    assert_eq!(s.authed("/api/version", &[("Origin", &same)]).status, 200);
}

#[test]
fn missing_or_wrong_token_is_forbidden() {
    let s = Server::start();
    let host = s.host();
    s.get("/api/version", Some(&host), &[])
        .assert_forbidden("api__guard__no_token");
    let wrong = [
        "wrong".to_string(),
        String::new(),
        format!("{}x", s.token),
        s.token[..s.token.len() - 1].to_string(),
        s.token.to_uppercase(),
    ];
    for t in wrong {
        let r = s.get("/api/version", Some(&host), &[("x-dashboard-token", &t)]);
        r.assert_forbidden("api__guard__no_token");
        assert!(!r.body.contains(&s.token));
    }
}

#[test]
fn token_header_name_is_case_insensitive() {
    let s = Server::start();
    for name in [
        "X-Dashboard-Token",
        "x-dashboard-token",
        "X-DASHBOARD-TOKEN",
    ] {
        let r = s.get("/api/version", Some(&s.host()), &[(name, &s.token)]);
        assert_eq!(r.status, 200, "{name}");
    }
}

#[test]
fn query_token_is_not_accepted_on_api_version() {
    let s = Server::start();
    let r = s.get(
        &format!("/api/version?token={}", s.token),
        Some(&s.host()),
        &[],
    );
    r.assert_forbidden("api__guard__no_token");
    // and a query token beside a wrong header does not rescue it
    let r = s.get(
        &format!("/api/version?token={}", s.token),
        Some(&s.host()),
        &[("x-dashboard-token", "nope")],
    );
    r.assert_forbidden("api__guard__no_token");
}

#[test]
fn csp_is_on_every_response_and_replaces_nothing_it_should_not() {
    let s = Server::start();
    // 404 outside /api/ needs no token, like Python
    let r = s.get("/nothing-here", Some(&s.host()), &[]);
    assert_eq!(r.status, 404);
    r.assert_csp();
    // 404 inside /api/ with a token
    let r = s.authed("/api/not-a-route", &[]);
    assert_eq!(r.status, 404);
    r.assert_csp();
    // /api/ without a token is refused before routing
    s.get("/api/not-a-route", Some(&s.host()), &[])
        .assert_forbidden("api__guard__no_token");
    // a method the route does not take
    let req = format!(
        "POST /api/version HTTP/1.1\r\nHost: {}\r\nx-dashboard-token: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        s.host(),
        s.token
    );
    let r = s.raw(&req);
    assert_eq!(r.status, 405);
    r.assert_csp();
}

#[test]
fn binds_only_127_0_0_1() {
    let s = Server::start();
    let reach =
        |addr: SocketAddr| TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok();
    assert!(reach(SocketAddr::from(([127, 0, 0, 1], s.port))));
    // a wildcard bind would answer on every other loopback address and on IPv6
    assert!(
        !reach(SocketAddr::from(([127, 0, 0, 2], s.port))),
        "answers on 127.0.0.2: not bound to 127.0.0.1 only"
    );
    assert!(!reach(
        "[::1]:0"
            .parse::<SocketAddr>()
            .map(|a| SocketAddr::new(a.ip(), s.port))
            .unwrap()
    ));
}

#[test]
fn ctrl_c_stops_the_server_frees_the_port_and_leaks_no_token() {
    let mut s = Server::start();
    // exercise the refusal paths so that any logging of them would show
    s.get(
        "/api/version",
        Some(&s.host()),
        &[("x-dashboard-token", "wrong")],
    );
    s.get(
        &format!("/api/version?token={}", s.token),
        Some(&s.host()),
        &[],
    );
    assert_eq!(s.authed("/api/version", &[]).status, 200);
    let (port, token) = (s.port, s.token.clone());
    let (status, rest, err) = s.stop_with_sigint();
    assert!(status.success(), "{status:?}");
    assert!(
        !rest.contains(&token) && !err.contains(&token),
        "token leaked: {rest:?} {err:?}"
    );
    assert!(
        TcpListener::bind(("127.0.0.1", port)).is_ok(),
        "port still held"
    );
}

// ---- T014: usage, warnings and plan over HTTP -------------------------------------------------------------
// The oracle is the Python output in `rust/parity/golden*/`. Each server gets a temp copy of a parity store
// (never the fixture itself, never `~/.easy-personal`) and `TZ=UTC`, the zone the goldens were made under.
// `warnings` and `plan` depend on the current time, so they are compared in-process with a fixed clock
// (`src/api_tests.rs`); there is no env var or flag in the binary that changes the clock.

const FORBIDDEN_ROUTES: [&str; 6] = [
    "/api/usage/summary",
    "/api/usage/breakdown?by=day",
    "/api/warnings",
    "/api/plan",
    "/api/usage/sessions",
    "/api/usage/outliers",
];

fn parity(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity")
        .join(rel)
}

/// Percent-encodes everything but RFC 3986 unreserved characters (a space becomes `%20`).
fn urlencode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The manifest's usage entries that carry no `workspace` / `account` (`scoped` false) or that do (`scoped` true):
/// `(request path, status, golden)`.
fn usage_entries(dir: &str, scoped: bool) -> Vec<(String, u16, serde_json::Value)> {
    let read = |rel: String| -> serde_json::Value {
        serde_json::from_str(&std::fs::read_to_string(parity(&rel)).unwrap()).unwrap()
    };
    let manifest = read(format!("{dir}/manifest.json"));
    manifest["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| {
            // the `api__guard__*` entries name a usage route too, but they are 403 cases (tested below)
            let file = e["file"].as_str().unwrap();
            (file.starts_with("api__usage_summary__") || file.starts_with("api__usage_breakdown__"))
                && (e["params"].get("workspace").is_some() || e["params"].get("account").is_some())
                    == scoped
        })
        .map(|e| {
            let query: Vec<String> = e["params"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v.as_str().unwrap())))
                .collect();
            let path = format!("{}?{}", e["route"].as_str().unwrap(), query.join("&"));
            let golden = read(format!("{dir}/{}", e["file"].as_str().unwrap()));
            (path, e["status"].as_u64().unwrap() as u16, golden)
        })
        .collect()
}

fn assert_json_response(r: &Response, status: u16) {
    assert_eq!(r.status, status, "{}", r.body);
    assert_eq!(r.header("content-type"), vec!["application/json"]);
    r.assert_csp();
}

#[test]
fn usage_routes_equal_the_goldens_over_the_default_store() {
    let entries = usage_entries("golden", false);
    assert_eq!(
        entries.len(),
        8,
        "summary all/window, breakdown day/model/model_window/project/tool/bad_by"
    );
    let s = Server::start_with_store(&parity("store-v11.sqlite"));
    for (path, status, golden) in entries {
        let r = s.authed(&path, &[]);
        assert_json_response(&r, status);
        assert_eq!(r.json(), golden, "{path}");
    }
}

#[test]
fn usage_routes_equal_the_goldens_over_the_rich_store() {
    let entries = usage_entries("golden-rich", false);
    assert_eq!(
        entries.len(),
        13,
        "the rich manifest's unscoped usage entries"
    );
    let s = Server::start_with_store(&parity("store-rich/store-v11.sqlite"));
    for (path, status, golden) in entries {
        let r = s.authed(&path, &[]);
        assert_json_response(&r, status);
        assert_eq!(r.json(), golden, "{path}");
    }
}

#[test]
fn warnings_and_plan_answer_over_the_binary_with_the_real_clock() {
    // Only the shape: the answer depends on the current time, the exact bodies are compared in-process.
    let s = Server::start_with_store(&parity("store-v11.sqlite"));
    let warnings = s.authed("/api/warnings", &[]);
    assert_json_response(&warnings, 200);
    assert!(warnings.json()["hits"].is_array() && warnings.json()["abstained"].is_array());
    let plan = s.authed("/api/plan", &[]);
    assert_json_response(&plan, 200);
    assert!(plan.json()["groups"].is_array());
}

#[test]
fn the_query_is_decoded_like_starlette() {
    let s = Server::start_with_store(&parity("store-v11.sqlite"));
    let window = golden("api__usage_summary__window");
    let get = |q: &str| s.authed(&format!("/api/usage/summary?{q}"), &[]);
    let invalid =
        |value: &str| serde_json::json!({"error": format!("Invalid isoformat string: {value}")});
    // percent-decoding: `%3A` is `:`
    let r = get("since=2026-01-15T10%3A05%3A00Z&until=2026-01-15T10%3A15%3A00Z");
    assert_json_response(&r, 200);
    assert_eq!(r.json(), window);
    // `%2B` is a literal `+`, so a `+07:00` offset survives when it is encoded
    let r = get("since=2026-01-15T17%3A05%3A00%2B07%3A00&until=2026-01-15T10:15:00Z");
    assert_eq!(r.json(), window);
    // a bare `+` is a space (Python: `since="2026-01-15T10:05:00 00:00"`, which is not ISO)
    let r = get("since=2026-01-15T10:05:00+00:00");
    assert_json_response(&r, 400);
    assert_eq!(r.json(), invalid("'2026-01-15T10:05:00 00:00'"));
    // a repeated key: the last value wins, in either order
    let r = get("since=garbage&since=2026-01-15T10:05:00Z&until=2026-01-15T10:15:00Z");
    assert_eq!(r.json(), window);
    let r = get("since=2026-01-15T10:05:00Z&since=garbage");
    assert_eq!(r.json(), invalid("'garbage'"));
    // a key is decoded too
    let r = get("s%69nce=2026-01-15T10:05:00Z&until=2026-01-15T10:15:00Z");
    assert_eq!(r.json(), window);
    // an empty value, and a key without `=`, are the empty string (an error, not "no bound")
    for q in ["since=", "since", "since=&until=2026-01-15T10:15:00Z"] {
        let r = get(q);
        assert_json_response(&r, 400);
        assert_eq!(r.json(), invalid("''"), "{q}");
    }
    // an empty pair (`&&`) and a trailing `&` are skipped
    assert_eq!(
        get("&&since=2026-01-15T10:05:00Z&&until=2026-01-15T10:15:00Z&").json(),
        window
    );
    // an unknown key is ignored
    assert_eq!(get("nothing=1").json(), golden("api__usage_summary__all"));
}

#[test]
fn odd_queries_never_panic() {
    let s = Server::start_with_store(&parity("store-v11.sqlite"));
    // percent escapes that are not UTF-8 become U+FFFD, as Python's `unquote` does: a 400 naming it
    let r = s.authed("/api/usage/summary?since=%ff%fe", &[]);
    assert_json_response(&r, 400);
    assert_eq!(
        r.json(),
        serde_json::json!({"error": "Invalid isoformat string: '\u{fffd}\u{fffd}'"})
    );
    // a lone or malformed `%` stays as typed
    let r = s.authed("/api/usage/summary?since=%&until=%zz", &[]);
    assert_json_response(&r, 400);
    assert_eq!(
        r.json(),
        serde_json::json!({"error": "Invalid isoformat string: '%'"})
    );
    // many repeated keys: the last one wins
    let many = "by=day&".repeat(3000) + "by=model";
    let r = s.authed(&format!("/api/usage/breakdown?{many}"), &[]);
    assert_eq!(r.json(), golden("api__usage_breakdown__model"));
    // one long value: Python echoes it in the 400 text, so does this
    let r = s.authed(
        &format!("/api/usage/summary?since={}", "9".repeat(60_000)),
        &[],
    );
    assert_eq!(r.status, 400);
    // raw non-ASCII bytes in the target are refused by the HTTP layer before routing (a 4xx), never a 200
    let r = s.get(
        "/api/usage/summary?since=é",
        Some(&s.host()),
        &[("x-dashboard-token", &s.token)],
    );
    assert!((400..500).contains(&r.status), "{}", r.status);
    assert_eq!(
        s.authed("/api/usage/summary", &[]).status,
        200,
        "still alive"
    );
}

const RICH_STORE: &str = "store-rich/store-v11.sqlite";

/// Runs every scoped usage entry of the rich manifest against `server`: `(compared, entries that differ)`.
fn compare_scoped_usage(server: &Server) -> (usize, Vec<String>) {
    let entries = usage_entries("golden-rich", true);
    let (mut compared, mut wrong) = (0, Vec::new());
    for (path, status, golden) in entries {
        compared += 1;
        let r = server.authed(&path, &[]);
        if r.status != status
            || serde_json::from_str::<serde_json::Value>(&r.body).ok() != Some(golden)
        {
            wrong.push(format!("{path}: {} {}", r.status, r.body));
        }
    }
    (compared, wrong)
}

#[test]
fn scoped_usage_routes_equal_the_goldens_over_the_rich_store() {
    let entries = usage_entries("golden-rich", true);
    let count = |route: &str| {
        entries
            .iter()
            .filter(|(p, _, _)| p.starts_with(route))
            .count()
    };
    assert_eq!(
        count("/api/usage/summary?"),
        8,
        "summary: workspace, account, both, window, unknown x2"
    );
    assert_eq!(count("/api/usage/breakdown?"), 11);
    assert_eq!(
        entries
            .iter()
            .filter(|(_, status, _)| *status == 400)
            .count(),
        2
    );
    let s = Server::start_with_store(&parity(RICH_STORE));
    for (path, status, golden) in entries {
        let r = s.authed(&path, &[]);
        assert_json_response(&r, status);
        assert_eq!(r.json(), golden, "{path}");
    }
}

#[test]
fn an_empty_workspace_or_account_is_no_filter_over_the_binary() {
    let s = Server::start_with_store(&parity(RICH_STORE));
    for (path, golden_file) in [
        (
            "/api/usage/summary?workspace=&account=",
            "api__usage_summary__all",
        ),
        (
            "/api/usage/summary?workspace&account",
            "api__usage_summary__all",
        ),
        (
            "/api/usage/summary?workspace=nope&workspace=",
            "api__usage_summary__all",
        ),
        (
            "/api/usage/breakdown?by=model&workspace=&account=",
            "api__usage_breakdown__model",
        ),
    ] {
        let r = s.authed(path, &[]);
        assert_json_response(&r, 200);
        let golden: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(parity(&format!("golden-rich/{golden_file}.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(r.json(), golden, "{path}");
    }
}

#[test]
fn an_unknown_workspace_or_account_is_400_on_all_four_routes_and_a_known_one_is_answered() {
    let s = Server::start_with_store(&parity(RICH_STORE));
    let account = "unknown account; valid labels: Claude default, Claude work";
    for route in [
        "/api/usage/summary?",
        "/api/usage/breakdown?by=model&",
        "/api/warnings?",
        "/api/plan?",
    ] {
        for (query, text) in [
            ("workspace=nope", "unknown workspace"),
            ("account=nope", account),
            ("workspace=nope&account=nope", "unknown workspace"),
            ("workspace=%2Fetc%2Fpasswd", "unknown workspace"),
        ] {
            let r = s.authed(&format!("{route}{query}"), &[]);
            assert_json_response(&r, 400);
            assert_eq!(
                r.json(),
                serde_json::json!({ "error": text }),
                "{route}{query}"
            );
        }
        // known scopes are answered (warnings and plan read the real clock here: only their shape is checked)
        for query in [
            "workspace=acme",
            "account=Claude+work",
            "workspace=acme&account=Claude%20work",
        ] {
            let r = s.authed(&format!("{route}{query}"), &[]);
            assert_json_response(&r, 200);
        }
    }
    let r = s.authed("/api/warnings?workspace=acme", &[]);
    assert!(r.json()["hits"].is_array() && r.json()["abstained"].is_array());
    let r = s.authed("/api/plan?account=1", &[]);
    assert!(r.json()["groups"].is_array());
    // `by` is still checked first
    let r = s.authed("/api/usage/breakdown?by=bad&workspace=nope", &[]);
    assert_json_response(&r, 400);
    assert_eq!(
        r.json(),
        serde_json::json!({"error": "by must be one of day, model, project, tool"})
    );
}

#[test]
fn a_workspace_or_account_is_never_logged_and_never_reaches_a_refused_request() {
    let mut s = Server::start_with_store(&parity(RICH_STORE));
    let marker = "ZZ-SCOPE-MARKER-7431";
    let host = s.host();
    for route in FORBIDDEN_ROUTES {
        let sep = if route.contains('?') { "&" } else { "?" };
        for key in ["workspace", "account"] {
            let query = format!("{route}{sep}{key}={marker}");
            // refused by the guard: the body is the guard's, with nothing of the query in it
            let r = s.get(&query, Some(&host), &[]);
            r.assert_forbidden("api__guard__no_token");
            assert!(!r.body.contains(marker), "{}", r.body);
            // an unknown value: Python's fixed text, no echo
            let r = s.authed(&query, &[]);
            assert_json_response(&r, 400);
            assert!(!r.body.contains(marker), "{}", r.body);
        }
    }
    let (status, rest, err) = s.stop_with_sigint();
    assert!(status.success(), "{status:?}");
    assert!(
        !rest.contains(marker) && !err.contains(marker),
        "scope value logged: {rest:?} {err:?}"
    );
}

#[test]
fn moving_one_event_to_the_other_account_breaks_the_scoped_usage_comparison() {
    let make = |changed: bool| {
        Server::launch(
            |home| {
                let store = home.join("store.sqlite");
                std::fs::copy(parity(RICH_STORE), &store).unwrap();
                if changed {
                    // the first priced message of account 2 now belongs to account 1
                    let n = rusqlite::Connection::open(&store)
                        .unwrap()
                        .execute(
                            "UPDATE events SET agent_account_id = 1 WHERE id = \
                             (SELECT MIN(id) FROM events WHERE agent_account_id = 2 AND input_tokens IS NOT NULL)",
                            [],
                        )
                        .unwrap();
                    assert_eq!(n, 1);
                }
            },
            &[("TZ", "UTC")],
        )
    };
    let (compared, wrong) = compare_scoped_usage(&make(false));
    assert_eq!((compared, wrong.len()), (19, 0), "{wrong:#?}");
    let (compared, wrong) = compare_scoped_usage(&make(true));
    assert_eq!(compared, 19);
    assert!(
        wrong.iter().any(|w| w.contains("account=Claude%20work")),
        "{wrong:#?}"
    );
    assert!(wrong.iter().any(|w| w.contains("account=1")), "{wrong:#?}");
}

#[test]
fn a_missing_store_is_503_and_the_data_folder_stays_empty() {
    let s = Server::start();
    for route in FORBIDDEN_ROUTES {
        let r = s.authed(route, &[]);
        assert_json_response(&r, 503);
        assert_eq!(
            r.json(),
            serde_json::json!({"error": "no store yet: nothing has been ingested into the data folder"}),
            "{route}"
        );
    }
    assert_eq!(
        std::fs::read_dir(&s.home).unwrap().count(),
        0,
        "the server must not create a store"
    );
}

#[test]
fn a_newer_unreadable_or_non_file_store_is_503_with_a_fixed_text_and_no_path() {
    let newer = Server::launch(
        |home| {
            let store = home.join("store.sqlite");
            std::fs::copy(parity("store-v11.sqlite"), &store).unwrap();
            rusqlite::Connection::open(&store)
                .unwrap()
                .execute_batch("PRAGMA user_version = 12")
                .unwrap();
        },
        &[("TZ", "UTC")],
    );
    let garbage = Server::launch(
        |home| {
            std::fs::write(
                home.join("store.sqlite"),
                "this is not a database, PRIVATE-ROW-TEXT",
            )
            .unwrap()
        },
        &[],
    );
    let directory = Server::launch(
        |home| std::fs::create_dir(home.join("store.sqlite")).unwrap(),
        &[],
    );
    let fixed = "the store cannot be read right now; try again in a moment";
    for route in FORBIDDEN_ROUTES {
        let r = newer.authed(route, &[]);
        assert_json_response(&r, 503);
        assert_eq!(
            r.json(),
            serde_json::json!({"error": "the store is at version 12, newer than the 11 this build knows; update easy-personal-rs (nothing was changed)"}),
            "{route}"
        );
        for server in [&garbage, &directory] {
            let r = server.authed(route, &[]);
            assert_json_response(&r, 503);
            assert_eq!(r.json(), serde_json::json!({ "error": fixed }), "{route}");
            let home = server.home.to_string_lossy().into_owned();
            assert!(
                !r.body.contains(&home) && !r.body.contains("PRIVATE-ROW-TEXT"),
                "{}",
                r.body
            );
        }
    }
}

#[test]
fn the_guard_covers_the_new_routes() {
    let s = Server::start_with_store(&parity("store-v11.sqlite"));
    let host = s.host();
    for route in FORBIDDEN_ROUTES {
        let sep = if route.contains('?') { "&" } else { "?" };
        s.get(route, Some(&host), &[])
            .assert_forbidden("api__guard__no_token");
        s.get(route, Some(&host), &[("x-dashboard-token", "wrong")])
            .assert_forbidden("api__guard__no_token");
        s.get(&format!("{route}{sep}token={}", s.token), Some(&host), &[])
            .assert_forbidden("api__guard__no_token");
        s.get(
            route,
            Some(&format!("localhost:{}", s.port)),
            &[("x-dashboard-token", &s.token)],
        )
        .assert_forbidden("api__guard__bad_host");
        s.authed(route, &[("Origin", "http://evil.example")])
            .assert_forbidden("api__guard__bad_origin");
        assert_eq!(s.authed(route, &[]).status, 200, "{route}");
    }
}

#[test]
fn encoded_or_doubled_api_paths_are_never_served() {
    // The guard keys on the raw path; the router matches the raw path too, so no encoded form reaches a handler.
    let s = Server::start_with_store(&parity("store-v11.sqlite"));
    for path in [
        "/%61pi/usage/summary",
        "//api/usage/summary",
        "/api%2fusage%2fsummary",
        "/api%2Fusage%2Fsummary",
        "/api/usage/../usage/summary",
        "/api/./usage/summary",
        "/api/usage/summary/",
        "/%61pi/usage/sessions",
        "//api/usage/outliers",
        "/api%2fusage%2fsessions",
        "/api%2Fusage%2Foutliers",
        "/api/usage/../usage/sessions",
        "/api/usage/sessions/",
        "/api/usage/outliers/",
    ] {
        for with_token in [true, false] {
            let r = if with_token {
                s.authed(path, &[])
            } else {
                s.get(path, Some(&s.host()), &[])
            };
            assert!(r.status == 404 || r.status == 403, "{path}: {}", r.status);
            r.assert_csp();
            assert!(!r.body.contains("sessions"), "{path}: {}", r.body);
        }
    }
}

#[test]
fn head_is_answered_like_get_and_other_methods_are_405() {
    // Python (Starlette): a function route is GET plus HEAD; POST, PUT and DELETE answer 405 `Method Not Allowed`
    // with `Allow: GET, HEAD`, after the guard. axum does the same (an empty 405 body, the CSP still set).
    let s = Server::start_with_store(&parity("store-v11.sqlite"));
    let send = |method: &str, route: &str| {
        s.raw(&format!(
            "{method} {route} HTTP/1.1\r\nHost: {}\r\nx-dashboard-token: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            s.host(),
            s.token
        ))
    };
    for route in FORBIDDEN_ROUTES {
        let head = send("HEAD", route);
        assert_eq!(head.status, 200, "{route}");
        assert!(head.body.is_empty(), "{:?}", head.body);
        head.assert_csp();
        for method in ["POST", "PUT", "DELETE"] {
            let r = send(method, route);
            assert_eq!(r.status, 405, "{method} {route}");
            r.assert_csp();
        }
    }
    // the guard still comes first: no token, no 405
    let r = s.raw(&format!(
        "POST /api/plan HTTP/1.1\r\nHost: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        s.host()
    ));
    r.assert_forbidden("api__guard__no_token");
}

// ---- T020: `/api/usage/sessions` and `/api/usage/outliers` ---------------------------------------------------
// The oracle is the manifests' `api__usage_sessions__*` and `api__usage_outliers__*` entries (default store 4, rich
// store 16, `bad_since` included). Python's `/api/usage/sessions` passes `detail_sessions=50`; every golden carries
// `"detail_sessions": 50`, so an equal answer proves it.

/// One manifest entry of the two routes: the request, the status, the golden, and whether only the status and the
/// `error` key are compared (the entry's `compare` note says the message text is CPython-specific).
struct SessionEntry {
    path: String,
    status: u16,
    golden: serde_json::Value,
    error_key_only: bool,
}

fn session_entries(dir: &str) -> Vec<SessionEntry> {
    let read = |rel: String| -> serde_json::Value {
        serde_json::from_str(&std::fs::read_to_string(parity(&rel)).unwrap()).unwrap()
    };
    read(format!("{dir}/manifest.json"))["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| {
            let file = e["file"].as_str().unwrap();
            file.starts_with("api__usage_sessions__") || file.starts_with("api__usage_outliers__")
        })
        .map(|e| {
            let query: Vec<String> = e["params"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v.as_str().unwrap())))
                .collect();
            SessionEntry {
                path: format!("{}?{}", e["route"].as_str().unwrap(), query.join("&")),
                status: e["status"].as_u64().unwrap() as u16,
                golden: read(format!("{dir}/{}", e["file"].as_str().unwrap())),
                error_key_only: e.get("compare").is_some(),
            }
        })
        .collect()
}

/// Runs every session / outlier entry of `dir` against `server`: `(compared, entries that differ)`.
fn compare_session_routes(server: &Server, dir: &str) -> (usize, Vec<String>) {
    let (mut compared, mut wrong) = (0, Vec::new());
    for entry in session_entries(dir) {
        compared += 1;
        let r = server.authed(&entry.path, &[]);
        let body = serde_json::from_str::<serde_json::Value>(&r.body).ok();
        let same = if entry.error_key_only {
            // an `error` key holding a string, and nothing else
            r.status == entry.status
                && body.as_ref().is_some_and(|b| {
                    b.as_object()
                        .is_some_and(|o| o.len() == 1 && o["error"].is_string())
                })
        } else {
            r.status == entry.status && body == Some(entry.golden)
        };
        if !same {
            wrong.push(format!("{}: {} {}", entry.path, r.status, r.body));
        }
    }
    (compared, wrong)
}

fn manifest_count(dir: &str, route: &str) -> usize {
    session_entries(dir)
        .iter()
        .filter(|e| e.path.starts_with(route))
        .count()
}

#[test]
fn sessions_and_outliers_equal_the_goldens_over_the_default_store() {
    // counted from the manifest: sessions all / window / bad_since, outliers all
    assert_eq!(manifest_count("golden", "/api/usage/sessions"), 3);
    assert_eq!(manifest_count("golden", "/api/usage/outliers"), 1);
    let s = Server::start_with_store(&parity("store-v11.sqlite"));
    for entry in session_entries("golden") {
        let r = s.authed(&entry.path, &[]);
        assert_json_response(&r, entry.status);
        assert_eq!(r.json(), entry.golden, "{}", entry.path);
    }
    let (compared, wrong) = compare_session_routes(&s, "golden");
    assert_eq!((compared, wrong.len()), (4, 0), "{wrong:#?}");
}

#[test]
fn sessions_and_outliers_equal_the_goldens_over_the_rich_store() {
    // sessions: all, window, window_mid, account_default, account_work, workspace_acme, workspace_acme_account_work,
    // workspace_personal, bad_since; outliers: all, window, account_default, account_work, workspace_acme,
    // workspace_acme_account_work, workspace_personal
    assert_eq!(manifest_count("golden-rich", "/api/usage/sessions"), 9);
    assert_eq!(manifest_count("golden-rich", "/api/usage/outliers"), 7);
    let s = Server::start_with_store(&parity(RICH_STORE));
    let (compared, wrong) = compare_session_routes(&s, "golden-rich");
    assert_eq!((compared, wrong.len()), (16, 0), "{wrong:#?}");
    for entry in session_entries("golden-rich") {
        let r = s.authed(&entry.path, &[]);
        assert_json_response(&r, entry.status);
        if !entry.error_key_only {
            assert_eq!(r.json(), entry.golden, "{}", entry.path);
        }
    }
}

#[test]
fn a_bad_period_is_400_on_both_routes_with_pythons_text_and_before_an_unknown_scope() {
    let s = Server::start_with_store(&parity(RICH_STORE));
    let bad =
        |value: &str| serde_json::json!({"error": format!("Invalid isoformat string: {value}")});
    for route in ["/api/usage/sessions", "/api/usage/outliers"] {
        for (query, text) in [
            ("since=bad", "'bad'"),
            ("until=bad", "'bad'"),
            // both bad: Python reads `since` first
            ("since=bad&until=worse", "'bad'"),
            ("until=worse&since=bad", "'bad'"),
            // a bad period wins over an unknown scope
            ("since=bad&workspace=nope", "'bad'"),
            ("until=bad&account=nope", "'bad'"),
            ("since=", "''"),
            ("until", "''"),
        ] {
            let r = s.authed(&format!("{route}?{query}"), &[]);
            assert_json_response(&r, 400);
            assert_eq!(r.json(), bad(text), "{route}?{query}");
        }
    }
}

#[test]
fn an_unknown_workspace_or_account_is_400_on_both_routes_and_an_empty_one_is_no_filter() {
    let s = Server::start_with_store(&parity(RICH_STORE));
    let account = "unknown account; valid labels: Claude default, Claude work";
    let read = |name: &str| -> serde_json::Value {
        serde_json::from_str(
            &std::fs::read_to_string(parity(&format!("golden-rich/{name}.json"))).unwrap(),
        )
        .unwrap()
    };
    for (route, all) in [
        ("/api/usage/sessions", "api__usage_sessions__all"),
        ("/api/usage/outliers", "api__usage_outliers__all"),
    ] {
        for (query, text) in [
            ("workspace=nope", "unknown workspace"),
            ("account=nope", account),
            ("workspace=nope&account=nope", "unknown workspace"),
            ("workspace=%2Fetc%2Fpasswd", "unknown workspace"),
        ] {
            let r = s.authed(&format!("{route}?{query}"), &[]);
            assert_json_response(&r, 400);
            assert_eq!(
                r.json(),
                serde_json::json!({ "error": text }),
                "{route}?{query}"
            );
        }
        for query in [
            "workspace=&account=",
            "workspace&account",
            "workspace=nope&workspace=&account=nope&account=",
        ] {
            let r = s.authed(&format!("{route}?{query}"), &[]);
            assert_json_response(&r, 200);
            assert_eq!(r.json(), read(all), "{route}?{query}");
        }
    }
}

#[test]
fn the_query_rules_apply_to_the_session_routes() {
    let s = Server::start_with_store(&parity("store-v11.sqlite"));
    let sessions_window = golden("api__usage_sessions__window");
    let outliers_all = golden("api__usage_outliers__all");
    let get = |q: &str| s.authed(&format!("/api/usage/sessions?{q}"), &[]);
    // percent-decoding, an encoded `+`, a repeated key (the last wins), a decoded key
    for q in [
        "since=2026-01-15T10%3A05%3A00Z&until=2026-01-15T10%3A15%3A00Z",
        "since=2026-01-15T17%3A05%3A00%2B07%3A00&until=2026-01-15T10:15:00Z",
        "since=garbage&since=2026-01-15T10:05:00Z&until=2026-01-15T10:15:00Z",
        "s%69nce=2026-01-15T10:05:00Z&&until=2026-01-15T10:15:00Z&",
    ] {
        let r = get(q);
        assert_json_response(&r, 200);
        assert_eq!(r.json(), sessions_window, "{q}");
    }
    // a bare `+` is a space: not an ISO date
    let r = get("since=2026-01-15T10:05:00+00:00");
    assert_json_response(&r, 400);
    assert_eq!(
        r.json(),
        serde_json::json!({"error": "Invalid isoformat string: '2026-01-15T10:05:00 00:00'"})
    );
    let r = s.authed("/api/usage/outliers?nothing=1", &[]);
    assert_eq!(r.json(), outliers_all);
    // many repeated keys, one very long value and raw non-ASCII bytes never panic
    let many = "since=garbage&".repeat(3000) + "since=2026-01-15T10:05:00Z";
    let r = s.authed(&format!("/api/usage/sessions?{many}"), &[]);
    assert_eq!(r.status, 200);
    let r = s.authed(
        &format!("/api/usage/outliers?until={}", "9".repeat(60_000)),
        &[],
    );
    assert_eq!(r.status, 400);
    let r = s.get(
        "/api/usage/sessions?since=é",
        Some(&s.host()),
        &[("x-dashboard-token", &s.token)],
    );
    assert!((400..500).contains(&r.status), "{}", r.status);
    assert_eq!(s.authed("/api/usage/sessions", &[]).status, 200);
}

#[test]
fn changing_one_timestamp_breaks_the_sessions_comparison_and_names_the_entry() {
    let make = |changed: bool| {
        Server::launch(
            |home| {
                let store = home.join("store.sqlite");
                std::fs::copy(parity("store-v11.sqlite"), &store).unwrap();
                if changed {
                    // the first message of the first session moves 30 s
                    let n = rusqlite::Connection::open(&store)
                        .unwrap()
                        .execute(
                            "UPDATE events SET ts_utc = '2026-01-15T10:01:30.000000Z' WHERE id = 1",
                            [],
                        )
                        .unwrap();
                    assert_eq!(n, 1);
                }
            },
            &[("TZ", "UTC")],
        )
    };
    let (compared, wrong) = compare_session_routes(&make(false), "golden");
    assert_eq!((compared, wrong.len()), (4, 0), "{wrong:#?}");
    let (compared, wrong) = compare_session_routes(&make(true), "golden");
    assert_eq!(compared, 4);
    assert!(
        wrong.iter().any(|w| w.starts_with("/api/usage/sessions?")),
        "{wrong:#?}"
    );
}

#[test]
fn the_session_routes_write_nothing_but_the_two_startup_lines() {
    let mut s = Server::start_with_store(&parity(RICH_STORE));
    let marker = "ZZ-SESSION-MARKER-5519";
    let token = s.token.clone();
    assert_eq!(s.first_lines.len(), 2);
    for route in ["/api/usage/sessions", "/api/usage/outliers"] {
        assert_eq!(s.authed(route, &[]).status, 200);
        assert_eq!(
            s.authed(&format!("{route}?since={marker}"), &[]).status,
            400
        );
        assert_eq!(
            s.authed(&format!("{route}?workspace={marker}"), &[]).status,
            400
        );
        s.get(route, Some(&s.host()), &[])
            .assert_forbidden("api__guard__no_token");
    }
    let (status, rest, err) = s.stop_with_sigint();
    assert!(status.success(), "{status:?}");
    assert_eq!(rest, "", "stdout after the startup lines");
    assert_eq!(err, "", "stderr");
    assert!(!rest.contains(marker) && !err.contains(&token));
}

#[test]
fn serving_never_touches_the_parity_fixtures() {
    let stores = [
        parity("store-v11.sqlite"),
        parity("store-rich/store-v11.sqlite"),
    ];
    let before: Vec<Vec<u8>> = stores.iter().map(|p| std::fs::read(p).unwrap()).collect();
    for store in &stores {
        let s = Server::start_with_store(store);
        for route in FORBIDDEN_ROUTES {
            assert_eq!(s.authed(route, &[]).status, 200, "{route}");
        }
    }
    for (store, bytes) in stores.iter().zip(before) {
        assert_eq!(&std::fs::read(store).unwrap(), &bytes, "{store:?} changed");
        for suffix in ["-wal", "-shm"] {
            let mut side = store.clone().into_os_string();
            side.push(suffix);
            assert!(!PathBuf::from(side).exists(), "{suffix} next to {store:?}");
        }
    }
}

// ---- T021: `serve --static-dir <DIR>` -----------------------------------------------------------------------
// The oracle is Starlette's `StaticFiles(html=True)` for the cases the guide lists, and the T016 findings (the page
// and its script load without a token, `/api/*` still needs it, the dashboard CSP is on every answer). Every test uses a
// fixture folder made here, with a sibling "outside" folder that holds a marker no response may ever carry.

const OUTSIDE_MARKER: &str = "OUTSIDE-SECRET-MARKER-7731";
const MIB: u64 = 1024 * 1024;

/// `<temp>/site` is the served folder; `<temp>/outside` and `<temp>/site-evil` (a sibling whose name starts with the
/// same text) hold the marker. Removed on drop.
struct Fixture {
    top: PathBuf,
    site: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        use std::os::unix::fs::symlink;
        static N: AtomicU32 = AtomicU32::new(0);
        let top = std::env::temp_dir().join(format!(
            "easy-personal-rs-t021-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let site = top.join("site");
        let write = |rel: &str, body: &[u8]| {
            let path = top.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, body).unwrap();
        };
        write("outside/secret.txt", OUTSIDE_MARKER.as_bytes());
        write("outside/index.html", OUTSIDE_MARKER.as_bytes());
        write("site-evil/secret.txt", OUTSIDE_MARKER.as_bytes());
        write("site/index.html", b"<!doctype html><title>INDEX</title>");
        write("site/app.js", b"console.log('root app');");
        write("site/assets/app.js", b"console.log('asset app');");
        write("site/assets/style.css", b"body{margin:0}");
        write("site/data.json", b"{\"a\":1}");
        write(
            "site/logo.svg",
            b"<svg xmlns='http://www.w3.org/2000/svg'/>",
        );
        write(
            "site/pic.png",
            &[0x89, b'P', b'N', b'G', 0, 255, 254, 10, 13],
        );
        write("site/favicon.ico", &[0, 0, 1, 0, 0xff]);
        write("site/font.woff2", &[b'w', b'O', b'F', b'2', 0, 0xfe]);
        write("site/notes.txt", b"plain notes");
        write("site/noext", b"no extension");
        write("site/empty.txt", b"");
        write("site/docs/index.html", b"DOCS-INDEX");
        write("site/docs/page.html", b"DOCS-PAGE");
        write("site/dir.js/index.html", b"DIR-DOT-JS");
        write("site/\u{fc}n\u{ef}.txt", b"utf8 name");
        write("site/.env", b"DOTFILE-SECRET");
        write("site/.web-stamp", b"DOTFILE-STAMP");
        write("site/docs/.hidden", b"DOTFILE-HIDDEN");
        write("site/.hiddendir/inner.txt", b"DOTDIR-INNER");
        write("site/api/usage/summary", b"STATIC-API-FILE");
        write("site/api/extra", b"STATIC-API-EXTRA");
        std::fs::create_dir_all(site.join("emptydir")).unwrap();
        symlink("../outside/secret.txt", site.join("link-out")).unwrap();
        symlink("../outside", site.join("link-dir-out")).unwrap();
        symlink("../site-evil/secret.txt", site.join("link-prefix")).unwrap();
        symlink(top.join("outside/secret.txt"), site.join("link-abs-out")).unwrap();
        symlink("index.html", site.join("link-in")).unwrap();
        symlink("docs", site.join("link-dir-in")).unwrap();
        symlink(".env", site.join("link-to-dotfile")).unwrap();
        symlink("no-such-file", site.join("link-dangling")).unwrap();
        let fifo = Command::new("mkfifo")
            .arg(site.join("fifo"))
            .status()
            .unwrap();
        assert!(fifo.success(), "mkfifo");
        Fixture { top, site }
    }

    /// Every entry below the served folder: relative path, kind, and the bytes (a symlink: its target).
    fn snapshot(&self) -> Vec<(String, String)> {
        fn walk(dir: &Path, top: &Path, out: &mut Vec<(String, String)>) {
            let mut entries: Vec<_> = std::fs::read_dir(dir)
                .unwrap()
                .map(|e| e.unwrap())
                .collect();
            entries.sort_by_key(|e| e.file_name());
            for entry in entries {
                let path = entry.path();
                let rel = path.strip_prefix(top).unwrap().display().to_string();
                let kind = entry.file_type().unwrap();
                if kind.is_symlink() {
                    out.push((
                        rel,
                        format!("link {:?}", std::fs::read_link(&path).unwrap()),
                    ));
                } else if kind.is_dir() {
                    out.push((rel, "dir".into()));
                    walk(&path, top, out);
                } else if kind.is_file() {
                    let len = std::fs::metadata(&path).unwrap().len();
                    let body = if len <= MIB {
                        std::fs::read(&path).unwrap()
                    } else {
                        Vec::new()
                    };
                    out.push((rel, format!("file {len} {body:?}")));
                } else {
                    out.push((rel, "other".into()));
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.site, &self.site, &mut out);
        out
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.top);
    }
}

impl Server {
    fn start_static(dir: &Path) -> Self {
        Self::launch_args(
            |_| {},
            &[],
            &["--static-dir".as_ref(), dir.as_os_str()],
            None,
        )
    }

    /// Host only, no token: what a page load sends.
    fn page(&self, path: &str) -> Response {
        self.get(path, Some(&self.host()), &[])
    }

    fn method(&self, method: &str, path: &str) -> Response {
        self.raw(&format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            self.host()
        ))
    }
}

/// `serve --static-dir` that must fail: its status, stdout and stderr (the child is killed if it keeps running).
fn serve_refused(
    args: &[&std::ffi::OsStr],
    cwd: Option<&Path>,
) -> (std::process::ExitStatus, String, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"));
    command
        .arg("serve")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("serve did not exit for {args:?}");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let (mut out, mut err) = (String::new(), String::new());
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut err)
        .unwrap();
    (status, out, err)
}

#[test]
fn without_the_flag_the_root_and_any_file_path_stay_404() {
    // Pins today's answer: an empty 404 with the CSP, for every method. The fixture is not served because it is not named.
    let s = Server::start();
    for path in ["/", "/index.html", "/app.js", "/assets/app.js"] {
        let r = s.page(path);
        assert_eq!(r.status, 404, "{path}");
        assert_eq!(r.body, "", "{path}");
        r.assert_csp();
    }
    assert_eq!(s.method("POST", "/").status, 404);
}

#[test]
fn a_static_dir_that_is_not_a_directory_stops_serve_with_one_line_and_creates_nothing() {
    let fixture = Fixture::new();
    let missing = fixture.top.join("not-there-ZZ4471");
    let a_file = fixture.site.join("app.js");
    let before = fixture.snapshot();
    for arg in [missing.clone(), a_file, fixture.site.join("fifo")] {
        let (status, out, err) = serve_refused(&["--static-dir".as_ref(), arg.as_os_str()], None);
        assert!(!status.success(), "{arg:?}");
        assert_eq!(out, "", "nothing is printed before the check: {arg:?}");
        assert_eq!(err.lines().count(), 1, "one line: {err:?}");
        assert!(err.starts_with("easy-personal-rs: "), "{err:?}");
        assert!(
            !err.contains(fixture.top.to_str().unwrap()) && !err.contains("ZZ4471"),
            "the message names no path: {err:?}"
        );
    }
    assert!(!missing.exists(), "nothing is created");
    assert_eq!(fixture.snapshot(), before);
    // a flag with no value, or an empty one, is clap's own usage error
    for args in [
        &["--static-dir".as_ref()][..],
        &["--static-dir".as_ref(), "".as_ref()],
    ] {
        let (status, out, err) = serve_refused(args, None);
        assert!(
            !status.success() && out.is_empty() && err.contains("--static-dir"),
            "{err:?}"
        );
    }
}

#[test]
fn the_static_dir_may_be_given_with_a_trailing_slash_relative_or_through_a_symlink() {
    let fixture = Fixture::new();
    let via_link = fixture.top.join("site-link");
    std::os::unix::fs::symlink(&fixture.site, &via_link).unwrap();
    let slash = format!("{}/", fixture.site.display());
    let first = Server::launch_args(
        |_| {},
        &[],
        &["--static-dir".as_ref(), slash.as_ref()],
        None,
    );
    let second = Server::launch_args(
        |_| {},
        &[],
        &["--static-dir".as_ref(), "site".as_ref()],
        Some(&fixture.top),
    );
    let third = Server::launch_args(
        |_| {},
        &[],
        &["--static-dir".as_ref(), via_link.as_os_str()],
        None,
    );
    for s in [&first, &second, &third] {
        let r = s.page("/");
        assert_eq!(r.status, 200);
        assert!(r.body.contains("INDEX"));
        // the canonical root is what counts: a symlink inside that points out is still refused
        assert_eq!(s.page("/link-out").status, 404);
        assert_eq!(s.page("/link-in").status, 200);
        assert_eq!(s.page("/../outside/secret.txt").status, 404);
    }
}

#[test]
fn files_are_served_with_their_type_length_and_the_csp() {
    let fixture = Fixture::new();
    let s = Server::start_static(&fixture.site);
    for (path, content_type, body) in [
        (
            "/",
            "text/html; charset=utf-8",
            &b"<!doctype html><title>INDEX</title>"[..],
        ),
        (
            "/index.html",
            "text/html; charset=utf-8",
            b"<!doctype html><title>INDEX</title>",
        ),
        ("/docs/", "text/html; charset=utf-8", b"DOCS-INDEX"),
        (
            "/docs/index.html",
            "text/html; charset=utf-8",
            b"DOCS-INDEX",
        ),
        ("/docs/page.html", "text/html; charset=utf-8", b"DOCS-PAGE"),
        ("/dir.js/", "text/html; charset=utf-8", b"DIR-DOT-JS"),
        (
            "/app.js",
            "text/javascript; charset=utf-8",
            b"console.log('root app');",
        ),
        (
            "/assets/app.js",
            "text/javascript; charset=utf-8",
            b"console.log('asset app');",
        ),
        (
            "/assets/style.css",
            "text/css; charset=utf-8",
            b"body{margin:0}",
        ),
        ("/data.json", "application/json", b"{\"a\":1}"),
        (
            "/logo.svg",
            "image/svg+xml",
            b"<svg xmlns='http://www.w3.org/2000/svg'/>",
        ),
        (
            "/pic.png",
            "image/png",
            &[0x89, b'P', b'N', b'G', 0, 255, 254, 10, 13],
        ),
        (
            "/favicon.ico",
            "image/vnd.microsoft.icon",
            &[0, 0, 1, 0, 0xff],
        ),
        (
            "/font.woff2",
            "font/woff2",
            &[b'w', b'O', b'F', b'2', 0, 0xfe],
        ),
        ("/notes.txt", "text/plain; charset=utf-8", b"plain notes"),
        ("/noext", "application/octet-stream", b"no extension"),
        ("/empty.txt", "text/plain; charset=utf-8", b""),
        (
            "/link-in",
            "text/html; charset=utf-8",
            b"<!doctype html><title>INDEX</title>",
        ),
        ("/link-dir-in/", "text/html; charset=utf-8", b"DOCS-INDEX"),
        (
            "/link-dir-in/page.html",
            "text/html; charset=utf-8",
            b"DOCS-PAGE",
        ),
        // percent-decoded UTF-8 works (the dashboard has no such name)
        (
            "/%C3%BCn%C3%AF.txt",
            "text/plain; charset=utf-8",
            b"utf8 name",
        ),
        // `%2e` inside a name is a dot, not a dot segment
        (
            "/assets/app%2ejs",
            "text/javascript; charset=utf-8",
            b"console.log('asset app');",
        ),
        // a query string (a token included) is ignored on a static path
        (
            "/app.js?token=abc&x=1",
            "text/javascript; charset=utf-8",
            b"console.log('root app');",
        ),
    ] {
        let r = s.page(path);
        assert_eq!(r.status, 200, "{path}: {}", r.body);
        assert_eq!(r.header("content-type"), vec![content_type], "{path}");
        assert_eq!(
            r.header("content-length"),
            vec![body.len().to_string()],
            "{path}"
        );
        assert_eq!(r.bytes, body, "{path}");
        r.assert_csp();
        assert!(!r.body.contains(OUTSIDE_MARKER));
        // HEAD: the same headers, no body
        let head = s.method("HEAD", path.split('?').next().unwrap());
        assert_eq!(head.status, 200, "HEAD {path}");
        assert_eq!(
            head.header("content-type"),
            vec![content_type],
            "HEAD {path}"
        );
        assert_eq!(
            head.header("content-length"),
            vec![body.len().to_string()],
            "HEAD {path}"
        );
        assert!(head.bytes.is_empty(), "HEAD {path}");
        head.assert_csp();
    }
}

#[test]
fn a_miss_is_a_fixed_404_never_a_listing_and_other_methods_are_405() {
    let fixture = Fixture::new();
    let s = Server::start_static(&fixture.site);
    let mut bodies = std::collections::BTreeSet::new();
    for path in [
        "/nothing-here-ZZ9",
        "/assets/missing.js",
        "/emptydir/", // a directory with no index.html: never a listing
        "/emptydir",
        "/docs",           // a directory without a slash: no redirect
        "/assets/",        // no index.html there
        "/app.js/",        // a file with a trailing slash
        "/assets//app.js", // an empty segment
        "/link-dangling",
    ] {
        let r = s.page(path);
        assert_eq!(r.status, 404, "{path}");
        assert!(r.header("location").is_empty(), "{path}: no redirect");
        r.assert_csp();
        assert!(
            !r.body.contains("ZZ9") && !r.body.contains("emptydir") && !r.body.contains(path),
            "{path}: {}",
            r.body
        );
        bodies.insert(r.body);
    }
    assert_eq!(bodies.len(), 1, "one fixed body: {bodies:?}");
    for method in ["POST", "PUT", "DELETE", "PATCH", "OPTIONS"] {
        for path in ["/", "/app.js", "/nothing-here-ZZ9"] {
            let r = s.method(method, path);
            assert_eq!(r.status, 405, "{method} {path}");
            assert_eq!(r.header("allow"), vec!["GET, HEAD"], "{method} {path}");
            r.assert_csp();
        }
    }
    // the guarded API keeps its own answers with the flag on
    let r = s.raw(&format!(
        "POST /api/version HTTP/1.1\r\nHost: {}\r\nx-dashboard-token: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        s.host(),
        s.token
    ));
    assert_eq!(r.status, 405);
    r.assert_csp();
}

#[test]
fn the_guard_covers_static_paths_and_needs_no_token_for_them() {
    let fixture = Fixture::new();
    let s = Server::start_static(&fixture.site);
    let host = s.host();
    for path in [
        "/",
        "/app.js",
        "/assets/app.js",
        "/docs/",
        "/nothing-here-ZZ9",
    ] {
        // no token: the page must load before it can read the fragment
        assert!(matches!(s.page(path).status, 200 | 404), "{path}");
        // a token in the query is simply ignored
        let with_query = s.get(&format!("{path}?token={}", s.token), Some(&host), &[]);
        assert_eq!(with_query.status, s.page(path).status, "{path}");
        // a wrong host, a foreign origin: 403 with the golden body and the CSP, hit or miss
        for bad in [
            format!("localhost:{}", s.port),
            "evil.example".to_string(),
            String::new(),
        ] {
            s.get(path, Some(&bad), &[])
                .assert_forbidden("api__guard__bad_host");
        }
        for origin in ["http://evil.example", "null", ""] {
            s.get(path, Some(&host), &[("Origin", origin)])
                .assert_forbidden("api__guard__bad_origin");
        }
        let own = format!("http://{host}");
        assert_ne!(
            s.get(path, Some(&host), &[("Origin", &own)]).status,
            403,
            "{path}"
        );
    }
    // two Host headers never get through
    let req = format!(
        "GET / HTTP/1.1\r\nHost: {host}\r\nHost: localhost:{}\r\nConnection: close\r\n\r\n",
        s.port
    );
    assert!(matches!(s.raw(&req).status, 400 | 403));
    // `/api/*` still needs the token exactly as before
    s.page("/api/version")
        .assert_forbidden("api__guard__no_token");
    s.get(&format!("/api/version?token={}", s.token), Some(&host), &[])
        .assert_forbidden("api__guard__no_token");
    let r = s.authed("/api/version", &[]);
    assert_eq!(r.status, 200);
    assert_eq!(
        r.json(),
        serde_json::json!({"version": env!("CARGO_PKG_VERSION")})
    );
    // the token appears in no static answer
    for path in ["/", "/app.js", "/nothing-here-ZZ9"] {
        let r = s.page(path);
        assert!(!r.body.contains(&s.token) && !format!("{:?}", r.headers).contains(&s.token));
    }
}

/// Every form the guide's path-safety table names, as sent on the wire (the request target is not normalised).
const TRAVERSAL_TARGETS: [&str; 44] = [
    "/..",
    "/../outside/secret.txt",
    "/../site-evil/secret.txt",
    "/../../outside/secret.txt",
    "/assets/../../outside/secret.txt",
    "/assets/../../../../../../../../etc/passwd",
    "/.%2e/outside/secret.txt",
    "/%2e%2e/outside/secret.txt",
    "/%2E%2E/outside/secret.txt",
    "/%2e./outside/secret.txt",
    "/.%2E/outside/secret.txt",
    "/assets/%2e%2e/%2e%2e/outside/secret.txt",
    "/..%2foutside/secret.txt",
    "/..%2Foutside%2Fsecret.txt",
    "/%2e%2e%2foutside/secret.txt",
    "/%2E%2E%2Foutside/secret.txt",
    "/..\\outside\\secret.txt",
    "/..%5coutside%5csecret.txt",
    "/..%5Coutside%5Csecret.txt",
    "/%2e%2e%5coutside%5csecret.txt",
    "/%252e%252e/outside/secret.txt",
    "/%252e%252e%252foutside/secret.txt",
    "/%25252e%25252e/outside/secret.txt",
    "/app.js%00",
    "/app.js%00.txt",
    "/%00",
    "/..%00/outside/secret.txt",
    "//etc/passwd",
    "/%2fetc/passwd",
    "/%2Fetc%2Fpasswd",
    "///etc/passwd",
    "//outside/secret.txt",
    "/./outside/secret.txt",
    "/./app.js",
    "/assets/./app.js",
    "/app.js.",
    "/app.js/.",
    "/app.js/..",
    "/assets/..",
    "/assets/../",
    "/..;/outside/secret.txt",
    "/link-out",
    "/link-dir-out/secret.txt",
    "/link-dir-out/",
];

#[test]
fn path_safety_table_no_form_returns_a_file_outside_the_directory() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    let s = Server::start_static(&fixture.site);
    let mut statuses = Vec::new();
    for target in TRAVERSAL_TARGETS {
        for with_token in [false, true] {
            let r = if with_token {
                s.authed(target, &[])
            } else {
                s.page(target)
            };
            statuses.push(format!("{target} -> {}", r.status));
            // `/app.js/..`-style forms that name something *inside* the folder may be 200 only when they are a normalised
            // inside file; none of the table's forms is allowed to, so every one is refused
            assert!(
                matches!(r.status, 400 | 403 | 404),
                "{target} (token {with_token}) answered {}: {}",
                r.status,
                r.body
            );
            assert!(
                !r.body.contains(OUTSIDE_MARKER),
                "{target}: marker in the body"
            );
            assert!(!r.body.contains("root:"), "{target}: /etc/passwd content");
            assert!(
                !format!("{:?}", r.headers).contains(OUTSIDE_MARKER),
                "{target}: marker in a header"
            );
            if r.status != 400 {
                r.assert_csp();
            }
        }
    }
    // a raw `..` before the HTTP layer sees it, written by hand
    for line in [
        "GET /../outside/secret.txt HTTP/1.1",
        "GET /\u{0}../outside/secret.txt HTTP/1.1",
        "GET http://127.0.0.1/../outside/secret.txt HTTP/1.1",
        "GET * HTTP/1.1",
        "GET  HTTP/1.1",
    ] {
        let r = s.raw(&format!(
            "{line}\r\nHost: {}\r\nConnection: close\r\n\r\n",
            s.host()
        ));
        assert_ne!(r.status, 200, "{line}");
        assert!(!r.body.contains(OUTSIDE_MARKER), "{line}");
    }
    // the server is alive and the folder is as it was
    assert_eq!(s.page("/").status, 200);
    assert_eq!(fixture.snapshot(), before, "serving wrote nothing");
    eprintln!("{}", statuses.join("\n"));
}

#[test]
fn a_long_or_deep_path_never_panics() {
    let fixture = Fixture::new();
    let s = Server::start_static(&fixture.site);
    for target in [
        format!("/{}", "a".repeat(60_000)),
        format!("/{}", "../".repeat(3_000)),
        format!("/{}app.js", "x/".repeat(3_000)),
        format!("/{}", "%2e%2e/".repeat(3_000)),
        format!("/{}", "/".repeat(3_000)),
        format!("/{}", "%".repeat(5_000)),
        format!("/app.js{}", "%".repeat(100)),
        "/%ff%fe".to_string(),
        "/%c0%ae%c0%ae/outside/secret.txt".to_string(),
        "/%e2%80%2e/x".to_string(),
        "/%".to_string(),
        "/%zz".to_string(),
        "/%2".to_string(),
    ] {
        let r = s.page(&target);
        assert!(
            matches!(r.status, 400 | 404 | 414 | 431),
            "{}: {}",
            &target[..target.len().min(40)],
            r.status
        );
        assert!(!r.body.contains(OUTSIDE_MARKER));
    }
    assert_eq!(s.page("/app.js").status, 200, "still alive");
}

#[test]
fn symlinks_that_leave_the_folder_are_404_and_ones_that_stay_inside_may_be_served() {
    let fixture = Fixture::new();
    let s = Server::start_static(&fixture.site);
    for path in [
        "/link-out",
        "/link-abs-out",
        "/link-prefix",
        "/link-dir-out/secret.txt",
        "/link-dir-out/index.html",
        "/link-dir-out/",
        "/link-dir-out",
        "/link-dangling",
        // a symlink to a dotfile inside the folder is still a dotfile
        "/link-to-dotfile",
    ] {
        let r = s.page(path);
        assert_eq!(r.status, 404, "{path}");
        assert!(
            !r.body.contains(OUTSIDE_MARKER) && !r.body.contains("DOTFILE"),
            "{path}"
        );
        r.assert_csp();
    }
    assert_eq!(s.page("/link-in").status, 200);
    assert_eq!(s.page("/link-dir-in/page.html").bytes, b"DOCS-PAGE");
}

#[test]
fn a_dotfile_or_a_file_under_a_dot_folder_is_404() {
    // Differs from Python: Starlette serves `.env` and `.web-stamp`. Here nothing starting with `.` is served.
    let fixture = Fixture::new();
    let s = Server::start_static(&fixture.site);
    for path in [
        "/.env",
        "/.web-stamp",
        "/docs/.hidden",
        "/.hiddendir/inner.txt",
        "/.hiddendir/",
        "/.%65nv",
        "/%2eenv",
        "/%2Eweb-stamp",
        "/.well-known/security.txt",
    ] {
        let r = s.page(path);
        assert_eq!(r.status, 404, "{path}");
        assert!(
            !r.body.contains("DOTFILE") && !r.body.contains("DOTDIR"),
            "{path}"
        );
        r.assert_csp();
    }
}

#[test]
fn a_fifo_is_404_and_does_not_block_the_server() {
    let fixture = Fixture::new();
    let s = Server::start_static(&fixture.site);
    let started = Instant::now();
    for _ in 0..3 {
        // `raw` reads for at most 5 s and the assertion below is far tighter: a blocked open would show here
        let r = s.page("/fifo");
        assert_eq!(r.status, 404);
        r.assert_csp();
    }
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(s.page("/").status, 200, "the server still answers");
}

#[test]
fn api_paths_are_never_served_as_files_even_when_a_file_of_that_name_exists() {
    let fixture = Fixture::new();
    let s = Server::start_static(&fixture.site);
    let file_markers = ["STATIC-API-FILE", "STATIC-API-EXTRA"];
    for path in [
        "/%61pi/usage/summary",
        "/%41PI/usage/summary",
        "/%61%70%69/usage/summary",
        "//api/usage/summary",
        "/api%2fusage%2fsummary",
        "/api%2Fusage%2Fsummary",
        "/api/usage/../usage/summary",
        "/api/./usage/summary",
        "/./api/usage/summary",
        "/assets/../api/usage/summary",
        "/%61pi/extra",
        "//api/extra",
        "/api%2fextra",
        "/api/usage/summary/",
        "/api/usage/sessions/",
        "/%61pi/usage/sessions",
        "/api",
        "/api/",
        "/api/extra",
        "/API/extra",
    ] {
        for with_token in [true, false] {
            let r = if with_token {
                s.authed(path, &[])
            } else {
                s.page(path)
            };
            assert!(
                matches!(r.status, 400 | 403 | 404 | 503) && r.status != 200,
                "{path} (token {with_token}): {}",
                r.status
            );
            for marker in file_markers {
                assert!(
                    !r.body.contains(marker),
                    "{path}: static file served as an API path"
                );
            }
            r.assert_csp();
        }
    }
    // an `/api/` path no route serves is today's answer, not a file: an empty 404
    let r = s.authed("/api/extra", &[]);
    assert_eq!((r.status, r.body.as_str()), (404, ""));
    // `/api/usage/summary` with the token is the real route (503: no store in this temp home), never the file
    let r = s.authed("/api/usage/summary", &[]);
    assert_eq!(r.status, 503);
    assert!(r.json()["error"].is_string());
}

#[test]
fn a_file_is_read_per_request_and_the_32_mib_cap_is_a_404() {
    let fixture = Fixture::new();
    let s = Server::start_static(&fixture.site);
    let changing = fixture.site.join("changing.txt");
    std::fs::write(&changing, b"first").unwrap();
    assert_eq!(s.page("/changing.txt").bytes, b"first");
    std::fs::write(&changing, b"second, longer").unwrap();
    assert_eq!(s.page("/changing.txt").bytes, b"second, longer");
    std::fs::remove_file(&changing).unwrap();
    assert_eq!(s.page("/changing.txt").status, 404);
    // exactly 32 MiB is served; one byte more is a 404 (the dashboard's biggest file is far smaller)
    for (name, len, status) in [
        ("big.bin", 32 * MIB, 200),
        ("bigger.bin", 32 * MIB + 1, 404),
    ] {
        let file = std::fs::File::create(fixture.site.join(name)).unwrap();
        file.set_len(len).unwrap();
        let r = s.page(&format!("/{name}"));
        assert_eq!(r.status, status, "{name}");
        r.assert_csp();
        if status == 200 {
            assert_eq!(r.bytes.len() as u64, len);
            assert_eq!(r.header("content-length"), vec![len.to_string()]);
        } else {
            assert!(r.bytes.len() < 100);
        }
    }
}

#[test]
fn serving_static_files_writes_nothing_and_changes_no_byte_of_the_folder() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    let mut s = Server::start_static(&fixture.site);
    for path in [
        "/",
        "/app.js",
        "/docs/",
        "/.env",
        "/link-out",
        "/fifo",
        "/nothing",
        "/%2e%2e/outside/secret.txt",
    ] {
        let _ = s.page(path);
        let _ = s.method("HEAD", path);
        let _ = s.method("POST", path);
    }
    let (status, rest, err) = s.stop_with_sigint();
    assert!(status.success(), "{status:?}");
    assert_eq!(
        (rest.as_str(), err.as_str()),
        ("", ""),
        "only the two startup lines"
    );
    assert_eq!(fixture.snapshot(), before);
}

#[test]
fn the_window_can_load_the_page_and_its_script_without_a_token_under_the_dashboard_csp() {
    // The T016 findings with the real server (plain HTTP, no browser): the printed URL carries the fragment, `/` and
    // `/app.js` load with no token, `/api/version` needs the header, and the CSP is the dashboard CSP on all of them.
    let fixture = Fixture::new();
    let mut s = Server::start_static(&fixture.site);
    let url = s.first_lines[0].clone();
    let (base, fragment) = url
        .split_once("/#")
        .expect("the printed URL carries a fragment");
    assert_eq!(base, format!("dashboard: http://127.0.0.1:{}", s.port));
    assert_eq!(fragment, s.token);
    assert_eq!(s.first_lines[1], "Ctrl-C to stop.");
    let page = s.page("/");
    assert_eq!(page.status, 200);
    assert_eq!(
        page.header("content-type"),
        vec!["text/html; charset=utf-8"]
    );
    let script = s.page("/app.js");
    assert_eq!(script.status, 200);
    assert_eq!(
        script.header("content-type"),
        vec!["text/javascript; charset=utf-8"]
    );
    let api = s.authed("/api/version", &[]);
    assert_eq!(api.status, 200);
    let no_token = s.page("/api/version");
    no_token.assert_forbidden("api__guard__no_token");
    for r in [&page, &script, &api, &no_token] {
        r.assert_csp();
    }
    let token = s.token.clone();
    let (status, rest, err) = s.stop_with_sigint();
    assert!(status.success(), "{status:?}");
    assert_eq!(
        (rest.as_str(), err.as_str()),
        ("", ""),
        "stdout and stderr hold only the two startup lines"
    );
    assert!(!rest.contains(&token) && !err.contains(&token));
}

#[test]
fn a_folder_with_no_index_html_makes_the_root_a_404() {
    let fixture = Fixture::new();
    std::fs::remove_file(fixture.site.join("index.html")).unwrap();
    let s = Server::start_static(&fixture.site);
    assert_eq!(s.page("/").status, 404);
    assert_eq!(s.page("/app.js").status, 200);
}
