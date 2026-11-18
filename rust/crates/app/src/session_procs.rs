//! Terminal sessions' process files and *Stop* (T116, FR-124).
//!
//! Claude Code writes `<config dir>/sessions/<pid>.json` for each running session (beside a `<pid>.<hash>.key` file this module
//! never opens). [`read_all`] reads those files of the Claude accounts' folders (every field optional; a file that is not a
//! regular file, is over [`FILE_MAX`], is not JSON, is not ours or names another pid than its own file name is skipped; the
//! open is `O_NOFOLLOW | O_NONBLOCK`, judged on the handle, like the live tail's) and keeps seven fields in memory: the pid, the
//! session id, `procStart`, the status, the name, the kind and the entrypoint. Nothing is written, and no other file of the
//! folder is opened. The page reads the result through the bridge (`answer_bridge.rs`) as `proc {busy, name}` of a session in
//! `GET /api/live` only (DDR-0005).
//!
//! [`stop`] sends SIGTERM to a session's process, and only when it is plainly a Claude Code session of ours: the process is
//! pinned with a pidfd first (so its pid cannot be reused while the checks run), `/proc/<pid>` is owned by our uid, its start
//! time (field 22 of `/proc/<pid>/stat`, clock ticks since boot, which is what Claude Code writes as `procStart`) equals
//! `procStart` within 2 s, and its executable file name or `argv[0]` base name is `claude` (or it is a `node` or `bun`
//! running a script path that has a `claude` or `claude-code` component). Anything else is refused with one line. It never
//! signals a process group and never uses a pid that was not checked.

use crate::answer_bridge::own_uid;
use crate::api::{self, Api};
use crate::writes::{self, fail, raw_body};
use axum::extract::{Path as RoutePath, Request, State};
use axum::http::StatusCode;
use axum::response::Response;
use easy_personal_core::live::clean_line;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

/// A process file is at most this long.
const FILE_MAX: u64 = 64 * 1024;
/// At most this many files of one folder are looked at.
const FILES_MAX: usize = 512;
const NAME_MAX: usize = 100;
const WORD_MAX: usize = 40;
/// What is kept of one process file. No `Debug`: the name is the owner's text.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ProcInfo {
    pub(crate) pid: i32,
    /// `procStart` as written (digits).
    pub(crate) proc_start: String,
    pub(crate) busy: bool,
    pub(crate) name: Option<String>,
    pub(crate) kind: Option<String>,
    pub(crate) entrypoint: Option<String>,
}

impl ProcInfo {
    /// An app session (`sdk` entrypoint): its Stop is FR-110's, not this one.
    pub(crate) fn is_sdk(&self) -> bool {
        self.entrypoint
            .as_deref()
            .is_some_and(|e| e.starts_with("sdk"))
    }
}

/// The process files of every folder in `config_dirs`, by session id. Two files of one session: the higher pid wins.
pub(crate) fn read_all(config_dirs: &[PathBuf]) -> HashMap<String, ProcInfo> {
    let mut all: HashMap<String, ProcInfo> = HashMap::new();
    for dir in config_dirs {
        for (session, info) in read_dir(&dir.join("sessions")) {
            if all.get(&session).is_none_or(|old| old.pid < info.pid) {
                all.insert(session, info);
            }
        }
    }
    all
}

fn read_dir(dir: &Path) -> Vec<(String, ProcInfo)> {
    // a symlinked `sessions` folder is not followed
    if !fs::symlink_metadata(dir).is_ok_and(|m| m.is_dir()) {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .take(FILES_MAX)
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let stem = name.strip_suffix(".json")?;
            if stem.is_empty() || !stem.bytes().all(|b| b.is_ascii_digit()) {
                return None; // `<pid>.<hash>.key` and anything else
            }
            read_file(&entry.path(), stem.parse().ok()?)
        })
        .collect()
}

/// One process file of the pid `file_pid`: `(session id, kept fields)`.
fn read_file(path: &Path, file_pid: i32) -> Option<(String, ProcInfo)> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .ok()?;
    let meta = file.metadata().ok()?;
    if !meta.is_file() || meta.len() > FILE_MAX || meta.uid() != own_uid() {
        return None;
    }
    let mut raw = Vec::new();
    file.take(FILE_MAX + 1).read_to_end(&mut raw).ok()?;
    if raw.len() as u64 > FILE_MAX {
        return None;
    }
    parse_file(&raw, file_pid)
}

