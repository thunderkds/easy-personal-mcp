//! The `claude` CLI as a subprocess (T050, FR-019, DDR-0001): Python's `writer._claude`, `account_env` and
//! `launcher_path`, behind core's [`Claude`] trait (the argument lists are built in `core/src/sync.rs`).
//!
//! `claude` is found on `PATH` like `shutil.which` (an executable regular file; an empty `PATH` entry is skipped, where
//! Python would look in the working folder). It runs with an argument list, never a shell; with this process's
//! environment minus `CLAUDE_CONFIG_DIR`, plus `CLAUDE_CONFIG_DIR=<folder>` for an account that is not the default one;
//! stdin closed; in a process group of its own; for at most 60 s (then the whole group is killed, so no process it
//! started lives on (T066), and the call answers 127, as Python's `TimeoutExpired` path does).
//! stdout is kept up to 1 MiB and only parsed (`Command:` / `Args:` of `claude mcp get`); stderr is read and dropped.
//! Neither is ever shown, logged or sent.

use easy_personal_core::sync::{Claude, LAUNCHER, Ran};
use std::ffi::OsString;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Python's `timeout=60`.
const TIMEOUT: Duration = Duration::from_secs(60);
/// The most of `claude`'s stdout kept (the rest is read and dropped, so the process never blocks on a full pipe).
const MAX_STDOUT: usize = 1 << 20;
/// Python's answer when `claude` could not run or timed out.
const NOT_RUN: i32 = 127;
const SIGKILL: i32 = 9;

unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

pub struct SystemClaude {
    /// The `PATH` `claude` and the launcher are looked up on (this process's, read once per request).
    search: Option<OsString>,
    timeout: Duration,
}

impl SystemClaude {
    /// The real one: this process's `PATH`, Python's 60 s.
    pub fn new() -> Self {
        SystemClaude {
            search: std::env::var_os("PATH"),
            timeout: TIMEOUT,
        }
    }

    fn which(&self, name: &str) -> Option<PathBuf> {
        // `shutil.which` with no PATH uses `os.defpath`
        let search = self
            .search
            .clone()
            .unwrap_or_else(|| "/bin:/usr/bin".into());
        std::env::split_paths(&search)
            .filter(|dir| !dir.as_os_str().is_empty())
            .map(|dir| dir.join(name))
            .find(|path| is_executable(path))
    }
}

fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

impl Claude for SystemClaude {
    fn on_path(&self) -> bool {
        self.which("claude").is_some()
    }

    fn run(&self, args: &[&str], config_dir_env: Option<&str>) -> Ran {
        let not_run = Ran {
            code: NOT_RUN,
            stdout: String::new(),
            not_run: true,
        };
        let Some(exe) = self.which("claude") else {
            return not_run;
        };
        let mut cmd = Command::new(exe);
        cmd.args(args)
            .env_remove("CLAUDE_CONFIG_DIR")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0); // its own group: a timeout kills what it started too
        if let Some(dir) = config_dir_env {
            cmd.env("CLAUDE_CONFIG_DIR", dir);
        }
        match cmd.spawn() {
            Ok(child) => finish(child, self.timeout),
            Err(_) => not_run,
        }
    }

    /// `launcher_path`: beside this program, else on `PATH`.
    fn launcher(&self) -> Result<String, String> {
        let beside = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join(LAUNCHER)))
            .filter(|path| path.is_file());
        beside
            .or_else(|| self.which(LAUNCHER))
            .and_then(|path| path.to_str().map(str::to_string))
            .ok_or_else(|| {
                format!("cannot find {LAUNCHER}; reinstall easy-personal (uv tool install) so it is on disk")
            })
    }
}

