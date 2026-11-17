//! App sessions (T103, FR-109..FR-111, FR-115, DDR-0006): the app starts the owner's own `claude` with full CLI parity
//! and drives it over stream-json. T102 (`tasks/evidence-T102/SPIKE.md`, claude 2.1.326) proved the wiring used here.
//!
//! - **The child**: `claude -p --input-format stream-json --output-format stream-json --verbose` plus the form's choices
//!   (a model from [`MODELS`], a mode from [`MODES`]), `--permission-prompt-tool` and `--mcp-config <0600 temp file>`
//!   (not `--strict-mcp-config`: the owner's own MCP servers, settings, hooks and skills apply). The argument list holds
//!   no text of the owner's: the prompt goes in on stdin. No `--allowedTools` / `--disallowedTools`. Its own process
//!   group, `PR_SET_PDEATHSIG` (so it is started from the thread that supervises it), stdin and stdout piped, stderr on
//!   `/dev/null`. Environment: this process's minus `CLAUDE_CONFIG_DIR`, `ANTHROPIC_*`, `CLAUDECODE`, every
//!   `CLAUDE_CODE_*` and the FR-102 list ([`crate::agent_run::DROPPED`]), plus the chosen account's `CLAUDE_CONFIG_DIR`
//!   and `MCP_TOOL_TIMEOUT` (without it Claude Code cuts a tool call at 60 s and retries it).
//! - **Talk**: one stdin line per message. **Interrupt** is the stdin `control_request` of subtype `interrupt` (SIGINT
//!   ends the process, T102). **Stop** is SIGTERM to the process group, then SIGKILL after [`STOP_GRACE`]. The
//!   `system/init` event comes again for every user message: it is not a new session.
//! - **Prompts**: the child calls the tool `permission_prompt` of an HTTP MCP route of the dashboard process
//!   (`app_session_routes.rs`, per-session bearer secret). A call becomes a pending card; the owner's answer is the tool's
//!   documented result. The app never answers by itself: the call waits until an answer, the end of the turn or of the
//!   session. A retried call for the same `tool_use_id` is the same prompt and gets the same single answer.
//! - **FR-115**: the owner's text, the assistant text, the cards and the answers live in this module's memory only (no
//!   `Debug` on a type that holds text, no log line). They go out only through the guarded routes, with the hidden
//!   characters of FR-092 removed. A session's text is dropped when it has ended and its panel is closed, and all of it at
//!   exit ([`Sessions::shutdown`]).
//! - **Limits**: [`MAX_RUNNING`] sessions at once (question 41); no time limit. Setting turned off (or the session's account
//!   no longer a confirmed Claude account) stops the session.

use crate::agent_run::DROPPED;
use crate::claude_cli::which;
use easy_personal_core::ingest::json as pyjson;
use easy_personal_core::live::call_label;
use serde_json::{Map, Value, json};
use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tokio::sync::watch;

/// At most this many sessions run at once (PRD question 41); one more is a 409.
pub(crate) const MAX_RUNNING: usize = 4;
/// Ended sessions that are kept in the list until the owner closes them; the oldest go first past this.
const MAX_KEPT: usize = 12;
/// SIGTERM to SIGKILL.
const STOP_GRACE: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(50);
/// How often a going session checks that the setting is still on.
const SETTING_EVERY: Duration = Duration::from_secs(1);
/// The fixed lists of the form (FR-109); the first of each is "no flag".
pub(crate) const MODELS: [&str; 4] = ["default", "opus", "sonnet", "haiku"];
pub(crate) const MODES: [&str; 6] = [
    "default",
    "acceptEdits",
    "auto",
    "plan",
    "dontAsk",
    "bypassPermissions",
];
/// The permission tool's server name in `--mcp-config` and its tool: `mcp__easy-personal-app__permission_prompt`.
pub(crate) const SERVER: &str = "easy-personal-app";
pub(crate) const TOOL: &str = "permission_prompt";
/// The route of the permission tool (outside `/api/`: it carries the session's bearer secret, not the page token).
pub(crate) const MCP_PATH: &str = "/app-session-mcp";
/// `MCP_TOOL_TIMEOUT` of the child, in ms (one hour: a prompt that waits longer is cut and retried, and a retry is
/// answered once).
const TOOL_TIMEOUT_MS: &str = "3600000";
/// Text kept per item and items kept per session (the full record stays in Claude Code's own transcript).
const MAX_TEXT: usize = 20_000;
const MAX_ITEMS: usize = 500;
const SUMMARY_CHARS: usize = 300;
const MAX_CHANGED: usize = 200;
const DENIED: &str = "The owner denied this in the app.";
const GONE: &str = "The session ended or the turn was interrupted before this was answered.";

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(crate) fn random_hex(bytes: usize) -> Option<String> {
    let mut raw = vec![0u8; bytes];
    getrandom::fill(&mut raw).ok()?;
    Some(raw.iter().map(|b| format!("{b:02x}")).collect())
}

pub(crate) fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

/// FR-092's control and hidden characters (C0, DEL, C1, line and paragraph separators, zero-width and bidi marks).
fn hidden(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{2028}' | '\u{2029}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}'
                       | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
}

/// Text for the page: hidden characters removed, but a line break and a tab kept.
fn shown(text: &str) -> String {
    text.chars()
        .filter(|&c| c == '\n' || c == '\t' || !hidden(c))
        .collect()
}

/// One line for the page: every run of white space and hidden characters is one space, cut to `max` characters.
fn one_line(text: &str, max: usize) -> String {
    let flat: String = text
        .chars()
        .map(|c| {
            if c.is_whitespace() || hidden(c) {
                ' '
            } else {
                c
            }
        })
        .collect();
    flat.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max)
        .collect()
}

fn cut(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

/// A tool name that is one safe word (a fixed tool name, never input).
fn safe_word(text: &str) -> Option<String> {
    let ok = !text.is_empty()
        && text.len() <= 64
        && text.starts_with(|c: char| c.is_ascii_alphanumeric())
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '.' | '_' | '-'));
    ok.then(|| text.to_string())
}

// ---- the plan: what a start asked for, already checked against the fixed lists and the store -------------------------

pub(crate) struct Account {
    pub id: String,
    pub label: String,
    /// `CLAUDE_CONFIG_DIR` of the account; `None` for the default account (the variable is removed).
    pub config_dir: Option<String>,
}

