//! A fake of easy-context's MCP service for T051 tests: a thread on a free `127.0.0.1` port that answers by a script
//! (the `server` of a case in `rust/parity/golden-context/golden.json`, as `Handler` in `rust/parity/gen_context.py`
//! answers it). Shared by `core/tests/context_parity.rs` and `app/tests/context_ingest.rs` (included by path).

#![allow(dead_code)]

use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// What the fake saw.
#[derive(Default)]
pub struct Seen {
    pub calls: Vec<String>,
    pub roots: Vec<Option<String>>,
    pub deletes: usize,
}

pub struct Fake {
    pub port: u16,
    pub seen: Arc<Mutex<Seen>>,
}

impl Fake {
    /// A service on a free port that answers by `script` (the golden's `server`, already expanded).
    pub fn start(script: Value) -> Fake {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(Seen::default()));
        let (script, shared) = (Arc::new(script), Arc::clone(&seen));
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (script, seen) = (Arc::clone(&script), Arc::clone(&shared));
                thread::spawn(move || {
                    let _ = serve_one(stream, &script, &seen);
                });
            }
        });
        Fake { port, seen }
    }
}

struct Request {
    method: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<Request> {
    let mut raw = Vec::new();
    let mut chunk = [0u8; 4096];
    let split = loop {
        if let Some(at) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
            break at;
        }
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        raw.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let mut lines = head.split("\r\n");
    let method = lines
        .next()
        .unwrap_or("")
        .split(' ')
        .next()
        .unwrap_or("")
        .to_string();
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    let len: usize = headers
        .iter()
        .find(|(k, _)| k == "content-length")
        .map_or(0, |(_, v)| v.parse().unwrap_or(0));
    let mut body = raw[split + 4..].to_vec();
    while body.len() < len {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    Ok(Request {
        method,
        headers,
        body,
    })
}

fn send(
    stream: &mut TcpStream,
    status: u16,
    ctype: Option<&str>,
    extra: &[(&str, &str)],
    body: &[u8],
    chunked: bool,
) -> std::io::Result<()> {
    let mut head = format!("HTTP/1.1 {status} X\r\n");
    for (k, v) in extra {
        head += &format!("{k}: {v}\r\n");
    }
    if let Some(ctype) = ctype {
        head += &format!("Content-Type: {ctype}\r\n");
    }
    if chunked {
        head += "Transfer-Encoding: chunked\r\n\r\n";
    } else {
        head += &format!("Content-Length: {}\r\n\r\n", body.len());
    }
    stream.write_all(head.as_bytes())?;
    if chunked {
        for part in body.chunks(37) {
            stream.write_all(format!("{:x}\r\n", part.len()).as_bytes())?;
            stream.write_all(part)?;
            stream.write_all(b"\r\n")?;
        }
        stream.write_all(b"0\r\n\r\n")?;
    } else {
        stream.write_all(body)?;
    }
    stream.flush()
}

fn message(
    stream: &mut TcpStream,
    script: &Value,
    payload: &Value,
    extra: &[(&str, &str)],
) -> std::io::Result<()> {
    let mode = script["mode"].as_str().unwrap();
    let text = payload.to_string();
    let (body, ctype) = if mode.starts_with("sse") {
        (
            format!("event: message\ndata: {text}\n\n"),
            "text/event-stream",
        )
    } else {
        (text, "application/json")
    };
    send(
        stream,
        200,
        Some(ctype),
        extra,
        body.as_bytes(),
        mode.ends_with("chunked"),
    )
}

/// One request, as `Handler` in `gen_context.py` answers it.
fn serve_one(mut stream: TcpStream, script: &Value, seen: &Mutex<Seen>) -> std::io::Result<()> {
    let request = read_request(&mut stream)?;
    match request.method.as_str() {
        "GET" => return send(&mut stream, 405, Some("text/plain"), &[], b"", false),
        "DELETE" => {
            seen.lock().unwrap().deletes += 1;
            return send(&mut stream, 200, Some("text/plain"), &[], b"", false);
        }
        _ => {}
    }
    let msg: Value = serde_json::from_slice(&request.body).unwrap_or(Value::Null);
    match script["break"].as_str() {
        Some("status500") => {
            return send(&mut stream, 500, Some("text/plain"), &[], b"boom", false);
        }
        Some("redirect") => {
            return send(
                &mut stream,
                302,
                None,
                &[("Location", "http://127.0.0.1:1/elsewhere")],
                b"",
                false,
            );
        }
        Some("garbage") => {
            return send(
                &mut stream,
                200,
                Some("application/json"),
                &[],
                b"{not json",
                false,
            );
        }
        Some("text") => return send(&mut stream, 200, Some("text/plain"), &[], b"hello", false),
        Some("empty") => return send(&mut stream, 200, Some("application/json"), &[], b"", false),
        _ => {}
    }
    let method = msg["method"].as_str().unwrap_or("");
    if script["session"] == true
        && method != "initialize"
        && request.header("mcp-session-id") != Some("sess-1")
    {
        return send(
            &mut stream,
            400,
            Some("text/plain"),
            &[],
            b"no session",
            false,
        );
    }
    match method {
        "initialize" => {
            let extra: &[(&str, &str)] = if script["session"] == true {
                &[("mcp-session-id", "sess-1")]
            } else {
                &[]
            };
            let result = json!({"protocolVersion": msg["params"]["protocolVersion"], "capabilities": {"tools": {}},
                "serverInfo": {"name": "fake-easy-context", "version": "1"}});
            message(
                &mut stream,
                script,
                &json!({"jsonrpc": "2.0", "id": msg["id"], "result": result}),
                extra,
            )
        }
        "notifications/initialized" => send(&mut stream, 202, None, &[], b"", false),
        "tools/call" => tool_call(&mut stream, script, seen, &request, &msg),
        _ => send(&mut stream, 400, Some("text/plain"), &[], b"?", false),
    }
}

fn tool_call(
    stream: &mut TcpStream,
    script: &Value,
    seen: &Mutex<Seen>,
    request: &Request,
    msg: &Value,
) -> std::io::Result<()> {
    let root = request.header("x-easy-context-root").map(str::to_string);
    {
        let mut seen = seen.lock().unwrap();
        seen.calls
            .push(msg["params"]["name"].as_str().unwrap_or("").to_string());
        seen.roots.push(root.clone());
    }
    let delay = script["delay_s"].as_f64().unwrap_or(0.0);
    if delay > 0.0 {
        thread::sleep(Duration::from_secs_f64(delay));
    }
    if script["break"] == "rpc_error" {
        let error =
            json!({"jsonrpc": "2.0", "id": msg["id"], "error": {"code": -32000, "message": "x"}});
        return message(stream, script, &error, &[]);
    }
    let rejected = script["reject_roots"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r.as_str() == root.as_deref());
    let result = if rejected {
        json!({"content": [{"type": "text", "text": "outside the workspace"}], "isError": true})
    } else {
        let body = json!({"project_id": "x", "root": root, "indexes": script["indexes"]});
        let text = body.to_string();
        match script["reply"].as_str().unwrap() {
            "structured" => {
                json!({"content": [{"type": "text", "text": text}], "structuredContent": body, "isError": false})
            }
            "text" => json!({"content": [{"type": "text", "text": text}], "isError": false}),
            "wrapped" => {
                let wrapped = json!({"result": body});
                json!({"content": [{"type": "text", "text": wrapped.to_string()}], "structuredContent": wrapped, "isError": false})
            }
            "no_content" => json!({"structuredContent": body, "isError": false}),
            "text_garbage" => {
                json!({"content": [{"type": "text", "text": "nope"}], "isError": false})
            }
            "list" => {
                json!({"content": [{"type": "text", "text": json!([body]).to_string()}], "structuredContent": null, "isError": false})
            }
            other => panic!("unknown reply kind {other}"),
        }
    };
    message(
        stream,
        script,
        &json!({"jsonrpc": "2.0", "id": msg["id"], "result": result}),
        &[],
    )
}
