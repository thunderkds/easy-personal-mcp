//! Shared by the `ingest_folders_*` tests (T035): one oracle case as `rust/parity/gen_ingest.py` `Case` builds it
//! (fresh folders, the work root padded to `repos.json` `work_path_length`, `%WORK%` replaced, the layout steps run
//! with the fixed git env), the Rust ingest over it, and the four tables dumped in the goldens' form.
#![allow(dead_code)] // each test crate uses a part

use easy_personal_core::ingest::folders::{Options, Summary, ingest_accounts};
use easy_personal_core::writer::open_for_write;
use rusqlite::Connection;
use serde_json::{Map, Value, json};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

pub fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

pub fn committed() -> PathBuf {
    repo().join("tests/fixtures/claude_code")
}

pub fn fixtures() -> PathBuf {
    repo().join("rust/parity/ingest/fixtures")
}

pub fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

pub fn golden(name: &str) -> Value {
    read_json(&repo().join("rust/parity/ingest/golden").join(name))
}

pub fn repos() -> Value {
    read_json(&fixtures().join("repos.json"))
}

/// One scenario's folders, removed on drop.
pub struct Case {
    pub root: PathBuf,
    pub home: PathBuf,
    pub agents: PathBuf,
    pub projects: PathBuf,
    pub nowhere: PathBuf,
    pub work: PathBuf,
}

