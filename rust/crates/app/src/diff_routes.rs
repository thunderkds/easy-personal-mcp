//! `GET /api/diff/repo`, `/api/diff/files` and `/api/diff/file` (T136, FR-139, FR-140, DDR-0010): the changes of one of
//! the owner's repos, read by the core (`repo_diff.rs`) anew for every request. **The seventh exception of NFR-002**: the
//! answer's body is the only place file contents and git names go; nothing is kept, logged or counted, and no write route
//! exists. The guard in `server.rs` covers the routes.
//!
//! A request never names a folder. It names `workspace` + `repo_no` (a stored workspace repo) or `session` (the work tree
//! of the newest folder that session's transcript records, found as the transcript route finds the file), and `tree` (an
//! id from `/api/diff/repo`) picks another work tree of that repo. Every refusal is a fixed line without a path.

use crate::api::{Api, Query, error_body};
use crate::app_session::ResumeId;
use crate::transcript_route::{answer, main_file};
use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, Uri};
use axum::response::Response;
use axum::routing::get;
use easy_personal_core::repo_diff::{self, Compare, DiffError, Worktree};
use easy_personal_core::report_files::parse_repo_no;
use easy_personal_core::{store, transcript, workspaces};
use serde_json::{Value, json};
use std::path::PathBuf;

const PICK: &str = "name a repo: workspace and repo_no, or session";
const NO_REPO: &str = "no such repo";
const NO_TREE: &str = "no such work tree";
const NO_SESSION: &str = "no such session";
const NO_FOLDER: &str = "the folder of this session is not known";

type Refusal = (StatusCode, &'static str);
/// The repo's work trees, and whether some were left out.
type Trees = (Vec<Worktree>, bool);

pub(crate) fn routes() -> Router<Api> {
    Router::new()
        .route("/api/diff/repo", get(repo))
        .route("/api/diff/files", get(files))
        .route("/api/diff/file", get(file))
}

fn refusal(error: DiffError) -> Refusal {
    let status = match error {
        DiffError::NotARepo | DiffError::NotListed => StatusCode::NOT_FOUND,
        DiffError::BadCompare
        | DiffError::UnknownRef
        | DiffError::NoBase
        | DiffError::NoCommonHistory => StatusCode::BAD_REQUEST,
        DiffError::Filter => StatusCode::CONFLICT,
        DiffError::TooSlow => StatusCode::SERVICE_UNAVAILABLE,
        DiffError::Failed => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, error.text())
}

/// The picked work tree and every work tree of its repo.
fn picked(api: &Api, q: &Query) -> Result<(PathBuf, Trees), Refusal> {
    let value = |key| q.get(key).filter(|v| !v.is_empty());
    let folder = match (value("session"), value("workspace"), value("repo_no")) {
        (Some(id), None, None) => {
            let unknown = (StatusCode::NOT_FOUND, NO_SESSION);
            let id = ResumeId::new(id).ok_or(unknown)?;
            let from_tail = api.live_session_origin(id.as_str()).map(|(path, _)| path);
            let main = main_file(api, id.as_str(), from_tail).map_err(|_| unknown)?;
            PathBuf::from(transcript::last_folder(&main).ok_or((StatusCode::NOT_FOUND, NO_FOLDER))?)
        }
        (None, Some(workspace), Some(repo_no)) => {
            let missing = (StatusCode::NOT_FOUND, NO_REPO);
            let repo_no = parse_repo_no(repo_no).ok_or(missing)?;
            let failed = (StatusCode::INTERNAL_SERVER_ERROR, crate::api::INTERNAL);
            let conn = store::open_checked(&(api.store_path)().map_err(|_| failed)?)
                .map_err(|_| failed)?;
            PathBuf::from(
                workspaces::repo_at(&conn, workspace, repo_no)
                    .map_err(|_| failed)?
                    .ok_or(missing)?,
            )
        }
        _ => return Err((StatusCode::BAD_REQUEST, PICK)),
    };
    let here = repo_diff::toplevel(&folder).ok_or_else(|| refusal(DiffError::NotARepo))?;
    let (trees, cut) = repo_diff::worktrees(&here).map_err(refusal)?;
    let tree = match value("tree") {
        Some(id) => trees
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.path.clone())
            .ok_or((StatusCode::NOT_FOUND, NO_TREE))?,
        None => here,
    };
    Ok((tree, (trees, cut)))
}

