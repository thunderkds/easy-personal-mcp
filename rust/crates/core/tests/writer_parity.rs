//! T027: the writer replays what Python's store layer did in T026's ingest goldens (`rust/parity/ingest/golden/`).
//! Each scenario's rows are fed back through `Store::save_file` / `retire_files` / `prune` on a fresh temp store; the
//! tables after each step and the `added` counts must equal the goldens (paths and inodes as the goldens tokenise them).
//! The parser is T028: here the rows come from the goldens, so this proves the store layer, not the reading of lines.

use easy_personal_core::writer::{Event, Line, Signal, Store, open_for_write};
use rusqlite::Connection;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t027-parity-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn store(&self) -> PathBuf {
        self.0.join("store.sqlite")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn golden(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity/ingest/golden")
        .join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn runs(g: &Value) -> &Vec<Value> {
    g["runs"].as_array().unwrap()
}

fn rows<'a>(tables: &'a Value, table: &str) -> &'a Vec<Value> {
    tables[table]["rows"].as_array().unwrap()
}

/// A fresh store with the golden run's agent accounts; returns it and folder -> id.
fn fresh(dir: &TempDir, tables: &Value) -> (Store, HashMap<String, i64>) {
    let store = open_for_write(&dir.store()).unwrap();
    let c = Connection::open(dir.store()).unwrap();
    let mut ids = HashMap::new();
    for a in rows(tables, "agent_accounts") {
        c.execute(
            "INSERT INTO agent_accounts (provider, folder, label, status, config_dir_env) VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![
                a["provider"].as_str(),
                a["folder"].as_str(),
                a["label"].as_str(),
                a["status"].as_str(),
                a["config_dir_env"].as_str()
            ],
        )
        .unwrap();
        ids.insert(
            a["folder"].as_str().unwrap().to_owned(),
            c.last_insert_rowid(),
        );
    }
    (store, ids)
}

fn text(v: &Value) -> Option<String> {
    v.as_str().map(str::to_owned)
}

fn flag(v: &Value) -> Option<bool> {
    v.as_i64().map(|i| {
        assert!(i == 0 || i == 1, "not a flag: {i}");
        i == 1
    })
}

fn event(r: &Value, signals: Vec<Signal>) -> Event {
    Event {
        abstained: flag(&r["abstained"]).unwrap(),
        record_type: text(&r["record_type"]),
        version: text(&r["version"]),
        ts_utc: text(&r["ts_utc"]),
        session_id: text(&r["session_id"]),
        project_path: text(&r["project_path"]),
        git_branch: text(&r["git_branch"]),
        is_sidechain: flag(&r["is_sidechain"]),
        is_api_error: flag(&r["is_api_error"]),
        entrypoint: text(&r["entrypoint"]),
        tool_error_count: r["tool_error_count"].as_i64(),
        model: text(&r["model"]),
        message_id: text(&r["message_id"]),
        tool_names: text(&r["tool_names"]),
        input_tokens: r["input_tokens"].as_i64(),
        output_tokens: r["output_tokens"].as_i64(),
        cache_creation_input_tokens: r["cache_creation_input_tokens"].as_i64(),
        cache_read_input_tokens: r["cache_read_input_tokens"].as_i64(),
        signals,
    }
}

