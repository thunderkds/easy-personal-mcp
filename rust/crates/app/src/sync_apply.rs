//! `POST /api/sync` (T050, FR-019, FR-033): Python's `sync_apply` handler (`surfaces/api.py`) over `sync.plan_all` and
//! `sync.apply`. The body is `{"account"?: id, "confirm"?: bool}`: no `account` is every confirmed Claude account.
//! The plan's lines are the answer; with `confirm: true` it is applied through `claude` step by step (`core/src/sync.rs`),
//! each step recorded right after it succeeds, the first failure stops the run (status 1; `connector sync` or the same
//! request finishes it). The account's folders come from its row in the store, never from the request: the request names
//! only an account id. Agent folders are detected first, as Python's `accounts.detect` does.

use crate::api::{Api, agent_root};
use crate::claude_cli::SystemClaude;
use crate::connector_writes::{applied, outcome};
use crate::writes::{account_body, answer, fail, run};
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::Response;
use easy_personal_core::sync::{self, SyncError};
use easy_personal_core::workspaces::WriteFail;
use serde_json::Value;

const NOT_AN_ID: &str = "account must be an account id";

/// `_sync_account` on a JSON value: absent, `null` or `""` is every account; an integer or a digit string is an id.
fn account(raw: Option<&Value>) -> Result<Option<String>, ()> {
    match raw {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.is_empty() => Ok(None),
        Some(Value::String(s)) if s.bytes().all(|b| b.is_ascii_digit()) => Ok(Some(s.clone())),
        Some(Value::Number(n)) if n.is_i64() || n.is_u64() => Ok(Some(n.to_string())),
        Some(_) => Err(()),
    }
}

pub(crate) async fn sync_apply(State(api): State<Api>, req: Request) -> Response {
    let (yes, account) = match account_body(req).await {
        Ok((_, body)) => match account(body.get("account")) {
            Ok(account) => (
                body.get("confirm")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                account,
            ),
            Err(()) => return fail(StatusCode::BAD_REQUEST, NOT_AN_ID),
        },
        Err(response) => return response,
    };
    run(
        api,
        Some(agent_root()),
        StatusCode::OK,
        || (),
        move |conn, ()| {
            let claude = SystemClaude::new();
            // `-5` (a negative integer) names no account: Python looks it up as a folder and finds none
            let steps = sync::plan_all(conn, account.as_deref(), &claude).map_err(|e| match e {
                SyncError::NotFound(text) | SyncError::Invalid(text) => {
                    WriteFail::Refused { status: 404, text }
                }
                other => WriteFail::from(other),
            })?;
            let mut lines: Vec<String> = steps.iter().map(sync::Step::line).collect();
            let o = if !steps.iter().any(sync::Step::is_applied) {
                lines.push("nothing to change".into());
                outcome(0, lines, false)
            } else if !yes {
                outcome(0, lines, true)
            } else {
                let status = applied(conn, &steps, &claude, &mut lines)?;
                outcome(status, lines, false)
            }?;
            Ok(answer(&o))
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::account;
    use serde_json::json;

    #[test]
    fn the_account_value_is_read_as_pythons_sync_account() {
        assert_eq!(account(None), Ok(None));
        assert_eq!(account(Some(&json!(null))), Ok(None));
        assert_eq!(account(Some(&json!(""))), Ok(None));
        assert_eq!(account(Some(&json!(2))), Ok(Some("2".into())));
        assert_eq!(account(Some(&json!(-5))), Ok(Some("-5".into())));
        assert_eq!(account(Some(&json!("12"))), Ok(Some("12".into())));
        for bad in [
            json!(true),
            json!(1.5),
            json!("x"),
            json!("-1"),
            json!([1]),
            json!({}),
        ] {
            assert_eq!(account(Some(&bad)), Err(()), "{bad}");
        }
    }
}
