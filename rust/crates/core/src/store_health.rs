//! Read-only health check of the local store (T059, FR-091): a port of Python's `core/store_health.py` (HUN-244).
//!
//! One function, [`check`], returns one [`Health`] that the CLI (`store-check`) and the route (`GET /api/store/health`)
//! both use, so they give the same answer. Nothing is written: no migration, no `VACUUM`, no backup, no row, and no side
//! file next to the store. The answer holds counts, versions, sizes, times and file names only: never a row's content, an
//! absolute path or the text of an error (a fixed word stands in). Every text that comes from the store (table and source
//! names, times) is cut to [`TEXT_LIMIT`] characters and its non-printable characters become `?`.
//!
//! `healthy` is three-valued: `Some(true)` (every check finished and found nothing), `Some(false)` (a problem was found;
//! it always wins) and `None` (no problem found, but a check did not finish: "not fully checked").
//!
//! Speed (NFR-013): `PRAGMA integrity_check` needs about 30 ms per MB, so a store over
//! [`Options::full_check_max_bytes`] gets `PRAGMA quick_check` (`mode: "quick"`). The four reads run in parallel
//! threads, each on its own read-only connection, and all stop at one deadline ([`Options::budget`]). A read that did not
//! finish is named in `incomplete`. With [`Options::full`] there is no deadline and the check is always the full
//! `integrity_check`. The caller can stop every read at any time by setting the `cancel` flag.
//!
//! The stop is SQLite's progress handler (every [`PROGRESS_OPS`] VM steps, like Python), set through `rusqlite::ffi`
//! because rusqlite's own wrapper sits behind its `hooks` feature, which this workspace does not enable. That call is the
//! only `unsafe` here ([`Reader`]).

use rusqlite::{Connection, ErrorCode, OpenFlags, ffi, types::ValueRef};
use serde_json::{Map, Value, json};
use std::ffi::c_void;
use std::fs;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

use crate::store::KNOWN_MIGRATION;

/// Python's `BUDGET_S`.
pub const BUDGET: Duration = Duration::from_millis(1500);
/// Python's `FULL_CHECK_MAX_BYTES`.
pub const FULL_CHECK_MAX_BYTES: u64 = 32_000_000;
/// How many SQLite VM steps pass between two looks at the clock (Python's `PROGRESS_OPS`).
const PROGRESS_OPS: c_int = 1000;
/// The wait for a lock when there is no deadline (Python's `BUSY_TIMEOUT_MS`).
const BUSY_WAIT: Duration = Duration::from_millis(5000);
/// The longest text of the store that is shown.
pub const TEXT_LIMIT: usize = 64;
/// The sources that always have an entry, in this order ("no data" when the store has no row for them).
pub const SOURCES: [&str; 5] = ["claude_code", "kit_trace", "context", "ui", "verifier"];
/// Source -> the query that gives (newest time, abstained count or NULL).
const SOURCE_QUERIES: [(&str, &str); 4] = [
    (
        "kit_trace",
        "SELECT MAX(ts_utc), SUM(abstained) FROM kit_trace_events",
    ),
    (
        "context",
        "SELECT MAX(last_index_at_utc), NULL FROM context_index",
    ),
    ("ui", "SELECT MAX(started_at_utc), NULL FROM ui_sessions"),
    (
        "verifier",
        "SELECT MAX(generated_at_utc), SUM(overall_abstained) FROM verifier_scores",
    ),
];

/// What a run may change. [`Options::default`] is what the CLI and the route use; a test or the oracle forces the others.
#[derive(Clone, Debug)]
pub struct Options {
    /// `--full`: no deadline, always the full `integrity_check`.
    pub full: bool,
    /// The one deadline of a default run.
    pub budget: Duration,
    /// A store over this size gets `quick_check` in a default run.
    pub full_check_max_bytes: u64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            full: false,
            budget: BUDGET,
            full_check_max_bytes: FULL_CHECK_MAX_BYTES,
        }
    }
}

