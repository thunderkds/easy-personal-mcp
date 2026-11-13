//! `mcp --transport http` (T051): the MCP tools over streamable HTTP on `127.0.0.1`, path `/mcp`, no token (as Python's
//! `FastMCP`). The sockets, the guard and the HTTP rules are here; the protocol is `mcp::http_exchange`.
//!
//! - Only `127.0.0.1` is ever bound (`mcp::main` refuses another host before this runs).
//! - The guard is Python's DNS-rebinding check: the `Host` header must be `127.0.0.1:<port>` or `localhost:<port>` (421
//!   otherwise) and an `Origin`, when sent, must be `http://127.0.0.1` or `http://localhost` with any port (403).
//! - `POST /mcp` takes one JSON-RPC message (`Content-Type: application/json`, at most 1 MiB) and answers with
//!   `application/json` (200), or 202 with no body for a notification. `GET` and `DELETE` are 405: there are no
//!   sessions and no server-initiated stream. Python's FastMCP answers with an SSE body; both are allowed by the spec.
//! - Ctrl-C stops the server, exit 0: open requests get 3 s to finish, then the connections are dropped (a client stuck
//!   mid-request cannot keep the process alive). A port in use is one line, exit 1, with no path.
//! - A connection has 10 s to send a whole request (header and body) after it opens or after the last answer; an
//!   idle or stalled one is closed. At most 64 connections are open at once: the next is closed at once. The clock
//!   stands still while a tool exchange is running (a tool may take longer than 10 s).

use crate::mcp::{self, Exchange};
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::{Request, State};
use axum::http::header::{ACCEPT, ALLOW, CONTENT_TYPE, HOST, ORIGIN};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::Response;
use serde_json::{Value, json};
use std::future::Future;
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, oneshot};
use tokio::time::{Instant, Sleep};

/// The largest request body.
const MAX_BODY: usize = 1 << 20;
const PATH: &str = "/mcp";
/// A connection must send its whole request within this long of opening or of the last answer.
const READ_LIMIT: Duration = Duration::from_secs(10);
/// The most connections open at once.
const MAX_CONNECTIONS: usize = 64;
/// How long open requests get to finish after Ctrl-C.
const STOP_GRACE: Duration = Duration::from_secs(3);
/// Tool exchanges running now: while there is one, no connection times out (a tool may take longer than the limit).
static EXCHANGES: AtomicUsize = AtomicUsize::new(0);

/// Counts one running exchange.
struct Running;

impl Running {
    fn start() -> Running {
        EXCHANGES.fetch_add(1, Ordering::SeqCst);
        Running
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        EXCHANGES.fetch_sub(1, Ordering::SeqCst);
    }
}

/// An accepted connection: reads fail with `TimedOut` once [`READ_LIMIT`] passed since it opened or since the last write
/// (an answer), unless an exchange is running. Holds one of the [`MAX_CONNECTIONS`] permits until it is dropped.
struct Guarded {
    io: TcpStream,
    deadline: Pin<Box<Sleep>>,
    _permit: OwnedSemaphorePermit,
}

impl AsyncRead for Guarded {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = &mut *self;
        match Pin::new(&mut this.io).poll_read(cx, buf) {
            Poll::Ready(result) => Poll::Ready(result),
            Poll::Pending => loop {
                if this.deadline.as_mut().poll(cx).is_pending() {
                    return Poll::Pending;
                }
                if EXCHANGES.load(Ordering::SeqCst) == 0 {
                    return Poll::Ready(Err(io::ErrorKind::TimedOut.into()));
                }
                this.deadline.as_mut().reset(Instant::now() + READ_LIMIT);
            },
        }
    }
}

impl AsyncWrite for Guarded {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = &mut *self;
        let done = Pin::new(&mut this.io).poll_write(cx, buf);
        if done.is_ready() {
            this.deadline.as_mut().reset(Instant::now() + READ_LIMIT);
        }
        done
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.io).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.io).poll_shutdown(cx)
    }
}

/// The listener `axum::serve` runs: it hands out [`Guarded`] connections and closes at once any over the cap.
struct Capped {
    listener: TcpListener,
    permits: Arc<Semaphore>,
}

impl axum::serve::Listener for Capped {
    type Io = Guarded;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Guarded, SocketAddr) {
        loop {
            let (io, addr) = match self.listener.accept().await {
                Ok(pair) => pair,
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(50)).await; // e.g. out of file descriptors: try again
                    continue;
                }
            };
            match Arc::clone(&self.permits).try_acquire_owned() {
                Ok(permit) => {
                    let deadline = Box::pin(tokio::time::sleep(READ_LIMIT));
                    return (
                        Guarded {
                            io,
                            deadline,
                            _permit: permit,
                        },
                        addr,
                    );
                }
                Err(_) => drop(io), // over the cap: closed at once
            }
        }
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }
}

