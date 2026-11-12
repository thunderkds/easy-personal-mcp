//! The workspace list and the projects no workspace covers, ported from `src/easy_personal/core/workspaces.py`
//! (`_COLUMNS`, `_dict`, `list_all`, `_under`, `unassigned`; FR-035, FR-069, FR-070) and served as `GET /api/workspaces`
//! and `GET /api/workspaces/unassigned`. The writes of `workspaces.py` (add, edit, delete: T048) are at the end of the file.
//!
//! The two statements of `list_all` and the one of `unassigned` have no caller input: nothing is bound because nothing
//! varies, and no SQL is built from text. `unassigned` costs each API message once (`GROUP BY message_id`, the maximum of
//! each token column), with the price table and the abstaining rule of `usage_breakdown` (`PriceTable::cost`, a model
//! missing from the table abstains), and finds a project's key with the shared `metrics::project_map` (owner project, else
//! `Scratch`, else the path). Path-aware cover is `scope::under` (`/a/b` never covers `/a/bc`).

use crate::metrics::{MetricsError, SCRATCH, folder, project_map, resolve_path, round_py};
use crate::prices::PriceTable;
use crate::scope::under;
use rusqlite::{Connection, OptionalExtension};
use serde_json::{Map, Value, json};
use std::collections::{BTreeSet, HashMap, HashSet};

use crate::store::{self, StoreError};
use crate::sync::{self, SyncError};
use crate::writer::{self, WriteError};
use rusqlite::{ErrorCode, Transaction, TransactionBehavior, params, params_from_iter};
use std::path::Path;

type Result<T> = std::result::Result<T, MetricsError>;
/// The result of a write (shared by `accounts`): its refusal, or the value.
pub(crate) type Done<T> = std::result::Result<T, WriteFail>;
/// A `repos` value already normalised: see [`prepare_repos`].
pub type Repos = Done<Vec<Done<String>>>;

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

// ---- writes (T048) ----
//
// `add`, `edit_in_txn`, `delete_to_orphan` and the plan-first `patch` / `delete` of `workspaces.py` and
// `connectors/sync.py` (`rename_workspace`, `delete_workspace`, `_change`), store only. Every refusal is a
// [`WriteFail::Refused`] with Python's status and text; nothing is stored on error (an uncommitted transaction rolls
// back when it is dropped). What Python does with the `claude` CLI (`sync.apply`) is not done here: a confirmed change
// whose plan holds an `add` or `remove` step is [`WriteFail::NeedsClaude`] and rolls back (T050 will run it).

const ORPHAN: &str = "orphan";
const TEXT_FIELDS: [(&str, usize); 4] = [
    ("color", 7),
    ("icon", 32),
    ("description", 500),
    ("email", 254),
];
pub const FIELDS: [&str; 6] = ["name", "color", "icon", "description", "email", "repos"];
const NAME_RULE: &str = "name must match [a-z0-9][a-z0-9-]{0,39}";

/// Why a write did not happen.
#[derive(Debug)]
pub enum WriteFail {
    /// Python's `WorkspaceError` / `AccountError` / `ConnectorError`: its status and fixed text (none names the input).
    Refused { status: u16, text: String },
    /// The change would need the `claude` CLI (an account registration to add or remove): refused until T050.
    NeedsClaude,
    /// The store could not be read or written.
    Store(StoreError),
    /// The write handle could not be opened (a stale or newer schema, busy, not a store).
    Write(WriteError),
}

/// A rule of the store (UNIQUE, CHECK, foreign key, trigger) that no check above it caught: refused, nothing stored
/// (Python lets such an `IntegrityError` escape as a 500).
impl From<rusqlite::Error> for WriteFail {
    fn from(e: rusqlite::Error) -> Self {
        if is_constraint(&e) {
            refuse(400, "the store refused the change; nothing was changed")
        } else {
            WriteFail::Store(StoreError::from(e))
        }
    }
}

