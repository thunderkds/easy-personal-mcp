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

/// One Claude account's plan-limit percents, for the Watch sampler (T073): the 5 h and 7 d `used_percentage` from the
/// status-line snapshot through [`limits`] (no second parser). A window that is missing, broken or stale (its reset time
/// passed) is `None`.
#[derive(Clone, Debug)]
pub struct LimitPercents {
    pub id: i64,
    pub label: String,
    pub five_h_pct: Option<f64>,
    pub seven_d_pct: Option<f64>,
}

/// [`LimitPercents`] of every stored Claude account that is not ignored, by id. `now` is microseconds since the epoch.
pub fn limit_percents(conn: &Connection, now: i64) -> Result<Vec<LimitPercents>> {
    let percent = |limits: &Value, window: &str| -> Option<f64> {
        let w = limits.get(window)?;
        if w.get("stale")?.as_bool()? {
            return None;
        }
        w.get("used_percentage")?.as_f64()
    };
    Ok(list(conn)?
        .iter()
        .filter(|a| a.provider == "claude" && a.status != "ignored")
        .map(|a| {
            let l = limits(a, now);
            LimitPercents {
                id: a.id,
                label: a.label.clone().unwrap_or_else(|| a.provider.clone()),
                five_h_pct: percent(&l, "five_hour"),
                seven_d_pct: percent(&l, "seven_day"),
            }
        })
        .collect())
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
/// `_find` resolves for it afterwards. `made` is set only for a folder [`create`] made: unless [`add`] stores it, the
/// folder is removed when this value is dropped (a refused or failed store, a request that ended early).
pub struct Folder {
    name: String,
    key: PathBuf,
    made: Option<PathBuf>,
}

impl Drop for Folder {
    /// Only what `create` made: its `settings.json`, then the then-empty folder (`remove_dir`, never recursive).
    fn drop(&mut self) {
        if let Some(dir) = self.made.take() {
            let _ = fs::remove_file(dir.join("settings.json"));
            let _ = fs::remove_dir(dir);
        }
    }
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
        made: None,
    })
}

const MAX_NAME: usize = 30;

/// A new account's name: 1 to 30 of `a-z`, `0-9`, `-`, not starting with `-` (`^[a-z0-9][a-z0-9-]{0,29}$`).
fn valid_name(name: &Value) -> Option<&str> {
    let name = name.as_str()?;
    let mut bytes = name.bytes();
    let first = bytes.next()?;
    let plain = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit();
    (name.len() <= MAX_NAME && plain(first) && bytes.all(|b| plain(b) || b == b'-')).then_some(name)
}

/// Makes a new Claude account folder `<agent root>/.claude-<name>` and returns it for [`add`] (T132). The name is checked
/// before anything is touched. `create_dir` (not `create_dir_all`) makes the folder 0700: a missing agent root is an
/// error, and any existing path (folder, file, dangling link) is a 409 that changes nothing. `settings.json` is `{}`,
/// created new, 0600. Like [`prepare_add`] it needs no store: a route runs it before the write lock. The result removes
/// the folder it made unless [`add`] stores it.
pub fn create(agent_root: &Path, name: &Value) -> Done<Folder> {
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    let Some(name) = valid_name(name) else {
        return Err(refuse(
            400,
            "name must be 1 to 30 characters: a-z, 0-9 and -, not starting with -",
        ));
    };
    let dir = agent_root.join(format!(".claude-{name}"));
    let failed = || {
        refuse(
            500,
            "the account folder could not be created; nothing was changed",
        )
    };
    match fs::DirBuilder::new().mode(0o700).create(&dir) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(refuse(409, "a folder with that name already exists"));
        }
        Err(_) => return Err(failed()),
    }
    // from here the folder is ours: `Folder` removes it on any failure
    let mut folder = Folder {
        name: String::new(),
        key: PathBuf::new(),
        made: Some(dir.clone()),
    };
    let settings = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(dir.join("settings.json"))
        .and_then(|mut file| file.write_all(b"{}"));
    settings.map_err(|_| failed())?;
    let made = prepare_add(&Value::String(dir.to_string_lossy().into_owned()))?;
    (folder.name, folder.key) = (made.name.clone(), made.key.clone());
    Ok(folder)
}

