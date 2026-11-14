//! T031: coexistence of the two writers of one schema-v11 store. Real processes only: the Rust binary
//! (`target/debug/easy-personal-rs`) and, in the `#[ignore]`d half, the real Python app from the parity venv
//! (`rust/parity/.venv`, `PYTHONPATH=src`). Every child gets an environment built from nothing, with
//! `EASY_PERSONAL_HOME`, `EASY_PERSONAL_AGENT_ROOT` and `HOME` inside its own temp folder (a guard refuses anything
//! else), runs in its own process group, is polled to a deadline and is reaped; its group is then checked empty by PID.
//!
//! Shown: two ingests started at the same instant never write at once (the second finds nothing new or exits 75), the
//! store stays intact with no duplicate key and equal to one clean run (the T026 goldens); a SIGKILL at fixed and
//! seeded points leaves whole files only and the next run resumes to a clean run's tables; the lock is free at once
//! after a kill and a stopped holder makes the next ingest exit 75; `serve`, `mcp` and `store-info` keep answering
//! while an ingest runs; copies, version guards and odd data folders behave the same on both sides.
//!
//! Rust half: `cargo test -p easy-personal-rs --test ingest_coexist -- --nocapture`
//! Python half (needs the parity venv): `cargo test -p easy-personal-rs --test ingest_coexist -- --ignored --nocapture`

#[path = "../../core/tests/ingest_folders_common/mod.rs"]
mod common;
mod ingest_coexist_common;

use common::{Case, committed, fixtures, golden, repos};
use easy_personal_core::writer::open_for_write;
use ingest_coexist_common::*;
use rusqlite::Connection;
use serde_json::Value;
use std::fs;
use std::io::BufRead;
use std::os::unix::fs::PermissionsExt;
use std::thread::sleep;
use std::time::{Duration, Instant};

/// The large set: 20 files of 700 lines (about 6 MB), seed fixed.
const FILES: usize = 20;
const LINES: usize = 700;
const SET_SEED: u64 = 31;
/// The seeds of the random kill points (pasted in the report).
const KILL_SEEDS: [u64; 5] = [7, 19, 31, 42, 77];
const REPEAT: usize = 20;
/// The red control's own set: many small events so that one Rust ingest runs for seconds (see the test).
const BYPASS_FILES: usize = 300;
const BYPASS_LINES: usize = 200;
const BYPASS_PAD: usize = 40;
const BYPASS_ATTEMPTS: usize = 30;
const LADDER: [u64; 4] = [0, 5, 20, 50];
/// What a run prints when SQLite's own busy wait ends (the lock bypassed, a loaded machine): exit 1.
const SQLITE_BUSY_LINE: &str =
    "easy-personal-rs: the store is busy (another process is writing); try again in a moment\n";
/// How long the readers test may take to see 50 polls during an ingest. A wall-clock bound, not a count of rounds: a
/// slow or loaded machine runs fewer, longer rounds and would trip a round cap although nothing is wrong. Equal to
/// the per-process [`LIMIT`], so one stuck round still fails by [`LIMIT`] and the whole test never runs unbounded.
const READERS_DEADLINE: Duration = Duration::from_secs(120);

type Maker = fn(&Case) -> Spec;

fn rust_ingest(case: &Case) -> Spec {
    Spec::rust(case, &["ingest"])
}

fn python_ingest(case: &Case) -> Spec {
    Spec::python(case, &[])
}

fn s01_case() -> Case {
    let case = Case::new();
    case.copy(&committed(), &case.projects);
    case
}

fn s06_case() -> Case {
    let layout = repos();
    let case = Case::new();
    case.apply(&layout["work"]);
    case.copy(&fixtures().join("synthetic"), &case.projects);
    case.copy(&fixtures().join("outside"), &case.agents.join("outside"));
    case.apply(&layout["synthetic"]);
    case
}

fn large_case() -> Case {
    let case = Case::new();
    write_large_set(&case.projects, "big", FILES, LINES, SET_SEED);
    case
}

fn precreate_store(case: &Case) {
    fs::create_dir_all(&case.home).unwrap();
    drop(open_for_write(&case.store_path()).unwrap());
}

