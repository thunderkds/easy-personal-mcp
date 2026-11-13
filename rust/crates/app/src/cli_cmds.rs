//! The CLI commands that Python's `easy-personal` has and `easy-personal-rs` had not (T058): `ping`, `usage`, `outliers`,
//! `kit-findings`, `workspace list|add|edit|delete` and `account list|confirm|ignore|add|use` (`surfaces/cli.py`:
//! `_workspace`, `_account` and `main`). `dashboard` is `serve` under its Python name (`main.rs`).
//!
//! No store logic is written here. Every command calls what the API routes and the MCP tools already run: the core's
//! scoped usage, outliers and kit findings, and the store-only writes of T048 (`workspaces::add`, `patch`, `delete`,
//! `accounts::confirm`, ...). This module reads the arguments, prints what Python prints (the same JSON text,
//! `json.dumps(indent=2, sort_keys=True)`), asks the confirmation question, and maps a refusal to an exit code.
//!
//! What differs from Python, by design (all listed in `rust/README.md`): a store that is missing or newer than this build
//! is refused with the API's fixed text and nothing is created or migrated (Python creates and migrates); a change that would
//! run the `claude` CLI is refused with the T048 text, exit 2 (Python would run it); the confirmation question is asked with no
//! write lock held, and the plan is worked out a second time when the answer is yes; a changed plan is refused.
//! Every write opens the store only for the call (`workspaces::write_session`), takes the same busy wait as the routes, and
//! the account commands detect new agent folders only under `EASY_PERSONAL_AGENT_ROOT`, else `HOME`. No message names a
//! path or a row; an error text is the core's own fixed text (Python's), which names none of the caller's input (NFR-002).

use crate::{api, mcp, writes};
use clap::{Args, Subcommand};
use easy_personal_core::accounts;
use easy_personal_core::clock::now_micros;
use easy_personal_core::kitperf::kit_findings_local;
use easy_personal_core::metrics::{MetricsError, usage_breakdown_scoped, usage_summary_scoped};
use easy_personal_core::scope::Scope;
use easy_personal_core::sessions::session_outliers_scoped;
use easy_personal_core::store::{self, StoreError};
use easy_personal_core::workspaces::{self, Outcome, WriteFail, py_strip};
use rusqlite::Connection;
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::io::{BufRead, Write};
use std::process::ExitCode;

/// What the owner asked for is not done: the exit code, and the one stderr line (none when the command printed its own).
pub(crate) struct Stop {
    code: u8,
    line: Option<String>,
}

impl Stop {
    /// A refused request (Python's exit 2).
    fn refused(text: impl Into<String>) -> Stop {
        Stop {
            code: 2,
            line: Some(text.into()),
        }
    }

    fn failed(code: i32, text: impl Into<String>) -> Stop {
        Stop {
            code: u8::try_from(code).unwrap_or(1),
            line: Some(text.into()),
        }
    }

    /// The command printed its own answer; only the exit code is left.
    fn silent(code: u8) -> Stop {
        Stop { code, line: None }
    }
}

type Run = Result<(), Stop>;

fn finish(run: Run) -> ExitCode {
    match run {
        Ok(()) => ExitCode::SUCCESS,
        Err(Stop { code, line }) => {
            if let Some(line) = line {
                eprintln!("easy-personal-rs: {line}");
            }
            ExitCode::from(code)
        }
    }
}

/// Writes to stdout at once (the question comes before the answer is read). A closed stdout ends the command.
fn put(text: &str) -> Run {
    let mut out = std::io::stdout().lock();
    out.write_all(text.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|_| Stop::failed(1, "cannot write the result to stdout"))
}

fn put_lines(lines: &[String]) -> Run {
    lines.iter().try_for_each(|line| put(&format!("{line}\n")))
}

fn put_json(value: &Value) -> Run {
    put(&format!("{}\n", py_json(value)))
}

// ---- the store ----

/// What the API says for a store problem (`api::store_failure`); exit 2 for a newer store, 1 for the rest.
fn store_stop(error: StoreError) -> Stop {
    let code = error.exit_code();
    Stop::failed(code, api::store_failure(error).1)
}

