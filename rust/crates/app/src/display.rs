//! The display preflight of `app` (T023, RS-006): can a window server be reached at all? It runs before any Tauri or
//! GTK code is touched, because those fail badly without a display (T016, `spike-T016.md`): with nothing set, `tao`
//! panics (exit 101); with a dead `DISPLAY=:99`, GTK's init hangs on a TCP connect for good.
//!
//! The check only *connects* (never reads or writes a byte, never opens a file) and every connect has a time limit,
//! run on a helper thread, so the whole preflight takes at most `2 * STEP_LIMIT` (4 s) and cannot hang:
//! - no `DISPLAY` and no `WAYLAND_DISPLAY` (an empty value counts as unset): refused at once;
//! - `WAYLAND_DISPLAY`: the socket is `$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY`, or the value itself when it is an absolute
//!   path; a relative name with no `XDG_RUNTIME_DIR` fails;
//! - `DISPLAY`: `:N` and `:N.S` are the unix socket `/tmp/.X11-unix/X<N>`, `host:N` is TCP `host:6000+N`, `unix:<N>`
//!   is `:N`, and `unix:/path` is that unix socket;
//! - one working display is enough ("either works": GTK tries Wayland first and falls back to X11).
//!
//! The error names the variable and the problem, never a socket path or a value.
//! Linux and the BSDs only (X11 and Wayland). macOS (WKWebView) and Windows (WebView2) have no such step: a window
//! server is always there, so this module is not used on them (P4, P5).

use std::net::{TcpStream, ToSocketAddrs};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Time limit of one check (the Wayland socket, then the X11 display): the preflight takes at most twice this.
pub const STEP_LIMIT: Duration = Duration::from_secs(2);

/// The three variables the check reads. An empty value is stored as `None`.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Env {
    pub display: Option<String>,
    pub wayland_display: Option<String>,
    pub xdg_runtime_dir: Option<String>,
}

impl Env {
    #[cfg(feature = "window")]
    pub fn from_process() -> Env {
        let get = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
        Env {
            display: get("DISPLAY"),
            wayland_display: get("WAYLAND_DISPLAY"),
            xdg_runtime_dir: get("XDG_RUNTIME_DIR"),
        }
    }
}

/// Where an X server would be listening.
#[derive(Debug, PartialEq)]
pub enum X11Target {
    Unix(PathBuf),
    Tcp { host: String, port: u16 },
}

/// Reads a `DISPLAY` value (see the module comment). `None` is "not a display name".
pub fn parse_x11(display: &str) -> Option<X11Target> {
    if let Some(path) = display.strip_prefix("unix:/") {
        // A path with a NUL or a path that is not absolute cannot be a socket we can connect to.
        return (!path.contains('\0')).then(|| X11Target::Unix(PathBuf::from(format!("/{path}"))));
    }
    let display = display
        .strip_prefix("unix")
        .filter(|r| r.starts_with(':'))
        .unwrap_or(display);
    let (host, rest) = display.rsplit_once(':')?;
    let (number, screen) = rest.split_once('.').unwrap_or((rest, "0"));
    let all_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !all_digits(number) || !all_digits(screen) {
        return None;
    }
    let number: u16 = number.parse().ok()?;
    if host.is_empty() {
        return Some(X11Target::Unix(PathBuf::from(format!(
            "/tmp/.X11-unix/X{number}"
        ))));
    }
    let host = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);
    let plain = |b: u8| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b':');
    if host.is_empty() || !host.bytes().all(plain) {
        return None;
    }
    Some(X11Target::Tcp {
        host: host.to_string(),
        port: 6000u16.checked_add(number)?,
    })
}

/// The Wayland socket for `WAYLAND_DISPLAY`: the value itself when absolute, else inside `XDG_RUNTIME_DIR`.
/// The path is only ever connected to, never read as a file. `None`: a relative name and no runtime folder.
pub fn wayland_socket(name: &str, runtime_dir: Option<&str>) -> Option<PathBuf> {
    if name.contains('\0') {
        return None;
    }
    if name.starts_with('/') {
        return Some(PathBuf::from(name));
    }
    runtime_dir.map(|dir| Path::new(dir).join(name))
}

/// Runs `f` on a helper thread and waits at most `limit` for its answer. A stuck `f` is left behind (the process
/// ends soon after), so this never blocks longer than `limit`.
fn within<T: Send + 'static>(limit: Duration, f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(limit).ok()
}

/// True when something accepts a connection on the unix socket `path` within `limit`.
pub fn unix_answers(path: &Path, limit: Duration) -> bool {
    let path = path.to_path_buf();
    within(limit, move || UnixStream::connect(path).is_ok()).unwrap_or(false)
}

