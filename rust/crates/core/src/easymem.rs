//! The easy-memory store reader behind the memory graph (T043), ported from `src/easy_personal/core/easymem.py`.
//!
//! easy-memory keeps `<store>/<key>/notes/*.json`, `meta.json` and `prompts.log` (`$EASY_MEMORY_HOME`, else
//! `~/.easy-memory`). Only a note's `time` and `files` map and a log line's `time`, `effective` and `injected` are kept;
//! `prompt`, `answer` and `session_id` are parsed away and never returned, logged or put in an error. `report.json` is read
//! for its on-vs-off numbers only. `trash/` is never opened. A cited file is opened only to hash it, only inside the
//! project folder. Read-only: nothing is written anywhere.
//!
//! Every file is opened from a held folder handle with `O_NOFOLLOW`, judged on the open handle and read through a cap
//! (Python follows a symlinked note, log or report: here it is skipped, decided for you). A cited path is classified like
//! Python (its real path must stay inside the project folder; metadata only) and then hashed through a no-follow chain.
//! `Time`s are parsed as RFC 3339 / ISO 8601 by `jiff`, a time without an offset is UTC; Python's `fromisoformat` accepts a
//! few more spellings (decided for you).

use crate::board::{open_regular, read_capped, strip};
use crate::memgraph::names;
use crate::report_files::sys;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufRead, Read};
use std::path::{Component, Path, PathBuf};

const FORMAT_VERSION: i64 = 1;
const MAX_NOTE_BYTES: u64 = 1024 * 1024;
const MAX_META_BYTES: u64 = 64 * 1024;
const MAX_REPORT_BYTES: u64 = 64 * 1024;
const MAX_GENERATED_CHARS: usize = 64;
const REPORT_VERSION: i64 = 1;
const MAX_LOG_LINE: usize = 64 * 1024;
/// Python reads `prompts.log` to its end; a log longer than this is read only as far as this (decided for you).
const MAX_LOG_BYTES: u64 = 64 * 1024 * 1024;
const MAX_HASH_BYTES: u64 = 10 * 1024 * 1024;
const MAX_NODES: usize = 30;
const DAYS: i64 = 30;

/// `os.path.realpath` (not strict): symlinks resolved as far as the path exists, the rest joined lexically.
fn realpath(path: &str) -> String {
    let mut base = PathBuf::new();
    let mut rest: Vec<std::ffi::OsString> = Vec::new();
    for part in Path::new(path).components() {
        match part {
            Component::RootDir => base.push("/"),
            Component::Normal(name) => rest.push(name.to_owned()),
            Component::ParentDir => {
                rest.pop();
            }
            _ => {}
        }
    }
    let mut resolved = base.clone();
    let mut missing = false;
    for part in rest {
        let next = resolved.join(&part);
        if missing {
            resolved = next;
        } else if let Ok(real) = std::fs::canonicalize(&next) {
            resolved = real;
        } else {
            missing = true;
            resolved = next;
        }
    }
    resolved.to_string_lossy().into_owned()
}

/// The store, indexed once: `meta` = real identity path -> folder, `keyed` = folder name -> folder (no usable meta).
pub struct Store {
    meta: BTreeMap<String, PathBuf>,
    keyed: BTreeMap<String, PathBuf>,
}

fn meta_path(dir: &File) -> Option<String> {
    let (file, meta) = open_regular(dir, "meta.json")?;
    let data: Value = serde_json::from_slice(&read_capped(file, &meta, MAX_META_BYTES)?).ok()?;
    let path = data.as_object()?.get("path")?.as_str()?;
    (!path.is_empty() && !path.contains('\0')).then(|| path.to_string())
}

pub fn load(home: &Path) -> Store {
    let mut store = Store {
        meta: BTreeMap::new(),
        keyed: BTreeMap::new(),
    };
    let Some(root) = home.to_str().and_then(|h| sys::open_root(h).ok()) else {
        return store;
    };
    for name in names(&root) {
        let Ok(dir) = sys::open_dir_at(&root, &name) else {
            continue;
        };
        let folder = home.join(&name);
        let Some(path) = meta_path(&dir) else {
            store.keyed.insert(name, folder);
            continue;
        };
        let real = realpath(&path);
        store
            .meta
            .entry(real.clone())
            .or_insert_with(|| folder.clone());
        if let Some(repo) = real.strip_suffix("/.git") {
            store.meta.entry(repo.to_string()).or_insert(folder);
        }
    }
    store
}

