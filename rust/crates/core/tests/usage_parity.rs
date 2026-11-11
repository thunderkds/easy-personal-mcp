//! T005: `usage_summary` / `usage_breakdown` and the price-table parser against the Python goldens
//! (`rust/parity/golden`, made over `rust/parity/store-v11.sqlite` with TZ=UTC and a frozen clock).
//! Goldens are compared as parsed JSON (keys sorted, list order kept; `1000` and `1000.0` differ).
//! Variations are built on temp copies; the owner's real `~/.easy-personal` is never opened.

use easy_personal_core::metrics::{
    round_py, usage_breakdown, usage_breakdown_with_prices, usage_summary,
};
use easy_personal_core::prices::PriceTable;
use easy_personal_core::store::open_read_only;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

const WINDOW: (&str, &str) = ("2026-01-15T10:05:00Z", "2026-01-15T10:15:00Z");

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

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t005-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A writable copy of the fixture store (the only way a test changes a value).
fn copy_fixture(dir: &TempDir) -> PathBuf {
    let path = dir.0.join("store.sqlite");
    fs::copy(fixture(), &path).unwrap();
    path
}

fn assert_golden(actual: Value, name: &str) {
    assert_eq!(actual, golden(name), "{name}");
}

// ---- 1. usage_summary ----

#[test]
fn summary_all_matches_golden() {
    let v = usage_summary(&open_fixture(), None, None).unwrap();
    assert_golden(v, "api__usage_summary__all");
}

#[test]
fn summary_window_matches_golden() {
    let v = usage_summary(&open_fixture(), Some(WINDOW.0), Some(WINDOW.1)).unwrap();
    assert_golden(v, "api__usage_summary__window");
}

// ---- 2. usage_breakdown ----

#[test]
fn breakdown_by_model_matches_golden() {
    assert_golden(
        usage_breakdown(&open_fixture(), "model", None, None).unwrap(),
        "api__usage_breakdown__model",
    );
}

#[test]
fn breakdown_by_model_window_matches_golden() {
    let v = usage_breakdown(&open_fixture(), "model", Some(WINDOW.0), Some(WINDOW.1)).unwrap();
    assert_golden(v, "api__usage_breakdown__model_window");
}

/// The `day` keys are local days (T009) and the golden was made under `TZ=UTC`, so the check runs in a child process
/// with `TZ=UTC` (the zone is cached per process, as in `usage_tz_parity.rs`) and passes under any machine zone.
#[test]
fn breakdown_by_day_matches_golden() {
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "breakdown_by_day_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("TZ", "UTC")
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success() && text.contains("1 passed"), "{text}");
}

#[test]
#[ignore = "run by breakdown_by_day_matches_golden, in a child process with TZ=UTC"]
fn breakdown_by_day_child() {
    assert_golden(
        usage_breakdown(&open_fixture(), "day", None, None).unwrap(),
        "api__usage_breakdown__day",
    );
}

#[test]
fn breakdown_by_project_matches_golden() {
    assert_golden(
        usage_breakdown(&open_fixture(), "project", None, None).unwrap(),
        "api__usage_breakdown__project",
    );
}

#[test]
fn breakdown_by_tool_matches_golden() {
    assert_golden(
        usage_breakdown(&open_fixture(), "tool", None, None).unwrap(),
        "api__usage_breakdown__tool",
    );
}

// ---- 3. error text equals the Python 400 message ----

#[test]
fn bad_by_error_text_equals_golden() {
    let err = usage_breakdown(&open_fixture(), "bad", None, None).unwrap_err();
    assert_eq!(
        json!(err.to_string()),
        golden("api__usage_breakdown__bad_by")["error"]
    );
}

#[test]
fn bad_since_error_text_equals_golden() {
    let golden_text = golden("api__usage_sessions__bad_since")["error"].clone();
    let err = usage_summary(&open_fixture(), Some("bad"), None).unwrap_err();
    assert_eq!(json!(err.to_string()), golden_text);
    let err = usage_breakdown(&open_fixture(), "day", None, Some("bad")).unwrap_err();
    assert!(err.to_string().starts_with("Invalid isoformat string: "));
}

