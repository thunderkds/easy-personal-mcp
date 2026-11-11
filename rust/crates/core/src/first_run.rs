//! First-run checklist, read side (Python: `core/first_run.py`, FR-042): four done / not-done checks over the store and the
//! systemd user unit folder. Read-only: the store is only read, and the folder only has the timer file looked up.
//!
//! The timer check is the one Python makes: `<unit folder>/easy-personal-ingest.timer` exists. The unit folder is
//! `XDG_CONFIG_HOME` (empty counts as unset), else `$HOME/.config`, then `systemd/user`; the caller passes it, so a test
//! controls it. No hint is worked out by asking `systemctl` (Python does, for the hint of a machine without a systemd user
//! session): the hint is always the install command, which on such a machine prints the cron line itself.

use rusqlite::Connection;
use serde_json::{Value, json};
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

const TIMER: &str = "easy-personal-ingest.timer";
const INGEST_HINT: &str = "easy-personal ingest";
const TIMER_HINT: &str = "easy-personal schedule install";

/// The unit folder from the two variables as Python reads them: `XDG_CONFIG_HOME` (empty = unset), else `$HOME/.config`,
/// then `systemd/user`; a `.` part is dropped as `Path` does. `None` when neither variable is usable (Python then asks
/// the passwd entry; like `schedule install` this asks for the variable only, and the timer then counts as not installed).
pub fn unit_dir_from(xdg_config_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let non_empty = |v: Option<OsString>| v.filter(|v| !v.is_empty());
    let base = match non_empty(xdg_config_home) {
        Some(base) => PathBuf::from(base),
        None => PathBuf::from(non_empty(home)?).join(".config"),
    };
    let kept: PathBuf = base
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect();
    Some(kept.join("systemd").join("user"))
}

/// [`unit_dir_from`] over the process environment, read now (per request, never cached).
pub fn unit_dir() -> Option<PathBuf> {
    unit_dir_from(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    )
}

/// `{"steps": [{id, done, hint}, ...]}` in checklist order.
pub fn steps(conn: &Connection, unit_dir: Option<&Path>) -> rusqlite::Result<Value> {
    let ingested = any_row(conn, "SELECT 1 FROM events LIMIT 1")?;
    let timer = unit_dir.is_some_and(|dir| dir.join(TIMER).exists());
    // `confirmed` here is the stored status with its folder still on disk: Python's account list turns a confirmed account
    // whose folder is gone into `missing`
    let mut folders =
        conn.prepare("SELECT folder FROM agent_accounts WHERE status = 'confirmed'")?;
    let mut confirmed = false;
    for folder in folders.query_map([], |row| row.get::<_, String>(0))? {
        confirmed |= Path::new(&folder?).is_dir();
    }
    let workspace = any_row(conn, "SELECT 1 FROM workspaces WHERE is_orphan = 0 LIMIT 1")?;
    Ok(json!({
        "steps": [
            {"id": "first_ingest", "done": ingested, "hint": INGEST_HINT},
            {"id": "ingest_timer", "done": timer, "hint": TIMER_HINT},
            {"id": "accounts_confirmed", "done": confirmed, "hint": "Open Agents and confirm an account"},
            {"id": "first_workspace", "done": workspace, "hint": "Open Workspaces and create one"},
        ]
    }))
}

fn any_row(conn: &Connection, sql: &str) -> rusqlite::Result<bool> {
    match conn.query_row(sql, [], |_| Ok(())) {
        Ok(()) => Ok(true),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(other) => Err(other),
    }
}
