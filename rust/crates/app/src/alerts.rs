//! Watch 3 (T074, FR-097): the alert engine. Every [`TICK`] one thread (a [`Sampler`], `sampler.rs`) reads the live tail's
//! snapshot, the newest machine sample and the newest plan-limit round, and raises an *alert* once per state change:
//! a session *waiting for you* or *failing* (re-armed when the session leaves the state), a plan limit at or above 90 %
//! per account and window, disk or RAM at or above 90 % used (each re-armed below 85 %), a battery at or below 15 % and
//! not charging (re-armed at 20 % or more, or charging). A measure with no data changes nothing.
//!
//! An alert is kept in a list of the last [`KEEP`] (`GET /api/alerts`), always, and goes to the desktop through a
//! [`Notifier`] unless the alerts are paused. At most one popup per [`POPUP_GAP`] of monotonic time (a clock jump makes
//! no burst); what is raised meanwhile is summarised in the next popup ("3 sessions waiting"). Pausing stops popups and
//! drops what waits for one; it never stops the state or the list. Nothing is written to disk or to the store.
//!
//! A text names only the state, the tool name, the project folder name, the short session id (8 characters), or the
//! measure and its number (NFR-002). It is built from fixed keys of the snapshot, never from a message, a tool input or
//! an output, and every free piece passes the secret check of the core (`ingest::secret::is_secret`). The real notifier
//! calls `org.freedesktop.Notifications.Notify` on the session D-Bus (NFR-001) and fails soft.

use crate::keychain::off_runtime;
use crate::sampler::{History, Sampler};
use easy_personal_core::clock::now_micros;
use easy_personal_core::ingest::secret::is_secret;
use easy_personal_core::limits_watch::Round;
use easy_personal_core::machine::Sample;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// Alerts kept for `GET /api/alerts`.
pub(crate) const KEEP: usize = 50;
/// The engine looks every this long.
const TICK: Duration = Duration::from_secs(5);
/// The least time between two popups.
const POPUP_GAP: Duration = Duration::from_secs(10);
/// A measure at or above this percent raises an alert.
const HIGH: f64 = 90.0;
/// A measure below this percent re-arms it.
const REARM_HIGH: f64 = 85.0;
/// A battery at or below this percent (and not charging) raises an alert.
const LOW_BATTERY: u8 = 15;
/// A battery at or above this percent re-arms it.
const REARM_BATTERY: u8 = 20;
const SHORT_ID: usize = 8;
const PIECE_MAX: usize = 64;

/// One alert: what `GET /api/alerts` lists.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Alert {
    /// Seconds since the epoch (UTC).
    pub t: i64,
    /// `session_waiting`, `session_failing`, `plan_limit`, `disk`, `ram` or `battery`.
    pub kind: &'static str,
    pub text: String,
    pub account: Option<i64>,
    /// The short session id.
    pub session: Option<String>,
}

impl Alert {
    fn json(&self) -> Value {
        json!({ "t": self.t, "kind": self.kind, "text": self.text, "account": self.account, "session": self.session })
    }
}

/// What the route reads and the engine writes: the last [`KEEP`] alerts, and whether popups are paused (in memory only).
#[derive(Default)]
pub(crate) struct Shared {
    list: Mutex<VecDeque<Alert>>,
    paused: AtomicBool,
}

impl Shared {
    pub(crate) fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub(crate) fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    fn push(&self, alert: Alert) {
        let mut list = self.list.lock().unwrap_or_else(PoisonError::into_inner);
        if list.len() == KEEP {
            list.pop_front();
        }
        list.push_back(alert);
    }

    /// `{alerts: [...newest first], paused}`.
    pub(crate) fn json(&self) -> Value {
        let list = self.list.lock().unwrap_or_else(PoisonError::into_inner);
        json!({
            "alerts": list.iter().rev().map(Alert::json).collect::<Vec<_>>(),
            "paused": self.paused(),
        })
    }
}

/// Shows a desktop popup. It never fails the caller.
pub(crate) trait Notifier: Send {
    fn notify(&mut self, summary: &str, body: &str);
}

