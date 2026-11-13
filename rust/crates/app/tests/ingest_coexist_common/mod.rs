//! T031 helpers: guarded child processes (the real Rust binary and the real Python app), a barrier start, kill and
//! signal by process group, the one store check (integrity, duplicate keys, the four tables), a seeded large
//! transcript set, and small `serve` / `mcp` clients. Included by `ingest_coexist.rs`; the T035 oracle helpers
//! (`crate::common`) are included there by path.
//!
//! No assertion message here carries a path, SQL or a row value (only table names, counts and labels).
#![allow(dead_code)] // each test uses a part

use crate::common::{Case, repo};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::thread::sleep;
use std::time::{Duration, Instant};

pub const BUSY: &str = "another ingest is running; try again later\n";
pub const LOCK_TIMEOUT: &str = "EASY_PERSONAL_LOCK_TIMEOUT";
/// Every child is killed (with its process group) if it runs longer than this.
pub const LIMIT: Duration = Duration::from_secs(120);
pub const TABLES: [&str; 4] = ["events", "tool_signals", "source_files", "agent_accounts"];
const ENTRY: &str = "import sys; from easy_personal.surfaces.cli import main; sys.exit(main())";
/// Waits at `<dir>/go`, then (optionally) sleeps, then `exec`s the child: the PID stays the child's.
const BARRIER: &str = r#"d=$1; w=$2; shift 2; : > "$d/ready.$$"; while [ ! -e "$d/go" ]; do :; done; [ "$w" = 0.000 ] || sleep "$w"; exec "$@""#;

/// Children whose process group was checked empty after they were reaped (printed as evidence).
pub static REAPED: AtomicU32 = AtomicU32::new(0);

pub fn python() -> PathBuf {
    let python = repo().join("rust/parity/.venv/bin/python");
    assert!(
        python.exists(),
        "no parity venv at rust/parity/.venv (see rust/parity/README.md)"
    );
    python
}

/// The committed fixtures name `/work/demo-app` and the large set `/nonexistent-t031/...` as their `cwd`: both must be
/// gone, or project resolution (and so the tables) would depend on this machine.
pub fn assert_cwds_are_gone() {
    for p in ["/work/demo-app", "/nonexistent-t031"] {
        assert!(
            !Path::new(p).exists(),
            "a fixture cwd exists on this machine"
        );
    }
}

/// One child to start: program, arguments, an environment built from nothing, the case's work root as cwd.
#[derive(Clone)]
pub struct Spec {
    program: OsString,
    args: Vec<OsString>,
    env: BTreeMap<String, OsString>,
    cwd: PathBuf,
    stdin: bool,
}

impl Spec {
    fn base(case: &Case, program: OsString, args: Vec<OsString>) -> Spec {
        let mut env = BTreeMap::new();
        env.insert(
            "PATH".into(),
            std::env::var_os("PATH").unwrap_or_else(|| "/usr/bin:/bin".into()),
        );
        env.insert("HOME".into(), case.nowhere.clone().into());
        env.insert("EASY_PERSONAL_HOME".into(), case.home.clone().into());
        env.insert(
            "EASY_PERSONAL_AGENT_ROOT".into(),
            case.agents.clone().into(),
        );
        env.insert("GIT_CONFIG_NOSYSTEM".into(), "1".into());
        env.insert("GIT_CONFIG_GLOBAL".into(), "/dev/null".into());
        if let Some(tz) = std::env::var_os("TZ") {
            env.insert("TZ".into(), tz); // the TZ runs of criterion 9 reach the children
        }
        Spec {
            program,
            args,
            env,
            cwd: case.work.clone(),
            stdin: false,
        }
    }

    /// `easy-personal-rs <args>`.
    pub fn rust(case: &Case, args: &[&str]) -> Spec {
        Spec::base(
            case,
            env!("CARGO_BIN_EXE_easy-personal-rs").into(),
            args.iter().map(OsString::from).collect(),
        )
    }

