//! T034: `easy-personal-rs schedule install|remove` through the real binary, against the oracle goldens of the real
//! Python scheduler (`rust/parity/gen_schedule.py`, `rust/parity/schedule/golden/`).
//!
//! Host safety (never the real systemd, never the real `~/.config`): every run is in a throwaway case folder under
//! cargo's own `target/tmp`; the child's environment is built from nothing (`HOME`, `XDG_CONFIG_HOME` and
//! `EASY_PERSONAL_HOME` inside the case) and its `PATH` is one folder of the case, which holds at most the fake
//! `systemctl` (`rust/parity/schedule/fake-systemctl.sh`). `command` refuses to build a child that breaks that rule.
//!
//! The Python side regenerated: `rust/parity/.venv/bin/python rust/parity/gen_schedule.py --check`

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_easy-personal-rs");
const FAKE_SYSTEMCTL: &str = include_str!("../../../parity/schedule/fake-systemctl.sh");
const SERVICE: &str = "easy-personal-ingest.service";
const TIMER: &str = "easy-personal-ingest.timer";
const SNAP: [&str; 4] = ["xdg", "rel", "data", "home"];

fn golden(id: &str) -> Value {
    let path = format!(
        "{}/../../parity/schedule/golden/{id}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

/// A throwaway case: `home/`, `bin/` (with the binary as `easy-personal`), `data/`, `log/`. Removed on drop.
struct Case {
    root: PathBuf,
    cmd_dir: String,
}

/// A hard link keeps `current_exe` equal to this path (a symlink would resolve to the real binary); it needs the same
/// filesystem, which `target/tmp` shares with the binary. A copy is the fallback.
fn link_or_copy(from: &Path, to: &Path) {
    let _ = fs::remove_file(to);
    if fs::hard_link(from, to).is_err() {
        fs::copy(from, to).unwrap();
    }
}

fn place_binary(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    link_or_copy(Path::new(BIN), &dir.join("easy-personal"));
}

/// The fake `systemctl`, written once per test process and then only linked (a file that is being written must not be
/// executed: the kernel answers "text file busy").
fn fake_systemctl() -> PathBuf {
    static FAKE: OnceLock<PathBuf> = OnceLock::new();
    FAKE.get_or_init(|| {
        let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("t034-fake-systemctl-{}", std::process::id()));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, FAKE_SYSTEMCTL).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    })
    .clone()
}

impl Case {
    fn new() -> Case {
        static N: AtomicUsize = AtomicUsize::new(0);
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "t034-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let text = root.to_str().unwrap();
        assert!(
            text.chars()
                .all(|c| c.is_ascii_alphanumeric() || "@%+=:,./-_".contains(c)),
            "the case folder path must need no shell quoting, or the quoting goldens cannot compare: {text}"
        );
        let _ = fs::remove_dir_all(&root);
        for name in ["home", "data", "log"] {
            fs::create_dir_all(root.join(name)).unwrap();
        }
        place_binary(&root.join("bin"));
        Case {
            root,
            cmd_dir: "bin".into(),
        }
    }

    /// The only way a child is made. Refuses anything that could reach the real machine.
    fn command(&self, exe: &Path, cmd_dir: &str) -> Command {
        let path = self.root.join(cmd_dir);
        assert!(
            path.starts_with(&self.root) && exe.starts_with(&self.root),
            "PATH and the binary must be inside the case"
        );
        let mut c = Command::new(exe);
        c.current_dir(&self.root)
            .env_clear()
            .env("PATH", &path)
            .env("HOME", self.root.join("home"))
            .env("XDG_CONFIG_HOME", self.root.join("xdg"))
            .env("EASY_PERSONAL_HOME", self.root.join("data"))
            .env("FAKE_SYSTEMCTL_LOG", self.root.join("log").join("calls"));
        c
    }

