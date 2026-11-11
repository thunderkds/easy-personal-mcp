//! The agent accounts list, read part of `src/easy_personal/core/accounts.py` (`_row`, `_with_picks`, `list_accounts`,
//! `group_by_provider`, `status_lines`, `grouped`; HUN-62, HUN-115), served as `GET /api/accounts` (T041).
//!
//! Read-only, and **no account detection**: Python's `grouped` runs `detect(conn)` first, which INSERTs the `.claude*` /
//! `.codex*` folders it finds. Here that stays in `ingest` (`ingest::accounts::detect`): a new account folder shows after
//! the next ingest, not at page load (decision Q3). Nothing is written; the connection may be read-only.
//!
//! Each account is its stored row, a `status` (`missing` when the folder is gone and the account is not ignored) and its
//! `status_line`. The status line reads the store (tokens since local midnight and in the last 7 x 24 h, one row per
//! message; the newest model) and, for a Claude account, exactly two small files in its folder, as Python does:
//! three keys of `<folder>/.claude.json` (`oauthAccount`: email, plan, organization; for the default account also the file
//! beside the folder) and `<folder>/easy-personal-limits.json` (the status-line snapshot). Nothing else in a folder is
//! opened or listed; no key, token or other field of those files is read out. All SQL has bound values only.

use crate::metrics::{DAY_MICROS, MetricsError, format_stored, is_dir};
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
    let default_folder = resolve(&agent_root.join(".claude"))
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

/// Python's `Path.resolve()` (not strict): the longest existing part is resolved, the rest is appended.
fn resolve(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let (mut head, mut tail) = (absolute.clone(), Vec::new());
    loop {
        if let Ok(mut done) = fs::canonicalize(&head) {
            done.extend(tail.iter().rev());
            return done;
        }
        match (head.parent(), head.file_name()) {
            (Some(parent), Some(name)) => {
                tail.push(name.to_os_string());
                head = parent.to_path_buf();
            }
            _ => return absolute,
        }
    }
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
fn read_json(path: &Path, max_bytes: u64) -> Option<Value> {
    let meta = fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > max_bytes {
        return None;
    }
    serde_json::from_slice(&fs::read(path).ok()?)
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
