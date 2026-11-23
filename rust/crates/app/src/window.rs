//! The `app` subcommand (T023, RS-005): the loopback server of `server.rs` in this process and one Tauri v2 window on
//! `http://127.0.0.1:<port>/#<token>` (the token travels in the fragment only: it never reaches the server, a query,
//! a log line or `ps`). Tauri sits behind the off-by-default `window` Cargo feature and every `tauri` use is in this
//! file, like `rmcp` in `mcp.rs`: a default build links no Tauri, GTK or WebKit code, and `serve`, `mcp` and
//! `store-info` run with no display.
//!
//! Order of `app` (feature build): the folder is validated (as `serve` does), the display preflight (`display.rs`,
//! at most 4 s, no Tauri code yet), the server, the window. Exit codes: 0 the window closed, a SIGINT/SIGTERM or the
//! self-test passed; 1 any other failure; 2 a command-line error (clap); 3 the self-test saw no authenticated request
//! within 20 s or its live check failed; 4 a build without the `window` feature; 5 no display (or a toolkit that
//! cannot start).
//!
//! The window has no IPC power (T016, answer 4): `tauri.conf.json` turns off the global API, names no plugin, and
//! `build.rs` points the capability pattern at a folder with no files, so every Tauri command is refused for the page.
//! `security.csp` stays null on purpose: the CSP is the server's header (`server.rs`). The webview may move only
//! inside the server's origin and may open no other window (`navigation_allowed`), because the dashboard's sidebar has
//! plain external links.
//!
//! `--self-test` (hidden) is for verification without eyes: once the page made one `/api/` request that passed the
//! guard with the token (else exit 3 after 20 s), the window closes and the live check runs (T057): the live feed was
//! given to the server and runs, and `GET /api/live?token=<token>` (the page's `EventSource` request) gives a first
//! `snapshot` with the snapshot keys. Then it prints `self-test: live snapshot ok` and `self-test: ok` and exits 0,
//! else exits 3.
//!
//! A plain `app` (T101: no `--self-test`, no `--no-tray`) opens no window of its own: after the folder and display
//! checks it starts `tray --show` as a process of its own (same exe, same folder, own process group, stdio on /dev/null)
//! and exits 0, so the tray owns the only dashboard window; a tray that cannot start is one stderr line, and `app`
//! then opens its own window as before.
//!
//! The `tray` subcommand (T076) lives here too: one lock per user, the same server with the alert engine on, a tray
//! icon and native menu (`tray.rs` builds the rows), and the same window opened from "Open dashboard". With `--show`
//! (T101) it opens the window at start, or, when a tray already runs, asks that tray to show its window and exits: the
//! running tray puts its pid in its lock file and shows the window on `SIGUSR1` (`tray::hand_over`).
//!
//! Not done here: macOS and Windows. They need no display preflight (a window server is always there; `display.rs` is
//! Unix-only) and use other webviews (WKWebView, WebView2), the signal watch uses Unix signals, and the icon is a PNG
//! (no `.icns` or `.ico` yet). A second launch does not focus the first (RS-005's last clause).

#[cfg(not(feature = "window"))]
use std::path::Path;
#[cfg(not(feature = "window"))]
use std::process::ExitCode;

#[cfg(not(feature = "window"))]
pub fn run(_static_dir: &Path, _self_test: bool, _alerts: bool, _no_tray: bool) -> ExitCode {
    eprintln!("this build has no window: rebuild with --features window");
    ExitCode::from(EXIT_NO_WINDOW_BUILD)
}

/// The `tray` subcommand (T076) needs the window build too.
#[cfg(not(feature = "window"))]
pub fn tray(_static_dir: Option<&Path>, _autostart: Option<&str>, _show: bool) -> ExitCode {
    eprintln!("this build has no window: rebuild with --features window");
    ExitCode::from(EXIT_NO_WINDOW_BUILD)
}

#[cfg(not(feature = "window"))]
const EXIT_NO_WINDOW_BUILD: u8 = 4;

/// True when `url` is inside the server's origin `http://127.0.0.1:<port>`: that exact text, then nothing, `/`, `?` or
/// `#`. A longer port, a host that continues the text (`.evil.com`), a `user@` part and every other scheme or host
/// (`localhost` included) fail. The caller passes the URL as the webview serialises it.
#[cfg(any(feature = "window", test))]
fn navigation_allowed(url: &str, port: u16) -> bool {
    let origin = format!("http://127.0.0.1:{port}");
    match url.strip_prefix(&origin) {
        Some("") => true,
        Some(rest) => matches!(rest.as_bytes()[0], b'/' | b'?' | b'#'),
        None => false,
    }
}

/// True when `url` is a terminal popped out of the dashboard (T135): inside the server's origin, the path `/`, the query
/// exactly `term=<id>` (1..=128 of ASCII letters, digits, `-`, `_`, `.`, `%`: the id as the page percent-encodes it), then
/// nothing or a fragment. Only this address may open a second window; every other new window stays denied. The check
/// limits the address's shape, not the id's content: the page only looks the id up in the server's list.
#[cfg(any(feature = "window", test))]
fn is_term_popout(url: &str, port: u16) -> bool {
    if !navigation_allowed(url, port) {
        return false;
    }
    let rest = &url[format!("http://127.0.0.1:{port}").len()..];
    let Some(query) = rest.strip_prefix("/?term=") else {
        return false;
    };
    let id = query.split('#').next().unwrap_or("");
    (1..=128).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'%'))
}

/// What a log line may say about a blocked URL: the host only, or the scheme when there is no host. Never the
/// user, port, path, query or fragment, and only plain characters.
#[cfg(any(feature = "window", test))]
fn blocked_target(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once(':') else {
        return "a URL without a scheme".into();
    };
    let plain = |s: &str| {
        s.chars()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':'))
            .take(100)
            .collect::<String>()
    };
    let Some(authority) = rest.strip_prefix("//") else {
        return format!("scheme \"{}\"", plain(scheme));
    };
    let authority = authority.split(['/', '?', '#']).next().unwrap_or("");
    let host_port = authority.rsplit('@').next().unwrap_or("");
    let host = match host_port.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or(""),
        None => host_port.split(':').next().unwrap_or(""),
    };
    match plain(host).as_str() {
        "" => format!("an empty host (scheme \"{}\")", plain(scheme)),
        host => format!("host \"{host}\""),
    }
}

