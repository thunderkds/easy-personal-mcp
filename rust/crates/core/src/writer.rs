//! Store writer (P2, T027): Rust as a second writer of the schema-v11 store, next to the Python app.
//!
//! Python (`src/easy_personal/core/store.py`) stays the schema owner. This module never migrates: it opens a v11 store,
//! creates a brand-new one from the embedded snapshot of a fresh Python store (`schema_v11.sql`, T026), and refuses
//! every other version before writing a byte. The writes copy `store.py` statement by statement (quoted below), so a
//! row saved here is the row Python would save. The read-only API in `store.rs` is unchanged and still never writes.
//! No error text carries a path, SQL or a row value (NFR-002). Take [`crate::ingest_lock::ingest_lock`] before writing.

use crate::metrics::format_stored;
use crate::store::{KNOWN_MIGRATION, StoreError};
use rusqlite::{Connection, ErrorCode, OpenFlags, OptionalExtension, TransactionBehavior, params};
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
use std::time::Duration;

/// The `sqlite_master` of a fresh Python store, its seed row and `PRAGMA user_version = 11`
/// (a byte-equal copy of `rust/parity/ingest/golden/schema-v11.sql`; a test checks it).
pub const SCHEMA_V11: &str = include_str!("schema_v11.sql");

/// `BUSY_TIMEOUT_MS` in `store.py`.
const BUSY_TIMEOUT: Duration = Duration::from_millis(5000);

const DAY_MICROS: i64 = 86_400_000_000;
/// 0001-01-01T00:00:00Z and 10000-01-01T00:00:00Z: the range of Python's `datetime` (it raises outside it).
const MIN_MICROS: i64 = -62_135_596_800_000_000;
const END_MICROS: i64 = 253_402_300_800_000_000;

#[derive(Debug)]
pub enum WriteError {
    /// `user_version` above [`KNOWN_MIGRATION`]: the reader's `StoreError::Newer` text, exit code 2.
    Newer { found: i64, known: i64 },
    /// A store with tables and `user_version` below [`KNOWN_MIGRATION`]: the Python app migrates it, this never does.
    Older { found: i64 },
    /// A directory, a symlink loop or a file that is not SQLite where the store should be.
    NotAStore,
    /// Another connection held the write lock for longer than the busy wait.
    Busy,
    /// A foreign key, CHECK, UNIQUE or trigger rule refused a row; the transaction was rolled back.
    Rejected,
    /// The retention period puts the cutoff outside the dates Python can write.
    OutOfRange,
    /// No random bytes for a new `machine_id`.
    NoRandom,
    /// The store could not be opened (the I/O error kind only).
    Io(io::ErrorKind),
    /// Any other SQLite failure (its code name only).
    Sqlite(String),
}

impl WriteError {
    /// Process exit code: 2 for a store of another schema version (like the reader), 1 for every other failure.
    pub fn exit_code(&self) -> i32 {
        match self {
            WriteError::Newer { .. } | WriteError::Older { .. } => 2,
            _ => 1,
        }
    }
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WriteError::Newer { found, known } => StoreError::Newer {
                found: *found,
                known: *known,
            }
            .fmt(f),
            WriteError::Older { found } => write!(
                f,
                "store is schema v{found}: run the Python app once to migrate it"
            ),
            WriteError::NotAStore => write!(f, "the store path is not a SQLite store file"),
            WriteError::Busy => write!(
                f,
                "the store is busy (another process is writing); try again in a moment"
            ),
            WriteError::Rejected => write!(
                f,
                "the store refused a row (a foreign key, check or unique rule); nothing of this write was saved"
            ),
            WriteError::OutOfRange => write!(f, "the retention period is out of range"),
            WriteError::NoRandom => write!(f, "cannot get random bytes for a new machine id"),
            WriteError::Io(kind) => write!(f, "cannot open the store: {kind:?}"),
            WriteError::Sqlite(code) => write!(f, "cannot write the store: SQLite error {code}"),
        }
    }
}

