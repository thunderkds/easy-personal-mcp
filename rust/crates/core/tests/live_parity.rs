//! T049: the live tail engine (`easy_personal_core::live`) against Python's `core/live.py`.
//!
//! 1. `golden-live/items.json` (AC 1) and `labels.json` (AC 2): Python's `_item`, `_safe`, `_bash_label`, `_call_label`.
//! 2. `golden-live/scripts.json` (AC 4-7): the same file operations and frozen clock as `gen_live.py`, replayed over a
//!    temp `projects/` tree and a temp store; the new events and the snapshot must equal Python's after every step.
//! 3. Rust-only: hostile files under a timeout (AC 8), publish-only and read-only (AC 9), canaries (AC 10).
//!
//! Every folder is a temp one under `CARGO_TARGET_TMPDIR`, removed even when an assertion fails; no real home is read.

use easy_personal_core::ingest::json;
use easy_personal_core::live::{LiveTail, Root, bash_label, call_label, line_item, safe};
use easy_personal_core::prices::PriceTable;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SECOND: i64 = 1_000_000;

fn golden(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity/golden-live")
        .join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

/// A temp folder removed on drop (also after a failed assertion).
struct Temp(PathBuf);

impl Temp {
    fn new(tag: &str) -> Temp {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "live-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("projects")).unwrap();
        Temp(dir)
    }
    fn projects(&self) -> PathBuf {
        self.0.join("projects")
    }
    fn store(&self) -> PathBuf {
        self.0.join("store.sqlite")
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The tables the tail reads, nothing else (a store another process keeps).
fn make_store(path: &Path) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT);
         INSERT INTO meta VALUES ('machine_id', 'machine-1');
         CREATE TABLE source_files (machine_id TEXT, source TEXT, path TEXT, inode INTEGER, byte_offset INTEGER,
                                    PRIMARY KEY (machine_id, source, path));
         CREATE TABLE agent_accounts (id INTEGER PRIMARY KEY, provider TEXT, status TEXT, folder TEXT,
                                      config_dir_env TEXT, label TEXT);",
    )
    .unwrap();
}

fn tail() -> LiveTail {
    LiveTail::new(PriceTable::shipped().unwrap())
}

