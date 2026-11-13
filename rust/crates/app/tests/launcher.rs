//! T050 AC3 / AC11 / NFR-006: the `easy-personal-launch` binary over a fake keychain (the debug-build
//! `EASY_PERSONAL_TEST_KEYCHAIN` file) and a fake provider script. The environment is built from nothing, and the real
//! Secret Service is out of reach: `DBUS_SESSION_BUS_ADDRESS` points at a socket that does not exist.
//! Expected texts are Python's (`src/easy_personal/launch.py`).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

const LAUNCH: &str = env!("CARGO_BIN_EXE_easy-personal-launch");
const SECRET: &str = "xoxb-PLANTED-t050-\u{e9}-secret";
const NO_BACKEND: &str = "easy-personal: no usable OS keychain (Secret Service); unlock or install one, e.g. gnome-keyring. Secrets are never stored in plaintext\n";

struct Dir(PathBuf);

impl Dir {
    fn new() -> Dir {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t050-launch-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Dir(dir)
    }

    fn keychain(&self) -> PathBuf {
        self.0.join("keychain.json")
    }

    /// One item as Python's `keychain.store` writes it (`json.dumps`).
    fn item(&self, name: &str, raw: &str) {
        let mut map = std::fs::read_to_string(self.keychain())
            .ok()
            .and_then(|t| {
                serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&t).ok()
            })
            .unwrap_or_default();
        map.insert(format!("easy-personal/{name}"), raw.into());
        std::fs::write(self.keychain(), serde_json::Value::Object(map).to_string()).unwrap();
    }

    fn script(&self, name: &str, body: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();
        path
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn launcher(keychain: Option<&Path>, args: &[&str]) -> Command {
    let mut cmd = Command::new(LAUNCH);
    cmd.args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env(
            "DBUS_SESSION_BUS_ADDRESS",
            "unix:path=/nonexistent/t050-no-bus",
        )
        .env("KEEP_ME", "inherited");
    if let Some(file) = keychain {
        cmd.env("EASY_PERSONAL_TEST_KEYCHAIN", file);
    }
    cmd
}

fn run(cmd: &mut Command, stdin: &[u8]) -> Output {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin).unwrap();
    child.wait_with_output().unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn item(command: &[&str], env: &[(&str, &str)], headers: &[(&str, &str)]) -> String {
    let pairs = |p: &[(&str, &str)]| {
        p.iter()
            .map(|(k, v)| format!("{}: {}", serde_json::json!(k), serde_json::json!(v)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "{{\"command\": {}, \"env\": {{{}}}, \"headers\": {{{}}}}}",
        serde_json::json!(command),
        pairs(env),
        pairs(headers)
    )
}

#[test]
fn exec_puts_the_secret_in_the_providers_env_never_its_argv_and_forwards_io_and_exit_code() {
    let dir = Dir::new();
    // the provider: its own argv as the kernel shows it to `ps`, whether the env holds the secret, stdin echoed, an
    // error line, and its own exit code
    let provider = dir.script(
        "provider",
        "tr '\\0' ' ' < /proc/$$/cmdline; echo\n\
         [ -n \"$SLACK_TOKEN\" ] && [ \"$SLACK_TOKEN\" = \"$EXPECT\" ] && echo env-ok\n\
         echo \"keep=$KEEP_ME\"\n\
         cat\n\
         echo to-stderr >&2\n\
         exit 7",
    );
    let p = provider.to_str().unwrap();
    dir.item(
        "slack",
        &item(
            &[p, "--serve", "x y"],
            &[("SLACK_TOKEN", SECRET), ("EXPECT", SECRET)],
            &[],
        ),
    );
    let out = run(
        launcher(Some(&dir.keychain()), &["exec", "slack"]).arg0("easy-personal-launch"),
        b"from-stdin\n",
    );
    let stdout = text(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(7),
        "the provider's exit code: {stdout}"
    );
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines[0].trim_end(),
        format!("/bin/sh {p} --serve x y"),
        "argv is the item's command"
    );
    assert!(
        !lines[0].contains("PLANTED"),
        "a secret in the provider's argv"
    );
    assert_eq!(&lines[1..], ["env-ok", "keep=inherited", "from-stdin"]);
    assert_eq!(
        text(&out.stderr),
        "to-stderr\n",
        "stderr is the provider's, nothing of the launcher"
    );
}

trait Arg0 {
    fn arg0(&mut self, name: &str) -> &mut Self;
}

impl Arg0 for Command {
    fn arg0(&mut self, name: &str) -> &mut Self {
        std::os::unix::process::CommandExt::arg0(self, name)
    }
}

#[test]
fn headers_prints_one_json_line_as_python() {
    let dir = Dir::new();
    dir.item("slack", &item(&["x"], &[], &[("Authorization", SECRET)]));
    let out = run(
        &mut launcher(Some(&dir.keychain()), &["headers", "slack"]),
        b"",
    );
    assert_eq!(out.status.code(), Some(0));
    // Python: json.dumps({"Authorization": SECRET}) + "\n" (the e-acute as \\u00e9)
    assert_eq!(
        text(&out.stdout),
        "{\"Authorization\": \"xoxb-PLANTED-t050-\\u00e9-secret\"}\n"
    );
    assert_eq!(out.stderr, b"");
}

#[test]
fn refusals_have_pythons_fixed_texts_and_exit_one_and_never_show_a_secret() {
    let dir = Dir::new();
    dir.item("noprov", &item(&[], &[("T", SECRET)], &[]));
    dir.item(
        "missing",
        &item(&["definitely-not-on-path-t050"], &[("T", SECRET)], &[]),
    );
    dir.item("broken", &format!("{{\"command\": \"{SECRET}\"}}"));
    dir.item("badenv", &item(&["/bin/true"], &[("A=B", SECRET)], &[]));
    let kc = Some(dir.keychain());
    let cases: [(&[&str], Option<&Path>, String); 8] = [
        (
            &["exec", "nope"],
            kc.as_deref(),
            "easy-personal: connector 'nope' is not configured\n".into(),
        ),
        (
            &["exec", "noprov"],
            kc.as_deref(),
            "easy-personal: connector 'noprov' has no provider command\n".into(),
        ),
        (
            &["exec", "missing"],
            kc.as_deref(),
            "easy-personal: cannot start provider command for 'missing'\n".into(),
        ),
        (
            &["exec", "badenv"],
            kc.as_deref(),
            "easy-personal: cannot start provider command for 'badenv'\n".into(),
        ),
        (
            &["exec", "broken"],
            kc.as_deref(),
            "easy-personal: connector 'broken' has an unreadable keychain entry\n".into(),
        ),
        (
            &["headers", "broken"],
            kc.as_deref(),
            "easy-personal: connector 'broken' has an unreadable keychain entry\n".into(),
        ),
        (
            &["exec", "Bad/../x"],
            kc.as_deref(),
            "easy-personal: invalid connector name\n".into(),
        ),
        (
            &["exec", "slack"],
            Some(Path::new("unusable")),
            NO_BACKEND.into(),
        ),
    ];
    for (args, keychain, want) in cases {
        let out = run(&mut launcher(keychain, args), b"");
        assert_eq!(
            (out.status.code(), text(&out.stderr)),
            (Some(1), want),
            "{args:?}"
        );
        assert_eq!(out.stdout, b"", "{args:?}");
        assert!(!text(&out.stderr).contains("PLANTED"));
    }
}

#[test]
fn bad_usage_is_exit_two() {
    for args in [
        &[][..],
        &["exec"],
        &["run", "slack"],
        &["exec", "slack", "extra"],
        &["headers"],
    ] {
        let out = run(&mut launcher(None, args), b"");
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert_eq!(
            text(&out.stderr),
            "usage: easy-personal-launch exec|headers <connector>\n"
        );
    }
}

/// The real backend (no test switch) with no session bus: Python's refusal, nothing started. This is the release
/// build's path; no keychain exists to be touched.
#[test]
fn with_no_secret_service_the_real_backend_refuses() {
    for action in ["exec", "headers"] {
        let out = run(&mut launcher(None, &[action, "slack"]), b"");
        assert_eq!(
            (out.status.code(), text(&out.stderr)),
            (Some(1), NO_BACKEND.to_string()),
            "{action}"
        );
    }
}

/// NFR-006: the launcher adds less than 200 ms before the provider runs (best of 5, debug build, fake keychain).
#[test]
fn exec_adds_less_than_200_ms() {
    let dir = Dir::new();
    dir.item("slack", &item(&["/bin/true"], &[("T", SECRET)], &[]));
    let time = |cmd: &mut Command| {
        (0..5)
            .map(|_| {
                let start = Instant::now();
                assert!(cmd.status().unwrap().success());
                start.elapsed()
            })
            .min()
            .unwrap()
    };
    let direct = time(Command::new("/bin/true").env_clear());
    let launched = time(&mut launcher(Some(&dir.keychain()), &["exec", "slack"]));
    let added = launched.saturating_sub(direct);
    println!("launcher overhead: {added:?} (direct {direct:?}, launched {launched:?})");
    assert!(added < Duration::from_millis(200), "{added:?}");
}
