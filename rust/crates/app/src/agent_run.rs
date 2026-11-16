//! The agent run (T090, FR-101, FR-102): the app starts the owner's own `claude -p` for the one fixed job `investigate`,
//! billed to the Claude subscription login of Claude Code on this machine. The app never reads, copies or stores a
//! Claude credential: it only starts the installed program.
//!
//! - **Setting**: off by default; on is the flag file `<data dir>/agent-run.on` (mode 0600; the data folder is made 0700
//!   when missing). No store column. Only the CLI changes it (`agent-run on|off`); `off` also stops a going run, because
//!   the run checks the flag every [`POLL`].
//! - **One run at a time** on the machine: `flock` on `<data dir>/agent-run.lock`, taken without waiting. The kernel lets
//!   go of it when the holder exits or is killed, so a lock whose process is gone is taken over with no clean-up.
//! - **The child**: `claude` found on `PATH` (as `claude_cli.rs`), started with an argument list (never a shell), the
//!   prompt of the MCP `investigate` prompt (`mcp::investigate_text`, one source) as one argument, only this binary's
//!   `mcp` server (`--mcp-config`, `--strict-mcp-config`), only its five tools allowed, no built-in tool, permission
//!   mode `dontAsk` with prompts answered by no one (refused, never asked), no settings file (`--restricted`), in an
//!   empty folder the app owns (`<data dir>/agent-run`, made before and removed after the run), stdin, stdout and stderr
//!   on `/dev/null` (nothing of the output is read into this process, so none of it can be kept, logged or shown, and a
//!   full pipe can never block the child), in its own process group, killed with the whole group after [`LIMIT`].
//!   `PR_SET_PDEATHSIG`: if this process dies (Ctrl-C, a kill), the kernel kills `claude` too; its `mcp` server then
//!   reads the end of its stdin and exits. The signal follows the *thread* that started the child: start it from a
//!   thread that outlives the run.
//! - **Environment**: this process's, minus `CLAUDE_CONFIG_DIR`, every `ANTHROPIC_*` variable and the other variables
//!   in [`DROPPED`] (an API key, a token or another provider would bill something else than the subscription); plus
//!   `CLAUDE_CONFIG_DIR=<dir>` for an account that is not the default one, as `claude_cli.rs` runs `claude`.
//! - **Outcome**: start, end, [`Ended`], and the id of the newest report saved after the start for the run's scope.

use crate::claude_cli::{kill_group, which};
use clap::Subcommand;
use easy_personal_core::scope::account_config_dir_env;
use easy_personal_core::store::{open_checked, store_path};
use serde_json::{Map, Value, json};
use std::ffi::{OsStr, OsString};
use std::fs::{self, DirBuilder, File, OpenOptions, Permissions, TryLockError};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

/// Printed by `agent-run on` before the flag is written (FR-101).
pub const NOTICE: &str = "Agent runs use the Claude subscription login of Claude Code on this machine and count against your plan limits.";
/// The longest a run may take; then its whole process group is killed.
pub const LIMIT: Duration = Duration::from_secs(600);
/// How often a going run checks its child, the limit and the flag.
const POLL: Duration = Duration::from_millis(50);
const FLAG: &str = "agent-run.on";
const LOCK: &str = "agent-run.lock";
const WORK: &str = "agent-run";
/// The MCP server's name in the run: the prompt's last step names it ("reconnect the easy-personal server").
const SERVER: &str = "easy-personal";
/// The tools the `investigate` prompt names: the only ones the run may use.
const TOOLS: [&str; 5] = [
    "warnings",
    "improvement_plan",
    "usage_breakdown",
    "session_outliers",
    "save_report",
];
/// Removed from the child's environment besides every `ANTHROPIC_*` variable and `CLAUDE_CONFIG_DIR`: each one makes
/// `claude` bill an API key, a token or another provider instead of the subscription login.
const DROPPED: [&str; 5] = [
    "AWS_BEARER_TOKEN_BEDROCK",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
];

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    Ok,
    Failed,
    TimedOut,
    Stopped,
    ClaudeNotFound,
    Off,
}

impl Ended {
    pub fn word(self) -> &'static str {
        match self {
            Ended::Ok => "ok",
            Ended::Failed => "failed",
            Ended::TimedOut => "timed out",
            Ended::Stopped => "stopped",
            Ended::ClaudeNotFound => "claude not found",
            Ended::Off => "off",
        }
    }
}

/// The one record a run leaves, in memory only: start and end (UTC, seconds), how it ended, the report it saved.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    pub start_utc: String,
    pub end_utc: String,
    pub ended: Ended,
    pub report: Option<i64>,
}

impl Outcome {
    pub fn line(&self) -> String {
        let report = self.report.map_or("none".to_string(), |id| id.to_string());
        format!(
            "investigate: {}, started {}, ended {}, report {report}",
            self.ended.word(),
            self.start_utc,
            self.end_utc
        )
    }
}

