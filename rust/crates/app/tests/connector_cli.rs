//! T060: what the Python oracle cannot be asked about `connector ...` (AC2, AC4 and the T050 rule on a `claude` that
//! cannot run). The world is the oracle's (`connector_world`): a temp store copy, the debug-build fake keychain file and
//! the fake `claude` that logs its argv. No real keychain, `claude` or account folder is touched.
//!
//! - A secret typed at a terminal is not echoed (a pseudo-terminal from util-linux `script`), and reaches the keychain only.
//! - Ctrl-C at the secret prompt ends with Python's `input ended` text, exit 2, nothing written.
//! - With no terminal and no stdin, every command that asks answers at once (no hang).
//! - The secret is never in a `claude` argument, stdout, stderr or the store, through add and sync.
//! - A `claude` that cannot run (it does not start) is never "no entry": nothing is removed or forgotten.
//! - A connector name the rules refuse reaches neither the keychain nor `claude`.

mod connector_world;

use connector_world::*;
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const SECRET: &str = "xoxb-T060-TYPED-6d1f0c";

unsafe extern "C" {
    fn setsid() -> i32;
}

fn world(setup: &[&str], keychain: Value, claude: Value) -> (World, Value) {
    let case = json!({
        "name": "t060", "setup": setup, "dirs": ["agent-root/.claude", "agent-root/.claude-work/projects"],
        "keychain": keychain, "claude": claude, "fail": null,
    });
    (World::build(&case), case)
}

/// The binary with the world's environment and no controlling terminal.
fn command(w: &World, case: &Value, args: &[&str]) -> Command {
    let mut cmd = Command::new(SERVE);
    w.env(&mut cmd, case, "UTC");
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    // SAFETY: `setsid` is async-signal-safe; it only detaches the child from this terminal
    unsafe {
        cmd.pre_exec(|| {
            setsid();
            Ok(())
        });
    }
    cmd
}

/// Waits at most `secs` for the child; a hang fails the test (the child is killed first).
fn finish(mut child: Child, secs: u64) -> Output {
    let deadline = Instant::now() + Duration::from_secs(secs);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("the command did not end within {secs} s");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    child.wait_with_output().unwrap()
}