/// Runs `read` on a blocking thread and answers its JSON (`no-store`) or its fixed refusal.
async fn serve(
    api: Api,
    uri: Uri,
    read: impl FnOnce(&Query, PathBuf, Trees) -> Result<Value, Refusal> + Send + 'static,
) -> Response {
    let q = Query::parse(uri.query().unwrap_or(""));
    let outcome = tokio::task::spawn_blocking(move || {
        picked(&api, &q).and_then(|(tree, trees)| read(&q, tree, trees))
    })
    .await;
    match outcome {
        Ok(Ok(body)) => answer(StatusCode::OK, &body),
        Ok(Err((status, text))) => answer(status, &error_body(text)),
        Err(_) => answer(
            StatusCode::INTERNAL_SERVER_ERROR,
            &error_body(crate::api::INTERNAL),
        ),
    }
}

/// `{name, tree, worktrees: [{id, folder, branch, head, main}], branches, base, base_shown, commits, truncated}`
/// (`truncated`: some branches, tags or work trees were left out).
async fn repo(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |_, tree, (trees, cut)| {
        let mut body = repo_diff::overview(&tree).map_err(refusal)?;
        body["truncated"] = json!(cut || body["truncated"] == true);
        body["name"] = json!(trees.first().map(|t| t.folder.as_str()));
        body["tree"] = json!(trees.iter().find(|t| t.path == tree).map(|t| t.id.as_str()));
        body["worktrees"] = trees
            .iter()
            .map(|t| {
                json!({"id": t.id, "folder": t.folder, "branch": t.branch, "head": t.head,
                       "main": t.main})
            })
            .collect();
        Ok(body)
    })
    .await
}

fn compare(q: &Query) -> Result<Compare, Refusal> {
    Compare::parse(q.get("compare"), q.get("base"), q.get("head")).map_err(refusal)
}

async fn files(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |q, tree, _| {
        repo_diff::files(&tree, &compare(q)?).map_err(refusal)
    })
    .await
}

