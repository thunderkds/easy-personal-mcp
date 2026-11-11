//! T006: `rules::evaluate` against the Python goldens (`rust/parity/golden/api__warnings__all.json` and
//! `mcp__warnings__all.json`, made over `rust/parity/store-v11.sqlite` with TZ=UTC and the frozen clock below),
//! the rule-file parser, and one synthetic store per rule checked against Python's own answer.
//! Goldens are compared as parsed JSON (key order ignored; `1000` and `1000.0` differ).
//! Variations are built on temp copies; the owner's real `~/.easy-personal` is never opened.

use easy_personal_core::rules::{Num, RulesError, evaluate, parse_rules, shipped_rules};
use easy_personal_core::store::open_read_only;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 2026-01-20T00:00:00Z, the golden's frozen clock (`frozen_now` in the manifest), in microseconds.
const FROZEN_NOW: i64 = 1_768_867_200_000_000;

fn parity_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity")
}

fn fixture() -> PathBuf {
    parity_dir().join("store-v11.sqlite")
}

fn golden(name: &str) -> Value {
    let text =
        fs::read_to_string(parity_dir().join("golden").join(format!("{name}.json"))).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn open_fixture() -> Connection {
    open_read_only(&fixture()).unwrap()
}

fn warnings(conn: &Connection) -> Value {
    evaluate(conn, FROZEN_NOW, &shipped_rules().unwrap()).unwrap()
}

/// A temp folder per test; `T006_KEEP=1` keeps it (to run Python over the store it holds).
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t006-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if std::env::var_os("T006_KEEP").is_none() {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

// ---- 1. the goldens (AC1, AC2, AC4, AC5) ----

#[test]
fn warnings_match_the_api_golden() {
    assert_eq!(warnings(&open_fixture()), golden("api__warnings__all"));
}

#[test]
fn warnings_match_the_mcp_golden() {
    assert_eq!(warnings(&open_fixture()), golden("mcp__warnings__all"));
}

// ---- 2. the fixture is read-only (RS-002) ----

#[test]
fn fixture_is_unchanged_and_gets_no_sidecars() {
    let before = fs::read(fixture()).unwrap();
    {
        let c = open_fixture();
        warnings(&c);
    }
    assert_eq!(fs::read(fixture()).unwrap(), before);
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut name = fixture().into_os_string();
        name.push(suffix);
        assert!(!Path::new(&name).exists(), "sidecar {suffix} appeared");
    }
}

// ---- 3. the tests are not constant ----

#[test]
fn changing_one_fixture_value_breaks_the_golden_comparison() {
    let dir = TempDir::new("mutation");
    let path = dir.0.join("store.sqlite");
    fs::copy(fixture(), &path).unwrap();
    // Python's records_unreadable measured 0.25 (1 of 4 records of version 2.1.302): make that record readable.
    Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE events SET abstained = 0 WHERE version = '2.1.302' AND abstained = 1",
            [],
        )
        .unwrap();
    let out = warnings(&open_read_only(&path).unwrap());
    assert_ne!(out, golden("api__warnings__all"));
    assert_ne!(out["hits"], golden("api__warnings__all")["hits"]);
}

// ---- 4. the rule file parser (AC3) ----

/// Expected values: `rust/parity/.venv/bin/python -c "from easy_personal.core import rules;
/// [print(r.id, r.metric, r.op, r.threshold, r.label, r.priority, r.window_days, r.min_history_days) for r in rules.load_rules()]"`
#[test]
fn shipped_pack_has_the_seven_rules_with_python_values() {
    let pack = shipped_rules().unwrap();
    let got: Vec<_> = pack
        .iter()
        .map(|r| {
            (
                r.id.as_str(),
                r.threshold,
                r.label.as_str(),
                r.priority,
                r.window_days,
                r.min_history_days,
            )
        })
        .collect();
    let want = [
        ("cache_hit_low", Num::Float(0.5), "warning", 1, 7, 7),
        (
            "session_huge_no_compact",
            Num::Int(150000),
            "improve",
            2,
            7,
            7,
        ),
        ("records_unreadable", Num::Float(0.02), "aware", 2, 7, 0),
        ("tokens_jump", Num::Float(0.5), "aware", 3, 7, 14),
        ("project_dominates", Num::Float(0.6), "aware", 3, 7, 7),
        ("tool_errors_high", Num::Float(0.1), "warning", 2, 7, 0),
        ("context_index_stale", Num::Int(0), "improve", 3, 7, 0),
    ];
    assert_eq!(got, want);
    assert_eq!(pack[0].title, "Low cache reuse");
    assert_eq!(pack[1].steps.len(), 2);
}

const ONE: &str = "  r1: metric=cache_hit_ratio op=lt threshold=0.5 label=warning priority=1 window_days=7 min_history_days=7 title=T why=W steps=a|b";

fn parse_error_line(text: &str) -> usize {
    match parse_rules(text) {
        Err(RulesError::Parse { line, .. }) => line,
        other => panic!("expected a parse error, got {other:?}"),
    }
}

#[test]
fn parser_skips_comments_blanks_and_other_sections_and_keeps_the_last_of_a_repeat() {
    let text = format!(
        "# c\nother:\n  x: not a rule\n\nrules:\n  # inside\n\n{ONE}\n{}\n",
        ONE.replace("threshold=0.5", "threshold=0.25")
    );
    let pack = parse_rules(&text).unwrap();
    assert_eq!(pack.len(), 1, "a repeated id is one rule");
    assert_eq!(pack[0].threshold, Num::Float(0.25), "the later value wins");
    assert_eq!(parse_rules("rules:\n").unwrap().len(), 0);
}

#[test]
fn parser_errors_name_the_line_and_never_its_text() {
    let secret = "SECRET-ROW-TEXT";
    let cases = [
        ("missing title", ONE.replace(" title=T", "")),
        ("missing why", ONE.replace(" why=W", "")),
        ("missing steps", ONE.replace(" steps=a|b", "")),
        ("four steps", format!("{ONE}|c|d")),
        ("bad op", ONE.replace("op=lt", "op=eq")),
        ("bad label", ONE.replace("label=warning", "label=loud")),
        ("bad metric", ONE.replace("cache_hit_ratio", "nonsense")),
        ("no equals", ONE.replace("op=lt", "oplt")),
        ("unknown key", ONE.replace("op=lt", "op=lt extra=1")),
        ("bad threshold", ONE.replace("0.5", secret)),
        (
            "bad integer",
            ONE.replace("priority=1", &format!("priority={secret}")),
        ),
        ("missing key", ONE.replace(" priority=1", "")),
    ];
    for (name, line) in cases {
        let text = format!("# one\nrules:\n{line}\n");
        let err = parse_rules(&text).unwrap_err().to_string();
        assert_eq!(parse_error_line(&text), 3, "{name}");
        assert!(err.contains("line 3"), "{name}: {err}");
        assert!(
            !err.contains(secret) && !err.contains("r1"),
            "{name}: {err}"
        );
    }
}

