//! `easy-personal-rs ingest` (P2a, T029): Python's `easy-personal ingest` (`surfaces/cli.py` `_ingest`) for the
//! `claude_code` source only. Same environment, same order (lock, then store, then accounts, then transcripts, then
//! prune, then print), same JSON formatting, same exit codes. The core does the work (`ingest::folders`,
//! `writer`, `ingest_lock`); this module reads the environment and the process, and prints.
//!
//! After the events (P2b, T033) it runs `kit_trace`, `ui` and `verifier` in Python's order, each in its own
//! transaction, and prints their counts under Python's keys. The result names what it did not ingest (`not_ingested`:
//! `context`) instead of printing that source with an invented value. No message here carries a path, SQL or a row value (NFR-002).

use easy_personal_core::clock::now_micros;
use easy_personal_core::ingest::config::{ui_reports_dir, verifier_reports_dir};
use easy_personal_core::ingest::folders::{Options, SKIPPED_NOTE, ingest_accounts};
use easy_personal_core::ingest::{kit_trace, ui, verifier};
use easy_personal_core::ingest_lock::{DEFAULT_TIMEOUT, LockError, ingest_lock};
use easy_personal_core::store::store_path_from;
use easy_personal_core::writer::{WriteError, open_for_write};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

/// The source Python's `ingest` runs last that this command does not (T033: `kit_trace`, `ui` and `verifier` are ported).
const NOT_INGESTED: [&str; 1] = ["context"];

/// The longest lock wait we hand to `Instant` arithmetic (about 31 years); `inf` and `nan` wait this long.
const MAX_WAIT_SECS: u64 = 1_000_000_000;

/// Why a run stopped.
enum Failure {
    /// Another ingest holds the lock: Python's exact stderr line, exit 75.
    Busy,
    /// A store that must not be touched (newer or older than v11): exit 2.
    Refused(String),
    /// Any other failure: exit 1.
    Failed(String),
}

impl From<WriteError> for Failure {
    fn from(e: WriteError) -> Self {
        match e.exit_code() {
            2 => Failure::Refused(e.to_string()),
            _ => Failure::Failed(e.to_string()),
        }
    }
}

impl From<LockError> for Failure {
    fn from(e: LockError) -> Self {
        match e {
            LockError::Busy => Failure::Busy,
            other => Failure::Failed(other.to_string()),
        }
    }
}

pub fn run(source: Option<PathBuf>, retention_days: Option<i64>) -> ExitCode {
    match ingest(source, retention_days) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::Busy) => {
            eprintln!("another ingest is running; try again later");
            ExitCode::from(75)
        }
        Err(Failure::Refused(text)) => {
            eprintln!("easy-personal-rs: {text}");
            ExitCode::from(2)
        }
        Err(Failure::Failed(text)) => {
            eprintln!("easy-personal-rs: {text}");
            ExitCode::FAILURE
        }
    }
}

fn ingest(source: Option<PathBuf>, retention_days: Option<i64>) -> Result<(), Failure> {
    // cli.py:160 comes first: a malformed timeout stops the run before the data folder is made.
    let timeout = lock_timeout(std::env::var_os("EASY_PERSONAL_LOCK_TIMEOUT").as_deref())?;
    let home = std::env::var_os("HOME");
    let store_file = store_path_from(std::env::var_os("EASY_PERSONAL_HOME"), home.clone())
        .map_err(|e| Failure::Failed(e.to_string()))?;
    let data_dir = store_file
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let agent_root =
        match non_empty(std::env::var_os("EASY_PERSONAL_AGENT_ROOT")).or(non_empty(home.clone())) {
            Some(root) => PathBuf::from(root),
            None => {
                return Err(Failure::Failed(
                    "cannot find the agent folders: set EASY_PERSONAL_AGENT_ROOT or HOME".into(),
                ));
            }
        };
    // `--source` wins over the variable; an empty variable is unset (`config.claude_projects_override`).
    let projects_override = source
        .or_else(|| non_empty(std::env::var_os("EASY_PERSONAL_CLAUDE_PROJECTS")).map(PathBuf::from))
        .map(|p| like_python_path(&p));
    // The Python process's cwd; when it cannot be read, "." resolves against the (missing) cwd as the OS would.
    let cwd_base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let result = ingest_locked(&Config {
        timeout,
        store_file,
        data_dir,
        agent_root,
        projects_override,
        cwd_base,
        retention_days,
        ui_env: std::env::var_os("EASY_PERSONAL_UI_REPORTS"),
        verifier_env: std::env::var_os("EASY_PERSONAL_VERIFIER_REPORTS"),
        home,
    })?;
    let skipped = result.get("skipped").and_then(Value::as_u64).unwrap_or(0);
    // Python prints after its `with` block: the lock is free before a slow stdout reader (`| less`) gets the result.
    print_json(&result)?;
    if skipped > 0 {
        eprintln!("easy-personal-rs: {SKIPPED_NOTE}");
    }
    Ok(())
}

