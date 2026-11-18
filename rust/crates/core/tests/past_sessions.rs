//! T117 (FR-126): `sessions::past_sessions` over temp copies of the committed parity store. The owner's real data folder
//! is never opened.

use easy_personal_core::sessions::{PAST_PAGE, PastFilter, past_query, past_sessions};
use rusqlite::{Connection, params};
use serde_json::Value;
use std::path::{Path, PathBuf};

struct Fx {
    dir: PathBuf,
    conn: Connection,
    n: i64,
}

impl Drop for Fx {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

impl Fx {
    fn new(name: &str) -> Fx {
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t117-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let parity = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/store-v11.sqlite");
        std::fs::copy(parity, dir.join("store.sqlite")).unwrap();
        let conn = Connection::open(dir.join("store.sqlite")).unwrap();
        conn.execute_batch("DELETE FROM events").unwrap();
        Fx { dir, conn, n: 0 }
    }

    /// One event: ts, session, (project, branch), entrypoint, model, account id.
    fn add(
        &mut self,
        ts: &str,
        session: &str,
        place: (&str, &str),
        entry: Option<&str>,
        model: Option<&str>,
        account: Option<i64>,
    ) {
        let (project, branch) = place;
        self.n += 1;
        self.conn
            .execute(
                "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, record_type, ts_utc, \
                 session_id, project_path, git_branch, model, entrypoint, agent_account_id) \
                 VALUES ('m', 'claude_code', 'f.jsonl', 1, ?1, 0, 'assistant', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![self.n, ts, session, project, branch, model, entry, account],
            )
            .unwrap();
    }

