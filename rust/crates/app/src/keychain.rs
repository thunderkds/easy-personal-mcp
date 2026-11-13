//! Connector secrets in the OS keychain (T050, FR-017): Python's `launch.py` (`service_name`, `require_backend`, `load`)
//! and `connectors/keychain.py` (`store`, `remove`). One item per connector: service `easy-personal/<name>`, user
//! `connector`, secret = the JSON `{"command": [...], "env": {...}, "headers": {...}}` written as Python's `json.dumps`
//! writes it (`", "` and `": "`, every character outside `' '..='~'` as `\uXXXX`), so an item either program wrote is
//! read by the other.
//!
//! The backend is behind the [`Keychain`] trait. [`system`] gives the real one: the Linux Secret Service through the
//! `keyring` crate (an item Python wrote has no `target` attribute; `keyring` finds it by its fallback search of the
//! default collection). On any other OS there is no backend: every write is refused with [`NO_BACKEND`], never kept in
//! memory or in a file. A **debug build** also honours `EASY_PERSONAL_TEST_KEYCHAIN` (a JSON file, or `unusable`): the
//! fake the tests and the leak gate use, so no test touches a real keychain. A release build has no such switch.
//!
//! No error text holds a secret, and none is built from the backend's own error (which can name an item).

use easy_personal_core::pyjson::py_list;
pub use easy_personal_core::pyjson::py_object;
use serde_json::{Map, Value};

pub const SERVICE_PREFIX: &str = "easy-personal/";
pub const ITEM_USER: &str = "connector";
pub const NO_BACKEND: &str = "no usable OS keychain (Secret Service); unlock or install one, e.g. gnome-keyring. \
                              Secrets are never stored in plaintext";
/// The debug-build switch to the file-backed fake (see the module text).
pub const TEST_KEYCHAIN_ENV: &str = "EASY_PERSONAL_TEST_KEYCHAIN";

/// Python's `LaunchError`: an expected failure whose text is safe to print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchError(pub String);

impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The backend failed (locked, gone, refused); the reason is not kept, so it can never be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failed;

/// What a backend does. `service` is always `easy-personal/<valid name>`; the user is always [`ITEM_USER`].
pub trait Keychain: Send + Sync {
    /// Python's `require_backend`: a backend is there and answers.
    fn usable(&self) -> bool;
    fn get(&self, service: &str) -> Result<Option<String>, Failed>;
    fn set(&self, service: &str, secret: &str) -> Result<(), Failed>;
    /// Removing an absent item is not a failure.
    fn delete(&self, service: &str) -> Result<(), Failed>;
}

/// `[a-z0-9][a-z0-9_-]{0,63}` (Python's `_NAME`), on the whole text.
pub fn valid_name(name: &str) -> bool {
    let b = name.as_bytes();
    (1..=64).contains(&b.len())
        && matches!(b[0], b'a'..=b'z' | b'0'..=b'9')
        && b[1..]
            .iter()
            .all(|c| matches!(c, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-'))
}

/// `service_name`: the item's service, or `invalid connector name`. Checked before any backend call.
pub fn service_name(connector: &str) -> Result<String, LaunchError> {
    if valid_name(connector) {
        Ok(format!("{SERVICE_PREFIX}{connector}"))
    } else {
        Err(LaunchError("invalid connector name".into()))
    }
}

fn no_backend() -> LaunchError {
    LaunchError(NO_BACKEND.into())
}

/// `require_backend`.
pub fn require_backend(kc: &dyn Keychain) -> Result<(), LaunchError> {
    if kc.usable() {
        Ok(())
    } else {
        Err(no_backend())
    }
}

/// `writer._keychain_item`: whether the connector has an item (its value is never read out).
pub fn has_item(kc: &dyn Keychain, name: &str) -> Result<bool, LaunchError> {
    let service = service_name(name)?;
    require_backend(kc)?;
    Ok(kc.get(&service).map_err(|Failed| no_backend())?.is_some())
}

/// `keychain.store`: one item holding the provider command and its secrets, written as Python writes it.
pub fn store(
    kc: &dyn Keychain,
    connector: &str,
    command: &[String],
    env: &[(String, String)],
    headers: &[(String, String)],
) -> Result<(), LaunchError> {
    let service = service_name(connector)?;
    require_backend(kc)?;
    kc.set(&service, &item_json(command, env, headers))
        .map_err(|Failed| no_backend())
}

/// `keychain.remove`: an absent item is fine.
pub fn remove(kc: &dyn Keychain, connector: &str) -> Result<(), LaunchError> {
    let service = service_name(connector)?;
    require_backend(kc)?;
    kc.delete(&service).map_err(|Failed| no_backend())
}

/// A connector's item as the launcher uses it.
#[derive(Debug, PartialEq, Eq)]
pub struct Item {
    pub command: Vec<String>,
    pub env: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
}

/// `load`: the item of a configured connector, or Python's fixed text (none holds a secret).
pub fn load(kc: &dyn Keychain, connector: &str) -> Result<Item, LaunchError> {
    let service = service_name(connector)?;
    require_backend(kc)?;
    let raw = kc
        .get(&service)
        .map_err(|Failed| no_backend())?
        .ok_or_else(|| LaunchError(format!("connector '{connector}' is not configured")))?;
    parse_item(&raw).ok_or_else(|| {
        LaunchError(format!(
            "connector '{connector}' has an unreadable keychain entry"
        ))
    })
}

/// Python's checks on `json.loads(raw)`: an object; `command` (default `[]`) a list of strings; `env` and `headers`
/// (default `{}`) objects of strings. Anything else is unreadable. Keys keep `serde_json`'s order (sorted): only the
/// `headers` output shows an order, and Python's own writes always store `{}` there.
fn parse_item(raw: &str) -> Option<Item> {
    let Value::Object(item) = serde_json::from_str::<Value>(raw).ok()? else {
        return None;
    };
    let command = match item.get("command") {
        None => Vec::new(),
        Some(Value::Array(list)) => list
            .iter()
            .map(|v| v.as_str().map(str::to_string))
            .collect::<Option<_>>()?,
        Some(_) => return None,
    };
    let strings = |key: &str| -> Option<Vec<(String, String)>> {
        match item.get(key) {
            None => Some(Vec::new()),
            Some(Value::Object(map)) => map
                .iter()
                .map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_string())))
                .collect(),
            Some(_) => None,
        }
    };
    Some(Item {
        command,
        env: strings("env")?,
        headers: strings("headers")?,
    })
}

