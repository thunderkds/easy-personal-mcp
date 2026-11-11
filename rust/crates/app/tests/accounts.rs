//! T041: `GET /api/accounts` and `GET /api/sync` over the real `serve` binary: the goldens, the hand-made oracle of
//! `rust/parity/golden-accounts/` (the sync plans, which do not depend on the clock), the guard, the methods, the errors,
//! and the read-only promise (a new agent folder adds no account, the store file is never changed).
//! The binary runs under `TZ=UTC` over a temp copy of a parity store; `HOME`, `EASY_PERSONAL_HOME`,
//! `EASY_PERSONAL_AGENT_ROOT` and `XDG_CONFIG_HOME` are temp folders, so neither the owner's data folder nor any agent
//! folder is reached. The binary reads the system clock, which is far past the fixtures: the token counts of a status
//! line are 0 there and are compared in `crates/core/tests/accounts_parity.rs` under the frozen clock. The harness is a
//! trimmed copy of that of `tests/reports.rs` (a test target cannot import another).

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// Same string as `CSP` in `src/easy_personal/surfaces/api.py`.
const CSP: &str = "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'";

const ROUTES: [&str; 3] = ["/api/accounts", "/api/accounts?by=provider", "/api/sync"];

fn parity(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity")
        .join(rel)
}

fn read_json(rel: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(parity(rel)).unwrap()).unwrap()
}

/// Points the folder columns of the store copy at the temp data folder (a writable handle, before the server starts).
fn replace_in_folders(store: &Path, from: &str, to: &str) {
    let conn = rusqlite::Connection::open(store).unwrap();
    conn.execute(
        "UPDATE agent_accounts SET folder = replace(folder, ?1, ?2), config_dir_env = replace(config_dir_env, ?1, ?2)",
        [from, to],
    )
    .unwrap();
}

/// `value` with every `from` in a string replaced by `to`.
fn replaced(value: &Value, from: &str, to: &str) -> Value {
    match value {
        Value::String(s) => Value::String(s.replace(from, to)),
        Value::Array(items) => Value::Array(items.iter().map(|v| replaced(v, from, to)).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), replaced(v, from, to)))
                .collect(),
        ),
        other => other.clone(),
    }
}

struct Server {
    child: Child,
    _stdout: BufReader<ChildStdout>,
    port: u16,
    token: String,
    home: PathBuf,
}

impl Server {
    /// A server whose data folder holds a copy of `fixture` as `store.sqlite`, run under `TZ=UTC` like the goldens. The
    /// rich store's accounts name `/work/rich/...` folders (Python's golden run had them): the copy's rows are pointed at
    /// temp folders under the agent root that exist, as the golden run's were.
    fn with_store(fixture: &str) -> Self {
        Self::launch(|home| {
            let store = home.join("store.sqlite");
            std::fs::copy(parity(fixture), &store).unwrap();
            for folder in [".claude", ".claude-work"] {
                std::fs::create_dir_all(home.join("agent-root").join(folder)).unwrap();
            }
            replace_in_folders(&store, "/work/rich", home.to_str().unwrap());
        })
    }

    /// The hand-made store of `golden-accounts/` with its folders and files built under the data folder.
    fn with_hand_made() -> Self {
        Self::launch(|home| {
            let dir = parity("golden-accounts");
            let manifest: Value =
                serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json")).unwrap())
                    .unwrap();
            for d in manifest["tree"]["dirs"].as_array().unwrap() {
                std::fs::create_dir_all(home.join(d.as_str().unwrap())).unwrap();
            }
            for (rel, content) in manifest["tree"]["files"].as_object().unwrap() {
                let file = home.join(rel);
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                std::fs::write(file, content.as_str().unwrap()).unwrap();
            }
            let store = home.join("store.sqlite");
            std::fs::copy(dir.join("store.sqlite"), &store).unwrap();
            replace_in_folders(&store, "@W@", home.to_str().unwrap());
        })
    }

