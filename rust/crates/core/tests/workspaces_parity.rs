//! T022: `workspaces::list_all` and `workspaces::unassigned` against the four Python goldens, `api__workspaces__all.json`
//! and `api__workspaces_unassigned__all.json` of `rust/parity/golden` (over `store-v11.sqlite`) and of
//! `rust/parity/golden-rich` (over `store-rich/store-v11.sqlite`), plus Python's edge cases (section 3).
//!
//! The entries come from the manifests (`file` starting `api__workspaces__` or `api__workspaces_unassigned__`; the
//! `api__guard__*` and `api__kit_workspaces__*` entries are other routes), so a missing or extra golden fails the count.
//! Compared as parsed JSON: the files hold sorted keys, list order is kept, `1` and `1.0` differ.
//!
//! The rich goldens were made over the live work folder (token `<WORK>`), the saved store holds `/work/rich` instead
//! (`gen_golden.py`, `RICH_PREFIX`). So `<WORK>` is read as `/work/rich`, and the `repos` of each workspace are compared in
//! byte order on both sides: the two spellings sort differently. Rust's own order is checked to be byte order too.
//!
//! Every store is a temp copy or the parity file opened read-only: the owner's real `~/.easy-personal` is never opened.

use easy_personal_core::store::open_read_only;
use easy_personal_core::workspaces::{list_all, unassigned};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

fn parity_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity")
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap_or_else(|e| panic!("{path:?}: {e}")))
        .unwrap()
}

/// `(manifest folder, store file)` of the two oracles.
const ORACLES: [(&str, &str); 2] = [
    ("golden", "store-v11.sqlite"),
    ("golden-rich", "store-rich/store-v11.sqlite"),
];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    List,
    Unassigned,
}

/// The entries of one manifest that name one of the two routes: `(golden file, kind)`.
fn manifest_entries(dir: &str) -> Vec<(String, Kind)> {
    let manifest = read_json(&parity_dir().join(dir).join("manifest.json"));
    manifest["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| {
            let file = e["file"].as_str().unwrap();
            let kind = if file.starts_with("api__workspaces__") {
                assert_eq!(e["route"], "/api/workspaces", "{file}");
                Kind::List
            } else if file.starts_with("api__workspaces_unassigned__") {
                assert_eq!(e["route"], "/api/workspaces/unassigned", "{file}");
                Kind::Unassigned
            } else {
                return None;
            };
            assert_eq!(e["method"], "GET", "{file}");
            assert_eq!(e["status"], 200, "{file}");
            assert_eq!(
                e["params"],
                json!({}),
                "{file}: the routes take no parameter"
            );
            Some((file.to_string(), kind))
        })
        .collect()
}

fn answer(conn: &Connection, kind: Kind) -> Value {
    match kind {
        Kind::List => list_all(conn),
        Kind::Unassigned => unassigned(conn),
    }
    .unwrap_or_else(|e| panic!("{kind:?}: {e}"))
}

/// Replaces the `<WORK>` token of a rich golden by the folder name the saved store holds.
fn untokened(value: &Value) -> Value {
    match value {
        Value::String(s) => Value::String(s.replace("<WORK>", "/work/rich")),
        Value::Array(items) => Value::Array(items.iter().map(untokened).collect()),
        Value::Object(map) => {
            Value::Object(map.iter().map(|(k, v)| (k.clone(), untokened(v))).collect())
        }
        other => other.clone(),
    }
}

/// `repos` of every workspace in byte order (what SQLite's `ORDER BY repo_path` gives for one spelling of the paths).
fn repos_sorted(mut body: Value) -> Value {
    if let Some(workspaces) = body.get_mut("workspaces").and_then(Value::as_array_mut) {
        for workspace in workspaces {
            if let Some(repos) = workspace.get_mut("repos").and_then(Value::as_array_mut) {
                repos.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
            }
        }
    }
    body
}

