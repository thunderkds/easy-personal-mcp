# Plan: Rust replaces the Python app

**Date**: 2026-11-13
**Owner request**: "plan to remove the python in the app, let Rust do that. For the repo's config, keep."
**Status**: plan only. Nothing is removed. Each step below needs the owner's go-ahead before it starts.

## What goes and what stays

**Goes (in T063, last)**: the Python app: `src/easy_personal/`, the app tests under `tests/`, `pyproject.toml`, `uv.lock`.

**Stays**: the repo's config and the kit. This means `AGENTS.md`, `CLAUDE.md`, `agents/`, `skills/`, `templates/`, `tasks/`, `memory/`, `packs/`, `docs/`, `scripts/`, `.claude/`, the PRD and spec files, the boards, `web/` (the React bundle that Rust serves) and `site/`.
`scripts/check_static.py` names the app today. T063 changes that one file.

## What Rust still lacks (checked 2026-11-13)

| Python feature | Rust today | Task |
|---|---|---|
| `connector` list, add, remove, sync, move, alias, adopt-legacy; `doctor` | not built | T050, then T060 |
| `POST /api/connectors`, alias and move routes, `POST /api/sync` | not served | T050 |
| keychain, launcher, `claude` subprocesses | not built | T050 (owner gate: crates, licences) |
| `store-check` and `GET /api/store/health` (HUN-244, merged today) | not built | T059 |
| `ping`, `usage`, `outliers`, `kit-findings`, `workspace …`, `account …` as CLI commands | the data is served by the API and MCP; there is no CLI command | T058 |
| `dashboard` (ingest, then serve) | `serve` does the same since T051 | T058 (alias or a README line) |
| macOS and Windows | Ubuntu only | not planned; owner decides in T062 |

Already equal: the dashboard routes (read side), the MCP tools and prompt (stdio and HTTP), ingest of all sources, the live feed, the window app, the systemd timer.

## Tasks

| ID | Task | C / Risk | Needs the owner |
|---|---|---|---|
| T050 | Keychain, launcher, connector writes, sync apply, legacy | C3 / High | yes: crate choice and licences |
| T058 | Rust CLI commands for what Python's CLI has and Rust has not: `ping`, `usage`, `outliers`, `kit-findings`, `workspace list/create/edit/delete`, `account list/confirm/ignore/add/use`, `dashboard` | C2 / Medium | no |
| T059 | Rust `store-check` (with `--full`) and `GET /api/store/health`; same fields, exit codes 0, 1, 3, 130 | C2 / Medium | no |
| T060 | Rust `connector …` and `doctor` commands over T050's code | C3 / High | after T050 |
| T061 | Parity check: one run of every Python command and route against the Rust one on a copy of a store; list the differences | C2 / Medium | no |
| T062 | Cutover: README quick start for Rust, MCP registration and timer reinstall steps, then one week of use with Rust as the default | C1 / Medium | yes: go-ahead, and the macOS/Windows question |
| T063 | Retire Python: freeze the parity goldens, remove the Python app, change `scripts/check_static.py`, update docs and CI. One commit that `git` can undo | C2 / High | yes: go-ahead after the week |

## Order

1. **Now, no gate**: T059, T058 (they touch different files; parallel is fine).
2. **Owner gate**: T050, then T060.
3. T061 when T050, T058, T059 and T060 are merged.
4. T062, then one week.
5. T063 last.

## Risks

- **Goldens.** The Rust tests compare with Python output. T063 freezes the goldens (they stay in `rust/parity/`). The generators (`gen_*.py`) and the tests that run Python live (`--ignored`) cannot stay without the app. They move to an archive folder, or are dropped, on the owner's call.
- **Two writers.** Python and Rust share one store and one lock. Run them together only for the trial.
- **Keychain.** Secrets stay in the OS keychain. T050 must read the secrets Python already stored.
- **Rollback.** Before T063 the Python app still works. T063 is one commit, so a revert brings it back.
