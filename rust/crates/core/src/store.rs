//! Read-only view of the shared store (`~/.easy-personal/store.sqlite`).
//!
//! The Python app owns the schema and the writes. This module only opens the file read-only and
//! refuses a store newer than this build knows. Nothing here ever writes, and no message or output
//! carries a row value or an absolute path.

use rusqlite::{Connection, ErrorCode, OpenFlags};
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Number of the newest migration this build knows: the last `m00NN_*.py` file in
/// `src/easy_personal/core/migrations/`. Python's `PRAGMA user_version` equals the last migration applied.
pub const KNOWN_MIGRATION: i64 = 11;

/// Same wait as Python's `BUSY_TIMEOUT_MS` in `core/store.py`.
const BUSY_TIMEOUT: Duration = Duration::from_millis(5000);

#[derive(Debug)]
pub enum StoreError {
    /// `user_version` is greater than [`KNOWN_MIGRATION`].
    Newer { found: i64, known: i64 },
    /// No file at the store path: nothing has been ingested yet (only [`open_checked`] returns this).
    NoStore,
    /// Something other than a regular file sits where the store should be.
    NotAFile,
    /// A writer held the store for longer than the busy wait.
    Busy,
    /// Neither `EASY_PERSONAL_HOME` nor `HOME` is set.
    NoHome,
    /// Any other failure, described without the path.
    Unreadable(String),
}

impl StoreError {
    /// Process exit code: 2 for a newer store, 1 for every other failure.
    pub fn exit_code(&self) -> i32 {
        match self {
            StoreError::Newer { .. } => 2,
            _ => 1,
        }
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Newer { found, known } => write!(
                f,
                "the store is at version {found}, newer than the {known} this build knows; update easy-personal-rs (nothing was changed)"
            ),
            StoreError::NoStore => write!(
                f,
                "no store yet: nothing has been ingested into the data folder"
            ),
            StoreError::NotAFile => write!(f, "the store path is not a regular file"),
            StoreError::Busy => write!(
                f,
                "the store is busy (another process is writing); try again in a moment"
            ),
            StoreError::NoHome => write!(
                f,
                "cannot find the data folder: set EASY_PERSONAL_HOME or HOME"
            ),
            StoreError::Unreadable(why) => write!(f, "cannot read the store: {why}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        match e {
            rusqlite::Error::SqliteFailure(f, _) => match f.code {
                ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked => StoreError::Busy,
                ErrorCode::NotADatabase => {
                    StoreError::Unreadable("the file is not a SQLite database".into())
                }
                code => StoreError::Unreadable(format!("SQLite error {code:?}")),
            },
            // The variant name only: its Display and Debug text can carry a column name, SQL or a value (NFR-002).
            other => {
                let debug = format!("{other:?}");
                let name: String = debug
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                StoreError::Unreadable(format!("unexpected SQLite error ({name})"))
            }
        }
    }
}

#[derive(Debug)]
pub struct StoreInfo {
    /// File name only, never the full path.
    pub file: String,
    pub state: State,
}

#[derive(Debug)]
pub enum State {
    Missing,
    Present {
        user_version: i64,
        tables: Vec<(String, i64)>,
    },
}

impl StoreInfo {
    pub fn text(&self) -> String {
        match &self.state {
            State::Missing => format!("store: {}\nexists: no\nno data\n", self.file),
            State::Present {
                user_version,
                tables,
            } => {
                let pending = if *user_version < KNOWN_MIGRATION {
                    "yes"
                } else {
                    "no"
                };
                let mut out = format!(
                    "store: {}\nexists: yes\nuser_version: {user_version}\nknown: {KNOWN_MIGRATION}\npending: {pending}\ntables:\n",
                    self.file
                );
                for (name, rows) in tables {
                    out.push_str(&format!("  {name}: {rows}\n"));
                }
                out
            }
        }
    }