/// The engine's state: which alerts are tripped (so they raise once), and the popups that wait.
pub(crate) struct Engine {
    shared: Arc<Shared>,
    notifier: Box<dyn Notifier>,
    /// Measure keys (`disk`, `ram`, `battery`, `limit:<account>:5h`) that raised and are not re-armed yet.
    tripped: HashSet<String>,
    /// The (session id, state) pairs seen on the last tick.
    sessions_on: HashSet<(String, &'static str)>,
    waiting: Vec<Alert>,
    last_popup: Option<Instant>,
}

/// Raise-once with hysteresis. Returns true when `trigger` holds and the key was armed. A tripped key is re-armed
/// only when `rearm` holds; between the two the state stays.
fn cross(tripped: &mut HashSet<String>, key: &str, trigger: bool, rearm: bool) -> bool {
    if tripped.contains(key) {
        if rearm {
            tripped.remove(key);
        }
        false
    } else if trigger {
        tripped.insert(key.to_string());
        true
    } else {
        false
    }
}

fn pct(used: Option<u64>, total: Option<u64>) -> Option<f64> {
    match (used, total) {
        (Some(u), Some(t)) if t > 0 => Some(u as f64 * 100.0 / t as f64),
        _ => None,
    }
}

/// A piece of free text for a notification: control characters and the markup characters of the notification body
/// (`<`, `>`, `&`) dropped, cut to [`PIECE_MAX`] characters; `None` when empty or secret-looking.
fn clean(text: &str) -> Option<String> {
    let kept: String = text
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '<' | '>' | '&'))
        .take(PIECE_MAX)
        .collect();
    let kept = kept.trim().to_string();
    (!kept.is_empty() && !is_secret(kept.as_bytes()) && !is_secret(text.as_bytes())).then_some(kept)
}

/// A tool name or session id as the live tail labels it: `^[A-Za-z0-9][A-Za-z0-9:._-]{0,63}$` and not secret-looking.
fn safe(text: &str) -> Option<String> {
    let ok = text
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_alphanumeric())
        && text.len() <= PIECE_MAX
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b":._-".contains(&b));
    (ok && !is_secret(text.as_bytes())).then(|| text.to_string())
}

