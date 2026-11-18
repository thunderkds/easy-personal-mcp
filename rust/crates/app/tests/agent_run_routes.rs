//! T091: the agent run routes over the real `serve` binary, with a fake `claude` on `PATH` (never the real one) and temp
//! folders for `HOME`, `EASY_PERSONAL_HOME` and `EASY_PERSONAL_AGENT_ROOT`: `--help` says `serve` is not read-only for
//! this feature; Ctrl-C and a kill of the server during a run leave no `claude` process; ignoring the agent run account
//! through the account routes turns the setting off and stops a going run. The in-process tests of every route and
//! body rule are in `src/agent_routes.rs`.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

struct Server {
    child: Child,
    _stdout: BufReader<ChildStdout>,
    port: u16,
    token: String,
    home: PathBuf,
}

/// The fake `claude`: it writes its pid and its child's, then waits for that child (so the group has two processes).
const FAKE: &str = "#!/bin/sh\nL=\"$EASY_PERSONAL_HOME/log\"\nmkdir -p \"$L\"\necho $$ > \"$L/pid\"\nsleep 60 &\necho $! > \"$L/grandchild\"\nwait\n";

impl Server {
    fn start() -> Server {
        static N: AtomicU32 = AtomicU32::new(0);
        let home = std::env::temp_dir().join(format!(
            "easy-personal-rs-t091-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let root = home.join("agent-root");
        std::fs::create_dir_all(root.join(".claude-work")).unwrap();
        std::fs::create_dir_all(home.join("bin")).unwrap();
        let claude = home.join("bin/claude");
        std::fs::write(&claude, FAKE).unwrap();
        std::fs::set_permissions(&claude, std::fs::Permissions::from_mode(0o755)).unwrap();
        let store = home.join("store.sqlite");
        drop(easy_personal_core::writer::open_for_write(&store).unwrap());
        let folder = root.join(".claude-work");
        rusqlite::Connection::open(&store)
            .unwrap()
            .execute(
                "INSERT INTO agent_accounts (id, provider, folder, label, status, config_dir_env) \
                 VALUES (1, 'claude', ?1, 'work', 'confirmed', ?1)",
                [folder.to_str().unwrap()],
            )
            .unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
            .args(["serve", "--no-ingest"])
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", home.join("bin").display()),
            )
            .env("HOME", &home)
            .env("EASY_PERSONAL_HOME", &home)
            .env("XDG_CONFIG_HOME", home.join("xdg"))
            .env("EASY_PERSONAL_AGENT_ROOT", &root)
            .env_remove("CLAUDE_CONFIG_DIR")
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

    /// `method path` with a good Host and token and a JSON body; the status and the parsed body.
    fn call(&self, method: &str, path: &str, body: &str) -> (u16, Value) {
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nx-dashboard-token: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            self.port,
            self.token,
            body.len()
        );
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut bytes = Vec::new();
        let _ = stream.read_to_end(&mut bytes);
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((text.as_str(), ""));
        let status = head
            .split(' ')
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| panic!("no status line in {text:?}"));
        (status, serde_json::from_str(body).unwrap_or(Value::Null))
    }

    fn log(&self, name: &str) -> String {
        std::fs::read_to_string(self.home.join("log").join(name)).unwrap_or_default()
    }

