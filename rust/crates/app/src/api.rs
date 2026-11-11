//! The read routes of the dashboard API: `/api/usage/summary`, `/api/usage/breakdown`, `/api/usage/sessions`,
//! `/api/usage/outliers`, `/api/warnings`, `/api/plan`, `/api/workspaces`, `/api/workspaces/unassigned`.
//! They answer the same JSON and status codes as `src/easy_personal/surfaces/api.py` (`_query`, `_period`, `_who`);
//! the goldens in `rust/parity/golden*/` are the oracle. Every route sits behind the guard in `server.rs`.
//! The first six take the optional `workspace` (a name) and `account` (an id or a label) and pass them to the core's scoped
//! functions; an unknown one is a 400 with the core's fixed text (it names none of the input). The sessions route
//! always asks the core for 50 detailed sessions (`TIMELINE_DETAIL_SESSIONS` in Python): it has no parameter for it.
//! The two workspace routes are read-only (the writes of Python's `/api/workspaces` stay absent: those methods are 405)
//! and, like Python's `_query` handlers, ignore the whole query string.
//!
//! Each request reads the store on its own: a new read-only connection inside `spawn_blocking` (never on an async
//! worker), closed when the answer is built. The server never creates or writes the store. The core's JSON is passed
//! through as a `serde_json::Value`; no body carries a path, SQL or row text (NFR-002): a store problem becomes one of
//! a few fixed texts.

use axum::Router;
use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use easy_personal_core::clock::now_micros;
use easy_personal_core::metrics::{self, MetricsError};
use easy_personal_core::plan::improvement_plan_scoped;
use easy_personal_core::rules::{self, RulesError, ScopedRulesError, evaluate_scoped};
use easy_personal_core::scope::Scope;
use easy_personal_core::sessions::{session_outliers_scoped, session_timeline_scoped};
use easy_personal_core::store::{self, StoreError};
use easy_personal_core::workspaces;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;

/// Answer for a store that is busy, not a regular file or unreadable. Fixed on purpose: the core's text for these
/// can name a SQLite error code, and a body must never carry a path, SQL or row text.
const UNREADABLE: &str = "the store cannot be read right now; try again in a moment";
const INTERNAL: &str = "internal error";

/// Sessions that carry `points` and `gaps` in `/api/usage/sessions` (Python's `TIMELINE_DETAIL_SESSIONS`: the page draws 50).
const TIMELINE_DETAIL_SESSIONS: usize = 50;

type Failure = (StatusCode, String);
type StorePath = Arc<dyn Fn() -> Result<PathBuf, StoreError> + Send + Sync>;
type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;

/// What the routes need from outside: where the store is, and what time it is (microseconds since the epoch, UTC).
/// The binary passes the real ones ([`Api::system`]); the clock can be injected only in-process, by a test.
#[derive(Clone)]
pub struct Api {
    store_path: StorePath,
    clock: Clock,
}

impl Api {
    pub fn new(
        store_path: impl Fn() -> Result<PathBuf, StoreError> + Send + Sync + 'static,
        clock: impl Fn() -> i64 + Send + Sync + 'static,
    ) -> Self {
        Api {
            store_path: Arc::new(store_path),
            clock: Arc::new(clock),
        }
    }

    /// `EASY_PERSONAL_HOME` / `HOME` read on each request (like Python's `config.store_path()`), and the system clock.
    pub fn system() -> Self {
        Api::new(store::store_path, now_micros)
    }
}

pub fn routes() -> Router<Api> {
    Router::new()
        .route("/api/usage/summary", get(summary))
        .route("/api/usage/breakdown", get(breakdown))
        .route("/api/usage/sessions", get(sessions))
        .route("/api/usage/outliers", get(outliers))
        .route("/api/warnings", get(warnings))
        .route("/api/plan", get(plan))
        .route("/api/workspaces", get(workspace_list))
        .route("/api/workspaces/unassigned", get(workspaces_unassigned))
}

async fn summary(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, q, _now| {
        metrics::usage_summary_scoped(conn, q.get("since"), q.get("until"), q.scope())
            .map_err(metrics_failure)
    })
    .await
}

async fn breakdown(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, q, _now| {
        let by = q.get("by").unwrap_or("");
        metrics::usage_breakdown_scoped(conn, by, q.get("since"), q.get("until"), q.scope())
            .map_err(metrics_failure)
    })
    .await
}

async fn sessions(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, q, _now| {
        session_timeline_scoped(
            conn,
            q.get("since"),
            q.get("until"),
            Some(TIMELINE_DETAIL_SESSIONS),
            q.scope(),
        )
        .map_err(metrics_failure)
    })
    .await
}

async fn outliers(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, q, _now| {
        session_outliers_scoped(conn, q.get("since"), q.get("until"), q.scope())
            .map_err(metrics_failure)
    })
    .await
}

async fn warnings(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, q, now| {
        let rules = rules::shipped_rules().map_err(rules_failure)?;
        evaluate_scoped(conn, now, &rules, q.scope()).map_err(scoped_rules_failure)
    })
    .await
}