fn listing(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

fn is_root() -> bool {
    fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .any(|l| l.starts_with("Uid:") && l.split_whitespace().nth(1) == Some("0"))
}

// -- criterion 1: the guard

#[test]
#[should_panic(expected = "refusing to run: EASY_PERSONAL_HOME is not set")]
fn the_guard_refuses_a_child_without_easy_personal_home() {
    let case = Case::new();
    let _ = rust_ingest(&case).without("EASY_PERSONAL_HOME").spawn();
}

#[test]
#[should_panic(expected = "refusing to run: EASY_PERSONAL_AGENT_ROOT is outside the temp folder")]
fn the_guard_refuses_an_agent_root_outside_the_temp_folder() {
    let case = Case::new();
    let _ = rust_ingest(&case)
        .env("EASY_PERSONAL_AGENT_ROOT", "/")
        .spawn();
}

// -- criterion 2: simultaneous writers

/// `REPEAT` races of `first` and `second` on one fresh store, started at one barrier, one of them delayed by the
/// 0/5/20/50 ms ladder (alternately). `timeout`: `EASY_PERSONAL_LOCK_TIMEOUT` for both (None: the 300 s default).
fn races(
    label: &str,
    make: fn() -> Case,
    scenario: &str,
    first: Maker,
    second: Maker,
    timeout: Option<&str>,
) {
    assert_cwds_are_gone();
    let g = golden(scenario);
    let run0 = &g["runs"][0];
    let want = golden_tables(run0);
    let total = run0["stdout_json"]["new_events"].as_i64().unwrap();
    let (mut both_ok, mut one_busy, mut waited) = (0, 0, 0);
    for i in 0..REPEAT {
        let case = make();
        let (mut a, mut b) = (first(&case), second(&case));
        if let Some(t) = timeout {
            a = a.env(LOCK_TIMEOUT, t);
            b = b.env(LOCK_TIMEOUT, t);
        }
        let step = LADDER[i % LADDER.len()];
        let delays = if i % 2 == 0 { (0, step) } else { (step, 0) };
        let (oa, ob) = race(&case, &a, &b, delays, i);
        let what = format!("{label} #{i}");
        let mut news = Vec::new();
        for out in [&oa, &ob] {
            match (out.code, timeout) {
                (Some(0), _) => {
                    assert_eq!(out.stderr, "", "{what}: stderr of a clean run");
                    news.push(out.new_events());
                }
                (Some(75), Some(_)) => {
                    assert_eq!(out.stderr, BUSY, "{what}: the busy line");
                    assert_eq!(out.stdout, "", "{what}: stdout of a busy run");
                    one_busy += 1;
                }
                (code, _) => panic!("{what}: exit {code:?}"),
            }
        }
        news.sort();
        // One run wrote everything; the other found nothing new (it waited) or was busy: never two writers at once.
        let expect: Vec<i64> = if news.len() == 2 {
            vec![0, total]
        } else {
            vec![total]
        };
        assert_eq!(news, expect, "{what}: new_events of the two runs");
        if news.len() == 2 {
            both_ok += 1;
            let (winner, loser) = if oa.new_events() == total {
                (&oa, &ob)
            } else {
                (&ob, &oa)
            };
            waited += usize::from(loser.elapsed >= winner.elapsed);
        }
        assert_same(&checked_dump(&case, &what), &want, &what);
    }
    println!(
        "{label}: {REPEAT} of {REPEAT} equal {scenario} tables, integrity ok, no duplicate key; \
         both exit 0: {both_ok} (the second ran no shorter than the first: {waited}); one exit 75: {one_busy}"
    );
}

#[test]
fn rust_and_rust_at_the_same_instant_on_the_committed_fixtures() {
    races(
        "rust||rust S01",
        s01_case,
        "S01_cold_committed.json",
        rust_ingest,
        rust_ingest,
        None,
    );
}

#[test]
fn rust_and_rust_at_the_same_instant_on_the_synthetic_set() {
    races(
        "rust||rust S06",
        s06_case,
        "S06_cold_synthetic.json",
        rust_ingest,
        rust_ingest,
        None,
    );
}

/// T051: the start-up ingest of `serve` and an `ingest` command started at one barrier take the same lock: one writes
/// everything, the other finds nothing new, and the store ends equal to one clean run (the S01 tables). The `serve`
/// is stopped once it printed its dashboard line (its ingest is done by then).
#[test]
fn a_start_up_ingest_and_an_ingest_at_the_same_instant_leave_one_clean_store() {
    assert_cwds_are_gone();
    let g = golden("S01_cold_committed.json");
    let want = golden_tables(&g["runs"][0]);
    let total = g["runs"][0]["stdout_json"]["new_events"].as_i64().unwrap();
    for i in 0..8 {
        let case = s01_case();
        let dir = case.root.join(format!("barrier-startup-{i}"));
        std::fs::create_dir_all(&dir).unwrap();
        let step = LADDER[i % LADDER.len()];
        let delays = if i % 2 == 0 { (0, step) } else { (step, 0) };
        let mut serve =
            Spec::rust(&case, &["serve"]).spawn_at(&dir, Duration::from_millis(delays.0));
        let ingest = rust_ingest(&case).spawn_at(&dir, Duration::from_millis(delays.1));
        let deadline = Instant::now() + Duration::from_secs(30);
        while std::fs::read_dir(&dir).unwrap().count() < 2 {
            assert!(
                Instant::now() < deadline,
                "the two children never reached the barrier"
            );
            sleep(Duration::from_millis(1));
        }
        std::fs::write(dir.join("go"), b"").unwrap();
        let out = ingest.wait(LIMIT);
        let what = format!("startup||ingest #{i}");
        assert_eq!(out.code, Some(0), "{what}: {}", out.stderr);
        assert_eq!(out.stderr, "", "{what}: stderr of the ingest");
        assert!(
            [0, total].contains(&out.new_events()),
            "{what}: new_events {}",
            out.new_events()
        );
        // The server starts whatever happened: its dashboard line comes, and by then its own ingest is over.
        let stdout = serve.child.stdout.take().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let _ = std::io::BufReader::new(stdout).read_line(&mut line);
            let _ = tx.send(line);
        });
        let line = rx
            .recv_timeout(LIMIT)
            .expect("serve printed its first line");
        assert!(
            line.starts_with("dashboard: http://127.0.0.1:"),
            "{what}: {line}"
        );
        serve.kill();
        assert_same(&checked_dump(&case, &what), &want, &what);
    }
}

