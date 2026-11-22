//! Terminal sessions (T133, FR-136..FR-138, DDR-0009): the owner's own interactive `claude` in a PTY the app owns, beside
//! the stream-json app sessions of `app_session.rs` (which this module does not change).
//!
//! - **The PTY** (Path A of `BRAINSTORMING_LOG_TERMINAL.md`, no PTY crate): `posix_openpt(O_RDWR | O_NOCTTY | O_CLOEXEC)`
//!   and the slave opened with `O_CLOEXEC`, so no other child of this process (an app session, `git`) inherits either end
//!   in the moment before a flag is set (`openpty` gives fds that are not close-on-exec). The child gets the slave as
//!   stdin, stdout and stderr; in `pre_exec` it calls `setsid` (its own session leader), `ioctl(TIOCSCTTY)` (the slave is
//!   its controlling tty) and `PR_SET_PDEATHSIG` (SIGKILL, like app sessions: it is started from the thread that waits
//!   for it, so quitting the tray ends it; no daemon).
//! - **The child**: `claude` with fixed words only (a model from `MODELS`, a mode from `MODES`; no prompt, no free
//!   argument, FR-116), in a stored repo or a new worktree, with [`child_env`] (the FR-102 scrub) plus
//!   `TERM=xterm-256color` and `COLORTERM=truecolor`. `MCP_TOOL_TIMEOUT` is not set: it exists for the permission tool of
//!   an app session, which a terminal session has not, and the owner's MCP servers keep their own timeout (full parity).
//! - **Out**: a reader thread copies the PTY bytes into the first [`PREFIX`] bytes plus a [`RING`] ring and sends them to
//!   every attached page (`tokio::sync::broadcast`). The bytes are never decoded, stored or logged (NFR-002 sixth
//!   exception, FR-138): there is no `Debug` on a type that holds them. They are dropped when the session has ended and
//!   its last page detached, when it is pushed out of the list, and at exit ([`Terms::shutdown`]).
//! - **In**: the pages' typed bytes go to a writer thread in order; over [`INPUT_PER_SECOND`] per session they are
//!   dropped (the caller tells the page once per second). The last resize wins (FR-137, question 47).
//! - **Stop**: the master is closed (the kernel sends the child SIGHUP), and the process group gets SIGKILL after 5 s.
//! - **Live**: the list names the Claude session id once `claude` has written `<config dir>/sessions/<pid>.json` for the
//!   child's pid (read with `session_procs::read_all`, the Live view's own reader), so the page can merge the row.
//! - **Limits**: one cap for app and terminal sessions ([`Sessions::take_slot`], question 41).

use crate::app_session::{
    Account, MAX_KEPT, ResumeId, SETTING_EVERY, Sessions, Slot, StartError, child_env,
    make_worktree, random_hex, signal_group,
};
use crate::claude_cli::find_claude;
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::ffi::{CStr, OsString};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

/// The first output bytes kept beside the ring (private modes set at start survive a wrap, FR-138).
pub(crate) const PREFIX: usize = 4 * 1024;
/// The ring of later output, per session.
pub(crate) const RING: usize = 1 << 20;
/// Typed bytes per second per session; more are dropped (FR-137).
pub(crate) const INPUT_PER_SECOND: usize = 64 * 1024;
/// The size limits of a resize (FR-137); a zero is ignored.
pub(crate) const COLS: (u16, u16) = (10, 500);
pub(crate) const ROWS: (u16, u16) = (5, 200);
/// The size before the first page tells its own (the T130 spike's).
const START_SIZE: (u16, u16) = (120, 32);
const STOP_GRACE: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(50);
/// How often the reader and the writer look whether the master is being closed.
const WAKE: Duration = Duration::from_millis(100);
/// Messages a page may be behind before it is closed (it attaches again and gets the replay).
const BACKLOG: usize = 256;
/// Typed chunks waiting for the writer.
const QUEUE: usize = 256;

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

// ---- the PTY ----------------------------------------------------------------------------------------------------------

fn check(ret: libc::c_int) -> io::Result<libc::c_int> {
    if ret == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ret)
    }
}

fn set_size(master: RawFd, cols: u16, rows: u16) -> io::Result<()> {
    let size = libc::winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: TIOCSWINSZ reads one `winsize` from the pointer
    check(unsafe { libc::ioctl(master, libc::TIOCSWINSZ, &size) }).map(|_| ())
}

/// A new PTY pair `(master, slave)`, both close-on-exec, the master non-blocking, of the given size.
pub(crate) fn open_pty(cols: u16, rows: u16) -> io::Result<(OwnedFd, OwnedFd)> {
    // SAFETY: plain libc calls on a fd this function owns; `name` is a NUL-terminated buffer `ptsname_r` fills
    unsafe {
        let fd = check(libc::posix_openpt(
            libc::O_RDWR | libc::O_NOCTTY | libc::O_CLOEXEC,
        ))?;
        let master = OwnedFd::from_raw_fd(fd);
        check(libc::grantpt(fd))?;
        check(libc::unlockpt(fd))?;
        let mut name = [0 as libc::c_char; 128];
        let ret = libc::ptsname_r(fd, name.as_mut_ptr(), name.len());
        if ret != 0 {
            return Err(io::Error::from_raw_os_error(ret));
        }
        let path = CStr::from_ptr(name.as_ptr());
        let slave = OwnedFd::from_raw_fd(check(libc::open(
            path.as_ptr(),
            libc::O_RDWR | libc::O_NOCTTY | libc::O_CLOEXEC,
        ))?);
        let flags = check(libc::fcntl(fd, libc::F_GETFL))?;
        check(libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK))?;
        set_size(fd, cols, rows)?;
        Ok((master, slave))
    }
}

