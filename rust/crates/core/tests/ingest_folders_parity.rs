//! T035: the Rust ingest of folders (`ingest::folders::ingest_accounts`, `ingest::accounts::detect`) replays the T026
//! ingest oracle. Each scenario rebuilds the oracle's inputs exactly (`gen_ingest.py` `Case`: the work root padded to
//! 240 characters, `repos.json` steps, the fixed git env) and the `events`, `tool_signals`, `source_files` and
//! `agent_accounts` tables after every run must equal the golden row for row, every column (ids and machine id
//! dropped, paths and inodes tokenised as the golden says), with Python's `files` / `new_events`.
//! Then the four "Rust differs" inputs (Python crashes), and the offset and account rules the goldens do not reach.

mod ingest_folders_common;

use ingest_folders_common::{Case, committed, fixtures, golden, repos};
use rusqlite::Connection;
use serde_json::Value;
use std::fs;
use std::io::Write;

fn runs(g: &Value) -> &Vec<Value> {
    g["runs"].as_array().unwrap()
}

/// One ingest run checked against golden run `i`.
fn run_and_check(case: &Case, g: &Value, i: usize, projects_override: Option<&std::path::Path>) {
    let run = &runs(g)[i];
    let what = format!("{} run {i}", g["scenario"].as_str().unwrap());
    assert_eq!(run["exit_code"], 0, "{what}: Python succeeded");
    let summary = case.ingest(projects_override);
    Case::assert_summary(summary, run, &what);
    case.assert_tables(run, &what);
}

#[test]
fn s01_cold_ingest_of_the_committed_fixtures() {
    let g = golden("S01_cold_committed.json");
    let case = Case::new();
    case.copy(&committed(), &case.projects);
    run_and_check(&case, &g, 0, None);
}

#[test]
fn s02_the_same_ingest_twice_adds_nothing() {
    let g = golden("S02_rerun_unchanged.json");
    let case = Case::new();
    case.copy(&committed(), &case.projects);
    run_and_check(&case, &g, 0, None);
    run_and_check(&case, &g, 1, None);
}

#[test]
fn s03_an_appended_line_is_read_incrementally() {
    let g = golden("S03_line_appended.json");
    let case = Case::new();
    let target = case.projects.join("s3/session.jsonl");
    case.copy(&fixtures().join("s3/session.jsonl"), &target);
    run_and_check(&case, &g, 0, None);
    let extra = fs::read(fixtures().join("s3/append.jsonl")).unwrap();
    fs::OpenOptions::new()
        .append(true)
        .open(&target)
        .unwrap()
        .write_all(&extra)
        .unwrap();
    run_and_check(&case, &g, 1, None);
}

#[test]
fn s04_offsets_reset_back_fill_without_new_rows() {
    let g = golden("S04_offsets_reset_backfill.json");
    let case = Case::new();
    case.copy(&committed(), &case.projects);
    run_and_check(&case, &g, 0, None);
    case.sql(
        "UPDATE events SET entrypoint = NULL, tool_error_count = NULL; UPDATE source_files SET byte_offset = 0;",
    );
    case.assert_tables(&runs(&g)[1], "S04 after the reset");
    run_and_check(&case, &g, 2, None);
}

#[test]
fn s05_a_truncated_last_line_is_read_once_completed() {
    let g = golden("S05_truncated_then_completed.json");
    let case = Case::new();
    let target = case.projects.join("s5/session.jsonl");
    case.copy(&fixtures().join("s5/session.jsonl"), &target);
    run_and_check(&case, &g, 0, None);
    let rest = fs::read(fixtures().join("s5/rest.part")).unwrap();
    fs::OpenOptions::new()
        .append(true)
        .open(&target)
        .unwrap()
        .write_all(&rest)
        .unwrap();
    run_and_check(&case, &g, 1, None);
}

/// Project identity through git (repo, subfolder, worktree, empty repo, symlinks, trailing slash, gone, relative)
/// and the symlinked folder that is not followed.
#[test]
fn s06_cold_ingest_of_the_synthetic_set() {
    let g = golden("S06_cold_synthetic.json");
    let layout = repos();
    let case = Case::new();
    case.apply(&layout["work"]);
    case.copy(&fixtures().join("synthetic"), &case.projects);
    case.copy(&fixtures().join("outside"), &case.agents.join("outside"));
    case.apply(&layout["synthetic"]);
    run_and_check(&case, &g, 0, None);
}