#[test]
fn rust_and_rust_with_no_lock_wait_one_may_exit_75() {
    races(
        "rust||rust S01 timeout 0",
        s01_case,
        "S01_cold_committed.json",
        rust_ingest,
        rust_ingest,
        Some("0"),
    );
}

/// The negative control (the red run of the Test Plan, without a product change): two data folders whose
/// `store.sqlite` is the same file (a symlink) have two different `ingest.lock` files, so the lock is bypassed. The
/// oracle of the races above must then see two writers at once (both runs add rows) in at least one attempt.
/// SQLite's own locking and the UNIQUE key keep the file intact even so (printed, not the point).
///
/// A timing control, made safe by size and by attempts, not by luck: the Rust ingest of the shared large set takes
/// about 100 ms, so two such runs would overlap only by chance. This one uses its own bigger set (`BYPASS_FILES` x
/// `BYPASS_LINES` small events: one copy for all attempts, a fresh store per attempt) so each ingest runs for 2 to 3
/// seconds. Even then SQLite hands the write lock to whichever run leads, and the leader often takes every file (the
/// other adds 0 rows); the lead flips by chance in roughly one attempt in three, so it stops at the first attempt
/// where both runs added rows, and `BYPASS_ATTEMPTS` is the reserve that makes a miss practically impossible.
#[test]
fn the_race_oracle_sees_two_writers_when_the_lock_is_bypassed() {
    assert_cwds_are_gone();
    let mut case = Case::new();
    let bytes = write_set_padded(
        &case.projects,
        "bypass",
        BYPASS_FILES,
        BYPASS_LINES,
        SET_SEED,
        BYPASS_PAD,
    );
    let mut seen = None;
    let mut took: Vec<Duration> = Vec::new();
    let mut busy_attempts = 0;
    for attempt in 0..BYPASS_ATTEMPTS {
        // A fresh store per attempt (the set stays); the previous attempt's folders are this test's own: remove them.
        case.home = case.root.join(format!("home-{attempt}"));
        if attempt > 0 {
            let _ = fs::remove_dir_all(case.root.join(format!("home-{}", attempt - 1)));
            let _ = fs::remove_dir_all(case.root.join(format!("home-bypass-{}", attempt - 1)));
        }
        precreate_store(&case);
        let other = case.root.join(format!("home-bypass-{attempt}"));
        fs::create_dir_all(&other).unwrap();
        std::os::unix::fs::symlink(case.store_path(), other.join("store.sqlite")).unwrap();
        let a = rust_ingest(&case);
        let b = rust_ingest(&case).env("EASY_PERSONAL_HOME", &other);
        let step = LADDER[attempt % LADDER.len()];
        let delays = if attempt % 2 == 0 {
            (0, step)
        } else {
            (step, 0)
        };
        let (oa, ob) = race(&case, &a, &b, delays, attempt);
        // With the lock bypassed, two writers meet in SQLite itself. When the machine is loaded, SQLite's own busy wait can
        // end first: that run exits 1 with the fixed busy line. This is what the control provokes, not a failure, but the
        // attempt cannot show two writers that both added rows: try the next one. Any other outcome is a real failure.
        let sqlite_busy = |o: &Out| o.code == Some(1) && o.stderr == SQLITE_BUSY_LINE;
        if sqlite_busy(&oa) || sqlite_busy(&ob) {
            let other_ok = [&oa, &ob]
                .iter()
                .all(|o| o.code == Some(0) || sqlite_busy(o));
            assert!(
                other_ok,
                "bypass attempt {attempt}: stderr {:?} / {:?}",
                oa.stderr, ob.stderr
            );
            busy_attempts += 1;
            continue;
        }
        assert_eq!(
            (oa.code, ob.code),
            (Some(0), Some(0)),
            "bypass attempt {attempt}: stderr {:?} / {:?}",
            oa.stderr,
            ob.stderr
        );
        took.push(oa.elapsed.min(ob.elapsed));
        let (na, nb) = (oa.new_events(), ob.new_events());
        if na > 0 && nb > 0 {
            assert_intact(&case.store_path(), "bypass");
            seen = Some((attempt, na, nb, oa.elapsed, ob.elapsed));
            break;
        }
    }
    let (attempt, na, nb, ta, tb) = seen.unwrap_or_else(|| {
        panic!(
            "this is a timing control: the bypassed lock was never seen as two writers in {BYPASS_ATTEMPTS} attempts \
             ({busy_attempts} of them ended in SQLite's own busy refusal) over a {bytes}-byte set; the shortest of the two ingest durations per attempt was {took:?} (the set must \
             keep each ingest well past 1 s for the two write windows to overlap)"
        )
    });
    println!(
        "lock bypassed: two writers seen at attempt {attempt} (new_events {na} and {nb}; ingests took {ta:?} and {tb:?} \
         over a {BYPASS_FILES} files x {BYPASS_LINES} lines set of {bytes} bytes); integrity ok, no duplicate key"
    );
}

