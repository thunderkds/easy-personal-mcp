//! The dashboard API's routes: the read routes `/api/usage/summary`, `/api/usage/breakdown`, `/api/usage/sessions`,
//! `/api/usage/outliers`, `/api/warnings`, `/api/plan`, `/api/workspaces`, `/api/workspaces/unassigned`, `/api/connectors`, `/api/kit`,
//! `/api/kit/workspaces`, `/api/kit/reports`, `/api/kit/report-files`, `/api/kit/report-file` (T045), `/api/reports`, `/api/reports/{id}`,
//! `/api/board` and `/api/memory-graph` (T043), `/api/store/health` (T059, below), `/api/first-run`, `/api/accounts`, `/api/sync`, `/api/dirs` and `/api/jobs` (T040: an id that is not
//! plain digits, or has no report, is a 404 `no such report`; a saved report whose snapshot cannot be read is a fixed 500),
//! and the seven store-only write routes of T048, which `writes.rs` serves.
//! `GET /api/live` (T056) is the one stream: Server-Sent Events from the live feed (`live_feed.rs`); it reads no store
//! itself, and it is the one route whose token may come in the query (`server.rs`).
//! The read routes are all `GET`. `/api/dirs` lists folders of the file system and `/api/jobs` runs the read-only `systemctl` / `crontab`
//! calls of `jobs.rs` (the first word of an agent command only); `/api/kit/report-file` reads one capped file of a
//! report folder by handle (`report_files.rs`). The others read the store.
//! They answer the same JSON and status codes as `src/easy_personal/surfaces/api.py` (`_query`, `_period`, `_who`);
//! the goldens in `rust/parity/golden*/` are the oracle. Every route sits behind the guard in `server.rs`.
//! The first six take the optional `workspace` (a name) and `account` (an id or a label) and pass them to the core's scoped
//! functions; an unknown one is a 400 with the core's fixed text (it names none of the input). The sessions route
//! always asks the core for 50 detailed sessions (`TIMELINE_DETAIL_SESSIONS` in Python): it has no parameter for it.
//! The two workspace read routes ignore the whole query string, like Python's `_query` handlers. The store-only writes (T048:
//! `POST/PATCH/DELETE /api/workspaces[/{id}]`, `POST /api/accounts[/{id}/confirm|ignore]`, `PUT /api/accounts/{id}/workspaces`)
//! are registered here and served by `writes.rs`; every other method of these paths is a 405.
//!
//! Each request that reads the store does so on its own: a new read-only connection inside `spawn_blocking` (never on an
//! async worker), closed when the answer is built (`/api/dirs`, `/api/jobs` and `/api/version` do not open it). No read route
//! creates or writes the store, and neither does start-up: only the seven write routes of `writes.rs` open it for writing,
//! inside their request, and none creates a store. The core's JSON is passed
//! through as a `serde_json::Value`; no body carries a path, SQL or row text (NFR-002): a store problem becomes one of
//! a few fixed texts.

use crate::live_feed::{LIVE_PATH, SharedTail, SseBody, resume_id};
use crate::writes;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::header::{
    CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS,
};
use axum::http::{HeaderMap, HeaderName, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post, put};
use easy_personal_core::accounts;
use easy_personal_core::board;
use easy_personal_core::clock::now_micros;
use easy_personal_core::first_run;
use easy_personal_core::folders;
use easy_personal_core::kit;
use easy_personal_core::memgraph::{self, Env};
use easy_personal_core::metrics::{self, MetricsError};
use easy_personal_core::plan::improvement_plan_scoped;
use easy_personal_core::report_files::{self, ReportError, parse_repo_no};
use easy_personal_core::reports::{self, ReportsError};
use easy_personal_core::rules::{self, RulesError, ScopedRulesError, evaluate_scoped};
use easy_personal_core::scope::Scope;
use easy_personal_core::sessions::{session_outliers_scoped, session_timeline_scoped};
use easy_personal_core::store::{self, StoreError};
use easy_personal_core::store_health;
use easy_personal_core::suggestions::{self, ViewError};
use easy_personal_core::sync::{self, SyncError};
use easy_personal_core::workspaces;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Answer for a store that is busy, not a regular file or unreadable. Fixed on purpose: the core's text for these
/// can name a SQLite error code, and a body must never carry a path, SQL or row text.
const UNREADABLE: &str = "the store cannot be read right now; try again in a moment";
pub(crate) const INTERNAL: &str = "internal error";

/// Sessions that carry `points` and `gaps` in `/api/usage/sessions` (Python's `TIMELINE_DETAIL_SESSIONS`: the page draws 50).
const TIMELINE_DETAIL_SESSIONS: usize = 50;

