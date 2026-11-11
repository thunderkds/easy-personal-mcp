//! The workspace list and the projects no workspace covers, ported from `src/easy_personal/core/workspaces.py`
//! (`_COLUMNS`, `_dict`, `list_all`, `_under`, `unassigned`; FR-035, FR-069, FR-070) and served as `GET /api/workspaces`
//! and `GET /api/workspaces/unassigned`. Read-only: every write of `workspaces.py` (add, edit, delete) stays in Python.
//!
//! The two statements of `list_all` and the one of `unassigned` have no caller input: nothing is bound because nothing
//! varies, and no SQL is built from text. `unassigned` costs each API message once (`GROUP BY message_id`, the maximum of
//! each token column), with the price table and the abstaining rule of `usage_breakdown` (`PriceTable::cost`, a model
//! missing from the table abstains), and finds a project's key with the shared `metrics::project_map` (owner project, else
//! `Scratch`, else the path). Path-aware cover is `scope::under` (`/a/b` never covers `/a/bc`).

use crate::metrics::{MetricsError, SCRATCH, folder, project_map, round_py};
use crate::prices::PriceTable;
use crate::scope::under;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap, HashSet};

type Result<T> = std::result::Result<T, MetricsError>;

/// `{"workspaces": [...]}`: orphan first, then by name (SQLite's `BINARY` order, as Python's `ORDER BY is_orphan DESC, name`);
/// each workspace `{id, name, color, icon, description, email, is_orphan, repos}` with `repos` in `repo_path` order.
pub fn list_all(conn: &Connection) -> Result<Value> {
    Ok(json!({ "workspaces": workspaces(conn)? }))
}

fn workspaces(conn: &Connection) -> Result<Vec<Value>> {
    let mut list = conn.prepare(
        "SELECT id, name, color, icon, description, email, is_orphan FROM workspaces ORDER BY is_orphan DESC, name",
    )?;
    let mut repos = conn.prepare(
        "SELECT repo_path FROM workspace_repos WHERE workspace_id = ?1 ORDER BY repo_path",
    )?;
    let rows = list.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<String>>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, Option<String>>(5)?,
            r.get::<_, i64>(6)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, name, color, icon, description, email, is_orphan) = row?;
        let repos: Vec<String> = repos
            .query_map([id], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        out.push(json!({
            "id": id, "name": name, "color": color, "icon": icon, "description": description,
            "email": email, "is_orphan": is_orphan != 0, "repos": repos,
        }));
    }
    Ok(out)
}

/// What one project has cost and when it was used, over every stored message.
#[derive(Default)]
struct Project {
    /// Python's `set`: a NULL session id is one member of its own.
    sessions: HashSet<Option<String>>,
    last: String,
    usd: f64,
    priced: usize,
    abstained: BTreeSet<String>,
}

/// `{"repos": [...]}`: real projects with stored usage that no workspace repo equals or lies above, newest use first
/// (`(last_used_utc, path)` descending). Each is `{path, name, sessions, api_equivalent_cost, last_used_utc}` over all
/// stored time; `api_equivalent_cost` is `{usd, abstained_models, price_table_date}`, `usd` rounded to six places and
/// `null` when nothing was priced and a model abstained.
pub fn unassigned(conn: &Connection) -> Result<Value> {
    let prices = PriceTable::shipped().map_err(MetricsError::Prices)?;
    let roots: Vec<String> = workspaces(conn)?
        .iter()
        .flat_map(|w| w["repos"].as_array().into_iter().flatten())
        .filter_map(|r| r.as_str().map(str::to_string))
        .collect();
    let pmap = project_map(conn)?;
    let mut projects: HashMap<String, Project> = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT MAX(project_path), MAX(session_id), MIN(ts_utc), MAX(model), MAX(input_tokens), MAX(output_tokens), \
         MAX(cache_creation_input_tokens), MAX(cache_read_input_tokens) \
         FROM events WHERE abstained = 0 AND message_id IS NOT NULL AND input_tokens IS NOT NULL \
         AND project_path IS NOT NULL GROUP BY message_id",
    )?;
    let rows = stmt.query_map([], |r| {
        let n = |i| r.get::<_, Option<i64>>(i).map(Option::unwrap_or_default);
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<String>>(3)?,
            [n(4)?, n(5)?, n(6)?, n(7)?],
        ))
    })?;
    for row in rows {
        let (path, session, last, model, t) = row?;
        let key = pmap.get(&path).unwrap_or(&path);
        if key == SCRATCH || roots.iter().any(|root| under(key, root)) {
            continue;
        }
        let project = projects.entry(key.clone()).or_default();
        project.sessions.insert(session);
        project.last = project.last.clone().max(last.unwrap_or_default());
        match prices.cost(model.as_deref(), t[0], t[1], t[2], t[3]) {
            Some(usd) => {
                project.usd += usd;
                project.priced += 1;
            }
            None => {
                let name = model.filter(|m| !m.is_empty());
                project
                    .abstained
                    .insert(name.unwrap_or_else(|| "(unknown)".into()));
            }
        }
    }
    let mut repos: Vec<(String, String, Value)> = projects
        .into_iter()
        .map(|(key, p)| {
            let usd = (p.priced > 0 || p.abstained.is_empty()).then(|| round_py(p.usd, 6));
            let body = json!({
                "path": key,
                "name": name_of(&key),
                "sessions": p.sessions.len(),
                "api_equivalent_cost": {
                    "usd": usd,
                    "abstained_models": p.abstained,
                    "price_table_date": prices.as_of,
                },
                "last_used_utc": p.last,
            });
            (p.last, key, body)
        })
        .collect();
    repos.sort_by(|a, b| (&b.0, &b.1).cmp(&(&a.0, &a.1)));
    Ok(json!({ "repos": repos.into_iter().map(|r| r.2).collect::<Vec<_>>() }))
}

/// Python's `Path(key).name or key`: the last component, or the key itself when it has none (`/`, an empty key).
fn name_of(key: &str) -> String {
    if key.is_empty() {
        return String::new();
    }
    folder(Some(key))
}

#[cfg(test)]
mod tests {
    use super::name_of;

    /// Expected values are Python's `Path(key).name or key` for the same keys.
    #[test]
    fn name_of_matches_python() {
        let cases = [
            ("/a/b", "b"),
            ("/a/b/", "b"),
            ("/", "/"),
            ("//", "//"),
            ("/a/./b", "b"),
            ("/a/b/.", "b"),
            (".", "."),
            ("..", ".."),
            ("/a/..", ".."),
            ("", ""),
            ("/./", "/./"),
            ("a//b", "b"),
            ("/a/b c", "b c"),
            ("/home/\u{e9}", "\u{e9}"),
        ];
        for (key, want) in cases {
            assert_eq!(name_of(key), want, "{key:?}");
        }
    }
}