impl From<MetricsError> for WriteFail {
    fn from(e: MetricsError) -> Self {
        match e {
            MetricsError::Store(e) => WriteFail::Store(e),
            // the list read takes no caller text: nothing else can come out of it
            _ => WriteFail::Store(StoreError::Unreadable("workspace list".into())),
        }
    }
}

impl From<SyncError> for WriteFail {
    fn from(e: SyncError) -> Self {
        match e {
            SyncError::Store(e) => WriteFail::Store(e),
            // `plan` is only asked for ids that exist, and takes no text: neither can happen
            _ => WriteFail::Store(StoreError::Unreadable("sync plan".into())),
        }
    }
}

pub(crate) fn refuse(status: u16, text: impl Into<String>) -> WriteFail {
    WriteFail::Refused {
        status,
        text: text.into(),
    }
}

/// An `IntegrityError` in Python: a UNIQUE, CHECK, foreign key or trigger rule said no.
pub(crate) fn is_constraint(e: &rusqlite::Error) -> bool {
    matches!(e, rusqlite::Error::SqliteFailure(f, _) if f.code == ErrorCode::ConstraintViolation)
}

/// `BEGIN IMMEDIATE`: waits for the write lock up to the connection's busy timeout (5 s).
pub(crate) fn begin(conn: &Connection) -> Done<Transaction<'_>> {
    Ok(Transaction::new_unchecked(
        conn,
        TransactionBehavior::Immediate,
    )?)
}

/// One write request's handle on the store: checks that a store with tables is there (a write never creates one),
/// opens it for writing (`writer::open_for_write`: 5 s busy timeout, foreign keys on, schema v11 only), runs
/// `detect_in` (Python's `accounts.detect` that precedes an account write: an `INSERT OR IGNORE` of the agent folders
/// of that root) and then `work`, and closes the handle before it returns. Opened only here, never at start-up.
pub fn write_session<T>(
    path: &Path,
    detect_in: Option<&Path>,
    work: impl FnOnce(&Connection) -> Done<T>,
) -> Done<T> {
    let probe = store::open_checked(path).map_err(WriteFail::Store)?;
    let has_tables: bool =
        probe.query_row("SELECT EXISTS (SELECT 1 FROM sqlite_master)", [], |r| {
            r.get(0)
        })?;
    drop(probe);
    if !has_tables {
        return Err(WriteFail::Store(StoreError::NoStore));
    }
    let mut store = writer::open_for_write(path).map_err(WriteFail::Write)?;
    if let Some(root) = detect_in {
        crate::ingest::accounts::detect(&mut store, root).map_err(WriteFail::Write)?;
    }
    work(store.conn())
}

/// What a plan-first write answered: Python's `status` (0 applied, 1 stopped at the confirmation), the shown `lines`
/// and `pending` (the plan was shown and nothing was changed).
#[derive(Debug, PartialEq)]
pub struct Outcome {
    pub status: i64,
    pub lines: Vec<String>,
    pub pending: bool,
}

/// `str.isspace()`: Unicode White_Space plus U+001C to U+001F (Rust's `char::is_whitespace` lacks the four).
pub fn py_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// `str.strip()`: [`py_space`] at both ends. The one copy: `catalog` uses it too.
pub fn py_strip(text: &str) -> &str {
    text.trim_matches(py_space)
}

/// `os.path.expanduser`: `~` is `$HOME` (an unset `HOME` leaves the text as it is: Python asks the password database
/// for the current user, which this build does not), `~name` is that user's home from `/etc/passwd`; an unknown name
/// leaves the text as it is. A trailing `/` of the home is dropped.
pub(crate) fn expanduser(path: &str) -> String {
    let Some(rest) = path.strip_prefix('~') else {
        return path.to_string();
    };
    let end = rest.find('/').unwrap_or(rest.len());
    let (name, tail) = rest.split_at(end);
    let home = if name.is_empty() {
        std::env::var("HOME").ok()
    } else {
        std::fs::read_to_string("/etc/passwd")
            .ok()
            .and_then(|passwd| {
                passwd.lines().find_map(|line| {
                    let mut fields = line.split(':');
                    (fields.next() == Some(name)).then(|| fields.nth(4).map(str::to_string))?
                })
            })
    };
    match home {
        Some(home) => {
            let joined = format!("{}{tail}", home.trim_end_matches('/'));
            if joined.is_empty() {
                "/".into()
            } else {
                joined
            }
        }
        None => path.to_string(),
    }
}

