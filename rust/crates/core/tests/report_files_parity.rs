//! T045: `report_files::report_files` and `report_files::report_file` against Python.
//!
//! 1. The 9 goldens of `rust/parity/golden-rich` (`api__kit_report_files__*` 5, `api__kit_report_file__*` 4), over a temp
//!    copy of the rich store and of `fixtures-rich/kit`.
//! 2. The attack table: `rust/parity/golden-report-files/` (made by `gen_report_files.py` with Python's own functions)
//!    holds Python's answer to 53 requests over an attack tree (`tree.json`: symlinks out and in, a hard link, a FIFO, a
//!    folder named like a file, files of the cap, cap + 1 and 100 MB, odd names, odd `repo_no`s). This test builds the same
//!    tree in a temp folder and compares every case. The cases where Rust refuses what Python serves, or lists what
//!    Python crashes on, are named in `DECIDED` and asserted as such. Nothing is read outside the temp folder: the "secret"
//!    is a canary file the test creates beside the repository, and no answer may ever hold it.
//! 3. A file swapped for a symlink after the listing is refused (the race), a FIFO never blocks, the cap is a read limit.

use easy_personal_core::report_files::{
    ReportError, parse_repo_no, report_file, report_file_capped, report_files,
};
use easy_personal_core::store::open_read_only;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::ffi::{CString, OsString};
use std::fs;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, UNIX_EPOCH};

const MTIME: u64 = 1_768_694_400; // 2026-01-18T00:00:00Z, the goldens' file time

unsafe extern "C" {
    fn mkfifo(path: *const std::ffi::c_char, mode: u32) -> i32;
}

fn parity_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity")
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap_or_else(|e| panic!("{path:?}: {e}")))
        .unwrap()
}

fn set_mtime(path: &Path) {
    fs::File::open(path)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_secs(MTIME))
        .unwrap();
}

/// A temp folder removed on drop.
struct Work {
    dir: PathBuf,
}

