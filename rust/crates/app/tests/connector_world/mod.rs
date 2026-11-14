//! The world of the connector oracles (T050 `connector_parity.rs`, T060 `connector_cli_parity.rs`): a temp copy of
//! `store-rich` with a case's folders and setup SQL, the debug-build fake keychain file, the oracle's fake `claude`
//! (`golden-connectors/fake-claude.sh`) with its per-account state, and the readers that normalise what a run left
//! (the work folder as `<WORK>`, the launcher as `<LAUNCHER>`, a connector id the run made as the golden's).
#![allow(dead_code)] // each test crate uses a part

use rusqlite::Connection;
use rusqlite::types::ValueRef;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

pub const SERVE: &str = env!("CARGO_BIN_EXE_easy-personal-rs");
pub const LAUNCH: &str = env!("CARGO_BIN_EXE_easy-personal-launch");
pub const NO_BUS: &str = "unix:path=/nonexistent/t050-no-bus";

pub fn parity(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity")
        .join(rel)
}

pub fn temp(tag: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "easy-personal-rs-t050-{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::canonicalize(dir).unwrap()
}

/// One case's folder: `home/store.sqlite`, the keychain file, `bin/claude` (when the case has a `claude`), its state and log.
pub struct World {
    pub dir: PathBuf,
    pub work: String,
    pub store: PathBuf,
    /// The value of `EASY_PERSONAL_TEST_KEYCHAIN`: the file, or `unusable` (no backend).
    pub keychain_env: PathBuf,
    pub keychain: PathBuf,
    pub state: PathBuf,
    pub log: PathBuf,
    /// The connector ids the store held before the run.
    pub known: Vec<String>,
}

impl World {
    /// Builds the case's world (`dirs`, `setup`, `keychain`, `claude` as `gen_connectors.py` writes them).
    pub fn build(case: &Value) -> World {
        let name = case["name"].as_str().unwrap();
        let dir = temp("case");
        let work = dir.to_str().unwrap().to_string();
        for d in case["dirs"].as_array().unwrap() {
            std::fs::create_dir_all(dir.join(d.as_str().unwrap())).unwrap();
        }
        for sub in ["home", "nowhere", "bin"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        let store = dir.join("home/store.sqlite");
        std::fs::copy(parity("store-rich/store-v11.sqlite"), &store).unwrap();
        let mut known = Vec::new();
        {
            let conn = Connection::open(&store).unwrap();
            conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))
                .unwrap();
            conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
            conn.execute(
                "UPDATE agent_accounts SET folder = replace(folder, '/work/rich', ?1), config_dir_env = replace(config_dir_env, '/work/rich', ?1)",
                [&work],
            )
            .unwrap();
            for sql in case["setup"].as_array().unwrap() {
                conn.execute(&sql.as_str().unwrap().replace("@W@", &work), [])
                    .unwrap();
            }
            let mut stmt = conn.prepare("SELECT id FROM connectors").unwrap();
            known.extend(
                stmt.query_map([], |r| r.get::<_, String>(0))
                    .unwrap()
                    .flatten(),
            );
        }
        let keychain = dir.join("keychain.json");
        let keychain_env: PathBuf = match &case["keychain"] {
            Value::String(none) if none == "none" => PathBuf::from("unusable"),
            Value::Object(items) => {
                std::fs::write(&keychain, Value::Object(items.clone()).to_string()).unwrap();
                keychain.clone()
            }
            other => panic!("{name}: keychain {other}"),
        };
        let (state, log) = (dir.join("claude-state"), dir.join("claude.log"));
        if let Value::Object(accounts) = &case["claude"] {
            std::fs::copy(
                parity("golden-connectors/fake-claude.sh"),
                dir.join("bin/claude"),
            )
            .unwrap();
            for (ccd, entries) in accounts {
                let value = ccd.replace("@W@", &work);
                let folder = state.join(ccd_key(&value));
                std::fs::create_dir_all(&folder).unwrap();
                std::fs::write(folder.join(".ccd"), &value).unwrap();
                for (alias, entry) in entries.as_object().unwrap() {
                    let command = entry[0].as_str().unwrap().replace("@LAUNCHER@", LAUNCH);
                    std::fs::write(
                        folder.join(alias),
                        format!("Command: {command}\nArgs: {}\n", entry[1].as_str().unwrap()),
                    )
                    .unwrap();
                }
            }
        }
        World {
            dir,
            work,
            store,
            keychain_env,
            keychain,
            state,
            log,
            known,
        }
    }

    /// The environment of a run, built from nothing: no real `claude` (PATH is the fake's folder, `/usr/bin` and `/bin`),
    /// no real keychain (the fake file, and a session bus address that leads nowhere), temp HOME and data folders.
    pub fn env(&self, cmd: &mut Command, case: &Value, tz: &str) {
        let dir = &self.dir;
        cmd.env_clear()
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", dir.join("bin").display()),
            )
            .env("HOME", dir.join("nowhere"))
            .env("XDG_CONFIG_HOME", dir.join("nowhere/.config"))
            .env("EASY_PERSONAL_HOME", dir.join("home"))
            .env("EASY_PERSONAL_AGENT_ROOT", dir.join("agent-root"))
            .env("EASY_PERSONAL_TEST_KEYCHAIN", &self.keychain_env)
            .env("DBUS_SESSION_BUS_ADDRESS", NO_BUS)
            .env("FAKE_CLAUDE_LOG", &self.log)
            .env("FAKE_CLAUDE_STATE", &self.state)
            .env("FAKE_CLAUDE_STDERR", "")
            .env("CLAUDE_CONFIG_DIR", "/runner/dir")
            .env("TZ", tz)
            .current_dir(dir);
        if let Some(fail) = case["fail"].as_str() {
            cmd.env("FAKE_CLAUDE_FAIL", fail);
        }
    }

    /// The keychain file after the run, or `"none"` for no backend.
    pub fn keychain_after(&self) -> Value {
        if self.keychain_env == Path::new("unusable") {
            json!("none")
        } else {
            serde_json::from_str(
                &std::fs::read_to_string(&self.keychain).unwrap_or_else(|_| "{}".into()),
            )
            .unwrap()
        }
    }
}

