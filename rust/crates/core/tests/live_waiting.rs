//! T094: the *your turn* state (FR-105) and the question text of an open `AskUserQuestion` call (FR-106, DDR-0005) in
//! the live tail. Rust-only: the Python app gets neither, so no golden is involved; `live_parity.rs` proves the shared
//! part stays equal to Python.
//!
//! Every folder is a temp one under `CARGO_TARGET_TMPDIR`, removed even when an assertion fails; no real home is read.

use easy_personal_core::ingest::json;
use easy_personal_core::live::{LiveTail, Root, call_questions, line_item};
use easy_personal_core::prices::PriceTable;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, UNIX_EPOCH};

const SECOND: i64 = 1_000_000;
/// 2026-01-20T00:00:10Z.
const NOW: i64 = 1_768_867_200 * SECOND + 10 * SECOND;
const TS: &str = "2026-01-20T00:00:05Z";
const SINCE: &str = "2026-01-20T00:00:05.000000Z";

struct Temp(PathBuf);

impl Temp {
    fn new(tag: &str) -> Temp {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "t094-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("projects/p")).unwrap();
        Temp(dir)
    }
    fn projects(&self) -> PathBuf {
        self.0.join("projects")
    }
    fn roots(&self) -> Vec<Root> {
        vec![Root {
            account: None,
            projects: self.projects(),
        }]
    }
    /// Appends `text` to `projects/p/<file>` (folders made as needed).
    fn append(&self, file: &str, text: &str) {
        let path = self.projects().join("p").join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        File::options()
            .create(true)
            .append(true)
            .open(path)
            .unwrap()
            .write_all(text.as_bytes())
            .unwrap();
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn tail() -> LiveTail {
    LiveTail::new(PriceTable::shipped().unwrap())
}

/// A tail whose first scan is done over the empty tree: everything written after it is live.
fn primed(tmp: &Temp) -> LiveTail {
    let mut live = tail();
    live.poll_once(&tmp.roots(), NOW, None);
    live
}

/// One transcript record: `kind` (`user` / `assistant`), the `entrypoint` (`None`: no such key), and `message`.
fn rec(kind: &str, entry: Option<&str>, sidechain: bool, ts: &str, message: Value) -> String {
    let mut r = json!({"type": kind, "sessionId": "s", "timestamp": ts, "cwd": "/work/demo",
                       "isSidechain": sidechain, "message": message});
    if let Some(e) = entry {
        r["entrypoint"] = json!(e);
    }
    format!("{r}\n")
}

fn user(entry: Option<&str>) -> String {
    rec(
        "user",
        entry,
        false,
        TS,
        json!({"role": "user", "content": "hi"}),
    )
}

/// A main-thread assistant record with message id `id` and this `stop_reason` (`None`: no such key).
fn assistant(entry: Option<&str>, id: &str, stop: Option<Value>) -> String {
    let mut m = json!({"id": id, "model": "x", "content": [{"type": "text", "text": "done"}]});
    if let Some(s) = stop {
        m["stop_reason"] = s;
    }
    rec("assistant", entry, false, TS, m)
}

fn end_turn(entry: Option<&str>, id: &str) -> String {
    assistant(entry, id, Some(json!("end_turn")))
}

fn states(live: &LiveTail, now: i64) -> Vec<String> {
    let snap = live.snapshot(now);
    snap["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["attention"].as_array().unwrap().clone())
        .map(|a| a["state"].as_str().unwrap().to_owned())
        .collect()
}

// ---- AC 1 -------------------------------------------------------------------------------------------------------------

#[test]
fn a_typed_turn_that_ended_is_your_turn_until_the_next_record() {
    let tmp = Temp::new("your-turn");
    let mut live = primed(&tmp);
    tmp.append(
        "s.jsonl",
        &(user(Some("cli")) + &end_turn(Some("cli"), "m1")),
    );
    live.poll_once(&tmp.roots(), NOW, None);
    let snap = live.snapshot(NOW);
    assert_eq!(
        snap["sessions"][0]["attention"],
        json!([{"state": "your_turn", "tool": null, "count": 1, "is_sidechain": false, "since": SINCE}]),
        "{snap}"
    );
    // A following user record ends it on the next poll.
    tmp.append("s.jsonl", &user(Some("cli")));
    live.poll_once(&tmp.roots(), NOW, None);
    assert_eq!(states(&live, NOW), Vec::<String>::new());
    // Again, then a following assistant record (not an end) ends it.
    tmp.append("s.jsonl", &end_turn(Some("cli"), "m2"));
    live.poll_once(&tmp.roots(), NOW, None);
    assert_eq!(states(&live, NOW), ["your_turn"]);
    tmp.append("s.jsonl", &assistant(Some("cli"), "m3", None));
    live.poll_once(&tmp.roots(), NOW, None);
    assert_eq!(states(&live, NOW), Vec::<String>::new());
    // A message split over several records: only the newest one decides.
    tmp.append(
        "s.jsonl",
        &(assistant(Some("cli"), "m4", Some(Value::Null)) + &end_turn(Some("cli"), "m4")),
    );
    live.poll_once(&tmp.roots(), NOW, None);
    assert_eq!(states(&live, NOW), ["your_turn"]);
    tmp.append("s.jsonl", &assistant(Some("cli"), "m4", Some(Value::Null)));
    live.poll_once(&tmp.roots(), NOW, None);
    assert_eq!(states(&live, NOW), Vec::<String>::new());
}

/// A record that is neither `assistant` nor `user` (here `system`) does not end the turn.
#[test]
fn a_system_record_does_not_end_the_turn() {
    let tmp = Temp::new("system");
    let mut live = primed(&tmp);
    tmp.append("s.jsonl", &end_turn(Some("cli"), "m1"));
    tmp.append(
        "s.jsonl",
        &rec("system", Some("cli"), false, TS, json!({"content": "x"})),
    );
    live.poll_once(&tmp.roots(), NOW, None);
    assert_eq!(states(&live, NOW), ["your_turn"]);
}

// ---- AC 2 -------------------------------------------------------------------------------------------------------------

#[test]
fn no_your_turn_for_agent_work_unknown_entrypoints_other_stop_reasons_or_an_open_call() {
    let sub = |stop: &str| {
        rec(
            "assistant",
            Some("cli"),
            true,
            TS,
            json!({"id": "sub1", "model": "x", "stop_reason": stop, "content": []}),
        )
    };
    let read_call = rec(
        "assistant",
        Some("cli"),
        false,
        TS,
        json!({"id": "m0", "model": "x", "content": [{"type": "tool_use", "id": "b1", "name": "Read", "input": {}}]}),
    );
    let end_with_call = rec(
        "assistant",
        Some("cli"),
        false,
        TS,
        json!({"id": "m1", "model": "x", "stop_reason": "end_turn",
               "content": [{"type": "tool_use", "id": "b2", "name": "Read", "input": {}}]}),
    );
    let end_no_id = rec(
        "assistant",
        Some("cli"),
        false,
        TS,
        json!({"model": "x", "stop_reason": "end_turn", "content": []}),
    );
    let user_end = rec(
        "user",
        Some("cli"),
        false,
        TS,
        json!({"role": "user", "stop_reason": "end_turn", "content": "x"}),
    );
    let cases: Vec<(&str, Vec<(&str, String)>)> = vec![
        (
            "entrypoint sdk-cli",
            vec![("s.jsonl", end_turn(Some("sdk-cli"), "m1"))],
        ),
        (
            "entrypoint sdk-ts",
            vec![("s.jsonl", end_turn(Some("sdk-ts"), "m1"))],
        ),
        ("no entrypoint", vec![("s.jsonl", end_turn(None, "m1"))]),
        (
            "a subagent stream's end_turn",
            vec![
                ("s.jsonl", user(Some("cli"))),
                ("s/subagents/agent-a1.jsonl", sub("end_turn")),
            ],
        ),
        (
            "a sidechain record's end_turn in the main file",
            vec![("s.jsonl", user(Some("cli")) + &sub("end_turn"))],
        ),
        (
            "stop_reason tool_use",
            vec![(
                "s.jsonl",
                assistant(Some("cli"), "m1", Some(json!("tool_use"))),
            )],
        ),
        (
            "stop_reason stop_sequence",
            vec![(
                "s.jsonl",
                assistant(Some("cli"), "m1", Some(json!("stop_sequence"))),
            )],
        ),
        (
            "stop_reason max_tokens",
            vec![(
                "s.jsonl",
                assistant(Some("cli"), "m1", Some(json!("max_tokens"))),
            )],
        ),
        (
            "stop_reason null",
            vec![("s.jsonl", assistant(Some("cli"), "m1", Some(Value::Null)))],
        ),
        (
            "stop_reason a number",
            vec![("s.jsonl", assistant(Some("cli"), "m1", Some(json!(1))))],
        ),
        (
            "stop_reason an object",
            vec![(
                "s.jsonl",
                assistant(Some("cli"), "m1", Some(json!({"end_turn": true}))),
            )],
        ),
        (
            "stop_reason END_TURN",
            vec![(
                "s.jsonl",
                assistant(Some("cli"), "m1", Some(json!("END_TURN"))),
            )],
        ),
        (
            "an open call in the ending record",
            vec![("s.jsonl", end_with_call)],
        ),
        (
            "an open call before an ending record with no id",
            vec![("s.jsonl", read_call + &end_no_id)],
        ),
        ("stop_reason on a user record", vec![("s.jsonl", user_end)]),
    ]; // fmt: skip
    for (name, files) in cases {
        let tmp = Temp::new("negative");
        let mut live = primed(&tmp);
        for (file, text) in &files {
            tmp.append(file, text);
        }
        live.poll_once(&tmp.roots(), NOW, None);
        let got = states(&live, NOW);
        assert!(!got.iter().any(|s| s == "your_turn"), "{name}: {got:?}");
        assert!(
            !live.snapshot(NOW).to_string().contains("your_turn"),
            "{name}"
        );
    }
}

// ---- AC 3 -------------------------------------------------------------------------------------------------------------

#[test]
fn states_are_ordered_failing_waiting_your_turn_needs_attention() {
    let tmp = Temp::new("order");
    let mut live = primed(&tmp);
    let uses: Vec<Value> = (1..=3)
        .map(|i| json!({"type": "tool_use", "id": format!("f{i}"), "name": "Bash", "input": {}}))
        .collect();
    let fails: Vec<Value> = (1..=3)
        .map(|i| json!({"type": "tool_result", "tool_use_id": format!("f{i}"), "is_error": true}))
        .collect();
    let main = rec(
        "assistant",
        Some("cli"),
        false,
        TS,
        json!({"id": "m1", "model": "x", "content": uses}),
    ) + &rec(
        "user",
        Some("cli"),
        false,
        TS,
        json!({"role": "user", "content": fails}),
    ) + &end_turn(Some("cli"), "m2");
    let sub = rec(
        "assistant",
        Some("cli"),
        true,
        TS,
        json!({"id": "m3", "model": "x", "content": [
            {"type": "tool_use", "id": "w1", "name": "AskUserQuestion", "input": {}},
            {"type": "tool_use", "id": "r1", "name": "Read", "input": {}}]}),
    );
    tmp.append("s.jsonl", &main);
    tmp.append("s/subagents/agent-a1.jsonl", &sub);
    let later = NOW + 11 * 60 * SECOND; // the session wrote nothing for 11 minutes: needs attention
    live.poll_once(&tmp.roots(), later, None);
    assert_eq!(
        states(&live, later),
        ["failing", "waiting", "your_turn", "needs_attention"]
    );
}

// ---- AC 4 -------------------------------------------------------------------------------------------------------------

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

/// Writes `text` as `projects/p/<file>`, modified at `NOW`, with the store cursor at its end (all of it is history).
fn history(tmp: &Temp, file: &str, text: &str) {
    let path = tmp.projects().join("p").join(file);
    fs::write(&path, text).unwrap();
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_micros(NOW as u64))
        .unwrap();
    Connection::open(tmp.0.join("store.sqlite"))
        .unwrap()
        .execute(
            "INSERT INTO source_files VALUES ('machine-1', 'claude_code', ?, ?, ?)",
            rusqlite::params![
                path.to_str().unwrap(),
                fs::metadata(&path).unwrap().ino() as i64,
                text.len() as i64
            ],
        )
        .unwrap();
}

#[test]
fn a_warm_start_shows_a_turn_that_ended_before_the_tail_started() {
    let tmp = Temp::new("warm");
    make_store(&tmp.0.join("store.sqlite"));
    let at = |ts: &str, sid: &str| {
        (user(Some("cli")) + &end_turn(Some("cli"), "m1"))
            .replace(TS, ts)
            .replace("\"s\"", &format!("\"{sid}\""))
    };
    // 20 minutes before NOW: listed with its state (it holds until the one-hour rule drops the session).
    history(&tmp, "recent.jsonl", &at("2026-01-19T23:40:10Z", "recent"));
    // 2 hours before NOW: dropped by the one-hour rule, as before.
    history(&tmp, "old.jsonl", &at("2026-01-19T22:00:10Z", "old"));
    let mut live = tail();
    live.poll_store(&tmp.0.join("store.sqlite"), Some(&tmp.projects()), NOW);
    let snap = live.snapshot(NOW);
    let sessions = snap["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 1, "{snap}");
    assert_eq!(sessions[0]["session_id"], "recent");
    assert_eq!(sessions[0]["attention"][0]["state"], "your_turn");
    assert_eq!(
        sessions[0]["attention"][0]["since"],
        "2026-01-19T23:40:10.000000Z"
    );
    assert!(
        live.events_since(0).is_empty(),
        "a replay publishes nothing"
    );
    // 61 minutes after its record the session leaves the list, with its state.
    let gone = NOW + 41 * 60 * SECOND;
    live.poll_store(&tmp.0.join("store.sqlite"), Some(&tmp.projects()), gone);
    assert_eq!(live.snapshot(gone)["sessions"], json!([]));
}

// ---- AC 5 (engine half; the stream half is in `live_feed.rs`) ----------------------------------------------------------

fn ask(id: &str, block: &str, input: Value) -> String {
    rec(
        "assistant",
        Some("cli"),
        false,
        TS,
        json!({"id": id, "model": "x", "stop_reason": "tool_use",
               "content": [{"type": "tool_use", "id": block, "name": "AskUserQuestion", "input": input}]}),
    )
}

fn questions_input(qs: &[&str]) -> Value {
    json!({"questions": qs.iter().map(|q| json!({"question": q, "header": "H", "multiSelect": false,
                                                  "options": [{"label": "A", "description": "a"}]}))
                         .collect::<Vec<_>>()})
}

#[test]
fn the_question_text_is_held_beside_the_snapshot_and_dropped_with_the_result() {
    let tmp = Temp::new("questions");
    let mut live = primed(&tmp);
    tmp.append(
        "s.jsonl",
        &ask(
            "m1",
            "b1",
            questions_input(&["Which colour?", "Ship it now?"]),
        ),
    );
    live.poll_once(&tmp.roots(), NOW, None);
    let snap = live.snapshot(NOW);
    assert_eq!(snap["sessions"][0]["attention"][0]["state"], "waiting");
    assert!(
        !snap.to_string().contains("questions") && !snap.to_string().contains("colour"),
        "{snap}"
    );
    let held = live.waiting_questions();
    assert_eq!(
        format!("{:?}", held.get("s")),
        "Some([(0, [\"Which colour?\", \"Ship it now?\"])])"
    );
    let version = live.state_version();
    tmp.append(
        "s.jsonl",
        &rec(
            "user",
            Some("cli"),
            false,
            TS,
            json!({"role": "user", "content": [{"type": "tool_result", "tool_use_id": "b1", "content": "Red"}]}),
        ),
    );
    live.poll_once(&tmp.roots(), NOW, None);
    assert!(live.waiting_questions().is_empty());
    assert_ne!(live.state_version(), version);
}

/// Several open `AskUserQuestion` calls in one stream: the questions of the oldest one, and a questions-only change
/// moves `state_version` (the stream sends a new snapshot).
#[test]
fn the_oldest_open_call_gives_the_questions_and_a_change_moves_the_version() {
    let tmp = Temp::new("oldest");
    let mut live = primed(&tmp);
    let two = rec(
        "assistant",
        Some("cli"),
        false,
        TS,
        json!({"id": "m1", "model": "x", "content": [
            {"type": "tool_use", "id": "b1", "name": "AskUserQuestion", "input": questions_input(&["first?"])},
            {"type": "tool_use", "id": "b2", "name": "AskUserQuestion", "input": questions_input(&["second?"])}]}),
    );
    tmp.append("s.jsonl", &two);
    live.poll_once(&tmp.roots(), NOW, None);
    assert_eq!(
        format!("{:?}", live.waiting_questions().get("s")),
        "Some([(0, [\"first?\"])])"
    );
    // A new message closes both (FR-049) and opens one call with the same time: the state JSON is the same but for
    // the count; then one more such message: only the questions differ.
    tmp.append("s.jsonl", &ask("m2", "b3", questions_input(&["third?"])));
    live.poll_once(&tmp.roots(), NOW, None);
    let version = live.state_version();
    let snap = live.snapshot(NOW)["sessions"][0]["attention"].clone();
    tmp.append("s.jsonl", &ask("m3", "b4", questions_input(&["fourth?"])));
    live.poll_once(&tmp.roots(), NOW, None);
    assert_eq!(live.snapshot(NOW)["sessions"][0]["attention"], snap);
    assert_ne!(live.state_version(), version, "only the questions changed");
    assert_eq!(
        format!("{:?}", live.waiting_questions().get("s")),
        "Some([(0, [\"fourth?\"])])"
    );
    // Nothing new: the version stays.
    let version = live.state_version();
    live.poll_once(&tmp.roots(), NOW, None);
    assert_eq!(live.state_version(), version);
}

// ---- AC 6 -------------------------------------------------------------------------------------------------------------

fn qs(input: &str) -> Option<Vec<String>> {
    call_questions("AskUserQuestion", &json::loads(input.as_bytes()).unwrap())
}

fn one(q: &str) -> String {
    json!({"questions": [{"question": q}]}).to_string()
}

#[test]
fn question_limits_and_odd_inputs() {
    let five = r#"{"questions":[{"question":"q1"},{"question":"q2"},{"question":"q3"},{"question":"q4"},{"question":"q5"}]}"#;
    assert_eq!(qs(five).unwrap(), ["q1", "q2", "q3", "q4"]);
    let long = qs(&one(&"\u{e9}".repeat(10_000))).unwrap();
    assert_eq!(long[0].chars().count(), 300);
    assert!(long[0].chars().all(|c| c == '\u{e9}'), "cut on a character");
    // 299 characters (no 32-character run: that would be secret-shaped), a 3-byte character, then more.
    let head = format!("{}a", "a ".repeat(149));
    let mixed = format!("{head}\u{20ac}{}", "b ".repeat(25));
    assert_eq!(
        qs(&one(&mixed)).unwrap()[0],
        format!("{head}\u{20ac}"),
        "a 3-byte character at the cut"
    );
    for hidden in [
        '\u{0}', '\u{1}', '\t', '\n', '\r', '\u{1b}', '\u{1f}', '\u{7f}', '\u{80}', '\u{85}',
        '\u{9f}', '\u{2028}', '\u{2029}', '\u{200b}', '\u{200c}', '\u{200d}', '\u{200e}',
        '\u{200f}', '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}', '\u{202e}', '\u{2066}',
        '\u{2067}', '\u{2068}', '\u{2069}', '\u{feff}',
    ] {
        assert_eq!(
            qs(&one(&format!("a{hidden}b"))).unwrap(),
            ["ab"],
            "U+{:04X}",
            hidden as u32
        );
    }
    // A lone surrogate is dropped too, never a panic.
    assert_eq!(
        qs(r#"{"questions":[{"question":"a\ud800b"}]}"#).unwrap(),
        ["ab"]
    );
    // A secret-shaped question is dropped whole; its neighbour stays. Also one that is secret-shaped only once the
    // hidden characters are gone, and one whose secret lies past the cut.
    let key = "sk-ant-api03-ABCDEFG";
    let secrets = json!({"questions": [{"question": format!("Use {key}?")}, {"question": "Plain one?"},
        {"question": "sk-\u{200b}abc"}, {"question": format!("{} {}", "x".repeat(400), "A".repeat(40))},
        {"question": "-----BEGIN key"}]});
    assert_eq!(qs(&secrets.to_string()).unwrap(), ["Plain one?"]);
    for odd in [
        r#"{"questions":"q"}"#,
        r#"{"questions":{"question":"q"}}"#,
        r#"{"questions":null}"#,
        r#"{"questions":[]}"#,
        r#"{"questions":["q", 1, null, ["q"]]}"#,
        r#"{"questions":[{"question":1},{"question":null},{"question":["q"]},{"question":{"q":1}}]}"#,
        r#"{"questions":[{"question":""},{"question":"​\n"}]}"#,
        r#"{"question":"q"}"#,
        r#"["q"]"#,
        r#""q""#,
        r#"null"#,
    ] {
        assert_eq!(qs(odd), None, "{odd}");
    }
    assert_eq!(
        qs(r#"{"questions":["q",{"question":""},{"question":"kept"}]}"#).unwrap(),
        ["kept"]
    );
    // Only AskUserQuestion is read.
    let input = json::loads(one("q?").as_bytes()).unwrap();
    for name in [
        "ExitPlanMode",
        "Read",
        "askuserquestion",
        "mcp__x__AskUserQuestion",
    ] {
        assert_eq!(call_questions(name, &input), None, "{name}");
    }
    // A huge list is bounded and quick.
    let many = format!(
        "{{\"questions\":[{}]}}",
        vec![r#"{"question":""}"#; 100_000].join(",")
    );
    let began = std::time::Instant::now();
    assert_eq!(qs(&many), None);
    assert!(began.elapsed() < Duration::from_secs(5));
}

// ---- AC 7 -------------------------------------------------------------------------------------------------------------

#[test]
fn header_options_multiselect_and_plan_are_never_read() {
    let tmp = Temp::new("never-read");
    let mut live = primed(&tmp);
    let input = json!({"questions": [{"question": "QUESTION-MARKER pick one?", "header": "PLANTED-HEADER",
        "multiSelect": "PLANTED-MULTI",
        "options": [{"label": "PLANTED-LABEL", "description": "PLANTED-DESCRIPTION"}]}],
        "PLANTED-KEY": "PLANTED-OTHER"});
    let line = ask("m1", "b1", input.clone());
    let plan = rec(
        "assistant",
        Some("cli"),
        true,
        TS,
        json!({"id": "m2", "model": "x", "content": [
            {"type": "tool_use", "id": "b2", "name": "ExitPlanMode", "input": {"plan": "PLANTED-PLAN"}}]}),
    );
    tmp.append("s.jsonl", &line);
    tmp.append("s/subagents/agent-a1.jsonl", &plan);
    live.poll_once(&tmp.roots(), NOW, None);
    let held = live.waiting_questions();
    assert_eq!(
        format!("{:?}", held.get("s")),
        "Some([(0, [\"QUESTION-MARKER pick one?\"])])",
        "positive control"
    );
    let item = line_item(line.as_bytes(), &mut |c: &str| c.to_owned()).unwrap();
    let mut outputs = vec![
        live.snapshot(NOW).to_string(),
        serde_json::to_string(&live.events_since(0)).unwrap(),
        format!("{live:?}"),
        format!("{held:?}"),
        format!("{item:?}"),
        item.to_json().to_string(),
    ];
    let questions = call_questions(
        "AskUserQuestion",
        &json::loads(input.to_string().as_bytes()).unwrap(),
    );
    assert_eq!(
        questions.as_deref(),
        Some(&["QUESTION-MARKER pick one?".to_owned()][..])
    );
    outputs.push(format!("{questions:?}"));
    for out in &outputs {
        assert!(!out.contains("PLANTED"), "{out}");
    }
    // The question text itself is never in a `Debug` line, the snapshot or an event.
    for out in [
        &outputs[0],
        &outputs[1],
        &outputs[2],
        &outputs[4],
        &outputs[5],
    ] {
        assert!(!out.contains("QUESTION-MARKER"), "{out}");
    }
}