impl Case {
    pub fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let root = std::env::temp_dir().join(format!(
            "ep-t035-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let root = fs::canonicalize(root).unwrap();
        let len = repos()["work_path_length"].as_u64().unwrap() as usize;
        let pad = len - root.to_str().unwrap().len() - 1;
        assert!(
            pad >= 4,
            "temp folder path too long for the padded work root"
        );
        let case = Case {
            home: root.join("home"),
            agents: root.join("agents"),
            projects: root.join("agents/.claude/projects"),
            nowhere: root.join("nowhere"),
            work: root.join("w".repeat(pad)),
            root,
        };
        for d in [&case.agents, &case.nowhere, &case.work] {
            fs::create_dir_all(d).unwrap();
        }
        assert_eq!(case.work.to_str().unwrap().len(), len);
        case
    }

    /// `Case.copy`: a fixture file or folder (its regular files, recursively), `%WORK%` replaced by the work root.
    pub fn copy(&self, source: &Path, target: &Path) {
        let work = self.work.to_str().unwrap().as_bytes().to_vec();
        let put = |from: &Path, to: &Path| {
            fs::create_dir_all(to.parent().unwrap()).unwrap();
            let bytes = fs::read(from).unwrap();
            let mut out = Vec::with_capacity(bytes.len());
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i..].starts_with(b"%WORK%") {
                    out.extend_from_slice(&work);
                    i += 6;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            fs::write(to, out).unwrap();
        };
        if source.is_file() {
            put(source, target);
            return;
        }
        let mut pending = vec![source.to_path_buf()];
        while let Some(dir) = pending.pop() {
            for entry in fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    pending.push(path);
                } else if path.is_file() {
                    put(&path, &target.join(path.strip_prefix(source).unwrap()));
                }
            }
        }
    }

    /// `Case.apply`: the `repos.json` layout steps, in order.
    pub fn apply(&self, ops: &Value) {
        let real = |text: &str| {
            text.replace("<WORK>", self.work.to_str().unwrap())
                .replace("<PROJECTS>", self.projects.to_str().unwrap())
                .replace("<AGENT_ROOT>", self.agents.to_str().unwrap())
        };
        let git_env = repos()["git_env"].as_object().unwrap().clone();
        for op in ops.as_array().unwrap() {
            let path = PathBuf::from(real(
                op.get("path")
                    .or(op.get("cwd"))
                    .and_then(Value::as_str)
                    .unwrap(),
            ));
            match op["op"].as_str().unwrap() {
                "mkdir" => fs::create_dir_all(&path).unwrap(),
                "rmdir" => fs::remove_dir(&path).unwrap(),
                "write" => fs::write(&path, op["text"].as_str().unwrap()).unwrap(),
                "symlink" => {
                    fs::create_dir_all(path.parent().unwrap()).unwrap();
                    std::os::unix::fs::symlink(real(op["target"].as_str().unwrap()), &path)
                        .unwrap();
                }
                "git" => {
                    let mut cmd = Command::new("timeout");
                    cmd.arg("30").arg("git").arg("-C").arg(&path);
                    for a in op["args"].as_array().unwrap() {
                        cmd.arg(real(a.as_str().unwrap()));
                    }
                    cmd.env("HOME", &self.nowhere);
                    for (k, v) in &git_env {
                        cmd.env(k, v.as_str().unwrap());
                    }
                    let out = cmd.output().unwrap();
                    assert!(out.status.success(), "layout git step failed");
                }
                other => panic!("unknown layout op {other}"),
            }
        }
    }

    pub fn store_path(&self) -> PathBuf {
        self.home.join("store.sqlite")
    }

    /// The Rust ingest as T029 will call it: cwd base = the work root (the oracle's process cwd).
    pub fn ingest(&self, projects_override: Option<&Path>) -> Summary {
        fs::create_dir_all(&self.home).unwrap();
        let mut store = open_for_write(&self.store_path()).unwrap();
        let opts = Options {
            agent_root: self.agents.clone(),
            projects_override: projects_override.map(Path::to_path_buf),
            cwd_base: self.work.clone(),
        };
        ingest_accounts(&mut store, &opts).unwrap()
    }

    pub fn sql(&self, script: &str) {
        Connection::open(self.store_path())
            .unwrap()
            .execute_batch(script)
            .unwrap();
    }

    /// `Case.tok`: the longest path first.
    pub fn tok(&self, text: &str) -> String {
        let mut pairs = [
            (self.work.to_str().unwrap(), "<WORK>"),
            (self.agents.to_str().unwrap(), "<AGENT_ROOT>"),
            (self.home.to_str().unwrap(), "<HOME>"),
            (self.root.to_str().unwrap(), "<CASE>"),
        ];
        pairs.sort_by_key(|(p, _)| std::cmp::Reverse(p.len()));
        let mut out = text.to_owned();
        for (needle, token) in pairs {
            out = out.replace(needle, token);
        }
        out
    }

    fn inode_token(path: &str, inode: i64) -> &'static str {
        match fs::metadata(path) {
            Ok(m) if m.ino() as i64 == inode => "<INODE_OF_FILE>",
            Ok(_) => "<INODE_OTHER>",
            Err(_) => "<INODE_FILE_GONE>",
        }
    }

    /// `Case._dump` for the four tables, rows as JSON objects; `machine_id_check` as the golden has it.
    pub fn dump(&self) -> Value {
        let c = Connection::open(self.store_path()).unwrap();
        let mid: String = c
            .query_row("SELECT value FROM meta WHERE key = 'machine_id'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let folders: std::collections::HashMap<i64, String> = c
            .prepare("SELECT id, folder FROM agent_accounts")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let mut out = Map::new();
        let mut check = Map::new();
        for table in ["events", "tool_signals", "source_files", "agent_accounts"] {
            let mut stmt = c
                .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
                .unwrap();
            let names: Vec<String> = stmt.column_names().map(str::to_owned).collect();
            let mut same = true;
            let mut rows: Vec<Value> = stmt
                .query_map([], |r| {
                    let mut row = Map::new();
                    for (i, name) in names.iter().enumerate() {
                        let v: rusqlite::types::Value = r.get(i)?;
                        row.insert(
                            name.clone(),
                            match v {
                                rusqlite::types::Value::Null => Value::Null,
                                rusqlite::types::Value::Integer(n) => json!(n),
                                rusqlite::types::Value::Text(t) => json!(t),
                                other => panic!("unexpected {other:?}"),
                            },
                        );
                    }
                    Ok(row)
                })
                .unwrap()
                .map(|row| {
                    let mut row = row.unwrap();
                    if let Some(m) = row.remove("machine_id") {
                        same &= m == json!(mid);
                    }
                    if let Some(inode) = row.get("inode").and_then(Value::as_i64) {
                        let path = row
                            .get("source_path")
                            .or(row.get("path"))
                            .and_then(Value::as_str)
                            .unwrap();
                        row.insert("inode".into(), json!(Self::inode_token(path, inode)));
                    }
                    if table == "events"
                        && let Some(id) = row["agent_account_id"].as_i64()
                    {
                        row.insert("agent_account_id".into(), json!(folders[&id]));
                    }
                    if table == "events" || table == "agent_accounts" {
                        row.remove("id");
                    }
                    let text = self.tok(&Value::Object(row).to_string());
                    serde_json::from_str(&text).unwrap()
                })
                .collect();
            if table != "agent_accounts" {
                rows.sort_by_key(Value::to_string);
                check.insert(table.into(), json!(same));
            }
            out.insert(table.into(), Value::Array(rows));
        }
        out.insert("machine_id_check".into(), Value::Object(check));
        Value::Object(out)
    }

    /// The four tables (and `machine_id_check`) equal to a golden run's `tables_after`, row for row, every column.
    pub fn assert_tables(&self, run: &Value, what: &str) {
        let got = self.dump();
        let want = &run["tables_after"];
        for table in ["events", "tool_signals", "source_files", "agent_accounts"] {
            let mut rows = want[table]["rows"].as_array().unwrap().clone();
            if table != "agent_accounts" {
                rows.sort_by_key(Value::to_string);
            }
            let have = got[table].as_array().unwrap();
            assert_eq!(have.len(), rows.len(), "{what}: {table} row count");
            for (g, w) in have.iter().zip(&rows) {
                assert_eq!(g, w, "{what}: {table}");
            }
        }
        for (table, flag) in want["machine_id_check"].as_object().unwrap() {
            if let Some(mine) = got["machine_id_check"].get(table) {
                assert_eq!(mine, flag, "{what}: machine_id_check {table}");
            }
        }
    }

    /// The run's `files` / `new_events` equal the golden's `stdout_json`.
    pub fn assert_summary(summary: Summary, run: &Value, what: &str) {
        assert_eq!(
            summary.files,
            run["stdout_json"]["files"].as_i64().unwrap(),
            "{what}: files"
        );
        assert_eq!(
            summary.new_events,
            run["stdout_json"]["new_events"].as_i64().unwrap(),
            "{what}: new_events"
        );
        assert_eq!(summary.skipped, 0, "{what}: skipped");
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