/// Why a run was not started.
#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// Another run holds the lock.
    Going,
    /// The account is unknown or not a Claude account, or the store cannot be read for it (fixed texts, no input).
    Scope(String),
    /// The data folder, the lock or the working folder could not be made (the error kind only, never a path).
    Io(io::ErrorKind),
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refused::Going => write!(f, "a run is going"),
            Refused::Scope(text) => write!(f, "{text}"),
            Refused::Io(kind) => write!(f, "cannot prepare the run: {kind:?}"),
        }
    }
}

fn utc_now() -> String {
    jiff::Timestamp::now()
        .strftime("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

/// The setting, the lock and the runner over one data folder.
pub struct Runner {
    /// The store's folder (`~/.easy-personal`, or `EASY_PERSONAL_HOME`).
    dir: PathBuf,
    /// This process's environment (the child's is made from it; `PATH` finds `claude`).
    env: Vec<(OsString, OsString)>,
    /// This program: the run's only MCP server is its `mcp` command.
    exe: PathBuf,
    limit: Duration,
}

impl Runner {
    /// The real one: the store's folder, this process's environment and program, the 10 minute limit.
    pub fn system() -> Result<Runner, String> {
        let store = store_path().map_err(|e| e.to_string())?;
        // absolute: the child runs in its own folder, and its MCP server gets this folder as `EASY_PERSONAL_HOME`
        let dir = store
            .parent()
            .and_then(|dir| std::path::absolute(dir).ok())
            .ok_or("no data folder")?;
        let exe = std::env::current_exe()
            .map_err(|e| format!("cannot find this program: {:?}", e.kind()))?;
        Ok(Runner {
            dir,
            env: std::env::vars_os().collect(),
            exe,
            limit: LIMIT,
        })
    }

    fn store(&self) -> PathBuf {
        self.dir.join("store.sqlite")
    }

    pub fn enabled(&self) -> bool {
        fs::symlink_metadata(self.dir.join(FLAG)).is_ok_and(|m| m.is_file())
    }

    /// Writes (owner-only) or removes the flag; either again is no error.
    pub fn turn(&self, on: bool) -> io::Result<()> {
        let flag = self.dir.join(FLAG);
        if !on {
            return match fs::remove_file(flag) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                other => other,
            };
        }
        self.make_dir()?;
        let file = owner_only(&flag, false)?;
        file.set_permissions(Permissions::from_mode(0o600))
    }

    /// The data folder, made 0700 (with its parents) when missing; one that exists is used as it is.
    fn make_dir(&self) -> io::Result<()> {
        match DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&self.dir)
        {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(()),
            other => other,
        }
    }

    fn var(&self, name: &str) -> Option<&OsStr> {
        self.env
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_os_str())
    }

    pub fn claude(&self) -> Option<PathBuf> {
        which(self.var("PATH"), "claude")
    }

    /// Another process (or thread) holds the lock. Makes no file.
    pub fn running(&self) -> bool {
        let path = self.dir.join(LOCK);
        fs::symlink_metadata(&path).is_ok_and(|m| m.is_file())
            && matches!(
                owner_only(&path, true).map(|f| f.try_lock()),
                Ok(Err(TryLockError::WouldBlock))
            )
    }

    /// Runs the job once in the foreground: `Ended::Off` when the setting is off (nothing starts), [`Refused`] when the
    /// account is unknown or a run is going. An empty `workspace` or `account` is none, as the MCP prompt reads it.
    pub fn run(&self, workspace: Option<&str>, account: Option<&str>) -> Result<Outcome, Refused> {
        let start_utc = utc_now();
        let workspace = workspace.filter(|s| !s.is_empty());
        let account = account.filter(|s| !s.is_empty());
        let outcome = |ended, report| Outcome {
            start_utc: start_utc.clone(),
            end_utc: utc_now(),
            ended,
            report,
        };
        if !self.enabled() {
            return Ok(outcome(Ended::Off, None));
        }
        let config_dir = match account {
            None => None,
            Some(account) => {
                let conn =
                    open_checked(&self.store()).map_err(|e| Refused::Scope(e.to_string()))?;
                account_config_dir_env(&conn, account).map_err(|e| Refused::Scope(e.to_string()))?
            }
        };
        self.make_dir().map_err(|e| Refused::Io(e.kind()))?;
        let lock = owner_only(&self.dir.join(LOCK), false).map_err(|e| Refused::Io(e.kind()))?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(Refused::Going),
            Err(TryLockError::Error(e)) => return Err(Refused::Io(e.kind())),
        }
        let Some(claude) = self.claude() else {
            return Ok(outcome(Ended::ClaudeNotFound, None));
        };
        let before = self.newest_report(0, workspace, account).unwrap_or(0);
        let work = self.dir.join(WORK);
        let _ = fs::remove_dir_all(&work); // left by a run that was killed: emptied before this one
        DirBuilder::new()
            .mode(0o700)
            .create(&work)
            .map_err(|e| Refused::Io(e.kind()))?;
        let mut cmd = Command::new(claude);
        cmd.args(self.argv(workspace, account))
            .current_dir(&work)
            .env_clear()
            .envs(self.child_env(config_dir.as_deref()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0); // its own group: the limit and `off` kill what it started too
        let parent = std::process::id() as libc::pid_t;
        // SAFETY: only async-signal-safe calls (`prctl`, `getppid`) between fork and exec, and no allocation
        unsafe {
            cmd.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL as libc::c_ulong) == -1 {
                    return Err(io::Error::last_os_error());
                }
                if libc::getppid() != parent {
                    return Err(io::Error::from_raw_os_error(libc::ESRCH)); // this process died before the prctl
                }
                Ok(())
            })
        };
        let ended = match cmd.spawn() {
            Ok(mut child) => self.wait(&mut child),
            Err(_) => Ended::Failed,
        };
        let _ = fs::remove_dir_all(&work);
        let report = self.newest_report(before, workspace, account);
        drop(lock);
        Ok(outcome(ended, report))
    }

    fn wait(&self, child: &mut Child) -> Ended {
        let deadline = Instant::now() + self.limit;
        loop {
            match child.try_wait() {
                Ok(Some(status)) if status.success() => return Ended::Ok,
                Ok(Some(_)) => return Ended::Failed,
                Ok(None) => {}
                Err(_) => {
                    kill_group(child);
                    return Ended::Failed;
                }
            }
            if !self.enabled() {
                kill_group(child);
                return Ended::Stopped;
            }
            if Instant::now() >= deadline {
                kill_group(child);
                return Ended::TimedOut;
            }
            std::thread::sleep(POLL);
        }
    }

    /// The newest report with an id above `after` saved for exactly this scope; `None` with no store or no such row.
    fn newest_report(
        &self,
        after: i64,
        workspace: Option<&str>,
        account: Option<&str>,
    ) -> Option<i64> {
        let conn = open_checked(&self.store()).ok()?;
        conn.query_row(
            "SELECT MAX(id) FROM investigate_reports WHERE id > ?1 AND workspace IS ?2 AND account IS ?3",
            rusqlite::params![after, workspace, account],
            |r| r.get(0),
        )
        .ok()
        .flatten()
    }

    /// The whole argument list after `claude`: one argument per value, the prompt as one.
    fn argv(&self, workspace: Option<&str>, account: Option<&str>) -> Vec<String> {
        let mut scope = Map::new();
        for (key, value) in [("workspace", workspace), ("account", account)] {
            if let Some(value) = value {
                scope.insert(key.into(), Value::String(value.into()));
            }
        }
        let prompt = crate::mcp::investigate_text(&scope).expect("a string scope is always valid");
        let config = json!({"mcpServers": {SERVER: {
            "type": "stdio",
            "command": self.exe.to_string_lossy(),
            "args": ["mcp"],
            // the same store as this process, whatever environment `claude` gives its MCP servers
            "env": {"EASY_PERSONAL_HOME": self.dir.to_string_lossy()},
        }}});
        let allowed: Vec<String> = TOOLS
            .iter()
            .map(|t| format!("mcp__{SERVER}__{t}"))
            .collect();
        [
            "-p",
            &prompt,
            "--mcp-config",
            &config.to_string(),
            "--strict-mcp-config",
            "--allowedTools",
            &allowed.join(","),
            "--tools",
            "",
            "--permission-mode",
            "dontAsk",
            "--permission-prompts",
            "none",
            "--restricted",
        ]
        .map(str::to_string)
        .to_vec()
    }

    fn child_env(&self, config_dir: Option<&str>) -> Vec<(OsString, OsString)> {
        let dropped = |name: &OsStr| {
            name == "CLAUDE_CONFIG_DIR"
                || name.as_encoded_bytes().starts_with(b"ANTHROPIC_")
                || DROPPED.iter().any(|d| name == *d)
        };
        let mut env: Vec<(OsString, OsString)> = self
            .env
            .iter()
            .filter(|(k, _)| !dropped(k))
            .cloned()
            .collect();
        if let Some(dir) = config_dir {
            env.push(("CLAUDE_CONFIG_DIR".into(), dir.into()));
        }
        env
    }
}