fn run(w: &World, case: &Value, args: &[&str], stdin: &str) -> Output {
    let mut child = command(w, case, args)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let _ = input.write_all(stdin.as_bytes());
    drop(input);
    finish(child, 30)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn store_text(w: &World) -> String {
    format!("{:?}", dump(&w.store))
}

/// util-linux `script`, found on this test's `PATH` as a shell finds it; `None` (the caller skips with a printed reason)
/// when there is none (T066: no test hard-codes its folder).
fn script_on_path() -> Option<PathBuf> {
    let found = std::env::split_paths(&std::env::var_os("PATH")?)
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(|dir| dir.join("script"))
        .find(|path| {
            std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        });
    if found.is_none() {
        println!("skipped: util-linux `script` is not on PATH (it gives the command a pseudo-terminal)");
    }
    found
}

/// util-linux `script` gives the command a pseudo-terminal as its controlling terminal; what the command (and the
/// terminal's echo) writes there comes out on `script`'s stdout. The secret is typed only once the prompt is shown,
/// as a person would (input that arrives earlier is flushed by `TCSAFLUSH`, as in Python).
fn at_a_terminal(
    script: &Path,
    w: &World,
    case: &Value,
    args: &str,
    typed: &[u8],
) -> (Option<i32>, String) {
    // `exec`: the command itself is the shell's process, so the terminal's SIGINT reaches only it
    let line = format!("exec {SERVE} {args}");
    let mut cmd = Command::new(script);
    w.env(&mut cmd, case, "UTC");
    let mut child = cmd
        .env("SHELL", "/bin/sh")
        .args(["-q", "-e", "-c", &line, "/dev/null"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut stdout = child.stdout.take().unwrap();
    let reader = {
        let seen = Arc::clone(&seen);
        std::thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            while let Ok(n) = stdout.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                seen.lock().unwrap().extend_from_slice(&chunk[..n]);
            }
        })
    };
    let deadline = Instant::now() + Duration::from_secs(20);
    while !text(&seen.lock().unwrap()).contains("Bot token (xoxb-...): ") {
        assert!(
            Instant::now() < deadline,
            "no prompt: {:?}",
            text(&seen.lock().unwrap())
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(200)); // the echo is off before the prompt is written; a margin all the same
    let mut input = child.stdin.take().unwrap();
    input.write_all(typed).unwrap();
    input.flush().unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("no end: {:?}", text(&seen.lock().unwrap()));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    drop(input);
    reader.join().unwrap();
    let out = text(&seen.lock().unwrap());
    (status.code(), out)
}

#[test]
fn a_secret_typed_at_a_terminal_is_not_echoed_and_goes_to_the_keychain_only() {
    let Some(script) = script_on_path() else {
        return;
    };
    let (w, case) = world(&[], json!({}), json!({}));
    let (code, screen) = at_a_terminal(
        &script,
        &w,
        &case,
        "connector add slack --workspace acme --yes",
        format!("{SECRET}\n").as_bytes(),
    );
    assert_eq!(code, Some(0), "{screen}");
    assert!(!screen.contains(SECRET), "echoed: {screen:?}");
    assert!(
        !screen.contains("Warning: Password input may be echoed."),
        "the terminal path has no fallback warning: {screen:?}"
    );
    assert!(screen.contains("added to workspace 'acme'"), "{screen:?}");
    let kept = w.keychain_after().to_string();
    assert!(
        kept.contains(SECRET),
        "the value typed is the keychain's: {kept}"
    );
    assert!(!store_text(&w).contains(SECRET));
}

#[test]
fn ctrl_c_at_the_secret_prompt_ends_with_pythons_text_and_writes_nothing() {
    let Some(script) = script_on_path() else {
        return;
    };
    let (w, case) = world(&[], json!({}), json!({}));
    let before = store_text(&w);
    // the terminal's interrupt character (^C) sends SIGINT to the command
    let (code, screen) = at_a_terminal(
        &script,
        &w,
        &case,
        "connector add slack --workspace acme --yes",
        b"\x03",
    );
    assert_eq!(code, Some(2), "{screen:?}");
    assert!(
        screen.contains("easy-personal-rs: input ended at the prompt; nothing was written"),
        "{screen:?}"
    );
    assert_eq!(w.keychain_after(), json!({}));
    assert_eq!(store_text(&w), before);
}

#[test]
fn with_no_terminal_and_no_stdin_every_question_is_answered_at_once() {
    let acme = [
        "INSERT INTO connectors VALUES ('slack-acme', 'acme-slack', 0, 'slack', 2)",
        "INSERT INTO account_registrations VALUES (2, 'acme-slack', 'slack-acme')",
        "INSERT INTO account_workspaces VALUES (1, 2)", // the default account picks `acme`: sync has a step to ask about
    ];
    let (w, case) = world(
        &acme,
        json!({"easy-personal/slack-acme": "{}"}),
        json!({"@W@/agent-root/.claude-work": {"acme-slack": ["@LAUNCHER@", "exec slack-acme"]}}),
    );
    let before = store_text(&w);
    for (args, code) in [
        (
            &["connector", "add", "slack", "--workspace", "personal"][..],
            2,
        ),
        (
            &["connector", "add", "teams", "--workspace", "personal"][..],
            2,
        ),
        (&["connector", "remove", "slack-acme"][..], 1),
        (&["connector", "sync"][..], 1),
        (&["connector", "move", "slack-acme", "personal"][..], 1),
        (&["connector", "alias", "slack-acme", "x"][..], 1),
        (&["connector", "remove", "discord"][..], 1),
    ] {
        let started = Instant::now();
        let child = command(&w, &case, args)
            .stdin(Stdio::null())
            .spawn()
            .unwrap();
        let out = finish(child, 20);
        assert_eq!(
            out.status.code(),
            Some(code),
            "{args:?}: {}",
            text(&out.stderr)
        );
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "{args:?}: {:?}",
            started.elapsed()
        );
    }
    assert_eq!(store_text(&w), before, "nothing was written");
    assert_eq!(
        w.keychain_after(),
        json!({"easy-personal/slack-acme": "{}"})
    );
}

#[test]
fn the_secret_is_never_in_a_claude_argument_stdout_stderr_or_the_store() {
    let (w, case) = world(&[], json!({}), json!({}));
    let added = run(
        &w,
        &case,
        &["connector", "add", "slack", "--workspace", "acme"],
        &format!("{SECRET}\ny\n"),
    );
    assert_eq!(added.status.code(), Some(0), "{}", text(&added.stderr));
    let synced = run(&w, &case, &["connector", "sync", "--yes"], "");
    assert_eq!(synced.status.code(), Some(0), "{}", text(&synced.stderr));
    let log = std::fs::read_to_string(&w.log).unwrap();
    assert!(
        log.contains("add-json"),
        "sync registered the connector: {log}"
    );
    for (label, blob) in [
        ("add stdout", text(&added.stdout)),
        ("add stderr", text(&added.stderr)),
        ("sync stdout", text(&synced.stdout)),
        ("sync stderr", text(&synced.stderr)),
        ("claude argv", log),
        ("store", store_text(&w)),
    ] {
        assert!(!blob.contains(SECRET), "{label}: {blob}");
    }
    assert!(text(&added.stdout).contains("SLACK_MCP_XOXB_TOKEN=***"));
    assert!(w.keychain_after().to_string().contains(SECRET));
}

