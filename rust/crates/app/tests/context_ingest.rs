//! T051: `easy-personal-rs ingest` and the `context` source through the real binary. The service is a fake on a free
//! `127.0.0.1` port (`core/tests/context_fake`), or a closed port. `HOME`, `EASY_PERSONAL_HOME` and
//! `EASY_PERSONAL_AGENT_ROOT` are inside one temp folder; the owner's folders and any real service are never reached
//! (`EASY_PERSONAL_CONTEXT_URL` always names the fake or a closed port).

#[path = "../../core/tests/context_fake/mod.rs"]
mod fake;

use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

struct Case {
    dir: PathBuf,
}

impl Case {
    /// A temp folder with the fixture transcript, whose project folder (`<dir>/proj`) exists.
    fn new() -> Case {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t051-ctx-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let projects = dir.join("agent/.claude/projects/p");
        fs::create_dir_all(&projects).unwrap();
        fs::create_dir_all(dir.join("proj")).unwrap();
        fs::create_dir_all(dir.join("nowhere")).unwrap();
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/fixtures/claude_code/2.1.295/session.jsonl");
        let text = fs::read_to_string(fixture).unwrap();
        let text = text.replace("/work/demo-app", dir.join("proj").to_str().unwrap());
        fs::write(projects.join("s.jsonl"), text).unwrap();
        Case { dir }
    }

    fn ingest(&self, url: &str) -> (Option<i32>, Value, String) {
        let out = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
            .arg("ingest")
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", self.dir.join("nowhere"))
            .env("EASY_PERSONAL_HOME", self.dir.join("home"))
            .env("EASY_PERSONAL_AGENT_ROOT", self.dir.join("agent"))
            .env("EASY_PERSONAL_CONTEXT_URL", url)
            .env("TZ", "UTC")
            .output()
            .unwrap();
        let stdout: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
        (
            out.status.code(),
            stdout,
            String::from_utf8(out.stderr).unwrap(),
        )
    }

    fn rows(&self) -> Vec<(String, Option<String>)> {
        let conn = Connection::open_with_flags(
            self.dir.join("home/store.sqlite"),
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let mut query = conn
            .prepare("SELECT project_path, last_index_at_utc FROM context_index ORDER BY 1")
            .unwrap();
        query
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn dead_url() -> String {
    let port = TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    format!("http://127.0.0.1:{port}/mcp")
}

#[test]
fn a_service_that_is_down_is_no_data_and_the_run_exits_0() {
    let case = Case::new();
    let (code, out, stderr) = case.ingest(&dead_url());
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(out["context"], json!({"status": "no data"}));
    assert!(
        out.get("not_ingested").is_none(),
        "context is not listed as not ingested any more"
    );
    assert!(
        out["new_events"].as_i64().unwrap() > 0,
        "the other sources still ran"
    );
    assert_eq!(case.rows(), vec![]);
}

#[test]
fn a_refused_url_is_no_data_and_the_run_exits_0() {
    let case = Case::new();
    for url in [
        "http://localhost:1/mcp",
        "http://example.invalid/mcp",
        "https://127.0.0.1:1/mcp",
        "nonsense",
    ] {
        let (code, out, _) = case.ingest(url);
        assert_eq!(code, Some(0), "{url}");
        assert_eq!(out["context"], json!({"status": "no data"}), "{url}");
    }
}

#[test]
fn the_service_gives_the_stored_freshness_and_nothing_else_is_printed() {
    let case = Case::new();
    let root = case.dir.join("proj");
    let canary = "LEAKCANARY-context-0123456789";
    let script = json!({"mode": "json", "session": false, "reply": "structured", "reject_roots": [],
    "delay_s": 0, "break": null, "indexes": [
        {"root": root, "root_exists": true, "last_index_at": "2026-10-21T10:00:00.000+00:00",
         "files": 3, "slices": 9, "status": "done", "summary": canary},
        {"root": case.dir.join("other"), "root_exists": true, "last_index_at": "2026-10-21T10:00:00+00:00"}
    ]});
    let service = fake::Fake::start(script);
    let (code, out, stderr) = case.ingest(&format!("http://127.0.0.1:{}/mcp", service.port));
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(out["context"], json!({"status": "ok", "indexes": 1}));
    assert_eq!(
        case.rows(),
        vec![(
            root.to_str().unwrap().to_string(),
            Some("2026-10-21T10:00:00+00:00".to_string())
        )]
    );
    let seen = service.seen.lock().unwrap();
    assert_eq!(
        seen.calls,
        ["list_indexes"],
        "only the allow-listed read tool is sent"
    );
    assert_eq!(seen.roots, [Some(root.to_str().unwrap().to_string())]);
    assert!(
        !out.to_string().contains(canary) && !stderr.contains(canary),
        "index content is never printed"
    );
}
