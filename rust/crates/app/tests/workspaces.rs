//! T022: `GET /api/workspaces` and `GET /api/workspaces/unassigned` over the real `serve` binary, against the four Python
//! goldens (`api__workspaces__all.json`, `api__workspaces_unassigned__all.json` of `rust/parity/golden` and
//! `golden-rich`), and the guard, store-state, method and raw-path cases of the two routes. The binary runs under
//! `TZ=UTC` over a temp copy of a parity store; `HOME` and `EASY_PERSONAL_HOME` are temp folders, so the owner's real
//! data folder is never reached. The helpers are a trimmed copy of those of `tests/server.rs` (a test target cannot
//! import another).
//!
//! The rich goldens hold the token `<WORK>` where the saved store holds `/work/rich`, and the two spellings sort
//! differently, so the comparison reads the token as `/work/rich` and compares `repos` in byte order on both sides (the
//! same rule as `crates/core/tests/workspaces_parity.rs`, which also proves the core's own repo order).

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// Same string as `CSP` in `src/easy_personal/surfaces/api.py`.
const CSP: &str = "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'";

const ROUTES: [&str; 2] = ["/api/workspaces", "/api/workspaces/unassigned"];

fn parity(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity")
        .join(rel)
}

fn read_json(rel: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(parity(rel)).unwrap()).unwrap()
}

struct Server {
    child: Child,
    _stdout: BufReader<ChildStdout>,
    port: u16,
    token: String,
    home: PathBuf,
}

impl Server {
    /// A server whose data folder holds a copy of `fixture` as `store.sqlite`, run under `TZ=UTC` like the goldens.
    fn with_store(fixture: &str) -> Self {
        Self::launch(|home| {
            std::fs::copy(parity(fixture), home.join("store.sqlite")).unwrap();
        })
    }