/// `json.dumps({"command": command, "env": env, "headers": headers})` with Python's defaults.
pub fn item_json(
    command: &[String],
    env: &[(String, String)],
    headers: &[(String, String)],
) -> String {
    format!(
        "{{\"command\": {}, \"env\": {}, \"headers\": {}}}",
        py_list(command),
        py_object(env),
        py_object(headers)
    )
}

// ---- backends ----

/// The backend this process uses: the debug-build fake when `EASY_PERSONAL_TEST_KEYCHAIN` is set, else the OS one.
pub fn system() -> Box<dyn Keychain> {
    #[cfg(debug_assertions)]
    if let Some(path) = std::env::var_os(TEST_KEYCHAIN_ENV) {
        return Box::new(FileKeychain(path.into()));
    }
    Box::new(SecretService)
}

/// The Linux Secret Service through `keyring` (Python's `SecretService` backend reads and writes the same items).
pub struct SecretService;

#[cfg(target_os = "linux")]
impl SecretService {
    fn entry(service: &str) -> Result<keyring::Entry, Failed> {
        keyring::Entry::new(service, ITEM_USER).map_err(|_| Failed)
    }
}

#[cfg(target_os = "linux")]
impl Keychain for SecretService {
    /// Python asks the backend's priority, which connects to the service; here a read of an item no connector can have
    /// (its name breaks the name rule) does the same: it answers, found or not, only when the service is there.
    fn usable(&self) -> bool {
        Self::entry("easy-personal/.probe")
            .map(|e| matches!(e.get_password(), Ok(_) | Err(keyring::Error::NoEntry)))
            .unwrap_or(false)
    }

    fn get(&self, service: &str) -> Result<Option<String>, Failed> {
        match Self::entry(service)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(Failed),
        }
    }

    fn set(&self, service: &str, secret: &str) -> Result<(), Failed> {
        Self::entry(service)?
            .set_password(secret)
            .map_err(|_| Failed)
    }

    fn delete(&self, service: &str) -> Result<(), Failed> {
        match Self::entry(service)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(Failed),
        }
    }
}

/// Linux only (the task's scope): elsewhere there is no backend, so nothing is ever written.
#[cfg(not(target_os = "linux"))]
impl Keychain for SecretService {
    fn usable(&self) -> bool {
        false
    }
    fn get(&self, _: &str) -> Result<Option<String>, Failed> {
        Err(Failed)
    }
    fn set(&self, _: &str, _: &str) -> Result<(), Failed> {
        Err(Failed)
    }
    fn delete(&self, _: &str) -> Result<(), Failed> {
        Err(Failed)
    }
}

