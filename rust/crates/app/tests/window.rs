//! Real windows (T023). These open a Tauri window, so they need a display (WSLg or X11) and the Tauri system packages
//! (`rust/docs/spike-T001.md`): they are ignored by default. Run them with
//! `cargo test --features window --test window -- --ignored --test-threads=1`.
//! The page is a fixture (no dashboard build): `index.html` plus an `app.js` served as a file, because the dashboard CSP
//! blocks inline scripts. The self-test uses the committed `tests/fixtures/self-test` (it reads the token from
//! `location.hash` and calls `/api/version` with the token header); the others make theirs in a temp folder.
//! `HOME` and `EASY_PERSONAL_HOME` point into a temp folder.
#![cfg(feature = "window")]

use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t023-win-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    /// A page folder: `index.html` loads `/app.js`, which is `script`.
    fn page(&self, body: &str, script: &str) -> PathBuf {
        let dir = self.0.join("page");
        fs::create_dir_all(&dir).unwrap();
        let html = format!(
            "<!doctype html><meta charset=\"utf-8\"><title>fixture</title>{body}<script src=\"/app.js\"></script>"
        );
        fs::write(dir.join("index.html"), html).unwrap();
        fs::write(dir.join("app.js"), script).unwrap();
        dir
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct App {
    child: Child,
    stderr: mpsc::Receiver<String>,
    stdout: mpsc::Receiver<String>,
}

impl App {
    fn start(home: &TempDir, page: &Path, args: &[&str]) -> App {
        let mut child = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
            .arg("app")
            .arg("--no-ingest")
            .args(args)
            .arg("--static-dir")
            .arg(page)
            .env("HOME", &home.0)
            .env("EASY_PERSONAL_HOME", &home.0)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = lines(child.stdout.take().unwrap());
        let stderr = lines(child.stderr.take().unwrap());
        App {
            child,
            stderr,
            stdout,
        }
    }

    /// Waits for the process to end; kills it and fails after `limit`.
    fn wait(&mut self, limit: Duration) -> i32 {
        let end = Instant::now() + limit;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status.code().unwrap_or(-1);
            }
            if Instant::now() > end {
                let _ = self.child.kill();
                panic!("the app did not end within {limit:?}");
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Our own log lines (the graphics stack writes others), up to now.
    fn log(&self) -> Vec<String> {
        thread::sleep(Duration::from_millis(200));
        self.stderr
            .try_iter()
            .filter(|l| l.starts_with("easy-personal-rs:"))
            .collect()
    }

    fn stdout(&self) -> Vec<String> {
        self.stdout.try_iter().collect()
    }

    /// Blocks until a stderr line contains `needle`.
    fn wait_for_log(&self, needle: &str, limit: Duration) {
        let end = Instant::now() + limit;
        while Instant::now() < end {
            if let Ok(line) = self.stderr.recv_timeout(Duration::from_millis(100))
                && line.contains(needle)
            {
                return;
            }
        }
        panic!("no stderr line with {needle:?} within {limit:?}");
    }
}

fn lines(source: impl Read + Send + 'static) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(source).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                return;
            }
        }
    });
    rx
}

/// Every process below `pid` (children, grandchildren...), read from `/proc`.
fn descendants(pid: u32) -> Vec<u32> {
    let mut parent_of = Vec::new();
    for entry in fs::read_dir("/proc").unwrap().flatten() {
        let Ok(id) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let Ok(stat) = fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        let after_name = stat.rsplit_once(')').map(|(_, rest)| rest.to_string());
        if let Some(ppid) =
            after_name.and_then(|r| r.split_whitespace().nth(1)?.parse::<u32>().ok())
        {
            parent_of.push((id, ppid));
        }
    }
    let mut found = vec![pid];
    let mut i = 0;
    while i < found.len() {
        let current = found[i];
        found.extend(
            parent_of
                .iter()
                .filter(|(_, p)| *p == current)
                .map(|(c, _)| *c),
        );
        i += 1;
    }
    found.remove(0);
    found
}

#[test]
#[ignore = "needs a display (WSLg or X11) and the Tauri system packages"]
fn the_page_makes_an_authenticated_request_and_the_self_test_passes() {
    let home = TempDir::new();
    // The committed fixture of the guide's verification command (`--static-dir crates/app/tests/fixtures/self-test`).
    let page = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/self-test");
    let mut app = App::start(&home, &page, &["--self-test"]);
    let code = app.wait(Duration::from_secs(30));
    assert_eq!(code, 0);
    assert_eq!(app.stdout(), ["self-test: ok"]);
    let log = app.log();
    assert_eq!(log.len(), 2, "{log:?}");
    assert!(log[0].starts_with("easy-personal-rs: window open on http://127.0.0.1:"));
    assert!(log[0].ends_with('/'), "no fragment in the log: {}", log[0]);
    assert_eq!(log[1], "easy-personal-rs: window closed");
}

