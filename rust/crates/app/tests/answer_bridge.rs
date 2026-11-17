//! T106: the answer bridge over the real binary (FR-113, FR-114, FR-115). Every test has its own temp folder for `HOME`,
//! `EASY_PERSONAL_HOME` and `EASY_PERSONAL_AGENT_ROOT`, a copy of `parity/store-v11.sqlite` with one confirmed Claude
//! account `Work` (id 1) whose folder is `<home>/.claude`; the owner's `~/.claude*` and `~/.easy-personal` are never reached.
//!
//! - `hooks install|remove|status` over a settings file with other hooks (AC 1, 2);
//! - `hook ask` with no app, a stale socket, a wrong owner and bad input: exit 0, empty stdout, fast (AC 4, 6);
//! - `serve` with a transcript that has an open `Bash` call: a hook waits, `GET /api/live` shows `pending_answer`,
//!   `POST /api/answer` makes `hook ask` print the documented JSON, a second answer is a 409, the socket is 0600 in a 0700
//!   folder and goes at Ctrl-C, and no prompt text reaches the server's stdout or stderr (AC 3, 5, 7, 8, 9).

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_easy-personal-rs");

struct Home(PathBuf);

impl Home {
    fn new(tag: &str) -> Home {
        static N: AtomicU32 = AtomicU32::new(0);
        // short, so `answer.sock` fits a socket path
        let home = std::env::temp_dir().join(format!(
            "ep-t106-{tag}-{}-{}",
            std::process::id() % 100_000,
            N.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(home.join(".claude/projects/p")).unwrap();
        std::fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../parity/store-v11.sqlite"),
            home.join("store.sqlite"),
        )
        .unwrap();
        let conn = rusqlite::Connection::open(home.join("store.sqlite")).unwrap();
        conn.execute(
            "INSERT INTO agent_accounts (id, provider, folder, label, status) VALUES (1, 'claude', ?1, 'Work', 'confirmed')",
            [home.join(".claude").to_str().unwrap()],
        )
        .unwrap();
        Home(home)
    }

    fn settings(&self) -> PathBuf {
        self.0.join(".claude/settings.json")
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(BIN);
        command
            .args(args)
            .env("HOME", &self.0)
            .env("EASY_PERSONAL_HOME", &self.0)
            .env("EASY_PERSONAL_AGENT_ROOT", &self.0)
            .env_remove("CLAUDE_CONFIG_DIR");
        command
    }

    /// Runs the binary with `stdin` and waits for it.
    fn run(&self, args: &[&str], stdin: &str) -> Output {
        let mut child = self
            .command(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn home_status(home: &Home) -> String {
    let out = home.run(&["hooks", "status"], "");
    text(&out.stdout)
        .trim_end()
        .rsplit(": ")
        .next()
        .unwrap()
        .to_string()
}

fn text(out: &[u8]) -> String {
    String::from_utf8_lossy(out).into_owned()
}

fn json_of(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

const OWNER: &str = r#"{
  "permissions": {"allow": ["Bash(ls:*)"]},
  "model": "opus",
  "hooks": {
    "PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "/usr/bin/audit"}]}],
    "Stop": [{"hooks": [{"type": "command", "command": "echo stop"}]}]
  }
}"#;

// ---- hooks install | remove | status (AC 1, 2) ----

#[test]
fn hooks_install_status_and_remove_keep_every_other_key() {
    let home = Home::new("cli");
    std::fs::write(home.settings(), OWNER).unwrap();
    let before = json_of(&home.settings());

    let out = home.run(&["hooks", "status", "--account", "1"], "");
    assert_eq!(text(&out.stdout), "1 \"Work\": off\n");

    // no: the question is asked, the file is as it was
    let out = home.run(&["hooks", "install", "--account", "1"], "n\n");
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stderr));
    let shown = text(&out.stdout);
    assert!(
        shown.contains("\"matcher\": \"AskUserQuestion\"") && shown.contains("\"timeout\": 130")
    );
    assert!(shown.contains(&format!("{BIN} hook ask")), "{shown}");
    assert!(shown.contains("already running are not covered"), "{shown}");
    assert!(
        shown.contains(
            "A running app picks this up after a restart or when Settings turns the bridge on.\n"
        ),
        "{shown}"
    );
    assert_eq!(json_of(&home.settings()), before);
    assert!(!home.0.join("backups").exists());

    // yes: the entries are added and the rest kept
    let out = home.run(&["hooks", "install", "--account", "1", "--yes"], "");
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(
        text(&out.stdout).ends_with("done: on\n"),
        "{}",
        text(&out.stdout)
    );
    let after = json_of(&home.settings());
    let command = format!("{BIN} hook ask");
    let ours = json!({"type": "command", "command": command, "timeout": 130});
    assert_eq!(after["permissions"], before["permissions"]);
    assert_eq!(after["model"], before["model"]);
    assert_eq!(after["hooks"]["Stop"], before["hooks"]["Stop"]);
    assert_eq!(
        after["hooks"]["PreToolUse"][0],
        before["hooks"]["PreToolUse"][0]
    );
    assert_eq!(
        after["hooks"]["PreToolUse"][1],
        json!({"matcher": "AskUserQuestion", "hooks": [ours.clone()]})
    );
    assert_eq!(
        after["hooks"]["PermissionRequest"],
        json!([{"hooks": [ours]}])
    );
    assert_eq!(after["hooks"]["PreToolUse"].as_array().unwrap().len(), 2);
    let backups: Vec<_> = std::fs::read_dir(home.0.join("backups")).unwrap().collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(
        std::fs::read_to_string(backups[0].as_ref().unwrap().path()).unwrap(),
        OWNER,
        "the old file, byte for byte"
    );
    assert_eq!(
        text(&home.run(&["hooks", "status"], "").stdout),
        "1 \"Work\": on\n"
    );

    // twice: nothing is added, nothing is written
    let out = home.run(&["hooks", "install", "--yes"], "");
    assert!(
        text(&out.stdout).ends_with("nothing to change: on\n"),
        "{}",
        text(&out.stdout)
    );
    assert_eq!(json_of(&home.settings()), after);
    assert_eq!(
        std::fs::read_dir(home.0.join("backups")).unwrap().count(),
        1
    );

    // a hand-edited entry: status says so, remove keeps it
    let mut edited = after.clone();
    edited["hooks"]["PermissionRequest"][0]["hooks"][0]["timeout"] = json!(9);
    std::fs::write(home.settings(), edited.to_string()).unwrap();
    assert_eq!(
        text(&home.run(&["hooks", "status"], "").stdout),
        "1 \"Work\": changed by hand\n"
    );
    let out = home.run(&["hooks", "install", "--yes"], "");
    assert_eq!(
        out.status.code(),
        Some(2),
        "install refuses over a changed entry"
    );
    std::fs::write(home.settings(), after.to_string()).unwrap();

    // remove: back to the owner's file
    let out = home.run(&["hooks", "remove", "--account", "Work", "--yes"], "");
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(json_of(&home.settings()), before);
    assert_eq!(
        text(&home.run(&["hooks", "status"], "").stdout),
        "1 \"Work\": off\n"
    );
}

/// AC 1: a settings file that is not valid JSON is refused and left byte-identical.
#[test]
fn a_settings_file_that_is_not_json_is_refused_byte_identical() {
    let home = Home::new("badjson");
    let broken = "{\"hooks\": {\"PreToolUse\": [ oops";
    std::fs::write(home.settings(), broken).unwrap();
    let out = home.run(&["hooks", "install", "--account", "1", "--yes"], "");
    assert_eq!(out.status.code(), Some(2));
    assert!(
        text(&out.stderr).contains("not a JSON object"),
        "{}",
        text(&out.stderr)
    );
    assert!(
        !text(&out.stderr).contains(home.0.to_str().unwrap()),
        "no path in the message"
    );
    assert_eq!(std::fs::read_to_string(home.settings()).unwrap(), broken);
    assert!(!home.0.join("backups").exists());
    assert_eq!(
        text(&home.run(&["hooks", "status"], "").stdout),
        "1 \"Work\": unreadable\n"
    );
}

/// The default account and a missing account folder.
#[test]
fn install_without_an_account_uses_the_default_one_and_never_makes_the_folder() {
    let home = Home::new("default");
    let out = home.run(&["hooks", "install", "--yes"], "");
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(home.settings().is_file(), "a new file, no backup");
    let out = home.run(&["hooks", "install", "--account", "nope", "--yes"], "");
    assert_eq!(out.status.code(), Some(2));
    std::fs::remove_dir_all(home.0.join(".claude")).unwrap();
    let out = home.run(&["hooks", "install", "--yes"], "");
    assert_eq!(out.status.code(), Some(2), "{}", text(&out.stderr));
    assert!(!home.0.join(".claude").exists());
}

// ---- hook ask with no app (AC 4, 6) ----

fn hook_input(session: &str) -> String {
    json!({"hook_event_name": "PermissionRequest", "tool_name": "Bash", "session_id": session,
           "tool_input": {"command": "PLANTED-CMD echo hi"}})
    .to_string()
}

/// AC 4: no socket: exit 0, empty stdout, under 50 ms. Twenty runs of the real binary: the median and the 90th percentile
/// are under 50 ms (a loaded machine may delay one run); every run exits 0 with nothing on stdout.
#[test]
fn hook_ask_with_no_app_is_silent_exit_zero_and_fast() {
    let home = Home::new("noapp");
    let mut times = Vec::new();
    for _ in 0..20 {
        let began = Instant::now();
        let out = home.run(&["hook", "ask"], &hook_input("s1"));
        times.push(began.elapsed());
        assert_eq!(out.status.code(), Some(0));
        assert!(out.stdout.is_empty(), "{}", text(&out.stdout));
        assert!(out.stderr.is_empty(), "{}", text(&out.stderr));
    }
    times.sort();
    let (median, p90) = (times[9], times[17]);
    eprintln!(
        "hook ask, no app: median {median:?}, p90 {p90:?}, max {:?}",
        times[19]
    );
    assert!(median < Duration::from_millis(50), "median {median:?}");
    assert!(p90 < Duration::from_millis(50), "p90 {p90:?}");
}

#[test]
fn hook_ask_hands_back_for_a_stale_socket_bad_input_and_a_socket_that_is_not_private() {
    let home = Home::new("stale");
    // a stale socket: bound, the listener gone, the file left
    let sock = home.0.join("answer.sock");
    drop(UnixListener::bind(&sock).unwrap());
    std::fs::set_permissions(&home.0, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::set_permissions(&sock, std::fs::Permissions::from_mode(0o600)).unwrap();
    for stdin in [
        hook_input("s1").as_str(),
        "",
        "not json",
        "{\"hook_event_name\":",
    ] {
        let out = home.run(&["hook", "ask"], stdin);
        assert_eq!(
            (out.status.code(), out.stdout.len()),
            (Some(0), 0),
            "stale / bad input {stdin:?}"
        );
    }
    // a live socket that a stranger could reach (mode 0666) is not used: nothing connects
    std::fs::remove_file(&sock).unwrap();
    let listener = UnixListener::bind(&sock).unwrap();
    std::fs::set_permissions(&sock, std::fs::Permissions::from_mode(0o666)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let out = home.run(&["hook", "ask"], &hook_input("s1"));
    assert_eq!((out.status.code(), out.stdout.len()), (Some(0), 0));
    assert!(
        listener.accept().is_err(),
        "the client never connected to a socket that is not private"
    );
}

// ---- serve + hook ask end to end (AC 3, 5, 7, 8, 9) ----

struct Serve {
    child: Child,
    stdout: BufReader<ChildStdout>,
    port: u16,
    token: String,
}

impl Serve {
    fn start(home: &Home) -> Serve {
        let mut child = home
            .command(&["serve", "--no-ingest"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        let url = line
            .trim_end()
            .strip_prefix("dashboard: http://127.0.0.1:")
            .unwrap();
        let (port, token) = url.split_once("/#").unwrap();
        let mut second = String::new();
        stdout.read_line(&mut second).unwrap();
        Serve {
            port: port.parse().unwrap(),
            token: token.into(),
            child,
            stdout,
        }
    }

    /// One request with `Connection: close`: the status and the body.
    fn request(&self, method: &str, path: &str, body: Option<&Value>, token: bool) -> (u16, Value) {
        let mut tcp = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        let body = body.map(Value::to_string);
        let mut req = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n",
            self.port
        );
        if token {
            req += &format!("x-dashboard-token: {}\r\n", self.token);
        }
        if let Some(body) = &body {
            req += &format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                body.len()
            );
        }
        req += "\r\n";
        req += body.as_deref().unwrap_or("");
        tcp.write_all(req.as_bytes()).unwrap();
        let mut raw = String::new();
        tcp.read_to_string(&mut raw).unwrap();
        let status = raw.split(' ').nth(1).unwrap().parse().unwrap();
        let body = raw.split_once("\r\n\r\n").map_or("", |(_, b)| b);
        (status, serde_json::from_str(body).unwrap_or(Value::Null))
    }

    /// Opens `GET /api/live` and keeps it open: the bridge holds a hook only while a stream is open. Returns once the
    /// first message arrived (the server has counted the stream by then).
    fn watch(&self) -> Live {
        let mut tcp = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        tcp.write_all(
            format!(
                "GET /api/live?token={} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
                self.token, self.port
            )
            .as_bytes(),
        )
        .unwrap();
        tcp.set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let mut live = Live {
            tcp,
            buffer: String::new(),
        };
        live.next_snapshot(|_| true);
        live
    }

    fn sigint(&mut self) -> (Option<i32>, String, String) {
        assert!(
            Command::new("kill")
                .args(["-INT", &self.child.id().to_string()])
                .status()
                .unwrap()
                .success()
        );
        let began = Instant::now();
        while self.child.try_wait().unwrap().is_none() {
            assert!(began.elapsed() < Duration::from_secs(15), "no exit");
            std::thread::sleep(Duration::from_millis(20));
        }
        let (mut out, mut err) = (String::new(), String::new());
        self.stdout.read_to_string(&mut out).unwrap();
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut err)
            .unwrap();
        (self.child.wait().unwrap().code(), out, err)
    }
}

impl Drop for Serve {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// An open `GET /api/live` stream, read message by message.
struct Live {
    tcp: TcpStream,
    buffer: String,
}

impl Live {
    /// The next snapshot (in the order the server sent them, from where the last call stopped) that `ready` accepts;
    /// the ones before it are passed over. Fails after 15 s.
    fn next_snapshot(&mut self, ready: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            while let Some(end) = self.buffer.find("\n\n") {
                let message: String = self.buffer.drain(..end + 2).collect();
                let found = message
                    .lines()
                    .filter_map(|l| l.strip_prefix("data: "))
                    .filter_map(|d| serde_json::from_str::<Value>(d).ok())
                    .find(|d| d["sessions"].is_array() && ready(d));
                if let Some(found) = found {
                    return found;
                }
            }
            assert!(
                Instant::now() < deadline,
                "the live stream never got there: {}",
                self.buffer
            );
            let mut chunk = [0u8; 65536];
            if let Ok(n) = self.tcp.read(&mut chunk) {
                self.buffer.push_str(&text(&chunk[..n]));
            }
        }
    }

    /// The next snapshot whose sessions hold exactly `n` `pending_answer`s.
    fn pending(&mut self, n: usize) -> Value {
        self.next_snapshot(|d| pending_count(d) == n)
    }
}

fn pending_count(snapshot: &Value) -> usize {
    snapshot["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["pending_answer"].is_object())
        .count()
}

/// An open `Bash` call of session `sid`, written just now: the live tail lists the session with the call open.
fn open_call(home: &Home, sid: &str) {
    let stamp = jiff::Timestamp::now().strftime("%Y-%m-%dT%H:%M:%S.000Z");
    let line = format!(
        "{{\"type\":\"assistant\",\"sessionId\":\"{sid}\",\"timestamp\":\"{stamp}\",\"cwd\":\"/tmp\",\"message\":{{\"id\":\"m-{sid}\",\"model\":\"x\",\"content\":[{{\"type\":\"tool_use\",\"id\":\"tu-{sid}\",\"name\":\"Bash\",\"input\":{{\"command\":\"ls\"}}}}]}}}}\n"
    );
    std::fs::write(home.0.join(format!(".claude/projects/p/{sid}.jsonl")), line).unwrap();
}

fn wait_for(what: &str, mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !ready() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn spawn_hook(home: &Home, stdin: &str) -> Child {
    let mut child = home
        .command(&["hook", "ask"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child
}

#[test]
fn an_answer_from_the_app_reaches_the_waiting_hook_and_leaks_nowhere() {
    let home = Home::new("e2e");
    open_call(&home, "sess-a");
    open_call(&home, "sess-b");
    // the bridge is on for the account (the owner's `hooks install`), so `serve` opens the socket at its start
    assert_eq!(
        home.run(&["hooks", "install", "--yes"], "").status.code(),
        Some(0)
    );
    let mut serve = Serve::start(&home);
    let mut live = serve.watch(); // a stream is open: only then does the bridge hold a hook

    // AC 8: the socket is 0600 in a 0700 folder
    let sock = home.0.join("answer.sock");
    wait_for("the socket", || sock.exists());
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!((mode(&sock), mode(&home.0)), (0o600, 0o700));

    // two hooks wait, one per session
    let hook_a = spawn_hook(&home, &hook_input("sess-a"));
    let hook_b = spawn_hook(&home, &hook_input("sess-b"));
    let last = live.pending(2);
    let pending = |sid: &str| -> Value {
        last["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["session_id"] == sid)
            .unwrap()["pending_answer"]
            .clone()
    };
    let (pa, pb) = (pending("sess-a"), pending("sess-b"));
    assert_eq!(
        (pa["kind"].as_str(), pa["tool"].as_str()),
        (Some("permission"), Some("Bash"))
    );
    assert_eq!(pa["input"], "PLANTED-CMD echo hi");
    assert_eq!(pa["complete"], true);
    assert_eq!(
        pa.as_object().unwrap().keys().collect::<Vec<_>>().len(),
        5,
        "id, kind, tool, input, complete: {pa}"
    );
    let (ida, idb) = (pa["id"].as_str().unwrap(), pb["id"].as_str().unwrap());
    assert_ne!(ida, idb);

    // the guard and the body rules
    let deny_b = json!({"id": idb, "confirmed": true, "decision": "deny", "message": "no thanks"});
    assert_eq!(
        serve.request("POST", "/api/answer", Some(&deny_b), false).0,
        403,
        "no token"
    );
    assert_eq!(
        serve
            .request(
                "POST",
                "/api/answer",
                Some(&json!({"id": idb, "decision": "deny"})),
                true
            )
            .0,
        400,
        "not confirmed"
    );
    assert_eq!(
        serve
            .request(
                "POST",
                "/api/answer",
                Some(&json!({"id": idb, "confirmed": true, "decision": "maybe"})),
                true
            )
            .0,
        400
    );
    assert_eq!(
        serve
            .request(
                "POST",
                "/api/answer",
                Some(&json!({"id": "f".repeat(32), "confirmed": true, "decision": "allow"})),
                true
            )
            .0,
        404
    );

    // answers never cross: deny B, allow A; a second answer to B is a 409
    assert_eq!(
        serve.request("POST", "/api/answer", Some(&deny_b), true).0,
        200
    );
    let allow_a = json!({"id": ida, "confirmed": true, "decision": "allow"});
    assert_eq!(
        serve.request("POST", "/api/answer", Some(&deny_b), true).0,
        409,
        "second answer"
    );
    assert_eq!(
        serve.request("POST", "/api/answer", Some(&allow_a), true).0,
        200
    );
    let (out_a, out_b) = (
        hook_a.wait_with_output().unwrap(),
        hook_b.wait_with_output().unwrap(),
    );
    assert_eq!(
        (out_a.status.code(), out_b.status.code()),
        (Some(0), Some(0))
    );
    let reply = |out: &Output| -> Value { serde_json::from_slice(&out.stdout).unwrap() };
    assert_eq!(
        reply(&out_a),
        json!({"hookSpecificOutput": {"hookEventName": "PermissionRequest", "decision": {"behavior": "allow"}}})
    );
    assert_eq!(
        reply(&out_b),
        json!({"hookSpecificOutput": {"hookEventName": "PermissionRequest",
                                      "decision": {"behavior": "deny", "message": "no thanks"}}})
    );
    assert!(out_a.stdout.ends_with(b"\n") && out_a.stderr.is_empty() && out_b.stderr.is_empty());

    // release: the hook exits 0 with nothing
    let mut hook_c = spawn_hook(&home, &hook_input("sess-a"));
    let snapshot = live.next_snapshot(|d| {
        d["sessions"].as_array().unwrap().iter().any(|s| {
            s["pending_answer"]["id"]
                .as_str()
                .is_some_and(|id| id != ida && id != idb)
        })
    });
    let id = snapshot["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|s| s["pending_answer"]["id"].as_str().map(str::to_string))
        .unwrap();
    assert_eq!(
        serve
            .request(
                "POST",
                "/api/answer/release",
                Some(&json!({"id": id, "confirmed": true})),
                true
            )
            .0,
        200
    );
    assert_eq!(
        serve
            .request(
                "POST",
                "/api/answer/release",
                Some(&json!({"id": id, "confirmed": true})),
                true
            )
            .0,
        404
    );
    let out_c = {
        wait_for("the released hook", || hook_c.try_wait().unwrap().is_some());
        let mut out = Vec::new();
        hook_c.stdout.take().unwrap().read_to_end(&mut out).unwrap();
        out
    };
    assert!(out_c.is_empty());

    // F1, fails closed: a command over the cap is cut (`complete: false`), its allow is a 409 (the item stays), its deny a 200
    let big = format!("PLANTED-BIG {}", "x".repeat(20_000));
    let big_input =
        json!({"hook_event_name": "PermissionRequest", "tool_name": "Bash", "session_id": "sess-a",
                           "tool_input": {"command": big}})
        .to_string();
    let hook_d = spawn_hook(&home, &big_input);
    let snapshot = live.next_snapshot(|d| {
        d["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["pending_answer"]["complete"] == false)
    });
    let card = snapshot["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|s| {
            s["pending_answer"]
                .as_object()
                .filter(|p| p["complete"] == false)
        })
        .unwrap();
    assert_eq!(card["input"].as_str().unwrap().chars().count(), 16_000);
    let big_id = card["id"].as_str().unwrap().to_string();
    let allow_big = json!({"id": big_id, "confirmed": true, "decision": "allow"});
    assert_eq!(
        serve
            .request("POST", "/api/answer", Some(&allow_big), true)
            .0,
        409,
        "an input that was not shown whole is never allowed"
    );
    let deny_big = json!({"id": big_id, "confirmed": true, "decision": "deny"});
    assert_eq!(
        serve
            .request("POST", "/api/answer", Some(&deny_big), true)
            .0,
        200,
        "deny still works"
    );
    let out_d = hook_d.wait_with_output().unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&out_d.stdout).unwrap()["hookSpecificOutput"]["decision"]["behavior"],
        "deny"
    );

    // AC 3: the routes (GET shows the entry text; PUT writes only after `confirmed`)
    let (status, body) = serve.request("GET", "/api/hooks", None, true);
    assert_eq!(status, 200);
    assert_eq!(
        body["accounts"],
        json!([{"id": "1", "label": "Work", "status": "on"}])
    );
    assert_eq!(body["listening"], true, "the socket is open");
    assert_eq!(body["command"], format!("{BIN} hook ask"));
    assert!(
        body["entries_text"]
            .as_str()
            .unwrap()
            .contains("\"timeout\": 130")
    );
    assert_eq!(
        serve
            .request(
                "PUT",
                "/api/hooks",
                Some(&json!({"account": "1", "on": false})),
                true
            )
            .0,
        400
    );
    assert_eq!(
        home_status(&home),
        "on",
        "nothing written without confirmed"
    );
    let off = json!({"account": "1", "on": false, "confirmed": true});
    assert_eq!(serve.request("PUT", "/api/hooks", Some(&off), false).0, 403);
    let (status, body) = serve.request("PUT", "/api/hooks", Some(&off), true);
    assert_eq!((status, body["status"].as_str()), (200, Some("off")));
    assert_eq!(body["listening"], true);
    assert_eq!(home_status(&home), "off");
    let on = json!({"account": "1", "on": true, "confirmed": true});
    let (status, body) = serve.request("PUT", "/api/hooks", Some(&on), true);
    assert_eq!((status, body["status"].as_str()), (200, Some("on")));
    assert_eq!(
        serve
            .request(
                "PUT",
                "/api/hooks",
                Some(&json!({"account": "9", "on": true, "confirmed": true})),
                true
            )
            .0,
        400
    );

    // AC 9: nothing of the prompt on /api/alerts, and (below) none on the server's stdout or stderr
    let (_, alerts) = serve.request("GET", "/api/alerts", None, true);
    assert!(!alerts.to_string().contains("PLANTED"), "{alerts}");

    // AC 8: Ctrl-C removes the socket
    let (code, out, err) = serve.sigint();
    assert_eq!(code, Some(0), "{err}");
    assert!(!sock.exists(), "removed at a clean exit");
    for log in [&out, &err] {
        assert!(
            !log.contains("PLANTED") && !log.contains("no thanks"),
            "{log}"
        );
    }
}

/// AC 6: the app going away mid-wait: the hook exits 0 with no output.
#[test]
fn the_app_exiting_mid_wait_hands_the_hook_back_to_the_terminal() {
    let home = Home::new("exit");
    open_call(&home, "sess-x");
    assert_eq!(
        home.run(&["hooks", "install", "--yes"], "").status.code(),
        Some(0)
    );
    let mut serve = Serve::start(&home);
    let mut live = serve.watch();
    wait_for("the socket", || home.0.join("answer.sock").exists());
    let hook = spawn_hook(&home, &hook_input("sess-x"));
    live.pending(1);
    let (code, _, _) = serve.sigint();
    assert_eq!(code, Some(0));
    let out = hook.wait_with_output().unwrap();
    assert_eq!((out.status.code(), out.stdout.len()), (Some(0), 0));
}

/// With the bridge off nothing is created in the data folder; turning it on from the page opens the socket (and Ctrl-C
/// removes it), so a run that never uses the bridge leaves the data folder as it found it.
#[test]
fn with_the_bridge_off_serve_makes_no_socket_and_turning_it_on_opens_one() {
    let home = Home::new("lazy");
    let mut serve = Serve::start(&home);
    let sock = home.0.join("answer.sock");
    std::thread::sleep(Duration::from_millis(300));
    assert!(!sock.exists(), "off: no socket");
    let mut names: Vec<_> = std::fs::read_dir(&home.0)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    names.sort();
    assert!(!names.iter().any(|n| n == "answer.sock" || n == "backups"));
    let on = json!({"account": "1", "on": true, "confirmed": true});
    assert_eq!(serve.request("PUT", "/api/hooks", Some(&on), true).0, 200);
    wait_for("the socket", || sock.exists());
    let (code, _, err) = serve.sigint();
    assert_eq!(code, Some(0), "{err}");
    assert!(!sock.exists());
}

/// F3: an entry of exactly our shape naming another existing copy reads *other copy*; `install` replaces it and prints one
/// fixed notice; `remove` removes it.
#[test]
fn an_entry_of_another_copy_reads_other_copy_and_install_replaces_it_with_one_notice() {
    let home = Home::new("othercopy");
    let other = home.0.join("other/easy-personal-rs");
    std::fs::create_dir_all(other.parent().unwrap()).unwrap();
    std::fs::write(&other, "").unwrap();
    let entry = |matcher: bool| {
        let hook = json!({"type": "command", "command": format!("{} hook ask", other.display()), "timeout": 130});
        if matcher {
            json!([{"matcher": "AskUserQuestion", "hooks": [hook]}])
        } else {
            json!([{"hooks": [hook]}])
        }
    };
    let planted = json!({"model": "opus", "hooks": {"PreToolUse": entry(true), "PermissionRequest": entry(false)}});
    std::fs::write(home.settings(), planted.to_string()).unwrap();
    assert_eq!(
        text(&home.run(&["hooks", "status"], "").stdout),
        "1 \"Work\": other copy\n"
    );
    let out = home.run(&["hooks", "install", "--yes"], "");
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let shown = text(&out.stdout);
    assert_eq!(
        shown
            .matches(
                "Replaced the entries that named another copy of easy-personal-rs with this copy.\n"
            )
            .count(),
        1,
        "{shown}"
    );
    assert!(shown.ends_with("done: on\n"), "{shown}");
    let after = std::fs::read_to_string(home.settings()).unwrap();
    assert!(!after.contains("other/easy-personal-rs"), "{after}");
    assert_eq!(json_of(&home.settings())["model"], "opus");

    std::fs::write(home.settings(), planted.to_string()).unwrap();
    let out = home.run(&["hooks", "remove", "--yes"], "");
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(json_of(&home.settings()), json!({"model": "opus"}));
    assert!(!text(&out.stdout).contains("Replaced"));
}

/// F2: with the app running and the bridge on but no page connected (no `GET /api/live` stream), a hook is handed back at
/// once: exit 0, empty stdout, no waiting; and a hook of a session the live list does not show is the same (after a
/// one-second grace for the tail to read a line written just before the hook ran).
#[test]
fn a_hook_is_handed_back_when_no_page_is_connected_or_its_session_is_not_listed() {
    let home = Home::new("nopage");
    open_call(&home, "sess-a");
    assert_eq!(
        home.run(&["hooks", "install", "--yes"], "").status.code(),
        Some(0)
    );
    let mut serve = Serve::start(&home);
    wait_for("the socket", || home.0.join("answer.sock").exists());
    let began = Instant::now();
    let out = home.run(&["hook", "ask"], &hook_input("sess-a"));
    assert_eq!((out.status.code(), out.stdout.len()), (Some(0), 0));
    assert!(
        began.elapsed() < Duration::from_secs(5),
        "not held: {:?}",
        began.elapsed()
    );

    let mut live = serve.watch();
    let began = Instant::now();
    let out = home.run(&["hook", "ask"], &hook_input("sess-not-listed"));
    assert_eq!((out.status.code(), out.stdout.len()), (Some(0), 0));
    assert!(
        began.elapsed() < Duration::from_secs(5),
        "{:?}",
        began.elapsed()
    );
    // and a listed session is still held while the page is connected
    let hook = spawn_hook(&home, &hook_input("sess-a"));
    live.pending(1);
    let (code, _, _) = serve.sigint();
    assert_eq!(code, Some(0));
    assert_eq!(hook.wait_with_output().unwrap().stdout.len(), 0);
}

/// F4 (d), F4 (c): `PUT /api/hooks` over a settings file that is not JSON is a 409 and the file is byte-identical (no backup
/// either); `GET` and `PUT` say whether the answer socket is open.
#[test]
fn put_hooks_over_a_non_json_settings_file_is_a_409_and_leaves_it_byte_identical() {
    let home = Home::new("putbad");
    let broken = "{\"hooks\": {\"PreToolUse\": [ oops";
    std::fs::write(home.settings(), broken).unwrap();
    let mut serve = Serve::start(&home);
    let (status, body) = serve.request("GET", "/api/hooks", None, true);
    assert_eq!(status, 200);
    assert_eq!(body["listening"], false, "off: no socket");
    assert_eq!(body["accounts"][0]["status"], "unreadable");
    for on in [true, false] {
        let put = json!({"account": "1", "on": on, "confirmed": true});
        let (status, body) = serve.request("PUT", "/api/hooks", Some(&put), true);
        assert_eq!(status, 409, "on={on}: {body}");
        assert!(
            body.to_string().contains("not a JSON object"),
            "a fixed message: {body}"
        );
        assert!(!body.to_string().contains(home.0.to_str().unwrap()));
        assert_eq!(std::fs::read_to_string(home.settings()).unwrap(), broken);
        assert!(!home.0.join("backups").exists());
    }
    assert!(
        !home.0.join("answer.sock").exists(),
        "a refused turn-on opens no socket"
    );
    let (code, _, err) = serve.sigint();
    assert_eq!(code, Some(0), "{err}");
}
