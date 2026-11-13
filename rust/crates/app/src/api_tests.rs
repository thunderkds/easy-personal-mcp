//! In-process tests of the API routes: the real router (guard included) is called with `tower::ServiceExt::oneshot`,
//! with a store path and a clock injected through `Api`. `warnings` and `plan` depend on the current time, so they are
//! compared with the goldens here under the manifests' `frozen_now`; the binary has no way to set the clock.
//! Every store is a temp copy of a parity store: the fixtures are never opened, nor `~/.easy-personal`.

use super::*;
use crate::server::router;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use tower::ServiceExt;

const TOKEN: &str = "test-token";
const PORT: u16 = 4242;
/// `frozen_now` of both manifests (`2026-01-20T00:00:00+00:00`), in microseconds since the epoch.
const FROZEN_NOW: i64 = 1_768_867_200_000_000;

fn parity(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity")
        .join(rel)
}

fn json_file(rel: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(parity(rel)).unwrap()).unwrap()
}

/// A temp folder holding a copy of `fixture` as `store.sqlite`; removed on drop.
struct TempStore {
    dir: PathBuf,
}

impl TempStore {
    fn new(fixture: &str) -> Self {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t014-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::copy(parity(fixture), dir.join("store.sqlite")).unwrap();
        TempStore { dir }
    }

    fn path(&self) -> PathBuf {
        self.dir.join("store.sqlite")
    }
}