fn apply(op: &Value, root: &Path, store: &Path, t0: i64) {
    let path = op["path"]
        .as_str()
        .map_or(root.to_path_buf(), |p| root.join(p));
    let data: Vec<u8> = match op.get("hex") {
        Some(h) => {
            let h = h.as_str().unwrap();
            (0..h.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
                .collect()
        }
        None => op["text"].as_str().unwrap_or("").as_bytes().to_vec(),
    };
    match op["op"].as_str().unwrap() {
        kind @ ("write" | "append") => {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut f = File::options()
                .create(true)
                .write(true)
                .append(kind == "append")
                .truncate(kind == "write")
                .open(&path)
                .unwrap();
            f.write_all(&data).unwrap();
        }
        "replace" => {
            let tmp = path.with_extension("jsonl.tmp");
            fs::write(&tmp, &data).unwrap();
            fs::rename(&tmp, &path).unwrap();
        }
        "delete" => fs::remove_file(&path).unwrap(),
        "mtime" => {
            let secs = t0 as f64 + op["secs"].as_f64().unwrap();
            let at = UNIX_EPOCH + Duration::from_secs_f64(secs);
            File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(at)
                .unwrap();
        }
        "cursor" => {
            let conn = Connection::open(store).unwrap();
            conn.execute(
                "INSERT OR REPLACE INTO source_files VALUES ('machine-1', 'claude_code', ?, ?, ?)",
                rusqlite::params![
                    path.to_str().unwrap(),
                    fs::metadata(&path).unwrap().ino() as i64,
                    op["offset"].as_i64().unwrap()
                ],
            )
            .unwrap();
        }
        "symlink" => {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(root.join(op["target"].as_str().unwrap()), &path).unwrap();
        }
        "account" => {
            let conn = Connection::open(store).unwrap();
            conn.execute(
                "INSERT INTO agent_accounts (id, provider, status, folder, config_dir_env, label) \
                 VALUES (?, 'claude', ?, ?, ?, ?)",
                rusqlite::params![
                    op["id"].as_i64().unwrap(),
                    op["status"].as_str().unwrap(),
                    root.parent()
                        .unwrap()
                        .join(op["folder"].as_str().unwrap())
                        .to_str()
                        .unwrap(),
                    op["env"].as_str(),
                    op["label"].as_str()
                ],
            )
            .unwrap();
        }
        other => panic!("unknown op {other}"),
    }
}

/// Replays every script; checks each step against Python when `check`, and returns every output text.
fn run_scripts(file: &str, check: bool) -> (usize, Vec<String>) {
    let g = golden(file);
    let t0 = g["t0"].as_i64().unwrap();
    let mut outputs = Vec::new();
    let mut steps = 0;
    for script in g["scripts"].as_array().unwrap() {
        let name = script["name"].as_str().unwrap();
        let tmp = Temp::new(name);
        make_store(&tmp.store());
        let mut live = tail();
        let mut last = 0;
        for (i, step) in script["steps"].as_array().unwrap().iter().enumerate() {
            for op in step["ops"].as_array().unwrap() {
                apply(op, &tmp.projects(), &tmp.store(), t0);
            }
            let now = (t0 as f64 * 1e6 + step["now"].as_f64().unwrap() * 1e6).round() as i64;
            live.poll_store(&tmp.store(), Some(&tmp.projects()), now);
            let events = live.events_since(last);
            let snapshot = live.snapshot(now);
            if check {
                match step.get("count") {
                    Some(n) => assert_eq!(
                        events.len() as u64,
                        n.as_u64().unwrap(),
                        "{name} step {i}: count"
                    ),
                    None => {
                        let want = step["events"].as_array().unwrap();
                        assert_eq!(events.len(), want.len(), "{name} step {i}: {events:#?}");
                        for (k, (got, want)) in events.iter().zip(want).enumerate() {
                            assert_eq!(got, want, "{name} step {i} event {k}");
                        }
                    }
                }
                assert_eq!(snapshot, step["snapshot"], "{name} step {i}: snapshot");
                if let Some(probes) = step.get("probes") {
                    for p in probes.as_array().unwrap() {
                        let k = p[0].as_i64().unwrap();
                        let got = live.events_since(k);
                        let mine = json!([
                            k,
                            got.len(),
                            got.first().map(|e| e["id"].clone()),
                            got.last().map(|e| e["id"].clone()),
                            live.rolled_past(k)
                        ]);
                        assert_eq!(&mine, p, "{name} step {i}: probe {k}");
                    }
                }
            }
            last = live.last_id();
            outputs.push(serde_json::to_string(&events).unwrap());
            outputs.push(snapshot.to_string());
            outputs.push(format!("{live:?}"));
            steps += 1;
        }
    }
    (steps, outputs)
}

#[test]
fn items_equal_python() {
    let g = golden("items.json");
    assert_eq!(g["source"], "claude_code");
    let rows = g["items"].as_array().unwrap();
    for row in rows {
        let line = row["line"].as_str().unwrap();
        let got = line_item(line.as_bytes(), &mut |c: &str| c.to_owned()).map(|i| i.to_json());
        assert_eq!(got.unwrap_or(Value::Null), row["item"], "line {line}");
    }
    assert_eq!(rows.len(), 28, "items");
}

#[test]
fn labels_equal_python() {
    let rows = golden("labels.json");
    let rows = rows.as_array().unwrap();
    for row in rows {
        let input = json::loads(row["input"].as_str().unwrap().as_bytes()).unwrap();
        let got = match row["fn"].as_str().unwrap() {
            "safe" => json!(safe(&input)),
            "bash" => json!(bash_label(&input)),
            _ => call_label(row["name"].as_str().unwrap(), &input)
                .map_or(Value::Null, |l| l.to_json()),
        };
        assert_eq!(got, row["out"], "{row}");
    }
    assert_eq!(rows.len(), 100, "label inputs");
}

#[test]
fn scripts_equal_python_after_every_step() {
    let (steps, _) = run_scripts("scripts.json", true);
    assert_eq!(steps, 72, "steps of scripts.json");
}

#[test]
fn no_canary_in_any_output() {
    let canary = golden("scripts.json")["canary"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, mut outputs) = run_scripts("scripts.json", false);
    outputs.extend(run_scripts("scripts-2.json", false).1);
    for row in golden("items.json")["items"].as_array().unwrap() {
        let line = row["line"].as_str().unwrap();
        let item = line_item(line.as_bytes(), &mut |c: &str| c.to_owned());
        outputs.push(format!("{item:?}"));
        outputs.extend(item.map(|i| i.to_json().to_string()));
    }
    for needle in canary
        .split(' ')
        .chain(["CANARY", "sk-ABCDEF", "/home/canary"])
    {
        for out in &outputs {
            assert!(!out.contains(needle), "{needle} leaked");
        }
    }
}

/// Runs `f` on a thread; fails when it takes longer than 60 s (a blocking open or read).
fn within_timeout(f: impl FnOnce() + Send + 'static) {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        f();
        tx.send(()).unwrap();
    });
    rx.recv_timeout(Duration::from_secs(60))
        .expect("the step blocked or panicked");
}