/// `normalise_repo`: `~` expanded, made absolute and resolved, no trailing slash. The folder need not exist.
pub fn normalise_repo(path: &Value) -> Done<String> {
    let text = match path {
        Value::String(text) if !py_strip(text).is_empty() && !text.contains('\0') => text,
        _ => return Err(refuse(400, "repo path must be a non-empty string")),
    };
    let expanded = expanduser(py_strip(text));
    if !expanded.starts_with('/') {
        return Err(refuse(400, "repo path must be absolute or start with ~"));
    }
    // a resolved path that is not UTF-8 matches no folder the store can hold (Python answers 500): refused
    match resolve_path(Path::new(&expanded)).to_str() {
        Some(resolved) => Ok(resolved.to_string()),
        None => Err(refuse(400, "repo path must be a non-empty string")),
    }
}

/// The repo paths of a `repos` value after [`normalise_repo`], one result each in list order (the list stops at the
/// first refusal: nothing after it is read), or the refusal for a value that is not a list. Resolving a path is
/// filesystem work: a route does it before it takes the write lock and passes this to [`add`] / [`edit`] / [`patch`],
/// where [`check_repos`] reports the first refusal at the position Python reports it. `None` is an absent `repos`: `[]`.
pub fn prepare_repos(repos: Option<&Value>) -> Repos {
    let list = match repos {
        None => return Ok(Vec::new()),
        Some(Value::Array(list)) => list,
        Some(_) => return Err(refuse(400, "repos must be a list of paths")),
    };
    let mut out = Vec::new();
    for repo in list {
        let one = normalise_repo(repo);
        let stop = one.is_err();
        out.push(one);
        if stop {
            break;
        }
    }
    Ok(out)
}

fn check_name(name: &Value) -> Done<String> {
    let slug = |s: &str| {
        let bytes = s.as_bytes();
        (1..=40).contains(&bytes.len())
            && matches!(bytes[0], b'a'..=b'z' | b'0'..=b'9')
            && bytes[1..]
                .iter()
                .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'-'))
    };
    match name {
        Value::String(s) if slug(s) => {
            if s == ORPHAN {
                Err(refuse(400, "the name 'orphan' is reserved"))
            } else {
                Ok(s.clone())
            }
        }
        _ => Err(refuse(400, NAME_RULE)),
    }
}

fn check_text(field: &str, max: usize, value: &Value) -> Done<Option<String>> {
    match value {
        Value::Null => Ok(None),
        Value::String(s) if s.chars().count() <= max => {
            let color =
                s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| b.is_ascii_hexdigit());
            if field == "color" && !color {
                return Err(refuse(400, "color must look like #rrggbb"));
            }
            Ok(Some(s.clone()))
        }
        _ => Err(refuse(
            400,
            format!("{field} must be text of at most {max} characters"),
        )),
    }
}

fn check_repos(repos: Repos) -> Done<Vec<String>> {
    let mut clean = BTreeSet::new();
    for repo in repos? {
        clean.insert(repo?);
    }
    Ok(clean.into_iter().collect())
}

/// `get`: one workspace as the list shows it, or 404 `unknown workspace`.
pub fn get(conn: &Connection, id: i64) -> Done<Value> {
    workspaces(conn)?
        .into_iter()
        .find(|w| w["id"] == id)
        .ok_or_else(|| refuse(404, "unknown workspace"))
}

fn insert_repos(conn: &Connection, id: i64, repos: &[String]) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("INSERT INTO workspace_repos VALUES (?1, ?2)")?;
    repos
        .iter()
        .try_for_each(|r| stmt.execute(params![id, r]).map(drop))
}

