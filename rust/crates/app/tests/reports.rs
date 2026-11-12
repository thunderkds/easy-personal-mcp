//! T040: `GET /api/reports`, `GET /api/reports/{id}` and `GET /api/first-run` over the real `serve` binary, against the
//! Python goldens (`api__reports__all`, `api__reports_one__{found,missing,scoped}`, `api__first_run__all` of
//! `rust/parity/golden` and `golden-rich`), and the guard, id, store-state, method and raw-path cases of the three routes.
//! The binary runs under `TZ=UTC` over a temp copy of a parity store; `HOME`, `EASY_PERSONAL_HOME` and `XDG_CONFIG_HOME`
//! are temp folders, so neither the owner's data folder nor the real unit folder is reached. The helpers are a trimmed copy
//! of those of `tests/workspaces.rs` (a test target cannot import another).
//!
//! The binary reads the system clock, and a report's "now" side depends on it: the goldens equal the `before` side and the
//! fixed fields here, and the `now` side is compared in `crates/core/tests/reports_parity.rs` under the frozen clock.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// Same string as `CSP` in `src/easy_personal/surfaces/api.py`.
const CSP: &str = "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'";

const ROUTES: [&str; 3] = ["/api/reports", "/api/reports/1", "/api/first-run"];

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
    /// A server whose data folder holds a copy of `fixture` as `store.sqlite`, run under `TZ=UTC` like the goldens. The
    /// rich store's confirmed accounts name `/work/rich/...` folders (Python's golden run had them): the copy's rows are
    /// pointed at temp folders that exist, as the golden run's were.
    fn with_store(fixture: &str) -> Self {
        Self::launch(|home| {
            let store = home.join("store.sqlite");
            std::fs::copy(parity(fixture), &store).unwrap();
            let conn = rusqlite::Connection::open(&store).unwrap();
            for id in 1..=2 {
                let folder = home.join(format!("account-{id}"));
                std::fs::create_dir_all(&folder).unwrap();
                conn.execute(
                    "UPDATE agent_accounts SET folder = ?1 WHERE id = ?2",
                    rusqlite::params![folder.to_str().unwrap(), id],
                )
                .unwrap();
            }
        })
    }

    fn launch(setup: impl FnOnce(&Path)) -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let home = std::env::temp_dir().join(format!(
            "easy-personal-rs-t040-{}-{}",
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

/// `(request path, golden file, status)` of the slice's entries in one manifest; the id of a `reports_one` entry is the
/// one `gen_golden.py` asked for.
fn entries(dir: &str) -> Vec<(String, String, u16)> {
    read_json(&format!("{dir}/manifest.json"))["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| {
            let file = e["file"].as_str().unwrap();
            let path = match (dir, file) {
                (_, "api__reports__all.json") => "/api/reports",
                (_, "api__first_run__all.json") => "/api/first-run",
                (_, "api__reports_one__found.json") => "/api/reports/1",
                (_, "api__reports_one__scoped.json") => "/api/reports/2",
                ("golden", "api__reports_one__missing.json") => "/api/reports/1",
                ("golden-rich", "api__reports_one__missing.json") => "/api/reports/99",
                _ => return None,
            };
            assert_eq!(e["method"], "GET");
            assert_eq!(e["params"], json!({}));
            Some((
                path.to_string(),
                file.to_string(),
                e["status"].as_u64().unwrap() as u16,
            ))
        })
        .collect()
}

/// The report body without its "now" side: that side is measured against the system clock here.
fn without_now(mut body: Value) -> Value {
    if let Some(rules) = body.get_mut("rules").and_then(Value::as_array_mut) {
        for rule in rules {
            rule.as_object_mut().unwrap().remove("now");
        }
    }
    if let Some(object) = body.as_object_mut() {
        object.remove("now_as_of_utc");
    }
    body
}

fn compare_oracle(dir: &str, store: &str) -> usize {
    let list = entries(dir);
    let server = Server::with_store(store);
    for (path, file, status) in &list {
        let r = server.authed(path);
        r.assert_json(*status);
        let golden = read_json(&format!("{dir}/{file}"));
        let got = if path.starts_with("/api/reports/") {
            (without_now(r.json()), without_now(golden))
        } else {
            (r.json(), golden)
        };
        assert_eq!(got.0, got.1, "{dir}/{file} over {path}");
    }
    list.len()
}

#[test]
fn the_three_routes_equal_the_goldens_over_the_binary() {
    let total = compare_oracle("golden", "store-v11.sqlite")
        + compare_oracle("golden-rich", "store-rich/store-v11.sqlite");
    assert_eq!(
        total, 8,
        "3 entries over the plain store, 5 over the rich one"
    );
}

#[test]
fn the_now_side_is_measured_on_the_request_for_the_saved_scope() {
    let s = Server::with_store("store-rich/store-v11.sqlite");
    // the binary reads the system clock, so no value of the now side is asserted: only that it was measured at all (a
    // state per rule, a time of its own that is not the saved one) for the report's saved scope (report 2 is the acme one)
    for id in [1, 2] {
        let body = s.authed(&format!("/api/reports/{id}")).json();
        let now_as_of = body["now_as_of_utc"].as_str().unwrap();
        assert!(now_as_of.ends_with("+00:00"), "{now_as_of}");
        assert_ne!(
            now_as_of, body["as_of_utc"],
            "measured on the request, not copied"
        );
        let rules = body["rules"].as_array().unwrap();
        assert!(!rules.is_empty());
        for rule in rules {
            let state = rule["now"]["state"].as_str().unwrap();
            assert!(
                ["fired", "clear", "abstained", "not_in_use"].contains(&state),
                "{rule}"
            );
        }
    }
}

#[test]
fn a_report_whose_fixes_or_snapshot_lack_what_save_writes_is_the_same_fixed_500() {
    let s = Server::launch(|home| {
        let store = home.join("store.sqlite");
        std::fs::copy(parity("store-rich/store-v11.sqlite"), &store).unwrap();
        rusqlite::Connection::open(&store)
            .unwrap()
            .execute_batch(
                "UPDATE investigate_reports SET fixes_json = 'PRIVATE-FIXES-TEXT' WHERE id = 1; \
                 UPDATE investigate_reports SET snapshot_json = json_remove(snapshot_json, '$.as_of_utc') WHERE id = 2",
            )
            .unwrap();
    });
    for id in [1, 2] {
        let r = s.authed(&format!("/api/reports/{id}"));
        r.assert_json(500);
        assert_eq!(
            r.json(),
            json!({"error": "the saved report cannot be read"}),
            "{id}"
        );
        assert!(!r.body.contains("PRIVATE-FIXES-TEXT"));
    }
}

#[test]
fn first_run_reads_the_unit_folder_named_by_xdg_config_home_on_each_request() {
    let s = Server::with_store("store-rich/store-v11.sqlite");
    let done = |s: &Server| s.authed("/api/first-run").json()["steps"][1]["done"].clone();
    assert_eq!(done(&s), false);
    let units = s.home.join("xdg").join("systemd").join("user");
    std::fs::create_dir_all(&units).unwrap();
    std::fs::write(units.join("easy-personal-ingest.service"), "").unwrap();
    assert_eq!(done(&s), false, "the service file alone is not the timer");
    std::fs::write(units.join("easy-personal-ingest.timer"), "").unwrap();
    assert_eq!(done(&s), true, "read per request, not at start-up");
    std::fs::remove_file(units.join("easy-personal-ingest.timer")).unwrap();
    assert_eq!(done(&s), false);
}

// ---- the id ----

#[test]
fn an_id_that_is_not_plain_digits_or_has_no_report_is_the_same_404() {
    let s = Server::with_store("store-rich/store-v11.sqlite");
    for id in [
        "99",
        "0",
        "abc",
        "1x",
        "x1",
        "-1",
        "+1",
        "1.0",
        "1e0",
        "%201",
        "1%20",
        "%D9%A1",              // ARABIC-INDIC DIGIT ONE: isdecimal, not ASCII
        "999999999999999999",  // 18 digits, no such report
        "1000000000000000000", // 19 digits: Python answers 404 without a lookup
        "99999999999999999999999",
        "%FF",
    ] {
        let r = s.authed(&format!("/api/reports/{id}"));
        r.assert_json(404);
        assert_eq!(r.json(), json!({"error": "no such report"}), "{id}");
    }
    // leading zeros and an encoded digit are the id 1, as Python's `int(raw)` and the decoded path
    for id in ["001", "%31", "0000000000000000001"] {
        let r = s.authed(&format!("/api/reports/{id}"));
        let want = if id.len() < 19 { 200 } else { 404 };
        assert_eq!(r.status, want, "{id}: {}", r.body);
        if want == 200 {
            assert_eq!(r.json()["id"], 1, "{id}");
        }
    }
}

#[test]
fn the_query_string_is_ignored() {
    let s = Server::with_store("store-rich/store-v11.sqlite");
    for route in ["/api/reports", "/api/first-run"] {
        let plain = s.authed(route);
        for query in ["workspace=nope", "account=nope&since=garbage", "id=2"] {
            let r = s.authed(&format!("{route}?{query}"));
            r.assert_json(200);
            assert_eq!(r.body, plain.body, "{route}?{query}");
        }
    }
}

// ---- methods ----

#[test]
fn writes_are_405_with_the_csp_and_head_is_answered_like_get() {
    let s = Server::with_store("store-rich/store-v11.sqlite");
    let before = std::fs::read(s.home.join("store.sqlite")).unwrap();
    for route in ROUTES {
        for method in ["POST", "PATCH", "PUT", "DELETE"] {
            let r = s.send(method, route, &[]);
            assert_eq!(r.status, 405, "{method} {route}");
            r.assert_csp();
            assert!(r.body.is_empty(), "{method} {route}: {:?}", r.body);
        }
        let head = s.send("HEAD", route, &[]);
        assert_eq!(head.status, 200, "{route}");
        assert!(head.body.is_empty());
        head.assert_csp();
    }
    assert_eq!(
        std::fs::read(s.home.join("store.sqlite")).unwrap(),
        before,
        "no write reached the store"
    );
}

// ---- the guard ----

#[test]
fn the_guard_covers_all_three_routes() {
    let s = Server::with_store("store-v11.sqlite");
    let host = s.host();
    for route in ROUTES
        .into_iter()
        .chain(["/api/reports/99", "/api/reports/abc"])
    {
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
        // a write without the token is a 403 before it is a 405
        s.raw(&format!(
            "POST {route} HTTP/1.1\r\nHost: {host}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ))
        .assert_forbidden("api__guard__no_token");
    }
    assert_eq!(s.authed("/api/reports/abc").status, 404);
}

#[test]
fn encoded_or_doubled_paths_never_reach_a_handler_unguarded() {
    let s = Server::with_store("store-rich/store-v11.sqlite");
    for path in [
        "/%61pi/reports",
        "//api/reports",
        "/api%2freports",
        "/api%2Freports/1",
        "/api/./reports",
        "/api/reports/",
        "/api/reports/1/",
        "/api/reports/../reports",
        "/api/%72eports",
        "/%61pi/first-run",
        "//api/first-run",
        "/api%2ffirst-run",
        "/api/first-run/",
        "/api/first%2drun",
        "/api/reports%2f1",
    ] {
        for with_token in [true, false] {
            let r = if with_token {
                s.authed(path)
            } else {
                s.get(path, Some(&s.host()), &[])
            };
            assert!(r.status == 404 || r.status == 403, "{path}: {}", r.status);
            r.assert_csp();
            assert!(
                !r.body.contains("Invented") && !r.body.contains("steps"),
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
            for banned in [home.as_str(), "PRIVATE-ROW-TEXT", "no such table", "SQLITE"] {
                assert!(!r.body.contains(banned), "{route}: {}", r.body);
            }
        }
    }
}

#[test]
fn a_report_whose_snapshot_cannot_be_read_is_a_fixed_500_that_shows_none_of_it() {
    let s = Server::launch(|home| {
        let store = home.join("store.sqlite");
        std::fs::copy(parity("store-rich/store-v11.sqlite"), &store).unwrap();
        rusqlite::Connection::open(&store)
            .unwrap()
            .execute_batch(
                "UPDATE investigate_reports SET snapshot_json = 'PRIVATE-SNAPSHOT-TEXT' WHERE id = 1",
            )
            .unwrap();
    });
    let r = s.authed("/api/reports/1");
    r.assert_json(500);
    assert_eq!(
        r.json(),
        json!({"error": "the saved report cannot be read"})
    );
    // the other report and the list still answer; the list counts the unreadable one as 0 fired
    s.authed("/api/reports/2").assert_json(200);
    let list = s.authed("/api/reports");
    list.assert_json(200);
    assert_eq!(list.json()["reports"][1]["fired_count"], 0);
    assert!(!list.body.contains("PRIVATE-SNAPSHOT-TEXT"));
}
