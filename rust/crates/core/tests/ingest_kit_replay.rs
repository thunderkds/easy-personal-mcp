//! T033: the Rust ingest of `kit_trace`, `ui` and `verifier` replays the Python oracle (`rust/parity/gen_ingest_kit.py`,
//! goldens in `rust/parity/ingest/golden-kit/`). Each test plays one scenario's steps and, after every `ingest` step,
//! the printed counts and the four tables must equal the golden's. No Python is needed: the goldens are committed.

mod ingest_kit_common;

use ingest_kit_common::replay;

#[test]
fn k01_kit_trace_lines() {
    replay("K01_kit_trace_lines.json");
}

#[test]
fn k02_kit_trace_inode_shrunk_deleted() {
    replay("K02_kit_trace_inode_shrunk_deleted.json");
}

#[test]
fn k03_kit_trace_projects_gone() {
    replay("K03_kit_trace_projects_gone.json");
}

#[test]
fn k04_kit_trace_symlinks() {
    replay("K04_kit_trace_symlinks.json");
}

/// Python crashes on the folder named `x.jsonl` (and on every later run); Rust skips it and reads `z9.jsonl` too.
#[test]
fn k04c_a_folder_named_jsonl_is_skipped_not_a_crash() {
    replay("K04c_crash_directory_named_jsonl.json");
}

#[test]
fn k05_ui_files() {
    replay("K05_ui_files.json");
}

#[test]
fn k06_ui_folder_rules() {
    replay("K06_ui_folder_rules.json");
}

#[test]
fn k07_verifier_reports() {
    replay("K07_verifier_reports.json");
}

#[test]
fn k08_verifier_container() {
    replay("K08_verifier_container.json");
}

#[test]
fn k09_empty_store() {
    replay("K09_empty_store.json");
}
