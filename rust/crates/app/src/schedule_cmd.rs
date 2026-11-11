//! `easy-personal-rs schedule install|remove` (P2c, T034): Python's `easy-personal schedule` (`core/schedule.py`,
//! `surfaces/cli.py` `_schedule`) for the systemd user timer that runs the daily ingest. The same unit files (the
//! templates are a copy of `src/easy_personal/data/systemd/*`; a test compares the bytes), the same unit names (so
//! installing from Rust replaces Python's units and the reverse: one scheduled ingest per machine), the same messages,
//! the same exit codes and the same `systemctl --user` calls in the same order.
//!
//! The one difference is on purpose: the service runs this binary (`<path of the running binary> ingest`), not
//! `easy-personal`. Where Python dies with a traceback (a failing `systemctl`, a folder it cannot write) this exits 1
//! with one line; no message carries a path or anything the tool printed (NFR-002). The store is never opened.

use clap::Subcommand;
use std::fs;
use std::io::Write;
use std::path::{Component, PathBuf};
use std::process::{Command, ExitCode};

const SERVICE: &str = "easy-personal-ingest.service";
const TIMER: &str = "easy-personal-ingest.timer";
const SERVICE_TEMPLATE: &str = include_str!("../units/easy-personal-ingest.service");
const TIMER_TEMPLATE: &str = include_str!("../units/easy-personal-ingest.timer");

#[derive(Subcommand)]
pub enum Action {
    /// Install and enable the daily timer
    Install {
        /// Passed to the scheduled ingest
        #[arg(long, value_name = "N", allow_negative_numbers = true, value_parser = clap::value_parser!(i64).range(1..))]
        retention_days: Option<i64>,
        /// Print the units, write nothing
        #[arg(long)]
        dry_run: bool,
    },
    /// Disable and delete the timer
    Remove {
        /// Print what would go, change nothing
        #[arg(long)]
        dry_run: bool,
    },
}

pub fn run(action: Action) -> ExitCode {
    match execute(action) {
        Ok(code) => code,
        Err(text) => {
            eprintln!("easy-personal-rs: {text}");
            ExitCode::FAILURE
        }
    }
}

fn execute(action: Action) -> Result<ExitCode, String> {
    match action {
        Action::Remove { dry_run } => {
            let dir = unit_dir()?;
            if dry_run {
                say(&format!(
                    "would disable {TIMER} and delete it and {SERVICE} from {}\n",
                    dir.display()
                ))?;
            } else {
                remove(&dir)?;
                say(&format!("removed {TIMER} from {}\n", dir.display()))?;
            }
            Ok(ExitCode::SUCCESS)
        }
        Action::Install {
            retention_days,
            dry_run,
        } => {
            let cmd = command()?;
            if dry_run {
                for (name, text) in render_units(&cmd, retention_days) {
                    say(&format!("# {name}\n{text}\n"))?;
                }
                return Ok(ExitCode::SUCCESS);
            }
            if !systemd_available() {
                say("no systemd user session found; nothing written. Use cron instead:\n")?;
                say(&format!(
                    "# add with: crontab -e\n17 3 * * * {}\n",
                    ingest_command(&cmd, retention_days)
                ))?;
                return Ok(ExitCode::FAILURE);
            }
            let dir = unit_dir()?;
            install(&dir, &cmd, retention_days)?;
            say(&format!("installed {TIMER} in {}\n", dir.display()))?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

/// `print(...)` that reports a closed stdout instead of panicking.
fn say(text: &str) -> Result<(), String> {
    let mut out = std::io::stdout().lock();
    out.write_all(text.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|_| "cannot write to stdout".to_string())
}

/// `XDG_CONFIG_HOME` (empty counts as unset, like Python's `or`), else `$HOME/.config`, then `systemd/user`. Python
/// reads the passwd entry when `HOME` is unset; like `ingest` this asks for the variable instead. A `.` part is
/// dropped as `Path` does (`./rel` is `rel`).
fn unit_dir() -> Result<PathBuf, String> {
    let non_empty = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    let base = match non_empty("XDG_CONFIG_HOME") {
        Some(base) => PathBuf::from(base),
        None => PathBuf::from(
            non_empty("HOME")
                .ok_or("cannot find the config folder: set XDG_CONFIG_HOME or HOME")?,
        )
        .join(".config"),
    };
    let kept: PathBuf = base
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect();
    Ok(kept.join("systemd").join("user"))
}

/// The running binary's absolute path, quoted for a shell. Python quotes `shutil.which("easy-personal")`; here it is
/// `current_exe`. A path that is not UTF-8 cannot be written to a unit file the way Python would write it: refused.
fn command() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|_| "cannot find the path of this program")?;
    let text = exe
        .to_str()
        .ok_or("the path of this program is not UTF-8; it cannot go in a unit file")?;
    Ok(shell_quote(text))
}

/// `shlex.quote`: bare when it is non-empty and only `[A-Za-z0-9_@%+=:,./-]`, else single-quoted with `'` as `'"'"'`.
fn shell_quote(text: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c);
    if !text.is_empty() && text.chars().all(safe) {
        text.to_string()
    } else {
        format!("'{}'", text.replace('\'', "'\"'\"'"))
    }
}