    fn exe(&self) -> PathBuf {
        self.root.join(&self.cmd_dir).join("easy-personal")
    }

    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.root.join("log").join("calls"))
            .unwrap_or_default()
            .lines()
            .map(String::from)
            .collect()
    }

    fn tok(&self, text: &str, cmd: bool) -> String {
        let root = self.root.to_str().unwrap();
        let text = if cmd {
            text.replace(&format!("{root}/bin/easy-personal"), "<CMD>")
        } else {
            text.to_string()
        };
        text.replace(root, "<CASE>")
    }

    /// One step the oracle did between two runs.
    fn apply(&self, step: &Value) {
        let text = |key: &str| step[key].as_str().unwrap();
        let path = || self.root.join(text("path"));
        match text("op") {
            "systemctl" => {
                let target = self.root.join("bin").join("systemctl");
                if step["present"].as_bool().unwrap() {
                    link_or_copy(&fake_systemctl(), &target);
                } else {
                    let _ = fs::remove_file(&target);
                }
            }
            "put" => {
                fs::create_dir_all(path().parent().unwrap()).unwrap();
                fs::write(path(), text("text")).unwrap();
            }
            "mkdir" => fs::create_dir_all(path()).unwrap(),
            "rm" => {
                if path().is_dir() {
                    fs::remove_dir(path()).unwrap()
                } else {
                    fs::remove_file(path()).unwrap()
                }
            }
            "stub" => place_binary(&self.root.join(text("dir"))),
            other => panic!("unknown step {other}"),
        }
    }

    /// What the case holds now, as the oracle records it: every file (content tokenised) and folder under the
    /// watched tops.
    fn snapshot(&self, cmd: bool) -> (BTreeMap<String, String>, BTreeSet<String>) {
        fn walk(
            case: &Case,
            dir: &Path,
            cmd: bool,
            files: &mut BTreeMap<String, String>,
            dirs: &mut BTreeSet<String>,
        ) {
            let Ok(entries) = fs::read_dir(dir) else {
                return;
            };
            for entry in entries {
                let path = entry.unwrap().path();
                let rel = path.strip_prefix(&case.root).unwrap().to_str().unwrap();
                if path.is_dir() {
                    dirs.insert(rel.to_string());
                    walk(case, &path, cmd, files, dirs);
                } else {
                    let text = fs::read_to_string(&path).unwrap();
                    files.insert(rel.to_string(), case.tok(&text, cmd));
                }
            }
        }
        let (mut files, mut dirs) = (BTreeMap::new(), BTreeSet::new());
        for top in SNAP {
            let path = self.root.join(top);
            if path.exists() && top != "home" && top != "data" {
                dirs.insert(top.to_string());
            }
            walk(self, &path, cmd, &mut files, &mut dirs);
        }
        (files, dirs)
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn s(v: &Value) -> &str {
    v.as_str().unwrap()
}

/// Replay one oracle scenario through the binary and compare every run.
fn replay(id: &str) {
    let g = golden(id);
    let mut case = Case::new();
    for (i, run) in g["runs"].as_array().unwrap().iter().enumerate() {
        let what = format!("{id} run {i} ({})", s(&run["label"]));
        for step in run["before"].as_array().unwrap() {
            case.apply(step);
        }
        // The command is the stub's path in every scenario but Q01, whose runs name the folder the binary is found in.
        let cmd = id != "Q01_quoting";
        case.cmd_dir = s(&run["cmd_dir"]).to_string();
        let mut c = case.command(&case.exe(), &case.cmd_dir);
        c.args(run["argv"].as_array().unwrap().iter().skip(1).map(s));
        for (k, v) in run["env_extra"].as_object().unwrap() {
            match v.as_str() {
                Some(value) => c.env(k, value),
                None => c.env_remove(k),
            };
        }
        if let Some(verb) = run["fake_systemctl"]["fail"].as_str() {
            c.env("FAKE_SYSTEMCTL_FAIL", verb);
        }
        if run["fake_systemctl"]["no_session"].as_bool().unwrap() {
            c.env("FAKE_SYSTEMCTL_NO_SESSION", "1");
        }
        let _ = fs::write(case.root.join("log").join("calls"), "");
        let out = c.output().unwrap();
        let (stdout, stderr) = (
            String::from_utf8(out.stdout).unwrap(),
            String::from_utf8(out.stderr).unwrap(),
        );
        assert_eq!(
            out.status.code(),
            run["exit_code"].as_i64().map(|n| n as i32),
            "{what}: exit code; stderr: {stderr}"
        );
        assert_eq!(case.tok(&stdout, cmd), s(&run["stdout"]), "{what}: stdout");
        let kind = s(&run["stderr"]);
        if kind == "empty" {
            assert_eq!(stderr, "", "{what}: stderr");
        } else if kind.starts_with("usage error") {
            // clap: `error: ...`, or the help text when no action is given; either way on stderr, never stdout.
            assert!(!stderr.is_empty(), "{what}: stderr");
            assert_eq!(stdout, "", "{what}: stdout");
        } else {
            // Python's traceback; Rust: one clean line with no path and none of the tool's output.
            assert!(kind.starts_with("traceback: "), "{what}: {kind}");
            assert_eq!(stderr.lines().count(), 1, "{what}: {stderr}");
            assert!(stderr.starts_with("easy-personal-rs: "), "{what}: {stderr}");
            for leak in [
                case.root.to_str().unwrap(),
                "FAKE-TOOL-OUTPUT",
                "/home/",
                "/tmp",
            ] {
                assert!(
                    !stderr.contains(leak),
                    "{what}: stderr leaks {leak}: {stderr}"
                );
            }
        }
        assert!(
            !stdout.contains("FAKE-TOOL-OUTPUT"),
            "{what}: the tool's output reached stdout"
        );
        let (files, dirs) = case.snapshot(cmd);
        let want_files: BTreeMap<String, String> = run["files"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), s(v).to_string()))
            .collect();
        let want_dirs: BTreeSet<String> = run["dirs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| s(v).to_string())
            .collect();
        assert_eq!(files, want_files, "{what}: files");
        assert_eq!(dirs, want_dirs, "{what}: folders");
        let calls: Vec<String> = run["systemctl_calls"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| s(v).to_string())
            .collect();
        assert_eq!(case.calls(), calls, "{what}: systemctl calls");
    }
}

