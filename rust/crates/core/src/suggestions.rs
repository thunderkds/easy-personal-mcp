//! The read-only connector views, ported from `src/easy_personal/connectors/suggestions.py` (the catalog view, the
//! status, the usage-based suggestions) and the read half of `GET /api/connectors` (`connectors_list` and `_entry_json`
//! of `surfaces/api.py`). T046.
//!
//! **Presence is never asked.** Python reports a connector as configured only when the OS keychain holds its item *and*
//! `claude mcp get` knows the entry; with no keychain backend, or no `claude` on the path, every connector is "not
//! configured" (the state of the goldens). This build reads no keychain and starts no process (both arrive with T050,
//! behind the owner's approval of a keychain crate), so [`configured_ids`] is empty: on a machine that does have both,
//! Python can say `true` where this says `false`. [`configured_ids`] is the one place T050 changes.
//!
//! Nothing here writes, and nothing returns a secret: the catalog holds public names and URLs, and the stored
//! connector rows hold only an id, an alias, a provider id and a workspace name (the secrets live in the keychain).
//! The suggestions read counters over WebFetch hosts and tool names, never message content.

use crate::catalog::{self, CatalogError};
use crate::store::StoreError;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Why a view could not be built. The catalog cannot fail in a shipped build (a test parses every embedded file);
/// the store can.
#[derive(Debug)]
pub enum ViewError {
    Catalog(CatalogError),
    Store(StoreError),
}

impl fmt::Display for ViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewError::Catalog(e) => write!(f, "{e}"),
            ViewError::Store(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ViewError {}

impl From<CatalogError> for ViewError {
    fn from(e: CatalogError) -> Self {
        ViewError::Catalog(e)
    }
}

impl From<rusqlite::Error> for ViewError {
    fn from(e: rusqlite::Error) -> Self {
        ViewError::Store(StoreError::from(e))
    }
}

type Result<T> = std::result::Result<T, ViewError>;

/// Registrable domain -> provider id, in Python's `DOMAINS` order (the first match wins). A host matches the domain
/// itself or any subdomain of it.
const DOMAINS: [(&str, &str); 8] = [
    ("slack.com", "slack"),
    ("discord.com", "discord"),
    ("discord.gg", "discord"),
    ("sendgrid.com", "sendgrid"),
    ("mail.google.com", "gmail"),
    ("gmail.googleapis.com", "gmail"),
    ("teams.microsoft.com", "teams"),
    ("graph.microsoft.com", "teams"),
];

const NOTE: &str = "Counts cover sessions ingested since this version; run `easy-personal ingest` to refresh. Bash first words are counted but map to no provider yet.";

/// The catalog ids whose connector is configured: always none (see the module text).
pub fn configured_ids() -> Vec<&'static str> {
    Vec::new()
}

/// `connector_catalog`: `{"connectors": [{id, provider, category, claude_ai_connector, claude_ai_link, fields:[{label, secret}]}]}`.
pub fn catalog_view() -> Result<Value> {
    let connectors: Vec<Value> = catalog::entries()?
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "provider": e.provider,
                "category": e.category,
                "claude_ai_connector": e.claude_ai_connector,
                "claude_ai_link": e.claude_ai_link,
                "fields": e.fields.iter().map(|f| json!({"label": f.label, "secret": f.secret})).collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(json!({ "connectors": connectors }))
}

/// `connector_status`: `{"connectors": [{id, configured}]}` for every catalog id.
pub fn status() -> Value {
    let done = configured_ids();
    let connectors: Vec<Value> = catalog::ids()
        .into_iter()
        .map(|id| json!({"id": id, "configured": done.contains(&id)}))
        .collect();
    json!({ "connectors": connectors })
}

fn provider_of_host(host: &str) -> Option<&'static str> {
    DOMAINS
        .iter()
        .find(|(domain, _)| {
            host == *domain || host.strip_suffix(domain).is_some_and(|h| h.ends_with('.'))
        })
        .map(|(_, provider)| *provider)
}