type Failure = (StatusCode, String);
type StorePath = Arc<dyn Fn() -> Result<PathBuf, StoreError> + Send + Sync>;
pub(crate) type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;

/// What the routes need from outside: where the store is, and what time it is (microseconds since the epoch, UTC).
/// The binary passes the real ones ([`Api::system`]); the clock can be injected only in-process, by a test.
#[derive(Clone)]
pub struct Api {
    pub(crate) store_path: StorePath,
    clock: Clock,
    /// The tail `GET /api/live` reads (`None`: no feed runs, the stream sends Python's empty snapshot).
    live: Option<SharedTail>,
    /// Set when the server stops: every open stream ends at its next tick.
    stopping: Arc<AtomicBool>,
    /// The answers of `GET /api/store/health` (shared by every clone of the `Api`).
    health: Arc<HealthCache>,
}

impl Api {
    pub fn new(
        store_path: impl Fn() -> Result<PathBuf, StoreError> + Send + Sync + 'static,
        clock: impl Fn() -> i64 + Send + Sync + 'static,
    ) -> Self {
        Api {
            store_path: Arc::new(store_path),
            clock: Arc::new(clock),
            live: None,
            stopping: Arc::default(),
            health: Arc::default(),
        }
    }

    /// The live feed's tail for `GET /api/live`, and the flag that ends the open streams.
    pub(crate) fn with_live(mut self, live: Option<SharedTail>, stopping: Arc<AtomicBool>) -> Self {
        self.live = live;
        self.stopping = stopping;
        self
    }

    /// The time a store check is reused (a test sets it to 0 to see a second check run).
    #[cfg(test)]
    pub(crate) fn with_health_ttl(mut self, ttl: Duration) -> Self {
        self.health = Arc::new(HealthCache {
            ttl,
            ..HealthCache::default()
        });
        self
    }

    /// `EASY_PERSONAL_HOME` / `HOME` read on each request (like Python's `config.store_path()`), and the system clock.
    pub fn system() -> Self {
        Api::new(store::store_path, now_micros)
    }
}

pub fn routes() -> Router<Api> {
    Router::new()
        .route(LIVE_PATH, get(live))
        .route("/api/usage/summary", get(summary))
        .route("/api/usage/breakdown", get(breakdown))
        .route("/api/usage/sessions", get(sessions))
        .route("/api/usage/outliers", get(outliers))
        .route("/api/warnings", get(warnings))
        .route("/api/plan", get(plan))
        .route(
            "/api/workspaces",
            get(workspace_list).post(writes::workspace_add),
        )
        .route(
            "/api/workspaces/unassigned",
            get(workspaces_unassigned)
                .patch(writes::workspace_edit)
                .delete(writes::workspace_delete),
        )
        .route(
            "/api/workspaces/{id}",
            patch(writes::workspace_edit).delete(writes::workspace_delete),
        )
        .route("/api/connectors", get(connectors))
        .route("/api/kit", get(kit_projects))
        .route("/api/kit/workspaces", get(kit_workspaces))
        .route("/api/kit/reports", get(kit_reports))
        .route("/api/kit/report-files", get(kit_report_files))
        .route("/api/kit/report-file", get(kit_report_file))
        .route("/api/reports", get(report_list))
        .route("/api/reports/{id}", get(report_one))
        .route("/api/board", get(board_view))
        .route("/api/memory-graph", get(memory_graph))
        .route(
            "/api/store/health",
            get(store_health_get).head(store_health_head),
        )
        .route("/api/first-run", get(first_run_steps))
        .route("/api/accounts", get(account_list).post(writes::account_add))
        .route("/api/accounts/{id}/confirm", post(writes::account_confirm))
        .route("/api/accounts/{id}/ignore", post(writes::account_ignore))
        .route(
            "/api/accounts/{id}/workspaces",
            put(writes::account_workspaces),
        )
        .route("/api/sync", get(sync_plan))
        .route("/api/dirs", get(dirs))
        .route("/api/jobs", get(jobs))
}