pub(crate) struct Plan {
    /// A repo path the store holds (an existing folder).
    pub repo: PathBuf,
    pub account: Option<Account>,
    pub model: &'static str,
    pub mode: &'static str,
    pub worktree: bool,
    pub prompt: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StartError {
    /// 409: [`MAX_RUNNING`] sessions are going.
    Busy,
    /// 400 with a fixed text.
    Refused(&'static str),
    /// 500.
    Internal,
}

/// The whole argument list after `claude`: fixed words, a model and a mode from the fixed lists, the permission tool's
/// name and the temp config's path. Nothing the owner typed.
pub(crate) fn argv(plan: &Plan, config: &Path) -> Vec<String> {
    let mut args: Vec<String> = [
        "-p",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
    ]
    .map(str::to_string)
    .to_vec();
    if plan.model != "default" {
        args.extend(["--model".into(), plan.model.into()]);
    }
    match plan.mode {
        "default" => {}
        "bypassPermissions" => args.push("--dangerously-skip-permissions".into()),
        mode => args.extend(["--permission-mode".into(), mode.into()]),
    }
    args.extend([
        "--permission-prompt-tool".into(),
        format!("mcp__{SERVER}__{TOOL}"),
        "--mcp-config".into(),
        config.to_string_lossy().into_owned(),
    ]);
    args
}

/// The child's environment (see the module doc).
pub(crate) fn child_env(
    env: &[(OsString, OsString)],
    config_dir: Option<&str>,
) -> Vec<(OsString, OsString)> {
    let dropped = |name: &OsStr| {
        let bytes = name.as_encoded_bytes();
        name == "CLAUDE_CONFIG_DIR"
            || name == "CLAUDECODE"
            || name == "MCP_TOOL_TIMEOUT"
            || bytes.starts_with(b"ANTHROPIC_")
            || bytes.starts_with(b"CLAUDE_CODE_")
            || DROPPED.iter().any(|d| name == *d)
    };
    let mut out: Vec<(OsString, OsString)> =
        env.iter().filter(|(k, _)| !dropped(k)).cloned().collect();
    if let Some(dir) = config_dir {
        out.push(("CLAUDE_CONFIG_DIR".into(), dir.into()));
    }
    out.push(("MCP_TOOL_TIMEOUT".into(), TOOL_TIMEOUT_MS.into()));
    out
}

// ---- worktree ---------------------------------------------------------------------------------------------------------

fn git(repo: &Path, args: &[&str]) -> Option<bool> {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .map(|s| s.success())
}

/// `git worktree add` under `<root>/worktrees/` on the new branch `easy/<stamp>-<hex>`; the folder and the branch names
/// are made from the stamp, 4 random hex characters and the repo folder's name with every unusual character replaced.
pub(crate) fn make_worktree(
    repo: &Path,
    root: &Path,
    stamp: &str,
    hex: &str,
) -> Result<(PathBuf, String), &'static str> {
    if git(repo, &["rev-parse", "--is-inside-work-tree"]) != Some(true) {
        return Err("the folder is not a git repository");
    }
    let branch = format!("easy/{stamp}-{hex}");
    if git(
        repo,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ],
    ) == Some(true)
    {
        return Err("that branch already exists; try again");
    }
    let name: String = repo
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let base = root.join("worktrees");
    let folder = base.join(format!("{}-{stamp}-{hex}", name.trim_start_matches('.')));
    if fs::symlink_metadata(&folder).is_ok() {
        return Err("the worktree folder already exists; try again");
    }
    match DirBuilder::new().recursive(true).mode(0o700).create(&base) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return Err("the worktree could not be made"),
    }
    let made = git(
        repo,
        &["worktree", "add", "-b", &branch, &folder.to_string_lossy()],
    );
    if made == Some(true) {
        Ok((folder, branch))
    } else {
        Err("the worktree could not be made")
    }
}

/// `git status --porcelain -z` of a worktree: the changed file names only (at most [`MAX_CHANGED`]).
pub(crate) fn changed_files(folder: &Path) -> Vec<String> {
    let Ok(out) = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(["status", "--porcelain", "-z"])
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    else {
        return Vec::new();
    };
    let mut names = Vec::new();
    let mut records = out.stdout.split(|&b| b == 0).filter(|r| !r.is_empty());
    while let Some(record) = records.next() {
        if record.len() < 4 {
            continue;
        }
        if matches!(record[0], b'R' | b'C') || matches!(record[1], b'R' | b'C') {
            records.next(); // the original name of a rename or copy
        }
        names.push(one_line(&String::from_utf8_lossy(&record[3..]), 300));
        if names.len() >= MAX_CHANGED {
            break;
        }
    }
    names
}

// ---- one session ------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Working,
    Idle,
    Ended,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Pending,
    Answered,
    Cancelled,
}

impl Status {
    fn word(self) -> &'static str {
        match self {
            Status::Pending => "pending",
            Status::Answered => "answered",
            Status::Cancelled => "cancelled",
        }
    }
}

struct Prompt {
    id: String,
    tool_use_id: String,
    tool: String,
    /// The tool's input as the child sent it: an allow gives it back (with the owner's `answers` for a question).
    input: Value,
    /// What the page shows (cleaned and cut), without the raw input.
    card: Value,
    status: Status,
    /// The result text of the tool call, once there is one.
    tx: watch::Sender<Option<String>>,
}

enum Item {
    User(String),
    Assistant(String),
    Tool {
        use_id: String,
        name: Option<String>,
        label: Option<Value>,
        outcome: &'static str,
    },
    Prompt(String),
}

struct State {
    phase: Phase,
    /// A fixed word, set with the end.
    reason: Option<&'static str>,
    claude_id: Option<String>,
    items: VecDeque<Item>,
    prompts: Vec<Prompt>,
    /// User messages sent and not yet answered by a `result`.
    outstanding: usize,
    /// Assistant text items since the last message (a `result` of a local command has the text only in itself).
    turn_texts: usize,
    seen_init: bool,
    stopping: bool,
    folder: String,
    branch: Option<String>,
    cwd: PathBuf,
    requests: u64,
}