    pub fn json(&self) -> String {
        match &self.state {
            State::Missing => format!(r#"{{"file":{},"exists":false}}"#, json_str(&self.file)),
            State::Present {
                user_version,
                tables,
            } => {
                let counts: Vec<String> = tables
                    .iter()
                    .map(|(n, c)| format!("{}:{c}", json_str(n)))
                    .collect();
                format!(
                    r#"{{"file":{},"exists":true,"user_version":{user_version},"known":{KNOWN_MIGRATION},"pending":{},"tables":{{{}}}}}"#,
                    json_str(&self.file),
                    *user_version < KNOWN_MIGRATION,
                    counts.join(",")
                )
            }
        }
    }
}

/// A JSON string literal (the only escaping this output needs; names come from the store file).
fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Where the store lives: `$EASY_PERSONAL_HOME/store.sqlite`, else `~/.easy-personal/store.sqlite`
/// (`data_dir()` in `src/easy_personal/core/config.py`). Unlike Python, this never creates the folder.
pub fn store_path() -> Result<PathBuf, StoreError> {
    store_path_from(
        std::env::var_os("EASY_PERSONAL_HOME"),
        std::env::var_os("HOME"),
    )
}

/// [`store_path`] over explicit inputs. An empty override counts as unset, like Python's `or`.
pub fn store_path_from(
    override_home: Option<OsString>,
    home: Option<OsString>,
) -> Result<PathBuf, StoreError> {
    let dir = match override_home.filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(home.filter(|v| !v.is_empty()).ok_or(StoreError::NoHome)?)
            .join(".easy-personal"),
    };
    Ok(dir.join("store.sqlite"))
}

/// Open the store with SQLite's read-only flag and `PRAGMA query_only=ON`; waits up to the busy timeout.
pub fn open_read_only(path: &Path) -> Result<Connection, StoreError> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    conn.execute_batch("PRAGMA query_only=ON")?;
    Ok(conn)
}

/// Open the store at `path` read-only for a surface that serves data: refuses a missing file
/// ([`StoreError::NoStore`]), a non-file ([`StoreError::NotAFile`]) and a store whose `user_version` is above
/// [`KNOWN_MIGRATION`] ([`StoreError::Newer`]). Never creates a file; no error carries the path (NFR-002).
pub fn open_checked(path: &Path) -> Result<Connection, StoreError> {
    match fs::metadata(path) {
        Ok(meta) if meta.is_file() => {}
        Ok(_) => return Err(StoreError::NotAFile),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(StoreError::NoStore),
        Err(e) => return Err(StoreError::Unreadable(format!("{:?}", e.kind()))),
    }
    let conn = open_read_only(path)?;
    let found: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if found > KNOWN_MIGRATION {
        return Err(StoreError::Newer {
            found,
            known: KNOWN_MIGRATION,
        });
    }
    Ok(conn)
}

/// Report on the store at `path`: `State::Missing` when there is no file (nothing is created),
/// `StoreError::Newer` when its `user_version` is above [`KNOWN_MIGRATION`].
pub fn read_info(path: &Path) -> Result<StoreInfo, StoreError> {
    let file = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    match fs::metadata(path) {
        Ok(meta) if meta.is_file() => {}
        Ok(_) => return Err(StoreError::NotAFile),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(StoreInfo {
                file,
                state: State::Missing,
            });
        }
        Err(e) => return Err(StoreError::Unreadable(format!("{:?}", e.kind()))),
    }

    let conn = open_read_only(path)?;
    let user_version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if user_version > KNOWN_MIGRATION {
        return Err(StoreError::Newer {
            found: user_version,
            known: KNOWN_MIGRATION,
        });
    }

    let names: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut tables = Vec::with_capacity(names.len());
    for name in names {
        let quoted = format!("\"{}\"", name.replace('"', "\"\""));
        let rows: i64 =
            conn.query_row(&format!("SELECT COUNT(*) FROM {quoted}"), [], |r| r.get(0))?;
        tables.push((name, rows));
    }
    Ok(StoreInfo {
        file,
        state: State::Present {
            user_version,
            tables,
        },
    })
}
