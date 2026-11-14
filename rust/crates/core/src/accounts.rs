//! The agent accounts list, read part of `src/easy_personal/core/accounts.py` (`_row`, `_with_picks`, `list_accounts`,
//! `group_by_provider`, `status_lines`, `grouped`; HUN-62, HUN-115), served as `GET /api/accounts` (T041).
//!
//! Read-only here (the writes are at the end of the file, T048), and **no account detection**: Python's `grouped` runs `detect(conn)` first, which INSERTs the `.claude*` /
//! `.codex*` folders it finds. Here that stays in `ingest` (`ingest::accounts::detect`): a new account folder shows after
//! the next ingest, not at page load (decision Q3). Nothing is written; the connection may be read-only.
//!
//! Each account is its stored row, a `status` (`missing` when the folder is gone and the account is not ignored) and its
//! `status_line`. The status line reads the store (tokens since local midnight and in the last 7 x 24 h, one row per
//! message; the newest model) and, for a Claude account, exactly two small files in its folder, as Python does:
//! three keys of `<folder>/.claude.json` (`oauthAccount`: email, plan, organization; for the default account also the file
//! beside the folder) and `<folder>/easy-personal-limits.json` (the status-line snapshot). Nothing else in a folder is
//! opened or listed; no key, token or other field of those files is read out. All SQL has bound values only.

use crate::metrics::{DAY_MICROS, MetricsError, format_stored, is_dir, resolve_path};
use crate::workspaces::{Done, WriteFail, begin, expanduser, is_constraint, py_strip, refuse};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, MetricsError>;

const MAX_CLAUDE_JSON: u64 = 32 * 1024 * 1024;
const MAX_LIMITS_JSON: u64 = 64 * 1024;
const LIMITS_FILE: &str = "easy-personal-limits.json";
/// Keeps every event of a message that starts inside the window (`_MESSAGE_PAD`).
const MESSAGE_PAD: i64 = DAY_MICROS;
const BY_ERROR: &str = "by must be account or provider";

struct Account {
    id: i64,
    provider: String,
    folder: String,
    label: Option<String>,
    status: String,
    env: Option<String>,
}

/// `{"by": "account", "accounts": [...]}` or `{"by": "provider", "providers": {provider: [...]}}`, with the zone read
/// from `TZ` / the OS (Python's `datetime.astimezone()`). `now` is microseconds since the epoch (UTC); `agent_root` is
/// where Python's `config.agent_root()` points (it only decides `is_default`).
pub fn grouped(conn: &Connection, by: &str, now: i64, agent_root: &Path) -> Result<Value> {
    grouped_in(conn, by, now, agent_root, &TimeZone::system())
}

/// [`grouped`] with the zone given, so a test needs no environment variable.
pub fn grouped_in(
    conn: &Connection,
    by: &str,
    now: i64,
    agent_root: &Path,
    zone: &TimeZone,
) -> Result<Value> {
    if by != "account" && by != "provider" {
        return Err(MetricsError::Invalid(BY_ERROR.to_string()));
    }
    let default_folder = resolve_path(&agent_root.join(".claude"))
        .to_string_lossy()
        .into_owned();
    let accounts = list(conn)?;
    let default = default_id(conn)?;
    let sums = token_sums(conn, now, zone)?;
    let mut providers: Map<String, Value> = Map::new();
    let mut flat = Vec::with_capacity(accounts.len());
    for account in &accounts {
        let status = if account.status != "ignored" && !is_dir(&account.folder) {
            "missing"
        } else {
            account.status.as_str()
        };
        let row = json!({
            "id": account.id, "provider": account.provider, "label": account.label, "folder": account.folder,
            "status": status, "config_dir_env": account.env,
            "is_default": account.folder == default_folder,
            "workspaces": picks(conn, account.id)?,
            "status_line": status_line(conn, account, default, &sums, now)?,
        });
        if by == "provider" {
            if let Value::Array(rows) = providers
                .entry(account.provider.clone())
                .or_insert_with(|| json!([]))
            {
                rows.push(row);
            }
        } else {
            flat.push(row);
        }
    }
    Ok(if by == "provider" {
        json!({ "by": by, "providers": providers })
    } else {
        json!({ "by": by, "accounts": flat })
    })
}

