//! The connector write routes (T050, FR-017..FR-019): `POST /api/connectors` (add to a workspace), `DELETE
//! /api/connectors/{name}` (remove a stored connector, or a pre-workspace catalog one), `POST /api/connectors/{id}/move`
//! and `PUT /api/connectors/{id}/alias`. Python's `surfaces/api.py` (`connectors_add`, `connectors_remove`,
//! `connectors_move`, `connectors_alias`, `_run_writer`) over `connectors/writer.py` (`add_to_workspace`, `remove`) and
//! `connectors/sync.py` (`remove_connector`, `move_connector`, `set_alias`); the oracle is `rust/parity/golden-connectors/`.
//!
//! Every route is behind the guard in `server.rs`, reads its body as Python's `_json_body` does (415, 400, the 1 MiB cap
//! of `writes.rs`), and runs under the one write lock in a write session (`writes::run`: the store must exist and be v11).
//! The answer is Python's `{"status", "lines", "pending"}`: `pending` is true when the plan was shown and nothing was
//! changed (no `confirm: true`). A refusal is Python's status and fixed text; a `LaunchError` (no usable keychain, no
//! launcher, a bad connector name) is status 1 with its text as the last line. The connector name is checked before any
//! keychain or `claude` call. A secret typed into `values` goes to the keychain only: no line, log or store row holds it.
//!
//! Each write that touches both sides keeps them consistent as Python does: `add` writes the keychain item, then the row,
//! and deletes the item again when the row is refused; `remove` drops the account registrations through `claude` first
//! and deletes the item and the row only once no account holds one; a `claude` failure stops there, and
//! `connector sync` (or the same request again) finishes it.

// The error of the body readers is the finished answer (a `Response`), which an early `return` would send as it is.
#![allow(clippy::result_large_err)]

use crate::api::Api;
use crate::claude_cli::SystemClaude;
use crate::keychain::{self, Keychain, LaunchError};
use crate::writes::{account_body, answer, fail, run};
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::Response;
use easy_personal_core::catalog;
use easy_personal_core::pyjson::sh_quote;
use easy_personal_core::sync::{self, Claude};
use easy_personal_core::workspaces::{self, Outcome, WriteFail};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

type Done<T> = Result<T, WriteFail>;

pub(crate) fn outcome(status: i64, lines: Vec<String>, pending: bool) -> Done<Outcome> {
    Ok(Outcome {
        status,
        lines,
        pending,
    })
}

/// `_run_writer`'s `LaunchError` branch: status 1, the text as the last line.
pub(crate) fn launch_failed(mut lines: Vec<String>, error: LaunchError) -> Done<Outcome> {
    lines.push(error.0);
    outcome(1, lines, false)
}

/// `apply`'s answer as `_run_writer` shows it.
pub(crate) fn applied(
    conn: &Connection,
    steps: &[sync::Step],
    claude: &dyn Claude,
    lines: &mut Vec<String>,
) -> Done<i64> {
    match sync::apply(conn, steps, claude, lines).map_err(WriteFail::Store)? {
        Ok(status) => Ok(status),
        Err(text) => {
            lines.push(text);
            Ok(1)
        }
    }
}

/// A refusal that `_run_writer` answers with 400 whatever its own status (`own_status=False`).
fn as_400(failure: WriteFail) -> WriteFail {
    match failure {
        WriteFail::Refused { text, .. } => WriteFail::Refused { status: 400, text },
        other => other,
    }
}

fn refuse(status: u16, text: &str) -> WriteFail {
    WriteFail::Refused {
        status,
        text: text.into(),
    }
}