fn metrics_stop(error: MetricsError) -> Stop {
    match error {
        MetricsError::Invalid(text) => Stop::refused(text),
        MetricsError::Store(error) => store_stop(error),
        MetricsError::Prices(_) => Stop::failed(1, api::INTERNAL),
    }
}

fn write_stop(failure: WriteFail) -> Stop {
    let code = match &failure {
        WriteFail::Refused { .. } | WriteFail::NeedsClaude => 2,
        WriteFail::Store(error) => error.exit_code(),
        WriteFail::Write(error) => error.exit_code(),
    };
    Stop::failed(code, writes::refusal_parts(failure).1)
}

/// The store, read-only, for one command. A store that is missing, not a file or newer is refused; nothing is created.
fn open_read() -> Result<Connection, Stop> {
    let path = store::store_path().map_err(store_stop)?;
    store::open_checked(&path).map_err(store_stop)
}

/// The store for writing, for one command (`workspaces::write_session`: never creates a store, needs schema v11, waits 5 s
/// for a busy writer). `detect` runs Python's `accounts.detect` first, over the agent root only.
fn write<T>(
    detect: bool,
    work: impl FnOnce(&Connection) -> Result<T, WriteFail>,
) -> Result<T, Stop> {
    let path = store::store_path().map_err(store_stop)?;
    let root = api::agent_root();
    workspaces::write_session(&path, detect.then_some(root.as_path()), work).map_err(write_stop)
}

/// An absent filter is none. An empty one is a name no workspace and no account has: Python passes `""` on, and its
/// scope check refuses it (`unknown workspace`), where the core reads `""` as no filter.
fn filter(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(|v| if v.is_empty() { "\u{0}" } else { v })
}

fn scope<'a>(workspace: &'a Option<String>, account: &'a Option<String>) -> Scope<'a> {
    Scope {
        workspace: filter(workspace),
        account: filter(account),
    }
}

// ---- the arguments ----

/// `positive_int` of Python's CLI: `int(text)` (surrounding space, a sign, digits with single underscores between them)
/// that is at least 1. A number past 64 bits stays valid (the core caps a limit at 10). Digits of other scripts are refused.
fn positive_int(text: &str) -> Result<i64, String> {
    let trimmed = py_strip(text);
    let digits = trimmed
        .strip_prefix('+')
        .or_else(|| trimmed.strip_prefix('-'))
        .unwrap_or(trimmed);
    let bytes = digits.as_bytes();
    let shaped = !bytes.is_empty()
        && bytes[0].is_ascii_digit()
        && bytes[bytes.len() - 1].is_ascii_digit()
        && bytes.iter().all(|b| b.is_ascii_digit() || *b == b'_')
        && !digits.contains("__");
    if !shaped {
        return Err("not a whole number".into());
    }
    if trimmed.starts_with('-') {
        return Err("must be at least 1".into());
    }
    let digits: String = digits.chars().filter(|c| *c != '_').collect();
    match digits.parse::<i64>() {
        Ok(0) => Err("must be at least 1".into()),
        Ok(n) => Ok(n),
        Err(_) if digits.trim_start_matches('0').is_empty() => Err("must be at least 1".into()),
        Err(_) => Ok(i64::MAX),
    }
}

#[derive(Args)]
pub struct UsageArgs {
    /// ISO date or datetime, inclusive
    #[arg(long, value_name = "SINCE")]
    since: Option<String>,
    /// ISO date or datetime, exclusive
    #[arg(long, value_name = "UNTIL")]
    until: Option<String>,
    /// Break tokens down by this dimension: day, model, project or tool
    #[arg(long, value_name = "BY")]
    by: Option<String>,
    /// Only events under this workspace's repo paths
    #[arg(long)]
    workspace: Option<String>,
    /// Only events of this agent account (id or label)
    #[arg(long)]
    account: Option<String>,
}