/// `Ok` when Rust's answer equals the golden of `file`; the error names the entry.
fn compare(file: &str, actual: &Value, golden: &Value) -> Result<(), String> {
    if repos_sorted(actual.clone()) != *actual {
        return Err(format!("{file}: the repos are not in byte order"));
    }
    let want = repos_sorted(untokened(golden));
    if *actual != want {
        return Err(format!("{file}: differs\n  got  {actual}\n  want {want}"));
    }
    Ok(())
}

/// Compares every entry of one oracle over `store`; returns how many were compared.
fn compare_oracle(dir: &str, store: &Path) -> Result<usize, String> {
    let conn = open_read_only(store).unwrap();
    let mut compared = 0;
    for (file, kind) in manifest_entries(dir) {
        let golden = read_json(&parity_dir().join(dir).join(&file));
        compare(&file, &answer(&conn, kind), &golden)?;
        compared += 1;
    }
    Ok(compared)
}

// ---- 1. the four goldens ----

#[test]
fn the_manifests_name_two_entries_each() {
    for (dir, _) in ORACLES {
        let entries = manifest_entries(dir);
        assert_eq!(entries.len(), 2, "{dir}: {entries:?}");
        assert!(entries.contains(&("api__workspaces__all.json".to_string(), Kind::List)));
        assert!(entries.contains(&(
            "api__workspaces_unassigned__all.json".to_string(),
            Kind::Unassigned
        )));
    }
}

#[test]
fn list_all_and_unassigned_equal_the_four_goldens() {
    let mut total = 0;
    for (dir, store) in ORACLES {
        total += compare_oracle(dir, &parity_dir().join(store)).unwrap();
    }
    assert_eq!(total, 4, "two oracles x (list, unassigned)");
}

