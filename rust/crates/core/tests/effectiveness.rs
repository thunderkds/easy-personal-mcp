//! T075: `effectiveness::effectiveness_in` (FR-098) over fixture stores and temp git repos. Each test copies the
//! committed parity store into its own temp folder and empties `events`; the owner's real data folder is never opened.

use easy_personal_core::effectiveness::effectiveness_in;
use easy_personal_core::scope::Scope;
use easy_personal_core::store::open_read_only;
use easy_personal_core::writer::open_existing_for_write;
use jiff::tz::TimeZone;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

const PRICED: &str = "claude-sonnet-5-5";

struct Fx {
    dir: PathBuf,
    conn: Connection,
    offset: i64,
}

impl Drop for Fx {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// One event row: (ts, session, project, branch, sidechain, message id, model, [in, out, cache write, cache read]).
struct Ev<'a> {
    ts: &'a str,
    session: &'a str,
    project: &'a str,
    branch: &'a str,
    sidechain: i64,
    message: &'a str,
    model: &'a str,
    tokens: [i64; 4],
    tools: Option<&'a str>,
    errors: Option<i64>,
}

impl<'a> Ev<'a> {
    fn new(session: &'a str, project: &'a str, branch: &'a str, message: &'a str) -> Self {
        Ev {
            ts: "2026-01-10T10:00:00.000000Z",
            session,
            project,
            branch,
            sidechain: 0,
            message,
            model: PRICED,
            tokens: [100, 10, 0, 0],
            tools: None,
            errors: None,
        }
    }
}

impl Fx {
    fn new(name: &str) -> Fx {
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t075-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let parity = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/store-v11.sqlite");
        std::fs::copy(parity, dir.join("store.sqlite")).unwrap();
        let conn = Connection::open(dir.join("store.sqlite")).unwrap();
        conn.execute_batch("DELETE FROM events").unwrap();
        Fx {
            dir,
            conn,
            offset: 0,
        }
    }

    fn store(&self) -> PathBuf {
        self.dir.join("store.sqlite")
    }

    /// A folder inside the temp folder (created), as a string.
    fn folder(&self, name: &str) -> String {
        let path = self.dir.join(name);
        std::fs::create_dir_all(&path).unwrap();
        path.to_string_lossy().into_owned()
    }

    fn add(&mut self, e: Ev) {
        self.offset += 1;
        self.conn
            .execute(
                "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, record_type, ts_utc,
                 session_id, project_path, git_branch, is_sidechain, model, message_id, tool_names, input_tokens,
                 output_tokens, cache_creation_input_tokens, cache_read_input_tokens, entrypoint, tool_error_count)
                 VALUES ('m', 'claude_code', 'f.jsonl', 1, ?1, 0, 'assistant', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                 ?12, ?13, 'cli', ?14)",
                params![
                    self.offset,
                    e.ts,
                    e.session,
                    e.project,
                    e.branch,
                    e.sidechain,
                    e.model,
                    e.message,
                    e.tools,
                    e.tokens[0],
                    e.tokens[1],
                    e.tokens[2],
                    e.tokens[3],
                    e.errors
                ],
            )
            .unwrap();
    }

    fn run(&self) -> Value {
        let conn = open_read_only(&self.store()).unwrap();
        effectiveness_in(&conn, None, None, Scope::default(), &TimeZone::UTC).unwrap()
    }
}

fn task<'a>(body: &'a Value, id: &str) -> &'a Value {
    body["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["task_id"] == id)
        .unwrap_or_else(|| panic!("{id} not in {body:#}"))
}

fn ids(body: &Value) -> Vec<String> {
    let mut ids: Vec<String> = body["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            format!(
                "{}@{}",
                t["task_id"].as_str().unwrap(),
                t["project"].as_str().unwrap()
            )
        })
        .collect();
    ids.sort();
    ids
}