    /// `python -P -c <code> <args>` from the parity venv, `PYTHONPATH=src`, the oracle's fixed environment.
    pub fn python_code(case: &Case, code: &str, args: &[&OsStr]) -> Spec {
        let mut all: Vec<OsString> = vec!["-P".into(), "-c".into(), code.into()];
        all.extend(args.iter().map(|a| a.to_os_string()));
        let mut s = Spec::base(case, python().into(), all);
        for (k, v) in [
            (
                "XDG_CONFIG_HOME",
                case.nowhere.join(".config").into_os_string(),
            ),
            (
                "EASY_PERSONAL_VERIFIER_REPORTS",
                case.root.join("verifier-none").into_os_string(),
            ),
            (
                "EASY_MEMORY_HOME",
                case.root.join("memory-none").into_os_string(),
            ),
            ("EASY_PERSONAL_CONTEXT_URL", "http://127.0.0.1:9/mcp".into()),
            ("LC_ALL", "C.UTF-8".into()),
            ("LANG", "C.UTF-8".into()),
            ("PYTHONHASHSEED", "0".into()),
            ("PYTHONPATH", repo().join("src").into_os_string()),
            ("PYTHONDONTWRITEBYTECODE", "1".into()),
        ] {
            s.env.insert(k.into(), v);
        }
        s.env.entry("TZ".into()).or_insert_with(|| "UTC".into());
        s
    }

    /// `easy-personal ingest <args>` (the CLI entry the `easy-personal` script runs).
    pub fn python(case: &Case, args: &[&str]) -> Spec {
        let mut all: Vec<&OsStr> = vec![OsStr::new("ingest")];
        all.extend(args.iter().map(OsStr::new));
        Spec::python_code(case, ENTRY, &all)
    }

    pub fn env(mut self, key: &str, value: impl Into<OsString>) -> Spec {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn without(mut self, key: &str) -> Spec {
        self.env.remove(key);
        self
    }

    pub fn with_stdin(mut self) -> Spec {
        self.stdin = true;
        self
    }

    /// The guard: no child starts unless `EASY_PERSONAL_HOME`, `EASY_PERSONAL_AGENT_ROOT` and `HOME` are set, sit in
    /// the temp folder, and are not inside the real home.
    fn check(&self) {
        let tmp = fs::canonicalize(std::env::temp_dir()).unwrap();
        let real = std::env::var_os("HOME").map(PathBuf::from);
        for key in ["EASY_PERSONAL_HOME", "EASY_PERSONAL_AGENT_ROOT", "HOME"] {
            let value = self
                .env
                .get(key)
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| panic!("refusing to run: {key} is not set"));
            let path = Path::new(value);
            assert!(
                path.starts_with(&tmp),
                "refusing to run: {key} is outside the temp folder"
            );
            if let Some(real) = &real {
                assert!(
                    real.parent().is_none() || !path.starts_with(real),
                    "refusing to run: {key} is inside the real home"
                );
            }
        }
    }

    fn command(&self, barrier: Option<(&Path, Duration)>) -> Command {
        self.check();
        let mut c = match barrier {
            None => {
                let mut c = Command::new(&self.program);
                c.args(&self.args);
                c
            }
            Some((dir, delay)) => {
                let mut c = Command::new("sh");
                c.arg("-c")
                    .arg(BARRIER)
                    .arg("sh")
                    .arg(dir)
                    .arg(format!("{:.3}", delay.as_secs_f64()))
                    .arg(&self.program)
                    .args(&self.args);
                c
            }
        };
        c.env_clear()
            .envs(&self.env)
            .current_dir(&self.cwd)
            .stdin(if self.stdin {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0); // its own group: a kill reaches any `git` it started
        c
    }

    pub fn spawn(&self) -> Proc {
        Proc::new(self.command(None))
    }

    /// Started now, but waits at `<dir>/go` (see [`race`]), then sleeps `delay`.
    pub fn spawn_at(&self, dir: &Path, delay: Duration) -> Proc {
        Proc::new(self.command(Some((dir, delay))))
    }

    /// Start, wait (killed after [`LIMIT`]), collect.
    pub fn run(&self) -> Out {
        self.spawn().wait(LIMIT)
    }
}

pub struct Out {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub elapsed: Duration,
}

impl Out {
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.stdout).expect("stdout is one JSON value")
    }

    pub fn new_events(&self) -> i64 {
        self.json()["new_events"].as_i64().expect("new_events")
    }

    /// One line, our prefix, no path of the temp folder, no panic: the shape of every clean error.
    pub fn assert_clean_error(&self, code: i32, what: &str) {
        assert_eq!(self.code, Some(code), "{what}: exit code");
        assert_eq!(self.stdout, "", "{what}: stdout");
        assert_eq!(self.stderr.lines().count(), 1, "{what}: one stderr line");
        assert!(
            self.stderr.starts_with("easy-personal-rs: "),
            "{what}: prefix"
        );
        assert!(
            !self.stderr.contains("/tmp"),
            "{what}: a path in the message"
        );
        assert!(!self.stderr.contains("panicked"), "{what}: a panic");
    }
}