#[test]
fn the_rich_list_keeps_what_the_golden_names() {
    // A guard on the comparison itself: the rich golden holds three workspaces, two of them with repos.
    let conn = open_read_only(&parity_dir().join("store-rich/store-v11.sqlite")).unwrap();
    let body = list_all(&conn).unwrap();
    let names: Vec<&str> = body["workspaces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["orphan", "acme", "personal"]);
    assert_eq!(
        body["workspaces"][1]["repos"],
        json!(["/work/rich/gone-a", "/work/rich/kit/alpha"])
    );
    assert_eq!(body["workspaces"][0]["is_orphan"], json!(true));
    assert_eq!(body["workspaces"][1]["id"], json!(2));
}

// ---- 2. the comparison is not vacuous, and reading changes nothing ----

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t022-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    /// A writable copy of a parity store, changed by `sql` (which must touch a row).
    fn store(&self, store: &str, sql: &str) -> PathBuf {
        let path = self.0.join("store.sqlite");
        fs::copy(parity_dir().join(store), &path).unwrap();
        let n = Connection::open(&path).unwrap().execute(sql, []).unwrap();
        assert!(n >= 1, "the change touched no row: {sql}");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn removing_or_moving_one_workspace_repo_fails_the_comparison_and_names_the_entry() {
    let rich = "store-rich/store-v11.sqlite";
    for sql in [
        "DELETE FROM workspace_repos WHERE repo_path = '/work/rich/kit/beta'",
        "UPDATE workspace_repos SET repo_path = '/work/rich/kit/betx' WHERE repo_path = '/work/rich/kit/beta'",
    ] {
        let dir = TempDir::new();
        let path = dir.store(rich, sql);
        let error = compare_oracle("golden-rich", &path).unwrap_err();
        assert!(
            error.contains("api__workspaces__all.json"),
            "{sql}: {error}"
        );
    }
    // the same store without the change passes (so the failures above come from the change)
    assert_eq!(
        compare_oracle("golden-rich", &parity_dir().join(rich)).unwrap(),
        2
    );
}

#[test]
fn a_changed_workspace_field_fails_the_comparison_too() {
    let dir = TempDir::new();
    let path = dir.store(
        "store-rich/store-v11.sqlite",
        "UPDATE workspaces SET email = 'other@example.invalid' WHERE name = 'personal'",
    );
    let error = compare_oracle("golden-rich", &path).unwrap_err();
    assert!(error.contains("api__workspaces__all.json"), "{error}");
}

#[test]
fn reading_leaves_the_store_bytes_and_the_folder_as_they_were() {
    for (_, store) in ORACLES {
        let dir = TempDir::new();
        let path = dir.store(store, "UPDATE events SET abstained = abstained");
        let before = fs::read(&path).unwrap();
        let conn = open_read_only(&path).unwrap();
        list_all(&conn).unwrap();
        unassigned(&conn).unwrap();
        drop(conn);
        assert_eq!(fs::read(&path).unwrap(), before, "{store}");
        let mut names: Vec<String> = fs::read_dir(&dir.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, ["store.sqlite"], "{store}: no -wal, -shm or journal");
    }
}

#[test]
fn a_store_that_cannot_answer_is_an_error_with_no_path_and_no_sql() {
    let dir = TempDir::new();
    let path = dir.0.join("empty.sqlite");
    Connection::open(&path)
        .unwrap()
        .execute_batch("CREATE TABLE t (x)")
        .unwrap();
    let conn = open_read_only(&path).unwrap();
    let dir_text = dir.0.to_string_lossy().into_owned();
    for error in [
        list_all(&conn).unwrap_err().to_string(),
        unassigned(&conn).unwrap_err().to_string(),
    ] {
        assert!(!error.contains(&dir_text), "{error}");
        for sql_word in ["SELECT", "workspaces", "events", "workspace_repos"] {
            assert!(!error.contains(sql_word), "{error}");
        }
    }
}

// ---- 3. Python's edge cases ----
//
// A store built here from the default one (schema only): git repos under `CARGO_TARGET_TMPDIR` (outside the system
// temp folder, which is the scratch rule (a), and under no hidden folder), gone folders, a gone worktree whose parent is
// a repo, a gone folder whose branch names a repo's branch, a model missing from the price table, ties, repeated
// records of one message, workspaces that share a repo and one that has none. The expected values are Python's:
//
//   T022_KEEP=1 cargo test -p easy-personal-core --test workspaces_parity -- --exact edge_store_python_values
//
// keeps the store and its repos in `$CARGO_TARGET_TMPDIR/t022-kept`, then (HOME and EASY_PERSONAL_HOME are scratch
// folders, never the real ones)
//
//   rust/parity/.venv/bin/python -B -I -c '
//   import json, sqlite3, sys
//   from easy_personal.core import workspaces
//   conn = sqlite3.connect(sys.argv[1])
//   print(json.dumps({"workspaces": workspaces.list_all(conn)}, sort_keys=True))
//   print(json.dumps({"repos": workspaces.unassigned(conn)}, sort_keys=True))' <kept>/store.sqlite
//
// (`<R>` below stands for the folder that holds the repos.)

const EDGE_WORKSPACES: &str = r##"{
  "workspaces": [
    {
      "color": null,
      "description": null,
      "email": null,
      "icon": null,
      "id": 1,
      "is_orphan": true,
      "name": "orphan",
      "repos": []
    },
    {
      "color": null,
      "description": null,
      "email": null,
      "icon": null,
      "id": 4,
      "is_orphan": false,
      "name": "1x",
      "repos": []
    },
    {
      "color": null,
      "description": "no repo",
      "email": null,
      "icon": null,
      "id": 5,
      "is_orphan": false,
      "name": "a",
      "repos": []
    },
    {
      "color": null,
      "description": null,
      "email": null,
      "icon": null,
      "id": 3,
      "is_orphan": false,
      "name": "a-1",
      "repos": [
        "<R>/alp",
        "<R>/covered"
      ]
    },
    {
      "color": "#00ff7f",
      "description": "text ünï",
      "email": "a@b.invalid",
      "icon": "rocket",
      "id": 2,
      "is_orphan": false,
      "name": "b2",
      "repos": [
        "<R>/covered",
        "/work/Z",
        "/work/a",
        "/work/z",
        "/work/é"
      ]
    }
  ]
}"##;
const EDGE_UNASSIGNED: &str = r##"{
  "repos": [
    {
      "api_equivalent_cost": {
        "abstained_models": [],
        "price_table_date": "2026-10-21",
        "usd": 0.018825
      },
      "last_used_utc": "2026-01-18T10:00:00.000000Z",
      "name": "alpha",
      "path": "<R>/alpha",
      "sessions": 3
    },
    {
      "api_equivalent_cost": {
        "abstained_models": [],
        "price_table_date": "2026-10-21",
        "usd": 0.00175
      },
      "last_used_utc": "2026-01-15T10:00:00.000000Z",
      "name": "gamma",
      "path": "<R>/gamma",
      "sessions": 1
    },
    {
      "api_equivalent_cost": {
        "abstained_models": [],
        "price_table_date": "2026-10-21",
        "usd": 0.00105
      },
      "last_used_utc": "2026-01-15T10:00:00.000000Z",
      "name": "beta",
      "path": "<R>/beta",
      "sessions": 1
    },
    {
      "api_equivalent_cost": {
        "abstained_models": [
          "claude-unlisted-model"
        ],
        "price_table_date": "2026-10-21",
        "usd": null
      },
      "last_used_utc": "2026-01-11T10:00:00.000000Z",
      "name": "delta",
      "path": "<R>/delta",
      "sessions": 1
    },
    {
      "api_equivalent_cost": {
        "abstained_models": [
          "(unknown)",
          "claude-unlisted-model"
        ],
        "price_table_date": "2026-10-21",
        "usd": 0.02205
      },
      "last_used_utc": "2026-01-09T13:00:00.000000Z",
      "name": "epsilon",
      "path": "<R>/epsilon",
      "sessions": 2
    }
  ]
}"##;