/// The first [`SHORT_ID`] characters of a session id that is a safe label; `None` otherwise.
fn short_id(id: &str) -> Option<String> {
    safe(id).map(|s| s.chars().take(SHORT_ID).collect())
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

impl Engine {
    pub(crate) fn new(shared: Arc<Shared>, notifier: Box<dyn Notifier>) -> Engine {
        Engine {
            shared,
            notifier,
            tripped: HashSet::new(),
            sessions_on: HashSet::new(),
            waiting: Vec::new(),
            last_popup: None,
        }
    }

    /// One look. `now` is seconds since the epoch (shown in the list); `at` is monotonic (spaces the popups). `live` is a
    /// snapshot of the live tail; a missing input leaves its alerts as they are.
    pub(crate) fn tick(
        &mut self,
        now: i64,
        at: Instant,
        live: Option<&Value>,
        machine: Option<&Sample>,
        limits: Option<&Round>,
    ) {
        let mut raised = Vec::new();
        if let Some(live) = live {
            self.sessions(now, live, &mut raised);
        }
        if let Some(sample) = machine {
            self.machine(now, sample, &mut raised);
        }
        if let Some(rows) = limits.and_then(|r| r.rows.as_ref()) {
            self.limits(now, rows, &mut raised);
        }
        let paused = self.shared.paused();
        for alert in raised {
            if !paused {
                self.waiting.push(alert.clone());
            }
            self.shared.push(alert);
        }
        if paused {
            self.waiting.clear();
        } else if !self.waiting.is_empty()
            && self
                .last_popup
                .is_none_or(|last| at.saturating_duration_since(last) >= POPUP_GAP)
        {
            let (summary, body) = popup(&self.waiting);
            self.waiting.clear();
            self.last_popup = Some(at);
            self.notifier.notify(summary, &body);
        }
    }

    fn sessions(&mut self, now: i64, live: &Value, raised: &mut Vec<Alert>) {
        let mut on = HashSet::new();
        let mut found = Vec::new();
        for s in live["sessions"].as_array().into_iter().flatten() {
            // the full id keys the state; only a safe short form is ever shown
            let Some(full) = s["session_id"].as_str() else {
                continue;
            };
            let id = short_id(full);
            let project = s["project"].as_str().and_then(clean);
            for a in s["attention"].as_array().into_iter().flatten() {
                let (state, kind, head) = match a["state"].as_str() {
                    Some("waiting") => ("waiting", "session_waiting", "Waiting for you"),
                    Some("failing") => ("failing", "session_failing", "Failing"),
                    _ => continue,
                };
                let key = (full.to_string(), state);
                if !self.sessions_on.contains(&key) && !on.contains(&key) {
                    let mut parts = vec![head.to_string()];
                    parts.extend(a["tool"].as_str().and_then(safe));
                    parts.extend(project.as_ref().map(|p| format!("project {p}")));
                    parts.extend(id.as_ref().map(|id| format!("session {id}")));
                    found.push(Alert {
                        t: now,
                        kind,
                        text: parts.join(", "),
                        account: None,
                        session: id.clone(),
                    });
                }
                on.insert(key);
            }
        }
        self.sessions_on = on;
        raised.extend(found);
    }

    fn machine(&mut self, now: i64, s: &Sample, raised: &mut Vec<Alert>) {
        let mut raise = |kind, text: String| {
            raised.push(Alert {
                t: now,
                kind,
                text,
                account: None,
                session: None,
            });
        };
        for (kind, label, value) in [
            ("disk", "Disk", pct(s.disk_used, s.disk_total)),
            ("ram", "RAM", pct(s.ram_used, s.ram_total)),
        ] {
            if let Some(v) = value
                && cross(&mut self.tripped, kind, v >= HIGH, v < REARM_HIGH)
            {
                raise(kind, format!("{label} {:.0} % used", v));
            }
        }
        if let Some(b) = s.battery_pct {
            let off_charger = s.charging == Some(false);
            let trigger = b <= LOW_BATTERY && off_charger;
            let rearm = b >= REARM_BATTERY || s.charging == Some(true);
            if cross(&mut self.tripped, "battery", trigger, rearm) {
                raise("battery", format!("Battery {b} %, not charging"));
            }
        }
    }

    fn limits(
        &mut self,
        now: i64,
        rows: &[easy_personal_core::accounts::LimitPercents],
        raised: &mut Vec<Alert>,
    ) {
        for row in rows {
            for (window, label, value) in [
                ("5h", "5-hour", row.five_h_pct),
                ("7d", "7-day", row.seven_d_pct),
            ] {
                let Some(v) = value else { continue };
                let key = format!("limit:{}:{window}", row.id);
                if cross(&mut self.tripped, &key, v >= HIGH, v < REARM_HIGH) {
                    raised.push(Alert {
                        t: now,
                        kind: "plan_limit",
                        text: format!("Plan limit {label} at {v:.0} % (account {})", row.id),
                        account: Some(row.id),
                        session: None,
                    });
                }
            }
        }
    }
}

/// The popup for what waits: the text of one alert, or a count per group for several.
fn popup(waiting: &[Alert]) -> (&'static str, String) {
    const TITLE: &str = "Easy Personal";
    if let [one] = waiting {
        return (TITLE, one.text.clone());
    }
    let mut counts: BTreeMap<u8, usize> = BTreeMap::new();
    for a in waiting {
        let group = match a.kind {
            "session_waiting" => 0,
            "session_failing" => 1,
            "plan_limit" => 2,
            _ => 3,
        };
        *counts.entry(group).or_default() += 1;
    }
    let parts: Vec<String> = counts
        .into_iter()
        .map(|(group, n)| match group {
            0 => format!("{} waiting", plural(n, "session", "sessions")),
            1 => format!("{} failing", plural(n, "session", "sessions")),
            2 => plural(n, "plan limit alert", "plan limit alerts"),
            _ => plural(n, "machine alert", "machine alerts"),
        })
        .collect();
    (TITLE, parts.join(", "))
}

/// The real notifier: `org.freedesktop.Notifications.Notify` on the session D-Bus. A failure (no session bus, no
/// notification service, a service that does not answer) costs one log line, the first time; later ones are silent.
pub(crate) struct DBusNotifier {
    warned: bool,
    /// A bus address to use instead of the session bus (a test points it at a socket that is not there).
    address: Option<String>,
}

impl DBusNotifier {
    pub(crate) fn new() -> DBusNotifier {
        DBusNotifier {
            warned: false,
            address: None,
        }
    }
}

/// How long one `Notify` call may take (it also bounds how long `Running::stop` can wait for this thread).
const NOTIFY_WAIT: Duration = Duration::from_secs(3);

fn send(address: Option<&str>, summary: &str, body: &str) -> Result<(), String> {
    use std::collections::HashMap;
    use zbus::zvariant::Value as Variant;
    off_runtime(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "no runtime".to_string())?;
        runtime.block_on(async {
            let call = async {
                let connection = match address {
                    Some(address) => zbus::connection::Builder::address(address)?.build().await?,
                    None => zbus::Connection::session().await?,
                };
                connection
                    .call_method(
                        Some("org.freedesktop.Notifications"),
                        "/org/freedesktop/Notifications",
                        Some("org.freedesktop.Notifications"),
                        "Notify",
                        // app name, replaces id, icon, summary, body, actions, hints, expire (ms)
                        &(
                            "easy-personal",
                            0u32,
                            "",
                            summary,
                            body,
                            Vec::<&str>::new(),
                            HashMap::<&str, Variant>::new(),
                            10_000i32,
                        ),
                    )
                    .await
                    .map(|_| ())
            };
            match tokio::time::timeout(NOTIFY_WAIT, call).await {
                Ok(Ok(())) => Ok(()),
                Ok(Err(_)) => Err("the notification service did not take it".to_string()),
                Err(_) => Err("the notification service did not answer".to_string()),
            }
        })
    })
}