pub(crate) struct Session {
    pub(crate) id: String,
    secret: String,
    account: Option<(String, String)>,
    model: &'static str,
    mode: &'static str,
    worktree: bool,
    state: Mutex<State>,
    /// Apart from `state`: a write that blocks must not block a read of the state.
    stdin: Mutex<Option<ChildStdin>>,
    /// The child's pid, which is its process group's id (0: none yet).
    pgid: AtomicI32,
    /// The stop asked for: when, and the reason word of the end.
    stop: Mutex<Option<(Instant, &'static str)>>,
    config: Mutex<Option<PathBuf>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Session {
    fn new(
        id: String,
        secret: String,
        account: Option<&Account>,
        plan: &Plan,
        folder: String,
    ) -> Session {
        Session {
            id,
            secret,
            account: account.map(|a| (a.id.clone(), a.label.clone())),
            model: plan.model,
            mode: plan.mode,
            worktree: plan.worktree,
            state: Mutex::new(State {
                phase: Phase::Working,
                reason: None,
                claude_id: None,
                items: VecDeque::new(),
                prompts: Vec::new(),
                outstanding: 0,
                turn_texts: 0,
                seen_init: false,
                stopping: false,
                folder,
                branch: None,
                cwd: plan.repo.clone(),
                requests: 0,
            }),
            stdin: Mutex::new(None),
            pgid: AtomicI32::new(0),
            stop: Mutex::new(None),
            config: Mutex::new(None),
            thread: Mutex::new(None),
        }
    }

    pub(crate) fn secret_ok(&self, given: &str) -> bool {
        constant_time_eq(given.as_bytes(), self.secret.as_bytes())
    }

    pub(crate) fn ended(&self) -> bool {
        lock(&self.state).phase == Phase::Ended
    }

    fn state_word(state: &State) -> &'static str {
        match state.phase {
            Phase::Ended => "ended",
            _ if state.stopping => "stopping",
            _ if state.prompts.iter().any(|p| p.status == Status::Pending) => "waiting",
            Phase::Working => "working",
            Phase::Idle => "idle",
        }
    }

    /// The answer of `GET`: no tool input or output; the cards carry what FR-115 allows. `changed` is the changed file
    /// names of a worktree (read by the caller, outside the lock).
    pub(crate) fn json(&self, changed: &[String]) -> Value {
        let state = lock(&self.state);
        let items: Vec<Value> = state
            .items
            .iter()
            .map(|item| match item {
                Item::User(text) => json!({"kind": "user", "text": shown(text)}),
                Item::Assistant(text) => json!({"kind": "assistant", "text": shown(text)}),
                Item::Tool {
                    name,
                    label,
                    outcome,
                    ..
                } => {
                    json!({"kind": "tool", "name": name, "label": label, "outcome": outcome})
                }
                Item::Prompt(id) => {
                    let mut card = state.prompts.iter().find(|p| &p.id == id).map_or_else(
                        || json!({}),
                        |p| {
                            let mut card = p.card.clone();
                            card["status"] = json!(p.status.word());
                            card["prompt_id"] = json!(p.id);
                            card
                        },
                    );
                    card["kind"] = json!("prompt");
                    card
                }
            })
            .collect();
        json!({
            "id": self.id,
            "state": Self::state_word(&state),
            "reason": state.reason,
            "session_id": state.claude_id,
            "account": self.account.as_ref().map(|(id, label)| json!({"id": id, "label": label})),
            "model": self.model,
            "mode": self.mode,
            "folder": state.folder,
            "worktree": self.worktree,
            "branch": state.branch,
            "changed_files": changed,
            "items": items,
        })
    }

    /// The worktree folder whose changed files `GET` lists; `None` for a session in a stored repo itself.
    pub(crate) fn worktree_folder(&self) -> Option<PathBuf> {
        self.worktree.then(|| lock(&self.state).cwd.clone())
    }

    fn push(state: &mut State, item: Item) {
        state.items.push_back(item);
        while state.items.len() > MAX_ITEMS {
            state.items.pop_front();
        }
    }

    /// One stdin line. `Err` when the child's stdin is gone (the session has ended).
    fn send_line(&self, line: &Value) -> Result<(), ()> {
        let mut text = line.to_string();
        text.push('\n');
        let mut stdin = lock(&self.stdin);
        let Some(pipe) = stdin.as_mut() else {
            return Err(());
        };
        if pipe
            .write_all(text.as_bytes())
            .and_then(|()| pipe.flush())
            .is_err()
        {
            *stdin = None;
            return Err(());
        }
        Ok(())
    }

    /// One more user message (the first prompt too). The text is sent as typed.
    pub(crate) fn message(&self, text: &str) -> Result<(), ()> {
        {
            let mut state = lock(&self.state);
            if state.phase == Phase::Ended || state.stopping {
                return Err(());
            }
            Self::push(&mut state, Item::User(cut(text, MAX_TEXT)));
            state.outstanding += 1;
            state.turn_texts = 0;
            state.phase = Phase::Working;
        }
        self.send_line(&json!({"type": "user", "message": {"role": "user", "content": text}}))
    }

    /// The stream-json interrupt: the turn aborts and the process stays alive. Never SIGINT.
    pub(crate) fn interrupt(&self) -> Result<(), ()> {
        let request = {
            let mut state = lock(&self.state);
            if state.phase == Phase::Ended {
                return Err(());
            }
            state.requests += 1;
            state.requests
        };
        self.send_line(&json!({
            "type": "control_request",
            "request_id": format!("req_{request}"),
            "request": {"subtype": "interrupt"},
        }))
    }

    /// Asks the supervisor to end the process group (SIGTERM, then SIGKILL after the grace). The first reason wins.
    pub(crate) fn request_stop(&self, reason: &'static str) {
        let mut stop = lock(&self.stop);
        if stop.is_none() {
            *stop = Some((Instant::now(), reason));
            lock(&self.state).stopping = true;
        }
    }

    // ---- stdout ----

    fn handle_line(&self, line: &[u8]) {
        let Ok(event) = serde_json::from_slice::<Value>(line) else {
            return;
        };
        let mut state = lock(&self.state);
        if state.phase == Phase::Ended {
            return;
        }
        match event["type"].as_str() {
            Some("system") if event["subtype"] == "init" => {
                state.seen_init = true;
                if state.claude_id.is_none() {
                    state.claude_id = event["session_id"].as_str().map(|s| cut(s, 100));
                }
            }
            Some("assistant") => {
                for block in event["message"]["content"].as_array().into_iter().flatten() {
                    match block["type"].as_str() {
                        Some("text") => {
                            let text = block["text"].as_str().unwrap_or("");
                            if !text.trim().is_empty() {
                                Self::push(&mut state, Item::Assistant(cut(text, MAX_TEXT)));
                                state.turn_texts += 1;
                            }
                        }
                        Some("tool_use") => {
                            let name = block["name"].as_str().unwrap_or("");
                            let label = tool_label(name, &block["input"]);
                            Self::push(
                                &mut state,
                                Item::Tool {
                                    use_id: block["id"].as_str().unwrap_or("").to_string(),
                                    name: safe_word(name),
                                    label,
                                    outcome: "running",
                                },
                            );
                        }
                        _ => {}
                    }
                }
            }
            Some("user") => {
                for block in event["message"]["content"].as_array().into_iter().flatten() {
                    if block["type"] != "tool_result" {
                        continue;
                    }
                    let use_id = block["tool_use_id"].as_str().unwrap_or("");
                    let outcome = if block["is_error"] == true {
                        "error"
                    } else {
                        "ok"
                    };
                    for item in &mut state.items {
                        if let Item::Tool {
                            use_id: id,
                            outcome: o,
                            ..
                        } = item
                            && id == use_id
                        {
                            *o = outcome;
                        }
                    }
                }
            }
            Some("result") => {
                if state.turn_texts == 0
                    && let Some(text) = event["result"].as_str()
                    && !text.trim().is_empty()
                {
                    // a local command (`/context`, `/help`): its output is only in the result
                    Self::push(&mut state, Item::Assistant(cut(text, MAX_TEXT)));
                }
                state.turn_texts = 0;
                state.outstanding = state.outstanding.saturating_sub(1);
                if state.outstanding == 0 {
                    state.phase = Phase::Idle;
                }
                settle(&mut state);
            }
            _ => {}
        }
    }

    // ---- prompts (the MCP route) ----

    /// The prompt for one `tools/call` of the permission tool: a new pending card, or, for a `tool_use_id` seen before
    /// (Claude Code retries a call it cut), the same prompt. The receiver yields the result text once there is one.
    pub(crate) fn open_prompt(
        &self,
        args: &Value,
        meta_id: Option<&str>,
    ) -> Option<watch::Receiver<Option<String>>> {
        let tool = args["tool_name"].as_str()?;
        let use_id = args["tool_use_id"]
            .as_str()
            .or(meta_id)
            .map_or_else(|| random_hex(8).unwrap_or_default(), |s| cut(s, 200));
        let mut state = lock(&self.state);
        if state.phase == Phase::Ended {
            return None;
        }
        if let Some(prompt) = state.prompts.iter().find(|p| p.tool_use_id == use_id) {
            return Some(prompt.tx.subscribe());
        }
        let input = if args["input"].is_object() {
            args["input"].clone()
        } else {
            json!({})
        };
        let id = random_hex(4)?;
        let (tx, rx) = watch::channel(None);
        let card = card(tool, &input);
        state.prompts.push(Prompt {
            id: id.clone(),
            tool_use_id: use_id,
            tool: cut(tool, 100),
            input,
            card,
            status: Status::Pending,
            tx,
        });
        Self::push(&mut state, Item::Prompt(id));
        Some(rx)
    }

    /// The owner's answer to a pending prompt: `Ok` when it was the first.
    pub(crate) fn answer(&self, prompt_id: &str, answer: &Answer) -> Result<(), AnswerError> {
        let mut state = lock(&self.state);
        let Some(prompt) = state.prompts.iter_mut().find(|p| p.id == prompt_id) else {
            return Err(AnswerError::Unknown);
        };
        if prompt.status != Status::Pending {
            return Err(AnswerError::Settled);
        }
        let result = result_text(prompt, answer)?;
        prompt.status = Status::Answered;
        prompt.tx.send_replace(Some(result));
        Ok(())
    }
}

/// What a prompt's waiting call is given when it is cancelled: a deny.
fn deny_text(message: &str) -> String {
    json!({"behavior": "deny", "message": message}).to_string()
}

/// Every pending prompt is cancelled (the turn ended or the session did): its call gets a deny, its card closes.
fn settle(state: &mut State) {
    for prompt in &mut state.prompts {
        if prompt.status == Status::Pending {
            prompt.status = Status::Cancelled;
            prompt.tx.send_replace(Some(deny_text(GONE)));
        }
    }
}

fn tool_label(name: &str, input: &Value) -> Option<Value> {
    let parsed = pyjson::loads(&serde_json::to_vec(input).ok()?)?;
    call_label(name, &parsed).map(|label| label.to_json())
}

/// What the page shows of a permission prompt (FR-115): the tool name and a one-line summary, or the question, or the plan.
fn card(tool: &str, input: &Value) -> Value {
    match tool {
        "AskUserQuestion" => {
            let questions: Vec<Value> = input["questions"]
                .as_array()
                .into_iter()
                .flatten()
                .take(10)
                .map(|q| {
                    let options: Vec<Value> = q["options"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .take(20)
                        .map(|o| {
                            json!({
                                "label": cut(&shown(o["label"].as_str().unwrap_or("")), 200),
                                "description": cut(&shown(o["description"].as_str().unwrap_or("")), 500),
                            })
                        })
                        .collect();
                    json!({
                        "question": cut(&shown(q["question"].as_str().unwrap_or("")), 1000),
                        "header": cut(&shown(q["header"].as_str().unwrap_or("")), 100),
                        "multiSelect": q["multiSelect"] == true,
                        "options": options,
                    })
                })
                .collect();
            json!({"type": "question", "tool": tool, "questions": questions})
        }
        "ExitPlanMode" => json!({
            "type": "plan", "tool": tool,
            "plan": cut(&shown(input["plan"].as_str().unwrap_or("")), MAX_TEXT),
        }),
        _ => {
            let field = match tool {
                "Bash" => "command",
                "Edit" | "Write" | "MultiEdit" => "file_path",
                "NotebookEdit" => "notebook_path",
                _ => "",
            };
            let summary = input[field].as_str().map(|s| one_line(s, SUMMARY_CHARS));
            json!({"type": "tool", "tool": safe_word(tool).unwrap_or_default(), "summary": summary})
        }
    }
}

/// The owner's answer, as the route parsed it.
pub(crate) struct Answer {
    pub allow: bool,
    /// For a question: the answer per question text.
    pub answers: Option<Map<String, Value>>,
    pub message: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AnswerError {
    Unknown,
    /// An answer was given already, or the prompt was cancelled.
    Settled,
    Bad(&'static str),
}

/// The tool's documented result text. An allow gives the child's own input back; a question's `answers` (keyed by the
/// question text) are added to it. Nothing else the page sends reaches the input.
fn result_text(prompt: &Prompt, answer: &Answer) -> Result<String, AnswerError> {
    if !answer.allow {
        let note = answer.message.as_deref().filter(|m| !m.trim().is_empty());
        return Ok(deny_text(note.unwrap_or(DENIED)));
    }
    let mut input = prompt.input.clone();
    if prompt.tool == "AskUserQuestion" {
        let given = answer
            .answers
            .as_ref()
            .filter(|a| !a.is_empty())
            .ok_or(AnswerError::Bad("answers must name the questions answered"))?;
        let mut keyed = Map::new();
        for (key, value) in given {
            let text = value
                .as_str()
                .ok_or(AnswerError::Bad("each answer must be text"))?;
            // the page shows the cleaned question; the child's own text is the key it expects
            let raw = input["questions"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|q| q["question"].as_str())
                .find(|q| shown(q) == *key || *q == key.as_str())
                .ok_or(AnswerError::Bad("answers must be keyed by a question text"))?;
            keyed.insert(raw.to_string(), json!(text));
        }
        input["answers"] = Value::Object(keyed);
    }
    Ok(json!({"behavior": "allow", "updatedInput": input}).to_string())
}

// ---- the registry -----------------------------------------------------------------------------------------------------

/// The sessions of one server, and how to start one.
pub(crate) struct Sessions {
    env: Vec<(OsString, OsString)>,
    list: Mutex<Vec<Arc<Session>>>,
    grace: Duration,
    poll: Duration,
}

impl Sessions {
    pub(crate) fn new(env: Vec<(OsString, OsString)>) -> Sessions {
        Sessions::timed(env, STOP_GRACE, POLL)
    }

    pub(crate) fn timed(
        env: Vec<(OsString, OsString)>,
        grace: Duration,
        poll: Duration,
    ) -> Sessions {
        Sessions {
            env,
            list: Mutex::new(Vec::new()),
            grace,
            poll,
        }
    }

    pub(crate) fn all(&self) -> Vec<Arc<Session>> {
        lock(&self.list).clone()
    }

    pub(crate) fn get(&self, id: &str) -> Option<Arc<Session>> {
        lock(&self.list).iter().find(|s| s.id == id).cloned()
    }

    /// Takes the session out of the list and drops its text, once it has ended (its panel is closed): `Err(true)` when
    /// it has not ended yet, `Err(false)` when there is none.
    pub(crate) fn close(&self, id: &str) -> Result<(), bool> {
        let mut list = lock(&self.list);
        let at = list.iter().position(|s| s.id == id).ok_or(false)?;
        if !list[at].ended() {
            return Err(true);
        }
        let session = list.remove(at);
        drop_text(&session);
        Ok(())
    }

    /// Starts a session: the slot, the worktree (when asked), the temp config, the child. `Ok(id)`; a `claude` that is
    /// not found or does not start is an `Ok` session that has ended with a fixed reason.
    pub(crate) fn start(
        &self,
        plan: Plan,
        dir: &Path,
        port: u16,
        allowed: Box<dyn Fn() -> bool + Send>,
    ) -> Result<String, StartError> {
        let (Some(id), Some(secret)) = (random_hex(8), random_hex(24)) else {
            return Err(StartError::Internal);
        };
        let claude = which(
            self.env
                .iter()
                .find(|(k, _)| k == "PATH")
                .map(|(_, v)| v.as_os_str()),
            "claude",
        );
        let folder = plan
            .repo
            .file_name()
            .map(|n| one_line(&n.to_string_lossy(), 100))
            .unwrap_or_default();
        let session = Arc::new(Session::new(
            id.clone(),
            secret,
            plan.account.as_ref(),
            &plan,
            folder,
        ));
        {
            let mut list = lock(&self.list);
            if list.iter().filter(|s| !s.ended()).count() >= MAX_RUNNING {
                return Err(StartError::Busy);
            }
            while list.iter().filter(|s| s.ended()).count() >= MAX_KEPT {
                if let Some(at) = list.iter().position(|s| s.ended()) {
                    drop_text(&list.remove(at));
                }
            }
            list.push(Arc::clone(&session));
        }
        let Some(claude) = claude else {
            end_now(&session, "claude not found");
            return Ok(id);
        };
        match self.launch(&session, &claude, &plan, dir, port, allowed) {
            Ok(()) => Ok(id),
            Err(e) => {
                lock(&self.list).retain(|s| s.id != id);
                drop_text(&session);
                Err(e)
            }
        }
    }

    fn launch(
        &self,
        session: &Arc<Session>,
        claude: &Path,
        plan: &Plan,
        dir: &Path,
        port: u16,
        allowed: Box<dyn Fn() -> bool + Send>,
    ) -> Result<(), StartError> {
        let mut cwd = plan.repo.clone();
        if plan.worktree {
            let stamp = jiff::Timestamp::now().strftime("%Y%m%d-%H%M").to_string();
            let hex = random_hex(2).ok_or(StartError::Internal)?;
            let (folder, branch) =
                make_worktree(&plan.repo, dir, &stamp, &hex).map_err(StartError::Refused)?;
            let mut state = lock(&session.state);
            state.folder = folder
                .file_name()
                .map(|n| one_line(&n.to_string_lossy(), 100))
                .unwrap_or_default();
            state.branch = Some(branch);
            state.cwd = folder.clone();
            cwd = folder;
        }
        let config = write_config(dir, &session.id, &session.secret, port)
            .map_err(|_| StartError::Internal)?;
        *lock(&session.config) = Some(config.clone());
        let mut cmd = Command::new(claude);
        cmd.args(argv(plan, &config))
            .current_dir(&cwd)
            .env_clear()
            .envs(child_env(
                &self.env,
                plan.account.as_ref().and_then(|a| a.config_dir.as_deref()),
            ))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .process_group(0);
        let parent = std::process::id() as libc::pid_t;
        // SAFETY: only async-signal-safe calls (`prctl`, `getppid`) between fork and exec, and no allocation
        unsafe {
            cmd.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL as libc::c_ulong) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::getppid() != parent {
                    return Err(std::io::Error::from_raw_os_error(libc::ESRCH));
                }
                Ok(())
            })
        };
        let (ready, started) = mpsc::channel::<bool>();
        let (grace, poll) = (self.grace, self.poll);
        let own = Arc::clone(session);
        // the thread that starts the child lives as long as it (`PR_SET_PDEATHSIG` follows the thread)
        let handle = std::thread::Builder::new()
            .name("app-session".into())
            .spawn(move || {
                let mut child = match cmd.spawn() {
                    Ok(child) => child,
                    Err(_) => {
                        let _ = ready.send(false);
                        return;
                    }
                };
                own.pgid.store(child.id() as i32, Ordering::SeqCst);
                *lock(&own.stdin) = child.stdin.take();
                let stdout = child.stdout.take();
                let _ = ready.send(true);
                supervise(&own, child, stdout, grace, poll, allowed);
            })
            .map_err(|_| StartError::Internal)?;
        *lock(&session.thread) = Some(handle);
        if started.recv() != Ok(true) {
            end_now(session, "failed to start");
            return Ok(());
        }
        // the first prompt, on stdin (never in the argument list); a child that died at once is `ended` by its supervisor
        let _ = session.message(&plan.prompt);
        Ok(())
    }

    /// At exit: every child is stopped (SIGTERM, then SIGKILL after the grace), its thread joined, all text dropped.
    pub(crate) fn shutdown(&self) {
        let all = std::mem::take(&mut *lock(&self.list));
        for session in &all {
            session.request_stop("app closed");
        }
        for session in &all {
            let handle = lock(&session.thread).take();
            if let Some(handle) = handle {
                let _ = handle.join();
            }
            drop_text(session);
        }
    }
}

/// An ended record with no process (nothing could be started).
fn end_now(session: &Session, reason: &'static str) {
    {
        let mut state = lock(&session.state);
        state.phase = Phase::Ended;
        state.reason = Some(reason);
        settle(&mut state);
    }
    if let Some(path) = lock(&session.config).take() {
        let _ = fs::remove_file(path);
    }
}

/// The text of a session leaves memory (items and prompts); what is left is a fixed record.
fn drop_text(session: &Session) {
    let mut state = lock(&session.state);
    settle(&mut state);
    state.items.clear();
    state.prompts.clear();
    state.claude_id = None;
}

/// `<data>/app-sessions/<id>.mcp.json`, mode 0600 in a 0700 folder; removed when the session ends.
fn write_config(dir: &Path, id: &str, secret: &str, port: u16) -> std::io::Result<PathBuf> {
    let folder = dir.join("app-sessions");
    match DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&folder)
    {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    let path = folder.join(format!("{id}.mcp.json"));
    let body = json!({"mcpServers": {SERVER: {
        "type": "http",
        "url": format!("http://127.0.0.1:{port}{MCP_PATH}/{id}"),
        "headers": {"Authorization": format!("Bearer {secret}")},
    }}});
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)?;
    file.write_all(body.to_string().as_bytes())?;
    Ok(path)
}