/// Starts `cmd` on the slave as its own session leader with the slave as its controlling tty and `PR_SET_PDEATHSIG`.
/// Call it from the thread that waits for the child (the death signal follows the thread).
pub(crate) fn spawn_on(mut cmd: Command, slave: &OwnedFd) -> io::Result<Child> {
    cmd.stdin(Stdio::from(slave.try_clone()?))
        .stdout(Stdio::from(slave.try_clone()?))
        .stderr(Stdio::from(slave.try_clone()?));
    let parent = std::process::id() as libc::pid_t;
    // SAFETY: only async-signal-safe calls (`setsid`, `ioctl`, `prctl`, `getppid`) between fork and exec, no allocation;
    // std has put the slave on fds 0, 1 and 2 before this runs
    unsafe {
        cmd.pre_exec(move || {
            if libc::setsid() == -1 || libc::ioctl(0, libc::TIOCSCTTY, 0) == -1 {
                return Err(io::Error::last_os_error());
            }
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL as libc::c_ulong) == -1 {
                return Err(io::Error::last_os_error());
            }
            if libc::getppid() != parent {
                return Err(io::Error::from_raw_os_error(libc::ESRCH));
            }
            Ok(())
        })
    };
    cmd.spawn() // `cmd` (and its copies of the slave) is dropped here
}

/// Waits until `fd` is ready for `events` or `WAKE` passes.
fn wait_for(fd: RawFd, events: libc::c_short) -> io::Result<bool> {
    let mut poll = libc::pollfd {
        fd,
        events,
        revents: 0,
    };
    // SAFETY: one `pollfd`
    match unsafe { libc::poll(&mut poll, 1, WAKE.as_millis() as libc::c_int) } {
        -1 => {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::Interrupted {
                Ok(false)
            } else {
                Err(e)
            }
        }
        0 => Ok(false),
        _ => Ok(true),
    }
}

// ---- the plan ---------------------------------------------------------------------------------------------------------

/// What a terminal start asked for, already checked by the form of `POST /api/app-sessions` and the store.
pub(crate) struct Plan {
    pub repo: PathBuf,
    pub account: Option<Account>,
    pub model: &'static str,
    pub mode: &'static str,
    pub worktree: bool,
}

/// The whole argument list after `claude`: a listed model and mode, nothing the owner typed.
pub(crate) fn argv(plan: &Plan) -> Vec<String> {
    let mut args = Vec::new();
    if plan.model != "default" {
        args.extend(["--model".into(), plan.model.into()]);
    }
    match plan.mode {
        "default" => {}
        "bypassPermissions" => args.push("--dangerously-skip-permissions".into()),
        mode => args.extend(["--permission-mode".into(), mode.into()]),
    }
    args
}

/// The child's environment: [`child_env`] without the app session's `MCP_TOOL_TIMEOUT`, plus the terminal type.
pub(crate) fn term_env(
    env: &[(OsString, OsString)],
    config_dir: Option<&str>,
) -> Vec<(OsString, OsString)> {
    let mut out: Vec<(OsString, OsString)> = child_env(env, config_dir)
        .into_iter()
        .filter(|(k, _)| k != "MCP_TOOL_TIMEOUT" && k != "TERM" && k != "COLORTERM")
        .collect();
    out.push(("TERM".into(), "xterm-256color".into()));
    out.push(("COLORTERM".into(), "truecolor".into()));
    out
}

// ---- one session ------------------------------------------------------------------------------------------------------

/// What an attached page is sent.
#[derive(Clone)]
pub(crate) enum Out {
    Bytes(Arc<[u8]>),
    Exit(Option<i32>),
    Pages(usize),
}

/// The first bytes and the ring (FR-138): never more than [`PREFIX`] + [`RING`].
#[derive(Default)]
struct Screen {
    prefix: Vec<u8>,
    ring: VecDeque<u8>,
}

impl Screen {
    fn push(&mut self, mut bytes: &[u8]) {
        let room = PREFIX - self.prefix.len();
        if room > 0 {
            let take = room.min(bytes.len());
            self.prefix.extend_from_slice(&bytes[..take]);
            bytes = &bytes[take..];
        }
        if bytes.len() >= RING {
            self.ring.clear();
            bytes = &bytes[bytes.len() - RING..];
        } else {
            let over = (self.ring.len() + bytes.len()).saturating_sub(RING);
            self.ring.drain(..over);
        }
        self.ring.extend(bytes);
    }

    fn replay(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.prefix.len() + self.ring.len());
        out.extend_from_slice(&self.prefix);
        out.extend(self.ring.iter());
        out
    }

    #[cfg(test)]
    fn held(&self) -> usize {
        self.prefix.len() + self.ring.len()
    }

    fn clear(&mut self) {
        *self = Screen::default();
    }
}

struct State {
    screen: Screen,
    /// `Some(code)` once the child has ended (`None` inside: no exit code, a signal or nothing started).
    exit: Option<Option<i32>>,
    reason: Option<&'static str>,
    pages: usize,
    size: (u16, u16),
    branch: Option<String>,
}

/// The input cap's window.
struct Typed {
    since: Instant,
    used: usize,
    noticed: bool,
}