/// Opens (or makes, mode 0600) a file of the data folder; a symlink there is refused (`O_NOFOLLOW`).
fn owner_only(path: &std::path::Path, existing: bool) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(!existing)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
}

#[derive(Subcommand)]
pub enum Action {
    /// Turn agent runs on (prints what they use first)
    On,
    /// Turn agent runs off, and stop a run that is going
    Off,
    /// Print on or off, whether `claude` is found and whether a run is going
    Status,
    /// Run the `investigate` job now in the foreground and print its outcome (exit 0 only for `ok`)
    Now {
        /// The workspace the report is for
        #[arg(long)]
        workspace: Option<String>,
        /// The agent account (id or label) whose Claude Code runs it
        #[arg(long)]
        account: Option<String>,
    },
}

pub fn main(action: Action) -> ExitCode {
    match Runner::system() {
        Ok(runner) => ExitCode::from(command(
            &runner,
            action,
            &mut io::stdout(),
            &mut io::stderr(),
        )),
        Err(e) => {
            eprintln!("easy-personal-rs: {e}");
            ExitCode::FAILURE
        }
    }
}

fn command(r: &Runner, action: Action, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let mut refuse = |text: String| {
        let _ = writeln!(err, "easy-personal-rs: {text}");
        1
    };
    let done = match action {
        Action::On => {
            let _ = writeln!(out, "{NOTICE}");
            r.turn(true).map(|()| "agent run: on".to_string())
        }
        Action::Off => r.turn(false).map(|()| "agent run: off".to_string()),
        Action::Status => Ok(format!(
            "agent run: {}\nclaude: {}\nrun going: {}",
            if r.enabled() { "on" } else { "off" },
            if r.claude().is_some() {
                "found"
            } else {
                "not found"
            },
            if r.running() { "yes" } else { "no" }
        )),
        Action::Now { workspace, account } => {
            if !r.enabled() {
                return refuse(
                    "agent runs are off; turn them on with `easy-personal-rs agent-run on`".into(),
                );
            }
            return match r.run(workspace.as_deref(), account.as_deref()) {
                Ok(outcome) => {
                    let _ = writeln!(out, "{}", outcome.line());
                    u8::from(outcome.ended != Ended::Ok)
                }
                Err(e) => refuse(e.to_string()),
            };
        }
    };
    match done {
        Ok(text) => {
            let _ = writeln!(out, "{text}");
            0
        }
        Err(e) => refuse(format!("cannot change the setting: {:?}", e.kind())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use serde_json::{Map, Value};
    use std::os::unix::fs::PermissionsExt;
    use std::time::Instant;

    /// The lock is `flock`, which belongs to the open file description: a child that another test's thread forks while
    /// a run holds the lock keeps it until that child calls `exec` (std forks, not `posix_spawn`, when `pre_exec` is
    /// set). The tests of this module take the lock, so they run one at a time. The CLI has one thread, so this race
    /// is a test-only one there; a multi-threaded caller (T091) sees at most a short "a run is going".
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn serial() -> std::sync::MutexGuard<'static, ()> {
        SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Planted in the fake's stdout and stderr (AC 7).
    const SECRET: &str = "sk-ant-T090-planted-0123456789abcdefABCDEF0123456789";
    const MESSAGE: &str = "T090 planted message text: the model says hello";
    const BILLING: [&str; 14] = [
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_PROFILE",
        "ANTHROPIC_BASE_URL",
        "ANTHROPIC_FEDERATION_RULE_ID",
        "ANTHROPIC_ORGANIZATION_ID",
        "ANTHROPIC_FOUNDRY_API_KEY",
        "ANTHROPIC_FOUNDRY_AUTH_TOKEN",
        "ANTHROPIC_AWS_API_KEY",
        "AWS_BEARER_TOKEN_BEDROCK",
        "CLAUDE_CODE_OAUTH_TOKEN",
        "CLAUDE_CODE_USE_BEDROCK",
        "CLAUDE_CODE_USE_VERTEX",
        "CLAUDE_CODE_USE_FOUNDRY",
    ];

    /// A temp root with `bin/claude` (a fake that logs its argv, working folder, folder listing and environment, NUL
    /// separated, to `log/`, then runs `tail`), the data folder `data/` (not made yet) and a runner over them.
    struct Fake {
        root: PathBuf,
    }

    impl Fake {
        fn new(tag: &str, tail: &str) -> Fake {
            let root = std::env::temp_dir().join(format!(
                "easy-personal-rs-t090-{tag}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(root.join("bin")).unwrap();
            std::fs::create_dir_all(root.join("log")).unwrap();
            let log = root.join("log");
            let script = format!(
                "#!/bin/sh\nL='{log}'\nfor a in \"$@\"; do printf '%s\\0' \"$a\" >> \"$L/argv\"; done\n\
                 pwd -P > \"$L/cwd\"; ls -A | wc -l > \"$L/entries\"; env -0 > \"$L/env\"\n\
                 echo $$ > \"$L/pid\"\n{tail}\n",
                log = log.display()
            );
            let exe = root.join("bin/claude");
            std::fs::write(&exe, script).unwrap();
            std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
            Fake { root }
        }

        fn dir(&self) -> PathBuf {
            self.root.join("data")
        }

        fn runner_with(&self, extra: &[(&str, &str)], limit: Duration) -> Runner {
            let mut env: Vec<(OsString, OsString)> = vec![
                (
                    "PATH".into(),
                    format!(
                        "/nonexistent-t090:{}:/usr/bin:/bin",
                        self.root.join("bin").display()
                    )
                    .into(),
                ),
                ("HOME".into(), self.root.join("home").into()),
            ];
            env.extend(
                extra
                    .iter()
                    .map(|(k, v)| (OsString::from(k), OsString::from(v))),
            );
            Runner {
                dir: self.dir(),
                env,
                exe: PathBuf::from("/opt/t090/easy-personal-rs"),
                limit,
            }
        }

        fn runner(&self) -> Runner {
            self.runner_with(&[], Duration::from_secs(30))
        }

        fn log(&self, name: &str) -> String {
            std::fs::read_to_string(self.root.join("log").join(name)).unwrap_or_default()
        }

        fn argv(&self) -> Vec<String> {
            self.log("argv")
                .split_terminator('\0')
                .map(str::to_string)
                .collect()
        }

        fn child_env(&self) -> Vec<(String, String)> {
            self.log("env")
                .split_terminator('\0')
                .filter_map(|kv| kv.split_once('='))
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        }

        fn wait_for(&self, name: &str) {
            let deadline = Instant::now() + Duration::from_secs(20);
            while !self.root.join("log").join(name).exists() {
                assert!(Instant::now() < deadline, "the fake never wrote {name}");
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        /// A fresh v11 store in the data folder with two confirmed Claude accounts: 1 `home` (the default, no
        /// `CLAUDE_CONFIG_DIR`) and 2 `work` (`/acc/.claude-work`), plus `odd` (3) whose label holds shell text.
        fn store(&self) -> Connection {
            std::fs::create_dir_all(self.dir()).unwrap();
            drop(
                easy_personal_core::writer::open_for_write(&self.dir().join("store.sqlite"))
                    .unwrap(),
            );
            let conn = Connection::open(self.dir().join("store.sqlite")).unwrap();
            conn.execute_batch(
                "INSERT INTO agent_accounts (id, provider, folder, label, status, config_dir_env) VALUES \
                 (1, 'claude', '/acc/.claude', 'home', 'confirmed', NULL), \
                 (2, 'claude', '/acc/.claude-work', 'work', 'confirmed', '/acc/.claude-work'), \
                 (3, 'claude', '/acc/.claude-odd', '-o''d\"d $(x)\nq', 'confirmed', '/acc/.claude-odd'), \
                 (4, 'codex', '/acc/.codex', 'cx', 'confirmed', NULL);",
            )
            .unwrap();
            conn
        }
    }

    impl Drop for Fake {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn insert_report(conn: &Connection, workspace: Option<&str>) -> i64 {
        conn.execute(
            "INSERT INTO investigate_reports (saved_utc, server_version, workspace, account, summary, fixes_json, \
             snapshot_json) VALUES ('2026-11-16T00:00:00Z', 't', ?1, NULL, 's', '[]', '{}')",
            [workspace],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn cli(r: &Runner, action: Action) -> (u8, String, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = command(r, action, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    fn now(workspace: Option<&str>, account: Option<&str>) -> Action {
        Action::Now {
            workspace: workspace.map(str::to_string),
            account: account.map(str::to_string),
        }
    }

    /// Gone, or a zombie no one has reaped yet: either way it runs no more.
    fn runs(pid: &str) -> bool {
        let stat = format!("/proc/{}/stat", pid.trim());
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let live = std::fs::read_to_string(&stat)
                .ok()
                .and_then(|s| {
                    s.rsplit(')')
                        .next()
                        .map(|r| !r.trim_start().starts_with('Z'))
                })
                .unwrap_or(false);
            if !live || Instant::now() > deadline {
                return live;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn scope_map(workspace: Option<&str>, account: Option<&str>) -> Map<String, Value> {
        let mut map = Map::new();
        for (key, value) in [("workspace", workspace), ("account", account)] {
            if let Some(value) = value {
                map.insert(key.into(), Value::String(value.into()));
            }
        }
        map
    }

    // ---- AC 1: the setting ----

    #[test]
    fn the_setting_is_off_by_default_on_writes_an_owner_only_flag_off_removes_it() {
        let _serial = serial();
        let fake = Fake::new("setting", "exit 0");
        let r = fake.runner();
        assert!(!r.enabled());
        let (code, out, _) = cli(&r, Action::Status);
        assert_eq!(
            (code, out.as_str()),
            (0, "agent run: off\nclaude: found\nrun going: no\n")
        );
        assert!(!fake.dir().exists(), "status writes nothing");

        let (code, out, err) = cli(&r, Action::On);
        assert_eq!((code, err.as_str()), (0, ""));
        assert_eq!(out, format!("{NOTICE}\nagent run: on\n"));
        assert!(r.enabled());
        let flag = std::fs::metadata(fake.dir().join("agent-run.on")).unwrap();
        assert_eq!(flag.permissions().mode() & 0o777, 0o600);
        let dir = std::fs::metadata(fake.dir()).unwrap();
        assert_eq!(dir.permissions().mode() & 0o777, 0o700);
        assert_eq!(cli(&r, Action::On).0, 0, "on twice is no error");
        let (_, out, _) = cli(&r, Action::Status);
        assert_eq!(out, "agent run: on\nclaude: found\nrun going: no\n");

        assert_eq!(
            cli(&r, Action::Off),
            (0, "agent run: off\n".into(), String::new())
        );
        assert!(!r.enabled() && !fake.dir().join("agent-run.on").exists());
        assert_eq!(cli(&r, Action::Off).0, 0, "off twice is no error");

        std::fs::remove_file(fake.root.join("bin/claude")).unwrap();
        let (_, out, _) = cli(&r, Action::Status);
        assert_eq!(out, "agent run: off\nclaude: not found\nrun going: no\n");
    }

    // ---- AC 2: the argument list ----

    fn expected_argv(r: &Runner, prompt: &str) -> Vec<String> {
        let config = format!(
            r#"{{"mcpServers":{{"easy-personal":{{"args":["mcp"],"command":"/opt/t090/easy-personal-rs","env":{{"EASY_PERSONAL_HOME":"{}"}},"type":"stdio"}}}}}}"#,
            r.dir.display()
        );
        [
            "-p",
            prompt,
            "--mcp-config",
            &config,
            "--strict-mcp-config",
            "--allowedTools",
            "mcp__easy-personal__warnings,mcp__easy-personal__improvement_plan,mcp__easy-personal__usage_breakdown,mcp__easy-personal__session_outliers,mcp__easy-personal__save_report",
            "--tools",
            "",
            "--permission-mode",
            "dontAsk",
            "--permission-prompts",
            "none",
            "--restricted",
        ]
        .map(str::to_string)
        .to_vec()
    }

    #[test]
    fn the_argv_is_one_list_with_the_mcp_prompt_only_this_binary_and_the_five_tools() {
        let _serial = serial();
        let fake = Fake::new("argv", "exit 0");
        let r = fake.runner();
        r.turn(true).unwrap();
        let got = r.run(None, None).unwrap();
        assert_eq!((got.ended, got.report), (Ended::Ok, None));
        let prompt = crate::mcp::investigate_text(&Map::new()).unwrap();
        let argv = fake.argv();
        assert_eq!(argv, expected_argv(&r, &prompt));
        assert!(
            !argv
                .iter()
                .any(|a| a == "--bare" || a.contains("dangerously"))
        );
        let config: Value = serde_json::from_str(&argv[3]).unwrap();
        let servers = config["mcpServers"].as_object().unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(
            servers["easy-personal"]["command"],
            "/opt/t090/easy-personal-rs"
        );
        assert_eq!(servers["easy-personal"]["args"], serde_json::json!(["mcp"]));
        let allowed: Vec<&str> = argv[6].split(',').collect();
        assert_eq!(allowed.len(), 5);
        for tool in TOOLS {
            assert!(allowed.contains(&format!("mcp__{SERVER}__{tool}").as_str()));
        }
        // the working folder: the app's own, empty while the fake ran, gone after the run
        let work = fake
            .dir()
            .join("agent-run")
            .canonicalize()
            .unwrap_or(fake.dir().join("agent-run"));
        assert_eq!(
            fake.log("cwd").trim(),
            std::fs::canonicalize(fake.dir())
                .unwrap()
                .join("agent-run")
                .display()
                .to_string()
        );
        assert_eq!(fake.log("entries").trim(), "0");
        assert!(!work.exists());
    }

    // ---- AC 3: the environment ----

    #[test]
    fn billing_variables_never_reach_the_child_and_the_account_sets_claude_config_dir() {
        let _serial = serial();
        let fake = Fake::new("env", "exit 0");
        let mut planted: Vec<(&str, &str)> = BILLING.iter().map(|k| (*k, "planted")).collect();
        planted.push(("CLAUDE_CONFIG_DIR", "/parent/.claude-x"));
        planted.push(("ANTHROPIC_VERTEX_PROJECT_ID", "planted"));
        planted.push(("T090_KEEP", "kept"));
        let r = fake.runner_with(&planted, Duration::from_secs(30));
        drop(fake.store());
        r.turn(true).unwrap();
        for (account, want) in [
            (None, None),
            (Some("home"), None),
            (Some("work"), Some("/acc/.claude-work")),
            (Some("2"), Some("/acc/.claude-work")),
        ] {
            let _ = std::fs::remove_file(fake.root.join("log/env"));
            assert_eq!(
                r.run(None, account).unwrap().ended,
                Ended::Ok,
                "{account:?}"
            );
            let env = fake.child_env();
            for name in BILLING.iter().chain(["ANTHROPIC_VERTEX_PROJECT_ID"].iter()) {
                assert!(
                    !env.iter().any(|(k, _)| k == name),
                    "{name} reached the child"
                );
            }
            assert!(env.iter().any(|(k, v)| k == "T090_KEEP" && v == "kept"));
            let ccd = env
                .iter()
                .find(|(k, _)| k == "CLAUDE_CONFIG_DIR")
                .map(|(_, v)| v.as_str());
            assert_eq!(ccd, want, "{account:?}");
        }
        // an unknown account, and a Codex one, start nothing
        std::fs::remove_file(fake.root.join("log/pid")).unwrap();
        for account in ["nope", "cx"] {
            assert!(
                matches!(r.run(None, Some(account)), Err(Refused::Scope(_))),
                "{account}"
            );
        }
        assert!(!fake.root.join("log/pid").exists());
    }

    // ---- AC 4: the scope is data inside the prompt ----

    #[test]
    fn a_scope_with_shell_text_reaches_the_child_only_inside_the_prompt() {
        let _serial = serial();
        let fake = Fake::new("scope", "exit 0");
        let r = fake.runner();
        drop(fake.store());
        r.turn(true).unwrap();
        let odd = "-o'd\"d $(x)\nq";
        let workspace = "-w'o\"rk $(x)\n; rm -rf /";
        assert_eq!(r.run(Some(workspace), Some(odd)).unwrap().ended, Ended::Ok);
        let argv = fake.argv();
        let prompt = crate::mcp::investigate_text(&scope_map(Some(workspace), Some(odd))).unwrap();
        assert_eq!(
            argv,
            expected_argv(&r, &prompt),
            "nothing else in argv changes"
        );
        assert!(argv[1].contains(&serde_json::to_string(workspace).unwrap()));
        assert!(argv[1].contains(&serde_json::to_string(odd).unwrap()));
        let env = fake.child_env();
        assert!(
            env.iter()
                .any(|(k, v)| k == "CLAUDE_CONFIG_DIR" && v == "/acc/.claude-odd")
        );
    }

    // ---- AC 5: one run at a time ----

    #[test]
    fn a_second_start_while_a_run_is_going_starts_nothing() {
        let _serial = serial();
        let fake = Fake::new(
            "one",
            "while [ ! -e \"$L/go\" ]; do sleep 0.02; done; exit 0",
        );
        let r = fake.runner();
        r.turn(true).unwrap();
        std::thread::scope(|s| {
            let first = s.spawn(|| r.run(None, None));
            fake.wait_for("pid");
            assert!(r.running());
            let other = fake.runner();
            assert_eq!(other.run(None, None), Err(Refused::Going));
            let (code, _, err) = cli(&other, now(None, None));
            assert_eq!(
                (code, err.as_str()),
                (1, "easy-personal-rs: a run is going\n")
            );
            assert_eq!(
                cli(&other, Action::Status).1,
                "agent run: on\nclaude: found\nrun going: yes\n"
            );
            assert_eq!(fake.log("pid").lines().count(), 1, "one process started");
            std::fs::write(fake.root.join("log/go"), "").unwrap();
            assert_eq!(first.join().unwrap().unwrap().ended, Ended::Ok);
        });
        assert!(!r.running());
        assert_eq!(
            r.run(None, None).unwrap().ended,
            Ended::Ok,
            "after the first ends, a start works"
        );
    }

    #[test]
    fn a_lock_left_by_a_process_that_is_gone_is_taken_over() {
        let _serial = serial();
        let fake = Fake::new("stale", "exit 0");
        let r = fake.runner();
        r.turn(true).unwrap();
        // a process took the lock and died holding it: the kernel let go of it, the file stays
        let held = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!(
                "exec 9>>'{}/agent-run.lock'; flock -n 9 && echo 99999 >&9",
                fake.dir().display()
            ))
            .status()
            .unwrap();
        assert!(held.success());
        assert!(fake.dir().join("agent-run.lock").exists());
        assert!(!r.running());
        assert_eq!(r.run(None, None).unwrap().ended, Ended::Ok);
    }

    // ---- AC 6: the limit and the stop ----

    const GROUP: &str = "sleep 30 & echo $! > \"$L/grandchild\"; wait";

    #[test]
    fn a_run_past_the_limit_is_killed_with_its_whole_group() {
        let _serial = serial();
        let fake = Fake::new("limit", GROUP);
        let r = fake.runner_with(&[], Duration::from_secs(2));
        r.turn(true).unwrap();
        let started = Instant::now();
        let got = r.run(None, None).unwrap();
        assert_eq!(got.ended, Ended::TimedOut);
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "{:?}",
            started.elapsed()
        );
        for file in ["pid", "grandchild"] {
            assert!(!runs(&fake.log(file)), "{file} still runs");
        }
        assert!(!r.running());
    }

    #[test]
    fn off_during_a_run_stops_it_and_no_process_of_its_group_lives_on() {
        let _serial = serial();
        let fake = Fake::new("stop", GROUP);
        let r = fake.runner();
        r.turn(true).unwrap();
        std::thread::scope(|s| {
            let run = s.spawn(|| r.run(None, None));
            fake.wait_for("grandchild");
            // another process turns it off
            assert_eq!(cli(&fake.runner(), Action::Off).0, 0);
            let got = run.join().unwrap().unwrap();
            assert_eq!(got.ended, Ended::Stopped);
        });
        for file in ["pid", "grandchild"] {
            assert!(!runs(&fake.log(file)), "{file} still runs");
        }
    }

    // ---- AC 7: nothing of the output is kept ----

    #[test]
    fn a_planted_secret_in_stdout_and_stderr_is_in_no_line_no_file_and_no_row() {
        let _serial = serial();
        let tail = format!(
            "for i in 1 2 3; do echo '{SECRET} {MESSAGE}'; echo '{SECRET} {MESSAGE}' >&2; done; \
             head -c 300000 /dev/zero | tr '\\0' 's'; exit 3"
        );
        let fake = Fake::new("leak", &tail);
        let r = fake.runner();
        drop(fake.store());
        assert_eq!(cli(&r, Action::On).0, 0);
        let (code, out, err) = cli(&r, now(None, Some("work")));
        assert_eq!(code, 1);
        let mut seen = format!("{out}{err}");
        assert!(out.starts_with("investigate: failed, started "), "{out}");
        let (code, out, err) = cli(&r, now(None, None));
        assert_eq!(code, 1);
        seen += &format!("{out}{err}");
        let (_, out, err) = cli(&r, Action::Status);
        seen += &format!("{out}{err}");
        for planted in [SECRET, MESSAGE, "sssss"] {
            assert!(!seen.contains(planted), "printed: {planted}");
        }
        let mut stack = vec![fake.dir()];
        let mut files = 0;
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    files += 1;
                    let bytes = std::fs::read(&path).unwrap();
                    let text = String::from_utf8_lossy(&bytes);
                    assert!(
                        !text.contains(SECRET) && !text.contains(MESSAGE),
                        "{}",
                        path.display()
                    );
                }
            }
        }
        assert!(files >= 3, "flag, lock and store were read: {files}");
        let conn = Connection::open(fake.dir().join("store.sqlite")).unwrap();
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM investigate_reports", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 0);
    }

    // ---- AC 8: the outcomes ----

    #[test]
    fn exit_0_with_a_report_saved_after_the_start_is_ok_with_that_id() {
        let _serial = serial();
        let fake = Fake::new(
            "report",
            "while [ ! -e \"$L/go\" ]; do sleep 0.02; done; exit 0",
        );
        let r = fake.runner();
        let conn = fake.store();
        let before = insert_report(&conn, None);
        r.turn(true).unwrap();
        std::thread::scope(|s| {
            let run = s.spawn(|| r.run(None, None));
            fake.wait_for("pid");
            let first = insert_report(&conn, None);
            let newest = insert_report(&conn, None);
            insert_report(&conn, Some("other")); // another scope: not this run's
            assert!(before < first && first < newest);
            std::fs::write(fake.root.join("log/go"), "").unwrap();
            let got = run.join().unwrap().unwrap();
            assert_eq!((got.ended, got.report), (Ended::Ok, Some(newest)));
            assert_eq!(
                got.line(),
                format!(
                    "investigate: ok, started {}, ended {}, report {newest}",
                    got.start_utc, got.end_utc
                )
            );
        });
        // a report saved before the start is never this run's
        let (code, out, _) = cli(&r, now(None, None));
        assert_eq!(code, 0);
        assert!(
            out.starts_with("investigate: ok, started ") && out.ends_with(", report none\n"),
            "{out}"
        );
    }

    #[test]
    fn a_non_zero_exit_is_failed_and_no_claude_is_claude_not_found() {
        let _serial = serial();
        let fake = Fake::new("fail", "exit 7");
        let r = fake.runner();
        r.turn(true).unwrap();
        assert_eq!(r.run(None, None).unwrap().ended, Ended::Failed);
        std::fs::remove_file(fake.root.join("bin/claude")).unwrap();
        let got = r.run(None, None).unwrap();
        assert_eq!((got.ended, got.report), (Ended::ClaudeNotFound, None));
        let (code, out, _) = cli(&r, now(None, None));
        assert_eq!(code, 1);
        assert!(
            out.starts_with("investigate: claude not found, started "),
            "{out}"
        );
    }

    #[test]
    fn with_the_setting_off_now_exits_1_with_one_line_and_starts_nothing() {
        let _serial = serial();
        let fake = Fake::new("off", "exit 0");
        let r = fake.runner();
        assert_eq!(r.run(None, None).unwrap().ended, Ended::Off);
        let (code, out, err) = cli(&r, now(None, None));
        assert_eq!((code, out.as_str()), (1, ""));
        assert_eq!(
            err,
            "easy-personal-rs: agent runs are off; turn them on with `easy-personal-rs agent-run on`\n"
        );
        assert!(!fake.root.join("log/pid").exists(), "no process started");
    }
}