fn parse_file(raw: &[u8], file_pid: i32) -> Option<(String, ProcInfo)> {
    let Value::Object(file) = serde_json::from_slice(raw).ok()? else {
        return None;
    };
    let word = |key: &str, max: usize| {
        file.get(key)
            .and_then(Value::as_str)
            .map(|s| clean_line(s, max))
            .filter(|s| !s.is_empty())
    };
    let pid = match file.get("pid") {
        None | Some(Value::Null) => file_pid,
        Some(v) => i32::try_from(v.as_i64()?).ok()?,
    };
    if pid != file_pid {
        return None; // a file that names another process than its own name
    }
    // no session id: nothing to attach it to
    let session = file.get("sessionId").and_then(Value::as_str)?;
    if session.is_empty() || session.len() > 200 {
        return None;
    }
    Some((
        session.to_string(),
        ProcInfo {
            pid,
            proc_start: word("procStart", WORD_MAX)
                .filter(|s| s.bytes().all(|b| b.is_ascii_digit()))
                .unwrap_or_default(),
            busy: file.get("status").and_then(Value::as_str) == Some("busy"),
            name: word("name", NAME_MAX),
            kind: word("kind", WORD_MAX),
            entrypoint: word("entrypoint", WORD_MAX),
        },
    ))
}

// ---- Stop ----

/// `procStart` and the process's own start time are the same moment within this many seconds.
const START_SLACK_S: u64 = 2;

/// Is the process one of Claude Code? `exe` and `argv` are what `/proc/<pid>/exe` and `/proc/<pid>/cmdline` say.
fn is_claude(exe: Option<&str>, argv: &[&str]) -> bool {
    let base = |path: &str| path.rsplit('/').next().unwrap_or("").to_string();
    if exe.map(&base).as_deref() == Some("claude")
        || argv.first().map(|a| base(a)).as_deref() == Some("claude")
    {
        return true;
    }
    let runner = |name: &str| name == "node" || name == "bun";
    let runs_node = exe.map(&base).is_some_and(|n| runner(&n))
        || argv.first().map(|a| base(a)).is_some_and(|n| runner(&n));
    runs_node
        && argv
            .iter()
            .skip(1)
            .find(|a| !a.starts_with('-'))
            .is_some_and(|script| {
                script
                    .split('/')
                    .any(|c| c == "claude" || c == "claude-code")
            })
}

/// Field 22 of `/proc/<pid>/stat` (clock ticks since boot). The name in field 2 may hold spaces and parentheses, so the
/// fields are counted from the last `)`.
fn start_ticks(stat: &str) -> Option<u64> {
    stat[stat.rfind(')')? + 1..]
        .split_whitespace()
        .nth(19)?
        .parse()
        .ok()
}

fn start_matches(ticks: u64, proc_start: &str, hz: u64) -> bool {
    proc_start
        .parse::<u64>()
        .is_ok_and(|written| written.abs_diff(ticks) <= START_SLACK_S * hz)
}

