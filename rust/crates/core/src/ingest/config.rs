//! `core/config.py` `ui_reports_dir` / `verifier_reports_dir` and the flat reader of `core/rules.py` `_read_flat`
//! (T033): where the easy-ui and easy-verifier report folders are. The environment value wins; else `reports_dir`
//! under `ui:` / `verifier:` of `config.yaml`; the verifier then falls back to `~/.easy-verifier-reports`. Nothing
//! here reads an environment variable: the caller passes the values.
//!
//! `~` is expanded from `HOME` as `Path.expanduser()` does for `~` and `~/...`; Python also expands `~user` from the
//! password database, Rust leaves it as written (decided for you). The folder is the text as `Path(...)` keeps it
//! (repeated and trailing separators dropped).

use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

/// `str.strip()`: Unicode white space, which for Python also holds `\x1c`-`\x1f`.
fn py_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || matches!(c, '\x1c'..='\x1f'))
}

/// `_read_flat(path, section).get(key)`: `section:` then indented `name: value` lines; the last duplicate wins. A file
/// that cannot be read or is not UTF-8 gives `None` (Python catches `OSError` and `UnicodeDecodeError`).
pub fn read_flat_value(file: &Path, section: &str, key: &str) -> Option<String> {
    let text = String::from_utf8(fs::read(file).ok()?).ok()?;
    let mut found = None;
    let mut inside = false;
    // `read_text()` makes every line break `\n`; `splitlines()` then splits on these characters.
    for line in text.split(|c| {
        matches!(
            c,
            '\n' | '\r' | '\x0b' | '\x0c' | '\x1c'..='\x1e' | '\u{85}' | '\u{2028}' | '\u{2029}'
        )
    }) {
        if py_trim(line).is_empty() || line.trim_start_matches(py_space).starts_with('#') {
            continue;
        }
        if !line.starts_with(' ') {
            inside = line.split(':').next() == Some(section);
        } else if inside {
            let (name, value) = py_trim(line).split_once(':').unwrap_or((py_trim(line), ""));
            if name == key {
                found = Some(py_trim(value).to_owned());
            }
        }
    }
    found
}

fn py_space(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\x1c'..='\x1f')
}

fn non_empty(value: Option<OsString>) -> Option<OsString> {
    value.filter(|v| !v.is_empty())
}

/// The configured folder: the (non-empty) environment value, else the config file's value.
fn configured(
    env: Option<OsString>,
    config_file: &Path,
    section: &str,
    home: Option<&OsStr>,
) -> Option<PathBuf> {
    let value = non_empty(env).or_else(|| {
        if config_file.exists() {
            non_empty(read_flat_value(config_file, section, "reports_dir").map(OsString::from))
        } else {
            None
        }
    })?;
    Some(expand_user(&value, home))
}

/// `Path(value).expanduser()` for `~` and `~/...`, then `Path`'s own spelling.
fn expand_user(value: &OsStr, home: Option<&OsStr>) -> PathBuf {
    let bytes = value.as_bytes();
    let expanded = match home {
        Some(home) if bytes == b"~" => home.to_os_string(),
        Some(home) if bytes.starts_with(b"~/") => {
            let mut path = home.as_bytes().to_vec();
            path.extend_from_slice(&bytes[1..]);
            OsString::from_vec(path)
        }
        _ => value.to_os_string(),
    };
    Path::new(&expanded).components().collect()
}

/// `config.ui_reports_dir()`: `None` when not configured (the UI checks have no data).
pub fn ui_reports_dir(
    config_file: &Path,
    env: Option<OsString>,
    home: Option<&OsStr>,
) -> Option<PathBuf> {
    configured(env, config_file, "ui", home)
}

/// `config.verifier_reports_dir()`; `None` only when nothing is configured and there is no `HOME` for the default.
pub fn verifier_reports_dir(
    config_file: &Path,
    env: Option<OsString>,
    home: Option<&OsStr>,
) -> Option<PathBuf> {
    configured(env, config_file, "verifier", home)
        .or_else(|| home.map(|h| Path::new(h).join(".easy-verifier-reports")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(text: &str, section: &str) -> Option<String> {
        let dir = std::env::temp_dir().join(format!("ep-t033-cfg-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join(format!("{}.yaml", text.len()));
        fs::write(&file, text).unwrap();
        read_flat_value(&file, section, "reports_dir")
    }

    #[test]
    fn the_flat_reader_follows_python() {
        let cfg = "# c\nthresholds:\n  reports_dir: no\nui:\n  reports_dir: /a:b  \n  x: 1\n";
        assert_eq!(value(cfg, "ui").as_deref(), Some("/a:b"));
        assert_eq!(
            value("ui: # note\n  reports_dir: /x\n", "ui").as_deref(),
            Some("/x")
        );
        assert_eq!(value("ui:\n\treports_dir: /x\n", "ui"), None);
        assert_eq!(value("ui :\n  reports_dir: /x\n", "ui"), None);
        assert_eq!(
            value("ui:\n  reports_dir: /1\n  reports_dir: /2\n", "ui").as_deref(),
            Some("/2")
        );
    }

    #[test]
    fn a_tilde_is_the_home_folder() {
        let home = OsString::from("/h");
        let p = |v: &str| expand_user(OsStr::new(v), Some(&home));
        assert_eq!(p("~"), Path::new("/h"));
        assert_eq!(p("~/a//b/"), Path::new("/h/a/b"));
        assert_eq!(p("~x/a"), Path::new("~x/a"));
        assert_eq!(p("/a/./b"), Path::new("/a/b"));
    }
}