pub(crate) struct Term {
    pub(crate) id: String,
    repo: PathBuf,
    account: Option<(String, String)>,
    started: String,
    /// The account's config folder, where `claude` writes `sessions/<pid>.json` (`None`: no `HOME` to find it).
    config: Option<PathBuf>,
    /// The Claude session id, once found in that file (it does not change for the life of the process).
    session_id: Mutex<Option<String>>,
    state: Mutex<State>,
    tx: broadcast::Sender<Out>,
    /// The session's handle on the master; taken (and so closed, once the reader and the writer let go) at a stop.
    master: Mutex<Option<Arc<OwnedFd>>>,
    input: Mutex<Option<SyncSender<Vec<u8>>>>,
    typed: Mutex<Typed>,
    /// Set when the master is being closed: the reader and the writer end at their next wake.
    closing: Arc<AtomicBool>,
    pgid: AtomicI32,
    stop: Mutex<Option<(Instant, &'static str)>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Term {
    fn new(id: String, plan: &Plan, config: Option<PathBuf>) -> Term {
        Term {
            id,
            config,
            session_id: Mutex::new(None),
            repo: plan.repo.clone(),
            account: plan
                .account
                .as_ref()
                .map(|a| (a.id.clone(), a.label.clone())),
            started: crate::agent_run::utc_now(),
            state: Mutex::new(State {
                screen: Screen::default(),
                exit: None,
                reason: None,
                pages: 0,
                size: START_SIZE,
                branch: None,
            }),
            tx: broadcast::channel(BACKLOG).0,
            master: Mutex::new(None),
            input: Mutex::new(None),
            typed: Mutex::new(Typed {
                since: Instant::now(),
                used: 0,
                noticed: false,
            }),
            closing: Arc::default(),
            pgid: AtomicI32::new(0),
            stop: Mutex::new(None),
            thread: Mutex::new(None),
        }
    }

    pub(crate) fn ended(&self) -> bool {
        lock(&self.state).exit.is_some()
    }

    /// The Claude session id of the running child: from its process file, once that exists.
    fn session_id(&self) -> Option<String> {
        let mut known = lock(&self.session_id);
        if known.is_none() && !self.ended() {
            let pid = self.pgid.load(Ordering::SeqCst);
            if let Some(config) = self.config.as_ref().filter(|_| pid > 0) {
                *known = crate::session_procs::read_all(std::slice::from_ref(config))
                    .into_iter()
                    .find(|(id, info)| info.pid == pid && ResumeId::new(id).is_some())
                    .map(|(id, _)| id);
            }
        }
        known.clone()
    }

    /// The list entry (FR-136): no screen byte.
    pub(crate) fn json(&self) -> Value {
        let session_id = self.session_id();
        let state = lock(&self.state);
        json!({
            "id": self.id,
            "repo": self.repo.to_string_lossy(),
            "branch": state.branch,
            "account": self.account.as_ref().map(|(id, label)| json!({"id": id, "label": label})),
            "started": self.started,
            "state": if state.exit.is_some() { "exited" } else { "running" },
            "code": state.exit.flatten(),
            "reason": state.reason,
            "pages": state.pages,
            "session_id": session_id,
        })
    }

    /// A page attaches: what it must draw first (the prefix and the ring), its feed from now on, and the exit code when
    /// the child has already ended. Every page is told the new page count.
    pub(crate) fn attach(&self) -> (Vec<u8>, broadcast::Receiver<Out>, Option<Option<i32>>) {
        let mut state = lock(&self.state);
        let rx = self.tx.subscribe();
        state.pages += 1;
        let _ = self.tx.send(Out::Pages(state.pages));
        (state.screen.replay(), rx, state.exit)
    }

    /// A page detached; after the end, the last one drops the bytes.
    pub(crate) fn detach(&self) {
        let mut state = lock(&self.state);
        state.pages = state.pages.saturating_sub(1);
        let _ = self.tx.send(Out::Pages(state.pages));
        if state.exit.is_some() && state.pages == 0 {
            state.screen.clear();
        }
    }

    /// The bytes held now (prefix + ring).
    #[cfg(test)]
    pub(crate) fn held(&self) -> usize {
        lock(&self.state).screen.held()
    }

    fn output(&self, bytes: &[u8]) {
        let mut state = lock(&self.state);
        state.screen.push(bytes);
        let _ = self.tx.send(Out::Bytes(Arc::from(bytes)));
    }

    /// Typed bytes, in order. `Err(true)`: over the cap (or the queue is full), dropped, and the page should be told
    /// (the first drop of this second); `Err(false)`: dropped, already told, or the session has ended.
    pub(crate) fn input(&self, bytes: Vec<u8>) -> Result<(), bool> {
        let mut typed = lock(&self.typed);
        if typed.since.elapsed() >= Duration::from_secs(1) {
            *typed = Typed {
                since: Instant::now(),
                used: 0,
                noticed: false,
            };
        }
        let over = |typed: &mut Typed| {
            let tell = !typed.noticed;
            typed.noticed = true;
            Err(tell)
        };
        if typed.used + bytes.len() > INPUT_PER_SECOND {
            return over(&mut typed);
        }
        let len = bytes.len();
        match lock(&self.input).as_ref().map(|tx| tx.try_send(bytes)) {
            Some(Ok(())) => {
                typed.used += len;
                Ok(())
            }
            Some(Err(TrySendError::Full(_))) => over(&mut typed),
            _ => Err(false),
        }
    }

    /// The last resize wins; sizes are clamped, a zero is ignored.
    pub(crate) fn resize(&self, cols: u64, rows: u64) {
        if cols == 0 || rows == 0 {
            return;
        }
        let clamp = |v: u64, (lo, hi): (u16, u16)| v.clamp(u64::from(lo), u64::from(hi)) as u16;
        let size = (clamp(cols, COLS), clamp(rows, ROWS));
        let mut state = lock(&self.state);
        state.size = size;
        if let Some(master) = lock(&self.master).as_ref() {
            let _ = set_size(master.as_raw_fd(), size.0, size.1);
        }
    }

    /// After a replay: one column off and back, so the program gets SIGWINCH and draws its whole screen again.
    pub(crate) fn nudge(&self) {
        let (cols, rows) = lock(&self.state).size;
        if let Some(master) = lock(&self.master).as_ref() {
            let fd = master.as_raw_fd();
            let other = if cols > COLS.0 { cols - 1 } else { cols + 1 };
            let _ = set_size(fd, other, rows);
            let _ = set_size(fd, cols, rows);
        }
    }

    #[cfg(test)]
    pub(crate) fn size(&self) -> (u16, u16) {
        lock(&self.state).size
    }

    /// Closes the master (the child gets SIGHUP); its supervisor sends SIGKILL to the group after the grace.
    pub(crate) fn request_stop(&self, reason: &'static str) {
        {
            let mut stop = lock(&self.stop);
            if stop.is_none() {
                *stop = Some((Instant::now(), reason));
            }
        }
        self.close_master();
    }

    fn close_master(&self) {
        self.closing.store(true, Ordering::SeqCst);
        lock(&self.master).take();
        lock(&self.input).take();
    }

    fn end(&self, code: Option<i32>, reason: &'static str) {
        let mut state = lock(&self.state);
        state.exit = Some(code);
        state.reason = Some(reason);
        // with no page attached the bytes stay for one that attaches later (until the entry is pushed out, or the app ends)
        let _ = self.tx.send(Out::Exit(code));
    }

    fn drop_bytes(&self) {
        lock(&self.state).screen.clear();
    }
}

/// Reads the master until the slave side is gone or the master is being closed.
fn read_loop(term: &Term, master: Arc<OwnedFd>, closing: &AtomicBool) {
    let fd = master.as_raw_fd();
    let mut buf = vec![0u8; 16 * 1024];
    while !closing.load(Ordering::SeqCst) {
        match wait_for(fd, libc::POLLIN) {
            Ok(false) => continue,
            Ok(true) => {}
            Err(_) => break,
        }
        // SAFETY: reads into `buf`, at most its length
        let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        if n > 0 {
            term.output(&buf[..n as usize]);
        } else if n == 0 {
            break;
        } else {
            let e = io::Error::last_os_error();
            if !matches!(
                e.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            ) {
                break; // EIO: every slave fd is closed
            }
        }
    }
}

/// Writes the typed chunks in order until the queue is closed or the master is being closed.
fn write_loop(rx: Receiver<Vec<u8>>, master: Arc<OwnedFd>, closing: &AtomicBool) {
    let fd = master.as_raw_fd();
    loop {
        let chunk = match rx.recv_timeout(WAKE) {
            Ok(chunk) => chunk,
            Err(RecvTimeoutError::Timeout) if !closing.load(Ordering::SeqCst) => continue,
            Err(_) => return,
        };
        let mut rest = &chunk[..];
        while !rest.is_empty() {
            if closing.load(Ordering::SeqCst) {
                return;
            }
            // SAFETY: writes from `rest`, at most its length
            let n = unsafe { libc::write(fd, rest.as_ptr().cast(), rest.len()) };
            if n > 0 {
                rest = &rest[n as usize..];
                continue;
            }
            let e = io::Error::last_os_error();
            match e.kind() {
                io::ErrorKind::WouldBlock => {
                    if wait_for(fd, libc::POLLOUT).is_err() {
                        return;
                    }
                }
                io::ErrorKind::Interrupted => {}
                _ => return,
            }
        }
    }
}

// ---- the registry -----------------------------------------------------------------------------------------------------

/// The terminal sessions of one server.
pub(crate) struct Terms {
    env: Vec<(OsString, OsString)>,
    list: Mutex<Vec<Arc<Term>>>,
    /// The app sessions, for the one running cap.
    slots: Arc<Sessions>,
    grace: Duration,
    poll: Duration,
}

impl Terms {
    pub(crate) fn new(env: Vec<(OsString, OsString)>, slots: Arc<Sessions>) -> Terms {
        Terms::timed(env, slots, STOP_GRACE, POLL)
    }