#[derive(Args)]
pub struct OutliersArgs {
    /// ISO date or datetime, inclusive
    #[arg(long, value_name = "SINCE")]
    since: Option<String>,
    /// ISO date or datetime, exclusive
    #[arg(long, value_name = "UNTIL")]
    until: Option<String>,
    /// Only events under this workspace's repo paths
    #[arg(long)]
    workspace: Option<String>,
    /// Only events of this agent account (id or label)
    #[arg(long)]
    account: Option<String>,
}

#[derive(Args)]
pub struct KitFindingsArgs {
    /// Print the full result as JSON
    #[arg(long)]
    json: bool,
    /// At most this many findings (the core caps it at 10)
    #[arg(long, default_value = "3", value_parser = positive_int)]
    limit: i64,
    /// ISO date or datetime, inclusive
    #[arg(long, value_name = "SINCE")]
    since: Option<String>,
}

#[derive(Subcommand)]
pub enum WorkspaceAction {
    /// List workspaces with their repo paths
    List,
    /// Create a workspace (`create` is the same command)
    #[command(alias = "create")]
    Add(AddArgs),
    /// Change a workspace; a new name of one that holds connectors shows the alias plan first
    Edit(EditArgs),
    /// Delete a workspace; its connectors move to orphan
    Delete(DeleteArgs),
}

#[derive(Args)]
pub struct AddArgs {
    name: String,
    #[arg(long)]
    color: Option<String>,
    #[arg(long)]
    icon: Option<String>,
    #[arg(long)]
    description: Option<String>,
    #[arg(long)]
    email: Option<String>,
    /// Repo path (repeatable)
    #[arg(long)]
    repo: Vec<String>,
}

#[derive(Args)]
pub struct EditArgs {
    name: String,
    /// An empty value clears it
    #[arg(long)]
    color: Option<String>,
    /// An empty value clears it
    #[arg(long)]
    icon: Option<String>,
    /// An empty value clears it
    #[arg(long)]
    description: Option<String>,
    /// An empty value clears it
    #[arg(long)]
    email: Option<String>,
    /// New name; the workspace id stays
    #[arg(long)]
    rename: Option<String>,
    /// Skip the confirmation question
    #[arg(long)]
    yes: bool,
    /// Repeatable
    #[arg(long)]
    add_repo: Vec<String>,
    /// Repeatable
    #[arg(long)]
    remove_repo: Vec<String>,
}

#[derive(Args)]
pub struct DeleteArgs {
    name: String,
    /// Skip the confirmation question
    #[arg(long)]
    yes: bool,
}

#[derive(Subcommand)]
pub enum AccountAction {
    /// Detect new agent folders (under the agent root only), then list every account
    List {
        /// Grouping
        #[arg(long, default_value = "account", value_parser = ["account", "provider"])]
        by: String,
    },
    /// Name an account and mark it confirmed
    Confirm {
        /// Account id or folder
        account: String,
        /// 1 to 60 characters, e.g. 'Claude work'
        #[arg(long)]
        label: String,
    },
    /// Hide an account from re-detection
    Ignore {
        /// Account id or folder
        account: String,
    },
    /// Add an agent folder by path
    Add { path: String },
    /// Set the workspaces a Claude account picks (none clears them)
    Use {
        /// Account id or folder
        account: String,
        /// Workspace names
        workspaces: Vec<String>,
    },
}

// ---- ping, usage, outliers, kit-findings ----

pub fn ping() -> ExitCode {
    finish(put(&format!("{}\n", mcp::PING_TEXT)))
}

pub fn usage(args: UsageArgs) -> ExitCode {
    finish((|| {
        let conn = open_read()?;
        let (since, until) = (args.since.as_deref(), args.until.as_deref());
        let scope = scope(&args.workspace, &args.account);
        let body = match args.by.as_deref() {
            Some(by) => usage_breakdown_scoped(&conn, by, since, until, scope),
            None => usage_summary_scoped(&conn, since, until, scope),
        }
        .map_err(metrics_stop)?;
        put_json(&body)
    })())
}

pub fn outliers(args: OutliersArgs) -> ExitCode {
    finish((|| {
        let conn = open_read()?;
        let scope = scope(&args.workspace, &args.account);
        let body =
            session_outliers_scoped(&conn, args.since.as_deref(), args.until.as_deref(), scope)
                .map_err(metrics_stop)?;
        put_json(&body)
    })())
}