/// `connector_suggestions`: providers seen in use but not yet configured, with the call counts that point to them,
/// most calls first (ties keep catalog order), and the fixed note.
pub fn suggestions(conn: &Connection) -> Result<Value> {
    let skip = configured_ids();
    let entries = catalog::entries()?;
    let mut domain_calls: BTreeMap<&str, i64> = BTreeMap::new();
    let mut hosts = conn
        .prepare("SELECT value, COUNT(*) FROM tool_signals WHERE kind = 'domain' GROUP BY value")?;
    for row in hosts.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
        let (host, n) = row?;
        if let Some(provider) = provider_of_host(&host) {
            *domain_calls.entry(provider).or_insert(0) += n;
        }
    }
    let mut mcp_calls: BTreeMap<&str, i64> = BTreeMap::new();
    let mut official: BTreeSet<&str> = BTreeSet::new(); // reached through a claude.ai connector: already available
    let mut names = conn
        .prepare("SELECT tool_names FROM events WHERE tool_names LIKE '%mcp\\_\\_%' ESCAPE '\\'")?;
    for row in names.query_map([], |r| r.get::<_, String>(0))? {
        for tool in row?.split(',') {
            let parts: Vec<&str> = tool.split("__").collect();
            if parts[0] == "mcp" && parts.len() >= 3 {
                let server = parts[1].to_lowercase();
                for entry in &entries {
                    if server.contains(entry.id.as_str()) {
                        if server.starts_with("claude_ai_") {
                            official.insert(entry.id.as_str());
                        }
                        *mcp_calls.entry(entry.id.as_str()).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    let count = |calls: &BTreeMap<&str, i64>, id: &str| calls.get(id).copied().unwrap_or(0);
    let mut found: Vec<(i64, Value)> = entries
        .iter()
        .filter(|e| {
            let id = e.id.as_str();
            !skip.contains(&id)
                && !official.contains(id)
                && (count(&domain_calls, id) != 0 || count(&mcp_calls, id) != 0)
        })
        .map(|e| {
            let (web, mcp) = (count(&domain_calls, &e.id), count(&mcp_calls, &e.id));
            (
                web + mcp,
                json!({"id": e.id, "provider": e.provider, "webfetch_calls": web, "mcp_calls": mcp}),
            )
        })
        .collect();
    found.sort_by_key(|(total, _)| std::cmp::Reverse(*total)); // stable, like Python's `sort(key=-total)`
    let found: Vec<Value> = found.into_iter().map(|(_, v)| v).collect();
    Ok(json!({ "suggestions": found, "note": NOTE }))
}

/// `GET /api/connectors`: `{"connectors": [...], "workspace_connectors": [...]}`. The catalog entries are sorted by
/// `(category, claude.ai connector first, provider)`; each carries its fields with the env name, label and `secret`
/// flag, and `configured`. `workspace_connectors` are the stored rows `{id, alias, provider, workspace}` by workspace
/// name, then alias.
pub fn connectors_list(conn: &Connection) -> Result<Value> {
    let mut entries = catalog::entries()?;
    entries.sort_by(|a, b| {
        (&a.category, !a.claude_ai_connector, &a.provider).cmp(&(
            &b.category,
            !b.claude_ai_connector,
            &b.provider,
        ))
    });
    let done = configured_ids();
    let connectors: Vec<Value> = entries
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "category": e.category,
                "provider": e.provider,
                "claude_ai_connector": e.claude_ai_connector,
                "claude_ai_link": e.claude_ai_link,
                "fields": e.fields.iter()
                    .map(|f| json!({"env": f.env, "label": f.label, "secret": f.secret}))
                    .collect::<Vec<_>>(),
                "configured": done.contains(&e.id.as_str()),
            })
        })
        .collect();
    let mut stored = conn.prepare(
        "SELECT c.id, c.alias, c.provider, w.name FROM connectors c \
         JOIN workspaces w ON w.id = c.workspace_id ORDER BY w.name, c.alias",
    )?;
    let workspace_connectors = stored
        .query_map([], |r| {
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "alias": r.get::<_, String>(1)?,
                "provider": r.get::<_, String>(2)?,
                "workspace": r.get::<_, String>(3)?,
            }))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(json!({ "connectors": connectors, "workspace_connectors": workspace_connectors }))
}