/// T050 rule: a `claude` that cannot run (here it never starts: its interpreter does not exist) is not "no entry".
#[test]
fn a_claude_that_cannot_run_removes_and_forgets_nothing() {
    let acme = [
        "INSERT INTO connectors VALUES ('slack-acme', 'acme-slack', 0, 'slack', 2)",
        "INSERT INTO account_registrations VALUES (2, 'acme-slack', 'slack-acme')",
        // registered, no longer wanted: with a `claude` that ran and found nothing, sync would `forget` it
        "INSERT INTO connectors VALUES ('gmail-personal', 'personal-gmail', 0, 'gmail', 3)",
        "INSERT INTO account_registrations VALUES (2, 'old-two', 'gmail-personal')",
    ];
    let items = json!({"easy-personal/slack": "{}", "easy-personal/slack-acme": "{}"});
    let (w, case) = world(&acme, items.clone(), json!({}));
    std::fs::write(
        w.dir.join("bin/claude"),
        "#!/nonexistent/t060-interpreter\n",
    )
    .unwrap();
    let before = store_text(&w);
    let pre = run(&w, &case, &["connector", "remove", "slack", "--yes"], "");
    assert_eq!(pre.status.code(), Some(1));
    assert_eq!(
        text(&pre.stdout),
        "the `claude` CLI could not run (it did not start or timed out); nothing was removed.\n"
    );
    let stored = run(
        &w,
        &case,
        &["connector", "remove", "slack-acme", "--yes"],
        "",
    );
    assert_eq!(stored.status.code(), Some(1), "{}", text(&stored.stdout));
    assert!(
        text(&stored.stdout).contains("Claude work: skipped (`claude` could not run)"),
        "{}",
        text(&stored.stdout)
    );
    let synced = run(&w, &case, &["connector", "sync", "--yes"], "");
    assert!(
        text(&synced.stdout).contains("Claude work: skipped (`claude` could not run)"),
        "{}",
        text(&synced.stdout)
    );
    assert!(!text(&synced.stdout).contains("forget"));
    assert_eq!(store_text(&w), before, "no row removed or forgotten");
    assert_eq!(w.keychain_after(), items, "no keychain item removed");
}

#[test]
fn a_name_the_rules_refuse_reaches_neither_the_keychain_nor_claude() {
    let items = json!({"easy-personal/slack": "{}"});
    let (w, case) = world(
        &[],
        items.clone(),
        json!({"<unset>": {"slack": ["@LAUNCHER@", "exec slack"]}}),
    );
    for name in [
        "a/b",
        "..",
        "Slack",
        &"a".repeat(65),
        "",
        "../slack",
        "slack\0x",
    ] {
        if name.contains('\0') {
            continue; // an argument cannot hold a NUL
        }
        let out = run(&w, &case, &["connector", "remove", name, "--yes"], "");
        assert_eq!(out.status.code(), Some(2), "{name:?}");
        assert_eq!(
            text(&out.stderr),
            "easy-personal-rs: unknown connector; nothing was removed\n"
        );
        let out = run(
            &w,
            &case,
            &["connector", "move", name, "personal", "--yes"],
            "",
        );
        assert_eq!(out.status.code(), Some(2), "{name:?}");
    }
    assert!(
        std::fs::read_to_string(&w.log)
            .unwrap_or_default()
            .is_empty(),
        "no claude call"
    );
    assert_eq!(w.keychain_after(), items);
}

/// `account use` ends with a hint naming `connector sync`: that command is here now.
#[test]
fn the_sync_command_the_account_hint_names_exists() {
    let (w, case) = world(&[], json!({}), json!({}));
    let used = run(&w, &case, &["account", "use", "2", "acme"], "");
    assert_eq!(used.status.code(), Some(0), "{}", text(&used.stderr));
    assert!(text(&used.stderr).contains("`easy-personal connector sync`"));
    let help = run(&w, &case, &["connector", "sync", "--help"], "");
    assert_eq!(help.status.code(), Some(0));
    assert!(text(&help.stdout).contains("--account"));
}