// ---- 5. synthetic stores, checked against Python (AC4, AC5, AC6) ----
//
// Each case builds a temp store (a copy of the fixture's schema with its rows removed), runs `rules::evaluate`
// and compares the answer for the rules under test with what Python's `rules.evaluate` gives over the SAME store
// file. The expected JSON at the bottom of this file was produced with, per case store `<store>`:
//
//   TZ=UTC rust/parity/.venv/bin/python -c '
//   import json, sqlite3, sys
//   from datetime import UTC, datetime
//   from easy_personal.core import rules
//   out = rules.evaluate(sqlite3.connect(sys.argv[1]), now=datetime(2026, 1, 20, tzinfo=UTC), rules=rules.load_rules())
//   ids = sys.argv[2].split(",")
//   print(json.dumps({i: {"hit": next((h for h in out["hits"] if h["rule_id"] == i), None),
//                         "abstained": next((a for a in out["abstained"] if a["rule_id"] == i), None)} for i in ids},
//                    sort_keys=True))' <store> <rule ids>
//
// The stores are made kept (not deleted) by running the tests with `T006_KEEP=1`; the git repos the stores point
// at must still exist when Python runs, because Python groups a gone folder under its repo by asking git.

const W1: &str = "2026-01-15T10:00:00.000000Z"; // in the 7-day window before the frozen clock
const W2: &str = "2026-01-16T10:00:00.000000Z";
const P1: &str = "2026-01-10T10:00:00.000000Z"; // in the 7 days before that

#[derive(Default, Clone)]
struct Ev<'a> {
    ts: &'a str,
    abstained: i64,
    record_type: Option<&'a str>,
    version: Option<&'a str>,
    session: Option<&'a str>,
    project: Option<&'a str>,
    model: Option<&'a str>,
    message: Option<String>,
    tokens: Option<[i64; 4]>, // input, output, cache creation, cache read
    tools: Option<&'a str>,
}

struct Db {
    dir: TempDir,
    conn: Connection,
    n: i64,
}

impl Db {
    /// An empty store with the fixture's schema and one old record, so every rule has the history it asks for.
    fn new(name: &str) -> Db {
        let dir = TempDir::new(name);
        let path = dir.0.join("store.sqlite");
        fs::copy(fixture(), &path).unwrap();
        let conn = Connection::open(&path).unwrap();
        for table in ["events", "kit_trace_events", "context_index"] {
            conn.execute(&format!("DELETE FROM {table}"), []).unwrap();
        }
        let mut db = Db { dir, conn, n: 0 };
        db.ev(Ev {
            ts: "2025-12-01T00:00:00.000000Z",
            ..Ev::default()
        });
        db
    }

    fn ev(&mut self, e: Ev) {
        self.n += 1;
        let t = e.tokens;
        self.conn
            .execute(
                "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, record_type,
                    version, ts_utc, session_id, project_path, is_sidechain, model, message_id, tool_names,
                    input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens, entrypoint)
                 VALUES ('m', 'claude_code', 'p', 1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 'cli')",
                params![
                    self.n,
                    e.abstained,
                    e.record_type,
                    e.version,
                    e.ts,
                    e.session,
                    e.project,
                    e.model,
                    e.message,
                    e.tools,
                    t.map(|t| t[0]),
                    t.map(|t| t[1]),
                    t.map(|t| t[2]),
                    t.map(|t| t[3])
                ],
            )
            .unwrap();
    }

    /// One assistant message of a session.
    fn msg(&mut self, ts: &str, session: &str, project: &str, model: &str, tokens: [i64; 4]) {
        let message = Some(format!("msg{}", self.n + 1));
        self.ev(Ev {
            ts,
            session: Some(session),
            project: Some(project),
            model: Some(model),
            message,
            tokens: Some(tokens),
            ..Ev::default()
        });
    }

    /// `n` records of one version in the window, the first `bad` of them unreadable.
    fn records(&mut self, version: Option<&str>, record_type: Option<&str>, n: i64, bad: i64) {
        for i in 0..n {
            self.ev(Ev {
                ts: W1,
                version,
                record_type: if i < bad { record_type } else { Some("user") },
                abstained: i64::from(i < bad),
                ..Ev::default()
            });
        }
    }

    /// Moves the one old record, so the store's history starts at `ts`.
    fn history_at(&self, ts: &str) {
        self.conn
            .execute(
                "UPDATE events SET ts_utc = ?1 WHERE id = (SELECT MIN(id) FROM events)",
                params![ts],
            )
            .unwrap();
    }

    fn trace(&mut self, tool: Option<&str>, project: &str, is_error: i64) {
        self.n += 1;
        self.conn
            .execute(
                "INSERT INTO kit_trace_events (machine_id, source_path, inode, byte_offset, project_path, kit_task_id,
                    abstained, ts_utc, tool_name, is_error)
                 VALUES ('m', 'p', 1, ?1, ?2, 'T001', 0, ?3, ?4, ?5)",
                params![self.n, project, W1, tool, is_error],
            )
            .unwrap();
    }

    fn index(&self, project: &str, at: &str) {
        self.conn
            .execute(
                "INSERT INTO context_index (project_path, last_index_at_utc) VALUES (?1, ?2)",
                params![project, at],
            )
            .unwrap();
    }

    /// A git repo folder under the store's temp dir; a gone sub-folder of it is grouped under it (FR-057).
    fn repo(&self, name: &str, commit_date: Option<&str>) -> String {
        let path = self.dir.0.join(name);
        fs::create_dir_all(&path).unwrap();
        git(&path, &["init", "-q"], None);
        if let Some(date) = commit_date {
            git(
                &path,
                &["commit", "-q", "--allow-empty", "-m", "c"],
                Some(date),
            );
        }
        path.to_str().unwrap().to_string()
    }
}

fn git(dir: &Path, args: &[&str], date: Option<&str>) {
    let mut cmd = Command::new("git");
    cmd.args([
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "-c",
        "init.defaultBranch=main",
    ])
    .arg("-C")
    .arg(dir)
    .args(args)
    .env("GIT_CONFIG_GLOBAL", "/dev/null")
    .env("GIT_CONFIG_NOSYSTEM", "1");
    if let Some(date) = date {
        cmd.env("GIT_AUTHOR_DATE", date)
            .env("GIT_COMMITTER_DATE", date);
    }
    assert!(
        cmd.output().unwrap().status.success(),
        "git {args:?} failed"
    );
}

/// Two repos and one gone folder in each: sessions of `a` and `b` land in the projects `alpha` and `beta`.
fn two_projects(db: &Db) -> (String, String) {
    let alpha = db.repo("alpha", None);
    let beta = db.repo("beta", None);
    (format!("{alpha}/gone1"), format!("{beta}/gone2"))
}

struct Case {
    name: &'static str,
    rules: &'static str,
    build: fn(&mut Db),
}

/// `rule id -> {hit, abstained}` for the rules under test, taken from an `evaluate` answer.
fn view(out: &Value, ids: &str) -> Value {
    let find = |list: &str, id: &str| {
        out[list]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["rule_id"] == id)
            .cloned()
            .unwrap_or(Value::Null)
    };
    let mut map = serde_json::Map::new();
    for id in ids.split(',') {
        map.insert(
            id.to_string(),
            json!({"hit": find("hits", id), "abstained": find("abstained", id)}),
        );
    }
    Value::Object(map)
}