pub fn kit_findings(args: KitFindingsArgs) -> ExitCode {
    finish((|| {
        let conn = open_read()?;
        let body = kit_findings_local(&conn, args.limit, args.since.as_deref(), now_micros())
            .map_err(metrics_stop)?;
        if args.json {
            put_json(&body)
        } else {
            put(&findings_text(&body))
        }
    })())
}

/// The text of `kit-findings` without `--json` (Python's `main`): each finding with its habit, then the others under a
/// heading, or why there is none.
fn findings_text(result: &Value) -> String {
    let list = |key| {
        result[key]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
    };
    let text = |finding: &Value, key| finding[key].as_str().unwrap_or_default().to_string();
    let (findings, other) = (list("findings"), list("other"));
    let mut out = String::new();
    for finding in findings {
        out += &format!("{} {}\n", text(finding, "text"), text(finding, "habit"));
    }
    if findings.is_empty() && !other.is_empty() {
        out += "No kit finding with a habit yet.\n";
    }
    if !other.is_empty() {
        out += "Other comparisons (no habit):\n";
        for finding in other {
            out += &format!("  {}\n", text(finding, "text"));
        }
    }
    if findings.is_empty() && other.is_empty() {
        let servers: Vec<String> = list("not_enough")
            .iter()
            .map(|n| format!("{} {} vs {}", text(n, "server"), n["with"], n["without"]))
            .collect();
        out += &format!("No kit finding yet: {}\n", servers.join(", "));
    }
    out
}

// ---- workspace ----

pub fn workspace(action: WorkspaceAction) -> ExitCode {
    finish(match action {
        WorkspaceAction::List => open_read().and_then(|conn| {
            let body = workspaces::list_all(&conn).map_err(metrics_stop)?;
            put_json(&body)
        }),
        WorkspaceAction::Add(args) => workspace_add(args),
        WorkspaceAction::Edit(args) => workspace_edit(args),
        WorkspaceAction::Delete(args) => workspace_delete(args),
    })
}

fn strings(values: &[String]) -> Value {
    Value::Array(values.iter().cloned().map(Value::String).collect())
}

fn workspace_add(args: AddArgs) -> Run {
    let mut body = Map::new();
    body.insert("name".into(), Value::String(args.name));
    for (field, text) in [
        ("color", args.color),
        ("icon", args.icon),
        ("description", args.description),
        ("email", args.email),
    ] {
        if let Some(text) = text {
            body.insert(field.into(), Value::String(text));
        }
    }
    let repos = workspaces::prepare_repos(Some(&strings(&args.repo)));
    let made = write(false, |conn| workspaces::add(conn, &body, repos))?;
    put_json(&made)
}

/// `workspaces.by_name`: the workspace as the list shows it, or `unknown workspace`.
fn by_name(conn: &Connection, name: &str) -> Result<Value, WriteFail> {
    workspaces::list_all(conn)?["workspaces"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|w| w["name"] == name)
        .cloned()
        .ok_or(WriteFail::Refused {
            status: 404,
            text: "unknown workspace".into(),
        })
}

fn workspace_edit(args: EditArgs) -> Run {
    let EditArgs {
        name,
        color,
        icon,
        description,
        email,
        rename,
        yes,
        add_repo,
        remove_repo,
    } = args;
    // Python's order: the text fields, then the new name, then the repos; an empty text clears the field
    let mut changes: Vec<(String, Value)> = Vec::new();
    for (field, text) in [
        ("color", color),
        ("icon", icon),
        ("description", description),
        ("email", email),
    ] {
        if let Some(text) = text {
            let value = if text.is_empty() {
                Value::Null
            } else {
                Value::String(text)
            };
            changes.push((field.into(), value));
        }
    }
    if let Some(new) = rename {
        changes.push(("name".into(), Value::String(new)));
    }
    let (add_repo, remove_repo) = (strings(&add_repo), strings(&remove_repo));
    let edited = change(yes, |conn, apply| {
        let current = by_name(conn, &name)?;
        let id = current["id"].as_i64().unwrap_or_default();
        let mut changes = changes.clone();
        let mut repos = None;
        if add_repo != Value::Array(Vec::new()) || remove_repo != Value::Array(Vec::new()) {
            let drop: HashSet<String> = remove_repo
                .as_array()
                .into_iter()
                .flatten()
                .map(workspaces::normalise_repo)
                .collect::<Result<_, _>>()?;
            let mut all: Vec<Value> = current["repos"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|r| r.as_str().is_none_or(|r| !drop.contains(r)))
                .cloned()
                .collect();
            all.extend(add_repo.as_array().cloned().unwrap_or_default());
            let all = Value::Array(all);
            repos = Some(workspaces::prepare_repos(Some(&all)));
            changes.push(("repos".into(), all));
        }
        let outcome = workspaces::patch(conn, id, changes, repos, apply)?;
        Ok((outcome, workspaces::get(conn, id)?))
    })?;
    put_json(&edited)
}