// -- criteria 3 and 4: kill and resume, lock hygiene

/// A clean single Rust run of the large set: its tables and how long it took.
fn clean_large() -> (Value, i64, Duration) {
    let case = large_case();
    let out = rust_ingest(&case).run();
    assert_eq!(out.code, Some(0), "clean run");
    let dump = checked_dump(&case, "clean run");
    let events = dump["events"].as_array().unwrap().len() as i64;
    assert_eq!(out.new_events(), events);
    (dump, events, out.elapsed)
}

/// One kill: a fresh large case, `killer` SIGKILLed (with its group) `delay` after its start; the store opens, is
/// intact and holds whole files only; the lock is free at once (`resumer` with a zero lock wait succeeds) and
/// `ingest.lock` is kept (it must exist if the killer reached its lock step, and always after the resumer ran);
/// the final tables equal `clean`.
fn kill_then_resume(
    label: &str,
    killer: Maker,
    resumer: Maker,
    delay: Duration,
    clean: &Value,
    total: i64,
    precreate: bool,
) -> String {
    let case = large_case();
    if precreate {
        precreate_store(&case);
    }
    let p = killer(&case).spawn();
    let at = p.started + delay;
    while Instant::now() < at {
        sleep(Duration::from_micros(200));
    }
    let finished_first = p.kill();
    // What the kill left, before this test's own connection opens (and cleanly closes) the store.
    let left = listing(&case.home);
    let (mut files, mut events) = (0, 0);
    let mut machine_id_written = false;
    if case.store_path().exists() {
        assert_intact(&case.store_path(), label);
        let has_id = Connection::open(case.store_path())
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM meta WHERE key = 'machine_id'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
            == 1;
        machine_id_written = has_id;
        if has_id {
            (files, events) = assert_no_half_file(&case, &case.dump(), clean, label);
        }
    }
    // The product never deletes `ingest.lock`, but only a process that reached its lock step creates it. A Python
    // ingest killed in its import phase dies before it opens the file, and the store made by `precreate_store`
    // (Rust `open_for_write`, which writes `machine_id` but no lock file) does not hold it either: an absent file
    // is then legitimate. A killer that saved anything (or, on a store it made itself, wrote `machine_id`) was past
    // the lock step, so the file must be there.
    let reached_lock = files > 0 || events > 0 || (!precreate && machine_id_written);
    let lock_file = case.home.join("ingest.lock");
    if reached_lock {
        assert!(lock_file.is_file(), "{label}: ingest.lock is kept");
    }
    let phase = if reached_lock || lock_file.is_file() {
        "past the lock step"
    } else {
        "died before the lock step"
    };
    let out = resumer(&case).env(LOCK_TIMEOUT, "0").run();
    assert_eq!(
        out.code,
        Some(0),
        "{label}: the lock was not free at once after the kill"
    );
    // The resumer reached its lock step, so the file exists from now on.
    assert!(
        lock_file.is_file(),
        "{label}: ingest.lock exists after the resumer ran"
    );
    let resumed = checked_dump(&case, label);
    assert_same(&resumed, clean, label);
    assert_eq!(
        out.new_events() + events as i64,
        total,
        "{label}: events lost or doubled"
    );
    format!(
        "{label}: kill at {:.1} ms (exited before: {finished_first}; {phase}), whole files {files} of {FILES}, events {events}, \
         left {left:?}; resume exit 0 with zero lock wait, new_events {}, tables = clean",
        delay.as_secs_f64() * 1000.0,
        out.new_events()
    )
}

/// The fixed points and one point per seed, uniform in [lo, hi) of `span`.
fn kill_points(fixed: &[Duration], span: Duration, lo: f64, hi: f64) -> Vec<(String, Duration)> {
    let mut points: Vec<(String, Duration)> =
        fixed.iter().map(|d| (format!("fixed {d:?}"), *d)).collect();
    for seed in KILL_SEEDS {
        let unit = (Rng(seed).next() % 1_000_000) as f64 / 1_000_000.0;
        points.push((format!("seed {seed}"), span.mul_f64(lo + (hi - lo) * unit)));
    }
    points
}

#[test]
fn a_killed_rust_ingest_leaves_whole_files_and_a_rust_rerun_equals_a_clean_run() {
    assert_cwds_are_gone();
    let (clean, total, took) = clean_large();
    println!("large set: {FILES} files x {LINES} lines, {total} events, clean Rust run {took:?}");
    let points = kill_points(
        &[Duration::from_millis(30), Duration::from_millis(100)],
        took,
        0.0,
        1.0,
    );
    for (name, delay) in points {
        println!(
            "{}",
            kill_then_resume(
                &format!("rust kill, rust resume, {name}"),
                rust_ingest,
                rust_ingest,
                delay,
                &clean,
                total,
                false
            )
        );
    }
    println!(
        "children of this binary reaped with an empty process group so far: {}",
        REAPED.load(std::sync::atomic::Ordering::SeqCst)
    );
}

