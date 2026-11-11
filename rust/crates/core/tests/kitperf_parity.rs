//! T044: `kitperf::kit_findings` against the four `mcp__kit_findings__*` goldens (`golden`: 1, `golden-rich`: 3), replayed at
//! the manifest's `frozen_now` over the committed stores (opened read-only), and the bounds Python has (limit clamp, bad
//! `since`). The owner's real data folder is never opened.

use easy_personal_core::kitperf::kit_findings;
use easy_personal_core::metrics::MetricsError;
use easy_personal_core::store::open_read_only;
use jiff::tz::TimeZone;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// 2026-01-20T00:00:00Z in microseconds: the manifest's `frozen_now`.
const FROZEN_NOW: i64 = 1_768_867_200_000_000;

fn parity() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity")
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path:?}: {e}")))
        .unwrap()
}

fn replay(golden: &str, store: &str) -> (usize, usize) {
    let dir = parity().join(golden);
    let conn = open_read_only(&parity().join(store)).unwrap();
    let manifest = read_json(&dir.join("manifest.json"));
    let (mut equal, mut total) = (0, 0);
    for entry in manifest["entries"].as_array().unwrap() {
        if entry["tool"] != "kit_findings" {
            continue;
        }
        total += 1;
        let params = &entry["params"];
        let limit = params["limit"].as_i64().unwrap_or(3);
        let since = params["since"].as_str();
        let got = kit_findings(&conn, limit, since, FROZEN_NOW, &TimeZone::UTC).unwrap();
        let want = read_json(&dir.join(entry["file"].as_str().unwrap()));
        assert_eq!(got, want, "{}", entry["file"]);
        equal += 1;
    }
    (equal, total)
}

#[test]
fn the_four_kit_findings_goldens_are_equal() {
    let (a, b) = replay("golden", "store-v11.sqlite");
    let (c, d) = replay("golden-rich", "store-rich/store-v11.sqlite");
    println!("kit_findings goldens equal/total: {}/{}", a + c, b + d);
    assert_eq!((a + c, b + d), (4, 4));
}

#[test]
fn the_limit_is_clamped_to_one_through_ten_and_a_bad_since_is_a_plain_error() {
    let conn = open_read_only(&parity().join("store-rich/store-v11.sqlite")).unwrap();
    let ask = |limit| kit_findings(&conn, limit, None, FROZEN_NOW, &TimeZone::UTC).unwrap();
    let findings = |v: &Value| v["findings"].as_array().unwrap().len();
    assert_eq!(findings(&ask(0)), 1);
    assert_eq!(findings(&ask(-5)), 1);
    assert_eq!(findings(&ask(i64::MAX)), findings(&ask(10)));
    assert_eq!(findings(&ask(1)), 1);
    match kit_findings(&conn, 3, Some("garbage"), FROZEN_NOW, &TimeZone::UTC) {
        Err(MetricsError::Invalid(text)) => assert_eq!(text, "Invalid isoformat string: 'garbage'"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_read_changes_nothing_and_the_answer_holds_no_path_or_id() {
    let store = parity().join("store-rich/store-v11.sqlite");
    let before = std::fs::read(&store).unwrap();
    let conn = open_read_only(&store).unwrap();
    let body = kit_findings(&conn, 10, None, FROZEN_NOW, &TimeZone::UTC)
        .unwrap()
        .to_string();
    assert_eq!(std::fs::read(&store).unwrap(), before);
    for leak in ["/work/", "session_id", "uuid", "path", "branch", "model"] {
        assert!(!body.contains(leak), "{leak}");
    }
}
