# BRAINSTORMING_LOG — Board Run (T131) and Agents add/remove (T132)
**Date**: 2026-11-20 · **Tier**: Standard · **Board**: `rust/PROJECT_KANBAN.md`

## Owner request (verbatim)
"from the board section, let add the button the can execute the task from board, from the agents, let suppoert feature to remove tha account and adding the account"

## Owner decisions (2026-11-20, AskUserQuestion)
| Question | Answer |
|---|---|
| Run button | **Start immediately** (no form) |
| Remove account | **From the app only**: folder and past usage kept, never re-detected, default `~/.claude` refused |
| Add account | **Create a new account**: app makes `~/.claude-<name>` and shows how to sign in |
| Design | **Follow existing styles**; screenshots at review |

## Verified facts (claim gate)
- Board entries carry `project` = folder basename only (`rust/crates/core/src/board.rs:417`); projects come from usage (`scoped_projects`, `board.rs:459`), not from workspaces.
- `/api/board` is a byte-exact parity golden (`rust/parity/gen_golden.py:334`, `:1160`): any new field means regenerating goldens while Python still exists (T063 not done).
- `POST /api/app-sessions` needs `repo` + `workspace` where the repo is a stored repo of that workspace (`stored_repo`, `rust/crates/app/src/app_session_routes.rs:271`), 403 when app sessions are off (`:468`), `account` may be null.
- `NewSession.jsx:5` states "Nothing starts without the confirm step" (FR-109). The owner's "start immediately" is a deliberate exception for the board button only.
- `agent_accounts.status` CHECK IN ('detected','confirmed','ignored') (`schema_v11.sql:40`); `events`, `account_workspaces`, `account_registrations` reference the row; `detect()` uses `INSERT OR IGNORE` (`ingest/accounts.rs:40`), so **a kept row is never re-detected**, while a deleted row comes back on the next ingest.
- `ignore` already refuses the default account (`accounts.rs:490`); sync treats an ignored account as "remove its registrations" (`sync.rs:374-380`).
- `accounts::add` needs `projects/`, `settings.json` or `config.toml` in the folder (`accounts.rs:540`); `POST /api/accounts` exists; the Agents page has no add or remove UI (`web/src/AgentsView.jsx`).

## T131 — Board Run: three paths
| Path | What | Invasiveness | Why it might fail |
|---|---|---|---|
| **A. Page-only match (recommended)** | `BoardView` loads `/api/workspaces` (+ `/api/agent-run`); a card's project matches the workspace repo whose basename equals it; Run POSTs `{repo, workspace, account: null, model: "default", effort: default, mode: "default", worktree: false, prompt}` and opens Live on the new id | Low: 1–2 web files + tests | Two repos with the same basename → ambiguous: button disabled with the reason. A project in no workspace → disabled "add this repo to a workspace". |
| B. Board API adds `path`/`workspace` | Server returns the full folder and its workspace | Medium: core + golden regen + Python parity | Breaks the byte-exact golden; touches a parity hotspot for a UI feature. |
| C. Run opens the prefilled form | Reuse `NewSessionSheet` | Low | Owner rejected it (start immediately). |

**50% less code**: Path A reuses `sendJson` and the existing Live navigation (`onStarted(id)` pattern from `NewSessionSheet`); no new component beyond a button and its reason text.

**Edge cases (T131)**: app sessions off (403) → toast with the server reason + Settings hint; Claude not found; double click → one session (busy state); Run hidden on Done / Closed columns; `mode` is always `default`, never bypass (a one-click start must never skip permission prompts); title is plain text in the prompt and capped (prompt ≤ 100 000 bytes); a task id that repeats across projects stays tied to its own card's project; 409/other errors shown, nothing retried.

**Prompt** (fixed text): `Work on task <ID>: <plain title>. Follow this project's workflow; its guide is tasks/TASK_GUIDE_<ID>.md if that file exists.`

## T132 — Agents add (create) / remove: three paths
| Path | Remove | Create | Invasiveness | Why it might fail |
|---|---|---|---|---|
| **A. No schema (recommended)** | `Remove` = the existing `ignore` (kept row → never re-detected, sync drops its registrations); the page hides ignored accounts in a collapsed "Removed (N)" section with "Use this account" to bring one back | New route `POST /api/accounts/create {name}`: name `[a-z0-9][a-z0-9-]{0,29}`, makes `<agent root>/.claude-<name>` (0700, `create_dir`, refuses an existing path or link), writes `settings.json` `{}` (0600, create-new), then `accounts::add`; page shows `CLAUDE_CONFIG_DIR=~/.claude-<name> claude` to sign in, then `/login` | Medium: 1 core fn + 1 route + page | "Ignore" and "Remove" become one action: the page renames the button to Remove for confirmed accounts (detected keep Ignore). |
| B. New `removed` status | Schema migration: rebuild `agent_accounts` with a new CHECK value | same | **High**: SQLite table rebuild under FKs, parity schema golden, every `status` reader | Large blast radius for one hidden flag. |
| C. Side file of removed ids | `removed.json` in the app folder; list filters it | same | Medium | Two sources of truth for one account's state; sync/agent-run do not see it. |

**50% less code (create)**: the page only shows the sign-in command and relies on `detect()` after Claude makes the folder. Rejected: the owner asked the app to create it, and Claude may not write `settings.json`/`projects/` until first use, so `detect()` could miss it.

**Edge cases (T132)**: name collides with an existing folder or a dangling link → 409, nothing written; `..`, `/`, upper case, empty → 400 before any filesystem work; agent root not writable → error, no row; the half-made folder is removed if `settings.json` fails; create holds the write lock only for the store step (the `prepare_add` rule); removing the agent-run account turns the run off (T099 rule, check it holds through `ignore`); remove of the default account refused (server already does it); the new account starts `detected` → page opens its Name field (existing flow); route under the same token/origin guard and leak gate as `POST /api/accounts`; parity: new route is Rust-only, listed in the parity "extra" set.

## Surgical scope
| Task | Should touch | Must NOT touch |
|---|---|---|
| T131 | `web/src/BoardView.jsx`, `web/src/BoardView.test.jsx`, `web/src/app.css` (button only), `web/src/App.jsx` (pass `onStarted` to Live) | `rust/**`, parity goldens, `NewSession.jsx` confirm rule |
| T132 | `rust/crates/core/src/accounts.rs` (`create`), `rust/crates/app/src/writes.rs` + `api.rs` (route), their tests, `web/src/AgentsView.jsx` + test, README accounts part, parity extra list | `schema_v11.sql`, `ingest/accounts.rs` detect rules, `sync.rs`, keychain, the user's real `~/.claude*` in tests (temp agent root only) |

## Recommended path
**T131 Path A** (page-only, Medium) and **T132 Path A** (no schema: Remove = ignore + "Removed" section; Create = new route; High because it writes into the home folder). T131 and T132 are independent and can run in parallel worktrees.

## Next actions for Stage 2
1. Owner picks the paths (one question).
2. Write `tasks/TASK_GUIDE_T131.md`, `tasks/TASK_GUIDE_T132.md` with the edge cases above as ACs; add both to `rust/PROJECT_KANBAN.md` Todo.
3. Spawn `frontend-developer` for T131, `backend-developer` then `frontend-developer` for T132; Stage 4 code-review both, security-review T132 (and T131: one-click process start).