/// True when `host:port` accepts a TCP connection within `limit` (name lookup included).
pub fn tcp_answers(host: &str, port: u16, limit: Duration) -> bool {
    let host = host.to_string();
    within(limit, move || {
        (host.as_str(), port)
            .to_socket_addrs()
            .map(|mut addrs| addrs.any(|a| TcpStream::connect_timeout(&a, limit).is_ok()))
            .unwrap_or(false)
    })
    .unwrap_or(false)
}

/// Ok when a display can be reached; else one line (no path, no value) saying which variable and what is wrong.
pub fn check(env: &Env, limit: Duration) -> Result<(), String> {
    if env.display.is_none() && env.wayland_display.is_none() {
        return Err("no display: neither WAYLAND_DISPLAY nor DISPLAY is set".into());
    }
    let mut problems = Vec::new();
    if let Some(name) = &env.wayland_display {
        match wayland_socket(name, env.xdg_runtime_dir.as_deref()) {
            Some(socket) if unix_answers(&socket, limit) => return Ok(()),
            Some(_) => problems.push("WAYLAND_DISPLAY is set but no compositor answers on it"),
            None => {
                problems.push("WAYLAND_DISPLAY is a relative name and XDG_RUNTIME_DIR is not set")
            }
        }
    }
    if let Some(value) = &env.display {
        match parse_x11(value) {
            Some(X11Target::Unix(socket)) if unix_answers(&socket, limit) => return Ok(()),
            Some(X11Target::Tcp { host, port }) if tcp_answers(&host, port, limit) => return Ok(()),
            Some(_) => problems.push("DISPLAY is set but no X server answers on it"),
            None => problems.push("DISPLAY is not a valid display name"),
        }
    }
    Err(format!("no display: {}", problems.join("; ")))
}