fn run(cases: &[Case]) {
    for case in cases {
        let mut db = Db::new(case.name);
        (case.build)(&mut db);
        let path = db.dir.0.join("store.sqlite");
        let Db { dir, conn, .. } = db;
        drop(conn);
        let conn = open_read_only(&path).unwrap();
        let got = view(&warnings(&conn), case.rules);
        drop(conn);
        if std::env::var_os("T006_KEEP").is_some() {
            drop(dir);
            continue;
        }
        let want: Value = serde_json::from_str(expected(case.name))
            .unwrap_or_else(|_| panic!("{}: no expected JSON", case.name));
        assert_eq!(got, want, "case {}", case.name);
        drop(dir);
    }
}

// cache_hit_low

fn cache_switching(db: &mut Db) {
    let (a, b) = two_projects(db);
    db.msg(W1, "s1", &a, "m1", [100, 10, 50, 50]);
    db.msg(W2, "s1", &a, "m1", [100, 10, 50, 50]);
    db.msg(W1, "s2", &b, "m1", [100, 10, 100, 0]);
    db.msg(W2, "s2", &b, "m2", [100, 10, 100, 0]);
}

fn cache_unstable(db: &mut Db) {
    let (a, b) = two_projects(db);
    db.msg(W1, "s1", &a, "m1", [100, 10, 50, 50]);
    db.msg(W1, "s2", &b, "m1", [100, 10, 100, 0]);
    db.msg(W2, "s2", &b, "m1", [100, 10, 100, 0]);
}

fn cache_switch_equal(db: &mut Db) {
    // the multi-model session reuses the cache as much as the others: not "model switching"
    let (a, b) = two_projects(db);
    db.msg(W1, "s1", &a, "m1", [100, 10, 50, 50]);
    db.msg(W1, "s2", &b, "m1", [100, 10, 50, 50]);
    db.msg(W2, "s2", &b, "m2", [100, 10, 50, 50]);
}

fn cache_edge_message(db: &mut Db) {
    // One message has records on both sides of the window start. Python reads it by its first record, which is
    // before the window, so it is left out; its big cache read must not count.
    let (a, _) = two_projects(db);
    db.msg(W1, "s1", &a, "m1", [100, 10, 1000, 100]);
    for ts in ["2026-01-12T23:00:00.000000Z", "2026-01-13T01:00:00.000000Z"] {
        db.ev(Ev {
            ts,
            session: Some("s2"),
            project: Some(&a),
            model: Some("m1"),
            message: Some("edge".into()),
            tokens: Some([0, 0, 0, 100_000]),
            ..Ev::default()
        });
    }
}

fn cache_history_exact(db: &mut Db) {
    // exactly 7 days of history is enough for a 7-day rule
    let (a, _) = two_projects(db);
    db.history_at("2026-01-13T00:00:00.000000Z");
    db.msg(W1, "s1", &a, "m1", [100, 10, 100, 0]);
}

fn cache_history_short(db: &mut Db) {
    let (a, _) = two_projects(db);
    db.history_at("2026-01-13T00:00:00.000001Z");
    db.msg(W1, "s1", &a, "m1", [100, 10, 100, 0]);
}

fn cache_just_below(db: &mut Db) {
    let (a, _) = two_projects(db);
    db.msg(W1, "s1", &a, "m1", [0, 0, 5001, 4999]);
}

fn cache_boundary(db: &mut Db) {
    let (a, _) = two_projects(db);
    db.msg(W1, "s1", &a, "m1", [0, 0, 5000, 5000]);
}

#[test]
fn cache_hit_low_matches_python() {
    run(&[
        Case {
            name: "cache_switching",
            rules: "cache_hit_low",
            build: cache_switching,
        },
        Case {
            name: "cache_unstable",
            rules: "cache_hit_low",
            build: cache_unstable,
        },
        Case {
            name: "cache_switch_equal",
            rules: "cache_hit_low",
            build: cache_switch_equal,
        },
        Case {
            name: "cache_edge_message",
            rules: "cache_hit_low",
            build: cache_edge_message,
        },
        Case {
            name: "cache_history_exact",
            rules: "cache_hit_low",
            build: cache_history_exact,
        },
        Case {
            name: "cache_history_short",
            rules: "cache_hit_low",
            build: cache_history_short,
        },
        Case {
            name: "cache_just_below",
            rules: "cache_hit_low",
            build: cache_just_below,
        },
        Case {
            name: "cache_boundary",
            rules: "cache_hit_low",
            build: cache_boundary,
        },
    ]);
}

// session_huge_no_compact

fn ctx_odd(db: &mut Db) {
    let (a, b) = two_projects(db);
    db.msg(W1, "s1", &a, "m1", [160_000, 5, 0, 0]);
    db.msg(W1, "s2", &b, "m1", [200_000, 5, 0, 0]);
    db.msg(W2, "s3", &a, "m1", [170_000, 5, 0, 0]);
}

fn ctx_even_tie(db: &mut Db) {
    let (a, b) = two_projects(db);
    db.msg(W1, "s1", &a, "m1", [150_001, 5, 0, 0]);
    db.msg(W1, "s2", &b, "m1", [150_002, 5, 0, 0]);
}

fn ctx_half_exact(db: &mut Db) {
    // 200k then exactly 100k: not below half of the peak, so the session is open and counts
    let (a, b) = two_projects(db);
    db.msg(W1, "s1", &a, "m1", [200_000, 5, 0, 0]);
    db.msg(W2, "s1", &a, "m1", [100_000, 5, 0, 0]);
    db.msg(W1, "s2", &b, "m1", [160_000, 5, 0, 0]);
}

fn ctx_ok_compacted(db: &mut Db) {
    let (a, b) = two_projects(db);
    db.msg(W1, "s1", &a, "m1", [200_000, 5, 0, 0]);
    db.msg(W2, "s1", &a, "m1", [90_000, 5, 0, 0]); // below half of the peak: compacted, so not counted
    db.msg(W1, "s2", &b, "m1", [100_000, 5, 0, 0]);
}

#[test]
fn session_huge_no_compact_matches_python() {
    run(&[
        Case {
            name: "ctx_odd",
            rules: "session_huge_no_compact",
            build: ctx_odd,
        },
        Case {
            name: "ctx_even_tie",
            rules: "session_huge_no_compact",
            build: ctx_even_tie,
        },
        Case {
            name: "ctx_half_exact",
            rules: "session_huge_no_compact",
            build: ctx_half_exact,
        },
        Case {
            name: "ctx_ok_compacted",
            rules: "session_huge_no_compact",
            build: ctx_ok_compacted,
        },
    ]);
}

// tokens_jump

fn jump_bigger(db: &mut Db) {
    let (a, _) = two_projects(db);
    db.msg(P1, "s1", &a, "m1", [1000, 0, 0, 0]);
    db.msg(W1, "s1", &a, "m1", [3000, 0, 0, 0]);
}

