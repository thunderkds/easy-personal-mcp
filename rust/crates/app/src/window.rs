//! The `app` subcommand (T023, RS-005): the loopback server of `server.rs` in this process and one Tauri v2 window on
//! `http://127.0.0.1:<port>/#<token>` (the token travels in the fragment only: it never reaches the server, a query,
//! a log line or `ps`). Tauri sits behind the off-by-default `window` Cargo feature and every `tauri` use is in this
//! file, like `rmcp` in `mcp.rs`: a default build links no Tauri, GTK or WebKit code, and `serve`, `mcp` and
//! `store-info` run with no display.
//!
//! Order of `app` (feature build): the folder is validated (as `serve` does), the display preflight (`display.rs`,
//! at most 4 s, no Tauri code yet), the server, the window. Exit codes: 0 the window closed, a SIGINT/SIGTERM or the
//! self-test passed; 1 any other failure; 2 a command-line error (clap); 3 the self-test saw no authenticated request
//! within 20 s; 4 a build without the `window` feature; 5 no display (or a toolkit that cannot start).
//!
//! The window has no IPC power (T016, answer 4): `tauri.conf.json` turns off the global API, names no plugin, and
//! `build.rs` points the capability pattern at a folder with no files, so every Tauri command is refused for the page.
//! `security.csp` stays null on purpose: the CSP is the server's header (`server.rs`). The webview may move only
//! inside the server's origin and may open no other window (`navigation_allowed`), because the dashboard's sidebar has
//! plain external links.
//!
//! `--self-test` (hidden) is for verification without eyes: the process prints `self-test: ok` and exits 0 as soon as
//! the page made one `/api/` request that passed the guard with the token, or exits 3 after 20 s.
//!
//! Not done here: macOS and Windows. They need no display preflight (a window server is always there; `display.rs` is
//! Unix-only) and use other webviews (WKWebView, WebView2), the signal watch uses Unix signals, and the stub icon is
//! one 32x32 square. A second launch does not focus the first (RS-005's last clause).

#[cfg(not(feature = "window"))]
use std::path::Path;
#[cfg(not(feature = "window"))]
use std::process::ExitCode;

#[cfg(not(feature = "window"))]
pub fn run(_static_dir: &Path, _self_test: bool) -> ExitCode {
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

#[cfg(feature = "window")]
pub use shell::run;

#[cfg(feature = "window")]
mod shell {
    use super::{blocked_target, navigation_allowed};
    use crate::server::{self, Running};
    use crate::{display, static_files};
    use std::io::{self, Write};
    use std::panic::{self, AssertUnwindSafe};
    use std::path::Path;
    use std::process::ExitCode;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::{Duration, Instant};
    use tauri::webview::NewWindowResponse;
    use tauri::{AppHandle, Url, WebviewUrl, WebviewWindowBuilder};

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

    /// A failure before the window ran: its exit code and one line for stderr.
    struct Failure(u8, String);

    pub fn run(static_dir: &Path, self_test: bool) -> ExitCode {
        let root = match static_files::canonical_root(static_dir) {
            Ok(root) => root,
            Err(e) => return fail(&Failure(1, e)),
        };
        if let Err(e) = display::preflight() {
            let hint = "serve, mcp and store-info need no display";
            return fail(&Failure(EXIT_NO_DISPLAY, format!("{e} ({hint})")));
        }
        let running = match server::start(Some(&root)) {
            Ok(running) => running,
            Err(e) => return fail(&Failure(1, e)),
        };
        let signalled = Arc::new(AtomicBool::new(false));
        let outcome = open(&running, self_test, &signalled);
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
        let port = running.port;
        let url: Url = format!("http://127.0.0.1:{port}/#{}", running.token)
            .parse()
            .map_err(|_| Failure(1, "cannot form the window address".into()))?;
        let started = Arc::new(AtomicBool::new(false));
        watch_startup(&started);
        // Without a display `tao` panics inside `build` (T016): turn that into the no-display exit.
        let hook = panic::take_hook();
        panic::set_hook(Box::new(|_| {}));
        let built = panic::catch_unwind(AssertUnwindSafe(|| {
            tauri::Builder::default().build(tauri::generate_context!())
        }));
        panic::set_hook(hook);
        let app = match built {
            Ok(Ok(app)) => app,
            Ok(Err(e)) => return Err(Failure(1, format!("cannot start the window toolkit: {e}"))),
            Err(_) => {
                return Err(Failure(
                    EXIT_NO_DISPLAY,
                    "no display: the window toolkit could not start".into(),
                ));
            }
        };
        WebviewWindowBuilder::new(&app, "main", WebviewUrl::External(url))
            .title("easy-personal")
            .inner_size(1280.0, 800.0)
            .resizable(true)
            .devtools(false)
            .initialization_script(LINK_GUARD)
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
            .on_new_window(|url, _features| {
                eprintln!(
                    "easy-personal-rs: blocked new window to {}",
                    blocked_target(url.as_str())
                );
                NewWindowResponse::Deny
            })
            .build()
            .map_err(|e| Failure(1, format!("cannot open the window: {e}")))?;
        started.store(true, Ordering::Relaxed);
        eprintln!("easy-personal-rs: window open on http://127.0.0.1:{port}/");
        watch_signals(running, app.handle().clone(), Arc::clone(signalled));
        if self_test {
            watch_self_test(Arc::clone(&running.authenticated), app.handle().clone());
        }
        Ok(app.run_return(|_, _| {}))
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

    /// `--self-test`: exit 0 with `self-test: ok` once the guard saw an authenticated `/api/` request, else exit 3.
    fn watch_self_test(authenticated: Arc<AtomicBool>, handle: AppHandle) {
        thread::spawn(move || {
            let deadline = Instant::now() + SELF_TEST_LIMIT;
            loop {
                if authenticated.load(Ordering::Relaxed) {
                    let _ = writeln!(io::stdout(), "self-test: ok");
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

    const PORT: u16 = 41234;

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
        assert!(manifest.contains("[features]\nwindow = [\"dep:tauri\", \"dep:tauri-build\"]"));
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
            "tray-icon",
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

    #[cfg(not(feature = "window"))]
    #[test]
    fn without_the_feature_the_exit_code_is_4() {
        assert_eq!(EXIT_NO_WINDOW_BUILD, 4);
    }
}