    pub(crate) fn timed(
        env: Vec<(OsString, OsString)>,
        slots: Arc<Sessions>,
        grace: Duration,
        poll: Duration,
    ) -> Terms {
        Terms {
            env,
            list: Mutex::new(Vec::new()),
            slots,
            grace,
            poll,
        }
    }

    pub(crate) fn all(&self) -> Vec<Arc<Term>> {
        lock(&self.list).clone()
    }

    pub(crate) fn get(&self, id: &str) -> Option<Arc<Term>> {
        lock(&self.list).iter().find(|t| t.id == id).cloned()
    }

    /// Starts a terminal session: the slot, the worktree (when asked), the PTY, the child. `Ok(id)`; a `claude` that is
    /// not found or does not start is an `Ok` entry that has exited with a fixed reason.
    pub(crate) fn start(
        &self,
        plan: Plan,
        dir: &Path,
        allowed: Box<dyn Fn() -> bool + Send>,
    ) -> Result<String, StartError> {
        let id = random_hex(8).ok_or(StartError::Internal)?;
        let var = |name: &str| {
            self.env
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.as_os_str())
        };
        // an absolute path: the child runs in the repo, and a relative `PATH` entry must not resolve there
        let claude =
            find_claude(var("PATH"), var("HOME")).and_then(|found| std::path::absolute(found).ok());
        let config = match plan.account.as_ref().and_then(|a| a.config_dir.as_deref()) {
            Some(dir) => Some(PathBuf::from(dir)),
            None => var("HOME").map(|home| Path::new(home).join(".claude")),
        };
        let slot = self.slots.take_slot().ok_or(StartError::Busy)?;
        let term = Arc::new(Term::new(id.clone(), &plan, config));
        let mut cwd = plan.repo.clone();
        if plan.worktree {
            let stamp = jiff::Timestamp::now().strftime("%Y%m%d-%H%M").to_string();
            let hex = random_hex(2).ok_or(StartError::Internal)?;
            let (folder, branch) =
                make_worktree(&plan.repo, dir, &stamp, &hex).map_err(StartError::Refused)?;
            lock(&term.state).branch = Some(branch);
            cwd = folder;
        }
        {
            let mut list = lock(&self.list);
            while list.iter().filter(|t| t.ended()).count() >= MAX_KEPT {
                if let Some(at) = list.iter().position(|t| t.ended()) {
                    list.remove(at).drop_bytes();
                }
            }
            list.push(Arc::clone(&term));
        }
        let Some(claude) = claude else {
            term.end(None, "claude not found");
            return Ok(id);
        };
        let mut cmd = Command::new(claude);
        cmd.args(argv(&plan))
            .current_dir(&cwd)
            .env_clear()
            .envs(term_env(
                &self.env,
                plan.account.as_ref().and_then(|a| a.config_dir.as_deref()),
            ));
        if self.launch(&term, cmd, slot, allowed).is_err() {
            term.end(None, "failed to start");
        }
        Ok(id)
    }