impl Drop for World {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

// ---- normalising ----

/// Connector ids this run made (`c` + 8 lowercase hex digits, not in `known`), in order of first appearance.
pub fn new_ids(text: &str, known: &[String], into: &mut Vec<String>) {
    let b = text.as_bytes();
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'-';
    for i in 0..b.len().saturating_sub(8) {
        let Some(token) = text.get(i..i + 9) else {
            continue;
        };
        let hex = token[1..]
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c));
        let bounded = (i == 0 || !word(b[i - 1])) && b.get(i + 9).is_none_or(|c| !word(*c));
        if b[i] == b'c'
            && hex
            && bounded
            && !known.iter().any(|k| k == token)
            && !into.iter().any(|k| k == token)
        {
            into.push(token.to_string());
        }
    }
}

pub struct Norm {
    pub work: String,
    pub launcher: String,
    pub ids: Vec<(String, String)>,
}

impl Norm {
    pub fn text(&self, s: &str) -> String {
        let mut s = s
            .replace(&self.work, "<WORK>")
            .replace(&self.launcher, "<LAUNCHER>");
        for (ours, theirs) in &self.ids {
            s = s.replace(ours, theirs);
        }
        s
    }

    pub fn value(&self, v: &Value) -> Value {
        match v {
            Value::String(s) => Value::String(self.text(s)),
            Value::Array(items) => Value::Array(items.iter().map(|i| self.value(i)).collect()),
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(k, v)| (self.text(k), self.value(v)))
                    .collect(),
            ),
            other => other.clone(),
        }
    }
}