// ---- T066 ----

/// The secret value the keychain item of the one connector `add` made holds (the debug fake keychain file).
fn stored_secret(w: &World) -> String {
    let items = w.keychain_after();
    let (_, raw) = items
        .as_object()
        .unwrap()
        .iter()
        .find(|(k, _)| k.starts_with("easy-personal/c"))
        .unwrap_or_else(|| panic!("no item: {items}"));
    let item: Value = serde_json::from_str(raw.as_str().unwrap()).unwrap();
    item["env"]["SLACK_MCP_XOXB_TOKEN"]
        .as_str()
        .unwrap()
        .to_string()
}

/// T066 AC1: Python's `getpass` fallback reads piped stdin as the real text-mode stdin does on Linux (`newline="\n"`)
/// and strips only the final `\n`: `printf 'abc\r\n' | python3 -c 'import getpass; print(repr(getpass.getpass()))'`
/// prints `'abc\r'`.
#[test]
fn a_piped_secret_keeps_a_trailing_cr_as_python_getpass_does() {
    for (typed, kept) in [
        ("abc\r\n", "abc\r"),
        ("abc\n", "abc"),
        ("abc", "abc"),
        ("\r\n", "\r"),
        ("a\rb\n", "a\rb"),
    ] {
        let (w, case) = world(&[], json!({}), json!({}));
        let out = run(
            &w,
            &case,
            &["connector", "add", "slack", "--workspace", "acme", "--yes"],
            typed,
        );
        assert_eq!(out.status.code(), Some(0), "{typed:?}: {}", text(&out.stderr));
        assert_eq!(stored_secret(&w), kept, "{typed:?}");
    }
}

/// A pseudo-terminal pair made with the C library (no crate): the master end for the test, the slave end to hand to the
/// command as its stdin. Neither becomes this process's controlling terminal (`O_NOCTTY`).
mod pty {
    use std::ffi::CStr;
    use std::fs::{File, OpenOptions};
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::fs::OpenOptionsExt;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Termios([u32; 16]);

    const O_RDWR: i32 = 2;
    const O_NOCTTY: i32 = 0o400;
    const ECHO: u32 = 0o10;

    unsafe extern "C" {
        fn posix_openpt(flags: i32) -> i32;
        fn grantpt(fd: i32) -> i32;
        fn unlockpt(fd: i32) -> i32;
        fn ptsname_r(fd: i32, buf: *mut std::ffi::c_char, len: usize) -> i32;
        fn tcgetattr(fd: i32, termios: *mut Termios) -> i32;
    }

    pub fn open() -> (File, File) {
        // SAFETY: plain C calls on a descriptor this function owns; `name` outlives `ptsname_r`
        unsafe {
            let master = posix_openpt(O_RDWR | O_NOCTTY);
            assert!(master >= 0, "posix_openpt");
            assert_eq!(grantpt(master), 0);
            assert_eq!(unlockpt(master), 0);
            let mut name = [0 as std::ffi::c_char; 128];
            assert_eq!(ptsname_r(master, name.as_mut_ptr(), name.len()), 0);
            let path = CStr::from_ptr(name.as_ptr()).to_str().unwrap().to_string();
            let slave = OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(O_NOCTTY)
                .open(path)
                .unwrap();
            (File::from_raw_fd(master), slave)
        }
    }

    /// Whether the terminal echoes what is typed (`ECHO` in `c_lflag`).
    pub fn echo(slave: &File) -> bool {
        let mut t = Termios([0; 16]);
        // SAFETY: `t` is larger than glibc's `struct termios` and lives for the call
        assert_eq!(unsafe { tcgetattr(slave.as_raw_fd(), &mut t) }, 0);
        t.0[3] & ECHO != 0
    }
}

unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