impl Drop for TempStore {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn app(path: PathBuf, now: i64) -> Router {
    router(
        TOKEN.into(),
        PORT,
        Api::new(move || Ok(path.clone()), move || now),
    )
}

/// `GET path` with a good Host and token (or none): status and parsed JSON body.
async fn get(app: &Router, path: &str, token: bool) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .uri(path)
        .header("host", format!("127.0.0.1:{PORT}"));
    if token {
        request = request.header("x-dashboard-token", TOKEN);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[test]
fn the_frozen_clock_constant_is_the_manifests() {
    for dir in ["golden", "golden-rich"] {
        let manifest = json_file(&format!("{dir}/manifest.json"));
        assert_eq!(manifest["frozen_now"], "2026-01-20T00:00:00+00:00", "{dir}");
    }
    // 2026-01-20 is 20,473 days after the epoch
    assert_eq!(FROZEN_NOW, 20_473 * 86_400 * 1_000_000);
}

#[tokio::test]
async fn warnings_and_plan_equal_the_goldens_under_the_frozen_clock() {
    for (dir, fixture) in [
        ("golden", "store-v11.sqlite"),
        ("golden-rich", "store-rich/store-v11.sqlite"),
    ] {
        let store = TempStore::new(fixture);
        let app = app(store.path(), FROZEN_NOW);
        for (route, file) in [
            ("/api/warnings", "api__warnings__all.json"),
            ("/api/plan", "api__plan__all.json"),
        ] {
            let (status, body) = get(&app, route, true).await;
            assert_eq!(status, 200, "{dir} {route}: {body}");
            assert_eq!(body, json_file(&format!("{dir}/{file}")), "{dir} {route}");
        }
    }
}

#[tokio::test]
async fn the_routes_use_the_injected_clock_not_the_system_one() {
    // The rich store's 7-day windows end at the frozen time; a clock a year later sees them empty.
    let store = TempStore::new("store-rich/store-v11.sqlite");
    let later = FROZEN_NOW + 366 * 86_400 * 1_000_000;
    let (_, now) = get(&app(store.path(), FROZEN_NOW), "/api/warnings", true).await;
    let (_, then) = get(&app(store.path(), later), "/api/warnings", true).await;
    assert_ne!(now, then);
    assert_eq!(now, json_file("golden-rich/api__warnings__all.json"));
}

#[tokio::test]
async fn a_refused_request_never_reaches_the_store() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let app = router(
        TOKEN.into(),
        PORT,
        Api::new(
            move || {
                counted.fetch_add(1, Ordering::SeqCst);
                Err(StoreError::NoStore)
            },
            || FROZEN_NOW,
        ),
    );
    for route in ["/api/usage/summary", "/api/warnings", "/api/plan"] {
        let (status, body) = get(&app, route, false).await;
        assert_eq!(
            (status, body),
            (StatusCode::FORBIDDEN, json!({"error": "forbidden"})),
            "{route}"
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0, "no token, no store lookup");
    let (status, _) = get(&app, "/api/usage/summary?workspace=x", true).await;
    assert_eq!(
        status, 503,
        "a filter is resolved against the store, not refused"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let (status, _) = get(&app, "/api/usage/summary", true).await;
    assert_eq!(status, 503);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn a_store_read_does_not_block_the_async_worker() {
    // One worker thread only. The store lookup sleeps 400 ms: if it ran on the worker, the guard's 403 for the
    // second request could not be answered before it ended.
    let app = router(
        TOKEN.into(),
        PORT,
        Api::new(
            || {
                std::thread::sleep(std::time::Duration::from_millis(400));
                Err(StoreError::NoStore)
            },
            || FROZEN_NOW,
        ),
    );
    let start = std::time::Instant::now();
    let slow = async {
        let (status, _) = get(&app, "/api/usage/summary", true).await;
        (status, start.elapsed())
    };
    let fast = async {
        tokio::task::yield_now().await; // let the slow request start its blocking read first
        let (status, _) = get(&app, "/api/plan", false).await;
        (status, start.elapsed())
    };
    let ((slow_status, slow_at), (fast_status, fast_at)) = tokio::join!(slow, fast);
    assert_eq!(
        (slow_status, fast_status),
        (StatusCode::SERVICE_UNAVAILABLE, StatusCode::FORBIDDEN)
    );
    assert!(
        slow_at >= std::time::Duration::from_millis(400),
        "{slow_at:?}"
    );
    assert!(
        fast_at < std::time::Duration::from_millis(200),
        "{fast_at:?}"
    );
}

#[test]
fn the_query_is_split_and_decoded_like_parse_qsl() {
    let q = Query::parse("a=1&b=2&a=3&&c&d=&=e&f=%41%2b+x&%67=%zz&h=%");
    let want = [
        ("a", "1"),
        ("b", "2"),
        ("a", "3"),
        ("c", ""),
        ("d", ""),
        ("", "e"),
        ("f", "A+ x"),
        ("g", "%zz"),
        ("h", "%"),
    ];
    let got: Vec<(&str, &str)> = q.0.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    assert_eq!(got, want);
    assert_eq!(q.get("a"), Some("3"), "the last value wins");
    assert_eq!(q.get("missing"), None);
    assert_eq!(Query::parse("").0.len(), 0);
    assert_eq!(Query::parse("since=%ff").get("since"), Some("\u{fffd}"));
    // `;` is not a separator (Python 3.10+)
    assert_eq!(Query::parse("a=1;b=2").get("a"), Some("1;b=2"));
}

#[test]
fn the_scope_has_a_value_only_when_it_is_not_empty() {
    for (query, workspace, account) in [
        ("", None, None),
        ("workspace=&account=", None, None),
        ("workspace&account", None, None),
        ("workspace=a", Some("a"), None),
        ("account=%20", None, Some(" ")),
        ("workspace=a&workspace=", None, None), // the last value is the one Python reads
        ("workspace=&workspace=a", Some("a"), None),
        (
            "account=Claude+work&workspace=acme",
            Some("acme"),
            Some("Claude work"),
        ),
    ] {
        let q = Query::parse(query);
        let scope = q.scope();
        assert_eq!(
            (scope.workspace, scope.account),
            (workspace, account),
            "{query:?}"
        );
    }
}

#[test]
fn store_and_core_failures_map_to_fixed_texts_without_a_path_or_row_text() {
    let secret = "/home/someone/.easy-personal/store.sqlite SELECT secret";
    for (error, status, text) in [
        (
            StoreError::NoStore,
            503,
            "no store yet: nothing has been ingested into the data folder",
        ),
        (StoreError::Busy, 503, UNREADABLE),
        (StoreError::NotAFile, 503, UNREADABLE),
        (StoreError::Unreadable(secret.into()), 503, UNREADABLE),
        (
            StoreError::NoHome,
            503,
            "cannot find the data folder: set EASY_PERSONAL_HOME or HOME",
        ),
        (
            StoreError::Newer {
                found: 12,
                known: 11,
            },
            503,
            "the store is at version 12, newer than the 11 this build knows; update easy-personal-rs (nothing was changed)",
        ),
    ] {
        assert_eq!(
            store_failure(error),
            (StatusCode::from_u16(status).unwrap(), text.to_string())
        );
    }
    let invalid =
        metrics::MetricsError::Invalid("by must be one of day, model, project, tool".into());
    assert_eq!(
        metrics_failure(invalid),
        (
            StatusCode::BAD_REQUEST,
            "by must be one of day, model, project, tool".to_string()
        )
    );
    let store = metrics::MetricsError::Store(StoreError::Unreadable(secret.into()));
    assert_eq!(
        metrics_failure(store),
        (StatusCode::SERVICE_UNAVAILABLE, UNREADABLE.to_string())
    );
    let rules = rules::RulesError::Parse {
        line: 3,
        reason: "bad line",
    };
    assert_eq!(
        rules_failure(rules),
        (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL.to_string())
    );
    assert_eq!(
        rules_failure(rules::RulesError::Time),
        (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL.to_string())
    );
}

// ---- T019: `workspace` and `account` --------------------------------------------------------------------------
// The 12 scoped warnings and plan goldens of the rich manifest run here under the frozen clock (the binary has no
// clock knob); the 19 scoped usage ones run through the spawned binary in `tests/server.rs` under `TZ=UTC`.
// Nothing in this section reads an offset-less bound or groups by day, so the zone of the test process does not matter.

const RICH: &str = "store-rich/store-v11.sqlite";

/// Percent-encodes everything but RFC 3986 unreserved characters (a space becomes `%20`).
fn urlencode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The rich manifest's HTTP entries of one slice (`api__<slice>__*`) whose parameters name a `workspace` or an
/// `account`: `(request path, status, golden)`.
fn scoped_entries(slice: &str) -> Vec<(String, u16, Value)> {
    let manifest = json_file("golden-rich/manifest.json");
    manifest["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| {
            e["file"]
                .as_str()
                .unwrap()
                .starts_with(&format!("api__{slice}__"))
                && ["workspace", "account"]
                    .iter()
                    .any(|k| e["params"].get(k).is_some())
        })
        .map(|e| {
            let query: Vec<String> = e["params"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v.as_str().unwrap())))
                .collect();
            let path = format!("{}?{}", e["route"].as_str().unwrap(), query.join("&"));
            let golden = json_file(&format!("golden-rich/{}", e["file"].as_str().unwrap()));
            (path, e["status"].as_u64().unwrap() as u16, golden)
        })
        .collect()
}

/// The scoped warnings and plan entries that differ from their goldens: `(compared, differing paths)`.
async fn compare_scoped_rules(app: &Router) -> (usize, Vec<String>) {
    let (mut compared, mut wrong) = (0, Vec::new());
    for slice in ["warnings", "plan"] {
        for (path, status, golden) in scoped_entries(slice) {
            compared += 1;
            let (got_status, body) = get(app, &path, true).await;
            if got_status.as_u16() != status || body != golden {
                wrong.push(format!("{path}: {got_status} {body}"));
            }
        }
    }
    (compared, wrong)
}

#[tokio::test]
async fn scoped_warnings_and_plan_equal_the_goldens_under_the_frozen_clock() {
    // 6 warnings + 6 plan entries, counted from the manifest; two of them are the 400 for an unknown scope
    assert_eq!(scoped_entries("warnings").len(), 6);
    assert_eq!(scoped_entries("plan").len(), 6);
    let errors = scoped_entries("warnings")
        .into_iter()
        .chain(scoped_entries("plan"))
        .filter(|(_, status, _)| *status == 400)
        .count();
    assert_eq!(errors, 2);
    let store = TempStore::new(RICH);
    let (compared, wrong) = compare_scoped_rules(&app(store.path(), FROZEN_NOW)).await;
    assert_eq!(compared, 12);
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[tokio::test]
async fn moving_one_event_to_the_other_account_breaks_the_scoped_comparison() {
    // every event of account 2 moves to account 1: the account entries of warnings and plan differ
    let store = TempStore::new(RICH);
    let changed = rusqlite::Connection::open(store.path())
        .unwrap()
        .execute(
            "UPDATE events SET agent_account_id = 1 WHERE agent_account_id = 2",
            [],
        )
        .unwrap();
    assert!(changed > 0);
    let (_, wrong) = compare_scoped_rules(&app(store.path(), FROZEN_NOW)).await;
    assert!(
        wrong.iter().any(|w| w.contains("Claude%20work")),
        "{wrong:#?}"
    );
}

#[tokio::test]
async fn an_unknown_workspace_or_account_is_400_with_pythons_text_on_all_four_routes() {
    let store = TempStore::new(RICH);
    let app = app(store.path(), FROZEN_NOW);
    let secret = "%2Fhome%2Fsomeone%2Fsecret-folder";
    let account = "unknown account; valid labels: Claude default, Claude work";
    for route in [
        "/api/usage/summary?",
        "/api/usage/breakdown?by=model&",
        "/api/warnings?",
        "/api/plan?",
    ] {
        for (query, text) in [
            ("workspace=nope".to_string(), "unknown workspace"),
            ("account=nope".to_string(), account),
            // the workspace is checked before the account
            (
                "workspace=nope&account=nope".to_string(),
                "unknown workspace",
            ),
            // a value that is a path or a SQL fragment is never echoed
            (format!("workspace={secret}"), "unknown workspace"),
            (format!("account={secret}%27%3B--"), account),
        ] {
            let (status, body) = get(&app, &format!("{route}{query}"), true).await;
            assert_eq!(status, 400, "{route}{query}: {body}");
            assert_eq!(body, json!({ "error": text }), "{route}{query}");
        }
    }
}

#[tokio::test]
async fn an_invalid_by_or_period_is_reported_before_an_unknown_scope() {
    let store = TempStore::new(RICH);
    let app = app(store.path(), FROZEN_NOW);
    let (status, body) = get(&app, "/api/usage/breakdown?by=bad&workspace=nope", true).await;
    assert_eq!(status, 400);
    assert_eq!(
        body,
        json!({"error": "by must be one of day, model, project, tool"})
    );
    // a missing `by` is invalid too, whatever the scope
    let (status, body) = get(&app, "/api/usage/breakdown?workspace=nope", true).await;
    assert_eq!(status, 400);
    assert_eq!(
        body,
        json!({"error": "by must be one of day, model, project, tool"})
    );
    // Python reads the bounds first: a bad `since` wins over an unknown workspace (checked against Python)
    for path in [
        "/api/usage/summary?since=garbage&workspace=nope",
        "/api/usage/breakdown?by=model&since=garbage&workspace=nope",
    ] {
        let (status, body) = get(&app, path, true).await;
        assert_eq!(status, 400, "{path}");
        assert_eq!(
            body,
            json!({"error": "Invalid isoformat string: 'garbage'"}),
            "{path}"
        );
    }
}

#[tokio::test]
async fn an_empty_workspace_or_account_is_no_filter() {
    let store = TempStore::new(RICH);
    let app = app(store.path(), FROZEN_NOW);
    for (route, golden) in [
        ("/api/usage/summary?", "api__usage_summary__all.json"),
        (
            "/api/usage/breakdown?by=model&",
            "api__usage_breakdown__model.json",
        ),
        ("/api/warnings?", "api__warnings__all.json"),
        ("/api/plan?", "api__plan__all.json"),
    ] {
        for filter in [
            "workspace=&account=",
            "workspace&account",
            // the last value of a repeated key is the one that counts
            "workspace=nope&workspace=&account=nope&account=",
        ] {
            let (status, body) = get(&app, &format!("{route}{filter}"), true).await;
            assert_eq!(status, 200, "{route}{filter}: {body}");
            assert_eq!(
                body,
                json_file(&format!("golden-rich/{golden}")),
                "{route}{filter}"
            );
        }
    }
}

#[tokio::test]
async fn the_two_keys_follow_the_query_rules() {
    let store = TempStore::new(RICH);
    let app = app(store.path(), FROZEN_NOW);
    let work = json_file("golden-rich/api__usage_summary__account_work.json");
    let acme = json_file("golden-rich/api__usage_summary__workspace_acme.json");
    let both = json_file("golden-rich/api__usage_summary__workspace_acme_account_work.json");
    for (query, golden) in [
        // percent-encoded spaces around the label are stripped by the core
        ("account=%20Claude%20work%20", &work),
        // `+` is a space; the label compares case-insensitively
        ("account=claude+WORK", &work),
        ("account=Claude+work", &work),
        // the id works too
        ("account=2", &work),
        // the last value of a repeated key wins, in either order
        ("workspace=nope&workspace=acme", &acme),
        ("account=nope&workspace=acme&account=Claude+work", &both),
        // a key is percent-decoded
        ("work%73pace=acme", &acme),
        ("workspace=acme&%61ccount=Claude%20work", &both),
    ] {
        let (status, body) = get(&app, &format!("/api/usage/summary?{query}"), true).await;
        assert_eq!(status, 200, "{query}: {body}");
        assert_eq!(&body, golden, "{query}");
    }
    // `%2B` is a literal plus, not a space: `claude+work` is no label
    let (status, body) = get(&app, "/api/usage/summary?account=claude%2Bwork", true).await;
    assert_eq!(status, 400);
    assert_eq!(
        body,
        json!({"error": "unknown account; valid labels: Claude default, Claude work"})
    );
}

#[tokio::test]
async fn an_unknown_scope_is_refused_even_when_the_store_has_too_little_history() {
    // the default fixture holds one day of events, so every rule abstains; the scope is still resolved first
    let store = TempStore::new("store-v11.sqlite");
    let app = app(store.path(), FROZEN_NOW);
    for route in ["/api/warnings", "/api/plan"] {
        let (status, body) = get(&app, &format!("{route}?workspace=nope"), true).await;
        assert_eq!(status, 400, "{route}");
        assert_eq!(body, json!({"error": "unknown workspace"}), "{route}");
    }
}

#[test]
fn a_scoped_rules_error_maps_to_400_for_a_scope_and_to_the_store_texts_otherwise() {
    let secret = "/home/someone/.easy-personal/store.sqlite SELECT secret";
    assert_eq!(
        scoped_rules_failure(rules::ScopedRulesError::UnknownScope(
            "unknown workspace".into()
        )),
        (StatusCode::BAD_REQUEST, "unknown workspace".to_string())
    );
    assert_eq!(
        scoped_rules_failure(rules::ScopedRulesError::Rules(RulesError::Store(
            StoreError::Unreadable(secret.into())
        ))),
        (StatusCode::SERVICE_UNAVAILABLE, UNREADABLE.to_string())
    );
    assert_eq!(
        scoped_rules_failure(rules::ScopedRulesError::Rules(RulesError::Store(
            StoreError::NoStore
        ))),
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "no store yet: nothing has been ingested into the data folder".to_string()
        )
    );
    assert_eq!(
        scoped_rules_failure(rules::ScopedRulesError::Rules(RulesError::Time)),
        (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL.to_string())
    );
}

#[tokio::test]
async fn a_store_problem_while_the_scope_is_resolved_is_the_fixed_503_on_all_four_routes() {
    // no `workspaces` table: looking the scope up fails, which is a store problem, not an unknown workspace
    let store = TempStore::new(RICH);
    rusqlite::Connection::open(store.path())
        .unwrap()
        .execute_batch("DROP TABLE workspace_repos; DROP TABLE workspaces;")
        .unwrap();
    let app = app(store.path(), FROZEN_NOW);
    for route in [
        "/api/usage/summary?",
        "/api/usage/breakdown?by=model&",
        "/api/warnings?",
        "/api/plan?",
    ] {
        let (status, body) = get(&app, &format!("{route}workspace=acme"), true).await;
        assert_eq!(status, 503, "{route}");
        assert_eq!(body, json!({ "error": UNREADABLE }), "{route}");
        // the same store without a scope is still served
        let (status, _) = get(&app, route.trim_end_matches(['?', '&']), true).await;
        assert_eq!(status, 200, "{route}");
    }
}

#[tokio::test]
async fn the_session_routes_map_scope_and_store_problems_like_the_other_routes() {
    let store = TempStore::new(RICH);
    let app = app(store.path(), FROZEN_NOW);
    for route in ["/api/usage/sessions", "/api/usage/outliers"] {
        // a bad period first (Python reads the bounds before the scope), then an unknown scope
        let (status, body) = get(&app, &format!("{route}?since=bad&workspace=nope"), true).await;
        assert_eq!(status, 400, "{route}");
        assert_eq!(
            body,
            json!({"error": "Invalid isoformat string: 'bad'"}),
            "{route}"
        );
        let (status, body) = get(&app, &format!("{route}?workspace=nope"), true).await;
        assert_eq!(status, 400, "{route}");
        assert_eq!(body, json!({"error": "unknown workspace"}), "{route}");
        // no token: refused before the store is read
        let (status, body) = get(&app, route, false).await;
        assert_eq!(status, 403, "{route}");
        assert_eq!(body, json!({"error": "forbidden"}), "{route}");
    }
    // no `workspaces` table: resolving the scope fails, which is a store problem, not an unknown workspace
    rusqlite::Connection::open(store.path())
        .unwrap()
        .execute_batch("DROP TABLE workspace_repos; DROP TABLE workspaces;")
        .unwrap();
    for route in ["/api/usage/sessions", "/api/usage/outliers"] {
        let (status, body) = get(&app, &format!("{route}?workspace=acme"), true).await;
        assert_eq!(status, 503, "{route}");
        assert_eq!(body, json!({ "error": UNREADABLE }), "{route}");
        let (status, _) = get(&app, route, true).await;
        assert_eq!(status, 200, "{route}");
    }
}

// ---- T022: `/api/workspaces` and `/api/workspaces/unassigned` ----

#[tokio::test]
async fn the_workspace_routes_answer_the_core_for_both_stores_and_ignore_the_query() {
    // The goldens are compared over the binary (`tests/workspaces.rs`); here the router in-process answers the two
    // default-store goldens exactly (no `<WORK>` token there) and a query string changes nothing.
    let store = TempStore::new("store-v11.sqlite");
    let app = app(store.path(), FROZEN_NOW);
    for (route, file) in [
        ("/api/workspaces", "api__workspaces__all.json"),
        (
            "/api/workspaces/unassigned",
            "api__workspaces_unassigned__all.json",
        ),
    ] {
        let want = json_file(&format!("golden/{file}"));
        for query in ["", "?workspace=nope", "?account=1&since=garbage&by=x"] {
            let (status, body) = get(&app, &format!("{route}{query}"), true).await;
            assert_eq!(status, 200, "{route}{query}: {body}");
            assert_eq!(body, want, "{route}{query}");
        }
    }
}

#[tokio::test]
async fn the_workspace_routes_are_guarded_and_map_store_failures_to_fixed_texts() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let app = router(
        TOKEN.into(),
        PORT,
        Api::new(
            move || {
                counted.fetch_add(1, Ordering::SeqCst);
                Err(StoreError::NoStore)
            },
            || FROZEN_NOW,
        ),
    );
    for route in ["/api/workspaces", "/api/workspaces/unassigned"] {
        let (status, body) = get(&app, route, false).await;
        assert_eq!(
            (status, body),
            (StatusCode::FORBIDDEN, json!({"error": "forbidden"})),
            "{route}"
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0, "no token, no store lookup");
    for route in ["/api/workspaces", "/api/workspaces/unassigned"] {
        let (status, body) = get(&app, route, true).await;
        assert_eq!(status, 503, "{route}");
        assert_eq!(
            body,
            json!({"error": "no store yet: nothing has been ingested into the data folder"})
        );
    }
}

// ---- GET /api/store/health (T059): the cache, the single flight and HEAD, where a check can be counted ---------------------

fn health_runs(api: &Api) -> usize {
    api.health.runs.load(Ordering::SeqCst)
}

/// A store whose path the test can change between requests.
fn health_app(path: PathBuf) -> (Router, Api, Arc<std::sync::Mutex<PathBuf>>) {
    let current = Arc::new(std::sync::Mutex::new(path));
    let seen = current.clone();
    let api = Api::new(move || Ok(seen.lock().unwrap().clone()), || FROZEN_NOW);
    (router(TOKEN.into(), PORT, api.clone()), api, current)
}

async fn method(app: &Router, method: &str, path: &str) -> (StatusCode, usize) {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("host", format!("127.0.0.1:{PORT}"))
        .header("x-dashboard-token", TOKEN)
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    (
        status,
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .len(),
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_health_requests_share_one_check_and_wait_for_it() {
    let store = TempStore::new("store-v11.sqlite");
    let (app, api, _) = health_app(store.path());
    let tasks: Vec<_> = (0..12)
        .map(|_| {
            let app = app.clone();
            tokio::spawn(async move { get(&app, "/api/store/health", true).await })
        })
        .collect();
    let mut answers = Vec::new();
    for task in tasks {
        answers.push(task.await.unwrap());
    }
    assert_eq!(health_runs(&api), 1, "twelve requests, one check");
    assert!(
        answers
            .iter()
            .all(|a| a.0 == StatusCode::OK && a.1 == answers[0].1)
    );
    assert_eq!(answers[0].1["healthy"], true);
}

#[tokio::test]
async fn a_health_answer_is_reused_for_five_seconds_then_checked_again() {
    let store = TempStore::new("store-v11.sqlite");
    let (app, api, _) = health_app(store.path());
    assert_eq!(HEALTH_CACHE, Duration::from_secs(5));
    get(&app, "/api/store/health", true).await;
    get(&app, "/api/store/health", true).await;
    assert_eq!(health_runs(&api), 1);
    // a store whose time-to-live is over is checked again
    let api = Api::new(
        {
            let p = store.path();
            move || Ok(p.clone())
        },
        || FROZEN_NOW,
    )
    .with_health_ttl(Duration::ZERO);
    let app = router(TOKEN.into(), PORT, api.clone());
    get(&app, "/api/store/health", true).await;
    get(&app, "/api/store/health", true).await;
    assert_eq!(health_runs(&api), 2);
}

#[tokio::test]
async fn the_health_cache_is_kept_per_store() {
    let (one, two) = (
        TempStore::new("store-v11.sqlite"),
        TempStore::new("store-rich/store-v11.sqlite"),
    );
    let (app, api, current) = health_app(one.path());
    let (_, first) = get(&app, "/api/store/health", true).await;
    *current.lock().unwrap() = two.path();
    let (_, second) = get(&app, "/api/store/health", true).await;
    assert_eq!(health_runs(&api), 2, "another store is another check");
    assert_ne!(first["tables"], second["tables"]);
    *current.lock().unwrap() = one.path();
    assert_eq!(get(&app, "/api/store/health", true).await.1, first);
    assert_eq!(
        health_runs(&api),
        2,
        "the first store's answer is still cached"
    );
}

#[tokio::test]
async fn head_runs_no_check_and_the_guard_still_applies() {
    let store = TempStore::new("store-v11.sqlite");
    let (app, api, _) = health_app(store.path());
    assert_eq!(
        method(&app, "HEAD", "/api/store/health").await,
        (StatusCode::OK, 0)
    );
    assert_eq!(health_runs(&api), 0);
    for verb in ["POST", "PUT", "PATCH", "DELETE"] {
        assert_eq!(
            method(&app, verb, "/api/store/health").await.0,
            StatusCode::METHOD_NOT_ALLOWED,
            "{verb}"
        );
    }
    assert_eq!(health_runs(&api), 0);
}

#[tokio::test]
async fn a_missing_data_folder_is_the_503_every_route_gives() {
    let app = router(
        TOKEN.into(),
        PORT,
        Api::new(|| Err(StoreError::NoHome), || FROZEN_NOW),
    );
    let (status, body) = get(&app, "/api/store/health", true).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body,
        json!({"error": "cannot find the data folder: set EASY_PERSONAL_HOME or HOME"})
    );
}