fn line(sid: &str, block: &str) -> String {
    format!(
        "{{\"type\":\"user\",\"sessionId\":\"{sid}\",\"timestamp\":\"2026-01-20T00:00:00Z\",\"message\":{{\"content\":[{{\"type\":\"tool_result\",\"tool_use_id\":\"{block}\"}}]}}}}\n"
    )
}

const NOW: i64 = 1_768_867_200 * SECOND + 10 * SECOND;

#[test]
fn hostile_files_never_block_or_leave_the_root() {
    within_timeout(|| {
        let tmp = Temp::new("hostile");
        let p = tmp.projects().join("p");
        fs::create_dir_all(&p).unwrap();
        let outside = tmp.0.join("outside.jsonl");
        fs::write(&outside, line("out", "o1").repeat(3)).unwrap();
        let mut live = tail();
        make_store(&tmp.store());
        live.poll_store(&tmp.store(), Some(&tmp.projects()), NOW);
        std::os::unix::fs::symlink(&outside, p.join("link.jsonl")).unwrap();
        let fifo = p.join("fifo.jsonl");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        fs::create_dir_all(p.join("dir.jsonl")).unwrap();
        fs::write(p.join("dir.jsonl").join("inner.jsonl"), line("in", "i1")).unwrap();
        fs::write(p.join("big.jsonl"), vec![b'x'; 20 * 1024 * 1024]).unwrap(); // 20 MB, no newline
        let mut odd = b"\xff\xfe{\n\0{}\n".to_vec();
        odd.extend_from_slice(line("odd", "d1").as_bytes());
        fs::write(p.join("odd.jsonl"), odd).unwrap();
        live.poll_store(&tmp.store(), Some(&tmp.projects()), NOW);
        let mut sessions: Vec<String> = live
            .events_since(0)
            .iter()
            .map(|e| e["session_id"].as_str().unwrap().to_owned())
            .collect();
        sessions.sort();
        assert_eq!(
            sessions,
            ["in", "odd"],
            "a symlink was followed or a line lost"
        );
        // A line over the bound is skipped once its newline arrives; the next line is read.
        let mut f = File::options()
            .append(true)
            .open(p.join("big.jsonl"))
            .unwrap();
        f.write_all(&vec![b'y'; 13 * 1024 * 1024]).unwrap();
        f.write_all(b"\n").unwrap();
        f.write_all(line("after", "a1").as_bytes()).unwrap();
        drop(f);
        let before = live.last_id();
        live.poll_store(&tmp.store(), Some(&tmp.projects()), NOW);
        let new = live.events_since(before);
        assert_eq!(new.len(), 1, "{new:?}");
        assert_eq!(new[0]["session_id"], "after");
        // A transcript replaced by a symlink between two steps is not read.
        fs::remove_file(p.join("odd.jsonl")).unwrap();
        let bigger = tmp.0.join("bigger.jsonl");
        fs::write(&bigger, line("out", "o2").repeat(50)).unwrap();
        std::os::unix::fs::symlink(&bigger, p.join("odd.jsonl")).unwrap();
        let before = live.last_id();
        live.poll_store(&tmp.store(), Some(&tmp.projects()), NOW + SECOND);
        assert!(live.events_since(before).is_empty());
    });
}

fn stamp(path: &Path) -> (Vec<u8>, SystemTime) {
    (
        fs::read(path).unwrap(),
        fs::metadata(path).unwrap().modified().unwrap(),
    )
}

