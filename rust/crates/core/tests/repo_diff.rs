//! T136 (FR-139, FR-140, DDR-0010): the read-only git diff of a repo. Real `git` on scratch repos.

use easy_personal_core::repo_diff::{self, Compare, DiffError};
use easy_personal_core::transcript;
use serde_json::Value;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Temp(PathBuf);

impl Temp {
    fn new() -> Temp {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "t136-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Temp(dir.canonicalize().unwrap())
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=Ann",
            "-c",
            "user.email=ann@example.invalid",
            "-c",
            "protocol.file.allow=always",
        ])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn write(repo: &Path, name: &str, text: &str) {
    let path = repo.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn commit(repo: &Path, message: &str) -> String {
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "-m", message]);
    git(repo, &["rev-parse", "HEAD"])
}

/// A repo on `main` with one commit of `a.txt`, `gone.txt` and `old.txt`.
fn repo(temp: &Temp) -> PathBuf {
    let dir = temp.0.join("proj");
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    write(&dir, "a.txt", "one\ntwo\n");
    write(&dir, "gone.txt", "bye\n");
    write(&dir, "old.txt", "r1\nr2\nr3\nr4\nr5\n");
    commit(&dir, "first");
    dir
}

/// One listed file: area, status, path, added, removed.
type Row = (String, String, String, Option<i64>, Option<i64>);

/// `(area, status, path, added, removed)` of every listed file.
fn rows(answer: &Value) -> Vec<Row> {
    answer["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["area"].as_str().unwrap().into(),
                f["status"].as_str().unwrap().into(),
                f["path"].as_str().unwrap().into(),
                f["added"].as_i64(),
                f["removed"].as_i64(),
            )
        })
        .collect()
}

fn row(area: &str, status: &str, path: &str, added: Option<i64>, removed: Option<i64>) -> Row {
    (area.into(), status.into(), path.into(), added, removed)
}