pub fn run(port: u16) -> Result<(), String> {
    let listener = std::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))
        .map_err(|e| format!("cannot bind 127.0.0.1:{port}: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let runtime =
        tokio::runtime::Runtime::new().map_err(|e| format!("cannot start the runtime: {e}"))?;
    let listener = {
        let _in_runtime = runtime.enter();
        TcpListener::from_std(listener).map_err(|e| e.to_string())?
    };
    let capped = Capped {
        listener,
        permits: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
    };
    let (stop, stopped) = oneshot::channel::<()>();
    let task = runtime.spawn(
        axum::serve(capped, router(port))
            .with_graceful_shutdown(async move {
                let _ = stopped.await;
            })
            .into_future(),
    );
    eprintln!("MCP over HTTP: http://127.0.0.1:{port}{PATH}\nCtrl-C to stop.");
    let outcome = runtime.block_on(async {
        let mut task = task;
        tokio::select! {
            ended = &mut task => Some(ended),
            _ = tokio::signal::ctrl_c() => {
                let _ = stop.send(());
                // Open requests get STOP_GRACE; a client stuck mid-request must not keep the process alive.
                tokio::time::timeout(STOP_GRACE, task).await.ok()
            }
        }
    });
    runtime.shutdown_timeout(Duration::from_secs(1));
    match outcome {
        Some(Ok(Err(e))) => Err(format!("server stopped: {e}")),
        _ => Ok(()),
    }
}

fn router(port: u16) -> Router {
    Router::new().fallback(handle).with_state(port)
}

fn reply(status: StatusCode, body: &Value) -> Response {
    let mut response = Response::new(Body::from(body.to_string()));
    *response.status_mut() = status;
    response.headers_mut().insert(
        CONTENT_TYPE,
        "application/json".parse().expect("a fixed value"),
    );
    response
}

/// A JSON-RPC error body (`id` null when the request had none readable).
fn error(status: StatusCode, code: i64, text: &str, id: &Value) -> Response {
    reply(
        status,
        &json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": text}}),
    )
}

fn plain(status: StatusCode, text: &'static str) -> Response {
    let mut response = Response::new(Body::from(text));
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, "text/plain".parse().expect("a fixed value"));
    response
}

fn header<'a>(headers: &'a HeaderMap, name: &axum::http::HeaderName) -> Option<&'a str> {
    headers.get(name).and_then(|v| v.to_str().ok())
}

/// Every `Host` value is this server's own name and port (and there is one at least): `Host: good, Host: evil` fails.
fn host_ok(headers: &HeaderMap, port: u16) -> bool {
    let mut values = headers.get_all(HOST).iter().peekable();
    values.peek().is_some()
        && values.all(|v| {
            v.to_str().is_ok_and(|h| {
                h.eq_ignore_ascii_case(&format!("127.0.0.1:{port}"))
                    || h.eq_ignore_ascii_case(&format!("localhost:{port}"))
            })
        })
}

/// No `Origin`, or only loopback ones (`http://127.0.0.1[:port]`, `http://localhost[:port]`): every value is checked.
fn origin_ok(headers: &HeaderMap) -> bool {
    headers.get_all(ORIGIN).iter().all(|value| {
        let Some(rest) = value.to_str().ok().and_then(|o| o.strip_prefix("http://")) else {
            return false;
        };
        let (host, port) = rest.split_once(':').unwrap_or((rest, ""));
        (host.eq_ignore_ascii_case("127.0.0.1") || host.eq_ignore_ascii_case("localhost"))
            && port.bytes().all(|b| b.is_ascii_digit())
    })
}

async fn handle(State(port): State<u16>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    if !host_ok(&parts.headers, port) {
        return plain(StatusCode::MISDIRECTED_REQUEST, "Invalid Host header");
    }
    if !origin_ok(&parts.headers) {
        return plain(StatusCode::FORBIDDEN, "Invalid Origin header");
    }
    if parts.uri.path() != PATH {
        return plain(StatusCode::NOT_FOUND, "Not Found");
    }
    if parts.method != Method::POST {
        let mut response = plain(StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed");
        response
            .headers_mut()
            .insert(ALLOW, "POST".parse().expect("a fixed value"));
        return response;
    }
    let json_type = header(&parts.headers, &CONTENT_TYPE).is_some_and(|t| {
        t.trim_start()
            .to_ascii_lowercase()
            .starts_with("application/json")
    });
    if !json_type {
        return plain(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Content-Type must be application/json",
        );
    }
    let accepts = header(&parts.headers, &ACCEPT)
        .is_some_and(|a| a.contains("application/json") || a.contains("*/*"));
    if !accepts {
        return plain(
            StatusCode::NOT_ACCEPTABLE,
            "Accept must include application/json",
        );
    }
    let Ok(bytes) = to_bytes(body, MAX_BODY).await else {
        return plain(StatusCode::PAYLOAD_TOO_LARGE, "Request body too large");
    };
    let Ok(message) = serde_json::from_slice::<Value>(&bytes) else {
        return error(StatusCode::BAD_REQUEST, -32700, "Parse error", &Value::Null);
    };
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    let _running = Running::start();
    match mcp::http_exchange(message).await {
        Exchange::Reply(answer) => reply(StatusCode::OK, &answer),
        Exchange::Accepted => {
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::ACCEPTED;
            response
        }
        Exchange::Invalid => error(StatusCode::BAD_REQUEST, -32600, "Invalid request", &id),
        Exchange::Failed => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            -32603,
            "Internal error",
            &id,
        ),
    }
}
