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
        hidden::read(&prompt)
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

    /// The value, or `None` when input ended (Python's `EOFError`; Ctrl-C at the terminal ends the process with exit 2).
    pub fn read(prompt: &str) -> Option<String> {
        #[cfg(target_os = "linux")]
        if let Some(answer) = tty::read(prompt) {
            return answer;
        }
        // `fallback_getpass`: the warning and the prompt on stderr, the line from stdin, then a line end on stderr
        let mut err = std::io::stderr().lock();
        let _ = write!(err, "{FALLBACK_WARNING}\n{prompt}");
        let _ = err.flush();
        let line = read_line(MAX_VALUE)?;
        let _ = writeln!(err);
        Some(line)
    }

    /// The controlling terminal, as `unix_getpass` uses it: open `/dev/tty`, turn `ECHO` off (`TCSAFLUSH`), write the
    /// prompt there, read one line there, restore the settings, write a line end. No terminal (or one whose settings
    /// cannot be read) is `None`: the caller falls back to stdin. Linux only: the `termios` layout below is glibc's on
    /// Linux (four `u32` flag words first, `c_lflag` the fourth; 60 bytes in all).
    #[cfg(target_os = "linux")]
    mod tty {
        use super::MAX_VALUE;
        use std::fs::OpenOptions;
        use std::io::{BufRead, BufReader, Read, Write};
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::OpenOptionsExt;
        use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};

        #[repr(C)]
        #[derive(Clone, Copy)]
        struct Termios([u32; 16]);

        const O_NOCTTY: i32 = 0o400;
        const TCSAFLUSH: i32 = 2;
        const ECHO: u32 = 0o10;
        const LFLAG: usize = 3;
        const SIGINT: i32 = 2;
        const ENDED: &[u8] = b"easy-personal-rs: input ended at the prompt; nothing was written\n";

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

        pub fn read(prompt: &str) -> Option<Option<String>> {
            let tty = OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(O_NOCTTY)
                .open("/dev/tty")
                .ok()?;
            let fd = tty.as_raw_fd();
            let mut old = Termios([0; 16]);
            // SAFETY: `old` is larger than glibc's `struct termios` (60 bytes) and lives for the call
            if unsafe { tcgetattr(fd, &mut old) } != 0 {
                return None;
            }
            for (slot, word) in SAVED.iter().zip(old.0) {
                slot.store(word, Ordering::Relaxed);
            }
            FD.store(fd, Ordering::Relaxed);
            let mut quiet = old;
            quiet.0[LFLAG] &= !ECHO;
            // SAFETY: the handler only calls async-signal-safe functions; the previous one is put back below
            let previous = unsafe { signal(SIGINT, on_interrupt as extern "C" fn(i32) as usize) };
            // SAFETY: `quiet` is a copy of what tcgetattr filled, with one flag cleared
            unsafe { tcsetattr(fd, TCSAFLUSH, &quiet) };
            let mut out = &tty;
            let _ = out.write_all(prompt.as_bytes());
            let _ = out.flush();
            let mut bytes = Vec::new();
            let read = BufReader::new((&tty).take(MAX_VALUE)).read_until(b'\n', &mut bytes);
            // SAFETY: as above; `old` is what tcgetattr filled
            unsafe {
                tcsetattr(fd, TCSAFLUSH, &old);
                signal(SIGINT, previous);
            }
            Some(match read {
                Ok(n) if n > 0 => String::from_utf8(bytes).ok().map(|mut line| {
                    // the line end Python writes after a value (not after end of input)
                    let _ = out.write_all(b"\n");
                    let _ = out.flush();
                    if line.ends_with('\n') {
                        line.pop();
                        if line.ends_with('\r') {
                            line.pop();
                        }
                    }
                    line
                }),
                _ => None,
            })
        }
    }
}
