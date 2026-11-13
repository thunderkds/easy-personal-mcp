//! Loopback HTTP server: one guard layer (Host, Origin, token, CSP) in front of every route.
//! With `--static-dir` the built dashboard is served from the same origin by a fallback (`static_files.rs`) that sits
//! behind the same layer, so a page load is guarded (Host, Origin) and carries the CSP, and needs no token.
//! The behaviour is a port of `Guard` in `src/easy_personal/surfaces/api.py`; the goldens
//! `rust/parity/golden/api__guard__*.json` are the oracle. The token only ever leaves this process in the
//! printed URL fragment (or the window's URL, `app`): no response, error or log line carries it.
//! `start` is the one way a server is made: `run` (the `serve` subcommand) and the window (`window.rs`) both call it.

use crate::api::{self, Api};
use crate::live_feed::{LIVE_PATH, LiveFeed};
use crate::static_files;
use axum::Router;
use axum::extract::{Request, State};
use axum::http::header::{CONTENT_SECURITY_POLICY, CONTENT_TYPE, HOST, ORIGIN};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use std::io::{self, Write};
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// Same value as `TOKEN_HEADER` in the Python app (header names are case-insensitive).
const TOKEN_HEADER: HeaderName = HeaderName::from_static("x-dashboard-token");
/// Same string as `CSP` in the Python app. No `connect-src`/`font-src`: they must keep falling back to `default-src 'self'`.
const CSP: &str = "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'";

struct Guard {
    token: String,
    host: String,
    origin: String,
    /// Set once a request to `/api/` passed the guard with the token (read by `app --self-test`, nothing else).
    authenticated: Arc<AtomicBool>,
}

/// The router with the guard around everything, the fallback (404) and method errors (405) included.
#[cfg(test)]
pub(crate) fn router(token: String, port: u16, api: Api) -> Router {
    router_with_static(token, port, api, None, Arc::default())
}

/// `static_root` is the canonical `--static-dir`; with `None` the fallback is the empty 404 of every other path.
/// `authenticated` is set when a request to `/api/` passes the guard.
pub(crate) fn router_with_static(
    token: String,
    port: u16,
    api: Api,
    static_root: Option<PathBuf>,
    authenticated: Arc<AtomicBool>,
) -> Router {
    let host = format!("127.0.0.1:{port}");
    let guard = Arc::new(Guard {
        origin: format!("http://{host}"),
        host,
        token,
        authenticated,
    });
    let app = Router::new()
        .route("/api/version", get(version))
        .merge(api::routes().with_state(api));
    let app = match static_root {
        Some(root) => {
            let root = Arc::new(root);
            app.fallback(move |req: Request| static_files::serve(Arc::clone(&root), req))
        }
        None => app,
    };
    app.layer(middleware::from_fn_with_state(guard, guard_layer))
}

async fn version() -> Response {
    json(
        StatusCode::OK,
        concat!(r#"{"version":""#, env!("CARGO_PKG_VERSION"), r#""}"#),
    )
}

fn json(status: StatusCode, body: &'static str) -> Response {
    (status, [(CONTENT_TYPE, "application/json")], body).into_response()
}

/// Host must be exactly one header equal to `127.0.0.1:<port>`; an Origin, if sent, must equal
/// `http://127.0.0.1:<port>`; `/api/` needs exactly one token header equal to the token.
/// Stricter than Python in one way: a repeated Host/token header is refused (Python keeps the last one).
/// One carve-out (T056, Python's `_query_token`): a `GET` of exactly `/api/live` with no token header may send the
/// token as the one `token` query parameter instead, because a browser `EventSource` cannot set headers. The value is
/// compared as sent (no percent-decoding: the token is URL-safe base64) and never echoed or logged.
fn allowed(guard: &Guard, method: &Method, uri: &Uri, headers: &HeaderMap) -> bool {
    let path = uri.path();
    let mut hosts = headers.get_all(HOST).iter();
    match (hosts.next(), hosts.next()) {
        (Some(h), None) if h.as_bytes() == guard.host.as_bytes() => {}
        _ => return false,
    }
    if !headers
        .get_all(ORIGIN)
        .iter()
        .all(|o| o.as_bytes() == guard.origin.as_bytes())
    {
        return false;
    }
    if path.starts_with("/api/") {
        let mut tokens = headers.get_all(TOKEN_HEADER).iter();
        return match (tokens.next(), tokens.next()) {
            (Some(t), None) => constant_time_eq(t.as_bytes(), guard.token.as_bytes()),
            (None, _) if method == Method::GET && path == LIVE_PATH => query_token(uri.query())
                .is_some_and(|t| constant_time_eq(t.as_bytes(), guard.token.as_bytes())),
            _ => false,
        };
    }
    true
}

/// The value of the one `token=` pair of the query; none, or more than one, is `None`.
fn query_token(query: Option<&str>) -> Option<&str> {
    let mut found = query?
        .split('&')
        .filter_map(|pair| pair.strip_prefix("token="));
    match (found.next(), found.next()) {
        (Some(token), None) => Some(token),
        _ => None,
    }
}

async fn guard_layer(State(guard): State<Arc<Guard>>, req: Request, next: Next) -> Response {
    let mut response = if allowed(&guard, req.method(), req.uri(), req.headers()) {
        if req.uri().path().starts_with("/api/") {
            guard.authenticated.store(true, Ordering::Relaxed);
        }
        next.run(req).await
    } else {
        json(StatusCode::FORBIDDEN, r#"{"error":"forbidden"}"#)
    };
    response
        .headers_mut()
        .insert(CONTENT_SECURITY_POLICY, HeaderValue::from_static(CSP));
    response
}

/// Compares every byte once a length matches (the length of the token is not secret).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

/// 32 bytes from the OS random source as URL-safe base64 without padding (like `secrets.token_urlsafe(32)`).
fn new_token() -> Result<String, getrandom::Error> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)?;
    Ok(base64url(&bytes))
}

fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..=chunk.len() {
            out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
    }
    out
}

/// A started server: the free port, the per-run token and the way to stop it. The server runs on its own runtime.
pub struct Running {
    pub port: u16,
    pub token: String,
    /// Set when a request to `/api/` has passed the guard with the token (the self-test of `app` waits for it).
    #[cfg_attr(not(feature = "window"), allow(dead_code))]
    pub authenticated: Arc<AtomicBool>,
    runtime: Runtime,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<Result<(), String>>,
    /// Ends every open `GET /api/live` stream at its next tick (set first by `stop`).
    stopping: Arc<AtomicBool>,
    /// The live feed (`serve` and `app`); stopped and joined last by `stop`.
    feed: Option<LiveFeed>,
}

/// How long `stop` waits for requests in flight before it drops them.
const STOP_GRACE: Duration = Duration::from_secs(1);

impl Running {
    /// The server's runtime, for the window's signal watch (`window.rs`).
    #[cfg_attr(not(feature = "window"), allow(dead_code))]
    pub fn handle(&self) -> tokio::runtime::Handle {
        self.runtime.handle().clone()
    }

    /// Ends the open live streams, stops accepting, gives requests in flight `STOP_GRACE` to finish, drops whatever
    /// is left, then stops and joins the live feed (at most 10 s, `LiveFeed::stop`).
    pub fn stop(mut self) {
        self.stopping.store(true, Ordering::Relaxed);
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        let Running { runtime, task, .. } = &mut self;
        let _ = runtime.block_on(async { tokio::time::timeout(STOP_GRACE, task).await });
        self.runtime.shutdown_timeout(STOP_GRACE);
        if let Some(feed) = self.feed.take() {
            feed.stop();
        }
    }
}

/// Binds `127.0.0.1` on a free port, makes the token and starts serving. `static_dir` is resolved once, before
/// anything is bound. `feed` (the live feed of `serve` and `app`, already polling) is what `GET /api/live` reads;
/// with `None` that route sends the empty snapshot, as Python does with no tail running.
pub fn start(static_dir: Option<&Path>, feed: Option<LiveFeed>) -> Result<Running, String> {
    let static_root = static_dir.map(static_files::canonical_root).transpose()?;
    let token = new_token().map_err(|e| format!("no OS random source: {e}"))?;
    let listener = std::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .map_err(|e| format!("cannot bind 127.0.0.1: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let runtime = Runtime::new().map_err(|e| format!("cannot start the runtime: {e}"))?;
    let authenticated = Arc::new(AtomicBool::new(false));
    let stopping = Arc::new(AtomicBool::new(false));
    let api = Api::system().with_live(feed.as_ref().map(LiveFeed::tail), Arc::clone(&stopping));
    let app = router_with_static(
        token.clone(),
        port,
        api,
        static_root,
        Arc::clone(&authenticated),
    );
    let (shutdown, stopped) = oneshot::channel::<()>();
    let listener = {
        let _in_runtime = runtime.enter();
        tokio::net::TcpListener::from_std(listener).map_err(|e| e.to_string())?
    };
    let task = runtime.spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = stopped.await;
            })
            .await
            .map_err(|e| format!("server stopped: {e}"))
    });
    Ok(Running {
        port,
        token,
        authenticated,
        runtime,
        shutdown: Some(shutdown),
        task,
        stopping,
        feed,
    })
}