fn list(conn: &Connection) -> Result<Vec<Account>> {
    let mut stmt = conn.prepare(
        "SELECT id, provider, folder, label, status, config_dir_env FROM agent_accounts ORDER BY id",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Account {
            id: r.get(0)?,
            provider: r.get(1)?,
            folder: r.get(2)?,
            label: r.get(3)?,
            status: r.get(4)?,
            env: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// Workspace names the account picked, by name (`_with_picks`).
fn picks(conn: &Connection, id: i64) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT w.name FROM account_workspaces p JOIN workspaces w ON w.id = p.workspace_id \
         WHERE p.account_id = ?1 ORDER BY w.name",
    )?;
    let names = stmt.query_map([id], |r| r.get(0))?;
    Ok(names.collect::<std::result::Result<_, _>>()?)
}

/// `default_id`: the first Claude account with no `CLAUDE_CONFIG_DIR`, by id.
fn default_id(conn: &Connection) -> Result<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT id FROM agent_accounts WHERE provider = 'claude' AND config_dir_env IS NULL ORDER BY id",
            [],
            |r| r.get(0),
        )
        .optional()?)
}

/// `{account id or None: (tokens since local midnight, tokens in the last 7 x 24 h)}`, one row per message (the largest
/// figure of each token column over its events), like the usage metrics. A NULL account is the default account's.
fn token_sums(
    conn: &Connection,
    now: i64,
    zone: &TimeZone,
) -> Result<HashMap<Option<i64>, (i64, i64)>> {
    let week = now - 7 * DAY_MICROS;
    let mut stmt = conn.prepare(
        "SELECT account, SUM(CASE WHEN ts >= ?1 THEN t ELSE 0 END), SUM(t) FROM (\
           SELECT agent_account_id AS account, MIN(ts_utc) AS ts, \
                  MAX(input_tokens) + MAX(output_tokens) + MAX(cache_creation_input_tokens) \
                  + MAX(cache_read_input_tokens) AS t \
           FROM events WHERE abstained = 0 AND message_id IS NOT NULL AND input_tokens IS NOT NULL \
           AND ts_utc >= ?2 GROUP BY agent_account_id, message_id\
         ) WHERE ts >= ?3 GROUP BY account",
    )?;
    let rows = stmt.query_map(
        params![
            format_stored(local_midnight(now, zone)),
            format_stored(week - MESSAGE_PAD),
            format_stored(week)
        ],
        |r| {
            Ok((
                r.get::<_, Option<i64>>(0)?,
                (
                    r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                ),
            ))
        },
    )?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// Local midnight of `now`'s day in `zone`, as UTC microseconds. Python: `datetime.combine(local.date(), time.min)
/// .astimezone()`, `fold=0`: a skipped local time reads with the offset before the change (`jiff`'s `compatible`).
fn local_midnight(now: i64, zone: &TimeZone) -> i64 {
    let utc_midnight = now - now.rem_euclid(DAY_MICROS);
    let Ok(stamp) = Timestamp::from_microsecond(now) else {
        return utc_midnight;
    };
    let midnight = zone.to_datetime(stamp).date().at(0, 0, 0, 0);
    zone.to_ambiguous_zoned(midnight)
        .compatible()
        .map_or(utc_midnight, |z| z.timestamp().as_microsecond())
}

/// `(ts_utc, model)` of the account's newest event that carries a model; `None` asks for the events with no account.
fn newest(conn: &Connection, account: Option<i64>) -> Result<Option<(String, String)>> {
    Ok(conn
        .query_row(
            "SELECT ts_utc, model FROM events WHERE agent_account_id IS ?1 AND abstained = 0 AND model IS NOT NULL \
             ORDER BY ts_utc DESC LIMIT 1",
            [account],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?)
}

fn status_line(
    conn: &Connection,
    account: &Account,
    default: Option<i64>,
    sums: &HashMap<Option<i64>, (i64, i64)>,
    now: i64,
) -> Result<Value> {
    let mut own = vec![Some(account.id)];
    if Some(account.id) == default {
        own.push(None); // events with no account: the default account's
    }
    let (mut today, mut week, mut latest) = (0, 0, None::<(String, String)>);
    for id in own {
        let (day, total) = sums.get(&id).copied().unwrap_or((0, 0));
        today += day;
        week += total;
        latest = latest.into_iter().chain(newest(conn, id)?).max();
    }
    let (email, plan, organization) = identity(account);
    Ok(json!({
        "email": email, "plan": plan, "organization": organization,
        "last_model": latest.as_ref().map(|l| &l.1), "last_active_utc": latest.as_ref().map(|l| &l.0),
        "tokens_today": today, "tokens_7d": week,
        "limits": limits(account, now),
    }))
}

/// Parsed JSON of a small regular file, or `None` when it is missing, not a file, too big, unreadable or not JSON
/// (a literal `null` too, as Python's `_read_json` + `is None`). A folder or a pipe is never read.
///
/// The file is opened once (`O_NONBLOCK`, so a pipe never blocks the open), judged on that handle (a regular file) and read
/// through `take(max + 1)`: more than `max` bytes is "too big" however the size changed since the open.
fn read_json(path: &Path, max_bytes: u64) -> Option<Value> {
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    /// The Linux value on x86, x86-64, arm, aarch64 and riscv64 (as in `report_files.rs`).
    const O_NONBLOCK: i32 = 0o4000;
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NONBLOCK)
        .open(path)
        .ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut data = Vec::new();
    file.take(max_bytes + 1).read_to_end(&mut data).ok()?;
    if data.len() as u64 > max_bytes {
        return None;
    }
    serde_json::from_slice(&data)
        .ok()
        .filter(|v: &Value| !v.is_null())
}

fn text(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(str::to_string)
}

/// Email, plan and organization from `<folder>/.claude.json`: only `oauthAccount`'s three keys are read. The default
/// account's file lives beside its folder when `CLAUDE_CONFIG_DIR` is unset.
fn identity(account: &Account) -> (Option<String>, Option<String>, Option<String>) {
    if account.provider != "claude" {
        return (None, None, None);
    }
    let folder = Path::new(&account.folder);
    let mut data = read_json(&folder.join(".claude.json"), MAX_CLAUDE_JSON);
    if data.is_none() && account.env.is_none() {
        let parent = folder.parent().unwrap_or(folder);
        data = read_json(&parent.join(".claude.json"), MAX_CLAUDE_JSON);
    }
    match data.as_ref().and_then(|d| d.get("oauthAccount")) {
        Some(oauth @ Value::Object(_)) => (
            text(oauth.get("emailAddress")),
            text(oauth.get("billingType")),
            text(oauth.get("organizationName")),
        ),
        _ => (None, None, None),
    }
}

/// A JSON number (not a bool: `serde_json` keeps those apart), as it is stored and as a float.
fn number(value: Option<&Value>) -> Option<(&Value, f64)> {
    let value = value?;
    Some((value, value.as_number()?.as_f64()?))
}

/// `datetime.fromtimestamp(epoch, UTC).strftime("%Y-%m-%dT%H:%M:%SZ")`: the epoch rounded to the microsecond
/// (half to even), seconds only; `None` outside the years 1 to 9999.
fn iso(epoch: Option<&Value>) -> Option<String> {
    let (_, epoch) = number(epoch)?;
    let mut secs = epoch.trunc();
    let micros = ((epoch - secs) * 1e6).round_ties_even();
    if micros >= 1e6 {
        secs += 1.0;
    } else if micros < 0.0 {
        secs -= 1.0;
    }
    if !(-62_135_596_800.0..=253_402_300_799.0).contains(&secs) {
        return None;
    }
    let stored = format_stored(secs as i64 * 1_000_000);
    Some(format!("{}Z", stored.get(..19)?))
}

/// The owner's status-line snapshot: `five_hour` / `seven_day` `{used_percentage, resets_at, stale}` + `written_at`.
fn limits(account: &Account, now: i64) -> Value {
    if account.provider != "claude" {
        return Value::Null;
    }
    let Some(Value::Object(data)) = read_json(
        &Path::new(&account.folder).join(LIMITS_FILE),
        MAX_LIMITS_JSON,
    ) else {
        return Value::Null;
    };
    let now_seconds = now as f64 / 1e6;
    let window = |raw: Option<&Value>| -> Option<(Value, bool)> {
        let raw = raw?.as_object()?;
        let (percent, _) = number(raw.get("used_percentage"))?;
        let resets = number(raw.get("resets_at")).map(|(_, f)| f);
        let stale = resets.is_some_and(|r| r <= now_seconds);
        Some((
            json!({ "used_percentage": percent, "resets_at": iso(raw.get("resets_at")), "stale": stale }),
            stale,
        ))
    };
    let (five, seven) = (window(data.get("five_hour")), window(data.get("seven_day")));
    if five.is_none() && seven.is_none() {
        return Value::Null;
    }
    let stale = [&five, &seven].iter().any(|w| matches!(w, Some((_, true))));
    json!({
        "five_hour": five.map(|w| w.0), "seven_day": seven.map(|w| w.0),
        "written_at": iso(data.get("written_at")), "stale": stale,
    })
}

// ---- writes (T048) ----
//
// `add`, `confirm`, `ignore` and `use` of `accounts.py` and the `_get` / `_find` they share, store only. Each answers
// the account row as `_get` does (no status line). A refusal is a [`WriteFail::Refused`] with Python's status (400, or
// 404 `no such account`) and its fixed text, which never names the caller's input. The account detection Python runs
// before each of these is `ingest::accounts::detect`, called by the route.

const MAX_LABEL: usize = 60;

/// The agent root's `.claude` folder, resolved: the default account's (`accounts.py`: `agent_root() / ".claude"`).
fn default_folder(agent_root: &Path) -> String {
    resolve_path(Path::new(
        &agent_root.join(".claude").to_string_lossy().into_owned(),
    ))
    .to_string_lossy()
    .into_owned()
}

/// `_get`: the stored row with its `status` (`missing` when the folder is gone and the account is not ignored),
/// `is_default` and the picked workspace names.
fn get(conn: &Connection, id: i64, agent_root: &Path) -> Done<Value> {
    let account = conn.query_row(
        "SELECT id, provider, folder, label, status, config_dir_env FROM agent_accounts WHERE id = ?1",
        [id],
        |r| {
            Ok(Account {
                id: r.get(0)?,
                provider: r.get(1)?,
                folder: r.get(2)?,
                label: r.get(3)?,
                status: r.get(4)?,
                env: r.get(5)?,
            })
        },
    )?;
    let status = if account.status != "ignored" && !is_dir(&account.folder) {
        "missing"
    } else {
        account.status.as_str()
    };
    let names = picks(conn, id).map_err(WriteFail::from)?;
    Ok(json!({
        "id": account.id, "provider": account.provider, "label": account.label, "folder": account.folder,
        "status": status, "config_dir_env": account.env,
        "is_default": account.folder == default_folder(agent_root),
        "workspaces": names,
    }))
}

/// `_find`: the account id for an id (plain digits) or a folder path; 404 `no such account`.
pub fn find(conn: &Connection, reference: &str) -> Done<i64> {
    let reference = py_strip(reference);
    let found: Option<i64> =
        if !reference.is_empty() && reference.bytes().all(|b| b.is_ascii_digit()) {
            match reference.parse::<i64>() {
                Ok(id) => conn
                    .query_row("SELECT id FROM agent_accounts WHERE id = ?1", [id], |r| {
                        r.get(0)
                    })
                    .optional()?,
                Err(_) => None, // past 64 bits: no account has it (Python's sqlite3 raises OverflowError: a 500)
            }
        } else {
            find_folder(conn, &resolve_path(Path::new(&expanduser(reference))))?
        };
    found.ok_or_else(|| refuse(404, "no such account"))
}

/// The account whose stored folder is this (already resolved) path.
fn find_folder(conn: &Connection, folder: &Path) -> rusqlite::Result<Option<i64>> {
    conn.query_row(
        "SELECT id FROM agent_accounts WHERE folder = ?1",
        [folder.to_string_lossy()],
        |r| r.get(0),
    )
    .optional()
}

/// `confirm`: name an account and mark it confirmed; confirming again with a new label renames it. The label is
/// stripped and must be 1 to 60 characters; that is checked before the account is looked up.
pub fn confirm(
    conn: &Connection,
    reference: &str,
    label: &Value,
    agent_root: &Path,
) -> Done<Value> {
    let label = match label {
        Value::String(text) => py_strip(text),
        _ => "",
    };
    let length = label.chars().count();
    if !(1..=MAX_LABEL).contains(&length) {
        return Err(refuse(
            400,
            format!("label must be 1 to {MAX_LABEL} characters"),
        ));
    }
    let id = find(conn, reference)?;
    let tx = begin(conn)?;
    match tx.execute(
        "UPDATE agent_accounts SET status = 'confirmed', label = ?1 WHERE id = ?2",
        params![label, id],
    ) {
        Ok(_) => {}
        // the table's own length rule (it stops at a NUL, Python's `len` does not): Python raises, this refuses
        Err(e) if is_constraint(&e) => {
            return Err(refuse(
                400,
                format!("label must be 1 to {MAX_LABEL} characters"),
            ));
        }
        Err(e) => return Err(e.into()),
    }
    tx.commit()?;
    get(conn, id, agent_root)
}

/// `ignore`: the default account (`<agent root>/.claude`) cannot be ignored.
pub fn ignore(conn: &Connection, reference: &str, agent_root: &Path) -> Done<Value> {
    let id = find(conn, reference)?;
    let folder: String = conn.query_row(
        "SELECT folder FROM agent_accounts WHERE id = ?1",
        [id],
        |r| r.get(0),
    )?;
    if folder == default_folder(agent_root) {
        return Err(refuse(400, "the default account cannot be ignored"));
    }
    let tx = begin(conn)?;
    tx.execute(
        "UPDATE agent_accounts SET status = 'ignored' WHERE id = ?1",
        [id],
    )?;
    tx.commit()?;
    get(conn, id, agent_root)
}

/// A folder for [`add`] after its filesystem work: the folder (resolved, UTF-8, an existing directory) and the path
/// `_find` resolves for it afterwards.
pub struct Folder {
    name: String,
    key: PathBuf,
}

/// The first half of `add`: the checks that need no store, and every resolution of a path. A route does this before it
/// takes the write lock (a hostile link chain must not hold it) and passes the result to [`add`].
pub fn prepare_add(path: &Value) -> Done<Folder> {
    let not_a_folder = || refuse(400, "not an agent folder: it must be an existing directory");
    let Value::String(text) = path else {
        return Err(not_a_folder());
    };
    if py_strip(text).is_empty() || text.contains('\0') {
        return Err(not_a_folder());
    }
    let folder = resolve_path(Path::new(&expanduser(text)));
    let Some(name) = folder.to_str().filter(|_| folder.is_dir()) else {
        return Err(not_a_folder());
    };
    // Python looks the new row up with `_find(str(folder))`, which strips the text and resolves it again
    let key = resolve_path(Path::new(&expanduser(py_strip(name))));
    Ok(Folder {
        name: name.to_string(),
        key,
    })
}

/// `add`: a folder by hand, anywhere; it must exist and pass the content check. A new one starts `detected`
/// (`confirmed` as "Claude default" for the agent root's own `.claude`); one already stored is returned as it is.
pub fn add(conn: &Connection, folder: Done<Folder>, agent_root: &Path) -> Done<Value> {
    let Folder { name, key } = folder?;
    let Some(provider) = crate::ingest::accounts::provider_of(Path::new(&name)) else {
        return Err(refuse(
            400,
            "not an agent folder: no projects/, settings.json or config.toml",
        ));
    };
    let tx = begin(conn)?;
    crate::ingest::accounts::insert(&tx, provider, &name, name == default_folder(agent_root))?;
    tx.commit()?;
    let id = find_folder(conn, &key)?.ok_or_else(|| refuse(404, "no such account"))?;
    get(conn, id, agent_root)
}

/// `use`: the workspaces a confirmed Claude account picks, by name (replaces the list; empty clears it). Changes only
/// the store: syncing the account's Claude config is T050.
pub fn use_workspaces(
    conn: &Connection,
    reference: &str,
    names: &Value,
    agent_root: &Path,
) -> Done<Value> {
    let id = find(conn, reference)?;
    let (provider, status): (String, String) = conn.query_row(
        "SELECT provider, status FROM agent_accounts WHERE id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if (provider.as_str(), status.as_str()) != ("claude", "confirmed") {
        return Err(refuse(
            400,
            "only confirmed Claude accounts pick workspaces",
        ));
    }
    let names: Vec<&str> = match names {
        Value::Array(list) => list.iter().map(Value::as_str).collect::<Option<_>>(),
        _ => None,
    }
    .ok_or_else(|| refuse(400, "workspaces must be a list of workspace names"))?;
    let mut ids = std::collections::BTreeSet::new();
    for name in names {
        let row: Option<(i64, i64)> = conn
            .query_row(
                "SELECT id, is_orphan FROM workspaces WHERE name = ?1",
                [name],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        match row {
            None => return Err(refuse(400, "unknown workspace")),
            Some((_, orphan)) if orphan != 0 => {
                return Err(refuse(400, "the orphan workspace cannot be picked"));
            }
            Some((workspace, _)) => {
                ids.insert(workspace);
            }
        }
    }
    let tx = begin(conn)?;
    tx.execute("DELETE FROM account_workspaces WHERE account_id = ?1", [id])?;
    for workspace in ids {
        tx.execute(
            "INSERT INTO account_workspaces VALUES (?1, ?2)",
            [id, workspace],
        )?;
    }
    tx.commit()?;
    get(conn, id, agent_root)
}

#[cfg(test)]
mod read_json_tests {
    use super::read_json;
    use std::io::Write;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t052-{tag}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The size is judged on what is read from the open file, not on a size looked up earlier: a file that says it is empty
    /// (every `/proc` file) and prints more than the cap is refused. `pid_max` is a number (valid JSON) of 7 or 8 digits.
    #[test]
    fn the_cap_is_applied_to_the_bytes_read_not_to_a_size_asked_for_earlier() {
        let proc = std::path::Path::new("/proc/sys/kernel/pid_max");
        if !proc.is_file() {
            return; // not Linux
        }
        assert_eq!(std::fs::metadata(proc).unwrap().len(), 0, "procfs says 0");
        assert!(
            read_json(proc, 1).is_none(),
            "more than 1 byte read against a cap of 1"
        );
        assert!(read_json(proc, 64).is_some_and(|v| v.is_number()));
    }

    /// A file of exactly the cap reads, one byte more is refused, a folder is not read, a literal `null` is no file.
    #[test]
    fn the_cap_is_exact_and_only_a_regular_file_is_read() {
        let dir = scratch("cap");
        let file = dir.join("a.json");
        let mut f = std::fs::File::create(&file).unwrap();
        f.write_all(br#"{"a":1}"#).unwrap();
        drop(f);
        assert_eq!(read_json(&file, 7).unwrap()["a"], 1);
        assert!(read_json(&file, 6).is_none());
        assert!(read_json(&dir, 1 << 20).is_none(), "a folder");
        assert!(read_json(&dir.join("missing.json"), 1 << 20).is_none());
        std::fs::write(&file, "null").unwrap();
        assert!(read_json(&file, 1 << 20).is_none(), "null is no file");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
