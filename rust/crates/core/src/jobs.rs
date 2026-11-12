//! Scheduled jobs (FR-040): systemd user timers and crontab lines that run an agent command.
//!
//! Port of `src/easy_personal/core/jobs.py`. Read-only. The only processes started are the rows of [`READ_ONLY`]
//! (`systemctl --user list-timers|show`, `crontab -l`), each with an argument list (never a shell), the [`ProcessEnv`] it
//! is given plus `LC_ALL=C` (the route passes this process's own environment, as Python does: the user bus variables
//! are in it; the tests pass a `PATH` alone), a closed stdin, a read cap and a [`TIMEOUT`]. Only the command's first word
//! (basename) leaves this module: arguments, the full `ExecStart` and environment lines are parsed in memory to find
//! that word and then dropped. Each kind that cannot be read (no tool, no user bus, timeout) is `no data`; the other
//! still lists.

use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Seconds a call may run before it is killed.
pub const TIMEOUT: Duration = Duration::from_secs(3);
/// The most output read from one call; a longer answer is `no data` (Python reads without a limit).
pub const MAX_OUTPUT: usize = 8 * 1024 * 1024;
/// Tool -> the only subcommands that may run. `systemctl` is always called with `--user` first.
pub const READ_ONLY: [(&str, &[&str]); 2] = [
    ("systemctl", &["list-timers", "show"]),
    ("crontab", &["-l"]),
];
/// Python's `JOBS_ALLOW_DEFAULT` (a `jobs: allow:` line in `config.yaml` is not read by this build).
pub const ALLOW: [&str; 7] = [
    "claude",
    "easy-personal",
    "easy-personal-launch",
    "easy-verifier",
    "easy-context",
    "easy-ui",
    "easy-memory",
];
const NO_DATA: &str = "no data";
const OK: &str = "ok";
const SHELLS: [&str; 4] = ["sh", "bash", "zsh", "dash"];

/// The environment a call runs in: exactly these variables plus `LC_ALL=C` (the child starts from an empty one).
#[derive(Debug, Clone)]
pub struct ProcessEnv(pub Vec<(OsString, OsString)>);

impl ProcessEnv {
    /// This process's environment (the user bus variables are in it).
    pub fn inherited() -> Self {
        ProcessEnv(std::env::vars_os().collect())
    }

    /// Nothing but a `PATH` (the tests' scratch folder of fakes).
    pub fn only_path(path: impl Into<OsString>) -> Self {
        ProcessEnv(vec![("PATH".into(), path.into())])
    }

    fn path(&self) -> Option<&OsString> {
        self.0
            .iter()
            .rev()
            .find(|(k, _)| k == "PATH")
            .map(|(_, v)| v)
    }

    /// `shutil.which`: the first executable regular file named `tool` in `PATH`; no `PATH` or an empty one finds nothing.
    fn which(&self, tool: &str) -> Option<PathBuf> {
        std::env::split_paths(self.path().filter(|p| !p.is_empty())?)
            .map(|dir| dir.join(tool))
            .find(|p| {
                p.metadata()
                    .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            })
    }
}

/// `argv` is not one of the [`READ_ONLY`] calls.
#[derive(Debug, PartialEq)]
pub struct NotReadOnly;

/// Whether `argv` (tool first) is on the allow-list: the whole argument list is checked, not just the verb.
pub fn is_read_only(argv: &[&str]) -> bool {
    let Some((tool, args)) = argv.split_first() else {
        return false;
    };
    let sub = match (*tool, args) {
        ("systemctl", ["--user", sub, ..]) => *sub,
        ("crontab", [sub, ..]) => *sub,
        _ => return false,
    };
    READ_ONLY
        .iter()
        .any(|(t, subs)| t == tool && subs.contains(&sub))
        && (*tool != "crontab" || args == ["-l"])
}

