//! T050 AC5 / AC6 / AC11 over the real `serve` binary: what the Python oracle (`connector_parity.rs`) cannot show.
//! The guard and the body cap on every new route, a locked keychain, the keychain rollback when the store refuses the row,
//! two adds at once, a hostile account folder (it reaches `claude` only as `CLAUDE_CONFIG_DIR`), and `claude`'s stderr
//! (never shown). The environment is built from nothing: the fake keychain file, the oracle's fake `claude`, temp folders,
//! a session bus address that leads nowhere.

use rusqlite::Connection;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

const CSP: &str = "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'";
const SECRET: &str = "xoxb-PLANTED-t050-writes";
const NO_BACKEND: &str = "no usable OS keychain (Secret Service); unlock or install one, e.g. gnome-keyring. Secrets are never stored in plaintext";
const ACME: [&str; 2] = [
    "INSERT INTO connectors VALUES ('slack-acme', 'acme-slack', 0, 'slack', 2)",
    "INSERT INTO account_registrations VALUES (2, 'acme-slack', 'slack-acme')",
];

fn parity(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity")
        .join(rel)
}

struct World {
    dir: PathBuf,
}

impl World {
    /// A copy of `store-rich` (accounts in this folder) plus `setup`, both agent folders, the fake `claude` on PATH and
    /// an empty fake keychain.
    fn new(setup: &[&str]) -> World {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t050-w-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        for sub in [
            "home",
            "nowhere",
            "bin",
            "state",
            "agent-root/.claude",
            "agent-root/.claude-work/projects",
        ] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        let dir = std::fs::canonicalize(dir).unwrap();
        std::fs::copy(
            parity("golden-connectors/fake-claude.sh"),
            dir.join("bin/claude"),
        )
        .unwrap();
        std::fs::copy(
            parity("store-rich/store-v11.sqlite"),
            dir.join("home/store.sqlite"),
        )
        .unwrap();
        let world = World { dir };
        let conn = world.conn();
        conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))
            .unwrap();
        let work = world.dir.to_str().unwrap();
        conn.execute(
            "UPDATE agent_accounts SET folder = replace(folder, '/work/rich', ?1), config_dir_env = replace(config_dir_env, '/work/rich', ?1)",
            [work],
        )
        .unwrap();
        for sql in setup {
            conn.execute(sql, []).unwrap();
        }
        std::fs::write(world.keychain(), "{}").unwrap();
        world
    }

    fn conn(&self) -> Connection {
        let conn = Connection::open(self.dir.join("home/store.sqlite")).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        conn
    }

    fn keychain(&self) -> PathBuf {
        self.dir.join("keychain.json")
    }

    fn log(&self) -> String {
        std::fs::read_to_string(self.dir.join("claude.log")).unwrap_or_default()
    }

    fn store_bytes(&self) -> Vec<u8> {
        std::fs::read(self.dir.join("home/store.sqlite")).unwrap()
    }

    fn serve(&self, extra: &[(&str, &str)]) -> Server {
        self.serve_with(extra, true)
    }

    /// `fake_keychain` false: the real Secret Service backend, which finds no session bus (a dead address).
    fn serve_with(&self, extra: &[(&str, &str)], fake_keychain: bool) -> Server {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"));
        cmd.args(["serve", "--no-ingest"])
            .env_clear()
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.dir.join("bin").display()),
            )
            .env("HOME", self.dir.join("nowhere"))
            .env("EASY_PERSONAL_HOME", self.dir.join("home"))
            .env("EASY_PERSONAL_AGENT_ROOT", self.dir.join("agent-root"))
            .env("EASY_PERSONAL_TEST_KEYCHAIN", self.keychain())
            .env(
                "DBUS_SESSION_BUS_ADDRESS",
                "unix:path=/nonexistent/t050-no-bus",
            )
            .env("FAKE_CLAUDE_LOG", self.dir.join("claude.log"))
            .env("FAKE_CLAUDE_STATE", self.dir.join("state"))
            .env("TZ", "UTC")
            .current_dir(&self.dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in extra {
            cmd.env(k, v);
        }
        if !fake_keychain {
            cmd.env_remove("EASY_PERSONAL_TEST_KEYCHAIN");
        }
        Server::start(cmd.spawn().unwrap())
    }
}