/// The verdict of a check pragma: `problems` is the number of rows SQLite returned.
#[derive(Clone, Debug, PartialEq)]
pub struct Verdict {
    pub ok: bool,
    pub problems: i64,
    /// `"full"` or `"quick"`; `None` for `foreign_key_check`, which has no such mode.
    pub mode: Option<&'static str>,
}

/// One source's newest record time and abstained count (`None`: no data).
#[derive(Clone, Debug, PartialEq)]
pub struct SourceInfo {
    pub last_ingest_utc: Option<String>,
    pub abstained: Option<i64>,
}

/// The answer. Field names are the JSON keys of Python's `check()`.
#[derive(Clone, Debug)]
pub struct Health {
    pub store_file: String,
    pub exists: bool,
    pub readable: bool,
    pub locked: bool,
    pub size_bytes: Option<u64>,
    pub wal_bytes: u64,
    pub user_version: Option<i64>,
    pub latest_migration: i64,
    pub migration_pending: bool,
    pub newer_than_code: bool,
    pub integrity_check: Option<Verdict>,
    pub foreign_key_check: Option<Verdict>,
    /// Table name -> row count (`None`: SQLite cannot read that table); in the order Python lists them.
    pub tables: Option<Vec<(String, Option<i64>)>>,
    pub sources: Option<Vec<(String, SourceInfo)>>,
    pub backups: Vec<String>,
    /// Parts that did not finish in time (or were cut off): `schema`, `integrity_check`, `foreign_key_check`, `tables`, `sources`.
    pub incomplete: Vec<&'static str>,
    /// Parts SQLite could not read.
    pub unreadable: Vec<&'static str>,
    pub healthy: Option<bool>,
}

// ---- the answer as JSON, as lines and as an exit code ------------------------------------------------------------------

impl Health {
    /// The JSON of Python's `check()`. Keys come out sorted (the `Map` is a `BTreeMap`).
    pub fn json(&self) -> Value {
        let verdict = |v: &Option<Verdict>| match v {
            None => Value::Null,
            Some(v) => {
                let mut m = Map::new();
                m.insert("ok".into(), json!(v.ok));
                m.insert("problems".into(), json!(v.problems));
                if let Some(mode) = v.mode {
                    m.insert("mode".into(), json!(mode));
                }
                Value::Object(m)
            }
        };
        let tables = match &self.tables {
            None => Value::Null,
            Some(t) => Value::Object(t.iter().map(|(n, c)| (n.clone(), json!(c))).collect()),
        };
        let sources = match &self.sources {
            None => Value::Null,
            Some(s) => Value::Object(
                s.iter()
                    .map(|(n, s)| {
                        (
                            n.clone(),
                            json!({"last_ingest_utc": s.last_ingest_utc, "abstained": s.abstained}),
                        )
                    })
                    .collect(),
            ),
        };
        json!({
            "store_file": self.store_file,
            "exists": self.exists,
            "readable": self.readable,
            "locked": self.locked,
            "size_bytes": self.size_bytes,
            "wal_bytes": self.wal_bytes,
            "user_version": self.user_version,
            "latest_migration": self.latest_migration,
            "migration_pending": self.migration_pending,
            "newer_than_code": self.newer_than_code,
            "integrity_check": verdict(&self.integrity_check),
            "foreign_key_check": verdict(&self.foreign_key_check),
            "tables": tables,
            "sources": sources,
            "backups": self.backups,
            "incomplete": self.incomplete,
            "unreadable": self.unreadable,
            "healthy": self.healthy,
        })
    }

    /// 0 healthy, 1 a problem was found, 3 not fully checked.
    pub fn exit_code(&self) -> u8 {
        match self.healthy {
            Some(true) => 0,
            Some(false) => 1,
            None => 3,
        }
    }