/// Only forms that name an offset (or are invalid) are checked here: they give the same answer in every zone. The
/// offset-less forms are local time since T009 and are pinned to UTC in `usage_tz_parity.rs`
/// (`offset_less_forms_read_as_local_time`).
#[test]
fn since_accepts_offset_and_refuses_bad_values() {
    let c = open_fixture();
    let utc = |s: &str| usage_summary(&c, Some(s), None).unwrap()["period"]["since_utc"].clone();
    assert_eq!(
        utc("2026-01-15T10:05:00Z"),
        json!("2026-01-15T10:05:00.000000Z")
    );
    assert_eq!(
        utc("2026-01-15T10:05:00+02:30"),
        json!("2026-01-15T07:35:00.000000Z")
    );
    assert_eq!(
        utc("2026-01-15T00:30:00-01:00"),
        json!("2026-01-15T01:30:00.000000Z")
    );
    for bad in [
        "",
        "2026-1-15",
        "2026-01-15T25:00",
        "2026-02-30",
        "x2026-01-15",
    ] {
        assert!(usage_summary(&c, Some(bad), None).is_err(), "{bad:?}");
    }
}

// ---- 4. price-table parser ----

const TABLE: &str = "\
# comment

as_of: 2026-10-21
models:
  # a comment inside models

  m-one: input=5 output=25 cache_write=6.25 cache_read=0.5
  m-two: input=1 output=2 cache_write=3 cache_read=4
";

#[test]
fn price_parser_reads_as_of_models_comments_and_blanks() {
    let t = PriceTable::parse(TABLE).unwrap();
    assert_eq!(t.as_of, "2026-10-21");
    let p = t.price("m-one").unwrap();
    assert_eq!(
        (p.input, p.output, p.cache_write, p.cache_read),
        (5.0, 25.0, 6.25, 0.5)
    );
    assert!(t.price("m-two").is_some());
    assert!(
        t.price("m-three").is_none(),
        "a model missing from the table is abstained"
    );
}

#[test]
fn shipped_table_parses_and_has_three_models() {
    let t = PriceTable::shipped().unwrap();
    assert_eq!(t.as_of, "2026-10-21");
    for m in ["claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-5-5"] {
        assert!(t.price(m).is_some(), "{m}");
    }
}

#[test]
fn price_parser_ignores_what_python_ignores() {
    // Python: unknown top-level keys and indented lines outside `models:` are skipped; only a model line can fail.
    let t = PriceTable::parse("note: x\nas_of: d\nother:\n  junk line\nmodels:\n  a: input=1 output=1 cache_write=1 cache_read=1\n").unwrap();
    assert_eq!(t.as_of, "d");
    assert!(t.price("a").is_some() && t.price("junk line").is_none());
    assert_eq!(PriceTable::parse("").unwrap().as_of, "");
}

#[test]
fn price_parser_fails_on_a_malformed_model_line_like_python() {
    // Python raises (ValueError / TypeError) on each of these; it never skips them.
    for body in [
        "m: input=1 output=1 cache_write=1", // missing key
        "m: input=1 output=1 cache_write=1 cache_read=1 extra=2", // unknown key
        "m: input=1 output=1 cache_write=1 cache_read", // item without `=`
        "m: input=1 output=1 cache_write=1 cache_read=1=2", // item with two `=`
        "m: input=x output=1 cache_write=1 cache_read=1", // not a number
        "m:",                                // no fields at all
    ] {
        let err = PriceTable::parse(&format!("as_of: d\nmodels:\n  {body}\n")).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("line 3"), "{text}");
        assert!(
            !text.contains("cache_write"),
            "the line text must not be echoed: {text}"
        );
    }
}

#[test]
fn unknown_model_is_abstained_and_a_priced_model_gets_a_cost() {
    // The fixture's model is not in the shipped table -> usd null + abstained_models (the golden).
    let v = usage_breakdown(&open_fixture(), "model", None, None).unwrap();
    assert_eq!(v["total"]["api_equivalent_cost"]["usd"], Value::Null);
    assert_eq!(
        v["total"]["api_equivalent_cost"]["abstained_models"],
        json!(["claude-fixture-model-1"])
    );
    // A custom table that knows it: cost = (15*1 + 8*2 + 250*3 + 1000*4) / 1e6 = 0.004781
    let t = PriceTable::parse(
        "as_of: x\nmodels:\n  claude-fixture-model-1: input=1 output=2 cache_write=3 cache_read=4\n",
    )
    .unwrap();
    let v = usage_breakdown_with_prices(&open_fixture(), "model", None, None, &t).unwrap();
    assert_eq!(v["total"]["api_equivalent_cost"]["usd"], json!(0.004781));
    assert_eq!(
        v["total"]["api_equivalent_cost"]["abstained_models"],
        json!([])
    );
    assert_eq!(
        v["total"]["api_equivalent_cost"]["price_table_date"],
        json!("x")
    );
}