    /// Starts the run with the account chosen and waits until the fake's group is up.
    fn start_run(&self) {
        let (status, body) = self.call(
            "PUT",
            "/api/agent-run/setting",
            r#"{"account":"1","confirmed":true}"#,
        );
        assert_eq!(status, 200, "{body}");
        let (status, body) = self.call("POST", "/api/agent-run", r#"{"job":"investigate"}"#);
        assert_eq!(status, 202, "{body}");
        assert_eq!(body["running"], true);
        let deadline = Instant::now() + Duration::from_secs(20);
        while self.log("grandchild").trim().is_empty() {
            assert!(Instant::now() < deadline, "the fake never started");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn signal(&self, name: &str) {
        let status = Command::new("kill")
            .arg(format!("-{name}"))
            .arg(self.child.id().to_string())
            .status()
            .unwrap();
        assert!(status.success());
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        for name in ["pid", "grandchild"] {
            let pid = self.log(name);
            if !pid.trim().is_empty() {
                let _ = Command::new("kill")
                    .args(["-KILL", pid.trim()])
                    .stderr(Stdio::null())
                    .status();
            }
        }
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

/// Gone (or a zombie nobody reaped): it runs no more. Waits up to 10 s for that.
fn gone(pid: &str) -> bool {
    let stat = format!("/proc/{}/stat", pid.trim());
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let live = std::fs::read_to_string(&stat)
            .ok()
            .and_then(|s| {
                s.rsplit(')')
                    .next()
                    .map(|r| !r.trim_start().starts_with('Z'))
            })
            .unwrap_or(false);
        if !live {
            return true;
        }
        if Instant::now() > deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn exists(path: &Path) -> bool {
    path.exists()
}

#[test]
fn serve_help_says_it_is_not_read_only_for_the_agent_run() {
    let out = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
        .args(["serve", "--help"])
        .output()
        .unwrap();
    let help = String::from_utf8(out.stdout).unwrap();
    assert!(
        help.contains("Not read-only for two features") && help.contains("/api/agent-run"),
        "{help}"
    );
}

#[test]
fn ctrl_c_on_the_server_during_a_run_leaves_no_claude_process() {
    let mut server = Server::start();
    server.start_run();
    let (pid, grandchild) = (server.log("pid"), server.log("grandchild"));
    assert_eq!(server.call("GET", "/api/agent-run", "").1["running"], true);
    server.signal("INT");
    let status = server.child.wait().unwrap();
    assert!(status.success(), "{status}");
    assert!(gone(&pid), "claude still runs");
    assert!(gone(&grandchild), "its child still runs");
}

#[test]
fn a_kill_of_the_server_during_a_run_leaves_no_claude_process() {
    let mut server = Server::start();
    server.start_run();
    let (pid, grandchild) = (server.log("pid"), server.log("grandchild"));
    server.child.kill().unwrap();
    let _ = server.child.wait();
    assert!(
        gone(&pid),
        "claude still runs after a SIGKILL of the server"
    );
    // the child of `claude` is in the same group; PDEATHSIG kills `claude` only, the group is cleaned by the next run
    let _ = Command::new("kill")
        .args(["-KILL", grandchild.trim()])
        .status();
}

#[test]
fn ignoring_the_agent_run_account_through_the_account_routes_turns_it_off_and_stops_the_run() {
    let server = Server::start();
    server.start_run();
    assert!(exists(&server.home.join("agent-run.on")));
    let (status, body) = server.call("POST", "/api/accounts/1/ignore", "{}");
    assert_eq!(status, 200, "{body}");
    assert!(
        !exists(&server.home.join("agent-run.on")),
        "the flag is gone"
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    let state = loop {
        let (status, state) = server.call("GET", "/api/agent-run", "");
        assert_eq!(status, 200);
        if state["running"] == false {
            break state;
        }
        assert!(
            Instant::now() < deadline,
            "the run was not stopped: {state}"
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(state["enabled"], false);
    assert_eq!(state["account"], Value::Null);
    assert_eq!(state["last"]["outcome"], "stopped");
    assert_eq!(
        server
            .call("POST", "/api/agent-run", r#"{"job":"investigate"}"#)
            .0,
        403
    );
    assert!(gone(&server.log("pid")) && gone(&server.log("grandchild")));
    // the same account confirmed again does not turn the setting back on by itself
    let (status, _) = server.call(
        "POST",
        "/api/accounts/1/confirm",
        &json!({"label": "work"}).to_string(),
    );
    assert_eq!(status, 200);
    assert_eq!(server.call("GET", "/api/agent-run", "").1["enabled"], false);
}
