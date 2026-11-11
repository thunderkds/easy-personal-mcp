//! `mcp` subcommand: the MCP server on stdio with `ping`, `usage_summary`, `usage_breakdown` (T008), `session_outliers`
//! (T020), `warnings` and `improvement_plan` (T015). `usage_summary`, `usage_breakdown`, `warnings` and
//! `improvement_plan` take an optional `workspace` and `account` (T019); `session_outliers` takes `since` and `until`
//! only, as Python's tool does. `connector_catalog`, `connector_status` and `connector_suggestions` (T046) take no
//! argument; presence is never asked (no keychain, no `claude`: `suggestions::configured_ids`).
//!
//! Everything that touches `rmcp` lives in this file, so a later major version of the SDK changes one module.
//! stdout is the protocol channel: nothing here prints to it, and nothing is logged.
//! Tool names, descriptions and schemas are copied from the Python app's `tools/list` golden
//! (`rust/parity/golden/mcp__tools_list.json`); a test compares them. The nine tools only read the store.

use easy_personal_core::clock;
use easy_personal_core::kit::kit_summary;
use easy_personal_core::metrics::{usage_breakdown_scoped, usage_summary_scoped};
use easy_personal_core::plan::improvement_plan_scoped;
use easy_personal_core::rules::{evaluate_scoped, shipped_rules};
use easy_personal_core::scope::Scope;
use easy_personal_core::sessions::session_outliers;
use easy_personal_core::store::{StoreError, open_checked, store_path};
use easy_personal_core::suggestions::{self, ViewError};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorData,
    Implementation, JsonObject, ListToolsResult, PaginatedRequestParams, ProtocolVersion,
    ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::{RequestContext, RxJsonRpcMessage, ServerInitializeError, TxJsonRpcMessage};
use rmcp::transport::Transport;
use rmcp::{RoleServer, ServerHandler, ServiceExt};
use serde_json::{Value, json};
use std::borrow::Cow;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};

/// What Python's `core.ping()` returns.
const PING_TEXT: &str = "easy-personal core ok";

const SUMMARY_DESCRIPTION: &str = "Claude Code usage for [since, until): sessions, messages, input/output and cache\ntokens, cache-hit ratio, records abstained per version, and `split`: tokens by typed (you),\nagent (sidechain, or an sdk* entrypoint) and unknown (not yet backfilled) work.\nBounds are ISO 8601 dates or datetimes; without an offset they are local time; omitted means all stored history.";

const OUTLIERS_DESCRIPTION: &str = "Sessions with at least 90 tool calls or a very long context, each with its measured\nvalues and the thresholds used.";

const WARNINGS_DESCRIPTION: &str = "Declared usage rules over the last 7 days (or 14 for week-over-week). Each hit has the\nrule id, label (improve/aware/warning), measured value, threshold and fix line. Rules that\ncannot be measured (too little history or data) are listed under `abstained`. Optional\n`workspace` (name) and `account` (id or label) limit every rule to that scope.";

const PLAN_DESCRIPTION: &str = "The fired rules grouped by priority (1 first), each with its measured value and fix.\nOptional `workspace` (name) and `account` (id or label) limit every rule to that scope.";

const KIT_DESCRIPTION: &str = "What the Easy Kit did per project (folder name): kernel-trace tool calls, errors, kit\ntasks and last activity. A part with nothing to read says \"no data\". Optional `workspace`\n(name) and `account` (id or label) keep only that scope's projects.";

const KIT_FINDINGS_DESCRIPTION: &str = "Which Easy Kit MCP servers (easy-ui, easy-verifier, easy-context, easy-memory) are linked to\ncheaper or better sessions, comparing sessions with and without each server inside the same\nproject, main model and tool-call band (tokens and cost per tool call, share of good results).\n`findings` = at most `limit` (default 3, max 10) comparisons where the server side is better by at least 5 %, each\nwith numbers, group sizes and one fixed habit; `other` = every other comparison that has at least 10\nsessions on each side (no habit); `not_enough` lists servers with fewer. Worded \"linked to\", never\n\"caused\". `since` is an ISO date or datetime (without an offset: local time); omitted means all\nstored history.";

const CONNECTOR_CATALOG_DESCRIPTION: &str = "The provider catalog: id, provider, category, whether Claude.ai has an official connector\n(and its link), and the field labels each needs. Read-only; never a secret value.";

const CONNECTOR_STATUS_DESCRIPTION: &str = "Per catalog id, whether the connector is configured (yes/no only): a keychain item and a\nClaude Code entry both exist. Never returns a secret or any config value.";