/// The `serve` subcommand: the live feed, `start`, print the two lines Python's `serve()` prints, run until Ctrl-C,
/// then `stop` (open streams end, the feed's thread is joined).
pub fn run(static_dir: Option<&Path>) -> Result<(), String> {
    let mut running = start(static_dir, Some(LiveFeed::system()?))?;
    let mut out = io::stdout();
    write!(
        out,
        "dashboard: http://127.0.0.1:{}/#{}\nCtrl-C to stop.\n",
        running.port, running.token
    )
    .and_then(|()| out.flush())
    .map_err(|e| e.to_string())?;
    let Running { runtime, task, .. } = &mut running;
    let served = runtime.block_on(async {
        tokio::select! {
            served = task => Some(served),
            _ = tokio::signal::ctrl_c() => None,
        }
    });
    match served {
        None => {
            running.stop();
            Ok(())
        }
        Some(served) => served.map_err(|e| format!("server stopped: {e}"))?,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64url_matches_the_rfc_4648_vectors_without_padding() {
        for (raw, want) in [
            ("", ""),
            ("f", "Zg"),
            ("fo", "Zm8"),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg"),
            ("fooba", "Zm9vYmE"),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64url(raw.as_bytes()), want);
        }
        assert_eq!(base64url(&[0xfb, 0xff, 0xfe]), "-__-", "URL-safe alphabet");
    }

    #[test]
    fn a_token_is_43_url_safe_chars_and_differs_per_call() {
        let (a, b) = (new_token().unwrap(), new_token().unwrap());
        assert_eq!(a.len(), 43);
        assert!(
            a.bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        );
        assert_ne!(a, b);
    }

    /// One request over a raw `TcpStream` with `Connection: close`; returns the status line's code.
    fn status(running: &Running, path: &str, token: Option<&str>) -> u16 {
        use std::io::Read;
        let mut req = format!(
            "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n",
            running.port
        );
        if let Some(token) = token {
            req += &format!("x-dashboard-token: {token}\r\n");
        }
        req += "Connection: close\r\n\r\n";
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", running.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.write_all(req.as_bytes()).unwrap();
        let mut answer = String::new();
        let _ = stream.read_to_string(&mut answer);
        answer.split(' ').nth(1).unwrap().parse().unwrap()
    }

    #[test]
    fn start_binds_loopback_and_marks_only_a_guarded_api_request_with_the_token() {
        let running = start(None, None).unwrap();
        assert_eq!(running.token.len(), 43);
        assert!(!running.authenticated.load(Ordering::Relaxed));
        // Refused or outside `/api/`: the flag stays off.
        assert_eq!(status(&running, "/api/version", None), 403);
        assert_eq!(status(&running, "/api/version", Some("wrong")), 403);
        assert_eq!(status(&running, "/", None), 404);
        assert!(!running.authenticated.load(Ordering::Relaxed));
        let token = running.token.clone();
        assert_eq!(status(&running, "/api/version", Some(&token)), 200);
        assert!(running.authenticated.load(Ordering::Relaxed));
        running.stop();
    }

    #[test]
    fn two_starts_get_two_ports_and_two_tokens() {
        let (a, b) = (start(None, None).unwrap(), start(None, None).unwrap());
        assert_ne!(a.port, b.port);
        assert_ne!(a.token, b.token);
        a.stop();
        b.stop();
    }

    #[test]
    fn start_refuses_a_static_dir_that_is_not_a_folder_before_it_binds() {
        let err = start(Some(Path::new("/this/folder/does/not/exist")), None)
            .err()
            .unwrap();
        assert_eq!(err, "--static-dir is not a readable directory");
    }

    #[test]
    fn stop_returns_in_bounded_time_with_a_request_stuck_in_flight_and_frees_the_port() {
        let running = start(None, None).unwrap();
        let port = running.port;
        // A connection that sent half a request line and then waits: graceful shutdown alone would never end.
        let mut stuck = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        stuck.write_all(b"GET /api/vers").unwrap();
        std::thread::sleep(Duration::from_millis(100));
        let began = std::time::Instant::now();
        running.stop();
        assert!(
            began.elapsed() < Duration::from_secs(4),
            "{:?}",
            began.elapsed()
        );
        assert!(
            std::net::TcpStream::connect(("127.0.0.1", port)).is_err(),
            "the listener is closed"
        );
    }

    /// T056: a stream whose client went away is dropped (its hold on the tail ends) at the next write, and `stop`
    /// ends the streams still open and joins the feed.
    #[test]
    fn a_disconnected_stream_ends_and_stop_joins_the_feed() {
        use easy_personal_core::live::Root;
        use easy_personal_core::prices::PriceTable;
        let dir =
            std::env::temp_dir().join(format!("easy-personal-rs-t056-srv-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("p")).unwrap();
        let roots = vec![Root {
            account: None,
            projects: dir.clone(),
        }];
        let feed = LiveFeed::start(
            PriceTable::shipped().unwrap(),
            easy_personal_core::clock::now_micros,
            move |tail, now| tail.poll_once(&roots, now, None),
        )
        .unwrap();
        let running = start(None, Some(feed)).unwrap();
        let tail = running.feed.as_ref().unwrap().tail();
        let baseline = Arc::strong_count(&tail);
        let open = || {
            let mut s = std::net::TcpStream::connect(("127.0.0.1", running.port)).unwrap();
            let req = format!(
                "GET /api/live?token={} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
                running.token, running.port
            );
            s.write_all(req.as_bytes()).unwrap();
            std::thread::sleep(Duration::from_millis(300));
            s
        };
        let client = open();
        // The connection's router copy and the stream hold the tail (axum clones the state per connection).
        assert!(
            Arc::strong_count(&tail) > baseline,
            "the stream holds the tail"
        );
        drop(client);
        // A write after the close: hyper sees the dead socket and drops the body.
        std::fs::write(
            dir.join("p/s.jsonl"),
            "{\"type\":\"user\",\"sessionId\":\"s\",\"timestamp\":\"2026-01-20T00:00:00Z\",\"message\":{\"content\":[{\"type\":\"tool_result\",\"tool_use_id\":\"b\"}]}}\n",
        )
        .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while Arc::strong_count(&tail) > baseline {
            assert!(
                std::time::Instant::now() < deadline,
                "the stream was not dropped"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        let _still_open = open();
        let began = std::time::Instant::now();
        running.stop();
        assert!(
            began.elapsed() < Duration::from_secs(3),
            "{:?}",
            began.elapsed()
        );
        assert_eq!(
            Arc::strong_count(&tail),
            1,
            "streams dropped, feed thread joined"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// T056: the query `token` counts only for `GET /api/live` with no token header; Host and Origin still apply.
    #[test]
    fn the_query_token_counts_only_for_get_on_the_live_route() {
        let guard = Guard {
            token: "tok".into(),
            host: "127.0.0.1:9".into(),
            origin: "http://127.0.0.1:9".into(),
            authenticated: Arc::default(),
        };
        let headers = |pairs: &[(&'static str, &'static str)]| {
            let mut h = HeaderMap::new();
            h.insert(HOST, HeaderValue::from_static("127.0.0.1:9"));
            for (k, v) in pairs {
                h.append(*k, HeaderValue::from_static(v));
            }
            h
        };
        let ok = |method: Method, uri: &str, h: &HeaderMap| {
            let uri: axum::http::Uri = uri.parse().unwrap();
            allowed(&guard, &method, &uri, h)
        };
        let plain = headers(&[]);
        assert!(ok(Method::GET, "/api/live?token=tok", &plain));
        assert!(ok(Method::GET, "/api/live?a=1&token=tok&b=2", &plain));
        assert!(ok(
            Method::GET,
            "/api/live",
            &headers(&[("x-dashboard-token", "tok")])
        ));
        for (method, uri) in [
            (Method::GET, "/api/live"),
            (Method::GET, "/api/live?token=bad"),
            (Method::GET, "/api/live?token="),
            (Method::GET, "/api/live?token"),
            (Method::GET, "/api/live?token=tok&token=tok"),
            (Method::GET, "/api/live?TOKEN=tok"),
            (Method::GET, "/api/live?token=to%6B"),
            (Method::GET, "/api/live/?token=tok"),
            (Method::GET, "/api/live/x?token=tok"),
            (Method::GET, "/api/version?token=tok"),
            (Method::GET, "/api/usage/summary?token=tok"),
            (Method::HEAD, "/api/live?token=tok"),
            (Method::POST, "/api/live?token=tok"),
        ] {
            assert!(!ok(method.clone(), uri, &plain), "{method} {uri}");
        }
        // A token header present decides alone, as on every other route.
        assert!(!ok(
            Method::GET,
            "/api/live?token=tok",
            &headers(&[("x-dashboard-token", "bad")])
        ));
        assert!(!ok(
            Method::GET,
            "/api/live?token=tok",
            &headers(&[("x-dashboard-token", "")])
        ));
        // Host and Origin still apply.
        assert!(!ok(
            Method::GET,
            "/api/live?token=tok",
            &headers(&[("origin", "http://evil.test")])
        ));
        let mut bad_host = HeaderMap::new();
        bad_host.insert(HOST, HeaderValue::from_static("evil.test"));
        assert!(!ok(Method::GET, "/api/live?token=tok", &bad_host));
    }

    #[test]
    fn constant_time_eq_is_plain_equality() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(!constant_time_eq(b"", b"a"));
        assert!(constant_time_eq(b"", b""));
    }
}