/// Criterion 4: a stopped (SIGSTOP) holder keeps the lock: the next ingest waits its 0.5 s and exits 75 without
/// touching the store; after SIGCONT the first finishes and a third run succeeds; the tables equal a clean run.
#[test]
fn a_stopped_holder_makes_the_next_ingest_exit_75_and_a_third_succeeds_after_sigcont() {
    assert_cwds_are_gone();
    let (clean, _, _) = clean_large();
    let case = large_case();
    let mut a = rust_ingest(&case).spawn();
    let lock = case.home.join("ingest.lock");
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut polls = 0;
    loop {
        assert!(
            a.running(),
            "the first ingest finished before it was seen holding the lock"
        );
        assert!(
            Instant::now() < deadline,
            "the first ingest never took the lock"
        );
        polls += 1;
        if let Ok(f) = fs::OpenOptions::new().append(true).open(&lock) {
            match f.try_lock() {
                Err(fs::TryLockError::WouldBlock) => break,
                Ok(()) => f.unlock().unwrap(),
                Err(fs::TryLockError::Error(e)) => panic!("lock probe: {:?}", e.kind()),
            }
        }
        sleep(Duration::from_millis(1));
    }
    a.signal("STOP");
    let snapshot = |case: &Case| -> Vec<Option<Vec<u8>>> {
        ["", "-wal", "-shm"]
            .iter()
            .map(|s| fs::read(format!("{}{s}", case.store_path().display())).ok())
            .collect()
    };
    let before = snapshot(&case);
    let b = rust_ingest(&case).env(LOCK_TIMEOUT, "0.5").run();
    assert_eq!(
        b.code,
        Some(75),
        "a second ingest while the holder is stopped"
    );
    assert_eq!((b.stderr.as_str(), b.stdout.as_str()), (BUSY, ""));
    assert!(
        b.elapsed >= Duration::from_millis(450),
        "it waited {:?}",
        b.elapsed
    );
    assert!(
        snapshot(&case) == before,
        "the busy run changed the store files"
    );
    a.signal("CONT");
    let c = rust_ingest(&case).run();
    let a = a.wait(LIMIT);
    assert_eq!(
        (a.code, c.code),
        (Some(0), Some(0)),
        "the first after SIGCONT and the third"
    );
    assert_same(
        &checked_dump(&case, "after SIGCONT"),
        &clean,
        "after SIGCONT",
    );
    println!(
        "SIGSTOP: holder seen after {polls} lock probes; second exit 75 after {:?} with the store files unchanged; \
         after SIGCONT first exit 0 (new_events {}), third exit 0 (new_events {}); tables = clean",
        b.elapsed,
        a.new_events(),
        c.new_events()
    );
}

// -- criterion 5: readers while writing

/// `serve` (`/api/usage/summary`, `/api/warnings`), `mcp` (`usage_summary`) and `store-info` while `writer` ingests
/// round after round of new large sets, until 50 polls were sent while an ingest was running and had already
/// committed rows of its round. Every answer is 200 / valid JSON
/// / never "database is locked"; `messages` never goes down. After it, the numbers equal a fresh read.
fn readers_while_writing(label: &str, writer: Maker) {
    assert_cwds_are_gone();
    let case = Case::new();
    precreate_store(&case);
    let serve = Serve::start(&case);
    let mut mcp = Mcp::start(&case);
    let (mut polls, mut during, mut rounds, mut mcp_calls, mut infos) = (0, 0, 0u64, 0, 0);
    let (mut last_messages, mut round_base) = (-1, 0);
    let mut ingest: Option<Proc> = None;
    let started = Instant::now();
    while during < 50 {
        if ingest.as_mut().is_none_or(|p| !p.running()) {
            if let Some(p) = ingest.take() {
                let out = p.wait(LIMIT);
                // The product's stderr carries no path or value by design (NFR-002), so it may be shown.
                assert_eq!(
                    out.code,
                    Some(0),
                    "{label}: an ingest round failed: {}",
                    out.stderr.trim()
                );
            }
            assert!(
                started.elapsed() < READERS_DEADLINE,
                "{label}: no 50 polls during an ingest within {READERS_DEADLINE:?} ({during} so far, {rounds} rounds)"
            );
            round_base = last_messages.max(0);
            write_large_set(
                &case.projects,
                &format!("r{rounds}"),
                FILES,
                LINES,
                SET_SEED + rounds,
            );
            rounds += 1;
            ingest = Some(writer(&case).spawn());
        }
        let busy_before = ingest.as_mut().unwrap().running();
        for path in ["/api/usage/summary", "/api/warnings"] {
            let (status, body) = serve.get(path);
            assert_eq!(status, 200, "{label}: {path} status");
            assert!(!body.contains("locked"), "{label}: {path} said locked");
            let v: Value = serde_json::from_str(&body).expect("valid JSON");
            assert!(v.is_object(), "{label}: {path} is an object");
            if path == "/api/usage/summary" {
                let m = v["messages"].as_i64().expect("messages");
                assert!(m >= last_messages, "{label}: messages went down");
                last_messages = m;
            }
        }
        if polls % 10 == 0 {
            mcp.usage_summary();
            mcp_calls += 1;
            let info = Spec::rust(&case, &["store-info", "--json"]).run();
            assert_eq!(info.code, Some(0), "{label}: store-info");
            assert!(
                info.json()["tables"]["events"].is_i64(),
                "{label}: store-info events"
            );
            infos += 1;
        }
        polls += 1;
        // A poll counts when it was sent while the ingest was running and after this round's first rows were
        // committed (so it read a store that was being written, not one waiting for Python's imports).
        if busy_before && last_messages > round_base {
            during += 1;
        }
    }
    let last = ingest.take().unwrap().wait(LIMIT);
    assert_eq!(
        last.code,
        Some(0),
        "{label}: the last ingest round: {}",
        last.stderr.trim()
    );
    // After the ingest: the long-running readers agree with fresh ones and with the store.
    let (_, final_summary) = serve.get("/api/usage/summary");
    let final_mcp = mcp.usage_summary();
    let fresh = Serve::start(&case);
    let (_, fresh_summary) = fresh.get("/api/usage/summary");
    let mut fresh_mcp = Mcp::start(&case);
    assert_eq!(
        serde_json::from_str::<Value>(&final_summary).unwrap(),
        serde_json::from_str::<Value>(&fresh_summary).unwrap(),
        "{label}: serve after the ingest = a fresh serve"
    );
    assert!(
        final_mcp == fresh_mcp.usage_summary(),
        "{label}: mcp after the ingest = a fresh mcp"
    );
    let info = Spec::rust(&case, &["store-info", "--json"]).run().json();
    let count: i64 = Connection::open(case.store_path())
        .unwrap()
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        info["tables"]["events"].as_i64(),
        Some(count),
        "{label}: store-info = the store"
    );
    assert_intact(&case.store_path(), label);
    fresh_mcp.stop();
    fresh.stop();
    mcp.stop();
    serve.stop();
    println!(
        "{label}: {polls} polls of summary + warnings (all 200, valid JSON, never locked), {during} sent while an ingest ran with its first rows committed, \
         over {rounds} rounds in {:?}; {mcp_calls} mcp usage_summary and {infos} store-info answered; final \
         messages {last_messages}; after it serve = fresh serve, mcp = fresh mcp, store-info events = {count}",
        started.elapsed()
    );
}

