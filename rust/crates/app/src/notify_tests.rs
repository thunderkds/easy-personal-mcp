//! T096 (FR-107): the popup action. A fake `org.freedesktop.Notifications` service on a private bus (a `dbus-daemon` of its
//! own with a config that allows only this bus; it never touches the owner's session bus or desktop) checks that a popup
//! carries the default action only when the service lists `actions`, that a click on a popup this process sent is passed
//! on with its session, and that every other signal is ignored. A machine without `dbus-daemon` skips the bus tests
//! (they say so); the filter itself ([`Sent`]) is tested with no bus.

use super::*;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command as Process, Stdio};
use std::sync::atomic::AtomicU32;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use zbus::zvariant::OwnedValue;

/// A private message bus: `dbus-daemon` with a config that listens on one socket in a temp folder and allows everything
/// on it. Killed, with the folder, when dropped.
struct Bus {
    child: Child,
    dir: PathBuf,
    address: String,
}

impl Bus {
    fn start() -> Option<Bus> {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ep-t096-bus-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).ok()?;
        let config = dir.join("bus.conf");
        std::fs::write(
            &config,
            format!(
                "<!DOCTYPE busconfig PUBLIC \"-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN\" \
                 \"http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd\">\n\
                 <busconfig><type>session</type><listen>unix:path={}</listen><auth>EXTERNAL</auth>\
                 <policy context=\"default\"><allow send_destination=\"*\" eavesdrop=\"true\"/>\
                 <allow eavesdrop=\"true\"/><allow own=\"*\"/></policy></busconfig>\n",
                dir.join("bus").display()
            ),
        )
        .ok()?;
        let Ok(mut child) = Process::new("dbus-daemon")
            .arg("--config-file")
            .arg(&config)
            .arg("--nofork")
            .arg("--print-address")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            eprintln!("skipped: no dbus-daemon on this machine");
            let _ = std::fs::remove_dir_all(&dir);
            return None;
        };
        let mut address = String::new();
        BufReader::new(child.stdout.take()?)
            .read_line(&mut address)
            .ok()?;
        Some(Bus {
            child,
            dir,
            address: address.trim().to_string(),
        })
    }
}

impl Drop for Bus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// What the fake service was asked: (summary, body, actions).
type Seen = Arc<Mutex<Vec<(String, String, Vec<String>)>>>;

/// The app icon and the `desktop-entry` hint of each `Notify` (T097), beside what [`Seen`] holds.
type Looks = Arc<Mutex<Vec<(String, Option<String>)>>>;

struct Service {
    capabilities: Vec<String>,
    seen: Seen,
    looks: Looks,
    /// Answer `Notify` only after this long.
    delay: Duration,
    next: AtomicU32,
}

#[zbus::interface(name = "org.freedesktop.Notifications")]
impl Service {
    fn get_capabilities(&self) -> Vec<String> {
        self.capabilities.clone()
    }

    #[allow(clippy::too_many_arguments)]
    async fn notify(
        &self,
        _app: String,
        _replaces: u32,
        icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        _expire: i32,
    ) -> u32 {
        let entry = match hints.get("desktop-entry").map(|v| &**v) {
            Some(zbus::zvariant::Value::Str(s)) => Some(s.to_string()),
            _ => None,
        };
        self.looks.lock().unwrap().push((icon, entry));
        self.seen.lock().unwrap().push((summary, body, actions));
        tokio::time::sleep(self.delay).await;
        self.next.fetch_add(1, Ordering::SeqCst) + 1
    }
}

