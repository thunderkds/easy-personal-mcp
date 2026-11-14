//! T058: what the Python oracle (`cli_parity.rs`) cannot ask, over the real binary. A store that is missing or newer is refused
//! and not touched; a read command writes nothing; a change that needs the `claude` CLI is refused with the T048 text and
//! changes nothing; a plan that changes while the question is open is refused; account detection reads only the agent root;
//! `workspace create` is `workspace add`; `dashboard` is `serve` (it ingests first, `--no-ingest` skips it). `HOME`,
//! `EASY_PERSONAL_HOME` and `EASY_PERSONAL_AGENT_ROOT` are all inside one temp folder, so the owner's `~/.easy-personal`,
//! `~/.claude*` and `~/.codex*` are never reached.

use rusqlite::Connection;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

const NO_STORE: &str = "no store yet: nothing has been ingested into the data folder";

struct Home {
    dir: PathBuf,
}

impl Home {
    fn new() -> Home {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t058-cmds-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        for sub in ["home", "nowhere", "agent-root"] {
            fs::create_dir_all(dir.join(sub)).unwrap();
        }
        Home {
            dir: dir.canonicalize().unwrap(),
        }
    }

    /// A copy of the rich parity store; its two accounts point into this folder's agent root.
    fn rich() -> Home {
        let home = Home::new();
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/store-rich/store-v11.sqlite"),
            home.store(),
        )
        .unwrap();
        let text = home.dir.to_str().unwrap();
        home.sql(&format!(
            "UPDATE agent_accounts SET folder = replace(folder, '/work/rich', '{text}'), \
             config_dir_env = replace(config_dir_env, '/work/rich', '{text}')"
        ));
        fs::create_dir_all(home.dir.join("agent-root/.claude")).unwrap();
        fs::create_dir_all(home.dir.join("agent-root/.claude-work")).unwrap();
        home
    }

    fn store(&self) -> PathBuf {
        self.dir.join("home/store.sqlite")
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"));
        c.args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", self.dir.join("nowhere"))
            .env("EASY_PERSONAL_HOME", self.dir.join("home"))
            .env("EASY_PERSONAL_AGENT_ROOT", self.dir.join("agent-root"))
            .env("EASY_PERSONAL_LOCK_TIMEOUT", "5")
            .env("EASY_PERSONAL_CONTEXT_URL", "http://127.0.0.1:9/mcp") // a closed port: never a real easy-context
            .env("TZ", "UTC")
            .current_dir(&self.dir);
        c
    }

    fn run(&self, args: &[&str]) -> Output {
        self.run_with(args, "")
    }

    fn run_with(&self, args: &[&str], stdin: &str) -> Output {
        let mut child = self
            .command(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        let _ = input.write_all(stdin.as_bytes());
        drop(input);
        child.wait_with_output().unwrap()
    }

    fn sql(&self, sql: &str) {
        let conn = Connection::open(self.store()).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        conn.execute_batch(sql).unwrap();
    }

    fn query(&self, sql: &str) -> Vec<String> {
        let conn = Connection::open(self.store()).unwrap();
        let mut stmt = conn.prepare(sql).unwrap();
        stmt.query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    /// Every file under the home folder: name and bytes (the store, its WAL files, anything a command made).
    fn files(&self) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        for entry in fs::read_dir(self.dir.join("home")).unwrap().flatten() {
            out.push((
                entry.file_name().to_string_lossy().into_owned(),
                fs::read(entry.path()).unwrap_or_default(),
            ));
        }
        out.sort();
        out
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn ping_prints_the_core_line_and_needs_no_store() {
    let home = Home::new();
    let done = home.run(&["ping"]);
    assert_eq!(done.status.code(), Some(0));
    assert_eq!(out(&done), "easy-personal core ok\n");
    assert_eq!(err(&done), "");
    assert!(home.files().is_empty(), "ping made a file");
}

#[test]
fn a_missing_store_is_refused_with_the_fixed_text_and_nothing_is_created() {
    let home = Home::new();
    let commands: [&[&str]; 9] = [
        &["usage"],
        &["usage", "--by", "day"],
        &["outliers"],
        &["kit-findings", "--json"],
        &["workspace", "list"],
        &["workspace", "add", "x"],
        &["workspace", "delete", "x", "--yes"],
        &["account", "list"],
        &["account", "confirm", "1", "--label", "x"],
    ];
    for args in commands {
        let done = home.run(args);
        assert_eq!(done.status.code(), Some(1), "{args:?}");
        assert_eq!(out(&done), "", "{args:?}");
        assert_eq!(
            err(&done),
            format!("easy-personal-rs: {NO_STORE}\n"),
            "{args:?}"
        );
    }
    assert!(home.files().is_empty(), "a command made a file");
}

#[test]
fn a_newer_store_is_refused_and_left_as_it_was() {
    let home = Home::new();
    home.sql("CREATE TABLE events (id INTEGER PRIMARY KEY, note TEXT); INSERT INTO events (note) VALUES ('keep'); PRAGMA user_version = 99;");
    let before = home.files();
    let commands: [&[&str]; 6] = [
        &["usage"],
        &["outliers"],
        &["kit-findings"],
        &["workspace", "list"],
        &["workspace", "add", "x"],
        &["account", "list"],
    ];
    for args in commands {
        let done = home.run(args);
        assert_eq!(done.status.code(), Some(2), "{args:?}");
        assert_eq!(out(&done), "");
        assert!(
            err(&done).contains("the store is at version 99, newer than the 11 this build knows"),
            "{}",
            err(&done)
        );
        assert!(
            !err(&done).contains(home.dir.to_str().unwrap()),
            "a path in {}",
            err(&done)
        );
    }
    assert_eq!(home.files(), before, "a refused command changed a file");
}

#[test]
fn a_read_command_writes_nothing() {
    let home = Home::rich();
    let before = fs::read(home.store()).unwrap();
    let commands: [&[&str]; 7] = [
        &["usage"],
        &["usage", "--by", "model", "--workspace", "acme"],
        &["outliers"],
        &["outliers", "--account", "2"],
        &["kit-findings"],
        &["kit-findings", "--json"],
        &["workspace", "list"],
    ];
    for args in commands {
        let done = home.run(args);
        assert_eq!(done.status.code(), Some(0), "{args:?}: {}", err(&done));
    }
    assert_eq!(
        fs::read(home.store()).unwrap(),
        before,
        "the store file changed"
    );
    // a delete that is answered no opens the store for writing, and changes no row (the file header may move)
    let rows = state(&home);
    let done = home.run(&["workspace", "delete", "acme"]);
    assert_eq!(done.status.code(), Some(1));
    assert_eq!(state(&home), rows);
    assert_eq!(
        home.query("SELECT name FROM workspaces ORDER BY id"),
        ["orphan", "acme", "personal"]
    );
}

#[test]
fn workspace_create_is_workspace_add() {
    let home = Home::rich();
    let done = home.run(&["workspace", "create", "made", "--color", "#010203"]);
    assert_eq!(done.status.code(), Some(0), "{}", err(&done));
    assert!(out(&done).contains("\"name\": \"made\""), "{}", out(&done));
    assert_eq!(
        home.query("SELECT name FROM workspaces WHERE name = 'made'"),
        ["made"]
    );
}

#[test]
fn a_question_with_no_terminal_does_not_hang_and_the_answer_is_no() {
    let home = Home::rich();
    let mut child = home
        .command(&["workspace", "delete", "acme"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        assert!(
            start.elapsed() < Duration::from_secs(20),
            "it waits for input"
        );
        sleep(Duration::from_millis(20));
    }
    let done = child.wait_with_output().unwrap();
    assert_eq!(done.status.code(), Some(1));
    assert!(
        out(&done).ends_with("Apply these changes? [y/N] \naborted; nothing was changed.\n"),
        "{}",
        out(&done)
    );
    assert_eq!(
        home.query("SELECT name FROM workspaces WHERE name = 'acme'"),
        ["acme"]
    );
}

/// `acme` holds two connectors and account 2 picks it and has them registered: the rename and the delete both need `claude`.
const WITH_REGISTRATIONS: &str = "\
    INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES ('slack-acme', 'acme-slack', 0, 'slack', 2); \
    INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES ('gmail-acme', 'acme-gmail', 0, 'gmail', 2); \
    INSERT INTO account_registrations (account_id, alias, connector_id) VALUES (2, 'acme-slack', 'slack-acme'); \
    INSERT INTO account_registrations (account_id, alias, connector_id) VALUES (2, 'acme-gmail', 'gmail-acme');";

fn state(home: &Home) -> Vec<String> {
    home.query(
        "SELECT 'w:' || id || ':' || name FROM workspaces UNION ALL SELECT 'c:' || id || ':' || alias || ':' || workspace_id FROM connectors \
         UNION ALL SELECT 'r:' || alias FROM account_registrations UNION ALL SELECT 'p:' || account_id || ':' || workspace_id FROM account_workspaces \
         ORDER BY 1",
    )
}

/// Since T050 the change runs the plan through the `claude` runner, as Python does: `PATH` is pinned to `/usr/bin:/bin`, so no
/// `claude` is found. The plan is shown first, the store change is made (exit 0), and the commands to run later are printed
/// (the T048 "needs the claude CLI" refusal is gone).
#[test]
fn a_change_with_no_claude_on_path_makes_the_store_change_and_prints_the_commands_as_python_does() {
    for (args, stdin) in [
        (
            vec!["workspace", "edit", "acme", "--rename", "company", "--yes"],
            "",
        ),
        (
            vec!["workspace", "edit", "acme", "--rename", "company"],
            "y\n",
        ),
        (vec!["workspace", "delete", "acme", "--yes"], ""),
        (vec!["workspace", "delete", "acme"], "yes\n"),
    ] {
        let home = Home::rich();
        home.sql(WITH_REGISTRATIONS);
        let before = state(&home);
        let done = home.run_with(&args, stdin);
        assert_eq!(done.status.code(), Some(0), "{args:?}: {}", err(&done));
        assert_eq!(err(&done), "", "{args:?}: nothing on stderr");
        assert!(
            out(&done).starts_with("will change ("),
            "the plan is shown first: {}",
            out(&done)
        );
        assert!(
            out(&done).contains("is not on PATH; nothing was written"),
            "{args:?}: {}",
            out(&done)
        );
        assert_ne!(state(&home), before, "{args:?} must change the store");
    }
}

#[test]
fn a_plan_that_changes_while_the_question_is_open_is_refused() {
    let home = Home::rich();
    home.sql("INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES ('slack-acme', 'acme-slack', 0, 'slack', 2); DELETE FROM account_workspaces;");
    let before = state(&home);
    let mut child = home
        .command(&["workspace", "delete", "acme"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut shown = Vec::new();
    let start = Instant::now();
    while !String::from_utf8_lossy(&shown).ends_with("[y/N] ") {
        assert!(
            start.elapsed() < Duration::from_secs(20),
            "no question: {}",
            String::from_utf8_lossy(&shown)
        );
        let mut byte = [0u8; 1];
        assert_eq!(
            stdout.read(&mut byte).unwrap(),
            1,
            "stdout closed: {}",
            String::from_utf8_lossy(&shown)
        );
        shown.push(byte[0]);
    }
    assert!(String::from_utf8_lossy(&shown).contains("connectors to orphan: slack-acme\n"));
    // the question is open and no store handle is held: another process changes what the plan would do
    home.sql("INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES ('gmail-acme', 'acme-gmail', 0, 'gmail', 2)");
    let changed = state(&home);
    child.stdin.take().unwrap().write_all(b"y\n").unwrap();
    let done = child.wait_with_output().unwrap();
    assert_eq!(done.status.code(), Some(2), "{}", err(&done));
    assert_eq!(
        err(&done),
        "easy-personal-rs: the plan changed while the question was open; nothing was changed\n"
    );
    assert_ne!(changed, before);
    assert_eq!(state(&home), changed, "the refused apply changed the store");
}

#[test]
fn account_detection_reads_only_the_agent_root() {
    let home = Home::new();
    // a store with no account yet; the agent root holds one new folder, HOME (a decoy) holds another
    fs::create_dir_all(home.dir.join("agent-root/.claude-found/projects")).unwrap();
    fs::create_dir_all(home.dir.join("nowhere/.claude-decoy/projects")).unwrap();
    fs::create_dir_all(home.dir.join("nowhere/.codex-decoy")).unwrap();
    fs::write(home.dir.join("nowhere/.codex-decoy/config.toml"), "").unwrap();
    drop(easy_personal_core::writer::open_for_write(&home.store()).unwrap());
    let done = home.run(&["account", "list"]);
    assert_eq!(done.status.code(), Some(0), "{}", err(&done));
    let text = out(&done);
    assert!(text.contains(".claude-found"), "{text}");
    assert!(
        !text.contains("decoy"),
        "a folder outside the agent root was detected: {text}"
    );
    assert_eq!(
        home.query("SELECT folder FROM agent_accounts ORDER BY id")
            .len(),
        1
    );
    // with no `EASY_PERSONAL_AGENT_ROOT`, the root is `HOME` (here a temp folder): Python's `config.agent_root()`
    let mut other = home.command(&["account", "list"]);
    other.env_remove("EASY_PERSONAL_AGENT_ROOT");
    let done = other.output().unwrap();
    assert!(out(&done).contains(".claude-decoy"), "{}", out(&done));
    // an explicit `account add` takes a folder anywhere, as Python's does
    let found = home.run(&[
        "account",
        "add",
        home.dir.join("nowhere/.codex-decoy").to_str().unwrap(),
    ]);
    assert_eq!(found.status.code(), Some(0), "{}", err(&found));
    assert!(out(&found).contains("\"provider\": \"codex\""));
}

#[test]
fn help_lists_every_new_command_and_dashboard_is_serve() {
    let home = Home::new();
    let help = out(&home.run(&["--help"]));
    for name in [
        "ping",
        "usage",
        "outliers",
        "kit-findings",
        "workspace",
        "account",
        "serve",
    ] {
        assert!(
            help.contains(&format!("\n  {name} ")),
            "{name} is not in --help:\n{help}"
        );
    }
    let dash = out(&home.run(&["dashboard", "--help"]));
    assert!(
        dash.contains("--no-ingest") && dash.contains("--static-dir"),
        "{dash}"
    );
    assert!(dash.starts_with("Serve the dashboard API"), "{dash}");
}

/// A started `dashboard`: the first line is read, then `finish` stops it with SIGINT (by its PID) and returns what it printed.
struct Dashboard {
    child: Child,
    stdout: BufReader<std::process::ChildStdout>,
    first: String,
}

impl Dashboard {
    fn start(home: &Home, args: &[&str]) -> Dashboard {
        let mut all = vec!["dashboard"];
        all.extend_from_slice(args);
        let mut child = home
            .command(&all)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut first = String::new();
        stdout.read_line(&mut first).unwrap();
        Dashboard {
            child,
            stdout,
            first,
        }
    }

    fn finish(mut self) -> (Option<i32>, String, String) {
        sleep(Duration::from_millis(300)); // the Ctrl-C handler is installed just after the two lines are printed
        let kill = Command::new("kill")
            .args(["-INT", &self.child.id().to_string()])
            .status()
            .unwrap();
        assert!(kill.success());
        let mut rest = String::new();
        let _ = self.stdout.read_to_string(&mut rest);
        let status = self.child.wait().unwrap();
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        (status.code(), format!("{}{rest}", self.first), stderr)
    }
}

const PROGRESS: &str = "ingesting before start (waits if another ingest runs)...";

fn transcript(home: &Home) {
    let projects = home.dir.join("agent-root/.claude/projects/p");
    fs::create_dir_all(&projects).unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/fixtures/claude_code/2.1.295/session.jsonl"),
        projects.join("s.jsonl"),
    )
    .unwrap();
}

fn events(home: &Home) -> i64 {
    match Connection::open(home.store()) {
        Ok(conn) => conn
            .query_row("SELECT count(*) FROM events", [], |r| r.get(0))
            .unwrap_or(0),
        Err(_) => 0,
    }
}

#[test]
fn dashboard_ingests_first_prints_the_url_line_and_stops_on_ctrl_c() {
    let home = Home::new();
    transcript(&home);
    let dash = Dashboard::start(&home, &[]);
    assert!(
        dash.first.starts_with("dashboard: http://127.0.0.1:"),
        "{}",
        dash.first
    );
    let token = dash
        .first
        .trim_end()
        .rsplit_once("/#")
        .unwrap()
        .1
        .to_string();
    assert_eq!(token.len(), 43, "{token}");
    assert!(events(&home) > 0, "it did not ingest before it started");
    let (code, stdout, stderr) = dash.finish();
    assert_eq!(code, Some(0), "{stderr}");
    assert!(stdout.ends_with("Ctrl-C to stop.\n"), "{stdout}");
    assert_eq!(stderr.lines().next(), Some(PROGRESS), "{stderr}");
}

#[test]
fn dashboard_no_ingest_does_not_ingest() {
    let home = Home::new();
    transcript(&home);
    let dash = Dashboard::start(&home, &["--no-ingest"]);
    assert!(
        dash.first.starts_with("dashboard: http://127.0.0.1:"),
        "{}",
        dash.first
    );
    assert_eq!(events(&home), 0);
    let (code, _, stderr) = dash.finish();
    assert_eq!(code, Some(0), "{stderr}");
    assert!(!stderr.contains(PROGRESS), "{stderr}");
}

#[test]
fn dashboard_says_one_fixed_line_when_the_ingest_fails_and_starts_anyway() {
    let home = Home::new();
    home.sql("CREATE TABLE events (id INTEGER PRIMARY KEY); PRAGMA user_version = 99;");
    let dash = Dashboard::start(&home, &[]);
    assert!(
        dash.first.starts_with("dashboard: http://127.0.0.1:"),
        "{}",
        dash.first
    );
    let (code, _, stderr) = dash.finish();
    assert_eq!(code, Some(0), "{stderr}");
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines[0], PROGRESS);
    assert!(
        lines.contains(&"ingest failed; starting anyway"),
        "{stderr}"
    );
    assert!(
        !stderr.contains(home.dir.to_str().unwrap()),
        "a path in {stderr}"
    );
}