/// `app` starts a tray unless it is the self-test or the owner passed `--no-tray` (T101).
#[cfg(any(feature = "window", test))]
fn wants_tray(self_test: bool, no_tray: bool) -> bool {
    !self_test && !no_tray
}

/// The program and arguments of the tray `app` starts (T101): this same executable, the same canonical folder, and
/// `--show` (open the window at start, or ask the running tray to show its own).
#[cfg(any(feature = "window", test))]
fn tray_command(
    exe: &std::path::Path,
    root: &std::path::Path,
) -> (std::path::PathBuf, Vec<std::ffi::OsString>) {
    (
        exe.to_path_buf(),
        vec![
            "tray".into(),
            "--static-dir".into(),
            root.into(),
            "--show".into(),
        ],
    )
}

/// The one stderr line for a tray that did not start (T101).
#[cfg(any(feature = "window", test))]
fn tray_not_started(reason: &str) -> String {
    format!("easy-personal-rs: tray not started: {reason}")
}

/// What `app` saw of the tray it started (T101).
#[cfg(any(feature = "window", test))]
enum Tray {
    /// Still running after the wait: it is the new tray.
    Running,
    /// It ended within the wait: `ok` for exit 0 (it handed over to the running tray), `status` as text.
    Exited { ok: bool, status: String },
}

/// Polls `child` for up to `limit`.
#[cfg(any(feature = "window", test))]
fn observe(child: &mut std::process::Child, limit: std::time::Duration) -> Result<Tray, String> {
    let deadline = std::time::Instant::now() + limit;
    loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => {
                return Ok(Tray::Exited {
                    ok: status.success(),
                    status: status.to_string(),
                });
            }
            None if std::time::Instant::now() >= deadline => return Ok(Tray::Running),
            None => std::thread::sleep(std::time::Duration::from_millis(25)),
        }
    }
}

/// What `app` does after it tried to start the tray (T101): `None` when the tray runs or handed over (`app` opens no
/// window and exits 0), else the stderr line (no start, or a failed exit such as a hand-over to a tray that cannot be
/// asked), and `app` opens its own window as today.
#[cfg(any(feature = "window", test))]
fn after_spawn(spawned: Result<Tray, String>) -> Option<String> {
    match spawned {
        Ok(Tray::Running | Tray::Exited { ok: true, .. }) => None,
        Ok(Tray::Exited { status, .. }) => {
            Some(tray_not_started(&format!("the tray exited with {status}")))
        }
        Err(reason) => Some(tray_not_started(&reason)),
    }
}

/// Starts `program` in a process group of its own with stdin, stdout and stderr on /dev/null, so Ctrl-C or a SIGTERM
/// to `app`'s group never reaches it and it outlives `app`.
#[cfg(any(feature = "window", test))]
fn spawn_detached(
    program: &std::path::Path,
    args: &[std::ffi::OsString],
) -> std::io::Result<std::process::Child> {
    use std::process::{Command, Stdio};
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    command.spawn()
}

#[cfg(feature = "window")]
pub use shell::{run, tray};

#[cfg(feature = "window")]
mod shell {
    use super::{
        after_spawn, blocked_target, is_term_popout, navigation_allowed, observe, spawn_detached,
        tray_command, wants_tray,
    };
    use crate::live_feed::LiveFeed;
    use crate::server::{self, Running};
    use crate::tray::{self, Row, State, Target, Theme};
    use crate::{display, static_files};
    use std::fs::File;
    use std::io::{self, Write};
    use std::panic::{self, AssertUnwindSafe};
    use std::path::Path;
    use std::process::ExitCode;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::mpsc::Receiver;
    use std::thread;
    use std::time::{Duration, Instant};
    use tauri::image::Image;
    use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;
    use tauri::webview::NewWindowResponse;
    use tauri::{App, AppHandle, Manager, RunEvent, Url, WebviewUrl, WebviewWindowBuilder, Wry};

    /// How long `app` watches the tray it started: the hand-over itself waits up to 2 s for the running tray's pid.
    const HAND_OVER_WAIT: Duration = Duration::from_secs(3);
    const EXIT_SELF_TEST_TIMEOUT: i32 = 3;
    const EXIT_NO_DISPLAY: u8 = 5;
    /// The self-test gives the page this long to make an authenticated request.
    const SELF_TEST_LIMIT: Duration = Duration::from_secs(20);
    /// A display that accepted the preflight's connect but never lets the toolkit start: give up after this.
    const STARTUP_LIMIT: Duration = Duration::from_secs(10);

