//! T099: the terminal `account ignore` of the agent run account turns the agent run off, as the dashboard's ignore route
//! does (T091): the flag file is removed, so `agent-run status` says off and a run already going stops at its next poll.
//! Every test runs the real binary over temp folders for `HOME`, `EASY_PERSONAL_HOME` and `EASY_PERSONAL_AGENT_ROOT`,
//! with a fake `claude` first on `PATH` (never the real one). The fake is a shell script like the one of
//! `agent_run_routes.rs`: `agent_run::tests::Fake` is `cfg(test)` inside the binary and cannot be reached from here.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// The fake `claude`: it writes its pid and its child's, then waits for that child (so the group has two processes).
const FAKE: &str = "#!/bin/sh\nL=\"$EASY_PERSONAL_HOME/log\"\nmkdir -p \"$L\"\necho $$ > \"$L/pid\"\nsleep 60 &\necho $! > \"$L/grandchild\"\nwait\n";

struct World {
    home: PathBuf,
    root: PathBuf,
}

impl World {
    /// A store with two confirmed Claude accounts: 1 `work` (`<root>/.claude-work`) and 2 `other` (`.claude-other`).
    fn new() -> World {
        static N: AtomicU32 = AtomicU32::new(0);
        let home = std::env::temp_dir().join(format!(
            "easy-personal-rs-t099-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let root = home.join("agent-root");
        for name in [".claude-work", ".claude-other"] {
            std::fs::create_dir_all(root.join(name)).unwrap();
        }
        std::fs::create_dir_all(home.join("bin")).unwrap();
        let claude = home.join("bin/claude");
        std::fs::write(&claude, FAKE).unwrap();
        std::fs::set_permissions(&claude, std::fs::Permissions::from_mode(0o755)).unwrap();
        let store = home.join("store.sqlite");
        drop(easy_personal_core::writer::open_for_write(&store).unwrap());
        let conn = rusqlite::Connection::open(&store).unwrap();
        for (id, folder, label) in [(1, ".claude-work", "work"), (2, ".claude-other", "other")] {
            let folder = root.join(folder);
            conn.execute(
                "INSERT INTO agent_accounts (id, provider, folder, label, status, config_dir_env) \
                 VALUES (?1, 'claude', ?2, ?3, 'confirmed', ?2)",
                rusqlite::params![id, folder.to_str().unwrap(), label],
            )
            .unwrap();
        }
        World { home, root }
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"));
        cmd.args(args)
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.home.join("bin").display()),
            )
            .env("HOME", &self.home)
            .env("EASY_PERSONAL_HOME", &self.home)
            .env("XDG_CONFIG_HOME", self.home.join("xdg"))
            .env("EASY_PERSONAL_AGENT_ROOT", &self.root)
            .env_remove("CLAUDE_CONFIG_DIR");
        cmd
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }

    fn flag(&self) -> Option<String> {
        std::fs::read_to_string(self.home.join("agent-run.on")).ok()
    }

    /// The first line of `agent-run status`.
    fn status(&self) -> String {
        let out = self.run(&["agent-run", "status"]);
        String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .to_string()
    }

    fn log(&self, name: &str) -> String {
        std::fs::read_to_string(self.home.join("log").join(name)).unwrap_or_default()
    }

    /// Turns the setting on for this account (id or label).
    fn on(&self, account: &str) {
        let out = self.run(&["agent-run", "on", "--account", account]);
        assert!(out.status.success(), "{out:?}");
    }
}

impl Drop for World {
    fn drop(&mut self) {
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

fn ignored_ok(world: &World, reference: &str) {
    let out = world.run(&["account", "ignore", reference]);
    assert!(out.status.success(), "{out:?}");
}

// AC 1
#[test]
fn ignoring_the_agent_run_account_removes_the_flag_and_the_setting_is_off() {
    let world = World::new();
    world.on("work");
    assert_eq!(world.flag().as_deref(), Some("1\n"));
    assert_eq!(world.status(), "agent run: on");
    ignored_ok(&world, "1");
    assert_eq!(world.flag(), None, "the flag is gone");
    assert_eq!(world.status(), "agent run: off");
}

// AC 2: the id comes from the row the ignore returned, not from the argument
#[test]
fn the_account_may_be_named_by_a_padded_id() {
    let world = World::new();
    world.on("work");
    ignored_ok(&world, "01");
    assert_eq!(world.flag(), None);
    assert_eq!(world.status(), "agent run: off");
}

// AC 2: `account ignore` takes an id or a folder path (not a label); the path is no id
#[test]
fn the_account_may_be_named_by_its_folder() {
    let world = World::new();
    world.on("work");
    let folder = world.root.join(".claude-work");
    ignored_ok(&world, folder.to_str().unwrap());
    assert_eq!(world.flag(), None);
    assert_eq!(world.status(), "agent run: off");
}

// AC 3
#[test]
fn another_accounts_flag_stays() {
    let world = World::new();
    world.on("other");
    ignored_ok(&world, "1");
    assert_eq!(world.flag().as_deref(), Some("2\n"));
    assert_eq!(world.status(), "agent run: on");
}

#[test]
fn the_default_accounts_empty_flag_stays() {
    let world = World::new();
    let out = world.run(&["agent-run", "on"]);
    assert!(out.status.success(), "{out:?}");
    ignored_ok(&world, "1");
    assert_eq!(world.flag().as_deref(), Some(""));
}

#[test]
fn no_flag_stays_no_flag() {
    let world = World::new();
    ignored_ok(&world, "1");
    assert_eq!(world.flag(), None);
    assert_eq!(world.status(), "agent run: off");
}

#[test]
fn a_failed_ignore_leaves_the_flag() {
    let world = World::new();
    world.on("work");
    let out = world.run(&["account", "ignore", "99"]);
    assert!(!out.status.success(), "{out:?}");
    assert_eq!(world.flag().as_deref(), Some("1\n"));
    assert_eq!(world.status(), "agent run: on");
}

// AC 4: `agent-run now` runs in the foreground; the ignore comes from a second process
#[test]
fn a_run_already_going_stops_at_its_next_poll_after_the_terminal_ignore() {
    let world = World::new();
    world.on("work");
    let mut run = world
        .command(&["agent-run", "now"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while world.log("grandchild").trim().is_empty() {
        assert!(Instant::now() < deadline, "the fake never started");
        std::thread::sleep(Duration::from_millis(20));
    }
    let (pid, grandchild) = (world.log("pid"), world.log("grandchild"));
    ignored_ok(&world, "1");
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = run.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = run.kill();
            panic!("the run was not stopped");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let out = run.wait_with_output().unwrap();
    let line = String::from_utf8(out.stdout).unwrap();
    assert!(!status.success(), "{status}");
    assert!(line.starts_with("investigate: stopped"), "{line}");
    assert!(gone(&pid) && gone(&grandchild), "claude still runs");
    assert_eq!(world.flag(), None);
}
