# SPEC — Rust port (requirements stand-in)

This project ports `easy-personal-mcp` to Rust. The behaviour contract is the Python project's `PRD.md` (94 FRs, 15 NFRs), which is the source of truth until the Python sunset (repo-root `PRD.md`). This file records only what is **new** for the Rust port; nothing here relaxes a Python requirement.

## Goal (owner, 2026-11-10)
"I need an application version with RUST. Or if possible we can convert python to rust also … note the plan to replace the python in the future." Priority: Ubuntu, then macOS, Windows after. Form: a desktop window (GUI); the CLI and MCP server live in the same binary.

## New requirements
| ID | Requirement |
|----|-------------|
| RS-001 | The app exposes its version in four places: `--version` on the CLI, the MCP server info, an `/api` field, and the window title and About. The one source is the Cargo package version. |
| RS-002 | The app opens `~/.easy-personal/store.sqlite` (override as in Python) and refuses a store whose `PRAGMA user_version` is newer than it knows, with a clear message, writing nothing. |
| RS-003 | Every `/api/*` route and MCP tool the Rust app serves returns the same JSON as Python on the shared fixtures (parity), before the surface flips. |
| RS-004 | One implementation writes the store at a time; the other reads or waits. |
| RS-005 | The window loads the dashboard from the in-process `127.0.0.1` server with the existing token guard; closing it stops the server and frees the lock; a second launch focuses the first. |
| RS-006 | The CLI and MCP stdio paths work with no display (headless / SSH). |
| RS-007 | Ubuntu is the first supported OS, macOS second, Windows last. Each OS has its own scheduler and keychain adapter. |
| RS-008 | Python sunset: per-surface flip → maintenance-only → one release of deprecation → removal; its tests are archived as the parity corpus. |

## Out of scope
Porting the React dashboard; changing any Python behaviour; Windows work before P5; signing and notarization (deferred).
