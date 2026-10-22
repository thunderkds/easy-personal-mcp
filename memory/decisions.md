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

### 2026-10-22 — Connectors register `easy-personal-launch exec <name>`
**Decision**: The command written by `claude mcp add-json` (and printed in the snippet fallback) is `easy-personal-launch exec <connector>` / `headers <connector>`, not `easy-personal exec`. PRD FR-018/FR-019, PROJECT_SPEC glossary, DDR-0001 and the HUN-45 guide updated.
**Why**: Owner, 2026-10-22 ("yes"). HUN-44 measured `easy_personal.surfaces.cli` import at ~300 ms vs. `easy_personal.launch` at ~50 ms; NFR-006 caps launcher start-up at 200 ms.
**Files**: `src/easy_personal/launch.py`, `pyproject.toml`, `PRD.md`, `docs/ddr/0001-native-runtime-and-write-targets.md`, `tasks/TASK_GUIDE_HUN-45.md`

### 2026-10-22 — Catalog servers are pinned; Slack = `slack-mcp-server@1.3.0`
**Decision**: Every catalog `command` that runs `npx`, `uvx` or `pipx run` must name an exact version (`name@1.2.3`); `connectors/catalog.parse` refuses unpinned, `@latest`, ranges and short versions. The Slack entry runs `npx -y slack-mcp-server@1.3.0 --transport stdio` (korotovsky, MIT) with one secret field `SLACK_MCP_XOXB_TOKEN` (bot token: invited channels only, no search).
**Why**: Owner, 2026-10-22 (accepted the recommendation). The first pick, `@modelcontextprotocol/server-slack`, is deprecated on npm and `npx -y` without a version would run whatever is newest, with the token in its env.
**Files**: `src/easy_personal/catalog/slack.yaml`, `src/easy_personal/connectors/catalog.py`


### 2026-10-22 — HUN-46 catalog servers: Gmail, SendGrid, Teams, Discord

**Decision**: Pinned community servers (no provider publishes its own npm MCP server): Gmail `@gongrzhe/server-gmail-autoauth-mcp@1.1.11` (claude.ai Gmail connector listed first), SendGrid `sendgrid-mcp@1.4.0`, Teams `@softeria/ms-365-mcp-server@0.170.2 --org-mode` (claude.ai Microsoft 365 connector listed first), Discord `@pasympa/discord-mcp@2.2.0`. New catalog key `claude_ai_link` (required when `claude_ai_connector: yes`), value `https://claude.ai/settings/connectors` for now.
**Why**: Checked on the npm registry 2026-10-22: none deprecated, MIT/ISC, most-used maintained option per provider. Rejected `@chrischall/microsoft-teams-mcp` (captured Teams web tokens, not the official login) and `mcp-discord@1.3.4` (no release since 2025-07). Gmail package has no release since 2025-08, accepted because the claude.ai connector is offered first.
**Approved by**: owner, 2026-10-22 ("continue your work" after the proposal).
**Files**: `src/easy_personal/catalog/*.yaml`, `src/easy_personal/connectors/catalog.py`

### 2026-10-22 — Landing site: sibling pattern, one board
**Decision**: README rewrite (HUN-56) and a static landing page in `site/` (HUN-57) deployed by Vercel's Git integration from `main` (`vercel.json` outputDirectory `site`, `.vercelignore` allowlist), Easy Kit dark neon theme, drift tests against README / `mcp.py` tools / catalog; screenshots from fixtures only. Site scope lives in `PROJECT_SPEC_SITE.md`; both tasks stay on the one Linear board — owner-approved exception to Hard-Stop Gate 4, same as easy-context-mcp.
**Why**: owner asked for the README and "the index page to deploy the guide" (2026-10-22); the sibling repos already ship this pattern.
**Approved by**: owner, 2026-10-22 ("Yes, as described").
**Files**: `README.md`, `site/`, `vercel.json`, `.vercelignore`, `PROJECT_SPEC_SITE.md`