impl Work {
    fn new() -> Work {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t045-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Work { dir }
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
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

/// A store (a temp copy of the empty v11 store) with workspace `acme` holding `repo`.
fn attack_store(work: &Work, repo: &Path) -> Connection {
    let path = work.dir.join("store.sqlite");
    fs::copy(parity_dir().join("store-v11.sqlite"), &path).unwrap();
    let conn = Connection::open(&path).unwrap();
    conn.execute("INSERT INTO workspaces (name) VALUES ('acme')", [])
        .unwrap();
    conn.execute(
        "INSERT INTO workspace_repos (workspace_id, repo_path) VALUES (last_insert_rowid(), ?1)",
        [repo.to_str().unwrap()],
    )
    .unwrap();
    drop(conn);
    open_read_only(&path).unwrap()
}

/// The attack tree of `tree.json` under `root`; returns the repository folder. `@OUT` is the folder outside it.
fn build_tree(root: &Path, spec: &Value) -> PathBuf {
    let entries = spec["outside"]
        .as_array()
        .unwrap()
        .iter()
        .chain(spec["tree"].as_array().unwrap());
    for entry in entries {
        let rel: Vec<u8> = match entry["p_hex"].as_str() {
            Some(hex) => (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect(),
            None => entry["p"].as_str().unwrap().as_bytes().to_vec(),
        };
        let outside = rel.starts_with(b"outside/");
        let path = root
            .join(if outside { "" } else { "repo" })
            .join(OsString::from_vec(rel));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let target = entry["to"]
            .as_str()
            .unwrap_or("")
            .replace("@OUT", root.join("outside").to_str().unwrap());
        match entry["t"].as_str().unwrap() {
            "file" => fs::write(&path, entry["text"].as_str().unwrap()).unwrap(),
            "fill" => {
                fs::write(&path, vec![b'a'; entry["size"].as_u64().unwrap() as usize]).unwrap()
            }
            "sparse" => fs::File::create(&path)
                .unwrap()
                .set_len(entry["size"].as_u64().unwrap())
                .unwrap(),
            "symlink" => std::os::unix::fs::symlink(&target, &path).unwrap(),
            "hardlink" => {
                let from = if target.starts_with('/') {
                    PathBuf::from(&target)
                } else {
                    root.join("repo").join(&target)
                };
                fs::hard_link(from, &path).unwrap();
            }
            "fifo" => {
                let c = CString::new(path.as_os_str().as_bytes()).unwrap();
                assert_eq!(unsafe { mkfifo(c.as_ptr(), 0o600) }, 0);
            }
            "dir" => fs::create_dir_all(&path).unwrap(),
            other => panic!("{other}"),
        }
        if matches!(entry["t"].as_str().unwrap(), "file" | "fill" | "sparse") {
            set_mtime(&path);
        }
    }
    root.join("repo")
}

/// What the core answers, as `(status, body)`; a served file is `{content_type, size, sha256-less}` described by the caller.
enum Answer {
    List(u16, Value),
    File(u16, Option<(&'static str, Vec<u8>)>),
}

fn status(error: &ReportError) -> u16 {
    match error {
        ReportError::NotFound => 404,
        ReportError::TooLarge => 413,
        ReportError::Store(_) => 503,
    }
}

fn ask(conn: &Connection, case: &Value) -> Answer {
    let q = &case["query"];
    let text = |key: &str| q[key].as_str();
    let (workspace, folder) = (text("workspace"), text("folder"));
    let repo_no = text("repo_no").and_then(parse_repo_no);
    if case["route"] == "files" {
        match report_files(conn, workspace, repo_no, folder) {
            Ok(body) => Answer::List(200, body),
            Err(e) => Answer::List(status(&e), json!({"error": "not found"})),
        }
    } else {
        match report_file(conn, workspace, repo_no, folder, text("path")) {
            Ok(file) => Answer::File(200, Some(file)),
            Err(e) => Answer::File(status(&e), None),
        }
    }
}

/// The answers Rust gives on purpose where Python's differ (listed in `rust/README.md`, "decided for you").
const DECIDED: [&str; 5] = [
    "file_hardlink_out", // Python serves the outside file through a second link to it
    "file_hardlink_in", // Python serves it; a file with a second link is refused (404) whatever it points at
    "file_hardlink_in_origin", // the same file under its first name
    "file_dotfile_with_ext", // a part that starts with a dot is refused before any file system call
    "files_bytes_reports", // Python: 500 (its JSON encoder fails on the name); Rust leaves the name out
];

#[test]
fn the_attack_table_equals_pythons_answers_except_the_decided_cases() {
    let spec = read_json(&parity_dir().join("golden-report-files/tree.json"));
    let golden = read_json(&parity_dir().join("golden-report-files/cases.json"));
    let canary = spec["canary"].as_str().unwrap().to_string();
    let work = Work::new();
    let repo = build_tree(&work.dir, &spec);
    let conn = attack_store(&work, &repo);
    let (mut equal, mut decided, mut total) = (0, 0, 0);
    for case in golden["cases"].as_array().unwrap() {
        total += 1;
        let id = case["id"].as_str().unwrap();
        let is_decided = DECIDED.contains(&id);
        let answer = ask(&conn, case);
        let line = format!("{id}: {case}");
        match answer {
            Answer::List(status, body) => {
                assert!(!body.to_string().contains(&canary), "{id}: canary");
                if id == "files_bytes_reports" {
                    assert_eq!(case["status"], "python_crash");
                    assert_eq!(status, 200, "{id}");
                    let paths: Vec<&str> = body["files"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|f| f["path"].as_str().unwrap())
                        .collect();
                    assert_eq!(paths, ["ok.txt"], "the name that is not UTF-8 is left out");
                } else {
                    assert_eq!(case["status"], status, "{line}");
                    assert_eq!(case["body"], body, "{id}");
                }
            }
            Answer::File(status, file) => {
                if let Some((_, bytes)) = &file {
                    assert!(
                        !String::from_utf8_lossy(bytes).contains(&canary),
                        "{id}: the canary left the folder"
                    );
                }
                if is_decided {
                    assert_eq!(case["status"], 200, "{id}: Python serves it");
                    assert_eq!(status, 404, "{id}: Rust refuses it");
                } else {
                    assert_eq!(case["status"], status, "{line}");
                    if let (Some((kind, bytes)), true) = (&file, status == 200) {
                        assert_eq!(case["content_type"], *kind, "{id}");
                        assert_eq!(case["size"], bytes.len(), "{id}");
                        assert!(case["sha256"].as_str().unwrap().len() == 64);
                        let rel = format!(
                            "{}/{}",
                            case["query"]["folder"].as_str().unwrap(),
                            case["query"]["path"].as_str().unwrap()
                        );
                        let planted = spec["tree"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|e| e["p"] == rel.as_str());
                        match planted.and_then(|e| e["text"].as_str()) {
                            Some(text) => assert_eq!(bytes, text.as_bytes(), "{id}"),
                            None => {
                                assert!(bytes.iter().all(|b| *b == b'a'), "{id}: the filled file")
                            }
                        }
                    }
                }
            }
        }
        if is_decided {
            decided += 1;
        } else {
            equal += 1;
        }
    }
    eprintln!("attack table: {equal} equal, {decided} decided, {total} total");
    assert_eq!(total, 53);
    assert_eq!(decided, DECIDED.len());
    assert_eq!(equal + decided, total);
}

#[test]
fn the_canary_never_comes_out_through_any_listed_or_guessed_path() {
    let spec = read_json(&parity_dir().join("golden-report-files/tree.json"));
    let canary = spec["canary"].as_str().unwrap().to_string();
    let work = Work::new();
    let repo = build_tree(&work.dir, &spec);
    let conn = attack_store(&work, &repo);
    let listing = report_files(&conn, Some("acme"), Some(0), Some("reports")).unwrap();
    for file in listing["files"].as_array().unwrap() {
        let path = file["path"].as_str().unwrap();
        if let Ok((_, bytes)) =
            report_file(&conn, Some("acme"), Some(0), Some("reports"), Some(path))
        {
            assert!(!String::from_utf8_lossy(&bytes).contains(&canary), "{path}");
        }
    }
    for path in [
        "../../outside/secret.txt",
        "..%2f..%2foutside%2fsecret.txt",
        "link-out.html",
        "link-dir/x.html",
        "hard.html",
        "/etc/passwd",
    ] {
        let out = report_file(&conn, Some("acme"), Some(0), Some("reports"), Some(path));
        assert!(matches!(out, Err(ReportError::NotFound)), "{path}");
    }
}

#[test]
fn a_file_or_folder_swapped_for_a_symlink_after_the_listing_is_refused() {
    let spec = read_json(&parity_dir().join("golden-report-files/tree.json"));
    let work = Work::new();
    let repo = build_tree(&work.dir, &spec);
    let conn = attack_store(&work, &repo);
    let get = |path: &str| report_file(&conn, Some("acme"), Some(0), Some("reports"), Some(path));
    let listing = report_files(&conn, Some("acme"), Some(0), Some("reports")).unwrap();
    let listed = |name: &str| {
        listing["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["path"] == name && f["openable"] == true)
    };
    assert!(listed("ok.html") && listed("sub/inner.json"));
    assert!(get("ok.html").is_ok() && get("sub/inner.json").is_ok());
    // the file becomes a link to the outside file
    let reports = repo.join("reports");
    fs::remove_file(reports.join("ok.html")).unwrap();
    std::os::unix::fs::symlink(work.dir.join("outside/secret.txt"), reports.join("ok.html"))
        .unwrap();
    assert!(matches!(get("ok.html"), Err(ReportError::NotFound)));
    // the folder becomes a link to the outside folder, which holds `x.html`: a file of that name is never reached
    fs::remove_dir_all(reports.join("sub")).unwrap();
    std::os::unix::fs::symlink(work.dir.join("outside/dir"), reports.join("sub")).unwrap();
    assert!(matches!(get("sub/x.html"), Err(ReportError::NotFound)));
    // and the report folder itself, replaced by a link, is no longer a folder of the rule
    fs::remove_dir_all(&reports).unwrap();
    std::os::unix::fs::symlink(work.dir.join("outside/dir"), &reports).unwrap();
    assert!(matches!(get("x.html"), Err(ReportError::NotFound)));
    assert!(matches!(
        report_files(&conn, Some("acme"), Some(0), Some("reports")),
        Err(ReportError::NotFound)
    ));
}

#[test]
fn a_fifo_never_blocks_and_the_cap_is_a_read_limit() {
    let spec = read_json(&parity_dir().join("golden-report-files/tree.json"));
    let work = Work::new();
    let repo = build_tree(&work.dir, &spec);
    let (tx, rx) = std::sync::mpsc::channel();
    let path = work.dir.join("store.sqlite");
    let conn = attack_store(&work, &repo);
    drop(conn);
    std::thread::spawn(move || {
        let conn = open_read_only(&path).unwrap();
        let out = report_file(
            &conn,
            Some("acme"),
            Some(0),
            Some("reports"),
            Some("fifo.html"),
        );
        tx.send(matches!(out, Err(ReportError::NotFound))).unwrap();
    });
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(10)),
        Ok(true),
        "a FIFO is refused at once"
    );
    // a small cap: 10 bytes is served, 11 is too large (the file is `<p>ok</p>\n`, 10 bytes)
    let conn = open_read_only(&work.dir.join("store.sqlite")).unwrap();
    let ask = |cap| {
        report_file_capped(
            &conn,
            Some("acme"),
            Some(0),
            Some("reports"),
            Some("up.HTML"),
            cap,
        )
    };
    assert_eq!(ask(13).unwrap().1.len(), 13);
    assert!(matches!(ask(12), Err(ReportError::TooLarge)));
}

// ---- the rich goldens ----

const RICH_MTIME: u64 = MTIME;

/// A temp copy of the rich store whose workspace repos point at a temp copy of `fixtures-rich/kit`, as
/// `gen_golden.py` ran over its work folder (`kit_parity.rs` does the same; the temp folder must sort before `/work/rich`).
fn rich_conn(work: &Work) -> Connection {
    let kit = work.dir.join("kit");
    copy_tree(&parity_dir().join("fixtures-rich/kit"), &kit);
    let mut stack = vec![kit.clone()];
    while let Some(folder) = stack.pop() {
        for entry in fs::read_dir(folder).unwrap() {
            let path = entry.unwrap().path();
            fs::File::open(&path)
                .unwrap()
                .set_modified(UNIX_EPOCH + Duration::from_secs(RICH_MTIME))
                .unwrap();
            if path.is_dir() {
                stack.push(path);
            }
        }
    }
    let store = work.dir.join("store.sqlite");
    fs::copy(parity_dir().join("store-rich/store-v11.sqlite"), &store).unwrap();
    let conn = Connection::open(&store).unwrap();
    conn.execute(
        "UPDATE workspace_repos SET repo_path = replace(repo_path, '/work/rich/kit', ?1)",
        [kit.to_str().unwrap()],
    )
    .unwrap();
    drop(conn);
    open_read_only(&store).unwrap()
}

#[test]
fn the_nine_rich_goldens_equal_the_answers() {
    let manifest = read_json(&parity_dir().join("golden-rich/manifest.json"));
    let work = Work::new();
    assert!(
        work.dir.to_str().unwrap() < "/work/rich",
        "the temp folder must sort before /work/rich: set TMPDIR to a folder that does"
    );
    let conn = rich_conn(&work);
    let (mut compared, mut lists, mut files) = (0, 0, 0);
    for entry in manifest["entries"].as_array().unwrap() {
        let route = entry["route"].as_str().unwrap_or("");
        if route != "/api/kit/report-files" && route != "/api/kit/report-file" {
            continue;
        }
        let name = entry["file"].as_str().unwrap();
        let golden = read_json(&parity_dir().join("golden-rich").join(name));
        let params: BTreeMap<String, String> = entry["params"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
            .collect();
        let get = |k: &str| params.get(k).map(String::as_str);
        let (workspace, folder) = (get("workspace"), get("folder"));
        let repo_no = get("repo_no").and_then(parse_repo_no);
        if route == "/api/kit/report-files" {
            lists += 1;
            let got = match report_files(&conn, workspace, repo_no, folder) {
                Ok(body) => body,
                Err(_) => json!({"error": "not found"}),
            };
            assert_eq!(got, golden, "{name}");
        } else {
            files += 1;
            match report_file(&conn, workspace, repo_no, folder, get("path")) {
                Ok((kind, bytes)) => {
                    let served = &golden["served_file"];
                    assert_eq!(served["headers"]["content-type"], kind, "{name}");
                    assert_eq!(served["text"], String::from_utf8(bytes).unwrap(), "{name}");
                }
                Err(_) => assert_eq!(golden, json!({"error": "not found"}), "{name}"),
            }
        }
        compared += 1;
    }
    eprintln!("rich goldens: {compared} equal / {compared} total ({lists} lists, {files} files)");
    assert_eq!((lists, files), (5, 4));
}