/// Feed one golden run's files back, in sorted path order (as Python's `sorted(rglob)`), each under inode
/// `base + index`; record number -> golden inode token in `inodes`. Returns the summed `added`.
fn replay(
    store: &mut Store,
    tables: &Value,
    accounts: &HashMap<String, i64>,
    base: i64,
    inodes: &mut HashMap<i64, String>,
) -> i64 {
    // (source, path) -> (inode token, end offset), from `source_files`.
    let mut files: BTreeMap<(String, String), (String, i64)> = BTreeMap::new();
    for f in rows(tables, "source_files") {
        files.insert(
            (text(&f["source"]).unwrap(), text(&f["path"]).unwrap()),
            (
                text(&f["inode"]).unwrap(),
                f["byte_offset"].as_i64().unwrap(),
            ),
        );
    }
    let mut signals: HashMap<(String, i64), Vec<Signal>> = HashMap::new();
    for s in rows(tables, "tool_signals") {
        signals
            .entry((
                text(&s["source_path"]).unwrap(),
                s["byte_offset"].as_i64().unwrap(),
            ))
            .or_default()
            .push(Signal {
                kind: text(&s["kind"]).unwrap(),
                value: text(&s["value"]).unwrap(),
            });
    }
    let mut added = 0;
    for (i, ((source, path), (token, end))) in files.iter().enumerate() {
        let mut evs: Vec<&Value> = rows(tables, "events")
            .iter()
            .filter(|e| e["source"] == *source && e["source_path"] == *path && e["inode"] == *token)
            .collect();
        evs.sort_by_key(|e| e["byte_offset"].as_i64().unwrap());
        let account = evs
            .first()
            .and_then(|e| e["agent_account_id"].as_str())
            .map(|f| accounts[f]);
        let mut lines: Vec<Line> = evs
            .iter()
            .map(|e| {
                assert_eq!(
                    e["agent_account_id"].as_str().map(|f| accounts[f]),
                    account,
                    "one account per file"
                );
                let at = e["byte_offset"].as_i64().unwrap();
                Line {
                    line_offset: at,
                    next_offset: at,
                    event: Some(event(
                        e,
                        signals.remove(&(path.clone(), at)).unwrap_or_default(),
                    )),
                }
            })
            .collect();
        lines.push(Line {
            line_offset: *end,
            next_offset: *end,
            event: None,
        });
        let inode = base + i as i64;
        inodes.insert(inode, token.clone());
        added += store
            .save_file(source, path, inode, 0, &lines, account)
            .unwrap();
    }
    assert!(
        signals.is_empty(),
        "signals of no stored event: {signals:?}"
    );
    added
}

fn sorted(mut v: Vec<Value>) -> Vec<Value> {
    v.sort_by_key(|x| x.to_string());
    v
}

/// The store's `events`, `tool_signals`, `source_files` in the goldens' form (ids and machine_id dropped; inode and
/// account replaced by their tokens), plus the `machine_id_check`.
fn dump(path: &Path, inodes: &HashMap<i64, String>) -> Value {
    let c = Connection::open(path).unwrap();
    let mid: String = c
        .query_row("SELECT value FROM meta WHERE key = 'machine_id'", [], |r| {
            r.get(0)
        })
        .unwrap();
    let folders: HashMap<i64, String> = c
        .prepare("SELECT id, folder FROM agent_accounts")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let mut out = Map::new();
    let mut check = Map::new();
    for (table, columns) in [
        (
            "events",
            "source, source_path, inode, byte_offset, abstained, record_type, version, ts_utc, session_id, project_path, \
             git_branch, is_sidechain, is_api_error, model, message_id, tool_names, input_tokens, output_tokens, \
             cache_creation_input_tokens, cache_read_input_tokens, agent_account_id, entrypoint, tool_error_count",
        ),
        (
            "tool_signals",
            "source_path, inode, byte_offset, kind, value, ts_utc",
        ),
        ("source_files", "source, path, inode, byte_offset"),
    ] {
        let names: Vec<&str> = columns.split(',').map(str::trim).collect();
        let mut stmt = c
            .prepare(&format!("SELECT {columns}, machine_id FROM {table}"))
            .unwrap();
        let mut all_mid = true;
        let got: Vec<Value> = stmt
            .query_map([], |r| {
                let mut row = Map::new();
                for (i, name) in names.iter().enumerate() {
                    let v = match r.get::<_, rusqlite::types::Value>(i)? {
                        rusqlite::types::Value::Null => Value::Null,
                        rusqlite::types::Value::Integer(n) if *name == "inode" => json!(inodes[&n]),
                        rusqlite::types::Value::Integer(n) if *name == "agent_account_id" => {
                            json!(folders[&n])
                        }
                        rusqlite::types::Value::Integer(n) => json!(n),
                        rusqlite::types::Value::Text(t) => json!(t),
                        other => panic!("unexpected {other:?}"),
                    };
                    row.insert((*name).to_owned(), v);
                }
                Ok((Value::Object(row), r.get::<_, String>(names.len())?))
            })
            .unwrap()
            .map(|r| {
                let (row, m) = r.unwrap();
                all_mid &= m == mid;
                row
            })
            .collect();
        out.insert(table.to_owned(), Value::Array(sorted(got)));
        check.insert(table.to_owned(), json!(all_mid));
    }
    out.insert("machine_id_check".into(), Value::Object(check));
    Value::Object(out)
}