/// `add`: create a workspace from the body's `name`, `repos` and text fields. The checks run in Python's order (the
/// text fields, then the repos, then the name) before the store is touched; a taken name is 409.
pub fn add(conn: &Connection, body: &Map<String, Value>, repos: Repos) -> Done<Value> {
    if body.keys().any(|k| !FIELDS.contains(&k.as_str())) {
        return Err(refuse(400, "unknown field"));
    }
    let null = Value::Null;
    let mut text = Vec::new();
    for (field, max) in TEXT_FIELDS {
        text.push(check_text(field, max, body.get(field).unwrap_or(&null))?);
    }
    let repos = check_repos(repos)?;
    let name = check_name(body.get("name").unwrap_or(&null))?;
    let tx = begin(conn)?;
    // the id is the workspace row's, read before the repo rows are inserted (Python's `cur.lastrowid`)
    let stored = tx
        .execute(
            "INSERT INTO workspaces (name, color, icon, description, email) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![name, text[0], text[1], text[2], text[3]],
        )
        .and_then(|_| {
            let id = tx.last_insert_rowid();
            insert_repos(&tx, id, &repos).map(|()| id)
        });
    let id = match stored {
        Ok(id) => id,
        Err(e) if is_constraint(&e) => {
            return Err(refuse(409, "a workspace with this name exists"));
        }
        Err(e) => return Err(e.into()),
    };
    tx.commit()?;
    get(conn, id)
}

/// `edit`: [`edit_in_txn`] in a transaction of its own.
pub fn edit(
    conn: &Connection,
    id: i64,
    changes: &[(String, Value)],
    repos: Option<Repos>,
) -> Done<Value> {
    let tx = begin(conn)?;
    let workspace = edit_in_txn(&tx, id, changes, repos)?;
    tx.commit()?;
    Ok(workspace)
}

/// `edit_in_txn`: any of `name`, `color`, `icon`, `description`, `email`, `repos`, given in the body's order (the first
/// bad text field is the one named). `repos` replaces the list; a rename keeps the id. The caller owns the transaction.
/// `repos` is [`prepare_repos`] of the `repos` change, `Some` exactly when there is one.
pub(crate) fn edit_in_txn(
    conn: &Connection,
    id: i64,
    changes: &[(String, Value)],
    repos: Option<Repos>,
) -> Done<Value> {
    if get(conn, id)?["is_orphan"] == true {
        return Err(refuse(403, "the orphan workspace cannot be changed"));
    }
    if changes.iter().any(|(k, _)| !FIELDS.contains(&k.as_str())) {
        return Err(refuse(400, "unknown field"));
    }
    let mut columns: Vec<(&str, Option<String>)> = Vec::new();
    for (field, value) in changes {
        if let Some((name, max)) = TEXT_FIELDS.iter().find(|(f, _)| f == field) {
            columns.push((name, check_text(name, *max, value)?));
        }
    }
    if let Some((_, value)) = changes.iter().find(|(k, _)| k == "name") {
        columns.push(("name", Some(check_name(value)?)));
    }
    let repos = repos.map(check_repos).transpose()?;
    let written = (|| {
        if !columns.is_empty() {
            // the column names are the fixed ones above; the values are bound
            let sets: Vec<String> = columns
                .iter()
                .enumerate()
                .map(|(i, (c, _))| format!("{c} = ?{}", i + 1))
                .collect();
            let sql = format!(
                "UPDATE workspaces SET {} WHERE id = ?{}",
                sets.join(", "),
                columns.len() + 1
            );
            let mut values: Vec<rusqlite::types::Value> =
                columns.iter().map(|(_, v)| v.clone().into()).collect();
            values.push(id.into());
            conn.execute(&sql, params_from_iter(values))?;
        }
        if let Some(repos) = &repos {
            conn.execute("DELETE FROM workspace_repos WHERE workspace_id = ?1", [id])?;
            insert_repos(conn, id, repos)?;
        }
        Ok(())
    })();
    match written {
        Ok(()) => get(conn, id),
        Err(e) if is_constraint(&e) => Err(refuse(409, "a workspace with this name exists")),
        Err(e) => Err(e.into()),
    }
}

