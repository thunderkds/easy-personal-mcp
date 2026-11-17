//! Watch 5 (T076, FR-099): what the `tray` command shows and the files it owns, with no Tauri code (every `tauri` use
//! stays in `window.rs`). The tray's state is the most urgent of: *problem* (a session failing, or machine pressure:
//! disk or RAM at or above 90 % used, a battery at or below 15 % and not charging), *attention* (a session waiting for
//! you or at *your turn* (T096), or a plan limit at or above 90 %), *active* (any active session), *idle*. The thresholds
//! are T074's (`alerts.rs`); the icon follows the current values, with no re-arm band (the popups keep theirs).
//!
//! Waiting sessions (T096, FR-107): the menu has one clickable row per *waiting session* (a session whose states hold
//! `failing`, `waiting` or `your_turn`), newest state first, at most [`SESSIONS_SHOWN`], then `+N more in dashboard…`. A
//! row holds the kind in fixed words, the project folder name and the short session id, never question text. A click
//! opens the Live view on that session: [`address`] builds the window address.
//!
//! The menu rows (owner, 2026-11-15, `tasks/TASK_REVIEW_T076.md` `## Menu proposal`) are built here as plain [`Row`]s.
//! A piece of free text (a project folder, an account label) passes the alert engine's `clean` (no control or markup
//! characters, the secret check of the core); a row never holds a message, a tool input or an output (NFR-002).
//!
//! Files: one lock per user, `$XDG_RUNTIME_DIR/easy-personal-tray.lock` (else `~/.easy-personal/`), held with
//! `flock` while the tray runs; and the autostart entry `$XDG_CONFIG_HOME/autostart/easy-personal-tray.desktop` (else
//! `~/.config/autostart/`), written or removed by `tray --autostart on|off`. Nothing else is written.
#![cfg_attr(not(feature = "window"), allow(dead_code))]

use crate::alerts::{self, Alert, HIGH, LOW_BATTERY};
use easy_personal_core::limits_watch::Round;
use easy_personal_core::machine::Sample;
use serde_json::Value;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// The icon state, most urgent last.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum State {
    Idle,
    Active,
    Attention,
    Problem,
}

impl State {
    /// The word of the first menu row and of the icon file name (`icons/tray/<theme>/<name>-<size>.png`).
    pub(crate) fn name(self) -> &'static str {
        match self {
            State::Idle => "idle",
            State::Active => "active",
            State::Attention => "attention",
            State::Problem => "problem",
        }
    }
}

/// The panel the icon is drawn for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Theme {
    Dark,
    Light,
}

/// The desktop's `org.freedesktop.appearance` `color-scheme` (0 no preference, 1 dark, 2 light) to a theme: light only
/// when the desktop says light; unreadable or anything else is dark.
pub(crate) fn theme_of(color_scheme: Option<u32>) -> Theme {
    if color_scheme == Some(2) {
        Theme::Light
    } else {
        Theme::Dark
    }
}

/// Menu item ids of the actions.
pub(crate) const OPEN: &str = "open";
/// "+N more in dashboard…" (T096): the Live view with no session selected.
pub(crate) const MORE: &str = "more";
/// "All alerts in dashboard…" (T088): the same handler as [`OPEN`].
pub(crate) const ALERTS: &str = "alerts";
pub(crate) const PAUSE: &str = "pause";
pub(crate) const QUIT: &str = "quit";

/// One menu row.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Row {
    /// A disabled row: text only.
    Info(String),
    /// A clickable row with its id.
    Action(&'static str, String),
    /// A clickable row of one waiting session (T096): the full session id and the text. Its menu item id is
    /// [`session_item_id`].
    Session(String, String),
    /// A check item with its id and whether it is ticked.
    Check(&'static str, String, bool),
    Separator,
}

/// True when two menus have the same rows in the same places (kind and id), so the shown one can be updated item by
/// item (text, tick) instead of rebuilt.
pub(crate) fn same_shape(a: &[Row], b: &[Row]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|pair| match pair {
            (Row::Info(_), Row::Info(_)) | (Row::Separator, Row::Separator) => true,
            (Row::Action(x, _), Row::Action(y, _)) | (Row::Check(x, _, _), Row::Check(y, _, _)) => {
                x == y
            }
            (Row::Session(x, _), Row::Session(y, _)) => x == y,
            _ => false,
        })
}

const SESSION_PREFIX: &str = "session:";

/// The menu item id of a session row: `session:<session id>`.
pub(crate) fn session_item_id(session: &str) -> String {
    format!("{SESSION_PREFIX}{session}")
}

/// The session id of a menu item id made by [`session_item_id`]; `None` for any other id.
pub(crate) fn session_of(item: &str) -> Option<&str> {
    item.strip_prefix(SESSION_PREFIX)
}

/// What a click asks the window for.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Target {
    /// "Open dashboard", "All alerts in dashboard…": the page as it starts.
    Dashboard,
    /// The Live view, with this session selected when it is given (a popup that sums several alerts, "+N more": none).
    Live(Option<String>),
}

/// A session id the address may carry: hex digits and `-`, 1 to 64 characters (the page checks the same).
fn is_session_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}

/// The window address of a click: `http://127.0.0.1:<port>/#<token>`, `/?live#<token>` or `/?live=<session id>#<token>`
/// (T095 reads the query once). The token stays in the fragment; an id that is not a session id is left out.
pub(crate) fn address(port: u16, token: &str, target: &Target) -> String {
    let origin = format!("http://127.0.0.1:{port}/");
    match target {
        Target::Dashboard => format!("{origin}#{token}"),
        Target::Live(Some(id)) if is_session_id(id) => format!("{origin}?live={id}#{token}"),
        Target::Live(_) => format!("{origin}?live#{token}"),
    }
}

/// What the tray shows at one look.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct View {
    pub state: State,
    pub rows: Vec<Row>,
}

fn sessions(live: &Value) -> &[Value] {
    live["sessions"].as_array().map_or(&[], Vec::as_slice)
}

fn has_state(session: &Value, state: &str) -> bool {
    session["attention"]
        .as_array()
        .is_some_and(|all| all.iter().any(|a| a["state"] == state))
}

fn pressure(s: &Sample) -> bool {
    let high = |v: Option<f64>| v.is_some_and(|v| v >= HIGH);
    high(alerts::pct(s.disk_used, s.disk_total))
        || high(alerts::pct(s.ram_used, s.ram_total))
        || s.battery_pct
            .is_some_and(|b| b <= LOW_BATTERY && s.charging == Some(false))
}

fn limit_high(round: &Round) -> bool {
    round.rows.iter().flatten().any(|r| {
        [r.five_h_pct, r.seven_d_pct]
            .into_iter()
            .flatten()
            .any(|v| v >= HIGH)
    })
}

/// The state rule of the module doc. A missing input counts as "nothing there".
pub(crate) fn pick(live: &Value, machine: Option<&Sample>, limits: Option<&Round>) -> State {
    let all = sessions(live);
    if all.iter().any(|s| has_state(s, "failing")) || machine.is_some_and(pressure) {
        State::Problem
    } else if all
        .iter()
        .any(|s| has_state(s, "waiting") || has_state(s, "your_turn"))
        || limits.is_some_and(limit_high)
    {
        State::Attention
    } else if !all.is_empty() {
        State::Active
    } else {
        State::Idle
    }
}

fn live_line(n: usize) -> String {
    match n {
        0 => "No sessions running".into(),
        1 => "1 session active".into(),
        n => format!("{n} sessions active"),
    }
}

/// The menu shows this many session rows; one more session adds the `+N more in dashboard…` row.
pub(crate) const SESSIONS_SHOWN: usize = 5;

/// What names a waiting session: the kind in fixed words and the `since` of the state that gives it, from the first of
/// `failing`, `waiting`, `your_turn` the session holds; `None` for a session with none of them.
fn named_state(session: &Value) -> Option<(&'static str, &str)> {
    let all = session["attention"].as_array()?;
    let hold = |state: &str| all.iter().find(|a| a["state"] == state);
    let (kind, a) = if let Some(a) = hold("failing") {
        ("failing", a)
    } else if let Some(a) = hold("waiting") {
        let kind = if a["tool"] == "ExitPlanMode" {
            "plan to approve"
        } else {
            "question"
        };
        (kind, a)
    } else {
        ("your turn", hold("your_turn")?)
    };
    Some((kind, a["since"].as_str().unwrap_or("")))
}