const CONNECTOR_SUGGESTIONS_DESCRIPTION: &str = "Providers not yet configured that your usage points to, from counted WebFetch hosts and\nMCP tool names (metadata only), with the call counts behind each suggestion.";

const BREAKDOWN_DESCRIPTION: &str = "Tokens grouped by day (OS local time), model, project (folder name) or tool for\n[since, until), with an API-equivalent cost (price-table date shown; a model missing\nfrom the table abstains). Each group has the same typed / agent / unknown `split`. `total`\nequals usage_summary; tool groups overlap. Project groups put every scratch folder (temp,\ngone, not in a git repo, or hidden) into one group `Scratch` (`scratch: true`) that lists\nthem in `folders`.";

pub fn run() -> Result<(), String> {
    let runtime =
        tokio::runtime::Runtime::new().map_err(|e| format!("cannot start the runtime: {e}"))?;
    runtime.block_on(async {
        // Ends when the client closes stdin.
        let service = match Server.serve(StdioLines::new()).await {
            Ok(service) => service,
            // stdin closed before the client said anything: a clean end, not a failure
            Err(ServerInitializeError::ConnectionClosed(_)) => return Ok(()),
            Err(e) => return Err(format!("cannot start the MCP server: {e}")),
        };
        service
            .waiting()
            .await
            .map(|_| ())
            .map_err(|e| format!("MCP server stopped: {e}"))
    })
}

/// Newline-delimited JSON-RPC over stdin / stdout.
///
/// rmcp's own `stdio()` transport silently drops a line that is not valid JSON; JSON-RPC (and T008 criterion 9)
/// wants a `-32700` answer with a null id, so this small transport does the framing itself. Whole lines are written
/// under the `Stdout` lock, so an error answer from `receive` and a tool answer from `send` never interleave.
struct StdioLines {
    reader: BufReader<tokio::io::Stdin>,
    /// Holds a partly read line if a read is cancelled while the service loop serves another branch.
    line: Vec<u8>,
}

impl StdioLines {
    fn new() -> Self {
        StdioLines {
            reader: BufReader::new(tokio::io::stdin()),
            line: Vec::new(),
        }
    }
}

/// Writes one JSON-RPC message as one line to stdout and flushes it (off the async workers: a pipe can block).
async fn write_line(mut bytes: Vec<u8>) -> std::io::Result<()> {
    bytes.push(b'\n');
    tokio::task::spawn_blocking(move || {
        let mut out = std::io::stdout().lock();
        out.write_all(&bytes)?;
        out.flush()
    })
    .await
    .map_err(std::io::Error::other)?
}

impl Transport<RoleServer> for StdioLines {
    type Error = std::io::Error;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleServer>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        let bytes = serde_json::to_vec(&item).map_err(std::io::Error::from);
        async move { write_line(bytes?).await }
    }

    async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleServer>> {
        loop {
            match self.reader.read_until(b'\n', &mut self.line).await {
                Ok(0) | Err(_) => return None, // stdin closed (a last line without a newline is not a message)
                Ok(_) => {}
            }
            if !self.line.ends_with(b"\n") {
                continue; // a read was cut short; the rest of the line is still to come
            }
            let parsed =
                serde_json::from_slice::<RxJsonRpcMessage<RoleServer>>(self.line.trim_ascii());
            let blank = self.line.trim_ascii().is_empty();
            let value = parsed
                .as_ref()
                .err()
                .map(|_| serde_json::from_slice::<Value>(&self.line));
            self.line.clear();
            match parsed {
                _ if blank => continue,
                Ok(message) => return Some(message),
                Err(_) => {
                    // Not JSON: Parse error. JSON but not a request (and it has an id): Invalid Request.
                    // (rmcp's own error type leaves the null `id` out; JSON-RPC wants it present.)
                    let error = match value {
                        Some(Err(_)) => Some((-32700, "Parse error")),
                        Some(Ok(v)) if v.get("id").is_some() => Some((-32600, "Invalid request")),
                        _ => None,
                    };
                    if let Some((code, message)) = error {
                        let reply = json!({"jsonrpc": "2.0", "id": null, "error": {"code": code, "message": message}});
                        if write_line(reply.to_string().into_bytes()).await.is_err() {
                            return None;
                        }
                    }
                }
            }
        }
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Clone)]
struct Server;