    fn launch(setup: impl FnOnce(&Path)) -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let home = std::env::temp_dir().join(format!(
            "easy-personal-rs-t041-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&home).unwrap();
        setup(&home);
        let mut child = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
            .arg("serve")
            .env("HOME", &home)
            .env("EASY_PERSONAL_HOME", &home)
            .env("XDG_CONFIG_HOME", home.join("xdg"))
            .env("EASY_PERSONAL_AGENT_ROOT", home.join("agent-root"))
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
        let _ = std::fs::remove_dir_all(&self.home);
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

// ---- the goldens ----

fn query(params: &Value) -> String {
    let pairs: Vec<String> = params
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| format!("{k}={}", v.as_str().unwrap().replace(' ', "%20")))
        .collect();
    if pairs.is_empty() {
        String::new()
    } else {
        format!("?{}", pairs.join("&"))
    }
}

/// The accounts and sync-plan entries of a default manifest: `(request path, golden file, status)`.
fn entries(dir: &str) -> Vec<(String, String, u16)> {
    read_json(&format!("{dir}/manifest.json"))["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| {
            let f = e["file"].as_str().unwrap();
            f.starts_with("api__accounts__") || f.starts_with("api__sync_plan__")
        })
        .map(|e| {
            (
                format!("{}{}", e["route"].as_str().unwrap(), query(&e["params"])),
                e["file"].as_str().unwrap().to_string(),
                e["status"].as_u64().unwrap() as u16,
            )
        })
        .collect()
}

/// A status line has no token counts here: the system clock is far past the fixtures' week.
fn without_tokens(mut value: Value) -> Value {
    match &mut value {
        Value::Object(map) => {
            for (key, v) in map.iter_mut() {
                if key == "tokens_today" || key == "tokens_7d" {
                    *v = json!(0);
                } else {
                    *v = without_tokens(v.take());
                }
            }
        }
        Value::Array(items) => {
            for v in items.iter_mut() {
                *v = without_tokens(v.take());
            }
        }
        _ => {}
    }
    value
}

#[test]
fn the_goldens_are_equal_over_the_binary() {
    let (mut equal, mut total) = (0, 0);
    for (dir, store) in [
        ("golden", "store-v11.sqlite"),
        ("golden-rich", "store-rich/store-v11.sqlite"),
    ] {
        let s = Server::with_store(store);
        let home = s.home.to_str().unwrap().to_string();
        for (path, file, status) in entries(dir) {
            let r = s.authed(&path);
            r.assert_json(status);
            let want = without_tokens(replaced(
                &read_json(&format!("{dir}/{file}")),
                "<WORK>",
                &home,
            ));
            assert_eq!(r.json(), want, "{dir}/{file} over {path}");
            equal += 1;
            total += 1;
        }
    }
    eprintln!("goldens over the binary: {equal} equal / {total} total");
    assert_eq!(total, 8);
}

#[test]
fn the_hand_made_oracle_is_equal_over_the_binary() {
    let s = Server::with_hand_made();
    let home = s.home.to_str().unwrap().to_string();
    let manifest = read_json("golden-accounts/manifest.json");
    let info = &manifest["zones"]["UTC"];
    let (mut equal, mut total) = (0, 0);
    for entry in info["entries"].as_array().unwrap() {
        let file = entry["file"].as_str().unwrap();
        let route = entry["route"].as_str().unwrap();
        let path = format!("{route}{}", query(&entry["params"]));
        let r = s.authed(&path);
        r.assert_json(entry["status"].as_u64().unwrap() as u16);
        let dir = info["dir"].as_str().unwrap();
        let want = replaced(
            &read_json(&format!("golden-accounts/{dir}/{file}")),
            "<WORK>",
            &home,
        );
        // the system clock is far past the oracle's: the token counts and the "stale" marks of a status line differ
        assert_eq!(
            r.json(),
            if route == "/api/accounts" {
                continue_with_clock(r.json(), want)
            } else {
                want
            },
            "{file} over {path}"
        );
        equal += 1;
        total += 1;
    }
    eprintln!("golden-accounts UTC over the binary: {equal} equal / {total} total");
    assert_eq!(total, 19);
}

/// What an accounts answer holds that depends on the clock: tokens, and the limits' `stale` marks. They are taken from
/// the answer itself (the core test compares them under the frozen clock); everything else must still equal the oracle.
fn continue_with_clock(got: Value, want: Value) -> Value {
    fn mask(mut value: Value) -> Value {
        match &mut value {
            Value::Object(map) => {
                for (key, v) in map.iter_mut() {
                    if key == "tokens_today" || key == "tokens_7d" || key == "stale" {
                        *v = json!(null);
                    } else {
                        *v = mask(v.take());
                    }
                }
            }
            Value::Array(items) => {
                for v in items.iter_mut() {
                    *v = mask(v.take());
                }
            }
            _ => {}
        }
        value
    }
    assert_eq!(mask(got.clone()), mask(want), "everything but the clock");
    got
}

#[test]
fn nothing_the_files_hold_beyond_the_three_keys_and_the_limits_ever_appears() {
    let s = Server::with_hand_made();
    for route in ROUTES {
        let body = s.authed(route).body;
        for planted in ["must-not-appear", "secret_token", "accessToken", "\"x\""] {
            assert!(!body.contains(planted), "{route} shows {planted}");
        }
    }
}

// ---- errors, parameters ----

#[test]
fn errors_are_pythons_with_fixed_texts() {
    let s = Server::with_store("store-rich/store-v11.sqlite");
    for (path, status, text) in [
        (
            "/api/accounts?by=bad",
            400,
            "by must be account or provider",
        ),
        ("/api/accounts?by=", 400, "by must be account or provider"),
        (
            "/api/accounts?by=Account",
            400,
            "by must be account or provider",
        ),
        (
            "/api/accounts?by=provider&by=x",
            400,
            "by must be account or provider",
        ),
        (
            "/api/sync?account=abc",
            400,
            "account must be an account id",
        ),
        ("/api/sync?account=-1", 400, "account must be an account id"),
        (
            "/api/sync?account=1.5",
            400,
            "account must be an account id",
        ),
        (
            "/api/sync?account=true",
            400,
            "account must be an account id",
        ),
        (
            "/api/sync?account=%202",
            400,
            "account must be an account id",
        ),
        (
            "/api/sync?account=%C2%B2",
            400,
            "account must be an account id",
        ),
        (
            "/api/sync?account=%D9%A3",
            400,
            "account must be an account id",
        ),
        (
            "/api/sync?account=1%3BDROP",
            400,
            "account must be an account id",
        ),
        ("/api/sync?account=99", 404, "no such account"),
        ("/api/sync?account=0", 404, "no such account"),
        (
            "/api/sync?account=99999999999999999999",
            404,
            "no such account",
        ),
    ] {
        let r = s.authed(path);
        r.assert_json(status);
        assert_eq!(r.json(), json!({ "error": text }), "{path}");
    }
    // empty = absent; a decoded digit and leading zeros select the account; the last value wins
    let all = s.authed("/api/sync").body;
    assert_eq!(s.authed("/api/sync?account=").body, all);
    assert_eq!(
        s.authed("/api/sync?account=%32").json(),
        s.authed("/api/sync?account=02").json()
    );
    assert_eq!(
        s.authed("/api/accounts?by=provider&by=account").json()["by"],
        "account"
    );
    assert_eq!(
        s.authed("/api/accounts").json()["by"],
        "account",
        "`by` defaults to account"
    );
    assert_eq!(
        s.authed("/api/accounts?workspace=nope&since=garbage")
            .status,
        200,
        "other keys are ignored"
    );
}

// ---- read-only ----

#[test]
fn a_new_agent_folder_shows_after_ingest_not_at_page_load() {
    let s = Server::with_store("store-rich/store-v11.sqlite");
    let before = std::fs::read(s.home.join("store.sqlite")).unwrap();
    std::fs::create_dir_all(s.home.join("agent-root/.claude-extra/projects")).unwrap();
    std::fs::create_dir_all(s.home.join("agent-root/.codex-extra")).unwrap();
    for route in ROUTES {
        let r = s.authed(route);
        r.assert_json(200);
        assert!(!r.body.contains("extra"), "{route}: {}", r.body);
    }
    assert_eq!(
        std::fs::read(s.home.join("store.sqlite")).unwrap(),
        before,
        "no account was inserted and the store file is unchanged"
    );
    assert!(!s.home.join("store.sqlite-wal").exists());
}

#[test]
fn writes_are_405_and_the_account_writes_are_not_served_here() {
    let s = Server::with_store("store-rich/store-v11.sqlite");
    let before = std::fs::read(s.home.join("store.sqlite")).unwrap();
    for route in ["/api/accounts", "/api/sync"] {
        for method in ["POST", "PATCH", "PUT", "DELETE"] {
            let r = s.send(method, route, &[]);
            assert_eq!(r.status, 405, "{method} {route}");
            r.assert_csp();
            assert!(r.body.is_empty(), "{method} {route}: {:?}", r.body);
        }
        let head = s.send("HEAD", route, &[]);
        assert_eq!(head.status, 200, "{route}");
        assert!(head.body.is_empty());
    }
    // T048 owns these: no route here, so 404 (or 405), never a change
    for (method, route) in [
        ("PUT", "/api/accounts/2/workspaces"),
        ("POST", "/api/accounts/2/confirm"),
        ("POST", "/api/accounts/2/ignore"),
    ] {
        let r = s.send(method, route, &[]);
        assert!(
            r.status == 404 || r.status == 405,
            "{method} {route}: {}",
            r.status
        );
    }
    assert_eq!(std::fs::read(s.home.join("store.sqlite")).unwrap(), before);
}

// ---- the guard ----

#[test]
fn the_guard_covers_both_routes() {
    let s = Server::with_store("store-v11.sqlite");
    let host = s.host();
    for route in ROUTES
        .into_iter()
        .chain(["/api/sync?account=2", "/api/sync?account=abc"])
    {
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
        s.send("GET", route, &[("Origin", "http://evil.example")])
            .assert_forbidden("api__guard__bad_origin");
        s.get(route, None, &[("x-dashboard-token", &s.token)])
            .assert_forbidden("api__guard__bad_host");
        s.raw(&format!(
            "POST {route} HTTP/1.1\r\nHost: {host}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ))
        .assert_forbidden("api__guard__no_token");
    }
}

// ---- the store states ----

#[test]
fn a_missing_store_is_503_with_the_fixed_text_and_the_server_creates_nothing() {
    let s = Server::launch(|_| {});
    for route in ROUTES {
        let r = s.authed(route);
        r.assert_json(503);
        assert_eq!(
            r.json(),
            json!({"error": "no store yet: nothing has been ingested into the data folder"}),
            "{route}"
        );
    }
    assert_eq!(std::fs::read_dir(&s.home).unwrap().count(), 0);
}