fn signal_group(pgid: i32, signal: libc::c_int) {
    if pgid > 0 {
        // SAFETY: a signal to the process group this program started
        unsafe { libc::kill(-pgid, signal) };
    }
}

/// Runs on the thread that started the child, until the child is gone: reads stdout on a thread of its own, carries out
/// a stop (SIGTERM, then SIGKILL), stops the session when the setting is turned off, and records the end.
fn supervise(
    session: &Arc<Session>,
    mut child: std::process::Child,
    stdout: Option<std::process::ChildStdout>,
    grace: Duration,
    poll: Duration,
    allowed: Box<dyn Fn() -> bool + Send>,
) {
    let reader = stdout.map(|out| {
        let own = Arc::clone(session);
        std::thread::spawn(move || {
            let mut reader = BufReader::new(out);
            let mut line = Vec::new();
            loop {
                line.clear();
                match reader.read_until(b'\n', &mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => own.handle_line(&line),
                }
            }
        })
    });
    let pgid = child.id() as i32;
    let (mut termed, mut last_check) = (false, Instant::now());
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(_) => break None,
        }
        if last_check.elapsed() >= SETTING_EVERY {
            last_check = Instant::now();
            if !allowed() {
                session.request_stop("account turned off");
            }
        }
        let stop = *lock(&session.stop);
        if let Some((at, _)) = stop {
            if !termed {
                signal_group(pgid, libc::SIGTERM);
                termed = true;
            } else if at.elapsed() >= grace {
                signal_group(pgid, libc::SIGKILL);
            }
        }
        std::thread::sleep(poll);
    };
    signal_group(pgid, libc::SIGKILL); // what the child left running
    let _ = child.wait();
    if let Some(reader) = reader {
        let deadline = Instant::now() + Duration::from_millis(500);
        while !reader.is_finished() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    let stop = *lock(&session.stop);
    *lock(&session.stdin) = None;
    {
        let mut state = lock(&session.state);
        state.reason = Some(match (stop, status) {
            (Some((_, reason)), _) => reason,
            (None, Some(s)) if s.success() => "exited",
            (None, _) if !state.seen_init => "failed to start",
            (None, _) => "failed",
        });
        state.phase = Phase::Ended;
        state.outstanding = 0;
        for item in &mut state.items {
            if let Item::Tool { outcome, .. } = item
                && *outcome == "running"
            {
                *outcome = "error";
            }
        }
        settle(&mut state);
    }
    if let Some(path) = lock(&session.config).take() {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::agent_run::tests::{Fake, runs, serial};
    use std::os::unix::fs::PermissionsExt;

    /// A fake `claude` that answers each stdin line like the stream-json child does, and logs what it was sent.
    pub(crate) const CHAT: &str = r#"echo '{"type":"system","subtype":"init","session_id":"sess-1"}'
while IFS= read -r line; do
  printf '%s\n' "$line" >> "$L/stdin"
  case "$line" in
    *control_request*) echo '{"type":"result","subtype":"error_during_execution","is_error":true,"num_turns":1,"terminal_reason":"aborted_streaming","session_id":"sess-1"}' ;;
    *) echo '{"type":"system","subtype":"init","session_id":"sess-1"}'
       echo '{"type":"assistant","message":{"content":[{"type":"text","text":"ANSWER-TEXT"},{"type":"tool_use","id":"tu1","name":"Bash","input":{"command":"ls -la TOOL-SECRET"}}]},"session_id":"sess-1"}'
       echo '{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tu1","is_error":false,"content":"TOOL-OUTPUT"}]}}'
       echo '{"type":"result","subtype":"success","is_error":false,"num_turns":1,"result":"ANSWER-TEXT","session_id":"sess-1"}' ;;
  esac
