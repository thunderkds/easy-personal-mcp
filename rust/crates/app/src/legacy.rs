//! Pre-workspace connectors (T050, FR-034, HUN-67): `GET /api/connectors/legacy` and `POST /api/connectors/legacy`,
//! Python's `connectors/legacy.py` (`detect`, `adopt`) behind `surfaces/api.py` (`connectors_legacy`,
//! `connectors_legacy_adopt`).
//!
//! Legacy = a catalog id with no `connectors` row that has a keychain item `easy-personal/<id>` and/or an entry in the
//! **default** Claude account that points at `easy-personal-launch exec <id>`. Adoption inserts the row
//! `(id, alias = id, orphan)` and, only after `confirm: true`, runs `claude mcp remove <id> --scope user` in the default
//! account. An entry of that name that is not ours is never removed; the keychain is never written; no other account
//! and no other name is touched. No line holds a secret.
//!
//! A `claude mcp get` that could not run (it did not start or timed out) is not "no entry" (T066; Python takes it as
//! one): adoption refuses with [`NOT_RUN`] and changes nothing, and `doctor`'s note says the check did not run. The
//! read-only `GET` keeps Python's answer (the entries it could not check are absent).

use crate::api::{Api, Failure, serve};
use crate::claude_cli::SystemClaude;
use crate::connector_writes::{launch_failed, outcome};
use crate::keychain::{self, Keychain, LaunchError};
use crate::writes::{account_body, answer, run};
use axum::extract::{Request, State};
use axum::http::{StatusCode, Uri};
use axum::response::Response;
use easy_personal_core::catalog;
use easy_personal_core::pyjson::sh_quote;
use easy_personal_core::sync::{self, Claude, NotRun};
use easy_personal_core::workspaces::{Outcome, WriteFail, is_constraint};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// `adopt`'s answer when a `claude mcp get` could not run.
pub(crate) const NOT_RUN: &str =
    "the `claude` CLI could not run (it did not start or timed out); nothing was adopted.";

/// What [`detect`] found, and whether a `claude mcp get` could not run (then an id that has only an entry is missing).
pub(crate) struct Detected {
    pub(crate) found: Vec<Legacy>,
    pub(crate) claude_not_run: bool,
}

pub(crate) struct Legacy {
    pub(crate) id: &'static str,
    keychain: bool,
    /// the default account holds an entry that points at our launcher (ours: may be removed)
    claude: bool,
    /// same name, but not our launcher entry: never removed
    foreign: bool,
}

/// `detect`: the legacy catalog ids not yet stored, in catalog order. The checks run in parallel, as Python's
/// thread pool does (each `claude mcp get` takes most of a second).
pub(crate) fn detect(
    conn: &Connection,
    kc: &dyn Keychain,
    claude: &(dyn Claude + Sync),
) -> Result<Detected, LaunchError> {
    let stored: BTreeSet<String> = conn
        .prepare("SELECT id FROM connectors")
        .and_then(|mut s| s.query_map([], |r| r.get(0))?.collect())
        .map_err(|_| LaunchError(crate::api::INTERNAL.into()))?;
    let has_claude = claude.on_path();
    let names: Vec<&'static str> = catalog::ids()
        .into_iter()
        .filter(|n| !stored.contains(*n))
        .collect();
    type Checked = Result<(Option<Legacy>, bool), LaunchError>;
    let check = |name: &'static str| -> Checked {
        let in_keychain = keychain::has_item(kc, name)?;
        let (got, not_run) = if has_claude {
            match sync::entry_exists(claude, name, None) {
                Ok(got) => (got, false),
                Err(NotRun) => (None, true),
            }
        } else {
            (None, false)
        };
        let ours = got
            .as_ref()
            .is_some_and(|g| sync::points_at_launcher(g, name));
        let item = (in_keychain || ours).then_some(Legacy {
            id: name,
            keychain: in_keychain,
            claude: ours,
            foreign: got.is_some() && !ours,
        });
        Ok((item, not_run))
    };
    let results: Vec<Checked> = std::thread::scope(|scope| {
        let handles: Vec<_> = names
            .iter()
            .map(|&name| scope.spawn(move || check(name)))
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or_else(|_| Err(LaunchError(crate::api::INTERNAL.into())))
            })
            .collect()
    });
    let mut detected = Detected {
        found: Vec::new(),
        claude_not_run: false,
    };
    for result in results {
        let (item, not_run) = result?;
        detected.found.extend(item);
        detected.claude_not_run |= not_run;
    }
    Ok(detected)
}