/// Two confirmed accounts. The oracle confirmed `.claude-work` through the real CLI (`account confirm`, which runs
/// `detect` first); this test runs `detect`, then applies the same final row state by SQL (status `confirmed`,
/// label "Claude work").
#[test]
fn s07_two_confirmed_accounts() {
    let g = golden("S07_two_accounts.json");
    let case = Case::new();
    for entry in fs::read_dir(fixtures().join("s7/agents")).unwrap() {
        let entry = entry.unwrap();
        let name = format!(".{}", entry.file_name().to_str().unwrap());
        case.copy(&entry.path(), &case.agents.join(name));
    }
    fs::create_dir_all(&case.home).unwrap();
    let mut store = easy_personal_core::writer::open_for_write(&case.store_path()).unwrap();
    easy_personal_core::ingest::accounts::detect(&mut store, &case.agents).unwrap();
    drop(store);
    let work = case.agents.join(".claude-work");
    Connection::open(case.store_path())
        .unwrap()
        .execute(
            "UPDATE agent_accounts SET status = 'confirmed', label = 'Claude work' WHERE folder = ?",
            [work.to_str().unwrap()],
        )
        .unwrap();
    case.assert_tables(&runs(&g)[0], "S07 after confirm");
    run_and_check(&case, &g, 1, None);
}

/// `--retention-days 36500` prunes nothing; the retention flag is T029's, the ingest itself is this one.
#[test]
fn s08a_ingest_under_a_retention_that_prunes_nothing() {
    let g = golden("S08a_retention_36500.json");
    let case = Case::new();
    case.copy(&committed(), &case.projects);
    run_and_check(&case, &g, 0, None);
}

#[test]
fn s10a_override_replaces_the_default_accounts_folder() {
    let g = golden("S10a_source_override_default_account.json");
    let case = Case::new();
    case.copy(
        &fixtures().join("s3/session.jsonl"),
        &case.projects.join("own/session.jsonl"),
    );
    let over = case.work.join("override");
    case.copy(&committed(), &over);
    run_and_check(&case, &g, 0, Some(&over));
}

#[test]
fn s10b_override_without_an_account_is_read_untagged() {
    let g = golden("S10b_source_override_no_account.json");
    let case = Case::new();
    let over = case.work.join("override");
    case.copy(&committed(), &over);
    run_and_check(&case, &g, 0, Some(&over));
}

#[test]
fn s13_a_replaced_file_is_read_again_from_zero() {
    let g = golden("S13_file_replaced_new_inode.json");
    let case = Case::new();
    let target = case.projects.join("s13/session.jsonl");
    case.copy(&fixtures().join("s3/session.jsonl"), &target);
    run_and_check(&case, &g, 0, None);
    let copy = target.with_file_name("copy.tmp");
    fs::write(&copy, fs::read(&target).unwrap()).unwrap();
    fs::rename(&copy, &target).unwrap();
    run_and_check(&case, &g, 1, None);
}

#[test]
fn s14_a_deleted_file_is_retired_and_its_events_stay() {
    let g = golden("S14_file_deleted.json");
    let case = Case::new();
    case.copy(
        &fixtures().join("s3/session.jsonl"),
        &case.projects.join("s14/a.jsonl"),
    );
    case.copy(
        &fixtures().join("s3/append.jsonl"),
        &case.projects.join("s14/b.jsonl"),
    );
    run_and_check(&case, &g, 0, None);
    fs::remove_file(case.projects.join("s14/b.jsonl")).unwrap();
    run_and_check(&case, &g, 1, None);
}

/// A root that does not exist now retires nothing (claude_code.py:254,268): the default account's folder vanishes,
/// its `source_files` rows stay.
#[test]
fn a_missing_root_retires_nothing() {
    let case = Case::new();
    case.copy(
        &fixtures().join("s3/session.jsonl"),
        &case.projects.join("s/a.jsonl"),
    );
    assert_eq!(case.ingest(None).files, 1);
    fs::remove_dir_all(&case.projects).unwrap();
    let again = case.ingest(None);
    assert_eq!((again.files, again.new_events), (0, 0));
    let n: i64 = Connection::open(case.store_path())
        .unwrap()
        .query_row("SELECT COUNT(*) FROM source_files", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 1, "the vanished root's offsets are kept");
}

// -- Rust differs: Python crashes on these inputs (exit 1, the file rolled back, later files never read, every next
// -- run crashes again). Rust abstains the line / skips the entry, reads the files after it, and a second run is clean.

