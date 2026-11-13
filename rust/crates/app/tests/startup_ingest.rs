//! T051: `serve` ingests before it starts, as Python's `dashboard` does. Every test runs the real binary with an
//! environment built from nothing: `HOME`, `EASY_PERSONAL_HOME` and `EASY_PERSONAL_AGENT_ROOT` are all inside one
//! temp folder, so the owner's `~/.easy-personal` and `~/.claude*` are never reached. The transcript is a fixture
//! of the Python app's tests.

use rusqlite::{Connection, OpenFlags};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

const PROGRESS: &str = "ingesting before start (waits if another ingest runs)...";
const FAILED: &str = "ingest failed; starting anyway";

/// A closed port on 127.0.0.1.
fn dead_url() -> String {
    let port = std::net::TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    format!("http://127.0.0.1:{port}/mcp")
}

struct Case {
    dir: PathBuf,
}

impl Case {
    fn new(with_transcript: bool) -> Case {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t051-start-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(dir.join("home")).unwrap();
        fs::create_dir_all(dir.join("nowhere")).unwrap();
        if with_transcript {
            let projects = dir.join("agent/.claude/projects/p");
            fs::create_dir_all(&projects).unwrap();
            let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../tests/fixtures/claude_code/2.1.295/session.jsonl");
            fs::copy(fixture, projects.join("s.jsonl")).unwrap();
        } else {
            fs::create_dir_all(dir.join("agent")).unwrap();
        }
        Case { dir }
    }

    fn store(&self) -> PathBuf {
        self.dir.join("home/store.sqlite")
    }