fn jump_more_sessions(db: &mut Db) {
    let (a, b) = two_projects(db);
    db.msg(P1, "s1", &a, "m1", [1000, 0, 0, 0]);
    db.msg(P1, "s2", &a, "m1", [1000, 0, 0, 0]);
    db.msg(W1, "s1", &a, "m1", [1000, 0, 0, 0]);
    db.msg(W1, "s2", &a, "m1", [1000, 0, 0, 0]);
    db.msg(W1, "s3", &b, "m1", [1000, 0, 0, 0]);
    db.msg(W1, "s4", &b, "m1", [1000, 0, 0, 0]);
}

fn jump_bigger_mild(db: &mut Db) {
    // tokens +160%; tokens per session up 30%: above the 10% line, so "bigger sessions" (not "more sessions")
    let (a, b) = two_projects(db);
    db.msg(P1, "s1", &a, "m1", [1000, 0, 0, 0]);
    db.msg(W1, "s1", &a, "m1", [1300, 0, 0, 0]);
    db.msg(W1, "s2", &b, "m1", [1300, 0, 0, 0]);
}

fn jump_boundary(db: &mut Db) {
    let (a, _) = two_projects(db);
    db.msg(P1, "s1", &a, "m1", [2000, 0, 0, 0]);
    db.msg(W1, "s1", &a, "m1", [3000, 0, 0, 0]); // exactly +50%: not above the threshold
}

fn jump_fell(db: &mut Db) {
    let (a, _) = two_projects(db);
    db.msg(P1, "s1", &a, "m1", [3000, 0, 0, 0]);
    db.msg(W1, "s1", &a, "m1", [1000, 0, 0, 0]);
}

#[test]
fn tokens_jump_matches_python() {
    run(&[
        Case {
            name: "jump_bigger",
            rules: "tokens_jump",
            build: jump_bigger,
        },
        Case {
            name: "jump_bigger_mild",
            rules: "tokens_jump",
            build: jump_bigger_mild,
        },
        Case {
            name: "jump_more_sessions",
            rules: "tokens_jump",
            build: jump_more_sessions,
        },
        Case {
            name: "jump_boundary",
            rules: "tokens_jump",
            build: jump_boundary,
        },
        Case {
            name: "jump_fell",
            rules: "tokens_jump",
            build: jump_fell,
        },
    ]);
}

// project_dominates

fn dominates_fires(db: &mut Db) {
    let (a, b) = two_projects(db);
    db.msg(W1, "sa1", &a, "m1", [4000, 0, 0, 0]);
    db.msg(W2, "sa2", &a, "m1", [3000, 0, 0, 0]);
    db.msg(W1, "sb1", &b, "m1", [2000, 0, 0, 0]);
}

fn dominates_boundary(db: &mut Db) {
    let (a, b) = two_projects(db);
    db.msg(W1, "sa1", &a, "m1", [6000, 0, 0, 0]);
    db.msg(W1, "sb1", &b, "m1", [4000, 0, 0, 0]);
}

fn dominates_single(db: &mut Db) {
    let (a, _) = two_projects(db);
    db.msg(W1, "sa1", &a, "m1", [6000, 0, 0, 0]);
}

#[test]
fn project_dominates_matches_python() {
    run(&[
        Case {
            name: "dominates_fires",
            rules: "project_dominates",
            build: dominates_fires,
        },
        Case {
            name: "dominates_boundary",
            rules: "project_dominates",
            build: dominates_boundary,
        },
        Case {
            name: "dominates_single",
            rules: "project_dominates",
            build: dominates_single,
        },
    ]);
}

// tool_errors_high

fn errors_one_tool(db: &mut Db) {
    let (a, _) = two_projects(db);
    for i in 0..3 {
        db.trace(Some("Bash"), &a, i64::from(i < 2));
    }
    for _ in 0..7 {
        db.trace(Some("Read"), &a, 0);
    }
}

fn errors_spread(db: &mut Db) {
    let (a, b) = two_projects(db);
    db.msg(W1, "s1", &a, "m1", [10, 0, 0, 0]); // makes both folders known, so traces group under their repos
    db.msg(W1, "s2", &b, "m1", [10, 0, 0, 0]);
    for i in 0..4 {
        db.trace(Some("Bash"), &a, i64::from(i == 0));
        db.trace(Some("Read"), &a, i64::from(i == 0));
    }
    db.trace(None, &b, 1);
    db.trace(None, &b, 0);
}

fn errors_even_split(db: &mut Db) {
    // two failed calls, one per tool: the top tool has half, not more than half, so the causes are projects
    let (a, _) = two_projects(db);
    db.msg(W1, "s1", &a, "m1", [10, 0, 0, 0]);
    db.trace(Some("Bash"), &a, 1);
    db.trace(Some("Read"), &a, 1);
    for _ in 0..8 {
        db.trace(Some("Edit"), &a, 0);
    }
}

fn errors_boundary(db: &mut Db) {
    let (a, _) = two_projects(db);
    db.trace(Some("Bash"), &a, 1);
    for _ in 0..9 {
        db.trace(Some("Bash"), &a, 0);
    }
}

#[test]
fn tool_errors_high_matches_python() {
    run(&[
        Case {
            name: "errors_one_tool",
            rules: "tool_errors_high",
            build: errors_one_tool,
        },
        Case {
            name: "errors_spread",
            rules: "tool_errors_high",
            build: errors_spread,
        },
        Case {
            name: "errors_even_split",
            rules: "tool_errors_high",
            build: errors_even_split,
        },
        Case {
            name: "errors_boundary",
            rules: "tool_errors_high",
            build: errors_boundary,
        },
    ]);
}

// context_index_stale

const COMMIT_DATE: &str = "2026-01-16T00:00:00Z";

fn in_use(db: &mut Db) {
    db.ev(Ev {
        ts: W1,
        tools: Some("Read,mcp__easy-context__search_code"),
        ..Ev::default()
    });
}

fn stale_fires(db: &mut Db) {
    in_use(db);
    let stale = db.repo("stale-repo", Some(COMMIT_DATE));
    let second = db.repo("stale-two", Some("2026-01-17T00:00:00Z"));
    let fresh = db.repo("fresh-repo", Some("2026-01-05T00:00:00Z"));
    db.index(&stale, "2026-01-10T00:00:00+00:00");
    db.index(&second, "2026-01-10T00:00:00+00:00");
    db.index(&fresh, "2026-01-10T00:00:00+00:00");
}

fn stale_clean(db: &mut Db) {
    in_use(db);
    let fresh = db.repo("fresh-repo", Some("2026-01-05T00:00:00Z"));
    db.index(&fresh, "2026-01-10T00:00:00+00:00");
}

fn stale_no_index(db: &mut Db) {
    in_use(db);
}

#[test]
fn context_index_stale_matches_python() {
    run(&[
        Case {
            name: "stale_fires",
            rules: "context_index_stale",
            build: stale_fires,
        },
        Case {
            name: "stale_clean",
            rules: "context_index_stale",
            build: stale_clean,
        },
        Case {
            name: "stale_no_index",
            rules: "context_index_stale",
            build: stale_no_index,
        },
    ]);
}

// records_unreadable

fn unreadable_mixed(db: &mut Db) {
    db.records(Some("2.1.1"), Some("x"), 50, 2);
    db.records(None, None, 10, 5);
}