/// `(file under projects/, record_type, abstained, session suffix)` of every events row in id order.
type Row = (&'static str, Option<&'static str>, i64, &'static str);

fn rust_differs(scenario: &str, folder: &str, want_files: &[&str], want_rows: &[Row]) {
    let g = golden(scenario);
    for run in runs(&g) {
        assert_eq!(run["exit_code"], 1, "{scenario}: Python crashes here");
    }
    let case = Case::new();
    case.copy(&fixtures().join(folder), &case.projects);
    if folder == "crash-dir" {
        case.apply(&repos()["crash-dir"]);
    }
    let skipped = i64::from(folder == "crash-dir");
    let first = case.ingest(None);
    assert_eq!(first.files, want_files.len() as i64, "{scenario}: files");
    assert_eq!(
        first.new_events,
        want_rows.len() as i64,
        "{scenario}: new_events"
    );
    assert_eq!(first.skipped, skipped, "{scenario}: skipped");
    let check = |what: &str| {
        let c = Connection::open(case.store_path()).unwrap();
        let prefix = format!("{}/", case.projects.to_str().unwrap());
        let rows: Vec<(String, Option<String>, i64, String)> = c
            .prepare(
                "SELECT source_path, record_type, abstained, session_id FROM events ORDER BY id",
            )
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .map(|r| {
                let (p, t, a, s): (String, Option<String>, i64, Option<String>) = r.unwrap();
                (
                    p.strip_prefix(&prefix).unwrap().to_owned(),
                    t,
                    a,
                    s.unwrap_or_default(),
                )
            })
            .collect();
        let want: Vec<(String, Option<String>, i64, String)> = want_rows
            .iter()
            .map(|(f, t, a, s)| {
                let session = if s.is_empty() {
                    String::new()
                } else {
                    format!("00000000-0000-4000-8000-00000000{s}")
                };
                ((*f).to_owned(), t.map(str::to_owned), *a, session)
            })
            .collect();
        assert_eq!(rows, want, "{scenario} {what}: events in id order");
        let files: Vec<(String, i64)> = c
            .prepare("SELECT path, byte_offset FROM source_files ORDER BY path")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        let want: Vec<(String, i64)> = want_files
            .iter()
            .map(|f| {
                let path = case.projects.join(f);
                (
                    path.to_str().unwrap().to_owned(),
                    fs::metadata(&path).unwrap().len() as i64,
                )
            })
            .collect();
        assert_eq!(files, want, "{scenario} {what}: every file read to its end");
    };
    check("first run");
    // The rows Python stored before it crashed are among ours, unchanged.
    let mine = case.dump();
    for row in runs(&g)[0]["tables_after"]["events"]["rows"]
        .as_array()
        .unwrap()
    {
        assert!(
            mine["events"].as_array().unwrap().contains(row),
            "{scenario}: Python's row kept"
        );
    }
    let second = case.ingest(None);
    assert_eq!(
        (second.files, second.new_events, second.skipped),
        (first.files, 0, skipped),
        "{scenario}: second run clean"
    );
    check("second run");
}

/// S06a (Python: RecursionError): 100,000 nested `[` is an all-NULL abstained row; `p-q/z.jsonl`, sorted after
/// `p/m-deep.jsonl` (path components), is read.
#[test]
fn s06a_deep_nesting_abstains_and_later_files_are_read() {
    rust_differs(
        "S06a_crash_deep_nesting.json",
        "crash-deep",
        &["a.jsonl", "p-q/z.jsonl", "p/m-deep.jsonl"],
        &[
            ("a.jsonl", Some("assistant"), 0, "1020"),
            ("p/m-deep.jsonl", Some("user"), 0, "1021"),
            ("p/m-deep.jsonl", None, 1, ""),
            ("p/m-deep.jsonl", Some("user"), 0, "1021"),
            ("p-q/z.jsonl", Some("assistant"), 0, "1022"),
        ],
    );
}

/// S06b (Python: OverflowError): `input_tokens` 2**63 abstains the record, keeping type/session.
#[test]
fn s06b_int_beyond_int64_abstains_and_later_files_are_read() {
    rust_differs(
        "S06b_crash_int_beyond_int64.json",
        "crash-bigint",
        &["a.jsonl", "m-bigint.jsonl", "z.jsonl"],
        &[
            ("a.jsonl", Some("assistant"), 0, "1023"),
            ("m-bigint.jsonl", Some("user"), 0, "1024"),
            ("m-bigint.jsonl", Some("assistant"), 1, "1024"),
            ("m-bigint.jsonl", Some("user"), 0, "1024"),
            ("z.jsonl", Some("assistant"), 0, "1025"),
        ],
    );
}

/// S06c (Python: IsADirectoryError): the folder `x.jsonl` is skipped (counted), not an error; `z.jsonl` is read.
#[test]
fn s06c_directory_named_jsonl_is_skipped_and_later_files_are_read() {
    rust_differs(
        "S06c_crash_directory_named_jsonl.json",
        "crash-dir",
        &["a.jsonl", "z.jsonl"],
        &[
            ("a.jsonl", Some("assistant"), 0, "1029"),
            ("z.jsonl", Some("assistant"), 0, "1032"),
        ],
    );
}

/// S06d (Python: TypeError, unhashable): `"type": ["user"]` abstains with `record_type` NULL.
#[test]
fn s06d_list_type_abstains_and_later_files_are_read() {
    rust_differs(
        "S06d_crash_unhashable_type.json",
        "crash-type",
        &["a.jsonl", "m-type.jsonl", "z.jsonl"],
        &[
            ("a.jsonl", Some("assistant"), 0, "1026"),
            ("m-type.jsonl", Some("user"), 0, "1027"),
            ("m-type.jsonl", None, 1, "1027"),
            ("m-type.jsonl", Some("user"), 0, "1027"),
            ("z.jsonl", Some("assistant"), 0, "1028"),
        ],
    );
}

// -- the offset and account rules

fn line(session: &str, ts: &str) -> String {
    format!(
        "{{\"type\":\"user\",\"sessionId\":\"{session}\",\"timestamp\":\"{ts}\",\"version\":\"2.1.400\",\
         \"cwd\":\"/nonexistent-t035/app\",\"message\":{{\"role\":\"user\",\"content\":\"x\"}}}}\n"
    )
}

/// `offset > st_size` restarts from 0: a file rewritten in place (same inode) shorter than the stored offset is read
/// from its start, and its stored offset ends at its new end.
#[test]
fn a_shrunk_file_with_the_same_inode_restarts_from_zero() {
    let case = Case::new();
    let path = case.projects.join("s/a.jsonl");
    let (a, b, c) = (
        line("t035-a", "2026-01-10T10:00:00Z"),
        line("t035-b", "2026-01-10T10:00:01Z"),
        line("t035-cc", "2026-01-10T10:00:02Z"),
    );
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, format!("{a}{b}")).unwrap();
    assert_eq!(case.ingest(None).new_events, 2);
    let mut f = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&path)
        .unwrap();
    f.write_all(c.as_bytes()).unwrap();
    drop(f);
    let again = case.ingest(None);
    let offset: i64 = Connection::open(case.store_path())
        .unwrap()
        .query_row("SELECT byte_offset FROM source_files", [], |r| r.get(0))
        .unwrap();
    assert_eq!(offset, c.len() as i64, "read from 0 to the new end");
    assert_eq!(
        again.new_events, 0,
        "offset 0 is a known key: the upsert keeps one row"
    );
}

