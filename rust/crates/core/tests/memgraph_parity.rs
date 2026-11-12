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
//! 4. T053: `golden-memgraph/cases-t053.json`, Python's answers over a second tree (`build_t053`): hostile notes, two Claude
//!    folders that resolve to one project (duplicate node ids), a worktree `.git`, a store path through a link and `..`, and
//!    an easy-memory store that holds notes (cited files, `prompts.log`, `report.json`, `trash/`, BOMs), the easy-memory
//!    panel under four time zones, and the hostile `.git` / link-count / limit cases that only Rust answers.
//!
//! The owner's real `~/.easy-personal`, `~/.claude*` and `~/.easy-memory` are never opened: every folder is a temp one.

use easy_personal_core::board::{board, parse_text, read_project};
use easy_personal_core::easymem;
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
    let mut want: Value = serde_json::from_str(&text).unwrap();
    sort_like_python(&mut want["list"]["projects"]);
    want
}

/// Python lists the projects sorted by key text (`sorted(_sources(..).items())`). The golden was recorded under
/// `/tmp/easy-personal-t043-gen-*` and `<T>` is put back as this run's temp folder, which sorts before or after
/// `/nonexistent-t043/zz` depending on `TMPDIR`: the same rows, sorted by Python's own rule for this path (T055).
fn sort_like_python(projects: &mut Value) {
    let rows = projects.as_array_mut().unwrap();
    rows.sort_by(|a, b| {
        a["project"]
            .as_str()
            .unwrap()
            .cmp(b["project"].as_str().unwrap())
    });
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
    assert_eq!(all.len(), 17);
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
    let started = std::time::Instant::now();
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
    // a regex that restarts a search from every `[` would take minutes on these inputs; the debug build needs about a second
    let took = started.elapsed();
    assert!(took < Duration::from_secs(20), "took {took:?}");
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

// ---- 4. T053: the easy-memory layer and the review fixes ----

/// The first place two JSON values differ, for a short failure message.
fn first_difference(got: &Value, want: &Value, at: &str) -> String {
    match (got, want) {
        (Value::Object(a), Value::Object(b)) => {
            for key in a.keys().chain(b.keys()) {
                let (x, y) = (a.get(key), b.get(key));
                if x != y {
                    return first_difference(
                        x.unwrap_or(&Value::Null),
                        y.unwrap_or(&Value::Null),
                        &format!("{at}.{key}"),
                    );
                }
            }
            String::new()
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                if x != y {
                    return first_difference(x, y, &format!("{at}[{i}]"));
                }
            }
            String::new()
        }
        _ => format!("{at}: got {got}, want {want}"),
    }
}

fn mkfifo(path: &Path) {
    let made = Command::new("mkfifo")
        .env_clear()
        .arg(path)
        .status()
        .unwrap();
    assert!(made.success());
}