fn expected(tables: &Value) -> Value {
    let mut out = Map::new();
    for table in ["events", "tool_signals", "source_files"] {
        out.insert(
            table.to_owned(),
            Value::Array(sorted(rows(tables, table).clone())),
        );
    }
    out.insert(
        "machine_id_check".into(),
        tables["machine_id_check"].clone(),
    );
    Value::Object(out)
}

fn new_events(run: &Value) -> i64 {
    run["stdout_json"]["new_events"].as_i64().unwrap()
}

fn assert_tables(dir: &TempDir, inodes: &HashMap<i64, String>, run: &Value, what: &str) {
    let got = dump(&dir.store(), inodes);
    let want = expected(&run["tables_after"]);
    for key in ["events", "tool_signals", "source_files", "machine_id_check"] {
        assert_eq!(got[key], want[key], "{what}: {key}");
    }
}

/// S01/S02: a cold ingest adds every row (Python's count), the same files again add 0 and change nothing.
#[test]
fn s02_the_same_files_twice() {
    let g = golden("S02_rerun_unchanged.json");
    let (first, second) = (&runs(&g)[0], &runs(&g)[1]);
    let dir = TempDir::new();
    let (mut store, accounts) = fresh(&dir, &first["tables_after"]);
    let mut inodes = HashMap::new();
    let added = replay(
        &mut store,
        &first["tables_after"],
        &accounts,
        1000,
        &mut inodes,
    );
    assert_eq!((added, new_events(first)), (39, 39));
    assert_tables(&dir, &inodes, first, "S02 first run");
    let again = replay(
        &mut store,
        &first["tables_after"],
        &accounts,
        1000,
        &mut inodes,
    );
    assert_eq!(again, new_events(second));
    assert_eq!(again, 0);
    assert_tables(&dir, &inodes, second, "S02 second run");
}

/// S04: offsets reset to 0 with entrypoint / tool_error_count cleared (the generator's SQL); the re-read back-fills
/// them through the upsert and adds no row.
#[test]
fn s04_a_reset_store_is_back_filled_without_new_rows() {
    let g = golden("S04_offsets_reset_backfill.json");
    let [first, reset, second] = &runs(&g)[..] else {
        panic!("three runs")
    };
    let dir = TempDir::new();
    let (mut store, accounts) = fresh(&dir, &first["tables_after"]);
    let mut inodes = HashMap::new();
    assert_eq!(
        replay(
            &mut store,
            &first["tables_after"],
            &accounts,
            1000,
            &mut inodes
        ),
        new_events(first)
    );
    assert_tables(&dir, &inodes, first, "S04 first run");
    Connection::open(dir.store())
        .unwrap()
        .execute_batch(
            "UPDATE events SET entrypoint = NULL, tool_error_count = NULL; UPDATE source_files SET byte_offset = 0;",
        )
        .unwrap();
    assert_tables(&dir, &inodes, reset, "S04 after the reset");
    let added = replay(
        &mut store,
        &first["tables_after"],
        &accounts,
        1000,
        &mut inodes,
    );
    assert_eq!((added, new_events(second)), (0, 0));
    assert_tables(&dir, &inodes, second, "S04 second run");
}

/// S06: the synthetic set (tool signals, abstained rows, sidechains, NULL fields) stores as Python stored it.
#[test]
fn s06_the_synthetic_set_stores_like_python() {
    let g = golden("S06_cold_synthetic.json");
    let run = &runs(&g)[0];
    let dir = TempDir::new();
    let (mut store, accounts) = fresh(&dir, &run["tables_after"]);
    let mut inodes = HashMap::new();
    let added = replay(
        &mut store,
        &run["tables_after"],
        &accounts,
        1000,
        &mut inodes,
    );
    assert_eq!((added, new_events(run)), (147, 147));
    assert_eq!(rows(&run["tables_after"], "tool_signals").len(), 9);
    assert_tables(&dir, &inodes, run, "S06");
}