fn unreadable_boundary(db: &mut Db) {
    db.records(Some("2.1.1"), Some("x"), 100, 2); // exactly 2%: not above the threshold
}

fn unreadable_tie(db: &mut Db) {
    db.records(Some("2.1.0"), Some("b"), 8, 2);
    db.records(Some("2.0.0"), Some("a"), 4, 1);
    db.records(Some("1.9.0"), Some("c"), 5, 1); // 0.2: lower than the tie at 0.25, which keeps the first version
}

#[test]
fn records_unreadable_matches_python() {
    run(&[
        Case {
            name: "unreadable_mixed",
            rules: "records_unreadable",
            build: unreadable_mixed,
        },
        Case {
            name: "unreadable_boundary",
            rules: "records_unreadable",
            build: unreadable_boundary,
        },
        Case {
            name: "unreadable_tie",
            rules: "records_unreadable",
            build: unreadable_tie,
        },
    ]);
}

// no data in the window: every rule abstains with its reason; the kit rule that is not in use is left out

#[test]
fn an_empty_window_abstains_with_python_reasons() {
    run(&[Case {
        name: "empty_window",
        rules: "cache_hit_low,session_huge_no_compact,records_unreadable,tokens_jump,project_dominates,tool_errors_high,context_index_stale",
        build: |_| {},
    }]);
}

