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
**Decision**: README rewrite (HUN-56) and a static landing page in `site/` (HUN-57) deployed by Vercel's Git integration from `main` (`vercel.json` outputDirectory `site`, `.vercelignore` allowlist), Easy Kit dark neon theme (superseded 2026-10-29 by HUN-132: Easy Kit family Graphite Mint layout — header with family nav, section aside, hero + install box with one inline Copy script, drift-tested stat strip, self-hosted Plex in `site/fonts/`; owner chose "follow the family layout"), drift tests against README / `mcp.py` tools / catalog; screenshots from fixtures only. Site scope lives in `PROJECT_SPEC_SITE.md`; both tasks stay on the one Linear board — owner-approved exception to Hard-Stop Gate 4, same as easy-context-mcp.
**Why**: owner asked for the README and "the index page to deploy the guide" (2026-10-22); the sibling repos already ship this pattern.
**Approved by**: owner, 2026-10-22 ("Yes, as described").
**Files**: `README.md`, `site/`, `vercel.json`, `.vercelignore`, `PROJECT_SPEC_SITE.md`

### 2026-10-26 — Workspaces group connectors; the agent account picks its workspace
**Decision**: New feature direction (not yet planned). A **workspace** (created from an Add button) holds identity (name, color/icon, description, main email), a plain list of repo paths, and its own connectors from the existing catalog (Slack, Gmail, Teams, Discord, SendGrid), with keychain items keyed per workspace + connector. A workspace holds no agent settings. Agents are a separate area, viewable by provider (Claude, Codex) or by account (Claude personal, Claude company, Codex personal, …). Each **agent account** picks the workspace whose connectors are written into its config (for Claude, its own config dir, e.g. `~/.claude-personal`). Codex accounts are listed only and get no connectors, so DDR-0001 (Claude Code only) stands. Repo-folder binding and "every account gets every workspace" were rejected. New providers (GitHub, Linear/Jira, Google Calendar, Notion) are cut from the first version.
**Why**: Owner wants many workspaces (e.g. company vs personal) on one machine with one person; the agent-account link keeps company Claude away from personal tokens without tying workspaces to agents.
**Approved by**: owner, 2026-10-26 ("agent picks workspace, codex listed only for now").
**Grilling 2026-10-26**: auto-detect accounts + owner confirms; several workspaces per Claude account, connectors `<workspace>-<provider>`; deleted workspace → connectors to built-in `orphan` (names kept there; renamed on assign out); move = rename; pre-workspace connectors → orphan; dashboard + CLI; usage filter by workspace repos and by account; ingest every confirmed Claude account; DB best practice required (NFR-009: versioned migrations, FKs, constraints, indexed time-first queries). See PRD FR-028..FR-036.
**Files**: `PRD.md`, `PROJECT_SPEC.md` glossary — Stage 0.5 brainstorming → Stage 2 pending

### 2026-10-31 — Work view inspired by Javis OS, read-only
**Decision**: New feature set "Work view" (not yet planned), ideas borrowed from github.com/blogminhquy/javis-os (inspiration only, no code). Read-only cross-project board from each kit repo's `PROJECT_KANBAN.md` (the Linear side comes from the kit's own pull into that file; easy-personal never calls the board API); you-vs-agent usage split (agent = sidechain or non-`cli` entrypoint; new `entrypoint` column, backfilled by the next ingest); agent-related systemd timers + cron (first word only); live memory graph of names + `[[links]]` from `<repo>/memory/` and `~/.claude*/projects/*/memory/`, never stored, no bodies; grouped rail + first-run checklist with no new writes. Rejected: Javis-style board that runs agents (breaks the read-only vision), live Linear API, storing the graph.
**Why**: owner liked Javis OS's work management, data display and board UX, but "core vision still go with ours".
**Approved by**: owner, 2026-10-31 (6 grill answers + 2 flag answers).
**Files**: `PRD.md` US-017..US-021, FR-037..FR-042, NFR-010, OQ 12–14, `BRAINSTORMING_LOG_WORKVIEW.md` — owner chose Option A (live-read modules, rings graph layout, no new dependency) on 2026-10-31 → Stage 2 pending
**Terminology grill 2026-10-31**: new glossary terms in `PROJECT_SPEC.md` (Work view, Board task, Last board pull, Typed work / Agent work, Entrypoint, Scheduled job, Memory note, Dangling link, First-run checklist); named exception to the metadata-only constraint (show-never-store task titles + note `name`/`description`); MCP surface: split fields added to `usage_summary`/`usage_breakdown` only, owner-approved. DDR gate: < 2 of 3 criteria → no DDR.
**Stage 2 2026-10-31**: 5 tasks on Linear — HUN-160 rail + first-run (P0), HUN-161 usage split + migration (P0, Medium), HUN-162 board (P0), HUN-163 jobs (P1, Medium), HUN-164 memory graph (P1, Medium); 162–164 depend on HUN-160. Report: `reports/thinking-report_main_20261031T144639.html`.
**Split rule revised 2026-10-31 (HUN-161 migration dry run)**: real transcripts carry `cli`, `sdk-cli` and `claude-desktop`; owner chose *agent = sidechain or entrypoint starting with `sdk`*, everything else is typed work (was: only `cli` typed). PRD FR-039 and PROJECT_SPEC glossary updated.