/// `add`: a folder by hand, anywhere; it must exist and pass the content check. A new one starts `detected`
/// (`confirmed` as "Claude default" for the agent root's own `.claude`); one already stored is returned as it is.
pub fn add(conn: &Connection, folder: Done<Folder>, agent_root: &Path) -> Done<Value> {
    let mut folder = folder?;
    let Some(provider) = crate::ingest::accounts::provider_of(Path::new(&folder.name)) else {
        return Err(refuse(
            400,
            "not an agent folder: no projects/, settings.json or config.toml",
        ));
    };
    let tx = begin(conn)?;
    crate::ingest::accounts::insert(
        &tx,
        provider,
        &folder.name,
        folder.name == default_folder(agent_root),
    )?;
    tx.commit()?;
    folder.made = None; // stored: the folder stays whatever happens next
    let id = find_folder(conn, &folder.key)?.ok_or_else(|| refuse(404, "no such account"))?;
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

#[cfg(test)]
mod create_tests {
    use super::*;
    use std::os::unix::fs::{MetadataExt, symlink};

    fn root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t132-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::canonicalize(dir).unwrap()
    }

    fn status(err: WriteFail) -> (u16, String) {
        match err {
            WriteFail::Refused { status, text } => (status, text),
            other => panic!("not a refusal: {other:?}"),
        }
    }

    fn listing(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    /// AC 1: every bad name is a 400 and nothing is made.
    #[test]
    fn a_bad_name_is_400_before_any_filesystem_work() {
        let root = root("bad");
        let long = "a".repeat(31);
        let bad = [
            json!(""),
            json!(".."),
            json!("../x"),
            json!("a/b"),
            json!("A"),
            json!("-a"),
            json!("a b"),
            json!("a_b"),
            json!("é"),
            json!("a\0"),
            json!("a\n"),
            json!(long),
            json!(5),
            json!(null),
            json!(["a"]),
        ];
        for name in bad {
            let got = create(&root, &name)
                .err()
                .unwrap_or_else(|| panic!("{name}"));
            assert_eq!(status(got).0, 400, "{name}");
        }
        assert!(listing(&root).is_empty(), "something was made");
        // the longest and the shortest good names
        for name in ["a".repeat(30), "0".into(), "a-b-9".into()] {
            drop(create(&root, &json!(name)).unwrap());
        }
        let _ = fs::remove_dir_all(&root);
    }

    /// AC 2: the folder is 0700, settings.json is `{}` 0600, and the folder is a Claude account for `add`.
    #[test]
    fn the_folder_and_settings_are_made_with_private_modes() {
        let root = root("make");
        let mut folder = create(&root, &json!("work")).unwrap();
        folder.made = None; // keep it for the checks
        let dir = root.join(".claude-work");
        assert_eq!(fs::metadata(&dir).unwrap().mode() & 0o7777, 0o700);
        let settings = dir.join("settings.json");
        assert_eq!(fs::metadata(&settings).unwrap().mode() & 0o7777, 0o600);
        assert_eq!(fs::read_to_string(&settings).unwrap(), "{}");
        assert_eq!(listing(&dir), ["settings.json"]);
        assert_eq!(folder.name, dir.to_str().unwrap());
        assert_eq!(crate::ingest::accounts::provider_of(&dir), Some("claude"));
        let _ = fs::remove_dir_all(&root);
    }

    /// AC 3: a directory, a file and a dangling link at the name are 409 and stay as they were.
    #[test]
    fn an_existing_path_is_409_and_nothing_changes() {
        let root = root("exists");
        fs::create_dir(root.join(".claude-dir")).unwrap();
        fs::write(root.join(".claude-dir/keep"), "owner data").unwrap();
        fs::write(root.join(".claude-file"), "owner file").unwrap();
        symlink(root.join("nowhere"), root.join(".claude-link")).unwrap();
        let before = listing(&root);
        for name in ["dir", "file", "link"] {
            let got = create(&root, &json!(name)).err().unwrap();
            assert_eq!(status(got).0, 409, "{name}");
        }
        assert_eq!(listing(&root), before);
        assert_eq!(
            fs::read_to_string(root.join(".claude-dir/keep")).unwrap(),
            "owner data"
        );
        assert_eq!(
            fs::read_to_string(root.join(".claude-file")).unwrap(),
            "owner file"
        );
        assert!(fs::symlink_metadata(root.join(".claude-link")).is_ok());
        assert!(!root.join("nowhere").exists(), "the link was followed");
        let _ = fs::remove_dir_all(&root);
    }

    /// A missing agent root is not made: an error, and no row or folder.
    #[test]
    fn a_missing_agent_root_is_an_error_and_is_not_created() {
        let root = root("noroot");
        let missing = root.join("absent");
        let got = create(&missing, &json!("work")).err().unwrap();
        assert_eq!(status(got).0, 500);
        assert!(!missing.exists());
        let _ = fs::remove_dir_all(&root);
    }

    /// AC 4: a folder never handed to a successful `add` is removed when it is dropped (the store failed, or the request
    /// ended), and only that folder: its neighbours stay.
    #[test]
    fn a_failure_after_the_folder_is_made_removes_only_that_folder() {
        let root = root("undo");
        fs::create_dir(root.join(".claude-other")).unwrap();
        fs::write(root.join(".claude-other/settings.json"), "{}").unwrap();
        // a connection with no tables: the insert fails after the folder exists
        let conn = Connection::open_in_memory().unwrap();
        let folder = create(&root, &json!("work"));
        assert!(root.join(".claude-work/settings.json").is_file());
        assert!(add(&conn, folder, &root).is_err());
        assert_eq!(listing(&root), [".claude-other"]);
        assert_eq!(
            fs::read_to_string(root.join(".claude-other/settings.json")).unwrap(),
            "{}"
        );
        // never reaching `add` (a busy store, a panic) undoes it the same way
        drop(create(&root, &json!("again")).unwrap());
        assert_eq!(listing(&root), [".claude-other"]);
        let _ = fs::remove_dir_all(&root);
    }
}