#[test]
#[ignore = "needs a display (WSLg or X11) and the Tauri system packages"]
fn without_an_authenticated_request_the_self_test_exits_3_after_20_seconds() {
    let home = TempDir::new();
    // The page never calls the API; a call without the token header is refused and does not count.
    let page = home.page("<h1>idle</h1>", "fetch('/api/version');");
    let mut app = App::start(&home, &page, &["--self-test"]);
    let began = Instant::now();
    let code = app.wait(Duration::from_secs(40));
    assert_eq!(code, 3);
    assert!(
        began.elapsed() >= Duration::from_secs(19),
        "{:?}",
        began.elapsed()
    );
    assert!(app.stdout().is_empty());
    let log = app.log();
    assert!(
        log.contains(
            &"easy-personal-rs: self-test: no authenticated request within 20 s".to_string()
        ),
        "{log:?}"
    );
}

/// The dashboard has plain external links in its sidebar. A click, `window.open` and a script navigation must all do
/// nothing: the page stays, and nothing connects to the link's target (a listener on another loopback port is the target).
#[test]
#[ignore = "needs a display (WSLg or X11) and the Tauri system packages"]
fn external_links_and_new_windows_do_nothing() {
    let other = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let other_port = other.local_addr().unwrap().port();
    other.set_nonblocking(true).unwrap();
    let connections = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&connections);
    let watcher = thread::spawn(move || {
        let end = Instant::now() + Duration::from_secs(25);
        while Instant::now() < end {
            if other.accept().is_ok() {
                counted.fetch_add(1, Ordering::SeqCst);
            }
            thread::sleep(Duration::from_millis(10));
        }
    });
    let home = TempDir::new();
    let target = format!("http://127.0.0.1:{other_port}/x?secret=1#frag");
    let body = format!(
        "<a id=\"a\" href=\"{target}\">other port</a><a id=\"b\" href=\"{target}\" target=\"_blank\">blank</a>\
         <a id=\"c\" href=\"http://localhost:{other_port}/\">localhost</a><a id=\"d\" href=\"https://example.invalid/\">remote</a>"
    );
    let script = format!(
        "(async () => {{ const token = location.hash.slice(1); \
           for (const id of ['a', 'b', 'c', 'd']) document.getElementById(id).click(); \
           window.open('{target}'); window.location.assign('{target}'); \
           await new Promise(r => setTimeout(r, 2000)); \
           await fetch('/api/version', {{ headers: {{ 'x-dashboard-token': token }} }}); }})();"
    );
    let page = home.page(&body, &script);
    let mut app = App::start(&home, &page, &["--self-test"]);
    // The last request only happens if this same document is still shown after every attempt to leave it.
    assert_eq!(app.wait(Duration::from_secs(30)), 0);
    assert_eq!(app.stdout(), ["self-test: ok"]);
    let log = app.log();
    assert!(
        log.iter()
            .any(|l| l == "easy-personal-rs: blocked navigation to host \"127.0.0.1\""),
        "{log:?}"
    );
    assert!(
        !log.iter()
            .any(|l| l.contains("secret") || l.contains("frag") || l.contains("x?")),
        "{log:?}"
    );
    drop(watcher);
    assert_eq!(
        connections.load(Ordering::SeqCst),
        0,
        "something connected to the link target"
    );
}

#[test]
#[ignore = "needs a display (WSLg or X11) and the Tauri system packages"]
fn sigint_and_sigterm_close_the_window_exit_0_and_leave_no_process() {
    for signal in ["-INT", "-TERM"] {
        let home = TempDir::new();
        let page = home.page("<h1>idle</h1>", "");
        let mut app = App::start(&home, &page, &[]);
        app.wait_for_log("window open on", Duration::from_secs(30));
        thread::sleep(Duration::from_millis(500));
        let pid = app.child.id();
        let below = descendants(pid);
        assert!(!below.is_empty(), "the web view runs in child processes");
        let sent = Command::new("kill")
            .args([signal, &pid.to_string()])
            .status()
            .unwrap();
        assert!(sent.success());
        assert_eq!(app.wait(Duration::from_secs(10)), 0, "{signal}");
        assert!(app.stdout().is_empty());
        assert!(
            app.log()
                .contains(&"easy-personal-rs: stopping on a signal".to_string())
        );
        thread::sleep(Duration::from_millis(500));
        let alive: Vec<u32> = below
            .into_iter()
            .filter(|p| {
                fs::read_to_string(format!("/proc/{p}/stat"))
                    .map(|s| {
                        !s.rsplit_once(')')
                            .is_some_and(|(_, r)| r.trim_start().starts_with('Z'))
                    })
                    .unwrap_or(false)
            })
            .collect();
        assert!(alive.is_empty(), "left behind after {signal}: {alive:?}");
    }
}
