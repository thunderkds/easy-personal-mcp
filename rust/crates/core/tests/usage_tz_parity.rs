//! T009: `usage_summary` / `usage_breakdown` in a non-UTC machine zone against the Python goldens in
//! `rust/parity/golden-tz` (made by `gen_golden.py --tz`: Python run under each zone, over `golden-tz/store-v11.sqlite`).
//! Goldens are compared as parsed JSON, like `usage_parity.rs`. Which calls to make comes from the manifest
//! (`zones.<zone>.entries`: route + query), so a new golden entry needs no change here.
//!
//! How the zone is set: the core reads the process zone (the `TZ` variable, else `/etc/localtime`) and the zone crate
//! (`jiff`) caches it for 5 minutes, so one process cannot switch zones. Every check therefore runs in a child process:
//! the `#[test]`s below start this same test binary with `TZ` set for the child only and run `zone_goldens_child`
//! (`#[ignore]`, so a plain `cargo test` never runs it directly). Nothing in this process touches `TZ`, so the tests need
//! no lock and may run in parallel. The owner's real `~/.easy-personal` is never opened: only the fixture stores.

use easy_personal_core::metrics::{usage_breakdown, usage_summary};
use easy_personal_core::store::open_read_only;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

const INVALID_ZONE: &str = "Nowhere/Invalid"; // no such zone: libc (so Python) falls back to UTC

fn source_tz_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/golden-tz")
}

/// The golden folder the child reads: `T009_GOLDEN_DIR` (the mutation test's changed copy) or the committed one.
fn tz_dir() -> PathBuf {
    std::env::var_os("T009_GOLDEN_DIR").map_or_else(source_tz_dir, PathBuf::from)
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap_or_else(|e| panic!("{path:?}: {e}")))
        .unwrap()
}

fn manifest(dir: &Path) -> Value {
    read_json(&dir.join("manifest.json"))
}

// ---- the child: compares every entry of one zone in the zone this process was started with ----

fn query(params: &Value, key: &str) -> Option<String> {
    params.get(key).and_then(Value::as_str).map(str::to_string)
}

/// Runs in the child process. `T009_ZONE` names the golden folder (a key of `zones` in the manifest).
#[test]
#[ignore = "run by the zone tests below, in a child process with TZ set"]
fn zone_goldens_child() {
    let zone = std::env::var("T009_ZONE").expect("T009_ZONE names the golden zone to compare");
    let dir = tz_dir();
    let store = dir.join("store-v11.sqlite");
    let before = fs::read(&store).unwrap();
    let manifest = manifest(&dir);
    let entries = manifest["zones"][&zone]["entries"].as_array().unwrap();
    assert!(
        !entries.is_empty(),
        "no golden entries for {zone} in the manifest"
    );
    let conn = open_read_only(&store).unwrap();
    let mut wrong = Vec::new();
    let (mut summaries, mut days, mut models) = (0, 0, 0);
    for entry in entries {
        let file = entry["file"].as_str().unwrap();
        let params = &entry["params"];
        let (since, until) = (query(params, "since"), query(params, "until"));
        let actual = match entry["route"].as_str().unwrap() {
            "/api/usage/summary" => {
                summaries += 1;
                usage_summary(&conn, since.as_deref(), until.as_deref())
            }
            "/api/usage/breakdown" => {
                let by = query(params, "by").unwrap();
                days += usize::from(by == "day");
                models += usize::from(by == "model");
                usage_breakdown(&conn, &by, since.as_deref(), until.as_deref())
            }
            other => panic!("unknown route {other} in the manifest"),
        }
        .unwrap();
        let golden = read_json(&dir.join(&zone).join(file));
        if actual != golden {
            wrong.push(file.to_string());
        }
    }
    // the oracle covers the three call kinds the guide asks for (criterion 6)
    assert!(
        summaries > 0 && days > 0 && models > 0,
        "{zone}: oracle too thin"
    );
    assert!(wrong.is_empty(), "{zone}: differs from Python in {wrong:?}");
    drop(conn);
    assert_eq!(fs::read(&store).unwrap(), before, "the store was changed");
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut name = store.clone().into_os_string();
        name.push(suffix);
        assert!(!Path::new(&name).exists(), "sidecar {suffix} appeared");
    }
}

/// Runs in the child, started with `TZ=UTC`. The offset-less forms `usage_parity.rs` used to check under whatever zone
/// the machine had (T005 read them as UTC): date, datetime, a space and a fraction.
#[test]
#[ignore = "run by offset_less_forms_read_as_local_time, in a child process with TZ=UTC"]
fn offset_less_forms_child() {
    let conn = open_read_only(&source_tz_dir().join("store-v11.sqlite")).unwrap();
    let since =
        |s: &str| usage_summary(&conn, Some(s), None).unwrap()["period"]["since_utc"].clone();
    assert_eq!(since("2026-01-15"), json!("2026-01-15T00:00:00.000000Z"));
    assert_eq!(
        since("2026-01-15T10:05"),
        json!("2026-01-15T10:05:00.000000Z")
    );
    assert_eq!(
        since("2026-01-15 10:05:07.5"),
        json!("2026-01-15T10:05:07.500000Z")
    );
}

// ---- the helper: one place that sets the process zone for a check ----

struct Child {
    ok: bool,
    output: String,
}

