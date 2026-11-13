//! The `context` source (P3, T051): Python `sources/context.py`. easy-context's index freshness, read through its
//! MCP service on `127.0.0.1` (DDR-0002: a read-only client, loopback only).
//!
//! - The client can send only the tools of [`ALLOWED_TOOLS`]; any other name is refused before a connection.
//! - The URL must be `http://127.0.0.1[:port][/path]`: another scheme or host, user info, a port above 65535 or 0,
//!   an IPv6 literal, a name (also `localhost`) is refused before a connection. Redirects are not followed.
//! - Every failure (service down, slow, a bad status, an unreadable or oversize reply, a JSON-RPC error) is "no data":
//!   [`ingest`] clears the table and reports `no data`; it never fails the run.
//! - Only a project's `root`, `last_index_at` and `root_exists` are read; no index content is kept or printed.
//!
//! The client is a small HTTP/1.1 client over `std::net::TcpStream` (no crate: the T051 dependency rule). One call is
//! `initialize`, `notifications/initialized`, `tools/call` and a best-effort `DELETE` of the session, each one POST
//! with `Connection: close`. A reply is JSON or an SSE stream (the streamable-HTTP transport allows both); the body
//! may be plain, `Content-Length` or chunked. A reply body over [`MAX_REPLY`] (1 MiB; Python has no cap) is "no data".
//! Time: each socket operation waits at most the timeout (3 s) and the whole call at most twice that (`wait_for`).

use super::time::epoch_micros_naive_is_utc;
use crate::writer::{Store, WriteError};
use rusqlite::{Transaction, TransactionBehavior, params};
use serde_json::Value;
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

/// `ALLOWED_TOOLS`: the only tool names the client sends.
pub const ALLOWED_TOOLS: [&str; 2] = ["list_indexes", "get_stats"];
pub const URL_ENV: &str = "EASY_PERSONAL_CONTEXT_URL";
pub const DEFAULT_URL: &str = "http://127.0.0.1:8767/mcp";
/// `TIMEOUT_S`.
pub const TIMEOUT: Duration = Duration::from_secs(3);
/// `ROOT_TRIES`: how many existing project folders are tried as the root header.
pub const ROOT_TRIES: usize = 3;
/// The largest reply body read (T051 decision; Python has no limit).
pub const MAX_REPLY: usize = 1 << 20;
/// The largest reply header block read.
const MAX_HEAD: usize = 16 << 10;
/// The raw reply (framing included) is cut off here: a body of [`MAX_REPLY`] fits with its chunk framing.
const MAX_RAW: usize = 2 * MAX_REPLY;
const PROTOCOL: &str = "2025-06-18";

/// Why a call gave nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallError {
    /// Python's `Refused`: the tool is not allowed or the URL is not loopback `http`; nothing was sent.
    Refused,
    /// Python's `ContextUnavailable`: down, slow, a bad status, an unreadable, oversize or error reply.
    Unavailable,
    /// Python's `ToolError`: the service answered, but the tool call failed (for example a root outside its workspace).
    Tool,
}

/// What a run did: `ok` with the number of stored indexes, or `no data`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub status: &'static str,
    pub indexes: i64,
}

// ---- the URL ----

/// Where a checked URL points: `127.0.0.1:<port>` and the request target (path and query, never empty).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Target {
    port: u16,
    host_header: String,
    path: String,
}

/// `check_url`. Stricter than Python in one way: a URL with a space, a control or a non-ASCII character, or any `@`,
/// is refused (Python percent-encodes the first two and lets `http://@127.0.0.1/` through).
fn check_url(url: &str) -> Result<Target, CallError> {
    if url.bytes().any(|b| b <= 0x20 || b >= 0x7f) {
        return Err(CallError::Refused);
    }
    let (scheme, rest) = url.split_once("://").ok_or(CallError::Refused)?;
    if !scheme.eq_ignore_ascii_case("http") {
        return Err(CallError::Refused);
    }
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    if authority.contains('@') {
        return Err(CallError::Refused);
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, port),
        None => (authority, ""),
    };
    if host != "127.0.0.1" {
        return Err(CallError::Refused);
    }
    let port: u16 = if port.is_empty() {
        80
    } else if port.bytes().all(|b| b.is_ascii_digit()) {
        port.parse().map_err(|_| CallError::Refused)?
    } else {
        return Err(CallError::Refused);
    };
    if port == 0 {
        return Err(CallError::Refused);
    }
    let tail = tail.split('#').next().unwrap_or("");
    let path = match tail {
        "" => "/".to_string(),
        t if t.starts_with('?') => format!("/{t}"),
        t => t.to_string(),
    };
    let host_header = if port == 80 {
        "127.0.0.1".to_string()
    } else {
        format!("127.0.0.1:{port}")
    };
    Ok(Target {
        port,
        host_header,
        path,
    })
}

