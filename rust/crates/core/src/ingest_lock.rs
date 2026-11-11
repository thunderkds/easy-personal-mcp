//! Single-writer lock for ingest, the same primitive as Python's `core/lock.py`.
//!
//! Python takes `fcntl.flock(handle, LOCK_EX | LOCK_NB)` on `<data_dir>/ingest.lock` opened with mode `"a"`,
//! polling every 0.05 s until its timeout. `std::fs::File::try_lock` is `flock(2)` with `LOCK_EX | LOCK_NB` on
//! Linux (not a POSIX `fcntl` record lock, which would not exclude an `flock` holder), so the Rust and Python
//! ingests exclude each other. The kernel drops the lock when the holder exits or crashes; the file is never deleted.

use std::fmt;
use std::fs::{self, DirBuilder, File, OpenOptions, TryLockError};
use std::io;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, Instant};

/// `POLL_SECONDS` in `core/lock.py`.
pub const POLL: Duration = Duration::from_millis(50);
/// `DEFAULT_TIMEOUT` in `core/lock.py` (300 s).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);
/// The lock file's name inside the data dir.
pub const LOCK_FILE: &str = "ingest.lock";

#[derive(Debug)]
pub enum LockError {
    /// Another ingest still holds the lock after the timeout (Python's `LockBusy`).
    Busy,
    /// The data dir or the lock file could not be made or opened (the I/O error kind only, never the path).
    Unavailable(io::ErrorKind),
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockError::Busy => write!(f, "another ingest is running; try again later"),
            LockError::Unavailable(kind) => write!(f, "cannot take the ingest lock: {kind:?}"),
        }
    }
}

impl std::error::Error for LockError {}

/// Holds the lock until dropped.
#[derive(Debug)]
pub struct LockGuard {
    file: File,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // `fcntl.flock(handle, fcntl.LOCK_UN)` in Python's `finally`; closing the file would release it too.
        let _ = self.file.unlock();
    }
}

/// Take the exclusive ingest lock on `<data_dir>/ingest.lock`, waiting up to `timeout` (0: one try).
/// `data_dir` is created first like Python's `config.data_dir()`.
pub fn ingest_lock(data_dir: &Path, timeout: Duration) -> Result<LockGuard, LockError> {
    create_data_dir(data_dir).map_err(|e| LockError::Unavailable(e.kind()))?;
    // `open(path, "a")`: append, created if missing, never truncated or deleted.
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(data_dir.join(LOCK_FILE))
        .map_err(|e| LockError::Unavailable(e.kind()))?;
    let deadline = Instant::now() + timeout;
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(LockGuard { file }),
            Err(TryLockError::WouldBlock) => {
                if Instant::now() >= deadline {
                    return Err(LockError::Busy);
                }
                sleep(POLL);
            }
            Err(TryLockError::Error(e)) => return Err(LockError::Unavailable(e.kind())),
        }
    }
}

/// `config.data_dir()`: `mkdir(mode=0o700, parents=True)`, then `chmod(0o700)` because the umask masks
/// mkdir's mode. A folder that already exists is used as is: its mode is not ours to change.
fn create_data_dir(dir: &Path) -> io::Result<()> {
    let made = match DirBuilder::new().mode(0o700).create(dir) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            if let Some(parent) = dir.parent() {
                fs::create_dir_all(parent)?;
            }
            DirBuilder::new().mode(0o700).create(dir)
        }
        other => other,
    };
    match made {
        Ok(()) => fs::set_permissions(dir, fs::Permissions::from_mode(0o700)),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }
}