    fn launch(setup: impl FnOnce(&Path)) -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let home = std::env::temp_dir().join(format!(
            "easy-personal-rs-t022-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&home).unwrap();
        setup(&home);
        let mut child = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
            .args(["serve", "--no-ingest"])
            .env("HOME", &home)
            .env("EASY_PERSONAL_HOME", &home)
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

/// Every `repos` list in byte order, and with `token` also `<WORK>` read as the folder name the saved store holds.
fn normalised(value: &Value, token: bool) -> Value {
    match value {
        Value::String(s) if token => Value::String(s.replace("<WORK>", "/work/rich")),
        Value::Array(items) => Value::Array(items.iter().map(|v| normalised(v, token)).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| {
                    let mut v = normalised(v, token);
                    if k == "repos" {
                        v.as_array_mut()
                            .unwrap()
                            .sort_by(|a, b| a.as_str().cmp(&b.as_str()));
                    }
                    (k.clone(), v)
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

/// The manifest's entries for the two routes: `(request path, golden file)`.
fn entries(dir: &str) -> Vec<(String, String)> {
    read_json(&format!("{dir}/manifest.json"))["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| {
            let file = e["file"].as_str().unwrap();
            file.starts_with("api__workspaces__")
                || file.starts_with("api__workspaces_unassigned__")
        })
        .map(|e| {
            assert_eq!(
                (e["method"].as_str(), e["status"].as_u64()),
                (Some("GET"), Some(200))
            );
            assert_eq!(e["params"], json!({}), "the routes take no parameter");
            (
                e["route"].as_str().unwrap().to_string(),
                e["file"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn compare_oracle(dir: &str, store: &str) -> usize {
    let list = entries(dir);
    let server = Server::with_store(store);
    for (route, file) in &list {
        let r = server.authed(route);
        r.assert_json(200);
        let golden = read_json(&format!("{dir}/{file}"));
        assert_eq!(
            normalised(&r.json(), true),
            normalised(&golden, true),
            "{dir}/{file} over {route}"
        );
        // the repos come back in byte order from the binary itself, in either spelling
        assert_eq!(
            r.json(),
            normalised(&r.json(), false),
            "{dir}/{file}: repos not in byte order"
        );
    }
    list.len()
}

#[test]
fn the_two_routes_equal_the_four_goldens_over_the_binary() {
    let total = compare_oracle("golden", "store-v11.sqlite")
        + compare_oracle("golden-rich", "store-rich/store-v11.sqlite");
    assert_eq!(total, 4, "two oracles x (list, unassigned)");
    // the rich store has real content: the list is not the default's single orphan
    let rich = Server::with_store("store-rich/store-v11.sqlite");
    let body = rich.authed("/api/workspaces").json();
    assert_eq!(body["workspaces"].as_array().unwrap().len(), 3);
    assert_eq!(body["workspaces"][1]["repos"][1], "/work/rich/kit/alpha");
}

#[test]
fn the_query_string_is_ignored() {
    let s = Server::with_store("store-rich/store-v11.sqlite");
    for route in ROUTES {
        let plain = s.authed(route);
        for query in [
            "workspace=nope",
            "account=nope&since=garbage",
            "by=x&workspace=",
        ] {
            let r = s.authed(&format!("{route}?{query}"));
            r.assert_json(200);
            assert_eq!(r.body, plain.body, "{route}?{query}");
        }
    }
}

// ---- methods ----

/// T048 serves `POST /api/workspaces` and `PATCH`/`DELETE` on `/api/workspaces/{id}` (and so on `/api/workspaces/unassigned`,
/// which Python's routing hands to `{id}`: a 404 after the body checks). Without a content type they answer 415 and open nothing.
#[test]
fn other_methods_are_405_with_the_csp_and_head_is_answered_like_get() {
    let s = Server::with_store("store-rich/store-v11.sqlite");
    let before = std::fs::read(s.home.join("store.sqlite")).unwrap();
    let served = [
        ("/api/workspaces", "POST"),
        ("/api/workspaces/unassigned", "PATCH"),
        ("/api/workspaces/unassigned", "DELETE"),
    ];
    for route in ROUTES {
        for method in ["POST", "PATCH", "PUT", "DELETE"] {
            let r = s.send(method, route, &[]);
            if served.contains(&(route, method)) {
                r.assert_json(415);
                continue;
            }
            assert_eq!(r.status, 405, "{method} {route}");
            r.assert_csp();
            assert!(r.body.is_empty(), "{method} {route}: {:?}", r.body);
        }
        let head = s.send("HEAD", route, &[]);
        assert_eq!(head.status, 200, "{route}");
        assert!(head.body.is_empty());
        head.assert_csp();
    }
    // `/api/workspaces/{id}` is a write route: a GET is a 405, a write without a content type is a 415
    assert_eq!(s.send("GET", "/api/workspaces/2", &[]).status, 405);
    for method in ["PATCH", "DELETE"] {
        assert_eq!(
            s.send(method, "/api/workspaces/2", &[]).status,
            415,
            "{method}"
        );
    }
    assert_eq!(
        std::fs::read(s.home.join("store.sqlite")).unwrap(),
        before,
        "no write reached the store"
    );
}

// ---- the guard ----

#[test]
fn the_guard_covers_both_routes() {
    let s = Server::with_store("store-v11.sqlite");
    let host = s.host();
    for route in ROUTES {
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
        assert_eq!(s.authed(route).status, 200, "{route}");
        // a write without the token is a 403 before it is a 405
        s.raw(&format!(
            "POST {route} HTTP/1.1\r\nHost: {host}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ))
        .assert_forbidden("api__guard__no_token");
    }
}

#[test]
fn encoded_or_doubled_paths_are_never_served() {
    let s = Server::with_store("store-rich/store-v11.sqlite");
    for path in [
        "/%61pi/workspaces",
        "//api/workspaces",
        "/api%2fworkspaces",
        "/api%2Fworkspaces",
        "/api/./workspaces",
        "/api/workspaces/",
        "/api/workspaces/../workspaces",
        "/api/%77orkspaces",
        "/%61pi/workspaces/unassigned",
        "//api/workspaces/unassigned",
        "/api%2fworkspaces%2funassigned",
        "/api/workspaces%2funassigned",
        "/api/workspaces/unassigned/",
        "/api/workspaces/%75nassigned",
        "/api/workspaces/./unassigned",
    ] {
        for with_token in [true, false] {
            let r = if with_token {
                s.authed(path)
            } else {
                s.get(path, Some(&s.host()), &[])
            };
            // `/api/workspaces/<escaped name>` is a write route's `{id}` since T048: a GET there is a 405
            let write_route = path.starts_with("/api/workspaces/") && !path.contains("//");
            assert!(
                r.status == 404 || r.status == 403 || (r.status == 405 && write_route),
                "{path}: {}",
                r.status
            );
            r.assert_csp();
            assert!(
                !r.body.contains("workspaces") && !r.body.contains("orphan"),
                "{path}: {}",
                r.body
            );
        }
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

#[test]
fn a_newer_unreadable_or_non_file_store_is_503_with_a_fixed_text_and_no_path_or_row_text() {
    let newer = Server::launch(|home| {
        let store = home.join("store.sqlite");
        std::fs::copy(parity("store-v11.sqlite"), &store).unwrap();
        rusqlite::Connection::open(&store)
            .unwrap()
            .execute_batch("PRAGMA user_version = 12")
            .unwrap();
    });
    let garbage = Server::launch(|home| {
        std::fs::write(
            home.join("store.sqlite"),
            "not a database, PRIVATE-ROW-TEXT",
        )
        .unwrap()
    });
    let directory = Server::launch(|home| std::fs::create_dir(home.join("store.sqlite")).unwrap());
    // a database of the right version that lacks the tables: the SQL error must not reach the body
    let no_tables = Server::launch(|home| {
        rusqlite::Connection::open(home.join("store.sqlite"))
            .unwrap()
            .execute_batch(
                "CREATE TABLE note (x TEXT); INSERT INTO note VALUES ('PRIVATE-ROW-TEXT')",
            )
            .unwrap()
    });
    let fixed = "the store cannot be read right now; try again in a moment";
    for route in ROUTES {
        let r = newer.authed(route);
        r.assert_json(503);
        assert_eq!(
            r.json(),
            json!({"error": "the store is at version 12, newer than the 11 this build knows; update easy-personal-rs (nothing was changed)"}),
            "{route}"
        );
        for server in [&garbage, &directory, &no_tables] {
            let r = server.authed(route);
            r.assert_json(503);
            assert_eq!(r.json(), json!({ "error": fixed }), "{route}");
            let home = server.home.to_string_lossy().into_owned();
            for banned in [
                home.as_str(),
                "PRIVATE-ROW-TEXT",
                "SELECT",
                "workspace",
                "no such table",
            ] {
                assert!(!r.body.contains(banned), "{route}: {}", r.body);
            }
        }
    }
}

#[test]
fn serving_leaves_the_store_bytes_and_the_folder_as_they_were() {
    for store in ["store-v11.sqlite", "store-rich/store-v11.sqlite"] {
        let s = Server::with_store(store);
        let before = std::fs::read(s.home.join("store.sqlite")).unwrap();
        for route in ROUTES {
            s.authed(route).assert_json(200);
        }
        assert_eq!(
            std::fs::read(s.home.join("store.sqlite")).unwrap(),
            before,
            "{store}"
        );
        let mut names: Vec<String> = std::fs::read_dir(&s.home)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, ["store.sqlite"], "{store}: no -wal, -shm or journal");
    }
}