/// stdout of one read-only command; `None` when the tool is missing, fails, times out or prints too much.
fn run(env: &ProcessEnv, argv: &[&str]) -> Result<Option<String>, NotReadOnly> {
    if !is_read_only(argv) {
        return Err(NotReadOnly);
    }
    let Some(program) = env.which(argv[0]) else {
        return Ok(None);
    };
    let Ok(mut child) = Command::new(program)
        .args(&argv[1..])
        .env_clear()
        .envs(env.0.iter().map(|(k, v)| (k, v)))
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Ok(None);
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Ok(None);
    };
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.take(MAX_OUTPUT as u64 + 1).read_to_end(&mut buf);
        let _ = tx.send(buf);
    });
    let deadline = Instant::now() + TIMEOUT;
    // the reader ends when the child closes its output (or has printed more than the cap)
    let buf = match rx.recv_timeout(TIMEOUT) {
        Ok(buf) if buf.len() <= MAX_OUTPUT => buf,
        _ => {
            let _ = child.kill();
            let _ = child.wait(); // no zombie
            return Ok(None);
        }
    };
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(None);
            }
        }
    };
    Ok(status
        .success()
        .then(|| String::from_utf8_lossy(&buf).into_owned()))
}

fn systemctl(env: &ProcessEnv, args: &[&str]) -> Option<String> {
    let mut argv = vec!["systemctl", "--user"];
    argv.extend_from_slice(args);
    run(env, &argv).ok().flatten()
}

// ---- words -----------------------------------------------------------------------------------------------------

/// Python's `shlex.split` (POSIX mode); `None` is its `ValueError` (an open quote or a trailing backslash).
fn shlex_split(line: &str) -> Option<Vec<String>> {
    let (mut words, mut word, mut in_word) = (Vec::new(), String::new(), false);
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' | '\r' | '\n' => {
                if in_word {
                    words.push(std::mem::take(&mut word));
                    in_word = false;
                }
            }
            '\\' => {
                word.push(chars.next()?);
                in_word = true;
            }
            '\'' => {
                loop {
                    match chars.next()? {
                        '\'' => break,
                        c => word.push(c),
                    }
                }
                in_word = true;
            }
            '"' => {
                loop {
                    match chars.next()? {
                        '"' => break,
                        '\\' => match chars.next()? {
                            c @ ('"' | '\\') => word.push(c),
                            c => {
                                word.push('\\');
                                word.push(c);
                            }
                        },
                        c => word.push(c),
                    }
                }
                in_word = true;
            }
            c => {
                word.push(c);
                in_word = true;
            }
        }
    }
    if in_word {
        words.push(word);
    }
    Some(words)
}

fn words(line: &str) -> Vec<String> {
    shlex_split(line).unwrap_or_else(|| line.split_whitespace().map(str::to_string).collect())
}

/// `^[A-Za-z_][A-Za-z0-9_]*=` (Python's `_ASSIGNMENT.match`).
fn is_assignment(text: &str) -> bool {
    let name_len = text
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .count();
    name_len > 0
        && !text.starts_with(|c: char| c.is_ascii_digit())
        && text[name_len..].starts_with('=')
}

/// `PurePath(word).name`: the last component, `.` and empty ones ignored.
fn base_name(word: &str) -> &str {
    word.split('/')
        .rfind(|p| !p.is_empty() && *p != ".")
        .unwrap_or("")
}