    /// Runs before the page's own scripts. A click on a link to another origin is cancelled in the capture phase: the
    /// navigation lock alone does not stop WebKitGTK from opening a connection to the link's host (DNS, TCP and for
    /// `https` a TLS hello) the moment the link is activated, even when the policy then refuses the navigation. Cancelling
    /// the click stops both. Not a security boundary (page code can bypass it): `navigation_allowed` is the lock.
    const LINK_GUARD: &str = r#"(() => {
  const cancel = (event) => {
    const link = event.target && event.target.closest ? event.target.closest("a[href], area[href]") : null;
    if (!link) return;
    const href = typeof link.href === "string" ? link.href : link.href && link.href.baseVal;
    let origin = "null";
    try { origin = new URL(href, location.href).origin; } catch (_) {}
    if (origin !== location.origin) event.preventDefault();
  };
  window.addEventListener("click", cancel, true);
  window.addEventListener("auxclick", cancel, true);
})();"#;

    /// Runs before the page's own scripts in every app window (T135): the page's Pop out reads it. Here the webview's own
    /// new window is denied and the app opens the pop-out itself, so `window.open` answers null with the window opening;
    /// without the flag the page would take that null for a pop-up blocker. Not a security boundary: `is_term_popout` is.
    const APP_FLAG: &str = "window.__EASY_PERSONAL_APP__ = true;";

    /// The number of the next terminal window (`term-<n>`): one label per pop-out, never reused while the app runs.
    static NEXT_TERM: AtomicU32 = AtomicU32::new(1);

    /// A failure before the window ran: its exit code and one line for stderr.
    struct Failure(u8, String);

    pub fn run(static_dir: &Path, self_test: bool, alerts: bool, no_tray: bool) -> ExitCode {
        if wants_tray(self_test, no_tray) {
            match hand_over_to_tray(static_dir) {
                Ok(None) => return ExitCode::SUCCESS,
                Ok(Some(line)) => eprintln!("{line}"),
                Err(failure) => return fail(&failure),
            }
        }
        let running = match start(static_dir, alerts, None) {
            Ok(running) => running,
            Err(failure) => return fail(&failure),
        };
        let signalled = Arc::new(AtomicBool::new(false));
        let outcome = match open(&running, self_test, &signalled) {
            // The page passed the guard (the self-test ended the loop): now the live check, before the server stops.
            Ok(0)
                if self_test
                    && running.authenticated.load(Ordering::Relaxed)
                    && !signalled.load(Ordering::Relaxed) =>
            {
                live_check(&running)
            }
            other => other,
        };
        running.stop();
        match outcome {
            Ok(code) => {
                if !signalled.load(Ordering::Relaxed) {
                    eprintln!("easy-personal-rs: window closed");
                }
                ExitCode::from(code.clamp(0, 255) as u8)
            }
            Err(failure) => fail(&failure),
        }
    }

    /// The folder check, the display preflight, the live feed, the two samplers and the server (with the alert
    /// engine when `alerts`): what `app` and `tray` both run before any Tauri code. With `clicks` (the `tray` only) a
    /// click on a popup is sent there; without, a popup carries no action.
    fn start(
        static_dir: &Path,
        alerts: bool,
        clicks: Option<crate::alerts::Clicks>,
    ) -> Result<Running, Failure> {
        let root = static_files::canonical_root(static_dir).map_err(|e| Failure(1, e))?;
        check_display()?;
        let feed = LiveFeed::system().map_err(|e| Failure(1, e))?;
        let machine = crate::sampler::machine().map_err(|e| Failure(1, e))?;
        let limits = crate::sampler::limits().map_err(|e| Failure(1, e))?;
        let notifier = alerts.then(|| {
            let notifier = crate::alerts::DBusNotifier::new();
            Box::new(match clicks {
                Some(clicks) => notifier.with_clicks(clicks),
                None => notifier,
            }) as Box<dyn crate::alerts::Notifier>
        });
        let mut running = server::start_with_alerts(
            Some(&root),
            Some(feed),
            Some(machine),
            Some(limits),
            notifier,
        )
        .map_err(|e| Failure(1, e))?;
        running.listen_for_answers(); // T106: terminal sessions can be answered from this window
        Ok(running)
    }

    /// The display preflight (`display.rs`, no Tauri code yet): exit 5 when there is none.
    fn check_display() -> Result<(), Failure> {
        display::preflight().map_err(|e| {
            let hint = "serve, mcp and store-info need no display";
            Failure(EXIT_NO_DISPLAY, format!("{e} ({hint})"))
        })
    }

    /// A plain `app` (T101): the folder and display checks as `start` makes them, then `<this exe> tray --static-dir
    /// <folder> --show` as a detached process, watched for up to [`HAND_OVER_WAIT`]. `Ok(None)` when it runs or handed
    /// over (`app` exits 0 and opens no window), `Ok(Some(line))` when it did not start or exited non-zero (a tray that
    /// cannot be asked, such as an older one: the stderr line; `app` opens its own window), `Err` for a bad folder or
    /// display.
    fn hand_over_to_tray(static_dir: &Path) -> Result<Option<String>, Failure> {
        let root = static_files::canonical_root(static_dir).map_err(|e| Failure(1, e))?;
        check_display()?;
        let spawned = std::env::current_exe()
            .map_err(|_| "cannot find this program's path".to_string())
            .and_then(|exe| {
                let (program, args) = tray_command(&exe, &root);
                let mut child = spawn_detached(&program, &args).map_err(|e| e.to_string())?;
                observe(&mut child, HAND_OVER_WAIT)
            });
        Ok(after_spawn(spawned))
    }

    /// `--self-test`, second half (T057): `GET /api/live?token=<token>` must give a first `snapshot` from a running
    /// feed. Prints `self-test: live snapshot ok` and `self-test: ok`, else fails with exit 3.
    fn live_check(running: &Running) -> Result<i32, Failure> {
        running.live_self_check().map_err(|e| {
            Failure(
                EXIT_SELF_TEST_TIMEOUT as u8,
                format!("self-test: live check: {e}"),
            )
        })?;
        let _ = writeln!(io::stdout(), "self-test: live snapshot ok\nself-test: ok");
        Ok(0)
    }

    fn fail(failure: &Failure) -> ExitCode {
        eprintln!("easy-personal-rs: {}", failure.1);
        ExitCode::from(failure.0)
    }

    /// Builds the app and the one window, runs the event loop until it ends, returns its exit code.
    fn open(
        running: &Running,
        self_test: bool,
        signalled: &Arc<AtomicBool>,
    ) -> Result<i32, Failure> {
        let started = Arc::new(AtomicBool::new(false));
        watch_startup(&started);
        let app = build_app()?;
        open_window(&app, running.port, &running.token, &Target::Dashboard)?;
        started.store(true, Ordering::Relaxed);
        eprintln!(
            "easy-personal-rs: window open on http://127.0.0.1:{}/",
            running.port
        );
        watch_signals(running, app.handle().clone(), Arc::clone(signalled));
        if self_test {
            watch_self_test(Arc::clone(&running.authenticated), app.handle().clone());
        }
        Ok(app.run_return(|_, _| {}))
    }

    /// The Tauri app with no window yet. Without a display `tao` panics inside `build` (T016): that is the no-display exit.
    fn build_app() -> Result<App, Failure> {
        let hook = panic::take_hook();
        panic::set_hook(Box::new(|_| {}));
        let built = panic::catch_unwind(AssertUnwindSafe(|| {
            tauri::Builder::default().build(tauri::generate_context!())
        }));
        panic::set_hook(hook);
        match built {
            Ok(Ok(app)) => Ok(app),
            Ok(Err(e)) => Err(Failure(1, format!("cannot start the window toolkit: {e}"))),
            Err(_) => Err(Failure(
                EXIT_NO_DISPLAY,
                "no display: the window toolkit could not start".into(),
            )),
        }
    }

    /// The window address of `target` (`tray::address`) as a URL. The failure names no part of the address: it holds the token.
    fn url_of(port: u16, token: &str, target: &Target) -> Result<Url, Failure> {
        tray::address(port, token, target)
            .parse()
            .map_err(|_| Failure(1, "cannot form the window address".into()))
    }

    /// The dashboard window `main` on the address of `target` (`http://127.0.0.1:<port>/#<token>` for the plain open),
    /// locked to the server's origin.
    fn open_window<M: Manager<Wry>>(
        manager: &M,
        port: u16,
        token: &str,
        target: &Target,
    ) -> Result<(), Failure> {
        let url = url_of(port, token, target)?;
        guarded_window(manager, "main", url, "easy-personal", (1280.0, 800.0), port)
    }

    /// One app window (the dashboard, or a popped-out terminal, T135) with the same guards: locked to the server's
    /// origin, the link guard, no devtools. A new window it asks for opens only for a terminal pop-out address
    /// (`is_term_popout`), as another guarded window built here; everything else is denied and logged by host only.
    fn guarded_window<M: Manager<Wry>>(
        manager: &M,
        label: &str,
        url: Url,
        title: &str,
        (width, height): (f64, f64),
        port: u16,
    ) -> Result<(), Failure> {
        let handle = manager.app_handle().clone();
        // Both `app` and the window the tray opens come through here, so one icon call covers both. Compiled in (no file read).
        WebviewWindowBuilder::new(manager, label, WebviewUrl::External(url))
            .icon(tauri::include_image!("icons/128x128.png"))
            .map_err(|e| Failure(1, format!("cannot set the window icon: {e}")))?
            .title(title)
            .inner_size(width, height)
            .resizable(true)
            .devtools(false)
            .initialization_script(LINK_GUARD)
            .initialization_script(APP_FLAG)
            .on_navigation(move |url| {
                let allowed = navigation_allowed(url.as_str(), port);
                if !allowed {
                    eprintln!(
                        "easy-personal-rs: blocked navigation to {}",
                        blocked_target(url.as_str())
                    );
                }
                allowed
            })
            .on_new_window(move |url, _features| {
                if is_term_popout(url.as_str(), port) {
                    // Built on the next turn of the main loop, not inside the webview's own callback. The page has already
                    // closed its panel (`window.open` answers null here), so a build that fails is one stderr line only.
                    let inner = handle.clone();
                    let _ = handle.run_on_main_thread(move || {
                        let label = format!("term-{}", NEXT_TERM.fetch_add(1, Ordering::Relaxed));
                        let opened = guarded_window(
                            &inner,
                            &label,
                            url,
                            "easy-personal · terminal",
                            (1100.0, 720.0),
                            port,
                        );
                        if let Err(failure) = opened {
                            eprintln!("easy-personal-rs: {}", failure.1);
                        }
                    });
                } else {
                    eprintln!(
                        "easy-personal-rs: blocked new window to {}",
                        blocked_target(url.as_str())
                    );
                }
                NewWindowResponse::Deny
            })
            .build()
            .map(|_| ())
            .map_err(|e| Failure(1, format!("cannot open the window: {e}")))
    }

    // -- tray (T076) ----------------------------------------------------------------------------------------------------

    const TRAY_ID: &str = "easy-personal";
    /// The tray looks every this long (the alert engine's tick).
    const TRAY_TICK: Duration = Duration::from_secs(5);

    /// `tray`: `--autostart on|off` writes or removes the autostart entry and exits; else the tray runs (one per user).
    /// `show` (T101): open the window at start, or hand over to the running tray.
    pub fn tray(static_dir: Option<&Path>, autostart: Option<&str>, show: bool) -> ExitCode {
        match autostart {
            Some(toggle) => set_autostart(static_dir, toggle == "on"),
            None => match static_dir {
                Some(dir) => run_tray(dir, show),
                None => {
                    eprintln!("easy-personal-rs: tray needs --static-dir <DIR>");
                    ExitCode::from(2)
                }
            },
        }
    }

    fn set_autostart(static_dir: Option<&Path>, on: bool) -> ExitCode {
        let path = match tray::autostart_path(
            std::env::var_os("XDG_CONFIG_HOME"),
            std::env::var_os("HOME"),
        ) {
            Ok(path) => path,
            Err(e) => return fail(&Failure(1, e)),
        };
        if !on {
            return match tray::autostart_off(&path) {
                Ok(true) => {
                    println!("autostart off: removed {}", path.display());
                    ExitCode::SUCCESS
                }
                Ok(false) => {
                    println!("autostart off: {} was not there", path.display());
                    ExitCode::SUCCESS
                }
                Err(e) => fail(&Failure(1, e)),
            };
        }
        let Some(dir) = static_dir else {
            eprintln!("easy-personal-rs: tray --autostart on needs --static-dir <DIR>");
            return ExitCode::from(2);
        };
        let written = static_files::canonical_root(dir)
            .and_then(|root| {
                let exe = std::env::current_exe()
                    .and_then(|e| e.canonicalize())
                    .map_err(|_| "cannot find this program's path".to_string())?;
                tray::desktop_entry(&exe, &root)
            })
            .and_then(|entry| tray::autostart_on(&path, &entry));
        match written {
            Ok(()) => {
                println!("autostart on: wrote {}", path.display());
                ExitCode::SUCCESS
            }
            Err(e) => fail(&Failure(1, e)),
        }
    }

    fn run_tray(static_dir: &Path, show: bool) -> ExitCode {
        let store = easy_personal_core::store::store_path().ok();
        let path = match tray::lock_path(std::env::var_os("XDG_RUNTIME_DIR"), store) {
            Ok(path) => path,
            Err(e) => return fail(&Failure(1, e)),
        };
        let held = match tray::try_lock(&path) {
            Ok(Some(file)) => file,
            Ok(None) if show => {
                // T101: another tray owns the window: ask it to show it (a failed hand-over is one line, exit 1)
                return match tray::hand_over(&path) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(e) => fail(&Failure(1, e)),
                };
            }
            Ok(None) => {
                println!("easy-personal-rs: a tray already runs for this user");
                return ExitCode::SUCCESS;
            }
            Err(e) => return fail(&Failure(1, e)),
        };
        let (clicks, clicked) = std::sync::mpsc::channel();
        let running = match start(static_dir, true, Some(clicks)) {
            Ok(running) => running,
            Err(failure) => return fail(&failure),
        };
        let signalled = Arc::new(AtomicBool::new(false));
        let outcome = tray_loop(&running, &signalled, clicked, &held, show);
        running.stop();
        match outcome {
            Ok(code) => ExitCode::from(code.clamp(0, 255) as u8),
            Err(failure) => fail(&failure),
        }
    }

    fn icon(theme: Theme, state: State) -> Image<'static> {
        use tauri::include_image;
        match (theme, state) {
            (Theme::Dark, State::Idle) => include_image!("icons/tray/dark/idle-32.png"),
            (Theme::Dark, State::Active) => include_image!("icons/tray/dark/active-32.png"),
            (Theme::Dark, State::Attention) => include_image!("icons/tray/dark/attention-32.png"),
            (Theme::Dark, State::Problem) => include_image!("icons/tray/dark/problem-32.png"),
            (Theme::Light, State::Idle) => include_image!("icons/tray/light/idle-32.png"),
            (Theme::Light, State::Active) => include_image!("icons/tray/light/active-32.png"),
            (Theme::Light, State::Attention) => include_image!("icons/tray/light/attention-32.png"),
            (Theme::Light, State::Problem) => include_image!("icons/tray/light/problem-32.png"),
        }
    }

    /// The handle of one shown row, to change its text or tick in place (`None` for a separator).
    enum Item {
        Plain(MenuItem<Wry>),
        Check(CheckMenuItem<Wry>),
        Separator,
    }

    fn menu_of(handle: &AppHandle, rows: &[Row]) -> tauri::Result<(Menu<Wry>, Vec<Item>)> {
        let menu = Menu::new(handle)?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let item = match row {
                Row::Info(text) => Item::Plain(MenuItem::new(handle, text, false, None::<&str>)?),
                Row::Action(id, text) => {
                    Item::Plain(MenuItem::with_id(handle, *id, text, true, None::<&str>)?)
                }
                Row::Session(session, text) => Item::Plain(MenuItem::with_id(
                    handle,
                    tray::session_item_id(session),
                    text,
                    true,
                    None::<&str>,
                )?),
                Row::Check(id, text, checked) => Item::Check(CheckMenuItem::with_id(
                    handle,
                    *id,
                    text,
                    true,
                    *checked,
                    None::<&str>,
                )?),
                Row::Separator => {
                    menu.append(&PredefinedMenuItem::separator(handle)?)?;
                    items.push(Item::Separator);
                    continue;
                }
            };
            match &item {
                Item::Plain(i) => menu.append(i)?,
                Item::Check(i) => menu.append(i)?,
                Item::Separator => {}
            }
            items.push(item);
        }
        Ok((menu, items))
    }

    /// A click: open the window on the address of `target`, or when it is open bring it to the front. "Open dashboard"
    /// and "All alerts in dashboard…" ([`Target::Dashboard`]) leave an open window as it is; a session row, "+N more" and
    /// a popup ([`Target::Live`]) navigate it to the Live view first (the page reloads and reads the address once, T095).
    /// Runs on the main thread: the menu event does, a popup click is sent there (`run_on_main_thread`). With the window
    /// still loading its first page the navigation lands on the wanted address; there is never a second window.
    fn show_dashboard(handle: &AppHandle, port: u16, token: &str, target: &Target) {
        if let Some(window) = handle.get_webview_window("main") {
            if *target != Target::Dashboard {
                match url_of(port, token, target) {
                    Ok(url) => {
                        let _ = window.navigate(url);
                    }
                    Err(failure) => eprintln!("easy-personal-rs: {}", failure.1),
                }
            }
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        } else if let Err(failure) = open_window(handle, port, token, target) {
            eprintln!("easy-personal-rs: {}", failure.1);
        }
    }

    /// The tray's event loop: the icon and the menu, refreshed every [`TRAY_TICK`]; a closed dashboard window does not
    /// end it, Quit or a signal does. The icon is always made: without a StatusNotifier host now, one line says so, and
    /// the tray library registers the icon when a host appears later (an extension turned on after login).
    fn tray_loop(
        running: &Running,
        signalled: &Arc<AtomicBool>,
        clicked: Receiver<Option<String>>,
        lock: &File,
        show: bool,
    ) -> Result<i32, Failure> {
        let started = Arc::new(AtomicBool::new(false));
        watch_startup(&started);
        let app = build_app()?;
        started.store(true, Ordering::Relaxed);
        if !tray::tray_host(None) {
            eprintln!(
                "easy-personal-rs: no tray area (no StatusNotifier host): the tray icon is not shown; alerts keep working"
            );
        }
        let icons = tray::icon_dir(
            std::env::var_os("XDG_RUNTIME_DIR"),
            easy_personal_core::store::store_path().ok(),
        )
        .map_err(|e| Failure(1, e))?;
        let inputs = running.tray_inputs();
        let theme = tray::theme_of(tray::desktop_color_scheme(None));
        let view = inputs.view();
        let (menu, items) = menu_of(app.handle(), &view.rows)
            .map_err(|e| Failure(1, format!("cannot make the tray menu: {e}")))?;
        let (port, token, shared) = (
            running.port,
            running.token.clone(),
            Arc::clone(&inputs.alerts),
        );
        watch_clicks(clicked, app.handle().clone(), port, token.clone());
        let shown = TrayIconBuilder::with_id(TRAY_ID)
            .temp_dir_path(&icons)
            .icon(icon(theme, view.state))
            .menu(&menu)
            .on_menu_event(move |handle, event| {
                let id = event.id().as_ref();
                if let Some(session) = tray::session_of(id) {
                    let target = Target::Live(Some(session.to_string()));
                    return show_dashboard(handle, port, &token, &target);
                }
                match id {
                    tray::OPEN | tray::ALERTS => {
                        show_dashboard(handle, port, &token, &Target::Dashboard)
                    }
                    tray::MORE => show_dashboard(handle, port, &token, &Target::Live(None)),
                    tray::PAUSE => shared.set_paused(!shared.paused()),
                    tray::QUIT => handle.exit(0),
                    _ => {}
                }
            })
            .build(&app)
            .map_err(|e| Failure(1, format!("cannot make the tray icon: {e}")))?;
        drop(shown); // the manager keeps it; a clone held here would outlive the exit and leave the icon file behind
        watch_tray(inputs, view, items, theme, app.handle().clone());
        watch_signals(running, app.handle().clone(), Arc::clone(signalled));
        // T101: the pid goes into the lock file only once the handler is there (`SIGUSR1` ends a process with none)
        if watch_show(running, app.handle().clone())
            && let Err(e) = tray::write_pid(lock, std::process::id())
        {
            eprintln!("easy-personal-rs: {e}");
        }
        if show
            && let Err(failure) =
                open_window(&app, running.port, &running.token, &Target::Dashboard)
        {
            eprintln!("easy-personal-rs: {}", failure.1);
        }
        eprintln!(
            "easy-personal-rs: tray running, dashboard on http://127.0.0.1:{}/",
            running.port
        );
        Ok(app.run_return(|_, event| {
            // the last window closed: the tray keeps running (an exit with a code is Quit or a signal)
            if let RunEvent::ExitRequested {
                code: None, api, ..
            } = event
            {
                api.prevent_exit();
            }
        }))
    }

    /// `SIGUSR1` (T101, the hand-over of a second `tray --show`) shows the dashboard window as a menu click on "Open
    /// dashboard" does: an open one is unminimized and focused as it is, with none one is opened. The window work runs on
    /// the main thread. False when the handler could not be installed (the pid must not be published then).
    fn watch_show(running: &Running, handle: AppHandle) -> bool {
        use tokio::signal::unix::{SignalKind, signal};
        let runtime = running.handle();
        let _in_runtime = runtime.enter();
        let Ok(mut usr1) = signal(SignalKind::user_defined1()) else {
            return false;
        };
        let (port, token) = (running.port, running.token.clone());
        runtime.spawn(async move {
            while usr1.recv().await.is_some() {
                let (window, token) = (handle.clone(), token.clone());
                let shown = handle.run_on_main_thread(move || {
                    show_dashboard(&window, port, &token, &Target::Dashboard);
                });
                if shown.is_err() {
                    return;
                }
            }
        });
        true
    }

    /// A click on a popup (`alerts::DBusNotifier`) opens the Live view, with its session when it has one. The click arrives
    /// on the notifier's thread and the window work runs on the main thread (`run_on_main_thread`). Ends when the notifier
    /// is gone or the event loop is.
    fn watch_clicks(
        clicked: Receiver<Option<String>>,
        handle: AppHandle,
        port: u16,
        token: String,
    ) {
        thread::spawn(move || {
            while let Ok(session) = clicked.recv() {
                let (window, token) = (handle.clone(), token.clone());
                let shown = handle.run_on_main_thread(move || {
                    show_dashboard(&window, port, &token, &Target::Live(session));
                });
                if shown.is_err() {
                    return;
                }
            }
        });
    }

    /// Every [`TRAY_TICK`]: a new view. The icon is set only when the state changed. When the rows keep their places
    /// (`tray::same_shape`) only the changed texts and ticks are set; else the menu is rebuilt. Tauri runs each call on
    /// the main thread. Ends when the icon is gone or a call fails (the event loop ended).
    fn watch_tray(
        inputs: tray::Inputs,
        mut last: tray::View,
        mut items: Vec<Item>,
        theme: Theme,
        handle: AppHandle,
    ) {
        thread::spawn(move || {
            loop {
                thread::sleep(TRAY_TICK);
                // looked up each tick and dropped at its end: at exit the manager's drop removes the icon file
                let Some(shown) = handle.tray_by_id(TRAY_ID) else {
                    return;
                };
                let view = inputs.view();
                if view.state != last.state
                    && shown.set_icon(Some(icon(theme, view.state))).is_err()
                {
                    return;
                }
                if tray::same_shape(&view.rows, &last.rows) {
                    for ((new, old), item) in view.rows.iter().zip(&last.rows).zip(&items) {
                        if new == old {
                            continue;
                        }
                        let done = match (new, item) {
                            (
                                Row::Info(text) | Row::Action(_, text) | Row::Session(_, text),
                                Item::Plain(i),
                            ) => i.set_text(text),
                            (Row::Check(_, text, checked), Item::Check(i)) => {
                                i.set_text(text).and_then(|()| i.set_checked(*checked))
                            }
                            _ => Ok(()),
                        };
                        if done.is_err() {
                            return;
                        }
                    }
                } else {
                    let Ok((menu, fresh)) = menu_of(shown.app_handle(), &view.rows) else {
                        return;
                    };
                    if shown.set_menu(Some(menu)).is_err() {
                        return;
                    }
                    items = fresh;
                }
                last = view;
            }
        });
    }

    /// If the window is not up within `STARTUP_LIMIT` the display is stuck: say so and leave (exit 5).
    fn watch_startup(started: &Arc<AtomicBool>) {
        let started = Arc::clone(started);
        thread::spawn(move || {
            let deadline = Instant::now() + STARTUP_LIMIT;
            while Instant::now() < deadline {
                if started.load(Ordering::Relaxed) {
                    return;
                }
                thread::sleep(Duration::from_millis(50));
            }
            eprintln!(
                "easy-personal-rs: no display: the window did not start within {} s",
                STARTUP_LIMIT.as_secs()
            );
            std::process::exit(i32::from(EXIT_NO_DISPLAY));
        });
    }

    /// SIGINT and SIGTERM end the window like a close: the event loop returns 0 and `run` stops the server.
    fn watch_signals(running: &Running, handle: AppHandle, signalled: Arc<AtomicBool>) {
        use tokio::signal::unix::{SignalKind, signal};
        let runtime = running.handle();
        let _in_runtime = runtime.enter();
        let (Ok(mut term), Ok(mut int)) = (
            signal(SignalKind::terminate()),
            signal(SignalKind::interrupt()),
        ) else {
            return;
        };
        runtime.spawn(async move {
            tokio::select! {
                _ = term.recv() => {}
                _ = int.recv() => {}
            }
            signalled.store(true, Ordering::Relaxed);
            eprintln!("easy-personal-rs: stopping on a signal");
            handle.exit(0);
        });
    }

    /// `--self-test`: end the event loop with 0 once the guard saw an authenticated `/api/` request (`run` then makes
    /// the live check and prints the result), else exit 3.
    fn watch_self_test(authenticated: Arc<AtomicBool>, handle: AppHandle) {
        thread::spawn(move || {
            let deadline = Instant::now() + SELF_TEST_LIMIT;
            loop {
                if authenticated.load(Ordering::Relaxed) {
                    handle.exit(0);
                    return;
                }
                if Instant::now() >= deadline {
                    eprintln!(
                        "easy-personal-rs: self-test: no authenticated request within {} s",
                        SELF_TEST_LIMIT.as_secs()
                    );
                    handle.exit(EXIT_SELF_TEST_TIMEOUT);
                    return;
                }
                thread::sleep(Duration::from_millis(25));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::Duration;

    const PORT: u16 = 41234;

    /// T135 AC 7: only the terminal pop-out address may open a second window.
    #[test]
    fn only_a_terminal_pop_out_opens_a_window() {
        for url in [
            "http://127.0.0.1:41234/?term=t1",
            "http://127.0.0.1:41234/?term=t1#TOKEN",
            "http://127.0.0.1:41234/?term=0123abcd-4567_x.y#TOK",
            "http://127.0.0.1:41234/?term=a%2Fb#TOK",
        ] {
            assert!(is_term_popout(url, PORT), "{url}");
        }
        let long = format!("http://127.0.0.1:41234/?term={}#T", "a".repeat(129));
        for url in [
            "http://127.0.0.1:41234/",
            "http://127.0.0.1:41234/#TOKEN",
            "http://127.0.0.1:41234/?term=",
            "http://127.0.0.1:41234/?term=#TOK",
            "http://127.0.0.1:41234/?term=t1&x=1",
            "http://127.0.0.1:41234/?live=t1",
            "http://127.0.0.1:41234/assets/?term=t1",
            "http://127.0.0.1:41234?term=t1",
            "http://127.0.0.1:41234/?term=a/b",
            "http://127.0.0.1:41234/?term=a b",
            "http://127.0.0.1:41235/?term=t1",
            "http://127.0.0.1:412345/?term=t1",
            "http://localhost:41234/?term=t1",
            "https://127.0.0.1:41234/?term=t1",
            "http://127.0.0.1:41234.evil.com/?term=t1",
            "http://user@127.0.0.1:41234/?term=t1",
            &long,
        ] {
            assert!(!is_term_popout(url, PORT), "{url}");
        }
    }

    #[test]
    fn only_the_servers_own_origin_is_allowed() {
        for url in [
            "http://127.0.0.1:41234",
            "http://127.0.0.1:41234/",
            "http://127.0.0.1:41234/#TOKEN",
            "http://127.0.0.1:41234/assets/app.js?v=1#frag",
            "http://127.0.0.1:41234?x=1",
            "http://127.0.0.1:41234#only-a-fragment",
        ] {
            assert!(navigation_allowed(url, PORT), "{url}");
        }
    }

    /// T096 AC 9: the three shapes of the address a click opens are inside the origin, and nothing else changed.
    #[test]
    fn the_address_of_a_click_is_allowed() {
        use crate::tray::{Target, address};
        for target in [
            Target::Dashboard,
            Target::Live(None),
            Target::Live(Some("0123abcd-4567".into())),
            Target::Live(Some("<script>".into())),
        ] {
            let url = address(PORT, "TOK", &target);
            assert!(navigation_allowed(&url, PORT), "{url}");
            assert!(!navigation_allowed(&url, PORT + 1), "{url}");
        }
    }

    #[test]
    fn everything_else_is_blocked() {
        for url in [
            "",
            "http://127.0.0.1:41235/",
            "http://127.0.0.1:4123/",
            "http://127.0.0.1:412345/",
            "http://127.0.0.1/",
            "http://localhost:41234/",
            "http://[::1]:41234/",
            "http://example.com/",
            "http://example.com:41234/",
            "https://127.0.0.1:41234/",
            "https://example.com/",
            "file:///etc/passwd",
            "data:text/html,<h1>x</h1>",
            "javascript:alert(1)",
            "about:blank",
            "ws://127.0.0.1:41234/",
            "HTTP://127.0.0.1:41234/",
            "http://127.0.0.1:41234.evil.com/",
            "http://127.0.0.1:41234@evil.com/",
            "http://127.0.0.1:41234\\@evil.com/",
            "http://user@127.0.0.1:41234/",
            "http://evil.com/#http://127.0.0.1:41234/",
            "http://evil.com/?u=http://127.0.0.1:41234/",
            " http://127.0.0.1:41234/",
            "//127.0.0.1:41234/",
            "/api/version",
        ] {
            assert!(!navigation_allowed(url, PORT), "{url:?}");
        }
    }

    #[test]
    fn a_blocked_url_is_logged_by_host_only() {
        assert_eq!(
            blocked_target("https://example.com/path?q=secret#frag"),
            "host \"example.com\""
        );
        assert_eq!(
            blocked_target("http://user:pw@host.test:8080/x"),
            "host \"host.test\""
        );
        assert_eq!(blocked_target("http://[::1]:9/x"), "host \"::1\"");
        assert_eq!(
            blocked_target("http://127.0.0.1:1/?token=abc#def"),
            "host \"127.0.0.1\""
        );
        assert_eq!(blocked_target("data:text/html,SECRET"), "scheme \"data\"");
        assert_eq!(
            blocked_target("javascript:alert(SECRET)"),
            "scheme \"javascript\""
        );
        assert_eq!(
            blocked_target("file:///etc/passwd"),
            "an empty host (scheme \"file\")"
        );
        assert_eq!(blocked_target(""), "a URL without a scheme");
        for logged in [
            blocked_target("https://example.com/path?q=SECRET#SECRET"),
            blocked_target("http://SECRET@example.com/"),
            blocked_target("data:text/html,SECRET"),
        ] {
            assert!(
                !logged.contains("SECRET") && !logged.contains('?') && !logged.contains('#'),
                "{logged}"
            );
        }
        let long = format!("http://{}/", "a".repeat(500));
        assert!(blocked_target(&long).len() < 120);
    }

    /// The Tauri config: no global API, no plugin, no dev server, a null CSP (the server sends it) and no window
    /// (the code makes the one window).
    #[test]
    fn the_tauri_config_gives_the_page_no_power() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(config["app"]["withGlobalTauri"], false);
        assert!(
            config["app"]["security"]["csp"].is_null(),
            "the CSP comes from the server header"
        );
        assert_eq!(config["app"]["windows"], serde_json::json!([]));
        assert_eq!(config["bundle"]["active"], false);
        assert_eq!(config["build"]["frontendDist"], "./stub-frontend");
        assert_eq!(config["productName"], "easy-personal");
        for absent in ["plugins", "devUrl", "beforeDevCommand"] {
            assert!(
                config.get(absent).is_none() && config["build"].get(absent).is_none(),
                "{absent}"
            );
        }
        // The default list of capabilities of the app section is not set (it would include every capability file).
        assert!(config["app"]["security"].get("capabilities").is_none());
        let text = include_str!("../tauri.conf.json");
        for word in [
            "shell",
            "fs",
            "updater",
            "opener",
            "devtools",
            "dangerous",
            "plugins",
            "capabilities",
        ] {
            assert!(!text.contains(word), "tauri.conf.json mentions {word}");
        }
    }

    /// No capability can be compiled in: no `capabilities/` folder, the build script's pattern points at a folder that
    /// holds no `*.json`, and the manifest enables no Tauri feature that adds power (`devtools`, `protocol-asset`, ...).
    #[test]
    fn no_capability_file_exists_and_no_tauri_feature_adds_power() {
        let app = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(!app.join("capabilities").exists());
        let pattern_dir = app.join("no-capabilities");
        let json_files = std::fs::read_dir(&pattern_dir)
            .map(|dir| {
                dir.flatten()
                    .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
                    .count()
            })
            .unwrap_or(0);
        assert_eq!(json_files, 0);
        let build = include_str!("../build.rs");
        assert!(build.contains("capabilities_path_pattern(\"./no-capabilities/*.json\")"));
        let manifest = include_str!("../Cargo.toml");
        // T076: `tray-icon` is the one Tauri feature added (the `tray` command's icon and menu; no IPC command)
        assert!(manifest.contains(
            "[features]\nwindow = [\"dep:tauri\", \"dep:tauri-build\", \"tauri/tray-icon\"]"
        ));
        assert!(
            !manifest.contains("default = [\"window\"]"),
            "the window feature is off by default"
        );
        let workspace = include_str!("../../../Cargo.toml");
        assert!(
            workspace.contains("tauri = \"=2.12.5\"")
                && workspace.contains("tauri-build = \"=2.7.4\"")
        );
        for feature in [
            "devtools",
            "protocol-asset",
            "isolation",
            "unstable",
            "native-tls",
            "rustls",
        ] {
            assert!(
                !manifest.contains(feature) && !workspace.contains(feature),
                "{feature}"
            );
        }
    }

    /// What `tauri_build` really compiled into this binary: the capability set is empty, so no origin (the loopback page
    /// included) is granted any command.
    #[cfg(feature = "window")]
    #[test]
    fn the_compiled_capability_set_is_empty() {
        let compiled: serde_json::Value =
            serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/capabilities.json")))
                .unwrap();
        assert_eq!(compiled, serde_json::json!({}));
    }

    #[test]
    fn only_a_plain_window_run_wants_a_tray() {
        assert!(wants_tray(false, false));
        assert!(!wants_tray(true, false), "--self-test starts no tray");
        assert!(!wants_tray(false, true), "--no-tray starts no tray");
        assert!(!wants_tray(true, true));
    }

    #[test]
    fn the_tray_runs_this_exe_with_the_same_folder() {
        let (program, args) = tray_command(
            Path::new("/usr/bin/easy-personal-rs"),
            Path::new("/usr/share/easy-personal/dashboard"),
        );
        assert_eq!(program, Path::new("/usr/bin/easy-personal-rs"));
        assert_eq!(
            args,
            [
                "tray",
                "--static-dir",
                "/usr/share/easy-personal/dashboard",
                "--show"
            ]
        );
    }

    #[test]
    fn a_failed_tray_start_is_one_line_and_app_opens_its_own_window() {
        assert_eq!(
            tray_not_started("cannot find this program's path"),
            "easy-personal-rs: tray not started: cannot find this program's path"
        );
        // A program that is not there: the error is the one line, and `app` falls back to its own window.
        let spawned = spawn_detached(Path::new("/this/program/does/not/exist"), &["tray".into()])
            .map_err(|e| e.to_string());
        let line = after_spawn(spawned.map(|mut c| observe(&mut c, Duration::ZERO).unwrap()))
            .expect("a spawn error means an own window");
        assert!(
            line.starts_with("easy-personal-rs: tray not started: "),
            "{line}"
        );
        assert_eq!(line.lines().count(), 1, "{line}");
    }

    /// What `app` does with what it saw of the tray it started: still running after the wait (the new tray) and a clean
    /// exit (it handed over) are done; a failed exit is the one line and an own window.
    #[test]
    fn app_exits_for_a_running_or_a_clean_tray_and_falls_back_for_a_failed_one() {
        let exited = |ok: bool, status: &str| {
            Ok(Tray::Exited {
                ok,
                status: status.into(),
            })
        };
        assert_eq!(after_spawn(Ok(Tray::Running)), None);
        assert_eq!(after_spawn(exited(true, "exit status: 0")), None);
        assert_eq!(
            after_spawn(exited(false, "exit status: 1")).as_deref(),
            Some("easy-personal-rs: tray not started: the tray exited with exit status: 1")
        );
        assert_eq!(
            after_spawn(Err("boom".into())).as_deref(),
            Some("easy-personal-rs: tray not started: boom")
        );
    }

    /// The real child: `exit 1` is seen as a failed exit (the fallback line), `exit 0` as a clean one, a process still
    /// running after the limit as the new tray (the test then ends it: it is its own child).
    #[test]
    fn observe_sees_a_failed_exit_a_clean_exit_and_a_running_child() {
        let sh = |script: &str| spawn_detached(Path::new("/bin/sh"), &["-c".into(), script.into()]);
        let limit = Duration::from_secs(5);
        let line = after_spawn(observe(&mut sh("exit 1").unwrap(), limit));
        assert!(
            line.as_deref().is_some_and(|l| l.starts_with(
                "easy-personal-rs: tray not started: the tray exited with exit status: 1"
            )),
            "{line:?}"
        );
        assert_eq!(
            after_spawn(observe(&mut sh("exit 0").unwrap(), limit)),
            None
        );
        let mut running = sh("sleep 30").unwrap();
        let seen = observe(&mut running, Duration::from_millis(150));
        running.kill().unwrap();
        running.wait().unwrap();
        assert_eq!(after_spawn(seen), None);
    }

    /// The child is its own process group leader with stdin, stdout and stderr on /dev/null.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_tray_child_has_its_own_group_and_null_stdio() {
        let out =
            std::env::temp_dir().join(format!("easy-personal-rs-t101-{}", std::process::id()));
        let script = format!(
            "echo \"$(readlink /proc/$$/fd/0) $(readlink /proc/$$/fd/1) $(readlink /proc/$$/fd/2) $(cut -d' ' -f5 /proc/$$/stat) $$\" > {}",
            out.display()
        );
        let mut child =
            spawn_detached(Path::new("/bin/sh"), &["-c".into(), script.into()]).unwrap();
        child.wait().unwrap();
        let seen = std::fs::read_to_string(&out).unwrap();
        let _ = std::fs::remove_file(&out);
        let f: Vec<&str> = seen.split_whitespace().collect();
        assert_eq!(&f[..3], ["/dev/null"; 3], "{seen}");
        assert_eq!(f[3], f[4], "pgid is the child's own pid: {seen}");
        assert_ne!(
            f[3].parse::<u32>().unwrap(),
            unsafe { libc::getpgrp() } as u32,
            "not the test's group"
        );
    }

    #[cfg(not(feature = "window"))]
    #[test]
    fn without_the_feature_the_exit_code_is_4() {
        assert_eq!(EXIT_NO_WINDOW_BUILD, 4);
    }
}