/// easy-memory's identity path of a folder without a `git` call: its git common dir, else its real path.
fn identity(project: &str) -> String {
    let real = realpath(project);
    let dot_git = format!("{real}/.git");
    if Path::new(&dot_git).is_dir() {
        return realpath(&dot_git);
    }
    let first = f_line(&dot_git);
    let Some(rest) = first.strip_prefix("gitdir:") else {
        return real;
    };
    let gitdir = realpath(&join(&real, strip(rest)));
    match std::fs::read(format!("{gitdir}/commondir")) {
        Ok(bytes) => {
            let line = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]).to_string();
            realpath(&join(&gitdir, strip(line.lines().next().unwrap_or(""))))
        }
        Err(_) => gitdir,
    }
}

/// The first line (at most 4096 bytes) of a small file, empty when it cannot be read as UTF-8.
fn f_line(path: &str) -> String {
    let Ok(file) = File::open(path) else {
        return String::new();
    };
    let mut bytes = Vec::new();
    if std::io::BufReader::new(file.take(4096))
        .read_until(b'\n', &mut bytes)
        .is_err()
    {
        return String::new();
    }
    String::from_utf8(bytes).unwrap_or_default()
}

/// `os.path.join(base, part)`: an absolute `part` replaces `base`.
fn join(base: &str, part: &str) -> String {
    if part.starts_with('/') {
        part.to_string()
    } else {
        format!("{base}/{part}")
    }
}

impl Store {
    /// The store folder of a project path, or `None`: by `meta.json` path first, else by the computed key.
    pub fn find(&self, project: &str) -> Option<PathBuf> {
        let id = identity(project);
        self.meta
            .get(&id)
            .or_else(|| self.meta.get(&realpath(project)))
            .or_else(|| self.keyed.get(&sha256_hex(id.as_bytes())[..16]))
            .cloned()
    }
}

fn notes_dir(folder: &Path) -> Option<File> {
    sys::open_dir_at(&sys::open_root(folder.to_str()?).ok()?, "notes").ok()
}

/// True when the folder's `notes/` holds at least one `.json` name (no file is opened).
pub fn has_notes(folder: &Path) -> bool {
    notes_dir(folder).is_some_and(|dir| names(&dir).iter().any(|n| n.ends_with(".json")))
}

/// `(UTC instant in microseconds, local calendar day)` of an ISO time.
fn when(value: &Value, zone: &TimeZone) -> Option<(i64, Date)> {
    let text = strip(value.as_str()?);
    let stamp: Timestamp = text
        .parse::<Timestamp>()
        .ok()
        .or_else(|| {
            text.parse::<jiff::civil::DateTime>()
                .ok()?
                .to_zoned(TimeZone::UTC)
                .ok()
                .map(|z| z.timestamp())
        })
        .or_else(|| {
            text.parse::<Date>()
                .ok()?
                .to_zoned(TimeZone::UTC)
                .ok()
                .map(|z| z.timestamp())
        })?;
    Some((stamp.as_microsecond(), zone.to_datetime(stamp).date()))
}

type Note = (Option<(i64, Date)>, BTreeMap<String, String>);

/// `_load_note`: `(time, {cited path: hash})`, or `None` when the note is abstained.
fn load_note(dir: &File, name: &str, zone: &TimeZone) -> Option<Note> {
    let (file, meta) = open_regular(dir, name)?;
    let data: Value = serde_json::from_slice(&read_capped(file, &meta, MAX_NOTE_BYTES)?).ok()?;
    let data = data.as_object()?;
    let version = data.get("v")?;
    if !(version.is_i64() || version.is_u64()) || !(1..=FORMAT_VERSION).contains(&version.as_i64()?)
    {
        return None;
    }
    let cited = data
        .get("files")?
        .as_object()?
        .iter()
        .filter(|(path, _)| !path.is_empty())
        .map(|(path, hash)| (path.clone(), hash.as_str().unwrap_or("").to_string()))
        .collect();
    Some((data.get("time").and_then(|t| when(t, zone)), cited))
}

/// `_read_notes`: the usable notes (oldest file name first) and the number of abstained ones.
fn read_notes(folder: &Path, zone: &TimeZone) -> (Vec<Note>, usize) {
    let Some(dir) = notes_dir(folder) else {
        return (Vec::new(), 0);
    };
    let (mut notes, mut abstained) = (Vec::new(), 0);
    for name in names(&dir).into_iter().filter(|n| n.ends_with(".json")) {
        match load_note(&dir, &name, zone) {
            Some(note) => notes.push(note),
            None => abstained += 1,
        }
    }
    (notes, abstained)
}

/// Usable notes in a store folder (abstained ones are not counted).
pub fn note_count(folder: &Path) -> usize {
    read_notes(folder, &TimeZone::UTC).0.len()
}

/// `_inside`: the real path of a cited file when it stays inside the project folder, else `None`.
fn inside(root: &str, path: &str) -> Option<String> {
    if path.starts_with('/') || path.split('/').any(|p| p == "..") || path.contains('\0') {
        return None;
    }
    let real = realpath(&format!("{root}/{path}"));
    (real == root || real.starts_with(&format!("{root}/"))).then_some(real)
}