/// `delete_to_orphan`: the workspace's connectors go to orphan (id and alias kept), its picks and repo rows go, then
/// it goes (FR-030). The caller owns the transaction. Returns the moved connector ids.
fn delete_to_orphan(conn: &Connection, id: i64) -> Done<Vec<String>> {
    if get(conn, id)?["is_orphan"] == true {
        return Err(refuse(403, "the orphan workspace cannot be deleted"));
    }
    let orphan: i64 = conn.query_row("SELECT id FROM workspaces WHERE is_orphan = 1", [], |r| {
        r.get(0)
    })?;
    let moved = conn
        .prepare("SELECT id FROM connectors WHERE workspace_id = ?1 ORDER BY id")?
        .query_map([id], |r| r.get(0))?
        .collect::<std::result::Result<Vec<String>, _>>()?;
    conn.execute(
        "UPDATE connectors SET workspace_id = ?1 WHERE workspace_id = ?2",
        [orphan, id],
    )?;
    conn.execute(
        "DELETE FROM account_workspaces WHERE workspace_id = ?1",
        [id],
    )?;
    conn.execute("DELETE FROM workspaces WHERE id = ?1", [id])?; // workspace_repos cascade
    Ok(moved)
}

/// `_change`: one owner action. `mutate` changes the desired state inside the transaction and returns the connector
/// ids it touched and the lines that describe it; the plan for the accounts that hold or pick those connectors is
/// added. Without `yes` the plan is the answer and everything is rolled back. With `yes` the change is committed, but
/// only if no step needs the `claude` CLI (see [`WriteFail::NeedsClaude`]); then Python's `apply` has nothing to run
/// and says so.
fn change(
    conn: &Connection,
    yes: bool,
    mutate: impl FnOnce(&Connection) -> Done<(BTreeSet<String>, Vec<String>)>,
) -> Done<Outcome> {
    let tx = begin(conn)?;
    let planned = (|| {
        let (cids, mut lines) = mutate(&tx)?;
        let marks = vec!["?"; cids.len()].join(",");
        let sql = format!(
            "SELECT account_id FROM account_registrations WHERE connector_id IN ({marks}) UNION \
             SELECT p.account_id FROM account_workspaces p JOIN connectors c ON c.workspace_id = p.workspace_id \
             WHERE c.id IN ({marks})"
        );
        let ids = tx
            .prepare(&sql)?
            .query_map(params_from_iter(cids.iter().chain(&cids)), |r| {
                r.get::<_, i64>(0)
            })?
            .collect::<std::result::Result<BTreeSet<_>, _>>()?;
        let mut steps = Vec::new();
        for id in ids {
            steps.extend(
                sync::plan(&tx, id)?
                    .into_iter()
                    .filter(|s| cids.contains(s.connector_id()) || s.is_skipped()),
            );
        }
        lines.extend(steps.iter().map(|s| format!("  {}", s.line())));
        lines.push("  keychain: untouched".into());
        Ok::<_, WriteFail>((lines, steps))
    })();
    let (mut lines, steps) = planned?;
    if !yes {
        return Ok(Outcome {
            status: 1,
            lines,
            pending: true,
        }); // the transaction is dropped: rolled back
    }
    if steps.iter().any(sync::Step::needs_claude) {
        return Err(WriteFail::NeedsClaude);
    }
    tx.commit()?;
    lines.push("nothing to change.".into());
    if steps.iter().any(sync::Step::is_skipped) {
        lines.push(
            "an account could not be changed now; the store change is kept, `connector sync` finishes it.".into(),
        );
    }
    Ok(Outcome {
        status: 0,
        lines,
        pending: false,
    })
}