fn workspace_delete(args: DeleteArgs) -> Run {
    change(args.yes, |conn, apply| {
        let id = by_name(conn, &args.name)?["id"]
            .as_i64()
            .unwrap_or_default();
        Ok((workspaces::delete(conn, id, apply)?, Value::Null))
    })
    .map(drop)
}

/// Python's `_change` for the two plan-first writes. The plan is worked out first (`call(conn, false)`: nothing is
/// stored). A write that needs no plan is already done and answers at once. Otherwise the plan is printed, the owner
/// is asked (unless `--yes`), and only a yes applies it. The question is asked with no store handle open; the apply
/// works the plan out once more and refuses it if it is not the one that was shown (it is the only guard against a
/// change made meanwhile: a few microseconds are left between that check and the commit, as in the routes). Returns
/// what `call` gave for the workspace after the change.
fn change(
    yes: bool,
    call: impl Fn(&Connection, bool) -> Result<(Outcome, Value), WriteFail>,
) -> Result<Value, Stop> {
    let (shown, body) = write(false, |conn| call(conn, false))?;
    if !shown.pending {
        return Ok(body);
    }
    put_lines(&shown.lines)?;
    if !yes && !ask()? {
        put("aborted; nothing was changed.\n")?;
        return Err(Stop::silent(1));
    }
    let (done, body) = write(false, |conn| {
        if call(conn, false)?.0.lines != shown.lines {
            return Err(WriteFail::Refused {
                status: 409,
                text: "the plan changed while the question was open; nothing was changed".into(),
            });
        }
        call(conn, true)
    })?;
    put_lines(done.lines.get(shown.lines.len()..).unwrap_or_default())?;
    Ok(body)
}

/// Python's `_confirm_text`: yes or no on stdin. Closed stdin (end of file) or an unreadable line is no.
fn ask() -> Result<bool, Stop> {
    put("Apply these changes? [y/N] ")?;
    let mut line = String::new();
    match std::io::stdin().lock().read_line(&mut line) {
        Ok(read) if read > 0 => Ok(matches!(
            py_strip(&line).to_lowercase().as_str(),
            "y" | "yes"
        )),
        _ => {
            put("\n")?;
            Ok(false)
        }
    }
}

// ---- account ----

pub fn account(action: AccountAction) -> ExitCode {
    finish(run_account(action))
}

fn run_account(action: AccountAction) -> Run {
    let root = api::agent_root();
    match action {
        AccountAction::List { by } => {
            let body = write(true, |conn| {
                Ok(accounts::grouped(conn, &by, now_micros(), &root)?)
            })?;
            put_json(&body)
        }
        AccountAction::Confirm { account, label } => {
            let body = write(true, |conn| {
                accounts::confirm(conn, &account, &Value::String(label), &root)
            })?;
            put_json(&body)
        }
        AccountAction::Ignore { account } => {
            let body = write(true, |conn| accounts::ignore(conn, &account, &root))?;
            put_json(&body)
        }
        AccountAction::Add { path } => {
            let folder = accounts::prepare_add(&Value::String(path));
            let body = write(true, |conn| accounts::add(conn, folder, &root))?;
            put_json(&body)
        }
        AccountAction::Use {
            account,
            workspaces,
        } => {
            let names = strings(&workspaces);
            let body = write(true, |conn| {
                accounts::use_workspaces(conn, &account, &names, &root)
            })?;
            put_json(&body)?;
            eprintln!(
                "run `easy-personal connector sync` to apply it to the account's Claude config."
            );
            Ok(())
        }
    }
}

