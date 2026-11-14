//! `easy-personal-rs doctor` (T060, FR-019): Python's `doctor` (`surfaces/cli.py`), which runs `writer.doctor`,
//! `legacy.doctor` and `sync.doctor` in that order. It only reads: the store (read-only), the keychain (whether an item
//! is there, never its value) and `claude mcp get`. Nothing is written, removed or registered. Exit 1 when any line is
//! `FAIL`, else 0. The oracle is `rust/parity/golden-connector-cli/` (the `doctor_*` cases).
//!
//! Lines, in Python's order and texts: `ok|FAIL keychain`, `ok|FAIL claude` (on `PATH`), `ok|FAIL launcher`; then each
//! catalog id that is not a stored connector (a stored one, adopted or added, is checked below) and has a keychain item or
//! a default-account entry; then a `note` when pre-workspace connectors wait for `connector adopt-legacy`; then each stored
//! connector without its keychain item, and (with `claude` on `PATH`) each account registration.
//!
//! Where Rust differs (`rust/README.md`): the keychain line names this build's backend (`SecretService`; Python prints its
//! backend's class name); a `claude mcp get` that could not run (it did not start or timed out) is never "no entry": the
//! first one is one `FAIL claude` line, `claude` is not run again, and the entries it would have checked are skipped
//! (T066; T060 printed one line per entry); the legacy note then says it was not checked; a keychain that answers but
//! refuses a read is a `FAIL` line with the keychain text (Python crashes).

use crate::claude_cli::SystemClaude;
use crate::cli_cmds::{Run, Stop, finish, open_read, put, store_stop};
use crate::{keychain, legacy};
use easy_personal_core::catalog;
use easy_personal_core::store::StoreError;
use easy_personal_core::sync::{self, Claude, LAUNCHER, NotRun, Ran};
use rusqlite::Connection;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

const NOT_RUN: &str = "`claude` could not run (it did not start or timed out); entries not checked";
const NOT_CHECKED: &str =
    "note legacy connectors not checked: `claude` could not run (it did not start or timed out)\n";

/// `claude` for one doctor run: once a call could not run, every later call answers "could not run" at once (no 60 s
/// wait per entry, one `FAIL` line for all of them).
struct Once<C> {
    claude: C,
    dead: AtomicBool,
}

impl<C: Claude> Claude for Once<C> {
    fn on_path(&self) -> bool {
        self.claude.on_path()
    }
    fn run(&self, args: &[&str], config_dir_env: Option<&str>) -> Ran {
        if self.dead.load(Ordering::Relaxed) {
            return Ran {
                code: 127,
                stdout: String::new(),
                not_run: true,
            };
        }
        let got = self.claude.run(args, config_dir_env);
        if got.not_run {
            self.dead.store(true, Ordering::Relaxed);
        }
        got
    }
    fn launcher(&self) -> Result<String, String> {
        self.claude.launcher()
    }
}

pub fn run() -> ExitCode {
    finish(doctor())
}

/// Prints one line per check and counts the failures (`report` of Python's three doctors).
struct Lines {
    bad: u32,
    /// the `FAIL claude` line for a `claude` that could not run is printed
    not_run_said: bool,
}

impl Lines {
    fn report(&mut self, ok: bool, what: &str, detail: &str) -> Run {
        self.bad += u32::from(!ok);
        put(&format!(
            "{} {what}: {detail}\n",
            if ok { "ok" } else { "FAIL" }
        ))
    }

    /// One `FAIL` line for every entry a `claude` that could not run left unchecked.
    fn not_run(&mut self) -> Run {
        if std::mem::replace(&mut self.not_run_said, true) {
            return Ok(());
        }
        self.report(false, "claude", NOT_RUN)
    }
}

fn read<T>(
    conn: &Connection,
    sql: &str,
    row: impl FnMut(&rusqlite::Row) -> rusqlite::Result<T>,
) -> Result<Vec<T>, Stop> {
    (|| {
        conn.prepare(sql)?
            .query_map([], row)?
            .collect::<rusqlite::Result<Vec<T>>>()
    })()
    .map_err(|e| store_stop(StoreError::from(e)))
}