// -- the oracle replay (criteria 1, 3, 4, 5)

#[test]
fn dry_runs_print_and_write_nothing() {
    replay("D01_dry_run");
}

#[test]
fn install_and_remove_with_the_fake_systemctl() {
    replay("I01_install_remove");
}

#[test]
fn remove_runs_systemctl_only_when_the_timer_file_existed() {
    replay("R01_remove_partial");
}

#[test]
fn no_systemctl_prints_the_cron_line_and_writes_nothing() {
    replay("N01_no_systemctl");
}

#[test]
fn no_user_session_prints_the_cron_line_and_writes_nothing() {
    replay("N02_no_session");
}

#[test]
fn a_failing_install_call_is_one_clean_line_with_python_files_and_calls() {
    replay("F01_install_fails");
}

#[test]
fn a_failing_remove_call_is_one_clean_line_with_python_files_and_calls() {
    replay("F02_remove_fails");
}

#[test]
fn an_unwritable_folder_is_one_clean_line() {
    replay("W01_unwritable");
}

#[test]
fn the_unit_folder_follows_xdg_config_home_then_home() {
    replay("X01_xdg");
}

#[test]
fn bad_arguments_exit_2_and_write_nothing() {
    replay("U01_usage_errors");
}

#[test]
fn the_command_is_quoted_like_shlex_quote() {
    replay("Q01_quoting");
}

// -- the one deliberate difference (criterion 3) and the units (criterion 2)

#[test]
fn execstart_runs_the_rust_binary_with_its_own_path() {
    let case = Case::new();
    let out = case
        .command(&case.exe(), "bin")
        .args(["schedule", "install", "--retention-days", "30", "--dry-run"])
        .output()
        .unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let want = format!(
        "ExecStart={}/bin/easy-personal ingest --retention-days 30\n",
        case.root.display()
    );
    assert!(text.contains(&want), "{text}");
    assert!(
        !text.contains("easy-personal-rs"),
        "the file name here is the stub's: {text}"
    );
}