/// Basename of the command in `words`, looking through `env`, `VAR=x`, `sh -c` and `python -m easy_personal`.
fn first_word(words: &[String], depth: usize) -> Option<String> {
    let mut i = 0;
    while i < words.len() {
        if is_assignment(&words[i]) {
            i += 1;
        } else if base_name(&words[i]) == "env" {
            i += 1;
            while i < words.len() && (words[i].starts_with('-') || is_assignment(&words[i])) {
                i += 1;
            }
        } else {
            break;
        }
    }
    let name = base_name(words.get(i)?);
    let rest = &words[i + 1..];
    if SHELLS.contains(&name)
        && depth < 3
        && let Some(at) = rest
            .iter()
            .position(|w| w == "-c")
            .filter(|at| at + 1 < rest.len())
    {
        return first_word(&self::words(&rest[at + 1]), depth + 1);
    }
    if name.starts_with("python")
        && rest.first().is_some_and(|w| w == "-m")
        && rest
            .get(1)
            .is_some_and(|m| m.split('.').next() == Some("easy_personal"))
    {
        return Some("easy-personal".to_string()); // the fallback command `schedule.command()` writes when the script is not on PATH
    }
    (!name.is_empty()).then(|| name.to_string())
}

/// The first word (basename) of a command line, or `None` when there is no command.
pub fn command_name(line: &str) -> Option<String> {
    first_word(&words(line), 0)
}

// ---- crontab ---------------------------------------------------------------------------------------------------

/// Python's `int(text)` for the plain forms: an optional sign and digits (single `_` between digits allowed). A Python
/// int has no limit; one beyond 64 bits is the nearest `i64`, which every use here (a bound against 0-59, a step) treats
/// the same.
fn py_int(text: &str) -> Option<i64> {
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    let clean = !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit() || c == '_')
        && !digits.starts_with('_')
        && !digits.ends_with('_')
        && !digits.contains("__");
    if !clean {
        return None;
    }
    let number = text.replace('_', "");
    Some(number.parse().unwrap_or(if number.starts_with('-') {
        i64::MIN
    } else {
        i64::MAX
    }))
}

fn field(text: &str, low: i64, high: i64) -> Option<BTreeSet<i64>> {
    let mut out = BTreeSet::new();
    for part in text.split(',') {
        let (span, step_text) = match part.split_once('/') {
            Some((span, step)) => (span, Some(step)),
            None => (part, None),
        };
        let step = match step_text {
            Some(s) => py_int(s)?,
            None => 1,
        };
        if step < 1 {
            return None;
        }
        let (first, last) = if span == "*" {
            (low, high)
        } else if let Some((a, b)) = span.split_once('-') {
            (py_int(a)?, py_int(b)?)
        } else {
            let first = py_int(span)?;
            (first, if step_text.is_some() { high } else { first })
        };
        if !(low <= first && first <= last && last <= high) {
            return None;
        }
        let mut n = first;
        while n <= last {
            out.insert(n);
            let Some(next) = n.checked_add(step) else {
                break; // past every i64: Python's `range` ends here too
            };
            n = next;
        }
    }
    Some(out)
}

fn alias(expr: &str) -> &str {
    match expr {
        "@hourly" => "0 * * * *",
        "@daily" | "@midnight" => "0 0 * * *",
        "@weekly" => "0 0 * * 0",
        "@monthly" => "0 0 1 * *",
        "@yearly" | "@annually" => "0 0 1 1 *",
        other => other,
    }
}

/// Next local time strictly after `after` that a 5-field cron expression fires; `None` if unreadable.
/// Names (`mon`, `jan`) and `@reboot` are not read: `None`, shown as "unknown".
pub fn next_cron(expr: &str, after: DateTime) -> Option<DateTime> {
    let fields: Vec<&str> = alias(expr).split_whitespace().collect();
    if fields.len() != 5 {
        return None;
    }
    let limits = [(0, 59), (0, 23), (1, 31), (1, 12), (0, 7)];
    let mut parsed = Vec::new();
    for (f, (lo, hi)) in fields.iter().zip(limits) {
        parsed.push(field(f, lo, hi)?);
    }
    let weekdays: BTreeSet<i64> = parsed[4].iter().map(|d| d % 7).collect();
    let start = after
        .with()
        .second(0)
        .subsec_nanosecond(0)
        .build()
        .ok()?
        .checked_add(1.minute())
        .ok()?;
    let either = fields[2] != "*" && fields[4] != "*"; // both restricted: a match on either field fires
    for offset in 0..366 * 5_i32 {
        let day: Date = start.date().checked_add(offset.days()).ok()?;
        if !parsed[3].contains(&i64::from(day.month())) {
            continue;
        }
        let in_day = parsed[2].contains(&i64::from(day.day()));
        let in_week = weekdays.contains(&i64::from(day.weekday().to_sunday_zero_offset()));
        if !(if either {
            in_day || in_week
        } else {
            in_day && in_week
        }) {
            continue;
        }
        for hour in &parsed[1] {
            for minute in &parsed[0] {
                let moment = day.at(*hour as i8, *minute as i8, 0, 0);
                if moment >= start {
                    return Some(moment);
                }
            }
        }
    }
    None
}

