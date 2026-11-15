//! T062 guard: `rust/docs/CUTOVER.md` stays true. Every `easy-personal-rs <subcommand ...>` the document shows (in a fenced
//! block or an inline code span) names a subcommand path the built binary knows: each word of the path answers
//! `<path> --help` with exit 0. Offline; every run gets an empty `HOME` and `EASY_PERSONAL_HOME` in a throwaway folder, and
//! `--help` opens no store, keychain or `claude`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_easy-personal-rs");
const NAME: &str = "easy-personal-rs";

fn doc() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/CUTOVER.md");
    std::fs::read_to_string(&path).expect("rust/docs/CUTOVER.md must exist")
}

/// One folder per call: the tests run in parallel and each removes its own folder at the end.
fn scratch() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("cutover-doc-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The text a reader would type: the lines of fenced blocks and the inline code spans of the prose.
fn command_texts(doc: &str) -> Vec<String> {
    let mut texts = Vec::new();
    let mut fenced = false;
    for line in doc.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        } else if fenced {
            texts.push(line.to_string());
        } else {
            texts.extend(line.split('`').skip(1).step_by(2).map(str::to_string));
        }
    }
    texts
}

/// Every run of words after `easy-personal-rs` (alone, or at the end of `.../bin/easy-personal-rs` or `.../debug/easy-personal-rs`) up to the first
/// token that is not a plain lowercase word: `["connector", "sync"]` for `easy-personal-rs connector sync --yes`.
fn word_runs(text: &str) -> Vec<Vec<String>> {
    let mut runs = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find(NAME) {
        let before = &rest[..at];
        let after = &rest[at + NAME.len()..];
        let own = before.is_empty()
            || before.ends_with(|c: char| c.is_whitespace() || c == '=')
            || before.ends_with("/bin/")
            || before.ends_with("/debug/");
        if own && (after.is_empty() || after.starts_with(' ')) {
            let words: Vec<String> = after
                .split_whitespace()
                .take_while(|t| {
                    t.starts_with(|c: char| c.is_ascii_lowercase())
                        && t.chars()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                })
                .map(str::to_string)
                .collect();
            runs.push(words);
        }
        rest = after;
    }
    runs
}

struct Help {
    ok: bool,
    text: String,
}

fn help(home: &Path, path: &[String]) -> Help {
    let out = Command::new(BIN)
        .args(path)
        .arg("--help")
        .env_clear()
        .env("HOME", home)
        .env("EASY_PERSONAL_HOME", home.join("data"))
        .env("PATH", "/usr/bin:/bin")
        .current_dir(home)
        .output()
        .expect("the binary runs");
    Help {
        ok: out.status.success(),
        text: String::from_utf8_lossy(&out.stdout).into_owned(),
    }
}

/// Walks each run word by word. A word is a subcommand when its parent lists `Commands:`; then `<path> <word> --help`
/// must exit 0 (this also covers aliases such as `dashboard`). A parent with no `Commands:` takes the rest as arguments.
/// Returns one message per path that does not exist.
fn problems(doc: &str) -> Vec<String> {
    let home = scratch();
    let mut found = Vec::new();
    for text in command_texts(doc) {
        for run in word_runs(&text) {
            let mut path: Vec<String> = Vec::new();
            for word in &run {
                let parent = help(&home, &path);
                if !parent.ok {
                    found.push(format!("`{NAME} {}` has no help", path.join(" ")));
                    break;
                }
                if !parent.text.lines().any(|l| l.trim_end() == "Commands:") {
                    break;
                }
                path.push(word.clone());
                if !help(&home, &path).ok {
                    found.push(format!("`{NAME} {}`: no such subcommand", path.join(" ")));
                    break;
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(&home);
    found.sort();
    found.dedup();
    found
}

#[test]
fn every_command_in_the_cutover_document_exists() {
    let doc = doc();
    let bad = problems(&doc);
    assert!(
        bad.is_empty(),
        "the document names commands the binary lacks: {bad:#?}"
    );
}

/// The guard is not vacuous: the document shows the commands the steps run.
#[test]
fn the_document_shows_the_commands_of_every_step() {
    let doc = doc();
    let runs: Vec<String> = command_texts(&doc)
        .iter()
        .flat_map(|t| word_runs(t))
        .map(|run| run.join(" "))
        .collect();
    for want in [
        "store-check",
        "mcp",
        "schedule install",
        "connector sync",
        "doctor",
        "dashboard",
        "app",
    ] {
        assert!(
            runs.iter().any(|r| r == want),
            "no `{NAME} {want}` in the document"
        );
    }
}

#[test]
fn a_made_up_subcommand_is_caught() {
    let fenced = format!("```bash\n{NAME} frobnicate\n```\n");
    assert_eq!(
        problems(&fenced),
        vec![format!("`{NAME} frobnicate`: no such subcommand")]
    );
    let inline = format!("Run `{NAME} connector frobnicate` now.\n");
    assert_eq!(
        problems(&inline),
        vec![format!("`{NAME} connector frobnicate`: no such subcommand")]
    );
}

#[test]
fn real_commands_arguments_and_paths_pass() {
    let text = format!(
        "```bash\n{NAME} schedule install --dry-run\nclaude mcp add --scope user easy-personal -- ~/.local/bin/{NAME} mcp\n\
         X=1 {NAME} connector alias c1a2b3c4 team-chat\n{NAME} --version\n```\nand `{NAME} dashboard --static-dir ~/s`; the folder ~/.local/share/{NAME}/static\n"
    );
    assert!(problems(&text).is_empty(), "{:?}", problems(&text));
    let runs: Vec<Vec<String>> = command_texts(&text)
        .iter()
        .flat_map(|t| word_runs(t))
        .collect();
    assert!(runs.contains(&vec!["mcp".to_string()]));
    assert!(!runs.iter().any(|r| r.iter().any(|w| w == "static")));
}

/// Since T078 `keychain.rs` is the `[lib]` target, so `--bin` lists no test and the gate would run zero tests. The command
/// in the document must name `--lib`, the package and the test, and that test must still exist in `keychain.rs`.
#[test]
fn the_keychain_gate_command_runs_the_lib_test() {
    const TEST: &str = "real_keychain_items_cross_check";
    let doc = doc();
    let lines: Vec<&str> = doc.lines().filter(|l| l.contains("cargo test")).collect();
    let gate: Vec<&&str> = lines.iter().filter(|l| l.contains(TEST)).collect();
    assert_eq!(
        gate.len(),
        1,
        "one `cargo test ... {TEST}` command expected: {gate:?}"
    );
    for want in ["-p easy-personal-rs", "--lib", "-- --ignored"] {
        assert!(
            gate[0].contains(want),
            "the keychain command lacks `{want}`: {}",
            gate[0]
        );
    }
    assert!(
        !gate[0].contains("--bin"),
        "a `--bin` filter lists no keychain test"
    );
    let source =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/keychain.rs"))
            .unwrap();
    assert!(
        source.contains(&format!("fn {TEST}")),
        "the test is gone from keychain.rs"
    );
    assert!(
        source.contains(&format!("--lib {TEST}")),
        "the doc comment in keychain.rs names a wrong command"
    );
}