/// The base folder of the edge store's repos. Under `CARGO_TARGET_TMPDIR`; the kept one is fixed so Python can read it.
struct Edge {
    root: PathBuf,
    conn: Connection,
    keep: bool,
    n: i64,
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

const TIE: &str = "2026-01-15T10:00:00.000000Z";

impl Edge {
    fn new() -> Edge {
        static N: AtomicU32 = AtomicU32::new(0);
        let keep = std::env::var_os("T022_KEEP").is_some();
        let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
        let root = if keep {
            base.join("t022-kept")
        } else {
            base.join(format!(
                "t022-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::SeqCst)
            ))
        };
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let root = fs::canonicalize(&root).unwrap();
        let temp = fs::canonicalize(std::env::temp_dir()).unwrap();
        assert!(
            !root.starts_with(&temp) && !root.starts_with("/tmp"),
            "the repos would be scratch (rule a): run with the target folder outside the temp folder"
        );
        let path = root.join("store.sqlite");
        fs::copy(parity_dir().join("store-v11.sqlite"), &path).unwrap();
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("DELETE FROM events; DELETE FROM workspace_repos")
            .unwrap();
        let mut edge = Edge {
            root,
            conn,
            keep,
            n: 0,
        };
        edge.build();
        edge
    }

    /// The path of a folder under the base, as the store holds it.
    fn p(&self, name: &str) -> String {
        self.root.join(name).to_str().unwrap().to_string()
    }

    fn repo(&self, name: &str) {
        let dir = self.root.join(name);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q"], None);
        git(
            &dir,
            &["commit", "-q", "--allow-empty", "-m", "c"],
            Some("2026-01-14T00:00:00Z"),
        );
    }

    /// One event row. `tokens` None = a record with no usage (it is not a message).
    #[allow(clippy::too_many_arguments)]
    fn ev(
        &mut self,
        ts: &str,
        session: Option<&str>,
        project: &str,
        branch: Option<&str>,
        model: Option<&str>,
        message: Option<&str>,
        tokens: Option<[i64; 4]>,
        abstained: i64,
    ) {
        self.n += 1;
        let t = tokens.map(|t| t.map(Some)).unwrap_or([None; 4]);
        self.conn
            .execute(
                "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id,
                    project_path, git_branch, is_sidechain, model, message_id, input_tokens, output_tokens,
                    cache_creation_input_tokens, cache_read_input_tokens, entrypoint)
                 VALUES ('m', 'claude_code', 'p', 1, ?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?8, ?9, ?10, ?11, ?12, 'cli')",
                params![
                    self.n, abstained, ts, session, project, branch, model, message, t[0], t[1], t[2], t[3]
                ],
            )
            .unwrap();
    }