/// `check` on the process environment with the standard time limit.
#[cfg(feature = "window")]
pub fn preflight() -> Result<(), String> {
    check(&Env::from_process(), STEP_LIMIT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::os::unix::net::UnixListener;
    use std::time::Instant;

    const LIMIT: Duration = Duration::from_millis(500);

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t023-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// True once `answers` is false. A socket whose listener was just dropped can still answer for a few
    /// milliseconds when another test forks a child that holds a copy of its descriptor until it execs.
    fn refused_soon(answers: impl Fn() -> bool) -> bool {
        (0..40).any(|_| {
            let refused = !answers();
            if !refused {
                thread::sleep(Duration::from_millis(25));
            }
            refused
        })
    }

    fn unix(path: &str) -> Option<X11Target> {
        Some(X11Target::Unix(PathBuf::from(path)))
    }

    fn tcp(host: &str, port: u16) -> Option<X11Target> {
        Some(X11Target::Tcp {
            host: host.into(),
            port,
        })
    }

    #[test]
    fn display_names_are_parsed_like_xlib_reads_them() {
        assert_eq!(parse_x11(":0"), unix("/tmp/.X11-unix/X0"));
        assert_eq!(parse_x11(":0.0"), unix("/tmp/.X11-unix/X0"));
        assert_eq!(parse_x11(":10.2"), unix("/tmp/.X11-unix/X10"));
        assert_eq!(parse_x11("unix:3"), unix("/tmp/.X11-unix/X3"));
        assert_eq!(parse_x11("unix:/run/x/sock"), unix("/run/x/sock"));
        assert_eq!(parse_x11("localhost:1"), tcp("localhost", 6001));
        assert_eq!(parse_x11("host:1"), tcp("host", 6001));
        assert_eq!(parse_x11("127.0.0.1:99"), tcp("127.0.0.1", 6099));
        assert_eq!(parse_x11("10.0.0.5:0.1"), tcp("10.0.0.5", 6000));
        assert_eq!(parse_x11("[::1]:2"), tcp("::1", 6002));
    }

    #[test]
    fn empty_and_garbage_display_names_are_refused() {
        for bad in [
            "",
            ":",
            ":x",
            ":0.",
            ":.0",
            ":0.x",
            "host",
            "host:",
            "host:x",
            ":-1",
            ":65536",
            "host:60000",
            "ho st:0",
            "ho/st:0",
            "host;rm:0",
            "\u{e9}:0",
            ":0\n",
            "unix:/a\0b",
        ] {
            assert_eq!(parse_x11(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_wayland_name_is_joined_to_the_runtime_dir_and_an_absolute_one_is_kept() {
        assert_eq!(
            wayland_socket("wayland-0", Some("/run/user/1000")),
            Some(PathBuf::from("/run/user/1000/wayland-0"))
        );
        assert_eq!(
            wayland_socket("/run/w/sock", None),
            Some(PathBuf::from("/run/w/sock"))
        );
        assert_eq!(
            wayland_socket("/run/w/sock", Some("/run/user/1000")),
            Some(PathBuf::from("/run/w/sock"))
        );
        assert_eq!(wayland_socket("wayland-0", None), None);
        assert_eq!(wayland_socket("a\0b", Some("/run")), None);
    }

    #[test]
    fn a_unix_socket_that_listens_answers_and_a_closed_or_missing_one_does_not() {
        let dir = temp_dir("unix");
        let live = dir.join("live");
        let _listener = UnixListener::bind(&live).unwrap();
        assert!(unix_answers(&live, LIMIT));
        let closed = dir.join("closed");
        drop(UnixListener::bind(&closed).unwrap());
        assert!(
            closed.exists(),
            "the socket file stays behind after the listener is gone"
        );
        assert!(refused_soon(|| unix_answers(&closed, LIMIT)));
        assert!(!unix_answers(&dir.join("missing"), LIMIT));
        // A regular file in the socket's place is connected to, never read.
        let file = dir.join("file");
        std::fs::write(&file, b"secret").unwrap();
        assert!(!unix_answers(&file, LIMIT));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_tcp_port_that_listens_answers_and_a_closed_one_does_not() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(tcp_answers("127.0.0.1", port, LIMIT));
        assert!(tcp_answers("localhost", port, LIMIT));
        drop(listener);
        assert!(refused_soon(|| tcp_answers("127.0.0.1", port, LIMIT)));
    }

    #[test]
    fn the_time_limit_holds_when_the_check_itself_is_stuck() {
        let began = Instant::now();
        assert_eq!(
            within(Duration::from_millis(100), || thread::sleep(
                Duration::from_secs(5)
            )),
            None
        );
        assert!(began.elapsed() < Duration::from_secs(2));
    }

    fn env(display: Option<&str>, wayland: Option<&str>, runtime: Option<&str>) -> Env {
        Env {
            display: display.map(str::to_string),
            wayland_display: wayland.map(str::to_string),
            xdg_runtime_dir: runtime.map(str::to_string),
        }
    }

    #[test]
    fn no_variable_at_all_is_refused_at_once() {
        let began = Instant::now();
        let err = check(&Env::default(), STEP_LIMIT).unwrap_err();
        assert_eq!(
            err,
            "no display: neither WAYLAND_DISPLAY nor DISPLAY is set"
        );
        assert!(began.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn a_live_wayland_socket_is_enough_even_when_display_is_dead() {
        let dir = temp_dir("wl");
        let _listener = UnixListener::bind(dir.join("wayland-test")).unwrap();
        let runtime = dir.to_str().unwrap();
        assert!(
            check(
                &env(Some("127.0.0.1:99"), Some("wayland-test"), Some(runtime)),
                LIMIT
            )
            .is_ok()
        );
        // Absolute path: no runtime dir needed.
        let absolute = dir.join("wayland-test");
        assert!(check(&env(None, Some(absolute.to_str().unwrap()), None), LIMIT).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_live_x_socket_is_enough_when_wayland_is_dead() {
        let dir = temp_dir("x11");
        let socket = dir.join("X7");
        let _listener = UnixListener::bind(&socket).unwrap();
        let display = format!("unix:{}", socket.display());
        assert!(
            check(
                &env(
                    Some(&display),
                    Some("wayland-gone"),
                    Some(dir.to_str().unwrap())
                ),
                LIMIT
            )
            .is_ok()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn when_both_fail_the_message_names_each_variable_and_no_path_or_value() {
        let dir = temp_dir("dead");
        let secret = dir.join("secret-socket-name");
        drop(UnixListener::bind(&secret).unwrap());
        assert!(
            refused_soon(|| unix_answers(&secret, LIMIT)),
            "the dropped socket kept answering"
        );
        let display = format!("unix:{}", secret.display());
        let err = check(
            &env(Some(&display), Some(secret.to_str().unwrap()), None),
            LIMIT,
        )
        .unwrap_err();
        assert_eq!(
            err,
            "no display: WAYLAND_DISPLAY is set but no compositor answers on it; DISPLAY is set but no X server answers on it"
        );
        assert!(
            !err.contains("secret") && !err.contains("/tmp") && !err.contains(".X11"),
            "{err}"
        );
        let err = check(&env(Some("garbage"), Some("wayland-0"), None), LIMIT).unwrap_err();
        assert_eq!(
            err,
            "no display: WAYLAND_DISPLAY is a relative name and XDG_RUNTIME_DIR is not set; DISPLAY is not a valid display name"
        );
        let err = check(&env(Some(":99"), None, None), LIMIT).unwrap_err();
        assert_eq!(
            err,
            "no display: DISPLAY is set but no X server answers on it"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_dead_tcp_display_fails_fast_and_never_takes_longer_than_the_limits_added() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        assert!(refused_soon(|| tcp_answers("127.0.0.1", port, STEP_LIMIT)));
        let began = Instant::now();
        assert!(!tcp_answers("127.0.0.1", port, STEP_LIMIT));
        assert!(
            began.elapsed() < Duration::from_secs(2),
            "{:?}",
            began.elapsed()
        );
        // Worst case of the whole check: both steps use their whole limit.
        assert!(STEP_LIMIT * 2 <= Duration::from_secs(5));
    }
}