/// The `{name}` / `{id}` of a connector path as Starlette passes it: percent-decoded (a byte that is not UTF-8 becomes
/// U+FFFD). Only ever compared with stored ids, catalog ids and the name rule; never echoed.
pub(crate) fn path_segment(path: &str, prefix: &str, suffix: &str) -> String {
    let raw = path
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_suffix(suffix))
        .unwrap_or("");
    let bytes = raw.as_bytes();
    let hex = |b: Option<&u8>| b.and_then(|b| (*b as char).to_digit(16));
    let (mut out, mut i) = (Vec::with_capacity(bytes.len()), 0);
    while i < bytes.len() {
        match (bytes[i], hex(bytes.get(i + 1)), hex(bytes.get(i + 2))) {
            (b'%', Some(high), Some(low)) => {
                out.push((high * 16 + low) as u8);
                i += 3;
            }
            (byte, _, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn yes(body: &Map<String, Value>) -> bool {
    body.get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// A connector route: the body (`_json_body`), the route's own checks (`check`, before the store is opened), then
/// `work` in a write session with the real keychain and `claude`.
async fn connector_route<P: Send + 'static>(
    api: Api,
    req: Request,
    check: impl FnOnce(&str, &Map<String, Value>) -> Result<P, Response>,
    work: impl FnOnce(&Connection, P, &dyn Keychain, &dyn Claude) -> Done<Outcome> + Send + 'static,
) -> Response {
    let (path, body) = match account_body(req).await {
        Ok(parsed) => parsed,
        Err(response) => return response,
    };
    let prepared = match check(&path, &body) {
        Ok(prepared) => prepared,
        Err(response) => return response,
    };
    run(
        api,
        None,
        StatusCode::OK,
        move || prepared,
        move |conn, prepared| {
            let kc = keychain::system();
            work(conn, prepared, &*kc, &SystemClaude::new()).map(|o| answer(&o))
        },
    )
    .await
}

// ---- add ----

struct Add {
    entry: catalog::Entry,
    values: Map<String, Value>,
    workspace: String,
    alias: Option<String>,
    yes: bool,
}

/// `POST /api/connectors` with `{"provider", "workspace", "values": {ENV: value}, "alias"?, "confirm"?}`.
pub(crate) async fn connector_add(State(api): State<Api>, req: Request) -> Response {
    connector_route(
        api,
        req,
        |_, body| {
            let values = match body.get("values") {
                None => Map::new(),
                Some(Value::Object(map)) if map.values().all(Value::is_string) => map.clone(),
                Some(_) => {
                    return Err(fail(
                        StatusCode::BAD_REQUEST,
                        "values must map field names to strings",
                    ));
                }
            };
            // Python loads `str(provider)`: only a catalog id (a string) names an entry
            let entry = body
                .get("provider")
                .and_then(Value::as_str)
                .filter(|id| catalog::ids().contains(id))
                .and_then(|id| catalog::parse_bytes(id, catalog::file(id)?).ok())
                .ok_or_else(|| {
                    fail(
                        StatusCode::BAD_REQUEST,
                        "unknown provider; see GET /api/connectors",
                    )
                })?;
            let workspace = match body.get("workspace") {
                Some(Value::String(name)) if !name.is_empty() => name.clone(),
                _ => {
                    return Err(fail(
                        StatusCode::BAD_REQUEST,
                        "workspace is required (a workspace name)",
                    ));
                }
            };
            let alias = match body.get("alias") {
                None | Some(Value::Null) => None,
                Some(Value::String(alias)) => Some(alias.clone()),
                Some(_) => return Err(fail(StatusCode::BAD_REQUEST, "alias must be a string")),
            };
            Ok(Add {
                entry,
                values,
                workspace,
                alias,
                yes: yes(body),
            })
        },
        |conn, add, kc, _| add_to_workspace(conn, add, kc).map_err(as_400),
    )
    .await
}

/// `writer.new_id`: `c` and 8 hex digits from the OS random source.
fn new_id() -> Done<String> {
    let mut bytes = [0u8; 4];
    // no OS random source: the fixed 500, nothing written
    getrandom::fill(&mut bytes).map_err(|_| refuse(500, crate::api::INTERNAL))?;
    Ok(format!(
        "c{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}

/// `writer.add_to_workspace`: secrets in keychain `easy-personal/<new id>`, one row in the store. No `claude` call:
/// `connector sync` registers it in the accounts that picked the workspace (FR-029, FR-033).
fn add_to_workspace(conn: &Connection, add: Add, kc: &dyn Keychain) -> Done<Outcome> {
    let Add {
        entry,
        values,
        workspace,
        alias,
        yes,
    } = add;
    let ws = workspaces::by_name(conn, &workspace)?;
    let (ws_id, ws_name) = (
        ws["id"].as_i64().unwrap_or_default(),
        ws["name"].as_str().unwrap_or_default(),
    );
    if ws["is_orphan"] == true {
        return Err(refuse(
            400,
            "connectors are added to a workspace, never to orphan",
        ));
    }
    let custom = alias.is_some();
    let alias = alias.unwrap_or_else(|| format!("{ws_name}-{}", entry.id));
    if !keychain::valid_name(&alias) {
        return Err(refuse(400, "alias must match [a-z0-9][a-z0-9_-]{0,63}"));
    }
    if let Err(e) = keychain::require_backend(kc) {
        return launch_failed(Vec::new(), e);
    }
    let provider_held = conn
        .query_row(
            "SELECT 1 FROM connectors WHERE workspace_id = ?1 AND provider = ?2",
            params![ws_id, entry.id],
            |_| Ok(()),
        )
        .optional()?;
    if provider_held.is_some() {
        let line = format!(
            "workspace '{ws_name}' already has a {} connector; refusing to add another.",
            entry.id
        );
        return outcome(1, vec![line], false);
    }
    let alias_held = conn
        .query_row(
            "SELECT 1 FROM connectors c JOIN workspaces w ON w.id = c.workspace_id WHERE c.alias = ?1 AND w.is_orphan = 0",
            [&alias],
            |_| Ok(()),
        )
        .optional()?;
    if alias_held.is_some() {
        let line =
            format!("alias '{alias}' is in use by another connector; choose another with --alias.");
        return outcome(1, vec![line], false);
    }
    let connector_id = new_id()?;
    match keychain::has_item(kc, &connector_id) {
        Err(e) => return launch_failed(Vec::new(), e),
        Ok(true) => {
            let line = format!(
                "keychain item easy-personal/{connector_id} already exists; refusing to overwrite it."
            );
            return outcome(1, vec![line], false);
        }
        Ok(false) => {}
    }
    let mut env = Vec::new();
    for field in &entry.fields {
        let value = values.get(&field.env).and_then(Value::as_str).unwrap_or("");
        if value.is_empty() {
            return outcome(
                2,
                vec![format!("{} is empty; nothing was written.", field.label)],
                false,
            );
        }
        env.push((field.env.clone(), value.to_string()));
    }
    let masked: Vec<String> = entry
        .fields
        .iter()
        .map(|f| format!("{}=***", f.env))
        .collect();
    let mut lines = vec![
        format!("will change ({}):", entry.provider),
        format!(
            "  keychain: service easy-personal/{connector_id}, fields {}",
            masked.join(", ")
        ),
        format!("  store: connector {connector_id}, alias {alias}, workspace {ws_name}"),
        "  Claude Code: no change now; `easy-personal connector sync` registers it".into(),
    ];
    if !yes {
        return outcome(1, lines, true);
    }
    if let Err(e) = keychain::store(kc, &connector_id, &entry.command, &env, &[]) {
        return launch_failed(lines, e);
    }
    let inserted = conn.execute(
        "INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![connector_id, alias, custom as i64, entry.id, ws_id],
    );
    match inserted {
        Ok(_) => {}
        Err(e) if workspaces::is_constraint(&e) => {
            // roll back: no secret without its row
            if let Err(e) = keychain::remove(kc, &connector_id) {
                return launch_failed(lines, e);
            }
            lines.push(
                "the store refused the connector (changed meanwhile); keychain write rolled back."
                    .into(),
            );
            return outcome(1, lines, false);
        }
        Err(e) => {
            let _ = keychain::remove(kc, &connector_id);
            return Err(e.into());
        }
    }
    lines.push(format!(
        "connector {connector_id} ({alias}) added to workspace '{ws_name}'."
    ));
    lines.push("run `easy-personal connector sync` to register it in the accounts that picked this workspace.".into());
    outcome(0, lines, false)
}

// ---- remove ----

/// `DELETE /api/connectors/{name}`: a stored connector (`sync.remove_connector`), else a catalog id (the pre-workspace
/// `writer.remove`), else 404 `unknown connector` with no `claude` call (HUN-72: never remove what we did not create).
pub(crate) async fn connector_remove(State(api): State<Api>, req: Request) -> Response {
    connector_route(
        api,
        req,
        |path, body| Ok((path_segment(path, "/api/connectors/", ""), yes(body))),
        |conn, (name, yes), kc, claude| {
            let stored: Option<String> = conn
                .query_row("SELECT alias FROM connectors WHERE id = ?1", [&name], |r| {
                    r.get(0)
                })
                .optional()?;
            match stored {
                Some(alias) => remove_connector(conn, &name, &alias, yes, kc, claude),
                None if catalog::ids().contains(&name.as_str()) => {
                    remove_pre_workspace(&name, yes, kc, claude)
                }
                None => Err(refuse(404, "unknown connector")),
            }
        },
    )
    .await
}

/// `sync.remove_connector`: unregister from every account, then delete the keychain item and the row, only once no
/// account holds a registration.
fn remove_connector(
    conn: &Connection,
    connector_id: &str,
    alias: &str,
    yes: bool,
    kc: &dyn Keychain,
    claude: &dyn Claude,
) -> Done<Outcome> {
    let holders: Vec<i64> = conn
        .prepare("SELECT DISTINCT account_id FROM account_registrations WHERE connector_id = ?1 ORDER BY account_id")?
        .query_map([connector_id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let drop = BTreeSet::from([connector_id.to_string()]);
    let mut steps = Vec::new();
    for account in holders {
        let plan = sync::plan(conn, account, &drop, claude).map_err(workspaces::WriteFail::from)?;
        steps.extend(
            plan.into_iter()
                .filter(|s| s.connector_id() == connector_id || s.is_skipped()),
        );
    }
    let mut lines = vec![format!(
        "will change (connector {connector_id}, alias {alias}):"
    )];
    lines.extend(steps.iter().map(|s| format!("  {}", s.line())));
    lines.push(format!(
        "  keychain: delete service easy-personal/{connector_id}"
    ));
    lines.push(format!("  store: delete connector {connector_id}"));
    if steps.iter().any(sync::Step::is_skipped) {
        lines.push("an account that holds it can not be changed now; nothing was removed.".into());
        return outcome(1, lines, false);
    }
    if !yes {
        return outcome(1, lines, true);
    }
    if !steps.is_empty() {
        let status = applied(conn, &steps, claude, &mut lines)?;
        if status != 0 {
            return outcome(status, lines, false);
        }
    }
    let left: i64 = conn.query_row(
        "SELECT COUNT(*) FROM account_registrations WHERE connector_id = ?1",
        [connector_id],
        |r| r.get(0),
    )?;
    if left > 0 {
        // `claude` missing: only the commands were listed
        lines.push(
            "the connector stays until every account has dropped it; run this again afterwards."
                .into(),
        );
        return outcome(0, lines, false);
    }
    if let Err(e) = keychain::remove(kc, connector_id) {
        return launch_failed(lines, e);
    }
    conn.execute("DELETE FROM connectors WHERE id = ?1", [connector_id])?;
    lines.push(format!("connector {connector_id} removed."));
    outcome(0, lines, false)
}

/// `writer.remove`: a pre-workspace connector (a catalog id with no row): its entry in the default account, if it is
/// ours (it points at the launcher), and its keychain item.
fn remove_pre_workspace(
    name: &str,
    yes: bool,
    kc: &dyn Keychain,
    claude: &dyn Claude,
) -> Done<Outcome> {
    if let Err(e) = keychain::service_name(name) {
        return launch_failed(Vec::new(), e);
    }
    let snippet = format!("claude mcp remove {} --scope user", sh_quote(name));
    if !claude.on_path() {
        let line = format!(
            "the `claude` CLI is not on PATH; nothing was removed. Run `{snippet}` once it is installed."
        );
        return outcome(0, vec![line], false);
    }
    let Ok(got) = sync::entry_exists(claude, name, None) else {
        // not "no entry": the default account's entry may still point at the launcher
        let line =
            "the `claude` CLI could not run (it did not start or timed out); nothing was removed.";
        return outcome(1, vec![line.into()], false);
    };
    if got
        .as_ref()
        .is_some_and(|got| !sync::points_at_launcher(got, name))
    {
        let line = format!(
            "the default Claude account has a '{name}' entry that easy-personal did not create; nothing was removed."
        );
        return outcome(1, vec![line], false);
    }
    let mut lines = vec![match got {
        None => format!(
            "will change: delete keychain service easy-personal/{name}; no Claude Code entry to remove"
        ),
        Some(_) => {
            format!("will change: delete keychain service easy-personal/{name}; run `{snippet}`")
        }
    }];
    if !yes {
        return outcome(1, lines, true);
    }
    if got.is_some() {
        let done = sync::remove_entry(claude, name, None);
        if done.code != 0 {
            lines.push(format!(
                "`claude mcp remove` failed (exit {}); keychain left as it was.",
                done.code
            ));
            return outcome(1, lines, false);
        }
    }
    if let Err(e) = keychain::remove(kc, name) {
        return launch_failed(lines, e);
    }
    lines.push(format!("connector '{name}' removed."));
    outcome(0, lines, false)
}

// ---- move and alias ----

/// `POST /api/connectors/{id}/move` with `{"workspace": name, "confirm"?}`.
pub(crate) async fn connector_move(State(api): State<Api>, req: Request) -> Response {
    connector_route(
        api,
        req,
        |path, body| match body.get("workspace") {
            Some(Value::String(name)) if !name.is_empty() => Ok((
                path_segment(path, "/api/connectors/", "/move"),
                name.clone(),
                yes(body),
            )),
            _ => Err(fail(
                StatusCode::BAD_REQUEST,
                "workspace is required (a workspace name)",
            )),
        },
        |conn, (id, workspace, yes), _, claude| {
            workspaces::move_connector(conn, &id, &workspace, yes, claude)
        },
    )
    .await
}

/// `PUT /api/connectors/{id}/alias` with `{"alias": text | null, "confirm"?}` (`null` or no `alias`: the default).
pub(crate) async fn connector_alias(State(api): State<Api>, req: Request) -> Response {
    connector_route(
        api,
        req,
        |path, body| {
            let alias = match body.get("alias") {
                None | Some(Value::Null) => None,
                Some(Value::String(alias)) => Some(alias.clone()),
                Some(_) => {
                    return Err(fail(
                        StatusCode::BAD_REQUEST,
                        "alias must be a string, or null for the default",
                    ));
                }
            };
            Ok((
                path_segment(path, "/api/connectors/", "/alias"),
                alias,
                yes(body),
            ))
        },
        |conn, (id, alias, yes), _, claude| {
            workspaces::set_alias(conn, &id, alias.as_deref(), yes, claude)
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::path_segment;

    #[test]
    fn a_path_segment_is_percent_decoded_like_starlette() {
        assert_eq!(
            path_segment("/api/connectors/slack", "/api/connectors/", ""),
            "slack"
        );
        assert_eq!(
            path_segment("/api/connectors/Bad%20Name", "/api/connectors/", ""),
            "Bad Name"
        );
        assert_eq!(
            path_segment("/api/connectors/c1/move", "/api/connectors/", "/move"),
            "c1"
        );
        assert_eq!(
            path_segment("/api/connectors/a%2Fb/alias", "/api/connectors/", "/alias"),
            "a/b"
        );
        assert_eq!(
            path_segment("/api/connectors/%ff%zz+", "/api/connectors/", ""),
            "\u{fffd}%zz+"
        );
    }
}