#[test]
fn publish_only_and_read_only() {
    let tmp = Temp::new("readonly");
    make_store(&tmp.store());
    let p = tmp.projects().join("p");
    fs::create_dir_all(&p).unwrap();
    let file = p.join("s.jsonl");
    fs::write(&file, line("s", "b1").repeat(4)).unwrap();
    {
        let conn = Connection::open(tmp.store()).unwrap();
        conn.execute(
            "INSERT INTO source_files VALUES ('machine-1', 'claude_code', ?, ?, 0)",
            rusqlite::params![
                file.to_str().unwrap(),
                fs::metadata(&file).unwrap().ino() as i64
            ],
        )
        .unwrap();
    }
    let (store_before, file_before) = (stamp(&tmp.store()), stamp(&file));
    let dir_before = fs::metadata(tmp.projects()).unwrap().modified().unwrap();
    let mut live = tail();
    for k in 0..3 {
        live.poll_store(&tmp.store(), Some(&tmp.projects()), NOW + k * SECOND);
    }
    fs::write(p.join("t.jsonl"), line("t", "b2")).unwrap();
    let file_t = stamp(&p.join("t.jsonl"));
    live.poll_store(&tmp.store(), Some(&tmp.projects()), NOW + 5 * SECOND);
    assert_eq!(live.events_since(0).len(), 5);
    assert_eq!(stamp(&tmp.store()), store_before, "the store changed");
    assert_eq!(stamp(&file), file_before);
    assert_eq!(stamp(&p.join("t.jsonl")), file_t);
    assert_eq!(
        fs::metadata(tmp.projects()).unwrap().modified().unwrap(),
        dir_before
    );
    let names: Vec<String> = fs::read_dir(&tmp.0)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        !names
            .iter()
            .any(|n| n.contains("lock") || n.contains("-wal") || n.contains("-journal")),
        "{names:?}"
    );
}

/// T049 fix round: the scripts added after review (`scripts-2.json`): accounts with labels (a NULL label included).
#[test]
fn scripts_2_equal_python_after_every_step() {
    let (steps, _) = run_scripts("scripts-2.json", true);
    assert_eq!(steps, SCRIPTS_2_STEPS, "steps of scripts-2.json");
}

const SCRIPTS_2_STEPS: usize = 3;

/// A store with one confirmed account (`folder/projects` is its root, not an override) and its transcript.
fn account_store(tmp: &Temp, lines: &str, cursor: Option<i64>) -> PathBuf {
    make_store(&tmp.store());
    let file = tmp.0.join("acct/projects/p/s.jsonl");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, lines).unwrap();
    let at = UNIX_EPOCH + Duration::from_micros(NOW as u64);
    File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_modified(at)
        .unwrap();
    let conn = Connection::open(tmp.store()).unwrap();
    conn.execute(
        "INSERT INTO agent_accounts VALUES (1, 'claude', 'confirmed', ?, '/e', 'Work')",
        [tmp.0.join("acct").to_str().unwrap()],
    )
    .unwrap();
    if let Some(offset) = cursor {
        conn.execute(
            "INSERT INTO source_files VALUES ('machine-1', 'claude_code', ?, ?, ?)",
            rusqlite::params![
                file.to_str().unwrap(),
                fs::metadata(&file).unwrap().ino() as i64,
                offset
            ],
        )
        .unwrap();
    }
    file
}

#[test]
fn a_failed_store_step_keeps_offsets_and_publishes_nothing_again() {
    let tmp = Temp::new("failed-step");
    account_store(&tmp, &line("s", "b1").repeat(3), None);
    let (store, gone) = (tmp.store(), tmp.0.join("store.gone"));
    let mut live = tail();
    live.poll_store(&store, None, NOW); // first scan: no cursor, so the file is read from 0
    let first = live.last_id();
    assert_eq!(first, 3);
    fs::rename(&store, &gone).unwrap(); // a missing store: the step fails
    live.poll_store(&store, None, NOW + SECOND);
    fs::rename(&gone, &store).unwrap();
    live.poll_store(&store, None, NOW + 2 * SECOND);
    assert_eq!(
        live.events_since(first),
        Vec::<Value>::new(),
        "history published again"
    );
}