// ---- json.dumps(value, indent=2, sort_keys=True) ----

/// Python's `json.dumps(value, indent=2, sort_keys=True)`: two-space indent, keys in code-point order, `ensure_ascii`
/// escapes (a character past U+007F is `\uXXXX`, a pair above U+FFFF), floats as `repr` writes them.
pub(crate) fn py_json(value: &Value) -> String {
    let mut out = String::new();
    dump(value, 0, &mut out);
    out
}

fn dump(value: &Value, level: usize, out: &mut String) {
    let pad = |out: &mut String, level: usize| out.push_str(&"  ".repeat(level));
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => match (n.as_i64(), n.as_u64()) {
            (Some(i), _) => out.push_str(&i.to_string()),
            (_, Some(u)) => out.push_str(&u.to_string()),
            _ => out.push_str(&py_float(n.as_f64().unwrap_or(f64::NAN))),
        },
        Value::String(text) => quote(text, out),
        Value::Array(items) if items.is_empty() => out.push_str("[]"),
        Value::Array(items) => {
            out.push_str("[\n");
            for (at, item) in items.iter().enumerate() {
                if at > 0 {
                    out.push_str(",\n");
                }
                pad(out, level + 1);
                dump(item, level + 1, out);
            }
            out.push('\n');
            pad(out, level);
            out.push(']');
        }
        Value::Object(map) if map.is_empty() => out.push_str("{}"),
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push_str("{\n");
            for (at, key) in keys.into_iter().enumerate() {
                if at > 0 {
                    out.push_str(",\n");
                }
                pad(out, level + 1);
                quote(key, out);
                out.push_str(": ");
                dump(&map[key.as_str()], level + 1, out);
            }
            out.push('\n');
            pad(out, level);
            out.push('}');
        }
    }
}

fn quote(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if c.is_ascii() && c >= ' ' => out.push(c),
            c => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out.push('"');
}

