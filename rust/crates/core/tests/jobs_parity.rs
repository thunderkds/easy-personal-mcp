//! T044: `jobs::list_jobs` against Python's own answers (`rust/parity/golden-jobs/jobs.json`: Python's `list_jobs` with the
//! fake `systemctl` / `crontab` of `tests/fakes/jobs_fakes.py` alone on `PATH`, frozen at 2026-11-11 10:00 local, `TZ=UTC`),
//! and the cases a golden cannot hold: a missing tool, a fake that hangs (cut at 3 s), one that prints 50 MB (capped), a
//! crontab of 10,000 lines, the zone, and that nothing but the command's first word leaves.
//!
//! Every process here is a fake: a `/bin/sh` script on a scratch `PATH` (the child's environment is cleared by the
//! module). The real `systemctl`, `crontab`, `claude` and keychain are never started.

use easy_personal_core::jobs::{ALLOW, NotReadOnly, ProcessEnv, list_jobs};
use jiff::civil::date;
use jiff::tz::TimeZone;
use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

fn golden() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/golden-jobs/jobs.json");
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn scratch(tag: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "easy-personal-rs-t044-{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir_all(dir.join("bin")).unwrap();
    fs::canonicalize(&dir).unwrap()
}

fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

fn write_script(bin: &Path, name: &str, body: &str) {
    let path = bin.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// A fake `tool` that logs `tool args` to `log` and answers each recorded call with its recorded stdout and exit code
/// (shell builtins only; any other argument list exits 3, as `jobs_fakes.py` does).
fn replay_fake(dir: &Path, tool: &str, calls: &[&Value], log: &Path) {
    let mut script = format!(
        "printf '%s\\n' \"{tool} $*\" >> {}\ncase \"$*\" in\n",
        quote(log.to_str().unwrap())
    );
    for (i, call) in calls.iter().enumerate() {
        let argv: Vec<&str> = call["argv"].as_array().unwrap()[1..]
            .iter()
            .map(|a| a.as_str().unwrap())
            .collect();
        let out = dir.join(format!("{tool}-{i}.out"));
        fs::write(&out, call["stdout"].as_str().unwrap()).unwrap();
        script += &format!(
            "  {}) while IFS= read -r l || [ -n \"$l\" ]; do printf '%s\\n' \"$l\"; done < {}; exit {};;\n",
            quote(&argv.join(" ")),
            quote(out.to_str().unwrap()),
            call["exit"]
        );
    }
    script += "esac\nexit 3\n";
    write_script(&dir.join("bin"), tool, &script);
}

fn now() -> i64 {
    date(2026, 11, 11)
        .at(10, 0, 0, 0)
        .to_zoned(TimeZone::UTC)
        .unwrap()
        .timestamp()
        .as_microsecond()
}

#[test]
fn every_jobs_case_equals_python_and_asks_only_the_same_read_only_calls() {
    let golden = golden();
    assert_eq!(golden["now"], json!([2026, 11, 11, 10, 0, 0]));
    let (mut equal, mut total) = (0, 0);
    for case in golden["cases"].as_array().unwrap() {
        let name = case["case"].as_str().unwrap();
        let dir = scratch(name);
        let log = dir.join("argv.log");
        fs::write(&log, "").unwrap();
        for tool in ["systemctl", "crontab"] {
            if case["tools"][tool] == true {
                let calls: Vec<&Value> = case["calls"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|c| c["argv"][0] == tool)
                    .collect();
                replay_fake(&dir, tool, &calls, &log);
            }
        }
        total += 1;
        let got = list_jobs(
            &ProcessEnv::only_path(dir.join("bin")),
            now(),
            &TimeZone::UTC,
        );
        assert_eq!(got, case["body"], "{name}");
        let asked: Vec<String> = fs::read_to_string(&log)
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect();
        let want: Vec<String> = case["calls"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                c["argv"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|a| a.as_str().unwrap())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        assert_eq!(asked, want, "{name}: argv");
        for line in &asked {
            assert!(
                line.starts_with("systemctl --user list-timers")
                    || line.starts_with("systemctl --user show -p")
                    || line == "crontab -l",
                "{line}"
            );
        }
        equal += 1;
        let _ = fs::remove_dir_all(&dir);
    }
    println!("jobs oracle equal/total: {equal}/{total}");
    assert_eq!(total, 7);
}

#[test]
fn only_the_first_word_of_a_command_leaves_and_never_an_argument() {
    let golden = golden();
    let text = golden["cases"][0]["body"].to_string();
    for secret in [
        "SECRET",
        "--token",
        "/usr/bin",
        "FOO=",
        "rm -rf",
        "hello there",
    ] {
        assert!(!text.contains(secret), "{secret}");
    }
    for job in golden["cases"][0]["body"]["jobs"].as_array().unwrap() {
        assert!(ALLOW.contains(&job["command"].as_str().unwrap()));
        assert_eq!(job.as_object().unwrap().len(), 7);
    }
}

#[test]
fn a_missing_tool_or_an_empty_path_is_no_data_and_starts_nothing() {
    for env in [
        ProcessEnv::only_path(""),
        ProcessEnv::only_path("/nonexistent-t044"),
        ProcessEnv(vec![]),
    ] {
        let got = list_jobs(&env, now(), &TimeZone::UTC);
        assert_eq!(
            got,
            json!({"jobs": [], "hidden": 0, "sources": {"systemd": "no data", "cron": "no data"}})
        );
    }
    assert_eq!(
        easy_personal_core::jobs::command_name("env A=1 sudo claude"),
        Some("sudo".into())
    );
    let _ = NotReadOnly;
}

#[test]
fn a_fake_that_hangs_is_cut_at_three_seconds() {
    let dir = scratch("hang");
    write_script(&dir.join("bin"), "systemctl", "exec /bin/sleep 30\n");
    write_script(&dir.join("bin"), "crontab", "exec /bin/sleep 30\n");
    let started = Instant::now();
    let got = list_jobs(
        &ProcessEnv::only_path(dir.join("bin")),
        now(),
        &TimeZone::UTC,
    );
    let took = started.elapsed().as_secs_f64();
    assert_eq!(
        got["sources"],
        json!({"systemd": "no data", "cron": "no data"})
    );
    assert!((2.9..12.0).contains(&took), "{took}"); // systemd tries the table too: up to two calls, then cron
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_fake_that_prints_without_end_is_capped_and_a_failing_one_is_no_data() {
    let dir = scratch("flood");
    write_script(
        &dir.join("bin"),
        "crontab",
        "while :; do printf '%s' 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'; done\n",
    );
    write_script(&dir.join("bin"), "systemctl", "exit 1\n");
    let started = Instant::now();
    let got = list_jobs(
        &ProcessEnv::only_path(dir.join("bin")),
        now(),
        &TimeZone::UTC,
    );
    assert_eq!(
        got["sources"],
        json!({"systemd": "no data", "cron": "no data"})
    );
    assert!(started.elapsed().as_secs_f64() < 3.5);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_crontab_of_ten_thousand_lines_with_hostile_lines_is_listed_fast() {
    let dir = scratch("big");
    let mut cron = String::new();
    for i in 0..10_000 {
        cron += &format!("{} {} * * * claude -p \"x{i}\n", i % 60, i % 24); // an unterminated quote on every line
    }
    cron += "0 0 31 2 * claude\n0 0 * * * claude\n";
    fs::write(dir.join("cron.txt"), &cron).unwrap();
    let out = dir.join("cron.txt");
    write_script(
        &dir.join("bin"),
        "crontab",
        &format!(
            "while IFS= read -r l || [ -n \"$l\" ]; do printf '%s\\n' \"$l\"; done < {}\n",
            quote(out.to_str().unwrap())
        ),
    );
    let started = Instant::now();
    let got = list_jobs(
        &ProcessEnv::only_path(dir.join("bin")),
        now(),
        &TimeZone::UTC,
    );
    assert_eq!(got["jobs"].as_array().unwrap().len(), 10_002);
    assert_eq!(got["sources"]["systemd"], "no data");
    assert!(started.elapsed().as_secs_f64() < 3.0);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_next_run_is_counted_in_the_zone_given() {
    // 2026-11-11T10:00:00Z is 2026-11-12 00:00 in Kiritimati (+14) and 2026-11-10 23:00 in Pago Pago (-11); `0 9 * * *` is 9:00 local.
    let dir = scratch("zone");
    write_script(
        &dir.join("bin"),
        "crontab",
        "printf '0 9 * * * claude\\n@daily claude\\n0 12 * * * claude\\n'\n",
    );
    for (zone, want) in [
        (
            "UTC",
            [
                "2026-11-12T09:00:00Z",
                "2026-11-12T00:00:00Z",
                "2026-11-11T12:00:00Z",
            ],
        ),
        (
            "Pacific/Kiritimati",
            [
                "2026-11-11T19:00:00Z",
                "2026-11-12T10:00:00Z",
                "2026-11-11T22:00:00Z",
            ],
        ),
        (
            "Pacific/Pago_Pago",
            [
                "2026-11-11T20:00:00Z",
                "2026-11-11T11:00:00Z",
                "2026-11-11T23:00:00Z",
            ],
        ),
    ] {
        let got = list_jobs(
            &ProcessEnv::only_path(dir.join("bin")),
            now(),
            &TimeZone::get(zone).unwrap(),
        );
        let nexts: Vec<&str> = got["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|j| j["next_run"].as_str().unwrap())
            .collect();
        assert_eq!(nexts, want, "{zone}");
    }
    let _ = fs::remove_dir_all(&dir);
}
