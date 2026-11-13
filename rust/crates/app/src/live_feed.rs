//! The live feed of `serve` and `app` (T056, FR-043..FR-049): one [`LiveTail`] (T049's engine) behind a lock, and one
//! thread that calls a poll step on it every [`POLL`] with the clock of `clock.rs`. It is a port of `start`, `stop`
//! and `_run` in `src/easy_personal/core/live.py`, publish-only (the owner's Path 1): the step is
//! `LiveTail::poll_store`, which opens the store read-only for that step, never takes `ingest.lock` and never writes.
//!
//! Only the two serving commands make a feed (`server::start` is given one); `store-info`, `mcp` and `ingest` start no
//! thread and open no store or file here. A step that fails (no store, a store that is not v11, an unreadable root)
//! is the engine's own skip; a step that panics is caught, skipped, and the thread goes on. A panic of this thread
//! prints one fixed line, once per process, never the panic's text (it could name a path).
//!
//! [`LiveStream`] turns the tail into the Server-Sent Events of Python's `live_stream` (`surfaces/api.py`): the same
//! messages in the same order, the JSON keys of `_SESSION_KEYS`, `_EVENT_KEYS` and `_snapshot_data` and nothing else.

use crate::api::Clock;
use axum::body::Bytes;
use easy_personal_core::clock::now_micros;
use easy_personal_core::live::{ACTIVE_WINDOW_S, LiveTail, OPEN_CALL_MINUTES};
use easy_personal_core::prices::PriceTable;
use easy_personal_core::store::store_path;
use futures_core::Stream;
use serde_json::{Map, Value, json};
use std::convert::Infallible;
use std::future::Future;
use std::panic::{self, AssertUnwindSafe};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, Once, PoisonError, TryLockError};
use std::task::{Context, Poll};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tokio::time::Sleep;

/// The one route that streams; also the one route that takes the token from the query (`server.rs`).
pub(crate) const LIVE_PATH: &str = "/api/live";
/// Python's `POLL_SECONDS` (the thread) and `POLL_S` (each stream).
pub(crate) const POLL: Duration = Duration::from_millis(100);
/// Python's `PING_S`: a `: ping` comment after this long without one.
pub(crate) const PING: Duration = Duration::from_secs(15);
/// Python's `stop()` joins with `timeout=10`.
const STOP_WAIT: Duration = Duration::from_secs(10);
/// Python's `THREAD_NAME`.
const THREAD_NAME: &str = "easy-personal-live-tail";

/// Python's `_SESSION_KEYS` (no cache counts).
const SESSION_KEYS: [&str; 11] = [
    "session_id",
    "account",
    "project",
    "last_ts",
    "tool_calls",
    "tokens_in",
    "tokens_out",
    "cost_usd",
    "attention",
    "stage",
    "stage_skill",
];
/// Python's `_EVENT_KEYS` (no cache counts).
const EVENT_KEYS: [&str; 14] = [
    "id",
    "session_id",
    "account",
    "project",
    "ts",
    "kind",
    "tool",
    "is_error",
    "is_sidechain",
    "label",
    "agent",
    "tokens_in",
    "tokens_out",
    "cost_usd",
];

/// The tail the thread polls and the streams read.
pub(crate) type SharedTail = Arc<Mutex<LiveTail>>;

/// A running feed. Dropping it stops it like [`LiveFeed::stop`].
pub struct LiveFeed {
    tail: SharedTail,
    stop: Option<mpsc::Sender<()>>,
    done: mpsc::Receiver<()>,
    thread: Option<JoinHandle<()>>,
}

impl LiveFeed {
    /// The feed of the binary: the shipped price table, the system clock, and a step over the store at
    /// `EASY_PERSONAL_HOME` / `HOME` (read on each step, like the routes), with no transcript override.
    pub fn system() -> Result<LiveFeed, String> {
        let prices = PriceTable::shipped().map_err(|_| "the shipped price table cannot be read")?;
        LiveFeed::start(prices, now_micros, |tail, now| {
            if let Ok(path) = store_path() {
                tail.poll_store(&path, None, now);
            }
        })
    }