impl std::error::Error for WriteError {}

impl From<rusqlite::Error> for WriteError {
    fn from(e: rusqlite::Error) -> Self {
        match e {
            rusqlite::Error::SqliteFailure(f, _) => match f.code {
                ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked => WriteError::Busy,
                ErrorCode::NotADatabase => WriteError::NotAStore,
                ErrorCode::ConstraintViolation => WriteError::Rejected,
                code => WriteError::Sqlite(format!("{code:?}")),
            },
            // The variant name only: its Display and Debug text can carry SQL, a column name or a value.
            other => {
                let debug = format!("{other:?}");
                let name: String = debug
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                WriteError::Sqlite(name)
            }
        }
    }
}

/// One stored event: the columns of `EVENT_COLUMNS` in `store.py`, plus the line's tool signals.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Event {
    pub abstained: bool,
    pub record_type: Option<String>,
    pub version: Option<String>,
    /// `%Y-%m-%dT%H:%M:%S.%fZ` (6 digits), as the parser writes it; stored as given.
    pub ts_utc: Option<String>,
    pub session_id: Option<String>,
    pub project_path: Option<String>,
    pub git_branch: Option<String>,
    pub is_sidechain: Option<bool>,
    pub is_api_error: Option<bool>,
    pub entrypoint: Option<String>,
    pub tool_error_count: Option<i64>,
    pub model: Option<String>,
    pub message_id: Option<String>,
    pub tool_names: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_creation_input_tokens: Option<i64>,
    pub cache_read_input_tokens: Option<i64>,
    /// `event["signals"]`: `(kind, value)` pairs saved to `tool_signals` with the event's `ts_utc`.
    pub signals: Vec<Signal>,
}

/// One `tool_signals` row's `(kind, value)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Signal {
    pub kind: String,
    pub value: String,
}

/// One consumed line of a file: Python's `(line_offset, next_offset, event or None)`.
/// `next_offset` is the start of the next unread line; the caller computes it, this module stores it.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub line_offset: i64,
    pub next_offset: i64,
    pub event: Option<Event>,
}

/// An open, writable v11 store with its `meta.machine_id`.
#[derive(Debug)]
pub struct Store {
    conn: Connection,
    machine_id: String,
}

/// Open the store at `path` for writing, or create it from [`SCHEMA_V11`] when there is no file, an empty file or a
/// file with no table. Refuses (before any write, the file untouched) a store whose `user_version` is not 11.
/// Sets `busy_timeout=5000`, `foreign_keys=ON`, `journal_mode=WAL` like `store.connect`, and never migrates.
pub fn open_for_write(path: &Path) -> Result<Store, WriteError> {
    open_store(path, true)
}

/// [`open_for_write`] for a store that must already be there (`save_report` and the workspace and account write routes, which
/// check it read-only first): without
/// `SQLITE_OPEN_CREATE`, a file removed since that check is `WriteError::Io(NotFound)` and is not created again.
pub fn open_existing_for_write(path: &Path) -> Result<Store, WriteError> {
    open_store(path, false)
}