/// SIGTERM to the process of `info`, after the checks of the module docs. `Err`: the one-line reason it was not sent.
#[cfg(target_os = "linux")]
pub(crate) fn stop(info: &ProcInfo) -> Result<(), &'static str> {
    use std::os::fd::{FromRawFd, OwnedFd};
    const GONE: &str = "that session's process is gone or is not the Claude Code process it was";
    if info.pid <= 1 {
        return Err(GONE);
    }
    // SAFETY: `pidfd_open` takes a pid and a flags word and returns a new fd or -1.
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, libc::c_long::from(info.pid), 0) };
    let Ok(fd) = i32::try_from(fd) else {
        return Err(GONE);
    };
    if fd < 0 {
        return Err(GONE);
    }
    // SAFETY: `fd` is a fresh descriptor this function owns.
    let pidfd = unsafe { OwnedFd::from_raw_fd(fd) };
    let root = PathBuf::from(format!("/proc/{}", info.pid));
    let owned = fs::metadata(&root).is_ok_and(|m| m.uid() == own_uid());
    // SAFETY: `sysconf` has no side effect.
    let hz = u64::try_from(unsafe { libc::sysconf(libc::_SC_CLK_TCK) })
        .ok()
        .filter(|&h| h > 0)
        .unwrap_or(100);
    let started = fs::read_to_string(root.join("stat"))
        .ok()
        .and_then(|s| start_ticks(&s))
        .is_some_and(|t| start_matches(t, &info.proc_start, hz));
    let exe = fs::read_link(root.join("exe")).ok();
    let mut cmdline = Vec::new();
    if let Ok(file) = fs::File::open(root.join("cmdline")) {
        let _ = file.take(8192).read_to_end(&mut cmdline);
    }
    let argv: Vec<&str> = cmdline
        .split(|&b| b == 0)
        .filter_map(|a| std::str::from_utf8(a).ok())
        .collect();
    let exe_name = exe.as_deref().and_then(|p| p.to_str());
    if !(owned && started && is_claude(exe_name, &argv)) {
        return Err(GONE);
    }
    use std::os::fd::AsRawFd;
    // SAFETY: `pidfd` is a valid pidfd; a null `siginfo` and flags 0 are what the call takes for a plain signal.
    let sent = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            libc::c_long::from(pidfd.as_raw_fd()),
            libc::c_long::from(libc::SIGTERM),
            std::ptr::null::<libc::c_void>(),
            0,
        )
    };
    if sent == 0 {
        Ok(())
    } else {
        Err("the signal could not be sent to that session")
    }
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn stop(_info: &ProcInfo) -> Result<(), &'static str> {
    Err("stopping a terminal session is supported on Linux only")
}

// ---- the route ----