enum Say {
    /// `ActionInvoked(id, key)` from the service's own connection.
    Action(u32, &'static str),
    Closed(u32),
    /// `ActionInvoked(id, key)` from a second connection on the bus (not the service).
    Spoof(u32, &'static str),
    /// The service goes away.
    Quit,
}

/// The fake notification service on `bus`, on a thread of its own.
struct Fake {
    seen: Seen,
    looks: Looks,
    say: tokio::sync::mpsc::UnboundedSender<Say>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Fake {
    fn start(bus: &Bus, capabilities: &[&str], delay: Duration) -> Fake {
        let seen = Seen::default();
        let looks = Looks::default();
        let service = Service {
            capabilities: capabilities.iter().map(|c| c.to_string()).collect(),
            seen: Arc::clone(&seen),
            looks: Arc::clone(&looks),
            delay,
            next: AtomicU32::new(40),
        };
        let (say, mut heard) = tokio::sync::mpsc::unbounded_channel::<Say>();
        let (ready, up) = channel();
        let address = bus.address.clone();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let connection = zbus::connection::Builder::address(address.as_str())
                    .unwrap()
                    .name(SERVICE)
                    .unwrap()
                    .serve_at(OBJECT, service)
                    .unwrap()
                    .build()
                    .await
                    .unwrap();
                let other = zbus::connection::Builder::address(address.as_str())
                    .unwrap()
                    .build()
                    .await
                    .unwrap();
                ready.send(()).unwrap();
                while let Some(line) = heard.recv().await {
                    match line {
                        Say::Action(id, key) => {
                            let _ = connection
                                .emit_signal(
                                    None::<&str>,
                                    OBJECT,
                                    SERVICE,
                                    "ActionInvoked",
                                    &(id, key),
                                )
                                .await;
                        }
                        Say::Closed(id) => {
                            let _ = connection
                                .emit_signal(
                                    None::<&str>,
                                    OBJECT,
                                    SERVICE,
                                    "NotificationClosed",
                                    &(id, 2u32),
                                )
                                .await;
                        }
                        Say::Spoof(id, key) => {
                            let _ = other
                                .emit_signal(
                                    None::<&str>,
                                    OBJECT,
                                    SERVICE,
                                    "ActionInvoked",
                                    &(id, key),
                                )
                                .await;
                        }
                        Say::Quit => break,
                    }
                }
            });
        });
        up.recv_timeout(Duration::from_secs(10))
            .expect("the fake service did not start");
        Fake {
            seen,
            looks,
            say,
            thread: Some(thread),
        }
    }

    fn say(&self, what: Say) {
        self.say.send(what).unwrap();
    }

    fn actions(&self) -> Vec<Vec<String>> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|s| s.2.clone())
            .collect()
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        let _ = self.say.send(Say::Quit);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn notifier(bus: &Bus, clicks: Option<Clicks>) -> DBusNotifier {
    let mut n = DBusNotifier::new();
    n.address = Some(bus.address.clone());
    n.clicks = clicks;
    n
}

/// Every click that arrives within `wait`.
fn clicks_within(rx: &Receiver<Option<String>>, wait: Duration) -> Vec<Option<String>> {
    let end = Instant::now() + wait;
    let mut got = Vec::new();
    loop {
        match rx.recv_timeout(end.saturating_duration_since(Instant::now())) {
            Ok(click) => got.push(click),
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return got,
        }
    }
}

const WIRE: &[&str] = &["body", "actions"];

/// AC 7, the filter alone (no bus): a click is a default action for an id this process sent, from the service that
/// answered; it is forgotten when the popup is closed, and only the last [`KEEP`] are kept.
#[test]
fn a_click_is_only_for_a_popup_this_process_sent() {
    let mut sent = Sent::default();
    sent.add(7, ":1.5", Some("abc-123".into()));
    sent.add(8, ":1.5", None);
    assert_eq!(
        sent.clicked(7, ":1.5", "default"),
        Some(Some("abc-123".into()))
    );
    assert_eq!(
        sent.clicked(8, ":1.5", "default"),
        Some(None),
        "a summed popup"
    );
    assert_eq!(sent.clicked(9, ":1.5", "default"), None, "an id not sent");
    assert_eq!(sent.clicked(7, ":1.6", "default"), None, "another sender");
    assert_eq!(sent.clicked(7, "", "default"), None, "no sender");
    for key in ["", "Default", "open", "default ", "other"] {
        assert_eq!(sent.clicked(7, ":1.5", key), None, "{key:?}");
    }
    sent.closed(7, ":1.6");
    assert!(
        sent.clicked(7, ":1.5", "default").is_some(),
        "another sender's close"
    );
    sent.closed(7, ":1.5");
    assert_eq!(sent.clicked(7, ":1.5", "default"), None, "closed");
    assert!(sent.clicked(8, ":1.5", "default").is_some());
    // the last KEEP only; an id the service reuses is the newer popup
    for id in 100..100 + KEEP as u32 {
        sent.add(id, ":1.5", None);
    }
    assert_eq!(sent.0.len(), KEEP);
    assert_eq!(
        sent.clicked(8, ":1.5", "default"),
        None,
        "the oldest fell out"
    );
    assert!(
        sent.clicked(100 + KEEP as u32 - 1, ":1.5", "default")
            .is_some()
    );
    sent.add(120, ":1.5", Some("new".into()));
    assert_eq!(
        sent.clicked(120, ":1.5", "default"),
        Some(Some("new".into()))
    );
    assert_eq!(sent.0.len(), KEEP);
}