    /// The human form: one line per field group, then the verdict (Python's `lines()`).
    pub fn lines(&self) -> Vec<String> {
        let late = &self.incomplete;
        let part = |name: &str, text: &dyn Fn() -> String| -> String {
            if late.contains(&name) {
                "not finished in time".into()
            } else if self.unreadable.contains(&name) {
                "unreadable".into()
            } else {
                text()
            }
        };
        let verdict = |v: &Verdict| {
            let text = if v.ok {
                "ok".to_string()
            } else {
                format!("{} problem(s)", v.problems)
            };
            if v.mode == Some("quick") {
                format!("{text} (quick check)")
            } else {
                text
            }
        };
        let check_text = |v: &Option<Verdict>| match v {
            None => "not checked".to_string(),
            Some(v) => verdict(v),
        };
        let size = format!(
            "{} bytes (wal {} bytes)",
            self.size_bytes.unwrap_or(0),
            self.wal_bytes
        );
        let file_line = if !self.exists {
            format!(
                "{} does not exist (wal {} bytes)",
                self.store_file, self.wal_bytes
            )
        } else if self.locked {
            format!("{} {size}, unreadable (locked)", self.store_file)
        } else if !self.readable {
            format!("{} {size}, not a readable database", self.store_file)
        } else {
            format!("{} {size}", self.store_file)
        };
        let schema = match self.user_version {
            None => "unknown".to_string(),
            Some(v) => format!(
                "user_version {v}, code has {}, migration pending: {}{}",
                self.latest_migration,
                if self.migration_pending { "yes" } else { "no" },
                if self.newer_than_code {
                    ", store is newer than this program"
                } else {
                    ""
                }
            ),
        };
        let tables = || match &self.tables {
            None => "not checked".to_string(),
            Some(t) if t.is_empty() => "none".to_string(),
            Some(t) => t
                .iter()
                .map(|(n, c)| match c {
                    None => format!("{n} ?"),
                    Some(c) => format!("{n} {c}"),
                })
                .collect::<Vec<_>>()
                .join(", "),
        };
        let sources = || match &self.sources {
            None => "not checked".to_string(),
            Some(s) => s
                .iter()
                .map(|(name, s)| match (&s.last_ingest_utc, s.abstained) {
                    (None, _) => format!("{name} no data"),
                    (Some(last), None) => format!("{name} last {last}"),
                    (Some(last), Some(a)) => format!("{name} last {last}, abstained {a}"),
                })
                .collect::<Vec<_>>()
                .join("; "),
        };
        let word = match self.healthy {
            Some(true) => "healthy".to_string(),
            Some(false) => "NOT healthy".to_string(),
            None => {
                let mut word = format!(
                    "not fully checked ({} not finished in time)",
                    late.join(", ")
                );
                if late.contains(&"integrity_check") {
                    word.push_str(
                        "; run `easy-personal store-check --full` for the integrity verdict",
                    );
                }
                word
            }
        };
        vec![
            format!("file: {file_line}"),
            format!("schema: {schema}"),
            format!(
                "integrity_check: {}",
                part("integrity_check", &|| check_text(&self.integrity_check))
            ),
            format!(
                "foreign_key_check: {}",
                part("foreign_key_check", &|| check_text(&self.foreign_key_check))
            ),
            format!("tables: {}", part("tables", &tables)),
            format!("sources: {}", part("sources", &sources)),
            format!(
                "backups: {}",
                if self.backups.is_empty() {
                    "none".to_string()
                } else {
                    self.backups.join(", ")
                }
            ),
            format!("verdict: {word}"),
        ]
    }
}

// ---- small helpers ---------------------------------------------------------------------------------------------------

/// A name-to-value list with Python's dict rule: a repeated key keeps its first place and takes the new value.
fn put<V>(list: &mut Vec<(String, V)>, key: String, value: V) {
    match list.iter_mut().find(|(k, _)| *k == key) {
        Some(slot) => slot.1 = value,
        None => list.push((key, value)),
    }
}