fn encode(path: &Path) -> String {
    path.to_str()
        .unwrap()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// A `prompts.log` line of exactly `total` bytes, newline included.
fn log_line(total: usize) -> Vec<u8> {
    let head = br#"{"time":"2026-01-19T12:00:00Z","effective":"on","injected":true,"pad":""#;
    let tail = b"\"}\n";
    let mut line = head.to_vec();
    line.resize(total - tail.len(), b'x');
    line.extend_from_slice(tail);
    line
}

/// `gen_memgraph.py` `build_t053`, in Rust: the tree is `<t>/t053`.
fn build_t053(t: &Path) {
    let fix = parity_dir().join("fixtures-memgraph");
    let t2 = t.join("t053");
    let projh = t2.join("projh/memory");
    fs::create_dir_all(&projh).unwrap();
    fs::write(
        projh.join("plain.md"),
        "---\nname: plain\ndescription: ok\n---\n[[nul-note]] [[big]] [[nested]]\n",
    )
    .unwrap();
    fs::write(
        projh.join("nul.md"),
        "---\nname: a\0b\ndescription: nul\n---\nbody [[plain]]\n",
    )
    .unwrap();
    fs::write(
        projh.join("bigdesc.md"),
        format!(
            "---\nname: big\ndescription: {}\n---\n",
            "d".repeat(100_000)
        ),
    )
    .unwrap();
    fs::write(
        projh.join("nested.md"),
        "---\nname:\n  first: 1\n  second: [a, b]\ndescription:\n  deep:\n    deeper: x\n---\n[[plain]]\n",
    )
    .unwrap();
    mkfifo(&projh.join("fifo.md"));
    let dup = t2.join("projdup");
    fs::create_dir_all(&dup).unwrap();
    let projects = t2.join("acct/projects");
    let first = projects.join(encode(&dup)).join("memory");
    fs::create_dir_all(&first).unwrap();
    fs::write(first.join("MEMORY.md"), "---\nname: idx-a\n---\n[[c1]]\n").unwrap();
    fs::write(first.join("c1.md"), "---\nname: c1\n---\n").unwrap();
    fs::create_dir_all(projects.join("zzsub/memory")).unwrap();
    fs::write(
        projects.join("zzsub/memory/MEMORY.md"),
        "---\nname: idx-b\n---\n",
    )
    .unwrap();
    fs::write(
        projects.join("zzsub/t.jsonl"),
        format!("{{\"cwd\": \"{}\"}}\n", dup.display()),
    )
    .unwrap();
    copy_tree(&fix.join("projem"), &t2.join("projem"));
    let root = t2.join("projem");
    let link = std::os::unix::fs::symlink;
    link("missing.txt", root.join("dang")).unwrap();
    link("/etc/nowhere", root.join("out")).unwrap();
    link("src/a.py", root.join("ln")).unwrap();
    link("src", root.join("linkdir")).unwrap();
    mkfifo(&root.join("pipe"));
    fs::File::create(root.join("big.bin"))
        .unwrap()
        .set_len(10 * 1024 * 1024 + 1)
        .unwrap();
    fs::create_dir_all(t2.join("projwt")).unwrap();
    fs::write(
        t2.join("projwt/.git"),
        format!("gitdir: {}/gd\n", t2.display()),
    )
    .unwrap();
    fs::create_dir_all(t2.join("gd")).unwrap();
    fs::write(t2.join("gd/commondir"), "../common\n").unwrap();
    fs::create_dir_all(t2.join("common")).unwrap();
    fs::create_dir_all(t2.join("projdd/sub")).unwrap();
    link("projdd/sub", t2.join("lnk")).unwrap();
    copy_tree(&fix.join("em-store"), &t2.join("em"));
    for key in ["k1", "k2", "k3"] {
        let meta = t2.join("em").join(key).join("meta.json");
        let bytes = fs::read(&meta).unwrap();
        let at = bytes.windows(3).position(|w| w == b"@T@").unwrap();
        let mut replaced = bytes[..at].to_vec();
        replaced.extend_from_slice(t.to_str().unwrap().as_bytes());
        replaced.extend_from_slice(&bytes[at + 3..]);
        fs::write(&meta, replaced).unwrap();
    }
    fs::create_dir(t2.join("em/k1/notes/x.json")).unwrap();
    let mut log = fs::OpenOptions::new()
        .append(true)
        .open(t2.join("em/k1/prompts.log"))
        .unwrap();
    for total in [65537, 65538, 200_000] {
        std::io::Write::write_all(&mut log, &log_line(total)).unwrap();
    }
    std::io::Write::write_all(
        &mut log,
        b"{\"time\":\"2026-01-19T15:00:00Z\",\"effective\":\"on\",\"injected\":true}\n",
    )
    .unwrap();
}

/// A temp copy of the default store with `events` rows for `projects` and the confirmed Claude account `acct`.
fn store_with(t: &Dir, acct: Option<&Path>, projects: &[String]) -> Connection {
    let store = t.0.join("store-t053.sqlite");
    fs::copy(parity_dir().join("store-v11.sqlite"), &store).unwrap();
    let mut sql = String::from("DELETE FROM events; DELETE FROM agent_accounts;");
    if let Some(acct) = acct {
        sql += &format!(
            "INSERT INTO agent_accounts (id, provider, folder, label, status) VALUES (1, 'claude', '{}', 'W', 'confirmed');",
            acct.display()
        );
    }
    for (i, project) in projects.iter().enumerate() {
        sql += &format!(
            "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, project_path) VALUES ('m', 'claude_code', '/x/{i}.jsonl', {i}, 0, 0, '{project}');"
        );
    }
    Connection::open(&store)
        .unwrap()
        .execute_batch(&sql)
        .unwrap();
    open_read_only(&store).unwrap()
}

fn seeded_t053(t: &Dir) -> Connection {
    build_t053(&t.0);
    let store = t.0.join("store-t053.sqlite");
    fs::copy(parity_dir().join("store-v11.sqlite"), &store).unwrap();
    let seed =
        read_text(&parity_dir().join("fixtures-memgraph/seed-t053.sql")).replace("@T@", t.text());
    Connection::open(&store)
        .unwrap()
        .execute_batch(&seed)
        .unwrap();
    open_read_only(&store).unwrap()
}

fn cases_t053(t: &Dir) -> Value {
    let text =
        read_text(&parity_dir().join("golden-memgraph/cases-t053.json")).replace("<T>", t.text());
    let mut want: Value = serde_json::from_str(&text).unwrap();
    if want["list"]["projects"].is_array() {
        sort_like_python(&mut want["list"]["projects"]);
    }
    want
}

fn env_t053(t: &Dir, zone: TimeZone) -> Env {
    Env {
        em_home: t.0.join("t053/em"),
        zone,
        now_micros: FROZEN_NOW,
    }
}

#[test]
fn the_t053_tree_gives_python_answers() {
    let t = Dir::new();
    let conn = seeded_t053(&t);
    let want = cases_t053(&t);
    let env = env_t053(&t, TimeZone::UTC);
    let list = project_list(&conn, &env).unwrap();
    assert!(
        list == want["list"],
        "list: {}",
        first_difference(&list, &want["list"], "")
    );
    let graphs = want["graphs"].as_object().unwrap();
    assert_eq!(graphs.len(), 5);
    for (key, graph) in graphs {
        let got = project_graph(&conn, &env, key).unwrap();
        assert!(
            &got == graph,
            "graph {key}: {}",
            first_difference(&got, graph, "")
        );
    }
}

#[test]
fn the_easy_memory_panel_is_python_s_under_four_time_zones() {
    let t = Dir::new();
    let conn = seeded_t053(&t);
    let want = cases_t053(&t);
    let project = format!("{}/t053/projem", t.text());
    let zones = want["zones"].as_object().unwrap();
    assert_eq!(zones.len(), 4);
    let mut days = std::collections::BTreeSet::new();
    for (name, panel) in zones {
        let zone = TimeZone::get(name).unwrap_or_else(|e| panic!("{name}: {e}"));
        let graph = project_graph(&conn, &env_t053(&t, zone), &project).unwrap();
        let got = &graph["easy_memory"];
        assert!(
            got == panel,
            "zone {name}: {}",
            first_difference(got, panel, "")
        );
        days.insert(graph["easy_memory"]["days"].to_string());
    }
    // "now" is 2026-01-20T00:00Z and the notes sit at 09:59:59Z..11:00:00Z: every zone has its own window and local days,
    // so the three `TZ` runs of the suite (which do not reach `Env { zone }`) were vacuous for this slice
    assert_eq!(days.len(), 4, "each zone gives its own day list");
}

#[test]
fn no_prompt_answer_or_session_text_leaves_the_easy_memory_layer() {
    let t = Dir::new();
    let conn = seeded_t053(&t);
    let env = env_t053(&t, TimeZone::UTC);
    let mut all = project_list(&conn, &env).unwrap().to_string();
    for name in ["projem", "projwt", "projdd", "projh", "projdup"] {
        let key = format!("{}/t053/{name}", t.text());
        all += &project_graph(&conn, &env, &key).unwrap().to_string();
    }
    assert!(all.contains("easy_memory"), "the layer is in the answers");
    assert!(
        !all.contains("CANARY"),
        "a planted easy-memory text came out"
    );
}

#[test]
fn an_easy_memory_store_without_a_note_is_no_layer() {
    let t = Dir::new();
    let conn = seeded_t053(&t);
    let key = format!("{}/t053/projdup", t.text());
    let graph = project_graph(&conn, &env_t053(&t, TimeZone::UTC), &key).unwrap();
    assert!(graph["easy_memory"].is_null());
}

#[test]
fn duplicate_node_ids_put_the_counts_on_the_later_node() {
    let t = Dir::new();
    let conn = seeded_t053(&t);
    let key = format!("{}/t053/projdup", t.text());
    let graph = project_graph(&conn, &env_t053(&t, TimeZone::UTC), &key).unwrap();
    let rows: Vec<(&str, u64, u64)> = graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| {
            (
                n["name"].as_str().unwrap(),
                n["links_in"].as_u64().unwrap(),
                n["links_out"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(rows, [("idx-a", 0, 0), ("c1", 1, 0), ("idx-b", 0, 1)]);
}

#[test]
fn a_store_path_through_a_link_and_dot_dot_is_resolved_link_first() {
    let t = Dir::new();
    let conn = seeded_t053(&t);
    let list = project_list(&conn, &env_t053(&t, TimeZone::UTC)).unwrap();
    let row = list["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "projdd")
        .expect("projdd has an easy-memory note through `lnk/..`");
    assert_eq!(row["easy_memory_notes"], 1);
}

#[test]
fn a_dangling_cited_link_is_gone_and_one_that_leaves_the_project_is_outside() {
    let t = Dir::new();
    let conn = seeded_t053(&t);
    let key = format!("{}/t053/projem", t.text());
    let graph = project_graph(&conn, &env_t053(&t, TimeZone::UTC), &key).unwrap();
    let nodes = graph["nodes"].as_array().unwrap();
    let state = |path: &str| {
        nodes
            .iter()
            .find(|n| n["path"] == path)
            .map(|n| n["stale"].clone())
    };
    assert_eq!(state("dang"), Some(json!("gone")));
    assert_eq!(state("out"), None, "a link out of the project is left out");
    assert!(graph["easy_memory"]["outside"].as_u64().unwrap() >= 3);
}

/// Runs `work` in a thread and waits `secs` for its answer: a hang is a test failure, not a stuck run.
fn within<T: Send + 'static>(secs: u64, work: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(work());
    });
    rx.recv_timeout(Duration::from_secs(secs))
        .expect("did not return in time (a blocking open?)")
}

/// Bytes this thread has asked the kernel to read (`rchar` of `/proc/thread-self/io`).
fn read_bytes() -> u64 {
    fs::read_to_string("/proc/thread-self/io")
        .unwrap()
        .lines()
        .find_map(|l| l.strip_prefix("rchar: "))
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

/// An easy-memory store whose one folder is keyed by `project`.
fn store_of(t: &Dir, project: &Path) -> easymem::Store {
    let home = t.0.join("em-hostile");
    fs::create_dir_all(home.join("k1")).unwrap();
    fs::write(
        home.join("k1/meta.json"),
        format!("{{\"path\": \"{}\"}}", project.display()),
    )
    .unwrap();
    easymem::load(&home)
}

/// A project whose `.git` file names a gitdir that holds `commondir`; `make` creates the `commondir`.
fn worktree_with(t: &Dir, make: impl FnOnce(&Path)) -> (PathBuf, easymem::Store) {
    let project = t.0.join("wt");
    let gitdir = t.0.join("gd");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&gitdir).unwrap();
    fs::write(
        project.join(".git"),
        format!("gitdir: {}\n", gitdir.display()),
    )
    .unwrap();
    make(&gitdir.join("commondir"));
    // keyed by a folder that does not exist: only a `commondir` that names it could match the project
    let store = store_of(t, &t.0.join("nowhere"));
    (project, store)
}

#[test]
fn a_dot_git_that_is_a_fifo_does_not_block() {
    let t = Dir::new();
    let project = t.0.join("p");
    fs::create_dir_all(&project).unwrap();
    mkfifo(&project.join(".git"));
    let store = store_of(&t, &project);
    let key = project.to_str().unwrap().to_string();
    let found = within(5, move || store.find(&key));
    assert!(found.is_some(), "the store path still matches the folder");
}

#[test]
fn a_commondir_is_read_for_at_most_4096_bytes() {
    let t = Dir::new();
    let (project, store) = worktree_with(&t, |path| {
        fs::File::create(path)
            .unwrap()
            .set_len(100 * 1024 * 1024)
            .unwrap();
    });
    let before = read_bytes();
    let found = store.find(project.to_str().unwrap());
    let read = read_bytes() - before;
    assert!(
        found.is_none(),
        "a commondir of NUL bytes names no store folder"
    );
    assert!(read <= 16 * 1024, "read {read} bytes of a 100 MB commondir");
}

#[test]
fn a_commondir_that_is_a_link_to_dev_zero_is_refused_without_a_read() {
    let t = Dir::new();
    let (project, store) = worktree_with(&t, |path| {
        std::os::unix::fs::symlink("/dev/zero", path).unwrap();
    });
    let key = project.to_str().unwrap().to_string();
    let (found, read) = within(10, move || {
        let before = read_bytes();
        (store.find(&key), read_bytes() - before)
    });
    assert!(found.is_none());
    assert!(
        read <= 16 * 1024,
        "read {read} bytes through a link to /dev/zero"
    );
}

#[test]
fn a_worktree_commondir_still_names_the_repo_it_belongs_to() {
    let t = Dir::new();
    let common = t.0.join("common");
    fs::create_dir_all(&common).unwrap();
    let (project, _) = worktree_with(&t, |path| {
        fs::write(path, "../common\n").unwrap();
    });
    // the store is keyed by the repo folder, so the worktree is found through its `commondir`
    let home = t.0.join("em-wt");
    fs::create_dir_all(home.join("k9")).unwrap();
    fs::write(
        home.join("k9/meta.json"),
        format!("{{\"path\": \"{}\"}}", common.display()),
    )
    .unwrap();
    let by_common = easymem::load(&home);
    assert!(by_common.find(project.to_str().unwrap()).is_some());
}

/// 5,000 notes with 40 distinct `[[links]]` each: 200,000 edges, each counted on both ends.
#[test]
fn two_hundred_thousand_links_are_counted_in_time() {
    let t = Dir::new();
    let project = t.0.join("pbig");
    fs::create_dir_all(project.join("memory")).unwrap();
    for i in 0..5000 {
        let links: String = (1..=40)
            .map(|k| format!("[[n{}]] ", (i + k) % 5000))
            .collect();
        fs::write(
            project.join(format!("memory/n{i}.md")),
            format!("---\nname: n{i}\n---\n{links}\n"),
        )
        .unwrap();
    }
    let conn = store_with(&t, None, &[project.to_str().unwrap().to_string()]);
    let started = std::time::Instant::now();
    let graph = project_graph(&conn, &t.env(), project.to_str().unwrap()).unwrap();
    let took = started.elapsed();
    assert_eq!(graph["counts"]["links"], 200_000);
    let nodes = graph["nodes"].as_array().unwrap();
    assert!(
        nodes
            .iter()
            .all(|n| n["links_in"] == 40 && n["links_out"] == 40)
    );
    assert!(took < Duration::from_secs(5), "took {took:?}");
    eprintln!("200,000 links: {took:?}");
}

// ---- 5. T053: Rust-only limits and refusals (no Python answer: Python follows, hangs or crashes) ----

fn paths_of(graph: &Value) -> Vec<String> {
    graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n["path"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn a_hard_linked_note_is_listed_once() {
    // Python lists both names (it dedupes by real path, and a hard link has two); Rust reads one inode once (decided for you)
    let t = Dir::new();
    let project = t.0.join("ph");
    fs::create_dir_all(project.join("memory")).unwrap();
    fs::write(project.join("memory/a.md"), "---\nname: a\n---\n").unwrap();
    fs::hard_link(project.join("memory/a.md"), project.join("memory/b.md")).unwrap();
    let key = project.to_str().unwrap().to_string();
    let conn = store_with(&t, None, std::slice::from_ref(&key));
    let graph = project_graph(&conn, &t.env(), &key).unwrap();
    assert_eq!(paths_of(&graph), ["memory/a.md"]);
}

#[test]
fn a_folder_deeper_than_64_levels_is_not_entered() {
    let t = Dir::new();
    let project = t.0.join("pd");
    let mut deep = project.join("memory");
    for level in 1..=65 {
        deep = deep.join("d");
        fs::create_dir_all(&deep).unwrap();
        if level >= 64 {
            fs::write(deep.join(format!("n{level}.md")), "---\nname: x\n---\n").unwrap();
        }
    }
    let key = project.to_str().unwrap().to_string();
    let conn = store_with(&t, None, std::slice::from_ref(&key));
    let graph = project_graph(&conn, &t.env(), &key).unwrap();
    let paths = paths_of(&graph);
    assert_eq!(paths.len(), 1, "{paths:?}");
    assert!(paths[0].ends_with("/n64.md"), "{paths:?}");
}

#[test]
fn a_name_that_is_not_utf8_is_skipped_and_the_rest_is_read() {
    use std::os::unix::ffi::OsStrExt;
    let t = Dir::new();
    let project = t.0.join("pu");
    fs::create_dir_all(project.join("memory")).unwrap();
    fs::write(project.join("memory/ok.md"), "---\nname: ok\n---\n").unwrap();
    fs::write(
        project
            .join("memory")
            .join(std::ffi::OsStr::from_bytes(b"\xff.md")),
        "---\nname: bad\n---\n",
    )
    .unwrap();
    let dir = project
        .join("memory")
        .join(std::ffi::OsStr::from_bytes(b"\xfe"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("inner.md"), "---\nname: inner\n---\n").unwrap();
    let key = project.to_str().unwrap().to_string();
    let conn = store_with(&t, None, std::slice::from_ref(&key));
    let graph = project_graph(&conn, &t.env(), &key).unwrap();
    assert_eq!(paths_of(&graph), ["memory/ok.md"]);
}

#[test]
fn a_symlinked_memory_folder_or_projects_folder_is_not_a_source() {
    let t = Dir::new();
    let (project, real) = (t.0.join("pl"), t.0.join("real-memory"));
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&real).unwrap();
    fs::write(real.join("a.md"), "---\nname: a\n---\n").unwrap();
    std::os::unix::fs::symlink(&real, project.join("memory")).unwrap();
    let key = project.to_str().unwrap().to_string();
    // a Claude account whose `projects` folder is a link to a tree that holds a memory folder for the same project
    let elsewhere = t.0.join("elsewhere");
    fs::create_dir_all(elsewhere.join(encode(&project)).join("memory")).unwrap();
    fs::write(
        elsewhere.join(encode(&project)).join("memory/c.md"),
        "---\nname: c\n---\n",
    )
    .unwrap();
    fs::create_dir_all(t.0.join("acct")).unwrap();
    std::os::unix::fs::symlink(&elsewhere, t.0.join("acct/projects")).unwrap();
    let conn = store_with(&t, Some(&t.0.join("acct")), std::slice::from_ref(&key));
    let list = project_list(&conn, &t.env()).unwrap();
    assert_eq!(list["projects"], json!([]), "{list}");
    match project_graph(&conn, &t.env(), &key) {
        Err(MetricsError::Invalid(text)) => assert_eq!(text, "unknown project"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_board_json_that_is_not_plain_json_names_no_board() {
    let t = Dir::new();
    for (name, json_text) in [
        ("nan", "{\"board\": NaN}".to_string()),
        ("nested", format!("{}1{}", "[".repeat(300), "]".repeat(300))),
        (
            "deep_object",
            format!("{}{}", "{\"board\":".repeat(300), "1"),
        ),
    ] {
        let project = t.0.join(name);
        fs::create_dir_all(project.join(".claude")).unwrap();
        fs::write(
            project.join("PROJECT_KANBAN.md"),
            "## Board\n### Todo\n- T-1 one\n",
        )
        .unwrap();
        fs::write(project.join(".claude/board.json"), json_text).unwrap();
        let entry = read_project(project.to_str().unwrap());
        assert_eq!(entry["status"], "ok", "{name}");
        assert!(
            entry["board"].is_null() && entry["last_pull"].is_null(),
            "{name}: {entry}"
        );
    }
}
