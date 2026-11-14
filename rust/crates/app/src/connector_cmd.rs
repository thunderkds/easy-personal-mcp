//! `easy-personal-rs connector list|add|remove|sync|move|alias|adopt-legacy` (T060): Python's `_connector`, `_connector_list`
//! and `_sync` (`surfaces/cli.py`). The oracle is `rust/parity/golden-connector-cli/`.
//!
//! No store, keychain or `claude` logic is written here. Each action calls what the T050 routes run (`connector_writes`,
//! `legacy`, the core's `sync` and `workspaces`) and T058's command frame (`cli_cmds`: the store sessions, the question
//! with no store handle open, the exit codes). This module reads the arguments, asks for the field values and prints.
//!
//! **Secrets** take Python's one route: the prompt. A secret field is read as `getpass.getpass` reads it: from the
//! controlling terminal (`/dev/tty`) with echo off, the prompt written there; with no terminal, from stdin after
//! `Warning: Password input may be echoed.` and the prompt on stderr. A plain field is `input()`: the prompt on stdout,
//! the line from stdin, stripped. No flag or environment variable carries a value. The value goes to the keychain only:
//! the plan shows `ENV=***`, and no line, error or `claude` argument holds it. End of input at a prompt is Python's
//! `input ended at the prompt; nothing was written` (exit 2), and nothing is written.

use crate::claude_cli::SystemClaude;
use crate::cli_cmds::{
    ABORTED, Run, Stop, ask, change, finish, open_read, put, put_lines, read_line, report,
    store_stop, write,
};
use crate::connector_writes::{self, Add};
use crate::{keychain, legacy};
use clap::Subcommand;
use easy_personal_core::store::StoreError;
use easy_personal_core::sync::{self, Step};
use easy_personal_core::workspaces::{self, Outcome, WriteFail, py_strip};
use easy_personal_core::{accounts, catalog};
use rusqlite::OptionalExtension;
use serde_json::Value;
use std::cell::Cell;
use std::process::ExitCode;

#[derive(Subcommand)]
pub enum ConnectorAction {
    /// List the provider catalog by category, then the stored connectors (no secret values)
    List,
    /// Store a connector in a workspace (asks each field; secrets go to the keychain only)
    Add {
        provider: String,
        /// Workspace name (see `workspace list`)
        #[arg(long)]
        workspace: String,
        /// Claude Code name (default <workspace>-<provider>)
        #[arg(long)]
        alias: Option<String>,
        /// Skip the confirmation question
        #[arg(long)]
        yes: bool,
    },
    /// Unregister a connector from every account, then delete it (a pre-workspace name such as slack too)
    Remove {
        /// Connector id (or a pre-workspace name such as slack)
        name: String,
        /// Skip the confirmation question
        #[arg(long)]
        yes: bool,
    },
    /// Show and apply each Claude account's add/remove plan
    Sync {
        /// Account id or folder (default: every confirmed Claude account)
        #[arg(long)]
        account: Option<String>,
        /// Skip the confirmation question
        #[arg(long)]
        yes: bool,
    },
    /// Move a connector to another workspace (orphan allowed)
    Move {
        /// Connector id
        id: String,
        /// Target workspace name
        workspace: String,
        /// Skip the confirmation question
        #[arg(long)]
        yes: bool,
    },
    /// Set a custom Claude Code name, or --default to reset it
    Alias {
        /// Connector id
        id: String,
        /// New alias
        alias: Option<String>,
        /// Reset to <workspace>-<provider>
        #[arg(long)]
        default: bool,
        /// Skip the confirmation question
        #[arg(long)]
        yes: bool,
    },
    /// Move pre-workspace connectors (slack, ...) into orphan
    AdoptLegacy {
        /// Skip the confirmation question
        #[arg(long)]
        yes: bool,
    },
}