/// One row per waiting session, newest state first, at most [`SESSIONS_SHOWN`], then the row for the rest. A row is
/// `<kind> · <project> · <short id>`; a piece the session lacks (or that fails the checks of `alerts.rs`) is left out.
/// A session with no id has no row: a click could not name it.
fn session_rows(all: &[Value]) -> Vec<Row> {
    let mut waiting: Vec<(&str, &str, &Value, &str)> = all
        .iter()
        .filter_map(|s| {
            let id = s["session_id"].as_str()?;
            let (kind, since) = named_state(s)?;
            Some((since, kind, s, id))
        })
        .collect();
    waiting.sort_by(|a, b| b.0.cmp(a.0)); // newest first; a tie keeps the live order
    let mut rows: Vec<Row> = waiting
        .iter()
        .take(SESSIONS_SHOWN)
        .map(|&(_, kind, s, id)| {
            let mut parts = vec![kind.to_string()];
            parts.extend(s["project"].as_str().and_then(alerts::clean));
            parts.extend(alerts::short_id(id));
            Row::Session(id.to_string(), parts.join(" · "))
        })
        .collect();
    if waiting.len() > SESSIONS_SHOWN {
        let more = waiting.len() - SESSIONS_SHOWN;
        rows.push(Row::Action(MORE, format!("+{more} more in dashboard…")));
    }
    rows
}

fn percent(v: Option<f64>) -> String {
    v.map_or_else(|| "no data".into(), |v| format!("{v:.0} %"))
}

fn limit_rows(limits: Option<&Round>) -> Vec<String> {
    let rows = limits.and_then(|r| r.rows.as_deref()).unwrap_or(&[]);
    let line = |five, seven| format!("5h limit · {} · 7d · {}", percent(five), percent(seven));
    match rows {
        [] => vec![line(None, None)],
        [one] => vec![line(one.five_h_pct, one.seven_d_pct)],
        many => many
            .iter()
            .map(|r| {
                let label = alerts::clean(&r.label).unwrap_or_else(|| format!("account {}", r.id));
                format!("{label}: {}", line(r.five_h_pct, r.seven_d_pct))
            })
            .collect(),
    }
}

fn machine_line(machine: Option<&Sample>) -> String {
    let Some(s) = machine else {
        return "Machine · no data".into();
    };
    let cpu = s.cpu_pct;
    let ram = alerts::pct(s.ram_used, s.ram_total);
    let disk = alerts::pct(s.disk_used, s.disk_total);
    if cpu.is_none() && ram.is_none() && disk.is_none() && s.battery_pct.is_none() {
        return "Machine · no data".into();
    }
    let mut parts = vec![
        format!("CPU {}", percent(cpu)),
        format!("RAM {}", percent(ram)),
        format!("disk {}", percent(disk)),
    ];
    // no battery (a desktop) leaves the part out: never 0 %
    if let Some(b) = s.battery_pct {
        parts.push(format!("battery {b} %"));
    }
    parts.join(" · ")
}

/// The alert block shows this many alerts; one more in the list adds the "All alerts in dashboard…" row.
pub(crate) const ALERTS_SHOWN: usize = 5;
/// The most characters of an alert's text in a row (a longer text ends with `…`), so a long project name does not widen
/// the menu.
const ROW_TEXT_MAX: usize = 60;

/// The local zone's offset from UTC, in seconds, at the UTC time `t` (so a summer-time day is right); 0 when the zone
/// cannot be read.
pub(crate) fn local_offset(t: i64) -> i64 {
    let time: libc::time_t = t;
    // SAFETY: `localtime_r` only writes into the `tm` given; a zeroed `tm` is a valid value of it.
    unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&time, &mut tm).is_null() {
            0
        } else {
            tm.tm_gmtoff
        }
    }
}

/// (local day number since 1970-01-01, local month, day, hour, minute) of the UTC time `t` in the zone `offset` gives.
fn local_parts(t: i64, offset: fn(i64) -> i64) -> (i64, i64, i64, i64, i64) {
    let local = t.saturating_add(offset(t));
    let days = local.div_euclid(86_400);
    let secs = local.rem_euclid(86_400);
    // civil date from the day number (H. Hinnant's days_from_civil, inverted)
    let z = days + 719_468;
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (days, month, day, secs / 3_600, secs % 3_600 / 60)
}

/// `HH:MM · <text>`, or `MM-DD HH:MM · <text>` when the alert is from an earlier local day than `now`; the text cut to
/// [`ROW_TEXT_MAX`] characters.
fn alert_row(a: &Alert, now: i64, offset: fn(i64) -> i64) -> String {
    let (day, month, date, hour, minute) = local_parts(a.t, offset);
    let when = if day == local_parts(now, offset).0 {
        format!("{hour:02}:{minute:02}")
    } else {
        format!("{month:02}-{date:02} {hour:02}:{minute:02}")
    };
    let text = if a.text.chars().count() > ROW_TEXT_MAX {
        let head: String = a.text.chars().take(ROW_TEXT_MAX - 1).collect();
        format!("{head}…")
    } else {
        a.text.clone()
    };
    format!("{when} · {text}")
}

/// The whole menu at one look, in the approved order. `alerts` is newest first (T088); with none, no block.
/// `now` is UTC seconds; `offset` gives the local zone's offset at a UTC time ([`local_offset`]).
pub(crate) fn view(
    live: &Value,
    machine: Option<&Sample>,
    limits: Option<&Round>,
    paused: bool,
    alerts: &[Alert],
    now: i64,
    offset: fn(i64) -> i64,
) -> View {
    let state = pick(live, machine, limits);
    let all = sessions(live);
    let live = live_line(all.len());
    let mut rows = vec![
        Row::Info(format!("Easy Personal · {}", state.name())),
        Row::Separator,
        Row::Info(live.clone()),
    ];
    rows.extend(session_rows(all));
    rows.extend(limit_rows(limits).into_iter().map(Row::Info));
    rows.push(Row::Info(machine_line(machine)));
    if !alerts.is_empty() {
        rows.extend([Row::Separator, Row::Info("Recent alerts".into())]);
        rows.extend(
            alerts
                .iter()
                .take(ALERTS_SHOWN)
                .map(|a| Row::Info(alert_row(a, now, offset))),
        );
        if alerts.len() > ALERTS_SHOWN {
            rows.push(Row::Action(ALERTS, "All alerts in dashboard…".into()));
        }
    }
    rows.extend([
        Row::Separator,
        Row::Action(OPEN, "Open dashboard".into()),
        Row::Check(PAUSE, "Pause alerts".into(), paused),
        Row::Separator,
        Row::Info("127.0.0.1 · local only".into()),
        Row::Action(QUIT, "Quit".into()),
    ]);
    View { state, rows }
}

/// What the tray reads every look: the server's live tail, its two samplers and the alerts' pause flag.
pub(crate) struct Inputs {
    pub tail: Option<crate::live_feed::SharedTail>,
    pub machine: Option<crate::sampler::History<Sample>>,
    pub limits: Option<crate::sampler::History<Round>>,
    pub alerts: std::sync::Arc<alerts::Shared>,
}

impl Inputs {
    pub(crate) fn view(&self) -> View {
        use std::sync::PoisonError;
        let now = easy_personal_core::clock::now_micros();
        let live = self.tail.as_ref().map_or(Value::Null, |t| {
            t.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .snapshot(now)
        });
        let sample = self.machine.as_ref().and_then(|h| {
            h.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .latest()
                .cloned()
        });
        let round = self.limits.as_ref().and_then(|h| {
            h.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .latest()
                .cloned()
        });
        // one more than shown tells `view` whether the "All alerts" row is due; the lock is gone before any row is built
        let recent = self.alerts.recent(ALERTS_SHOWN + 1);
        view(
            &live,
            sample.as_ref(),
            round.as_ref(),
            self.alerts.paused(),
            &recent,
            now / 1_000_000,
            local_offset,
        )
    }
}