/// Reads stdout (capped) and stderr (dropped) on two threads, waits up to `timeout`, kills on timeout.
fn finish(mut child: Child, timeout: Duration) -> Ran {
    let (tx, rx) = mpsc::channel();
    if let Some(mut out) = child.stdout.take() {
        std::thread::spawn(move || {
            let mut kept = Vec::new();
            let _ = (&mut out).take(MAX_STDOUT as u64).read_to_end(&mut kept);
            let _ = std::io::copy(&mut out, &mut std::io::sink());
            let _ = tx.send(kept);
        });
    }
    if let Some(mut err) = child.stderr.take() {
        std::thread::spawn(move || std::io::copy(&mut err, &mut std::io::sink()));
    }
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            _ => break None,
        }
    };
    let Some(status) = status else {
        // the group's id is the child's pid (`process_group(0)`); the child is not reaped yet, so the id is still its
        if let Ok(group) = i32::try_from(child.id()) {
            // SAFETY: a signal to the process group this function started
            unsafe { kill(-group, SIGKILL) };
        }
        let _ = child.kill();
        let _ = child.wait();
        return Ran {
            code: NOT_RUN,
            stdout: String::new(),
            not_run: true,
        };
    };
    // Python's `returncode`: the exit code, or minus the signal that ended it
    let code = status
        .code()
        .unwrap_or_else(|| -status.signal().unwrap_or(0));
    // a grandchild that keeps the pipe open must not hold the request: wait a moment for stdout, then give up on it
    let stdout = rx.recv_timeout(Duration::from_secs(2)).unwrap_or_default();
    Ran {
        code,
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        not_run: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        dir: PathBuf,
    }

    impl Fake {
        /// A folder with a fake `claude` that appends its argv and `CLAUDE_CONFIG_DIR` to `log`, then runs `tail`.
        fn new(tag: &str, tail: &str) -> Fake {
            let dir = std::env::temp_dir().join(format!(
                "easy-personal-rs-t050-cli-{tag}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.join("bin")).unwrap();
            let log = dir.join("log");
            let script = format!(
                "#!/bin/sh\nfor a in \"$@\"; do printf '%s\\n' \"$a\" >> '{log}'; done\n\
                 printf 'CCD=%s\\n--\\n' \"${{CLAUDE_CONFIG_DIR-<unset>}}\" >> '{log}'\n{tail}\n",
                log = log.display()
            );
            let exe = dir.join("bin/claude");
            std::fs::write(&exe, script).unwrap();
            std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
            Fake { dir }
        }

        fn claude(&self, timeout: Duration) -> SystemClaude {
            SystemClaude {
                search: Some(
                    format!("/nonexistent-t050::{}", self.dir.join("bin").display()).into(),
                ),
                timeout,
            }
        }

        fn log(&self) -> String {
            std::fs::read_to_string(self.dir.join("log")).unwrap_or_default()
        }
    }

    impl Drop for Fake {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn argv_is_passed_as_a_list_and_the_account_dir_only_through_the_env() {
        let fake = Fake::new(
            "argv",
            "echo '  Command: /x/easy-personal-launch'; echo boom-secret >&2; exit 3",
        );
        let claude = fake.claude(TIMEOUT);
        assert!(claude.on_path());
        let json =
            r#"{"type": "stdio", "command": "/a b/easy-personal-launch", "args": ["exec", "c1"]}"#;
        let got = claude.run(
            &["mcp", "add-json", "acme-slack", json, "--scope", "user"],
            Some("/acc/.claude-work"),
        );
        assert_eq!(got.code, 3);
        assert_eq!(
            got.stdout, "  Command: /x/easy-personal-launch\n",
            "stderr is never kept"
        );
        let none = claude.run(&["mcp", "get", "x;rm -rf /"], None);
        assert_eq!(none.code, 3);
        assert_eq!(
            fake.log(),
            format!(
                "mcp\nadd-json\nacme-slack\n{json}\n--scope\nuser\nCCD=/acc/.claude-work\n--\n\
                 mcp\nget\nx;rm -rf /\nCCD=<unset>\n--\n"
            ),
            "one argument each, no shell; CLAUDE_CONFIG_DIR set or removed"
        );
    }

    #[test]
    fn not_on_path_is_absent_and_a_directory_or_plain_file_is_not_claude() {
        let fake = Fake::new("absent", "exit 0");
        std::fs::remove_file(fake.dir.join("bin/claude")).unwrap();
        assert!(!fake.claude(TIMEOUT).on_path());
        std::fs::create_dir(fake.dir.join("bin/claude")).unwrap();
        assert!(!fake.claude(TIMEOUT).on_path());
        std::fs::remove_dir(fake.dir.join("bin/claude")).unwrap();
        std::fs::write(fake.dir.join("bin/claude"), "#!/bin/sh\n").unwrap(); // not executable
        assert!(!fake.claude(TIMEOUT).on_path());
        assert_eq!(
            fake.claude(TIMEOUT).run(&["mcp", "get", "a"], None).code,
            127
        );
    }

    #[test]
    fn a_slow_claude_is_killed_at_the_timeout_and_answers_127() {
        let fake = Fake::new("slow", "echo $$ > \"$(dirname \"$0\")/pid\"; exec sleep 30");
        let started = Instant::now();
        let got = fake
            .claude(Duration::from_millis(500))
            .run(&["mcp", "get", "a"], None);
        assert_eq!(
            got,
            Ran {
                code: 127,
                stdout: String::new(),
                not_run: true
            }
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "{:?}",
            started.elapsed()
        );
        let pid = std::fs::read_to_string(fake.dir.join("bin/pid")).unwrap();
        assert!(
            !Path::new(&format!("/proc/{}", pid.trim())).exists(),
            "the process is gone"
        );
    }

    /// T066 AC6: a timed-out `claude` that started a process of its own leaves no live process: the whole process group
    /// is killed, not only the direct child.
    #[test]
    fn a_timeout_kills_claudes_whole_process_group() {
        let fake = Fake::new(
            "group",
            "sleep 30 & echo $! > \"$(dirname \"$0\")/grandchild\"; echo $$ > \"$(dirname \"$0\")/pid\"; wait",
        );
        let got = fake
            .claude(Duration::from_millis(500))
            .run(&["mcp", "get", "a"], None);
        assert_eq!((got.code, got.not_run), (127, true));
        for file in ["pid", "grandchild"] {
            let pid = std::fs::read_to_string(fake.dir.join("bin").join(file)).unwrap();
            let stat = format!("/proc/{}/stat", pid.trim());
            // gone, or a zombie its new parent has not reaped yet: either way it runs no more
            let deadline = Instant::now() + Duration::from_secs(5);
            let live = loop {
                let live = std::fs::read_to_string(&stat)
                    .ok()
                    .and_then(|s| {
                        s.rsplit(')')
                            .next()
                            .map(|r| !r.trim_start().starts_with('Z'))
                    })
                    .unwrap_or(false);
                if !live || Instant::now() > deadline {
                    break live;
                }
                std::thread::sleep(Duration::from_millis(20));
            };
            assert!(!live, "{file} {} still runs", pid.trim());
        }
    }

    #[test]
    fn could_not_run_is_the_runners_own_signal_never_the_exit_code() {
        // a `claude` that exits 127 itself did run
        let fake = Fake::new("exit127", "exit 127");
        let got = fake.claude(TIMEOUT).run(&["mcp", "get", "a"], None);
        assert_eq!((got.code, got.not_run), (127, false));
        // a spawn that fails (missing interpreter) and a timeout did not
        std::fs::write(fake.dir.join("bin/claude"), "#!/nonexistent/t050\n").unwrap();
        let got = fake.claude(TIMEOUT).run(&["mcp", "get", "a"], None);
        assert_eq!((got.code, got.not_run), (127, true));
        let slow = Fake::new("slow2", "exec sleep 30");
        let got = slow
            .claude(Duration::from_millis(200))
            .run(&["mcp", "get", "a"], None);
        assert_eq!((got.code, got.not_run), (127, true));
    }

    #[test]
    fn stdout_is_capped_and_a_huge_stderr_never_blocks() {
        let fake = Fake::new(
            "big",
            "head -c 3000000 /dev/zero | tr '\\0' 'e' >&2; head -c 3000000 /dev/zero | tr '\\0' 'o'; exit 0",
        );
        let got = fake
            .claude(Duration::from_secs(20))
            .run(&["mcp", "get", "a"], None);
        assert_eq!(got.code, 0);
        assert_eq!(got.stdout.len(), MAX_STDOUT);
    }

    #[test]
    fn the_launcher_is_found_beside_this_program_or_on_path() {
        let fake = Fake::new("launcher", "exit 0");
        // a test binary lives in target/<profile>/deps: no launcher beside it, so PATH decides
        assert!(
            fake.claude(TIMEOUT)
                .launcher()
                .unwrap_err()
                .starts_with("cannot find easy-personal-launch;")
        );
        let exe = fake.dir.join("bin").join(LAUNCHER);
        std::fs::write(&exe, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            fake.claude(TIMEOUT).launcher().unwrap(),
            exe.to_str().unwrap()
        );
    }
}