done"#;

    pub(crate) fn plan(repo: &Path, mode: &'static str, worktree: bool) -> Plan {
        Plan {
            repo: repo.to_path_buf(),
            account: None,
            model: "default",
            mode,
            worktree,
            prompt: "FIRST-PROMPT".into(),
        }
    }

    fn until(what: &str, mut ok: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(20);
        while !ok() {
            assert!(Instant::now() < deadline, "never happened: {what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn sessions(fake: &Fake, grace_ms: u64) -> Sessions {
        Sessions::timed(
            fake.env(&[]),
            Duration::from_millis(grace_ms),
            Duration::from_millis(10),
        )
    }

    fn yes() -> Box<dyn Fn() -> bool + Send> {
        Box::new(|| true)
    }

    fn start(s: &Sessions, fake: &Fake, mode: &'static str) -> Arc<Session> {
        std::fs::create_dir_all(fake.root.join("repo")).unwrap();
        let id = s
            .start(
                plan(&fake.root.join("repo"), mode, false),
                &fake.dir(),
                4242,
                yes(),
            )
            .unwrap();
        s.get(&id).unwrap()
    }

    fn text_of(session: &Session) -> String {
        session.json(&[]).to_string()
    }

    // ---- AC 1, 2: the argument list and the environment ----

    #[test]
    fn the_argv_holds_fixed_words_a_listed_model_and_mode_and_no_text_of_the_owner() {
        let repo = Path::new("/r");
        let config = Path::new("/data/app-sessions/x.mcp.json");
        let base = [
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
        ];
        let tail = [
            "--permission-prompt-tool",
            "mcp__easy-personal-app__permission_prompt",
            "--mcp-config",
            "/data/app-sessions/x.mcp.json",
        ];
        let got = argv(&plan(repo, "default", false), config);
        assert_eq!(got, [&base[..], &tail[..]].concat());
        let mut p = plan(repo, "plan", false);
        p.model = "opus";
        let got = argv(&p, config);
        assert_eq!(
            got,
            [
                &base[..],
                &["--model", "opus", "--permission-mode", "plan"][..],
                &tail[..]
            ]
            .concat()
        );
        for mode in ["acceptEdits", "auto", "dontAsk"] {
            let got = argv(&plan(repo, mode, false), config);
            assert!(got.windows(2).any(|w| w == ["--permission-mode", mode]));
        }
        for mode in MODES.iter().filter(|m| **m != "bypassPermissions") {
            let got = argv(&plan(repo, mode, false), config);
            assert!(!got.iter().any(|a| a.contains("dangerously")), "{mode}");
        }
        let got = argv(&plan(repo, "bypassPermissions", false), config);
        assert!(got.contains(&"--dangerously-skip-permissions".to_string()));
        assert!(!got.contains(&"--permission-mode".to_string()));
        for mode in MODES {
            let got = argv(&plan(repo, mode, false), config);
            for banned in [
                "--allowedTools",
                "--disallowedTools",
                "--tools",
                "--strict-mcp-config",
                "--bare",
                "FIRST-PROMPT",
            ] {
                assert!(!got.iter().any(|a| a == banned), "{mode} {banned}");
            }
            assert!(got.iter().any(|a| a == "--permission-prompt-tool"));
        }
    }

    #[test]
    fn the_child_environment_drops_billing_and_parent_markers_and_keeps_the_accounts_folder() {
        let env: Vec<(OsString, OsString)> = [
            ("PATH", "/bin"),
            ("HOME", "/h"),
            ("CLAUDE_CONFIG_DIR", "/parent"),
            ("ANTHROPIC_API_KEY", "k"),
            ("ANTHROPIC_BASE_URL", "u"),
            ("AWS_BEARER_TOKEN_BEDROCK", "t"),
            ("CLAUDE_CODE_OAUTH_TOKEN", "t"),
            ("CLAUDE_CODE_USE_VERTEX", "1"),
            ("CLAUDE_CODE_ENTRYPOINT", "cli"),
            ("CLAUDECODE", "1"),
            ("MCP_TOOL_TIMEOUT", "1"),
            ("LANG", "C"),
        ]
        .map(|(k, v)| (OsString::from(k), OsString::from(v)))
        .to_vec();
        let got: Vec<(String, String)> = child_env(&env, Some("/acc/.claude-work"))
            .into_iter()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().into_owned(),
                    v.to_string_lossy().into_owned(),
                )
            })
            .collect();
        let names: Vec<&str> = got.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(
            names,
            [
                "PATH",
                "HOME",
                "LANG",
                "CLAUDE_CONFIG_DIR",
                "MCP_TOOL_TIMEOUT"
            ]
        );
        assert_eq!(got[3].1, "/acc/.claude-work");
        assert_eq!(got[4].1, "3600000");
        let default_account = child_env(&env, None);
        assert!(
            !default_account
                .iter()
                .any(|(k, _)| k == "CLAUDE_CONFIG_DIR")
        );
    }

    #[test]
    fn a_session_runs_the_fake_with_the_argv_the_temp_config_and_no_secret_in_its_environment() {
        let _serial = serial();
        let fake = Fake::new("t103-run", CHAT);
        let s = sessions(&fake, 300);
        let session = start(&s, &fake, "acceptEdits");
        fake.wait_for("pid");
        let argv_logged = fake.argv();
        let config = argv_logged.last().unwrap().clone();
        assert!(
            argv_logged
                .windows(2)
                .any(|w| w == ["--permission-mode", "acceptEdits"])
        );
        assert!(config.ends_with(&format!("app-sessions/{}.mcp.json", session.id)));
        let mode = std::fs::metadata(&config).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let body: Value = serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
        assert_eq!(body["mcpServers"][SERVER]["type"], "http");
        assert_eq!(
            body["mcpServers"][SERVER]["url"],
            format!("http://127.0.0.1:4242/app-session-mcp/{}", session.id)
        );
        assert_eq!(
            body["mcpServers"][SERVER]["headers"]["Authorization"],
            format!("Bearer {}", session.secret)
        );
        assert_eq!(
            fake.log("cwd").trim(),
            std::fs::canonicalize(fake.root.join("repo"))
                .unwrap()
                .display()
                .to_string()
        );
        let env = fake.child_env();
        assert!(
            env.iter()
                .any(|(k, v)| k == "MCP_TOOL_TIMEOUT" && v == "3600000")
        );
        // the first prompt is a stdin line, not an argument
        until("the prompt line", || {
            fake.log("stdin").contains("FIRST-PROMPT")
        });
        assert_eq!(
            fake.log("stdin").lines().next().unwrap(),
            r#"{"message":{"content":"FIRST-PROMPT","role":"user"},"type":"user"}"#
        );
        s.shutdown();
        assert!(
            !Path::new(&config).exists(),
            "the temp config is removed with the session"
        );
    }

    // ---- AC 3: the worktree ----

    fn repo_with_commit(root: &Path) -> PathBuf {
        let repo = root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        for args in [
            vec!["init", "-q"],
            vec![
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                "i",
            ],
        ] {
            assert!(
                Command::new("git")
                    .arg("-C")
                    .arg(&repo)
                    .args(args)
                    .env("GIT_CONFIG_GLOBAL", "/dev/null")
                    .status()
                    .unwrap()
                    .success()
            );
        }
        repo
    }

    #[test]
    fn a_worktree_is_made_on_a_new_easy_branch_and_the_refusals_are_fixed_messages() {
        let root =
            std::env::temp_dir().join(format!("easy-personal-rs-t103-wt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let repo = repo_with_commit(&root);
        let data = root.join("data");
        let (folder, branch) = make_worktree(&repo, &data, "20261117-1200", "ab12").unwrap();
        assert_eq!(branch, "easy/20261117-1200-ab12");
        assert_eq!(folder, data.join("worktrees/repo-20261117-1200-ab12"));
        assert!(folder.join(".git").exists());
        assert_eq!(
            std::fs::metadata(data.join("worktrees"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            make_worktree(&repo, &data, "20261117-1200", "ab12"),
            Err("that branch already exists; try again")
        );
        // the branch is free, the folder is taken
        std::fs::create_dir_all(data.join("worktrees/repo-20261117-1200-cd34")).unwrap();
        assert_eq!(
            make_worktree(&repo, &data, "20261117-1200", "cd34"),
            Err("the worktree folder already exists; try again")
        );
        let plain = root.join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        assert_eq!(
            make_worktree(&plain, &data, "20261117-1200", "ef56"),
            Err("the folder is not a git repository")
        );
        assert!(!data.join("worktrees/plain-20261117-1200-ef56").exists());
        std::fs::write(folder.join("new file.txt"), "x").unwrap();
        std::fs::write(folder.join("a;b $(c).txt"), "x").unwrap();
        let mut names = changed_files(&folder);
        names.sort();
        assert_eq!(names, ["a;b $(c).txt", "new file.txt"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    // ---- stdout: what the panel shows ----

    #[test]
    fn the_stream_gives_text_tool_labels_and_outcomes_but_no_tool_input_or_output() {
        let _serial = serial();
        let fake = Fake::new("t103-stream", CHAT);
        let s = sessions(&fake, 300);
        let session = start(&s, &fake, "default");
        until("the first turn", || session.json(&[])["state"] == "idle");
        let view = session.json(&[]);
        assert_eq!(view["session_id"], "sess-1");
        let kinds: Vec<&str> = view["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["kind"].as_str().unwrap())
            .collect();
        assert_eq!(kinds, ["user", "assistant", "tool"]);
        assert_eq!(view["items"][1]["text"], "ANSWER-TEXT");
        assert_eq!(
            view["items"][2],
            json!({"kind":"tool","name":"Bash","label":"ls","outcome":"ok"})
        );
        let all = view.to_string();
        for hidden in ["TOOL-SECRET", "TOOL-OUTPUT", "ls -la"] {
            assert!(!all.contains(hidden), "{hidden}");
        }
        // `init` again for the second message is not a new session; the id stays
        session.message("second").unwrap();
        until("the second turn", || {
            session.json(&[])["items"].as_array().unwrap().len() == 6
        });
        until("idle", || session.json(&[])["state"] == "idle");
        assert_eq!(session.json(&[])["session_id"], "sess-1");
        s.shutdown();
    }

    #[test]
    fn a_result_of_a_local_command_with_no_assistant_text_is_shown_as_the_output() {
        let _serial = serial();
        let fake = Fake::new("t103-local", "sleep 30");
        let s = sessions(&fake, 300);
        let session = start(&s, &fake, "default");
        session.handle_line(br#"{"type":"result","subtype":"success","is_error":false,"num_turns":0,"result":"/help isn't available in this environment."}"#);
        let view = session.json(&[]);
        assert_eq!(
            view["items"][1],
            json!({"kind":"assistant","text":"/help isn't available in this environment."})
        );
        assert_eq!(view["state"], "idle");
        s.shutdown();
    }

    #[test]
    fn the_owners_text_is_sent_as_typed_and_shown_without_control_or_bidi_characters() {
        let _serial = serial();
        let fake = Fake::new("t103-typed", CHAT);
        let s = sessions(&fake, 300);
        let session = start(&s, &fake, "default");
        until("idle", || session.json(&[])["state"] == "idle");
        let typed = "/ping \u{202E}evil\u{200B} a\u{1b}[31mb\u{7f}";
        session.message(typed).unwrap();
        until("two lines", || fake.log("stdin").lines().count() >= 2);
        let sent: Value = serde_json::from_str(fake.log("stdin").lines().nth(1).unwrap()).unwrap();
        assert_eq!(sent["message"]["content"], typed);
        let shown_text = session.json(&[])["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["text"].as_str().is_some_and(|t| t.starts_with("/ping")))
            .unwrap()["text"]
            .clone();
        assert_eq!(shown_text, "/ping evil a[31mb");
        s.shutdown();
    }

    // ---- AC 4: interrupt, stop, concurrency ----

    #[test]
    fn interrupt_is_a_control_request_on_stdin_and_the_process_stays_alive() {
        let _serial = serial();
        let fake = Fake::new("t103-int", CHAT);
        let s = sessions(&fake, 300);
        let session = start(&s, &fake, "default");
        until("idle", || session.json(&[])["state"] == "idle");
        session.interrupt().unwrap();
        until("the line", || fake.log("stdin").contains("control_request"));
        let sent: Value = serde_json::from_str(fake.log("stdin").lines().last().unwrap()).unwrap();
        assert_eq!(
            sent,
            json!({"type":"control_request","request_id":"req_1","request":{"subtype":"interrupt"}})
        );
        std::thread::sleep(Duration::from_millis(200));
        assert!(runs(&fake.log("pid")), "interrupt never ends the process");
        assert!(!session.ended());
        session.message("after").unwrap(); // the next message is answered
        until("answered", || fake.log("stdin").lines().count() == 3);
        s.shutdown();
    }

    #[test]
    fn stop_is_sigterm_to_the_group_and_sigkill_after_the_grace_for_a_child_that_ignores_it() {
        let _serial = serial();
        let fake = Fake::new(
            "t103-stop",
            "trap '' TERM\nsleep 30 & echo $! > \"$L/grandchild\"\nwhile :; do sleep 0.1; done",
        );
        let s = sessions(&fake, 400);
        let session = start(&s, &fake, "default");
        fake.wait_for("grandchild");
        let began = Instant::now();
        session.request_stop("stopped");
        assert_eq!(session.json(&[])["state"], "stopping");
        until("ended", || session.ended());
        assert!(
            began.elapsed() >= Duration::from_millis(350),
            "the child ignored SIGTERM, so SIGKILL came after the grace"
        );
        assert_eq!(session.json(&[])["reason"], "stopped");
        assert!(!runs(&fake.log("pid")));
        assert!(!runs(&fake.log("grandchild")), "the whole group is gone");
        assert!(
            session.message("late").is_err(),
            "a write to an ended session is refused"
        );
        s.shutdown();
    }

    #[test]
    fn a_child_that_obeys_sigterm_ends_at_once() {
        let _serial = serial();
        let fake = Fake::new("t103-term", "sleep 30 & wait");
        let s = sessions(&fake, 5000);
        let session = start(&s, &fake, "default");
        fake.wait_for("pid");
        session.request_stop("stopped");
        let began = Instant::now();
        until("ended", || session.ended());
        assert!(began.elapsed() < Duration::from_secs(3));
        s.shutdown();
    }

    #[test]
    fn a_fifth_concurrent_session_is_refused_and_a_slot_frees_when_one_ends() {
        let _serial = serial();
        let fake = Fake::new("t103-max", "sleep 30 & wait");
        let s = sessions(&fake, 300);
        let first = start(&s, &fake, "default");
        let repo = fake.root.join("repo");
        let mut more = Vec::new();
        for _ in 1..MAX_RUNNING {
            more.push(
                s.start(plan(&repo, "default", false), &fake.dir(), 1, yes())
                    .unwrap(),
            );
        }
        assert_eq!(
            s.start(plan(&repo, "default", false), &fake.dir(), 1, yes()),
            Err(StartError::Busy)
        );
        first.request_stop("stopped");
        until("ended", || first.ended());
        assert!(
            s.start(plan(&repo, "default", false), &fake.dir(), 1, yes())
                .is_ok()
        );
        s.shutdown();
    }

    // ---- edge cases ----

    #[test]
    fn no_claude_and_a_child_that_exits_at_once_are_ended_sessions_with_a_fixed_reason() {
        let _serial = serial();
        let fake = Fake::new("t103-edge", "exit 3");
        let s = sessions(&fake, 300);
        let quick = start(&s, &fake, "default");
        until("ended", || quick.ended());
        assert_eq!(quick.json(&[])["reason"], "failed to start");
        std::fs::remove_file(fake.root.join("bin/claude")).unwrap();
        let none = start(&s, &fake, "default");
        assert_eq!(none.json(&[])["state"], "ended");
        assert_eq!(none.json(&[])["reason"], "claude not found");
        s.shutdown();
    }

    #[test]
    fn turning_the_setting_off_stops_the_session_within_a_second_or_two() {
        let _serial = serial();
        let fake = Fake::new("t103-off", "sleep 30 & wait");
        let s = sessions(&fake, 300);
        std::fs::create_dir_all(fake.root.join("repo")).unwrap();
        let on = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let flag = Arc::clone(&on);
        let id = s
            .start(
                plan(&fake.root.join("repo"), "default", false),
                &fake.dir(),
                1,
                Box::new(move || flag.load(Ordering::SeqCst)),
            )
            .unwrap();
        let session = s.get(&id).unwrap();
        fake.wait_for("pid");
        on.store(false, Ordering::SeqCst);
        until("ended", || session.ended());
        assert_eq!(session.json(&[])["reason"], "account turned off");
        s.shutdown();
    }

    // ---- AC 9: the text is dropped ----

    #[test]
    fn text_is_dropped_when_an_ended_session_is_closed_and_all_of_it_at_shutdown() {
        let _serial = serial();
        let fake = Fake::new("t103-drop", CHAT);
        let s = sessions(&fake, 300);
        let session = start(&s, &fake, "default");
        until("idle", || session.json(&[])["state"] == "idle");
        assert!(
            text_of(&session).contains("ANSWER-TEXT") && text_of(&session).contains("FIRST-PROMPT")
        );
        assert_eq!(
            s.close(&session.id),
            Err(true),
            "a going session is not closed"
        );
        session.request_stop("stopped");
        until("ended", || session.ended());
        assert!(
            text_of(&session).contains("ANSWER-TEXT"),
            "an ended session keeps its text while its panel is open"
        );
        assert_eq!(s.close(&session.id), Ok(()));
        let left = text_of(&session);
        assert!(
            !left.contains("ANSWER-TEXT") && !left.contains("FIRST-PROMPT"),
            "{left}"
        );
        assert!(s.get(&session.id).is_none());
        assert_eq!(s.close(&session.id), Err(false));

        let other = start(&s, &fake, "default");
        until("idle", || other.json(&[])["state"] == "idle");
        assert!(text_of(&other).contains("ANSWER-TEXT"));
        s.shutdown();
        assert!(
            !text_of(&other).contains("ANSWER-TEXT") && !text_of(&other).contains("FIRST-PROMPT")
        );
        assert!(s.all().is_empty());
        assert!(other.ended());
    }

    // ---- AC 6: prompts (the unit view; the HTTP route is in app_session_routes) ----

    fn call(tool: &str, input: Value) -> Value {
        json!({"tool_name": tool, "input": input, "tool_use_id": "toolu_1"})
    }

    fn quiet() -> (Fake, Sessions, Arc<Session>) {
        let fake = Fake::new("t103-prompt", "sleep 30 & wait");
        let s = sessions(&fake, 300);
        let session = start(&s, &fake, "default");
        (fake, s, session)
    }

    fn result_of(rx: &watch::Receiver<Option<String>>) -> Value {
        serde_json::from_str(rx.borrow().as_deref().unwrap()).unwrap()
    }

    #[test]
    fn a_prompt_waits_for_the_owner_and_the_retry_of_the_same_call_gets_the_one_answer() {
        let _serial = serial();
        let (_fake, s, session) = quiet();
        let input = json!({"command": "rm -rf /tmp/x\nand more", "description": "d"});
        let rx = session
            .open_prompt(&call("Bash", input.clone()), None)
            .unwrap();
        assert!(rx.borrow().is_none(), "nothing answers it by itself");
        let view = session.json(&[]);
        assert_eq!(view["state"], "waiting");
        let card = &view["items"][1];
        assert_eq!(card["kind"], "prompt");
        assert_eq!(card["status"], "pending");
        assert_eq!(card["tool"], "Bash");
        assert_eq!(card["summary"], "rm -rf /tmp/x and more");
        let id = card["prompt_id"].as_str().unwrap().to_string();
        let retry = session
            .open_prompt(&call("Bash", input.clone()), None)
            .unwrap();
        assert_eq!(
            session.json(&[])["items"].as_array().unwrap().len(),
            2,
            "a retried call is the same prompt"
        );
        session
            .answer(
                &id,
                &Answer {
                    allow: true,
                    answers: None,
                    message: None,
                },
            )
            .unwrap();
        assert_eq!(
            result_of(&rx),
            json!({"behavior": "allow", "updatedInput": input})
        );
        assert_eq!(result_of(&retry), result_of(&rx));
        let late = session.open_prompt(&call("Bash", input), None).unwrap();
        assert_eq!(
            result_of(&late),
            result_of(&rx),
            "a retry after the answer gets it too"
        );
        assert_eq!(
            session.answer(
                &id,
                &Answer {
                    allow: false,
                    answers: None,
                    message: None
                }
            ),
            Err(AnswerError::Settled)
        );
        assert_eq!(
            session.answer(
                "nope",
                &Answer {
                    allow: true,
                    answers: None,
                    message: None
                }
            ),
            Err(AnswerError::Unknown)
        );
        assert_eq!(session.json(&[])["state"], "working");
        s.shutdown();
    }

    #[test]
    fn deny_gives_the_note_and_a_question_is_answered_by_its_text_with_free_text_accepted() {
        let _serial = serial();
        let (_fake, s, session) = quiet();
        let rx = session
            .open_prompt(&call("Bash", json!({"command": "x"})), None)
            .unwrap();
        let id = session.json(&[])["items"][1]["prompt_id"]
            .as_str()
            .unwrap()
            .to_string();
        session
            .answer(
                &id,
                &Answer {
                    allow: false,
                    answers: None,
                    message: Some("not now".into()),
                },
            )
            .unwrap();
        assert_eq!(
            result_of(&rx),
            json!({"behavior": "deny", "message": "not now"})
        );

        let questions = json!({"questions": [{"question": "Which colour?", "header": "Colour", "multiSelect": false,
            "options": [{"label": "Red", "description": "warm"}, {"label": "Blue", "description": "cold"}]}]});
        let args =
            json!({"tool_name": "AskUserQuestion", "input": questions, "tool_use_id": "toolu_2"});
        let rx = session.open_prompt(&args, None).unwrap();
        let view = session.json(&[]);
        let card = &view["items"][2];
        assert_eq!(card["type"], "question");
        assert_eq!(
            card["questions"][0]["options"][1],
            json!({"label": "Blue", "description": "cold"})
        );
        let id = card["prompt_id"].as_str().unwrap().to_string();
        let mut answers = Map::new();
        answers.insert("Which colour?".into(), json!("Green, typed by the owner"));
        let bad = Answer {
            allow: true,
            answers: Some(Map::from_iter([("Other?".to_string(), json!("x"))])),
            message: None,
        };
        assert_eq!(
            session.answer(&id, &bad),
            Err(AnswerError::Bad("answers must be keyed by a question text"))
        );
        let none = Answer {
            allow: true,
            answers: None,
            message: None,
        };
        assert!(matches!(
            session.answer(&id, &none),
            Err(AnswerError::Bad(_))
        ));
        session
            .answer(
                &id,
                &Answer {
                    allow: true,
                    answers: Some(answers),
                    message: None,
                },
            )
            .unwrap();
        let result = result_of(&rx);
        assert_eq!(result["behavior"], "allow");
        assert_eq!(
            result["updatedInput"]["answers"],
            json!({"Which colour?": "Green, typed by the owner"})
        );
        assert_eq!(result["updatedInput"]["questions"], questions["questions"]);

        let plan = json!({"plan": "1. do it", "planFilePath": "/x/plans/p.md"});
        let args = json!({"tool_name": "ExitPlanMode", "input": plan, "tool_use_id": "toolu_3"});
        session.open_prompt(&args, None).unwrap();
        let card = session.json(&[])["items"][3].clone();
        assert_eq!(
            (card["type"].as_str(), card["plan"].as_str()),
            (Some("plan"), Some("1. do it"))
        );
        assert!(
            !card.to_string().contains("planFilePath") && !card.to_string().contains("/x/plans")
        );
        s.shutdown();
    }

    #[test]
    fn the_end_of_a_turn_or_of_the_session_cancels_a_pending_prompt_with_a_deny() {
        let _serial = serial();
        let (_fake, s, session) = quiet();
        let rx = session
            .open_prompt(&call("Bash", json!({"command": "x"})), None)
            .unwrap();
        session.handle_line(br#"{"type":"result","subtype":"error_during_execution","is_error":true,"num_turns":1}"#);
        assert_eq!(result_of(&rx)["behavior"], "deny");
        let id = session.json(&[])["items"][1]["prompt_id"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(session.json(&[])["items"][1]["status"], "cancelled");
        assert_eq!(
            session.answer(
                &id,
                &Answer {
                    allow: true,
                    answers: None,
                    message: None
                }
            ),
            Err(AnswerError::Settled)
        );
        let rx = session
            .open_prompt(
                &json!({"tool_name": "Edit", "input": {"file_path": "/a/b.rs"}}),
                Some("meta"),
            )
            .unwrap();
        assert_eq!(session.json(&[])["items"][2]["summary"], "/a/b.rs");
        s.shutdown();
        assert_eq!(result_of(&rx)["behavior"], "deny");
        assert!(
            session
                .open_prompt(&call("Bash", json!({})), None)
                .is_none(),
            "an ended session takes no prompt"
        );
    }

    #[test]
    fn a_summary_is_one_clean_line_cut_to_300_and_only_for_a_command_or_a_path() {
        let _serial = serial();
        let (_fake, s, session) = quiet();
        let long = format!("echo {}\u{202E}", "a".repeat(400));
        session
            .open_prompt(&call("Bash", json!({"command": long})), None)
            .unwrap();
        session.open_prompt(&json!({"tool_name":"WebFetch","input":{"url":"https://SECRET.example"},"tool_use_id":"u2"}), None).unwrap();
        let view = session.json(&[]);
        assert_eq!(
            view["items"][1]["summary"]
                .as_str()
                .unwrap()
                .chars()
                .count(),
            300
        );
        assert!(
            !view["items"][1]["summary"]
                .as_str()
                .unwrap()
                .contains('\u{202E}')
        );
        assert_eq!(view["items"][2]["summary"], Value::Null);
        assert!(!view.to_string().contains("SECRET.example"));
        s.shutdown();
    }
}