async fn plan(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, q, now| {
        let rules = rules::shipped_rules().map_err(rules_failure)?;
        improvement_plan_scoped(conn, now, &rules, q.scope()).map_err(scoped_rules_failure)
    })
    .await
}

async fn workspace_list(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, _q, _now| {
        workspaces::list_all(conn).map_err(metrics_failure)
    })
    .await
}

async fn workspaces_unassigned(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, _q, _now| {
        workspaces::unassigned(conn).map_err(metrics_failure)
    })
    .await
}

/// Parses the query, then runs `read` on a fresh read-only connection off the async workers.
async fn serve<F>(api: Api, uri: Uri, read: F) -> Response
where
    F: FnOnce(&Connection, &Query, i64) -> Result<Value, Failure> + Send + 'static,
{
    let query = Query::parse(uri.query().unwrap_or(""));
    let outcome = tokio::task::spawn_blocking(move || {
        let path = (api.store_path)().map_err(store_failure)?;
        let conn = store::open_checked(&path).map_err(store_failure)?;
        read(&conn, &query, (api.clock)())
    })
    .await;
    match outcome {
        Ok(Ok(body)) => respond(StatusCode::OK, &body),
        Ok(Err((status, text))) => respond(status, &error_body(&text)),
        // the read task panicked: say nothing about why
        Err(_) => respond(StatusCode::INTERNAL_SERVER_ERROR, &error_body(INTERNAL)),
    }
}

fn error_body(text: &str) -> Value {
    json!({ "error": text })
}

fn respond(status: StatusCode, body: &Value) -> Response {
    (
        status,
        [(CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

fn internal() -> Failure {
    (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL.to_string())
}

/// 503 for every way the store cannot be served. A missing store and a newer one keep the core's own text (neither
/// has a path in it); busy, not-a-file and unreadable share one fixed text.
fn store_failure(error: StoreError) -> Failure {
    let text = match &error {
        StoreError::NoStore | StoreError::Newer { .. } | StoreError::NoHome => error.to_string(),
        StoreError::Busy | StoreError::NotAFile | StoreError::Unreadable(_) => {
            UNREADABLE.to_string()
        }
    };
    (StatusCode::SERVICE_UNAVAILABLE, text)
}

/// Python answers a `ValueError` from the core with 400 and `{"error": str(exc)}`; the core's text equals it.
fn metrics_failure(error: MetricsError) -> Failure {
    match error {
        MetricsError::Invalid(text) => (StatusCode::BAD_REQUEST, text),
        MetricsError::Store(error) => store_failure(error),
        MetricsError::Prices(_) => internal(),
    }
}

/// An unknown workspace or account is the caller's mistake: 400 with the core's fixed text (Python answers 400 with
/// `str(exc)`; the text names none of the input). Everything else is as for any rules error.
fn scoped_rules_failure(error: ScopedRulesError) -> Failure {
    match error {
        ScopedRulesError::UnknownScope(text) => (StatusCode::BAD_REQUEST, text),
        ScopedRulesError::Rules(error) => rules_failure(error),
    }
}

fn rules_failure(error: RulesError) -> Failure {
    match error {
        RulesError::Store(error) => store_failure(error),
        RulesError::Parse { .. } | RulesError::Time => internal(),
    }
}

/// The query string as Starlette reads it: `parse_qsl(query, keep_blank_values=True)` with a UTF-8 `unquote`
/// (an escape that is not UTF-8 becomes U+FFFD), and `QueryParams.get` returning the **last** value of a key.
/// Pairs split on `&` only; an empty pair is skipped; a key without `=` has the empty value; `+` is a space
/// (before percent-decoding, so `%2B` stays a plus). The HTTP layer already refuses raw non-ASCII bytes in the
/// target, so nothing here sees what Python would read as Latin-1.
struct Query(Vec<(String, String)>);

impl Query {
    fn parse(raw: &str) -> Self {
        Query(
            raw.split('&')
                .filter(|pair| !pair.is_empty())
                .map(|pair| {
                    let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
                    (decode(key), decode(value))
                })
                .collect(),
        )
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The `workspace` and `account` of the request. An empty value is no filter (Python's `q.get(..) or None`).
    fn scope(&self) -> Scope<'_> {
        let value = |key| self.get(key).filter(|value| !value.is_empty());
        Scope {
            workspace: value("workspace"),
            account: value("account"),
        }
    }
}

fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let hex = |b: Option<&u8>| b.and_then(|b| (*b as char).to_digit(16));
    let (mut out, mut i) = (Vec::with_capacity(bytes.len()), 0);
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' => match (hex(bytes.get(i + 1)), hex(bytes.get(i + 2))) {
                (Some(high), Some(low)) => {
                    out.push((high * 16 + low) as u8);
                    i += 2;
                }
                _ => out.push(b'%'), // not an escape: stays as typed
            },
            byte => out.push(byte),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
