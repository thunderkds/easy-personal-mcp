//! T117 tests: the names file (temp folders only, never `~/.easy-personal`), the guarded routes through the real router,
//! and the past-sessions route over a temp copy of the parity store.

use super::*;
use crate::server::router;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::sync::atomic::{AtomicUsize, Ordering};
use tower::ServiceExt;

const TOKEN: &str = "test-token";
const PORT: u16 = 4343;

struct Temp(PathBuf);

impl Temp {
    fn new() -> Temp {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t117-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        Temp(dir)
    }

    fn file(&self) -> PathBuf {
        self.0.join(FILE)
    }

    /// The router over this folder; `with_store` copies the parity store in first.
    fn app(&self, with_store: bool) -> Router {
        if with_store {
            let parity =
                FsPath::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/store-v11.sqlite");
            fs::copy(parity, self.0.join("store.sqlite")).unwrap();
        }
        let store = self.0.join("store.sqlite");
        router(
            TOKEN.into(),
            PORT,
            Api::new(move || Ok(store.clone()), || 0),
        )
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn mode(path: &FsPath) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    for (k, v) in headers {
        request = request.header(*k, *v);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn good() -> Vec<(&'static str, &'static str)> {
    vec![
        ("host", "127.0.0.1:4343"),
        ("x-dashboard-token", TOKEN),
        ("content-type", "application/json"),
    ]
}

async fn put_name(app: &Router, id: &str, body: &str) -> (StatusCode, Value) {
    call(
        app,
        "PUT",
        &format!("/api/sessions/{id}/name"),
        &good(),
        body,
    )
    .await
}

#[test]
fn a_name_is_cleaned_and_kept_whole_or_refused() {
    assert_eq!(
        clean("  Fix login\tbug\n").as_deref(),
        Some("Fix login bug")
    );
    assert_eq!(clean("a\u{202e}b\u{200b}c\u{0007}").as_deref(), Some("abc")); // hidden characters go
    assert_eq!(clean("   \n\t ").as_deref(), None);
    assert_eq!(clean("\u{200b}\u{202e}").as_deref(), None);
    assert_eq!(clean(&"é".repeat(80)).map(|s| s.chars().count()), Some(80));
    assert_eq!(clean(&"é".repeat(81)), None); // refused, not cut
}

#[test]
fn a_write_makes_an_owner_only_file_by_rename_and_leaves_no_temp_file() {
    let t = Temp::new();
    let dir = t.0.join("not-yet"); // a folder that does not exist yet is made 0700
    store_name(&dir, "s-1", Some("first".into())).unwrap();
    assert_eq!(mode(&dir.join(FILE)), 0o600);
    assert_eq!(mode(&dir), 0o700);
    store_name(&dir, "s-2", Some("second".into())).unwrap();
    store_name(&dir, "s-1", Some("renamed".into())).unwrap();
    assert_eq!(
        load(&dir),
        [
            ("s-2".to_string(), "second".to_string()),
            ("s-1".into(), "renamed".into())
        ]
    );
    store_name(&dir, "s-2", None).unwrap();
    assert_eq!(load(&dir), [("s-1".to_string(), "renamed".to_string())]);
    store_name(&dir, "never-named", None).unwrap(); // removing nothing is no error
    let left: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, [std::ffi::OsString::from(FILE)]);
}

#[test]
fn past_2000_names_the_oldest_is_dropped() {
    let t = Temp::new();
    let names: Vec<_> = (0..MAX_NAMES)
        .map(|i| (format!("s{i}"), format!("n{i}")))
        .collect();
    save(&t.0, &names).unwrap();
    store_name(&t.0, "newest", Some("n".into())).unwrap();
    let got = load(&t.0);
    assert_eq!(got.len(), MAX_NAMES);
    assert_eq!(got[0].0, "s1");
    assert_eq!(got.last().unwrap().0, "newest");
}

#[test]
fn a_missing_or_unusable_file_is_no_names_and_the_next_write_starts_over() {
    let t = Temp::new();
    assert!(load(&t.0).is_empty()); // missing
    let bad: Vec<(&str, String)> = vec![
        ("not json", "{{{".into()),
        ("empty", String::new()),
        ("array", "[]".into()),
        ("no names", "{}".into()),
        ("names not a list", r#"{"names": {"a": "b"}}"#.into()),
        ("entry without name", r#"{"names": [{"id": "a"}]}"#.into()),
        ("entry not an object", r#"{"names": ["a"]}"#.into()),
        (
            "bad id",
            r#"{"names": [{"id": "a b", "name": "x"}]}"#.into(),
        ),
        (
            "empty name",
            r#"{"names": [{"id": "a", "name": ""}]}"#.into(),
        ),
        (
            "uncleaned name",
            r#"{"names": [{"id": "a", "name": " x "}]}"#.into(),
        ),
        (
            "long name",
            format!(
                r#"{{"names": [{{"id": "a", "name": "{}"}}]}}"#,
                "x".repeat(81)
            ),
        ),
        ("binary", "\u{0}\u{1}\u{2}".into()),
    ];
    for (why, text) in bad {
        fs::write(t.file(), text).unwrap();
        fs::set_permissions(t.file(), fs::Permissions::from_mode(0o600)).unwrap();
        assert!(load(&t.0).is_empty(), "{why}");
        store_name(&t.0, "ok", Some("fresh".into())).unwrap(); // never a failure: it starts again
        assert_eq!(
            load(&t.0),
            [("ok".to_string(), "fresh".to_string())],
            "{why}"
        );
    }
    // too many entries, a huge file, a file others can write, a directory, a symlink
    let many: Vec<_> = (0..=MAX_NAMES)
        .map(|i| json!({"id": format!("s{i}"), "name": "n"}))
        .collect();
    fs::write(t.file(), json!({"names": many}).to_string()).unwrap();
    assert!(load(&t.0).is_empty(), "2001 entries");
    fs::write(t.file(), vec![b' '; (MAX_FILE + 1) as usize]).unwrap();
    assert!(load(&t.0).is_empty(), "over the size cap");
    fs::write(t.file(), r#"{"names": [{"id": "a", "name": "x"}]}"#).unwrap();
    fs::set_permissions(t.file(), fs::Permissions::from_mode(0o666)).unwrap();
    assert!(load(&t.0).is_empty(), "world-writable");
    fs::set_permissions(t.file(), fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(load(&t.0).len(), 1);
    fs::remove_file(t.file()).unwrap();
    fs::create_dir(t.file()).unwrap();
    assert!(load(&t.0).is_empty(), "a directory");
    fs::remove_dir(t.file()).unwrap();
    let elsewhere = t.0.join("elsewhere.json");
    fs::write(&elsewhere, r#"{"names": [{"id": "a", "name": "x"}]}"#).unwrap();
    symlink(&elsewhere, t.file()).unwrap();
    assert!(load(&t.0).is_empty(), "a symlink");
}

#[tokio::test]
async fn the_name_route_is_guarded_and_wants_confirmed_true() {
    let t = Temp::new();
    let app = t.app(false);
    let uri = "/api/sessions/s-1/name";
    let ok = r#"{"name":"x","confirmed":true}"#;
    let without = |drop: &str| -> Vec<(&'static str, &'static str)> {
        good().into_iter().filter(|(k, _)| *k != drop).collect()
    };
    assert_eq!(
        call(&app, "PUT", uri, &without("x-dashboard-token"), ok)
            .await
            .0,
        403
    );
    let mut wrong_token = without("x-dashboard-token");
    wrong_token.push(("x-dashboard-token", "nope"));
    assert_eq!(call(&app, "PUT", uri, &wrong_token, ok).await.0, 403);
    let mut host = without("host");
    host.push(("host", "evil.invalid:80"));
    assert_eq!(call(&app, "PUT", uri, &host, ok).await.0, 403);
    let mut origin = good();
    origin.push(("origin", "http://evil.invalid"));
    assert_eq!(call(&app, "PUT", uri, &origin, ok).await.0, 403);
    assert!(!t.file().exists(), "a refused call writes nothing");
    assert_eq!(
        call(&app, "PUT", uri, &without("content-type"), ok).await.0,
        415
    );
    for (body, want) in [
        (r#"{"name":"x"}"#, "confirmed must be true"),
        (
            r#"{"name":"x","confirmed":false}"#,
            "confirmed must be true",
        ),
        (
            r#"{"name":"x","confirmed":"true"}"#,
            "confirmed must be true",
        ),
        (
            r#"{"name":"x","confirmed":true,"extra":1}"#,
            "unknown field",
        ),
    ] {
        let (status, answer) = put_name(&app, "s-1", body).await;
        assert_eq!(
            (status, answer["error"].as_str()),
            (StatusCode::BAD_REQUEST, Some(want)),
            "{body}"
        );
    }
    assert_eq!(put_name(&app, "s-1", "not json").await.0, 400);
    assert_eq!(put_name(&app, "s-1", r#"{"confirmed":true}"#).await.0, 400); // no name key
    assert_eq!(
        put_name(&app, "s-1", r#"{"name":5,"confirmed":true}"#)
            .await
            .0,
        400
    );
    assert!(!t.file().exists());
    for verb in ["GET", "POST", "PATCH", "DELETE"] {
        assert_eq!(call(&app, verb, uri, &good(), ok).await.0, 405, "{verb}");
    }
}

#[tokio::test]
async fn naming_sets_cleans_refuses_and_removes() {
    let t = Temp::new();
    let app = t.app(false);
    let (status, body) = put_name(
        &app,
        "abc-123",
        r#"{"name":"  Fix\nlogin\u202e  ","confirmed":true}"#,
    )
    .await;
    assert_eq!(
        (status, &body),
        (
            StatusCode::OK,
            &json!({"id": "abc-123", "name": "Fix login"})
        )
    );
    assert_eq!(mode(&t.file()), 0o600);
    assert_eq!(
        load(&t.0),
        [("abc-123".to_string(), "Fix login".to_string())]
    );
    for bad in [
        r#""""#,
        r#""   ""#,
        r#""\u200b""#,
        &format!("\"{}\"", "x".repeat(81)),
    ] {
        let (status, body) = put_name(
            &app,
            "abc-123",
            &format!(r#"{{"name":{bad},"confirmed":true}}"#),
        )
        .await;
        assert_eq!(status, 400, "{bad}");
        assert_eq!(body["error"], BAD_NAME);
        assert!(
            !body.to_string().contains("xxxx"),
            "the refusal does not echo the name"
        );
    }
    assert_eq!(load(&t.0).len(), 1, "a refused name changes nothing");
    for id in ["a%20b", "a.b", &"x".repeat(129)] {
        assert_eq!(
            put_name(&app, id, r#"{"name":"x","confirmed":true}"#)
                .await
                .0,
            400,
            "{id}"
        );
    }
    let (status, body) = put_name(&app, "abc-123", r#"{"name":null,"confirmed":true}"#).await;
    assert_eq!(
        (status, &body),
        (StatusCode::OK, &json!({"id": "abc-123", "name": null}))
    );
    assert!(load(&t.0).is_empty());
}

#[tokio::test]
async fn a_corrupt_file_is_never_a_500_for_the_owner() {
    let t = Temp::new();
    fs::write(t.file(), "garbage").unwrap();
    let app = t.app(true);
    assert_eq!(
        put_name(&app, "s1", r#"{"name":"a","confirmed":true}"#)
            .await
            .0,
        200
    );
    fs::write(t.file(), "garbage").unwrap();
    let (status, body) = call(&app, "GET", "/api/sessions/past", &good(), "").await;
    assert_eq!(status, 200, "{body}");
    assert!(
        body["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["name"].is_null())
    );
}

#[tokio::test]
async fn the_past_route_lists_sessions_with_their_names_and_is_guarded() {
    let t = Temp::new();
    let app = t.app(true);
    let no_token: Vec<_> = good()
        .into_iter()
        .filter(|(k, _)| *k != "x-dashboard-token")
        .collect();
    assert_eq!(
        call(&app, "GET", "/api/sessions/past", &no_token, "")
            .await
            .0,
        403
    );
    for verb in ["POST", "PUT", "DELETE"] {
        assert_eq!(
            call(&app, verb, "/api/sessions/past", &good(), "{}")
                .await
                .0,
            405,
            "{verb}"
        );
    }
    let (status, before) = call(&app, "GET", "/api/sessions/past", &good(), "").await;
    assert_eq!(status, 200, "{before}");
    let sessions = before["sessions"].as_array().unwrap();
    assert!(!sessions.is_empty(), "the parity store has sessions");
    assert!(sessions.iter().all(|s| s["name"].is_null()));
    let id = sessions[0]["id"].as_str().unwrap().to_owned();
    assert_eq!(
        put_name(&app, &id, r#"{"name":"Mine","confirmed":true}"#)
            .await
            .0,
        200
    );
    let (_, after) = call(&app, "GET", "/api/sessions/past", &good(), "").await;
    let named: Vec<_> = after["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| !s["name"].is_null())
        .collect();
    assert_eq!(named.len(), 1);
    assert_eq!(
        (named[0]["id"].as_str(), named[0]["name"].as_str()),
        (Some(id.as_str()), Some("Mine"))
    );
}

#[tokio::test]
async fn bad_filters_are_one_line_400s() {
    let t = Temp::new();
    let app = t.app(true);
    for q in [
        "kind=other",
        "from=yesterday",
        "to=2026-99-99",
        "before=garbage",
        "before=2026-01-01T00:00:00Z%7Cbad%20id",
        "account=nope",
        "project=%0A",
        "from=sk-ant-LEAKCANARY",
    ] {
        let (status, body) =
            call(&app, "GET", &format!("/api/sessions/past?{q}"), &good(), "").await;
        let error = body["error"].as_str().unwrap_or_default();
        assert_eq!(status, 400, "{q}: {body}");
        assert!(
            !error.is_empty() && !error.contains('\n') && !error.contains("LEAKCANARY"),
            "{q}: {error}"
        );
    }
    let (status, _) = call(
        &app,
        "GET",
        "/api/sessions/past?from=2020-01-01&to=2030-01-01&kind=terminal&project=&branch=",
        &good(),
        "",
    )
    .await;
    assert_eq!(status, 200);
}
