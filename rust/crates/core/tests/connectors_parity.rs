//! T046: the catalog parser and the connector views against Python.
//!
//! * the embedded catalog files are byte for byte `src/easy_personal/catalog/*` (same list, same bytes);
//! * the four goldens of `rust/parity/golden` and `golden-rich` (`mcp__connector_{catalog,status,suggestions}__all`,
//!   `api__connectors__all`) equal the core's answer over each oracle's store (8 comparisons);
//! * `connectors_cases.json` (made by `rust/parity/gen_connectors_cases.py` from Python's own `catalog.parse`): 495
//!   texts, each answered with the same entry, the same `CatalogError` text, or, where Python raised something that is
//!   not a `CatalogError`, a clean refusal;
//! * the new modules start no process and open no socket.
//!
//! Stores are the parity files opened read-only; the owner's real `~/.easy-personal` is never opened.

use easy_personal_core::catalog::{self, Entry, parse, parse_bytes};
use easy_personal_core::store::open_read_only;
use easy_personal_core::suggestions;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

fn parity_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity")
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap_or_else(|e| panic!("{path:?}: {e}")))
        .unwrap()
}

fn python_catalog_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../src/easy_personal/catalog")
}

#[test]
fn the_embedded_catalog_files_are_byte_for_byte_pythons() {
    let mut python: Vec<String> = fs::read_dir(python_catalog_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    python.sort();
    let ours: Vec<String> = catalog::ids()
        .iter()
        .map(|id| format!("{id}.yaml"))
        .collect();
    assert_eq!(ours, python, "the same files, in Python's `ids()` order");
    for id in catalog::ids() {
        let theirs = fs::read(python_catalog_dir().join(format!("{id}.yaml"))).unwrap();
        assert_eq!(catalog::file(id).unwrap(), theirs.as_slice(), "{id}.yaml");
        // the copy on disk that the build embeds
        let copy =
            fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("catalog/{id}.yaml")))
                .unwrap();
        assert_eq!(copy, theirs, "catalog/{id}.yaml");
    }
}

#[test]
fn every_embedded_file_parses_to_the_entry_python_builds() {
    let entries = catalog::entries().unwrap();
    assert_eq!(entries.len(), 5);
    let slack = entries.iter().find(|e| e.id == "slack").unwrap();
    assert_eq!(slack.category, "Messaging");
    assert_eq!(slack.provider, "Slack");
    assert!(slack.claude_ai_connector);
    assert_eq!(
        slack.claude_ai_link.as_deref(),
        Some("https://claude.ai/settings/connectors")
    );
    assert_eq!(
        slack.command,
        [
            "npx",
            "-y",
            "slack-mcp-server@1.3.0",
            "--transport",
            "stdio"
        ]
    );
    assert_eq!(slack.fields.len(), 1);
    assert_eq!(slack.fields[0].env, "SLACK_MCP_XOXB_TOKEN");
    assert!(slack.fields[0].secret);
    let gmail = entries.iter().find(|e| e.id == "gmail").unwrap();
    assert!(!gmail.fields[0].secret, "plain field");
    let discord = entries.iter().find(|e| e.id == "discord").unwrap();
    assert_eq!(discord.claude_ai_link, None);
}

/// `(golden file name, Rust's answer)` over one store, for the four entries of the slice.
fn answers(store: &Path) -> Vec<(&'static str, Value)> {
    let conn = open_read_only(store).unwrap();
    vec![
        (
            "mcp__connector_catalog__all.json",
            suggestions::catalog_view().unwrap(),
        ),
        ("mcp__connector_status__all.json", suggestions::status()),
        (
            "mcp__connector_suggestions__all.json",
            suggestions::suggestions(&conn).unwrap(),
        ),
        (
            "api__connectors__all.json",
            suggestions::connectors_list(&conn).unwrap(),
        ),
    ]
}

#[test]
fn the_four_goldens_of_each_oracle_equal_the_core_answers() {
    let (mut equal, mut total) = (0, 0);
    for (dir, store) in [
        ("golden", "store-v11.sqlite"),
        ("golden-rich", "store-rich/store-v11.sqlite"),
    ] {
        let manifest = read_json(&parity_dir().join(dir).join("manifest.json"));
        let listed: Vec<&str> = manifest["entries"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|e| e["file"].as_str())
            .filter(|f| f.contains("connector"))
            .collect();
        for (file, got) in answers(&parity_dir().join(store)) {
            assert!(
                listed.contains(&file),
                "{dir}: {file} is not in the manifest"
            );
            total += 1;
            let want = read_json(&parity_dir().join(dir).join(file));
            if got == want {
                equal += 1;
            } else {
                panic!("{dir}/{file} differs\n  got  {got}\n  want {want}");
            }
        }
        assert_eq!(listed.len(), 4, "{dir}: the slice has four goldens");
    }
    assert_eq!((equal, total), (8, 8));
    println!("goldens equal / total: {equal} / {total}");
}