/// S13: the same file under a new inode is read again: duplicate rows, Python's count, the offset row takes the new inode.
#[test]
fn s13_a_replaced_file_makes_duplicate_rows() {
    let g = golden("S13_file_replaced_new_inode.json");
    let (first, second) = (&runs(&g)[0], &runs(&g)[1]);
    let dir = TempDir::new();
    let (mut store, accounts) = fresh(&dir, &first["tables_after"]);
    let mut inodes = HashMap::new();
    assert_eq!(
        replay(
            &mut store,
            &first["tables_after"],
            &accounts,
            1000,
            &mut inodes
        ),
        new_events(first)
    );
    assert_tables(&dir, &inodes, first, "S13 first run");
    for token in inodes.values_mut() {
        *token = "<INODE_OTHER>".into();
    }
    let added = replay(
        &mut store,
        &first["tables_after"],
        &accounts,
        2000,
        &mut inodes,
    );
    assert_eq!((added, new_events(second)), (2, 2));
    assert_tables(&dir, &inodes, second, "S13 second run");
}

/// S14: a deleted file's offset row is retired; its events and signals stay.
#[test]
fn s14_a_deleted_file_is_retired_and_its_events_stay() {
    let g = golden("S14_file_deleted.json");
    let (first, second) = (&runs(&g)[0], &runs(&g)[1]);
    let dir = TempDir::new();
    let (mut store, accounts) = fresh(&dir, &first["tables_after"]);
    let mut inodes = HashMap::new();
    assert_eq!(
        replay(
            &mut store,
            &first["tables_after"],
            &accounts,
            1000,
            &mut inodes
        ),
        new_events(first)
    );
    assert_tables(&dir, &inodes, first, "S14 first run");
    let kept: Vec<&Value> = rows(&second["tables_after"], "source_files")
        .iter()
        .map(|f| &f["path"])
        .collect();
    let gone: Vec<String> = rows(&first["tables_after"], "source_files")
        .iter()
        .filter(|f| !kept.contains(&&f["path"]))
        .map(|f| text(&f["path"]).unwrap())
        .collect();
    assert_eq!(gone.len(), 1);
    store.retire_files("claude_code", &gone).unwrap();
    // The golden names the deleted file's inode `<INODE_FILE_GONE>` (no file at that path any more).
    let sorted_paths: Vec<String> = {
        let mut p: Vec<String> = rows(&first["tables_after"], "source_files")
            .iter()
            .map(|f| text(&f["path"]).unwrap())
            .collect();
        p.sort();
        p
    };
    let gone_inode = 1000 + sorted_paths.iter().position(|p| *p == gone[0]).unwrap() as i64;
    inodes.insert(gone_inode, "<INODE_FILE_GONE>".into());
    assert_tables(&dir, &inodes, second, "S14 second run");
}

/// S08b: `--retention-days 1` after a cold ingest of the committed fixtures keeps only the NULL-timestamp rows.
#[test]
fn s08b_prune_keeps_only_null_timestamps() {
    let cold = golden("S01_cold_committed.json");
    let pruned = golden("S08b_retention_1.json");
    let source = &runs(&cold)[0]["tables_after"];
    let run = &runs(&pruned)[0];
    let dir = TempDir::new();
    let (mut store, accounts) = fresh(&dir, source);
    let mut inodes = HashMap::new();
    assert_eq!(
        replay(&mut store, source, &accounts, 1000, &mut inodes),
        new_events(run)
    );
    let now = "2026-11-11T00:00:00Z"
        .parse::<jiff::Timestamp>()
        .unwrap()
        .as_microsecond();
    let deleted = store.prune(Some(1), now).unwrap();
    let left = rows(&run["tables_after"], "events").len() as i64;
    assert_eq!((deleted, left), (39 - 13, 13));
    assert_tables(&dir, &inodes, run, "S08b");
}