/// AC 7: against the fake service, a popup carries `["default", "Open"]`; a click on it reaches `clicks` with its session;
/// a click on a summed popup reaches it with none; an unknown id, another key, another sender and a closed popup do nothing.
#[test]
fn a_popup_has_the_default_action_and_its_click_is_passed_on() {
    let Some(bus) = Bus::start() else { return };
    let fake = Fake::start(&bus, WIRE, Duration::ZERO);
    let (tx, rx) = channel();
    let mut n = notifier(&bus, Some(tx));
    n.notify(
        "Easy Personal",
        "Your turn, project shop, session 01234567",
        Some("0123abcd-77"),
    );
    n.notify("Easy Personal", "3 sessions waiting", None);
    n.notify("Easy Personal", "gone soon", Some("dead-beef"));
    assert!(!n.warned);
    assert_eq!(
        fake.actions(),
        vec![vec!["default".to_string(), "Open".to_string()]; 3],
        "one default action on every popup"
    );
    // the service numbers its answers 41, 42, 43
    fake.say(Say::Action(99, "default")); // an id this process did not send
    fake.say(Say::Action(41, "other")); // not the default action
    fake.say(Say::Spoof(41, "default")); // not the service
    fake.say(Say::Action(41, "default"));
    fake.say(Say::Action(42, "default"));
    fake.say(Say::Closed(43));
    fake.say(Say::Action(43, "default")); // closed
    assert_eq!(
        clicks_within(&rx, Duration::from_millis(800)),
        [Some("0123abcd-77".to_string()), None]
    );
    assert!(!n.warned);
}

/// T096 P1 fix round: a popup that carries the default action also carries T097's app icon and `desktop-entry` hint.
#[test]
fn a_popup_with_the_action_also_names_the_app_icon_and_the_desktop_entry() {
    let Some(bus) = Bus::start() else { return };
    let fake = Fake::start(&bus, WIRE, Duration::ZERO);
    let (tx, _rx) = channel();
    let mut n = notifier(&bus, Some(tx));
    n.notify("Easy Personal", "3 sessions waiting", None);
    assert!(!n.warned);
    assert_eq!(
        fake.actions(),
        vec![vec!["default".to_string(), "Open".to_string()]]
    );
    assert_eq!(
        *fake.looks.lock().unwrap(),
        [(
            "easy-personal".to_string(),
            Some("easy-personal".to_string())
        )]
    );
}

/// AC 7: a service without the capability `actions` gets the popup with no action, and a click is never listened for.
#[test]
fn a_service_without_actions_gets_a_popup_with_none() {
    let Some(bus) = Bus::start() else { return };
    let fake = Fake::start(&bus, &["body", "persistence"], Duration::ZERO);
    let (tx, rx) = channel();
    let mut n = notifier(&bus, Some(tx));
    n.notify("Easy Personal", "x", Some("abc"));
    assert_eq!(fake.actions(), [Vec::<String>::new()]);
    fake.say(Say::Action(41, "default"));
    assert!(clicks_within(&rx, Duration::from_millis(400)).is_empty());
    assert!(!n.warned);
}

/// AC 7: with no clicks given (`serve`, `app`), no action is offered even when the service lists `actions`.
#[test]
fn without_a_click_channel_no_action_is_offered() {
    let Some(bus) = Bus::start() else { return };
    let fake = Fake::start(&bus, WIRE, Duration::ZERO);
    let mut n = notifier(&bus, None);
    n.notify("Easy Personal", "x", Some("abc"));
    n.notify("Easy Personal", "y", None);
    assert_eq!(fake.actions(), vec![Vec::<String>::new(); 2]);
    assert!(!n.warned);
}

