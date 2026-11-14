//! T056: `GET /api/live` of the real `serve` binary, over raw `TcpStream`s. Each test has its own temp data folder
//! (`HOME`, `EASY_PERSONAL_HOME`, `EASY_PERSONAL_AGENT_ROOT`): a copy of `parity/store-v11.sqlite` with one confirmed
//! Claude account whose folder is `<home>/.claude`, and transcripts written there by the test. The owner's
//! `~/.easy-personal` and `~/.claude*` are never reached.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const EVENT_KEYS: [&str; 14] = [
    "account",
    "agent",
    "cost_usd",
    "id",
    "is_error",
    "is_sidechain",
    "kind",
    "label",
    "project",
    "session_id",
    "tokens_in",
    "tokens_out",
    "tool",
    "ts",
];

fn fixture_store() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../parity/store-v11.sqlite")
}

/// `YYYY-MM-DDTHH:MM:SS.mmmZ` of now (UTC), the transcripts' stamp form.
fn stamp_now() -> String {
    let since = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let (secs, ms) = (since.as_secs() as i64, since.subsec_millis());
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{ms:03}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

fn call_line(sid: &str, block: &str) -> String {
    format!(
        "{{\"type\":\"assistant\",\"sessionId\":\"{sid}\",\"timestamp\":\"{}\",\"cwd\":\"/tmp\",\"message\":{{\"id\":\"m-{block}\",\"model\":\"x\",\"content\":[{{\"type\":\"tool_use\",\"id\":\"{block}\",\"name\":\"Read\",\"input\":{{\"file_path\":\"CANARY-T056-INPUT\"}}}},{{\"type\":\"text\",\"text\":\"CANARY-T056-TEXT\"}}]}}}}\n",
        stamp_now()
    )
}

struct Serve {
    child: Child,
    _stdout: BufReader<ChildStdout>,
    port: u16,
    token: String,
    home: PathBuf,
}

impl Serve {
    fn start() -> Serve {
        static N: AtomicU32 = AtomicU32::new(0);
        let home = std::env::temp_dir().join(format!(
            "easy-personal-rs-t056-live-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(home.join(".claude/projects/p")).unwrap();
        std::fs::copy(fixture_store(), home.join("store.sqlite")).unwrap();
        let conn = rusqlite::Connection::open(home.join("store.sqlite")).unwrap();
        conn.execute(
            "INSERT INTO agent_accounts (id, provider, folder, label, status) VALUES (1, 'claude', ?1, 'Work', 'confirmed')",
            [home.join(".claude").to_str().unwrap()],
        )
        .unwrap();
        drop(conn);
        let mut child = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
            .args(["serve", "--no-ingest"])
            .env("HOME", &home)
            .env("EASY_PERSONAL_HOME", &home)
            .env("EASY_PERSONAL_AGENT_ROOT", &home)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        let url = line
            .trim_end()
            .strip_prefix("dashboard: http://127.0.0.1:")
            .unwrap()
            .to_string();
        let (port, token) = url.split_once("/#").unwrap();
        let mut second = String::new();
        stdout.read_line(&mut second).unwrap();
        Serve {
            port: port.parse().unwrap(),
            token: token.into(),
            child,
            _stdout: stdout,
            home,
        }
    }

    fn transcript(&self) -> PathBuf {
        self.home.join(".claude/projects/p/s.jsonl")
    }

    fn append(&self, text: &str) {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.transcript())
            .unwrap();
        f.write_all(text.as_bytes()).unwrap();
    }

    /// Sends `GET <path>` (or another method) with the good Host and the extra header lines; the connection stays open.
    fn open(&self, method: &str, path: &str, extra: &[(&str, &str)]) -> Stream {
        let mut tcp = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        let mut req = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n",
            self.port
        );
        for (k, v) in extra {
            req += &format!("{k}: {v}\r\n");
        }
        req += "\r\n";
        tcp.write_all(req.as_bytes()).unwrap();
        tcp.set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let mut stream = Stream {
            tcp,
            buf: Vec::new(),
            head: String::new(),
            received: 0,
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        while !stream.buf.windows(4).any(|w| w == b"\r\n\r\n") {
            assert!(Instant::now() < deadline, "no answer head");
            stream.fill();
        }
        let at = stream
            .buf
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .unwrap();
        stream.head = String::from_utf8_lossy(&stream.buf[..at]).into_owned();
        stream.buf.drain(..at + 4);
        stream
    }

    fn live(&self, extra: &[(&str, &str)]) -> Stream {
        let path = format!("/api/live?token={}", self.token);
        let stream = self.open("GET", &path, extra);
        assert_eq!(stream.status(), 200, "{}", stream.head);
        stream
    }

    fn sigint(&mut self) -> (std::process::ExitStatus, Duration, String) {
        let began = Instant::now();
        assert!(
            Command::new("kill")
                .args(["-INT", &self.child.id().to_string()])
                .status()
                .unwrap()
                .success()
        );
        let status = loop {
            if let Some(s) = self.child.try_wait().unwrap() {
                break s;
            }
            assert!(began.elapsed() < Duration::from_secs(15), "no exit");
            std::thread::sleep(Duration::from_millis(20));
        };
        let mut err = String::new();
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut err)
            .unwrap();
        (status, began.elapsed(), err)
    }
}

impl Drop for Serve {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

struct Stream {
    tcp: TcpStream,
    buf: Vec<u8>,
    head: String,
    /// Body bytes read so far.
    received: usize,
}

/// One SSE message: `event`, `id`, `data` (parsed), or a comment line.
#[derive(Debug)]
struct Msg {
    event: String,
    id: Option<i64>,
    data: serde_json::Value,
}

impl Stream {
    /// Reads what is there (at most 100 ms); false at the end of the stream.
    fn fill(&mut self) -> bool {
        let mut chunk = [0u8; 65536];
        match self.tcp.read(&mut chunk) {
            Ok(0) => false,
            Ok(n) => {
                self.buf.extend_from_slice(&chunk[..n]);
                self.received += n;
                true
            }
            Err(_) => true, // timeout
        }
    }

    fn status(&self) -> u16 {
        self.head.split(' ').nth(1).unwrap().parse().unwrap()
    }

    fn header(&self, name: &str) -> Vec<String> {
        self.head
            .split("\r\n")
            .skip(1)
            .filter_map(|l| l.split_once(':'))
            .filter(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim().to_string())
            .collect()
    }

    /// The SSE text of the body: the chunked framing taken off.
    fn take_text(&mut self) -> String {
        let mut text = String::new();
        while let Some(end) = self.buf.windows(2).position(|w| w == b"\r\n") {
            let size =
                usize::from_str_radix(std::str::from_utf8(&self.buf[..end]).unwrap(), 16).unwrap();
            if self.buf.len() < end + 2 + size + 2 {
                break;
            }
            text += std::str::from_utf8(&self.buf[end + 2..end + 2 + size]).unwrap();
            self.buf.drain(..end + 2 + size + 2);
        }
        text
    }

    /// Messages until `want` returns true for the list, or `limit` passes (then panics with what came).
    fn until(&mut self, limit: Duration, want: impl Fn(&[Msg]) -> bool) -> Vec<Msg> {
        let deadline = Instant::now() + limit;
        let mut text = String::new();
        loop {
            text += &self.take_text();
            let msgs = parse(&text);
            if want(&msgs) {
                return msgs;
            }
            assert!(Instant::now() < deadline, "timed out; got {msgs:?}");
            assert!(self.fill(), "stream ended; got {msgs:?}");
        }
    }

    /// True once the server closed the stream within `limit`.
    fn ends_within(&mut self, limit: Duration) -> bool {
        let deadline = Instant::now() + limit;
        while Instant::now() < deadline {
            if !self.fill() {
                return true;
            }
        }
        false
    }
}

fn parse(text: &str) -> Vec<Msg> {
    let complete = &text[..text.rfind("\n\n").map_or(0, |i| i + 2)];
    complete
        .split_terminator("\n\n")
        .filter(|block| !block.starts_with(':'))
        .map(|block| {
            let mut msg = Msg {
                event: String::new(),
                id: None,
                data: serde_json::Value::Null,
            };
            for line in block.lines() {
                if let Some(v) = line.strip_prefix("event: ") {
                    msg.event = v.into();
                } else if let Some(v) = line.strip_prefix("id: ") {
                    msg.id = Some(v.parse().unwrap());
                } else if let Some(v) = line.strip_prefix("data: ") {
                    msg.data = serde_json::from_str(v).unwrap();
                } else {
                    panic!("unexpected line {line:?}");
                }
            }
            msg
        })
        .collect()
}

fn lives(msgs: &[Msg]) -> Vec<i64> {
    msgs.iter()
        .filter(|m| m.event == "live")
        .filter_map(|m| m.id)
        .collect()
}

#[test]
fn a_new_transcript_line_arrives_as_a_live_event_within_2_s() {
    let serve = Serve::start();
    let mut stream = serve.live(&[]);
    assert_eq!(
        stream.header("content-type"),
        ["text/event-stream; charset=utf-8"]
    );
    assert_eq!(stream.header("cache-control"), ["no-store"]);
    assert_eq!(stream.header("x-accel-buffering"), ["no"]);
    assert_eq!(stream.header("content-security-policy").len(), 1);
    let first = stream.until(Duration::from_secs(5), |m| !m.is_empty());
    assert_eq!(first[0].event, "snapshot");
    assert_eq!(first[0].data["sessions"], serde_json::json!([]));
    assert_eq!(first[0].data["last_id"], 0);
    std::thread::sleep(Duration::from_millis(300)); // the tail's first scan is done
    let began = Instant::now();
    serve.append(&call_line("s1", "b1"));
    let msgs = stream.until(Duration::from_secs(2), |m| !lives(m).is_empty());
    assert!(began.elapsed() < Duration::from_secs(2));
    let live = msgs.iter().find(|m| m.event == "live").unwrap();
    assert_eq!(live.id, Some(1));
    let keys: Vec<&String> = live.data.as_object().unwrap().keys().collect();
    assert_eq!(keys, EVENT_KEYS);
    assert_eq!(live.data["tool"], "Read");
    assert_eq!(live.data["account"], "Work");
    // A header token works the same; the query token is not needed then.
    let header = serve.open(
        "GET",
        "/api/live",
        &[("x-dashboard-token", serve.token.as_str())],
    );
    assert_eq!(header.status(), 200);
}

#[test]
fn the_guard_refuses_the_live_route_without_the_token_and_the_query_token_elsewhere() {
    let serve = Serve::start();
    let token = serve.token.clone();
    let q = format!("?token={token}");
    let header = [("x-dashboard-token", token.as_str())];
    for (method, path, extra, want) in [
        ("GET", "/api/live".to_string(), &[][..], 403),
        ("GET", "/api/live?token=wrong".into(), &[], 403),
        ("GET", format!("/api/version{q}"), &[], 403),
        ("GET", format!("/api/usage/summary{q}"), &[], 403),
        ("HEAD", format!("/api/live{q}"), &[], 403),
        ("POST", format!("/api/live{q}"), &[], 403),
        ("POST", "/api/live".into(), &header[..], 405),
        (
            "GET",
            format!("/api/live{q}"),
            &[("last-event-id", "1"), ("last-event-id", "2")][..],
            403,
        ),
    ] {
        let mut extra = extra.to_vec();
        extra.push(("connection", "close"));
        let mut answer = serve.open(method, &path, &extra);
        assert_eq!(answer.status(), want, "{method} {path}");
        answer.ends_within(Duration::from_secs(2));
        let all = format!("{}{}", answer.head, String::from_utf8_lossy(&answer.buf));
        assert!(!all.contains(&token), "the token is never echoed");
        if want == 403 && method != "HEAD" {
            assert!(all.ends_with("{\"error\":\"forbidden\"}"), "{all}");
        }
    }
    // A bad Host with a good query token.
    let mut tcp = TcpStream::connect(("127.0.0.1", serve.port)).unwrap();
    tcp.write_all(
        format!("GET /api/live{q} HTTP/1.1\r\nHost: evil.test\r\nConnection: close\r\n\r\n")
            .as_bytes(),
    )
    .unwrap();
    let mut answer = String::new();
    tcp.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let _ = tcp.read_to_string(&mut answer);
    assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
}

#[test]
fn last_event_id_resumes_after_a_good_id_and_starts_over_after_a_stale_one() {
    let serve = Serve::start();
    let mut watch = serve.live(&[]);
    watch.until(Duration::from_secs(5), |m| !m.is_empty());
    std::thread::sleep(Duration::from_millis(300));
    serve.append(&(call_line("s1", "b1") + &call_line("s1", "b2") + &call_line("s1", "b3")));
    watch.until(Duration::from_secs(3), |m| lives(m) == [1, 2, 3]);
    let mut good = serve.live(&[("last-event-id", "1")]);
    let msgs = good.until(Duration::from_secs(3), |m| {
        m.iter().any(|x| x.event == "snapshot")
    });
    let order: Vec<(&str, Option<i64>)> = msgs.iter().map(|m| (m.event.as_str(), m.id)).collect();
    assert_eq!(
        order,
        [("live", Some(2)), ("live", Some(3)), ("snapshot", None)]
    );
    for stale in ["99", "-1", "x", " 7 "] {
        let mut s = serve.live(&[("last-event-id", stale)]);
        let msgs = s.until(Duration::from_secs(3), |m| !m.is_empty());
        assert_eq!(msgs[0].event, "snapshot", "{stale}");
        assert_eq!(msgs[0].data["last_id"], 3, "{stale}");
    }
}

#[test]
fn sigint_with_two_streams_open_exits_0_within_10_s_and_ends_the_streams() {
    let mut serve = Serve::start();
    let mut a = serve.live(&[]);
    let mut b = serve.live(&[]);
    a.until(Duration::from_secs(5), |m| !m.is_empty());
    b.until(Duration::from_secs(5), |m| !m.is_empty());
    let (status, took, err) = serve.sigint();
    assert_eq!(status.code(), Some(0), "{err}");
    assert!(took < Duration::from_secs(10), "{took:?}");
    assert!(a.ends_within(Duration::from_secs(2)));
    assert!(b.ends_within(Duration::from_secs(2)));
    assert!(!err.contains(&serve.token));
    assert!(!err.contains(serve.home.to_str().unwrap()), "{err}");
}

#[test]
fn ten_streams_get_the_events_while_one_client_never_reads() {
    let serve = Serve::start();
    // A client that asks and never reads: its socket buffers fill, and that must hold up no one.
    let stuck = serve.live(&[]);
    let mut streams: Vec<Stream> = (0..10).map(|_| serve.live(&[])).collect();
    for s in &mut streams {
        s.until(Duration::from_secs(5), |m| !m.is_empty());
    }
    std::thread::sleep(Duration::from_millis(300));
    let text: String = (0..1500)
        .map(|i| call_line("s1", &format!("b{i}")))
        .collect();
    serve.append(&text);
    let want: Vec<i64> = (1..=1500).collect();
    for s in &mut streams {
        s.until(Duration::from_secs(10), |m| lives(m) == want);
    }
    serve.append(&call_line("s1", "late"));
    for s in &mut streams {
        let msgs = s.until(Duration::from_secs(2), |m| lives(m).contains(&1501));
        assert_eq!(lives(&msgs), [1501]);
    }
    drop(stuck);
}

#[test]
fn the_tail_never_writes_the_store_and_the_ingest_runs_beside_it() {
    let serve = Serve::start();
    let store = serve.home.join("store.sqlite");
    let before = (
        std::fs::read(&store).unwrap(),
        std::fs::metadata(&store).unwrap().modified().unwrap(),
    );
    let mut stream = serve.live(&[]);
    stream.until(Duration::from_secs(5), |m| !m.is_empty());
    std::thread::sleep(Duration::from_millis(300));
    serve.append(&(call_line("s1", "b1") + &call_line("s1", "b2")));
    let msgs = stream.until(Duration::from_secs(3), |m| lives(m) == [1, 2]);
    for _ in 0..10 {
        assert!(
            !serve.home.join("ingest.lock").exists(),
            "the tail takes no lock"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        std::fs::read(&store).unwrap(),
        before.0,
        "store bytes unchanged"
    );
    assert_eq!(
        std::fs::metadata(&store).unwrap().modified().unwrap(),
        before.1
    );
    let text = format!("{:?}", msgs);
    assert!(
        !text.contains("CANARY"),
        "no text or tool input in an event"
    );

    // The scheduled ingest runs while the tail polls: it gets the lock, writes, exits 0; the stream goes on.
    let ingest = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
        .arg("ingest")
        .env("HOME", &serve.home)
        .env("EASY_PERSONAL_HOME", &serve.home)
        .env("EASY_PERSONAL_AGENT_ROOT", &serve.home)
        .output()
        .unwrap();
    assert!(
        ingest.status.success(),
        "{}",
        String::from_utf8_lossy(&ingest.stderr)
    );
    serve.append(&call_line("s1", "b3"));
    stream.until(Duration::from_secs(3), |m| lives(m) == [3]);
}

#[test]
fn a_store_that_is_missing_gives_an_empty_snapshot_and_the_tail_picks_it_up_later() {
    let serve = Serve::start();
    let store = serve.home.join("store.sqlite");
    let held = serve.home.join("held.sqlite");
    std::fs::rename(&store, &held).unwrap();
    let mut stream = serve.live(&[]);
    let first = stream.until(Duration::from_secs(5), |m| !m.is_empty());
    assert_eq!(first[0].data["sessions"], serde_json::json!([]));
    std::thread::sleep(Duration::from_millis(300));
    std::fs::rename(&held, &store).unwrap();
    std::thread::sleep(Duration::from_millis(300)); // warm start on the first good step
    serve.append(&call_line("s1", "b1"));
    stream.until(Duration::from_secs(3), |m| !lives(m).is_empty());
    assert!(!Path::new(&serve.home.join("ingest.lock")).exists());
}

/// Linux `setsockopt(SOL_SOCKET, SO_RCVBUF)` through the C library `std` already links (no new crate).
#[cfg(target_os = "linux")]
fn shrink_receive_buffer(tcp: &TcpStream, bytes: i32) {
    use std::os::fd::AsRawFd;
    unsafe extern "C" {
        fn setsockopt(fd: i32, level: i32, name: i32, value: *const i32, len: u32) -> i32;
    }
    const SOL_SOCKET: i32 = 1;
    const SO_RCVBUF: i32 = 8;
    // SAFETY: a valid open socket fd, a pointer to a live i32 and its size.
    let rc = unsafe { setsockopt(tcp.as_raw_fd(), SOL_SOCKET, SO_RCVBUF, &bytes, 4) };
    assert_eq!(rc, 0);
}

/// Fix round item 5: a client with a 4 KiB receive buffer asks and never reads, while more than the kernel's largest
/// send buffer (4 MiB here) is offered to every stream. It receives almost nothing (back-pressure: the server stops
/// polling its stream), a second stream still gets every later event, and SIGINT still exits 0 in time.
#[cfg(target_os = "linux")]
#[test]
fn a_client_that_never_reads_with_a_full_pipe_holds_up_no_one() {
    let mut serve = Serve::start();
    let mut stuck = TcpStream::connect(("127.0.0.1", serve.port)).unwrap();
    shrink_receive_buffer(&stuck, 4096);
    let ask = format!(
        "GET /api/live?token={} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
        serve.token, serve.port
    );
    stuck.write_all(ask.as_bytes()).unwrap();
    let mut other = serve.live(&[]);
    other.until(Duration::from_secs(5), |m| !m.is_empty());
    std::thread::sleep(Duration::from_millis(300));
    // A 200-byte session id (the parser's limit) and a 250-byte folder name: about 750 bytes per event.
    let sid = "s".repeat(200);
    let cwd = format!("\"cwd\":\"/tmp/{}\"", "d".repeat(250));
    let before = other.received;
    for round in 0..4 {
        let text: String = (0..2000)
            .map(|i| call_line(&sid, &format!("r{round}b{i}")).replace("\"cwd\":\"/tmp\"", &cwd))
            .collect();
        serve.append(&text);
        let last = (round + 1) * 2000;
        other.until(Duration::from_secs(5), |m| lives(m).contains(&last));
    }
    let offered = other.received - before;
    assert!(offered > 5 << 20, "{offered} bytes offered to every stream");
    serve.append(&call_line("s1", "late"));
    other.until(Duration::from_secs(2), |m| lives(m).contains(&8001));
    stuck.set_nonblocking(true).unwrap();
    let (mut got, mut chunk) = (0, [0u8; 65536]);
    while let Ok(n) = stuck.read(&mut chunk) {
        if n == 0 {
            break;
        }
        got += n;
    }
    assert!(got < 64 * 1024, "the stuck client holds {got} bytes");
    let (status, took, err) = serve.sigint();
    assert_eq!(status.code(), Some(0), "{err}");
    assert!(took < Duration::from_secs(10), "{took:?}");
    eprintln!("stuck client: {got} bytes received of {offered} offered; SIGINT exit in {took:?}");
}