pub fn dump(path: &Path) -> BTreeMap<String, Vec<Value>> {
    let conn = Connection::open(path).unwrap();
    let names: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let mut out = BTreeMap::new();
    for name in names {
        let mut stmt = conn.prepare(&format!("SELECT * FROM \"{name}\"")).unwrap();
        let width = stmt.column_count();
        let rows: Vec<Value> = stmt
            .query_map([], |r| {
                Ok(Value::Array(
                    (0..width)
                        .map(|i| match r.get_ref(i).unwrap() {
                            ValueRef::Null => Value::Null,
                            ValueRef::Integer(n) => json!(n),
                            ValueRef::Real(f) => json!(f),
                            ValueRef::Text(t) => json!(String::from_utf8_lossy(t)),
                            ValueRef::Blob(b) => {
                                json!(b.iter().map(|x| format!("{x:02x}")).collect::<String>())
                            }
                        })
                        .collect(),
                ))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        out.insert(name, rows);
    }
    out
}

/// Python's `json.dumps(value)` key used to sort rows (`sort_keys` off, default separators).
pub fn py_sort_key(v: &Value) -> String {
    match v {
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(py_sort_key).collect::<Vec<_>>().join(", ")
        ),
        Value::String(s) => easy_personal_core::pyjson::py_str(s),
        other => other.to_string(),
    }
}

/// The tables after the run against the golden's `changed` (a table not listed there must equal `before`).
pub fn table_problems(
    name: &str,
    before: &BTreeMap<String, Vec<Value>>,
    after: &BTreeMap<String, Vec<Value>>,
    changed: &Value,
    norm: &Norm,
) -> Vec<String> {
    let mut problems = Vec::new();
    for (table, rows) in after {
        let mut rows: Vec<Value> = rows.iter().map(|r| norm.value(r)).collect();
        rows.sort_by_key(py_sort_key);
        let want: Vec<Value> = match changed.get(table) {
            Some(golden) => golden.as_array().unwrap().clone(),
            None => {
                let mut b: Vec<Value> = before[table].iter().map(|r| norm.value(r)).collect();
                b.sort_by_key(py_sort_key);
                b
            }
        };
        if rows != want {
            problems.push(format!("{name}: table {table} {rows:?}\n  want {want:?}"));
        }
    }
    problems
}

/// The fake `claude`'s log as `[{"ccd", "argv"}]`, normalised, each run of `mcp get` calls sorted as the generator sorts
/// it (by Python's `json.dumps({"ccd": ..., "argv": [...]})`).
pub fn read_log(path: &Path, norm: &Norm) -> Vec<Value> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut calls: Vec<(String, Vec<String>)> = Vec::new();
    let mut cur: Option<(String, Vec<String>)> = None;
    for line in text.lines() {
        match cur.take() {
            None => {
                cur = Some((
                    norm.text(line.strip_prefix("CCD=").unwrap_or(line)),
                    Vec::new(),
                ))
            }
            Some(call) if line == "--" => calls.push(call),
            Some((ccd, mut argv)) => {
                argv.push(norm.text(line));
                cur = Some((ccd, argv));
            }
        }
    }
    let py = easy_personal_core::pyjson::py_str;
    let key = |(ccd, argv): &(String, Vec<String>)| {
        let argv: Vec<String> = argv.iter().map(|a| py(a)).collect();
        format!("{{\"ccd\": {}, \"argv\": [{}]}}", py(ccd), argv.join(", "))
    };
    let (mut out, mut run) = (Vec::new(), Vec::new());
    for call in calls.into_iter().map(Some).chain([None]) {
        if let Some(c) = call
            .as_ref()
            .filter(|(_, argv)| argv.len() > 1 && argv[0] == "mcp" && argv[1] == "get")
        {
            run.push(c.clone());
            continue;
        }
        run.sort_by_key(key);
        out.append(&mut run);
        out.extend(call);
    }
    out.into_iter()
        .map(|(ccd, argv)| json!({"ccd": ccd, "argv": argv}))
        .collect()
}

/// The fake's folder name for an account (`fake-claude.sh`): every byte outside `[A-Za-z0-9._-]` as `_`.
pub fn ccd_key(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"._-".contains(&b) {
                b as char
            } else {
                '_'
            }
        })
        .collect()
}

pub fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

pub fn read_state(dir: &Path) -> Value {
    let mut out = Map::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Value::Object(out);
    };
    let mut folders: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    folders.sort();
    for folder in folders {
        let ccd = std::fs::read_to_string(folder.join(".ccd")).unwrap();
        let mut files: Vec<PathBuf> = std::fs::read_dir(&folder)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.file_name().is_some_and(|n| n != ".ccd"))
            .collect();
        files.sort();
        let entries: Map<String, Value> = files
            .iter()
            .map(|f| {
                (
                    f.file_name().unwrap().to_string_lossy().into_owned(),
                    json!(std::fs::read_to_string(f).unwrap()),
                )
            })
            .collect();
        out.insert(ccd, Value::Object(entries));
    }
    Value::Object(out)
}