fn ingest_command(cmd: &str, retention_days: Option<i64>) -> String {
    match retention_days {
        Some(days) => format!("{cmd} ingest --retention-days {days}"),
        None => format!("{cmd} ingest"),
    }
}

/// The two unit files, service first (the order Python writes them). The service template's one placeholder is
/// `$exec_start` (Python's `string.Template.substitute`; the copy has no other `$`).
fn render_units(cmd: &str, retention_days: Option<i64>) -> [(&'static str, String); 2] {
    [
        (
            SERVICE,
            SERVICE_TEMPLATE.replace("$exec_start", &ingest_command(cmd, retention_days)),
        ),
        (TIMER, TIMER_TEMPLATE.to_string()),
    ]
}

/// `shutil.which("systemctl")` and `systemctl --user show-environment` succeeding: a user session exists.
fn systemd_available() -> bool {
    Command::new("systemctl")
        .args(["--user", "show-environment"])
        .output()
        .is_ok_and(|out| out.status.success())
}

/// `systemctl --user <args>`; a missing tool and a failing call are the same clean error (Python: a traceback).
fn systemctl(args: &[&str]) -> Result<(), String> {
    let done = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .is_ok_and(|out| out.status.success());
    if done {
        Ok(())
    } else {
        Err(format!("systemctl --user {} failed", args.join(" ")))
    }
}

fn install(dir: &std::path::Path, cmd: &str, retention_days: Option<i64>) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|_| "cannot create the systemd user folder")?;
    for (name, text) in render_units(cmd, retention_days) {
        fs::write(dir.join(name), text).map_err(|_| "cannot write a unit file")?;
    }
    systemctl(&["daemon-reload"])?;
    systemctl(&["enable", "--now", TIMER])
}

/// `disable --now` only when the timer file existed; the files go (a missing one is fine); `daemon-reload` only if it
/// had been installed.
fn remove(dir: &std::path::Path) -> Result<(), String> {
    let installed = dir.join(TIMER).exists();
    if installed {
        systemctl(&["disable", "--now", TIMER])?;
    }
    for name in [TIMER, SERVICE] {
        match fs::remove_file(dir.join(name)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("cannot delete a unit file".into()),
        }
    }
    if installed {
        systemctl(&["daemon-reload"])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting_is_shlex_quote() {
        for (given, want) in [
            ("/opt/bin/easy-personal-rs", "/opt/bin/easy-personal-rs"),
            ("a=b@c+d,e%f-g.h_i:j", "a=b@c+d,e%f-g.h_i:j"),
            ("", "''"),
            ("/a b/ep", "'/a b/ep'"),
            ("/it's/ep", "'/it'\"'\"'s/ep'"),
            ("/\u{fc}/ep", "'/\u{fc}/ep'"),
            ("/a$b/ep", "'/a$b/ep'"),
            ("/a\\b", "'/a\\b'"),
            ("/a\tb", "'/a\tb'"),
        ] {
            assert_eq!(shell_quote(given), want, "{given:?}");
        }
    }

    #[test]
    fn the_service_template_has_exactly_one_placeholder_and_no_other_dollar() {
        assert_eq!(SERVICE_TEMPLATE.matches('$').count(), 1);
        assert_eq!(SERVICE_TEMPLATE.matches("$exec_start").count(), 1);
        assert_eq!(TIMER_TEMPLATE.matches('$').count(), 0);
    }

    #[test]
    fn the_command_line_carries_the_retention_only_when_given() {
        let [(name, service), (timer_name, timer)] = render_units("/x/ep", None);
        assert_eq!((name, timer_name), (SERVICE, TIMER));
        assert!(service.contains("\nExecStart=/x/ep ingest\n"));
        assert_eq!(timer, TIMER_TEMPLATE);
        let [(_, with), _] = render_units("/x/ep", Some(90));
        assert!(with.contains("\nExecStart=/x/ep ingest --retention-days 90\n"));
    }

    #[test]
    fn a_dollar_in_the_command_is_not_expanded_again() {
        let [(_, service), _] = render_units("'/a$exec_start/ep'", None);
        assert!(service.contains("ExecStart='/a$exec_start/ep' ingest\n"));
    }
}
