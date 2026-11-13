//! `easy-personal-launch exec|headers <connector>` (T050, FR-018, NFR-006): Python's `launch.py`. Claude Code runs it
//! as a connector's MCP command (`claude mcp add-json` points at it). `exec` reads the connector's keychain item and
//! replaces this process with the provider command, the item's `env` added to this environment: a secret reaches the
//! provider only through its environment, never its argv. stdin, stdout, stderr, signals and the exit code are the
//! provider's own (`exec`, like Python's `os.execvpe`), so nothing is copied and nothing lingers. `headers` prints the
//! item's `headers` as one JSON line.
//!
//! The command comes only from the keychain item; nothing on the command line or in the environment chooses it. Every
//! message is one of Python's fixed texts on stderr, prefixed `easy-personal: `; none holds a secret. A separate binary
//! that uses only `keychain.rs` (std, `serde_json`, `keyring`, the core's `pyjson` text helpers): no server, no store.

#[path = "keychain.rs"]
#[allow(dead_code)] // the launcher reads items; it never stores or removes them
mod keychain;

use keychain::{Keychain, LaunchError};
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::process::{Command, ExitCode};

const USAGE: &str = "usage: easy-personal-launch exec|headers <connector>";

fn main() -> ExitCode {
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let action = args.first().and_then(|a| a.to_str());
    if args.len() != 2 || !matches!(action, Some("exec" | "headers")) {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    // a name that is not UTF-8 breaks the name rule like any other bad name
    let name = args[1].to_str().unwrap_or("\u{fffd}");
    let kc = keychain::system();
    let done = if action == Some("exec") {
        run_exec(&*kc, name).map(|never| match never {})
    } else {
        run_headers(&*kc, name)
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("easy-personal: {e}");
            ExitCode::from(1)
        }
    }
}

/// `run_exec`: returns only when the provider could not be started.
fn run_exec(kc: &dyn Keychain, name: &str) -> Result<std::convert::Infallible, LaunchError> {
    let item = keychain::load(kc, name)?;
    let Some((program, args)) = item.command.split_first() else {
        return Err(LaunchError(format!(
            "connector '{name}' has no provider command"
        )));
    };
    let cannot = || LaunchError(format!("cannot start provider command for '{name}'"));
    // Python's `execvpe` refuses an empty name or one with `=` (`ValueError`); `Command` would pass it on
    if item
        .env
        .iter()
        .any(|(k, _)| k.is_empty() || k.contains('='))
    {
        return Err(cannot());
    }
    let _failed = Command::new(program).args(args).envs(item.env).exec();
    Err(cannot())
}

/// `run_headers`: `json.dumps(headers)` and a newline on stdout.
fn run_headers(kc: &dyn Keychain, name: &str) -> Result<(), LaunchError> {
    let item = keychain::load(kc, name)?;
    let line = format!("{}\n", keychain::py_object(&item.headers));
    let mut out = std::io::stdout().lock();
    // a closed stdout is not a launcher error worth a message: the exit code says it
    out.write_all(line.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|_| LaunchError("cannot write the headers".into()))
}