pub fn run(action: ConnectorAction) -> ExitCode {
    finish(match action {
        ConnectorAction::List => list(),
        ConnectorAction::Add {
            provider,
            workspace,
            alias,
            yes,
        } => add(&provider, workspace, alias, yes),
        ConnectorAction::Remove { name, yes } => remove(&name, yes),
        ConnectorAction::Sync { account, yes } => sync_cmd(account.as_deref(), yes),
        ConnectorAction::Move { id, workspace, yes } => {
            let claude = SystemClaude::new();
            change(yes, ABORTED, false, |conn, apply| {
                let outcome = workspaces::move_connector(conn, &id, &workspace, apply, &claude)?;
                Ok((outcome, Value::Null))
            })
            .map(drop)
        }
        ConnectorAction::Alias {
            id,
            alias,
            default,
            yes,
        } => {
            if default == alias.is_some() {
                return finish(Err(Stop::refused("give either a new alias or --default")));
            }
            let claude = SystemClaude::new();
            change(yes, ABORTED, false, |conn, apply| {
                let outcome = workspaces::set_alias(conn, &id, alias.as_deref(), apply, &claude)?;
                Ok((outcome, Value::Null))
            })
            .map(drop)
        }
        ConnectorAction::AdoptLegacy { yes } => {
            let (kc, claude) = (keychain::system(), SystemClaude::new());
            change(yes, ABORTED, false, |conn, apply| {
                Ok((legacy::adopt(conn, apply, &*kc, &claude)?, Value::Null))
            })
            .map(drop)
        }
    })
}

fn refused(text: impl Into<String>) -> WriteFail {
    WriteFail::Refused {
        status: 400,
        text: text.into(),
    }
}

// ---- list ----

/// `_connector_list`: the catalog by category (official claude.ai connectors first), then the stored connectors.
fn list() -> Run {
    let conn = open_read()?;
    let mut entries = catalog::entries().map_err(|_| Stop::failed(1, crate::api::INTERNAL))?;
    entries.sort_by(|a, b| {
        (&a.category, !a.claude_ai_connector, &a.provider).cmp(&(
            &b.category,
            !b.claude_ai_connector,
            &b.provider,
        ))
    });
    let mut out = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        if i == 0 || entries[i - 1].category != e.category {
            out.push(e.category.clone());
        }
        let official = if e.claude_ai_connector {
            // the catalog parser requires the link with `yes`
            let link = e.claude_ai_link.as_deref().unwrap_or_default();
            format!("claude.ai connector: yes ({link})")
        } else {
            "claude.ai connector: no".to_string()
        };
        out.push(format!("  {} {} | {official}", e.id, e.provider));
        let labels: Vec<&str> = e.fields.iter().map(|f| f.label.as_str()).collect();
        out.push(format!("    fields: {}", labels.join(", ")));
    }
    let rows = (|| {
        conn.prepare(
            "SELECT c.id, c.alias, c.provider, w.name FROM connectors c \
             JOIN workspaces w ON w.id = c.workspace_id ORDER BY w.name, c.alias",
        )?
        .query_map([], |r| {
            Ok([r.get::<_, String>(0)?, r.get(1)?, r.get(2)?, r.get(3)?].join(" "))
        })?
        .collect::<Result<Vec<String>, _>>()
    })()
    .map_err(|e| store_stop(StoreError::from(e)))?;
    if !rows.is_empty() {
        out.push("configured (id alias provider workspace)".into());
    }
    out.extend(rows.into_iter().map(|row| format!("  {row}")));
    put_lines(&out)
}

// ---- add ----

const INPUT_ENDED: &str = "input ended at the prompt; nothing was written";

/// `catalog.load`: a shipped entry, or Python's text (it names the id the owner typed, on the owner's own terminal).
fn entry(provider: &str) -> Result<catalog::Entry, WriteFail> {
    let unknown = || {
        refused(format!(
            "unknown provider '{provider}'; available: {}",
            catalog::ids().join(", ")
        ))
    };
    let bytes = catalog::file(provider).ok_or_else(unknown)?;
    catalog::parse_bytes(provider, bytes).map_err(|e| refused(e.0))
}