fn open_store(path: &Path, may_create: bool) -> Result<Store, WriteError> {
    match fs::metadata(path) {
        Ok(meta) if meta.is_file() => {}
        Ok(_) => return Err(WriteError::NotAStore),
        Err(e) if may_create && e.kind() == io::ErrorKind::NotFound => {}
        // A symlink that cannot be followed for another reason than a missing target: a loop (ELOOP).
        Err(_) if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) => {
            return Err(WriteError::NotAStore);
        }
        Err(e) => return Err(WriteError::Io(e.kind())),
    }
    let mut flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    if may_create {
        flags |= OpenFlags::SQLITE_OPEN_CREATE;
    }
    let conn = Connection::open_with_flags(path, flags)?;
    // store.py:104-105. Neither pragma writes the file.
    conn.busy_timeout(BUSY_TIMEOUT)?;
    conn.execute_batch("PRAGMA foreign_keys=ON")?;
    // Read before any write: a refused store stays byte-identical (Python sets WAL first, then refuses in `migrate`).
    let found: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if found > KNOWN_MIGRATION {
        return Err(WriteError::Newer {
            found,
            known: KNOWN_MIGRATION,
        });
    }
    let has_tables = has_tables(&conn)?;
    if has_tables && found < KNOWN_MIGRATION {
        return Err(WriteError::Older { found });
    }
    // store.py:106 `PRAGMA journal_mode=WAL` (it answers one row: the mode).
    conn.query_row("PRAGMA journal_mode=WAL", [], |_| Ok(()))?;
    let mut conn = conn;
    if !has_tables {
        create(&mut conn)?;
    }
    let machine_id = machine_id(&conn)?;
    Ok(Store { conn, machine_id })
}

/// `_has_tables` in `store.py`.
fn has_tables(conn: &Connection) -> Result<bool, WriteError> {
    Ok(conn
        .query_row("SELECT 1 FROM sqlite_master LIMIT 1", [], |_| Ok(()))
        .optional()?
        .is_some())
}

/// The whole v11 schema, its seed row and `user_version = 11` in one write transaction. Re-checked under the lock:
/// another process may have created the store meanwhile.
fn create(conn: &mut Connection) -> Result<(), WriteError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if !has_tables(&tx)? {
        tx.execute_batch(SCHEMA_V11)?;
    }
    tx.commit()?;
    Ok(())
}

/// store.py:108-114: read first (even an `INSERT OR IGNORE` that changes nothing takes the write lock); only when
/// absent insert a new uuid4, `OR IGNORE` because two processes may create a store at once. Then read the winner.
fn machine_id(conn: &Connection) -> Result<String, WriteError> {
    const READ: &str = "SELECT value FROM meta WHERE key = 'machine_id'";
    if let Some(id) = conn
        .query_row(READ, [], |r| r.get::<_, String>(0))
        .optional()?
    {
        return Ok(id);
    }
    conn.execute(
        "INSERT OR IGNORE INTO meta (key, value) VALUES ('machine_id', ?)",
        [uuid4()?],
    )?;
    Ok(conn.query_row(READ, [], |r| r.get(0))?)
}

/// `str(uuid.uuid4())`: 16 random bytes, version 4, RFC 4122 variant, lower-case hex in 8-4-4-4-12.
fn uuid4() -> Result<String, WriteError> {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).map_err(|_| WriteError::NoRandom)?;
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    ))
}

/// store.py:178-186, verbatim (`EVENT_COLUMNS` spelt out in its order). A re-read finds its rows: only
/// `entrypoint` and `tool_error_count` may change, each only to a known value, and the `WHERE` skips the rewrite of
/// a row that would not change.
const UPSERT_EVENT: &str = "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, \
     agent_account_id, abstained, record_type, version, ts_utc, session_id, project_path, git_branch, \
     is_sidechain, is_api_error, entrypoint, tool_error_count, model, message_id, tool_names, input_tokens, \
     output_tokens, cache_creation_input_tokens, cache_read_input_tokens) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
     ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
     ON CONFLICT (machine_id, source, source_path, inode, byte_offset) DO UPDATE \
     SET entrypoint = COALESCE(excluded.entrypoint, events.entrypoint), \
     tool_error_count = COALESCE(excluded.tool_error_count, events.tool_error_count) \
     WHERE (excluded.entrypoint IS NOT NULL AND events.entrypoint IS NOT excluded.entrypoint) \
     OR (excluded.tool_error_count IS NOT NULL AND events.tool_error_count IS NOT excluded.tool_error_count)";

/// store.py:187 `in_file`.
const IN_FILE: &str = "machine_id = ? AND source = ? AND source_path = ? AND inode = ?";