#[test]
fn the_embedded_unit_templates_are_byte_for_byte_pythons() {
    let python =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../src/easy_personal/data/systemd");
    for (name, ours) in [
        (
            SERVICE,
            include_bytes!("../units/easy-personal-ingest.service").as_slice(),
        ),
        (
            TIMER,
            include_bytes!("../units/easy-personal-ingest.timer").as_slice(),
        ),
    ] {
        let theirs = fs::read(python.join(name)).unwrap_or_else(|_| {
            panic!("the Python template {name} is missing: the copy has no reference")
        });
        assert_eq!(
            ours,
            theirs.as_slice(),
            "{name} differs from the Python file; copy it again"
        );
    }
}

#[test]
fn a_binary_path_that_is_not_utf8_is_refused_not_mangled() {
    let case = Case::new();
    let odd = case.root.join(std::ffi::OsStr::from_bytes(b"odd-\xff-dir"));
    place_binary(&odd);
    let out = case
        .command(&odd.join("easy-personal"), "bin")
        .args(["schedule", "install", "--dry-run"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"");
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.starts_with("easy-personal-rs: ") && err.lines().count() == 1,
        "{err}"
    );
}

#[test]
fn without_xdg_or_home_the_unit_folder_cannot_be_found() {
    let case = Case::new();
    let out = case
        .command(&case.exe(), "bin")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("HOME")
        .args(["schedule", "remove", "--dry-run"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.starts_with("easy-personal-rs: ") && err.lines().count() == 1,
        "{err}"
    );
}

#[test]
fn a_retention_beyond_i64_is_a_usage_error() {
    let case = Case::new();
    let out = case
        .command(&case.exe(), "bin")
        .args([
            "schedule",
            "install",
            "--retention-days",
            "9223372036854775808",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

// -- host safety (criterion 6) and the help text (criterion 7)

#[test]
fn a_real_install_writes_only_inside_the_case_and_calls_only_the_fake() {
    let case = Case::new();
    case.apply(&serde_json::json!({"op": "systemctl", "present": true}));
    let before: Vec<String> = fs::read_dir(&case.root)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    let out = case
        .command(&case.exe(), "bin")
        .args(["schedule", "install"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    // The fake logged all three calls: it is the only `systemctl` that could run (the PATH is `bin/` alone).
    assert_eq!(case.calls().len(), 3, "{:?}", case.calls());
    let mut after: Vec<String> = fs::read_dir(&case.root)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    after.sort();
    let mut want = before;
    want.push("xdg".into());
    want.sort();
    assert_eq!(
        after, want,
        "something was created beside the case's xdg folder"
    );
    assert!(
        fs::read_dir(case.root.join("data"))
            .unwrap()
            .next()
            .is_none()
            && fs::read_dir(case.root.join("home"))
                .unwrap()
                .next()
                .is_none(),
        "the data folder and HOME stay empty: the command never opens the store"
    );
    assert!(case.root.join("xdg/systemd/user").join(SERVICE).is_file());
    assert!(
        !include_str!("../src/schedule_cmd.rs").contains(".easy-personal"),
        "the scheduler never names the data folder"
    );
}

#[test]
fn help_lists_schedule_and_its_two_actions() {
    let case = Case::new();
    let top = case
        .command(&case.exe(), "bin")
        .arg("--help")
        .output()
        .unwrap();
    assert!(
        String::from_utf8(top.stdout)
            .unwrap()
            .contains("  schedule ")
    );
    let sub = case
        .command(&case.exe(), "bin")
        .args(["schedule", "--help"])
        .output()
        .unwrap();
    let text = String::from_utf8(sub.stdout).unwrap();
    assert!(
        text.contains("  install ") && text.contains("  remove "),
        "{text}"
    );
}