/// Python's `str.isprintable()` for one character, as far as the standard library can tell: ASCII exactly; beyond ASCII,
/// not a control, separator, format or private-use character and not a noncharacter. (An unassigned code point counts as
/// printable here; Python's table says it is not.)
fn is_printable(c: char) -> bool {
    if c.is_ascii() {
        return c == ' ' || c.is_ascii_graphic();
    }
    let u = c as u32;
    let format = matches!(u,
        0xAD | 0x600..=0x605 | 0x61C | 0x6DD | 0x70F | 0x890..=0x891 | 0x8E2 | 0x180E | 0x200B..=0x200F
        | 0x202A..=0x202E | 0x2060..=0x2064 | 0x2066..=0x206F | 0xFEFF | 0xFFF9..=0xFFFB | 0x110BD | 0x110CD
        | 0x13430..=0x1343F | 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D17A | 0xE0001 | 0xE0020..=0xE007F);
    let private = matches!(u, 0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x100000..=0x10FFFD);
    let noncharacter = matches!(u, 0xFDD0..=0xFDEF) || (u & 0xFFFE) == 0xFFFE;
    !(c.is_control() || c.is_whitespace() || format || private || noncharacter)
}

/// A text from the store, safe to print: cut to [`TEXT_LIMIT`] characters, every non-printable character a `?`.
fn clean(text: &str) -> String {
    text.chars()
        .take(TEXT_LIMIT)
        .map(|c| if is_printable(c) { c } else { '?' })
        .collect()
}

fn file_size(path: &Path) -> Option<u64> {
    fs::metadata(path).ok().map(|m| m.len())
}

/// `<path>-wal` / `<path>-shm`: the name of SQLite's side file.
fn side(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// The names of the `<store>.bak-v<N>` files beside the store, lowest version first (never `-wal` / `-shm`).
fn backups(path: &Path) -> Vec<String> {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Vec::new();
    };
    let prefix = format!("{name}.bak-v");
    let Some(parent) = path.parent().map(|p| {
        if p.as_os_str().is_empty() {
            Path::new(".")
        } else {
            p
        }
    }) else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    // sort key: the number without its leading zeros (a longer one is bigger), then the name
    let mut found: Vec<(usize, String, String)> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter_map(|entry| {
            let digits = entry.strip_prefix(&prefix)?;
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let number = digits.trim_start_matches('0').to_string();
            Some((number.len(), number, entry))
        })
        .collect();
    found.sort();
    found.into_iter().map(|(_, _, name)| name).collect()
}

/// A path as a `file:` URI (every byte but `A-Za-z0-9-._~/` percent-encoded), the form SQLite reads its options from.
fn file_uri(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut uri = String::from("file:");
    for &b in path.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~' | b'/') {
            uri.push(b as char);
        } else {
            uri.push_str(&format!("%{b:02X}"));
        }
    }
    uri
}

// ---- what to open ------------------------------------------------------------------------------------------------------

/// A folder made for one run and removed with its content when dropped (a private copy of a store with a `-wal` and no `-shm`).
struct PrivateCopy(PathBuf);

