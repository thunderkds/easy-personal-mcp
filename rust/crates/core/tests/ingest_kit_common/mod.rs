//! Shared by the `ingest_kit_*` tests (T033): plays the *steps* of a `golden-kit` scenario (the same list
//! `rust/parity/gen_ingest_kit.py` `KCase.play` runs against Python), runs the Rust ingest at each `ingest` step, and
//! dumps `kit_trace_events`, `ui_sessions`, `verifier_scores` and the `kit_trace` rows of `source_files` in the goldens' form.
#![allow(dead_code)] // each test crate uses a part

#[path = "../ingest_folders_common/mod.rs"]
pub mod common;

use common::{Case, read_json, repo};
use easy_personal_core::ingest::config::{ui_reports_dir, verifier_reports_dir};
use easy_personal_core::ingest::folders::{Options, ingest_accounts};
use easy_personal_core::ingest::{kit_trace, ui, verifier};
use easy_personal_core::writer::open_for_write;
use rusqlite::Connection;
use serde_json::{Map, Value, json};
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const CANARY: &str = "INVENTED_CANARY";

pub fn kit_golden(name: &str) -> Value {
    read_json(&repo().join("rust/parity/ingest/golden-kit").join(name))
}

/// What one Rust run printed: the three K objects (the keys of Python's JSON) and the skipped count.
#[derive(Debug, Clone)]
pub struct Printed {
    pub kit_trace: Value,
    pub ui: Value,
    pub verifier: Value,
    pub kit_skipped: i64,
}

pub struct Kit {
    pub case: Case,
}

impl Kit {
    pub fn new() -> Self {
        let case = Case::new();
        fs::create_dir_all(case.root.join("verifier-none")).unwrap();
        Kit { case }
    }

