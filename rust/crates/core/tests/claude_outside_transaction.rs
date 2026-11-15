//! T078 intent 2: a `claude` subprocess must not run while a SQLite write transaction is open on the store.
//!
//! The fake `claude` here probes the store from a second connection at the moment `run` is called: it reads, and it
//! asks for the write lock with no wait. A held `BEGIN IMMEDIATE` shows as "database is locked" on the write probe
//! (WAL: a reader is never blocked, a second writer is). The fake is not slow in wall time: the probe at the moment of
//! the call proves the same thing as a sleep would, with no timing in the test.

use easy_personal_core::sync::{Claude, Ran};
use easy_personal_core::workspaces::{self, write_session};
use rusqlite::Connection;
use std::cell::RefCell;
use std::path::{Path, PathBuf};

/// What one `claude` call saw on the store.
#[derive(Debug)]
struct Probe {
    args: String,
    read_ok: bool,
    write_lock_free: bool,
}

struct Probing {
    store: PathBuf,
    seen: RefCell<Vec<Probe>>,
}

impl Probing {
    fn new(store: &Path) -> Self {
        Probing {
            store: store.to_path_buf(),
            seen: RefCell::new(Vec::new()),
        }
    }
}

impl Claude for Probing {
    fn on_path(&self) -> bool {
        true
    }
    fn run(&self, args: &[&str], _: Option<&str>) -> Ran {
        let second = Connection::open(&self.store).unwrap();
        second.busy_timeout(std::time::Duration::ZERO).unwrap();
        let read_ok = second
            .query_row("SELECT COUNT(*) FROM connectors", [], |r| {
                r.get::<_, i64>(0)
            })
            .is_ok();
        // `BEGIN IMMEDIATE` is the write lock itself; it is refused at once while another writer holds it
        let write_lock_free = second.execute_batch("BEGIN IMMEDIATE; ROLLBACK").is_ok();
        self.seen.borrow_mut().push(Probe {
            args: args.join(" "),
            read_ok,
            write_lock_free,
        });
        // `claude mcp get`: no such entry (exit 1); every other call succeeds
        Ran {
            code: i32::from(args.get(1) == Some(&"get")),
            stdout: String::new(),
            not_run: false,
        }
    }
    fn launcher(&self) -> Result<String, String> {
        Ok("/usr/bin/easy-personal-launch".into())
    }
}

/// `store-rich` with account 2 pointing at a real folder and one connector registered there, so a plan has to ask
/// `claude`.
fn world(tag: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "easy-personal-rs-t078-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let account = dir.join("acct");
    std::fs::create_dir_all(&account).unwrap();
    let store = dir.join("store.sqlite");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../parity/store-rich/store-v11.sqlite"
        ),
        &store,
    )
    .unwrap();
    let conn = Connection::open(&store).unwrap();
    conn.execute(
        "UPDATE agent_accounts SET folder = ?1 WHERE id = 2",
        [account.to_str().unwrap()],
    )
    .unwrap();
    conn.execute_batch(
        "INSERT INTO connectors VALUES ('c00000078', 'acme-slack', 0, 'slack', 2);
         INSERT INTO account_registrations VALUES (2, 'acme-slack', 'c00000078');",
    )
    .unwrap();
    (dir, store)
}

fn seen_nothing_locked(claude: &Probing) {
    let seen = claude.seen.borrow();
    assert!(
        !seen.is_empty(),
        "the scenario must make the plan ask `claude`"
    );
    for p in seen.iter() {
        assert!(p.read_ok, "a reader is blocked during `claude {}`", p.args);
        assert!(
            p.write_lock_free,
            "the store's write lock is held during `claude {}` (a writer on another connection gets SQLITE_BUSY)",
            p.args
        );
    }
}

#[test]
fn a_move_plan_does_not_hold_the_write_lock_while_claude_runs() {
    for yes in [false, true] {
        let (dir, store) = world(&format!("move{yes}"));
        let claude = Probing::new(&store);
        let done = write_session(&store, None, |conn| {
            workspaces::move_connector(conn, "c00000078", "personal", yes, &claude)
        });
        let _ = done.expect("the move answers");
        seen_nothing_locked(&claude);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn an_alias_change_does_not_hold_the_write_lock_while_claude_runs() {
    let (dir, store) = world("alias");
    let claude = Probing::new(&store);
    let done = write_session(&store, None, |conn| {
        workspaces::set_alias(conn, "c00000078", Some("renamed"), true, &claude)
    });
    let _ = done.expect("the alias change answers");
    seen_nothing_locked(&claude);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_workspace_rename_and_delete_do_not_hold_the_write_lock_while_claude_runs() {
    for yes in [false, true] {
        let (dir, store) = world(&format!("rename{yes}"));
        let claude = Probing::new(&store);
        let done = write_session(&store, None, |conn| {
            // a repo list rides along: the mutation runs twice and must see it both times
            let repos = Some(Ok(vec![Ok("/work/rich/acme-extra".to_string())]));
            let changes = vec![("name".to_string(), serde_json::json!("acme2"))];
            workspaces::patch(conn, 2, changes, repos, yes, &claude)
        });
        let _ = done.expect("the rename answers");
        seen_nothing_locked(&claude);
        let _ = std::fs::remove_dir_all(&dir);

        let (dir, store) = world(&format!("delete{yes}"));
        let claude = Probing::new(&store);
        let done = write_session(&store, None, |conn| {
            workspaces::delete(conn, 2, yes, &claude)
        });
        let _ = done.expect("the delete answers");
        seen_nothing_locked(&claude);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// The plan answers the same as before: a change that was committed is committed once, with its repo list, and a
/// confirmed move leaves the registration for `claude` to finish (here `claude` says "no entry": forgotten).
#[test]
fn a_confirmed_rename_commits_once_with_its_repos() {
    let (dir, store) = world("commit");
    let claude = Probing::new(&store);
    let done = write_session(&store, None, |conn| {
        let repos = Some(Ok(vec![Ok("/work/rich/acme-extra".to_string())]));
        let changes = vec![("name".to_string(), serde_json::json!("acme2"))];
        workspaces::patch(conn, 2, changes, repos, true, &claude)
    })
    .expect("the rename answers");
    assert_eq!(done.status, 0, "{:?}", done.lines);
    let conn = Connection::open(&store).unwrap();
    let alias: String = conn
        .query_row(
            "SELECT alias FROM connectors WHERE id = 'c00000078'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let repos: Vec<String> = conn
        .prepare("SELECT repo_path FROM workspace_repos WHERE workspace_id = 2")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(alias, "acme2-slack");
    assert_eq!(repos, ["/work/rich/acme-extra"]);
    drop(conn);
    let _ = std::fs::remove_dir_all(&dir);
}

/// `sync apply` (the route and the CLI use `plan_all` then `apply` on the session's connection): no transaction.
#[test]
fn a_sync_apply_does_not_hold_the_write_lock_while_claude_runs() {
    let (dir, store) = world("sync");
    let claude = Probing::new(&store);
    write_session(&store, None, |conn| {
        let steps = easy_personal_core::sync::plan_all(conn, None, &claude).unwrap();
        let mut lines = Vec::new();
        easy_personal_core::sync::apply(conn, &steps, &claude, &mut lines)
            .unwrap()
            .unwrap();
        Ok(())
    })
    .unwrap();
    seen_nothing_locked(&claude);
    assert!(
        claude
            .seen
            .borrow()
            .iter()
            .any(|p| p.args.starts_with("mcp add-json")),
        "the apply ran a `claude` write"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
