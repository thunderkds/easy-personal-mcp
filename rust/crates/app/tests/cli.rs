//! Runs the real binary against temp stores. `HOME` and `EASY_PERSONAL_HOME` both point into a temp
//! folder, so the owner's `~/.easy-personal` is never reached.

use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t002-cli-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(home: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
        .args(args)
        .env("HOME", &home.0)
        .env("EASY_PERSONAL_HOME", &home.0)
        .output()
        .unwrap()
}

fn make_store(home: &TempDir, user_version: i64) {
    let conn = Connection::open(home.0.join("store.sqlite")).unwrap();
    conn.execute_batch(&format!(
        "CREATE TABLE events (id INTEGER PRIMARY KEY, note TEXT);
         INSERT INTO events (note) VALUES ('secret-a'), ('secret-b'), ('secret-c');
         PRAGMA user_version = {user_version};"
    ))
    .unwrap();
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn version_prints_name_and_cargo_version() {
    let home = TempDir::new();
    let out = run(&home, &["--version"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        stdout(&out).trim(),
        format!("easy-personal-rs {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn store_info_json_on_a_version_11_store() {
    let home = TempDir::new();
    make_store(&home, 11);
    let out = run(&home, &["store-info", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(
        stdout(&out).trim(),
        r#"{"file":"store.sqlite","exists":true,"user_version":11,"known":11,"pending":false,"tables":{"events":3}}"#
    );
    assert!(!stdout(&out).contains(home.0.to_str().unwrap()));
}

#[test]
fn store_info_text_on_a_version_11_store() {
    let home = TempDir::new();
    make_store(&home, 11);
    let out = run(&home, &["store-info"]);
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    assert!(
        text.contains("user_version: 11") && text.contains("events: 3"),
        "{text}"
    );
    assert!(!text.contains("secret"));
}

#[test]
fn newer_store_exits_2_names_both_numbers_and_writes_nothing() {
    let home = TempDir::new();
    make_store(&home, 12);
    let before = fs::read(home.0.join("store.sqlite")).unwrap();
    let out = run(&home, &["store-info", "--json"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stdout(&out).is_empty());
    let msg = stderr(&out);
    assert!(msg.contains("12") && msg.contains("11"), "{msg}");
    assert_eq!(fs::read(home.0.join("store.sqlite")).unwrap(), before);
}

#[test]
fn missing_store_is_no_data_exit_0_and_creates_nothing() {
    let home = TempDir::new();
    let out = run(&home, &["store-info"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout(&out).contains("no data"));
    assert_eq!(fs::read_dir(&home.0).unwrap().count(), 0, "no file created");
}

#[test]
fn directory_in_place_of_the_store_exits_1() {
    let home = TempDir::new();
    fs::create_dir(home.0.join("store.sqlite")).unwrap();
    let out = run(&home, &["store-info"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(!stderr(&out).contains(home.0.to_str().unwrap()));
}
