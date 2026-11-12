//! T052: `save_report` checked the store read-only first and promises that a missing store is never created; its write
//! handle therefore must not be able to create one either (a store removed between the check and the write). The handle
//! `open_existing_for_write` opens without `SQLITE_OPEN_CREATE`; `open_for_write` (ingest) still creates.

use easy_personal_core::writer::{WriteError, open_existing_for_write, open_for_write};
use std::fs;
use std::path::PathBuf;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "easy-personal-rs-t052-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_missing_store_is_refused_and_not_created() {
    let dir = scratch("nocreate");
    let path = dir.join("store.sqlite");
    let Err(error) = open_existing_for_write(&path) else {
        panic!("a missing store must be refused");
    };
    assert!(matches!(error, WriteError::Io(_)), "{error:?}");
    assert!(!path.exists(), "the file was created");
    assert!(
        fs::read_dir(&dir).unwrap().next().is_none(),
        "something was created beside it"
    );
    // the ingest handle still creates the store
    drop(open_for_write(&path).unwrap());
    assert!(path.is_file());
    // and an existing store opens for the report write
    drop(open_existing_for_write(&path).unwrap());
    let _ = fs::remove_dir_all(&dir);
}