/// The environment, already resolved: what the locked part of the run needs.
struct Config {
    timeout: Duration,
    store_file: PathBuf,
    data_dir: PathBuf,
    agent_root: PathBuf,
    projects_override: Option<PathBuf>,
    cwd_base: PathBuf,
    retention_days: Option<i64>,
    /// `EASY_PERSONAL_UI_REPORTS`, `EASY_PERSONAL_VERIFIER_REPORTS` and `HOME`, as the process has them.
    ui_env: Option<OsString>,
    verifier_env: Option<OsString>,
    home: Option<OsString>,
}

/// The part of `_ingest` inside Python's `with` block: lock, then store, then accounts, then prune. The lock and the
/// store are dropped when this returns (store first, then the lock), so nothing printed after it holds the lock.
fn ingest_locked(cfg: &Config) -> Result<BTreeMap<&'static str, Value>, Failure> {
    // The lock before the store: a busy lock never touches the store.
    let _lock = ingest_lock(&cfg.data_dir, cfg.timeout)?;
    let mut store = open_for_write(&cfg.store_file)?;
    let summary = ingest_accounts(
        &mut store,
        &Options {
            agent_root: cfg.agent_root.clone(),
            projects_override: cfg.projects_override.clone(),
            cwd_base: cfg.cwd_base.clone(),
        },
    )?;
    // cli.py `_ingest`: after the events, `kit_trace`, then `ui`, then `verifier`; none of the last two fails the run.
    let kit = kit_trace::ingest(&mut store)?;
    let config_file = cfg.data_dir.join("config.yaml");
    let home = cfg.home.as_deref();
    let ui_folder = ui_reports_dir(&config_file, cfg.ui_env.clone(), home);
    let ui = match ui::ingest(&mut store, ui_folder.as_deref()) {
        Ok(s) => json!({"status": s.status, "sessions": s.sessions, "skipped": s.skipped}),
        Err(e) => json!({"status": "failed", "reason": ui_failure(&e)}),
    };
    let container = verifier_reports_dir(&config_file, cfg.verifier_env.clone(), home);
    let verifier = match verifier::ingest(&mut store, container.as_deref()) {
        Ok(s) => json!({"status": s.status, "projects": s.projects, "skipped": s.skipped}),
        Err(reason) => json!({"status": "failed", "reason": reason}),
    };
    let mut result: BTreeMap<&'static str, Value> = BTreeMap::new();
    result.insert("files", json!(summary.files));
    result.insert("new_events", json!(summary.new_events));
    result.insert("skipped", json!(summary.skipped + kit.skipped));
    result.insert(
        "kit_trace",
        json!({"files": kit.files, "new_events": kit.new_events}),
    );
    result.insert("ui", ui);
    result.insert("verifier", verifier);
    result.insert("not_ingested", json!(NOT_INGESTED));
    if let Some(days) = cfg.retention_days {
        result.insert("pruned", json!(store.prune(Some(days), now_micros())?));
    }
    Ok(result)
}

/// The exception class name Python would show for a store failure while saving the UI sessions.
fn ui_failure(e: &WriteError) -> &'static str {
    match e {
        WriteError::Rejected => "IntegrityError",
        _ => "OperationalError",
    }
}

/// `print(json.dumps(value, indent=2, sort_keys=True))`: two-space indent, sorted keys, one item per line, a newline.
/// A closed stdout is a failure with a message, not a panic.
fn print_json(value: &BTreeMap<&'static str, Value>) -> Result<(), Failure> {
    let mut text = serde_json::to_string_pretty(value)
        .map_err(|_| Failure::Failed("cannot format the result".into()))?;
    text.push('\n');
    let mut out = std::io::stdout().lock();
    out.write_all(text.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|_| Failure::Failed("cannot write the result to stdout".into()))
}

fn non_empty(value: Option<OsString>) -> Option<OsString> {
    value.filter(|v| !v.is_empty())
}

