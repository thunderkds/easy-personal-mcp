//! T048: the store-only write routes over the real `serve` binary. The oracle (`rust/parity/golden-writes/cases.json`,
//! made by `gen_writes.py` from Python's own handlers) is replayed case by case: one `serve` per case over a temp copy of
//! `store-rich` with the case's folders and setup SQL, the same request bytes, and the answer (status, body) and **every
//! table** compared with what Python left; a case that Python leaves unchanged must leave the store file byte for byte as
//! it was. Then what has no Python answer: the guard on every write route, a plan that would need the `claude` CLI
//! (409, nothing written), the body cap, hostile bodies and ids, a stale, newer or missing store, and the promise that
//! start-up and every GET leave the store file alone.
//!
//! The binary runs with an environment built from nothing: `HOME`, `EASY_PERSONAL_HOME` and `EASY_PERSONAL_AGENT_ROOT`
//! are folders under one temp folder, `PATH` has no `claude`, so neither the owner's data nor any agent folder, keychain
//! or `claude` is reached.

use rusqlite::Connection;
use rusqlite::types::ValueRef;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// Same string as `CSP` in `src/easy_personal/surfaces/api.py`.
const CSP: &str = "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'";
const NEEDS_CLAUDE: &str = "this change must update an agent account's Claude Code entries, which needs the claude CLI; this build cannot run it yet, so nothing was changed";
const CONNECTORS: [&str; 2] = [
    "INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES ('slack-acme', 'acme-slack', 0, 'slack', 2)",
    "INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES ('gmail-acme', 'acme-gmail', 0, 'gmail', 2)",
];
const REGISTERED: &str = "INSERT INTO account_registrations (account_id, alias, connector_id) VALUES (2, 'acme-slack', 'slack-acme')";

fn parity(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity")
        .join(rel)
}

// ---- a data folder and a server over it ----

struct Home {
    dir: PathBuf,
}

impl Home {
    fn empty() -> Home {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t048-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let dir = std::fs::canonicalize(dir).unwrap();
        for sub in ["agent-root/.claude", "agent-root/.claude-work", "nowhere"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        Home { dir }
    }

    /// A copy of `store-rich` whose account folders are this folder's, plus `setup` statements (`@W@` is this folder).
    fn rich(setup: &[&str]) -> Home {
        let home = Home::empty();
        std::fs::copy(parity("store-rich/store-v11.sqlite"), home.store()).unwrap();
        let work = home.work();
        let conn = Connection::open(home.store()).unwrap();
        // the real store is in WAL mode from Python's first open; the committed copy is not
        conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))
            .unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        conn.execute(
            "UPDATE agent_accounts SET folder = replace(folder, '/work/rich', ?1), config_dir_env = replace(config_dir_env, '/work/rich', ?1)",
            [&work],
        )
        .unwrap();
        for sql in setup {
            conn.execute(&sql.replace("@W@", &work), []).unwrap();
        }
        home
    }

    fn work(&self) -> String {
        self.dir.to_str().unwrap().to_string()
    }

    fn store(&self) -> PathBuf {
        self.dir.join("store.sqlite")
    }

    fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.store()).unwrap()
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

struct Server {
    child: Child,
    _stdout: BufReader<ChildStdout>,
    port: u16,
    token: String,
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

    fn error(&self) -> String {
        self.json()["error"].as_str().unwrap().to_string()
    }
}