    fn launch(
        &self,
        term: &Arc<Term>,
        cmd: Command,
        slot: Slot,
        allowed: Box<dyn Fn() -> bool + Send>,
    ) -> io::Result<()> {
        let (master, slave) = open_pty(START_SIZE.0, START_SIZE.1)?;
        let master = Arc::new(master);
        let (ready, started) = mpsc::channel::<bool>();
        let (grace, poll) = (self.grace, self.poll);
        let own = Arc::clone(term);
        let for_thread = Arc::clone(&master);
        // the thread that starts the child lives as long as it (`PR_SET_PDEATHSIG` follows the thread)
        let handle = std::thread::Builder::new()
            .name("term-session".into())
            .spawn(move || {
                let child = spawn_on(cmd, &slave);
                drop(slave); // only the child holds the slave: its end is the master's EIO
                match child {
                    Ok(child) => {
                        own.pgid.store(child.id() as i32, Ordering::SeqCst);
                        let _ = ready.send(true);
                        supervise(&own, child, for_thread, slot, grace, poll, allowed);
                    }
                    Err(_) => {
                        drop(slot);
                        let _ = ready.send(false);
                    }
                }
            })?;
        *lock(&term.thread) = Some(handle);
        if started.recv() != Ok(true) {
            return Err(io::Error::other("not started"));
        }
        *lock(&term.master) = Some(master);
        Ok(())
    }

    /// At exit: every child is stopped (the master closed, SIGKILL after the grace), its thread joined, all bytes
    /// dropped.
    pub(crate) fn shutdown(&self) {
        let all = std::mem::take(&mut *lock(&self.list));
        for term in &all {
            term.request_stop("app closed");
        }
        for term in &all {
            let handle = lock(&term.thread).take();
            if let Some(handle) = handle {
                let _ = handle.join();
            }
            term.drop_bytes();
        }
    }
}

/// Runs on the thread that started the child, until it is gone: the reader and the writer on threads of their own, a
/// stop (master closed, then SIGKILL to the group), the setting check, and the end.
fn supervise(
    term: &Arc<Term>,
    mut child: Child,
    master: Arc<OwnedFd>,
    slot: Slot,
    grace: Duration,
    poll: Duration,
    allowed: Box<dyn Fn() -> bool + Send>,
) {
    let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(QUEUE);
    *lock(&term.input) = Some(tx);
    let reader = {
        let (own, master, closing) = (
            Arc::clone(term),
            Arc::clone(&master),
            Arc::clone(&term.closing),
        );
        std::thread::spawn(move || read_loop(&own, master, &closing))
    };
    let writer = {
        let closing = Arc::clone(&term.closing);
        std::thread::spawn(move || write_loop(rx, master, &closing))
    };
    let pgid = child.id() as i32;
    let mut last_check = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(_) => break None,
        }
        if last_check.elapsed() >= SETTING_EVERY {
            last_check = Instant::now();
            if !allowed() {
                term.request_stop("account turned off");
            }
        }
        if let Some((at, _)) = *lock(&term.stop)
            && at.elapsed() >= grace
        {
            signal_group(pgid, libc::SIGKILL);
        }
        std::thread::sleep(poll);
    };
    signal_group(pgid, libc::SIGKILL); // what the child left running in its session
    let _ = child.wait();
    // the last output first: the reader ends at the master's EIO (or is told to after a short wait)
    let deadline = Instant::now() + Duration::from_millis(500);
    while !reader.is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    term.close_master();
    let _ = reader.join();
    let _ = writer.join();
    let stop = *lock(&term.stop);
    let code = status.and_then(|s| s.code());
    term.end(
        code,
        match stop {
            Some((_, reason)) => reason,
            None => "exited",
        },
    );
    drop(slot);
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::agent_run::tests::{Fake, runs, serial};

    /// A stub `claude` for a PTY (tail of the `Fake` script, which logs argv, cwd, env and pid first): it prints its tty,
    /// answers each line, prints its size on SIGWINCH and logs a SIGHUP before it exits.
    pub(crate) const STUB: &str = r#"trap 'echo "winch $(stty size)"' WINCH
trap 'echo hup > "$L/hup"; exit 0' HUP
echo "tty $(tty)"
echo ready
while :; do
  if read line; then
    [ "$line" = quit ] && exit 7
    echo "got:$line"
  else sleep 0.1; fi
done"#;

    /// The same, but it ignores SIGHUP (only SIGKILL ends it).
    pub(crate) const DEAF: &str = r#"trap '' HUP
echo ready
while :; do
  if read line; then echo "got:$line"; else sleep 0.1; fi