/// `GET /api/live` (T056): Python's `live_stream`, served by `live_feed::SseBody`. A repeated `Last-Event-ID` is
/// refused like a repeated Host or token header.
async fn live(State(api): State<Api>, headers: HeaderMap) -> Response {
    let mut ids = headers.get_all("last-event-id").iter();
    let resume = match (ids.next(), ids.next()) {
        (None, _) => None,
        (Some(id), None) => resume_id(id.as_bytes()),
        _ => {
            return (
                StatusCode::FORBIDDEN,
                [(CONTENT_TYPE, "application/json")],
                r#"{"error":"forbidden"}"#,
            )
                .into_response();
        }
    };
    let body = SseBody::new(resume, api.live, api.stopping, api.clock);
    (
        [
            (CONTENT_TYPE, "text/event-stream"),
            (CACHE_CONTROL, "no-store"),
            (HeaderName::from_static("x-accel-buffering"), "no"),
        ],
        Body::from_stream(body),
    )
        .into_response()
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

async fn kit_projects(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, q, now| {
        kit::kit_summary(conn, now, q.scope(), true).map_err(metrics_failure)
    })
    .await
}

/// Takes `since`, `until` and `account`; a `workspace` is ignored, as Python's route ignores it.
async fn kit_workspaces(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, q, _now| {
        kit::workspace_rollup(conn, q.get("since"), q.get("until"), q.scope().account)
            .map_err(metrics_failure)
    })
    .await
}

async fn kit_reports(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, _q, _now| {
        kit::report_folders(conn).map_err(metrics_failure)
    })
    .await
}

/// `GET /api/kit/report-files` (T045): the files of one report folder, `workspace`, `repo_no` and `folder` from the query
/// (`repo_no` is plain ASCII digits, else a miss). Every miss is the same fixed 404 `not found`.
async fn kit_report_files(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, q, _now| {
        let repo_no = q.get("repo_no").and_then(parse_repo_no);
        report_files::report_files(conn, q.get("workspace"), repo_no, q.get("folder"))
            .map_err(report_failure)
    })
    .await
}

/// `GET /api/kit/report-file` (T045): one file of a report folder, its bytes with Python's content type and headers
/// (`nosniff`, `no-store`, `inline`; the server layer adds the CSP). Opened by handle with no symlink followed, in
/// `report_files.rs`; a refusal is the fixed 404, a file over the cap the fixed 413.
async fn kit_report_file(State(api): State<Api>, uri: Uri) -> Response {
    let query = Query::parse(uri.query().unwrap_or(""));
    let outcome = tokio::task::spawn_blocking(move || {
        let path = (api.store_path)().map_err(store_failure)?;
        let conn = store::open_checked(&path).map_err(store_failure)?;
        let repo_no = query.get("repo_no").and_then(parse_repo_no);
        report_files::report_file(
            &conn,
            query.get("workspace"),
            repo_no,
            query.get("folder"),
            query.get("path"),
        )
        .map_err(report_failure)
    })
    .await;
    match outcome {
        Ok(Ok((content_type, data))) => (
            StatusCode::OK,
            [
                (CONTENT_TYPE, content_type),
                (X_CONTENT_TYPE_OPTIONS, "nosniff"),
                (CACHE_CONTROL, "no-store"),
                (CONTENT_DISPOSITION, "inline"),
            ],
            data,
        )
            .into_response(),
        Ok(Err((status, text))) => respond(status, &error_body(&text)),
        Err(_) => respond(StatusCode::INTERNAL_SERVER_ERROR, &error_body(INTERNAL)),
    }
}

/// `GET /api/connectors` (T046): the catalog and the stored connector rows. Read-only like the workspace routes (Python's
/// `POST` on it is a 405 here); presence is never asked (`suggestions::configured_ids`: no keychain, no `claude`).
async fn connectors(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, _q, _now| {
        suggestions::connectors_list(conn).map_err(view_failure)
    })
    .await
}

/// `GET /api/board` (T043): the kanban of each project folder in the `workspace` / `account` scope.
async fn board_view(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, q, _now| {
        board::board(conn, q.scope()).map_err(metrics_failure)
    })
    .await
}

/// `GET /api/memory-graph[?project=]` (T043): no `project` (or an empty one) lists the projects that have memory notes;
/// with one, that project's nodes and edges (an unknown one is a 400 `unknown project`).
async fn memory_graph(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, q, now| {
        let env = Env::system(now);
        match q.get("project").filter(|p| !p.is_empty()) {
            Some(project) => memgraph::project_graph(conn, &env, project),
            None => memgraph::project_list(conn, &env),
        }
        .map_err(metrics_failure)
    })
    .await
}

async fn report_list(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, _q, _now| {
        reports::list_reports(conn).map_err(reports_failure)
    })
    .await
}

/// The id is read from the path as Python's `raw.isascii() and raw.isdecimal() and len(raw) < 19` does, after the
/// percent-decoding Starlette does: plain ASCII digits, fewer than 19. Any other text (a byte that is not UTF-8
/// included), and an id with no report, is the same 404 with the same body.
async fn report_one(State(api): State<Api>, uri: Uri) -> Response {
    let id = report_id(uri.path().strip_prefix("/api/reports/").unwrap_or(""));
    serve(api, uri, move |conn, _q, now| {
        let found = match id {
            Some(id) => reports::get_report(conn, id, now).map_err(reports_failure)?,
            None => None,
        };
        found.ok_or((StatusCode::NOT_FOUND, "no such report".to_string()))
    })
    .await
}