impl Drop for PrivateCopy {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The URI to read, decided once before any thread starts. Nothing is ever created beside the store.
/// - No `-wal`: no process has the store open, so `immutable=1` (no locks, no side files) is safe.
/// - `-wal` and `-shm`: a live store; `mode=ro` creates nothing.
/// - `-wal` but no `-shm`: `mode=ro` would create the `-shm`. A private copy in a temp folder is read instead.
fn target(real: &Path) -> std::io::Result<(String, Option<PrivateCopy>)> {
    let (wal, shm) = (side(real, "-wal"), side(real, "-shm"));
    if !wal.exists() {
        return Ok((format!("{}?mode=ro&immutable=1", file_uri(real)), None));
    }
    if shm.exists() {
        return Ok((format!("{}?mode=ro", file_uri(real)), None));
    }
    static N: AtomicU32 = AtomicU32::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let folder = std::env::temp_dir().join(format!(
        "easy-personal-check-{}-{nanos}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new().mode(0o700).create(&folder)?; // fails on a name that exists: never reuses a planted folder
    }
    let copy = PrivateCopy(folder.clone());
    let store = folder.join("store.sqlite");
    fs::copy(real, &store)?;
    fs::copy(&wal, side(&store, "-wal"))?;
    Ok((format!("{}?mode=ro", file_uri(&store)), Some(copy)))
}

// ---- one read-only connection that stops at the deadline ---------------------------------------------------------------

/// When to stop the running statements: at `deadline`, or when `cancel` is set.
struct Stop {
    deadline: Option<Instant>,
    cancel: Arc<AtomicBool>,
}

/// SQLite's progress callback: a non-zero answer ends the running statement with "interrupted".
extern "C" fn stop_now(context: *mut c_void) -> c_int {
    // SAFETY: `context` is the `Arc<Stop>` that the `Reader` holds for as long as the callback is set (see `Reader`).
    let stop = unsafe { &*(context as *const Stop) };
    c_int::from(
        stop.cancel.load(Ordering::Relaxed) || stop.deadline.is_some_and(|d| Instant::now() >= d),
    )
}

/// A read-only connection with the progress callback set. The callback is removed before the connection closes.
struct Reader {
    conn: Connection,
    _stop: Arc<Stop>,
}

impl Reader {
    fn open(uri: &str, stop: &Arc<Stop>) -> rusqlite::Result<Reader> {
        let conn = Connection::open_with_flags(
            uri,
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        let wait = match stop.deadline {
            None => BUSY_WAIT,
            Some(d) => d.saturating_duration_since(Instant::now()),
        };
        conn.busy_timeout(wait)?;
        // SAFETY: `conn.handle()` is the open connection. The context pointer comes from an `Arc<Stop>` that this
        // `Reader` keeps alive until `drop`, which clears the callback first, so SQLite never sees a dangling pointer.
        unsafe {
            ffi::sqlite3_progress_handler(
                conn.handle(),
                PROGRESS_OPS,
                Some(stop_now),
                Arc::as_ptr(stop) as *mut c_void,
            );
        }
        Ok(Reader {
            conn,
            _stop: Arc::clone(stop),
        })
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        // SAFETY: the same open connection; a null callback removes the handler.
        unsafe { ffi::sqlite3_progress_handler(self.conn.handle(), 0, None, std::ptr::null_mut()) };
    }
}

// ---- the reads ---------------------------------------------------------------------------------------------------------

/// Why a read failed. `Bad` is a value of the store that is not what the column should hold (Python's `TypeError` and
/// `ValueError`).
enum Failure {
    Sqlite(rusqlite::Error),
    Bad,
}

impl From<rusqlite::Error> for Failure {
    fn from(e: rusqlite::Error) -> Self {
        Failure::Sqlite(e)
    }
}

fn code(e: &rusqlite::Error) -> Option<ErrorCode> {
    match e {
        rusqlite::Error::SqliteFailure(f, _) => Some(f.code),
        _ => None,
    }
}

fn interrupted(e: &rusqlite::Error) -> bool {
    code(e) == Some(ErrorCode::OperationInterrupted)
}

/// How one read ended. Never a panic, never an error text.
enum Status<T> {
    Ok(T),
    /// Cut off by the deadline or by `cancel`.
    Late,
    Locked,
    Error,
}

fn read<T>(
    uri: &str,
    stop: &Arc<Stop>,
    work: impl FnOnce(&Connection) -> Result<T, Failure>,
) -> Status<T> {
    let result = Reader::open(uri, stop)
        .map_err(Failure::from)
        .and_then(|reader| work(&reader.conn));
    match result {
        Ok(value) => Status::Ok(value),
        Err(Failure::Bad) => Status::Error,
        Err(Failure::Sqlite(e)) => match code(&e) {
            Some(ErrorCode::OperationInterrupted) => Status::Late,
            Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked) => Status::Locked,
            _ => Status::Error,
        },
    }
}

fn version(conn: &Connection) -> Result<i64, Failure> {
    // raises on "not a database"
    conn.query_row("SELECT 1 FROM sqlite_master LIMIT 1", [], |_| Ok(()))
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(()),
            e => Err(e),
        })?;
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

fn verdict(pragma: &'static str) -> impl FnOnce(&Connection) -> Result<Verdict, Failure> {
    move |conn| {
        let mut statement = conn.prepare(&format!("PRAGMA {pragma}"))?;
        let mut rows = statement.query([])?;
        let (mut count, mut only_ok) = (0i64, true);
        while let Some(row) = rows.next()? {
            count += 1;
            only_ok &= matches!(row.get_ref(0)?, ValueRef::Text(b"ok"));
        }
        let mode = Some(if pragma == "quick_check" {
            "quick"
        } else {
            "full"
        });
        Ok(if count == 1 && only_ok {
            Verdict {
                ok: true,
                problems: 0,
                mode,
            }
        } else {
            Verdict {
                ok: false,
                problems: count,
                mode,
            }
        })
    }
}

fn foreign_keys(conn: &Connection) -> Result<Verdict, Failure> {
    let mut statement = conn.prepare("PRAGMA foreign_key_check")?;
    let mut rows = statement.query([])?;
    let mut count = 0i64;
    while rows.next()?.is_some() {
        count += 1;
    }
    Ok(Verdict {
        ok: count == 0,
        problems: count,
        mode: None,
    })
}

/// Row count of every table in the schema (`None` for a table SQLite cannot read; a cut-off read is an error).
fn tables(conn: &Connection) -> Result<Vec<(String, Option<i64>)>, Failure> {
    let mut names: Vec<String> = Vec::new();
    let mut statement = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        match row.get_ref(0)? {
            ValueRef::Text(bytes) => names.push(
                std::str::from_utf8(bytes)
                    .map_err(|_| Failure::Bad)?
                    .to_string(),
            ),
            _ => return Err(Failure::Bad),
        }
    }
    let mut counts: Vec<(String, Option<i64>)> = Vec::new();
    for name in names {
        let sql = format!("SELECT COUNT(*) FROM \"{}\"", name.replace('"', "\"\""));
        let count = match conn.query_row(&sql, [], |r| r.get::<_, i64>(0)) {
            Ok(n) => Some(n),
            Err(e) if interrupted(&e) => return Err(e.into()),
            Err(_) => None,
        };
        put(&mut counts, clean(&name), count);
    }
    Ok(counts)
}