/// Starts this test binary with `TZ` set to `tz` (`None` = removed) for the child only, and runs
/// `zone_goldens_child` against the goldens of `zone`.
fn in_zone(tz: Option<&str>, zone: &str, golden_dir: Option<&Path>) -> Child {
    run_child("zone_goldens_child", tz, zone, golden_dir)
}

fn run_child(test: &str, tz: Option<&str>, zone: &str, golden_dir: Option<&Path>) -> Child {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args([
        "--exact",
        test,
        "--ignored",
        "--nocapture",
        "--test-threads=1",
    ])
    .env("T009_ZONE", zone)
    .env_remove("T009_GOLDEN_DIR");
    match tz {
        Some(tz) => cmd.env("TZ", tz),
        None => cmd.env_remove("TZ"),
    };
    if let Some(dir) = golden_dir {
        cmd.env("T009_GOLDEN_DIR", dir);
    }
    let out = cmd.output().unwrap();
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Child {
        ok: out.status.success() && output.contains("1 passed"),
        output,
    }
}

fn assert_matches(tz: Option<&str>, zone: &str) {
    let child = in_zone(tz, zone, None);
    assert!(child.ok, "TZ={tz:?} against {zone}:\n{}", child.output);
}

// ---- criteria 2-5: Python's answer under each zone ----

#[test]
fn bangkok_matches_python() {
    assert_matches(Some("Asia/Bangkok"), "Asia/Bangkok");
}

#[test]
fn new_york_matches_python_across_both_dst_changes() {
    assert_matches(Some("America/New_York"), "America/New_York");
}

#[test]
fn offset_less_forms_read_as_local_time() {
    let child = run_child("offset_less_forms_child", Some("UTC"), "", None);
    assert!(child.ok, "{}", child.output);
}

// ---- criterion 8: an unknown or malformed TZ ----

/// Python (libc) reads an unknown zone name, and an empty `TZ`, as UTC (checked by running Python with
/// `TZ=Nowhere/Invalid` and `TZ=`); `Nowhere/Invalid` in the goldens is that Python run.
#[test]
fn an_unknown_tz_reads_utc_like_python() {
    assert_matches(Some(INVALID_ZONE), INVALID_ZONE);
}

#[test]
fn an_empty_tz_reads_utc_like_python() {
    assert_matches(Some(""), INVALID_ZONE);
}

/// `TZ=:Asia/Bangkok` (a leading colon is how POSIX says "a file name") means the same zone, in libc and in the crate.
#[test]
fn a_tz_with_a_leading_colon_names_the_zone() {
    assert_matches(Some(":Asia/Bangkok"), "Asia/Bangkok");
}

// ---- "TZ unset: the machine's zone is used, as Python does" ----

/// With no `TZ`, the zone comes from `/etc/localtime`. The golden to compare with is the one named by that link, so the
/// test only runs on a machine whose zone has a golden folder (Asia/Bangkok or America/New_York).
#[test]
fn an_unset_tz_uses_the_machine_zone() {
    let link = fs::read_link("/etc/localtime").ok();
    let name = link.as_ref().and_then(|l| {
        let text = l.to_string_lossy();
        text.split_once("zoneinfo/").map(|(_, z)| z.to_string())
    });
    let Some(zone) = name.filter(|z| source_tz_dir().join(z).is_dir() && z != INVALID_ZONE) else {
        eprintln!("skipped: /etc/localtime is not a zone with a golden folder");
        return;
    };
    assert_matches(None, &zone);
}

// ---- the oracle is not constant ----

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t009-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A copy of the goldens with one number changed in one Bangkok day group must fail the comparison.
#[test]
fn one_changed_golden_value_fails_the_zone_comparison() {
    let dir = TempDir::new();
    copy_tree(&source_tz_dir(), &dir.0);
    let file = dir
        .0
        .join("Asia/Bangkok")
        .join("api__usage_breakdown__day_all.json");
    let mut body = read_json(&file);
    let tokens = &mut body["groups"][1]["output_tokens"];
    *tokens = (tokens.as_i64().unwrap() + 1).into();
    fs::write(&file, serde_json::to_string_pretty(&body).unwrap()).unwrap();
    let child = in_zone(Some("Asia/Bangkok"), "Asia/Bangkok", Some(&dir.0));
    assert!(!child.ok, "a changed golden value must fail the comparison");
    assert!(
        child.output.contains("api__usage_breakdown__day_all.json"),
        "the failure names the entry:\n{}",
        child.output
    );
    // and the unchanged copy passes, so the failure is the changed value
    let same = TempDir::new();
    copy_tree(&source_tz_dir(), &same.0);
    let child = in_zone(Some("Asia/Bangkok"), "Asia/Bangkok", Some(&same.0));
    assert!(child.ok, "{}", child.output);
}

/// The zone changes the answer: the same call gives different day groups in Bangkok and New York (and an event at
/// 20:00Z is on the next day in Bangkok), so a core that read UTC cannot pass both.
#[test]
fn the_oracle_differs_between_zones() {
    let dir = source_tz_dir();
    let days = |zone: &str| {
        read_json(&dir.join(zone).join("api__usage_breakdown__day_all.json"))["groups"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| g["key"].as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    };
    let (bangkok, new_york, utc) = (
        days("Asia/Bangkok"),
        days("America/New_York"),
        days(INVALID_ZONE),
    );
    assert_ne!(bangkok, new_york);
    assert_ne!(bangkok, utc);
    assert!(bangkok.contains(&"2026-01-16".to_string()));
    assert!(
        new_york.contains(&"2026-03-07".to_string()),
        "23-hour day neighbour"
    );
}