/// `float(os.environ.get("EASY_PERSONAL_LOCK_TIMEOUT", 300))`: seconds; unset is 300; anything that is not a number is
/// a clean error (Python: a traceback). A negative wait is one try; `inf` and `nan` wait as long as we can count.
fn lock_timeout(raw: Option<&OsStr>) -> Result<Duration, Failure> {
    let Some(raw) = raw else {
        return Ok(DEFAULT_TIMEOUT);
    };
    let secs: f64 = raw
        .to_str()
        .and_then(|text| text.trim().parse().ok())
        .ok_or_else(|| {
            Failure::Failed("EASY_PERSONAL_LOCK_TIMEOUT is not a number of seconds".into())
        })?;
    Ok(if secs <= 0.0 {
        Duration::ZERO
    } else if secs < MAX_WAIT_SECS as f64 {
        Duration::from_secs_f64(secs)
    } else {
        Duration::from_secs(MAX_WAIT_SECS)
    })
}

/// `Path(text)`: drops `.` parts (so a leading `./`), repeated and trailing separators; an empty result is `.`.
/// The core's `components()` keeps a leading `./`, so the folder keys would differ from Python's.
fn like_python_path(path: &Path) -> PathBuf {
    let kept: PathBuf = path
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect();
    if kept.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        kept
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(text: &str) -> Result<Duration, ()> {
        lock_timeout(Some(OsStr::new(text))).map_err(|_| ())
    }

    #[test]
    fn the_lock_timeout_reads_like_pythons_float() {
        assert_eq!(lock_timeout(None).ok(), Some(Duration::from_secs(300)));
        assert_eq!(secs("3"), Ok(Duration::from_secs(3)));
        assert_eq!(secs(" 0.2 "), Ok(Duration::from_millis(200)));
        assert_eq!(secs("1e1"), Ok(Duration::from_secs(10)));
        assert_eq!(secs("0"), Ok(Duration::ZERO));
        assert_eq!(secs("-5"), Ok(Duration::ZERO));
    }

    #[test]
    fn a_huge_or_non_finite_timeout_is_capped_not_a_panic() {
        let max = Duration::from_secs(MAX_WAIT_SECS);
        for text in ["inf", "nan", "1e300", "1e18"] {
            assert_eq!(secs(text), Ok(max), "{text}");
        }
    }

    #[test]
    fn a_malformed_timeout_is_an_error() {
        for text in ["", "abc", "3s", "1,5", "--"] {
            assert_eq!(secs(text), Err(()), "{text:?}");
        }
        assert!(lock_timeout(Some(&OsString::from("\u{e9}"))).is_err());
    }

    #[test]
    fn a_path_reads_like_pythons_path() {
        for (given, want) in [
            ("./a//b/", "a/b"),
            ("a/./b", "a/b"),
            ("./", "."),
            ("", "."),
            (".", "."),
            ("/x/./y/", "/x/y"),
            ("../x", "../x"),
            ("a/..", "a/.."),
        ] {
            assert_eq!(
                like_python_path(Path::new(given)),
                Path::new(want),
                "{given:?}"
            );
        }
    }

    #[test]
    fn the_lock_is_free_as_soon_as_the_locked_part_returns() {
        let base = std::env::temp_dir().join(format!(
            "ep-t029-lock-{}-{}",
            std::process::id(),
            now_micros()
        ));
        let (data_dir, agent_root) = (base.join("data"), base.join("agent"));
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::create_dir_all(&agent_root).unwrap();
        let cfg = Config {
            timeout: Duration::ZERO,
            store_file: data_dir.join("store.sqlite"),
            data_dir: data_dir.clone(),
            agent_root,
            projects_override: None,
            cwd_base: base.clone(),
            retention_days: None,
            ui_env: None,
            verifier_env: None,
            home: None,
        };
        let result = ingest_locked(&cfg).ok().expect("ingest ran");
        // Nothing has been printed yet, and the lock is already free (Python prints after its `with` block).
        let again = ingest_lock(&data_dir, Duration::ZERO);
        assert!(
            again.is_ok(),
            "the lock was still held after the locked part"
        );
        drop(again);
        let keys: Vec<&str> = result.keys().copied().collect();
        assert_eq!(
            keys,
            [
                "files",
                "kit_trace",
                "new_events",
                "not_ingested",
                "skipped",
                "ui",
                "verifier"
            ]
        );
        std::fs::remove_dir_all(&base).unwrap();
    }
}