/// `POST /api/live/sessions/{id}/stop` `{confirmed: true}`: SIGTERM to a terminal session's process, found by its process file
/// (the page's `proc`). 200 `{ok: true}`; 400 a bad body; 404 no process file for that session; 409 an app session (its own
/// Stop is `POST /api/app-sessions/{id}/stop`) or a process that is not that Claude Code session (see [`stop`]).
pub(crate) async fn stop_session(
    State(api): State<Api>,
    RoutePath(id): RoutePath<String>,
    req: Request,
) -> Response {
    let raw = match raw_body(req).await {
        Ok((_, raw)) => raw,
        Err(answer) => return answer,
    };
    let Some(Value::Object(body)) = writes::parse(&raw) else {
        return fail(StatusCode::BAD_REQUEST, "body must be a JSON object");
    };
    if body.keys().any(|k| k != "confirmed") {
        return fail(StatusCode::BAD_REQUEST, "unknown field");
    }
    if body.get("confirmed") != Some(&Value::Bool(true)) {
        return fail(
            StatusCode::BAD_REQUEST,
            "confirmed must be true: this ends a Claude Code session running in a terminal",
        );
    }
    if id.is_empty() || id.len() > 200 {
        return fail(StatusCode::BAD_REQUEST, "session id is not valid");
    }
    let done = tokio::task::spawn_blocking(move || {
        let found = api.answers.proc_of(&id);
        if api.answers.is_app_session(&id) || found.as_ref().is_some_and(ProcInfo::is_sdk) {
            return Err((
                StatusCode::CONFLICT,
                "that is an app session: stop it from its own panel",
            ));
        }
        let Some(info) = found else {
            return Err((
                StatusCode::NOT_FOUND,
                "no running terminal session has that id",
            ));
        };
        stop(&info).map_err(|why| (StatusCode::CONFLICT, why))
    })
    .await
    .unwrap_or(Err((StatusCode::INTERNAL_SERVER_ERROR, api::INTERNAL)));
    match done {
        Ok(()) => api::respond(StatusCode::OK, &json!({"ok": true})),
        Err((status, text)) => fail(status, text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::os::unix::fs::symlink;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    struct Temp(PathBuf);

    impl Temp {
        fn new(tag: &str) -> Temp {
            let dir = std::env::temp_dir()
                .join(format!("ep-t116-{tag}-{}", std::process::id() % 100_000));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(dir.join("sessions")).unwrap();
            Temp(dir)
        }

        fn put(&self, name: &str, body: &str) {
            fs::write(self.0.join("sessions").join(name), body).unwrap();
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn get(dir: &Temp) -> HashMap<String, ProcInfo> {
        read_all(std::slice::from_ref(&dir.0))
    }

    /// AC 1: the seven fields are kept (the name cleaned), every field is optional but the session id, and the folder's other
    /// files are never opened.
    #[test]
    fn the_seven_fields_are_kept_and_the_rest_of_the_file_is_dropped() {
        let t = Temp::new("keep");
        t.put(
            "41.json",
            &json!({"pid": 41, "sessionId": "s1", "procStart": "767343", "status": "busy", "name": "fix\nthe \u{200b}bug",
                    "kind": "interactive", "entrypoint": "cli", "cwd": "/secret/place", "messagingSocketPath": "/x"})
            .to_string(),
        );
        t.put("42.json", r#"{"sessionId":"s2"}"#);
        let all = get(&t);
        let one = &all["s1"];
        assert_eq!(
            (one.pid, one.proc_start.as_str(), one.busy),
            (41, "767343", true)
        );
        assert_eq!(one.name.as_deref(), Some("fix the bug"));
        assert_eq!(
            (one.kind.as_deref(), one.entrypoint.as_deref()),
            (Some("interactive"), Some("cli"))
        );
        let bare = &all["s2"];
        assert_eq!(
            (bare.pid, bare.busy, bare.name.is_none()),
            (42, false, true)
        );
        assert_eq!(bare.proc_start, "");
    }

    /// AC 1: a `*.key` file, a file over 64 KB, one that is not JSON, one with no session id, a symlink, a file that names
    /// another pid and a FIFO are skipped; a symlinked `sessions` folder is not followed.
    #[test]
    fn hostile_files_are_skipped() {
        let t = Temp::new("hostile");
        t.put("1.abc123.key", r#"{"sessionId":"key"}"#);
        t.put(
            "2.json",
            &format!(r#"{{"sessionId":"big","pad":"{}"}}"#, "x".repeat(70_000)),
        );
        t.put("3.json", "not json");
        t.put("4.json", r#"{"pid":4}"#);
        t.put("5.json", r#"{"pid":99,"sessionId":"other"}"#);
        t.put("6.json", r#"["sessionId"]"#);
        t.put("ok.json", r#"{"sessionId":"letters"}"#);
        t.put("7.json", r#"{"pid":7,"sessionId":"good"}"#);
        fs::write(t.0.join("elsewhere.json"), r#"{"sessionId":"linked"}"#).unwrap();
        symlink(t.0.join("elsewhere.json"), t.0.join("sessions/8.json")).unwrap();
        let fifo = t.0.join("sessions/9.json");
        assert!(
            Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        let began = Instant::now();
        let mut keys: Vec<_> = get(&t).into_keys().collect();
        keys.sort();
        assert_eq!(keys, ["good"]);
        assert!(
            began.elapsed() < Duration::from_secs(2),
            "a FIFO does not block"
        );
        // a symlinked folder
        let link = Temp::new("hostile-link");
        fs::remove_dir_all(link.0.join("sessions")).unwrap();
        symlink(t.0.join("sessions"), link.0.join("sessions")).unwrap();
        assert!(get(&link).is_empty());
        // no folder at all
        assert!(read_all(&[t.0.join("nowhere")]).is_empty());
    }

    /// Two files of one session: the higher pid wins; a name is cut and the folders merge.
    #[test]
    fn two_files_of_one_session_keep_the_higher_pid() {
        let t = Temp::new("dup");
        t.put("10.json", r#"{"sessionId":"s"}"#);
        t.put("20.json", r#"{"sessionId":"s"}"#);
        assert_eq!(get(&t)["s"].pid, 20);
        let long = "n".repeat(500);
        t.put(
            "30.json",
            &json!({"sessionId": "n", "name": long}).to_string(),
        );
        assert_eq!(get(&t)["n"].name.as_deref().map(str::len), Some(NAME_MAX));
    }

    #[test]
    fn the_process_checks_read_the_stat_field_and_the_names() {
        let stat = "123 (a) b) (c) S 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 99 1 2";
        // fields after the last `)`: state is field 3, so field 22 is the 20th of what follows
        assert_eq!(start_ticks(stat), Some(99));
        assert_eq!(start_ticks("garbage"), None);
        assert!(start_matches(1000, "1000", 100));
        assert!(start_matches(1000, "1200", 100));
        assert!(!start_matches(1000, "1201", 100));
        assert!(!start_matches(1000, "", 100));
        assert!(!start_matches(1000, "abc", 100));
        assert!(is_claude(Some("/x/claude"), &[]));
        assert!(is_claude(
            Some("/x/2.1.327"),
            &["/home/u/.local/bin/claude", "--flag"]
        ));
        assert!(is_claude(
            Some("/usr/bin/node"),
            &[
                "node",
                "/usr/lib/node_modules/@anthropic-ai/claude-code/cli.js"
            ]
        ));
        assert!(is_claude(
            Some("/usr/bin/bun"),
            &["bun", "-x", "/opt/claude"]
        ));
        assert!(!is_claude(Some("/usr/bin/node"), &["node", "server.js"]));
        assert!(!is_claude(Some("/usr/bin/sleep"), &["sleep", "5"]));
        assert!(!is_claude(Some("/usr/bin/vim"), &["vim", "claude"]));
        assert!(!is_claude(None, &[]));
    }

    /// A copy of `sleep` named `name` in `dir`, running.
    fn dummy(dir: &Path, name: &str) -> Child {
        let bin = dir.join(name);
        fs::copy("/bin/sleep", &bin).unwrap();
        Command::new(&bin)
            .arg("30")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    }

    fn info_of(child: &Child) -> ProcInfo {
        let stat = fs::read_to_string(format!("/proc/{}/stat", child.id())).unwrap();
        ProcInfo {
            pid: i32::try_from(child.id()).unwrap(),
            proc_start: start_ticks(&stat).unwrap().to_string(),
            busy: false,
            name: None,
            kind: None,
            entrypoint: None,
        }
    }

    fn wait_gone(child: &mut Child) -> Option<std::process::ExitStatus> {
        for _ in 0..200 {
            if let Some(status) = child.try_wait().unwrap() {
                return Some(status);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        None
    }

    /// AC 3: a dummy process named `claude` is stopped with SIGTERM; its start time must match, and a process with another
    /// name, a wrong start or a pid that is gone is refused and left running.
    #[cfg(target_os = "linux")]
    #[test]
    fn stop_signals_only_a_matching_claude_process() {
        use std::os::unix::process::ExitStatusExt;
        let t = Temp::new("stop");
        let mut other = dummy(&t.0, "not-it");
        assert!(stop(&info_of(&other)).is_err(), "another name");
        assert!(other.try_wait().unwrap().is_none(), "left running");
        other.kill().unwrap();
        other.wait().unwrap();

        let mut claude = dummy(&t.0, "claude");
        let mut wrong = info_of(&claude);
        wrong.proc_start = (wrong.proc_start.parse::<u64>().unwrap() + 1000).to_string();
        assert!(stop(&wrong).is_err(), "a start time 10 s off");
        wrong.proc_start = String::new();
        assert!(stop(&wrong).is_err(), "no procStart");
        assert!(claude.try_wait().unwrap().is_none(), "left running");
        let mut near = info_of(&claude);
        near.proc_start = (near.proc_start.parse::<u64>().unwrap() + 150).to_string();
        assert!(stop(&near).is_ok(), "1.5 s off is within the slack");
        let status = wait_gone(&mut claude).expect("SIGTERM ended it");
        assert_eq!(status.signal(), Some(libc::SIGTERM));
        assert!(stop(&info_of_gone(&claude)).is_err(), "a pid that is gone");
    }

    fn info_of_gone(child: &Child) -> ProcInfo {
        ProcInfo {
            pid: i32::try_from(child.id()).unwrap(),
            proc_start: "1".into(),
            busy: false,
            name: None,
            kind: None,
            entrypoint: None,
        }
    }

    /// pid 1 and a pid of another user's process are never signalled.
    #[cfg(target_os = "linux")]
    #[test]
    fn pid_one_and_other_users_processes_are_refused() {
        for pid in [1, 0, -1] {
            let mut info = info_of_gone(&Command::new("true").spawn().unwrap());
            info.pid = pid;
            assert!(stop(&info).is_err(), "pid {pid}");
        }
    }
}