/// claude_code.py:299-302: untagged rows (stored with no default account) become the default account's once it exists.
#[test]
fn untagged_rows_are_given_to_the_default_account() {
    let case = Case::new();
    let over = case.work.join("override");
    case.copy(&committed(), &over);
    assert_eq!(case.ingest(Some(&over)).new_events, 39);
    fs::create_dir_all(&case.projects).unwrap();
    let again = case.ingest(Some(&over));
    assert_eq!(again.new_events, 0);
    let c = Connection::open(case.store_path()).unwrap();
    let nulls: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM events WHERE agent_account_id IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(nulls, 0, "every untagged row now names the default account");
}

/// Files are saved in sorted path-component order (`p/` before `p-q/`), i.e. event ids follow that order.
#[test]
fn files_are_read_in_path_component_order() {
    let case = Case::new();
    for (i, rel) in ["p-q/z.jsonl", "p/m.jsonl", "a.jsonl"].iter().enumerate() {
        let path = case.projects.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, line(&format!("t035-{i}"), "2026-01-10T10:00:00Z")).unwrap();
    }
    case.ingest(None);
    let prefix = format!("{}/", case.projects.to_str().unwrap());
    let order: Vec<String> = Connection::open(case.store_path())
        .unwrap()
        .prepare("SELECT source_path FROM events ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|p| p.unwrap().strip_prefix(&prefix).unwrap().to_owned())
        .collect();
    assert_eq!(order, ["a.jsonl", "p/m.jsonl", "p-q/z.jsonl"]);
}

/// A folder that cannot be listed is a counted skip, never fatal; the other files are read.
#[test]
fn an_unreadable_folder_is_a_counted_skip() {
    use std::os::unix::fs::PermissionsExt;
    let case = Case::new();
    let locked = case.projects.join("locked");
    fs::create_dir_all(&locked).unwrap();
    fs::write(
        locked.join("x.jsonl"),
        line("t035-x", "2026-01-10T10:00:00Z"),
    )
    .unwrap();
    fs::write(
        case.projects.join("ok.jsonl"),
        line("t035-ok", "2026-01-10T10:00:00Z"),
    )
    .unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let listable = fs::read_dir(&locked).is_ok(); // root ignores the mode
    let summary = case.ingest(None);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    if listable {
        return;
    }
    assert_eq!(
        (summary.files, summary.new_events, summary.skipped),
        (1, 1, 1)
    );
}