/// Expected values below were produced by the Python app (`metrics.usage_breakdown`) over the same temp
/// copy: the fixture's model renamed to `claude-sonnet-5-5`, which the shipped table prices.
#[test]
fn priced_model_matches_python_output_on_a_changed_copy() {
    let dir = TempDir::new();
    let path = copy_fixture(&dir);
    Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE events SET model = 'claude-sonnet-5-5' WHERE model = 'claude-fixture-model-1'",
            [],
        )
        .unwrap();
    let c = open_read_only(&path).unwrap();
    let cost =
        |usd: f64| json!({"abstained_models": [], "price_table_date": "2026-10-21", "usd": usd});

    let v = usage_breakdown(&c, "model", None, None).unwrap();
    assert_eq!(v["groups"][0]["key"], json!("claude-sonnet-5-5"));
    assert_eq!(v["groups"][0]["api_equivalent_cost"], cost(0.001403)); // 0.0014025 in binary sits above the tie
    assert_eq!(v["total"]["api_equivalent_cost"], cost(0.001403));

    let v = usage_breakdown(&c, "tool", None, None).unwrap();
    assert_eq!(v["groups"][0]["key"], json!("Read"));
    assert_eq!(v["groups"][0]["api_equivalent_cost"], cost(0.001155));
    assert_eq!(v["groups"][1]["key"], json!("(none)"));
    assert_eq!(v["groups"][1]["api_equivalent_cost"], cost(0.000247));
}

// ---- 5. rounding ----

/// `round_py` is `format!("{:.N}")` then `parse`: Rust formats the exact binary value with
/// round-half-to-even, as CPython's `round(x, n)` does (`dtoa` mode 3). A naive `(x * 10^n).round() / 10^n`
/// rounds the already-multiplied (and itself rounded) value half away from zero and differs.
/// Vectors are Python's `round(x, n)` output.
#[test]
fn round_py_matches_python_round_on_ties_and_near_ties() {
    let vectors: [(f64, usize, f64); 21] = [
        (0.03125, 4, 0.0312), // exact tie, even digit below
        (0.09375, 4, 0.0938), // exact tie, odd digit below
        (0.15625, 4, 0.1562),
        (0.00005, 4, 0.0001), // binary value above the tie
        (0.00015, 4, 0.0001), // binary value below the tie
        (1.00005, 4, 1.0001),
        (2.00005, 4, 2.0), // naive (x*1e4).round()/1e4 gives 2.0001
        (0.0014025, 6, 0.001403),
        (0.0000005, 6, 0.0),
        (0.0000015, 6, 0.000002),
        (0.0000025, 6, 0.000003),
        (0.5, 0, 0.0),
        (0.7905, 4, 0.7905),
        (0.12345, 4, 0.1235),
        (0.00045, 4, 0.0004),
        (0.00015000000000000001, 4, 0.0002),
        (0.0625, 3, 0.062),
        (0.1875, 3, 0.188),
        (0.9999995, 6, 1.0),
        (0.0078125, 6, 0.007812),
        (0.0234375, 6, 0.023438),
    ];
    for (x, digits, want) in vectors {
        assert_eq!(round_py(x, digits), want, "round({x:?}, {digits})");
    }
}

// ---- 6. the fixture is read-only ----

#[test]
fn fixture_is_unchanged_and_gets_no_sidecars() {
    let before = fs::read(fixture()).unwrap();
    {
        let c = open_fixture();
        usage_summary(&c, None, None).unwrap();
        for by in ["day", "model", "project", "tool"] {
            usage_breakdown(&c, by, None, None).unwrap();
        }
        usage_summary(&c, Some(WINDOW.0), Some(WINDOW.1)).unwrap();
    }
    assert_eq!(fs::read(fixture()).unwrap(), before);
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut name = fixture().into_os_string();
        name.push(suffix);
        assert!(!Path::new(&name).exists(), "sidecar {suffix} appeared");
    }
}

// ---- tests are not constant ----