impl ServerHandler for Server {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(ProtocolVersion::LATEST_WITH_INITIALIZE)
            .with_server_info(Implementation::new(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ))
    }

    /// Every revision that has the `initialize` handshake; the `2026-07-28` no-handshake mode stays off.
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(ProtocolVersion::known_up_to(
            &ProtocolVersion::LATEST_WITH_INITIALIZE,
        ))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(tools()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let name = request.name.to_string();
        if !matches!(
            name.as_str(),
            "ping"
                | "usage_summary"
                | "usage_breakdown"
                | "session_outliers"
                | "warnings"
                | "improvement_plan"
                | "kit_summary"
                | "kit_findings"
                | "connector_catalog"
                | "connector_status"
                | "connector_suggestions"
        ) {
            return Err(ErrorData::invalid_params(
                format!("Unknown tool: {name}"),
                None,
            ));
        }
        let args = request.arguments.unwrap_or_default();
        // The store is read with blocking calls; keep them off the protocol's worker thread.
        let outcome = tokio::task::spawn_blocking({
            let name = name.clone();
            move || run_tool(&name, &args, clock::now_micros(), store_path)
        })
        .await
        .map_err(|_| ErrorData::internal_error("the tool task failed", None))?;
        Ok(match outcome {
            Ok(result) => result,
            // Python's FastMCP turns a raised error into a tool error with this prefix.
            Err(text) => CallToolResult::error(vec![ContentBlock::text(format!(
                "Error executing tool {name}: {text}"
            ))]),
        }
        .into())
    }
}

fn tools() -> Vec<Tool> {
    let nullable_string = |title: &str| json!({"anyOf": [{"type": "string"}, {"type": "null"}], "default": null, "title": title});
    let dict_output =
        |title: &str| json!({"additionalProperties": true, "title": title, "type": "object"});
    let scope_input = |title: &str| {
        json!({
            "properties": {"workspace": nullable_string("Workspace"), "account": nullable_string("Account")},
            "title": title,
            "type": "object"
        })
    };
    let no_input = |title: &str| json!({"properties": {}, "title": title, "type": "object"});
    vec![
        tool(
            "ping",
            "Placeholder tool: returns the shared core's status string.",
            json!({"properties": {}, "title": "pingArguments", "type": "object"}),
            json!({
                "properties": {"result": {"title": "Result", "type": "string"}},
                "required": ["result"],
                "title": "pingOutput",
                "type": "object"
            }),
        ),
        tool(
            "usage_summary",
            SUMMARY_DESCRIPTION,
            json!({
                "properties": {"since": nullable_string("Since"), "until": nullable_string("Until")},
                "title": "usage_summaryArguments",
                "type": "object"
            }),
            dict_output("usage_summaryDictOutput"),
        ),
        tool(
            "usage_breakdown",
            BREAKDOWN_DESCRIPTION,
            json!({
                "properties": {
                    "by": {"title": "By", "type": "string"},
                    "since": nullable_string("Since"),
                    "until": nullable_string("Until")
                },
                "required": ["by"],
                "title": "usage_breakdownArguments",
                "type": "object"
            }),
            dict_output("usage_breakdownDictOutput"),
        ),
        tool(
            "session_outliers",
            OUTLIERS_DESCRIPTION,
            json!({
                "properties": {"since": nullable_string("Since"), "until": nullable_string("Until")},
                "title": "session_outliersArguments",
                "type": "object"
            }),
            dict_output("session_outliersDictOutput"),
        ),
        tool(
            "warnings",
            WARNINGS_DESCRIPTION,
            scope_input("warningsArguments"),
            dict_output("warningsDictOutput"),
        ),
        tool(
            "improvement_plan",
            PLAN_DESCRIPTION,
            scope_input("improvement_planArguments"),
            dict_output("improvement_planDictOutput"),
        ),
        tool(
            "kit_summary",
            KIT_DESCRIPTION,
            scope_input("kit_summaryArguments"),
            dict_output("kit_summaryDictOutput"),
        ),
        tool(
            "kit_findings",
            KIT_FINDINGS_DESCRIPTION,
            json!({
                "properties": {"limit": {"default": 3, "title": "Limit", "type": "integer"}, "since": nullable_string("Since")},
                "title": "kit_findingsArguments",
                "type": "object"
            }),
            dict_output("kit_findingsDictOutput"),
        ),
        tool(
            "connector_catalog",
            CONNECTOR_CATALOG_DESCRIPTION,
            no_input("connector_catalogArguments"),
            dict_output("connector_catalogDictOutput"),
        ),
        tool(
            "connector_status",
            CONNECTOR_STATUS_DESCRIPTION,
            no_input("connector_statusArguments"),
            dict_output("connector_statusDictOutput"),
        ),
        tool(
            "connector_suggestions",
            CONNECTOR_SUGGESTIONS_DESCRIPTION,
            no_input("connector_suggestionsArguments"),
            dict_output("connector_suggestionsDictOutput"),
        ),
    ]
}