/// `_alias_ok`: the launcher-name rule, then (outside orphan) one alias per connector.
fn alias_ok(conn: &Connection, alias: String, connector_id: &str) -> Done<String> {
    let bytes = alias.as_bytes();
    let name = (1..=64).contains(&bytes.len())
        && matches!(bytes[0], b'a'..=b'z' | b'0'..=b'9')
        && bytes[1..]
            .iter()
            .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-'));
    if !name {
        return Err(refuse(
            400,
            format!(
                "alias for connector {connector_id} is too long or has invalid characters; it must match [a-z0-9][a-z0-9_-]{{0,63}}"
            ),
        ));
    }
    let other: Option<String> = conn
        .query_row(
            "SELECT c.id FROM connectors c JOIN workspaces w ON w.id = c.workspace_id \
             WHERE c.alias = ?1 AND w.is_orphan = 0 AND c.id <> ?2",
            params![alias, connector_id],
            |r| r.get(0),
        )
        .optional()?;
    match other {
        Some(other) => Err(refuse(
            400,
            format!("alias '{alias}' is in use by connector {other}; nothing was changed"),
        )),
        None => Ok(alias),
    }
}

/// `PATCH /api/workspaces/{id}`: the body's changes without `confirm`, and `confirm` as `yes`. A body whose `name` is
/// the current one has no rename. Without a rename, or when the workspace holds no connector, the edit is applied at
/// once; a rename of one that does is the plan first (`rename_workspace`: the default aliases follow the new name).
pub fn patch(
    conn: &Connection,
    id: i64,
    mut changes: Vec<(String, Value)>,
    repos: Option<Repos>,
    yes: bool,
) -> Done<Outcome> {
    let current = get(conn, id)?;
    changes.retain(|(k, v)| !(k == "name" && *v == current["name"]));
    let renamed = changes.iter().any(|(k, _)| k == "name");
    let held = conn
        .prepare("SELECT id FROM connectors WHERE workspace_id = ?1 ORDER BY id")?
        .query_map([id], |r| r.get(0))?
        .collect::<std::result::Result<Vec<String>, _>>()?;
    if !renamed || held.is_empty() {
        edit(conn, id, &changes, repos)?;
        return Ok(Outcome {
            status: 0,
            lines: Vec::new(),
            pending: false,
        });
    }
    change(conn, yes, |conn| {
        let name = edit_in_txn(conn, id, &changes, repos)?["name"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let mut lines = vec![format!("will change (workspace {id}, now '{name}'):")];
        let rows = conn
            .prepare(
                "SELECT id, alias, provider FROM connectors WHERE workspace_id = ?1 AND alias_custom = 0 ORDER BY id",
            )?
            .query_map([id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for (cid, old, provider) in rows {
            let new = alias_ok(conn, format!("{name}-{provider}"), &cid)?;
            if new != old {
                conn.execute(
                    "UPDATE connectors SET alias = ?1 WHERE id = ?2",
                    params![new, cid],
                )?;
                lines.push(format!("  connector {cid}: alias {old} -> {new}"));
            }
        }
        Ok((held.into_iter().collect(), lines))
    })
}

/// `DELETE /api/workspaces/{id}`: the plan first; with `yes` the workspace goes and its connectors move to orphan.
pub fn delete(conn: &Connection, id: i64, yes: bool) -> Done<Outcome> {
    change(conn, yes, |conn| {
        let workspace = get(conn, id)?;
        let picks: i64 = conn.query_row(
            "SELECT COUNT(*) FROM account_workspaces WHERE workspace_id = ?1",
            [id],
            |r| r.get(0),
        )?;
        let moved = delete_to_orphan(conn, id)?;
        let repos = workspace["repos"].as_array().map_or(0, Vec::len);
        let to_orphan = if moved.is_empty() {
            "none".to_string()
        } else {
            moved.join(", ")
        };
        let lines = vec![
            format!(
                "will change (delete workspace '{}'):",
                workspace["name"].as_str().unwrap_or_default()
            ),
            format!("  connectors to orphan: {to_orphan}"),
            format!("  account picks removed: {picks}; repo rows deleted: {repos}"),
        ];
        Ok((moved.into_iter().collect(), lines))
    })
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