/// A started child in its own process group; killed with its group when dropped unreaped.
pub struct Proc {
    pub child: std::process::Child,
    pub pid: u32,
    pub started: Instant,
    done: bool,
}

impl Proc {
    fn new(mut command: Command) -> Proc {
        let child = command.spawn().expect("spawn a child");
        Proc {
            pid: child.id(),
            child,
            started: Instant::now(),
            done: false,
        }
    }

    pub fn running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Polls to a deadline (never a bare sleep): past it, the group is killed and the test fails.
    pub fn wait(mut self, limit: Duration) -> Out {
        let deadline = Instant::now() + limit;
        let status = loop {
            if let Some(s) = self.child.try_wait().unwrap() {
                break s;
            }
            if Instant::now() >= deadline {
                kill_group(self.pid);
                let _ = self.child.wait();
                self.done = true;
                assert_group_gone(self.pid);
                panic!("a child ran past its {limit:?} deadline and was killed");
            }
            sleep(Duration::from_millis(2));
        };
        let elapsed = self.started.elapsed();
        self.done = true;
        let read = |r: Option<&mut dyn Read>| {
            let mut bytes = Vec::new();
            if let Some(r) = r {
                let _ = r.read_to_end(&mut bytes);
            }
            String::from_utf8_lossy(&bytes).into_owned()
        };
        let stdout = read(self.child.stdout.as_mut().map(|s| s as &mut dyn Read));
        let stderr = read(self.child.stderr.as_mut().map(|s| s as &mut dyn Read));
        assert_group_gone(self.pid);
        Out {
            code: status.code(),
            stdout,
            stderr,
            elapsed,
        }
    }

    /// SIGKILL to the whole group, then the child stays to be collected with [`Proc::wait`] (T055: a failing test reads
    /// the child's exit status, stdout and stderr).
    pub fn terminate(&self) {
        kill_group(self.pid);
    }

    /// SIGKILL to the whole group. Returns whether the child had already exited before the kill.
    pub fn kill(mut self) -> bool {
        let exited = matches!(self.child.try_wait(), Ok(Some(_)));
        kill_group(self.pid);
        let _ = self.child.wait();
        self.done = true;
        assert_group_gone(self.pid);
        exited
    }

    /// `kill -s <sig> <pid>` (the child only, not its group).
    pub fn signal(&self, sig: &str) {
        let ok = Command::new("kill")
            .args(["-s", sig, &self.pid.to_string()])
            .status()
            .unwrap()
            .success();
        assert!(ok, "kill -s {sig} failed");
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        if !self.done {
            kill_group(self.pid);
            let _ = self.child.wait();
        }
    }
}

fn kill_group(pgid: u32) {
    let _ = Command::new("kill")
        .args(["-s", "KILL", "--", &format!("-{pgid}")])
        .stderr(Stdio::null())
        .status();
}

/// The PIDs (from `/proc/*/stat`) whose process group is `pgid`.
pub fn group_members(pgid: u32) -> Vec<u32> {
    let mut found = Vec::new();
    let Ok(dir) = fs::read_dir("/proc") else {
        return found;
    };
    for entry in dir.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let Ok(stat) = fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        // after the `(comm)`: state, ppid, pgrp
        let rest = stat.rsplit_once(')').map_or("", |(_, r)| r);
        if rest.split_whitespace().nth(2) == Some(&pgid.to_string()) {
            found.push(pid);
        }
    }
    found
}