/// `_state`: `fresh`, `stale`, `gone` or `unchecked` for a cited file against the newest citing note's hash. The file is
/// opened below the project folder by a chain of no-follow opens (its real path has no symlink left to follow).
fn state(root: &str, real: &str, newest_hash: &str) -> &'static str {
    let rel = real
        .strip_prefix(root)
        .unwrap_or("")
        .trim_start_matches('/');
    if rel.is_empty() {
        return "unchecked"; // the project folder itself
    }
    let mut dirs: Vec<&str> = rel.split('/').collect();
    let last = dirs.pop().unwrap_or("");
    let Ok(mut dir) = sys::open_root(root) else {
        return "unchecked";
    };
    for part in dirs {
        dir = match sys::open_dir_at(&dir, part) {
            Ok(next) => next,
            Err(e)
                if matches!(e.kind(), std::io::ErrorKind::NotFound)
                    || e.raw_os_error() == Some(20) =>
            {
                return "gone";
            }
            Err(_) => return "unchecked",
        };
    }
    let file = match sys::open_file_at(&dir, last) {
        Ok(file) => file,
        Err(e)
            if matches!(e.kind(), std::io::ErrorKind::NotFound) || e.raw_os_error() == Some(20) =>
        {
            return "gone";
        }
        Err(_) => return "unchecked",
    };
    let Ok(meta) = file.metadata() else {
        return "unchecked";
    };
    if !meta.is_file() || meta.len() > MAX_HASH_BYTES {
        return "unchecked";
    }
    let mut bytes = Vec::new();
    if file
        .take(MAX_HASH_BYTES + 1)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() as u64 > MAX_HASH_BYTES
    {
        return "unchecked";
    }
    if sha256_hex(&bytes) == newest_hash.to_lowercase() {
        "fresh"
    } else {
        "stale"
    }
}

/// `_prompts`: `(prompts with memory on, of those injected)` over the window, from `prompts.log` line by line.
fn prompts(folder: &Path, start: Date, today: Date, zone: &TimeZone) -> (u64, u64) {
    let Some(file) = folder
        .to_str()
        .and_then(|f| sys::open_root(f).ok())
        .and_then(|dir| open_regular(&dir, "prompts.log"))
        .map(|(file, _)| file)
    else {
        return (0, 0);
    };
    let (mut on, mut injected) = (0, 0);
    let mut reader = std::io::BufReader::new(file.take(MAX_LOG_BYTES));
    let mut line = Vec::new();
    loop {
        line.clear();
        let Ok(read) = reader
            .by_ref()
            .take(MAX_LOG_LINE as u64 + 1)
            .read_until(b'\n', &mut line)
        else {
            break;
        };
        if read == 0 {
            break;
        }
        if line.len() > MAX_LOG_LINE && !line.ends_with(b"\n") {
            let mut rest = Vec::new();
            while let Ok(n) = reader
                .by_ref()
                .take(MAX_LOG_LINE as u64)
                .read_until(b'\n', &mut rest)
            {
                if n == 0 || rest.ends_with(b"\n") {
                    break;
                }
                rest.clear();
            }
            continue;
        }
        let Ok(record) = serde_json::from_slice::<Value>(&line) else {
            continue;
        };
        let Some(record) = record.as_object() else {
            continue;
        };
        let Some((_, day)) = record.get("time").and_then(|t| when(t, zone)) else {
            continue;
        };
        if day < start || day > today || record.get("effective") != Some(&json!("on")) {
            continue;
        }
        on += 1;
        injected += u64::from(record.get("injected") == Some(&json!(true)));
    }
    (on, injected)
}

type Cite = (usize, i64, String, Option<Date>);
type Inside<'a> = (usize, &'a String, String, &'a Cite);

/// One cited file of the graph.
pub struct Cited {
    pub path: String,
    pub cited: usize,
    pub newest: String,
    pub stale: &'static str,
}