fn patch(dir: &Path, compare: &Compare, area: &str, path: &str) -> String {
    repo_diff::file(dir, compare, area, path).unwrap()["patch"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn uncommitted_lists_staged_unstaged_and_untracked_apart() {
    let temp = Temp::new();
    let dir = repo(&temp);
    write(&dir, "a.txt", "one\nTWO\n");
    git(&dir, &["add", "a.txt"]);
    write(&dir, "a.txt", "one\nTWO\nthree\n");
    git(&dir, &["mv", "old.txt", "new name.txt"]);
    fs::remove_file(dir.join("gone.txt")).unwrap();
    write(&dir, "sub/fresh.txt", "x\ny\n");
    write(&dir, ".gitignore", "skip.log\n");
    write(&dir, "skip.log", "ignored\n");

    let answer = repo_diff::files(&dir, &Compare::Uncommitted).unwrap();
    assert_eq!(
        rows(&answer),
        [
            row("staged", "modified", "a.txt", Some(1), Some(1)),
            row("staged", "renamed", "new name.txt", Some(0), Some(0)),
            row("unstaged", "modified", "a.txt", Some(1), Some(0)),
            row("unstaged", "deleted", "gone.txt", Some(0), Some(1)),
            row("untracked", "untracked", ".gitignore", Some(1), Some(0)),
            row("untracked", "untracked", "sub/fresh.txt", Some(2), Some(0)),
        ]
    );
    assert_eq!(answer["files"][1]["old_path"], "old.txt");
    assert_eq!(answer["total"], 6);
    assert_eq!(answer["truncated"], false);

    let staged = patch(&dir, &Compare::Uncommitted, "staged", "a.txt");
    assert!(
        staged.contains("-two\n+TWO\n") && !staged.contains("+three"),
        "{staged}"
    );
    let unstaged = patch(&dir, &Compare::Uncommitted, "unstaged", "a.txt");
    assert!(
        unstaged.contains("+three\n") && !unstaged.contains("-two"),
        "{unstaged}"
    );
    let renamed = patch(&dir, &Compare::Uncommitted, "staged", "new name.txt");
    assert!(renamed.contains("rename from old.txt"), "{renamed}");
    let fresh = patch(&dir, &Compare::Uncommitted, "untracked", "sub/fresh.txt");
    assert!(fresh.contains("@@ -0,0 +1,2 @@\n+x\n+y\n"), "{fresh}");
}

#[test]
fn a_binary_file_is_marked_and_a_file_touched_without_a_change_is_not_listed() {
    let temp = Temp::new();
    let dir = repo(&temp);
    fs::write(dir.join("blob.bin"), [0u8, 1, 2]).unwrap();
    fs::write(dir.join("same image.bin"), [0u8, 4, 5]).unwrap();
    commit(&dir, "blob");
    fs::write(dir.join("blob.bin"), [0u8, 9, 9]).unwrap();
    fs::write(dir.join("raw.bin"), [0u8, 7]).unwrap();
    // same bytes, new mtime: only the stat data differs
    std::thread::sleep(std::time::Duration::from_millis(1100));
    write(&dir, "a.txt", "one\ntwo\n");
    fs::write(dir.join("same image.bin"), [0u8, 4, 5]).unwrap();
    let index = fs::read(dir.join(".git/index")).unwrap();

    let answer = repo_diff::files(&dir, &Compare::Uncommitted).unwrap();
    assert_eq!(
        rows(&answer),
        [
            row("unstaged", "modified", "blob.bin", None, None),
            row("untracked", "untracked", "raw.bin", None, None),
        ]
    );
    assert_eq!(answer["files"][0]["binary"], true);
    assert_eq!(answer["files"][1]["binary"], true, "untracked, in the list");
    for (area, path) in [("unstaged", "blob.bin"), ("untracked", "raw.bin")] {
        let one = repo_diff::file(&dir, &Compare::Uncommitted, area, path).unwrap();
        assert_eq!(one["binary"], true, "{path}");
    }
    assert_eq!(
        fs::read(dir.join(".git/index")).unwrap(),
        index,
        "the index was written"
    );
}

#[test]
fn branch_compares_the_merge_base_with_head_and_leaves_uncommitted_out() {
    let temp = Temp::new();
    let dir = repo(&temp);
    let fork = git(&dir, &["rev-parse", "HEAD"]);
    git(&dir, &["checkout", "-q", "-b", "topic"]);
    write(&dir, "topic.txt", "t\n");
    let tip = commit(&dir, "on topic");
    git(&dir, &["checkout", "-q", "main"]);
    write(&dir, "main-only.txt", "m\n");
    commit(&dir, "on main");
    git(&dir, &["checkout", "-q", "topic"]);
    write(&dir, "a.txt", "dirty\n");

    let compare = Compare::Branch { base: None };
    let answer = repo_diff::files(&dir, &compare).unwrap();
    assert_eq!(
        rows(&answer),
        [row("committed", "added", "topic.txt", Some(1), Some(0))]
    );
    assert_eq!(answer["base"], fork.as_str());
    assert_eq!(answer["head"], tip.as_str());
    assert!(patch(&dir, &compare, "committed", "topic.txt").contains("+t\n"));
}

#[test]
fn one_commit_compares_with_its_first_parent_and_the_root_commit_with_nothing() {
    let temp = Temp::new();
    let dir = repo(&temp);
    let root = git(&dir, &["rev-parse", "HEAD"]);
    write(&dir, "a.txt", "one\n");
    let second = commit(&dir, "second");

    let answer = repo_diff::files(&dir, &Compare::Commit { rev: second }).unwrap();
    assert_eq!(
        rows(&answer),
        [row("committed", "modified", "a.txt", Some(0), Some(1))]
    );
    assert_eq!(answer["base"], root.as_str());
    let first = repo_diff::files(&dir, &Compare::Commit { rev: root }).unwrap();
    assert_eq!(first["total"], 3);
    assert_eq!(first["files"][0]["status"], "added");
}

#[test]
fn two_refs_are_compared_directly_in_the_order_given() {
    let temp = Temp::new();
    let dir = repo(&temp);
    git(&dir, &["tag", "v1"]);
    write(&dir, "a.txt", "one\ntwo\nmore\n");
    commit(&dir, "more");
    let forward = Compare::Range {
        base: "v1".into(),
        head: "main".into(),
    };
    let back = Compare::Range {
        base: "main".into(),
        head: "v1".into(),
    };
    assert_eq!(
        rows(&repo_diff::files(&dir, &forward).unwrap()),
        [row("committed", "modified", "a.txt", Some(1), Some(0))]
    );
    assert!(patch(&dir, &back, "committed", "a.txt").contains("-more\n"));
}

#[test]
fn only_a_listed_path_is_answered() {
    let temp = Temp::new();
    let dir = repo(&temp);
    write(&dir, "a.txt", "changed\n");
    for (area, path) in [
        ("unstaged", "gone.txt"),
        ("staged", "a.txt"),
        ("unstaged", "../proj/a.txt"),
        ("unstaged", ":/a.txt"),
        ("unstaged", "*"),
        ("untracked", ".git/config"),
        ("untracked", "/etc/hostname"),
        ("committed", "a.txt"),
    ] {
        assert!(
            matches!(
                repo_diff::file(&dir, &Compare::Uncommitted, area, path),
                Err(DiffError::NotListed)
            ),
            "{area} {path}"
        );
    }
}

#[test]
fn a_name_that_looks_like_pathspec_magic_is_read_literally() {
    let temp = Temp::new();
    let dir = repo(&temp);
    write(&dir, ":(exclude)a.txt", "v1\n");
    write(&dir, "*.txt", "v1\n");
    git(&dir, &["--literal-pathspecs", "add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "odd names"]);
    write(&dir, "*.txt", "v2\n");
    write(&dir, "a.txt", "also changed\n");
    let text = patch(&dir, &Compare::Uncommitted, "unstaged", "*.txt");
    assert!(
        text.contains("+v2") && !text.contains("also changed"),
        "{text}"
    );
}

#[test]
fn a_ref_that_is_an_option_or_unknown_is_refused() {
    let temp = Temp::new();
    let dir = repo(&temp);
    let target = temp.0.join("written");
    for bad in [
        format!("--output={}", target.display()),
        "-p".into(),
        "nope".into(),
        "".into(),
        "a\nb".into(),
        "x".repeat(300),
    ] {
        let compare = Compare::Range {
            base: bad.clone(),
            head: "main".into(),
        };
        assert!(
            matches!(repo_diff::files(&dir, &compare), Err(DiffError::UnknownRef)),
            "{bad:?}"
        );
    }
    assert!(!target.exists());
}

#[test]
fn the_compare_query_needs_its_refs() {
    let parse = |c, b, h| Compare::parse(c, b, h);
    assert_eq!(
        parse(Some("uncommitted"), None, None).unwrap(),
        Compare::Uncommitted
    );
    assert_eq!(
        parse(Some("branch"), Some(""), None).unwrap(),
        Compare::Branch { base: None }
    );
    assert_eq!(
        parse(Some("commit"), None, Some("abc")).unwrap(),
        Compare::Commit { rev: "abc".into() }
    );
    for (c, b, h) in [
        (None, None, None),
        (Some("other"), None, None),
        (Some("commit"), None, None),
        (Some("range"), Some("a"), None),
        (Some("range"), None, Some("b")),
    ] {
        assert!(parse(c, b, h).is_err(), "{c:?} {b:?} {h:?}");
    }
}

#[test]
fn no_program_the_repo_names_runs_and_the_index_is_not_written() {
    let temp = Temp::new();
    let dir = repo(&temp);
    // a submodule with its own planted filter and a change in its work tree
    let lib = temp.0.join("lib");
    fs::create_dir_all(&lib).unwrap();
    git(&lib, &["init", "-q", "-b", "main"]);
    write(&lib, "l.txt", "l\n");
    commit(&lib, "lib");
    git(
        &dir,
        &["submodule", "add", "-q", lib.to_str().unwrap(), "vendor"],
    );
    // a binary file the app hashes to see whether it changed: its filter must stay off there too
    fs::write(dir.join("pic.bin"), [0u8, 1, 2]).unwrap();
    commit(&dir, "submodule");

    let ran = temp.0.join("ran");
    let spy = temp.0.join("spy.sh");
    fs::write(
        &spy,
        format!("#!/bin/sh\necho \"$0 $*\" >> '{}'\ncat\n", ran.display()),
    )
    .unwrap();
    fs::set_permissions(&spy, fs::Permissions::from_mode(0o755)).unwrap();
    let spy_text = spy.to_str().unwrap();
    for (key, value) in [
        ("filter.evil.clean", format!("{spy_text} clean")),
        ("filter.evil.smudge", format!("{spy_text} smudge")),
        ("filter.evil.required", "true".into()),
        ("filter.Dot.ted.process", format!("{spy_text} process")),
        ("diff.external", format!("{spy_text} external")),
        ("diff.evil.textconv", format!("{spy_text} textconv")),
        ("diff.evil.command", format!("{spy_text} command")),
        ("core.fsmonitor", format!("{spy_text} fsmonitor")),
        ("log.showSignature", "true".into()),
        ("gpg.program", spy_text.into()),
        ("diff.ignoreSubmodules", "none".into()),
    ] {
        git(&dir, &["config", key, &value]);
    }
    git(
        &dir.join("vendor"),
        &[
            "config",
            "filter.sub.clean",
            &format!("{spy_text} submodule"),
        ],
    );
    write(
        &dir,
        ".git/modules/vendor/info/attributes",
        "* filter=sub\n",
    );
    write(&dir.join("vendor"), "l.txt", "m\n");
    write(
        &dir,
        ".git/info/attributes",
        "a.txt filter=evil diff=evil\ngone.txt filter=Dot.ted\npic.bin filter=evil\n",
    );
    let hook = dir.join(".git/hooks/post-index-change");
    fs::copy(&spy, &hook).unwrap();
    write(&dir, "a.txt", "changed\n");
    write(&dir, "gone.txt", "changed too\n");
    fs::File::options()
        .write(true)
        .open(dir.join("pic.bin"))
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000))
        .unwrap();
    // a signed `HEAD`, so a log that checks signatures would start `gpg.program`
    let commit = git(&dir, &["cat-file", "commit", "HEAD"]);
    let signed = commit.replacen(
        "\n\n",
        "\ngpgsig -----BEGIN PGP SIGNATURE-----\n \n aWdub3JlZA==\n -----END PGP SIGNATURE-----\n\n",
        1,
    );
    let object = temp.0.join("signed");
    fs::write(&object, signed + "\n").unwrap();
    let id = git(
        &dir,
        &[
            "hash-object",
            "-t",
            "commit",
            "-w",
            object.to_str().unwrap(),
        ],
    );
    git(&dir, &["update-ref", "HEAD", &id]);
    let _ = fs::remove_file(&ran);
    let index = fs::read(dir.join(".git/index")).unwrap();

    let head = repo_diff::overview(&dir).unwrap();
    assert!(!head["commits"].as_array().unwrap().is_empty());
    repo_diff::worktrees(&dir).unwrap();
    let answer = repo_diff::files(&dir, &Compare::Uncommitted).unwrap();
    assert_eq!(answer["filters_off"], true);
    assert!(rows(&answer).iter().all(|r| r.2 != "pic.bin"), "{answer}");
    for path in ["a.txt", "gone.txt"] {
        assert!(patch(&dir, &Compare::Uncommitted, "unstaged", path).contains("+changed"));
    }
    let last = head["commits"][0]["id"].as_str().unwrap().to_string();
    repo_diff::files(&dir, &Compare::Commit { rev: last }).unwrap();

    assert!(
        !ran.exists(),
        "ran: {}",
        fs::read_to_string(&ran).unwrap_or_default()
    );
    assert_eq!(
        fs::read(dir.join(".git/index")).unwrap(),
        index,
        "the index was written"
    );
}

#[test]
fn a_filter_that_cannot_be_switched_off_refuses_the_uncommitted_compare_only() {
    let temp = Temp::new();
    let dir = repo(&temp);
    let ran = temp.0.join("ran");
    let config = format!(
        "[filter \"a=b\"]\n\tclean = touch '{}'; cat\n",
        ran.display()
    );
    let mut text = fs::read_to_string(dir.join(".git/config")).unwrap();
    text.push_str(&config);
    fs::write(dir.join(".git/config"), text).unwrap();
    write(&dir, ".git/info/attributes", "* filter=a=b\n");
    write(&dir, "a.txt", "changed\n");
    assert!(matches!(
        repo_diff::files(&dir, &Compare::Uncommitted),
        Err(DiffError::Filter)
    ));
    let head = git(&dir, &["rev-parse", "HEAD"]);
    assert!(repo_diff::files(&dir, &Compare::Commit { rev: head }).is_ok());
    assert!(!ran.exists());
}

#[test]
fn an_untracked_link_or_fifo_is_never_read() {
    let temp = Temp::new();
    let dir = repo(&temp);
    let secret = temp.0.join("secret.txt");
    fs::write(&secret, "TOP-SECRET\n").unwrap();
    symlink(&secret, dir.join("link.txt")).unwrap();
    let made = Command::new("mkfifo")
        .arg(dir.join("pipe"))
        .status()
        .unwrap();
    assert!(made.success());

    let answer = repo_diff::file(&dir, &Compare::Uncommitted, "untracked", "link.txt").unwrap();
    assert!(!answer.to_string().contains("TOP-SECRET"), "{answer}");
    assert_eq!(answer["patch"], "");
    assert!(answer["note"].as_str().unwrap().contains("link"));
    // git does not list a FIFO; asking for it is refused rather than blocking
    assert!(repo_diff::file(&dir, &Compare::Uncommitted, "untracked", "pipe").is_err());
}

#[test]
fn long_and_hidden_text_is_cut_and_made_visible() {
    let temp = Temp::new();
    let dir = repo(&temp);
    write(&dir, "a.txt", "one\nsafe\u{202e}txt\ttab\n");
    let text = patch(&dir, &Compare::Uncommitted, "unstaged", "a.txt");
    assert!(
        text.contains("+safe\\u{202e}txt\ttab\n") && !text.contains('\u{202e}'),
        "{text}"
    );

    let many: String = (0..6000).map(|n| format!("line {n}\n")).collect();
    write(&dir, "many.txt", &many);
    write(&dir, "wide.txt", &"w".repeat(3 * 1024 * 1024));
    for path in ["many.txt", "wide.txt"] {
        let one = repo_diff::file(&dir, &Compare::Uncommitted, "untracked", path).unwrap();
        let text = one["patch"].as_str().unwrap();
        assert_eq!(one["cut"], true, "{path}");
        assert!(text.len() <= repo_diff::PATCH_MAX_BYTES, "{path}");
        assert!(text.lines().count() <= repo_diff::PATCH_MAX_LINES, "{path}");
    }
    git(&dir, &["add", "many.txt", "wide.txt"]);
    for path in ["many.txt", "wide.txt"] {
        let one = repo_diff::file(&dir, &Compare::Uncommitted, "staged", path).unwrap();
        assert_eq!(one["cut"], true, "{path}");
        assert!(one["patch"].as_str().unwrap().len() <= repo_diff::PATCH_MAX_BYTES);
    }
}

#[test]
fn a_long_file_list_is_cut_and_keeps_its_total() {
    let temp = Temp::new();
    let dir = repo(&temp);
    for n in 0..repo_diff::FILES_MAX + 5 {
        write(&dir, &format!("many/f{n:05}"), "");
    }
    let answer = repo_diff::files(&dir, &Compare::Uncommitted).unwrap();
    assert_eq!(
        answer["files"].as_array().unwrap().len(),
        repo_diff::FILES_MAX
    );
    assert_eq!(answer["total"], repo_diff::FILES_MAX + 5);
    assert_eq!(answer["truncated"], true);
}

#[test]
fn worktrees_branches_and_commits_are_listed_without_a_full_path() {
    let temp = Temp::new();
    let dir = repo(&temp);
    let side = temp.0.join("side tree");
    git(
        &dir,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "topic",
            side.to_str().unwrap(),
        ],
    );
    let (trees, _) = repo_diff::worktrees(&dir).unwrap();
    assert_eq!(trees.len(), 2);
    assert!(trees[0].main && !trees[1].main);
    assert_eq!(trees[1].folder, "side tree");
    assert_eq!(trees[1].branch.as_deref(), Some("topic"));
    assert_eq!(trees[1].path, side);
    assert_ne!(trees[0].id, trees[1].id);
    // asked from the worktree, the list is the same
    assert_eq!(repo_diff::worktrees(&side).unwrap().0[1].id, trees[1].id);

    let head = repo_diff::overview(&side).unwrap();
    let names: Vec<&str> = head["branches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["main", "topic"]);
    assert_eq!(head["base"], "main");
    assert_eq!(head["commits"][0]["subject"], "first");
    assert_eq!(head["commits"][0]["author"], "Ann");
    assert!(!head.to_string().contains(temp.0.to_str().unwrap()));
}

#[test]
fn an_empty_repo_a_plain_folder_and_unrelated_histories_get_clear_answers() {
    let temp = Temp::new();
    let empty = temp.0.join("empty");
    fs::create_dir_all(&empty).unwrap();
    git(&empty, &["init", "-q", "-b", "main"]);
    write(&empty, "n.txt", "n\n");
    git(&empty, &["add", "n.txt"]);
    let answer = repo_diff::files(&empty, &Compare::Uncommitted).unwrap();
    assert_eq!(
        rows(&answer),
        [row("staged", "added", "n.txt", Some(1), Some(0))]
    );
    let head = repo_diff::overview(&empty).unwrap();
    assert_eq!(head["commits"].as_array().unwrap().len(), 0);
    assert_eq!(head["base"], Value::Null);
    assert!(matches!(
        repo_diff::files(&empty, &Compare::Branch { base: None }),
        Err(DiffError::UnknownRef | DiffError::NoBase)
    ));

    let plain = temp.0.join("plain");
    fs::create_dir_all(&plain).unwrap();
    assert!(matches!(
        repo_diff::overview(&plain),
        Err(DiffError::NotARepo)
    ));
    assert!(matches!(
        repo_diff::files(&plain, &Compare::Uncommitted),
        Err(DiffError::NotARepo)
    ));
    assert_eq!(repo_diff::toplevel(&plain), None);

    let dir = repo(&temp);
    git(&dir, &["checkout", "-q", "--orphan", "alone"]);
    commit(&dir, "alone");
    assert!(matches!(
        repo_diff::files(&dir, &Compare::Branch { base: None }),
        Err(DiffError::NoCommonHistory)
    ));
    assert_eq!(repo_diff::toplevel(&dir.join(".")), Some(dir.clone()));
}

#[test]
fn a_merge_conflict_is_listed_once_as_a_conflict() {
    let temp = Temp::new();
    let dir = repo(&temp);
    git(&dir, &["checkout", "-q", "-b", "other"]);
    write(&dir, "a.txt", "other\n");
    commit(&dir, "other");
    git(&dir, &["checkout", "-q", "main"]);
    write(&dir, "a.txt", "mine\n");
    commit(&dir, "mine");
    let merged = Command::new("git")
        .arg("-C")
        .arg(&dir)
        .args([
            "-c",
            "user.name=Ann",
            "-c",
            "user.email=ann@example.invalid",
            "merge",
            "other",
        ])
        .output()
        .unwrap();
    assert!(!merged.status.success());
    let answer = repo_diff::files(&dir, &Compare::Uncommitted).unwrap();
    let listed: Vec<_> = rows(&answer)
        .into_iter()
        .filter(|r| r.2 == "a.txt")
        .collect();
    assert_eq!(listed.len(), 1, "{answer}");
    assert_eq!(listed[0].1, "conflict");
    assert!(patch(&dir, &Compare::Uncommitted, &listed[0].0, "a.txt").contains("<<<<<<<"));
}

#[test]
fn an_untracked_file_past_the_read_cap_has_no_count() {
    let temp = Temp::new();
    let dir = repo(&temp);
    write(&dir, "big.txt", &"line\n".repeat(300_000));
    write(&dir, "open.txt", "no newline at the end");
    assert_eq!(
        rows(&repo_diff::files(&dir, &Compare::Uncommitted).unwrap()),
        [
            row("untracked", "untracked", "big.txt", None, None),
            row("untracked", "untracked", "open.txt", Some(1), Some(0)),
        ]
    );
}

#[test]
fn cut_ref_and_work_tree_lists_say_so() {
    let temp = Temp::new();
    let dir = repo(&temp);
    assert_eq!(repo_diff::overview(&dir).unwrap()["truncated"], false);
    assert!(!repo_diff::worktrees(&dir).unwrap().1);
    let head = git(&dir, &["rev-parse", "HEAD"]);
    let refs: String = (0..505)
        .map(|n| format!("{head} refs/heads/b{n:04}\n"))
        .collect();
    fs::write(dir.join(".git/packed-refs"), refs).unwrap();
    let answer = repo_diff::overview(&dir).unwrap();
    assert_eq!(answer["branches"].as_array().unwrap().len(), 500);
    assert_eq!(answer["truncated"], true);
    for n in 0..100 {
        let tree = temp.0.join(format!("w{n:03}"));
        git(
            &dir,
            &["worktree", "add", "-q", "--detach", tree.to_str().unwrap()],
        );
    }
    let (trees, cut) = repo_diff::worktrees(&dir).unwrap();
    assert_eq!((trees.len(), cut), (100, true));
}

#[test]
fn a_file_replaced_by_a_folder_answers_its_own_patch_only() {
    let temp = Temp::new();
    let dir = repo(&temp);
    fs::remove_file(dir.join("gone.txt")).unwrap();
    write(&dir, "gone.txt/inner.txt", "something else entirely\n");
    git(&dir, &["add", "-A"]);
    let text = patch(&dir, &Compare::Uncommitted, "staged", "gone.txt");
    assert!(
        text.contains("-bye") && !text.contains("inner.txt"),
        "{text}"
    );
}

#[test]
fn names_and_the_default_base_are_shown_with_no_raw_control_or_hidden_character() {
    let temp = Temp::new();
    let dir = temp.0.join("odd");
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "trunk"]);
    write(&dir, "n\nl\tx", "v\n");
    commit(&dir, "first");
    let head = git(&dir, &["rev-parse", "HEAD"]);
    let remote = "refs/remotes/origin/ev\u{202e}il";
    git(&dir, &["update-ref", remote, &head]);
    git(&dir, &["symbolic-ref", "refs/remotes/origin/HEAD", remote]);
    let answer = repo_diff::overview(&dir).unwrap();
    assert_eq!(answer["base"], "origin/ev\u{202e}il");
    assert_eq!(answer["base_shown"], "origin/ev\\u{202e}il");
    write(&dir, "n\nl\tx", "w\n");
    let list = repo_diff::files(&dir, &Compare::Uncommitted).unwrap();
    assert_eq!(list["files"][0]["path"], "n\nl\tx");
    assert_eq!(list["files"][0]["shown"], "n\\u{a}l\\u{9}x");
}

