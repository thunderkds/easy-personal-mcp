//! T043: `board::board`, `board::read_project`, `board::parse_text`, `memgraph::project_list` and `memgraph::project_graph`
//! against Python.
//!
//! 1. The goldens of the slice: `api__board__*` (7) and `api__memory_graph__*` (3) of `rust/parity/golden-rich`, and
//!    `api__board__all` / `api__memory_graph__list` of `golden`. The rich ones were made over a work folder: a temp copy of
//!    the store has its paths rewritten from `/work/rich/kit` to the temp work folder, the fixture folders are copied there
//!    with the generator's file time, and the golden's `<WORK>` token is the temp folder (as `kit_parity.rs` does).
//! 2. `rust/parity/golden-memgraph/cases.json`, made by `gen_memgraph.py` (Python's own functions over a hostile tree: notes
//!    with frontmatter, links, a dangling link, a note over the cap, a non-UTF-8 note, a symlinked note and folder that leave
//!    the memory folder, 600 notes, 5001 notes (the cap), claude memory, board files of every kind, fourteen kanban texts).
//!    The tree is built here the same way, in a temp folder, with `HOME` and the easy-memory folder pointing at temp folders.
//! 3. Rust-only cases for what Python does differently by decision (a symlink inside the memory folder is skipped) and the
//!    size cases (100,000 kanban lines, a 1 MB line).
//!
//! The owner's real `~/.easy-personal`, `~/.claude*` and `~/.easy-memory` are never opened: every folder is a temp one.

use easy_personal_core::board::{board, parse_text, read_project};
use easy_personal_core::memgraph::{Env, project_graph, project_list};
use easy_personal_core::metrics::MetricsError;
use easy_personal_core::scope::Scope;
use easy_personal_core::store::open_read_only;
use jiff::tz::TimeZone;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, UNIX_EPOCH};

const FROZEN_NOW: i64 = 1_768_867_200_000_000;
const RICH_MTIME: u64 = 1_768_694_400;

fn parity_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity")
}

fn read_text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{path:?}: {e}"))
}

fn set_mtime(path: &Path, secs: u64) {
    fs::File::open(path)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::new(secs, 0))
        .unwrap();
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// A temp folder, removed on drop. It sorts before `/work/rich` (the generator's work folder did).
struct Dir(PathBuf);

impl Dir {
    fn new() -> Dir {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t043-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        assert!(
            dir.to_str().unwrap() < "/work/rich",
            "set TMPDIR to a folder that sorts before /work/rich"
        );
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Dir(dir)
    }

    fn text(&self) -> &str {
        self.0.to_str().unwrap()
    }