/// AC 8: a service that never answers costs the 3 s bound and one log line, not a hang or a panic; the next popup works
/// once the service answers.
#[test]
fn a_service_that_never_answers_is_bounded() {
    let Some(bus) = Bus::start() else { return };
    let _fake = Fake::start(&bus, WIRE, Duration::from_secs(30));
    let (tx, _rx) = channel();
    let mut n = notifier(&bus, Some(tx));
    let began = Instant::now();
    n.notify("Easy Personal", "x", Some("abc"));
    let took = began.elapsed();
    assert!(took >= Duration::from_secs(2), "{took:?}");
    assert!(took < Duration::from_secs(5), "{took:?}");
    assert!(n.warned);
}

/// AC 8: a service that goes away while a popup waits for its answer fails that popup at once; the notifier does not panic
/// and serves the next popup when a service is back.
#[test]
fn a_service_that_drops_mid_wait_fails_soft() {
    let Some(bus) = Bus::start() else { return };
    let fake = Fake::start(&bus, WIRE, Duration::from_secs(30));
    let (tx, rx) = channel();
    let mut n = notifier(&bus, Some(tx));
    let say = fake.say.clone();
    let quitter = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        let _ = say.send(Say::Quit);
    });
    let began = Instant::now();
    n.notify("Easy Personal", "x", Some("abc"));
    assert!(
        began.elapsed() < Duration::from_secs(3),
        "{:?}",
        began.elapsed()
    );
    assert!(n.warned);
    quitter.join().unwrap();
    drop(fake);
    let again = Fake::start(&bus, WIRE, Duration::ZERO);
    n.notify("Easy Personal", "y", Some("abc-1"));
    assert_eq!(again.actions().len(), 1, "connected again");
    again.say(Say::Action(41, "default"));
    assert_eq!(
        clicks_within(&rx, Duration::from_millis(800)),
        [Some("abc-1".to_string())]
    );
}

/// AC 8: no bus at all, with a click channel: no panic, one warning, and the engine thread is not held.
#[test]
fn no_bus_with_a_click_channel_does_not_panic() {
    let (tx, _rx) = channel();
    let mut n = DBusNotifier::new().with_clicks(tx);
    n.address = Some("unix:path=/nonexistent-easy-personal-test/bus".into());
    let began = Instant::now();
    n.notify("Easy Personal", "one", Some("abc"));
    n.notify("Easy Personal", "two", None);
    assert!(n.warned);
    assert!(began.elapsed() < Duration::from_secs(5));
}

/// AC 8: the alert engine on its own thread with the real notifier against a service that never answers: the start is not
/// held, and `stop` (what `Running::stop` calls) returns within the notifier's 3 s bound plus a margin.
#[test]
fn a_hung_service_holds_neither_the_start_nor_the_stop() {
    let Some(bus) = Bus::start() else { return };
    let fake = Fake::start(&bus, WIRE, Duration::from_secs(30));
    let (tx, _rx) = channel();
    let shared = Arc::new(Shared::default());
    let began = Instant::now();
    let thread = start_every(
        Duration::from_millis(20),
        Arc::clone(&shared),
        Box::new(notifier(&bus, Some(tx))),
        None,
        Some(super::tests::hot_machine()),
        None,
    )
    .unwrap();
    assert!(
        began.elapsed() < Duration::from_millis(500),
        "{:?}",
        began.elapsed()
    );
    let end = Instant::now() + Duration::from_secs(10);
    while fake.seen.lock().unwrap().is_empty() {
        assert!(Instant::now() < end, "the popup never reached the service");
        std::thread::sleep(Duration::from_millis(10));
    }
    let stopping = Instant::now();
    thread.stop();
    assert!(
        stopping.elapsed() < Duration::from_secs(5),
        "{:?}",
        stopping.elapsed()
    );
    assert_eq!(shared.recent(5).len(), 1, "the alert stays in the list");
}