// EXPECTED-BEGIN
// Python's answers for the case stores above (command at the top of section 5). Regenerate after a case changes.
fn expected(name: &str) -> &'static str {
    match name {
        "cache_boundary" => r##"{"cache_hit_low": {"abstained": null, "hit": null}}"##,
        "cache_edge_message" => {
            r##"{"cache_hit_low": {"abstained": null, "hit": {"cache_read_share": 0.0826, "causes": [{"cache_reuse": 0.0833, "kind": "project", "name": "alpha", "text": "alpha read 8.3% of its prompt tokens from cache (100% of all tokens)", "token_share": 1.0, "tokens": 1210}], "diagnosis": "unstable_prefix", "fix": "Lowest cache reuse by project: alpha (8.3% of its prompt tokens came from cache). Keep the start of your prompts the same from one turn to the next. Avoid editing CLAUDE.md or switching models in the middle of a session.", "label": "warning", "measured": 0.0833, "priority": 1, "rule_id": "cache_hit_low", "steps": ["Lowest cache reuse by project: alpha (8.3% of its prompt tokens came from cache).", "Keep the start of your prompts the same from one turn to the next.", "Avoid editing CLAUDE.md or switching models in the middle of a session."], "summary": "8.3% of prompt tokens came from cache; target 50% or more, last 7 days", "threshold": 0.5, "title": "Low cache reuse", "why": "A prompt read from cache costs far less and returns faster, so a low share means you pay full price for text Claude has already seen.", "window_days": 7}}}"##
        }
        "cache_history_exact" => {
            r##"{"cache_hit_low": {"abstained": null, "hit": {"cache_read_share": 0.0, "causes": [{"cache_reuse": 0.0, "kind": "project", "name": "alpha", "text": "alpha read 0% of its prompt tokens from cache (100% of all tokens)", "token_share": 1.0, "tokens": 210}], "diagnosis": "unstable_prefix", "fix": "Lowest cache reuse by project: alpha (0% of its prompt tokens came from cache). Keep the start of your prompts the same from one turn to the next. Avoid editing CLAUDE.md or switching models in the middle of a session.", "label": "warning", "measured": 0.0, "priority": 1, "rule_id": "cache_hit_low", "steps": ["Lowest cache reuse by project: alpha (0% of its prompt tokens came from cache).", "Keep the start of your prompts the same from one turn to the next.", "Avoid editing CLAUDE.md or switching models in the middle of a session."], "summary": "0% of prompt tokens came from cache; target 50% or more, last 7 days", "threshold": 0.5, "title": "Low cache reuse", "why": "A prompt read from cache costs far less and returns faster, so a low share means you pay full price for text Claude has already seen.", "window_days": 7}}}"##
        }
        "cache_history_short" => {
            r##"{"cache_hit_low": {"abstained": {"reason": "less than 7 days of stored history", "rule_id": "cache_hit_low", "title": "Low cache reuse"}, "hit": null}}"##
        }
        "cache_just_below" => {
            r##"{"cache_hit_low": {"abstained": null, "hit": {"cache_read_share": 0.4999, "causes": [{"cache_reuse": 0.4999, "kind": "project", "name": "alpha", "text": "alpha read 50% of its prompt tokens from cache (100% of all tokens)", "token_share": 1.0, "tokens": 10000}], "diagnosis": "unstable_prefix", "fix": "Lowest cache reuse by project: alpha (50% of its prompt tokens came from cache). Keep the start of your prompts the same from one turn to the next. Avoid editing CLAUDE.md or switching models in the middle of a session.", "label": "warning", "measured": 0.4999, "priority": 1, "rule_id": "cache_hit_low", "steps": ["Lowest cache reuse by project: alpha (50% of its prompt tokens came from cache).", "Keep the start of your prompts the same from one turn to the next.", "Avoid editing CLAUDE.md or switching models in the middle of a session."], "summary": "50% of prompt tokens came from cache; target 50% or more, last 7 days", "threshold": 0.5, "title": "Low cache reuse", "why": "A prompt read from cache costs far less and returns faster, so a low share means you pay full price for text Claude has already seen.", "window_days": 7}}}"##
        }
        "cache_switch_equal" => {
            r##"{"cache_hit_low": {"abstained": null, "hit": {"cache_read_share": 0.2381, "causes": [{"cache_reuse": 0.25, "kind": "project", "name": "alpha", "text": "alpha read 25% of its prompt tokens from cache (33.3% of all tokens)", "token_share": 0.3333, "tokens": 210}, {"cache_reuse": 0.25, "kind": "project", "name": "beta", "text": "beta read 25% of its prompt tokens from cache (66.7% of all tokens)", "token_share": 0.6667, "tokens": 420}], "diagnosis": "unstable_prefix", "fix": "Lowest cache reuse by project: alpha (25% of its prompt tokens came from cache). Keep the start of your prompts the same from one turn to the next. Avoid editing CLAUDE.md or switching models in the middle of a session.", "label": "warning", "measured": 0.25, "priority": 1, "rule_id": "cache_hit_low", "steps": ["Lowest cache reuse by project: alpha (25% of its prompt tokens came from cache).", "Keep the start of your prompts the same from one turn to the next.", "Avoid editing CLAUDE.md or switching models in the middle of a session."], "summary": "25% of prompt tokens came from cache; target 50% or more, last 7 days", "threshold": 0.5, "title": "Low cache reuse", "why": "A prompt read from cache costs far less and returns faster, so a low share means you pay full price for text Claude has already seen.", "window_days": 7}}}"##
        }
        "cache_switching" => {
            r##"{"cache_hit_low": {"abstained": null, "hit": {"cache_read_share": 0.119, "causes": [{"cache_reuse": 0.0, "kind": "session", "models": 2, "name": "s2", "project": "beta", "text": "s2 in beta used 2 models and read 0% of its prompt tokens from cache", "tokens": 420}, {"cache_reuse": 0.0, "kind": "project", "name": "beta", "text": "beta read 0% of its prompt tokens from cache (50% of all tokens)", "token_share": 0.5, "tokens": 420}, {"cache_reuse": 0.25, "kind": "project", "name": "alpha", "text": "alpha read 25% of its prompt tokens from cache (50% of all tokens)", "token_share": 0.5, "tokens": 420}], "diagnosis": "model_switching", "fix": "1 sessions used 2 or more models; their cache reuse is 0% against 25% in the others. Use one model for a whole session: switching model in the middle restarts the cache. Biggest case: s2 in beta. Lowest reuse by project: beta (0%).", "label": "warning", "measured": 0.125, "priority": 1, "rule_id": "cache_hit_low", "steps": ["1 sessions used 2 or more models; their cache reuse is 0% against 25% in the others.", "Use one model for a whole session: switching model in the middle restarts the cache. Biggest case: s2 in beta.", "Lowest reuse by project: beta (0%)."], "summary": "12.5% of prompt tokens came from cache; target 50% or more, last 7 days", "threshold": 0.5, "title": "Low cache reuse", "why": "A prompt read from cache costs far less and returns faster, so a low share means you pay full price for text Claude has already seen.", "window_days": 7}}}"##
        }
        "cache_unstable" => {
            r##"{"cache_hit_low": {"abstained": null, "hit": {"cache_read_share": 0.0794, "causes": [{"cache_reuse": 0.0, "kind": "project", "name": "beta", "text": "beta read 0% of its prompt tokens from cache (66.7% of all tokens)", "token_share": 0.6667, "tokens": 420}, {"cache_reuse": 0.25, "kind": "project", "name": "alpha", "text": "alpha read 25% of its prompt tokens from cache (33.3% of all tokens)", "token_share": 0.3333, "tokens": 210}], "diagnosis": "unstable_prefix", "fix": "Lowest cache reuse by project: beta (0% of its prompt tokens came from cache). Keep the start of your prompts the same from one turn to the next. Avoid editing CLAUDE.md or switching models in the middle of a session.", "label": "warning", "measured": 0.0833, "priority": 1, "rule_id": "cache_hit_low", "steps": ["Lowest cache reuse by project: beta (0% of its prompt tokens came from cache).", "Keep the start of your prompts the same from one turn to the next.", "Avoid editing CLAUDE.md or switching models in the middle of a session."], "summary": "8.3% of prompt tokens came from cache; target 50% or more, last 7 days", "threshold": 0.5, "title": "Low cache reuse", "why": "A prompt read from cache costs far less and returns faster, so a low share means you pay full price for text Claude has already seen.", "window_days": 7}}}"##
        }
        "ctx_even_tie" => {
            r##"{"session_huge_no_compact": {"abstained": null, "hit": {"cache_read_share": 0.0, "causes": [{"context_tokens": 150002, "kind": "session", "name": "s2", "project": "beta", "text": "s2 in beta reached 150.0k tokens of context without compacting"}, {"context_tokens": 150001, "kind": "session", "name": "s1", "project": "alpha", "text": "s1 in alpha reached 150.0k tokens of context without compacting"}], "diagnosis": "huge_sessions", "fix": "The largest open session is s2 in beta: 150.0k tokens of context. Run /compact when a session grows long. Start a new session before the context passes 150,000 tokens.", "label": "improve", "measured": 150001.5, "priority": 2, "rule_id": "session_huge_no_compact", "steps": ["The largest open session is s2 in beta: 150.0k tokens of context.", "Run /compact when a session grows long.", "Start a new session before the context passes 150,000 tokens."], "summary": "The median open session reached 150,002 tokens of context; target 150,000 or fewer, last 7 days", "threshold": 150000, "title": "Sessions grow too large", "why": "Every turn re-sends the whole conversation, so a very long session makes each reply slower and more expensive.", "window_days": 7}}}"##
        }
        "ctx_half_exact" => {
            r##"{"session_huge_no_compact": {"abstained": null, "hit": {"cache_read_share": 0.0, "causes": [{"context_tokens": 200000, "kind": "session", "name": "s1", "project": "alpha", "text": "s1 in alpha reached 200.0k tokens of context without compacting"}, {"context_tokens": 160000, "kind": "session", "name": "s2", "project": "beta", "text": "s2 in beta reached 160.0k tokens of context without compacting"}], "diagnosis": "huge_sessions", "fix": "The largest open session is s1 in alpha: 200.0k tokens of context. Run /compact when a session grows long. Start a new session before the context passes 150,000 tokens.", "label": "improve", "measured": 180000.0, "priority": 2, "rule_id": "session_huge_no_compact", "steps": ["The largest open session is s1 in alpha: 200.0k tokens of context.", "Run /compact when a session grows long.", "Start a new session before the context passes 150,000 tokens."], "summary": "The median open session reached 180,000 tokens of context; target 150,000 or fewer, last 7 days", "threshold": 150000, "title": "Sessions grow too large", "why": "Every turn re-sends the whole conversation, so a very long session makes each reply slower and more expensive.", "window_days": 7}}}"##
        }
        "ctx_odd" => {
            r##"{"session_huge_no_compact": {"abstained": null, "hit": {"cache_read_share": 0.0, "causes": [{"context_tokens": 200000, "kind": "session", "name": "s2", "project": "beta", "text": "s2 in beta reached 200.0k tokens of context without compacting"}, {"context_tokens": 170000, "kind": "session", "name": "s3", "project": "alpha", "text": "s3 in alpha reached 170.0k tokens of context without compacting"}, {"context_tokens": 160000, "kind": "session", "name": "s1", "project": "alpha", "text": "s1 in alpha reached 160.0k tokens of context without compacting"}], "diagnosis": "huge_sessions", "fix": "The largest open session is s2 in beta: 200.0k tokens of context. Run /compact when a session grows long. Start a new session before the context passes 150,000 tokens.", "label": "improve", "measured": 170000, "priority": 2, "rule_id": "session_huge_no_compact", "steps": ["The largest open session is s2 in beta: 200.0k tokens of context.", "Run /compact when a session grows long.", "Start a new session before the context passes 150,000 tokens."], "summary": "The median open session reached 170,000 tokens of context; target 150,000 or fewer, last 7 days", "threshold": 150000, "title": "Sessions grow too large", "why": "Every turn re-sends the whole conversation, so a very long session makes each reply slower and more expensive.", "window_days": 7}}}"##
        }
        "ctx_ok_compacted" => r##"{"session_huge_no_compact": {"abstained": null, "hit": null}}"##,
        "dominates_boundary" => r##"{"project_dominates": {"abstained": null, "hit": null}}"##,
        "dominates_fires" => {
            r##"{"project_dominates": {"abstained": null, "hit": {"cache_read_share": 0.0, "causes": [{"kind": "session", "name": "sa1", "project": "alpha", "text": "sa1 in alpha used 4.0k tokens", "token_share": 0.4444, "tokens": 4000}, {"kind": "session", "name": "sa2", "project": "alpha", "text": "sa2 in alpha used 3.0k tokens", "token_share": 0.3333, "tokens": 3000}], "detail": {"project": "alpha"}, "diagnosis": "dominant_project", "fix": "alpha used 77.8% of tokens; its largest session, sa1, alone used 44.4% of all tokens. Confirm that this project should take most of your effort. Split long-running work into shorter sessions.", "label": "aware", "measured": 0.7778, "priority": 3, "rule_id": "project_dominates", "steps": ["alpha used 77.8% of tokens; its largest session, sa1, alone used 44.4% of all tokens.", "Confirm that this project should take most of your effort.", "Split long-running work into shorter sessions."], "summary": "One project (alpha) used 77.8% of tokens; target 60% or less, last 7 days", "threshold": 0.6, "title": "One project uses most of your tokens", "why": "A single project that crowds out the rest can hide waste, and your limit runs out sooner.", "window_days": 7}}}"##
        }
        "dominates_single" => {
            r##"{"project_dominates": {"abstained": {"reason": "fewer than two projects with tokens in the window", "rule_id": "project_dominates", "title": "One project uses most of your tokens"}, "hit": null}}"##
        }
        "empty_window" => {
            r##"{"cache_hit_low": {"abstained": {"reason": "no prompt tokens in the window", "rule_id": "cache_hit_low", "title": "Low cache reuse"}, "hit": null}, "context_index_stale": {"abstained": null, "hit": null}, "project_dominates": {"abstained": {"reason": "fewer than two projects with tokens in the window", "rule_id": "project_dominates", "title": "One project uses most of your tokens"}, "hit": null}, "records_unreadable": {"abstained": {"reason": "no records in the window", "rule_id": "records_unreadable", "title": "Some Claude Code records are unreadable"}, "hit": null}, "session_huge_no_compact": {"abstained": {"reason": "no sessions in the window", "rule_id": "session_huge_no_compact", "title": "Sessions grow too large"}, "hit": null}, "tokens_jump": {"abstained": {"reason": "no tokens in the previous window", "rule_id": "tokens_jump", "title": "Token use jumped"}, "hit": null}, "tool_errors_high": {"abstained": {"reason": "no kernel-trace tool calls in the window", "rule_id": "tool_errors_high", "title": "Many tool calls fail"}, "hit": null}}"##
        }
        "errors_boundary" => r##"{"tool_errors_high": {"abstained": null, "hit": null}}"##,
        "errors_even_split" => {
            r##"{"tool_errors_high": {"abstained": null, "hit": {"causes": [{"calls": 10, "error_rate": 0.2, "errors": 2, "kind": "project", "name": "alpha", "text": "alpha: 2 of 10 calls failed (20%)"}], "diagnosis": "errors_spread", "fix": "Failures are spread over many tools; alpha has the most failed calls: 2 of 2. Read the failing tool calls of alpha in memory/event-trace/. Fix the command or permission that keeps failing.", "label": "warning", "measured": 0.2, "priority": 2, "rule_id": "tool_errors_high", "steps": ["Failures are spread over many tools; alpha has the most failed calls: 2 of 2.", "Read the failing tool calls of alpha in memory/event-trace/.", "Fix the command or permission that keeps failing."], "summary": "20% of kernel-trace tool calls failed; target 10% or less, last 7 days", "threshold": 0.1, "title": "Many tool calls fail", "why": "Failed tool calls waste tokens and time because Claude retries and explains the error.", "window_days": 7}}}"##
        }
        "errors_one_tool" => {
            r##"{"tool_errors_high": {"abstained": null, "hit": {"causes": [{"calls": 3, "error_rate": 0.6667, "errors": 2, "kind": "tool", "name": "Bash", "text": "Bash: 2 of 3 calls failed (66.7%)"}], "diagnosis": "one_tool_fails", "fix": "Bash caused 2 of the 2 failed calls (100%). Read the failing Bash calls in memory/event-trace/ and fix the command or permission that keeps failing.", "label": "warning", "measured": 0.2, "priority": 2, "rule_id": "tool_errors_high", "steps": ["Bash caused 2 of the 2 failed calls (100%).", "Read the failing Bash calls in memory/event-trace/ and fix the command or permission that keeps failing."], "summary": "20% of kernel-trace tool calls failed; target 10% or less, last 7 days", "threshold": 0.1, "title": "Many tool calls fail", "why": "Failed tool calls waste tokens and time because Claude retries and explains the error.", "window_days": 7}}}"##
        }
        "errors_spread" => {
            r##"{"tool_errors_high": {"abstained": null, "hit": {"causes": [{"calls": 8, "error_rate": 0.25, "errors": 2, "kind": "project", "name": "alpha", "text": "alpha: 2 of 8 calls failed (25%)"}, {"calls": 2, "error_rate": 0.5, "errors": 1, "kind": "project", "name": "beta", "text": "beta: 1 of 2 calls failed (50%)"}], "diagnosis": "errors_spread", "fix": "Failures are spread over many tools; alpha has the most failed calls: 2 of 3. Read the failing tool calls of alpha in memory/event-trace/. Fix the command or permission that keeps failing.", "label": "warning", "measured": 0.3, "priority": 2, "rule_id": "tool_errors_high", "steps": ["Failures are spread over many tools; alpha has the most failed calls: 2 of 3.", "Read the failing tool calls of alpha in memory/event-trace/.", "Fix the command or permission that keeps failing."], "summary": "30% of kernel-trace tool calls failed; target 10% or less, last 7 days", "threshold": 0.1, "title": "Many tool calls fail", "why": "Failed tool calls waste tokens and time because Claude retries and explains the error.", "window_days": 7}}}"##
        }
        "jump_bigger" => {
            r##"{"tokens_jump": {"abstained": null, "hit": {"cache_read_share": 0.0, "causes": [{"kind": "session", "name": "s1", "project": "alpha", "text": "s1 in alpha used 3.0k tokens", "token_share": 1.0, "tokens": 3000}], "diagnosis": "bigger_sessions", "fix": "Each session got bigger: tokens per session went from 1.0k to 3.0k (1 → 1 sessions). The largest session, s1 in alpha, used 3.0k tokens. Run /compact in long sessions or start a new one. Decide if the rise is intended before it becomes your baseline.", "label": "aware", "measured": 2.0, "priority": 3, "rule_id": "tokens_jump", "steps": ["Each session got bigger: tokens per session went from 1.0k to 3.0k (1 → 1 sessions).", "The largest session, s1 in alpha, used 3.0k tokens. Run /compact in long sessions or start a new one.", "Decide if the rise is intended before it becomes your baseline."], "summary": "Token use rose 200% in the last 7 days against the 7 days before; target 50% or less", "threshold": 0.5, "title": "Token use jumped", "why": "A sudden rise is easy to miss and quickly becomes the new normal, along with its cost.", "window_days": 7}}}"##
        }
        "jump_bigger_mild" => {
            r##"{"tokens_jump": {"abstained": null, "hit": {"cache_read_share": 0.0, "causes": [{"kind": "session", "name": "s1", "project": "alpha", "text": "s1 in alpha used 1.3k tokens", "token_share": 0.5, "tokens": 1300}, {"kind": "session", "name": "s2", "project": "beta", "text": "s2 in beta used 1.3k tokens", "token_share": 0.5, "tokens": 1300}], "diagnosis": "bigger_sessions", "fix": "Each session got bigger: tokens per session went from 1.0k to 1.3k (1 → 2 sessions). The largest session, s1 in alpha, used 1.3k tokens. Run /compact in long sessions or start a new one. Decide if the rise is intended before it becomes your baseline.", "label": "aware", "measured": 1.6, "priority": 3, "rule_id": "tokens_jump", "steps": ["Each session got bigger: tokens per session went from 1.0k to 1.3k (1 → 2 sessions).", "The largest session, s1 in alpha, used 1.3k tokens. Run /compact in long sessions or start a new one.", "Decide if the rise is intended before it becomes your baseline."], "summary": "Token use rose 160% in the last 7 days against the 7 days before; target 50% or less", "threshold": 0.5, "title": "Token use jumped", "why": "A sudden rise is easy to miss and quickly becomes the new normal, along with its cost.", "window_days": 7}}}"##
        }
        "jump_boundary" => r##"{"tokens_jump": {"abstained": null, "hit": null}}"##,
        "jump_fell" => r##"{"tokens_jump": {"abstained": null, "hit": null}}"##,
        "jump_more_sessions" => {
            r##"{"tokens_jump": {"abstained": null, "hit": {"cache_read_share": 0.0, "causes": [{"kind": "project", "name": "beta", "new": true, "sessions_before": 0, "sessions_now": 2, "text": "beta: 0 → 2.0k tokens, 0 → 2 sessions (new)", "tokens_before": 0, "tokens_now": 2000}], "diagnosis": "more_sessions", "fix": "The rise comes from more sessions (2 → 4), not bigger ones: tokens per session went from 1.0k to 1.0k. New since the period before: beta. Check that it should run this often. Biggest rise: beta (+2.0k tokens). Decide if that pace is intended before it becomes your baseline.", "label": "aware", "measured": 1.0, "priority": 3, "rule_id": "tokens_jump", "steps": ["The rise comes from more sessions (2 → 4), not bigger ones: tokens per session went from 1.0k to 1.0k.", "New since the period before: beta. Check that it should run this often.", "Biggest rise: beta (+2.0k tokens). Decide if that pace is intended before it becomes your baseline."], "summary": "Token use rose 100% in the last 7 days against the 7 days before; target 50% or less", "threshold": 0.5, "title": "Token use jumped", "why": "A sudden rise is easy to miss and quickly becomes the new normal, along with its cost.", "window_days": 7}}}"##
        }
        "stale_clean" => r##"{"context_index_stale": {"abstained": null, "hit": null}}"##,
        "stale_fires" => {
            r##"{"context_index_stale": {"abstained": null, "hit": {"causes": [{"kind": "project", "last_index_at": "2026-01-10T00:00:00+00:00", "latest_commit_utc": "2026-01-16T00:00:00+00:00", "name": "stale-repo", "text": "stale-repo: indexed 2026-01-10T00:00:00+00:00, latest commit 2026-01-16T00:00:00+00:00"}, {"kind": "project", "last_index_at": "2026-01-10T00:00:00+00:00", "latest_commit_utc": "2026-01-17T00:00:00+00:00", "name": "stale-two", "text": "stale-two: indexed 2026-01-10T00:00:00+00:00, latest commit 2026-01-17T00:00:00+00:00"}], "detail": {"projects": ["stale-repo", "stale-two"]}, "diagnosis": "stale_index", "fix": "The index is older than the latest commit in: stale-repo, stale-two (2 in all). Call `index_path` in easy-context for the listed projects. You run it yourself; easy-personal never does.", "label": "improve", "measured": 2, "priority": 3, "rule_id": "context_index_stale", "steps": ["The index is older than the latest commit in: stale-repo, stale-two (2 in all).", "Call `index_path` in easy-context for the listed projects.", "You run it yourself; easy-personal never does."], "summary": "2 projects have a stale easy-context index; target none, last 7 days", "threshold": 0, "title": "Code index is out of date", "why": "Search in easy-context returns old results while its index is older than your latest commit.", "window_days": 7}}}"##
        }
        "stale_no_index" => {
            r##"{"context_index_stale": {"abstained": {"reason": "no easy-context data (service down or no index for a known project)", "rule_id": "context_index_stale", "title": "Code index is out of date"}, "hit": null}}"##
        }
        "unreadable_boundary" => r##"{"records_unreadable": {"abstained": null, "hit": null}}"##,
        "unreadable_mixed" => {
            r##"{"records_unreadable": {"abstained": null, "hit": {"causes": [{"kind": "record_type", "name": "(unknown)", "text": "(unknown) records from Claude Code unknown: 5 could not be read", "unreadable": 5, "version": "unknown"}, {"kind": "record_type", "name": "x", "text": "x records from Claude Code 2.1.1: 2 could not be read", "unreadable": 2, "version": "2.1.1"}], "detail": {"version": "unknown"}, "diagnosis": "unreadable_records", "fix": "Most unreadable records are (unknown) records from Claude Code unknown (5 records). Update easy-personal to the latest version. If it stays high, tell the easy-personal maintainer about (unknown) records from Claude Code unknown.", "label": "aware", "measured": 0.5, "priority": 2, "rule_id": "records_unreadable", "steps": ["Most unreadable records are (unknown) records from Claude Code unknown (5 records).", "Update easy-personal to the latest version.", "If it stays high, tell the easy-personal maintainer about (unknown) records from Claude Code unknown."], "summary": "50% of records from Claude Code unknown could not be read; target 2% or less, last 7 days", "threshold": 0.02, "title": "Some Claude Code records are unreadable", "why": "Records the parser cannot read are left out of every number you see, so the totals are too low.", "window_days": 7}}}"##
        }
        "unreadable_tie" => {
            r##"{"records_unreadable": {"abstained": null, "hit": {"causes": [{"kind": "record_type", "name": "b", "text": "b records from Claude Code 2.1.0: 2 could not be read", "unreadable": 2, "version": "2.1.0"}, {"kind": "record_type", "name": "a", "text": "a records from Claude Code 2.0.0: 1 could not be read", "unreadable": 1, "version": "2.0.0"}, {"kind": "record_type", "name": "c", "text": "c records from Claude Code 1.9.0: 1 could not be read", "unreadable": 1, "version": "1.9.0"}], "detail": {"version": "2.0.0"}, "diagnosis": "unreadable_records", "fix": "Most unreadable records are b records from Claude Code 2.1.0 (2 records). Update easy-personal to the latest version. If it stays high, tell the easy-personal maintainer about b records from Claude Code 2.1.0.", "label": "aware", "measured": 0.25, "priority": 2, "rule_id": "records_unreadable", "steps": ["Most unreadable records are b records from Claude Code 2.1.0 (2 records).", "Update easy-personal to the latest version.", "If it stays high, tell the easy-personal maintainer about b records from Claude Code 2.1.0."], "summary": "25% of records from Claude Code 2.0.0 could not be read; target 2% or less, last 7 days", "threshold": 0.02, "title": "Some Claude Code records are unreadable", "why": "Records the parser cannot read are left out of every number you see, so the totals are too low.", "window_days": 7}}}"##
        }
        _ => "null",
    }
}
// EXPECTED-END