/// `_ask`: a secret field through [`hidden::read`] (getpass), a plain one through `input()` (stripped).
fn ask_value(field: &catalog::Field) -> Result<String, WriteFail> {
    let prompt = format!("{}: ", field.label);
    let value = if field.secret {
        hidden::read(&prompt).map_err(refused)?
    } else {
        put(&prompt).ok().and_then(|()| read_line(MAX_VALUE))
    };
    value
        .map(|v| {
            if field.secret {
                v
            } else {
                py_strip(&v).to_string()
            }
        })
        .ok_or_else(|| refused(INPUT_ENDED))
}

/// The longest field value read from one line (a token or a file path is far shorter).
const MAX_VALUE: u64 = 64 * 1024;

/// `writer.add_to_workspace` with the CLI's prompts and question, in one store session (the connector id and the typed
/// values belong to this run: the plan cannot be worked out again after the question). No transaction is open while it
/// asks: the row is written only after a yes, as in Python.
fn add(provider: &str, workspace: String, alias: Option<String>, yes: bool) -> Run {
    let kc = keychain::system();
    let printed = Cell::new(0);
    let outcome = write(false, |conn| {
        let add = Add {
            entry: entry(provider)?,
            workspace,
            alias,
        };
        let mut confirm = |lines: &[String]| {
            if put_lines(lines).is_err() {
                return false;
            }
            printed.set(lines.len());
            yes || ask().unwrap_or(false)
        };
        connector_writes::add_to_workspace(conn, add, &*kc, &mut ask_value, &mut confirm)
    })?;
    if outcome.pending {
        put("aborted; nothing was written.\n")?;
        return Err(Stop::silent(1));
    }
    report(
        outcome.lines.get(printed.get()..).unwrap_or_default(),
        outcome.status,
    )
}

// ---- remove ----

/// A stored connector (`sync.remove_connector`), else a catalog id (the pre-workspace `writer.remove`), else Python's
/// refusal before any keychain or `claude` call (HUN-72: never remove what we did not create).
fn remove(name: &str, yes: bool) -> Run {
    let (kc, claude) = (keychain::system(), SystemClaude::new());
    change(
        yes,
        "aborted; nothing was removed.",
        false,
        |conn, apply| {
            let stored: Option<String> = conn
                .query_row("SELECT alias FROM connectors WHERE id = ?1", [name], |r| {
                    r.get(0)
                })
                .optional()?;
            let outcome = match stored {
                Some(alias) => {
                    connector_writes::remove_connector(conn, name, &alias, apply, &*kc, &claude)?
                }
                None if catalog::ids().contains(&name) => {
                    connector_writes::remove_pre_workspace(name, apply, &*kc, &claude)?
                }
                None => return Err(refused("unknown connector; nothing was removed")),
            };
            Ok((outcome, Value::Null))
        },
    )
    .map(drop)
}

// ---- sync ----

/// `_sync`: detect the agent folders, plan one account (an id or a folder) or every confirmed Claude account, print the
/// plan, ask, apply through `claude` step by step.
fn sync_cmd(account: Option<&str>, yes: bool) -> Run {
    let claude = SystemClaude::new();
    change(yes, "aborted; nothing was written.", true, |conn, apply| {
        let id = account
            .map(|reference| accounts::find(conn, reference).map(|id| id.to_string()))
            .transpose()?;
        let steps = sync::plan_all(conn, id.as_deref(), &claude)?;
        let mut lines: Vec<String> = steps.iter().map(Step::line).collect();
        let outcome = if !steps.iter().any(Step::is_applied) {
            lines.push("nothing to change.".into());
            Outcome {
                status: 0,
                lines,
                pending: false,
            }
        } else if !apply {
            Outcome {
                status: 1,
                lines,
                pending: true,
            }
        } else {
            let status = connector_writes::applied(conn, &steps, &claude, &mut lines)?;
            Outcome {
                status,
                lines,
                pending: false,
            }
        };
        Ok((outcome, Value::Null))
    })
    .map(drop)
}