fn tool(name: &'static str, description: &'static str, input: Value, output: Value) -> Tool {
    Tool::new(name, description, object(input)).with_raw_output_schema(Arc::new(object(output)))
}

fn object(value: Value) -> JsonObject {
    match value {
        Value::Object(map) => map,
        _ => unreachable!("schemas are written as objects"),
    }
}

/// Runs one tool. `Err` is the tool error text (without the `Error executing tool` prefix).
///
/// `now_micros` (UTC) is the clock the time-based tools read, and `store` says where the store is: the server
/// passes the system clock and [`store_path`]; only the in-process tests pass anything else. The clock cannot be
/// changed through the binary (no flag, no variable).
fn run_tool(
    name: &str,
    args: &JsonObject,
    now_micros: i64,
    store: impl Fn() -> Result<PathBuf, StoreError>,
) -> Result<CallToolResult, String> {
    match name {
        "ping" => {
            let mut result = CallToolResult::success(vec![ContentBlock::text(PING_TEXT)]);
            result.structured_content = Some(json!({"result": PING_TEXT}));
            Ok(result)
        }
        "usage_summary" => {
            let (since, until) = (optional(args, "since")?, optional(args, "until")?);
            let (workspace, account) = scope_args(args)?;
            let body = with_store(&store, |conn| {
                let scope = scope_of(&workspace, &account);
                usage_summary_scoped(conn, since.as_deref(), until.as_deref(), scope)
                    .map_err(|e| e.to_string())
            })?;
            Ok(CallToolResult::structured(body))
        }
        "usage_breakdown" => {
            let by = match args.get("by") {
                Some(Value::String(by)) => by.clone(),
                _ => return Err("argument `by` is required and must be a string".to_string()),
            };
            let (since, until) = (optional(args, "since")?, optional(args, "until")?);
            let (workspace, account) = scope_args(args)?;
            let body = with_store(&store, |conn| {
                let scope = scope_of(&workspace, &account);
                usage_breakdown_scoped(conn, &by, since.as_deref(), until.as_deref(), scope)
                    .map_err(|e| e.to_string())
            })?;
            Ok(CallToolResult::structured(body))
        }
        "session_outliers" => {
            // no `workspace` / `account`: Python's tool has only `since` and `until` (an extra argument is dropped)
            let (since, until) = (optional(args, "since")?, optional(args, "until")?);
            let body = with_store(&store, |conn| {
                session_outliers(conn, since.as_deref(), until.as_deref())
                    .map_err(|e| e.to_string())
            })?;
            Ok(CallToolResult::structured(body))
        }
        "warnings" | "improvement_plan" => {
            let (workspace, account) = scope_args(args)?;
            let rules = shipped_rules().map_err(|e| e.to_string())?;
            let body = with_store(&store, |conn| {
                let scope = scope_of(&workspace, &account);
                if name == "warnings" {
                    evaluate_scoped(conn, now_micros, &rules, scope).map_err(|e| e.to_string())
                } else {
                    improvement_plan_scoped(conn, now_micros, &rules, scope)
                        .map_err(|e| e.to_string())
                }
            })?;
            Ok(CallToolResult::structured(body))
        }
        "kit_summary" => {
            let (workspace, account) = scope_args(args)?;
            let body = with_store(&store, |conn| {
                kit_summary(conn, now_micros, scope_of(&workspace, &account), false)
                    .map_err(|e| e.to_string())
            })?;
            Ok(CallToolResult::structured(body))
        }
        "kit_findings" => {
            let limit = limit_arg(args)?;
            let since = optional(args, "since")?;
            let body = with_store(&store, |conn| {
                easy_personal_core::kitperf::kit_findings_local(
                    conn,
                    limit,
                    since.as_deref(),
                    now_micros,
                )
                .map_err(|e| e.to_string())
            })?;
            Ok(CallToolResult::structured(body))
        }
        // no arguments (an extra one is dropped, as in Python); the first two never open the store
        "connector_catalog" => Ok(CallToolResult::structured(
            suggestions::catalog_view().map_err(view_error)?,
        )),
        "connector_status" => Ok(CallToolResult::structured(suggestions::status())),
        "connector_suggestions" => {
            let body = with_store(&store, |conn| {
                suggestions::suggestions(conn).map_err(view_error)
            })?;
            Ok(CallToolResult::structured(body))
        }
        _ => unreachable!("call_tool checks the name first"),
    }
}

