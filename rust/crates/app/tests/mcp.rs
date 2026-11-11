//! T008: runs the real `mcp` binary and talks newline-delimited JSON-RPC to it over pipes (no lib.rs needed).
//! The oracle is the Python MCP goldens `rust/parity/golden/mcp__*.json`, made over `rust/parity/store-v11.sqlite`.
//! Each test serves a temp copy of that store through `EASY_PERSONAL_HOME`; `~/.easy-personal` is never reached.
//! The goldens were made under `TZ=UTC` and a `day` group is a local day, so the spawned server runs under `TZ=UTC`
//! whatever zone the test machine is in (T019).

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(10);

fn parity_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity")
}

fn golden(name: &str) -> Value {
    let text =
        std::fs::read_to_string(parity_dir().join("golden").join(format!("{name}.json"))).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// A temp folder that is `EASY_PERSONAL_HOME` and `HOME` of one server run.
struct Home(PathBuf);

impl Home {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t008-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Home(dir)
    }

    fn with_store() -> Self {
        let home = Home::new();
        home.put_store();
        home
    }

    fn store(&self) -> PathBuf {
        self.0.join("store.sqlite")
    }

    fn put_store(&self) {
        std::fs::copy(parity_dir().join("store-v11.sqlite"), self.store()).unwrap();
    }

    /// A home holding a copy of the rich store (the one with workspaces and two accounts).
    fn with_rich_store() -> Self {
        let home = Home::new();
        std::fs::copy(
            parity_dir().join("store-rich/store-v11.sqlite"),
            home.store(),
        )
        .unwrap();
        home
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Mcp {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    next_id: u64,
}

impl Mcp {
    fn start(home: &Home) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
            .arg("mcp")
            .env("HOME", &home.0)
            .env("EASY_PERSONAL_HOME", &home.0)
            .env("TZ", "UTC")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (tx, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(line) => {
                        if tx.send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Mcp {
            child,
            stdin,
            lines,
            next_id: 1,
        }
    }

    fn send_raw(&mut self, text: &str) {
        let stdin = self.stdin.as_mut().unwrap();
        stdin.write_all(text.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
    }

    fn send(&mut self, message: &Value) {
        self.send_raw(&message.to_string());
    }

    /// The next stdout line, which must be one JSON-RPC message (nothing else may be on stdout).
    fn recv(&mut self) -> Value {
        let line = match self.lines.recv_timeout(WAIT) {
            Ok(line) => line,
            Err(RecvTimeoutError::Timeout) => panic!("no answer within {WAIT:?}"),
            Err(RecvTimeoutError::Disconnected) => panic!("stdout closed before an answer"),
        };
        let message: Value = serde_json::from_str(&line)
            .unwrap_or_else(|_| panic!("stdout line is not JSON: {line:?}"));
        assert_eq!(
            message["jsonrpc"], "2.0",
            "stdout line is not JSON-RPC: {line}"
        );
        message
    }

    /// Sends a request and returns the answer with the same id.
    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        let answer = self.recv();
        assert_eq!(answer["id"], id, "answer for another request: {answer}");
        answer
    }

    fn initialize(&mut self, version: &str) -> Value {
        let answer = self.request(
            "initialize",
            json!({
                "protocolVersion": version,
                "capabilities": {},
                "clientInfo": {"name": "t008-test", "version": "0"}
            }),
        );
        self.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        answer
    }

    fn started(home: &Home) -> Self {
        let mut mcp = Mcp::start(home);
        let answer = mcp.initialize("2025-06-18");
        assert!(answer["result"].is_object(), "initialize failed: {answer}");
        mcp
    }

    fn call(&mut self, name: &str, arguments: Value) -> Value {
        self.request("tools/call", json!({"name": name, "arguments": arguments}))
    }

    /// Closes stdin, waits for the exit and returns (exit code, rest of stdout, stderr).
    fn finish(mut self) -> (Option<i32>, Vec<String>, String) {
        drop(self.stdin.take());
        let deadline = Instant::now() + WAIT;
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            assert!(
                Instant::now() < deadline,
                "server still running after stdin closed"
            );
            std::thread::sleep(Duration::from_millis(20));
        };
        let rest: Vec<String> = self.lines.try_iter().collect();
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        (status.code(), rest, stderr)
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        // A test that failed before `finish` must not leave the child behind.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn structured(answer: &Value) -> &Value {
    assert_eq!(answer["result"]["isError"], false, "tool failed: {answer}");
    &answer["result"]["structuredContent"]
}

fn error_text(answer: &Value) -> String {
    assert_eq!(
        answer["result"]["isError"], true,
        "expected a tool error: {answer}"
    );
    answer["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn initialize_reports_the_cargo_version_and_echoes_the_proven_protocol_versions() {
    for version in ["2024-11-05", "2025-06-18", "2025-11-25"] {
        let home = Home::with_store();
        let mut mcp = Mcp::start(&home);
        let answer = mcp.initialize(version);
        let result = &answer["result"];
        assert_eq!(result["protocolVersion"], version);
        assert_eq!(result["serverInfo"]["name"], env!("CARGO_PKG_NAME"));
        assert_eq!(result["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
        assert!(result["capabilities"]["tools"].is_object());
    }
}

#[test]
fn the_no_handshake_revision_is_not_enabled() {
    let home = Home::with_store();
    let mut mcp = Mcp::start(&home);
    let answer = mcp.initialize("2026-07-28");
    // Either refused, or negotiated down to a revision that has the handshake: never the newest.
    if answer["result"].is_object() {
        assert_eq!(answer["result"]["protocolVersion"], "2025-11-25");
    } else {
        assert!(answer["error"].is_object(), "{answer}");
    }
}

#[test]
fn tools_list_equals_the_golden_for_each_of_the_nine_tools() {
    let home = Home::with_store();
    let mut mcp = Mcp::started(&home);
    let answer = mcp.request("tools/list", json!({}));
    let tools = answer["result"]["tools"].as_array().unwrap();
    let mut names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    names.sort();
    assert_eq!(
        names,
        [
            "connector_catalog",
            "connector_status",
            "connector_suggestions",
            "improvement_plan",
            "ping",
            "session_outliers",
            "usage_breakdown",
            "usage_summary",
            "warnings"
        ]
    );

    let golden = golden("mcp__tools_list");
    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        let expected = golden
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["name"] == name)
            .unwrap();
        assert_eq!(
            tool["description"], expected["description"],
            "{name} description"
        );
        assert_eq!(
            tool["inputSchema"], expected["inputSchema"],
            "{name} inputSchema"
        );
        assert_eq!(
            tool["outputSchema"], expected["outputSchema"],
            "{name} outputSchema"
        );
    }
}

#[test]
fn ping_equals_the_golden() {
    let home = Home::with_store();
    let mut mcp = Mcp::started(&home);
    let answer = mcp.call("ping", json!({}));
    let expected = golden("mcp__ping__all");
    assert_eq!(structured(&answer), &expected);
    assert_eq!(answer["result"]["content"][0]["text"], expected["result"]);
}

#[test]
fn usage_summary_equals_the_goldens() {
    let home = Home::with_store();
    let mut mcp = Mcp::started(&home);
    let all = mcp.call("usage_summary", json!({}));
    assert_eq!(structured(&all), &golden("mcp__usage_summary__all"));
    // the text block carries the same body as the structured one
    let text: Value =
        serde_json::from_str(all["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(&text, &golden("mcp__usage_summary__all"));

    let window = mcp.call(
        "usage_summary",
        json!({"since": "2026-01-15T10:05:00Z", "until": "2026-01-15T10:15:00Z"}),
    );
    assert_eq!(structured(&window), &golden("mcp__usage_summary__window"));

    // `null` is how a client says "not given" (Python's `None` default)
    let nulls = mcp.call("usage_summary", json!({"since": null, "until": null}));
    assert_eq!(structured(&nulls), &golden("mcp__usage_summary__all"));
}

#[test]
fn usage_breakdown_equals_the_goldens_for_each_dimension() {
    let home = Home::with_store();
    let mut mcp = Mcp::started(&home);
    for by in ["day", "model", "project", "tool"] {
        let answer = mcp.call("usage_breakdown", json!({"by": by}));
        assert_eq!(
            structured(&answer),
            &golden(&format!("mcp__usage_breakdown__{by}")),
            "by={by}"
        );
    }
}

#[test]
fn bad_arguments_are_tool_errors_with_pythons_text() {
    let home = Home::with_store();
    let mut mcp = Mcp::started(&home);
    let answer = mcp.call("usage_breakdown", json!({"by": "bogus"}));
    assert_eq!(
        error_text(&answer),
        "Error executing tool usage_breakdown: by must be one of day, model, project, tool"
    );
    let answer = mcp.call("usage_summary", json!({"since": "garbage"}));
    assert_eq!(
        error_text(&answer),
        "Error executing tool usage_summary: Invalid isoformat string: 'garbage'"
    );
    let answer = mcp.call("usage_breakdown", json!({"by": "day", "until": "nope"}));
    assert_eq!(
        error_text(&answer),
        "Error executing tool usage_breakdown: Invalid isoformat string: 'nope'"
    );
    // the server is still alive after errors
    let answer = mcp.call("ping", json!({}));
    assert_eq!(structured(&answer), &golden("mcp__ping__all"));
}

#[test]
fn an_unknown_tool_and_malformed_json_get_json_rpc_errors_and_the_server_keeps_running() {
    let home = Home::with_store();
    let mut mcp = Mcp::started(&home);

    let answer = mcp.call("no_such_tool", json!({}));
    assert!(answer["error"].is_object(), "{answer}");
    assert!(answer["result"].is_null());

    mcp.send_raw("{this is not json");
    let answer = mcp.recv();
    assert!(answer["error"].is_object(), "{answer}");
    assert_eq!(answer["error"]["code"], -32700, "{answer}");
    assert_eq!(
        answer.get("id"),
        Some(&Value::Null),
        "a parse error carries a null id: {answer}"
    );

    // still serving
    let answer = mcp.call("ping", json!({}));
    assert_eq!(structured(&answer), &golden("mcp__ping__all"));
}

#[test]
fn json_of_the_wrong_shape_is_answered_only_when_it_has_an_id() {
    let home = Home::with_store();
    let mut mcp = Mcp::started(&home);
    mcp.send_raw(r#"{"jsonrpc": "2.0", "id": 77, "colour": "blue"}"#);
    let answer = mcp.recv();
    assert_eq!(answer["error"]["code"], -32600, "{answer}");
    assert_eq!(answer.get("id"), Some(&Value::Null), "{answer}");
    // no id: nothing to answer; the next answer on the pipe belongs to the ping
    mcp.send_raw(r#"{"hello": "there"}"#);
    let answer = mcp.call("ping", json!({}));
    assert_eq!(structured(&answer), &golden("mcp__ping__all"));
}

#[test]
fn requests_sent_back_to_back_are_all_answered() {
    let home = Home::with_store();
    let mut mcp = Mcp::started(&home);
    for id in 100..110 {
        mcp.send(&json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": "usage_breakdown", "arguments": {"by": "day"}}}));
    }
    let mut ids: Vec<u64> = (0..10)
        .map(|_| {
            let answer = mcp.recv();
            assert_eq!(structured(&answer), &golden("mcp__usage_breakdown__day"));
            answer["id"].as_u64().unwrap()
        })
        .collect();
    ids.sort();
    assert_eq!(ids, (100..110).collect::<Vec<u64>>());
}

#[test]
fn a_missing_store_is_a_tool_error_without_a_path_and_a_store_that_appears_later_is_served() {
    let home = Home::new();
    let mut mcp = Mcp::started(&home);
    let text = error_text(&mcp.call("usage_summary", json!({})));
    assert!(
        text.starts_with("Error executing tool usage_summary: "),
        "{text}"
    );
    assert!(
        !text.contains(home.0.to_str().unwrap()),
        "path leaked: {text}"
    );
    assert!(
        !home.store().exists(),
        "the server must not create the store"
    );

    // each call opens the store afresh, so one that shows up later is read
    home.put_store();
    let answer = mcp.call("usage_summary", json!({}));
    assert_eq!(structured(&answer), &golden("mcp__usage_summary__all"));
}

#[test]
fn a_store_newer_than_the_build_knows_is_a_tool_error() {
    let home = Home::with_store();
    {
        let conn = rusqlite::Connection::open(home.store()).unwrap();
        conn.execute_batch("PRAGMA user_version = 99").unwrap();
    }
    let mut mcp = Mcp::started(&home);
    for (tool, args) in [
        ("usage_summary", json!({})),
        ("usage_breakdown", json!({"by": "day"})),
        ("warnings", json!({})),
        ("improvement_plan", json!({})),
    ] {
        let text = error_text(&mcp.call(tool, args));
        assert!(
            text.starts_with(&format!("Error executing tool {tool}: ")),
            "{text}"
        );
        assert!(text.contains("99") && text.contains("newer"), "{text}");
        assert!(
            !text.contains(home.0.to_str().unwrap()),
            "path leaked: {text}"
        );
    }
    // the server's tool did not write the store (only a reader opened it)
    let conn = rusqlite::Connection::open(home.store()).unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 99);
}

#[test]
fn a_changed_fixture_value_breaks_the_matching_comparison() {
    // Mutation check: the goldens really are compared, not just parsed.
    let home = Home::with_store();
    {
        let conn = rusqlite::Connection::open(home.store()).unwrap();
        let changed = conn
            .execute("UPDATE events SET output_tokens = output_tokens + 1 WHERE output_tokens IS NOT NULL AND abstained = 0", [])
            .unwrap();
        assert!(changed > 0);
    }
    let mut mcp = Mcp::started(&home);
    let answer = mcp.call("usage_summary", json!({}));
    assert_ne!(structured(&answer), &golden("mcp__usage_summary__all"));
    let answer = mcp.call("usage_breakdown", json!({"by": "model"}));
    assert_ne!(structured(&answer), &golden("mcp__usage_breakdown__model"));
}

#[test]
fn closing_stdin_ends_the_server_with_exit_zero_and_nothing_but_json_rpc_on_stdout() {
    let home = Home::with_store();
    let mut mcp = Mcp::started(&home);
    let _ = mcp.call("usage_summary", json!({}));
    let (code, rest, stderr) = mcp.finish();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    for line in rest {
        let message: Value = serde_json::from_str(&line)
            .unwrap_or_else(|_| panic!("stdout line is not JSON: {line:?}"));
        assert_eq!(message["jsonrpc"], "2.0");
    }
    assert!(
        !stderr.contains(home.0.to_str().unwrap()),
        "path in stderr: {stderr}"
    );
}

#[test]
fn closing_stdin_before_any_message_also_exits_zero() {
    let home = Home::with_store();
    let mcp = Mcp::start(&home);
    let (code, rest, stderr) = mcp.finish();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(rest.is_empty(), "stdout had {rest:?}");
}

// ---- T015: `warnings` and `improvement_plan` ----
// Both read the clock, so the spawned binary is only asked what does not depend on it; the goldens at the
// manifest's `frozen_now` are compared in-process by the unit tests in `src/mcp.rs`.

const TIME_TOOLS: [&str; 2] = ["warnings", "improvement_plan"];

/// The text block of a tool answer, parsed: it must equal the structured body.
fn text_body(answer: &Value) -> Value {
    serde_json::from_str(answer["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

#[test]
fn warnings_and_plan_answer_over_the_fixture_store_with_their_top_level_keys() {
    let home = Home::with_store();
    let mut mcp = Mcp::started(&home);
    let answer = mcp.call("warnings", json!({}));
    let body = structured(&answer);
    assert!(body["hits"].is_array(), "{body}");
    assert!(body["abstained"].is_array(), "{body}");
    assert_eq!(&text_body(&answer), body);

    let answer = mcp.call("improvement_plan", json!({}));
    let body = structured(&answer);
    assert!(body["groups"].is_array(), "{body}");
    assert_eq!(&text_body(&answer), body);

    // the same with explicit "no filter" values
    for tool in TIME_TOOLS {
        let answer = mcp.call(tool, json!({"workspace": null, "account": ""}));
        structured(&answer);
    }
    let (code, rest, stderr) = mcp.finish();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(rest.is_empty(), "stdout had {rest:?}");
    assert_eq!(stderr, "", "stderr must stay empty");
}

#[test]
fn a_store_with_no_events_gives_an_empty_hits_and_groups_list() {
    let home = Home::with_store();
    {
        let conn = rusqlite::Connection::open(home.store()).unwrap();
        conn.execute_batch("DELETE FROM events").unwrap();
    }
    let mut mcp = Mcp::started(&home);
    let answer = mcp.call("warnings", json!({}));
    assert_eq!(structured(&answer)["hits"], json!([]));
    let answer = mcp.call("improvement_plan", json!({}));
    assert_eq!(structured(&answer)["groups"], json!([]));
}

fn rich_golden(name: &str) -> Value {
    let text = std::fs::read_to_string(
        parity_dir()
            .join("golden-rich")
            .join(format!("{name}.json")),
    )
    .unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn a_scope_narrows_all_four_tools_over_the_binary() {
    let home = Home::with_rich_store();
    let mut mcp = Mcp::started(&home);
    // the usage tools: the same bodies the HTTP goldens hold (Python's MCP tools take no scope; this one honours it)
    for (arguments, summary, breakdown) in [
        (
            json!({"workspace": "acme"}),
            "api__usage_summary__workspace_acme",
            "api__usage_breakdown__model_workspace_acme",
        ),
        (
            json!({"account": "Claude work"}),
            "api__usage_summary__account_work",
            "api__usage_breakdown__model_account_work",
        ),
        (
            json!({"workspace": "personal", "account": null}),
            "api__usage_summary__workspace_personal",
            "api__usage_breakdown__model_workspace_personal",
        ),
    ] {
        let answer = mcp.call("usage_summary", arguments.clone());
        assert_eq!(structured(&answer), &rich_golden(summary), "{arguments}");
        assert_eq!(&text_body(&answer), structured(&answer));
        let mut with_by = arguments.clone();
        with_by["by"] = json!("model");
        let answer = mcp.call("usage_breakdown", with_by);
        assert_eq!(structured(&answer), &rich_golden(breakdown), "{arguments}");
    }
    // `by=day` is a local day: under the pinned zone it equals the HTTP golden of the same scope
    let answer = mcp.call(
        "usage_breakdown",
        json!({"by": "day", "workspace": "personal"}),
    );
    assert_eq!(
        structured(&answer),
        &rich_golden("api__usage_breakdown__day_workspace_personal")
    );
    // an empty scope is no scope
    let answer = mcp.call("usage_summary", json!({"workspace": "", "account": ""}));
    assert_eq!(structured(&answer), &golden_rich_unscoped_summary());
    // warnings and plan: only the shape (they read the real clock; the bodies are compared in-process)
    for tool in TIME_TOOLS {
        let answer = mcp.call(tool, json!({"workspace": "acme", "account": "Claude work"}));
        structured(&answer);
    }
    let answer = mcp.call("ping", json!({}));
    assert_eq!(structured(&answer), &golden("mcp__ping__all"));
}

fn golden_rich_unscoped_summary() -> Value {
    rich_golden("api__usage_summary__all")
}

#[test]
fn an_unknown_scope_is_a_tool_error_with_pythons_text_and_the_server_keeps_serving() {
    // Python 3.14, `rust/parity/.venv`, a scratch copy of the rich store (see the T019 report): `isError` true and
    // `Error executing tool <name>: <text>` for warnings / improvement_plan.
    let home = Home::with_rich_store();
    let mut mcp = Mcp::started(&home);
    let account = "unknown account; valid labels: Claude default, Claude work";
    for tool in [
        "usage_summary",
        "usage_breakdown",
        "warnings",
        "improvement_plan",
    ] {
        for (arguments, text) in [
            (
                json!({"workspace": "nope", "by": "model"}),
                "unknown workspace",
            ),
            (json!({"account": "nope", "by": "model"}), account),
            (
                json!({"workspace": "nope", "account": "nope", "by": "model"}),
                "unknown workspace",
            ),
        ] {
            let answer = mcp.call(tool, arguments);
            assert_eq!(
                error_text(&answer),
                format!("Error executing tool {tool}: {text}")
            );
        }
        // wrong type: a tool error like the other argument errors
        let answer = mcp.call(tool, json!({"workspace": 5, "by": "model"}));
        assert_eq!(
            error_text(&answer),
            format!("Error executing tool {tool}: argument `workspace` must be a string or null")
        );
    }
    let answer = mcp.call("ping", json!({}));
    assert_eq!(structured(&answer), &golden("mcp__ping__all"));
}

#[test]
fn warnings_and_plan_with_no_store_or_a_directory_in_its_place_are_tool_errors_without_a_path() {
    let home = Home::new();
    let mut mcp = Mcp::started(&home);
    for tool in TIME_TOOLS {
        let text = error_text(&mcp.call(tool, json!({})));
        assert!(
            text.starts_with(&format!("Error executing tool {tool}: ")),
            "{text}"
        );
        assert!(
            !text.contains(home.0.to_str().unwrap()),
            "path leaked: {text}"
        );
    }
    assert!(
        !home.store().exists(),
        "the server must not create the store"
    );

    std::fs::create_dir(home.store()).unwrap();
    for tool in TIME_TOOLS {
        let text = error_text(&mcp.call(tool, json!({})));
        assert!(
            text.starts_with(&format!("Error executing tool {tool}: ")),
            "{text}"
        );
        assert!(
            !text.contains(home.0.to_str().unwrap()),
            "path leaked: {text}"
        );
    }
}

// ---- T020: `session_outliers` ----
// The tool takes `since` and `until` only (Python's schema); it depends on no clock, so the spawned binary under
// `TZ=UTC` is compared with the Python goldens directly.

/// The manifest's `session_outliers` entries of one oracle: `(arguments, golden file name)`.
fn outlier_entries(dir: &str) -> Vec<(Value, String)> {
    let text = std::fs::read_to_string(parity_dir().join(dir).join("manifest.json")).unwrap();
    let manifest: Value = serde_json::from_str(&text).unwrap();
    manifest["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["surface"] == "mcp" && e["tool"] == "session_outliers")
        .map(|e| (e["params"].clone(), e["file"].as_str().unwrap().to_string()))
        .collect()
}

#[test]
fn session_outliers_is_listed_after_usage_breakdown_in_pythons_registration_order() {
    let home = Home::with_store();
    let mut mcp = Mcp::started(&home);
    let answer = mcp.request("tools/list", json!({}));
    let names: Vec<&str> = answer["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "ping",
            "usage_summary",
            "usage_breakdown",
            "session_outliers",
            "warnings",
            "improvement_plan",
            "connector_catalog",
            "connector_status",
            "connector_suggestions"
        ]
    );
    let entry = answer["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "session_outliers")
        .unwrap();
    // no workspace / account in the schema: Python's tool has only since and until
    let properties: Vec<&String> = entry["inputSchema"]["properties"]
        .as_object()
        .unwrap()
        .keys()
        .collect();
    assert_eq!(properties, ["since", "until"]);
}

#[test]
fn session_outliers_equals_the_default_golden_as_structured_content_and_text() {
    let entries = outlier_entries("golden");
    assert_eq!(entries.len(), 1);
    let home = Home::with_store();
    let mut mcp = Mcp::started(&home);
    for (arguments, file) in entries {
        let expected = golden(file.trim_end_matches(".json"));
        let answer = mcp.call("session_outliers", arguments);
        assert_eq!(structured(&answer), &expected, "{file}");
        assert_eq!(&text_body(&answer), &expected, "{file}");
    }
}

#[test]
fn session_outliers_equals_the_two_rich_goldens() {
    let entries = outlier_entries("golden-rich");
    assert_eq!(entries.len(), 2, "all and window");
    let home = Home::with_rich_store();
    let mut mcp = Mcp::started(&home);
    for (arguments, file) in entries {
        let expected = rich_golden(file.trim_end_matches(".json"));
        let answer = mcp.call("session_outliers", arguments.clone());
        assert_eq!(structured(&answer), &expected, "{file}");
        assert_eq!(&text_body(&answer), &expected, "{file}");
    }
}

#[test]
fn session_outliers_takes_null_absent_or_a_string_and_drops_what_the_schema_lacks() {
    let home = Home::with_rich_store();
    let mut mcp = Mcp::started(&home);
    let all = rich_golden("mcp__session_outliers__all");
    for arguments in [
        json!({}),
        json!({"since": null, "until": null}),
        json!({"since": null}),
        // Python's tool has no scope: an extra argument is dropped, not an error
        json!({"workspace": "nope", "account": "nope"}),
    ] {
        let answer = mcp.call("session_outliers", arguments.clone());
        assert_eq!(structured(&answer), &all, "{arguments}");
    }
    let window = rich_golden("mcp__session_outliers__window");
    let answer = mcp.call(
        "session_outliers",
        json!({"since": "2026-01-13T00:00:00Z", "until": "2026-01-20T00:00:00Z"}),
    );
    assert_eq!(structured(&answer), &window);
}

#[test]
fn session_outliers_errors_are_tool_errors_with_pythons_text() {
    // Python 3.14, `rust/parity/.venv`, a scratch copy of the default store: a bad `since` or `until` is
    // `Error executing tool session_outliers: Invalid isoformat string: 'bad'` (isError true); with both bad, `since` wins.
    let home = Home::with_store();
    let mut mcp = Mcp::started(&home);
    for (arguments, text) in [
        (json!({"since": "bad"}), "Invalid isoformat string: 'bad'"),
        (json!({"until": "bad"}), "Invalid isoformat string: 'bad'"),
        (
            json!({"since": "bad", "until": "worse"}),
            "Invalid isoformat string: 'bad'",
        ),
        (json!({"since": ""}), "Invalid isoformat string: ''"),
        // a wrong type is an argument error, as for the other tools
        (
            json!({"since": 5}),
            "argument `since` must be a string or null",
        ),
        (
            json!({"until": ["a"]}),
            "argument `until` must be a string or null",
        ),
        (
            json!({"until": true}),
            "argument `until` must be a string or null",
        ),
    ] {
        let answer = mcp.call("session_outliers", arguments.clone());
        assert_eq!(
            error_text(&answer),
            format!("Error executing tool session_outliers: {text}"),
            "{arguments}"
        );
    }
    let answer = mcp.call("ping", json!({}));
    assert_eq!(structured(&answer), &golden("mcp__ping__all"));
}

#[test]
fn session_outliers_over_a_store_with_no_events_is_an_empty_list() {
    // Python over a copy of the default store with `DELETE FROM events`
    let home = Home::with_store();
    rusqlite::Connection::open(home.store())
        .unwrap()
        .execute_batch("DELETE FROM events")
        .unwrap();
    let mut mcp = Mcp::started(&home);
    let answer = mcp.call("session_outliers", json!({}));
    assert_eq!(
        structured(&answer),
        &json!({
            "period": {"since_utc": null, "until_utc": null},
            "sessions": [],
            "thresholds": {"context_tokens": 200000, "tool_calls": 90}
        })
    );
}

#[test]
fn session_outliers_store_states_are_tool_errors_without_a_path_and_nothing_is_created() {
    let home = Home::new();
    let mut mcp = Mcp::started(&home);
    let check = |text: &str| {
        assert!(
            text.starts_with("Error executing tool session_outliers: "),
            "{text}"
        );
        assert!(
            !text.contains(home.0.to_str().unwrap()),
            "path leaked: {text}"
        );
    };
    check(&error_text(&mcp.call("session_outliers", json!({}))));
    assert!(!home.store().exists(), "the server must not create a store");
    std::fs::create_dir(home.store()).unwrap();
    check(&error_text(&mcp.call("session_outliers", json!({}))));
    std::fs::remove_dir(home.store()).unwrap();

    // a newer store
    home.put_store();
    rusqlite::Connection::open(home.store())
        .unwrap()
        .execute_batch("PRAGMA user_version = 99")
        .unwrap();
    let text = error_text(&mcp.call("session_outliers", json!({})));
    check(&text);
    assert!(text.contains("99") && text.contains("newer"), "{text}");
}

#[test]
fn a_changed_row_breaks_the_session_outliers_comparison() {
    // Mutation check on the rich store: one more tool name on the first message of the 90-call outlier session
    // (additive, so the session stays in the answer and its count changes).
    let home = Home::with_rich_store();
    let n = rusqlite::Connection::open(home.store())
        .unwrap()
        .execute(
            "UPDATE events SET tool_names = tool_names || ',Edit' WHERE id = \
             (SELECT MIN(id) FROM events WHERE session_id = '00000000-0000-4000-8000-000000000138' \
              AND tool_names IS NOT NULL AND tool_names != '' AND abstained = 0)",
            [],
        )
        .unwrap();
    assert_eq!(n, 1);
    let mut mcp = Mcp::started(&home);
    let answer = mcp.call("session_outliers", json!({}));
    let calls = structured(&answer)["sessions"][0]["tool_calls"].clone();
    assert_eq!(calls, 91);
    assert_ne!(
        structured(&answer),
        &rich_golden("mcp__session_outliers__all")
    );
}