fn utc(seconds: i64) -> Option<String> {
    Timestamp::from_second(seconds)
        .ok()
        .map(|t| t.strftime("%Y-%m-%dT%H:%M:%SZ").to_string())
}

/// `str.split(None, max)`: whitespace runs split, the remainder keeps its inner spacing.
fn split_none(text: &str, max: usize) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut rest = text.trim_start();
    while !rest.is_empty() {
        if parts.len() == max {
            parts.push(rest);
            break;
        }
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        parts.push(&rest[..end]);
        rest = rest[end..].trim_start();
    }
    parts
}

fn cron_jobs(
    env: &ProcessEnv,
    now: DateTime,
    zone: &TimeZone,
) -> (Vec<Value>, usize, &'static str) {
    let Some(text) = run(env, &["crontab", "-l"]).ok().flatten() else {
        return (Vec::new(), 0, NO_DATA);
    };
    let (mut rows, mut hidden) = (Vec::new(), 0);
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || is_assignment(line) {
            continue;
        }
        let (schedule, command) = if line.starts_with('@') {
            let parts = split_none(line, 1);
            (
                parts[0].to_string(),
                parts.get(1).copied().unwrap_or("").to_string(),
            )
        } else {
            let parts = split_none(line, 5);
            let n = parts.len().min(5);
            (
                parts[..n].join(" "),
                parts.get(5).copied().unwrap_or("").to_string(),
            )
        };
        let Some(word) = command_name(&command) else {
            continue;
        };
        if !ALLOW.contains(&word.as_str()) {
            hidden += 1;
            continue;
        }
        let next = next_cron(&schedule, now)
            .and_then(|m| m.to_zoned(zone.clone()).ok())
            .and_then(|z| utc(z.timestamp().as_second()));
        rows.push(json!({
            "name": format!("(crontab) {word}"),
            "kind": "cron",
            "command": word,
            "schedule": schedule,
            "next_run": next,
            "last_run": null,
            "last_exit": null,
        }));
    }
    (rows, hidden, OK)
}

// ---- systemd ---------------------------------------------------------------------------------------------------

struct Timer {
    unit: String,
    activates: Option<String>,
    next: Option<i64>,
    last: Option<i64>,
}

/// A unit name that cannot pass as an option: `[A-Za-z0-9:_.@\][A-Za-z0-9:_.@\-]*`.
pub fn valid_unit(name: &str) -> bool {
    let ok = |c: char| c.is_ascii_alphanumeric() || ":_.@\\".contains(c);
    let mut chars = name.chars();
    chars.next().is_some_and(ok) && chars.all(|c| ok(c) || c == '-')
}