/// A catalog that does not parse cannot happen in a shipped build; the store's text has no path (NFR-002).
fn view_error(error: ViewError) -> String {
    match error {
        ViewError::Store(error) => error.to_string(),
        ViewError::Catalog(_) => "internal error".to_string(),
    }
}

/// A `since` / `until` argument: absent or `null` is `None` (Python's default), a string is the bound.
fn optional(args: &JsonObject, key: &str) -> Result<Option<String>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(format!("argument `{key}` must be a string or null")),
    }
}

/// `limit` of `kit_findings`: absent is Python's default 3; an integer (or a float with no fraction) is clamped by the core to
/// 1..10. Anything else (`null`, a string, a boolean, a fraction) is an argument error: Python's text is pydantic's.
fn limit_arg(args: &JsonObject) -> Result<i64, String> {
    let bad = || "argument `limit` must be an integer".to_string();
    match args.get("limit") {
        None => Ok(3),
        Some(Value::Number(n)) => n
            .as_i64()
            .or_else(|| n.as_u64().map(|_| i64::MAX))
            .or_else(|| {
                n.as_f64()
                    .filter(|f| f.is_finite() && f.fract() == 0.0)
                    .map(|f| f as i64)
            })
            .ok_or_else(bad),
        Some(_) => Err(bad()),
    }
}

/// The `workspace` and `account` arguments: absent, `null` and `""` mean no filter; a string is the scope; any other
/// type is an argument error. Checked before the store is opened.
///
/// Python's `usage_summary` and `usage_breakdown` have no such parameters (an extra argument is dropped there); this
/// build honours them, so a scope is never silently ignored. The input schemas stay equal to Python's.
fn scope_args(args: &JsonObject) -> Result<(Option<String>, Option<String>), String> {
    Ok((optional(args, "workspace")?, optional(args, "account")?))
}

/// The core's scope for two arguments; an empty string is no filter (the core reads it so, as Python's `or None`).
fn scope_of<'a>(workspace: &'a Option<String>, account: &'a Option<String>) -> Scope<'a> {
    Scope {
        workspace: workspace.as_deref(),
        account: account.as_deref(),
    }
}

/// Opens the store read-only for this one call. A store that is missing, newer than this build knows, or
/// unreadable is a plain message without the path (NFR-002); nothing is created or written.
fn with_store(
    store: &impl Fn() -> Result<PathBuf, StoreError>,
    read: impl FnOnce(&rusqlite::Connection) -> Result<Value, String>,
) -> Result<Value, String> {
    let path = store().map_err(|e| e.to_string())?;
    let conn = open_checked(&path).map_err(|e| e.to_string())?;
    read(&conn)
}

#[cfg(test)]
mod tests {
    //! `warnings` and `improvement_plan` read the clock, so their goldens (made by Python at the manifest's
    //! `frozen_now`) are compared here, in-process, with that moment passed to `run_tool`. The binary has no way
    //! to set the clock; `tests/mcp.rs` spawns it and checks what does not depend on the time.

    use super::*;
    use std::path::Path;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// 2026-01-20T00:00:00Z in microseconds: the `frozen_now` of both manifests (checked in `frozen_now_is_the_manifests`).
    const FROZEN_NOW: i64 = 1_768_867_200_000_000;