impl Store {
    /// `meta.machine_id` of this store (read once at open; never regenerated).
    pub fn machine_id(&self) -> &str {
        &self.machine_id
    }

    /// The connection, for the ingest's account rows (`ingest::accounts`, `ingest::folders`: Python's `detect`,
    /// `account_roots` and the `NULL -> default account` update run their SQL on the ingest's own connection).
    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }

    /// `store.file_offsets` (store.py:131-137): the known files of `source` on this machine, path -> (inode, offset of
    /// the next unread line). Read-only.
    pub fn file_offsets(&self, source: &str) -> Result<HashMap<String, (i64, i64)>, WriteError> {
        let mut stmt = self.conn.prepare(
            "SELECT path, inode, byte_offset FROM source_files WHERE machine_id = ? AND source = ?",
        )?;
        let rows = stmt.query_map(params![self.machine_id, source], |r| {
            Ok((r.get(0)?, (r.get(1)?, r.get(2)?)))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// `store.save_file`: one file's new events, the session-entrypoint back-fill, its tool signals and its new offset
    /// in **one** transaction; returns the rows added. The stored offset is the last `next_offset` in `lines`, else
    /// `offset`. `account_id` tags the events (None: untagged); it must name an `agent_accounts` row. Any failure
    /// rolls everything back.
    pub fn save_file(
        &mut self,
        source: &str,
        path: &str,
        inode: i64,
        offset: i64,
        lines: &[Line],
        account_id: Option<i64>,
    ) -> Result<i64, WriteError> {
        let lines = lines.iter().map(Ok::<&Line, std::convert::Infallible>);
        match self.save_file_streamed(source, path, inode, offset, lines, account_id)? {
            Ok(added) => Ok(added),
            Err(never) => match never {},
        }
    }

    /// [`Store::save_file`] reading the lines one by one inside the same single transaction (T032), so memory does
    /// not grow with the lines of one file. A line that is an `Err` stops the file: everything is rolled back and the
    /// error is returned as `Ok(Err(e))` (the store is unchanged); a store failure is the outer `Err`.
    pub fn save_file_streamed<L: std::borrow::Borrow<Line>, E>(
        &mut self,
        source: &str,
        path: &str,
        inode: i64,
        offset: i64,
        lines: impl IntoIterator<Item = Result<L, E>>,
        account_id: Option<i64>,
    ) -> Result<Result<i64, E>, WriteError> {
        let mid = self.machine_id.as_str();
        // `with conn:`. IMMEDIATE, not Python's implicit deferred BEGIN at the first INSERT: the write lock is taken
        // (with the busy wait) before `MAX(id)` is read, so the count below cannot see another writer's rows.
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        // store.py:189 `last_id = conn.execute("SELECT COALESCE(MAX(id), 0) FROM events")`
        let last_id: i64 =
            tx.query_row("SELECT COALESCE(MAX(id), 0) FROM events", [], |r| r.get(0))?;
        let mut end = offset;
        let mut signals: Vec<(i64, Signal, Option<String>)> = Vec::new();
        // store.py:158-159 `first_entry` (session -> its first entrypoint seen in this batch) and `sessions`.
        let mut first_entry: HashMap<String, String> = HashMap::new();
        let mut sessions: BTreeSet<String> = BTreeSet::new();
        {
            // store.py:190 `conn.executemany(sql, values())`
            let mut upsert = tx.prepare(UPSERT_EVENT)?;
            for line in lines {
                let line = match line {
                    Ok(line) => line,
                    Err(e) => return Ok(Err(e)), // `tx` is dropped: rolled back
                };
                let line = line.borrow();
                end = line.next_offset; // store.py:163 `end[0] = next_offset`
                let Some(e) = &line.event else { continue };
                let ts = e.ts_utc.as_ref();
                signals.extend(
                    e.signals
                        .iter()
                        .map(|s| (line.line_offset, s.clone(), ts.cloned())),
                );
                if let Some(session) = e.session_id.as_deref() {
                    if !sessions.contains(session) {
                        sessions.insert(session.to_owned());
                    }
                    if let Some(entry) = e.entrypoint.as_deref()
                        && !first_entry.contains_key(session)
                    {
                        first_entry.insert(session.to_owned(), entry.to_owned()); // `setdefault`
                    }
                }
                upsert.execute(params![
                    mid,
                    source,
                    path,
                    inode,
                    line.line_offset,
                    account_id,
                    e.abstained,
                    e.record_type,
                    e.version,
                    e.ts_utc,
                    e.session_id,
                    e.project_path,
                    e.git_branch,
                    e.is_sidechain,
                    e.is_api_error,
                    e.entrypoint,
                    e.tool_error_count,
                    e.model,
                    e.message_id,
                    e.tool_names,
                    e.input_tokens,
                    e.output_tokens,
                    e.cache_creation_input_tokens,
                    e.cache_read_input_tokens,
                ])?;
            }
        }
        // store.py:191 `added = conn.execute("SELECT COUNT(*) FROM events WHERE id > ?", (last_id,))`
        let added: i64 =
            tx.query_row("SELECT COUNT(*) FROM events WHERE id > ?", [last_id], |r| {
                r.get(0)
            })?;
        // store.py:192-207: a session's rows before its first entrypoint, or in an earlier run, take that entrypoint;
        // with none in the batch, any non-NULL entrypoint already stored for the session in this file.
        for session in sessions {
            let entry: Option<String> = match first_entry.get(&session) {
                Some(entry) => Some(entry.clone()),
                None => tx
                    .query_row(
                        &format!(
                            "SELECT entrypoint FROM events WHERE {IN_FILE} AND session_id = ? \
                             AND entrypoint IS NOT NULL LIMIT 1"
                        ),
                        params![mid, source, path, inode, session],
                        |r| r.get(0),
                    )
                    .optional()?,
            };
            if let Some(entry) = entry {
                tx.execute(
                    &format!(
                        "UPDATE events SET entrypoint = ? WHERE {IN_FILE} AND session_id = ? \
                         AND entrypoint IS NULL"
                    ),
                    params![entry, mid, source, path, inode, session],
                )?;
            }
        }
        {
            // store.py:208-212
            let mut signal = tx.prepare(
                "INSERT OR IGNORE INTO tool_signals (machine_id, source_path, inode, byte_offset, \
                 kind, value, ts_utc) VALUES (?, ?, ?, ?, ?, ?, ?)",
            )?;
            for (line_offset, s, ts) in signals {
                signal.execute(params![mid, path, inode, line_offset, s.kind, s.value, ts])?;
            }
        }
        // store.py:213-218
        tx.execute(
            "INSERT INTO source_files (machine_id, source, path, inode, byte_offset) \
             VALUES (?, ?, ?, ?, ?) ON CONFLICT (machine_id, source, path) \
             DO UPDATE SET inode = excluded.inode, byte_offset = excluded.byte_offset",
            params![mid, source, path, inode, end],
        )?;
        tx.commit()?;
        Ok(Ok(added))
    }

    /// `store.retire_files`: forget the offsets of files that no longer exist; their events stay.
    pub fn retire_files<S: AsRef<str>>(
        &mut self,
        source: &str,
        paths: &[S],
    ) -> Result<(), WriteError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        {
            // store.py:226-229
            let mut delete = tx.prepare(
                "DELETE FROM source_files WHERE machine_id = ? AND source = ? AND path = ?",
            )?;
            for path in paths {
                delete.execute(params![self.machine_id, source, path.as_ref()])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// `store.prune`: delete `events`, `kit_trace_events` and `tool_signals` rows whose `ts_utc` is before
    /// `now_micros - retention_days` (a **text** comparison on `%Y-%m-%dT%H:%M:%S.%fZ`, as Python); rows without a
    /// timestamp are kept, `source_files` is never touched. `None` keeps everything. Returns the rows deleted (the sum
    /// over the three tables, as Python).
    pub fn prune(
        &mut self,
        retention_days: Option<i64>,
        now_micros: i64,
    ) -> Result<i64, WriteError> {
        let Some(days) = retention_days else {
            return Ok(0);
        };
        let cutoff = days
            .checked_mul(DAY_MICROS)
            .and_then(|span| now_micros.checked_sub(span))
            .filter(|c| (MIN_MICROS..END_MICROS).contains(c))
            .ok_or(WriteError::OutOfRange)?;
        // store.py:243 `cutoff.strftime("%Y-%m-%dT%H:%M:%S.%fZ")`
        let cutoff = format_stored(cutoff);
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut deleted = 0;
        // store.py:241-252, in Python's order.
        for table in ["events", "kit_trace_events", "tool_signals"] {
            deleted += tx.execute(
                &format!("DELETE FROM {table} WHERE ts_utc IS NOT NULL AND ts_utc < ?"),
                [&cutoff],
            )?;
        }
        tx.commit()?;
        Ok(deleted as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Criterion 3: the writer's own connection carries Python's three pragmas, on a new store and on a v11 copy.
    #[test]
    fn every_connection_has_the_python_pragmas() {
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t027-pragmas-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let copy = dir.join("copy.sqlite");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/store-v11.sqlite"),
            &copy,
        )
        .unwrap();
        for path in [dir.join("new.sqlite"), copy] {
            let store = open_for_write(&path).unwrap();
            let read = |name: &str| -> String {
                store
                    .conn
                    .query_row(&format!("PRAGMA {name}"), [], |r| {
                        r.get::<_, rusqlite::types::Value>(0)
                    })
                    .map(|v| match v {
                        rusqlite::types::Value::Integer(i) => i.to_string(),
                        rusqlite::types::Value::Text(t) => t,
                        other => format!("{other:?}"),
                    })
                    .unwrap()
            };
            assert_eq!(read("busy_timeout"), "5000");
            assert_eq!(read("foreign_keys"), "1");
            assert_eq!(read("journal_mode"), "wal");
            assert_eq!(read("user_version"), "11");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    /// T032: a line that is an `Err` part-way through `save_file_streamed` rolls the whole file back (no event, signal
    /// or offset stored), as the collected read did before; the same lines without the error are saved.
    #[test]
    fn a_read_error_part_way_rolls_the_file_back() {
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t032-stream-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let mut store = open_for_write(&dir.join("store.sqlite")).unwrap();
        let line = |at: i64| Line {
            line_offset: at,
            next_offset: at + 10,
            event: Some(Event {
                record_type: Some("assistant".into()),
                session_id: Some("s-1".into()),
                signals: vec![Signal {
                    kind: "bash_command".into(),
                    value: "git".into(),
                }],
                ..Event::default()
            }),
        };
        let cut = vec![Ok(line(0)), Err(io::Error::other("cut")), Ok(line(10))];
        let got = store
            .save_file_streamed("claude_code", "p.jsonl", 7, 0, cut, None)
            .unwrap();
        assert!(got.is_err());
        let counts = |store: &Store| -> Vec<i64> {
            ["events", "tool_signals", "source_files"]
                .iter()
                .map(|t| {
                    store
                        .conn
                        .query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))
                        .unwrap()
                })
                .collect()
        };
        assert_eq!(counts(&store), [0, 0, 0]);
        let whole = vec![Ok::<_, io::Error>(line(0)), Ok(line(10))];
        let added = store
            .save_file_streamed("claude_code", "p.jsonl", 7, 0, whole, None)
            .unwrap()
            .unwrap();
        assert_eq!(added, 2);
        assert_eq!(counts(&store), [2, 2, 1]);
        let _ = fs::remove_dir_all(&dir);
    }
}
