//! `tray --show` when another tray holds the lock (T101): the hand-over, with no display and no real tray. The test holds
//! the tray lock itself in a temp `XDG_RUNTIME_DIR`, with the pid of a helper process it started (a plain `sleep`, a
//! stand-in that is not a tray, or a shell named `easy-personal-rs` running `tray` that records a `SIGUSR1`). `HOME` and
//! `EASY_PERSONAL_HOME` point into the temp folder; no tray, window or process of the owner is touched.
#![cfg(feature = "window")]

use std::fs::{self, File};
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t101-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn runtime(&self) -> PathBuf {
        let dir = self.0.join("run");
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The lock file held by this test (the "running tray"), holding `pid` as text when given.
    fn hold_lock(&self, pid: Option<u32>) -> File {
        let mut lock = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.runtime().join("easy-personal-tray.lock"))
            .unwrap();
        lock.try_lock().unwrap();
        if let Some(pid) = pid {
            write!(lock, "{pid}").unwrap();
        }
        lock
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn tray_show(home: &TempDir, extra: &[&str]) -> (Output, Duration) {
    let page = home.0.join("page");
    fs::create_dir_all(&page).unwrap();
    fs::write(page.join("index.html"), "<!doctype html>").unwrap();
    let began = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
        .arg("tray")
        .arg("--static-dir")
        .arg(&page)
        .args(extra)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env("XDG_RUNTIME_DIR", home.runtime())
        .env("HOME", &home.0)
        .env("EASY_PERSONAL_HOME", &home.0)
        .output()
        .unwrap();
    (out, began.elapsed())
}

fn one_line(out: &Output) -> String {
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(err.trim_end().lines().count(), 1, "{err}");
    assert!(
        out.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    err.trim_end().to_string()
}

fn wait_for(path: &Path) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if path.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

/// AC 5: the lock names a live process that is not a tray: no signal (the `sleep` would end on `SIGUSR1`), one line, 1.
#[test]
fn a_pid_that_is_not_a_tray_gets_no_signal() {
    let home = TempDir::new();
    let mut other = Command::new("sleep").arg("30").spawn().unwrap();
    let _lock = home.hold_lock(Some(other.id()));
    let (out, _) = tray_show(&home, &["--show"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let line = one_line(&out);
    assert!(
        line.starts_with("easy-personal-rs: ") && line.contains("not a tray"),
        "{line}"
    );
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        other.try_wait().unwrap().is_none(),
        "the process was not signalled"
    );
    other.kill().unwrap();
    other.wait().unwrap();
}

/// AC 5: a held lock with no pid (a tray of an older version): no signal, one line, exit 1 after the wait.
#[test]
fn a_lock_with_no_pid_is_exit_1_after_the_wait() {
    let home = TempDir::new();
    let _lock = home.hold_lock(None);
    let (out, took) = tray_show(&home, &["--show"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        took >= Duration::from_millis(1900),
        "waited for the pid: {took:?}"
    );
    assert!(took < Duration::from_secs(8), "{took:?}");
    let line = one_line(&out);
    assert!(line.contains("no pid"), "{line}");
}

/// Without `--show` a held lock is the old message and exit 0 (unchanged), pid or none.
#[test]
fn without_show_a_held_lock_is_unchanged() {
    let home = TempDir::new();
    let _lock = home.hold_lock(None);
    let (out, took) = tray_show(&home, &[]);
    assert_eq!(out.status.code(), Some(0));
    assert!(took < Duration::from_secs(1), "{took:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim_end(),
        "easy-personal-rs: a tray already runs for this user"
    );
}

/// The pid names a process whose command line is `easy-personal-rs tray` (a shell standing in for the tray, with a trap
/// that records `SIGUSR1`): the signal arrives and `tray --show` exits 0.
#[test]
fn a_running_tray_gets_sigusr1_and_the_ask_exits_0() {
    let home = TempDir::new();
    let work = home.0.join("fake");
    fs::create_dir_all(&work).unwrap();
    fs::write(
        work.join("tray"),
        "trap 'echo got > got' USR1\necho ready > ready\nwhile :; do sleep 0.05; done\n",
    )
    .unwrap();
    // argv = ["easy-personal-rs", "tray"]: `sh` runs the script file `tray` in its working folder
    let mut fake: Child = Command::new("sh")
        .arg0("easy-personal-rs")
        .arg("tray")
        .current_dir(&work)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    assert!(wait_for(&work.join("ready")), "the stand-in did not start");
    let cmdline = fs::read(format!("/proc/{}/cmdline", fake.id())).unwrap();
    assert!(
        cmdline.starts_with(b"easy-personal-rs\0tray\0"),
        "{cmdline:?}"
    );
    let _lock = home.hold_lock(Some(fake.id()));
    let (out, _) = tray_show(&home, &["--show"]);
    let signalled = wait_for(&work.join("got"));
    fake.kill().unwrap();
    fake.wait().unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty() && out.stderr.is_empty());
    assert!(signalled, "SIGUSR1 reached the stand-in tray");
}