#[test]
fn serve_mcp_and_store_info_answer_while_a_rust_ingest_runs() {
    readers_while_writing("readers during rust ingest", rust_ingest);
}

// -- criterion 8: a weird environment

#[test]
fn a_data_dir_with_a_space_and_a_non_ascii_character_works() {
    assert_cwds_are_gone();
    let mut case = s01_case();
    case.home = case.root.join("data dir \u{e9}t\u{e9}");
    let out = rust_ingest(&case).run();
    assert_eq!(out.code, Some(0), "exit");
    assert_eq!(out.new_events(), 39);
    let want = golden_tables(&golden("S01_cold_committed.json")["runs"][0]);
    assert_same(&checked_dump(&case, "odd home"), &want, "odd home");
    assert_eq!(listing(&case.home), ["ingest.lock", "store.sqlite"]);
}

#[test]
fn a_read_only_data_dir_is_a_clean_exit_1_never_a_hang() {
    if is_root() {
        println!("skipped: root ignores file modes");
        return;
    }
    let case = s01_case();
    // Empty and read-only: the lock file cannot be made.
    fs::create_dir_all(&case.home).unwrap();
    fs::set_permissions(&case.home, fs::Permissions::from_mode(0o555)).unwrap();
    let empty = rust_ingest(&case).spawn().wait(Duration::from_secs(30));
    fs::set_permissions(&case.home, fs::Permissions::from_mode(0o700)).unwrap();
    empty.assert_clean_error(1, "read-only empty data dir");
    // With a store and its lock already there: SQLite cannot make its -wal / -shm.
    assert_eq!(rust_ingest(&case).run().code, Some(0));
    let before = fs::read(case.store_path()).unwrap();
    case.copy(
        &fixtures().join("s3/session.jsonl"),
        &case.projects.join("new/session.jsonl"),
    );
    fs::set_permissions(&case.home, fs::Permissions::from_mode(0o555)).unwrap();
    let full = rust_ingest(&case).spawn().wait(Duration::from_secs(30));
    fs::set_permissions(&case.home, fs::Permissions::from_mode(0o700)).unwrap();
    full.assert_clean_error(1, "read-only data dir with a store");
    assert!(
        fs::read(case.store_path()).unwrap() == before,
        "the store changed"
    );
    println!(
        "read-only data dir: empty -> {:?}; with a store -> {:?}",
        empty.stderr.trim(),
        full.stderr.trim()
    );
}

#[test]
fn a_store_file_of_mode_0444_is_a_clean_error_and_is_not_changed() {
    if is_root() {
        println!("skipped: root ignores file modes");
        return;
    }
    let case = s01_case();
    assert_eq!(rust_ingest(&case).run().code, Some(0));
    case.copy(
        &fixtures().join("s3/session.jsonl"),
        &case.projects.join("new/session.jsonl"),
    );
    let before = fs::read(case.store_path()).unwrap();
    fs::set_permissions(case.store_path(), fs::Permissions::from_mode(0o444)).unwrap();
    let out = rust_ingest(&case).spawn().wait(Duration::from_secs(30));
    fs::set_permissions(case.store_path(), fs::Permissions::from_mode(0o644)).unwrap();
    out.assert_clean_error(1, "store 0444");
    assert!(
        fs::read(case.store_path()).unwrap() == before,
        "the store changed"
    );
    assert_intact(&case.store_path(), "store 0444");
    println!("store 0444 -> {:?}", out.stderr.trim());
}