/// Nothing of the child's group is left running (checked by PID, polled to a deadline).
pub fn assert_group_gone(pgid: u32) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let left = group_members(pgid);
        if left.is_empty() {
            REAPED.fetch_add(1, Ordering::SeqCst);
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{} process(es) of a child's group still running",
            left.len()
        );
        sleep(Duration::from_millis(10));
    }
}

/// Starts `a` and `b`, waits until both wait at the barrier (each wrote its ready file), opens it, waits for both.
/// `delays` = how long each sleeps after the barrier opens (the 0/5/20/50 ms ladder).
pub fn race(case: &Case, a: &Spec, b: &Spec, delays: (u64, u64), tag: usize) -> (Out, Out) {
    let dir = case.root.join(format!("barrier-{tag}"));
    fs::create_dir_all(&dir).unwrap();
    let pa = a.spawn_at(&dir, Duration::from_millis(delays.0));
    let pb = b.spawn_at(&dir, Duration::from_millis(delays.1));
    let deadline = Instant::now() + Duration::from_secs(30);
    while fs::read_dir(&dir).unwrap().count() < 2 {
        assert!(
            Instant::now() < deadline,
            "the two children never reached the barrier"
        );
        sleep(Duration::from_millis(1));
    }
    fs::write(dir.join("go"), b"").unwrap();
    (pa.wait(LIMIT), pb.wait(LIMIT))
}

/// `PRAGMA integrity_check` is `ok` and no event key `(machine_id, source, source_path, inode, byte_offset)` repeats.
pub fn assert_intact(store: &Path, what: &str) {
    let c = Connection::open(store).unwrap();
    let ok: String = c
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(ok, "ok", "{what}: integrity_check");
    let dups: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM (SELECT 1 FROM events \
             GROUP BY machine_id, source, source_path, inode, byte_offset HAVING COUNT(*) > 1)",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(dups, 0, "{what}: duplicate event keys");
}

/// The one store check: intact, no duplicate key, then the four tables in the goldens' form (`Case::dump`).
pub fn checked_dump(case: &Case, what: &str) -> Value {
    assert_intact(&case.store_path(), what);
    case.dump()
}

/// A golden run's `tables_after` in `Case::dump`'s form (rows sorted the same way).
pub fn golden_tables(run: &Value) -> Value {
    let mut out = serde_json::Map::new();
    for table in TABLES {
        let mut rows = run["tables_after"][table]["rows"]
            .as_array()
            .unwrap()
            .clone();
        if table != "agent_accounts" {
            rows.sort_by_key(Value::to_string);
        }
        out.insert(table.into(), Value::Array(rows));
    }
    Value::Object(out)
}

/// The four tables equal, row for row, every column; one machine id. The message names the table only.
pub fn assert_same(got: &Value, want: &Value, what: &str) {
    for table in TABLES {
        let (g, w) = (
            got[table].as_array().unwrap(),
            want[table].as_array().unwrap(),
        );
        assert_eq!(g.len(), w.len(), "{what}: {table} row count");
        let differ = g.iter().zip(w).filter(|(a, b)| a != b).count();
        assert_eq!(differ, 0, "{what}: {table} rows differ");
    }
    for table in ["events", "tool_signals", "source_files"] {
        assert_eq!(
            got["machine_id_check"][table],
            json!(true),
            "{what}: one machine id"
        );
    }
}

