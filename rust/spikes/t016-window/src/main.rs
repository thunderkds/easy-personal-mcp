//! T016 throwaway spike. Starts a tiny loopback server that mimics the dashboard guard, opens a Tauri window on
//! `http://127.0.0.1:<port>/#<token>`, waits for the page to POST a report from inside the webview, prints it as one
//! JSON line and exits 0 (non-zero after 20 s without a report). Nothing but 127.0.0.1 is contacted.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{WebviewUrl, WebviewWindowBuilder};

/// Same string as `CSP` in `rust/crates/app/src/server.rs`.
const CSP: &str = "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'";
const INDEX: &str = include_str!("page/index.html");
const APP_JS: &str = include_str!("page/app.js");
const TIMEOUT: Duration = Duration::from_secs(20);

static START: OnceLock<Instant> = OnceLock::new();

fn log(msg: impl AsRef<str>) {
    let ms = START.get().map_or(0, |s| s.elapsed().as_millis());
    eprintln!("[spike +{ms}ms] {}", msg.as_ref());
}

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

struct Shared {
    token: String,
    host: String,
    origin: String,
    tx: Mutex<Sender<String>>,
    requests: Mutex<Vec<String>>,
}

fn new_token() -> String {
    let mut bytes = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .expect("no /dev/urandom");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

/// Mimics `allowed()` in server.rs: exact single Host, any Origin must be ours, `/api/*` and `/report` need the token.
fn allowed(s: &Shared, path: &str, headers: &[(String, String)]) -> bool {
    fn all<'a>(headers: &'a [(String, String)], name: &str) -> Vec<&'a str> {
        headers.iter().filter(|(k, _)| k == name).map(|(_, v)| v.as_str()).collect()
    }
    let hosts = all(headers, "host");
    if hosts.len() != 1 || hosts[0] != s.host {
        return false;
    }
    if !all(headers, "origin").iter().all(|o| *o == s.origin) {
        return false;
    }
    if path.starts_with("/api/") || path == "/report" {
        let tokens = all(headers, "x-dashboard-token");
        return tokens.len() == 1 && eq(tokens[0].as_bytes(), s.token.as_bytes());
    }
    true
}