    /// Starts the thread: every [`POLL`] it calls `step(tail, clock())` under the lock, until [`LiveFeed::stop`].
    pub(crate) fn start(
        prices: PriceTable,
        clock: impl Fn() -> i64 + Send + 'static,
        mut step: impl FnMut(&mut LiveTail, i64) + Send + 'static,
    ) -> Result<LiveFeed, String> {
        quiet_panics();
        let tail = Arc::new(Mutex::new(LiveTail::new(prices)));
        let (stop, stopped) = mpsc::channel::<()>();
        let (finished, done) = mpsc::channel::<()>();
        let shared = Arc::clone(&tail);
        let thread = thread::Builder::new()
            .name(THREAD_NAME.into())
            .spawn(move || {
                let _finished = finished; // dropped when the thread ends: `done` then reads as disconnected
                while let Err(RecvTimeoutError::Timeout) = stopped.recv_timeout(POLL) {
                    let now = clock();
                    let mut tail = shared.lock().unwrap_or_else(PoisonError::into_inner);
                    let _ = panic::catch_unwind(AssertUnwindSafe(|| step(&mut tail, now)));
                }
            })
            .map_err(|_| "cannot start the live tail")?;
        Ok(LiveFeed {
            tail,
            stop: Some(stop),
            done,
            thread: Some(thread),
        })
    }

    pub(crate) fn tail(&self) -> SharedTail {
        Arc::clone(&self.tail)
    }

    /// Ends the thread and joins it. A step still running after [`STOP_WAIT`] is left behind (false), as Python's
    /// `join(timeout=10)` does.
    pub fn stop(mut self) -> bool {
        self.halt()
    }

    fn halt(&mut self) -> bool {
        drop(self.stop.take());
        let Some(thread) = self.thread.take() else {
            return true;
        };
        match self.done.recv_timeout(STOP_WAIT) {
            Err(RecvTimeoutError::Disconnected) | Ok(()) => thread.join().is_ok(),
            Err(RecvTimeoutError::Timeout) => false,
        }
    }
}

impl Drop for LiveFeed {
    fn drop(&mut self) {
        self.halt();
    }
}

/// A panic in a poll step prints one fixed line (the first time only) instead of the panic's text; every other
/// thread keeps the hook it had.
fn quiet_panics() {
    static ONCE: Once = Once::new();
    static PRINTED: AtomicBool = AtomicBool::new(false);
    ONCE.call_once(|| {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            if thread::current().name() == Some(THREAD_NAME) {
                if !PRINTED.swap(true, Ordering::Relaxed) {
                    eprintln!("easy-personal-rs: live tail: a poll step failed and was skipped");
                }
            } else {
                previous(info);
            }
        }));
    });
}

/// Python's `_resume_id`: the `Last-Event-ID` value stripped of the characters Python's `str.strip()` removes (the
/// header is read as Latin-1), then plain ASCII digits as a non-negative number; anything else is `None`. A number
/// past `i64::MAX` is `None` too: Python would take it as above the tail's last id, which also starts with a snapshot.
pub(crate) fn resume_id(value: &[u8]) -> Option<i64> {
    let space = |b: &u8| matches!(b, b' ' | b'\t' | 0x0a..=0x0d | 0x1c..=0x1f | 0x85 | 0xa0);
    let start = value.iter().position(|b| !space(b))?;
    let end = value.iter().rposition(|b| !space(b))? + 1;
    let digits = &value[start..end];
    if !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(digits).ok()?.parse().ok()
}

/// One client's place in the stream: Python's `cursor` and `seen` of `live_stream`.
pub(crate) struct LiveStream {
    cursor: Option<i64>,
    seen: Option<i64>,
}

impl LiveStream {
    /// `resume` is the client's `Last-Event-ID`; it counts only when a tail runs (Python: `live.current()`).
    pub(crate) fn new(resume: Option<i64>, tail_running: bool) -> Self {
        LiveStream {
            cursor: resume.filter(|_| tail_running),
            seen: None,
        }
    }