    fn build(&mut self) {
        for name in ["alpha", "beta", "gamma", "delta", "epsilon", "covered"] {
            self.repo(name);
        }
        git(&self.root.join("alpha"), &["branch", "feat-x"], None);
        let (alpha, beta, gamma, delta, epsilon, covered) = (
            self.p("alpha"),
            self.p("beta"),
            self.p("gamma"),
            self.p("delta"),
            self.p("epsilon"),
            self.p("covered"),
        );
        let (gone_wt, elsewhere, covered_sub) = (
            self.p("alpha/gone-wt"),
            "/work/edge/elsewhere-gone".to_string(),
            self.p("covered/gone-sub"),
        );
        let (sonnet, opus, unlisted) = (
            Some("claude-sonnet-5-5"),
            Some("claude-opus-5-5"),
            Some("claude-unlisted-model"),
        );
        // alpha: a message written as two records (the maxima count once), a gone worktree under it (parent rule),
        // a gone folder whose branch is alpha's `feat-x` (branch rule): three sessions, newest use 2026-01-18
        self.ev(
            "2026-01-10T10:00:00.000000Z",
            Some("s1"),
            &alpha,
            Some("main"),
            sonnet,
            Some("mA"),
            Some([10, 1, 0, 0]),
            0,
        );
        self.ev(
            "2026-01-10T10:00:05.000000Z",
            Some("s1"),
            &alpha,
            Some("main"),
            sonnet,
            Some("mA"),
            Some([10, 5, 200, 1000]),
            0,
        );
        self.ev(
            "2026-01-12T10:00:00.000000Z",
            Some("s2"),
            &gone_wt,
            Some("wt"),
            opus,
            Some("mB"),
            Some([1000, 500, 0, 200]),
            0,
        );
        self.ev(
            "2026-01-18T10:00:00.000000Z",
            Some("s3"),
            &elsewhere,
            Some("feat-x"),
            sonnet,
            Some("mC"),
            Some([7, 3, 1, 2]),
            0,
        );
        // beta and gamma: one message each at the same instant (a tie: the path, descending)
        self.ev(
            TIE,
            Some("sb"),
            &beta,
            Some("main"),
            sonnet,
            Some("mD"),
            Some([100, 50, 0, 0]),
            0,
        );
        self.ev(
            TIE,
            Some("sg"),
            &gamma,
            Some("main"),
            opus,
            Some("mE"),
            Some([100, 50, 0, 0]),
            0,
        );
        // an abstained record and a record with no usage never count (gamma's newest time stays the tie)
        self.ev(
            "2026-01-19T10:00:00.000000Z",
            Some("sg"),
            &gamma,
            None,
            sonnet,
            Some("mF"),
            Some([1, 1, 1, 1]),
            1,
        );
        self.ev(
            "2026-01-19T11:00:00.000000Z",
            Some("sg"),
            &gamma,
            None,
            None,
            None,
            None,
            0,
        );
        // delta: only a model missing from the price table -> usd null
        self.ev(
            "2026-01-11T10:00:00.000000Z",
            Some("sd"),
            &delta,
            Some("main"),
            unlisted,
            Some("mG"),
            Some([5, 5, 5, 5]),
            0,
        );
        // epsilon: a priced message, a message of the unlisted model, one with no model, one with an empty model, and
        // two sessions that are NULL (one session in Python's set) beside a named one
        self.ev(
            "2026-01-09T10:00:00.000000Z",
            Some("se"),
            &epsilon,
            Some("main"),
            sonnet,
            Some("mH"),
            Some([1000, 1000, 1000, 1000]),
            0,
        );
        self.ev(
            "2026-01-09T11:00:00.000000Z",
            None,
            &epsilon,
            Some("main"),
            unlisted,
            Some("mI"),
            Some([5, 5, 5, 5]),
            0,
        );
        self.ev(
            "2026-01-09T12:00:00.000000Z",
            None,
            &epsilon,
            Some("main"),
            None,
            Some("mJ"),
            Some([5, 5, 5, 5]),
            0,
        );
        self.ev(
            "2026-01-09T13:00:00.000000Z",
            None,
            &epsilon,
            Some("main"),
            Some(""),
            Some("mK"),
            Some([5, 5, 5, 5]),
            0,
        );
        // the scratch bucket: a gone folder with no owner (skipped), and a folder covered by a workspace repo
        self.ev(
            "2026-01-20T10:00:00.000000Z",
            Some("sx"),
            "/work/edge/scratch-a",
            Some("main"),
            sonnet,
            Some("mL"),
            Some([9, 9, 9, 9]),
            0,
        );
        self.ev(
            "2026-01-20T11:00:00.000000Z",
            Some("sc"),
            &covered,
            Some("main"),
            sonnet,
            Some("mM"),
            Some([9, 9, 9, 9]),
            0,
        );
        self.ev(
            "2026-01-20T12:00:00.000000Z",
            Some("sc"),
            &covered_sub,
            Some("main"),
            sonnet,
            Some("mN"),
            Some([9, 9, 9, 9]),
            0,
        );
        // workspaces: an id order that is not the name order, a repo shared by two, one with no repo, byte-order repos
        let alp = self.p("alp");
        let workspaces = [
            (
                "b2",
                [
                    Some("#00ff7f"),
                    Some("rocket"),
                    Some("text \u{fc}n\u{ef}"),
                    Some("a@b.invalid"),
                ],
                vec![&covered, "/work/Z", "/work/\u{e9}", "/work/a", "/work/z"],
            ),
            ("a-1", [None, None, None, None], vec![&covered, &alp]),
            ("1x", [None, None, None, None], vec![]),
            ("a", [None, None, Some("no repo"), None], vec![]),
        ];
        for (name, fields, repos) in workspaces {
            self.conn
                .execute(
                    "INSERT INTO workspaces (name, color, icon, description, email) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![name, fields[0], fields[1], fields[2], fields[3]],
                )
                .unwrap();
            let id = self.conn.last_insert_rowid();
            for repo in repos {
                self.conn
                    .execute(
                        "INSERT INTO workspace_repos VALUES (?1, ?2)",
                        params![id, repo],
                    )
                    .unwrap();
            }
        }
    }