done"#;

    fn until(what: &str, mut ok: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(20);
        while !ok() {
            assert!(Instant::now() < deadline, "never happened: {what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Alive now (not a zombie); `runs` waits up to 5 s for an end.
    fn alive(pid: &str) -> bool {
        std::fs::read_to_string(format!("/proc/{}/stat", pid.trim())).is_ok_and(|s| {
            !s.rsplit(')')
                .next()
                .unwrap_or("")
                .trim_start()
                .starts_with('Z')
        })
    }

    /// Fields of `/proc/<pid>/stat` after the name: `(state, ppid, pgrp, session, tty_nr)`.
    fn stat(pid: i32) -> (String, i32, i32, i32, i32) {
        let text = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
        let rest: Vec<&str> = text
            .rsplit(')')
            .next()
            .unwrap()
            .split_whitespace()
            .collect();
        let n = |i: usize| rest[i].parse::<i32>().unwrap();
        (rest[0].to_string(), n(1), n(2), n(3), n(4))
    }

    fn read_until(master: &OwnedFd, want: &str) -> String {
        let mut seen = Vec::new();
        let mut buf = [0u8; 4096];
        let deadline = Instant::now() + Duration::from_secs(10);
        while !String::from_utf8_lossy(&seen).contains(want) {
            assert!(
                Instant::now() < deadline,
                "never saw {want:?}: {:?}",
                String::from_utf8_lossy(&seen)
            );
            if wait_for(master.as_raw_fd(), libc::POLLIN).unwrap() {
                // SAFETY: reads into `buf`
                let n =
                    unsafe { libc::read(master.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len()) };
                if n > 0 {
                    seen.extend_from_slice(&buf[..n as usize]);
                }
            }
        }
        String::from_utf8_lossy(&seen).into_owned()
    }

    fn write_all(master: &OwnedFd, bytes: &[u8]) {
        // SAFETY: writes from `bytes`
        let n = unsafe { libc::write(master.as_raw_fd(), bytes.as_ptr().cast(), bytes.len()) };
        assert_eq!(n, bytes.len() as isize);
    }

    /// AC 0, the Step 0 gate (spike Q1, Q2, Q6 on Path A): `posix_openpt` + `pre_exec` (`setsid`, `TIOCSCTTY`,
    /// `PR_SET_PDEATHSIG`) gives a session leader with the slave as controlling tty; keys reach it; `TIOCSWINSZ`
    /// delivers SIGWINCH; closing the master sends SIGHUP and it exits.
    #[test]
    fn step_0_the_libc_pty_gives_a_session_leader_on_its_tty_keys_sigwinch_and_sighup_at_close() {
        let _serial = serial();
        let fake = Fake::new("t133-step0", STUB);
        let (master, slave) = open_pty(80, 24).unwrap();
        // both ends are close-on-exec, so no other child inherits them
        for fd in [master.as_raw_fd(), slave.as_raw_fd()] {
            // SAFETY: F_GETFD on an open fd
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
            assert_eq!(flags & libc::FD_CLOEXEC, libc::FD_CLOEXEC);
        }
        let (tx, rx) = mpsc::channel();
        let exe = fake.root.join("bin/claude");
        let waiter = std::thread::spawn(move || {
            let mut child = spawn_on(Command::new(exe), &slave).unwrap();
            drop(slave);
            tx.send(child.id() as i32).unwrap();
            child.wait().unwrap()
        });
        let pid = rx.recv().unwrap();
        let out = read_until(&master, "ready");
        // Q1: its own session leader and process group, a controlling tty
        let (_, _, pgrp, session, tty) = stat(pid);
        assert_eq!((session, pgrp), (pid, pid), "own session leader");
        assert_ne!(tty, 0, "a controlling tty");
        assert!(out.contains("tty /dev/pts/"), "{out}");
        // Q2: keys reach it
        write_all(&master, b"hello\r");
        read_until(&master, "got:hello");
        // Q3 (resize): TIOCSWINSZ -> SIGWINCH, and the tty has the size
        set_size(master.as_raw_fd(), 100, 30).unwrap();
        read_until(&master, "winch 30 100");
        // Q6: the master closes -> SIGHUP -> it exits
        drop(master);
        let status = waiter.join().unwrap();
        assert!(status.success(), "{status:?}");
        assert!(fake.root.join("log/hup").exists(), "it got SIGHUP");
    }

    #[test]
    fn the_argv_holds_only_a_listed_model_and_mode() {
        let plan = |model, mode| Plan {
            repo: PathBuf::from("/r"),
            account: None,
            model,
            mode,
            worktree: false,
        };
        assert!(argv(&plan("default", "default")).is_empty());
        assert_eq!(
            argv(&plan("opus", "plan")),
            ["--model", "opus", "--permission-mode", "plan"]
        );
        assert_eq!(
            argv(&plan("default", "bypassPermissions")),
            ["--dangerously-skip-permissions"]
        );
    }

    #[test]
    fn the_environment_is_the_scrubbed_one_with_the_terminal_type_and_no_tool_timeout() {
        let env: Vec<(OsString, OsString)> = [
            ("PATH", "/bin"),
            ("ANTHROPIC_API_KEY", "sk-ant-x"),
            ("CLAUDECODE", "1"),
            ("CLAUDE_CODE_ENTRYPOINT", "cli"),
            ("CLAUDE_CONFIG_DIR", "/parent"),
            ("TERM", "dumb"),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect();
        let out = term_env(&env, Some("/acct"));
        let get = |name: &str| {
            out.iter()
                .filter(|(k, _)| k == name)
                .map(|(_, v)| v.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(get("TERM"), ["xterm-256color"]);
        assert_eq!(get("COLORTERM"), ["truecolor"]);
        assert_eq!(get("CLAUDE_CONFIG_DIR"), ["/acct"]);
        assert_eq!(get("PATH"), ["/bin"]);
        for gone in [
            "ANTHROPIC_API_KEY",
            "CLAUDECODE",
            "CLAUDE_CODE_ENTRYPOINT",
            "MCP_TOOL_TIMEOUT",
        ] {
            assert!(get(gone).is_empty(), "{gone}");
        }
    }

    /// AC 9: the ring wraps without growing; the replay is the first 4 KiB plus the last 1 MiB.
    #[test]
    fn the_screen_keeps_the_first_4_kib_and_the_last_mib_and_never_more() {
        let mut screen = Screen::default();
        let block: Vec<u8> = (0..64 * 1024).map(|i| (i % 251) as u8).collect();
        let mut all = Vec::new();
        for _ in 0..48 {
            screen.push(&block); // 3 MiB
            all.extend_from_slice(&block);
            assert!(screen.held() <= PREFIX + RING);
        }
        assert_eq!(screen.held(), PREFIX + RING);
        let replay = screen.replay();
        assert_eq!(&replay[..PREFIX], &all[..PREFIX]);
        assert_eq!(&replay[PREFIX..], &all[all.len() - RING..]);
        // one write larger than the ring
        let mut screen = Screen::default();
        let big = vec![7u8; 2 * RING];
        screen.push(&big);
        assert_eq!(screen.held(), PREFIX + RING);
        // a short output is kept whole, with no byte twice
        let mut screen = Screen::default();
        screen.push(b"abc");
        screen.push(&vec![b'x'; PREFIX]);
        assert_eq!(screen.replay().len(), PREFIX + 3);
        assert_eq!(&screen.replay()[..3], b"abc");
        screen.clear();
        assert_eq!(screen.held(), 0);
    }

    /// AC 9 through a real PTY: 3 MiB written, at most 1 MiB + 4 KiB held.
    #[test]
    fn three_mib_of_output_hold_at_most_the_ring_and_the_prefix() {
        let _serial = serial();
        let fake = Fake::new(
            "t133-mem",
            "head -c 3145728 /dev/zero | tr '\\0' x; echo; echo all-written; sleep 60",
        );
        let (_slots, terms) = terms(&fake, 200);
        let id = terms.start(plan(&fake), &fake.dir(), yes()).unwrap();
        let term = terms.get(&id).unwrap();
        until("all written", || screen_text(&term).contains("all-written"));
        assert_eq!(term.held(), PREFIX + RING);
        terms.shutdown();
    }

    fn plan(fake: &Fake) -> Plan {
        let repo = fake.root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        Plan {
            repo,
            account: None,
            model: "default",
            mode: "default",
            worktree: false,
        }
    }

    fn yes() -> Box<dyn Fn() -> bool + Send> {
        Box::new(|| true)
    }

    fn terms(fake: &Fake, grace_ms: u64) -> (Arc<Sessions>, Terms) {
        let slots = Arc::new(Sessions::new(fake.env(&[])));
        let terms = Terms::timed(
            fake.env(&[]),
            Arc::clone(&slots),
            Duration::from_millis(grace_ms),
            Duration::from_millis(20),
        );
        (slots, terms)
    }

    fn pid(fake: &Fake) -> String {
        let path = fake.root.join("log/pid");
        until("the stub wrote its pid", || {
            std::fs::read_to_string(&path).is_ok_and(|p| !p.trim().is_empty())
        });
        std::fs::read_to_string(path).unwrap()
    }

    fn screen_text(term: &Term) -> String {
        String::from_utf8_lossy(&lock(&term.state).screen.replay()).into_owned()
    }

    /// AC 5: stop closes the master; a child that ignores SIGHUP gets SIGKILL after the grace.
    #[test]
    fn stop_closes_the_master_and_kills_a_child_that_ignores_sighup_after_the_grace() {
        let _serial = serial();
        let fake = Fake::new("t133-deaf", DEAF);
        let (_slots, terms) = terms(&fake, 800);
        let id = terms.start(plan(&fake), &fake.dir(), yes()).unwrap();
        let term = terms.get(&id).unwrap();
        until("ready", || screen_text(&term).contains("ready"));
        let pid = pid(&fake);
        let asked = Instant::now();
        term.request_stop("stopped");
        std::thread::sleep(Duration::from_millis(400));
        assert!(alive(&pid), "it ignores SIGHUP: alive before the grace");
        until("exited", || term.ended());
        assert!(asked.elapsed() >= Duration::from_millis(800));
        assert!(!runs(&pid));
        assert_eq!(term.json()["state"], "exited");
        assert_eq!(term.json()["reason"], "stopped");
        terms.shutdown();
    }

    /// AC 10: the program exits by itself -> the exit reaches the feed; the entry stays `exited`; its bytes go when the
    /// last page detaches.
    #[test]
    fn an_exit_by_itself_is_sent_and_the_bytes_go_when_the_last_page_detaches() {
        let _serial = serial();
        let fake = Fake::new("t133-exit", "echo bye-now; exit 3");
        let (slots, terms) = terms(&fake, 5000);
        let id = terms.start(plan(&fake), &fake.dir(), yes()).unwrap();
        let term = terms.get(&id).unwrap();
        until("exited", || term.ended());
        assert_eq!(term.json()["code"], 3);
        assert_eq!(term.json()["state"], "exited");
        assert!(screen_text(&term).contains("bye-now"), "kept for a page");
        let (replay, _rx, exit) = term.attach();
        assert!(String::from_utf8_lossy(&replay).contains("bye-now"));
        assert_eq!(exit, Some(Some(3)));
        assert_eq!(term.json()["pages"], 1);
        term.detach();
        assert_eq!(term.held(), 0, "dropped at the last detach after the end");
        assert_eq!(terms.all().len(), 1, "the entry stays");
        // the slot is free again
        let slot = slots.take_slot();
        assert!(slot.is_some());
        terms.shutdown();
    }

    /// The list names the Claude session id once the child's process file exists (the stub writes one under its fake
    /// `HOME`, never the owner's); a file of another pid or with a session id that is not hex and dashes is not taken.
    #[test]
    fn the_session_id_comes_from_the_childs_own_process_file() {
        let _serial = serial();
        let fake = Fake::new(
            "t133-sid",
            r#"d="$HOME/.claude/sessions"; mkdir -p "$d"
printf '{"pid":1,"sessionId":"0000-other"}' > "$d/1.json"
echo ready; sleep 1
printf '{"pid":%s,"sessionId":"$(id)"}' $$ > "$d/$$.json"; echo bad-written; sleep 1
printf '{"pid":%s,"sessionId":"0f1e2d3c-aaaa-4bbb-8ccc-0123456789ab"}' $$ > "$d/$$.json"; echo good-written
sleep 60"#,
        );
        let (_slots, terms) = terms(&fake, 200);
        let id = terms.start(plan(&fake), &fake.dir(), yes()).unwrap();
        let term = terms.get(&id).unwrap();
        until("ready", || screen_text(&term).contains("ready"));
        assert_eq!(
            term.json()["session_id"],
            Value::Null,
            "no file of its pid yet"
        );
        until("bad", || screen_text(&term).contains("bad-written"));
        assert_eq!(term.json()["session_id"], Value::Null, "not hex and dashes");
        until("good", || screen_text(&term).contains("good-written"));
        assert_eq!(
            term.json()["session_id"],
            "0f1e2d3c-aaaa-4bbb-8ccc-0123456789ab"
        );
        terms.shutdown();
    }

    #[test]
    fn no_claude_is_an_exited_entry_with_a_fixed_reason_and_frees_its_slot() {
        let _serial = serial();
        let fake = Fake::new("t133-none", STUB);
        std::fs::remove_file(fake.root.join("bin/claude")).unwrap();
        let (slots, terms) = terms(&fake, 5000);
        let id = terms.start(plan(&fake), &fake.dir(), yes()).unwrap();
        let term = terms.get(&id).unwrap();
        assert_eq!(term.json()["reason"], "claude not found");
        assert_eq!(term.json()["state"], "exited");
        let held: Vec<_> = (0..crate::app_session::MAX_RUNNING)
            .map(|_| slots.take_slot())
            .collect();
        assert!(held.iter().all(Option::is_some), "all four slots are free");
        terms.shutdown();
    }

    /// AC 3 (the registry side): four running terminals fill the one cap; a fifth is Busy; a slot frees at an end.
    #[test]
    fn the_fifth_terminal_is_busy_and_a_slot_frees_when_one_ends() {
        let _serial = serial();
        let fake = Fake::new("t133-cap", STUB);
        let (_slots, terms) = terms(&fake, 5000);
        let ids: Vec<String> = (0..crate::app_session::MAX_RUNNING)
            .map(|_| terms.start(plan(&fake), &fake.dir(), yes()).unwrap())
            .collect();
        assert_eq!(
            terms.start(plan(&fake), &fake.dir(), yes()),
            Err(StartError::Busy)
        );
        let first = terms.get(&ids[0]).unwrap();
        first.request_stop("stopped");
        until("ended", || first.ended());
        assert!(terms.start(plan(&fake), &fake.dir(), yes()).is_ok());
        terms.shutdown();
    }

    /// AC 7 (the cap): over 64 KiB in one second is dropped, the page is told once.
    #[test]
    fn typed_bytes_over_64_kib_a_second_are_dropped_with_one_notice() {
        let _serial = serial();
        let fake = Fake::new("t133-cap-in", STUB);
        let (_slots, terms) = terms(&fake, 5000);
        let id = terms.start(plan(&fake), &fake.dir(), yes()).unwrap();
        let term = terms.get(&id).unwrap();
        until("ready", || screen_text(&term).contains("ready"));
        let chunk = vec![b'a'; 16 * 1024];
        let results: Vec<Result<(), bool>> = (0..8).map(|_| term.input(chunk.clone())).collect();
        assert_eq!(&results[..4], &[Ok(()), Ok(()), Ok(()), Ok(())]);
        assert_eq!(results[4], Err(true), "the first drop tells the page");
        assert!(results[5..].iter().all(|r| *r == Err(false)), "only once");
        terms.shutdown();
    }

    #[test]
    fn a_resize_is_clamped_and_a_zero_is_ignored() {
        let _serial = serial();
        let fake = Fake::new("t133-size", STUB);
        let (_slots, terms) = terms(&fake, 5000);
        let id = terms.start(plan(&fake), &fake.dir(), yes()).unwrap();
        let term = terms.get(&id).unwrap();
        term.resize(1, 1);
        assert_eq!(term.size(), (10, 5));
        term.resize(9999, 9999);
        assert_eq!(term.size(), (500, 200));
        term.resize(0, 40);
        term.resize(90, 0);
        assert_eq!(term.size(), (500, 200));
        term.resize(90, 40);
        assert_eq!(term.size(), (90, 40));
        terms.shutdown();
    }

    /// AC 10: at exit every child ends and no stub process is left.
    #[test]
    fn shutdown_ends_every_child_and_drops_every_byte() {
        let _serial = serial();
        let fake = Fake::new("t133-down", DEAF);
        let (_slots, terms) = terms(&fake, 300);
        let id = terms.start(plan(&fake), &fake.dir(), yes()).unwrap();
        let term = terms.get(&id).unwrap();
        until("ready", || screen_text(&term).contains("ready"));
        let pid = pid(&fake);
        terms.shutdown();
        assert!(!runs(&pid));
        assert!(terms.all().is_empty());
        assert_eq!(term.held(), 0);
        let marker = fake.root.join("bin/claude").display().to_string();
        let left = Command::new("pgrep")
            .args(["-f", &marker])
            .output()
            .unwrap();
        assert!(left.stdout.is_empty(), "no stub left (pgrep)");
    }

    /// AC 10 (PDEATHSIG): when the thread that started the child goes, the child goes (as when the tray is killed).
    #[test]
    fn the_child_dies_with_the_thread_that_started_it() {
        let _serial = serial();
        let fake = Fake::new("t133-pdeath", DEAF);
        let (master, slave) = open_pty(80, 24).unwrap();
        let exe = fake.root.join("bin/claude");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let child = spawn_on(Command::new(exe), &slave).unwrap();
            tx.send(child.id()).unwrap();
            std::mem::forget(child); // the thread ends without waiting: the death signal fires
        })
        .join()
        .unwrap();
        let pid = rx.recv().unwrap().to_string();
        until("gone", || !runs(&pid));
        drop(master);
    }
}