    fn paths(&self) -> [(&'static str, PathBuf); 8] {
        let c = &self.case;
        [
            ("<WORK>", c.work.clone()),
            ("<PROJECTS>", c.projects.clone()),
            ("<AGENT_ROOT>", c.agents.clone()),
            ("<HOME>", c.home.clone()),
            ("<NOWHERE>", c.nowhere.clone()),
            ("<VERIFIER>", c.root.join("verifier-none")),
            ("<UI>", c.root.join("ui")),
            ("<CASE>", c.root.clone()),
        ]
    }

    pub fn expand(&self, text: &str) -> String {
        let mut out = text.to_owned();
        for (token, path) in self.paths() {
            out = out.replace(token, path.to_str().unwrap());
        }
        out.replace("%CANARY%", CANARY)
    }

    fn body(&self, step: &Value) -> Vec<u8> {
        if let Some(r) = step.get("repeat") {
            let s = |k: &str| self.expand(r.get(k).and_then(Value::as_str).unwrap_or(""));
            let count = r["count"].as_u64().unwrap() as usize;
            return format!(
                "{}{}{}{}",
                s("prefix"),
                s("unit").repeat(count),
                s("close").repeat(count),
                s("suffix")
            )
            .into_bytes();
        }
        self.expand(step["text"].as_str().unwrap()).into_bytes()
    }

    /// One non-`ingest` step of `KCase.play`.
    pub fn step(&self, step: &Value) {
        let op = step["op"].as_str().unwrap();
        let path = step
            .get("path")
            .and_then(Value::as_str)
            .map(|p| PathBuf::from(self.expand(p)));
        if let Some(parent) = path.as_deref().and_then(Path::parent)
            && matches!(
                op,
                "write" | "append" | "rewrite" | "replace" | "pad" | "symlink" | "mkfifo"
            )
        {
            fs::create_dir_all(parent).unwrap();
        }
        match op {
            "mkdir" => fs::create_dir_all(path.unwrap()).unwrap(),
            "write" | "rewrite" => fs::write(path.unwrap(), self.body(step)).unwrap(),
            "append" => {
                use std::io::Write;
                fs::OpenOptions::new()
                    .append(true)
                    .open(path.unwrap())
                    .unwrap()
                    .write_all(&self.body(step))
                    .unwrap();
            }
            "replace" => {
                let path = path.unwrap();
                let new = path.with_file_name(format!(
                    "{}.new",
                    path.file_name().unwrap().to_str().unwrap()
                ));
                fs::write(&new, self.body(step)).unwrap();
                fs::rename(new, path).unwrap();
            }
            "pad" => fs::File::create(path.unwrap())
                .unwrap()
                .set_len(step["size"].as_u64().unwrap())
                .unwrap(),
            "symlink" => std::os::unix::fs::symlink(
                self.expand(step["target"].as_str().unwrap()),
                path.unwrap(),
            )
            .unwrap(),
            "mkfifo" => assert!(
                Command::new("mkfifo")
                    .arg(path.unwrap())
                    .status()
                    .unwrap()
                    .success()
            ),
            "remove" => fs::remove_file(path.unwrap()).unwrap(),
            "rmtree" => fs::remove_dir_all(path.unwrap()).unwrap(),
            "transcript" => {
                let lines: Vec<String> = step["cwds"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .enumerate()
                    .map(|(i, cwd)| {
                        json!({"type": "user", "sessionId": "00000000-0000-4000-8000-000000001030",
                               "timestamp": format!("2026-01-10T15:00:0{i}.000Z"), "version": "2.1.400",
                               "cwd": self.expand(cwd.as_str().unwrap()), "gitBranch": "main", "isSidechain": false,
                               "userType": "external", "message": {"role": "user", "content": CANARY}})
                        .to_string()
                    })
                    .collect();
                let target = self.case.projects.join("p/session.jsonl");
                fs::create_dir_all(target.parent().unwrap()).unwrap();
                fs::write(target, lines.join("\n") + "\n").unwrap();
            }
            other => panic!("unknown step {other}"),
        }
    }

    /// The Rust `ingest` after one `ingest` step: the events, then the three kit sources, with the step's env.
    pub fn ingest(&self, step: &Value) -> Printed {
        let c = &self.case;
        fs::create_dir_all(&c.home).unwrap();
        let mut store = open_for_write(&c.store_path()).unwrap();
        ingest_accounts(
            &mut store,
            &Options {
                agent_root: c.agents.clone(),
                projects_override: None,
                cwd_base: c.work.clone(),
            },
        )
        .unwrap();
        let kit = kit_trace::ingest(&mut store).unwrap();
        // The env the oracle's child had: the verifier folder is set unless the step unsets it; the ui one is not.
        let env = |name: &str, default: Option<String>| -> Option<OsString> {
            match step.get("env").and_then(|e| e.get(name)) {
                Some(Value::String(v)) => Some(OsString::from(self.expand(v))),
                Some(_) => None,
                None => default.map(OsString::from),
            }
        };
        let config = c.home.join("config.yaml");
        let home = Some(c.nowhere.as_os_str());
        let ui_env = env("EASY_PERSONAL_UI_REPORTS", None);
        let folder = ui_reports_dir(&config, ui_env, home);
        let ui = ui::ingest(&mut store, folder.as_deref()).unwrap();
        let ver_env = env(
            "EASY_PERSONAL_VERIFIER_REPORTS",
            Some(self.expand("<VERIFIER>")),
        );
        let container = verifier_reports_dir(&config, ver_env, home);
        let verifier = verifier::ingest(&mut store, container.as_deref()).unwrap();
        Printed {
            kit_trace: json!({"files": kit.files, "new_events": kit.new_events}),
            ui: json!({"status": ui.status, "sessions": ui.sessions, "skipped": ui.skipped}),
            verifier: json!({"status": verifier.status, "projects": verifier.projects, "skipped": verifier.skipped}),
            kit_skipped: kit.skipped,
        }
    }

    fn inode_token(path: &str, inode: i64) -> &'static str {
        match fs::metadata(path) {
            Ok(m) if m.ino() as i64 == inode => "<INODE_OF_FILE>",
            Ok(_) => "<INODE_OTHER>",
            Err(_) => "<INODE_FILE_GONE>",
        }
    }

    /// `KCase._dump`: rows as objects (ids, machine id dropped; paths and inodes tokenised), sorted by their text.
    pub fn dump(&self) -> Value {
        let c = Connection::open(self.case.store_path()).unwrap();
        let mid: String = c
            .query_row("SELECT value FROM meta WHERE key = 'machine_id'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let mut out = Map::new();
        let mut check = Map::new();
        for (table, sql) in [
            ("kit_trace_events", "SELECT * FROM kit_trace_events"),
            ("ui_sessions", "SELECT * FROM ui_sessions"),
            ("verifier_scores", "SELECT * FROM verifier_scores"),
            (
                "source_files",
                "SELECT * FROM source_files WHERE source = 'kit_trace'",
            ),
        ] {
            let mut stmt = c.prepare(sql).unwrap();
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
                                rusqlite::types::Value::Real(f) => json!(f),
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
                    if table == "kit_trace_events" {
                        row.remove("id");
                    }
                    let text = self.case.tok(&Value::Object(row).to_string());
                    assert!(
                        !text.contains(CANARY),
                        "{table}: a planted canary reached the store"
                    );
                    serde_json::from_str(&text).unwrap()
                })
                .collect();
            rows.sort_by_key(Value::to_string);
            if table == "kit_trace_events" || table == "source_files" {
                check.insert(table.into(), json!(same));
            }
            out.insert(table.into(), Value::Array(rows));
        }
        out.insert("machine_id_check".into(), Value::Object(check));
        Value::Object(out)
    }