impl Server {
    fn start(home: &Home) -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
            .args(["serve", "--no-ingest"])
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", home.dir.join("nowhere"))
            .env("EASY_PERSONAL_HOME", &home.dir)
            .env("EASY_PERSONAL_AGENT_ROOT", home.dir.join("agent-root"))
            .env("XDG_CONFIG_HOME", home.dir.join("xdg"))
            .env("TZ", "UTC")
            .current_dir(&home.dir)
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
        }
    }

    fn raw(&self, request: &[u8]) -> Response {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        // a server that answers before it has read a huge body may reset the connection: the answer is still read
        let _ = stream.write_all(request);
        let mut bytes = Vec::new();
        let _ = stream.read_to_end(&mut bytes);
        Response::parse(&bytes)
    }

    /// One request with the good Host and token; `ctype` `None` sends no content type.
    fn send(&self, method: &str, path: &str, ctype: Option<&str>, body: &[u8]) -> Response {
        self.send_with(method, path, ctype, body, &[], true)
    }

    fn send_with(
        &self,
        method: &str,
        path: &str,
        ctype: Option<&str>,
        body: &[u8],
        extra: &[(&str, &str)],
        token: bool,
    ) -> Response {
        let mut head = format!("{method} {path} HTTP/1.1\r\n");
        if !extra.iter().any(|(k, _)| k.eq_ignore_ascii_case("host")) {
            head += &format!("Host: 127.0.0.1:{}\r\n", self.port);
        }
        if token {
            head += &format!("x-dashboard-token: {}\r\n", self.token);
        }
        if let Some(ctype) = ctype {
            head += &format!("content-type: {ctype}\r\n");
        }
        for (k, v) in extra {
            head += &format!("{k}: {v}\r\n");
        }
        head += &format!(
            "Content-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let mut request = head.into_bytes();
        request.extend_from_slice(body);
        self.raw(&request)
    }

    fn json(&self, method: &str, path: &str, body: &Value) -> Response {
        self.send(
            method,
            path,
            Some("application/json"),
            body.to_string().as_bytes(),
        )
    }

    fn stop(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ---- the whole database as JSON ----

/// Every table: its rows as JSON, sorted by their text, with the work folder as `<WORK>`.
fn dump(path: &Path, work: &str) -> BTreeMap<String, Vec<Value>> {
    let conn = Connection::open(path).unwrap();
    let names: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let mut out = BTreeMap::new();
    for name in names {
        let mut stmt = conn.prepare(&format!("SELECT * FROM \"{name}\"")).unwrap();
        let width = stmt.column_count();
        let mut rows: Vec<Value> = stmt
            .query_map([], |r| {
                Ok(Value::Array(
                    (0..width)
                        .map(|i| match r.get_ref(i).unwrap() {
                            ValueRef::Null => Value::Null,
                            ValueRef::Integer(n) => json!(n),
                            ValueRef::Real(f) => json!(f),
                            ValueRef::Text(t) => {
                                json!(String::from_utf8_lossy(t).replace(work, "<WORK>"))
                            }
                            ValueRef::Blob(b) => {
                                json!(b.iter().map(|x| format!("{x:02x}")).collect::<String>())
                            }
                        })
                        .collect(),
                ))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        rows.sort_by_key(Value::to_string);
        out.insert(name, rows);
    }
    out
}

fn scrubbed(value: &Value, work: &str) -> Value {
    match value {
        Value::String(s) => Value::String(s.replace(work, "<WORK>")),
        Value::Array(items) => Value::Array(items.iter().map(|v| scrubbed(v, work)).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), scrubbed(v, work)))
                .collect(),
        ),
        other => other.clone(),
    }
}

// ---- the oracle ----

fn unhex(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

/// `bytes` with every `from` replaced by `to` (the body is not always UTF-8).
fn replace_bytes(bytes: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    let (mut out, mut i) = (Vec::new(), 0);
    while i < bytes.len() {
        if bytes[i..].starts_with(from) {
            out.extend_from_slice(to);
            i += from.len();
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

fn build_tree(home: &Home, tree: &Value) {
    let work = home.work();
    for d in tree["dirs"].as_array().unwrap() {
        std::fs::create_dir_all(home.dir.join(d.as_str().unwrap())).unwrap();
    }
    for (rel, content) in tree["files"].as_object().unwrap() {
        let file = home.dir.join(rel);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, content.as_str().unwrap()).unwrap();
    }
    for (rel, target) in tree["symlinks"].as_object().unwrap() {
        std::os::unix::fs::symlink(
            target.as_str().unwrap().replace("@W@", &work),
            home.dir.join(rel),
        )
        .unwrap();
    }
}

/// Replays one case; returns what differs from Python (empty when it does not).
fn replay(case: &Value) -> Vec<String> {
    let name = case["name"].as_str().unwrap();
    let setup: Vec<&str> = case["setup"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    let home = Home::empty();
    build_tree(&home, &case["tree"]);
    std::fs::copy(parity("store-rich/store-v11.sqlite"), home.store()).unwrap();
    let work = home.work();
    {
        let conn = Connection::open(home.store()).unwrap();
        // the real store is in WAL mode from Python's first open; the committed copy is not
        conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))
            .unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        conn.execute(
            "UPDATE agent_accounts SET folder = replace(folder, '/work/rich', ?1), config_dir_env = replace(config_dir_env, '/work/rich', ?1)",
            [&work],
        )
        .unwrap();
        for sql in setup {
            conn.execute(&sql.replace("@W@", &work), []).unwrap();
        }
    }
    let before = dump(&home.store(), &work);
    let bytes = home.bytes();
    let server = Server::start(&home);
    let mut problems = Vec::new();
    for (request, answer) in case["requests"]
        .as_array()
        .unwrap()
        .iter()
        .zip(case["answers"].as_array().unwrap())
    {
        let body = replace_bytes(
            &unhex(request["body_hex"].as_str().unwrap()),
            b"@W@",
            work.as_bytes(),
        );
        let path = request["path"].as_str().unwrap().replace("@W@", &work);
        let got = server.send(
            request["method"].as_str().unwrap(),
            &path,
            request["type"].as_str(),
            &body,
        );
        let label = format!("{name}: {} {path}", request["method"].as_str().unwrap());
        if got.status as u64 != answer["status"].as_u64().unwrap() {
            problems.push(format!(
                "{label}: status {} want {} ({})",
                got.status, answer["status"], got.body
            ));
            continue;
        }
        if request["compare"] == "full" && !answer["json"].is_null() {
            if got.header("content-type") != ["application/json"]
                || got.header("content-security-policy") != [CSP]
            {
                problems.push(format!("{label}: headers {:?}", got.headers));
            }
            let want = &answer["json"];
            if scrubbed(&got.json(), &work) != *want {
                problems.push(format!("{label}: body {} want {want}", got.body));
            }
        }
    }
    server.stop();
    let after = dump(&home.store(), &work);
    let want_tables: Vec<&str> = case["tables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect();
    if after.keys().map(String::as_str).collect::<Vec<_>>() != want_tables {
        problems.push(format!(
            "{name}: tables {:?}",
            after.keys().collect::<Vec<_>>()
        ));
    }
    for (table, rows) in &after {
        let expected: Vec<Value> = match case["changed"].get(table) {
            Some(golden) => {
                let mut rows: Vec<Value> = golden.as_array().unwrap().clone();
                rows.sort_by_key(Value::to_string);
                rows
            }
            None => before[table].clone(),
        };
        if *rows != expected {
            problems.push(format!(
                "{name}: table {table} is {rows:?} want {expected:?}"
            ));
        }
    }
    if case["changed"].as_object().unwrap().is_empty() && home.bytes() != bytes {
        problems.push(format!(
            "{name}: a case Python leaves unchanged changed the store file"
        ));
    }
    problems
}

#[test]
fn every_write_equals_what_python_answered_and_left_in_the_store() {
    let golden: Value =
        serde_json::from_str(&std::fs::read_to_string(parity("golden-writes/cases.json")).unwrap())
            .unwrap();
    let cases = golden["cases"].as_array().unwrap();
    // the committed file is the generator's output: a smaller one (a dropped case) must fail here, not pass
    assert_eq!(cases.len(), 262, "cases");
    let mut problems = Vec::new();
    let mut requests = 0;
    for case in cases {
        let asked = case["requests"].as_array().unwrap().len();
        assert_eq!(
            asked,
            case["answers"].as_array().unwrap().len(),
            "{}: requests and answers",
            case["name"]
        );
        requests += asked;
        problems.extend(replay(case));
    }
    assert_eq!(requests, 355, "requests");
    println!(
        "writes oracle: {} cases, {requests} requests, {} different",
        cases.len(),
        problems.len()
    );
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

// ---- what Python cannot answer ----

/// The seven write routes with a body that would succeed.
fn write_routes(home: &Home) -> Vec<(&'static str, String, Value)> {
    std::fs::create_dir_all(home.dir.join("cfg/projects")).unwrap();
    vec![
        ("POST", "/api/workspaces".into(), json!({"name": "guarded"})),
        (
            "PATCH",
            "/api/workspaces/2".into(),
            json!({"description": "x"}),
        ),
        (
            "DELETE",
            "/api/workspaces/3".into(),
            json!({"confirm": true}),
        ),
        (
            "POST",
            "/api/accounts".into(),
            json!({"path": home.dir.join("cfg")}),
        ),
        (
            "POST",
            "/api/accounts/2/confirm".into(),
            json!({"label": "Guarded"}),
        ),
        ("POST", "/api/accounts/2/ignore".into(), json!({})),
        (
            "PUT",
            "/api/accounts/1/workspaces".into(),
            json!({"workspaces": []}),
        ),
    ]
}

#[test]
fn a_write_without_the_guard_is_403_and_changes_nothing_on_every_route() {
    let home = Home::rich(&[]);
    let server = Server::start(&home);
    for (method, path, body) in write_routes(&home) {
        let body = body.to_string();
        let bytes = home.bytes();
        let cases: [(&str, Response); 5] = [
            (
                "no token",
                server.send_with(
                    method,
                    &path,
                    Some("application/json"),
                    body.as_bytes(),
                    &[],
                    false,
                ),
            ),
            (
                "wrong token",
                server.send_with(
                    method,
                    &path,
                    Some("application/json"),
                    body.as_bytes(),
                    &[("x-dashboard-token", "nope")],
                    false,
                ),
            ),
            (
                "bad host",
                server.send_with(
                    method,
                    &path,
                    Some("application/json"),
                    body.as_bytes(),
                    &[("Host", "evil.test")],
                    true,
                ),
            ),
            (
                "cross origin",
                server.send_with(
                    method,
                    &path,
                    Some("application/json"),
                    body.as_bytes(),
                    &[("Origin", "http://evil.test")],
                    true,
                ),
            ),
            (
                "repeated token",
                server.send_with(
                    method,
                    &path,
                    Some("application/json"),
                    body.as_bytes(),
                    &[("x-dashboard-token", &server.token)],
                    true,
                ),
            ),
        ];
        for (what, got) in cases {
            assert_eq!(got.status, 403, "{method} {path} {what}: {}", got.body);
            assert_eq!(got.json(), json!({"error": "forbidden"}));
            assert_eq!(got.header("content-security-policy"), [CSP]);
        }
        assert_eq!(
            home.bytes(),
            bytes,
            "{method} {path}: a refused request changed the store"
        );
        // the same request with the guard satisfied is not refused (a 5xx or 403 here would make the above prove nothing)
        let ok = server.send(method, &path, Some("application/json"), body.as_bytes());
        assert!(
            ok.status == 200 || ok.status == 201,
            "{method} {path}: {} {}",
            ok.status,
            ok.body
        );
    }
}

#[test]
fn a_plan_that_needs_the_claude_cli_is_refused_with_409_and_writes_nothing_until_t050() {
    let home = Home::rich(&[CONNECTORS[0], CONNECTORS[1], REGISTERED]);
    let server = Server::start(&home);
    let bytes = home.bytes();
    // rename: the plan is shown (Python's, as if `claude` is not on PATH)
    let plan = server.json("PATCH", "/api/workspaces/2", &json!({"name": "firm"}));
    assert_eq!(plan.status, 200);
    assert_eq!(plan.json()["pending"], true);
    assert!(
        plan.body.contains("Claude work: remove acme-slack"),
        "{}",
        plan.body
    );
    // confirmed: refused, nothing changed
    for (method, path, body) in [
        (
            "PATCH",
            "/api/workspaces/2",
            json!({"name": "firm", "confirm": true}),
        ),
        ("DELETE", "/api/workspaces/2", json!({"confirm": true})),
    ] {
        let got = server.json(method, path, &body);
        assert_eq!(
            (got.status, got.error()),
            (409, NEEDS_CLAUDE.to_string()),
            "{method} {path}"
        );
        for secret in ["slack-acme", "acme-slack", "Claude work"] {
            assert!(!got.body.contains(secret), "{}", got.body);
        }
        assert_eq!(home.bytes(), bytes, "{method} {path}");
    }
    let list = server.send("GET", "/api/workspaces", None, b"");
    assert!(list.body.contains("\"acme\""), "{}", list.body);
    // a plan with nothing for `claude` to do is applied (the connectors of an unpicked workspace)
    drop(server);
    let home = Home::rich(&[
        CONNECTORS[0],
        CONNECTORS[1],
        "DELETE FROM account_workspaces",
    ]);
    let server = Server::start(&home);
    let done = server.json(
        "PATCH",
        "/api/workspaces/2",
        &json!({"name": "firm", "confirm": true}),
    );
    assert_eq!(
        (done.status, done.json()["status"].clone()),
        (200, json!(0))
    );
    let conn = Connection::open(home.store()).unwrap();
    let alias: String = conn
        .query_row(
            "SELECT alias FROM connectors WHERE id = 'slack-acme'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(alias, "firm-slack");
}

#[test]
fn a_body_over_one_mib_is_413_and_a_big_bad_one_is_an_ordinary_400() {
    let home = Home::rich(&[]);
    let server = Server::start(&home);
    let bytes = home.bytes();
    let big = format!(
        "{{\"name\":\"x\",\"description\":\"{}\"}}",
        "d".repeat(1 << 20)
    );
    let got = server.send(
        "POST",
        "/api/workspaces",
        Some("application/json"),
        big.as_bytes(),
    );
    assert_eq!(
        (got.status, got.error()),
        (413, "request body too large".to_string())
    );
    assert_eq!(got.header("content-security-policy"), [CSP]);
    let under = format!(
        "{{\"name\":\"x\",\"description\":\"{}\"}}",
        "d".repeat(600_000)
    );
    let got = server.send(
        "POST",
        "/api/workspaces",
        Some("application/json"),
        under.as_bytes(),
    );
    assert_eq!(
        (got.status, got.error()),
        (
            400,
            "description must be text of at most 500 characters".to_string()
        )
    );
    // the content type is judged before the body is read
    let got = server.send(
        "POST",
        "/api/workspaces",
        Some("text/plain"),
        big.as_bytes(),
    );
    assert_eq!(got.status, 415);
    assert_eq!(home.bytes(), bytes);
}

#[test]
fn hostile_bodies_and_ids_are_clean_refusals_and_change_nothing() {
    let home = Home::rich(&[]);
    let server = Server::start(&home);
    let bytes = home.bytes();
    let deep = format!("{}{}", "[".repeat(200), "]".repeat(200));
    let utf16: Vec<u8> = "{\"name\":\"x\"}"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let bad_json: [(&str, &[u8]); 6] = [
        ("NaN", b"{\"name\":\"x\",\"icon\":NaN}"),
        ("Infinity", b"{\"name\":\"x\",\"icon\":Infinity}"),
        ("past a double", b"{\"name\":\"x\",\"icon\":1e999}"),
        ("deep nesting", deep.as_bytes()),
        ("utf-16", &utf16),
        ("lone surrogate", b"{\"name\":\"\\ud800\"}"),
    ];
    for route in ["/api/workspaces", "/api/accounts"] {
        for (what, body) in bad_json {
            let got = server.send("POST", route, Some("application/json"), body);
            assert_eq!(
                (got.status, got.error()),
                (400, "body must be JSON".to_string()),
                "{route} {what}"
            );
        }
    }
    let not_found = [
        ("PATCH", "/api/workspaces/%D9%A3", "unknown workspace"),
        (
            "PATCH",
            "/api/workspaces/1234567890123456789",
            "unknown workspace",
        ),
        (
            "DELETE",
            "/api/workspaces/99999999999999999999",
            "unknown workspace",
        ),
        ("POST", "/api/accounts/%D9%A3/ignore", "no such account"),
        (
            "POST",
            "/api/accounts/1234567890123456789/ignore",
            "no such account",
        ),
    ];
    for (method, path, text) in not_found {
        let got = server.send(method, path, Some("application/json"), b"{}");
        assert_eq!(
            (got.status, got.error()),
            (404, text.to_string()),
            "{method} {path}"
        );
    }
    // a label that starts with NUL passes Python's length check and fails the store's: refused here, not a crash
    let got = server.json(
        "POST",
        "/api/accounts/2/confirm",
        &json!({"label": "\u{0}abc"}),
    );
    assert_eq!(
        (got.status, got.error()),
        (400, "label must be 1 to 60 characters".to_string())
    );
    let got = server.json("POST", "/api/accounts", &json!({"path": "a\u{0}b"}));
    assert_eq!(
        (got.status, got.error()),
        (
            400,
            "not an agent folder: it must be an existing directory".to_string()
        )
    );
    // no answer names what was sent
    let got = server.json(
        "POST",
        "/api/workspaces",
        &json!({"name": "LEAK-CANARY-123", "repos": ["rel-LEAK-CANARY-456"]}),
    );
    assert_eq!(got.status, 400);
    assert!(!got.body.contains("CANARY"), "{}", got.body);
    assert_eq!(home.bytes(), bytes);
    // an id spelt with an escape is read as Starlette reads it (the path is decoded before the route sees it)
    let got = server.json("POST", "/api/accounts/%32/ignore", &json!({}));
    assert_eq!(got.status, 200, "{}", got.body);
}

#[test]
fn a_stale_newer_or_missing_store_is_503_and_the_file_is_left_alone() {
    let body = json!({"name": "fresh"});
    // newer
    let home = Home::rich(&[]);
    Connection::open(home.store())
        .unwrap()
        .execute_batch("PRAGMA user_version = 12")
        .unwrap();
    let (server, bytes) = (Server::start(&home), home.bytes());
    for (method, path, b) in write_routes(&home) {
        let got = server.json(method, &path, &b);
        assert_eq!(got.status, 503, "{method} {path}: {}", got.body);
        assert!(got.error().contains("newer"), "{}", got.body);
    }
    assert_eq!(home.bytes(), bytes);
    // older
    let home = Home::rich(&[]);
    Connection::open(home.store())
        .unwrap()
        .execute_batch("PRAGMA user_version = 10")
        .unwrap();
    let (server, bytes) = (Server::start(&home), home.bytes());
    let got = server.json("POST", "/api/workspaces", &body);
    assert_eq!(
        (got.status, got.error()),
        (
            503,
            "store is schema v10: run the Python app once to migrate it".to_string()
        )
    );
    assert_eq!(home.bytes(), bytes);
    // missing: never created, and a bad request is refused before the store is looked at
    let home = Home::empty();
    let server = Server::start(&home);
    let got = server.json("POST", "/api/workspaces", &body);
    assert_eq!(got.status, 503);
    assert!(got.error().starts_with("no store yet"), "{}", got.body);
    let got = server.send(
        "POST",
        "/api/workspaces",
        Some("application/json"),
        b"{nope",
    );
    assert_eq!(
        (got.status, got.error()),
        (400, "body must be JSON".to_string())
    );
    assert!(!home.store().exists(), "a write created a store");
    // empty file and a file that is not a store
    std::fs::write(home.store(), b"").unwrap();
    let got = server.json("POST", "/api/accounts/1/ignore", &json!({}));
    assert_eq!(got.status, 503);
    assert_eq!(home.bytes(), b"");
    std::fs::write(
        home.store(),
        b"not a database, not a database, not a database",
    )
    .unwrap();
    let got = server.json("POST", "/api/workspaces", &body);
    assert_eq!(got.status, 503, "{}", got.body);
    assert!(
        !got.body.contains(home.dir.to_str().unwrap()),
        "{}",
        got.body
    );
    assert_eq!(
        home.bytes(),
        b"not a database, not a database, not a database"
    );
}

#[test]
fn start_up_and_every_get_leave_the_store_file_bytes_and_mtime_alone() {
    let home = Home::rich(&[]);
    let (bytes, mtime) = (
        home.bytes(),
        std::fs::metadata(home.store()).unwrap().modified().unwrap(),
    );
    std::thread::sleep(Duration::from_millis(50));
    let server = Server::start(&home);
    for path in [
        "/api/version",
        "/api/workspaces",
        "/api/workspaces/unassigned",
        "/api/accounts",
        "/api/accounts?by=provider",
        "/api/sync",
        "/api/usage/summary",
        "/api/warnings",
        "/api/connectors",
        "/api/first-run",
        "/api/reports",
    ] {
        let got = server.send("GET", path, None, b"");
        assert_eq!(got.status, 200, "{path}: {}", got.body);
    }
    assert_eq!(home.bytes(), bytes);
    assert_eq!(
        std::fs::metadata(home.store()).unwrap().modified().unwrap(),
        mtime
    );
    // a write is the first thing that opens a write handle, and it changes only what it says
    let got = server.json("POST", "/api/workspaces", &json!({"name": "after-gets"}));
    assert_eq!(got.status, 201);
    assert_ne!(home.bytes(), bytes);
}

#[test]
fn two_writes_at_once_both_finish_and_the_store_is_intact() {
    let home = Home::rich(&[]);
    let server = std::sync::Arc::new(Server::start(&home));
    let threads: Vec<_> = (0..6)
        .map(|n| {
            let server = std::sync::Arc::clone(&server);
            std::thread::spawn(move || {
                server.json(
                    "POST",
                    "/api/workspaces",
                    &json!({"name": format!("par-{n}")}),
                )
            })
        })
        .collect();
    for thread in threads {
        let got = thread.join().unwrap();
        assert_eq!(got.status, 201, "{}", got.body);
    }
    let names: Vec<Value> = dump(&home.store(), &home.work())["workspaces"].clone();
    assert_eq!(names.len(), 3 + 6);
    let conn = Connection::open(home.store()).unwrap();
    let check: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(check, "ok");
}

/// T054: a link chain that doubles at every hop (30 deep: 2^30 expansions without a cache of resolved links) as a repo
/// path and as an agent folder, and a 1 MiB PATCH body of 95,000 distinct keys, are each answered in under a second. T048
/// walked the chain while it held the write lock and scanned the keys in quadratic time.
#[test]
fn a_hostile_link_chain_and_a_body_of_95000_keys_are_answered_in_under_a_second() {
    let home = Home::rich(&[]);
    let chain = home.dir.join("chain");
    std::fs::create_dir_all(chain.join("cfg/projects")).unwrap();
    for i in 0..30 {
        std::os::unix::fs::symlink(
            format!("hop{}/hop{}", i + 1, i + 1),
            chain.join(format!("hop{i}")),
        )
        .unwrap();
    }
    std::os::unix::fs::symlink(".", chain.join("hop30")).unwrap();
    let work = chain.display();
    let server = Server::start(&home);
    let timed = |method: &str, path: &str, body: &[u8]| {
        let started = std::time::Instant::now();
        let got = server.send(method, path, Some("application/json"), body);
        (got, started.elapsed())
    };
    let (got, took) = timed(
        "POST",
        "/api/workspaces",
        json!({"name": "chained", "repos": [format!("{work}/hop0/x")]})
            .to_string()
            .as_bytes(),
    );
    assert_eq!(got.status, 201, "{}", got.body);
    assert_eq!(
        got.json()["workspace"]["repos"],
        json!([format!("{work}/x")])
    );
    assert!(took < Duration::from_secs(1), "repo chain took {took:?}");
    let (got, took) = timed(
        "POST",
        "/api/accounts",
        json!({"path": format!("{work}/hop0/cfg")})
            .to_string()
            .as_bytes(),
    );
    assert_eq!(got.status, 201, "{}", got.body);
    assert_eq!(got.json()["folder"], json!(format!("{work}/cfg")));
    assert!(took < Duration::from_secs(1), "account chain took {took:?}");
    let keys: Vec<String> = (0..95_000).map(|n| format!("\"k{n:05}\":1")).collect();
    let big = format!("{{{}}}", keys.join(","));
    assert!(big.len() < 1 << 20, "{} bytes", big.len());
    let (got, took) = timed("PATCH", "/api/workspaces/2", big.as_bytes());
    assert_eq!(
        (got.status, got.error()),
        (400, "unknown field".to_string())
    );
    assert!(took < Duration::from_secs(1), "95,000 keys took {took:?}");
}

#[test]
fn a_writer_that_holds_the_store_for_longer_than_five_seconds_gives_a_fixed_busy_answer() {
    let home = Home::rich(&[]);
    let server = Server::start(&home);
    let holder = Connection::open(home.store()).unwrap();
    holder.execute_batch("BEGIN IMMEDIATE").unwrap();
    let began = std::time::Instant::now();
    let got = server.json("POST", "/api/workspaces", &json!({"name": "waits"}));
    assert!(
        began.elapsed() >= Duration::from_secs(4),
        "{:?}",
        began.elapsed()
    );
    assert_eq!(got.status, 503);
    assert_eq!(
        got.error(),
        "the store is busy (another process is writing); try again in a moment"
    );
    assert!(!got.body.to_lowercase().contains("locked"), "{}", got.body);
    holder.execute_batch("ROLLBACK").unwrap();
    let got = server.json("POST", "/api/workspaces", &json!({"name": "waits"}));
    assert_eq!(got.status, 201);
}