/// `segment` is the path part as sent (percent-encoded); `None` unless it decodes to 1 to 18 ASCII digits.
pub(crate) fn report_id(segment: &str) -> Option<i64> {
    let bytes = segment.as_bytes();
    let hex = |b: Option<&u8>| b.and_then(|b| (*b as char).to_digit(16));
    let (mut digits, mut i) = (String::new(), 0);
    while i < bytes.len() {
        let byte = match (bytes[i], hex(bytes.get(i + 1)), hex(bytes.get(i + 2))) {
            (b'%', Some(high), Some(low)) => {
                i += 2;
                (high * 16 + low) as u8
            }
            (byte, _, _) => byte,
        };
        if !byte.is_ascii_digit() {
            return None;
        }
        digits.push(byte as char);
        i += 1;
    }
    if digits.is_empty() || digits.len() > 18 {
        return None;
    }
    digits.parse().ok()
}

/// The unit folder is read from `XDG_CONFIG_HOME` / `HOME` on each request, like Python's `schedule.unit_dir()`.
async fn first_run_steps(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, _q, _now| {
        first_run::steps(conn, first_run::unit_dir().as_deref())
            .map_err(|e| store_failure(StoreError::from(e)))
    })
    .await
}

/// `GET /api/accounts?by=account|provider` (T041): the stored accounts with their status lines. Read-only and with no
/// account detection (Python's handler INSERTs new account rows first; here that stays in `ingest`). `by` is `account`
/// when absent; its writes are in `writes.rs` (T048). The agent root (`EASY_PERSONAL_AGENT_ROOT`,
/// else `HOME`) only decides `is_default`.
async fn account_list(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, q, now| {
        let by = q.get("by").unwrap_or("account");
        accounts::grouped(conn, by, now, &agent_root()).map_err(metrics_failure)
    })
    .await
}

/// `GET /api/sync?account=<id>` (T041): the sync plan, no `claude` process and no keychain (see `core/src/sync.rs`).
/// Python's `POST /api/sync` (apply) is a 405 here.
async fn sync_plan(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, q, _now| {
        let account = sync::account_arg(q.get("account")).map_err(sync_failure)?;
        sync::plan_json(conn, account).map_err(sync_failure)
    })
    .await
}

/// Python's `config.agent_root()`: `EASY_PERSONAL_AGENT_ROOT`, else the home folder (an empty value counts as unset).
pub(crate) fn agent_root() -> PathBuf {
    let set = |name| std::env::var_os(name).filter(|v| !v.is_empty());
    set("EASY_PERSONAL_AGENT_ROOT")
        .or_else(|| set("HOME"))
        .map(PathBuf::from)
        .unwrap_or_default()
}

/// `GET /api/dirs?path=` (T044): sub-folder names for the folder browser. No store is opened; the path is never echoed.
async fn dirs(uri: Uri) -> Response {
    let path = Query::parse(uri.query().unwrap_or(""))
        .get("path")
        .map(str::to_string);
    match tokio::task::spawn_blocking(move || folders::list_dirs(path.as_deref())).await {
        Ok(Ok(body)) => respond(StatusCode::OK, &body),
        Ok(Err(error)) => {
            let status = match error {
                folders::DirsError::Invalid(_) => StatusCode::BAD_REQUEST,
                folders::DirsError::Unreadable => StatusCode::FORBIDDEN,
            };
            respond(status, &error_body(error.text()))
        }
        Err(_) => respond(StatusCode::INTERNAL_SERVER_ERROR, &error_body(INTERNAL)),
    }
}

/// `GET /api/jobs` (T044): the scheduled agent jobs. No store is opened; only `systemctl --user list-timers|show` and
/// `crontab -l` may run (`jobs::READ_ONLY`).
async fn jobs(State(api): State<Api>) -> Response {
    let now = (api.clock)();
    match tokio::task::spawn_blocking(move || easy_personal_core::jobs::list_jobs_here(now)).await {
        Ok(body) => respond(StatusCode::OK, &body),
        Err(_) => respond(StatusCode::INTERNAL_SERVER_ERROR, &error_body(INTERNAL)),
    }
}

/// How long a store check is reused (Python's `HEALTH_CACHE_S`).
const HEALTH_CACHE: Duration = Duration::from_secs(5);