    /// The expected JSON with `<R>` replaced by the base folder.
    fn expect(&self, text: &str) -> Value {
        serde_json::from_str(&text.replace("<R>", self.root.to_str().unwrap())).unwrap()
    }
}

impl Drop for Edge {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

/// With `T022_KEEP=1` this test only builds the store and leaves it for the Python command above.
#[test]
fn edge_store_python_values() {
    let edge = Edge::new();
    if edge.keep {
        return;
    }
    assert_eq!(list_all(&edge.conn).unwrap(), edge.expect(EDGE_WORKSPACES));
    assert_eq!(
        unassigned(&edge.conn).unwrap(),
        edge.expect(EDGE_UNASSIGNED)
    );
}

#[test]
fn a_store_with_no_workspace_row_and_no_usage_answers_empty_lists() {
    // Python: workspaces.list_all(conn) == [] and workspaces.unassigned(conn) == [] over the same shape.
    let dir = TempDir::new();
    let path = dir.store("store-v11.sqlite", "DELETE FROM events");
    Connection::open(&path)
        .unwrap()
        .execute_batch(
            "DROP TRIGGER workspaces_orphan_no_delete; DELETE FROM workspace_repos; DELETE FROM workspaces",
        )
        .unwrap();
    let conn = open_read_only(&path).unwrap();
    assert_eq!(list_all(&conn).unwrap(), json!({"workspaces": []}));
    assert_eq!(unassigned(&conn).unwrap(), json!({"repos": []}));
}