    fn command(&self, args: &[&str], lock_timeout: &str) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"));
        c.args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", self.dir.join("nowhere"))
            .env("EASY_PERSONAL_HOME", self.dir.join("home"))
            .env("EASY_PERSONAL_AGENT_ROOT", self.dir.join("agent"))
            .env("EASY_PERSONAL_LOCK_TIMEOUT", lock_timeout)
            .env("TZ", "UTC")
            .env("EASY_PERSONAL_CONTEXT_URL", dead_url()) // never a real easy-context service
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        c
    }

    /// `events` rows in the store file; 0 when there is no file. Opens read-only.
    fn events(&self) -> i64 {
        match Connection::open_with_flags(self.store(), OpenFlags::SQLITE_OPEN_READ_ONLY) {
            Ok(conn) => conn
                .query_row("SELECT count(*) FROM events", [], |r| r.get(0))
                .unwrap_or(0),
            Err(_) => 0,
        }
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// A started `serve`: the two stdout lines are read, then the test looks at the store, then `finish` stops it.
struct Serve {
    child: Child,
    stdout: BufReader<std::process::ChildStdout>,
    dashboard_line: String,
}

impl Serve {
    fn start(case: &Case, args: &[&str], lock_timeout: &str) -> Serve {
        let mut all = vec!["serve"];
        all.extend_from_slice(args);
        let mut child = case.command(&all, lock_timeout).spawn().unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        Serve {
            child,
            stdout,
            dashboard_line: line,
        }
    }

    /// SIGINT (Ctrl-C), then the exit code and everything on stderr.
    fn finish(mut self) -> (Option<i32>, String) {
        // The server installs its Ctrl-C handler just after it prints the two lines: give it a moment.
        sleep(Duration::from_millis(300));
        let pid = self.child.id().to_string();
        let kill = Command::new("kill").args(["-INT", &pid]).status().unwrap();
        assert!(kill.success());
        let mut rest = String::new();
        let _ = self.stdout.read_to_string(&mut rest);
        let status = self.child.wait().unwrap();
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        (status.code(), stderr)
    }
}

#[test]
fn serve_ingests_before_the_dashboard_line_is_printed() {
    let case = Case::new(true);
    let serve = Serve::start(&case, &[], "5");
    assert!(
        serve
            .dashboard_line
            .starts_with("dashboard: http://127.0.0.1:"),
        "{}",
        serve.dashboard_line
    );
    // The events are in the store at the moment the dashboard line appears (criterion 1, success criterion 2).
    assert!(case.events() > 0, "the transcript was not ingested first");
    let (code, stderr) = serve.finish();
    assert_eq!(code, Some(0));
    assert_eq!(stderr.lines().next(), Some(PROGRESS), "{stderr}");
    // One progress line; no path and no row value (NFR-002).
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(!stderr.contains(case.dir.to_str().unwrap()), "{stderr}");
}

#[test]
fn no_ingest_skips_the_ingest() {
    let case = Case::new(true);
    let serve = Serve::start(&case, &["--no-ingest"], "5");
    assert!(
        serve
            .dashboard_line
            .starts_with("dashboard: http://127.0.0.1:")
    );
    assert_eq!(case.events(), 0);
    assert!(!case.store().exists(), "--no-ingest must not make a store");
    let (code, stderr) = serve.finish();
    assert_eq!(code, Some(0));
    assert_eq!(stderr, "");
}

#[test]
fn a_run_with_no_store_and_no_transcripts_starts_empty() {
    let case = Case::new(false);
    let serve = Serve::start(&case, &[], "5");
    assert!(
        serve
            .dashboard_line
            .starts_with("dashboard: http://127.0.0.1:")
    );
    assert_eq!(case.events(), 0);
    let (code, stderr) = serve.finish();
    assert_eq!(code, Some(0));
    assert_eq!(stderr, format!("{PROGRESS}\n"));
}

#[test]
fn a_failed_ingest_is_one_fixed_line_and_the_server_still_starts() {
    // A store that is newer than the one this build knows: the ingest refuses it.
    let case = Case::new(true);
    let conn = Connection::open(case.store()).unwrap();
    conn.pragma_update(None, "user_version", 99).unwrap();
    drop(conn);
    let serve = Serve::start(&case, &[], "5");
    assert!(
        serve
            .dashboard_line
            .starts_with("dashboard: http://127.0.0.1:")
    );
    let (code, stderr) = serve.finish();
    assert_eq!(code, Some(0));
    assert_eq!(stderr, format!("{PROGRESS}\n{FAILED}\n"), "{stderr}");
}

#[test]
fn a_malformed_lock_timeout_is_the_same_fixed_line() {
    let case = Case::new(true);
    let serve = Serve::start(&case, &[], "abc");
    assert!(
        serve
            .dashboard_line
            .starts_with("dashboard: http://127.0.0.1:")
    );
    let (code, stderr) = serve.finish();
    assert_eq!(code, Some(0));
    assert_eq!(stderr, format!("{PROGRESS}\n{FAILED}\n"), "{stderr}");
}

#[test]
fn a_lock_that_never_frees_is_a_bounded_wait_then_the_server_starts() {
    let case = Case::new(true);
    let lock = File::options()
        .create(true)
        .append(true)
        .open(case.dir.join("home/ingest.lock"))
        .unwrap();
    lock.lock().unwrap();
    let began = Instant::now();
    let serve = Serve::start(&case, &[], "0.6");
    let waited = began.elapsed();
    assert!(
        serve
            .dashboard_line
            .starts_with("dashboard: http://127.0.0.1:")
    );
    assert!(
        waited >= Duration::from_millis(500),
        "did not wait: {waited:?}"
    );
    assert!(
        waited < Duration::from_secs(20),
        "waited too long: {waited:?}"
    );
    assert_eq!(case.events(), 0, "ingest ran without the lock");
    let (code, stderr) = serve.finish();
    assert_eq!(code, Some(0));
    assert_eq!(stderr, format!("{PROGRESS}\n{FAILED}\n"), "{stderr}");
    drop(lock);
}

#[test]
fn a_lock_that_frees_in_time_lets_the_ingest_run() {
    let case = Case::new(true);
    let lock_path = case.dir.join("home/ingest.lock");
    let lock = File::options()
        .create(true)
        .append(true)
        .open(&lock_path)
        .unwrap();
    lock.lock().unwrap();
    let holder = std::thread::spawn(move || {
        sleep(Duration::from_millis(700));
        drop(lock);
    });
    let serve = Serve::start(&case, &[], "20");
    assert!(
        serve
            .dashboard_line
            .starts_with("dashboard: http://127.0.0.1:")
    );
    assert!(case.events() > 0, "the ingest did not run after the wait");
    holder.join().unwrap();
    let (code, stderr) = serve.finish();
    assert_eq!(code, Some(0));
    assert_eq!(stderr, format!("{PROGRESS}\n"));
}

#[test]
fn the_help_documents_the_flag() {
    let case = Case::new(false);
    let out = case.command(&["serve", "--help"], "1").output().unwrap();
    let help = String::from_utf8(out.stdout).unwrap();
    assert!(help.contains("--no-ingest"), "{help}");
}