    /// One pass of Python's loop body (without the ping and the sleep): the bytes to send, maybe none.
    pub(crate) fn step(&mut self, tail: Option<&LiveTail>, now: i64) -> Vec<u8> {
        let mut out = Vec::new();
        if let (Some(t), Some(c)) = (tail, self.cursor)
            && (c > t.last_id() || t.rolled_past(c))
        {
            self.cursor = None; // an id from before a restart, or older than the buffer: start over
        }
        match (self.cursor, tail) {
            (None, _) => {
                let snapshot = tail.map_or_else(empty_snapshot, |t| t.snapshot(now));
                self.take(&snapshot);
                out.extend(sse("snapshot", &snapshot_data(&snapshot), None));
            }
            (Some(cursor), Some(t)) if self.seen != Some(t.state_version()) => {
                let snapshot = t.snapshot(now); // attention changed: events up to its `last_id` first
                let upto = snapshot["last_id"].as_i64().unwrap_or(0);
                for event in t.events_since(cursor) {
                    let id = event["id"].as_i64().unwrap_or(0);
                    if id > upto {
                        break;
                    }
                    out.extend(sse("live", &event_data(&event), Some(id)));
                }
                self.take(&snapshot);
                out.extend(sse("snapshot", &snapshot_data(&snapshot), None));
            }
            _ => {}
        }
        if let (Some(t), Some(cursor)) = (tail, self.cursor) {
            for event in t.events_since(cursor) {
                let id = event["id"].as_i64().unwrap_or(0);
                self.cursor = Some(id);
                out.extend(sse("live", &event_data(&event), Some(id)));
            }
        }
        out
    }

    fn take(&mut self, snapshot: &Value) {
        self.cursor = Some(snapshot["last_id"].as_i64().unwrap_or(0));
        self.seen = Some(snapshot["state_version"].as_i64().unwrap_or(0));
    }
}

/// The body of one `GET /api/live` answer: Python's `live_stream` loop. A first pass at once, then one every [`POLL`]:
/// the step's messages, and a `: ping` comment when [`PING`] passed since the last one. It ends when the server stops
/// (`stopping`) and when hyper drops it (the client went away: a write failed). The tail is read with `try_lock`, so a
/// poll step in progress never blocks a runtime worker (the stream tries again a tick later). Back-pressure: a client
/// that does not read is not polled, so nothing piles up here; what one pass can hold is bounded by the tail's buffer.
pub(crate) struct SseBody {
    stream: LiveStream,
    tail: Option<SharedTail>,
    stopping: Arc<AtomicBool>,
    clock: Clock,
    sleep: Pin<Box<Sleep>>,
    last_ping: Instant,
    first: bool,
}

impl SseBody {
    pub(crate) fn new(
        resume: Option<i64>,
        tail: Option<SharedTail>,
        stopping: Arc<AtomicBool>,
        clock: Clock,
    ) -> Self {
        SseBody {
            stream: LiveStream::new(resume, tail.is_some()),
            tail,
            stopping,
            clock,
            sleep: Box::pin(tokio::time::sleep(POLL)),
            last_ping: Instant::now(),
            first: true,
        }
    }

    fn pass(&mut self) -> Vec<u8> {
        let now = (self.clock)();
        let mut out = match &self.tail {
            None => self.stream.step(None, now),
            Some(tail) => match tail.try_lock() {
                Ok(guard) => self.stream.step(Some(&guard), now),
                Err(TryLockError::Poisoned(p)) => self.stream.step(Some(&p.into_inner()), now),
                Err(TryLockError::WouldBlock) => Vec::new(),
            },
        };
        if self.last_ping.elapsed() >= PING {
            self.last_ping = Instant::now();
            out.extend_from_slice(b": ping\n\n");
        }
        out
    }
}

impl Stream for SseBody {
    type Item = Result<Bytes, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;
        loop {
            if !this.first {
                if this.sleep.as_mut().poll(cx).is_pending() {
                    return Poll::Pending;
                }
                this.sleep
                    .as_mut()
                    .reset(tokio::time::Instant::now() + POLL);
            }
            this.first = false;
            if this.stopping.load(Ordering::Relaxed) {
                return Poll::Ready(None);
            }
            let out = this.pass();
            if !out.is_empty() {
                return Poll::Ready(Some(Ok(Bytes::from(out))));
            }
        }
    }
}

/// Python's `_EMPTY_SNAPSHOT` (no tail running).
fn empty_snapshot() -> Value {
    json!({
        "active_window_s": ACTIVE_WINDOW_S, "last_id": 0, "state_version": 0,
        "open_call_minutes": OPEN_CALL_MINUTES, "sessions": [],
    })
}

fn pick(value: &Value, keys: &[&str]) -> Value {
    let mut out = Map::new();
    for key in keys {
        out.insert(
            (*key).into(),
            value.get(*key).cloned().unwrap_or(Value::Null),
        );
    }
    Value::Object(out)
}

