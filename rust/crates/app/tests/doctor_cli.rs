//! T060 AC3: what the Python oracle cannot be asked about `doctor`. Python's lines are proven by
//! `connector_cli_parity.rs` (the `doctor_*` cases). Here: it only reads (store bytes and keychain unchanged), a `claude`
//! that cannot run is a `FAIL` line and never "no entry", a keychain that refuses a read is a `FAIL` line (Python crashes),
//! and a missing store is refused with nothing created.

mod connector_world;

use connector_world::*;
use serde_json::{Value, json};
use std::process::{Command, Output, Stdio};

fn world(setup: &[&str], keychain: Value, claude: Value) -> (World, Value) {
    let case = json!({
        "name": "t060-doctor", "setup": setup, "dirs": ["agent-root/.claude", "agent-root/.claude-work/projects"],
        "keychain": keychain, "claude": claude, "fail": null,
    });
    (World::build(&case), case)
}

fn doctor(w: &World, case: &Value) -> Output {
    let mut cmd = Command::new(SERVE);
    w.env(&mut cmd, case, "UTC");
    cmd.arg("doctor").stdin(Stdio::null()).output().unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

const REGISTERED: [&str; 2] = [
    "INSERT INTO connectors VALUES ('slack-acme', 'acme-slack', 0, 'slack', 2)",
    "INSERT INTO account_registrations VALUES (2, 'acme-slack', 'slack-acme')",
];

#[test]
fn doctor_reads_only() {
    let items = json!({"easy-personal/slack": "{}", "easy-personal/slack-acme": "{}"});
    let (w, case) = world(
        &REGISTERED,
        items.clone(),
        json!({"<unset>": {"slack": ["/usr/bin/other", "x"]}}),
    );
    let bytes = std::fs::read(&w.store).unwrap();
    let out = doctor(&w, &case);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stdout));
    assert_eq!(
        std::fs::read(&w.store).unwrap(),
        bytes,
        "the store file is unchanged"
    );
    assert_eq!(w.keychain_after(), items);
    let log = std::fs::read_to_string(&w.log).unwrap();
    let writes: Vec<&str> = log
        .lines()
        .filter(|l| *l == "add-json" || *l == "remove")
        .collect();
    assert!(writes.is_empty(), "only `claude mcp get`: {log}");
}

/// T066 AC3 and AC4 (replaces T060's per-entry lines): a `claude` that cannot run (it does not start) is one `FAIL`
/// line, never "no entry" and never one line per catalog id or registration; the legacy note says it was not checked.
#[test]
fn a_claude_that_cannot_run_is_one_fail_line_never_no_entry() {
    let items = json!({
        "easy-personal/slack": "{}", "easy-personal/gmail": "{}", "easy-personal/slack-acme": "{}",
    });
    let (w, case) = world(&REGISTERED, items, json!({}));
    std::fs::write(
        w.dir.join("bin/claude"),
        "#!/nonexistent/t060-interpreter\n",
    )
    .unwrap();
    let out = doctor(&w, &case);
    assert_eq!(out.status.code(), Some(1));
    let stdout = text(&out.stdout);
    assert!(stdout.contains("ok claude: on PATH\n"), "{stdout}");
    let fails: Vec<&str> = stdout.lines().filter(|l| l.starts_with("FAIL")).collect();
    assert_eq!(
        fails,
        [
            "FAIL claude: `claude` could not run (it did not start or timed out); entries not checked"
        ],
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "note legacy connectors not checked: `claude` could not run (it did not start or timed out)\n"
        ),
        "{stdout}"
    );
    assert!(!stdout.contains("no Claude Code entry"), "{stdout}");
    assert!(!stdout.contains("drift: missing"), "{stdout}");
}

/// T066 AC3: a `claude` that ran and answered "no entry" keeps the old lines.
#[test]
fn a_claude_that_answers_no_entry_keeps_the_old_lines() {
    let (w, case) = world(&[], json!({"easy-personal/slack": "{}"}), json!({}));
    let out = doctor(&w, &case);
    assert_eq!(out.status.code(), Some(1));
    let stdout = text(&out.stdout);
    assert!(
        stdout.contains(
            "FAIL slack: keychain item present but no Claude Code entry; run connector add\n"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "note legacy connectors not adopted: slack; run `easy-personal connector adopt-legacy`\n"
        ),
        "{stdout}"
    );
    assert!(!stdout.contains("could not run"), "{stdout}");
}

#[test]
fn a_keychain_that_refuses_a_read_is_a_fail_line() {
    let (w, case) = world(&REGISTERED, json!({}), json!({}));
    std::fs::write(&w.keychain, "locked").unwrap();
    let out = doctor(&w, &case);
    assert_eq!(out.status.code(), Some(1));
    let stdout = text(&out.stdout);
    assert!(
        stdout.starts_with("ok keychain: FileKeychain\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("FAIL slack: no usable OS keychain (Secret Service);"),
        "{stdout}"
    );
    assert!(!stdout.contains("note legacy"), "{stdout}");
    assert_eq!(std::fs::read_to_string(&w.keychain).unwrap(), "locked");
}

#[test]
fn a_missing_store_is_refused_and_nothing_is_created() {
    let (w, case) = world(&[], json!({}), json!({}));
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", w.store.display()));
    }
    let out = doctor(&w, &case);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(text(&out.stdout), "");
    assert_eq!(
        text(&out.stderr),
        "easy-personal-rs: no store yet: nothing has been ingested into the data folder\n"
    );
    assert!(!w.store.exists());
}