/// The test fake (debug builds only): a JSON object `{service: secret}` in one file. The value `unusable` is a machine
/// with no backend; a file holding exactly `locked` is a backend that answers but refuses every item.
#[cfg(any(debug_assertions, test))]
pub struct FileKeychain(pub std::path::PathBuf);

#[cfg(any(debug_assertions, test))]
impl FileKeychain {
    fn read(&self) -> Result<Map<String, Value>, Failed> {
        match std::fs::read_to_string(&self.0) {
            Ok(text) if text == "locked" => Err(Failed),
            Ok(text) => match serde_json::from_str(&text) {
                Ok(Value::Object(map)) => Ok(map),
                _ => Err(Failed),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Map::new()),
            Err(_) => Err(Failed),
        }
    }

    fn write(&self, map: &Map<String, Value>) -> Result<(), Failed> {
        std::fs::write(&self.0, Value::Object(map.clone()).to_string()).map_err(|_| Failed)
    }
}

#[cfg(any(debug_assertions, test))]
impl Keychain for FileKeychain {
    fn usable(&self) -> bool {
        self.0.as_os_str() != "unusable"
    }

    fn get(&self, service: &str) -> Result<Option<String>, Failed> {
        Ok(self
            .read()?
            .get(service)
            .and_then(Value::as_str)
            .map(str::to_string))
    }

    fn set(&self, service: &str, secret: &str) -> Result<(), Failed> {
        let mut map = self.read()?;
        map.insert(service.into(), Value::String(secret.into()));
        self.write(&map)
    }

    fn delete(&self, service: &str) -> Result<(), Failed> {
        let mut map = self.read()?;
        if map.remove(service).is_some() {
            self.write(&map)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t050-kc-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("keychain.json")
    }

    fn s(v: &str) -> String {
        v.to_string()
    }

    #[test]
    fn the_name_rule_is_pythons_and_refuses_before_the_backend() {
        for good in ["slack", "c0123abcd", "a", "0-_", &"a".repeat(64)] {
            assert!(valid_name(good), "{good:?}");
        }
        for bad in [
            "",
            "Slack",
            "-a",
            "_a",
            "a/b",
            "..",
            "a.b",
            "a b",
            "a\0",
            &"a".repeat(65),
            "é",
            "a\n",
        ] {
            assert!(!valid_name(bad), "{bad:?}");
            assert_eq!(
                service_name(bad),
                Err(LaunchError("invalid connector name".into()))
            );
        }
        // the name is checked first: an unusable backend is never asked about a bad name
        let kc = FileKeychain("unusable".into());
        assert_eq!(
            store(&kc, "Bad", &[], &[], &[]).unwrap_err().0,
            "invalid connector name"
        );
        assert_eq!(remove(&kc, "../x").unwrap_err().0, "invalid connector name");
        assert_eq!(load(&kc, "").unwrap_err().0, "invalid connector name");
    }

    #[test]
    fn the_item_is_written_byte_for_byte_as_python_json_dumps_writes_it() {
        // expected: Python `json.dumps({"command": [...], "env": {...}, "headers": {...}})`
        let got = item_json(
            &[s("npx"), s("-y"), s("@x/y@1.2.3")],
            &[(s("TOKEN"), s("xoxb-\u{e9}\n\"q\"\\\u{7f}\u{1f600}\t\u{1}"))],
            &[],
        );
        assert_eq!(
            got,
            concat!(
                r#"{"command": ["npx", "-y", "@x/y@1.2.3"], "env": {"TOKEN": "xoxb-"#,
                "\\u00e9",
                r#"\n\"q\"\\\u007f"#,
                "\\ud83d\\ude00",
                r#"\t\u0001"}, "headers": {}}"#
            )
        );
        assert_eq!(
            item_json(&[], &[], &[]),
            r#"{"command": [], "env": {}, "headers": {}}"#
        );
        assert_eq!(
            py_object(&[(s("B"), s("1")), (s("A"), s("2"))]),
            r#"{"B": "1", "A": "2"}"#,
            "pairs keep their order"
        );
    }

    #[test]
    fn store_load_remove_round_trip_and_an_absent_item_is_not_an_error() {
        let file = temp("round");
        let kc = FileKeychain(file.clone());
        let env = vec![(s("SLACK_TOKEN"), s("secret-1"))];
        store(
            &kc,
            "slack",
            &[s("prog"), s("arg")],
            &env,
            &[(s("Authorization"), s("Bearer x"))],
        )
        .unwrap();
        assert!(has_item(&kc, "slack").unwrap());
        let item = load(&kc, "slack").unwrap();
        assert_eq!(item.command, [s("prog"), s("arg")]);
        assert_eq!(item.env, env);
        assert_eq!(item.headers, [(s("Authorization"), s("Bearer x"))]);
        remove(&kc, "slack").unwrap();
        assert!(!has_item(&kc, "slack").unwrap());
        remove(&kc, "slack").unwrap(); // absent: fine
        assert_eq!(
            load(&kc, "slack").unwrap_err().0,
            "connector 'slack' is not configured"
        );
    }

    #[test]
    fn no_backend_or_a_locked_one_refuses_with_pythons_text_and_writes_nothing() {
        let kc = FileKeychain("unusable".into());
        for err in [
            store(&kc, "slack", &[], &[(s("K"), s("v"))], &[]).unwrap_err(),
            remove(&kc, "slack").unwrap_err(),
            load(&kc, "slack").unwrap_err(),
            has_item(&kc, "slack").unwrap_err(),
        ] {
            assert_eq!(err.0, NO_BACKEND);
        }
        assert!(NO_BACKEND.ends_with("Secrets are never stored in plaintext"));
        let file = temp("locked");
        std::fs::write(&file, "locked").unwrap();
        let locked = FileKeychain(file.clone());
        assert_eq!(
            store(&locked, "slack", &[], &[(s("K"), s("canary-v"))], &[])
                .unwrap_err()
                .0,
            NO_BACKEND
        );
        assert_eq!(load(&locked, "slack").unwrap_err().0, NO_BACKEND);
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "locked",
            "nothing written"
        );
    }