/// Python's `_snapshot_data`.
fn snapshot_data(snapshot: &Value) -> Value {
    let sessions: Vec<Value> = snapshot["sessions"]
        .as_array()
        .map(|all| all.iter().map(|s| pick(s, &SESSION_KEYS)).collect())
        .unwrap_or_default();
    json!({
        "active_window_s": snapshot["active_window_s"], "last_id": snapshot["last_id"],
        "open_call_minutes": snapshot["open_call_minutes"], "sessions": sessions,
    })
}

fn event_data(event: &Value) -> Value {
    pick(event, &EVENT_KEYS)
}

/// Python's `_sse`: `event:`, then `id:` when given, then one `data:` line of compact JSON, then a blank line.
fn sse(event: &str, data: &Value, id: Option<i64>) -> Vec<u8> {
    let id = id.map_or_else(String::new, |id| format!("id: {id}\n"));
    format!("event: {event}\n{id}data: {data}\n\n").into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use easy_personal_core::live::Root;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicUsize;

    const SECOND: i64 = 1_000_000;
    const NOW: i64 = 1_768_867_200 * SECOND + 10 * SECOND;

    fn prices() -> PriceTable {
        PriceTable::shipped().unwrap()
    }

    struct Temp(PathBuf);

    impl Temp {
        fn new(tag: &str) -> Temp {
            static N: AtomicUsize = AtomicUsize::new(0);
            let dir = std::env::temp_dir().join(format!(
                "easy-personal-rs-t056-{tag}-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::SeqCst)
            ));
            std::fs::create_dir_all(dir.join("p")).unwrap();
            Temp(dir)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn call_line(sid: &str, block: &str, ts: &str) -> String {
        format!(
            "{{\"type\":\"assistant\",\"sessionId\":\"{sid}\",\"timestamp\":\"{ts}\",\"message\":{{\"id\":\"m-{block}\",\"model\":\"x\",\"content\":[{{\"type\":\"tool_use\",\"id\":\"{block}\",\"name\":\"Read\",\"input\":{{}}}}]}}}}\n"
        )
    }

    fn result_line(sid: &str, block: &str, error: bool, ts: &str) -> String {
        format!(
            "{{\"type\":\"user\",\"sessionId\":\"{sid}\",\"timestamp\":\"{ts}\",\"message\":{{\"content\":[{{\"type\":\"tool_result\",\"tool_use_id\":\"{block}\",\"is_error\":{error}}}]}}}}\n"
        )
    }

    /// A tail over one temp tree, primed (first scan done) with nothing in it.
    fn primed(tmp: &Temp) -> (LiveTail, Vec<Root>) {
        let roots = vec![Root {
            account: None,
            projects: tmp.0.clone(),
        }];
        let mut tail = LiveTail::new(prices());
        tail.poll_once(&roots, NOW, None);
        (tail, roots)
    }

    fn append(tmp: &Temp, text: &str) {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(tmp.0.join("p/s.jsonl"))
            .unwrap();
        f.write_all(text.as_bytes()).unwrap();
    }

    /// `(event, id, data)` of every message in `bytes` (no comment lines expected).
    fn messages(bytes: &[u8]) -> Vec<(String, Option<i64>, Value)> {
        let text = std::str::from_utf8(bytes).unwrap();
        assert!(text.is_empty() || text.ends_with("\n\n"), "{text:?}");
        text.split_terminator("\n\n")
            .map(|block| {
                let (mut event, mut id, mut data) = (String::new(), None, Value::Null);
                for line in block.lines() {
                    if let Some(v) = line.strip_prefix("event: ") {
                        event = v.into();
                    } else if let Some(v) = line.strip_prefix("id: ") {
                        id = Some(v.parse().unwrap());
                    } else if let Some(v) = line.strip_prefix("data: ") {
                        data = serde_json::from_str(v).unwrap();
                    } else {
                        panic!("unexpected line {line:?}");
                    }
                }
                (event, id, data)
            })
            .collect()
    }

    // ---- the oracle (AC 7) ---------------------------------------------------------------------------------------

    /// `parity/golden-live-stream/cases.json` (Python's `live_stream`, `gen_live_stream.py`): per step the same file
    /// operations, one `poll_once` at the same frozen `now`, then one stream pass; the messages (event, id, data JSON)
    /// must equal Python's. Returns (equal steps, steps).
    #[test]
    fn the_stream_equals_python_live_stream_for_every_oracle_case() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../parity/golden-live-stream/cases.json");
        let golden: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let t0 = golden["t0"].as_i64().unwrap() * SECOND;
        let canary = golden["canary"].as_str().unwrap();
        let (mut equal, mut total, mut names) = (0, 0, Vec::new());
        for case in golden["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            names.push(name.to_owned());
            let tmp = Temp::new("oracle");
            let roots = vec![Root {
                account: None,
                projects: tmp.0.clone(),
            }];
            let mut tail = LiveTail::new(prices());
            let running = case["tail"].as_bool().unwrap();
            let open_at = case["open_at"].as_u64().unwrap() as usize;
            let resume = case["resume"]
                .as_str()
                .and_then(|r| resume_id(r.as_bytes()));
            let mut stream: Option<LiveStream> = None;
            for (index, step) in case["steps"].as_array().unwrap().iter().enumerate() {
                for op in step["ops"].as_array().unwrap() {
                    let file = tmp.0.join(op["path"].as_str().unwrap());
                    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                    let text = op["text"].as_str().unwrap();
                    match op["op"].as_str().unwrap() {
                        "write" => std::fs::write(&file, text).unwrap(),
                        "append" => {
                            use std::io::Write;
                            std::fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(&file)
                                .unwrap()
                                .write_all(text.as_bytes())
                                .unwrap();
                        }
                        other => panic!("op {other}"),
                    }
                }
                let now = t0 + (step["now"].as_f64().unwrap() * 1e6) as i64;
                tail.poll_once(&roots, now, None);
                if index == open_at {
                    stream = Some(LiveStream::new(resume, running));
                }
                let bytes = stream
                    .as_mut()
                    .map(|s| s.step(running.then_some(&tail), now))
                    .unwrap_or_default();
                assert!(!String::from_utf8_lossy(&bytes).contains(canary), "{name}");
                let got: Vec<Value> = messages(&bytes)
                    .into_iter()
                    .map(|(event, id, data)| json!([event, id, data]))
                    .collect();
                total += 1;
                if Value::Array(got.clone()) == step["messages"] {
                    equal += 1;
                } else {
                    eprintln!(
                        "{name} step {index}: rust {got:?}\n python {}",
                        step["messages"]
                    );
                }
            }
        }
        eprintln!(
            "live stream oracle: {equal} / {total} steps equal over {} cases {names:?}",
            names.len()
        );
        assert_eq!(names.len(), 10);
        assert_eq!(equal, total);
    }

    // ---- the thread --------------------------------------------------------------------------------------------

    #[test]
    fn the_thread_polls_every_tenth_of_a_second_with_the_injected_clock() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let ticks = Arc::new(AtomicUsize::new(0));
        let (s, t) = (Arc::clone(&seen), Arc::clone(&ticks));
        let feed = LiveFeed::start(
            prices(),
            move || 1000 + t.fetch_add(1, Ordering::SeqCst) as i64,
            move |_tail, now| s.lock().unwrap().push(now),
        )
        .unwrap();
        thread::sleep(Duration::from_millis(550));
        assert!(feed.stop());
        let seen = seen.lock().unwrap().clone();
        assert!((3..=6).contains(&seen.len()), "{seen:?}");
        let want: Vec<i64> = (0..seen.len() as i64).map(|i| 1000 + i).collect();
        assert_eq!(seen, want, "each step gets the clock's next value");
    }

    #[test]
    fn a_step_that_panics_is_skipped_and_the_thread_goes_on() {
        let calls = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&calls);
        let feed = LiveFeed::start(
            prices(),
            || NOW,
            move |_tail, _now| {
                if c.fetch_add(1, Ordering::SeqCst).is_multiple_of(2) {
                    panic!("injected failure /secret/path");
                }
            },
        )
        .unwrap();
        thread::sleep(Duration::from_millis(550));
        assert!(feed.tail().lock().is_ok(), "the lock is not poisoned");
        assert!(feed.stop());
        assert!(calls.load(Ordering::SeqCst) >= 3);
    }

    #[test]
    fn stop_waits_for_the_step_in_flight_and_returns_promptly_otherwise() {
        let feed = LiveFeed::start(prices(), || NOW, |_, _| {}).unwrap();
        let began = Instant::now();
        assert!(feed.stop());
        assert!(
            began.elapsed() < Duration::from_millis(500),
            "{:?}",
            began.elapsed()
        );

        let feed = LiveFeed::start(
            prices(),
            || NOW,
            |_, _| {
                thread::sleep(Duration::from_millis(400));
            },
        )
        .unwrap();
        thread::sleep(Duration::from_millis(150)); // a step is running now
        let began = Instant::now();
        assert!(
            feed.stop(),
            "the step ended in time and the thread was joined"
        );
        assert!(
            began.elapsed() < Duration::from_secs(2),
            "{:?}",
            began.elapsed()
        );
    }

    #[test]
    fn dropping_the_feed_stops_the_thread() {
        let calls = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&calls);
        let feed = LiveFeed::start(
            prices(),
            || NOW,
            move |_, _| {
                c.fetch_add(1, Ordering::SeqCst);
            },
        )
        .unwrap();
        thread::sleep(Duration::from_millis(250));
        drop(feed);
        let after = calls.load(Ordering::SeqCst);
        thread::sleep(Duration::from_millis(300));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            after,
            "no step after the drop"
        );
    }

    #[test]
    fn the_system_step_with_no_store_keeps_running_and_finds_nothing() {
        // The real step reads the store path from the environment; here it is a temp folder with no store.
        let tmp = Temp::new("nostore");
        let path = tmp.0.join("store.sqlite");
        let feed = LiveFeed::start(prices(), now_micros, move |tail, now| {
            tail.poll_store(&path, None, now)
        })
        .unwrap();
        thread::sleep(Duration::from_millis(350));
        let snapshot = feed.tail().lock().unwrap().snapshot(now_micros());
        assert_eq!(snapshot["sessions"], json!([]));
        assert_eq!(snapshot["last_id"], 0);
        assert!(feed.stop());
        assert!(!tmp.0.join("store.sqlite").exists(), "no store is created");
        assert!(!tmp.0.join("ingest.lock").exists());
    }

    // ---- Last-Event-ID -------------------------------------------------------------------------------------------

    #[test]
    fn resume_id_takes_python_decimal_numbers_only() {
        for (raw, want) in [
            (&b"5"[..], Some(5)),
            (b"0", Some(0)),
            (b"  12 \t", Some(12)),
            (b"\xa012\x85", Some(12)),
            (b"007", Some(7)),
            (b"9223372036854775807", Some(i64::MAX)),
            (b"9223372036854775808", None),
            (b"99999999999999999999999", None),
            (b"", None),
            (b"   ", None),
            (b"-1", None),
            (b"+1", None),
            (b"1 2", None),
            (b"1.0", None),
            (b"abc", None),
            (b"\xb2", None), // superscript two: a digit, not a decimal, for Python
        ] {
            assert_eq!(resume_id(raw), want, "{raw:?}");
        }
    }

    // ---- the stream ----------------------------------------------------------------------------------------------

    #[test]
    fn with_no_tail_the_stream_sends_one_empty_snapshot_then_nothing() {
        let mut stream = LiveStream::new(Some(5), false);
        let first = stream.step(None, NOW);
        assert_eq!(
            std::str::from_utf8(&first).unwrap(),
            "event: snapshot\ndata: {\"active_window_s\":300,\"last_id\":0,\"open_call_minutes\":10,\"sessions\":[]}\n\n"
        );
        assert!(stream.step(None, NOW).is_empty());
    }

    #[test]
    fn a_snapshot_then_one_live_event_per_new_event_with_python_keys_only() {
        let tmp = Temp::new("stream");
        let (mut tail, roots) = primed(&tmp);
        let mut stream = LiveStream::new(None, true);
        let first = messages(&stream.step(Some(&tail), NOW));
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].0, "snapshot");
        assert_eq!(first[0].2["sessions"], json!([]));

        append(&tmp, &call_line("s1", "b1", "2026-01-20T00:00:05Z"));
        tail.poll_once(&roots, NOW, None);
        let next = messages(&stream.step(Some(&tail), NOW));
        let kinds: Vec<(&str, Option<i64>)> = next.iter().map(|m| (m.0.as_str(), m.1)).collect();
        assert_eq!(kinds, [("live", Some(1))], "{next:?}");
        let keys: Vec<&String> = next[0].2.as_object().unwrap().keys().collect();
        let mut want: Vec<&str> = EVENT_KEYS.to_vec();
        want.sort_unstable();
        assert_eq!(keys, want, "no cache counts, no text");
        assert!(
            stream.step(Some(&tail), NOW).is_empty(),
            "nothing new: nothing sent"
        );
    }

    #[test]
    fn an_attention_change_sends_the_events_up_to_the_snapshot_then_the_snapshot() {
        let tmp = Temp::new("attention");
        let (mut tail, roots) = primed(&tmp);
        let mut stream = LiveStream::new(None, true);
        stream.step(Some(&tail), NOW);
        let mut text = String::new();
        for i in 0..3 {
            text += &call_line("s1", &format!("b{i}"), "2026-01-20T00:00:05Z");
            text += &result_line("s1", &format!("b{i}"), true, "2026-01-20T00:00:06Z");
        }
        append(&tmp, &text);
        let before = tail.state_version();
        tail.poll_once(&roots, NOW, None);
        assert_ne!(
            tail.state_version(),
            before,
            "three failed results: failing"
        );
        let sent = messages(&stream.step(Some(&tail), NOW));
        let names: Vec<&str> = sent.iter().map(|m| m.0.as_str()).collect();
        assert_eq!(
            names,
            ["live"; 6]
                .iter()
                .copied()
                .chain(["snapshot"])
                .collect::<Vec<_>>()
        );
        let ids: Vec<i64> = sent.iter().filter_map(|m| m.1).collect();
        assert_eq!(ids, [1, 2, 3, 4, 5, 6]);
        let snapshot = &sent[6].2;
        assert_eq!(snapshot["last_id"], 6);
        let keys: Vec<&String> = snapshot["sessions"][0]
            .as_object()
            .unwrap()
            .keys()
            .collect();
        let mut want: Vec<&str> = SESSION_KEYS.to_vec();
        want.sort_unstable();
        assert_eq!(keys, want);
        assert_eq!(snapshot["sessions"][0]["attention"][0]["state"], "failing");
        assert!(stream.step(Some(&tail), NOW).is_empty());
    }

    #[test]
    fn a_good_resume_replays_after_the_id_then_one_fresh_snapshot() {
        let tmp = Temp::new("resume");
        let (mut tail, roots) = primed(&tmp);
        append(
            &tmp,
            &(call_line("s1", "b1", "2026-01-20T00:00:05Z")
                + &call_line("s1", "b2", "2026-01-20T00:00:06Z")
                + &call_line("s1", "b3", "2026-01-20T00:00:07Z")),
        );
        tail.poll_once(&roots, NOW, None);
        let mut stream = LiveStream::new(Some(1), true);
        let sent = messages(&stream.step(Some(&tail), NOW));
        let kinds: Vec<(&str, Option<i64>)> = sent.iter().map(|m| (m.0.as_str(), m.1)).collect();
        assert_eq!(
            kinds,
            [("live", Some(2)), ("live", Some(3)), ("snapshot", None)]
        );
    }

    #[test]
    fn a_resume_past_the_last_id_or_with_no_tail_starts_with_a_snapshot() {
        let tmp = Temp::new("stale");
        let (mut tail, roots) = primed(&tmp);
        append(&tmp, &call_line("s1", "b1", "2026-01-20T00:00:05Z"));
        tail.poll_once(&roots, NOW, None);
        for resume in [Some(2), Some(i64::MAX), None] {
            let mut stream = LiveStream::new(resume, true);
            let sent = messages(&stream.step(Some(&tail), NOW));
            let kinds: Vec<(&str, Option<i64>)> =
                sent.iter().map(|m| (m.0.as_str(), m.1)).collect();
            assert_eq!(kinds, [("snapshot", None)], "{resume:?}");
            assert_eq!(sent[0].2["last_id"], 1);
        }
    }

    #[test]
    fn a_thousand_events_in_one_step_go_out_in_order_once_each() {
        let tmp = Temp::new("thousand");
        let (mut tail, roots) = primed(&tmp);
        let mut stream = LiveStream::new(None, true);
        stream.step(Some(&tail), NOW);
        let text: String = (0..1000)
            .map(|i| call_line("s1", &format!("b{i}"), "2026-01-20T00:00:05Z"))
            .collect();
        append(&tmp, &text);
        tail.poll_once(&roots, NOW, None);
        let mut ids = Vec::new();
        for _ in 0..3 {
            let sent = messages(&stream.step(Some(&tail), NOW));
            ids.extend(sent.iter().filter(|m| m.0 == "live").filter_map(|m| m.1));
        }
        assert_eq!(ids, (1..=1000).collect::<Vec<_>>());
    }
}