// ---- the HTTP exchange ----

struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// One call's clock: every socket operation is bounded by `step` and by what is left of the whole.
#[derive(Clone, Copy)]
struct Clock {
    step: Duration,
    end: Instant,
}

impl Clock {
    fn new(timeout: Duration) -> Clock {
        Clock {
            step: timeout,
            end: Instant::now() + timeout * 2,
        }
    }

    /// The wait for the next socket operation; `Unavailable` when the whole call has run out.
    fn wait(&self) -> Result<Duration, CallError> {
        let left = self.end.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(CallError::Unavailable);
        }
        Ok(left.min(self.step))
    }
}

/// The request line and headers plus the body, as bytes. `root` has been checked to be header-safe.
fn request(
    method: &str,
    target: &Target,
    root: Option<&str>,
    session: Option<&str>,
    protocol: Option<&str>,
    body: &str,
) -> Vec<u8> {
    let mut head = format!(
        "{method} {} HTTP/1.1\r\nHost: {}\r\nAccept: application/json, text/event-stream\r\nConnection: close\r\n",
        target.path, target.host_header
    );
    if method == "POST" {
        head += &format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            body.len()
        );
    }
    if let Some(root) = root {
        head += &format!("X-Easy-Context-Root: {root}\r\n");
    }
    if let Some(session) = session {
        head += &format!("mcp-session-id: {session}\r\n");
    }
    if let Some(protocol) = protocol {
        head += &format!("mcp-protocol-version: {protocol}\r\n");
    }
    head += "\r\n";
    let mut out = head.into_bytes();
    out.extend_from_slice(body.as_bytes());
    out
}

/// A header value is printable ASCII (or a tab): httpx encodes it as ASCII and h11 refuses the rest.
fn header_safe(value: &str) -> bool {
    value
        .bytes()
        .all(|b| b == b'\t' || (0x20..0x7f).contains(&b))
}

/// Sends one request and reads the reply. `want_id`: for an SSE reply, the read stops as soon as the JSON-RPC
/// response with that id is complete (the service may keep the stream open).
fn exchange(
    target: &Target,
    bytes: &[u8],
    clock: Clock,
    want_id: Option<i64>,
) -> Result<Reply, CallError> {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, target.port));
    let mut tcp =
        TcpStream::connect_timeout(&addr, clock.wait()?).map_err(|_| CallError::Unavailable)?;
    tcp.set_write_timeout(Some(clock.wait()?))
        .and_then(|()| tcp.write_all(bytes))
        .map_err(|_| CallError::Unavailable)?;
    let mut raw: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match parse_reply(&raw, false, want_id)? {
            Some(reply) => return Ok(reply),
            None if raw.len() > MAX_RAW => return Err(CallError::Unavailable),
            None => {}
        }
        tcp.set_read_timeout(Some(clock.wait()?))
            .map_err(|_| CallError::Unavailable)?;
        match tcp.read(&mut chunk) {
            Ok(0) => {
                // The peer closed: what has come is the reply (a body with no length ends here).
                return parse_reply(&raw, true, want_id)?.ok_or(CallError::Unavailable);
            }
            Ok(n) => raw.extend_from_slice(&chunk[..n]),
            Err(_) => return Err(CallError::Unavailable),
        }
    }
}