fn timers(env: &ProcessEnv) -> Option<Vec<Timer>> {
    let json_out = systemctl(
        env,
        &["list-timers", "--all", "--output=json", "--no-pager"],
    );
    if let Some(Value::Array(rows)) = json_out
        .as_deref()
        .and_then(|t| serde_json::from_str(t).ok())
    {
        let micros = |v: Option<&Value>| v.and_then(|v| v.as_i64());
        return Some(
            rows.iter()
                .filter_map(|r| {
                    let r = r.as_object()?;
                    Some(Timer {
                        unit: r.get("unit")?.as_str()?.to_string(),
                        activates: r
                            .get("activates")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        next: micros(r.get("next")),
                        last: micros(r.get("last")),
                    })
                })
                .collect(),
        );
    }
    // an older systemd ignores --output and prints the table
    let table = systemctl(env, &["list-timers", "--all", "--no-legend", "--no-pager"])?;
    let mut found = Vec::new();
    for line in table.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        let Some(at) = words.iter().rposition(|w| w.ends_with(".timer")) else {
            continue;
        };
        let service = words
            .get(at + 1)
            .filter(|w| w.ends_with(".service"))
            .map(|w| w.to_string());
        found.push(Timer {
            unit: words[at].to_string(),
            activates: service,
            next: None,
            last: None,
        });
    }
    Some(found)
}

/// One `systemctl show` call for every unit: `{Id: {property: value}}`, as pairs in the order read.
fn properties(env: &ProcessEnv, units: &[String]) -> Vec<(String, Vec<(String, String)>)> {
    if units.is_empty() {
        return Vec::new();
    }
    let mut args = vec![
        "show",
        "-p",
        "Id,TimersCalendar,TimersMonotonic,ExecStart,ExecMainStatus",
    ];
    args.extend(units.iter().map(String::as_str));
    let out = systemctl(env, &args).unwrap_or_default();
    let mut found: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for block in out.split("\n\n") {
        let props: Vec<(String, String)> = block
            .lines()
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let get = |k: &str| {
            props
                .iter()
                .rev()
                .find(|(pk, _)| pk == k)
                .map(|(_, v)| v.clone())
        };
        if let Some(id) = get("Id") {
            found.retain(|(i, _)| *i != id);
            found.push((id, props));
        }
    }
    found
}

fn prop<'a>(props: &'a [(String, String)], key: &str) -> &'a str {
    props
        .iter()
        .rev()
        .find(|(k, _)| k == key)
        .map_or("", |(_, v)| v.as_str())
}

/// The text up to the first `;` or `}` after `from`, trailing whitespace dropped (`([^;}]*?)\s*(?:;|\})`), with
/// the index after the terminator.
fn until_end(text: &str, from: usize) -> Option<(&str, usize)> {
    let end = text[from..].find([';', '}'])? + from;
    Some((text[from..end].trim_end(), end + 1))
}