/// `summary`: `(panel, cited)` of one project: the panel numbers and its top 30 cited files with their stale state.
pub fn summary(folder: &Path, project: &str, today: Date, zone: &TimeZone) -> (Value, Vec<Cited>) {
    let start = today.saturating_sub(Span::new().days(DAYS - 1));
    let days: Vec<Date> = (0..DAYS)
        .filter_map(|i| start.checked_add(Span::new().days(i)).ok())
        .collect();
    let mut counts: HashMap<Date, u64> = days.iter().map(|d| (*d, 0)).collect();
    let (notes, abstained) = read_notes(folder, zone);
    // path -> (count, newest key, newest hash, newest local day)
    let mut cites: HashMap<String, Cite> = HashMap::new();
    for (time, files) in &notes {
        if let Some((_, day)) = time
            && let Some(n) = counts.get_mut(day)
        {
            *n += 1;
        }
        let (key, day) = time.map_or((i64::MIN, None), |(k, d)| (k, Some(d)));
        for (path, hash) in files {
            match cites.get_mut(path) {
                None => {
                    cites.insert(path.clone(), (1, key, hash.clone(), day));
                }
                Some(entry) => {
                    entry.0 += 1;
                    if key >= entry.1 {
                        (entry.1, entry.2, entry.3) = (key, hash.clone(), day);
                    }
                }
            }
        }
    }
    let root = realpath(project);
    let mut inside_files: Vec<Inside<'_>> = cites
        .iter()
        .filter_map(|(path, entry)| inside(&root, path).map(|real| (entry.0, path, real, entry)))
        .collect();
    inside_files.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    let cited: Vec<Cited> = inside_files
        .iter()
        .take(MAX_NODES)
        .map(|(count, path, real, entry)| Cited {
            path: (*path).clone(),
            cited: *count,
            newest: entry.3.map_or(String::new(), |d| d.to_string()),
            stale: state(&root, real, &entry.2),
        })
        .collect();
    let (on, injected) = prompts(folder, start, today, zone);
    let panel = json!({
        "notes": notes.len(),
        "abstained": abstained,
        "days": days.iter().map(|d| json!({"date": d.to_string(), "count": counts[d]})).collect::<Vec<_>>(),
        "prompts_on": on,
        "injected": injected,
        "files_cited": cites.len(),
        "files_not_drawn": cites.len() - cited.len(),
        "outside": cites.len() - inside_files.len(),
    });
    (panel, cited)
}

fn is_count(value: &Value) -> bool {
    value.is_u64()
}

/// `_group`: `{sessions, tokens_all_median}` of one report group, or `None` when a field has the wrong type.
fn group(raw: Option<&Value>) -> Option<Value> {
    let raw = raw?.as_object()?;
    let sessions = raw.get("sessions").filter(|s| is_count(s))?;
    match raw.get("tokens_all_median").filter(|m| !m.is_null()) {
        None if sessions.as_u64() == Some(0) => {
            Some(json!({"sessions": 0, "tokens_all_median": null}))
        }
        Some(m) if m.as_f64().is_some_and(|x| x >= 0.0) => {
            Some(json!({"sessions": sessions, "tokens_all_median": m}))
        }
        _ => None,
    }
}

/// `report`: the on-vs-off comparison of a store folder's `report.json`. Never fails: `{"state": "none"}` for anything
/// missing, unreadable, too big (64 KB), malformed or wrongly typed.
pub fn report(folder: &Path) -> Value {
    let none = json!({"state": "none"});
    let Some((file, meta)) = folder
        .to_str()
        .and_then(|f| sys::open_root(f).ok())
        .and_then(|dir| open_regular(&dir, "report.json"))
    else {
        return none;
    };
    let Some(data) = read_capped(file, &meta, MAX_REPORT_BYTES)
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
    else {
        return none;
    };
    let Some(data) = data.as_object() else {
        return none;
    };
    let Some(version) = data.get("v").filter(|v| v.is_number()) else {
        return none;
    };
    if version.as_f64().is_some_and(|v| v > REPORT_VERSION as f64) {
        return json!({"state": "unsupported"});
    }
    let groups = data.get("groups").and_then(Value::as_object);
    let on = groups.and_then(|g| group(g.get("on")));
    let off = groups.and_then(|g| group(g.get("off")));
    let (generated, minimum, too_few) = (
        data.get("generated"),
        data.get("min_sessions"),
        data.get("too_few"),
    );
    let percent = data.get("on_vs_off_pct");
    let (Some(on), Some(off)) = (on, off) else {
        return none;
    };
    if version.as_i64() != Some(REPORT_VERSION) {
        return none;
    }
    let Some(generated) = generated
        .and_then(Value::as_str)
        .filter(|g| g.chars().count() <= MAX_GENERATED_CHARS)
    else {
        return none;
    };
    let Some(minimum) = minimum.filter(|m| is_count(m) && m.as_u64() >= Some(1)) else {
        return none;
    };
    let Some(too_few) = too_few.and_then(Value::as_bool) else {
        return none;
    };
    let Some(percent) = percent.filter(|p| p.is_null() || p.is_number()) else {
        return none;
    };
    json!({"state": "ok", "generated": generated, "min_sessions": minimum, "too_few": too_few,
           "on_vs_off_pct": percent, "on": on, "off": off})
}

/// SHA-256 as lowercase hex (FIPS 180-4); the cited-file hash and the store key of easy-memory are this digest.
fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut padded = data.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for block in padded.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in block.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for (a, b) in h.iter_mut().zip(v) {
            *a = a.wrapping_add(b);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_the_standard_vectors() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