#[test]
fn changing_one_fixture_value_breaks_the_golden_comparison() {
    let dir = TempDir::new();
    let path = copy_fixture(&dir);
    Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE events SET input_tokens = 11 WHERE message_id = 'msg_fixture_A'",
            [],
        )
        .unwrap();
    let v = usage_summary(&open_read_only(&path).unwrap(), None, None).unwrap();
    assert_ne!(v, golden("api__usage_summary__all"));
    assert_eq!(v["input_tokens"], json!(16)); // 15 + 1: it is the changed value that moved
}

// ---- project owner rules (FR-057 / FR-069) ----

/// A store with the columns the two functions read, one message per `(project_path, branch)`.
fn mini_store(dir: &TempDir, rows: &[(&str, &str, i64)]) -> Connection {
    let path = dir.0.join("mini.sqlite");
    let c = Connection::open(&path).unwrap();
    c.execute_batch(
        "CREATE TABLE events (id INTEGER PRIMARY KEY, abstained INTEGER NOT NULL, version TEXT, ts_utc TEXT,
            session_id TEXT, project_path TEXT, git_branch TEXT, is_sidechain INTEGER, model TEXT,
            message_id TEXT, tool_names TEXT, input_tokens INTEGER, output_tokens INTEGER,
            cache_creation_input_tokens INTEGER, cache_read_input_tokens INTEGER, entrypoint TEXT);",
    )
    .unwrap();
    for (i, (project, branch, message)) in rows.iter().enumerate() {
        // `message` 1 = a message with 100 input tokens, 0 = a bare record (it only makes the path known)
        c.execute(
            "INSERT INTO events (abstained, ts_utc, project_path, git_branch, message_id, model, input_tokens,
                output_tokens, cache_creation_input_tokens, cache_read_input_tokens, entrypoint, is_sidechain)
             VALUES (0, '2026-01-15T10:00:00.000000Z', ?1, ?2, ?3, 'm', ?4, 0, 0, 0, 'cli', 0)",
            rusqlite::params![
                project,
                branch,
                (*message == 1).then(|| format!("msg{i}")),
                (*message == 1).then_some(100_i64)
            ],
        )
        .unwrap();
    }
    drop(c);
    open_read_only(&path).unwrap()
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args([
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
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?} failed");
}

fn keys(v: &Value) -> Vec<String> {
    v["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["key"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn project_groups_a_gone_subfolder_under_its_nearest_existing_git_repo() {
    let dir = TempDir::new();
    let repo = dir.0.join("alpha");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    let gone = repo.join("sub").join("gone");
    let c = mini_store(&dir, &[(gone.to_str().unwrap(), "main", 1)]);
    let v = usage_breakdown(&c, "project", None, None).unwrap();
    assert_eq!(
        keys(&v),
        ["alpha"],
        "rule 1: the repo of the nearest existing parent"
    );
    assert!(v["groups"][0].get("scratch").is_none());
}

#[test]
fn project_groups_a_gone_worktree_under_the_repo_that_has_its_branch() {
    let dir = TempDir::new();
    let repo = dir.0.join("beta");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "first"]);
    git(&repo, &["branch", "feat-x"]);
    let gone = dir.0.join("missing").join("wt");
    let c = mini_store(
        &dir,
        &[
            (repo.to_str().unwrap(), "main", 0),
            (gone.to_str().unwrap(), "feat-x", 1),
        ],
    );
    let v = usage_breakdown(&c, "project", None, None).unwrap();
    assert_eq!(
        keys(&v),
        ["beta"],
        "rule 2: the one stored repo with the branch as a ref"
    );
}

#[test]
fn project_scratch_rules_and_unknown_project() {
    let dir = TempDir::new();
    let existing = dir.0.join("plain"); // exists, under the temp folder -> (a) scratch
    fs::create_dir_all(&existing).unwrap();
    let c = mini_store(
        &dir,
        &[
            (existing.to_str().unwrap(), "main", 1),
            ("/definitely/gone/app", "main", 1),
            ("relative/path", "main", 1),
        ],
    );
    let v = usage_breakdown(&c, "project", None, None).unwrap();
    assert_eq!(keys(&v), ["Scratch"]);
    assert_eq!(v["groups"][0]["scratch"], json!(true));
    let folders: Vec<_> = v["groups"][0]["folders"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["key"].as_str().unwrap())
        .collect();
    assert_eq!(folders, ["app", "path", "plain"]); // equal tokens: ties by key
}