// -- single instance ---------------------------------------------------------------------------------------------------

const LOCK_FILE: &str = "easy-personal-tray.lock";
const ICON_DIR: &str = "tray-icon";

/// The tray's own folder: `$XDG_RUNTIME_DIR` when that is an absolute path, else the store's folder (`~/.easy-personal/`,
/// or `EASY_PERSONAL_HOME`). Never the shared temp folder (`/tmp`): another user could plant a symlink there.
fn private_dir(runtime_dir: Option<OsString>, store: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(dir) = runtime_dir.map(PathBuf::from).filter(|d| d.is_absolute()) {
        return Ok(dir);
    }
    store
        .as_deref()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "no runtime folder and no home folder for the tray".into())
}

/// `<private dir>/easy-personal-tray.lock` ([`private_dir`]).
pub(crate) fn lock_path(
    runtime_dir: Option<OsString>,
    store: Option<PathBuf>,
) -> Result<PathBuf, String> {
    private_dir(runtime_dir, store).map(|dir| dir.join(LOCK_FILE))
}

/// `<private dir>/tray-icon` ([`private_dir`]): where the tray library writes the icon PNG it hands to the panel. Given
/// to it explicitly, because its own default falls back to `/tmp/tray-icon` when `XDG_RUNTIME_DIR` is unset.
pub(crate) fn icon_dir(
    runtime_dir: Option<OsString>,
    store: Option<PathBuf>,
) -> Result<PathBuf, String> {
    private_dir(runtime_dir, store).map(|dir| dir.join(ICON_DIR))
}

/// Takes the lock: `Some(file)` while it is held (drop it to let go), `None` when another process holds it. A symlink
/// at `path` is refused.
pub(crate) fn try_lock(path: &Path) -> Result<Option<File>, String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)
            .map_err(|e| format!("cannot make the lock folder: {}", e.kind()))?;
    }
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("the tray lock is a symlink: not opened".into());
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| format!("cannot open the tray lock: {}", e.kind()))?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(e)) => Err(format!("cannot take the tray lock: {}", e.kind())),
    }
}

// -- autostart ---------------------------------------------------------------------------------------------------------

pub(crate) const AUTOSTART_FILE: &str = "easy-personal-tray.desktop";

/// `$XDG_CONFIG_HOME/autostart/easy-personal-tray.desktop`; a relative or empty `XDG_CONFIG_HOME` is ignored (XDG
/// base-dir spec), then `$HOME/.config`.
pub(crate) fn autostart_path(
    config_home: Option<OsString>,
    home: Option<OsString>,
) -> Result<PathBuf, String> {
    let config = match config_home.map(PathBuf::from).filter(|d| d.is_absolute()) {
        Some(dir) => dir,
        None => {
            PathBuf::from(home.filter(|h| !h.is_empty()).ok_or("no home folder")?).join(".config")
        }
    };
    Ok(config.join("autostart").join(AUTOSTART_FILE))
}

/// One `Exec` argument: the path in double quotes. A path with a character the entry would have to escape (`"`, `` ` ``,
/// `$`, `\`, `%`) or a control character (a newline would start a new key) is refused, as is one that is not UTF-8.
fn exec_arg(path: &Path) -> Result<String, String> {
    let text = path
        .to_str()
        .ok_or_else(|| "the path is not UTF-8".to_string())?;
    if text
        .chars()
        .any(|c| c.is_control() || matches!(c, '"' | '`' | '$' | '\\' | '%'))
    {
        return Err("the path holds a character an autostart entry cannot carry".into());
    }
    Ok(format!("\"{text}\""))
}

/// The autostart entry: start `exe tray --static-dir <dir>` at login.
pub(crate) fn desktop_entry(exe: &Path, static_dir: &Path) -> Result<String, String> {
    Ok(format!(
        "[Desktop Entry]\nType=Application\nName=Easy Personal tray\n\
         Comment=Tray icon and desktop alerts of easy-personal\n\
         Exec={} tray --static-dir {}\nTerminal=false\nNoDisplay=true\nX-GNOME-Autostart-enabled=true\n",
        exec_arg(exe)?,
        exec_arg(static_dir)?
    ))
}

/// Writes `entry` to `path` (making the `autostart` folder when it is missing). A symlink at `path` is refused: the
/// write must not land anywhere else.
pub(crate) fn autostart_on(path: &Path, entry: &str) -> Result<(), String> {
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(format!("{} is a symlink: not written", path.display()));
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)
            .map_err(|e| format!("cannot make {}: {}", dir.display(), e.kind()))?;
    }
    fs::write(path, entry).map_err(|e| format!("cannot write {}: {}", path.display(), e.kind()))
}

/// Removes `path`; `Ok(false)` when it was not there. Nothing else is removed (not the folder).
pub(crate) fn autostart_off(path: &Path) -> Result<bool, String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("cannot remove {}: {}", path.display(), e.kind())),
    }
}

// -- the session D-Bus: is there a tray area, which theme --------------------------------------------------------------

/// How long one D-Bus question may take.
const DBUS_WAIT: std::time::Duration = std::time::Duration::from_secs(2);

/// One call on the session bus (or `address`), with [`DBUS_WAIT`]; `None` on any failure.
fn dbus_call<B>(
    address: Option<&str>,
    destination: &str,
    path: &str,
    interface: &str,
    method: &str,
    body: &B,
) -> Option<zbus::Message>
where
    B: zbus::export::serde::Serialize + zbus::zvariant::DynamicType + Sync,
{
    crate::keychain::off_runtime(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()?;
        runtime.block_on(async {
            let call = async {
                let connection = match address {
                    Some(address) => zbus::connection::Builder::address(address)?.build().await?,
                    None => zbus::Connection::session().await?,
                };
                connection
                    .call_method(Some(destination), path, Some(interface), method, body)
                    .await
            };
            tokio::time::timeout(DBUS_WAIT, call).await.ok()?.ok()
        })
    })
}

/// True when a StatusNotifier host is there to show the icon: the name `org.kde.StatusNotifierWatcher` has an owner
/// on the session bus (GNOME's AppIndicator extension, KDE, ...). Without the bus: false.
pub(crate) fn tray_host(address: Option<&str>) -> bool {
    dbus_call(
        address,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "NameHasOwner",
        &("org.kde.StatusNotifierWatcher",),
    )
    .and_then(|reply| reply.body().deserialize::<bool>().ok())
    .unwrap_or(false)
}