    /// AC9, by hand only (needs a session bus and an unlocked keyring; it writes two throwaway items and deletes them):
    /// `cargo test -p easy-personal-rs --bin easy-personal-rs real_keychain -- --ignored`. Python is
    /// `rust/parity/.venv/bin/python` (or `EASY_PERSONAL_PYTHON`), with the app's `keyring`.
    #[test]
    #[ignore = "touches the real Secret Service: run once by hand (T050 AC9)"]
    fn real_keychain_items_cross_check_with_python_keyring() {
        let mut bytes = [0u8; 6];
        getrandom::fill(&mut bytes).unwrap();
        let tag: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let (rust_name, py_name) = (
            format!("t050-crosscheck-{tag}-r"),
            format!("t050-crosscheck-{tag}-p"),
        );
        let python = std::env::var("EASY_PERSONAL_PYTHON").unwrap_or_else(|_| {
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../parity/.venv/bin/python").to_string()
        });
        let py = |code: &str| {
            let out = std::process::Command::new(&python)
                .args(["-c", code])
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "python failed: exit {:?}",
                out.status.code()
            );
            String::from_utf8(out.stdout).unwrap()
        };
        let kc = SecretService;
        assert!(kc.usable(), "no usable Secret Service in this session");
        let env = vec![(s("K"), s("v-\u{e9}"))];
        store(&kc, &rust_name, &[s("prog")], &env, &[]).unwrap();
        let read = py(&format!(
            "import keyring; print(keyring.get_password('easy-personal/{rust_name}', 'connector'))"
        ));
        py(&format!(
            "from easy_personal.connectors import keychain; keychain.store('{py_name}', ['prog'], env={{'K': 'v-\u{e9}'}})"
        ));
        let back = load(&kc, &py_name);
        remove(&kc, &rust_name).unwrap();
        remove(&kc, &py_name).unwrap();
        assert_eq!(
            read.trim_end(),
            item_json(&[s("prog")], &env, &[]),
            "Python reads what Rust wrote"
        );
        assert_eq!(back.unwrap().env, env, "Rust reads what Python wrote");
        assert!(!has_item(&kc, &rust_name).unwrap() && !has_item(&kc, &py_name).unwrap());
    }

    #[test]
    fn a_malformed_item_is_unreadable_and_the_text_holds_no_value() {
        let file = temp("bad");
        let kc = FileKeychain(file.clone());
        for raw in [
            "not json",
            "[]",
            r#"{"command": "prog"}"#,
            r#"{"command": ["a", 1]}"#,
            r#"{"env": ["x"]}"#,
            r#"{"env": {"K": 1}}"#,
            r#"{"env": null}"#,
            r#"{"headers": {"H": ["secret-canary"]}}"#,
        ] {
            kc.set("easy-personal/slack", raw).unwrap();
            let err = load(&kc, "slack").unwrap_err().0;
            assert_eq!(
                err, "connector 'slack' has an unreadable keychain entry",
                "{raw}"
            );
        }
        kc.set("easy-personal/slack", "{}").unwrap();
        assert_eq!(
            load(&kc, "slack").unwrap(),
            Item {
                command: vec![],
                env: vec![],
                headers: vec![]
            },
            "absent keys default as in Python"
        );
    }
}