impl Notifier for DBusNotifier {
    fn notify(&mut self, summary: &str, body: &str) {
        if let Err(why) = send(self.address.as_deref(), summary, body)
            && !self.warned
        {
            self.warned = true;
            // The text is fixed: it names no bus address, path or error detail.
            eprintln!(
                "easy-personal-rs: desktop notifications are not available ({why}); alerts stay in GET /api/alerts"
            );
        }
    }
}

/// Starts the alert thread (`easy-personal-alerts`): the first look is taken now, then one every [`TICK`].
pub(crate) fn start(
    shared: Arc<Shared>,
    notifier: Box<dyn Notifier>,
    tail: Option<crate::live_feed::SharedTail>,
    machine: Option<History<Sample>>,
    limits: Option<History<Round>>,
) -> Result<Sampler<()>, String> {
    start_every(TICK, shared, notifier, tail, machine, limits)
}

/// [`start`] with the interval given, for a test.
pub(crate) fn start_every(
    interval: Duration,
    shared: Arc<Shared>,
    notifier: Box<dyn Notifier>,
    tail: Option<crate::live_feed::SharedTail>,
    machine: Option<History<Sample>>,
    limits: Option<History<Round>>,
) -> Result<Sampler<()>, String> {
    let mut engine = Engine::new(shared, notifier);
    Sampler::start("easy-personal-alerts", 1, interval, move || {
        let now = now_micros();
        let live = tail.as_ref().map(|t| {
            t.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .snapshot(now)
        });
        let sample = machine.as_ref().and_then(|h| {
            h.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .latest()
                .cloned()
        });
        let round = limits.as_ref().and_then(|h| {
            h.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .latest()
                .cloned()
        });
        engine.tick(
            now / 1_000_000,
            Instant::now(),
            live.as_ref(),
            sample.as_ref(),
            round.as_ref(),
        );
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use easy_personal_core::accounts::LimitPercents;

    const T0: i64 = 1_800_000_000;

    /// Records every popup; never touches D-Bus.
    #[derive(Clone, Default)]
    struct Fake(Arc<Mutex<Vec<(String, String)>>>);
    impl Notifier for Fake {
        fn notify(&mut self, summary: &str, body: &str) {
            self.0.lock().unwrap().push((summary.into(), body.into()));
        }
    }
    impl Fake {
        fn bodies(&self) -> Vec<String> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .map(|(_, b)| b.clone())
                .collect()
        }
    }

    struct Rig {
        engine: Engine,
        shared: Arc<Shared>,
        fake: Fake,
        base: Instant,
        secs: u64,
    }

    impl Rig {
        fn new() -> Rig {
            let shared = Arc::new(Shared::default());
            let fake = Fake::default();
            Rig {
                engine: Engine::new(Arc::clone(&shared), Box::new(fake.clone())),
                shared,
                fake,
                base: Instant::now(),
                // far enough from the start that the first popup is never held back
                secs: 0,
            }
        }
        /// One look, `gap` seconds of monotonic time after the last one.
        fn look(&mut self, gap: u64, live: Option<&Value>, m: Option<&Sample>, l: Option<&Round>) {
            self.secs += gap;
            self.engine.tick(
                T0 + self.secs as i64,
                self.base + Duration::from_secs(self.secs),
                live,
                m,
                l,
            );
        }
        fn kinds(&self) -> Vec<&'static str> {
            self.shared
                .list
                .lock()
                .unwrap()
                .iter()
                .map(|a| a.kind)
                .collect()
        }
        fn count(&self) -> usize {
            self.shared.list.lock().unwrap().len()
        }
    }

    fn sample() -> Sample {
        Sample {
            t: T0,
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
    fn disk(p: u64) -> Sample {
        Sample {
            disk_used: Some(p),
            disk_total: Some(100),
            ..sample()
        }
    }
    fn ram(p: u64) -> Sample {
        Sample {
            ram_used: Some(p * 10),
            ram_total: Some(1000),
            ..sample()
        }
    }
    fn battery(p: u8, charging: Option<bool>) -> Sample {
        Sample {
            battery_pct: Some(p),
            charging,
            ..sample()
        }
    }
    fn limit(five: Option<f64>, seven: Option<f64>) -> Round {
        limits_of(&[(1, five, seven)])
    }
    fn limits_of(rows: &[(i64, Option<f64>, Option<f64>)]) -> Round {
        Round {
            t: T0,
            rows: Some(
                rows.iter()
                    .map(|&(id, five_h_pct, seven_d_pct)| LimitPercents {
                        id,
                        label: format!("secret-label-{id}"),
                        five_h_pct,
                        seven_d_pct,
                    })
                    .collect(),
            ),
        }
    }
    fn live(sessions: Value) -> Value {
        json!({ "sessions": sessions })
    }
    fn session(id: &str, project: &str, state: &str, tool: &str) -> Value {
        json!({ "session_id": id, "project": project, "account": 1,
                "attention": [{ "state": state, "tool": tool, "count": 1, "is_sidechain": false, "since": "x" }] })
    }

    /// AC 1: 89 -> 91 -> 90 -> 86 -> 91 raises one alert (86 is not below 85, so it is not re-armed); 84 -> 91 re-arms
    /// and raises a second. The same for disk, RAM and each limit window.
    fn flap(feed: impl Fn(u64) -> (Option<Sample>, Option<Round>), kind: &str) {
        let mut rig = Rig::new();
        for v in [89, 91, 90, 86, 91] {
            let (m, l) = feed(v);
            rig.look(1, None, m.as_ref(), l.as_ref());
        }
        assert_eq!(rig.count(), 1, "{kind}: one alert for the flap");
        for v in [84, 91] {
            let (m, l) = feed(v);
            rig.look(1, None, m.as_ref(), l.as_ref());
        }
        assert_eq!(rig.count(), 2, "{kind}: 84 re-arms, 91 raises again");
        assert!(rig.kinds().iter().all(|k| *k == kind));
        // exactly 85 is not below 85
        let (m, l) = feed(85);
        rig.look(1, None, m.as_ref(), l.as_ref());
        let (m, l) = feed(91);
        rig.look(1, None, m.as_ref(), l.as_ref());
        assert_eq!(rig.count(), 2, "{kind}: 85 does not re-arm");
    }

    #[test]
    fn disk_hysteresis() {
        flap(|v| (Some(disk(v)), None), "disk");
    }

    #[test]
    fn ram_hysteresis() {
        flap(|v| (Some(ram(v)), None), "ram");
    }

    #[test]
    fn plan_limit_hysteresis_per_window() {
        flap(|v| (None, Some(limit(Some(v as f64), None))), "plan_limit");
        flap(|v| (None, Some(limit(None, Some(v as f64)))), "plan_limit");
    }

    #[test]
    fn a_limit_is_per_account_and_window() {
        let mut rig = Rig::new();
        rig.look(
            1,
            None,
            None,
            Some(&limits_of(&[
                (1, Some(95.0), Some(10.0)),
                (2, Some(10.0), Some(95.0)),
            ])),
        );
        assert_eq!(rig.count(), 2);
        let alerts = rig.shared.json();
        let texts: Vec<&str> = alerts["alerts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["text"].as_str().unwrap())
            .collect();
        assert!(
            texts.contains(&"Plan limit 5-hour at 95 % (account 1)"),
            "{texts:?}"
        );
        assert!(
            texts.contains(&"Plan limit 7-day at 95 % (account 2)"),
            "{texts:?}"
        );
        // still high: nothing new; account 1 five-hour back under and over again: one more
        rig.look(
            1,
            None,
            None,
            Some(&limits_of(&[
                (1, Some(95.0), Some(10.0)),
                (2, Some(10.0), Some(95.0)),
            ])),
        );
        assert_eq!(rig.count(), 2);
        rig.look(
            1,
            None,
            None,
            Some(&limits_of(&[
                (1, Some(50.0), Some(10.0)),
                (2, Some(10.0), Some(95.0)),
            ])),
        );
        rig.look(
            1,
            None,
            None,
            Some(&limits_of(&[
                (1, Some(92.0), Some(10.0)),
                (2, Some(10.0), Some(95.0)),
            ])),
        );
        assert_eq!(rig.count(), 3);
        assert_eq!(alerts["alerts"][0]["account"], 2, "the newest is first");
    }

    #[test]
    fn no_data_raises_nothing_and_re_arms_nothing() {
        let mut rig = Rig::new();
        rig.look(1, None, Some(&sample()), Some(&Round { t: T0, rows: None }));
        rig.look(1, None, None, Some(&limit(None, None)));
        assert_eq!(rig.count(), 0, "a missing measure is not 0 and not high");
        // a tripped limit stays tripped through a stale (null) reading
        rig.look(1, None, None, Some(&limit(Some(95.0), None)));
        rig.look(1, None, None, Some(&limit(None, None)));
        rig.look(1, None, None, Some(&limit(Some(95.0), None)));
        assert_eq!(rig.count(), 1);
    }

    #[test]
    fn battery_alerts_at_15_off_the_charger_and_re_arms_at_20_or_charging() {
        let mut rig = Rig::new();
        for (p, c) in [
            (16, Some(false)),
            (15, None),
            (15, Some(false)),
            (14, Some(false)),
            (18, Some(false)),
            (15, Some(false)),
        ] {
            rig.look(1, None, Some(&battery(p, c)), None);
        }
        assert_eq!(
            rig.count(),
            1,
            "unknown charging is no alert; 18 does not re-arm"
        );
        rig.look(1, None, Some(&battery(20, Some(false))), None);
        rig.look(1, None, Some(&battery(10, Some(false))), None);
        assert_eq!(rig.count(), 2, "20 re-arms");
        rig.look(1, None, Some(&battery(10, Some(true))), None);
        rig.look(1, None, Some(&battery(10, Some(false))), None);
        assert_eq!(rig.count(), 3, "charging re-arms");
        assert_eq!(
            rig.shared.json()["alerts"][0]["text"],
            "Battery 10 %, not charging"
        );
    }

    /// AC 2.
    #[test]
    fn a_session_alerts_once_per_stay_and_again_after_it_leaves() {
        let mut rig = Rig::new();
        let waiting = live(json!([session(
            "0123456789abcdef",
            "shop",
            "waiting",
            "AskUserQuestion"
        )]));
        let none = live(json!([]));
        rig.look(1, Some(&waiting), None, None);
        assert_eq!(rig.kinds(), ["session_waiting"]);
        assert_eq!(
            rig.shared.json()["alerts"][0]["text"],
            "Waiting for you, AskUserQuestion, project shop, session 01234567"
        );
        assert_eq!(rig.shared.json()["alerts"][0]["session"], "01234567");
        rig.look(1, Some(&waiting), None, None);
        rig.look(1, Some(&waiting), None, None);
        assert_eq!(rig.count(), 1, "still waiting: none");
        rig.look(1, Some(&none), None, None);
        rig.look(1, Some(&waiting), None, None);
        assert_eq!(rig.count(), 2, "left and came back: a second alert");
        // waiting -> failing is a different state: one more; other states (needs_attention) raise nothing
        let failing = live(json!([session(
            "0123456789abcdef",
            "shop",
            "failing",
            "Bash"
        )]));
        rig.look(1, Some(&failing), None, None);
        assert_eq!(
            rig.kinds(),
            ["session_waiting", "session_waiting", "session_failing"]
        );
        let other = live(json!([session(
            "0123456789abcdef",
            "shop",
            "needs_attention",
            "Bash"
        )]));
        rig.look(1, Some(&other), None, None);
        assert_eq!(rig.count(), 3);
    }

    /// AC 3.
    #[test]
    fn no_secret_or_message_text_reaches_the_popup_or_the_list() {
        let planted_msg = "please wire the invoice to my-iban-DE89370400440532013000";
        let key = "sk-ant-api03-ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut rig = Rig::new();
        let mut s = session(
            "deadbeef-1111-2222",
            "ghp_abcdefghijklmnopqrstuvwxyz0123456789",
            "waiting",
            key,
        );
        s["text"] = json!(planted_msg);
        s["label"] = json!(planted_msg);
        s["attention"][0]["input"] = json!(planted_msg);
        s["attention"][0]["message"] = json!(key);
        let mut t = session(
            "sk-live-0123456789ABCDEF0123456789ABCDEF",
            "a<b>&c\nd",
            "failing",
            "Bash",
        );
        t["text"] = json!(planted_msg);
        let snapshot = live(json!([s, t]));
        rig.look(
            1,
            Some(&snapshot),
            Some(&disk(95)),
            Some(&limit(Some(99.0), None)),
        );
        rig.look(20, Some(&live(json!([]))), Some(&disk(10)), None);
        let everything = format!("{}\n{}", rig.fake.bodies().join("\n"), rig.shared.json());
        for needle in [
            planted_msg,
            "my-iban",
            "DE8937",
            key,
            "sk-ant",
            "ghp_",
            "sk-live",
            "ABCDEFGHIJKLMNOP",
            "secret-label",
        ] {
            assert!(
                !everything.contains(needle),
                "{needle} leaked: {everything}"
            );
        }
        for body in rig.fake.bodies() {
            assert!(
                !body.contains(['<', '>', '&']) && !body.chars().any(char::is_control),
                "{body:?}"
            );
        }
        // the state is still named
        assert!(everything.contains("Waiting for you"), "{everything}");
        assert!(everything.contains("Failing"), "{everything}");
        assert!(
            everything.contains("abc"),
            "the clean project part stays: {everything}"
        );
    }

    /// AC 4 (engine half; the route half is in `api_tests.rs`).
    #[test]
    fn paused_stops_popups_never_the_list_and_drops_what_waited() {
        let mut rig = Rig::new();
        rig.shared.set_paused(true);
        rig.look(1, None, Some(&disk(95)), None);
        rig.look(1, None, Some(&ram(95)), None);
        assert_eq!(rig.count(), 2, "the list goes on");
        assert!(rig.fake.bodies().is_empty(), "no popup while paused");
        rig.shared.set_paused(false);
        rig.look(60, None, Some(&disk(95)), None);
        assert!(
            rig.fake.bodies().is_empty(),
            "what was raised while paused is not shown later"
        );
        rig.look(1, None, Some(&disk(10)), None);
        rig.look(1, None, Some(&disk(95)), None);
        assert_eq!(rig.fake.bodies(), ["Disk 95 % used"]);
        assert_eq!(rig.count(), 3);
    }

    #[test]
    fn at_most_one_popup_per_ten_seconds_and_the_rest_is_summarised() {
        let mut rig = Rig::new();
        let ids = ["aaaaaaaa", "bbbbbbbb", "cccccccc", "dddddddd"];
        let one = live(json!([session(ids[0], "p", "waiting", "AskUserQuestion")]));
        rig.look(1, Some(&one), None, None);
        assert_eq!(rig.fake.bodies().len(), 1, "the first popup at once");
        let many: Vec<Value> = ids
            .iter()
            .map(|i| session(i, "p", "waiting", "AskUserQuestion"))
            .collect();
        rig.look(3, Some(&live(json!(many))), None, None);
        rig.look(3, Some(&live(json!(many))), None, None);
        assert_eq!(rig.fake.bodies().len(), 1, "inside 10 s: held");
        assert_eq!(rig.count(), 4, "but listed");
        rig.look(4, Some(&live(json!(many))), None, None);
        assert_eq!(rig.fake.bodies().len(), 2);
        assert_eq!(rig.fake.bodies()[1], "3 sessions waiting");
        rig.look(30, Some(&live(json!(many))), None, None);
        assert_eq!(rig.fake.bodies().len(), 2, "nothing new, nothing shown");
    }

    #[test]
    fn a_mix_is_counted_per_group() {
        let (title, body) = popup(&[
            alert("session_waiting"),
            alert("session_waiting"),
            alert("session_failing"),
            alert("disk"),
            alert("battery"),
            alert("plan_limit"),
        ]);
        assert_eq!(title, "Easy Personal");
        assert_eq!(
            body,
            "2 sessions waiting, 1 session failing, 1 plan limit alert, 2 machine alerts"
        );
    }

    fn alert(kind: &'static str) -> Alert {
        Alert {
            t: 0,
            kind,
            text: "x".into(),
            account: None,
            session: None,
        }
    }

    /// The throttle runs on monotonic time: a wall clock that jumps back and forth makes no burst and no stall.
    #[test]
    fn a_wall_clock_jump_changes_nothing_about_the_popup_gap() {
        let shared = Arc::new(Shared::default());
        let fake = Fake::default();
        let mut engine = Engine::new(Arc::clone(&shared), Box::new(fake.clone()));
        let base = Instant::now();
        let at = |s: u64| base + Duration::from_secs(s);
        engine.tick(T0, at(0), None, Some(&disk(95)), None);
        engine.tick(T0 - 86_400, at(1), None, Some(&ram(95)), None); // back a day
        engine.tick(
            T0 + 86_400 * 30,
            at(2),
            None,
            Some(&battery(5, Some(false))),
            None,
        ); // a month ahead
        assert_eq!(fake.bodies().len(), 1);
        engine.tick(T0, at(11), None, None, None);
        assert_eq!(fake.bodies().len(), 2);
        assert_eq!(fake.bodies()[1], "2 machine alerts");
    }

    #[test]
    fn the_list_keeps_the_last_fifty_newest_first() {
        let shared = Shared::default();
        for i in 0..60 {
            shared.push(Alert {
                t: i,
                ..alert("disk")
            });
        }
        let json = shared.json();
        let list = json["alerts"].as_array().unwrap();
        assert_eq!(list.len(), KEEP);
        assert_eq!(list[0]["t"], 59);
        assert_eq!(list[49]["t"], 10);
        assert_eq!(json["paused"], false);
    }

    /// AC 5, the soft half: a bus that is not there costs no panic and one log line (`warned`), not one per popup.
    #[test]
    fn no_dbus_does_not_panic_and_warns_once() {
        let mut n = DBusNotifier::new();
        n.address = Some("unix:path=/nonexistent-easy-personal-test/bus".into());
        assert!(!n.warned);
        n.notify("Easy Personal", "one");
        assert!(n.warned, "the first failure is logged");
        n.notify("Easy Personal", "two"); // no panic, no second line (nothing to assert on stderr; `warned` guards it)
        assert!(n.warned);
    }

    /// The whole path from a transcript: a call to `AskUserQuestion` with no result makes the live tail say *waiting*, and
    /// the engine thread, reading that tail, raises the alert with the tool, the project folder and the short session id.
    #[test]
    fn a_waiting_session_in_a_real_live_tail_raises_one_alert() {
        use crate::live_feed::LiveFeed;
        use easy_personal_core::live::Root;
        use easy_personal_core::prices::PriceTable;
        let dir =
            std::env::temp_dir().join(format!("easy-personal-rs-t074-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("p")).unwrap();
        let roots = vec![Root {
            account: None,
            projects: dir.clone(),
        }];
        let feed = LiveFeed::start(
            PriceTable::shipped().unwrap(),
            now_micros,
            move |tail, now| tail.poll_once(&roots, now, None),
        )
        .unwrap();
        std::thread::sleep(Duration::from_millis(400)); // the first scan is over: what comes next is new
        let (days, secs) = (
            now_micros() / 1_000_000 / 86_400,
            now_micros() / 1_000_000 % 86_400,
        );
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let (day, month) = (
            doy - (153 * mp + 2) / 5 + 1,
            if mp < 10 { mp + 3 } else { mp - 9 },
        );
        let year = yoe + era * 400 + i64::from(month <= 2);
        let ts = format!(
            "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
            secs / 3600,
            secs / 60 % 60,
            secs % 60
        );
        let line = format!(
            "{{\"type\":\"assistant\",\"sessionId\":\"0123abcd-ffff-4000-8000-000000000001\",\"cwd\":\"/home/x/shop-front\",\"timestamp\":\"{ts}\",\"message\":{{\"id\":\"m1\",\"model\":\"x\",\"content\":[{{\"type\":\"text\",\"text\":\"my password is hunter2-planted\"}},{{\"type\":\"tool_use\",\"id\":\"b1\",\"name\":\"AskUserQuestion\",\"input\":{{\"question\":\"planted question text\"}}}}]}}}}\n"
        );
        std::fs::write(dir.join("p/s.jsonl"), line).unwrap();
        let fake = Fake::default();
        let end = Instant::now() + Duration::from_secs(10);
        let thread = loop {
            // a fresh engine thread each round: its first look is taken at once, so it is a poll with no sleep
            let probe = Arc::new(Shared::default());
            let t = start_every(
                Duration::from_secs(3600),
                Arc::clone(&probe),
                Box::new(fake.clone()),
                Some(feed.tail()),
                None,
                None,
            )
            .unwrap();
            if probe.list.lock().unwrap().len() == 1 {
                break (t, probe);
            }
            t.stop();
            assert!(Instant::now() < end, "the live tail never said waiting");
            std::thread::sleep(Duration::from_millis(100));
        };
        let text = thread.1.json()["alerts"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(
            text,
            "Waiting for you, AskUserQuestion, project shop-front, session 0123abcd"
        );
        let all = format!("{} {}", thread.1.json(), fake.bodies().join(" "));
        assert!(
            !all.contains("hunter2") && !all.contains("planted"),
            "{all}"
        );
        thread.0.stop();
        feed.stop();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The engine on its own thread through the sampler: it looks at once, reads the shared inputs, and stops quickly.
    #[test]
    fn the_thread_looks_at_the_inputs_and_stops_quickly() {
        let machine: History<Sample> =
            Arc::new(Mutex::new(easy_personal_core::machine::Ring::new(4)));
        machine.lock().unwrap().push(disk(97));
        let shared = Arc::new(Shared::default());
        let fake = Fake::default();
        let thread = start_every(
            Duration::from_secs(3600),
            Arc::clone(&shared),
            Box::new(fake.clone()),
            None,
            Some(Arc::clone(&machine)),
            None,
        )
        .unwrap();
        assert_eq!(shared.list.lock().unwrap().len(), 1);
        assert_eq!(fake.bodies(), ["Disk 97 % used"]);
        let began = Instant::now();
        thread.stop();
        assert!(began.elapsed() < Duration::from_secs(2));
    }
}