/// The last answer of `GET /api/store/health` per store path. The lock is held for the whole check, so one check runs at a
/// time: a request that comes meanwhile waits, then gets that answer from the cache.
struct HealthCache {
    answers: Mutex<HashMap<PathBuf, (Instant, Value)>>,
    ttl: Duration,
    /// How many checks ran (a test counts them; the route reads nothing else from it).
    #[cfg(test)]
    runs: std::sync::atomic::AtomicUsize,
    /// The options of every check that ran (a test proves the route never asks for `--full`).
    #[cfg(test)]
    used: Mutex<Vec<store_health::Options>>,
}

impl Default for HealthCache {
    fn default() -> Self {
        HealthCache {
            answers: Mutex::default(),
            ttl: HEALTH_CACHE,
            #[cfg(test)]
            runs: Default::default(),
            #[cfg(test)]
            used: Mutex::default(),
        }
    }
}

impl HealthCache {
    /// The answer for `path`: the cached one when it is younger than the time-to-live, else a new check
    /// (default options: the 1.5 s budget; never `--full`).
    fn answer(&self, path: &Path) -> Value {
        let mut answers = self.answers.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((at, value)) = answers.get(path)
            && at.elapsed() < self.ttl
        {
            return value.clone();
        }
        #[cfg(test)]
        self.runs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let options = store_health::Options::default(); // never `full`: that is for the command line only
        #[cfg(test)]
        self.used.lock().unwrap().push(options.clone());
        let value = store_health::check(path, &options, &Arc::default()).json();
        answers.insert(path.to_path_buf(), (Instant::now(), value.clone()));
        value
    }
}

/// `GET /api/store/health` (T059, FR-091): the read-only store check of Python's HUN-244, the same JSON as
/// `store-check --json`. A store that is missing, empty or damaged is a 200 answer (the check says so); only "no data folder"
/// is a 503. Never writes and never opens the store for writing; the answer holds no path and no row.
async fn store_health_get(State(api): State<Api>) -> Response {
    let outcome = tokio::task::spawn_blocking(move || {
        let path = (api.store_path)().map_err(store_failure)?;
        Ok::<_, Failure>(api.health.answer(&path))
    })
    .await;
    match outcome {
        Ok(Ok(body)) => (
            StatusCode::OK,
            [
                (CONTENT_TYPE, "application/json"),
                (CACHE_CONTROL, "no-store"),
            ],
            body.to_string(),
        )
            .into_response(),
        Ok(Err((status, text))) => respond(status, &error_body(&text)),
        Err(_) => respond(StatusCode::INTERNAL_SERVER_ERROR, &error_body(INTERNAL)),
    }
}

/// `HEAD`: headers only, and no check runs.
async fn store_health_head() -> Response {
    (StatusCode::OK, [(CACHE_CONTROL, "no-store")]).into_response()
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

pub(crate) fn error_body(text: &str) -> Value {
    json!({ "error": text })
}

pub(crate) fn respond(status: StatusCode, body: &Value) -> Response {
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
pub(crate) fn store_failure(error: StoreError) -> Failure {
    let text = match &error {
        StoreError::NoStore | StoreError::Newer { .. } | StoreError::NoHome => error.to_string(),
        StoreError::Busy | StoreError::NotAFile | StoreError::Unreadable(_) => {
            UNREADABLE.to_string()
        }
    };
    (StatusCode::SERVICE_UNAVAILABLE, text)
}

/// A catalog that does not parse cannot happen in a shipped build (a test parses every embedded file): fixed 500.
fn view_failure(error: ViewError) -> Failure {
    match error {
        ViewError::Store(error) => store_failure(error),
        ViewError::Catalog(_) => internal(),
    }
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

/// The misses of the report-folder routes have fixed bodies that never echo the input (FR-090).
fn report_failure(error: ReportError) -> Failure {
    match error {
        ReportError::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
        ReportError::TooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "too large".to_string()),
        ReportError::Store(error) => metrics_failure(error),
    }
}

/// 400 for an `account` that is not an id, 404 for one no account has: Python's texts, which name none of the input.
fn sync_failure(error: SyncError) -> Failure {
    match error {
        SyncError::Invalid(text) => (StatusCode::BAD_REQUEST, text),
        SyncError::NotFound(text) => (StatusCode::NOT_FOUND, text),
        SyncError::Store(error) => store_failure(error),
    }
}

fn reports_failure(error: ReportsError) -> Failure {
    match error {
        ReportsError::Corrupt => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
        ReportsError::Rules(error) => rules_failure(error),
        ReportsError::Store(error) => store_failure(error),
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