/// After a kill: every `source_files` offset sits at a line start of its file, every stored event belongs to a saved
/// file and lies below its offset, and a saved file's events are exactly the clean run's events below that offset.
/// Returns (files saved, events saved).
pub fn assert_no_half_file(case: &Case, got: &Value, clean: &Value, what: &str) -> (usize, usize) {
    let agents = case.agents.to_str().unwrap();
    let mut saved: HashMap<String, i64> = HashMap::new();
    for row in got["source_files"].as_array().unwrap() {
        let path = row["path"].as_str().unwrap();
        let off = row["byte_offset"].as_i64().unwrap();
        let bytes = fs::read(path.replace("<AGENT_ROOT>", agents)).unwrap();
        let at_line_start =
            off == 0 || (off as usize <= bytes.len() && bytes[off as usize - 1] == b'\n');
        assert!(at_line_start, "{what}: an offset not at a line start");
        saved.insert(path.to_owned(), off);
    }
    let events = got["events"].as_array().unwrap();
    for e in events {
        let below = saved
            .get(e["source_path"].as_str().unwrap())
            .is_some_and(|off| e["byte_offset"].as_i64().unwrap() < *off);
        assert!(below, "{what}: an event outside its file's saved offset");
    }
    let clean_events = clean["events"].as_array().unwrap();
    for (path, off) in &saved {
        let of = |rows: &'_ [Value]| -> Vec<Value> {
            rows.iter()
                .filter(|e| {
                    e["source_path"] == json!(path) && e["byte_offset"].as_i64().unwrap() < *off
                })
                .cloned()
                .collect()
        };
        let (have, want) = (of(events), of(clean_events));
        assert_eq!(
            have.len(),
            want.len(),
            "{what}: a file's events are incomplete"
        );
        assert!(
            have == want,
            "{what}: a file's events differ from a clean run"
        );
    }
    (saved.len(), events.len())
}

/// `splitmix64`: the seeded generator of the large set and of the random kill points.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

/// A deterministic set of valid Claude Code transcripts under `<projects>/<name>-p<k>/s<NNN>.jsonl`: `files` files of
/// `lines` lines (user and assistant records in turn, about 450 bytes each, invented text only). Returns the bytes.
pub fn write_large_set(projects: &Path, name: &str, files: usize, lines: usize, seed: u64) -> u64 {
    write_set_padded(projects, name, files, lines, seed, 200)
}

/// [`write_large_set`] with `pad` random letters of invented text per line (200 there): fewer bytes per event, more
/// events for the same disk.
pub fn write_set_padded(
    projects: &Path,
    name: &str,
    files: usize,
    lines: usize,
    seed: u64,
    pad: usize,
) -> u64 {
    let mut rng = Rng(seed);
    let mut total = 0;
    for f in 0..files {
        let dir = projects.join(format!("{name}-p{}", f % 3));
        fs::create_dir_all(&dir).unwrap();
        let mut text = String::with_capacity(lines * 480);
        for i in 0..lines {
            let ts = format!(
                "2026-01-{:02}T{:02}:{:02}:{:02}.000Z",
                10 + f % 5,
                10 + i / 3600,
                (i / 60) % 60,
                i % 60
            );
            let pad: String = (0..pad)
                .map(|_| (b'a' + (rng.next() % 26) as u8) as char)
                .collect();
            let common = format!(
                r#""sessionId":"t031-{name}-{f:03}","timestamp":"{ts}","version":"2.1.316","cwd":"/nonexistent-t031/p{}","isSidechain":false"#,
                f % 3
            );
            if i % 2 == 0 {
                text += &format!(
                    r#"{{"type":"user",{common},"entrypoint":"cli","message":{{"role":"user","content":"invented {pad}"}}}}"#
                );
            } else {
                let (a, b, c, d) = (
                    rng.next() % 5000,
                    rng.next() % 2000,
                    rng.next() % 300,
                    rng.next() % 90000,
                );
                text += &format!(
                    r#"{{"type":"assistant",{common},"message":{{"id":"msg_t031_{name}_{f}_{i}","model":"claude-sonnet-5-5","usage":{{"input_tokens":{a},"output_tokens":{b},"cache_creation_input_tokens":{c},"cache_read_input_tokens":{d}}},"content":[{{"type":"text","text":"invented {pad}"}}]}}}}"#
                );
            }
            text.push('\n');
        }
        fs::write(dir.join(format!("s{f:03}.jsonl")), &text).unwrap();
        total += text.len() as u64;
    }
    total
}

fn lines_of(stdout: impl Read + Send + 'static) -> Receiver<String> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    rx
}