fn no_sources() -> Vec<(String, SourceInfo)> {
    SOURCES
        .iter()
        .map(|n| {
            (
                n.to_string(),
                SourceInfo {
                    last_ingest_utc: None,
                    abstained: None,
                },
            )
        })
        .collect()
}

/// A text or NULL value as an optional cleaned text. Anything else is `Bad` (Python's `_clean` raises `TypeError`).
fn cleaned(value: ValueRef<'_>) -> Result<Option<String>, Failure> {
    match value {
        ValueRef::Null => Ok(None),
        ValueRef::Text(bytes) => Ok(Some(clean(
            std::str::from_utf8(bytes).map_err(|_| Failure::Bad)?,
        ))),
        _ => Err(Failure::Bad),
    }
}

/// `int(value)` of a `SUM`: NULL is `None`; a real number is cut toward zero.
fn counted(value: ValueRef<'_>) -> Result<Option<i64>, Failure> {
    match value {
        ValueRef::Null => Ok(None),
        ValueRef::Integer(i) => Ok(Some(i)),
        ValueRef::Real(f) if f.is_finite() && f.abs() < 9.2e18 => Ok(Some(f as i64)),
        _ => Err(Failure::Bad),
    }
}

/// A cell of a per-source query.
enum Cell {
    Null,
    Text(String),
    NotUtf8,
    Other,
}