impl Drop for World {
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

struct Answer {
    status: u16,
    head: String,
    body: String,
}

impl Answer {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or_else(|_| panic!("not JSON: {:?}", self.body))
    }
}

impl Server {
    fn start(mut child: Child) -> Server {
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        let rest = line
            .trim_end()
            .strip_prefix("dashboard: http://127.0.0.1:")
            .unwrap();
        let (port, token) = rest.split_once("/#").unwrap();
        Server {
            port: port.parse().unwrap(),
            token: token.into(),
            child,
            _stdout: stdout,
        }
    }

    fn raw(&self, method: &str, path: &str, headers: &[(&str, &str)], body: &[u8]) -> Answer {
        let mut head = format!("{method} {path} HTTP/1.1\r\n");
        for (k, v) in headers {
            head += &format!("{k}: {v}\r\n");
        }
        head += &format!(
            "Content-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(60)))
            .unwrap();
        let mut request = head.into_bytes();
        request.extend_from_slice(body);
        let _ = stream.write_all(&request);
        let mut bytes = Vec::new();
        let _ = stream.read_to_end(&mut bytes);
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
        Answer {
            status: head
                .split(' ')
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            head: head.to_string(),
            body: body.to_string(),
        }
    }

    fn json(&self, method: &str, path: &str, body: &Value) -> Answer {
        let host = format!("127.0.0.1:{}", self.port);
        let headers = [
            ("Host", host.as_str()),
            ("x-dashboard-token", &self.token),
            ("content-type", "application/json"),
        ];
        self.raw(method, path, &headers, body.to_string().as_bytes())
    }

    /// Stops the server; its stderr (logs) for the caller to scan.
    fn stop(mut self) -> String {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let mut err = String::new();
        if let Some(mut e) = self.child.stderr.take() {
            let _ = e.read_to_string(&mut err);
        }
        err
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn routes() -> Vec<(&'static str, &'static str, Value)> {
    vec![
        (
            "POST",
            "/api/connectors",
            json!({"provider": "slack", "workspace": "acme", "values": {"SLACK_MCP_XOXB_TOKEN": SECRET}, "confirm": true}),
        ),
        (
            "POST",
            "/api/connectors/slack-acme/move",
            json!({"workspace": "personal", "confirm": true}),
        ),
        (
            "PUT",
            "/api/connectors/slack-acme/alias",
            json!({"alias": "x", "confirm": true}),
        ),
        ("POST", "/api/connectors/legacy", json!({"confirm": true})),
        ("POST", "/api/sync", json!({"confirm": true})),
        (
            "DELETE",
            "/api/connectors/slack-acme",
            json!({"confirm": true}),
        ), // last: the served pass removes it
    ]
}

#[test]
fn every_new_route_is_guarded_and_a_refused_request_changes_nothing() {
    let world = World::new(&ACME);
    std::fs::write(
        world.keychain(),
        json!({"easy-personal/slack": "{}"}).to_string(),
    )
    .unwrap();
    let server = world.serve(&[]);
    let host = format!("127.0.0.1:{}", server.port);
    let (keychain, store) = (
        std::fs::read(world.keychain()).unwrap(),
        world.store_bytes(),
    );
    for (method, path, body) in routes() {
        let body = body.to_string();
        let cases: [(&str, Vec<(&str, &str)>); 5] = [
            (
                "no token",
                vec![("Host", &host), ("content-type", "application/json")],
            ),
            (
                "wrong token",
                vec![
                    ("Host", &host),
                    ("x-dashboard-token", "nope"),
                    ("content-type", "application/json"),
                ],
            ),
            (
                "bad host",
                vec![
                    ("Host", "evil.test"),
                    ("x-dashboard-token", &server.token),
                    ("content-type", "application/json"),
                ],
            ),
            (
                "cross origin",
                vec![
                    ("Host", &host),
                    ("Origin", "http://evil.test"),
                    ("x-dashboard-token", &server.token),
                    ("content-type", "application/json"),
                ],
            ),
            (
                "repeated token",
                vec![
                    ("Host", &host),
                    ("x-dashboard-token", &server.token),
                    ("x-dashboard-token", &server.token),
                    ("content-type", "application/json"),
                ],
            ),
        ];
        for (what, headers) in cases {
            let got = server.raw(method, path, &headers, body.as_bytes());
            assert_eq!(
                (got.status, got.json()),
                (403, json!({"error": "forbidden"})),
                "{method} {path} {what}"
            );
            assert!(got.head.to_ascii_lowercase().contains(&format!(
                "content-security-policy: {}",
                CSP.to_ascii_lowercase()
            )));
        }
    }
    assert_eq!(
        std::fs::read(world.keychain()).unwrap(),
        keychain,
        "the keychain was written"
    );
    assert_eq!(world.store_bytes(), store, "the store was written");
    assert_eq!(world.log(), "", "claude ran");
    // the same requests with the guard satisfied are served (else the above proves nothing)
    for (method, path, body) in routes() {
        assert_eq!(
            server.json(method, path, &body).status,
            200,
            "{method} {path}"
        );
    }
}

#[test]
fn a_body_over_one_mib_is_413_and_the_wrong_methods_are_405() {
    let world = World::new(&ACME);
    let server = world.serve(&[]);
    let big = json!({"provider": "slack", "pad": "x".repeat(1 << 20)});
    for (method, path, _) in routes() {
        let got = server.json(method, path, &big);
        assert_eq!(
            (got.status, got.json()),
            (413, json!({"error": "request body too large"})),
            "{method} {path}"
        );
    }
    for (method, path) in [
        ("GET", "/api/connectors/slack-acme"),
        ("PATCH", "/api/connectors"),
        ("GET", "/api/connectors/x/move"),
        ("POST", "/api/connectors/x/alias"),
    ] {
        assert_eq!(
            server.json(method, path, &json!({})).status,
            405,
            "{method} {path}"
        );
    }
    assert_eq!(world.log(), "");
}

#[test]
fn a_locked_keychain_refuses_with_the_fixed_text_and_writes_nothing() {
    let world = World::new(&[]);
    std::fs::write(world.keychain(), "locked").unwrap();
    let store = world.store_bytes();
    let server = world.serve(&[]);
    let body = json!({"provider": "slack", "workspace": "acme", "values": {"SLACK_MCP_XOXB_TOKEN": SECRET}, "confirm": true});
    let got = server.json("POST", "/api/connectors", &body);
    assert_eq!(got.status, 200);
    let lines = got.json()["lines"].clone();
    assert_eq!(got.json()["status"], 1);
    assert!(
        lines
            .to_string()
            .contains("no usable OS keychain (Secret Service)"),
        "{lines}"
    );
    assert!(!got.body.contains(SECRET));
    let legacy = server.raw(
        "GET",
        "/api/connectors/legacy",
        &[
            ("Host", &format!("127.0.0.1:{}", server.port)),
            ("x-dashboard-token", &server.token),
        ],
        b"",
    );
    assert_eq!(legacy.status, 503);
    let logs = server.stop();
    assert_eq!(std::fs::read_to_string(world.keychain()).unwrap(), "locked");
    assert_eq!(world.store_bytes(), store);
    assert!(!logs.contains(SECRET));
}

#[test]
fn when_the_store_refuses_the_row_the_keychain_item_is_rolled_back() {
    let world = World::new(&[
        "CREATE TRIGGER t050_refuse BEFORE INSERT ON connectors BEGIN SELECT RAISE(ABORT, 'refused'); END",
    ]);
    let server = world.serve(&[]);
    let body = json!({"provider": "slack", "workspace": "acme", "values": {"SLACK_MCP_XOXB_TOKEN": SECRET}, "confirm": true});
    let got = server.json("POST", "/api/connectors", &body).json();
    assert_eq!(got["status"], 1, "{got}");
    assert_eq!(
        got["lines"].as_array().unwrap().last().unwrap(),
        "the store refused the connector (changed meanwhile); keychain write rolled back."
    );
    assert_eq!(
        std::fs::read_to_string(world.keychain()).unwrap(),
        "{}",
        "no secret without its row"
    );
}

#[test]
fn two_adds_at_once_store_one_connector_and_one_item() {
    let world = World::new(&[]);
    let server = std::sync::Arc::new(world.serve(&[]));
    let handles: Vec<_> = (0..2)
        .map(|n| {
            let server = std::sync::Arc::clone(&server);
            std::thread::spawn(move || {
                let body = json!({"provider": "slack", "workspace": "acme", "values": {"SLACK_MCP_XOXB_TOKEN": format!("{SECRET}-{n}")}, "confirm": true});
                server.json("POST", "/api/connectors", &body).json()["status"].as_i64().unwrap()
            })
        })
        .collect();
    let mut statuses: Vec<i64> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    statuses.sort();
    assert_eq!(
        statuses,
        [0, 1],
        "one added, one refused (one per provider)"
    );
    let items: Value =
        serde_json::from_str(&std::fs::read_to_string(world.keychain()).unwrap()).unwrap();
    assert_eq!(items.as_object().unwrap().len(), 1);
    let rows: i64 = world
        .conn()
        .query_row("SELECT COUNT(*) FROM connectors", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 1);
}

/// A `claude` on `PATH` that cannot start (its interpreter is missing: the spawn itself fails, as a timeout would end it).
fn claude_that_cannot_run(world: &World) {
    use std::os::unix::fs::PermissionsExt;
    let exe = world.dir.join("bin/claude");
    std::fs::write(&exe, "#!/nonexistent/t050-no-interpreter\n").unwrap();
    std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
}

const CLAUDE_NOT_RUN: &str =
    "the `claude` CLI could not run (it did not start or timed out); nothing was removed.";

/// "Could not run" is not "no entry": removing a stored connector keeps its row, its registration and its keychain item
/// (else the plan would `forget` the registration and delete the item while the Claude entry stays).
#[test]
fn a_claude_that_cannot_run_removes_no_stored_connector() {
    let world = World::new(&ACME);
    let items = json!({"easy-personal/slack-acme": "{}"}).to_string();
    std::fs::write(world.keychain(), &items).unwrap();
    claude_that_cannot_run(&world);
    let store = world.store_bytes();
    let server = world.serve(&[]);
    let got = server.json(
        "DELETE",
        "/api/connectors/slack-acme",
        &json!({"confirm": true}),
    );
    assert_eq!(got.status, 200, "{}", got.body);
    let body = got.json();
    assert_eq!(body["status"], 1, "{body}");
    assert!(
        body["lines"]
            .to_string()
            .contains("skipped (`claude` could not run)"),
        "{body}"
    );
    server.stop();
    assert_eq!(std::fs::read_to_string(world.keychain()).unwrap(), items);
    assert_eq!(world.store_bytes(), store, "the store was written");
    // and the sync plan does not `forget` it either
    let server = world.serve(&[]);
    let got = server.json("POST", "/api/sync", &json!({"account": 2, "confirm": true}));
    assert!(
        got.json()["lines"]
            .to_string()
            .contains("skipped (`claude` could not run)"),
        "{}",
        got.body
    );
    server.stop();
    assert_eq!(world.store_bytes(), store, "the store was written");
    assert_eq!(std::fs::read_to_string(world.keychain()).unwrap(), items);
}

/// The pre-workspace remove: when `claude mcp get` could not run, the keychain item stays (the default account's entry
/// may still point at the launcher).
#[test]
fn a_claude_that_cannot_run_removes_no_pre_workspace_item() {
    let world = World::new(&[]);
    let items = json!({"easy-personal/slack": "{}"}).to_string();
    std::fs::write(world.keychain(), &items).unwrap();
    claude_that_cannot_run(&world);
    let store = world.store_bytes();
    let server = world.serve(&[]);
    let got = server.json("DELETE", "/api/connectors/slack", &json!({"confirm": true}));
    assert_eq!(
        (got.status, got.json()),
        (
            200,
            json!({"status": 1, "lines": [CLAUDE_NOT_RUN], "pending": false})
        )
    );
    server.stop();
    assert_eq!(std::fs::read_to_string(world.keychain()).unwrap(), items);
    assert_eq!(world.store_bytes(), store);
}

/// The account folder comes from the store row, never from the request, and reaches `claude` only as the value of
/// `CLAUDE_CONFIG_DIR` (no shell: the text is never run). Python passes it the same way and refuses nothing.
#[test]
fn a_hostile_account_folder_is_only_ever_an_environment_value() {
    let world = World::new(&ACME);
    let hostile = world
        .dir
        .join("agent-root/.claude-$(touch PWNED) `touch PWNED2`; x");
    std::fs::create_dir_all(&hostile).unwrap();
    world
        .conn()
        .execute(
            "UPDATE agent_accounts SET folder = ?1, config_dir_env = ?1 WHERE id = 2",
            [hostile.to_str().unwrap()],
        )
        .unwrap();
    // the registration is not in that account's Claude entries: the plan re-adds it, and that `claude` call fails
    let server = world.serve(&[
        ("FAKE_CLAUDE_FAIL", "add-json acme-slack"),
        ("FAKE_CLAUDE_STDERR", SECRET),
    ]);
    let got = server.json("POST", "/api/sync", &json!({"account": 2, "confirm": true}));
    assert_eq!(got.json()["status"], 1, "{}", got.body);
    // claude's stderr (a secret here) is never shown or logged
    assert!(
        !got.body.contains(SECRET) && !got.body.contains("boom"),
        "{}",
        got.body
    );
    let logs = server.stop();
    assert!(!logs.contains(SECRET) && !logs.contains("boom"));
    let log = world.log();
    assert!(
        log.contains(&format!(
            "CCD={}\nmcp\nadd-json\nacme-slack\n",
            hostile.display()
        )),
        "{log}"
    );
    for marker in ["PWNED", "PWNED2"] {
        assert!(
            !world.dir.join(marker).exists() && !world.dir.join("home").join(marker).exists(),
            "{marker}: a shell ran"
        );
    }
}

/// HUN-72 / AC11: a name that is not a stored connector and not a catalog id is refused before any keychain or `claude`
/// call, whatever it holds.
#[test]
fn a_hostile_connector_name_reaches_neither_the_keychain_nor_claude() {
    let world = World::new(&[]);
    std::fs::write(
        world.keychain(),
        json!({"easy-personal/slack": "{}"}).to_string(),
    )
    .unwrap();
    let server = world.serve(&[]);
    for name in [
        "..%2f..%2fslack",
        "slack%00",
        "SLACK",
        "a%20b",
        "%24(touch%20x)",
        "slack;rm",
        &"a".repeat(65),
    ] {
        let got = server.json(
            "DELETE",
            &format!("/api/connectors/{name}"),
            &json!({"confirm": true}),
        );
        assert_eq!(
            (got.status, got.json()),
            (404, json!({"error": "unknown connector"})),
            "{name}"
        );
    }
    assert_eq!(world.log(), "");
    assert_eq!(
        std::fs::read_to_string(world.keychain()).unwrap(),
        json!({"easy-personal/slack": "{}"}).to_string()
    );
}

/// The real backend (`keyring` over `zbus`, which blocks on its own runtime) is called from inside the server's tokio
/// runtime: it must answer the fixed text, never panic ("cannot start a runtime from within a runtime") or hang. The bus
/// address leads nowhere, so no real keychain is reached.
#[test]
fn the_real_backend_inside_the_server_refuses_cleanly_with_no_session_bus() {
    let world = World::new(&ACME);
    let server = world.serve_with(&[], false);
    let host = format!("127.0.0.1:{}", server.port);
    let body = json!({"provider": "slack", "workspace": "acme", "values": {"SLACK_MCP_XOXB_TOKEN": SECRET}, "confirm": true});
    let got = server.json("POST", "/api/connectors", &body);
    assert_eq!(got.status, 200, "{}", got.body);
    assert_eq!(
        got.json()["lines"][0].as_str().unwrap_or(""),
        NO_BACKEND,
        "{}",
        got.body
    );
    let legacy = server.raw(
        "GET",
        "/api/connectors/legacy",
        &[("Host", &host), ("x-dashboard-token", &server.token)],
        b"",
    );
    assert_eq!(
        (legacy.status, legacy.json()),
        (503, json!({"error": NO_BACKEND}))
    );
    let removed = server.json(
        "DELETE",
        "/api/connectors/slack-acme",
        &json!({"confirm": true}),
    );
    assert_eq!(removed.status, 200, "{}", removed.body);
    let logs = server.stop();
    assert!(!logs.contains("panicked"), "{logs}");
}