/// `git` in `repo` at a fixed author and committer time (`@<unix seconds> +0000`).
fn git(repo: &str, at: i64, args: &[&str]) {
    let date = format!("@{at} +0000");
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
            "-C",
            repo,
        ])
        .args(args)
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn commit(repo: &str, at: i64, file: &str) {
    std::fs::write(Path::new(repo).join(file), format!("{at}")).unwrap();
    git(repo, at, &["add", file]);
    git(repo, at, &["commit", "-q", "-m", file]);
}

// 2026-01-10T00:00:00Z
const DAY: i64 = 1_768_003_200;

// ---- AC1: attribution ----

#[test]
fn branch_events_and_sidechains_of_a_branch_session_go_to_the_task_the_rest_is_unattributed() {
    let mut fx = Fx::new("attr");
    let p = fx.folder("proj");
    let other = fx.folder("other");
    // S1: main thread on T10; its sub-agent works in a worktree branch -> T10
    fx.add(Ev::new("S1", &p, "T10", "m1"));
    fx.add(Ev::new("S1", &p, "T10", "m2"));
    fx.add(Ev {
        sidechain: 1,
        ..Ev::new("S1", &p, "worktree-agent-abc", "m3")
    });
    // S2: main thread on main; its sidechains on main and on worktree-agent-x are unattributed
    fx.add(Ev::new("S2", &p, "main", "m4"));
    fx.add(Ev {
        sidechain: 1,
        ..Ev::new("S2", &p, "main", "m5")
    });
    fx.add(Ev {
        sidechain: 1,
        ..Ev::new("S2", &p, "worktree-agent-x", "m6")
    });
    // ...but a sidechain on its own task branch is that task's
    fx.add(Ev {
        sidechain: 1,
        ..Ev::new("S2", &p, "T11", "m7")
    });
    // id-less branches: fix-foo, HEAD, worktree-agent-* (even holding an id), a word ending in T5
    fx.add(Ev::new("S3", &p, "fix-foo", "m8"));
    fx.add(Ev::new("S4", &p, "HEAD", "m9"));
    fx.add(Ev::new("S5", &p, "worktree-agent-T99", "m10"));
    fx.add(Ev::new("S6", &p, "LAST5", "m11"));
    // two ids in one name: the first one
    fx.add(Ev::new("S7", &p, "feat/HUN-12-and-T13", "m12"));
    // the same id in another repo is another task
    fx.add(Ev::new("S8", &other, "T10", "m13"));
    // one message seen twice (two records) counts once
    fx.add(Ev::new("S8", &other, "T10", "m13"));

    let body = fx.run();
    assert_eq!(
        ids(&body),
        ["HUN-12@proj", "T10@other", "T10@proj", "T11@proj"],
        "{body:#}"
    );
    let t10 = body["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["task_id"] == "T10" && t["project"] == "proj")
        .unwrap();
    assert_eq!(t10["tokens"], 330, "m1 + m2 + the sidechain m3");
    assert_eq!(t10["input_tokens"], 300);
    assert_eq!(t10["output_tokens"], 30);
    assert_eq!(t10["sessions"], 1);
    assert_eq!(t10["branches"], json!(["T10", "worktree-agent-abc"]));
    assert_eq!(task(&body, "T11")["tokens"], 110);
    assert_eq!(task(&body, "T11")["sessions"], 1);
    let other_t10 = body["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["task_id"] == "T10" && t["project"] == "other")
        .unwrap();
    assert_eq!(other_t10["tokens"], 110, "m13 once");
    // unattributed: m4, m5, m6 (S2) + m8 (fix-foo) + m9 (HEAD) + m10 (worktree-agent-T99) + m11 (LAST5)
    assert_eq!(body["unattributed"]["tokens"], 7 * 110, "{body:#}");
    assert_eq!(body["unattributed"]["sessions"], 5, "S2, S3, S4, S5, S6");
    assert_eq!(body["totals"]["tokens"], 13 * 110, "13 distinct messages");
    assert_eq!(body["totals"]["sessions"], 8);
}

// ---- AC2: done time from local git ----

#[test]
fn done_is_the_first_merge_into_main_and_a_squash_or_fast_forward_is_not_done() {
    let mut fx = Fx::new("done");
    let repo = fx.folder("repo");
    git(&repo, DAY, &["init", "-q"]);
    commit(&repo, DAY, "base");
    // T1: merged twice (a fix round); the first merge wins
    git(&repo, DAY + 10, &["checkout", "-q", "-b", "T1"]);
    commit(&repo, DAY + 20, "t1a");
    git(&repo, DAY + 30, &["checkout", "-q", "main"]);
    git(
        &repo,
        DAY + 100,
        &["merge", "-q", "--no-ff", "--no-edit", "T1"],
    );
    git(&repo, DAY + 110, &["checkout", "-q", "T1"]);
    commit(&repo, DAY + 120, "t1b");
    git(&repo, DAY + 130, &["checkout", "-q", "main"]);
    git(
        &repo,
        DAY + 200,
        &["merge", "-q", "--no-ff", "--no-edit", "T1"],
    );
    // T2: squash-merged (no merge commit)
    git(&repo, DAY + 210, &["checkout", "-q", "-b", "T2"]);
    commit(&repo, DAY + 220, "t2");
    git(&repo, DAY + 230, &["checkout", "-q", "main"]);
    git(&repo, DAY + 240, &["merge", "-q", "--squash", "T2"]);
    git(&repo, DAY + 250, &["commit", "-q", "-m", "T2 squashed"]);
    // T3: fast-forwarded (rebased, no merge commit)
    git(&repo, DAY + 260, &["checkout", "-q", "-b", "T3"]);
    commit(&repo, DAY + 270, "t3");
    git(&repo, DAY + 280, &["checkout", "-q", "main"]);
    git(&repo, DAY + 290, &["merge", "-q", "--ff-only", "T3"]);
    // T4: a merge into another branch, not into main, is not done
    git(&repo, DAY + 300, &["checkout", "-q", "-b", "side"]);
    git(&repo, DAY + 310, &["checkout", "-q", "-b", "T4"]);
    commit(&repo, DAY + 320, "t4");
    git(&repo, DAY + 330, &["checkout", "-q", "side"]);
    git(
        &repo,
        DAY + 340,
        &["merge", "-q", "--no-ff", "--no-edit", "T4"],
    );
    git(&repo, DAY + 350, &["checkout", "-q", "main"]);
    // T8's sub-agent branch worktree-agent-zz is merged into main: only a branch that holds T8 makes T8 done
    git(
        &repo,
        DAY + 360,
        &["checkout", "-q", "-b", "worktree-agent-zz"],
    );
    commit(&repo, DAY + 370, "zz");
    git(&repo, DAY + 380, &["checkout", "-q", "main"]);
    git(
        &repo,
        DAY + 390,
        &["merge", "-q", "--no-ff", "--no-edit", "worktree-agent-zz"],
    );
    // T9: merged into main, then main is fast-forwarded to `ff`, a branch that had merged main into itself, so
    // the T9 merge is no longer on main's first-parent chain: it still counts
    git(&repo, DAY + 400, &["checkout", "-q", "-b", "ff"]);
    commit(&repo, DAY + 410, "ff");
    git(&repo, DAY + 420, &["checkout", "-q", "main"]);
    git(&repo, DAY + 430, &["checkout", "-q", "-b", "T9"]);
    commit(&repo, DAY + 440, "t9");
    git(&repo, DAY + 450, &["checkout", "-q", "main"]);
    git(
        &repo,
        DAY + 460,
        &["merge", "-q", "--no-ff", "--no-edit", "T9"],
    );
    git(&repo, DAY + 470, &["checkout", "-q", "ff"]);
    git(
        &repo,
        DAY + 480,
        &["merge", "-q", "--no-ff", "--no-edit", "main"],
    );
    git(&repo, DAY + 490, &["checkout", "-q", "main"]);
    git(&repo, DAY + 500, &["merge", "-q", "--ff-only", "ff"]);
    // `side` (holding `Merge branch 'T4' into side`) now reaches main: a merge into another branch still does not count
    git(
        &repo,
        DAY + 510,
        &["merge", "-q", "--no-ff", "--no-edit", "side"],
    );
    fx.add(Ev::new("S4", &repo, "T9", "m22"));
    fx.add(Ev::new("S3", &repo, "T8", "m20"));
    fx.add(Ev {
        sidechain: 1,
        ..Ev::new("S3", &repo, "worktree-agent-zz", "m21")
    });
    let plain = fx.folder("plain"); // not a git folder
    for (i, branch) in ["T1", "T2", "T3", "T4"].iter().enumerate() {
        fx.add(Ev::new("S1", &repo, branch, &format!("m{i}")));
    }
    fx.add(Ev::new("S2", &plain, "T5", "m9"));

    let body = fx.run();
    let t1 = task(&body, "T1");
    assert_eq!(
        t1["done_utc"], "2026-01-10T00:01:40Z",
        "the first merge, not the second"
    );
    assert_eq!(t1["done_state"], "done");
    assert_eq!(t1["tokens_per_task_done"], 110);
    assert_eq!(task(&body, "T8")["tokens"], 220, "its sidechain counts");
    assert_eq!(
        task(&body, "T9")["done_utc"],
        "2026-01-10T00:07:40Z",
        "off the first-parent chain, still done"
    );
    for id in ["T2", "T3", "T4", "T8"] {
        let t = task(&body, id);
        assert_eq!(t["done_utc"], Value::Null, "{id}");
        assert_eq!(t["done_state"], "not done", "{id}");
        assert_eq!(t["tokens_per_task_done"], Value::Null, "{id}");
        assert!(
            t["tokens"].as_i64().unwrap() > 0,
            "tokens still shown for {id}"
        );
    }
    let t5 = task(&body, "T5");
    assert_eq!(t5["done_state"], "unknown", "no git history");
    assert_eq!(t5["done_utc"], Value::Null);
    assert!(
        body["notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n.as_str().unwrap().contains("no local git history")),
        "{body:#}"
    );
    assert_eq!(body["totals"]["tasks"], 7);
    assert_eq!(body["totals"]["tasks_done"], 2);
    assert_eq!(body["totals"]["tokens_per_task_done"], 110.0);
}

// ---- AC3: numbers ----

#[test]
fn zero_good_results_leave_tokens_per_good_result_blank_and_an_unknown_price_is_never_zero() {
    let mut fx = Fx::new("numbers");
    let p = fx.folder("proj");
    fx.add(Ev {
        model: "mystery-model",
        ..Ev::new("S1", &p, "T1", "m1")
    });
    fx.add(Ev::new("S2", &p, "T2", "m2"));
    fx.add(Ev {
        model: "mystery-model",
        ..Ev::new("S2", &p, "T2", "m3")
    });
    let body = fx.run();
    let t1 = task(&body, "T1");
    assert_eq!(t1["good_results"], 0);
    assert_eq!(t1["tokens_per_good_result"], Value::Null);
    assert_eq!(
        t1["api_equivalent_cost"],
        json!({"usd": null, "unknown_price": true, "abstained_models": ["mystery-model"]})
    );
    // partly priced: the priced part, flagged
    let cost = &task(&body, "T2")["api_equivalent_cost"];
    assert_eq!(cost["usd"], 0.00045, "100 in x $3/M + 10 out x $15/M");
    assert_eq!(cost["unknown_price"], true);
    assert_eq!(body["totals"]["tokens_per_good_result"], Value::Null);
    assert_eq!(body["unattributed"]["api_equivalent_cost"]["usd"], 0.0);
    assert_eq!(
        body["unattributed"]["api_equivalent_cost"]["unknown_price"],
        false
    );
}

#[test]
fn a_good_result_of_fr_061_divides_the_tasks_tokens() {
    let mut fx = Fx::new("good");
    let repo = fx.folder("repo");
    git(&repo, DAY, &["init", "-q"]);
    commit(&repo, DAY, "base");
    git(&repo, DAY + 3600, &["checkout", "-q", "-b", "T7"]);
    // a commit 10 min after the session's last event: inside the 30 min window
    commit(&repo, DAY + 36_000 + 600, "work");
    git(&repo, DAY + 36_000 + 700, &["checkout", "-q", "main"]);
    // S1 (10:00) on T7, one tool call, no tool error -> good result; S2 on T7 has no tool error count -> unknown
    fx.add(Ev {
        tools: Some("Bash"),
        errors: Some(0),
        ..Ev::new("S1", &repo, "T7", "m1")
    });
    fx.add(Ev {
        tools: Some("Bash"),
        ..Ev::new("S2", &repo, "T7", "m2")
    });
    let body = fx.run();
    let t7 = task(&body, "T7");
    assert_eq!(t7["sessions"], 2);
    assert_eq!(t7["good_results"], 1, "{body:#}");
    assert_eq!(t7["tokens_per_good_result"], 220.0);
    assert_eq!(body["totals"]["good_results"], 1);
}

// ---- AC4: retention ----

#[test]
fn a_pruned_store_states_its_oldest_stored_day() {
    let mut fx = Fx::new("retention");
    let p = fx.folder("proj");
    fx.add(Ev {
        ts: "2026-01-01T09:00:00.000000Z",
        ..Ev::new("S1", &p, "T1", "m1")
    });
    fx.add(Ev {
        ts: "2026-01-08T09:00:00.000000Z",
        ..Ev::new("S1", &p, "T1", "m2")
    });
    fx.add(Ev {
        ts: "2026-01-09T23:30:00.000000Z",
        ..Ev::new("S1", &p, "T1", "m3")
    });
    assert_eq!(fx.run()["oldest_stored_day"], "2026-01-01");
    // keep 5 days from 2026-01-12: the 2026-01-01 event goes
    let mut store = open_existing_for_write(&fx.store()).unwrap();
    let deleted = store
        .prune(Some(5), (DAY + 2 * 86_400) * 1_000_000)
        .unwrap();
    assert_eq!(deleted, 1);
    drop(store);
    let body = fx.run();
    assert_eq!(body["oldest_stored_day"], "2026-01-08");
    assert!(
        body["retention_note"]
            .as_str()
            .unwrap()
            .contains("2026-01-08")
    );
    assert_eq!(task(&body, "T1")["tokens"], 220);
    // the day is the local day: with m2 gone, the oldest is 23:30 UTC on the 9th, the 10th in Tokyo
    fx.conn
        .execute("DELETE FROM events WHERE message_id = 'm2'", [])
        .unwrap();
    assert_eq!(fx.run()["oldest_stored_day"], "2026-01-09");
    let tokyo = TimeZone::get("Asia/Tokyo").unwrap();
    let conn = open_read_only(&fx.store()).unwrap();
    let local = effectiveness_in(&conn, None, None, Scope::default(), &tokyo).unwrap();
    assert_eq!(local["oldest_stored_day"], "2026-01-10");
}

// ---- read-only, scope, period, privacy ----

#[test]
fn a_read_changes_nothing_honours_the_period_and_the_scope_and_names_no_path_or_session() {
    let mut fx = Fx::new("readonly");
    let p = fx.folder("proj");
    fx.add(Ev {
        ts: "2026-01-05T09:00:00.000000Z",
        ..Ev::new("secret-session-1", &p, "T1", "m1")
    });
    fx.add(Ev {
        ts: "2026-01-09T09:00:00.000000Z",
        ..Ev::new("secret-session-1", &p, "T1", "m2")
    });
    let before = std::fs::read(fx.store()).unwrap();
    let conn = open_read_only(&fx.store()).unwrap();
    let all = effectiveness_in(&conn, None, None, Scope::default(), &TimeZone::UTC).unwrap();
    let window = effectiveness_in(
        &conn,
        Some("2026-01-08"),
        None,
        Scope::default(),
        &TimeZone::UTC,
    )
    .unwrap();
    assert_eq!(task(&all, "T1")["tokens"], 220);
    assert_eq!(task(&window, "T1")["tokens"], 110);
    assert_eq!(window["period"]["since_utc"], "2026-01-08T00:00:00.000000Z");
    let unknown = effectiveness_in(
        &conn,
        None,
        None,
        Scope {
            workspace: Some("nope"),
            account: None,
        },
        &TimeZone::UTC,
    );
    assert_eq!(unknown.unwrap_err().to_string(), "unknown workspace");
    let bad = effectiveness_in(
        &conn,
        Some("garbage"),
        None,
        Scope::default(),
        &TimeZone::UTC,
    );
    assert_eq!(
        bad.unwrap_err().to_string(),
        "Invalid isoformat string: 'garbage'"
    );
    drop(conn);
    assert_eq!(
        std::fs::read(fx.store()).unwrap(),
        before,
        "nothing written"
    );
    let text = all.to_string();
    assert!(!text.contains("secret-session"), "{text}");
    assert!(!text.contains(fx.dir.to_str().unwrap()), "{text}");
}

#[test]
fn a_relative_or_missing_project_never_reads_the_git_history_of_the_working_folder() {
    // the test runs in `crates/core`; `../../..` is this repo's checkout, whose main has `Merge branch 'T071'`
    let mut fx = Fx::new("noproject");
    fx.add(Ev::new("S1", "../../..", "T071", "m1"));
    fx.add(Ev::new("S2", "", "T072", "m2"));
    fx.conn
        .execute(
            "UPDATE events SET project_path = NULL WHERE session_id = 'S2'",
            [],
        )
        .unwrap();
    let body = fx.run();
    assert_eq!(task(&body, "T071")["done_state"], "unknown", "{body:#}");
    assert_eq!(task(&body, "T072")["done_state"], "unknown", "{body:#}");
    assert_eq!(task(&body, "T072")["project"], "(unknown)");
}

#[test]
fn a_good_result_follows_the_main_thread_even_when_sub_agent_events_outnumber_it() {
    let mut fx = Fx::new("sidechainheavy");
    let repo = fx.folder("repo");
    git(&repo, DAY, &["init", "-q"]);
    commit(&repo, DAY, "base");
    // the sub-agent commits on its worktree branch inside the session's window
    git(
        &repo,
        DAY + 3600,
        &["checkout", "-q", "-b", "worktree-agent-x"],
    );
    commit(&repo, DAY + 36_000 + 600, "agent-work");
    git(&repo, DAY + 36_000 + 700, &["checkout", "-q", "main"]);
    // S1: one main-thread event on T30, three sidechain events on worktree-agent-x (the majority of the session)
    fx.add(Ev {
        tools: Some("Bash"),
        errors: Some(0),
        ..Ev::new("S1", &repo, "T30", "m1")
    });
    for i in 2..5 {
        fx.add(Ev {
            sidechain: 1,
            tools: Some("Bash"),
            errors: Some(0),
            ..Ev::new("S1", &repo, "worktree-agent-x", &format!("m{i}"))
        });
    }
    let body = fx.run();
    let t30 = task(&body, "T30");
    assert_eq!(t30["tokens"], 440, "the main thread and its sidechains");
    assert_eq!(t30["good_results"], 1, "{body:#}");
    assert_eq!(t30["tokens_per_good_result"], 440.0);
    let lines: i64 = body["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["good_results"].as_i64().unwrap())
        .sum::<i64>()
        + body["unattributed"]["good_results"].as_i64().unwrap();
    assert_eq!(body["totals"]["good_results"].as_i64().unwrap(), lines);
}