## 2026-10-31 — Live trace spike runs inside easy-personal-mcp, metadata only

**Decision**: The owner's "Claude Live Trace — Experimental Plan" (claude.ai doc) runs as a spike in `experiments/live-trace/` (HUN-165..170), not a fresh repo: the repo already has the transcript reader, a PostToolUse trace hook, the metadata-only rule and the dashboard. FR-002 holds for the spike, so the plan's `--capture-content` flag is dropped. Plan items below Phase 0 are hypotheses to measure, not requirements. E5 (Admin usage API) is docs-only: no Admin key exists and the owner said not to test a service without its key. The prototype (collector + live dashboard) waits for the owner's review after HUN-170.
**Files**: `tasks/TASK_GUIDE_HUN-165.md`..`HUN-170.md`, `PROJECT_SPEC.md` Tasks rows.

## 2026-10-31 — v0.3.0 live view = transcript tail only (Option A)

**Decision**: The live view for v0.3.0 reads only Claude Code transcripts: a ~100 ms tail runs inside `easy-personal dashboard`, writes through the normal store path, and streams to a new Live view over `GET /api/live` (SSE). No hooks and no OpenTelemetry, so nothing is written to any Claude config. Why: the spike (HUN-165..170) showed OTel tokens equal the transcript's to 0.000% and the product already prices tokens; the tail is under 2 s for 95.7% of tool calls with zero setup; hooks/OTel need settings writes outside DDR-0001 and a slow collector blocks sessions (`async` is ignored on http hooks). Rejected: B (tail + hooks + OTel), C (tail + hooks). Hooks stay a possible later opt-in. Also in scope: FR-046 (8 record types new in 2.1.316).
**Files**: `BRAINSTORMING_LOG_LIVE.md`, `PRD.md` US-022, FR-043..046, NFR-011

## 2026-11-02 — v0.4.0 live attention states = snapshot-pushed state (Option C)

**Decision**: The Live view marks a session *failing* (a stream's last 3 tool results failed), *waiting for you* (open `AskUserQuestion` / `ExitPlanMode`) or *needs attention* (another open call ≥ 10 min while the whole session wrote nothing). All states are worked out in `core/live.py`; the stream re-sends `snapshot` only when a session's set of states changes; the page only renders a badge. Idea source: github.com/agentblazo/blazo (ideas only). Rejected after measuring the owner's history (1,101 files): Blazo's same-tool-N-times and tool-loop rules (noise); page-derived state (lost on reload); state on every event (needs a second path for timer states). Performance review moved the 10-min rule from the page into the tail so ordinary tool calls never cause a re-send (owner peak 78 calls/min). No new dependency; nothing new stored.
**Approved by**: owner, 2026-11-02 (4 grill answers + 2 flag answers + badge mockup + quiet rule; "let go" with Option C).
**Files**: `PRD.md` US-023, FR-044 (amended), FR-047..FR-050, NFR-012, OQ 15–17; `BRAINSTORMING_LOG_ATTENTION.md`; `PROJECT_SPEC.md` glossary (Live tail, Stream, Open call, Attention state). DDR gate: < 2 of 3 criteria → no DDR.
**Stage 2 2026-11-02**: 3 tasks on Linear — HUN-181 live tail attention + stream re-send (P0, C2, Medium → `security-review`), HUN-182 Live view badge (P0, C2, Low, UI Evidence rows), HUN-183 release 0.4.0 (P1, C1, Low); chain 181 → 182 → 183.