#[test]
fn an_ingest_lock_of_mode_0444_is_a_clean_exit_1_never_a_hang() {
    if is_root() {
        println!("skipped: root ignores file modes");
        return;
    }
    let case = s01_case();
    fs::create_dir_all(&case.home).unwrap();
    let lock = case.home.join("ingest.lock");
    fs::write(&lock, b"").unwrap();
    fs::set_permissions(&lock, fs::Permissions::from_mode(0o444)).unwrap();
    let out = rust_ingest(&case).spawn().wait(Duration::from_secs(30));
    out.assert_clean_error(1, "ingest.lock 0444");
    assert!(
        out.elapsed < Duration::from_secs(5),
        "it took {:?}",
        out.elapsed
    );
    assert!(!case.store_path().exists(), "the store is not created");
    println!(
        "ingest.lock 0444 -> {:?} in {:?}",
        out.stderr.trim(),
        out.elapsed
    );
}

// -- the Python half (ignored: needs rust/parity/.venv; run with `-- --ignored`)

#[test]
#[ignore = "needs the parity venv: rust/parity/.venv (see rust/parity/README.md)"]
fn py_rust_and_python_at_the_same_instant_on_the_committed_fixtures() {
    races(
        "rust||python S01",
        s01_case,
        "S01_cold_committed.json",
        rust_ingest,
        python_ingest,
        None,
    );
}

#[test]
#[ignore = "needs the parity venv: rust/parity/.venv (see rust/parity/README.md)"]
fn py_rust_and_python_at_the_same_instant_on_the_synthetic_set() {
    races(
        "rust||python S06",
        s06_case,
        "S06_cold_synthetic.json",
        rust_ingest,
        python_ingest,
        None,
    );
}

#[test]
#[ignore = "needs the parity venv: rust/parity/.venv (see rust/parity/README.md)"]
fn py_rust_and_python_with_no_lock_wait_one_may_exit_75() {
    races(
        "rust||python S01 timeout 0",
        s01_case,
        "S01_cold_committed.json",
        rust_ingest,
        python_ingest,
        Some("0"),
    );
}

#[test]
#[ignore = "needs the parity venv: rust/parity/.venv (see rust/parity/README.md)"]
fn py_python_and_python_at_the_same_instant_control_s01() {
    races(
        "python||python S01",
        s01_case,
        "S01_cold_committed.json",
        python_ingest,
        python_ingest,
        None,
    );
}

#[test]
#[ignore = "needs the parity venv: rust/parity/.venv (see rust/parity/README.md)"]
fn py_python_and_python_at_the_same_instant_control_s06() {
    races(
        "python||python S06",
        s06_case,
        "S06_cold_synthetic.json",
        python_ingest,
        python_ingest,
        None,
    );
}

#[test]
#[ignore = "needs the parity venv: rust/parity/.venv (see rust/parity/README.md)"]
fn py_a_killed_rust_ingest_resumed_by_python_equals_a_clean_run() {
    assert_cwds_are_gone();
    let (clean, total, took) = clean_large();
    println!("large set: {total} events; clean Rust run {took:?}");
    let fixed = [Duration::from_millis(30), Duration::from_millis(100)];
    for (name, delay) in kill_points(&fixed, took, 0.0, 1.0) {
        println!(
            "{}",
            kill_then_resume(
                &format!("rust kill, python resume, {name}"),
                rust_ingest,
                python_ingest,
                delay,
                &clean,
                total,
                false
            )
        );
    }
}

#[test]
#[ignore = "needs the parity venv: rust/parity/.venv (see rust/parity/README.md)"]
fn py_a_killed_python_ingest_resumed_by_rust_equals_a_clean_run() {
    assert_cwds_are_gone();
    let (clean, total, _) = clean_large();
    let case = large_case();
    precreate_store(&case);
    let py = python_ingest(&case).run();
    assert_eq!(py.code, Some(0), "clean Python run");
    assert_same(
        &checked_dump(&case, "clean python"),
        &clean,
        "clean Python run = clean Rust run",
    );
    println!(
        "large set: {total} events; clean Python run {:?} (tables = the clean Rust run)",
        py.elapsed
    );
    // Python spends its first part importing: its points are fractions of its own clean run, early enough to land
    // while it writes (the store is made first, so a kill never lands in Python's migration).
    let fixed: Vec<Duration> = [0.45, 0.55, 0.65]
        .iter()
        .map(|f| py.elapsed.mul_f64(*f))
        .collect();
    for (name, delay) in kill_points(&fixed, py.elapsed, 0.35, 0.75) {
        println!(
            "{}",
            kill_then_resume(
                &format!("python kill, rust resume, {name}"),
                python_ingest,
                rust_ingest,
                delay,
                &clean,
                total,
                true
            )
        );
    }
}

#[test]
#[ignore = "needs the parity venv: rust/parity/.venv (see rust/parity/README.md)"]
fn py_serve_mcp_and_store_info_answer_while_a_python_ingest_runs() {
    readers_while_writing("readers during python ingest", python_ingest);
}