fn doctor() -> Run {
    let conn = open_read()?;
    let kc = keychain::system();
    let claude = Once {
        claude: SystemClaude::new(),
        dead: AtomicBool::new(false),
    };
    let mut out = Lines {
        bad: 0,
        not_run_said: false,
    };
    let stored: BTreeSet<String> = read(&conn, "SELECT id FROM connectors", |r| r.get(0))?
        .into_iter()
        .collect();

    // writer.doctor: the keychain, `claude`, the launcher, and each pre-workspace catalog id
    let backend_ok = match keychain::require_backend(&*kc) {
        Ok(()) => {
            out.report(true, "keychain", kc.name())?;
            true
        }
        Err(e) => {
            out.report(false, "keychain", &e.0)?;
            false
        }
    };
    let has_claude = claude.on_path();
    out.report(
        has_claude,
        "claude",
        if has_claude { "on PATH" } else { "not on PATH" },
    )?;
    match claude.launcher() {
        Ok(path) => out.report(true, "launcher", &path)?,
        Err(text) => out.report(false, "launcher", &text)?,
    }
    for name in catalog::ids() {
        if stored.contains(name) {
            continue; // a stored connector: the registration checks below cover it
        }
        let in_keychain = if backend_ok {
            match keychain::has_item(&*kc, name) {
                Ok(found) => found,
                Err(e) => {
                    out.report(false, name, &e.0)?;
                    continue;
                }
            }
        } else {
            false
        };
        let got = if has_claude {
            match sync::entry_exists(&claude, name, None) {
                Ok(got) => got,
                Err(NotRun) => {
                    out.not_run()?;
                    continue;
                }
            }
        } else {
            None
        };
        let Some(got) = got else {
            if in_keychain {
                out.report(
                    false,
                    name,
                    "keychain item present but no Claude Code entry; run connector add",
                )?;
            }
            continue; // neither: not a configured connector
        };
        let (command, _) = sync::entry_target(&got.stdout);
        let points = Path::new(&command)
            .file_name()
            .is_some_and(|n| n == LAUNCHER);
        let detail = if points {
            "entry points at the launcher"
        } else {
            "entry does not point at easy-personal-launch"
        };
        out.report(points, name, detail)?;
        if !in_keychain {
            out.report(
                false,
                name,
                "Claude Code entry present but no keychain item",
            )?;
        }
    }

    // legacy.doctor: one note (no keychain: the line above already says so). A `claude` that could not run is the one
    // `FAIL claude` line (printed here if no check above ran it), so it fails the run (exit 1), unlike Python's note
    match legacy::detect(&conn, &*kc, &claude) {
        Ok(detected) if detected.claude_not_run => {
            out.not_run()?;
            put(NOT_CHECKED)?;
        }
        Ok(detected) if !detected.found.is_empty() => {
            let ids: Vec<&str> = detected.found.iter().map(|i| i.id).collect();
            put(&format!(
                "note legacy connectors not adopted: {}; run `easy-personal connector adopt-legacy`\n",
                ids.join(", ")
            ))?;
        }
        _ => {}
    }

    // sync.doctor: each stored connector has its item; each registration is present and points at the launcher
    for id in read(&conn, "SELECT id FROM connectors ORDER BY id", |r| {
        r.get::<_, String>(0)
    })? {
        if keychain::has_item(&*kc, &id) == Ok(false) {
            out.report(
                false,
                &id,
                "no keychain item; remove and add the connector again",
            )?;
        }
    }
    if has_claude {
        registrations(&conn, &claude, &mut out)?;
    }
    if out.bad > 0 {
        return Err(Stop::silent(1));
    }
    Ok(())
}

fn registrations(conn: &Connection, claude: &dyn Claude, out: &mut Lines) -> Run {
    type Row = (Option<String>, String, Option<String>, String, String);
    let rows: Vec<Row> = read(
        conn,
        "SELECT a.label, a.folder, a.config_dir_env, r.alias, r.connector_id \
         FROM account_registrations r JOIN agent_accounts a ON a.id = r.account_id ORDER BY a.id, r.alias",
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    )?;
    for (label, folder, config_dir_env, alias, id) in rows {
        let account = label
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| sync::folder_name(&folder));
        let name = format!("{account} {alias}");
        if !Path::new(&folder).is_dir() {
            out.report(false, &name, "account folder missing")?;
            continue;
        }
        match sync::entry_exists(claude, &alias, config_dir_env.as_deref()) {
            Err(NotRun) => out.not_run()?,
            Ok(None) => out.report(
                false,
                &name,
                "drift: missing from Claude Code; `connector sync` re-adds it",
            )?,
            Ok(Some(got)) if !sync::points_at_launcher(&got, &id) => out.report(
                false,
                &name,
                &format!(
                    "drift: does not point at easy-personal-launch exec {id}; `connector sync` replaces it"
                ),
            )?,
            Ok(Some(_)) => out.report(true, &name, &format!("points at the launcher (exec {id})"))?,
        }
    }
    Ok(())
}
