//! The `app` subcommand as the real binary, with no display (T023). Nothing here opens a window: a build without the
//! `window` feature prints one line and exits 4; a build with it must refuse a bad folder (1) and a missing or dead
//! display (5, in time) before any Tauri code runs, and `serve`, `mcp` and `store-info` must still work with no display.
//! `HOME` and `EASY_PERSONAL_HOME` point into a temp folder, so the owner's data folder is never reached.
//! The tests that need a display are in `window.rs` (ignored by default).

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t023-app-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Runs `app` (or any subcommand) with the three display variables removed, then `env` applied on top.
fn run(home: &TempDir, args: &[&str], env: &[(&str, &str)]) -> (Output, Duration) {
    let began = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
        .args(args)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("XDG_RUNTIME_DIR")
        .env("HOME", &home.0)
        .env("EASY_PERSONAL_HOME", &home.0)
        .envs(env.iter().copied())
        .output()
        .unwrap();
    (out, began.elapsed())
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn static_dir(home: &TempDir) -> String {
    let dir = home.0.join("dashboard");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("index.html"), "<!doctype html>").unwrap();
    dir.to_str().unwrap().to_string()
}

#[test]
fn app_needs_a_static_dir() {
    let home = TempDir::new();
    let (out, _) = run(&home, &["app"], &[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(
        text(&out.stderr).contains("--static-dir"),
        "{}",
        text(&out.stderr)
    );
}

#[test]
fn app_help_names_static_dir_and_hides_the_self_test() {
    let home = TempDir::new();
    let (out, _) = run(&home, &["app", "--help"], &[]);
    assert_eq!(out.status.code(), Some(0));
    let help = text(&out.stdout);
    assert!(help.contains("--static-dir"), "{help}");
    assert!(!help.contains("self-test"), "{help}");
    let (out, _) = run(&home, &["--help"], &[]);
    let help = text(&out.stdout);
    assert!(
        help.contains("app") && !help.contains("self-test"),
        "{help}"
    );
}

#[cfg(not(feature = "window"))]
mod without_the_window_feature {
    use super::*;

    const LINE: &str = "this build has no window: rebuild with --features window";

    #[test]
    fn app_prints_one_line_without_a_path_and_exits_4() {
        let home = TempDir::new();
        let dir = static_dir(&home);
        for args in [
            vec!["app", "--static-dir", dir.as_str()],
            vec!["app", "--self-test", "--static-dir", dir.as_str()],
            vec!["app", "--static-dir", "/this/folder/does/not/exist"],
        ] {
            let (out, _) = run(&home, &args, &[("DISPLAY", ":0")]);
            assert_eq!(out.status.code(), Some(4), "{args:?}");
            assert!(out.stdout.is_empty());
            let err = text(&out.stderr);
            assert_eq!(err.trim_end(), LINE);
            assert!(!err.contains(&home.0.to_string_lossy().to_string()));
        }
    }
}

#[cfg(feature = "window")]
mod with_the_window_feature {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpStream;
    use std::process::Stdio;

    fn one_line(out: &Output) -> String {
        let err = text(&out.stderr);
        assert_eq!(err.trim_end().lines().count(), 1, "{err}");
        err.trim_end().to_string()
    }

    #[test]
    fn a_folder_that_is_missing_or_a_file_exits_1_before_the_display_is_looked_at() {
        let home = TempDir::new();
        let file = home.0.join("a-file");
        fs::write(&file, "x").unwrap();
        for bad in ["/this/folder/does/not/exist", file.to_str().unwrap()] {
            let (out, _) = run(&home, &["app", "--static-dir", bad], &[]);
            assert_eq!(out.status.code(), Some(1), "{bad}");
            assert!(out.stdout.is_empty());
            assert_eq!(
                one_line(&out),
                "easy-personal-rs: --static-dir is not a readable directory"
            );
        }
    }

    #[test]
    fn no_display_variable_at_all_exits_5_within_a_second_with_one_line() {
        let home = TempDir::new();
        let dir = static_dir(&home);
        let (out, took) = run(&home, &["app", "--static-dir", &dir], &[]);
        assert_eq!(out.status.code(), Some(5), "{}", text(&out.stderr));
        assert!(took < Duration::from_secs(1), "{took:?}");
        assert!(out.stdout.is_empty());
        let line = one_line(&out);
        assert!(
            line.starts_with(
                "easy-personal-rs: no display: neither WAYLAND_DISPLAY nor DISPLAY is set"
            ),
            "{line}"
        );
        // An empty value counts as unset.
        let (out, _) = run(
            &home,
            &["app", "--static-dir", &dir],
            &[("DISPLAY", ""), ("WAYLAND_DISPLAY", "")],
        );
        assert_eq!(out.status.code(), Some(5));
    }

    #[test]
    fn a_dead_x_display_exits_5_naming_the_variable_and_no_path() {
        let home = TempDir::new();
        let dir = static_dir(&home);
        let (out, took) = run(
            &home,
            &["app", "--static-dir", &dir],
            &[("DISPLAY", ":4242"), ("WAYLAND_DISPLAY", "")],
        );
        assert_eq!(out.status.code(), Some(5), "{}", text(&out.stderr));
        assert!(took < Duration::from_secs(5), "{took:?}");
        assert!(out.stdout.is_empty());
        let line = one_line(&out);
        assert!(
            line.contains("DISPLAY is set but no X server answers on it"),
            "{line}"
        );
        assert!(
            !line.contains(".X11-unix") && !line.contains("4242"),
            "{line}"
        );
    }

    #[test]
    fn a_dead_tcp_display_ends_within_five_seconds() {
        // T016's hang: GTK waited for ever on `DISPLAY=127.0.0.1:99`. The preflight gives up after its time limit.
        let home = TempDir::new();
        let dir = static_dir(&home);
        let (out, took) = run(
            &home,
            &["app", "--static-dir", &dir],
            &[
                ("DISPLAY", "127.0.0.1:99"),
                ("WAYLAND_DISPLAY", ""),
                ("GDK_BACKEND", "x11"),
            ],
        );
        assert_eq!(out.status.code(), Some(5), "{}", text(&out.stderr));
        assert!(took < Duration::from_secs(5), "{took:?}");
        assert!(one_line(&out).contains("DISPLAY is set but no X server answers on it"));
    }

    #[test]
    fn a_dead_wayland_socket_exits_5_naming_the_variable() {
        let home = TempDir::new();
        let dir = static_dir(&home);
        let runtime = home.0.to_str().unwrap();
        let (out, _) = run(
            &home,
            &["app", "--static-dir", &dir],
            &[
                ("WAYLAND_DISPLAY", "wayland-none"),
                ("XDG_RUNTIME_DIR", runtime),
            ],
        );
        assert_eq!(out.status.code(), Some(5), "{}", text(&out.stderr));
        let line = one_line(&out);
        assert!(
            line.contains("WAYLAND_DISPLAY is set but no compositor answers on it"),
            "{line}"
        );
        assert!(
            !line.contains("wayland-none") && !line.contains(runtime),
            "{line}"
        );
    }

    /// RS-006: the feature build still serves `store-info`, `serve` and `mcp` with no display variable.
    #[test]
    fn store_info_serve_and_mcp_work_with_no_display() {
        let home = TempDir::new();
        let store = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../parity/store-v11.sqlite");
        fs::copy(store, home.0.join("store.sqlite")).unwrap();
        let (out, _) = run(&home, &["store-info", "--json"], &[("TZ", "UTC")]);
        assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
        assert!(text(&out.stdout).contains("\"user_version\":11"));

        // mcp: initialize and tools/list over stdio.
        let mut mcp = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
            .arg("mcp")
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("XDG_RUNTIME_DIR")
            .env("HOME", &home.0)
            .env("EASY_PERSONAL_HOME", &home.0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = mcp.stdin.take().unwrap();
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"t","version":"0"}}}}}}"#).unwrap();
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#
        )
        .unwrap();
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":2,"method":"tools/list"}}"#).unwrap();
        drop(stdin);
        let out = mcp.wait_with_output().unwrap();
        assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
        let lines: Vec<String> = text(&out.stdout).lines().map(str::to_string).collect();
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[0].contains("easy-personal-rs") && lines[1].contains("usage_summary"));
        assert!(out.stderr.is_empty());

        // serve: start, one authenticated request, stop with SIGINT.
        let mut serve = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
            .arg("serve")
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("XDG_RUNTIME_DIR")
            .env("HOME", &home.0)
            .env("EASY_PERSONAL_HOME", &home.0)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdout = BufReader::new(serve.stdout.take().unwrap());
        let mut first = String::new();
        stdout.read_line(&mut first).unwrap();
        let (port, token) = first
            .trim_end()
            .strip_prefix("dashboard: http://127.0.0.1:")
            .and_then(|rest| rest.split_once("/#"))
            .unwrap_or_else(|| panic!("unexpected first line {first:?}"));
        let mut stream = TcpStream::connect(("127.0.0.1", port.parse::<u16>().unwrap())).unwrap();
        write!(
            stream,
            "GET /api/version HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nx-dashboard-token: {token}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut answer = String::new();
        stream.read_to_string(&mut answer).unwrap();
        assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
        assert!(
            Command::new("kill")
                .args(["-INT", &serve.id().to_string()])
                .status()
                .unwrap()
                .success()
        );
        let status = serve.wait().unwrap();
        assert_eq!(status.code(), Some(0));
        let mut stderr = String::new();
        serve
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        assert!(stderr.is_empty(), "{stderr}");
    }
}