async fn file(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |q, tree, _| {
        let (area, path) = (q.get("area").unwrap_or(""), q.get("path").unwrap_or(""));
        repo_diff::file(&tree, &compare(q)?, area, path).map_err(refusal)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::router;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tower::ServiceExt;

    const TOKEN: &str = "test-token";
    const NOW: i64 = 1_768_867_200_000_000;
    const OK: [(&str, &str); 2] = [("host", "127.0.0.1:4243"), ("x-dashboard-token", TOKEN)];

    struct Temp(PathBuf);

    impl Temp {
        fn new() -> Temp {
            static N: AtomicUsize = AtomicUsize::new(0);
            let dir = std::env::temp_dir().join(format!(
                "t136-routes-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Temp(dir.canonicalize().unwrap())
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn git(repo: &Path, args: &[&str]) {
        let ok = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args([
                "-c",
                "user.name=Ann",
                "-c",
                "user.email=ann@example.invalid",
            ])
            .args(args)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?}");
    }

    /// A repo `proj` with one commit, one unstaged change and a second work tree `side`; the parity store with workspace
    /// `t136` holding it, and its first Claude session pointed at a transcript whose newest folder is in `side`.
    fn world(temp: &Temp) -> (Router, String) {
        let repo = temp.0.join("proj");
        fs::create_dir_all(repo.join("sub")).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        fs::write(repo.join("a.txt"), "one\n").unwrap();
        fs::write(repo.join("sub/b.txt"), "b\n").unwrap();
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "-q", "-m", "first"]);
        let side = temp.0.join("side");
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "topic",
                side.to_str().unwrap(),
            ],
        );
        fs::write(repo.join("a.txt"), "SECRET-CHANGE\n").unwrap();
        fs::write(side.join("a.txt"), "from the side tree\n").unwrap();

        let projects = temp.0.join("projects/p");
        fs::create_dir_all(&projects).unwrap();
        let transcript = projects.join("s.jsonl");
        let record = |cwd: &Path| format!("{}\n", json!({"type": "user", "cwd": cwd}));
        let side_record = format!(
            "{}\n",
            json!({"type": "user", "cwd": repo, "isSidechain": true})
        );
        fs::write(
            &transcript,
            record(&repo) + &record(&side.join("sub")) + &side_record,
        )
        .unwrap();

        let store = temp.0.join("store.sqlite");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/store-v11.sqlite"),
            &store,
        )
        .unwrap();
        let conn = rusqlite::Connection::open(&store).unwrap();
        conn.execute(
            "INSERT INTO agent_accounts (provider, folder, label, status) VALUES ('claude', ?1, 'T136', 'confirmed')",
            [temp.0.display().to_string()],
        )
        .unwrap();
        conn.execute("INSERT INTO workspaces (name) VALUES ('t136')", [])
            .unwrap();
        conn.execute(
            "INSERT INTO workspace_repos VALUES (?1, ?2)",
            rusqlite::params![conn.last_insert_rowid(), repo.display().to_string()],
        )
        .unwrap();
        let session: String = conn
            .query_row(
                "SELECT session_id FROM events WHERE source = 'claude_code' AND COALESCE(is_sidechain,0) = 0 LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        conn.execute(
            "UPDATE events SET source_path = ?1 WHERE session_id = ?2",
            rusqlite::params![transcript.display().to_string(), session],
        )
        .unwrap();
        let api = Api::new(move || Ok(store.clone()), || NOW);
        (router(TOKEN.into(), 4243, api), session)
    }

    async fn send(
        app: &Router,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
    ) -> (StatusCode, Option<String>, Value) {
        let mut request = Request::builder().method(method).uri(path);
        for (k, v) in headers {
            request = request.header(*k, *v);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let cache = response
            .headers()
            .get("cache-control")
            .map(|v| v.to_str().unwrap().to_owned());
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            cache,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn ok(app: &Router, path: &str) -> Value {
        let (status, cache, body) = send(app, "GET", path, &OK).await;
        assert_eq!(status, StatusCode::OK, "{path}: {body}");
        assert_eq!(cache.as_deref(), Some("no-store"), "{path}");
        body
    }

    #[tokio::test]
    async fn a_workspace_repo_is_listed_and_read_and_another_work_tree_is_picked_by_id() {
        let temp = Temp::new();
        let (app, _) = world(&temp);
        let head = ok(&app, "/api/diff/repo?workspace=t136&repo_no=0").await;
        assert_eq!(head["name"], "proj");
        assert_eq!(head["base"], "main");
        assert_eq!(head["worktrees"].as_array().unwrap().len(), 2);
        assert_eq!(head["tree"], head["worktrees"][0]["id"]);
        assert_eq!(head["truncated"], false);
        assert!(!head.to_string().contains(temp.0.to_str().unwrap()));

        let base = "workspace=t136&repo_no=0&compare=uncommitted";
        let list = ok(&app, &format!("/api/diff/files?{base}")).await;
        assert_eq!(list["files"][0]["path"], "a.txt");
        assert!(!list.to_string().contains("SECRET-CHANGE"));
        let one = ok(
            &app,
            &format!("/api/diff/file?{base}&area=unstaged&path=a.txt"),
        )
        .await;
        assert!(one["patch"].as_str().unwrap().contains("+SECRET-CHANGE"));

        let side = head["worktrees"][1]["id"].as_str().unwrap();
        let one = ok(
            &app,
            &format!("/api/diff/file?{base}&tree={side}&area=unstaged&path=a.txt"),
        )
        .await;
        assert!(
            one["patch"]
                .as_str()
                .unwrap()
                .contains("+from the side tree")
        );
        // the planted change reached the answer and nothing the app keeps
        for name in ["store.sqlite", "store.sqlite-wal"] {
            let kept = fs::read(temp.0.join(name)).unwrap_or_default();
            assert!(!kept.windows(13).any(|w| w == b"SECRET-CHANGE"), "{name}");
        }
    }

    #[tokio::test]
    async fn a_session_opens_the_work_tree_of_its_newest_main_thread_folder() {
        let temp = Temp::new();
        let (app, session) = world(&temp);
        let head = ok(&app, &format!("/api/diff/repo?session={session}")).await;
        assert_eq!(head["name"], "proj");
        assert_eq!(head["tree"], head["worktrees"][1]["id"]);
        let one = ok(
            &app,
            &format!(
                "/api/diff/file?session={session}&compare=uncommitted&area=unstaged&path=a.txt"
            ),
        )
        .await;
        assert!(
            one["patch"]
                .as_str()
                .unwrap()
                .contains("+from the side tree")
        );
    }

    #[tokio::test]
    async fn a_bad_pick_ref_path_or_method_is_refused_with_a_fixed_line() {
        let temp = Temp::new();
        let (app, session) = world(&temp);
        let repo = "workspace=t136&repo_no=0";
        for (path, status) in [
            ("/api/diff/repo".to_string(), 400),
            (format!("/api/diff/repo?{repo}&session={session}"), 400),
            ("/api/diff/repo?workspace=t136".to_string(), 400),
            ("/api/diff/repo?workspace=t136&repo_no=1".to_string(), 404),
            ("/api/diff/repo?workspace=t136&repo_no=-1".to_string(), 404),
            ("/api/diff/repo?workspace=orphan&repo_no=0".to_string(), 404),
            ("/api/diff/repo?workspace=none&repo_no=0".to_string(), 404),
            ("/api/diff/repo?session=..%2F..".to_string(), 404),
            (
                "/api/diff/repo?session=00000000-0000-4000-8000-000000000000".to_string(),
                404,
            ),
            (format!("/api/diff/repo?{repo}&tree=0000000000000000"), 404),
            (format!("/api/diff/files?{repo}"), 400),
            (
                format!("/api/diff/files?{repo}&compare=range&base=--output%3Dx&head=main"),
                400,
            ),
            (
                format!(
                    "/api/diff/file?{repo}&compare=uncommitted&area=unstaged&path=..%2Fstore.sqlite"
                ),
                404,
            ),
        ] {
            let (got, _, body) = send(&app, "GET", &path, &OK).await;
            assert_eq!(got.as_u16(), status, "{path}: {body}");
            let line = body["error"].as_str().unwrap();
            assert!(!line.contains('/') && !line.contains("--output"), "{line}");
        }
        for method in ["POST", "PUT", "DELETE"] {
            let (got, _, _) = send(&app, method, &format!("/api/diff/files?{repo}"), &OK).await;
            assert_eq!(got, StatusCode::METHOD_NOT_ALLOWED, "{method}");
        }
    }

    #[tokio::test]
    async fn the_guard_covers_the_routes() {
        let temp = Temp::new();
        let (app, _) = world(&temp);
        for route in ["repo", "files", "file"] {
            let path = format!("/api/diff/{route}?workspace=t136&repo_no=0");
            let query_token = format!("{path}&token={TOKEN}");
            for (path, headers) in [
                (&path, vec![OK[0]]),
                (&query_token, vec![OK[0]]),
                (&path, vec![("host", "evil.example"), OK[1]]),
                (&path, vec![OK[0], OK[1], ("origin", "http://evil.example")]),
            ] {
                let (got, _, body) = send(&app, "GET", path, &headers).await;
                assert_eq!(got, StatusCode::FORBIDDEN, "{path}: {body}");
            }
        }
    }
}
