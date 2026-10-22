# decisions.md — Cold Tier: Architectural & Infrastructure Decisions

> **Rules**: Supervisor-only writes. Each entry: `### YYYY-MM-DD — Title`, then **Decision**, **Why**, and **Files** (cite paths — the diff-driven pass greps this file by changed file path).

## Architecture

### 2026-10-21 — v1 = Python core + React/Vite page, on demand
**Decision**: One Python package (sources/core/surfaces/connectors, isolated `launch.py`) plus a `web/` React + Vite SPA over a JSON API; `easy-personal dashboard` serves it on `127.0.0.1` only while running; a systemd user timer runs ingest. → see BRAINSTORMING_LOG.md (Option B)
**Why**: Owner chose B over the server-rendered Option A for a richer page; on-demand avoids an always-on process.
**Files**: `src/easy_personal/`, `web/`, `src/easy_personal/dashboard/static/`

### 2026-10-21 — Read-only MCP client of easy-context
**Decision**: The `context` source calls only `list_indexes`/`get_stats` on `127.0.0.1:8767`; never `index_path`; "no data" on failure; NFR-001 relaxed to "no traffic off the machine". → see DDR-0002
**Why**: Public tool contract over easy-context's private SQLite schema; read-only kit promise stays testable.
**Implemented (HUN-41, 2026-10-22)**: `sources/context.call_tool` refuses any tool outside `ALLOWED_TOOLS` and any URL whose host is not exactly `127.0.0.1` before a network call; URL from `EASY_PERSONAL_CONTEXT_URL`; one `list_indexes` call per ingest (it lists every index); `last_index_at` stored in table `context_index`; any failure clears the table → "no data".
**Files**: `src/easy_personal/sources/context.py`, `docs/ddr/0002-read-only-mcp-client-of-easy-context.md`

### 2026-10-21 — Native runtime; connector writes only via `claude mcp` user scope
**Decision**: Native `uv tool install`; writes limited to `~/.easy-personal/`, the keychain and `claude mcp add-json|remove --scope user`; Codex/Cursor writers dropped. → see DDR-0001
**Why**: The launcher must run on the host and reach the keychain; `~/.claude.json` holds sign-in and trust.
**Files**: `src/easy_personal/launch.py`, `src/easy_personal/connectors/`, `docs/ddr/0001-native-runtime-and-write-targets.md`

## Infrastructure

### 2026-10-21 — Board scoped to Linear project easy-personal-mcp (P-HUN-3)
**Decision**: `.claude/board.json` `project` = `2a59e842-7dd5-483b-9bdb-f2287019e3f3`; the trashed first project (HUN-21…29) does not carry over.
**Why**: An empty `project` made `board_sync.py pull` fetch the whole HUN team, mixing in easy-context-mcp tasks (one-project-per-board rule).
**Files**: `.claude/board.json`, `PROJECT_KANBAN.md`, `tasks/`

### 2026-10-21 — Stage 3–5 git flow: local merge to main, push in batches
**Decision**: After a task passes code-review (+ security-review when Medium/High) and verify, the Supervisor merges its branch into local `main` (`--no-ff`) without a PR and pushes `main` later in batches when the owner asks. The board skips "Ready for Review": tasks move In Progress → Done. Supersedes Phase 0 D "branch + PR per task".
**Why**: Owner, 2026-10-21: "you can merge to main and automate, push later". The Linear team HUN has no review-type status.
**Files**: `.claude/board.json`, `PROJECT_KANBAN.md`