#[test]
fn a_tag_named_main_does_not_take_the_place_of_the_branch() {
    let temp = Temp::new();
    let dir = repo(&temp);
    git(&dir, &["tag", "main"]);
    write(&dir, "second.txt", "s\n");
    let second = commit(&dir, "second");
    git(&dir, &["checkout", "-q", "-b", "topic"]);
    write(&dir, "third.txt", "t\n");
    commit(&dir, "third");
    let answer = repo_diff::files(&dir, &Compare::Branch { base: None }).unwrap();
    assert_eq!(answer["base"], second.as_str());
    assert_eq!(repo_diff::overview(&dir).unwrap()["base"], "main");
}

#[test]
fn a_folder_with_its_own_repo_is_listed_and_opens_with_a_note() {
    let temp = Temp::new();
    let dir = repo(&temp);
    let inner = dir.join("inner");
    fs::create_dir_all(&inner).unwrap();
    git(&inner, &["init", "-q"]);
    write(&inner, "kept.txt", "not shown\n");
    let one = repo_diff::file(&dir, &Compare::Uncommitted, "untracked", "inner/").unwrap();
    assert_eq!(one["patch"], "");
    assert!(one["note"].as_str().unwrap().contains("own git repository"));
}

#[test]
fn filters_are_named_off_only_where_they_would_apply() {
    let temp = Temp::new();
    let dir = repo(&temp);
    git(&dir, &["config", "filter.lfs.clean", "cat"]);
    let head = git(&dir, &["rev-parse", "HEAD"]);
    let on = |c: &Compare| repo_diff::files(&dir, c).unwrap()["filters_off"].clone();
    assert_eq!(on(&Compare::Uncommitted), true);
    assert_eq!(on(&Compare::Commit { rev: head }), false);
}