    fn past(&self, f: PastFilter) -> Result<Value, String> {
        past_sessions(&self.conn, f).map_err(|e| e.to_string())
    }
}

fn ids(body: &Value) -> Vec<&str> {
    body["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect()
}

const T1: &str = "2026-01-10T10:00:00.000000Z";
const T2: &str = "2026-01-10T11:00:00.000000Z";
const T3: &str = "2026-01-11T09:00:00.000000Z";
const T4: &str = "2026-01-12T09:00:00.000000Z";

fn three(fx: &mut Fx) {
    fx.add(
        T1,
        "aaa",
        ("/w/shop", "main"),
        Some("cli"),
        Some("m1"),
        None,
    );
    fx.add(
        T2,
        "aaa",
        ("/w/shop", "main"),
        Some("cli"),
        Some("m1"),
        None,
    );
    fx.add(
        T3,
        "bbb",
        ("/w/blog", "feat-x"),
        Some("sdk-cli"),
        None,
        None,
    );
    fx.add(
        T4,
        "ccc",
        ("/w/shop", "feat-y"),
        Some("sdk-ts"),
        Some("m2"),
        None,
    );
}

#[test]
fn lists_one_row_per_session_newest_last_first_with_metadata_only() {
    let mut fx = Fx::new("list");
    three(&mut fx);
    let body = fx.past(PastFilter::default()).unwrap();
    assert_eq!(ids(&body), ["ccc", "bbb", "aaa"]);
    assert_eq!(body["next"], Value::Null);
    let a = &body["sessions"][2];
    assert_eq!(a["project"], "shop");
    assert_eq!(a["branch"], "main");
    assert_eq!(
        (a["first"].as_str(), a["last"].as_str()),
        (Some(T1), Some(T2))
    );
    assert_eq!(a["kind"], "terminal");
    assert_eq!(a["model"], "m1");
    assert_eq!(a["account"], "Claude default");
    let keys: Vec<_> = a.as_object().unwrap().keys().cloned().collect();
    let mut want = [
        "id", "project", "branch", "first", "last", "kind", "account", "model",
    ];
    want.sort_unstable();
    let mut got = keys.clone();
    got.sort_unstable();
    assert_eq!(got, want);
    assert_eq!(body["sessions"][1]["kind"], "app");
    assert_eq!(body["sessions"][0]["kind"], "headless");
}

#[test]
fn filters_by_project_branch_kind_and_dates() {
    let mut fx = Fx::new("filters");
    three(&mut fx);
    let f = |f: PastFilter| ids(&fx.past(f).unwrap()).join(",");
    assert_eq!(
        f(PastFilter {
            project: Some("shop"),
            ..Default::default()
        }),
        "ccc,aaa"
    );
    assert_eq!(
        f(PastFilter {
            project: Some("/w/blog"),
            ..Default::default()
        }),
        "bbb"
    );
    assert_eq!(
        f(PastFilter {
            project: Some("hop"),
            ..Default::default()
        }),
        ""
    ); // a folder name, not a suffix
    assert_eq!(
        f(PastFilter {
            branch: Some("main"),
            ..Default::default()
        }),
        "aaa"
    );
    assert_eq!(
        f(PastFilter {
            kind: Some("headless"),
            ..Default::default()
        }),
        "ccc"
    );
    assert_eq!(
        f(PastFilter {
            kind: Some("terminal"),
            ..Default::default()
        }),
        "aaa"
    );
    assert_eq!(
        f(PastFilter {
            from: Some("2026-01-11T00:00:00Z"),
            to: Some("2026-01-12T00:00:00Z"),
            ..Default::default()
        }),
        "bbb"
    );
    assert_eq!(
        f(PastFilter {
            from: Some("2026-01-11T00:00:00Z"),
            ..Default::default()
        }),
        "ccc,bbb"
    );
    // an empty value is no filter
    assert_eq!(
        f(PastFilter {
            project: Some(""),
            kind: Some(""),
            ..Default::default()
        }),
        "ccc,bbb,aaa"
    );
}

#[test]
fn a_filter_value_is_a_value_never_sql_or_a_pattern() {
    let mut fx = Fx::new("inject");
    three(&mut fx);
    for p in ["x' OR '1'='1", "%", "_hop", "shop%"] {
        let body = fx
            .past(PastFilter {
                project: Some(p),
                ..Default::default()
            })
            .unwrap();
        assert!(ids(&body).is_empty(), "{p}");
    }
}

#[test]
fn account_filter_uses_the_scope_rules_and_the_label_is_shown() {
    let mut fx = Fx::new("account");
    fx.conn
        .execute_batch(
            "INSERT INTO agent_accounts (id, provider, folder, label, status, config_dir_env) VALUES \
             (1, 'claude', '/a/.claude', 'home', 'confirmed', NULL), \
             (2, 'claude', '/a/.claude-work', 'work', 'confirmed', '/a/.claude-work');",
        )
        .unwrap();
    fx.add(T1, "aaa", ("/w/shop", "main"), Some("cli"), None, Some(2));
    fx.add(T2, "bbb", ("/w/shop", "main"), Some("cli"), None, Some(1));
    fx.add(T3, "ccc", ("/w/shop", "main"), Some("cli"), None, None); // no account = the default one
    let f = |a| {
        ids(&fx
            .past(PastFilter {
                account: Some(a),
                ..Default::default()
            })
            .unwrap())
        .join(",")
    };
    assert_eq!(f("work"), "aaa");
    assert_eq!(f("2"), "aaa");
    assert_eq!(f("home"), "ccc,bbb");
    let all = fx.past(PastFilter::default()).unwrap();
    let label = |i: usize| all["sessions"][i]["account"].as_str().unwrap().to_owned();
    assert_eq!(
        (label(0), label(1), label(2)),
        ("home".into(), "home".into(), "work".into())
    );
    assert!(
        fx.past(PastFilter {
            account: Some("nope"),
            ..Default::default()
        })
        .is_err()
    );
}

#[test]
fn bad_filter_values_are_one_line_errors_that_hold_none_of_the_input() {
    let fx = Fx::new("bad");
    let secret = "sk-ant-LEAK\nsecond line";
    for f in [
        PastFilter {
            kind: Some(secret),
            ..Default::default()
        },
        PastFilter {
            from: Some(secret),
            ..Default::default()
        },
        PastFilter {
            to: Some("yesterday"),
            ..Default::default()
        },
        PastFilter {
            from: Some("2026-13-45"),
            ..Default::default()
        },
        PastFilter {
            before: Some(secret),
            ..Default::default()
        },
        PastFilter {
            before: Some("2026-01-10T10:00:00Z|bad id"),
            ..Default::default()
        },
        PastFilter {
            project: Some(secret),
            ..Default::default()
        },
        PastFilter {
            branch: Some(&"b".repeat(300)),
            ..Default::default()
        },
    ] {
        let e = fx.past(f).unwrap_err();
        assert!(
            !e.contains('\n') && !e.contains("LEAK") && !e.contains("yesterday"),
            "{e}"
        );
        assert!(!e.is_empty());
    }
}

#[test]
fn pages_of_50_with_a_cursor_and_no_session_twice_or_missed() {
    let mut fx = Fx::new("pages");
    // 120 sessions; every third pair shares one `last` time, so the cursor must break ties by id
    for i in 0..120 {
        let ts = format!(
            "2026-02-01T00:{:02}:{:02}.000000Z",
            (i / 2) / 60,
            (i / 2) % 60
        );
        fx.add(
            &ts,
            &format!("s{i:03}"),
            ("/w/shop", "main"),
            Some("cli"),
            None,
            None,
        );
    }
    let mut seen = Vec::new();
    let mut cursor: Option<String> = None;
    let mut pages = 0;
    loop {
        let body = fx
            .past(PastFilter {
                before: cursor.as_deref(),
                ..Default::default()
            })
            .unwrap();
        pages += 1;
        assert!(ids(&body).len() <= PAST_PAGE);
        seen.extend(ids(&body).iter().map(|s| s.to_string()));
        match body["next"].as_str() {
            Some(n) => cursor = Some(n.to_string()),
            None => break,
        }
    }
    assert_eq!(pages, 3);
    assert_eq!(seen.len(), 120);
    let mut sorted = seen.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 120);
    let mut want = seen.clone();
    want.sort_by(|a, b| b.cmp(a)); // equal times -> id descending; later pairs have later times
    assert_eq!(seen, want);
}

#[test]
fn the_time_index_serves_a_dated_query_and_no_query_builds_an_index_or_reads_a_second_table() {
    let mut fx = Fx::new("plan");
    fx.conn.execute_batch("BEGIN").unwrap();
    for i in 0..100_000 {
        let ts = format!(
            "2026-03-{:02}T{:02}:{:02}:{:02}.000000Z",
            1 + i % 28,
            (i / 60) % 24,
            i % 60,
            (i / 7) % 60
        );
        fx.add(
            &ts,
            &format!("s{}", i % 3000),
            ("/w/shop", "main"),
            Some("cli"),
            Some("m"),
            None,
        );
    }
    fx.conn.execute_batch("COMMIT").unwrap();
    let plan = |f: PastFilter| -> String {
        let (sql, params) = past_query(&fx.conn, f).unwrap();
        let mut stmt = fx
            .conn
            .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
            .unwrap();
        stmt.query_map(rusqlite::params_from_iter(&params), |r| {
            r.get::<_, String>(3)
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect::<Vec<_>>()
        .join(" | ")
    };
    let dated = plan(PastFilter {
        from: Some("2026-03-05"),
        to: Some("2026-03-06"),
        ..Default::default()
    });
    assert!(
        dated.contains("SEARCH events USING INDEX events_ts_message"),
        "{dated}"
    );
    let all = plan(PastFilter::default());
    assert!(!all.contains("AUTOMATIC"), "{all}");
    println!("plan dated: {dated}\nplan all: {all}");
    // and the real call on 100k rows answers one page
    let t = std::time::Instant::now();
    let body = fx.past(PastFilter::default()).unwrap();
    assert_eq!(ids(&body).len(), PAST_PAGE);
    println!("unbounded page over 100k rows: {:?}", t.elapsed()); // measured, not asserted: a loaded machine is slow
}