/// Per source: newest record time and abstained rows; both `None` when the source has no rows ("no data").
fn sources(conn: &Connection) -> Result<Vec<(String, SourceInfo)>, Failure> {
    let mut found = no_sources();
    let mut statement =
        conn.prepare("SELECT source, MAX(ts_utc), SUM(abstained) FROM events GROUP BY source")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let name = cleaned(row.get_ref(0)?)?.ok_or(Failure::Bad)?;
        let last = cleaned(row.get_ref(1)?)?;
        let abstained = counted(row.get_ref(2)?)?.unwrap_or(0);
        put(
            &mut found,
            name,
            SourceInfo {
                last_ingest_utc: last,
                abstained: Some(abstained),
            },
        );
    }
    drop(rows);
    drop(statement);
    for (name, sql) in SOURCE_QUERIES {
        let row = conn.query_row(sql, [], |r| {
            let last = match r.get_ref(0)? {
                ValueRef::Null => Cell::Null,
                ValueRef::Text(bytes) => {
                    std::str::from_utf8(bytes).map_or(Cell::NotUtf8, |t| Cell::Text(clean(t)))
                }
                _ => Cell::Other,
            };
            Ok((last, r.get_ref(1).map(|v| counted(v).ok())?))
        });
        match row {
            Ok((Cell::Text(last), abstained)) => {
                let abstained = abstained.ok_or(Failure::Bad)?;
                put(
                    &mut found,
                    name.to_string(),
                    SourceInfo {
                        last_ingest_utc: Some(last),
                        abstained,
                    },
                );
            }
            Ok((Cell::Other, _)) => return Err(Failure::Bad), // a time that is not a text
            Ok((Cell::Null | Cell::NotUtf8, _)) => {} // no rows, or text SQLite cannot decode: no data
            // a table missing in an old schema: no data (Python catches `OperationalError`, not a corrupt file)
            Err(e) if interrupted(&e) => return Err(e.into()),
            Err(e)
                if matches!(
                    code(&e),
                    Some(ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase)
                ) =>
            {
                return Err(e.into());
            }
            Err(rusqlite::Error::SqliteFailure(..)) => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(found)
}

// ---- the check ---------------------------------------------------------------------------------------------------------

/// The health of the store file at `path`. Never writes, never migrates, never panics for a bad store.
///
/// `cancel`: set it to stop every running read; what did not finish is then named in `incomplete`.
pub fn check(path: &Path, options: &Options, cancel: &Arc<AtomicBool>) -> Health {
    let deadline = if options.full {
        None
    } else {
        Some(Instant::now() + options.budget)
    };
    let stop = Arc::new(Stop {
        deadline,
        cancel: Arc::clone(cancel),
    });
    // SQLite names `-wal` and `-shm` after the real file, not after a symlink
    let real = fs::canonicalize(path).unwrap_or_else(|_| match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => {
            fs::canonicalize(parent).map_or_else(|_| path.to_path_buf(), |p| p.join(name))
        }
        _ => path.to_path_buf(),
    });
    let exists = fs::metadata(path).is_ok_and(|m| m.is_file());
    let mut out = Health {
        store_file: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        exists,
        readable: false,
        locked: false,
        size_bytes: if exists { file_size(path) } else { None },
        wal_bytes: file_size(&side(&real, "-wal")).unwrap_or(0),
        user_version: None,
        latest_migration: KNOWN_MIGRATION,
        migration_pending: false,
        newer_than_code: false,
        integrity_check: None,
        foreign_key_check: None,
        tables: None,
        sources: None,
        backups: backups(path),
        incomplete: Vec::new(),
        unreadable: Vec::new(),
        healthy: None,
    };
    if exists {
        match target(&real) {
            Ok((uri, _copy)) => fill(&mut out, &uri, &stop, options), // `_copy` lives until here
            Err(_) => out.unreadable.push("file"), // the private copy could not be made
        }
    }
    out.healthy =
        if !exists || !out.unreadable.is_empty() || (!out.readable && out.incomplete.is_empty()) {
            Some(false) // no store, or a file SQLite cannot read: a problem found
        } else {
            judge(&out)
        };
    out
}