/// `connector add` with no controlling terminal (`setsid`: `/dev/tty` cannot be opened) and a pseudo-terminal as stdin.
/// Returns the child, the master end, the slave end and the master's output so far (a reader thread).
fn with_a_terminal_stdin(
    w: &World,
    case: &Value,
) -> (Child, std::fs::File, std::fs::File, Arc<Mutex<Vec<u8>>>) {
    let (master, slave) = pty::open();
    assert!(pty::echo(&slave), "a new terminal echoes");
    let child = command(
        w,
        case,
        &["connector", "add", "slack", "--workspace", "acme", "--yes"],
    )
    .stdin(Stdio::from(slave.try_clone().unwrap()))
    .spawn()
    .unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    {
        let seen = Arc::clone(&seen);
        let mut reader = master.try_clone().unwrap();
        std::thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            // ends with EIO once no slave end is open
            while let Ok(n) = reader.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                seen.lock().unwrap().extend_from_slice(&chunk[..n]);
            }
        });
    }
    // the command turns echo off on its stdin before it reads; the line is typed only after that, as a person would
    let deadline = Instant::now() + Duration::from_secs(10);
    while pty::echo(&slave) {
        if Instant::now() > deadline {
            let mut child = child;
            let _ = child.kill();
            let out = child.wait_with_output().unwrap();
            panic!(
                "echo was never turned off on the terminal stdin; stderr: {:?}",
                text(&out.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    (child, master, slave, seen)
}

/// T066 AC2: `/dev/tty` cannot be opened but stdin is a terminal: as Python's `unix_getpass`, echo is off on stdin while
/// the line is read (the prompt on stderr, no fallback warning), and back on after it.
#[test]
fn with_no_dev_tty_a_terminal_stdin_has_echo_off_during_the_read_and_back_after() {
    let (w, case) = world(&[], json!({}), json!({}));
    let (child, mut master, slave, seen) = with_a_terminal_stdin(&w, &case);
    master.write_all(format!("{SECRET}\n").as_bytes()).unwrap();
    let out = finish(child, 20);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(pty::echo(&slave), "echo is back on after the read");
    let stderr = text(&out.stderr);
    assert_eq!(
        stderr, "Bot token (xoxb-...): \n",
        "the prompt and Python's line end, no fallback warning"
    );
    assert_eq!(stored_secret(&w), SECRET);
    drop(slave);
    std::thread::sleep(Duration::from_millis(100));
    let screen = text(&seen.lock().unwrap());
    assert!(!screen.contains(SECRET), "echoed: {screen:?}");
    assert!(!text(&out.stdout).contains(SECRET));
}

/// T066 AC2: a signal at that prompt restores echo on stdin (Python's `input ended` text, exit 2, nothing written).
#[test]
fn ctrl_c_with_a_terminal_stdin_restores_echo_and_writes_nothing() {
    let (w, case) = world(&[], json!({}), json!({}));
    let before = store_text(&w);
    let (child, _master, slave, _seen) = with_a_terminal_stdin(&w, &case);
    // SAFETY: a signal to the child this test started
    assert_eq!(unsafe { kill(child.id() as i32, 2) }, 0);
    let out = finish(child, 20);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out.stderr));
    assert!(pty::echo(&slave), "echo is back on after the signal");
    assert!(
        text(&out.stderr).ends_with("easy-personal-rs: input ended at the prompt; nothing was written\n"),
        "{}",
        text(&out.stderr)
    );
    assert_eq!(w.keychain_after(), json!({}));
    assert_eq!(store_text(&w), before);
}

const CANNOT_RUN: &str = "#!/nonexistent/t066-interpreter\n";

/// T066 AC3: `adopt-legacy` with a `claude` that cannot run (it does not start) says so and adopts nothing; with a
/// `claude` that ran and answered "no entry" (exit 1, or 127 from the program itself) the old lines stay.
#[test]
fn adopt_legacy_says_when_claude_could_not_run_and_adopts_nothing() {
    let items = json!({"easy-personal/slack": "{}"});
    let (w, case) = world(&[], items.clone(), json!({}));
    std::fs::write(w.dir.join("bin/claude"), CANNOT_RUN).unwrap();
    let before = store_text(&w);
    let out = run(&w, &case, &["connector", "adopt-legacy", "--yes"], "");
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stdout));
    assert_eq!(
        text(&out.stdout),
        "the `claude` CLI could not run (it did not start or timed out); nothing was adopted.\n"
    );
    assert_eq!(store_text(&w), before, "no row adopted");
    assert_eq!(w.keychain_after(), items);

    for no_entry in [None, Some("#!/bin/sh\nexit 127\n")] {
        let (w, case) = world(&[], items.clone(), json!({}));
        if let Some(script) = no_entry {
            std::fs::write(w.dir.join("bin/claude"), script).unwrap();
        }
        let out = run(&w, &case, &["connector", "adopt-legacy", "--yes"], "");
        assert_eq!(out.status.code(), Some(0), "{}", text(&out.stdout));
        let stdout = text(&out.stdout);
        assert!(
            stdout.starts_with(
                "pre-workspace connectors found:\n  slack: keychain yes, Claude Code default account no\n"
            ),
            "{stdout}"
        );
        assert!(stdout.contains("done: slack adopted into orphan.\n"), "{stdout}");
    }
}