fn entry_json(e: &Entry) -> Value {
    json!({
        "category": e.category, "provider": e.provider, "claude_ai_connector": e.claude_ai_connector,
        "claude_ai_link": e.claude_ai_link, "command": e.command,
        "fields": e.fields.iter().map(|f| json!({"env": f.env, "label": f.label, "secret": f.secret})).collect::<Vec<_>>(),
    })
}

#[test]
fn the_parser_answers_every_python_case_the_same() {
    let cases =
        read_json(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/connectors_cases.json"));
    let cases = cases["parse"].as_array().unwrap();
    assert_eq!(cases.len(), 495);
    let (mut ok, mut error, mut crash) = (0, 0, 0);
    for (n, case) in cases.iter().enumerate() {
        let (id, text) = (case["id"].as_str().unwrap(), case["text"].as_str().unwrap());
        let got = parse(id, text);
        let short: String = text.chars().take(80).collect();
        match (
            &case["want"]["ok"],
            &case["want"]["error"],
            &case["want"]["crash"],
        ) {
            (want, Value::Null, Value::Null) => {
                ok += 1;
                let entry =
                    got.unwrap_or_else(|e| panic!("case {n} {short:?}: Python accepts, got {e}"));
                assert_eq!(&entry_json(&entry), want, "case {n} {short:?}");
                assert_eq!(entry.id, id);
            }
            (Value::Null, want, Value::Null) => {
                error += 1;
                let got = got.expect_err(&format!("case {n} {short:?}: Python refuses"));
                assert_eq!(&Value::String(got.0), want, "case {n} {short:?}");
            }
            (Value::Null, Value::Null, class) => {
                // Python raised `ValueError` (shlex) and crashed: this build refuses with a fixed text
                crash += 1;
                assert_eq!(class, "ValueError", "case {n}");
                let got = got.expect_err(&format!("case {n} {short:?}: Python crashes"));
                assert_eq!(
                    got.0, "demo: command has an unclosed quote or a trailing backslash",
                    "case {n}"
                );
            }
            other => panic!("case {n}: odd expectation {other:?}"),
        }
    }
    assert_eq!((ok, error, crash), (129, 364, 2));
    println!(
        "parser cases equal / total: {} / {}",
        ok + error + crash,
        cases.len()
    );
}

#[test]
fn a_file_that_is_not_utf8_is_refused_where_python_crashes() {
    let good = fs::read(python_catalog_dir().join("slack.yaml")).unwrap();
    assert!(parse_bytes("slack", &good).is_ok());
    let mut bad = good.clone();
    bad.extend_from_slice(b"# \xff\xfe\n");
    let error = parse_bytes("slack", &bad).unwrap_err();
    assert_eq!(error.0, "slack: the catalog file is not valid UTF-8");
    // a BOM is kept as a character, so the first key is unknown (as in Python, where `read_text` keeps it)
    let mut bom = b"\xef\xbb\xbf".to_vec();
    bom.extend_from_slice(&good);
    let error = parse_bytes("slack", &bom).unwrap_err();
    assert!(
        error.0.starts_with("slack: unknown or repeated key '"),
        "{error}"
    );
}

#[test]
fn a_hostile_name_or_line_is_refused_cleanly() {
    let valid = "category: C\nprovider: P\nclaude_ai_connector: no\ncommand: npx -y p@1.0.0\nfield: A | L | plain\n";
    for id in [
        "",
        "../etc",
        "a/b",
        "A",
        "-a",
        "a b",
        "a\n",
        &"a".repeat(65),
        "é",
    ] {
        assert_eq!(
            parse(id, valid).unwrap_err().0,
            "invalid catalog entry name",
            "{id:?}"
        );
    }
    assert!(parse(&"a".repeat(64), valid).is_ok());
    // a very long line and a very long run of lines are only text
    let long = format!(
        "{valid}# {}\nnote: {}\n",
        "x".repeat(2_000_000),
        "y".repeat(10)
    );
    assert_eq!(
        parse("p", &long).unwrap_err().0,
        "p: unknown or repeated key 'note'"
    );
    let many = format!("{valid}{}", "# c\n".repeat(200_000));
    assert!(parse("p", &many).is_ok());
    // Python's `\d` also matches other Unicode decimal digits; here only ASCII digits pin a version
    let wide = valid.replace("p@1.0.0", "p@\u{ff11}.\u{ff10}.\u{ff10}");
    assert_eq!(
        parse("p", &wide).unwrap_err().0,
        "p: package 'p@\u{ff11}.\u{ff10}.\u{ff10}' must be pinned (name@1.2.3)"
    );
}

#[test]
fn the_new_modules_start_no_process_open_no_socket_and_name_no_keychain_crate() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let forbidden = [
        "std::process",
        "Command::new",
        "std::net",
        "TcpStream",
        "UdpSocket",
        "tokio",
        "reqwest",
        "hyper",
        "keyring",
        "dbus",
        "libc::",
    ];
    for file in ["catalog.rs", "suggestions.rs"] {
        let text = fs::read_to_string(src.join(file)).unwrap();
        for word in forbidden {
            assert!(!text.contains(word), "{file} mentions {word}");
        }
    }
}