fn fill(out: &mut Health, uri: &str, stop: &Arc<Stop>, options: &Options) {
    let found = match read(uri, stop, version) {
        Status::Ok(v) => v,
        status => {
            out.locked = matches!(status, Status::Locked);
            if !matches!(status, Status::Error) {
                out.incomplete.push("schema");
            }
            return;
        }
    };
    out.readable = true;
    out.user_version = Some(found);
    out.migration_pending = found < out.latest_migration;
    out.newer_than_code = found > out.latest_migration;
    let size = out.size_bytes.unwrap_or(0);
    let pragma = if options.full || size <= options.full_check_max_bytes {
        "integrity_check"
    } else {
        "quick_check"
    };
    let mode = Some(if pragma == "quick_check" {
        "quick"
    } else {
        "full"
    });
    let (integrity, foreign, counts, found_sources) = std::thread::scope(|scope| {
        let integrity = scope.spawn(|| read(uri, stop, verdict(pragma)));
        let foreign = scope.spawn(|| read(uri, stop, foreign_keys));
        let counts = scope.spawn(|| read(uri, stop, tables));
        let found_sources = scope.spawn(|| read(uri, stop, sources));
        (
            integrity.join(),
            foreign.join(),
            counts.join(),
            found_sources.join(),
        )
    });
    // a part whose thread panicked is unreadable, never a crash
    let fallback_verdict = |ok_mode| Verdict {
        ok: false,
        problems: 1,
        mode: ok_mode,
    };
    match integrity {
        Ok(Status::Ok(v)) => out.integrity_check = Some(v),
        Ok(Status::Error) => out.integrity_check = Some(fallback_verdict(mode)),
        Ok(_) => out.incomplete.push("integrity_check"),
        Err(_) => out.unreadable.push("integrity_check"),
    }
    match foreign {
        Ok(Status::Ok(v)) => out.foreign_key_check = Some(v),
        Ok(Status::Error) => out.foreign_key_check = Some(fallback_verdict(None)),
        Ok(_) => out.incomplete.push("foreign_key_check"),
        Err(_) => out.unreadable.push("foreign_key_check"),
    }
    match counts {
        Ok(Status::Ok(v)) => out.tables = Some(v),
        Ok(Status::Error) | Err(_) => out.unreadable.push("tables"),
        Ok(_) => out.incomplete.push("tables"),
    }
    match found_sources {
        Ok(Status::Ok(v)) => out.sources = Some(v),
        Ok(Status::Error) | Err(_) => out.unreadable.push("sources"),
        Ok(_) => out.incomplete.push("sources"),
    }
}

/// `Some(false)` when a problem was found (it always wins), `None` when a check did not finish, else `Some(true)`.
fn judge(out: &Health) -> Option<bool> {
    if out.migration_pending || out.newer_than_code || !out.unreadable.is_empty() {
        return Some(false);
    }
    if [&out.integrity_check, &out.foreign_key_check]
        .iter()
        .any(|v| v.as_ref().is_some_and(|v| !v.ok))
    {
        return Some(false);
    }
    if out
        .tables
        .as_ref()
        .is_some_and(|t| t.iter().any(|(_, c)| c.is_none()))
    {
        return Some(false);
    }
    if out.incomplete.is_empty() {
        Some(true)
    } else {
        None
    }
}