fn schedule(props: &[(String, String)]) -> Option<String> {
    let (calendar_text, monotonic_text) = (
        prop(props, "TimersCalendar"),
        prop(props, "TimersMonotonic"),
    );
    let mut calendar = Vec::new();
    let mut at = 0;
    while let Some(found) = calendar_text[at..].find("OnCalendar=") {
        let from = at + found + "OnCalendar=".len();
        match until_end(calendar_text, from) {
            Some((value, next)) => {
                calendar.push(value.to_string());
                at = next;
            }
            None => break,
        }
    }
    let mut monotonic = Vec::new();
    let mut i = 0;
    while let Some(found) = monotonic_text[i..].find("On") {
        let start = i + found;
        let run = monotonic_text[start + 2..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .count();
        let word_end = start
            + 2
            + monotonic_text[start + 2..]
                .char_indices()
                .nth(run)
                .map_or(monotonic_text.len() - start - 2, |(b, _)| b);
        let word = &monotonic_text[start + 2..word_end];
        if monotonic_text[word_end..].starts_with('=')
            && word.ends_with("Sec")
            && word.len() > 3
            && let Some((value, next)) = until_end(monotonic_text, word_end + 1)
        {
            monotonic.push(format!("On{word}={value}"));
            i = next;
            continue;
        }
        i = start
            + monotonic_text[start..]
                .chars()
                .next()
                .map_or(1, char::len_utf8);
    }
    let many = calendar.len() > 1;
    let all: Vec<String> = calendar
        .into_iter()
        .map(|c| if many { format!("OnCalendar={c}") } else { c })
        .chain(monotonic)
        .collect();
    (!all.is_empty()).then(|| all.join(", "))
}

/// The command of the first `argv[]=... ; ignore_errors=` on one line of `ExecStart`.
fn exec_command(props: &[(String, String)]) -> Option<String> {
    let text = prop(props, "ExecStart");
    let mut from = 0;
    while let Some(found) = text[from..].find("argv[]=") {
        let start = from + found + "argv[]=".len();
        let line_end = text[start..].find('\n').map_or(text.len(), |n| start + n);
        if let Some(end) = text[start..line_end].find(" ; ignore_errors=") {
            return command_name(&text[start..start + end]);
        }
        from = start;
    }
    None
}

fn micro(value: Option<i64>) -> Option<String> {
    value.filter(|v| *v > 0).and_then(|v| utc(v / 1_000_000))
}

fn systemd_jobs(env: &ProcessEnv) -> (Vec<Value>, usize, &'static str) {
    let Some(timers) = timers(env) else {
        return (Vec::new(), 0, NO_DATA);
    };
    let timers: Vec<(Timer, Option<String>)> = timers
        .into_iter()
        .filter(|t| t.unit.ends_with(".timer") && valid_unit(&t.unit))
        .map(|t| {
            let service = t
                .activates
                .clone()
                .unwrap_or_else(|| format!("{}.service", &t.unit[..t.unit.len() - 6]));
            let service = valid_unit(&service).then_some(service);
            (t, service)
        })
        .collect();
    let units: Vec<String> = timers
        .iter()
        .flat_map(|(t, s)| std::iter::once(t.unit.clone()).chain(s.clone()))
        .collect();
    let props = properties(env, &units);
    let of = |id: &str| {
        props
            .iter()
            .find(|(i, _)| i == id)
            .map(|(_, p)| p.as_slice())
            .unwrap_or(&[])
    };
    let (mut rows, mut hidden) = (Vec::new(), 0);
    for (timer, service) in &timers {
        let service_props = of(service.as_deref().unwrap_or(""));
        let word = exec_command(service_props);
        let Some(word) = word.filter(|w| ALLOW.contains(&w.as_str())) else {
            hidden += 1;
            continue;
        };
        let last = micro(timer.last);
        let status = prop(service_props, "ExecMainStatus");
        let exit =
            (last.is_some() && !status.is_empty() && status.bytes().all(|b| b.is_ascii_digit()))
                .then(|| status.parse::<i64>().ok())
                .flatten();
        rows.push(json!({
            "name": timer.unit,
            "kind": "systemd",
            "command": word,
            "schedule": schedule(of(&timer.unit)),
            "next_run": micro(timer.next),
            "last_run": last,
            "last_exit": exit,
        }));
    }
    (rows, hidden, OK)
}

/// `{jobs, hidden, sources}`: agent jobs, the count of other jobs left out, and ok / no data per kind. `now_micros`
/// and `zone` give the local time the cron `next_run` is counted from.
pub fn list_jobs(env: &ProcessEnv, now_micros: i64, zone: &TimeZone) -> Value {
    let now = Timestamp::from_microsecond(now_micros)
        .map_or(DateTime::constant(1970, 1, 1, 0, 0, 0, 0), |t| {
            t.to_zoned(zone.clone()).datetime()
        });
    let (mut jobs, timer_hidden, timer_state) = systemd_jobs(env);
    let (cron_rows, cron_hidden, cron_state) = cron_jobs(env, now, zone);
    jobs.extend(cron_rows);
    json!({
        "jobs": jobs,
        "hidden": timer_hidden + cron_hidden,
        "sources": {"systemd": timer_state, "cron": cron_state},
    })
}

/// [`list_jobs`] over this process's environment and the machine's local zone: what the route runs.
pub fn list_jobs_here(now_micros: i64) -> Value {
    list_jobs(&ProcessEnv::inherited(), now_micros, &TimeZone::system())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    /// Python's `set(range(first, last + 1, step))` for a step no `i64` can add twice: one value, and no panic or wrap.
    #[test]
    fn a_step_too_big_to_add_stops_the_field_instead_of_overflowing() {
        let set = |v: &[i64]| Some(v.iter().copied().collect::<BTreeSet<i64>>());
        assert_eq!(field("1/9223372036854775807", 0, 59), set(&[1]));
        assert_eq!(field("*/9223372036854775807", 0, 59), set(&[0]));
        assert_eq!(field("0-59/9223372036854775807", 0, 59), set(&[0]));
        assert_eq!(field("5/99999999999999999999999", 0, 59), set(&[5])); // a Python int has no limit
        assert_eq!(field("*/100", 0, 59), set(&[0])); // a step wider than the range
        assert_eq!(field("*/20", 0, 59), set(&[0, 20, 40]));
        // Python refuses a step below 1, and an out-of-range or negative bound
        for bad in [
            "*/0",
            "*/-1",
            "5/0",
            "*/-9223372036854775808",
            "-1/2",
            "1/",
            "1/x",
        ] {
            assert_eq!(field(bad, 0, 59), None, "{bad}");
        }
        let after = date(2026, 11, 11).at(10, 0, 0, 0);
        for line in [
            "1/9223372036854775807 * * * *",
            "0 1/9223372036854775807 * * *",
            "1/99999999999999999999999 * * * *",
        ] {
            assert!(next_cron(line, after).is_some(), "{line}");
        }
        assert_eq!(next_cron("*/-1 * * * *", after), None);
    }

    #[test]
    fn only_the_read_only_calls_pass_the_allow_list() {
        assert!(is_read_only(&[
            "systemctl",
            "--user",
            "list-timers",
            "--all"
        ]));
        assert!(is_read_only(&[
            "systemctl",
            "--user",
            "show",
            "-p",
            "Id",
            "a.timer"
        ]));
        assert!(is_read_only(&["crontab", "-l"]));
        for bad in [
            &["systemctl", "list-timers"][..],
            &["systemctl", "--user", "start", "x.service"],
            &["systemctl", "--user"],
            &["crontab", "-r"],
            &["crontab", "-l", "-u", "root"],
            &["crontab"],
            &["rm", "-rf", "/"],
            &[],
        ] {
            assert!(!is_read_only(bad), "{bad:?}");
        }
        assert_eq!(
            run(&ProcessEnv::only_path(""), &["crontab", "-r"]),
            Err(NotReadOnly)
        );
    }

    #[test]
    fn unit_names_cannot_pass_as_options() {
        for ok in [
            "a.timer",
            "x@1.timer",
            "a-b.timer",
            "a\\x2db.timer",
            "0.timer",
        ] {
            assert!(valid_unit(ok), "{ok}");
        }
        for bad in ["", "-a.timer", "a b.timer", "--user", "a;b", "a\nb.timer"] {
            assert!(!valid_unit(bad), "{bad:?}");
        }
    }

    #[test]
    fn shlex_split_is_pythons() {
        assert_eq!(
            shlex_split(r#"a "b c" 'd e' f\ g"#).unwrap(),
            ["a", "b c", "d e", "f g"]
        );
        assert_eq!(shlex_split(r#"a"b c"d"#).unwrap(), ["ab cd"]);
        assert_eq!(shlex_split(r#""a\$b" "c\"d""#).unwrap(), [r"a\$b", "c\"d"]);
        assert!(shlex_split("a 'b").is_none());
        assert!(shlex_split("a\\").is_none());
        assert_eq!(words("a 'b"), ["a", "'b"]); // the fallback is a plain split
    }
}