    fn env(&self) -> Env {
        Env {
            em_home: self.0.join("em-none"),
            zone: TimeZone::UTC,
            now_micros: FROZEN_NOW,
        }
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn outcome(result: Result<Value, MetricsError>) -> (u16, Value) {
    match result {
        Ok(body) => (200, body),
        Err(MetricsError::Invalid(text)) => (400, json!({ "error": text })),
        Err(other) => panic!("not a caller error: {other}"),
    }
}

// ---- 1. the goldens ----

/// The rich store and fixture folders in a temp work folder.
fn rich_work() -> (Dir, Connection) {
    let dir = Dir::new();
    let kit = dir.0.join("kit");
    copy_tree(&parity_dir().join("fixtures-rich/kit"), &kit);
    let mut stack = vec![kit];
    while let Some(folder) = stack.pop() {
        for entry in fs::read_dir(folder).unwrap() {
            let path = entry.unwrap().path();
            set_mtime(&path, RICH_MTIME);
            if path.is_dir() {
                stack.push(path);
            }
        }
    }
    let store = dir.0.join("store.sqlite");
    fs::copy(parity_dir().join("store-rich/store-v11.sqlite"), &store).unwrap();
    let conn = Connection::open(&store).unwrap();
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    for table in tables {
        let columns: Vec<String> = conn
            .prepare(&format!("PRAGMA table_info(\"{table}\")"))
            .unwrap()
            .query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, String>(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .filter(|(_, kind)| kind.eq_ignore_ascii_case("TEXT"))
            .map(|(name, _)| name)
            .collect();
        for column in columns {
            conn.execute(
                &format!("UPDATE \"{table}\" SET \"{column}\" = replace(\"{column}\", ?1, ?2) WHERE instr(\"{column}\", ?1) > 0"),
                ["/work/rich/kit", &format!("{}/kit", dir.text())],
            )
            .unwrap();
        }
    }
    drop(conn);
    let conn = open_read_only(&store).unwrap();
    (dir, conn)
}

/// Replays the board and memory-graph goldens of `golden_dir` over `conn`; `work` is the folder `<WORK>` stands for.
fn compare_dir(golden_dir: &str, conn: &Connection, work: &Dir) -> Result<usize, String> {
    let dir = parity_dir().join(golden_dir);
    let manifest: Value = serde_json::from_str(&read_text(&dir.join("manifest.json"))).unwrap();
    let mut compared = 0;
    for entry in manifest["entries"].as_array().unwrap() {
        let route = entry["route"].as_str().unwrap_or("");
        if entry["surface"] != "http" || !matches!(route, "/api/board" | "/api/memory-graph") {
            continue;
        }
        let file = entry["file"].as_str().unwrap();
        let params = &entry["params"];
        let param = |key: &str| {
            params[key]
                .as_str()
                .filter(|v| !v.is_empty())
                .map(|v| v.replace("<WORK>", work.text()))
        };
        let (status, body) = if route == "/api/board" {
            let (workspace, account) = (param("workspace"), param("account"));
            outcome(board(
                conn,
                Scope {
                    workspace: workspace.as_deref(),
                    account: account.as_deref(),
                },
            ))
        } else {
            match param("project") {
                Some(project) => outcome(project_graph(conn, &work.env(), &project)),
                None => outcome(project_list(conn, &work.env())),
            }
        };
        let want_status = entry["status"].as_u64().map_or(200, |s| s as u16);
        if status != want_status {
            return Err(format!("{file}: status {status}, want {want_status}"));
        }
        let golden: Value =
            serde_json::from_str(&read_text(&dir.join(file)).replace("<WORK>", work.text()))
                .unwrap();
        if body != golden {
            return Err(format!("{file}: differs\n  got  {body}\n  want {golden}"));
        }
        compared += 1;
    }
    Ok(compared)
}

#[test]
fn the_ten_rich_goldens_equal_the_answers() {
    let (work, conn) = rich_work();
    assert_eq!(compare_dir("golden-rich", &conn, &work).unwrap(), 10);
}

#[test]
fn the_two_default_goldens_equal_the_answers() {
    let work = Dir::new();
    let conn = open_read_only(&parity_dir().join("store-v11.sqlite")).unwrap();
    assert_eq!(compare_dir("golden", &conn, &work).unwrap(), 2);
}

#[test]
fn a_changed_note_breaks_the_golden_comparison_and_names_the_entry() {
    let (work, conn) = rich_work();
    fs::write(
        work.0.join("kit/alpha/memory/glossary.md"),
        "---\nname: renamed\n---\n",
    )
    .unwrap();
    let error = compare_dir("golden-rich", &conn, &work).unwrap_err();
    assert!(error.contains("api__memory_graph__"), "{error}");
}

// ---- 2. the hostile tree against Python ----

/// `gen_memgraph.py` `build`, in Rust.
fn build(t: &Path) {
    let fix = parity_dir().join("fixtures-memgraph");
    let enc: String = format!("{}/proj", t.to_str().unwrap())
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    copy_tree(&fix.join("proj"), &t.join("proj"));
    let claude = t.join("acct/projects");
    copy_tree(
        &fix.join("claude-memory"),
        &claude.join(&enc).join("memory"),
    );
    copy_tree(&fix.join("claude-other"), &claude.join("zzother"));
    copy_tree(&fix.join("claude-bare"), &claude.join("claude-bare"));
    let memory = t.join("proj/memory");
    fs::write(memory.join("big.md"), vec![b'x'; 1024 * 1024 + 1]).unwrap();
    fs::write(memory.join("bad.md"), b"---\nname: bad\n---\n\xff\xfe\n").unwrap();
    fs::create_dir_all(t.join("outside/dir")).unwrap();
    fs::write(t.join("outside/secret.md"), "---\nname: secret\n---\n").unwrap();
    fs::write(t.join("outside/dir/d.md"), "---\nname: d\n---\n").unwrap();
    std::os::unix::fs::symlink(t.join("outside/secret.md"), memory.join("link-out.md")).unwrap();
    std::os::unix::fs::symlink(t.join("outside/dir"), memory.join("link-dir")).unwrap();
    fs::create_dir_all(t.join("proj600/memory")).unwrap();
    for i in 0..600 {
        fs::write(
            t.join(format!("proj600/memory/n{i:03}.md")),
            format!(
                "---\nname: note-{i}\n---\n[[note-{}]] [[gone-{}]]\n",
                (i + 1) % 600,
                i % 7
            ),
        )
        .unwrap();
    }
    fs::create_dir_all(t.join("projcap/memory")).unwrap();
    for i in 0..5001 {
        fs::write(t.join(format!("projcap/memory/c{i:04}.md")), "x\n").unwrap();
    }
    let boards = t.join("boards");
    let ok = "## Board\n### Todo\n- T-1 one\n";
    let long = json!({"board": "b".repeat(60)}).to_string();
    for (name, kanban, board_json) in [
        ("ok", ok, " {\"board\": \"  linear  \"} "),
        ("bad_json", ok, "{not json"),
        ("board_not_str", ok, "{\"board\": 5}"),
        ("board_long", ok, long.as_str()),
        ("no_board_section", "# nothing\n", "{\"board\": \"linear\"}"),
    ] {
        fs::create_dir_all(boards.join(name).join(".claude")).unwrap();
        fs::write(boards.join(name).join("PROJECT_KANBAN.md"), kanban).unwrap();
        fs::write(boards.join(name).join(".claude/board.json"), board_json).unwrap();
    }
    for (name, bytes) in [
        ("over_cap", vec![b'x'; 5_000_001]),
        ("bad_utf8", b"## Board\n### Todo\n- T-1 \xff\n".to_vec()),
        ("at_cap", {
            let mut v = b"## Board\n### Todo\n- T-1 a\n".to_vec();
            v.resize(5_000_000, b' ');
            v
        }),
    ] {
        fs::create_dir_all(boards.join(name)).unwrap();
        fs::write(boards.join(name).join("PROJECT_KANBAN.md"), bytes).unwrap();
    }
    fs::create_dir_all(boards.join("none")).unwrap();
    fs::create_dir_all(boards.join("dir_file/PROJECT_KANBAN.md")).unwrap();
    fs::create_dir_all(boards.join("fifo")).unwrap();
    let made = Command::new("mkfifo")
        .env_clear()
        .arg(boards.join("fifo/PROJECT_KANBAN.md"))
        .status()
        .unwrap();
    assert!(made.success());
    for name in [
        "ok",
        "bad_json",
        "board_not_str",
        "board_long",
        "no_board_section",
        "over_cap",
        "bad_utf8",
        "at_cap",
    ] {
        set_mtime(&boards.join(name).join("PROJECT_KANBAN.md"), RICH_MTIME);
    }
}

fn seeded(t: &Dir) -> Connection {
    build(&t.0);
    let store = t.0.join("store.sqlite");
    fs::copy(parity_dir().join("store-v11.sqlite"), &store).unwrap();
    let seed = read_text(&parity_dir().join("fixtures-memgraph/seed.sql")).replace("@T@", t.text());
    Connection::open(&store)
        .unwrap()
        .execute_batch(&seed)
        .unwrap();
    open_read_only(&store).unwrap()
}

fn cases(t: &Dir) -> Value {
    let text = read_text(&parity_dir().join("golden-memgraph/cases.json")).replace("<T>", t.text());
    serde_json::from_str(&text).unwrap()
}

#[test]
fn the_hostile_tree_gives_python_answers() {
    let t = Dir::new();
    let conn = seeded(&t);
    let want = cases(&t);
    let env = t.env();
    assert_eq!(project_list(&conn, &env).unwrap(), want["list"], "list");
    assert_eq!(
        board(&conn, Scope::default()).unwrap(),
        want["board_all"],
        "board_all"
    );
    for (key, graph) in want["graphs"].as_object().unwrap() {
        let got = project_graph(&conn, &env, key).unwrap();
        assert_eq!(&got, graph, "graph {key}");
    }
    let cap = project_graph(&conn, &env, &format!("{}/projcap", t.text())).unwrap();
    assert_eq!(cap["counts"], want["cap"]["counts"]);
    assert_eq!(cap["nodes"].as_array().unwrap().len(), want["cap"]["nodes"]);
    assert_eq!(cap["edges"].as_array().unwrap().len(), want["cap"]["edges"]);
    assert_eq!(cap["counts"]["capped"], true);
    match project_graph(&conn, &env, "nope") {
        Err(MetricsError::Invalid(text)) => assert_eq!(text, want["unknown"].as_str().unwrap()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn board_files_of_every_kind_give_python_entries() {
    let t = Dir::new();
    build(&t.0);
    let want = cases(&t);
    let boards = want["boards"].as_object().unwrap();
    assert_eq!(boards.len(), 11);
    for (path, entry) in boards {
        assert_eq!(&read_project(path), entry, "{path}");
    }
}

#[test]
fn kanban_texts_parse_as_python_does() {
    let t = Dir::new();
    let want = cases(&t);
    let all = want["parse"].as_array().unwrap();
    assert_eq!(all.len(), 14);
    for case in all {
        let got = parse_text(case["text"].as_str().unwrap()).unwrap_or(Value::Null);
        assert_eq!(got, case["result"], "{}", case["name"]);
    }
}

// ---- 3. Rust-only cases ----

#[test]
fn a_symlink_inside_the_memory_folder_is_skipped_unlike_python() {
    let t = Dir::new();
    let conn = seeded(&t);
    let memory = t.0.join("proj/memory");
    std::os::unix::fs::symlink("decisions.md", memory.join("aaa-link.md")).unwrap();
    let graph = project_graph(&conn, &t.env(), &format!("{}/proj", t.text())).unwrap();
    let paths: Vec<&str> = graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n["path"].as_str())
        .collect();
    assert!(!paths.contains(&"memory/aaa-link.md"), "{paths:?}");
    assert!(paths.contains(&"memory/decisions.md"));
}

#[test]
fn a_symlinked_kanban_or_board_file_is_no_data_and_no_board() {
    let t = Dir::new();
    let real = t.0.join("real.md");
    fs::write(&real, "## Board\n### Todo\n- T-1 one\n").unwrap();
    fs::create_dir_all(t.0.join("p/.claude")).unwrap();
    std::os::unix::fs::symlink(&real, t.0.join("p/PROJECT_KANBAN.md")).unwrap();
    assert_eq!(
        read_project(&format!("{}/p", t.text()))["status"],
        "no data"
    );
    fs::remove_file(t.0.join("p/PROJECT_KANBAN.md")).unwrap();
    fs::copy(&real, t.0.join("p/PROJECT_KANBAN.md")).unwrap();
    fs::write(t.0.join("real.json"), "{\"board\": \"linear\"}").unwrap();
    std::os::unix::fs::symlink(t.0.join("real.json"), t.0.join("p/.claude/board.json")).unwrap();
    let entry = read_project(&format!("{}/p", t.text()));
    assert_eq!(
        (entry["status"].as_str(), entry["board"].is_null()),
        (Some("ok"), true)
    );
}

#[test]
fn a_huge_kanban_is_read_in_time() {
    let mut text = String::from("## Board\n### Todo\n");
    for i in 0..100_000 {
        text += &format!("- T-{i} task\n");
    }
    let parsed = parse_text(&text).unwrap();
    assert_eq!(parsed["tasks"].as_array().unwrap().len(), 100_000);
    let line = format!("## Board\n### Todo\n- T-1 {}\n", "x ".repeat(500_000));
    assert_eq!(
        parse_text(&line).unwrap()["tasks"][0]["title"]
            .as_str()
            .unwrap()
            .len(),
        300
    );
}

#[test]
fn an_unknown_scope_is_the_fixed_400_text() {
    let conn = open_read_only(&parity_dir().join("store-v11.sqlite")).unwrap();
    for scope in [
        Scope {
            workspace: Some("nope"),
            account: None,
        },
        Scope {
            workspace: None,
            account: Some("nope"),
        },
    ] {
        match board(&conn, scope) {
            Err(MetricsError::Invalid(text)) => assert!(!text.contains("nope"), "{text}"),
            other => panic!("{other:?}"),
        }
    }
}
