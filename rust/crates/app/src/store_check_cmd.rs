//! `store-check [--json] [--full]` (T059, FR-091): the read-only health check of the store, as Python's `store-check`.
//!
//! The answer is `easy_personal_core::store_health::check`; this file only prints it and keeps the exit codes:
//! 0 healthy, 1 a problem found, 3 not fully checked, 130 for Ctrl-C. `--json` prints Python's
//! `json.dumps(indent=2, sort_keys=True)` byte for byte (every non-ASCII character as `\uXXXX`). Nothing is written, and
//! no answer carries an absolute path or a row's content. Ctrl-C stops every running read (the `cancel` flag of the check)
//! (SIGTERM counts as Ctrl-C; a second signal ends the process at once) and the command ends with one line on stderr, no partial answer and no panic text.

use easy_personal_core::store::store_path;
use easy_personal_core::store_health::{Options, check};
use serde_json::Value;
use std::io::Write;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub fn run(json: bool, full: bool) -> ExitCode {
    let path = match store_path() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("easy-personal-rs: {e}");
            return ExitCode::from(1);
        }
    };
    let cancel = Arc::new(AtomicBool::new(false));
    watch_ctrl_c(&cancel); // before the notice below: a Ctrl-C that follows the notice is already caught
    if full {
        eprintln!("full check: this can take minutes; Ctrl-C stops it");
    }
    let health = check(
        &path,
        &Options {
            full,
            ..Options::default()
        },
        &cancel,
    );
    if cancel.load(Ordering::SeqCst) {
        eprintln!("store-check: stopped");
        return ExitCode::from(130);
    }
    let text = if json {
        python_json(&health.json())
    } else {
        health.lines().join("\n") + "\n"
    };
    // a closed pipe is not an error of the check: no panic text
    let _ = std::io::stdout().write_all(text.as_bytes());
    let _ = std::io::stdout().flush();
    ExitCode::from(health.exit_code())
}

/// Sets `cancel` on the first SIGINT or SIGTERM (a stopped check ends with exit 130, and its private copy is removed as it
/// drops). A second signal ends the process at once with 130 (a private copy may stay: the next check sweeps it). Returns once
/// the handlers are installed (or could not be: then the signals act as they always do).
fn watch_ctrl_c(cancel: &Arc<AtomicBool>) {
    let flag = Arc::clone(cancel);
    let (ready, installed) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            let _ = ready.send(());
            return;
        };
        runtime.block_on(async {
            use tokio::signal::unix::{SignalKind, signal};
            let handlers = (
                signal(SignalKind::interrupt()),
                signal(SignalKind::terminate()),
            );
            let _ = ready.send(());
            if let (Ok(mut int), Ok(mut term)) = handlers {
                tokio::select! { _ = int.recv() => {}, _ = term.recv() => {} }
                flag.store(true, Ordering::SeqCst);
                tokio::select! { _ = int.recv() => {}, _ = term.recv() => {} }
                std::process::exit(130);
            }
        });
    });
    let _ = installed.recv_timeout(Duration::from_secs(2));
}

/// Python's `json.dumps(value, indent=2, sort_keys=True)` + newline: serde_json's pretty form (sorted keys, two spaces),
/// then every character outside `' '..='~'` as `\uXXXX` (a surrogate pair above U+FFFF), like `ensure_ascii`.
fn python_json(value: &Value) -> String {
    let pretty = serde_json::to_string_pretty(value).unwrap_or_default();
    let mut out = String::with_capacity(pretty.len() + 1);
    for c in pretty.chars() {
        if c == '\n' || (' '..='~').contains(&c) {
            out.push(c);
        } else {
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::python_json;
    use serde_json::json;

    #[test]
    fn non_ascii_becomes_escapes_like_python() {
        let value = json!({"b": ["caf\u{e9} \u{2615} \u{1F600}", "\u{7f}"], "a": []});
        assert_eq!(
            python_json(&value),
            "{\n  \"a\": [],\n  \"b\": [\n    \"caf\\u00e9 \\u2615 \\ud83d\\ude00\",\n    \"\\u007f\"\n  ]\n}\n"
        );
    }
}