#[test]
fn the_session_folder_is_the_newest_one_even_before_a_huge_record() {
    let temp = Temp::new();
    let file = temp.0.join("s.jsonl");
    let record = |cwd: &str| format!("{{\"type\":\"user\",\"cwd\":\"{cwd}\"}}\n");
    let filler =
        |bytes: usize| format!("{{\"type\":\"user\",\"text\":\"{}\"}}\n", "x".repeat(bytes));
    let early = record("/work/old") + &filler(2 * 1024 * 1024) + &record("/work/newer");
    fs::write(&file, early.clone() + &filler(1_200_000)).unwrap();
    assert_eq!(
        transcript::last_folder(&file).as_deref(),
        Some("/work/newer")
    );
    // further back than the read reaches: not known, never an older folder
    fs::write(&file, early + &filler(1024 * 1024).repeat(17)).unwrap();
    assert_eq!(transcript::last_folder(&file), None);
}

#[test]
fn a_partial_clone_is_never_fetched_into() {
    let temp = Temp::new();
    let up = repo(&temp);
    git(&up, &["config", "uploadpack.allowFilter", "true"]);
    write(&up, "a.txt", "newer\n");
    commit(&up, "second");
    let clone = temp.0.join("clone");
    let url = format!("file://{}", up.display());
    git(
        &temp.0,
        &[
            "clone",
            "-q",
            "--filter=blob:none",
            &url,
            clone.to_str().unwrap(),
        ],
    );
    let ran = temp.0.join("ran");
    let spy = temp.0.join("pack.sh");
    fs::write(
        &spy,
        format!(
            "#!/bin/sh\ntouch '{}'\nexec git-upload-pack \"$@\"\n",
            ran.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&spy, fs::Permissions::from_mode(0o755)).unwrap();
    git(
        &clone,
        &["config", "remote.origin.uploadpack", spy.to_str().unwrap()],
    );
    // the first commit's blob was never fetched
    let old = git(&clone, &["rev-parse", "HEAD~1"]);
    let compare = Compare::Range {
        base: old,
        head: "HEAD".into(),
    };
    let _ = repo_diff::files(&clone, &compare);
    assert!(!ran.exists(), "the app fetched");
}

/// Run by [`the_parents_git_variables_are_not_passed_on`] in a process of its own, with `GIT_*` set.
#[test]
#[ignore = "run in a child process with GIT_* set"]
fn child_with_git_variables() {
    let dir = PathBuf::from(std::env::var("T136_REPO").unwrap());
    let list = repo_diff::files(&dir, &Compare::Uncommitted).unwrap();
    assert_eq!(
        rows(&list),
        [row("unstaged", "modified", "a.txt", Some(1), Some(2))]
    );
    assert!(patch(&dir, &Compare::Uncommitted, "unstaged", "a.txt").contains("+mine"));
}

#[test]
fn the_parents_git_variables_are_not_passed_on() {
    let temp = Temp::new();
    let dir = repo(&temp);
    write(&dir, "a.txt", "mine\n");
    let other = temp.0.join("other");
    fs::create_dir_all(&other).unwrap();
    git(&other, &["init", "-q", "-b", "main"]);
    let ran = temp.0.join("ran");
    let spy = temp.0.join("ext.sh");
    fs::write(&spy, format!("#!/bin/sh\ntouch '{}'\n", ran.display())).unwrap();
    fs::set_permissions(&spy, fs::Permissions::from_mode(0o755)).unwrap();
    let out = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_with_git_variables", "--ignored"])
        .env("T136_REPO", &dir)
        .env("GIT_DIR", other.join(".git"))
        .env("GIT_WORK_TREE", &other)
        .env("GIT_EXTERNAL_DIFF", &spy)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("1 passed"));
    assert!(!ran.exists());
}
