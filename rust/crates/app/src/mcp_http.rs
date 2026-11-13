//! `mcp --transport http` (T051): the MCP tools over streamable HTTP on `127.0.0.1`, path `/mcp`, no token (as Python's
//! `FastMCP`). The sockets, the guard and the HTTP rules are here; the protocol is `mcp::http_exchange`.
//!
//! - Only `127.0.0.1` is ever bound (`mcp::main` refuses another host before this runs).
//! - The guard is Python's DNS-rebinding check: the `Host` header must be `127.0.0.1:<port>` or `localhost:<port>` (421
//!   otherwise) and an `Origin`, when sent, must be `http://127.0.0.1` or `http://localhost` with any port (403).
//! - `POST /mcp` takes one JSON-RPC message (`Content-Type: application/json`, at most 1 MiB) and answers with
//!   `application/json` (200), or 202 with no body for a notification. `GET` and `DELETE` are 405: there are no
//!   sessions and no server-initiated stream. Python's FastMCP answers with an SSE body; both are allowed by the spec.
//! - Ctrl-C stops the server, exit 0. A port in use is one line, exit 1, with no path.

use crate::mcp::{self, Exchange};
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::{Request, State};
use axum::http::header::{ACCEPT, ALLOW, CONTENT_TYPE, HOST, ORIGIN};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::Response;
use serde_json::{Value, json};
use std::net::{Ipv4Addr, SocketAddr};

/// The largest request body.
const MAX_BODY: usize = 1 << 20;
const PATH: &str = "/mcp";

pub fn run(port: u16) -> Result<(), String> {
    let listener = std::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))
        .map_err(|e| format!("cannot bind 127.0.0.1:{port}: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let runtime =
        tokio::runtime::Runtime::new().map_err(|e| format!("cannot start the runtime: {e}"))?;
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::from_std(listener).map_err(|e| e.to_string())?;
        eprintln!("MCP over HTTP: http://127.0.0.1:{port}{PATH}\nCtrl-C to stop.");
        axum::serve(listener, router(port))
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await
            .map_err(|e| format!("server stopped: {e}"))
    })
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

/// `Host` is this server's own name and port.
fn host_ok(headers: &HeaderMap, port: u16) -> bool {
    header(headers, &HOST).is_some_and(|h| {
        h.eq_ignore_ascii_case(&format!("127.0.0.1:{port}"))
            || h.eq_ignore_ascii_case(&format!("localhost:{port}"))
    })
}

/// No `Origin`, or a loopback one (`http://127.0.0.1[:port]`, `http://localhost[:port]`).
fn origin_ok(headers: &HeaderMap) -> bool {
    let Some(value) = headers.get(ORIGIN) else {
        return true;
    };
    let Some(rest) = value.to_str().ok().and_then(|o| o.strip_prefix("http://")) else {
        return false;
    };
    let (host, port) = rest.split_once(':').unwrap_or((rest, ""));
    (host.eq_ignore_ascii_case("127.0.0.1") || host.eq_ignore_ascii_case("localhost"))
        && port.bytes().all(|b| b.is_ascii_digit())
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