/// The reply in `raw`, when it is complete; `None` when more bytes are needed. `eof`: the peer has closed.
fn parse_reply(raw: &[u8], eof: bool, want_id: Option<i64>) -> Result<Option<Reply>, CallError> {
    let Some(split) = raw.windows(4).position(|w| w == b"\r\n\r\n") else {
        if raw.len() > MAX_HEAD || eof {
            return Err(CallError::Unavailable);
        }
        return Ok(None);
    };
    if split > MAX_HEAD {
        return Err(CallError::Unavailable);
    }
    let head = std::str::from_utf8(&raw[..split]).map_err(|_| CallError::Unavailable)?;
    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or("");
    let mut parts = status_line.splitn(3, ' ');
    let version = parts.next().unwrap_or("");
    let status: u16 = parts
        .next()
        .filter(|s| s.len() == 3 && s.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|s| s.parse().ok())
        .filter(|_| version.starts_with("HTTP/1."))
        .ok_or(CallError::Unavailable)?;
    let mut headers = Vec::new();
    for line in lines {
        let (k, v) = line.split_once(':').ok_or(CallError::Unavailable)?;
        headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
    }
    let find = |name: &str| {
        headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    };
    let body_raw = &raw[split + 4..];
    let (body, complete) =
        if find("transfer-encoding").is_some_and(|v| v.to_ascii_lowercase().contains("chunked")) {
            dechunk(body_raw)?
        } else if let Some(len) = find("content-length") {
            let len: usize = len.parse().map_err(|_| CallError::Unavailable)?;
            if len > MAX_REPLY {
                return Err(CallError::Unavailable);
            }
            (
                body_raw.get(..len).unwrap_or(body_raw).to_vec(),
                body_raw.len() >= len,
            )
        } else {
            (body_raw.to_vec(), eof)
        };
    if body.len() > MAX_REPLY {
        return Err(CallError::Unavailable);
    }
    let sse =
        find("content-type").is_some_and(|v| v.to_ascii_lowercase().contains("text/event-stream"));
    let early = sse && want_id.is_some_and(|id| sse_response(&body, id).is_some());
    if !complete && !early && !eof {
        return Ok(None);
    }
    Ok(Some(Reply {
        status,
        headers,
        body,
    }))
}

/// A chunked body: the data so far, and whether the last chunk (size 0) has come. A bad size is an error.
fn dechunk(mut raw: &[u8]) -> Result<(Vec<u8>, bool), CallError> {
    let mut out = Vec::new();
    loop {
        let Some(eol) = raw.windows(2).position(|w| w == b"\r\n") else {
            return Ok((out, false));
        };
        let size_text = std::str::from_utf8(&raw[..eol]).map_err(|_| CallError::Unavailable)?;
        let size_text = size_text.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_text, 16).map_err(|_| CallError::Unavailable)?;
        if size == 0 {
            return Ok((out, true));
        }
        if size > MAX_REPLY || out.len() + size > MAX_REPLY {
            return Err(CallError::Unavailable);
        }
        let data = &raw[eol + 2..];
        if data.len() < size + 2 {
            out.extend_from_slice(&data[..data.len().min(size)]); // the data that has come of an unfinished chunk
            return Ok((out, false));
        }
        out.extend_from_slice(&data[..size]);
        raw = &data[size + 2..];
    }
}

/// The JSON-RPC response with `id` in an SSE body: the `data:` lines of one event joined, parsed as JSON.
fn sse_response(body: &[u8], id: i64) -> Option<Value> {
    let text = String::from_utf8_lossy(body)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    text.split("\n\n").find_map(|event| {
        let data: Vec<&str> = event
            .split('\n')
            .filter_map(|l| l.strip_prefix("data:"))
            .map(|d| d.strip_prefix(' ').unwrap_or(d))
            .collect();
        if data.is_empty() {
            return None;
        }
        let value: Value = serde_json::from_str(&data.join("\n")).ok()?;
        response_with_id(value, id)
    })
}

/// `value` when it is the JSON-RPC response (a result or an error) for `id`; the match inside a batch.
fn response_with_id(value: Value, id: i64) -> Option<Value> {
    match value {
        Value::Array(items) => items.into_iter().find_map(|v| response_with_id(v, id)),
        v @ Value::Object(_) => {
            let ours = v["id"].as_i64() == Some(id);
            let answer = !v["result"].is_null() || !v["error"].is_null();
            (ours && answer).then_some(v)
        }
        _ => None,
    }
}