/// `easy-personal-rs serve` on the case's data folder, read over a raw `TcpStream`.
pub struct Serve {
    proc: Proc,
    port: u16,
    token: String,
    _lines: Receiver<String>,
}

impl Serve {
    pub fn start(case: &Case) -> Serve {
        let mut proc = Spec::rust(case, &["serve", "--no-ingest"]).spawn();
        let lines = lines_of(proc.child.stdout.take().unwrap());
        let first = lines
            .recv_timeout(Duration::from_secs(30))
            .expect("serve printed its URL");
        let url = first
            .strip_prefix("dashboard: http://127.0.0.1:")
            .expect("the URL line");
        let (port, token) = url.split_once("/#").expect("a token fragment");
        Serve {
            port: port.parse().unwrap(),
            token: token.to_owned(),
            proc,
            _lines: lines,
        }
    }

    /// `GET path` with the good Host and token: (status, body).
    pub fn get(&self, path: &str) -> (u16, String) {
        let mut s = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(30))).unwrap();
        write!(
            s,
            "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nx-dashboard-token: {}\r\nConnection: close\r\n\r\n",
            self.port, self.token
        )
        .unwrap();
        let mut bytes = Vec::new();
        let _ = s.read_to_end(&mut bytes);
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
        let status = head
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let chunked = head
            .to_ascii_lowercase()
            .contains("transfer-encoding: chunked");
        (
            status,
            if chunked {
                dechunk(body)
            } else {
                body.to_owned()
            },
        )
    }

    pub fn stop(self) {
        self.proc.kill();
    }
}

fn dechunk(mut body: &str) -> String {
    let mut out = String::new();
    while let Some((size, rest)) = body.split_once("\r\n") {
        let n = usize::from_str_radix(size.trim(), 16).unwrap_or(0);
        if n == 0 || rest.len() < n {
            break;
        }
        out += &rest[..n];
        body = rest[n..].trim_start_matches("\r\n");
    }
    out
}

/// `easy-personal-rs mcp` on the case's data folder: initialize, then `tools/call`.
pub struct Mcp {
    proc: Proc,
    stdin: ChildStdin,
    lines: Receiver<String>,
    id: u64,
}

impl Mcp {
    pub fn start(case: &Case) -> Mcp {
        let mut proc = Spec::rust(case, &["mcp"]).with_stdin().spawn();
        let stdin = proc.child.stdin.take().unwrap();
        let lines = lines_of(proc.child.stdout.take().unwrap());
        let mut mcp = Mcp {
            proc,
            stdin,
            lines,
            id: 0,
        };
        let init = mcp.request(
            "initialize",
            json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t031", "version": "0"}}),
        );
        assert!(init["result"].is_object(), "mcp initialize failed");
        writeln!(
            mcp.stdin,
            "{}",
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
        )
        .unwrap();
        mcp
    }

    pub fn request(&mut self, method: &str, params: Value) -> Value {
        self.id += 1;
        let id = self.id;
        writeln!(
            self.stdin,
            "{}",
            json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
        )
        .unwrap();
        self.stdin.flush().unwrap();
        loop {
            let line = self
                .lines
                .recv_timeout(Duration::from_secs(60))
                .expect("an MCP answer in time");
            let v: Value = serde_json::from_str(&line).expect("an MCP line is JSON");
            if v["id"] == json!(id) {
                return v;
            }
        }
    }

    /// `tools/call usage_summary {}`: its structured result (a tool error fails).
    pub fn usage_summary(&mut self) -> Value {
        let a = self.request(
            "tools/call",
            json!({"name": "usage_summary", "arguments": {}}),
        );
        assert_eq!(a["result"]["isError"], json!(false), "usage_summary failed");
        a["result"]["structuredContent"].clone()
    }

    /// Closes stdin and waits for a clean exit.
    pub fn stop(self) {
        let Mcp { proc, stdin, .. } = self;
        drop(stdin);
        let out = proc.wait(Duration::from_secs(30));
        assert_eq!(out.code, Some(0), "mcp exit");
    }
}