fn respond(stream: &mut TcpStream, status: &str, ctype: &str, body: &str) {
    let head = format!(
        "HTTP/1.1 {status}\r\ncontent-type: {ctype}\r\ncontent-length: {}\r\ncontent-security-policy: {CSP}\r\ncache-control: no-store\r\nconnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body.as_bytes());
}

fn handle(mut stream: TcpStream, s: Arc<Shared>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
        if buf.len() > 65536 {
            return;
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let mut first = lines.next().unwrap_or("").split(' ');
    let (method, path) = (first.next().unwrap_or("").to_string(), first.next().unwrap_or("").to_string());
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    let content_length: usize = headers
        .iter()
        .find(|(k, _)| k == "content-length")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    while buf.len() < head_end + content_length.min(65536) {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    let body = String::from_utf8_lossy(&buf[head_end..]).to_string();

    let origin_note = headers.iter().find(|(k, _)| k == "origin").map_or("no Origin".to_string(), |(_, v)| format!("Origin {v}"));
    let status = if !allowed(&s, &path, &headers) {
        respond(&mut stream, "403 Forbidden", "application/json", r#"{"error":"forbidden"}"#);
        403
    } else {
        match (method.as_str(), path.as_str()) {
            ("GET", "/") => {
                respond(&mut stream, "200 OK", "text/html; charset=utf-8", INDEX);
                200
            }
            ("GET", "/app.js") => {
                respond(&mut stream, "200 OK", "text/javascript; charset=utf-8", APP_JS);
                200
            }
            ("GET", "/api/version") => {
                respond(&mut stream, "200 OK", "application/json", r#"{"version":"spike"}"#);
                200
            }
            ("POST", "/report") => {
                respond(&mut stream, "200 OK", "application/json", r#"{"ok":true}"#);
                // Test knob for the 20 s timeout path: pretend the page never reported.
                if std::env::var_os("T016_DROP_REPORT").is_none() {
                    let _ = s.tx.lock().unwrap().send(body);
                }
                200
            }
            _ => {
                respond(&mut stream, "404 Not Found", "text/plain", "");
                404
            }
        }
    };
    s.requests.lock().unwrap().push(format!("{method} {path} -> {status} ({origin_note})"));
}

fn strip_fragment(url: &tauri::Url) -> String {
    let mut u = url.clone();
    u.set_fragment(None);
    u.to_string()
}

fn main() {
    START.set(Instant::now()).unwrap();
    let token = new_token();
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).expect("cannot bind 127.0.0.1");
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel::<String>();
    let shared = Arc::new(Shared {
        host: format!("127.0.0.1:{port}"),
        origin: format!("http://127.0.0.1:{port}"),
        token: token.clone(),
        tx: Mutex::new(tx),
        requests: Mutex::new(Vec::new()),
    });
    let accept_state = shared.clone();
    thread::spawn(move || {
        for conn in listener.incoming().flatten() {
            let s = accept_state.clone();
            thread::spawn(move || handle(conn, s));
        }
    });
    log(format!("server on 127.0.0.1:{port}, token length {}", token.len()));
    let env = |k: &str| std::env::var(k).map_or("unset".to_string(), |v| format!("'{v}'"));
    log(format!(
        "env DISPLAY={} WAYLAND_DISPLAY={} GDK_BACKEND={} WEBKIT_DISABLE_COMPOSITING_MODE={} LIBGL_ALWAYS_SOFTWARE={}",
        env("DISPLAY"),
        env("WAYLAND_DISPLAY"),
        env("GDK_BACKEND"),
        env("WEBKIT_DISABLE_COMPOSITING_MODE"),
        env("LIBGL_ALWAYS_SOFTWARE")
    ));
    let url = format!("http://127.0.0.1:{port}/#{token}");
    // Without this, tao panics ("Failed to initialize gtk backend!", exit 101) when there is no display.
    // gtk::init() returns an Err instead, so the product can fall back to "CLI and MCP only" (RS-006).
    if let Err(e) = gtk::init() {
        eprintln!("t016: no display, cannot open a window ({e}); set DISPLAY or WAYLAND_DISPLAY");
        std::process::exit(5);
    }
    let report_state = shared.clone();

    let built = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping])
        .on_page_load(|_, payload| log(format!("page load {:?} url={}", payload.event(), strip_fragment(payload.url()))))
        .on_window_event(|w, e| log(format!("window event on '{}': {e:?}", w.label())))
        .setup(move |app| {
            let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.parse()?))
                .title("t016 spike")
                .inner_size(800.0, 600.0)
                .build()?;
            log("window created: label 'main'");
            if let Ok(gtk_window) = window.gtk_window() {
                use gtk::prelude::{ObjectExt, WidgetExt};
                log(format!("gdk display type: {}", gtk_window.display().type_().name()));
            }
            let handle = app.handle().clone();
            thread::spawn(move || {
                match rx.recv_timeout(TIMEOUT.saturating_sub(START.get().unwrap().elapsed())) {
                    Ok(body) => {
                        log("report received from the webview");
                        match serde_json::from_str::<serde_json::Value>(&body) {
                            Ok(v) => println!("{v}"),
                            Err(e) => {
                                eprintln!("report was not JSON: {e}");
                                handle.exit(4);
                                return;
                            }
                        }
                        for line in report_state.requests.lock().unwrap().iter() {
                            log(format!("server saw: {line}"));
                        }
                        handle.exit(0);
                    }
                    Err(_) => {
                        eprintln!("t016: no report from the webview within {}s (window may not have opened or the page did not run)", TIMEOUT.as_secs());
                        for line in report_state.requests.lock().unwrap().iter() {
                            log(format!("server saw: {line}"));
                        }
                        handle.exit(3);
                    }
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!());
    match built {
        Ok(app) => app.run(|_, _| {}),
        Err(e) => {
            eprintln!("t016: tauri could not start: {e}");
            std::process::exit(1);
        }
    }
}