/// Python's `repr(float)`: the shortest digits that read back the same, in fixed notation from 1e-4 up to (not including)
/// 1e16 and in exponent notation (`1e-05`, `1.5e+16`) outside it; a whole number keeps `.0`.
fn py_float(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-Infinity" } else { "Infinity" }.into();
    }
    let sign = if x.is_sign_negative() { "-" } else { "" };
    if x == 0.0 {
        return format!("{sign}0.0");
    }
    let sci = format!("{:e}", x.abs());
    let (mantissa, exponent) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    if (-4..16).contains(&exponent) {
        if exponent >= 0 {
            let whole = exponent as usize + 1;
            let padded = format!("{digits:0<whole$}");
            let (int, frac) = padded.split_at(whole);
            format!("{sign}{int}.{}", if frac.is_empty() { "0" } else { frac })
        } else {
            format!("{sign}0.{}{digits}", "0".repeat((-exponent - 1) as usize))
        }
    } else {
        let mantissa = match digits.split_at(1) {
            (first, "") => first.to_string(),
            (first, rest) => format!("{first}.{rest}"),
        };
        let marker = if exponent < 0 { '-' } else { '+' };
        format!("{sign}{mantissa}e{marker}{:02}", exponent.abs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Expected texts are `json.dumps(value, indent=2, sort_keys=True)` of Python 3.13 for the same value.
    #[test]
    fn json_text_is_pythons() {
        let value = json!({"b": [], "a": {}, "c": [1, {"z": null, "y": true}], "d": "q\"\\\n\t\u{1}\u{7f}é☕\u{1f680}"});
        assert_eq!(
            py_json(&value),
            "{\n  \"a\": {},\n  \"b\": [],\n  \"c\": [\n    1,\n    {\n      \"y\": true,\n      \"z\": null\n    }\n  ],\n  \"d\": \"q\\\"\\\\\\n\\t\\u0001\u{7f}\\u00e9\\u2615\\ud83d\\ude80\"\n}"
        );
        assert_eq!(py_json(&json!([])), "[]");
        assert_eq!(py_json(&json!(5)), "5");
    }

    /// Expected texts are Python's `repr(float)`.
    #[test]
    fn floats_are_written_as_python_repr_does() {
        for (x, want) in [
            (0.0, "0.0"),
            (-0.0, "-0.0"),
            (1.0, "1.0"),
            (3.0, "3.0"),
            (0.5, "0.5"),
            (0.1, "0.1"),
            (1234.5, "1234.5"),
            (100.0, "100.0"),
            (0.0001, "0.0001"),
            (0.00012, "0.00012"),
            (0.00001, "1e-05"),
            (0.000012345, "1.2345e-05"),
            (1e15, "1000000000000000.0"),
            (123456789012345.0, "123456789012345.0"),
            (1e16, "1e+16"),
            (1.5e16, "1.5e+16"),
            (1.2345678901234568e17, "1.2345678901234568e+17"),
            (1e-7, "1e-07"),
            (1e100, "1e+100"),
            (-2.5, "-2.5"),
            (-0.00001, "-1e-05"),
            (0.30000000000000004, "0.30000000000000004"),
            (5e-324, "5e-324"),
        ] {
            assert_eq!(py_float(x), want, "{x:e}");
        }
        assert_eq!(
            py_json(&json!({"x": 2.0, "y": 7})),
            "{\n  \"x\": 2.0,\n  \"y\": 7\n}"
        );
    }

    #[test]
    fn a_limit_reads_like_pythons_int() {
        for (text, want) in [
            ("1", Some(1)),
            ("10", Some(10)),
            (" 4 ", Some(4)),
            ("+3", Some(3)),
            ("1_0", Some(10)),
            ("007", Some(7)),
            ("99999999999999999999", Some(i64::MAX)),
            ("0", None),
            ("00", None),
            ("-1", None),
            ("-0", None),
            ("", None),
            ("abc", None),
            ("1.5", None),
            ("_1", None),
            ("1_", None),
            ("1__0", None),
            ("+", None),
            ("1 2", None),
        ] {
            assert_eq!(positive_int(text).ok(), want, "{text:?}");
        }
    }

    fn finding(text: &str, habit: Value) -> Value {
        json!({"text": text, "habit": habit})
    }

    /// The four shapes of Python's text for `kit-findings` (`main`).
    #[test]
    fn the_findings_text_is_pythons_in_each_shape() {
        let both = json!({"findings": [finding("A.", json!("Do a."))], "other": [finding("B.", Value::Null)], "not_enough": []});
        assert_eq!(
            findings_text(&both),
            "A. Do a.\nOther comparisons (no habit):\n  B.\n"
        );
        let only_other = json!({"findings": [], "other": [finding("B.", Value::Null), finding("C.", Value::Null)], "not_enough": []});
        assert_eq!(
            findings_text(&only_other),
            "No kit finding with a habit yet.\nOther comparisons (no habit):\n  B.\n  C.\n"
        );
        let none = json!({"findings": [], "other": [], "not_enough": [
            {"server": "easy-ui", "with": 0, "without": 3}, {"server": "easy-memory", "with": 12, "without": 0}]});
        assert_eq!(
            findings_text(&none),
            "No kit finding yet: easy-ui 0 vs 3, easy-memory 12 vs 0\n"
        );
        let findings_only = json!({"findings": [finding("A.", json!("Do a.")), finding("B.", json!("Do b."))], "other": [], "not_enough": []});
        assert_eq!(findings_text(&findings_only), "A. Do a.\nB. Do b.\n");
    }

    /// Python passes an empty `--workspace` / `--account` on, and its scope check refuses it; the core would read it as no filter.
    #[test]
    fn an_empty_filter_is_a_name_nobody_has() {
        assert_eq!(filter(&None), None);
        assert_eq!(filter(&Some("acme".into())), Some("acme"));
        assert_eq!(filter(&Some(String::new())), Some("\u{0}"));
    }
}