/// Criterion 6: a transcript copied to a new name and replaced by a copy (new inode, same path), ingested Rust then
/// Python and Python then Rust: the same rows. The inode is part of the key, so each copy adds the rows again on both
/// sides (golden S13): a documented weak spot, not a defect.
#[test]
#[ignore = "needs the parity venv: rust/parity/.venv (see rust/parity/README.md)"]
fn py_copied_and_restored_files_give_the_same_rows_on_both_sides() {
    let s13 = golden("S13_file_replaced_new_inode.json");
    assert_eq!(
        s13["runs"][1]["stdout_json"]["new_events"], 2,
        "S13: Python reads the replaced file again"
    );
    let one = |first: Maker, second: Maker, what: &str| -> Value {
        let case = Case::new();
        let file = case.projects.join("s13/session.jsonl");
        case.copy(&fixtures().join("s3/session.jsonl"), &file);
        let a = first(&case).run();
        assert_eq!((a.code, a.new_events()), (Some(0), 2), "{what}: first");
        fs::copy(&file, file.with_file_name("session-copy.jsonl")).unwrap();
        let tmp = file.with_file_name("session.jsonl.tmp");
        fs::copy(&file, &tmp).unwrap();
        fs::rename(&tmp, &file).unwrap();
        let b = second(&case).run();
        assert_eq!(
            (b.code, b.new_events()),
            (Some(0), 4),
            "{what}: second reads the copy and the new inode"
        );
        checked_dump(&case, what)
    };
    let rp = one(rust_ingest, python_ingest, "rust then python");
    let pr = one(python_ingest, rust_ingest, "python then rust");
    assert_same(&rp, &pr, "rust-then-python = python-then-rust");
    assert_eq!(rp["events"].as_array().unwrap().len(), 6);
    println!(
        "copies: rust->python and python->rust both 6 events (2 + 2 for the new name + 2 for the new inode), equal rows"
    );
}

const MAKE_V10: &str = "import sqlite3, sys\nfrom pathlib import Path\nfrom easy_personal.core import store\n\
from easy_personal.core.migrations import MIGRATIONS\np = Path(sys.argv[1])\nc = sqlite3.connect(p)\n\
store.migrate(c, p, MIGRATIONS[:10])\nc.close()";

/// Criterion 7: a v10 store (made like S12) is refused by Rust and left byte-identical; Python migrates it; Rust then
/// ingests into it. A v12 store is refused by both and left byte-identical.
#[test]
#[ignore = "needs the parity venv: rust/parity/.venv (see rust/parity/README.md)"]
fn py_version_guards_while_the_other_side_acts() {
    assert_cwds_are_gone();
    let case = s01_case();
    fs::create_dir_all(&case.home).unwrap();
    let made = Spec::python_code(&case, MAKE_V10, &[case.store_path().as_os_str()]).run();
    assert_eq!(made.code, Some(0), "make the v10 store");
    let v10 = fs::read(case.store_path()).unwrap();
    let r = rust_ingest(&case).run();
    assert_eq!(r.code, Some(2), "rust on v10");
    assert!(
        fs::read(case.store_path()).unwrap() == v10,
        "v10 bytes changed"
    );
    let left_v10 = listing(&case.home);
    let p = python_ingest(&case).run();
    assert_eq!(
        (p.code, p.new_events()),
        (Some(0), 39),
        "python migrates v10 and ingests"
    );
    case.copy(
        &fixtures().join("s3/session.jsonl"),
        &case.projects.join("new/session.jsonl"),
    );
    let r2 = rust_ingest(&case).run();
    assert_eq!(
        (r2.code, r2.new_events()),
        (Some(0), 2),
        "rust ingests into the migrated store"
    );
    let version: i64 = Connection::open(case.store_path())
        .unwrap()
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 11);
    assert_intact(&case.store_path(), "after the migration");
    println!(
        "v10: rust exit 2 ({:?}), bytes kept, files {left_v10:?}; python exit 0 new_events 39; rust exit 0 new_events 2; user_version 11",
        r.stderr.trim()
    );

    let case = s01_case();
    precreate_store(&case);
    case.sql("PRAGMA user_version = 12");
    let v12 = fs::read(case.store_path()).unwrap();
    let r = rust_ingest(&case).run();
    assert_eq!(r.code, Some(2), "rust on v12");
    assert!(
        fs::read(case.store_path()).unwrap() == v12,
        "v12 bytes changed by rust"
    );
    let p = python_ingest(&case).run();
    assert_eq!(p.code, Some(1), "python on v12");
    assert!(
        p.stderr.contains("newer than this program"),
        "python's refusal"
    );
    assert!(
        fs::read(case.store_path()).unwrap() == v12,
        "v12 bytes changed by python"
    );
    let left = listing(&case.home);
    let r = rust_ingest(&case).run();
    assert_eq!(r.code, Some(2), "rust on v12 after python");
    assert!(
        fs::read(case.store_path()).unwrap() == v12,
        "v12 bytes changed"
    );
    println!(
        "v12: rust exit 2, python exit 1, rust exit 2 again; bytes kept; files after python {left:?}"
    );
}