/// `GET /api/connectors/legacy`: `{"legacy": [{id, keychain, claude, foreign}]}`, or 503 with the keychain text. The
/// store is only read.
pub(crate) async fn legacy_status(State(api): State<Api>, uri: Uri) -> Response {
    serve(api, uri, |conn, _q, _now| {
        let kc = keychain::system();
        let found = detect(conn, &*kc, &SystemClaude::new())
            .map_err(|e| -> Failure { (StatusCode::SERVICE_UNAVAILABLE, e.0) })?
            .found;
        let items: Vec<Value> = found
            .iter()
            .map(|i| json!({"id": i.id, "keychain": i.keychain, "claude": i.claude, "foreign": i.foreign}))
            .collect();
        Ok(json!({ "legacy": items }))
    })
    .await
}

/// `POST /api/connectors/legacy` with `{"confirm"?}`: the plan, or (confirmed) the adoption.
pub(crate) async fn legacy_adopt(State(api): State<Api>, req: Request) -> Response {
    let yes = match account_body(req).await {
        Ok((_, body)) => body
            .get("confirm")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        Err(response) => return response,
    };
    run(
        api,
        None,
        StatusCode::OK,
        || (),
        move |conn, ()| {
            let kc = keychain::system();
            adopt(conn, yes, &*kc, &SystemClaude::new()).map(|o| answer(&o))
        },
    )
    .await
}

pub(crate) fn adopt(
    conn: &Connection,
    yes: bool,
    kc: &dyn Keychain,
    claude: &(dyn Claude + Sync),
) -> Result<Outcome, WriteFail> {
    let found = match detect(conn, kc, claude) {
        Ok(detected) if detected.claude_not_run => {
            return outcome(1, vec![NOT_RUN.into()], false);
        }
        Ok(detected) => detected.found,
        Err(e) => return launch_failed(Vec::new(), e),
    };
    if found.is_empty() {
        return outcome(
            0,
            vec!["nothing to adopt: no pre-workspace connector found.".into()],
            false,
        );
    }
    let yes_no = |b: bool| if b { "yes" } else { "no" };
    let mut lines = vec!["pre-workspace connectors found:".to_string()];
    for item in &found {
        let note = if item.keychain {
            ""
        } else {
            " (no keychain item: it fails at launch until you add it again)"
        };
        lines.push(format!(
            "  {}: keychain {}, Claude Code default account {}{note}",
            item.id,
            yes_no(item.keychain),
            yes_no(item.claude)
        ));
    }
    lines.push("will change:".into());
    for item in &found {
        lines.push(format!(
            "  store: adopt {0} (id {0}, alias {0}) into orphan",
            item.id
        ));
        if item.claude {
            lines.push(format!(
                "  Claude Code (default account): remove {}",
                item.id
            ));
        } else if item.foreign {
            lines.push(format!(
                "  Claude Code (default account): {} is not an easy-personal entry; left as it is",
                item.id
            ));
        }
    }
    lines.push("  keychain: untouched".into());
    if !yes {
        return outcome(1, lines, true);
    }
    let orphan: i64 = conn.query_row("SELECT id FROM workspaces WHERE is_orphan = 1", [], |r| {
        r.get(0)
    })?;
    let mut failed = 0;
    for item in &found {
        let inserted = conn.execute(
            "INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES (?1, ?1, 0, ?1, ?2)",
            rusqlite::params![item.id, orphan],
        );
        match inserted {
            Ok(_) => {}
            Err(e) if is_constraint(&e) => {
                lines.push(format!(
                    "skipped {}: the store refused it (changed meanwhile).",
                    item.id
                ));
                failed = 1;
                continue;
            }
            Err(e) => return Err(e.into()),
        }
        lines.push(format!("done: {} adopted into orphan.", item.id));
        if !item.claude {
            continue;
        }
        let done = sync::remove_entry(claude, item.id, None);
        if done.code != 0 {
            failed = 1;
            lines.push(format!(
                "pending: `claude` could not remove {} (exit {}); the row stays in orphan. Run: CLAUDE_CONFIG_DIR unset: \
                 claude mcp remove {} --scope user",
                item.id,
                done.code,
                sh_quote(item.id)
            ));
        } else {
            lines.push(format!(
                "done: removed {} from Claude Code (default account).",
                item.id
            ));
        }
    }
    if failed == 0 {
        lines.push(
            "assign them to a workspace with `easy-personal connector move <id> <workspace>` when you are ready.".into(),
        );
    }
    outcome(failed, lines, false)
}