    /// The four tables equal to a golden run's `tables_after`, row for row, every column.
    pub fn assert_tables(&self, run: &Value, what: &str) {
        let got = self.dump();
        for table in [
            "kit_trace_events",
            "ui_sessions",
            "verifier_scores",
            "source_files",
        ] {
            let mut want = run["tables_after"][table]["rows"]
                .as_array()
                .unwrap()
                .clone();
            want.sort_by_key(Value::to_string);
            assert_eq!(got[table].as_array().unwrap(), &want, "{what}: {table}");
        }
        for (table, flag) in run["tables_after"]["machine_id_check"].as_object().unwrap() {
            assert_eq!(
                &got["machine_id_check"][table], flag,
                "{what}: machine_id_check {table}"
            );
        }
    }

    /// Python's printed K objects for one golden run equal what Rust printed.
    pub fn assert_printed(printed: &Printed, run: &Value, what: &str) {
        let j = &run["stdout_json"];
        assert_eq!(printed.kit_trace, j["kit_trace"], "{what}: kit_trace");
        assert_eq!(printed.ui, j["ui"], "{what}: ui");
        assert_eq!(printed.verifier, j["verifier"], "{what}: verifier");
    }
}

/// Plays a whole golden scenario and compares every `ingest` step. A golden run where Python crashed (exit code not 0)
/// is not copied: Rust must skip, so its rows must contain Python's and the entry must be counted as skipped.
pub fn replay(name: &str) {
    let g = kit_golden(name);
    let kit = Kit::new();
    let runs = g["runs"].as_array().unwrap();
    let mut next = 0;
    for step in g["steps"].as_array().unwrap() {
        if step["op"] != "ingest" {
            kit.step(step);
            continue;
        }
        let run = &runs[next];
        next += 1;
        assert_eq!(run["label"], step["label"]);
        let what = format!(
            "{} run {next} ({})",
            g["scenario"].as_str().unwrap(),
            run["label"].as_str().unwrap()
        );
        let printed = kit.ingest(step);
        if run["exit_code"] == 0 {
            Kit::assert_printed(&printed, run, &what);
            kit.assert_tables(run, &what);
        } else {
            assert!(
                printed.kit_skipped >= 1,
                "{what}: the entry Python crashed on is counted as skipped"
            );
            let got = kit.dump();
            for row in run["tables_after"]["kit_trace_events"]["rows"]
                .as_array()
                .unwrap()
            {
                assert!(
                    got["kit_trace_events"].as_array().unwrap().contains(row),
                    "{what}: a row Python stored is missing"
                );
            }
        }
    }
    assert_eq!(next, runs.len(), "every golden run was replayed");
}