    fn parity_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity")
    }

    fn read_json(path: &Path) -> Value {
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    /// A temp folder holding a copy of one parity store (a read-only open can leave `-wal`/`-shm` beside it).
    struct Copy(PathBuf);

    impl Copy {
        fn of(store: &str) -> Self {
            static N: AtomicU32 = AtomicU32::new(0);
            let dir = std::env::temp_dir().join(format!(
                "easy-personal-rs-t015-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::SeqCst)
            ));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::copy(parity_dir().join(store), dir.join("store.sqlite")).unwrap();
            Copy(dir)
        }

        fn store(&self) -> PathBuf {
            self.0.join("store.sqlite")
        }
    }

    impl Drop for Copy {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn args(value: Value) -> JsonObject {
        object(value)
    }

    fn run(store: &Path, name: &str, arguments: Value) -> Result<CallToolResult, String> {
        run_tool(name, &args(arguments), FROZEN_NOW, || {
            Ok(store.to_path_buf())
        })
    }

    /// The body of a successful result, after checking that the text block carries the same JSON.
    fn body(result: &CallToolResult) -> Value {
        assert_eq!(result.is_error, Some(false));
        let structured = result
            .structured_content
            .clone()
            .expect("structured content");
        let text = serde_json::to_value(&result.content[0]).unwrap();
        let parsed: Value = serde_json::from_str(text["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            parsed, structured,
            "text block and structured content differ"
        );
        structured
    }

    fn check_goldens(golden_dir: &str, store: &str) {
        let manifest = read_json(&parity_dir().join(golden_dir).join("manifest.json"));
        assert_eq!(manifest["frozen_now"], "2026-01-20T00:00:00+00:00");
        let copy = Copy::of(store);
        for (tool, file) in [
            ("warnings", "mcp__warnings__all.json"),
            ("improvement_plan", "mcp__improvement_plan__all.json"),
        ] {
            let expected = read_json(&parity_dir().join(golden_dir).join(file));
            let result = run(&copy.store(), tool, json!({})).unwrap();
            assert_eq!(body(&result), expected, "{golden_dir} {tool}");
        }
    }

    #[test]
    fn frozen_now_is_the_manifests() {
        // 2026-01-20: 20_473 days after 1970-01-01
        assert_eq!(FROZEN_NOW, 20_473 * 86_400 * 1_000_000);
    }

    #[test]
    fn warnings_and_plan_equal_the_default_goldens_at_the_frozen_clock() {
        check_goldens("golden", "store-v11.sqlite");
    }

    #[test]
    fn warnings_and_plan_equal_the_rich_goldens_at_the_frozen_clock() {
        check_goldens("golden-rich", "store-rich/store-v11.sqlite");
    }

    #[test]
    fn the_clock_passed_in_is_the_clock_used() {
        // Mutation guard: a later "now" moves the 7-day window off the rich store's events.
        let copy = Copy::of("store-rich/store-v11.sqlite");
        let golden = read_json(&parity_dir().join("golden-rich/mcp__warnings__all.json"));
        let later = 400 * 86_400 * 1_000_000;
        let result = run_tool("warnings", &args(json!({})), FROZEN_NOW + later, || {
            Ok(copy.store())
        })
        .unwrap();
        assert_ne!(body(&result), golden);
    }

    #[test]
    fn a_changed_fixture_value_breaks_the_golden_comparison() {
        let copy = Copy::of("store-rich/store-v11.sqlite");
        {
            let conn = rusqlite::Connection::open(copy.store()).unwrap();
            let changed = conn
                .execute(
                    "UPDATE events SET output_tokens = output_tokens * 3 WHERE output_tokens IS NOT NULL AND abstained = 0",
                    [],
                )
                .unwrap();
            assert!(changed > 0);
        }
        for (tool, file) in [
            ("warnings", "mcp__warnings__all.json"),
            ("improvement_plan", "mcp__improvement_plan__all.json"),
        ] {
            let golden = read_json(&parity_dir().join("golden-rich").join(file));
            let result = run(&copy.store(), tool, json!({})).unwrap();
            assert_ne!(body(&result), golden, "{tool}");
        }
    }

    #[test]
    fn no_filter_means_null_absent_or_an_empty_string() {
        let copy = Copy::of("store-v11.sqlite");
        let plain = body(&run(&copy.store(), "warnings", json!({})).unwrap());
        for arguments in [
            json!({"workspace": null, "account": null}),
            json!({"workspace": "", "account": ""}),
            json!({"workspace": null}),
            json!({"account": ""}),
        ] {
            let result = run(&copy.store(), "warnings", arguments.clone()).unwrap();
            assert_eq!(body(&result), plain, "{arguments}");
            assert!(run(&copy.store(), "improvement_plan", arguments).is_ok());
        }
    }

    /// The rich manifest's MCP entries of one tool whose arguments name a `workspace` or an `account`:
    /// `(arguments, golden file name)`.
    fn scoped_entries(tool: &str) -> Vec<(Value, String)> {
        let manifest = read_json(&parity_dir().join("golden-rich/manifest.json"));
        manifest["entries"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["surface"] == "mcp" && e["tool"] == tool)
            .filter(|e| {
                ["workspace", "account"]
                    .iter()
                    .any(|k| e["params"].get(k).is_some())
            })
            .map(|e| (e["params"].clone(), e["file"].as_str().unwrap().to_string()))
            .collect()
    }

    #[test]
    fn the_ten_scoped_goldens_equal_structured_content_and_text() {
        // 5 warnings + 5 improvement_plan entries, counted from the manifest
        let copy = Copy::of("store-rich/store-v11.sqlite");
        let mut compared = 0;
        for tool in ["warnings", "improvement_plan"] {
            let entries = scoped_entries(tool);
            assert_eq!(entries.len(), 5, "{tool}");
            for (arguments, file) in entries {
                let expected = read_json(&parity_dir().join("golden-rich").join(&file));
                let result = run(&copy.store(), tool, arguments).unwrap();
                assert_eq!(body(&result), expected, "{file}");
                compared += 1;
            }
        }
        assert_eq!(compared, 10);
    }

    #[test]
    fn a_scoped_golden_breaks_when_an_event_moves_to_the_other_account() {
        let copy = Copy::of("store-rich/store-v11.sqlite");
        rusqlite::Connection::open(copy.store())
            .unwrap()
            .execute(
                "UPDATE events SET agent_account_id = 1 WHERE agent_account_id = 2",
                [],
            )
            .unwrap();
        let (arguments, file) = scoped_entries("warnings")
            .into_iter()
            .find(|(a, _)| a == &json!({"account": "Claude work"}))
            .unwrap();
        let expected = read_json(&parity_dir().join("golden-rich").join(file));
        let result = run(&copy.store(), "warnings", arguments).unwrap();
        assert_ne!(body(&result), expected);
    }

    #[test]
    fn the_usage_tools_with_a_scope_answer_what_the_core_and_the_http_goldens_say() {
        // Python's usage tools take no scope (it drops an extra argument); this build honours it (T019).
        let copy = Copy::of("store-rich/store-v11.sqlite");
        let conn = easy_personal_core::store::open_read_only(&copy.store()).unwrap();
        for (arguments, scope) in [
            (
                json!({"workspace": "acme"}),
                Scope {
                    workspace: Some("acme"),
                    account: None,
                },
            ),
            (
                json!({"account": "Claude work"}),
                Scope {
                    workspace: None,
                    account: Some("Claude work"),
                },
            ),
            (
                json!({"workspace": "acme", "account": "Claude work"}),
                Scope {
                    workspace: Some("acme"),
                    account: Some("Claude work"),
                },
            ),
            (
                json!({"workspace": "personal"}),
                Scope {
                    workspace: Some("personal"),
                    account: None,
                },
            ),
        ] {
            let summary = run(&copy.store(), "usage_summary", arguments.clone()).unwrap();
            assert_eq!(
                body(&summary),
                usage_summary_scoped(&conn, None, None, scope).unwrap(),
                "{arguments}"
            );
            let mut with_by = arguments.clone();
            with_by["by"] = json!("model");
            let breakdown = run(&copy.store(), "usage_breakdown", with_by).unwrap();
            assert_eq!(
                body(&breakdown),
                usage_breakdown_scoped(&conn, "model", None, None, scope).unwrap(),
                "{arguments}"
            );
        }
        // and the HTTP goldens of the same scopes (`by=model` and the summary do not depend on the zone)
        for (arguments, summary, breakdown) in [
            (
                json!({"workspace": "acme"}),
                "api__usage_summary__workspace_acme.json",
                "api__usage_breakdown__model_workspace_acme.json",
            ),
            (
                json!({"account": "Claude work"}),
                "api__usage_summary__account_work.json",
                "api__usage_breakdown__model_account_work.json",
            ),
            (
                json!({"workspace": "acme", "account": "Claude work"}),
                "api__usage_summary__workspace_acme_account_work.json",
                "api__usage_breakdown__model_workspace_acme_account_work.json",
            ),
        ] {
            let golden = |file: &str| read_json(&parity_dir().join("golden-rich").join(file));
            let result = run(&copy.store(), "usage_summary", arguments.clone()).unwrap();
            assert_eq!(body(&result), golden(summary), "{arguments}");
            let mut with_by = arguments.clone();
            with_by["by"] = json!("model");
            let result = run(&copy.store(), "usage_breakdown", with_by).unwrap();
            assert_eq!(body(&result), golden(breakdown), "{arguments}");
        }
    }

    #[test]
    fn an_unknown_workspace_or_account_is_a_tool_error_with_pythons_text_and_no_input() {
        let copy = Copy::of("store-rich/store-v11.sqlite");
        let account = "unknown account; valid labels: Claude default, Claude work";
        for tool in [
            "usage_summary",
            "usage_breakdown",
            "warnings",
            "improvement_plan",
        ] {
            for (arguments, text) in [
                (json!({"workspace": "nope"}), "unknown workspace"),
                (json!({"account": "nope"}), account),
                (
                    json!({"workspace": "nope", "account": "nope"}),
                    "unknown workspace",
                ),
                (
                    json!({"workspace": "/home/someone/secret-folder"}),
                    "unknown workspace",
                ),
                (json!({"account": "x'; DROP TABLE events;--"}), account),
            ] {
                let mut arguments = arguments;
                arguments["by"] = json!("model"); // ignored by the tools that take no `by`
                let error = run(&copy.store(), tool, arguments.clone()).unwrap_err();
                assert_eq!(error, text, "{tool} {arguments}");
            }
        }
    }

    #[test]
    fn an_invalid_by_or_period_is_reported_before_an_unknown_scope() {
        let copy = Copy::of("store-rich/store-v11.sqlite");
        let error = run(
            &copy.store(),
            "usage_breakdown",
            json!({"by": "bad", "workspace": "nope"}),
        );
        assert_eq!(
            error.unwrap_err(),
            "by must be one of day, model, project, tool"
        );
        for (tool, arguments) in [
            (
                "usage_summary",
                json!({"since": "garbage", "workspace": "nope"}),
            ),
            (
                "usage_breakdown",
                json!({"by": "model", "since": "garbage", "account": "nope"}),
            ),
        ] {
            let error = run(&copy.store(), tool, arguments).unwrap_err();
            assert_eq!(error, "Invalid isoformat string: 'garbage'", "{tool}");
        }
    }

    #[test]
    fn an_unknown_scope_is_refused_even_when_the_store_has_too_little_history() {
        let copy = Copy::of("store-v11.sqlite");
        for tool in ["warnings", "improvement_plan"] {
            let error = run(&copy.store(), tool, json!({"workspace": "nope"})).unwrap_err();
            assert_eq!(error, "unknown workspace", "{tool}");
        }
    }

    #[test]
    fn a_store_problem_while_the_scope_is_resolved_is_a_tool_error_without_a_path_or_sql() {
        // no `workspaces` table: the lookup of the scope fails, which is a store problem, not an unknown workspace
        let copy = Copy::of("store-rich/store-v11.sqlite");
        rusqlite::Connection::open(copy.store())
            .unwrap()
            .execute_batch("DROP TABLE workspace_repos; DROP TABLE workspaces;")
            .unwrap();
        for tool in ["usage_summary", "warnings", "improvement_plan"] {
            let error = run(
                &copy.store(),
                tool,
                json!({"workspace": "acme", "by": "model"}),
            )
            .unwrap_err();
            assert!(
                error.starts_with("cannot read the store"),
                "{tool}: {error}"
            );
            assert!(
                !error.contains("workspaces")
                    && !error.contains("SELECT")
                    && !error.contains(copy.0.to_str().unwrap()),
                "{error}"
            );
        }
    }

    #[test]
    fn a_filter_of_the_wrong_type_is_an_error() {
        let copy = Copy::of("store-v11.sqlite");
        for tool in [
            "usage_summary",
            "usage_breakdown",
            "warnings",
            "improvement_plan",
        ] {
            for (key, value) in [
                ("workspace", json!(5)),
                ("account", json!(["a"])),
                ("workspace", json!(true)),
            ] {
                let text =
                    run(&copy.store(), tool, json!({ key: value, "by": "model" })).unwrap_err();
                assert_eq!(text, format!("argument `{key}` must be a string or null"));
            }
        }
    }

    #[test]
    fn the_scope_type_check_comes_before_the_store_is_opened() {
        // no store at all: a wrong type is still named first, and a good scope reaches the store lookup
        let none = || Err(StoreError::NoStore);
        for tool in ["usage_summary", "warnings"] {
            let error = run_tool(tool, &args(json!({"workspace": 5})), FROZEN_NOW, none);
            assert_eq!(
                error.unwrap_err(),
                "argument `workspace` must be a string or null"
            );
            let error = run_tool(tool, &args(json!({"workspace": "acme"})), FROZEN_NOW, none);
            assert_eq!(
                error.unwrap_err(),
                "no store yet: nothing has been ingested into the data folder"
            );
        }
    }
}
