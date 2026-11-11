//! Ingest parse layer (P2a, T028): the bytes of one Claude Code transcript to the [`crate::writer::Line`] values the
//! Python `claude_code` source would store. Pure functions: no file, folder, git, store or clock.
//!
//! - [`claude_code`] - line framing and the record dispatch (the entry point, `claude_code::read_lines`)
//! - [`json`] - `json.loads` on bytes, as CPython does it (encodings, `NaN`, big integers, a depth limit)
//! - [`time`] - `datetime.fromisoformat` (the C one) to the stored UTC text
//! - [`label`] - `_LABEL` (with Python's `$` quirk) and `_WORD`
//! - [`secret`] - `secretlike.SECRET`
//! - [`signals`] - the Bash first word and the WebFetch host
//!
//! P2b (T033) adds the three kit sources over real folders: [`kit_trace`], [`ui`], [`verifier`] (and [`config`], the
//! report-folder settings they share).
//!
//! The helper modules are public so the differential corpora (`parity/ingest/golden/corpus-*.json`) can test them.

pub mod accounts;
pub mod claude_code;
pub mod config;
pub mod folders;
pub mod json;
pub mod kit_trace;
pub mod label;
pub mod secret;
pub mod signals;
pub mod time;
pub mod ui;
pub mod verifier;