/// The JSON-RPC response with `id` from a reply, JSON or SSE.
fn rpc_result(reply: &Reply, id: i64) -> Result<Value, CallError> {
    if reply.status != 200 {
        return Err(CallError::Unavailable);
    }
    let kind = reply
        .header("content-type")
        .unwrap_or("")
        .to_ascii_lowercase();
    let found = if kind.contains("text/event-stream") {
        sse_response(&reply.body, id)
    } else if kind.contains("application/json") {
        serde_json::from_slice(&reply.body)
            .ok()
            .and_then(|v| response_with_id(v, id))
    } else {
        None
    };
    let message = found.ok_or(CallError::Unavailable)?;
    if !message["error"].is_null() {
        return Err(CallError::Unavailable); // McpError in Python
    }
    match &message["result"] {
        r @ Value::Object(_) => Ok(r.clone()),
        _ => Err(CallError::Unavailable),
    }
}

// ---- one call ----

/// `call_tool`: one allow-listed tool, decoded as Python's `_decode` does. `url`: the value of
/// `EASY_PERSONAL_CONTEXT_URL`, `None` or empty for the default.
pub fn call_tool(
    tool: &str,
    root: Option<&str>,
    url: Option<&OsStr>,
    timeout: Duration,
) -> Result<Value, CallError> {
    if !ALLOWED_TOOLS.contains(&tool) {
        return Err(CallError::Refused);
    }
    let url = match url.filter(|u| !u.is_empty()) {
        Some(u) => u.to_str().ok_or(CallError::Refused)?,
        None => DEFAULT_URL,
    };
    let target = check_url(url)?;
    if root.is_some_and(|r| !header_safe(r)) {
        return Err(CallError::Unavailable);
    }
    let clock = Clock::new(timeout);
    let init = format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{{\"protocolVersion\":\"{PROTOCOL}\",\"capabilities\":{{}},\"clientInfo\":{{\"name\":\"easy-personal-rs\",\"version\":\"0\"}}}}}}"
    );
    let first = exchange(
        &target,
        &request("POST", &target, root, None, None, &init),
        clock,
        Some(1),
    )?;
    let initialized = rpc_result(&first, 1)?;
    let session = first.header("mcp-session-id").map(str::to_string);
    if session
        .as_deref()
        .is_some_and(|s| !header_safe(s) || s.is_empty())
    {
        return Err(CallError::Unavailable);
    }
    let protocol = initialized["protocolVersion"]
        .as_str()
        .filter(|p| header_safe(p))
        .unwrap_or(PROTOCOL)
        .to_string();
    let outcome = (|| {
        let note = "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}";
        let ack = exchange(
            &target,
            &request(
                "POST",
                &target,
                root,
                session.as_deref(),
                Some(&protocol),
                note,
            ),
            clock,
            None,
        )?;
        if !(200..300).contains(&ack.status) {
            return Err(CallError::Unavailable);
        }
        let call = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{{\"name\":\"{tool}\",\"arguments\":{{}}}}}}"
        );
        let second = exchange(
            &target,
            &request(
                "POST",
                &target,
                root,
                session.as_deref(),
                Some(&protocol),
                &call,
            ),
            clock,
            Some(2),
        )?;
        decode(&rpc_result(&second, 2)?)
    })();
    if session.is_some() {
        // Like the Python client: end the session. Best effort, within what is left of the clock.
        let _ = exchange(
            &target,
            &request(
                "DELETE",
                &target,
                root,
                session.as_deref(),
                Some(&protocol),
                "",
            ),
            clock,
            None,
        );
    }
    outcome
}

/// `_decode`: the tool's `structuredContent`, else the JSON in the first text block; `{"result": x}` is `x`.
fn decode(result: &Value) -> Result<Value, CallError> {
    let Value::Array(content) = &result["content"] else {
        return Err(CallError::Unavailable); // the result schema needs `content`
    };
    match &result["isError"] {
        Value::Null | Value::Bool(false) => {}
        Value::Bool(true) => return Err(CallError::Tool),
        _ => return Err(CallError::Unavailable),
    }
    let data = match &result["structuredContent"] {
        Value::Null => {
            let text = content.first().and_then(|b| b["text"].as_str());
            serde_json::from_str(text.ok_or(CallError::Unavailable)?)
                .map_err(|_| CallError::Unavailable)?
        }
        v @ Value::Object(_) => v.clone(),
        _ => return Err(CallError::Unavailable),
    };
    if let Value::Object(map) = &data
        && map.len() == 1
        && let Some(inner) = map.get("result")
    {
        return Ok(inner.clone());
    }
    Ok(data)
}

// ---- the store step ----

/// `_epoch` then `_iso`: the stored text of one `last_index_at` value, `None` when it is unreadable, empty or out
/// of the years 1-9999 (Python raises there; the run does not fail here, the row keeps an empty time).
fn stored_time(value: &Value) -> Option<String> {
    let micros = match value {
        Value::Number(n) => float_to_micros(n.as_f64()?)?,
        Value::String(s) if !s.is_empty() => {
            let micros = epoch_micros_naive_is_utc(s.as_bytes())?;
            // `moment.timestamp()` is a float; `fromtimestamp` rounds it back to a microsecond.
            float_to_micros(micros as f64 / 1e6)?
        }
        _ => return None,
    };
    Some(iso(micros))
}

/// `datetime.fromtimestamp(epoch, UTC)`: split at the integer part, round the fraction half to even to a
/// microsecond; `None` outside the years 1-9999.
fn float_to_micros(epoch: f64) -> Option<i64> {
    if !epoch.is_finite() {
        return None;
    }
    let whole = epoch.trunc();
    let mut micros = (epoch - whole) * 1e6;
    micros = micros.round_ties_even();
    let mut secs = whole;
    if micros >= 1e6 {
        micros -= 1e6;
        secs += 1.0;
    } else if micros < 0.0 {
        micros += 1e6;
        secs -= 1.0;
    }
    const MIN: f64 = -62_135_596_800.0;
    const END: f64 = 253_402_300_800.0;
    (MIN..END)
        .contains(&secs)
        .then(|| secs as i64 * 1_000_000 + micros as i64)
}

/// `datetime.isoformat()` of an aware UTC time: `YYYY-MM-DDTHH:MM:SS[.ffffff]+00:00`.
fn iso(micros: i64) -> String {
    let stored = crate::metrics::format_stored(micros); // `YYYY-MM-DDTHH:MM:SS.ffffffZ`
    let (second, fraction) = stored[..stored.len() - 1].split_at(19);
    let fraction = if fraction == ".000000" { "" } else { fraction };
    format!("{second}{fraction}+00:00")
}

/// `ingest`: store `last_index_at` for each known project. A service problem is `no data`, never an error; only a
/// store failure is. `url`: `EASY_PERSONAL_CONTEXT_URL` as the process has it.
pub fn ingest(store: &mut Store, url: Option<&OsStr>) -> Result<Summary, WriteError> {
    ingest_with(store, url, TIMEOUT)
}

/// [`ingest`] with another timeout (the oracle's slow-service case runs Python with 0.3 s).
pub fn ingest_with(
    store: &mut Store,
    url: Option<&OsStr>,
    timeout: Duration,
) -> Result<Summary, WriteError> {
    let projects: Vec<String> = {
        let mut query = store.conn().prepare(
            "SELECT DISTINCT project_path FROM events WHERE source = 'claude_code' AND project_path IS NOT NULL ORDER BY 1",
        )?;
        query
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(Result::ok)
            .collect()
    };
    let mut reply = None;
    for root in projects
        .iter()
        .filter(|p| std::path::Path::new(p).is_dir())
        .take(ROOT_TRIES)
    {
        if !header_safe(root) {
            continue; // not sendable as a header value: this root only is skipped (Python's header encoder stops the whole run)
        }
        match call_tool("list_indexes", Some(root), url, timeout) {
            Ok(value) => {
                reply = Some(value);
                break;
            }
            Err(CallError::Tool) => continue,
            Err(_) => break,
        }
    }
    let listed = match reply.as_ref().map(|r| &r["indexes"]) {
        Some(Value::Array(items)) => Some(items),
        _ => None,
    };
    let tx = Transaction::new_unchecked(store.conn(), TransactionBehavior::Immediate)?;
    tx.execute("DELETE FROM context_index", [])?;
    let Some(listed) = listed else {
        tx.commit()?;
        return Ok(Summary {
            status: "no data",
            indexes: 0,
        });
    };
    let mut rows: Vec<(&String, Option<String>)> = Vec::new();
    for item in listed {
        if !item.is_object() {
            continue;
        }
        let Some(root) = item["root"]
            .as_str()
            .and_then(|r| projects.iter().find(|p| p.as_str() == r))
        else {
            continue;
        };
        if item["root_exists"] == Value::Bool(false) {
            continue; // index kept by easy-context for a deleted folder
        }
        rows.push((root, stored_time(&item["last_index_at"])));
    }
    {
        // Python's INSERT raises on a root listed twice and rolls back; here the later entry wins.
        let mut insert = tx.prepare(
            "INSERT OR REPLACE INTO context_index (project_path, last_index_at_utc) VALUES (?, ?)",
        )?;
        for (root, at) in &rows {
            insert.execute(params![root, at])?;
        }
    }
    let indexes = tx.query_row("SELECT count(*) FROM context_index", [], |r| r.get(0))?;
    tx.commit()?;
    Ok(Summary {
        status: "ok",
        indexes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reply_cap_is_one_mebibyte() {
        assert_eq!(MAX_REPLY, 1_048_576);
        let reply = |len: usize, sent: usize| {
            let mut raw = format!("HTTP/1.1 200 OK\r\ncontent-length: {len}\r\n\r\n").into_bytes();
            raw.extend(std::iter::repeat_n(b'x', sent));
            raw
        };
        let at_cap = parse_reply(&reply(1_048_576, 1_048_576), false, None);
        assert!(
            matches!(at_cap, Ok(Some(_))),
            "a body of exactly 1,048,576 bytes is read"
        );
        let over = parse_reply(&reply(1_048_577, 0), false, None);
        assert_eq!(
            over.err(),
            Some(CallError::Unavailable),
            "1,048,577 bytes is over the cap"
        );
        let chunked = format!(
            "HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n{:x}\r\n",
            1_048_577
        );
        assert_eq!(
            parse_reply(chunked.as_bytes(), false, None).err(),
            Some(CallError::Unavailable)
        );
    }

    #[test]
    fn only_loopback_http_urls_pass() {
        for url in [
            "http://127.0.0.1:8767/mcp",
            "http://127.0.0.1/mcp",
            "HTTP://127.0.0.1:1/",
            "http://127.0.0.1:65535",
            "http://127.0.0.1?x=1",
            "http://127.0.0.1:/mcp",
        ] {
            assert!(check_url(url).is_ok(), "{url}");
        }
        for url in [
            "http://example.com/mcp",
            "http://localhost:8767/mcp",
            "http://0.0.0.0:8767/mcp",
            "http://[::1]:8767/mcp",
            "https://127.0.0.1:8767/mcp",
            "http://127.0.0.1@evil.example/mcp",
            "http://evil.example@127.0.0.1:1/mcp",
            "http://@127.0.0.1/mcp",
            "http://127.0.0.1.evil.example/mcp",
            "http://127.0.0.1:0/mcp",
            "http://127.0.0.1:65536/mcp",
            "http://127.0.0.1:abc/mcp",
            "http://127.0.0.1:80:90/mcp",
            "http://127.0.0.1:8767/a b",
            "http://127.0.0.1:8767/a\r\nHost: x",
            "http://127.0.0.2/mcp",
            "file:///x",
            "127.0.0.1:8767",
            "",
        ] {
            assert_eq!(check_url(url), Err(CallError::Refused), "{url:?}");
        }
        let t = check_url("http://127.0.0.1:8767?x=1#frag").unwrap();
        assert_eq!((t.port, t.path.as_str()), (8767, "/?x=1"));
    }

    #[test]
    fn a_tool_outside_the_allow_list_is_refused_before_any_connection() {
        // No server listens: a refusal comes back before a connect, so it is `Refused`, never `Unavailable`.
        for tool in ["index_path", "search_semantic", "anything", ""] {
            let url = OsStr::new("http://127.0.0.1:9/mcp");
            assert_eq!(
                call_tool(tool, Some("/x"), Some(url), TIMEOUT),
                Err(CallError::Refused)
            );
        }
        for url in ["http://example.com/mcp", "http://localhost:8767/mcp"] {
            assert_eq!(
                call_tool("list_indexes", Some("/x"), Some(OsStr::new(url)), TIMEOUT),
                Err(CallError::Refused)
            );
        }
    }

    #[test]
    fn chunked_bodies_are_decoded_and_a_partial_one_waits() {
        assert_eq!(
            dechunk(b"3\r\nabc\r\n2\r\nde\r\n0\r\n\r\n").unwrap(),
            (b"abcde".to_vec(), true)
        );
        assert_eq!(
            dechunk(b"3\r\nabc\r\n2\r\nd").unwrap(),
            (b"abcd".to_vec(), false)
        );
        assert_eq!(
            dechunk(b"3;ext=1\r\nabc\r\n").unwrap(),
            (b"abc".to_vec(), false)
        );
        assert!(dechunk(b"zz\r\nabc").is_err());
        assert!(dechunk(format!("{:x}\r\n", MAX_REPLY + 1).as_bytes()).is_err());
    }

    #[test]
    fn an_sse_body_gives_the_response_with_our_id() {
        let body = b"event: message\r\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/x\"}\r\n\r\nevent: message\r\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\r\ndata: \"result\":{\"a\":1}}\r\n\r\n";
        let found = sse_response(body, 2).unwrap();
        assert!(found["result"].is_object());
        assert!(sse_response(body, 3).is_none());
    }

    #[test]
    fn a_reply_over_the_cap_is_unavailable() {
        let head = format!(
            "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n",
            MAX_REPLY + 1
        );
        assert_eq!(
            parse_reply(head.as_bytes(), false, None).err(),
            Some(CallError::Unavailable)
        );
        let mut open = b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n".to_vec();
        open.extend(std::iter::repeat_n(b'x', MAX_REPLY + 1));
        assert_eq!(
            parse_reply(&open, false, None).err(),
            Some(CallError::Unavailable)
        );
    }

    #[test]
    fn a_time_is_stored_the_way_pythons_isoformat_writes_it() {
        let text = |s: &str| stored_time(&Value::String(s.to_string()));
        assert_eq!(
            text("2026-10-21T10:00:00.000+00:00").as_deref(),
            Some("2026-10-21T10:00:00+00:00")
        );
        assert_eq!(
            text("2026-10-21T10:00:00").as_deref(),
            Some("2026-10-21T10:00:00+00:00")
        );
        assert_eq!(
            text("2026-10-21T12:00:00+02:00").as_deref(),
            Some("2026-10-21T10:00:00+00:00")
        );
        assert_eq!(
            text("2026-10-21T10:00:00.123456Z").as_deref(),
            Some("2026-10-21T10:00:00.123456+00:00")
        );
        assert_eq!(text("garbage"), None);
        assert_eq!(text(""), None);
        assert_eq!(
            stored_time(&serde_json::json!(1_760_000_000)).as_deref(),
            Some("2025-10-09T08:53:20+00:00")
        );
        assert_eq!(stored_time(&Value::Bool(true)), None);
        assert_eq!(stored_time(&serde_json::json!(i64::MAX)), None);
        assert_eq!(
            stored_time(&serde_json::json!(1_760_000_000.5)).as_deref(),
            Some("2025-10-09T08:53:20.500000+00:00")
        );
    }

    #[test]
    fn a_result_decodes_like_python() {
        let parse = |s: &str| serde_json::from_str::<Value>(s).unwrap();
        let ok = decode(&parse(
            r#"{"content":[],"structuredContent":{"indexes":[1]}}"#,
        ))
        .unwrap();
        assert!(ok["indexes"].is_array());
        let wrapped = decode(&parse(
            r#"{"content":[],"structuredContent":{"result":{"indexes":[]}}}"#,
        ))
        .unwrap();
        assert!(wrapped["indexes"].is_array());
        let text = decode(&parse(
            r#"{"content":[{"type":"text","text":"{\"indexes\":[]}"}]}"#,
        ))
        .unwrap();
        assert!(text["indexes"].is_array());
        assert_eq!(
            decode(&parse(r#"{"content":[],"isError":true}"#)),
            Err(CallError::Tool)
        );
        assert_eq!(
            decode(&parse(r#"{"content":[]}"#)),
            Err(CallError::Unavailable)
        );
        assert_eq!(
            decode(&parse(r#"{"content":[{"type":"text","text":"nope"}]}"#)),
            Err(CallError::Unavailable)
        );
        assert_eq!(
            decode(&parse(r#"{"isError":false}"#)),
            Err(CallError::Unavailable)
        );
    }
}