/// The desktop's `color-scheme` from the settings portal, when it answers.
pub(crate) fn desktop_color_scheme(address: Option<&str>) -> Option<u32> {
    let reply = dbus_call(
        address,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Settings",
        "ReadOne",
        &("org.freedesktop.appearance", "color-scheme"),
    )?;
    let value: zbus::zvariant::OwnedValue = reply.body().deserialize().ok()?;
    u32::try_from(value).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use easy_personal_core::accounts::LimitPercents;
    use serde_json::json;

    /// The menu with no alerts: what every test before T088 calls.
    fn view(live: &Value, machine: Option<&Sample>, limits: Option<&Round>, paused: bool) -> View {
        super::view(live, machine, limits, paused, &[], 0, utc)
    }
    fn utc(_: i64) -> i64 {
        0
    }
    fn plus7(_: i64) -> i64 {
        7 * 3600
    }

    fn sample() -> Sample {
        Sample {
            t: 0,
            cpu_pct: None,
            ram_used: None,
            ram_total: None,
            disk_used: None,
            disk_total: None,
            load1: None,
            battery_pct: None,
            charging: None,
            claude_procs: None,
        }
    }
    fn round(rows: &[(i64, &str, Option<f64>, Option<f64>)]) -> Round {
        Round {
            t: 0,
            rows: Some(
                rows.iter()
                    .map(|&(id, label, five_h_pct, seven_d_pct)| LimitPercents {
                        id,
                        label: label.into(),
                        five_h_pct,
                        seven_d_pct,
                    })
                    .collect(),
            ),
        }
    }
    fn session(id: &str, project: &str, states: &[&str]) -> Value {
        let attention: Vec<Value> = states
            .iter()
            .map(|s| json!({ "state": s, "tool": "Bash", "count": 1, "is_sidechain": false, "since": "x" }))
            .collect();
        json!({ "session_id": id, "project": project, "account": 1, "attention": attention })
    }
    fn live(sessions: Vec<Value>) -> Value {
        json!({ "sessions": sessions })
    }
    /// A session whose states are `(state, tool, since)`.
    fn session_at(id: &str, project: &str, states: &[(&str, Option<&str>, &str)]) -> Value {
        let attention: Vec<Value> = states
            .iter()
            .map(|(s, tool, since)| json!({ "state": s, "tool": tool, "count": 1, "is_sidechain": false, "since": since }))
            .collect();
        json!({ "session_id": id, "project": project, "account": 1, "attention": attention })
    }
    /// The rows that name a session or the `+N more` row.
    fn session_rows_of(rows: &[Row]) -> Vec<&Row> {
        rows.iter()
            .filter(|r| matches!(r, Row::Session(..) | Row::Action(MORE, _)))
            .collect()
    }

    /// AC 1: the state rule, input by input, and the order problem > attention > active > idle.
    #[test]
    fn the_state_is_the_most_urgent_input() {
        let none = live(vec![]);
        assert_eq!(pick(&none, None, None), State::Idle);
        assert_eq!(pick(&Value::Null, None, None), State::Idle, "no live tail");
        assert_eq!(
            pick(&none, Some(&sample()), Some(&round(&[]))),
            State::Idle,
            "no data is not a state"
        );
        let one = live(vec![session("a", "p", &[])]);
        assert_eq!(pick(&one, None, None), State::Active);
        let waiting = live(vec![session("a", "p", &["waiting"])]);
        assert_eq!(pick(&waiting, None, None), State::Attention);
        let high = round(&[(1, "x", Some(90.0), None)]);
        assert_eq!(
            pick(&none, None, Some(&high)),
            State::Attention,
            "a 5h limit at 90 %"
        );
        let seven = round(&[(1, "x", Some(10.0), Some(95.0))]);
        assert_eq!(
            pick(&none, None, Some(&seven)),
            State::Attention,
            "a 7d limit at 95 %"
        );
        let below = round(&[(1, "x", Some(89.9), Some(89.0))]);
        assert_eq!(
            pick(&one, None, Some(&below)),
            State::Active,
            "89.9 % is not high"
        );
        let failing = live(vec![session("a", "p", &["failing"])]);
        assert_eq!(pick(&failing, None, None), State::Problem);
        let disk = Sample {
            disk_used: Some(90),
            disk_total: Some(100),
            ..sample()
        };
        assert_eq!(
            pick(&none, Some(&disk), None),
            State::Problem,
            "disk at 90 %"
        );
        let ram = Sample {
            ram_used: Some(950),
            ram_total: Some(1000),
            ..sample()
        };
        assert_eq!(pick(&none, Some(&ram), None), State::Problem, "RAM at 95 %");
        let low = Sample {
            battery_pct: Some(15),
            charging: Some(false),
            ..sample()
        };
        assert_eq!(
            pick(&none, Some(&low), None),
            State::Problem,
            "battery 15 % off the charger"
        );
        let charging = Sample {
            battery_pct: Some(5),
            charging: Some(true),
            ..sample()
        };
        assert_eq!(
            pick(&none, Some(&charging), None),
            State::Idle,
            "charging is no pressure"
        );
        let unknown = Sample {
            battery_pct: Some(5),
            charging: None,
            ..sample()
        };
        assert_eq!(
            pick(&none, Some(&unknown), None),
            State::Idle,
            "unknown charging is no pressure"
        );
        let calm = Sample {
            disk_used: Some(89),
            disk_total: Some(100),
            battery_pct: Some(16),
            charging: Some(false),
            ..sample()
        };
        assert_eq!(pick(&none, Some(&calm), None), State::Idle);
        // the order: a failing session beats a waiting one and a high limit
        let both = live(vec![
            session("a", "p", &["waiting"]),
            session("b", "q", &["failing"]),
        ]);
        assert_eq!(pick(&both, None, Some(&high)), State::Problem);
        assert_eq!(pick(&waiting, Some(&disk), None), State::Problem);
        // an unknown attention state ("attention" itself) is not waiting or failing
        let other = live(vec![session("a", "p", &["attention"])]);
        assert_eq!(pick(&other, None, None), State::Active);
    }

    /// The approved rows, in order, for the canvas's `attention` sample.
    #[test]
    fn the_menu_has_the_approved_rows_in_order() {
        let l = live(vec![
            session("a", "easy-memory", &["waiting"]),
            session("b", "kit", &[]),
        ]);
        let m = Sample {
            cpu_pct: Some(12.0),
            ram_used: Some(64),
            ram_total: Some(100),
            disk_used: Some(71),
            disk_total: Some(100),
            battery_pct: Some(80),
            charging: Some(true),
            ..sample()
        };
        let r = round(&[(1, "me", Some(78.0), Some(41.0))]);
        let v = view(&l, Some(&m), Some(&r), false);
        assert_eq!(v.state, State::Attention);
        assert_eq!(
            v.rows,
            vec![
                Row::Info("Easy Personal · attention".into()),
                Row::Separator,
                Row::Info("2 sessions active".into()),
                Row::Session("a".into(), "question · easy-memory · a".into()),
                Row::Info("5h limit · 78 % · 7d · 41 %".into()),
                Row::Info("CPU 12 % · RAM 64 % · disk 71 % · battery 80 %".into()),
                Row::Separator,
                Row::Action(OPEN, "Open dashboard".into()),
                Row::Check(PAUSE, "Pause alerts".into(), false),
                Row::Separator,
                Row::Info("127.0.0.1 · local only".into()),
                Row::Action(QUIT, "Quit".into()),
            ]
        );
        assert_eq!(
            view(&l, Some(&m), Some(&r), true).rows[8],
            Row::Check(PAUSE, "Pause alerts".into(), true)
        );
    }

    #[test]
    fn rows_say_no_data_and_never_zero() {
        let v = view(&Value::Null, None, None, false);
        assert_eq!(v.state, State::Idle);
        assert_eq!(v.rows[0], Row::Info("Easy Personal · idle".into()));
        assert_eq!(v.rows[2], Row::Info("No sessions running".into()));
        assert_eq!(
            v.rows[3],
            Row::Info("5h limit · no data · 7d · no data".into())
        );
        assert_eq!(v.rows[4], Row::Info("Machine · no data".into()));
        assert_eq!(v.rows.len(), 11, "no waiting or failing row");
        let desktop = Sample {
            cpu_pct: Some(3.4),
            ram_used: Some(1),
            ram_total: Some(4),
            ..sample()
        };
        let stale = round(&[(1, "me", None, Some(41.0))]);
        let v = view(
            &live(vec![session("a", "p", &[])]),
            Some(&desktop),
            Some(&stale),
            false,
        );
        assert_eq!(v.rows[2], Row::Info("1 session active".into()));
        assert_eq!(
            v.rows[3],
            Row::Info("5h limit · no data · 7d · 41 %".into())
        );
        assert_eq!(
            v.rows[4],
            Row::Info("CPU 3 % · RAM 25 % · disk no data".into()),
            "no battery part"
        );
        assert_eq!(
            view(&Value::Null, Some(&sample()), None, false).rows[4],
            Row::Info("Machine · no data".into())
        );
    }

    /// T096 AC 1: the two count rows are gone: one row per waiting session, a session with several states gets one row.
    #[test]
    fn several_sessions_and_accounts_and_failing_rows() {
        let l = live(vec![
            session("a", "easy-memory", &["waiting"]),
            session("b", "easy-memory", &["waiting", "failing"]),
            session("c", "kit", &["waiting"]),
            session("d", "web", &["waiting"]),
        ]);
        let r = round(&[
            (1, "work", Some(10.0), Some(20.0)),
            (2, "", Some(91.0), None),
        ]);
        let v = view(&l, None, Some(&r), false);
        assert_eq!(v.state, State::Problem);
        assert_eq!(v.rows[0], Row::Info("Easy Personal · problem".into()));
        let text = format!("{:?}", v.rows);
        assert!(
            !text.contains("waiting for you") && !text.contains("1 failing"),
            "{text}"
        );
        // the "x" since of every session ties: live order stays; `b` has failing first
        assert_eq!(
            v.rows[3..7],
            [
                Row::Session("a".into(), "question · easy-memory · a".into()),
                Row::Session("b".into(), "failing · easy-memory · b".into()),
                Row::Session("c".into(), "question · kit · c".into()),
                Row::Session("d".into(), "question · web · d".into()),
            ]
        );
        assert_eq!(
            v.rows[7],
            Row::Info("work: 5h limit · 10 % · 7d · 20 %".into())
        );
        assert_eq!(
            v.rows[8],
            Row::Info("account 2: 5h limit · 91 % · 7d · no data".into()),
            "an empty label"
        );
    }

    /// T096 AC 1: each of the four kinds, in fixed words, with the project folder and the 8-character session id.
    #[test]
    fn a_row_names_its_kind_the_project_and_the_short_id() {
        let l = live(vec![
            session_at(
                "0123abcd-0000-4000-8000-000000000001",
                "shop",
                &[("waiting", Some("AskUserQuestion"), "2026-01-20T00:00:01Z")],
            ),
            session_at(
                "4567ef01-0000-4000-8000-000000000002",
                "blog",
                &[("waiting", Some("ExitPlanMode"), "2026-01-20T00:00:02Z")],
            ),
            session_at(
                "89abcdef-0000-4000-8000-000000000003",
                "docs",
                &[("your_turn", None, "2026-01-20T00:00:03Z")],
            ),
            session_at(
                "deadbeef-0000-4000-8000-000000000004",
                "api",
                &[("failing", Some("Bash"), "2026-01-20T00:00:04Z")],
            ),
        ]);
        let rows = view(&l, None, None, false).rows;
        let texts: Vec<&str> = session_rows_of(&rows)
            .into_iter()
            .map(|r| match r {
                Row::Session(_, t) => t.as_str(),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(
            texts,
            [
                "failing · api · deadbeef",
                "your turn · docs · 89abcdef",
                "plan to approve · blog · 4567ef01",
                "question · shop · 0123abcd",
            ],
            "newest state first"
        );
        let ids: Vec<&str> = session_rows_of(&rows)
            .into_iter()
            .filter_map(|r| match r {
                Row::Session(id, _) => Some(id.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            ids[0], "deadbeef-0000-4000-8000-000000000004",
            "the row keeps the full id"
        );
    }

    /// T096 AC 1: at most 5 rows, then one clickable row for the rest.
    #[test]
    fn five_session_rows_at_most_and_a_row_for_the_rest() {
        let many = |n: usize| {
            live(
                (0..n)
                    .map(|i| {
                        session_at(
                            &format!("0000000{i}"),
                            "p",
                            &[("your_turn", None, &format!("2026-01-20T00:00:0{i}Z"))],
                        )
                    })
                    .collect(),
            )
        };
        for (n, shown, more) in [(0, 0, None), (5, 5, None), (6, 5, Some(1)), (9, 5, Some(4))] {
            let rows = view(&many(n), None, None, false).rows;
            let found = session_rows_of(&rows);
            let sessions = found
                .iter()
                .filter(|r| matches!(r, Row::Session(..)))
                .count();
            assert_eq!(sessions, shown, "{n} sessions");
            match more {
                None => assert_eq!(found.len(), shown, "{n}: no more row"),
                Some(k) => assert_eq!(
                    found.last(),
                    Some(&&Row::Action(MORE, format!("+{k} more in dashboard…"))),
                    "{n}"
                ),
            }
        }
        // newest first: the 6th (oldest) session is the one left out
        let rows = view(&many(6), None, None, false).rows;
        assert_eq!(
            rows.iter()
                .filter_map(|r| match r {
                    Row::Session(id, _) => Some(id.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            ["00000005", "00000004", "00000003", "00000002", "00000001"]
        );
    }

    /// T096 AC 1: one row per session, named by the first of failing, waiting, your_turn; a session with only another
    /// state (needs_attention) has no row; a calm machine's menu has no session row.
    #[test]
    fn a_session_with_several_states_gets_one_row_by_its_first_state() {
        let l = live(vec![
            session_at(
                "aaaaaaaa",
                "p",
                &[
                    ("your_turn", None, "2026-01-20T00:00:09Z"),
                    ("waiting", Some("AskUserQuestion"), "2026-01-20T00:00:05Z"),
                    ("failing", Some("Bash"), "2026-01-20T00:00:01Z"),
                ],
            ),
            session_at(
                "bbbbbbbb",
                "p",
                &[
                    ("your_turn", None, "2026-01-20T00:00:02Z"),
                    ("waiting", Some("ExitPlanMode"), "2026-01-20T00:00:03Z"),
                ],
            ),
            session_at(
                "cccccccc",
                "p",
                &[("needs_attention", Some("Bash"), "2026-01-20T00:00:04Z")],
            ),
            session("dddddddd", "p", &[]),
        ]);
        let rows = view(&l, None, None, false).rows;
        assert_eq!(
            session_rows_of(&rows),
            [
                &Row::Session("bbbbbbbb".into(), "plan to approve · p · bbbbbbbb".into()),
                &Row::Session("aaaaaaaa".into(), "failing · p · aaaaaaaa".into()),
            ],
            "sorted by the named state's own `since`: waiting 03 is newer than failing 01"
        );
        let calm = view(&live(vec![session("a", "p", &[])]), None, None, false).rows;
        assert!(session_rows_of(&calm).is_empty());
    }

    /// T096 edge cases: a short id, an empty or secret project, a missing id leave a piece out and never panic.
    #[test]
    fn a_row_leaves_out_a_piece_it_does_not_have() {
        let secret = "sk-ant-api03-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let l = live(vec![
            session_at("abc", "shop", &[("your_turn", None, "5")]),
            session_at("12345678", "", &[("your_turn", None, "4")]),
            session_at("87654321", secret, &[("your_turn", None, "3")]),
            session_at("a b<c>", "p", &[("your_turn", None, "2")]),
            json!({ "project": "p", "attention": [{"state": "your_turn", "since": "1"}] }),
            json!({ "session_id": 7, "project": "p", "attention": [{"state": "your_turn", "since": "1"}] }),
            json!({ "session_id": "11111111", "project": "p", "attention": "nope" }),
        ]);
        let rows = view(&l, None, None, false).rows;
        let texts: Vec<String> = session_rows_of(&rows)
            .into_iter()
            .map(|r| match r {
                Row::Session(_, t) => t.clone(),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(
            texts,
            [
                "your turn · shop · abc",
                "your turn · 12345678",
                "your turn · 87654321",
                "your turn · p",
            ],
            "a short id as it is; no project; a secret project and an id with a space or markup each left out"
        );
        let text = format!("{rows:?}");
        assert!(!text.contains("sk-ant"), "{text}");
    }

    /// T096 AC 3: a your-turn session alone makes the icon *attention*; a failing session beats it.
    #[test]
    fn a_your_turn_session_is_attention_and_failing_stays_more_urgent() {
        let turn = live(vec![session("a", "p", &["your_turn"])]);
        assert_eq!(pick(&turn, None, None), State::Attention);
        assert_eq!(view(&turn, None, None, false).state, State::Attention);
        let both = live(vec![
            session("a", "p", &["your_turn"]),
            session("b", "p", &["failing"]),
        ]);
        assert_eq!(pick(&both, None, None), State::Problem);
    }

    /// T096 AC 2, 9: the row id carries the session; the same text goes back to the session.
    #[test]
    fn a_menu_item_id_carries_the_session() {
        assert_eq!(session_item_id("0123-abcd"), "session:0123-abcd");
        assert_eq!(session_of("session:0123-abcd"), Some("0123-abcd"));
        assert_eq!(session_of("session:"), Some(""));
        for other in [
            OPEN,
            ALERTS,
            MORE,
            PAUSE,
            QUIT,
            "",
            "sessions:x",
            "Session:x",
        ] {
            assert_eq!(session_of(other), None, "{other}");
        }
    }

    /// T096 AC 2: the menu is rebuilt when the set of sessions changes, updated in place when only a text does.
    #[test]
    fn the_session_rows_decide_update_or_rebuild() {
        let one = |id: &str, project: &str| {
            view(
                &live(vec![session(id, project, &["your_turn"])]),
                None,
                None,
                false,
            )
            .rows
        };
        assert!(same_shape(&one("a", "p"), &one("a", "q")), "a text only");
        assert!(
            !same_shape(&one("a", "p"), &one("b", "p")),
            "another session in the same place"
        );
        let calm = view(&Value::Null, None, None, false).rows;
        assert!(!same_shape(&calm, &one("a", "p")), "a row comes");
        let two = view(
            &live(vec![
                session("a", "p", &["your_turn"]),
                session("b", "p", &["your_turn"]),
            ]),
            None,
            None,
            false,
        )
        .rows;
        assert!(!same_shape(&one("a", "p"), &two), "a second row");
        assert!(!same_shape(
            &[Row::Session("a".into(), "x".into())],
            &[Row::Action(MORE, "x".into())]
        ));
        assert!(!same_shape(
            &[Row::Info("x".into())],
            &[Row::Session("a".into(), "x".into())]
        ));
    }

    /// NFR-002: a project folder or label that looks like a secret, or holds markup or a newline, never reaches a row.
    #[test]
    fn no_secret_or_markup_reaches_a_row() {
        let secret = "sk-ant-api03-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let l = live(vec![
            session("a", secret, &["waiting"]),
            session("b", "a<b>&\nc", &["failing"]),
        ]);
        let r = round(&[(1, secret, Some(1.0), None), (2, "x", None, None)]);
        let v = view(&l, None, Some(&r), false);
        let text = format!("{:?}", v.rows);
        assert!(!text.contains("sk-ant"), "{text}");
        assert!(
            !text.contains('<') && !text.contains('&') && !text.contains('\n'),
            "{text}"
        );
        assert_eq!(v.rows[3], Row::Session("a".into(), "question · a".into()));
        assert_eq!(
            v.rows[4],
            Row::Session("b".into(), "failing · abc · b".into())
        );
        assert_eq!(
            v.rows[5],
            Row::Info("account 1: 5h limit · 1 % · 7d · no data".into())
        );
    }

    /// T094 (FR-106, NFR-002): a real live tail with an open `AskUserQuestion` whose question, header and option are
    /// planted: the tail holds the question for the stream (positive control), and no menu row holds any of it.
    #[test]
    fn no_question_text_reaches_a_row_from_a_real_live_tail() {
        use easy_personal_core::live::{LiveTail, Root};
        use easy_personal_core::prices::PriceTable;
        let dir =
            std::env::temp_dir().join(format!("easy-personal-rs-t094-tray-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("p")).unwrap();
        let roots = [Root {
            account: None,
            projects: dir.clone(),
        }];
        let now = 1_768_867_210 * 1_000_000; // 2026-01-20T00:00:10Z
        let mut tail = LiveTail::new(PriceTable::shipped().unwrap());
        tail.poll_once(&roots, now, None);
        let line = json!({"type": "assistant", "sessionId": "s1", "timestamp": "2026-01-20T00:00:05Z",
            "cwd": "/work/shop", "entrypoint": "cli", "message": {"id": "m1", "model": "x", "content": [
                {"type": "tool_use", "id": "b1", "name": "AskUserQuestion", "input": {"questions": [
                    {"question": "planted question text", "header": "planted header",
                     "options": [{"label": "planted label", "description": "planted description"}]}]}}]}});
        std::fs::write(dir.join("p/s.jsonl"), format!("{line}\n")).unwrap();
        tail.poll_once(&roots, now, None);
        let held = format!("{:?}", tail.waiting_questions());
        let v = view(&tail.snapshot(now), None, None, false);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(held.contains("planted question text"), "{held}");
        assert_eq!(
            v.rows[3],
            Row::Session("s1".into(), "question · shop · s1".into())
        );
        let text = format!("{:?}", v.rows);
        assert!(!text.contains("planted"), "{text}");
    }

    #[test]
    fn the_theme_is_light_only_when_the_desktop_says_light() {
        assert_eq!(theme_of(Some(2)), Theme::Light);
        for other in [None, Some(0), Some(1), Some(7)] {
            assert_eq!(theme_of(other), Theme::Dark, "{other:?}");
        }
    }

    /// AC 3 (Restated intent 3): with no session bus there is no tray area and no theme, and nothing panics.
    #[test]
    fn no_bus_means_no_tray_area_and_the_dark_theme() {
        let gone = Some("unix:path=/nonexistent-easy-personal-test/bus");
        assert!(!tray_host(gone));
        assert_eq!(theme_of(desktop_color_scheme(gone)), Theme::Dark);
    }

    /// AC 3: one tray per user. A second lock on the same file is refused while the first is held, and is given once
    /// the first lets go.
    #[test]
    fn the_lock_lets_one_tray_run() {
        let dir = std::env::temp_dir().join(format!("ep-tray-lock-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = lock_path(Some(dir.clone().into_os_string()), None).unwrap();
        assert_eq!(path, dir.join("easy-personal-tray.lock"));
        let first = try_lock(&path).unwrap();
        assert!(first.is_some());
        assert!(
            try_lock(&path).unwrap().is_none(),
            "a second tray is refused"
        );
        drop(first);
        assert!(
            try_lock(&path).unwrap().is_some(),
            "free again once the first ends"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    /// Security fix round (Stage 4): the icon folder follows the lock's rule and is never the shared temp folder.
    #[test]
    fn the_icon_folder_is_private_and_never_tmp() {
        let store = PathBuf::from("/home/u/.easy-personal/store.sqlite");
        let at = |rt: Option<&str>| icon_dir(rt.map(OsString::from), Some(store.clone())).unwrap();
        assert_eq!(
            at(Some("/run/user/1000")),
            PathBuf::from("/run/user/1000/tray-icon")
        );
        assert_eq!(
            at(None),
            PathBuf::from("/home/u/.easy-personal/tray-icon"),
            "XDG_RUNTIME_DIR unset"
        );
        assert_eq!(
            at(Some("")),
            PathBuf::from("/home/u/.easy-personal/tray-icon"),
            "empty"
        );
        assert_eq!(
            at(Some("tmp")),
            PathBuf::from("/home/u/.easy-personal/tray-icon"),
            "relative"
        );
        assert!(
            icon_dir(None, None).is_err(),
            "no folder: an error, not /tmp"
        );
        assert_eq!(
            lock_path(None, Some(store.clone())).unwrap().parent(),
            icon_dir(None, Some(store)).unwrap().parent(),
            "the same folder as the lock"
        );
    }

    #[test]
    fn a_symlinked_lock_is_refused() {
        let dir = std::env::temp_dir().join(format!("ep-tray-locklink-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("victim");
        fs::write(&target, "keep").unwrap();
        let path = dir.join("easy-personal-tray.lock");
        std::os::unix::fs::symlink(&target, &path).unwrap();
        assert!(try_lock(&path).is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "keep");
        fs::remove_dir_all(&dir).unwrap();
    }

    /// Code-review fix round: a menu whose rows keep their places is updated in place; a row that comes or goes, or
    /// changes kind, rebuilds it.
    #[test]
    fn the_shape_decides_update_or_rebuild() {
        let l = |s: Vec<Value>| live(s);
        let calm = view(&l(vec![]), None, None, false).rows;
        let paused = view(&l(vec![]), None, None, true).rows;
        let busy = view(&l(vec![session("a", "p", &[])]), None, None, false).rows;
        let cpu = view(
            &l(vec![]),
            Some(&Sample {
                cpu_pct: Some(5.0),
                ..sample()
            }),
            None,
            false,
        )
        .rows;
        assert!(same_shape(&calm, &paused), "a tick");
        assert!(same_shape(&calm, &busy), "a text");
        assert!(same_shape(&calm, &cpu), "the machine line");
        let waiting = view(&l(vec![session("a", "p", &["waiting"])]), None, None, false).rows;
        assert!(!same_shape(&calm, &waiting), "a session row comes");
        let two = round(&[(1, "a", None, None), (2, "b", None, None)]);
        assert!(
            !same_shape(&calm, &view(&l(vec![]), None, Some(&two), false).rows),
            "a second account row"
        );
        assert!(!same_shape(
            &[Row::Info("x".into())],
            &[Row::Action(OPEN, "x".into())]
        ));
        assert!(!same_shape(
            &[Row::Action(OPEN, "x".into())],
            &[Row::Action(QUIT, "x".into())]
        ));
    }

    #[test]
    fn the_lock_falls_back_beside_the_store() {
        let store = PathBuf::from("/home/u/.easy-personal/store.sqlite");
        let at = |rt: Option<&str>| lock_path(rt.map(OsString::from), Some(store.clone())).unwrap();
        assert_eq!(
            at(None),
            PathBuf::from("/home/u/.easy-personal/easy-personal-tray.lock")
        );
        assert_eq!(
            at(Some("relative")),
            PathBuf::from("/home/u/.easy-personal/easy-personal-tray.lock")
        );
        assert_eq!(
            at(Some("/run/user/1000")),
            PathBuf::from("/run/user/1000/easy-personal-tray.lock")
        );
        assert!(lock_path(None, None).is_err());
    }

    fn listing(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        let mut out = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    out.push((path.clone(), Vec::new()));
                    stack.push(path);
                } else {
                    out.push((path.clone(), fs::read(&path).unwrap()));
                }
            }
        }
        out.sort();
        out
    }

    /// AC 2: `on` writes exactly the one file with the exact text; `off` removes it; every other file and folder of the
    /// config home (another autostart entry included) is left as it was.
    #[test]
    fn autostart_writes_and_removes_only_its_file() {
        let root = std::env::temp_dir().join(format!("ep-tray-xdg-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("autostart")).unwrap();
        fs::write(
            root.join("autostart/other.desktop"),
            "[Desktop Entry]\nName=other\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("gtk-3.0")).unwrap();
        fs::write(root.join("gtk-3.0/settings.ini"), "x=1\n").unwrap();
        let before = listing(&root);
        let path = autostart_path(
            Some(root.clone().into_os_string()),
            Some("/nonexistent-home".into()),
        )
        .unwrap();
        assert_eq!(path, root.join("autostart/easy-personal-tray.desktop"));
        let entry = desktop_entry(
            Path::new("/opt/ep dir/easy-personal-rs"),
            Path::new("/srv/web/dist"),
        )
        .unwrap();
        assert_eq!(
            entry,
            "[Desktop Entry]\nType=Application\nName=Easy Personal tray\n\
             Comment=Tray icon and desktop alerts of easy-personal\n\
             Exec=\"/opt/ep dir/easy-personal-rs\" tray --static-dir \"/srv/web/dist\"\n\
             Terminal=false\nNoDisplay=true\nX-GNOME-Autostart-enabled=true\n"
        );
        autostart_on(&path, &entry).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), entry);
        let mut after = listing(&root);
        after.retain(|(p, _)| p != &path);
        assert_eq!(after, before, "only the one file was added");
        autostart_on(&path, &entry).unwrap(); // a second `on` overwrites, adds nothing
        assert_eq!(fs::read_to_string(&path).unwrap(), entry);
        assert!(autostart_off(&path).unwrap());
        assert_eq!(
            listing(&root),
            before,
            "off leaves everything else as it was"
        );
        assert!(
            !autostart_off(&path).unwrap(),
            "off again: it was not there"
        );
        assert_eq!(listing(&root), before);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn autostart_makes_the_folder_and_refuses_a_symlink() {
        let root = std::env::temp_dir().join(format!("ep-tray-xdg2-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let path = autostart_path(Some(root.clone().into_os_string()), None).unwrap();
        autostart_on(&path, "x").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "x");
        fs::remove_file(&path).unwrap();
        let target = root.join("target.txt");
        fs::write(&target, "keep").unwrap();
        std::os::unix::fs::symlink(&target, &path).unwrap();
        assert!(autostart_on(&path, "x").is_err());
        assert_eq!(
            fs::read_to_string(&target).unwrap(),
            "keep",
            "the link's target is not written"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn autostart_path_follows_the_xdg_rules() {
        let home = Some(OsString::from("/home/u"));
        let want = PathBuf::from("/home/u/.config/autostart/easy-personal-tray.desktop");
        assert_eq!(autostart_path(None, home.clone()).unwrap(), want);
        assert_eq!(
            autostart_path(Some("".into()), home.clone()).unwrap(),
            want,
            "empty is unset"
        );
        assert_eq!(
            autostart_path(Some("rel".into()), home.clone()).unwrap(),
            want,
            "relative is ignored"
        );
        assert_eq!(
            autostart_path(Some("/x/cfg".into()), home).unwrap(),
            PathBuf::from("/x/cfg/autostart/easy-personal-tray.desktop")
        );
        assert!(autostart_path(None, None).is_err());
    }

    /// A path that would need escaping, or could add a key with a newline, is refused rather than written.
    #[test]
    fn an_entry_refuses_a_path_it_cannot_carry() {
        let ok = Path::new("/bin/ep");
        for bad in [
            "/a\nExec=evil",
            "/a\"b",
            "/a$HOME",
            "/a`x`",
            "/a\\b",
            "/a%f",
        ] {
            assert!(desktop_entry(ok, Path::new(bad)).is_err(), "{bad:?}");
            assert!(desktop_entry(Path::new(bad), ok).is_err(), "{bad:?}");
        }
    }

    /// T096 AC 9: the address of a click, for a session, for the Live view alone and for the plain open.
    #[test]
    fn the_address_names_the_session_and_keeps_the_token_in_the_fragment() {
        let id = "0123abcd-4567-4000-8000-0123456789ab";
        assert_eq!(
            address(41234, "TOK", &Target::Live(Some(id.into()))),
            format!("http://127.0.0.1:41234/?live={id}#TOK")
        );
        assert_eq!(
            address(41234, "TOK", &Target::Live(None)),
            "http://127.0.0.1:41234/?live#TOK"
        );
        assert_eq!(
            address(41234, "TOK", &Target::Dashboard),
            "http://127.0.0.1:41234/#TOK"
        );
        // 64 characters are the most; the upper and lower case of hex digits both pass
        let longest = "aB-".repeat(21) + "c";
        assert_eq!(longest.len(), 64);
        assert_eq!(
            address(1, "T", &Target::Live(Some(longest.clone()))),
            format!("http://127.0.0.1:1/?live={longest}#T")
        );
    }

    /// T096 AC 9: an id that is not a session id never reaches the address (no markup, no second parameter, no
    /// escape): the Live view with no session is what the click gets.
    #[test]
    fn an_id_that_is_not_a_session_id_is_left_out() {
        let too_long = "a".repeat(65);
        for bad in [
            "a b",
            "x&y=1",
            "%0a",
            "<script>",
            "",
            "g1",
            "ab\n",
            "ab#cd",
            "ab?cd",
            "ab/cd",
            "é1",
            too_long.as_str(),
        ] {
            let got = address(41234, "TOK", &Target::Live(Some(bad.into())));
            assert_eq!(got, "http://127.0.0.1:41234/?live#TOK", "{bad:?}");
        }
    }

    /// T096 AC 9: the token comes after the `#` and nowhere else, in all three shapes.
    #[test]
    fn the_token_is_only_ever_in_the_fragment() {
        for target in [
            Target::Dashboard,
            Target::Live(None),
            Target::Live(Some("abc-123".into())),
            Target::Live(Some("x&token=y".into())),
        ] {
            let got = address(5, "SECRETTOKEN", &target);
            let (before, after) = got.split_once('#').unwrap();
            assert_eq!(after, "SECRETTOKEN", "{got}");
            assert!(!before.contains("SECRETTOKEN"), "{got}");
        }
    }

    /// T096 AC 9: a menu row and a popup reach the same function: the row's item id and the popup's stored id give
    /// the same address.
    #[test]
    fn a_row_and_a_popup_give_the_same_address() {
        let l = live(vec![session_at(
            "0123abcd-0000-4000-8000-000000000001",
            "shop",
            &[("your_turn", None, "1")],
        )]);
        let rows = view(&l, None, None, false).rows;
        let Some(Row::Session(id, _)) = rows.iter().find(|r| matches!(r, Row::Session(..))) else {
            panic!("{rows:?}")
        };
        let item = session_item_id(id);
        let from_row = address(9, "T", &Target::Live(session_of(&item).map(str::to_string)));
        let from_popup = address(9, "T", &Target::Live(Some(id.clone())));
        assert_eq!(from_row, from_popup);
        assert_eq!(
            from_row,
            "http://127.0.0.1:9/?live=0123abcd-0000-4000-8000-000000000001#T"
        );
    }

    // -- T088: the "Recent alerts" block ------------------------------------------------------------------------------

    /// 2026-11-15 14:02:00 UTC.
    const NOW: i64 = 1_794_751_320;

    fn alert(t: i64, text: &str) -> Alert {
        Alert {
            t,
            kind: "disk",
            text: text.into(),
            account: None,
            session: None,
            open: None,
        }
    }
    /// `n` alerts newest first, one minute apart, the newest at `NOW`; texts `a0` (newest), `a1`, ...
    fn recent(n: usize) -> Vec<Alert> {
        (0..n)
            .map(|i| alert(NOW - 60 * i as i64, &format!("a{i}")))
            .collect()
    }
    fn with(alerts: &[Alert], paused: bool, offset: fn(i64) -> i64) -> Vec<Row> {
        super::view(&Value::Null, None, None, paused, alerts, NOW, offset).rows
    }
    /// The rows between the machine line and the second separator, i.e. the alert block (separator included).
    fn block(rows: &[Row]) -> &[Row] {
        let from = rows
            .iter()
            .position(|r| matches!(r, Row::Info(t) if t == "Machine · no data"))
            .unwrap()
            + 1;
        let to = rows
            .iter()
            .position(|r| matches!(r, Row::Action(OPEN, _)))
            .unwrap();
        &rows[from..to]
    }

    /// AC 1: no alert, no block: the rows are today's rows.
    #[test]
    fn no_alert_means_no_block() {
        assert_eq!(
            with(&[], false, utc),
            view(&Value::Null, None, None, false).rows
        );
        assert_eq!(block(&with(&[], false, utc)), [Row::Separator]);
    }

    /// AC 2: header, then newest first as `HH:MM · text`; no more row.
    #[test]
    fn three_alerts_list_newest_first() {
        let rows = with(&recent(3), false, utc);
        assert_eq!(
            block(&rows),
            [
                Row::Separator,
                Row::Info("Recent alerts".into()),
                Row::Info("14:02 · a0".into()),
                Row::Info("14:01 · a1".into()),
                Row::Info("14:00 · a2".into()),
                Row::Separator,
            ]
        );
        assert!(!rows.iter().any(|r| matches!(r, Row::Action(ALERTS, _))));
        // local time: +07:00 moves the clock, the offset is the zone's at that moment
        assert_eq!(
            block(&with(&recent(1), false, plus7))[2],
            Row::Info("21:02 · a0".into())
        );
    }

    /// AC 3: at most 5 rows; the dashboard row only above 5.
    #[test]
    fn five_rows_at_most_and_a_row_for_the_rest() {
        for (n, rows, more) in [(1, 1, false), (5, 5, false), (6, 5, true), (50, 5, true)] {
            let all = with(&recent(n.min(6)), false, utc);
            let b = block(&all);
            let shown = b
                .iter()
                .filter(|r| matches!(r, Row::Info(t) if t.contains(" · a")))
                .count();
            assert_eq!(shown, rows, "{n} alerts");
            let action = Row::Action(ALERTS, "All alerts in dashboard…".into());
            assert_eq!(b.contains(&action), more, "{n} alerts");
            if more {
                assert_eq!(b[b.len() - 2], action, "the row ends the block");
            }
        }
        // the view is given 6 at most (`Shared::recent(6)`); a longer slice is still cut to 5
        let b = with(&recent(50), false, utc);
        let b = block(&b);
        assert_eq!(b.len(), 1 + 1 + 5 + 1 + 1);
    }

    /// AC 4: an earlier local day shows the date, in the local zone (18:30 UTC on the 14th is 01:30 on the 15th at +7).
    #[test]
    fn an_earlier_day_shows_the_date() {
        let a = [
            alert(NOW, "today"),
            alert(NOW - 86_400, "day before"),
            alert(NOW - 86_400 * 40, "month before"),
        ];
        assert_eq!(
            block(&with(&a, false, utc))[2..5],
            [
                Row::Info("14:02 · today".into()),
                Row::Info("11-14 14:02 · day before".into()),
                Row::Info("10-06 14:02 · month before".into()),
            ]
        );
        // 2026-11-14 18:30 UTC is 11-15 01:30 at +7: the same local day as NOW (21:02 at +7), no date
        let a = [alert(1_794_681_000, "late")];
        assert_eq!(
            block(&with(&a, false, plus7))[2],
            Row::Info("01:30 · late".into())
        );
        // the same instants in UTC are two days
        assert_eq!(
            block(&with(&a, false, utc))[2],
            Row::Info("11-14 18:30 · late".into())
        );
    }

    /// AC 5: paused stops popups only; the rows stay and the check item is ticked.
    #[test]
    fn paused_still_lists_the_alerts() {
        let rows = with(&recent(2), true, utc);
        assert_eq!(block(&rows)[2], Row::Info("14:02 · a0".into()));
        assert!(rows.contains(&Row::Check(PAUSE, "Pause alerts".into(), true)));
    }

    /// AC 6: the block coming is a rebuild; the same count between two looks is an update in place.
    #[test]
    fn the_block_changes_the_shape_once() {
        let none = with(&[], false, utc);
        let one = with(&recent(1), false, utc);
        let two = with(&recent(2), false, utc);
        let mut changed = recent(2);
        changed[0].text = "other".into();
        changed[0].t += 60;
        assert!(!same_shape(&none, &one), "the first alert");
        assert!(!same_shape(&one, &two), "a second row");
        assert!(same_shape(&two, &with(&changed, false, utc)), "same count");
        assert!(same_shape(
            &with(&recent(5), false, utc),
            &with(&recent(5), true, utc)
        ));
        assert!(
            !same_shape(&with(&recent(5), false, utc), &with(&recent(6), false, utc)),
            "the dashboard row comes"
        );
        assert!(same_shape(
            &with(&recent(6), false, utc),
            &with(&recent(6), false, utc)
        ));
    }

    /// A long text is cut with an ellipsis; a short one is kept whole.
    #[test]
    fn a_long_alert_text_is_cut() {
        let long = "x".repeat(200);
        let rows = with(&[alert(NOW, &long)], false, utc);
        let Row::Info(text) = &block(&rows)[2] else {
            panic!("{rows:?}")
        };
        assert_eq!(
            text.chars().count(),
            "14:02 · ".chars().count() + ROW_TEXT_MAX
        );
        assert!(text.ends_with('…'), "{text}");
        let fit = "y".repeat(ROW_TEXT_MAX);
        let rows = with(&[alert(NOW, &fit)], false, utc);
        assert_eq!(block(&rows)[2], Row::Info(format!("14:02 · {fit}")));
    }

    /// The real zone lookup answers a plausible offset (within a day) and never panics.
    #[test]
    fn the_local_offset_is_within_a_day() {
        assert!(local_offset(NOW).abs() <= 14 * 3600);
        assert!(
            local_offset(i64::MAX).abs() <= 14 * 3600,
            "an unreadable time gives 0"
        );
    }
}