#[test]
fn a_failed_first_step_keeps_the_warm_start() {
    let tmp = Temp::new("failed-first");
    let below = line("s", "b1").repeat(2);
    let all = below.clone() + &line("s", "b2");
    account_store(&tmp, &all, Some(below.len() as i64));
    let (store, gone) = (tmp.store(), tmp.0.join("store.gone"));
    let mut live = tail();
    fs::rename(&store, &gone).unwrap();
    live.poll_store(&store, None, NOW);
    fs::rename(&gone, &store).unwrap();
    live.poll_store(&store, None, NOW + SECOND);
    let events = live.events_since(0);
    assert_eq!(
        events.len(),
        1,
        "only the line from the cursor on is live: {events:?}"
    );
    assert_eq!(
        live.state_version(),
        1,
        "the warm replay ran on the first good step"
    );
    assert_eq!(events[0]["account"], "Work");
}

fn usage_line(mid: &str, input: i64, output: i64) -> String {
    format!(
        "{{\"type\":\"assistant\",\"sessionId\":\"s\",\"timestamp\":\"2026-01-20T00:00:00Z\",\"message\":{{\"id\":\"{mid}\",\"model\":\"x\",\"usage\":{{\"input_tokens\":{input},\"output_tokens\":{output},\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":0}},\"content\":[]}}}}\n"
    )
}

/// Rust-only (decided: token sums and growth saturate at the `i64` bounds; Python's ints are exact): hostile token
/// values never panic or wrap, and a client's `Last-Event-ID` of `i64::MAX` is answered.
#[test]
fn huge_token_values_saturate_and_never_panic() {
    let tmp = Temp::new("overflow");
    make_store(&tmp.store());
    let file = tmp.projects().join("p/s.jsonl");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    let mut live = tail();
    live.poll_store(&tmp.store(), Some(&tmp.projects()), NOW);
    let text = usage_line("m1", i64::MAX, i64::MIN)
        + &usage_line("m2", i64::MAX, 0)
        + &usage_line("m1", 0, 5);
    fs::write(&file, text).unwrap();
    live.poll_store(&tmp.store(), Some(&tmp.projects()), NOW);
    let snapshot = live.snapshot(NOW);
    assert_eq!(snapshot["sessions"][0]["tokens_in"], i64::MAX);
    let events = live.events_since(0);
    assert_eq!(events.len(), 3, "{events:?}");
    assert_eq!(events[2]["tokens_out"], i64::MAX, "5 - i64::MIN saturates");
    assert!(!live.rolled_past(i64::MAX));
    assert!(live.events_since(i64::MAX).is_empty());
}

/// Pairing 100,000 calls with their results is linear: one line with 100,000 `tool_use` blocks, then their results.
#[test]
fn pairing_100k_calls_is_fast() {
    within_timeout(|| {
        let tmp = Temp::new("pairing-100k");
        let file = tmp.projects().join("p/s.jsonl");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        let roots = [Root {
            account: None,
            projects: tmp.projects(),
        }];
        let mut live = tail();
        live.poll_once(&roots, NOW, None);
        let n = 100_000;
        let uses: Vec<String> = (0..n)
            .map(|i| format!("{{\"type\":\"tool_use\",\"id\":\"b{i}\",\"name\":\"Read\"}}"))
            .collect();
        let results: Vec<String> = (0..n)
            .map(|i| format!("{{\"type\":\"tool_result\",\"tool_use_id\":\"b{i}\"}}"))
            .collect();
        let head = "\"sessionId\":\"s\",\"timestamp\":\"2026-01-20T00:00:00Z\"";
        let text = format!(
            "{{\"type\":\"assistant\",{head},\"message\":{{\"id\":\"m1\",\"content\":[{}]}}}}\n\
             {{\"type\":\"user\",{head},\"message\":{{\"content\":[{}]}}}}\n",
            uses.join(","),
            results.join(",")
        );
        fs::write(&file, text).unwrap();
        let started = std::time::Instant::now();
        live.poll_once(&roots, NOW, None);
        let took = started.elapsed();
        let events = live.events_since(0);
        assert_eq!(events.len(), 2 * BUFFER_HELD.min(n));
        assert_eq!(
            events.last().unwrap()["tool"],
            "Read",
            "the last result is paired"
        );
        assert_eq!(live.snapshot(NOW)["sessions"][0]["attention"], json!([]));
        eprintln!("pairing 100k calls: {took:?}");
        assert!(took < Duration::from_secs(5), "{took:?}");
    });
}

/// The buffer keeps 2,000 events: 1,000 here are the last results.
const BUFFER_HELD: usize = 1000;
