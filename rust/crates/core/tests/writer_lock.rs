//! T027: `ingest_lock::ingest_lock` — the same `flock(2)` as Python's `core/lock.py`, proven against other processes:
//! util-linux `flock(1)`, the kernel's `/proc/locks`, and `python3` holding `fcntl.flock` (Python's own call) or a POSIX
//! `fcntl.lockf` lock (which must NOT exclude it). Temp folders only.

use easy_personal_core::ingest_lock::{LOCK_FILE, LockError, ingest_lock};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t027-lock-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn data(&self) -> PathBuf {
        self.0.join("data")
    }

    fn lock_file(&self) -> PathBuf {
        self.data().join(LOCK_FILE)
    }

    fn ready(&self) -> PathBuf {
        self.0.join("ready")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Wait until the holder process has written its ready file (it holds the lock from then on).
fn wait_ready(dir: &TempDir, child: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !dir.ready().exists() {
        assert!(child.try_wait().unwrap().is_none(), "holder exited early");
        assert!(Instant::now() < deadline, "holder never got the lock");
        sleep(Duration::from_millis(10));
    }
}

/// `flock -x <lock> -c 'touch <ready>; sleep <secs>'` (util-linux: `flock(2)`, the call Python makes).
fn flock_holder(dir: &TempDir, secs: f64) -> Child {
    fs::create_dir_all(dir.data()).unwrap();
    let mut child = Command::new("timeout")
        .args(["30", "flock", "-x"])
        .arg(dir.lock_file())
        .arg("-c")
        .arg(format!("touch '{}'; sleep {secs}", dir.ready().display()))
        .spawn()
        .expect("flock(1) from util-linux");
    wait_ready(dir, &mut child);
    child
}

/// `flock -n -x <lock> -c true`: 0 when it got the lock at once, 1 when it is held.
fn flock_try(dir: &TempDir) -> i32 {
    Command::new("timeout")
        .args(["10", "flock", "-n", "-x"])
        .arg(dir.lock_file())
        .args(["-c", "true"])
        .status()
        .unwrap()
        .code()
        .unwrap()
}

/// `python3` holding the lock with `call` (`flock` or `lockf`) until killed.
fn python_holder(dir: &TempDir, call: &str) -> Child {
    fs::create_dir_all(dir.data()).unwrap();
    let script = format!(
        "import fcntl, sys, time\n\
         h = open(sys.argv[1], 'a')\n\
         fcntl.{call}(h, fcntl.LOCK_EX | fcntl.LOCK_NB)\n\
         open(sys.argv[2], 'w').close()\n\
         time.sleep(30)\n"
    );
    // Spawned directly (no `timeout` wrapper) so `kill()` hits the holder itself; the script ends by itself after 30 s.
    let mut child = Command::new("python3")
        .args(["-I", "-c", &script])
        .arg(dir.lock_file())
        .arg(dir.ready())
        .stdout(Stdio::null())
        .spawn()
        .expect("python3");
    wait_ready(dir, &mut child);
    child
}

/// The `/proc/locks` lines on the lock file's inode (`<n>: FLOCK ADVISORY WRITE <pid> <maj>:<min>:<inode> ...`).
fn kernel_locks(path: &Path) -> Vec<String> {
    let inode = fs::metadata(path).unwrap().ino();
    fs::read_to_string("/proc/locks")
        .unwrap()
        .lines()
        .filter(|l| {
            l.split_whitespace()
                .nth(5)
                .and_then(|dev| dev.rsplit(':').next())
                .is_some_and(|i| i == inode.to_string())
        })
        .map(str::to_owned)
        .collect()
}

/// Criterion 6: another process holding a real `flock` makes the lock `Busy` until it ends, then it succeeds.
#[test]
fn a_flock_holder_in_another_process_excludes_the_writer() {
    let dir = TempDir::new();
    let mut holder = flock_holder(&dir, 1.5);
    let started = Instant::now();
    let err = ingest_lock(&dir.data(), Duration::from_millis(300)).unwrap_err();
    let waited = started.elapsed();
    assert!(matches!(err, LockError::Busy), "{err:?}");
    assert_eq!(
        err.to_string(),
        "another ingest is running; try again later"
    );
    assert!(
        waited >= Duration::from_millis(300),
        "gave up after {waited:?}"
    );
    assert!(
        ingest_lock(&dir.data(), Duration::ZERO).is_err(),
        "timeout 0: one try"
    );
    assert!(holder.wait().unwrap().success());
    let guard = ingest_lock(&dir.data(), Duration::ZERO).expect("free after the holder ended");
    drop(guard);
    assert!(dir.lock_file().exists(), "the lock file is never deleted");
}

/// Criterion 6: the wait polls, so a holder that ends inside the timeout lets the writer in.
#[test]
fn the_writer_waits_for_a_holder_that_ends_inside_the_timeout() {
    let dir = TempDir::new();
    let mut holder = flock_holder(&dir, 0.8);
    let started = Instant::now();
    let guard =
        ingest_lock(&dir.data(), Duration::from_secs(10)).expect("got it after the release");
    let waited = started.elapsed();
    assert!(
        waited >= Duration::from_millis(300),
        "did not wait: {waited:?}"
    );
    assert!(
        waited < Duration::from_secs(5),
        "waited too long: {waited:?}"
    );
    drop(guard);
    holder.wait().unwrap();
}

/// Criterion 6: the writer's lock is an `flock` in the kernel's table (not `POSIX`), excludes `flock(1)` while held,
/// and is released when the guard drops.
#[test]
fn the_writer_holds_a_kernel_flock_until_the_guard_drops() {
    let dir = TempDir::new();
    let guard = ingest_lock(&dir.data(), Duration::ZERO).unwrap();
    let locks = kernel_locks(&dir.lock_file());
    println!("/proc/locks while held: {locks:?}");
    assert_eq!(locks.len(), 1, "{locks:?}");
    let fields: Vec<&str> = locks[0].split_whitespace().collect();
    assert_eq!(&fields[1..4], ["FLOCK", "ADVISORY", "WRITE"], "{locks:?}");
    assert_eq!(fields[4], std::process::id().to_string());
    assert_eq!(flock_try(&dir), 1, "flock(1) got a held lock");
    drop(guard);
    assert!(kernel_locks(&dir.lock_file()).is_empty());
    assert_eq!(flock_try(&dir), 0, "the guard did not release");
}

/// Criterion 6: Python's own call (`fcntl.flock`) excludes the writer; the kernel drops it when that process dies.
#[test]
fn python_fcntl_flock_excludes_the_writer_until_the_holder_dies() {
    let dir = TempDir::new();
    let mut holder = python_holder(&dir, "flock");
    assert!(matches!(
        ingest_lock(&dir.data(), Duration::from_millis(200)),
        Err(LockError::Busy)
    ));
    holder.kill().unwrap(); // SIGKILL: no unlock runs in the holder
    holder.wait().unwrap();
    ingest_lock(&dir.data(), Duration::from_secs(2)).expect("released on process death");
}

/// Criterion 6, negative control: a POSIX record lock (`fcntl.lockf`) does not exclude the writer, so the writer is
/// not using POSIX locks (which would not exclude Python's `flock` either).
#[test]
fn a_posix_record_lock_does_not_exclude_the_writer() {
    let dir = TempDir::new();
    let mut holder = python_holder(&dir, "lockf");
    let got = ingest_lock(&dir.data(), Duration::ZERO);
    holder.kill().unwrap();
    holder.wait().unwrap();
    got.expect("a POSIX lock must not block an flock");
}

/// Criterion 6: the data dir is created 0700 (chmod after mkdir, like `config.data_dir()`), an existing one keeps its
/// mode, the lock file is opened for append (never truncated) and survives a busy attempt.
#[test]
fn the_data_dir_and_the_lock_file_follow_python() {
    let dir = TempDir::new();
    let nested = dir.0.join("a/b/data");
    drop(ingest_lock(&nested, Duration::ZERO).unwrap());
    let mode = fs::metadata(&nested).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
    assert!(nested.join(LOCK_FILE).exists());

    let existing = dir.0.join("existing");
    fs::create_dir(&existing).unwrap();
    fs::set_permissions(&existing, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(existing.join(LOCK_FILE), b"kept").unwrap();
    drop(ingest_lock(&existing, Duration::ZERO).unwrap());
    assert_eq!(
        fs::metadata(&existing).unwrap().permissions().mode() & 0o777,
        0o755
    );
    assert_eq!(fs::read(existing.join(LOCK_FILE)).unwrap(), b"kept");

    let mut holder = flock_holder(&dir, 0.5);
    assert!(ingest_lock(&dir.data(), Duration::ZERO).is_err());
    assert!(dir.lock_file().exists());
    holder.wait().unwrap();
}