// ---- the secret prompt ----

/// Python's `getpass.getpass` (`unix_getpass`): the terminal with echo off, else its fallback on stdin.
mod hidden {
    use super::MAX_VALUE;
    use crate::cli_cmds::read_line;
    use std::io::Write;

    const FALLBACK_WARNING: &str = "Warning: Password input may be echoed.";

    /// The value, or `Ok(None)` when input ended (Python's `EOFError`; Ctrl-C at the terminal ends the process with
    /// exit 2), or the fixed refusal when echo could not be turned off on a terminal (nothing was read).
    pub fn read(prompt: &str) -> Result<Option<String>, &'static str> {
        #[cfg(target_os = "linux")]
        if let Some(answer) = tty::read(prompt) {
            return answer;
        }
        // `fallback_getpass`: the warning and the prompt on stderr, the line from stdin, then a line end on stderr
        let mut err = std::io::stderr().lock();
        let _ = write!(err, "{FALLBACK_WARNING}\n{prompt}");
        let _ = err.flush();
        let Some(line) = read_line(MAX_VALUE) else {
            return Ok(None);
        };
        let _ = writeln!(err);
        Ok(Some(line))
    }

    /// A terminal, as `unix_getpass` uses it: the controlling terminal (`/dev/tty`: the prompt and the line there), else
    /// stdin when it is a terminal (T066: the prompt on stderr, the line from stdin). Echo is turned off (`TCSAFLUSH`)
    /// before the prompt and restored after the line, then a line end is written. No terminal (or one whose settings
    /// cannot be read) is `None`: the caller falls back to stdin with the warning. A terminal whose echo cannot be turned
    /// off is refused with [`NO_ECHO`] and nothing is read (Python falls back and reads with echo on). Linux only: the
    /// `termios` layout below is glibc's on Linux (four `u32` flag words first, `c_lflag` the fourth; 60 bytes in all).
    #[cfg(target_os = "linux")]
    pub(super) mod tty {
        use super::MAX_VALUE;
        use crate::cli_cmds::read_line;
        use std::fs::OpenOptions;
        use std::io::{BufRead, BufReader, Read, Write};
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::OpenOptionsExt;
        use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};

        #[repr(C)]
        #[derive(Clone, Copy, Default)]
        pub(crate) struct Termios([u32; 16]);

        #[cfg(test)]
        impl Termios {
            pub(crate) fn lflag(&self) -> u32 {
                self.0[LFLAG]
            }
        }

        const O_NOCTTY: i32 = 0o400;
        const TCSAFLUSH: i32 = 2;
        const ECHO: u32 = 0o10;
        const LFLAG: usize = 3;
        const SIGINT: i32 = 2;
        const ENDED: &[u8] = b"easy-personal-rs: input ended at the prompt; nothing was written\n";
        /// Echo could not be turned off: the secret is not read.
        pub(crate) const NO_ECHO: &str =
            "cannot turn off echo on the terminal; nothing was read or written";

        /// `tcsetattr`'s shape (a test passes one that fails).
        pub(crate) type SetAttr = unsafe extern "C" fn(i32, i32, *const Termios) -> i32;

        unsafe extern "C" {
            fn tcgetattr(fd: i32, termios: *mut Termios) -> i32;
            fn tcsetattr(fd: i32, action: i32, termios: *const Termios) -> i32;
            fn signal(signum: i32, handler: usize) -> usize;
            fn write(fd: i32, buf: *const std::ffi::c_void, count: usize) -> isize;
            fn _exit(status: i32) -> !;
        }

        /// The settings to restore when Ctrl-C ends the process at the prompt, and the terminal they belong to.
        static SAVED: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];
        static FD: AtomicI32 = AtomicI32::new(-1);

        /// Ctrl-C at the prompt: restore echo, say why (Python's `input ended` text, exit 2), and end. Nothing was written
        /// yet (the keychain and the store are written only after the question). Only async-signal-safe calls.
        extern "C" fn on_interrupt(_: i32) {
            let mut saved = Termios([0; 16]);
            for (word, slot) in saved.0.iter_mut().zip(&SAVED) {
                *word = slot.load(Ordering::Relaxed);
            }
            // SAFETY: tcsetattr, write and _exit are async-signal-safe; `saved` lives on this stack
            unsafe {
                tcsetattr(FD.load(Ordering::Relaxed), TCSAFLUSH, &saved);
                write(2, ENDED.as_ptr().cast(), ENDED.len());
                _exit(2);
            }
        }

        pub fn read(prompt: &str) -> Option<Result<Option<String>, &'static str>> {
            if let Ok(tty) = OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(O_NOCTTY)
                .open("/dev/tty")
            {
                let mut out = &tty;
                return quiet(tty.as_raw_fd(), tcsetattr, || {
                    let _ = out.write_all(prompt.as_bytes());
                    let _ = out.flush();
                    let mut bytes = Vec::new();
                    let read = BufReader::new((&tty).take(MAX_VALUE)).read_until(b'\n', &mut bytes);
                    match read {
                        Ok(n) if n > 0 => String::from_utf8(bytes).ok().map(|mut line| {
                            // a text wrapper on the terminal: universal newlines
                            if line.ends_with('\n') {
                                line.pop();
                                if line.ends_with('\r') {
                                    line.pop();
                                }
                            }
                            line
                        }),
                        _ => None,
                    }
                })
                .map(|got| got.inspect(|line| line_end(line, &mut out)));
            }
            // no controlling terminal: "see if stdin can be controlled" (the prompt on stderr, the line from stdin)
            let mut err = std::io::stderr();
            quiet(0, tcsetattr, || {
                let _ = err.write_all(prompt.as_bytes());
                let _ = err.flush();
                read_line(MAX_VALUE)
            })
            .map(|got| got.inspect(|line| line_end(line, &mut err)))
        }

        /// The line end Python writes after a value (not after end of input).
        fn line_end(line: &Option<String>, out: &mut impl Write) {
            if line.is_some() {
                let _ = out.write_all(b"\n");
                let _ = out.flush();
            }
        }

        /// Echo off on `fd`, `read`, echo restored. `None`: `fd` is not a terminal (its settings cannot be read).
        /// `Some(Err(NO_ECHO))`: `set` could not turn echo off; `read` is not called.
        pub(crate) fn quiet<T>(
            fd: i32,
            set: SetAttr,
            read: impl FnOnce() -> T,
        ) -> Option<Result<T, &'static str>> {
            let mut old = Termios([0; 16]);
            // SAFETY: `old` is larger than glibc's `struct termios` (60 bytes) and lives for the call
            if unsafe { tcgetattr(fd, &mut old) } != 0 {
                return None;
            }
            for (slot, word) in SAVED.iter().zip(old.0) {
                slot.store(word, Ordering::Relaxed);
            }
            FD.store(fd, Ordering::Relaxed);
            let mut silent = old;
            silent.0[LFLAG] &= !ECHO;
            // SAFETY: the handler only calls async-signal-safe functions; the previous one is put back below
            let previous = unsafe { signal(SIGINT, on_interrupt as extern "C" fn(i32) as usize) };
            // SAFETY: `silent` is a copy of what tcgetattr filled, with one flag cleared
            if unsafe { set(fd, TCSAFLUSH, &silent) } != 0 {
                // SAFETY: as above; nothing was changed on the terminal
                unsafe { signal(SIGINT, previous) };
                return Some(Err(NO_ECHO));
            }
            let got = read();
            // SAFETY: as above; `old` is what tcgetattr filled
            unsafe {
                set(fd, TCSAFLUSH, &old);
                signal(SIGINT, previous);
            }
            Some(Ok(got))
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::hidden::tty::{NO_ECHO, SetAttr, Termios, quiet};
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

    unsafe extern "C" {
        fn posix_openpt(flags: i32) -> i32;
        fn grantpt(fd: i32) -> i32;
        fn unlockpt(fd: i32) -> i32;
        fn ptsname_r(fd: i32, buf: *mut std::ffi::c_char, len: usize) -> i32;
        fn open(path: *const std::ffi::c_char, flags: i32, ...) -> i32;
        fn tcgetattr(fd: i32, termios: *mut Termios) -> i32;
        fn tcsetattr(fd: i32, action: i32, termios: *const Termios) -> i32;
    }

    /// A `tcsetattr` that always fails (as on a terminal that refuses the change).
    unsafe extern "C" fn refuse(_: i32, _: i32, _: *const Termios) -> i32 {
        -1
    }

    /// A pseudo-terminal pair (master, slave), neither this process's controlling terminal.
    fn pty() -> (OwnedFd, OwnedFd) {
        const O_RDWR_NOCTTY: i32 = 2 | 0o400;
        // SAFETY: plain C calls on descriptors this function owns; `name` outlives `ptsname_r` and `open`
        unsafe {
            let master = posix_openpt(O_RDWR_NOCTTY);
            assert!(master >= 0);
            assert_eq!(grantpt(master), 0);
            assert_eq!(unlockpt(master), 0);
            let mut name = [0 as std::ffi::c_char; 128];
            assert_eq!(ptsname_r(master, name.as_mut_ptr(), name.len()), 0);
            let slave = open(name.as_ptr(), O_RDWR_NOCTTY);
            assert!(slave >= 0);
            (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave))
        }
    }

    fn lflag(fd: &OwnedFd) -> u32 {
        let mut t = Termios::default();
        // SAFETY: `t` is larger than glibc's `struct termios` and lives for the call
        assert_eq!(unsafe { tcgetattr(fd.as_raw_fd(), &mut t) }, 0);
        t.lflag()
    }

    /// T066 AC2: a `tcsetattr` that fails is refused with the fixed text; nothing is read (so nothing typed can be
    /// shown) and the terminal is left as it was.
    #[test]
    fn a_failing_tcsetattr_is_refused_and_nothing_is_read() {
        let (_master, slave) = pty();
        let before = lflag(&slave);
        let mut called = false;
        let got = quiet(slave.as_raw_fd(), refuse as SetAttr, || called = true);
        assert_eq!(got, Some(Err(NO_ECHO)));
        assert!(!called, "the line is never read");
        assert_eq!(lflag(&slave), before);
    }

    /// The real `tcsetattr`: echo is off while the line is read and back on after.
    #[test]
    fn echo_is_off_during_the_read_and_restored_after() {
        let (_master, slave) = pty();
        let echo = 0o10;
        assert_ne!(lflag(&slave) & echo, 0);
        let during = quiet(slave.as_raw_fd(), tcsetattr as SetAttr, || lflag(&slave));
        assert_eq!(during.map(|r| r.map(|l| l & echo)), Some(Ok(0)));
        assert_ne!(lflag(&slave) & echo, 0);
    }

    /// T066 AC2: the refusal `ask_value` makes of `NO_ECHO` (`hidden::read(..).map_err(refused)`) ends the command with
    /// exit 2 and the fixed text on stderr, as every refused request does.
    #[test]
    fn the_no_echo_refusal_exits_2_with_its_fixed_text() {
        let stop = crate::cli_cmds::write_stop(super::refused(NO_ECHO));
        assert_eq!(stop.parts(), (2, Some(NO_ECHO)));
        assert_eq!(
            NO_ECHO,
            "cannot turn off echo on the terminal; nothing was read or written"
        );
    }

    /// Not a terminal: `None` (the caller takes Python's fallback with its warning).
    #[test]
    fn a_pipe_is_not_a_terminal() {
        let file = std::fs::File::open("/dev/null").unwrap();
        assert_eq!(quiet(file.as_raw_fd(), tcsetattr as SetAttr, || ()), None);
    }
}
