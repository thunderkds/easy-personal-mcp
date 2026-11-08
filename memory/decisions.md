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

### 2026-11-03 — Memory on vs off block reads easy-memory's report.json (HUN-198, FR-066)
**Decision**: `easymem.report(folder)` reads `<store>/report.json` (written by easy-memory `report --json`, its FR-015 / HUN-197): 64 KB cap, regular file inside the store folder, whitelisted fields only (`generated`, `min_sessions`, `too_few`, `on_vs_off_pct`, on/off `sessions` + `tokens_all_median`); `v` > 1 → `unsupported`, anything malformed → `none`; never raises. `memgraph` adds it as `easy_memory.report`; `EasyMemoryPanel` draws "Memory on vs off" under the hit-rate line ("Saves N%" / "Uses N% more" only when `too_few` is false).
**Why**: owner 2026-11-03 asked easy-memory to give easy-personal its numbers; easy-memory owns the on/off join logic, this repo only shows it (no MCP call, DDR-0002). Layout and wording are Supervisor defaults (PRD Open Questions row 23).
**Files**: `src/easy_personal/core/easymem.py`, `src/easy_personal/core/memgraph.py`, `web/src/MemoryView.jsx`, `tests/core/test_easymem.py`, `tests/surfaces/test_api_memory_graph.py`, `tests/leak/`

### 2026-11-03 — One floating toast host and one floating loading pill for the dashboard (HUN-200, FR-065)
**Decision**: `web/src/Toast.jsx` (`ToastProvider` in `App.jsx`, `useNotify()` → `notify(kind, text)`) is the only place action results and action errors show: bottom-right, max 3 (oldest dropped), success `role=status` hides after 5 s, error `role=alert` stays until closed. `web/src/Loading.jsx` is the only page/list loading line (fixed pill under the header, `role=status`). Failed page loads, the Rail card loading and the Live link status stay inline. Forms check fields client-side before any request (`HEX_COLOR`, `EMAIL`, required Name) with errors next to the field; the Confirm/Cancel plan stays inline after the buttons.
**Why**: owner 2026-11-03 (auto mode) asked for better form UX and floating notifications/loading. Placement, timing and what stays inline are auto-mode Supervisor defaults (`tasks/TASK_REVIEW_HUN-200.md` decisions 2–5). Open suggestions: long success toasts close after 5 s; pill overlaps the scope line at 375 px; Save has no primary style.
**Placement (HUN-202, owner 2026-11-03)**: toasts sit 16 px inside the centred shell's right edge, not the screen edge (`right: max(var(--space-4), calc((100% - var(--shell-max)) / 2 + var(--space-4)))`; `%` on a fixed box = viewport without scrollbar, as `.shell` centres on). The loading pill sits in the middle of the window, centred on the main area: `left: 50%` below 64rem, `calc(50% + var(--sidebar-width) / 2)` from 64rem.
**Files**: `web/src/Toast.jsx`, `web/src/Loading.jsx`, `web/src/App.jsx`, `web/src/WorkspacesView.jsx`, `web/src/AgentsView.jsx`, `web/src/app.css`

### 2026-11-03 — Easy Kit site is https://easy-kits.vercel.app/; landing flow drawn as one inline SVG (HUN-201, FR-067)
**Decision**: every Easy Kit link (dashboard `FAMILY` in `web/src/App.jsx`, site header + footer, `scripts/check_static.py` allowlist) points to `https://easy-kits.vercel.app/`; README GitHub repo links to `personal-agentic-claude` stay (repo, not site). `site/index.html` `#flow` holds one token-only inline `<svg role="img">` (both lanes, `data-node` per step, `data-boundary` "your machine", `data-outside` provider) above the kept `data-lane`/`data-step` text lists (now numbered cards with arrows); a `section.quickstart` (4 README commands, no id) sits between `#hero` and `#flow`. The SVG hides when its container is ≤ 720 px (viewport < ~1100 px); the cards carry the flow there. Site stays dark-only (`test_dark_only`).
**Why**: owner 2026-11-03: "the easy kit just update the link … update the index page more visualize, will be easy to catch with the flow, diagram also". Open suggestion (P2): lower the hide threshold to ~560 px so tablets see the diagram.
**Files**: `site/index.html`, `web/src/App.jsx`, `scripts/check_static.py`, `tests/test_site_content.py`, `PROJECT_SPEC_SITE.md`

### 2026-11-03 — Repos folder browser lists folder names from the server (HUN-203, FR-068)
**Decision**: `GET /api/dirs?path=` (`core/folders.py` `list_dirs`) answers sub-folder names + a `repo` flag (`.git` exists) for an absolute path, default `os.getcwd()` of the dashboard process; hidden entries left out, ≤ 500 entries (`truncated`), 400 bad path / 403 unreadable with fixed text, path never echoed or logged, behind `Guard` (header token only). The page's `FolderPicker.jsx` is a modal `<dialog>` (Up / Home / folder list / "Choose this folder"); "+ Type a path" keeps the text row. `.form-actions` gives Save/Back a `--space-4` gap; `main button.primary` is the first primary-button style.
**Why**: owner 2026-11-03: Repos should be a button that opens the current directory; a browser cannot return an absolute path from the OS dialog, so the server lists names. Accepted residual: an explicit `?path=` into a hidden folder lists its sub-folder names (token-gated, names only). The built-in `security-review` skill needs an `origin` remote (this repo has `github`), so HUN-203's security review was done by hand.
**Files**: `src/easy_personal/core/folders.py`, `src/easy_personal/surfaces/api.py`, `tests/surfaces/test_api_dirs.py`, `tests/leak/harness.py`, `web/src/FolderPicker.jsx`, `web/src/WorkspacesView.jsx`, `web/src/app.css`

### 2026-11-05 — Saved investigate reports = Option A (report + server rule snapshot)
**Decision**: MCP `save_report` stores the owner session's `investigate` summary (≤ 2,000 chars) and ≤ 5 fixes (≤ 300 chars) plus a server-taken rule snapshot of every declared rule, in one table (m0010, JSON columns); `rules.evaluate(checked=True)` lists every measured rule; the Warnings & plan page shows Saved reports with before / now per rule, "now" computed on load. Secret-looking text is refused. HUN-221 → HUN-222. → see DDR-0003, BRAINSTORMING_LOG_REPORTS.md
**Why**: Owner 2026-11-05 "log this report to the dashboard … trace the improvement"; chose report + numbers, before / now, narrow NFR-002 exception, Warnings page, Option A. Option C (store warnings output verbatim) loses the "now" number for a rule that stopped firing.
**Files**: `src/easy_personal/core/reports.py`, `src/easy_personal/core/rules.py`, `src/easy_personal/core/migrations/m0010_investigate_reports.py`, `src/easy_personal/surfaces/mcp.py`, `src/easy_personal/surfaces/api.py`, `web/src/WarningsPlanView.jsx`, `PRD.md`, `docs/ddr/0003-store-owner-saved-investigate-reports.md`

### 2026-11-05 — easy-context out of the family: hide when not in use (FR-080)
**Decision**: easy-context is "in use" when the scope's stored events have an `mcp__easy-context__*` tool call in the last 7 days. When it is not in use, `context_index_stale` is left out of warnings, plan and `checked` (not listed as abstained); saved reports drop an abstained-then rule and show `not_in_use` for a measured-then one; the Kit page hides the Code index part. Nothing is deleted; a new call brings it all back. HUN-225 + HUN-226.
**Why**: Owner 2026-11-05 "I removed the context out of family", chose "Hide when not in use" over removing support or a per-rule config switch. Signal from stored metadata, so no Claude config file is read (accounts.py reads only 3 `oauthAccount` keys).
**Files**: `src/easy_personal/sources/context.py`, `src/easy_personal/core/rules.py`, `src/easy_personal/core/reports.py`, `src/easy_personal/core/kit.py`, `web/src/KitView.jsx`

## Infrastructure

### 2026-11-06 — Live call labels + kit stage: show, never store (FR-081..FR-084)
**Decision**: The live tail reads `Skill.skill`, `Agent`/`Task` `subagent_type`, Bash first word, MCP server/tool, and sub-agent `agentType` (meta file) as call labels; stage = newest `Skill` call in a fixed code map; sent on `/api/live` only, never stored. Page: stage chip on the session row, labels in the panel feed. → see DDR-0004
**Why**: Owner 2026-11-06 "don't know what is agent doing … which tools, which steps of the kits"; chose Safe labels only, fixed map, agentType only, row chip + panel feed.
**Files**: `src/easy_personal/core/live.py`, `src/easy_personal/core/jobs.py`, `web/src/LiveView.jsx`, `docs/ddr/0004-live-call-labels-show-never-store.md`, `PRD.md`, `PROJECT_SPEC.md`

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

## 2026-11-02 — easy-memory layer in the Memory view = one graph response (Option A)

**Decision**: The Memory view also shows the easy-memory store (`$EASY_MEMORY_HOME`, else `~/.easy-memory/`), read as files — never through the easy-memory MCP (keeps DDR-0002) — metadata only: each note's `time` + `files` map and `prompts.log` `time` / `effective` / `injected`; `prompt`, `answer`, `session_id` and `trash/` never leave `core/easymem.py`. Per project: the 30 most-cited files as `easy_memory` nodes on ring 4 (merged into a kit / Claude node with the same path), stale = newest citing note's hash ≠ disk (or gone), a 30-day panel (notes per local day, hit rate), easy-memory-only projects listed. Same `/api/memory-graph` route, no new route, nothing stored.
**Why**: owner asked to "show memory visualize from easy-memory mcp with the kit family"; notes have no `[[links]]`, so cited files are the only graph-shaped metadata; MCP tools return note text, which the metadata-only rule bans. Options B (separate route) and C (panel first) rejected in `BRAINSTORMING_LOG_EASYMEMORY.md`.
**Approved by**: owner, 2026-11-02 (5 grill answers + Option A + "continue").
**Files**: `PRD.md` US-024, FR-051..FR-053, NFR-010 (amended), OQ 18–19; `BRAINSTORMING_LOG_EASYMEMORY.md`; `PROJECT_SPEC.md` glossary (easy-memory note, Cited file), read-only constraint, Content leakage row. DDR gate: < 2 of 3 criteria (easy to reverse, no new route or storage) → no DDR.
**Stage 2 2026-11-02**: 2 tasks on Linear — HUN-184 backend reader + graph fields (P1, C2, Medium → `security-review`), HUN-185 Memory view layer (P1, C2, Medium, UI Evidence rows); chain 184 → 185. Not part of the v0.4.0 release (HUN-183); release placement decided after HUN-185.

## 2026-11-03 — Kit performance: session findings read by `wake` (requirement grill)

**Decision**: easy-personal-mcp's main goal is Easy Kit performance (tokens/cost, result quality, speed). First release: per kit MCP server (easy-ui, easy-verifier, easy-context, easy-memory), compare sessions that used it with sessions that did not, like with like (same owner project, model, size band), ≥ 10 sessions per group, worded "linked to". Good result = a commit on the session's branch (first event → last event + 30 min) and a tool-error rate ≤ `kit_error_rate`. Tool errors are not stored today → add a per-message failed-tool-result count (FR-059, store migration 7 → 8, re-read). New MCP tool `kit_findings` (≤ 3 findings with a fixed habit text); `wake` adds ≤ 3 lines; Claude follows the habit in that session only, never edits kit files from a finding.
**Why**: owner, 2026-11-02: the MCP part had no clear use (its own tools: 0 calls in 624 sessions); the tool should make the whole kit work better. Like with like works on existing history (625 sessions, 81 with a kit server); a planned on/off test costs weaker sessions.
**Approved by**: owner, 2026-11-03 (6 grill answers + error-count answer after the Supervisor corrected its Q3 claim that errors were already stored).
**Files**: `PRD.md` Overview, US-026, FR-059..FR-063, KPI row, Out of Scope, OQ 22. Brainstorming next (Stage 0.5b).
**Brainstorm 2026-11-03 (owner)**: pool matched sessions over like-with-like groups (≥ 10 pooled per side); `wake` change = XR-2 on the kernel repo (`skills/wake` is kit-managed). Added NFR-014 (< 3 s, git time budget). Recommended build: Option A, compute on read (`BRAINSTORMING_LOG_KITPERF.md`).
**Stage 2 2026-11-03**: 3 tasks on Linear — HUN-192 store v8 `tool_error_count` + re-read (P0, C2, Medium → `migration-safety` + `security-review`), HUN-193 `core/kitperf.py` + `kit_findings` MCP + CLI (P0, C2, Medium → `security-review`), HUN-194 release 0.5.0 (P1, C1, Low; after HUN-183); chain 192 → 193 → 194. XR-2 not filed yet (kernel repo board).
**Revised 2026-11-03 (owner, after the HUN-193 real run)**: the first build linked all 4 servers to bigger sessions (easy-ui +236% tokens, +207% cost, +196% length; no habit anywhere), partly because the Supervisor's rule grouped by total tokens (a compared measure). Now: size band = tool-call tertile; sessions with 0 tool calls not compared; measures = tokens per tool call, cost per tool call, good-result share (raw length dropped); `findings` = habit-bearing comparisons only (wake shows only these), all others under `other`. Unknown error rate = no counted user record in the session. Error rate denominator = tool calls (PRD aligned). `kit_error_rate` default 0.04 (owner-store median 0.0433).

## 2026-11-03 — Cache reuse card follows the app-wide period + detail lines

**Decision**: The Usage period picker becomes app-wide state (App.jsx); the right-rail "Cache reuse" card uses it (was fixed to the last 7 days) and adds "Written to cache" (cache creation ÷ prompt tokens) and "Lowest" (model with the lowest ratio among models with ≥ 20 messages). Page-only, no API change (FR-064, US-027).
**Why**: owner saw "always 1 number". Measured 2026-11-03: the ratio is correct but steady (days 96.0–97.9 %, 7d/30d/all 97.4/97.4/97.5 %); the rail card ignored the picker. Per model it differs (Opus 5.5 97.8 %, Sonnet 5.5 95.2 %, Haiku 5.5 82.5 %); cache writes 2.64 % of prompt tokens (30 days).
**Approved by**: owner, 2026-11-03 ("let do it" + "Follow period + detail" mockup).
**Files**: `web/src/Rail.jsx`, `web/src/UsageView.jsx`, `web/src/App.jsx`, `PRD.md` US-027, FR-064

## 2026-11-05 — Dashboard reads share repeated work: single-flight memo keyed by `PRAGMA data_version` (HUN-227, HUN-228)

**Decision**: `core/memo.py` caches `metrics._messages` and `owners._stored_branches` per (store file identity, data version, function, arguments), with single flight and immutable results (tuples, frozensets, `MappingProxyType`); at most 32 entries and 8 watcher connections. The data version is `PRAGMA data_version` on one long-lived watcher connection per store, which changes on every commit by any other connection in any process. No caching when the watcher fails, the store is in memory, or the caller's connection is in a transaction. The Usage page renders each part when its own read answers, and Usage and the rail use one `since` rounded down to the whole minute.
**Why**: owner, 2026-11-05: "the data loading take so long". Measured on a store copy (462k events): ~14 parallel reads, page 5.1–6.0 s; `_messages` 0.31 s ran ~7×/page, `_stored_branches` 0.26 s per caller; mostly GIL-bound Python, so threads did not overlap. Debate (2 Advocates): whole-response cache in `api.py` rejected (live ingest invalidates it, cold page unchanged); `MAX(events.id)` key rejected (misses upserts, updates, deletes, workspace / account edits).
**Result**: page on a store copy 6.0–6.8 s → 2.4–2.6 s full, first card ~1 s (`/verify`, 2026-11-05). `/api/warnings` and `/api/usage/sessions` stay ~2.4 s in the cold parallel page (follow-up candidates).
**Approved by**: owner, 2026-11-05 ("yes, investigate and fix them [auto]"; merge).
**Files**: `src/easy_personal/core/memo.py`, `src/easy_personal/core/metrics.py`, `src/easy_personal/core/owners.py`, `web/src/UsageView.jsx`, `web/src/Rail.jsx`, `BRAINSTORMING_LOG_PERF.md`

## 2026-11-07 — Kit by workspace: easy-ui check results stay overall; the block lives on the Kit view (HUN-238, HUN-239; PRD open question 30)

**Decision**: `kit.workspace_rollup` + `GET /api/kit/workspaces` show per-workspace easy-verifier scores (from `verifier_scores`) and easy-ui use (sessions, calls, last call from stored `events`). easy-ui check results (passed/failed) stay in one overall `ui_checks`. The page block "Kit by workspace" sits above the project list in the Kit view and uses the app-wide period and account, not the workspace filter.
**Why**: owner, 2026-11-07: "for verifier mcp, let collect and show the report by workspace to the dashboard ... for ui mcp, do the same stuffs". `ui_sessions` stores no project and `target` is never stored (FR-013, NFR-002), so a per-workspace check would be a guess (time overlap) or a schema/privacy change. Two Debates of 2 Advocates each (Medium risk) chose keep-overall and the Kit view.
**Result**: median read 0.34 s on 160 projects / 10 workspaces; merged after `/verify` (2026-11-07).
**Approved by**: owner asked for the work in `[auto]` mode and later for verify and merge; the two debated choices still await the owner's review.
**Files**: `src/easy_personal/core/kit.py`, `src/easy_personal/surfaces/api.py`, `web/src/KitView.jsx`, `web/src/WorkspaceKit.jsx`, `BRAINSTORMING_LOG_KITWS.md`

## 2026-11-07 — Report folders per workspace, and their files: depth 1, name contains "report", relative path only, own routes (HUN-240, HUN-241, HUN-242; FR-089, FR-090, PRD questions 31 and 32)

**Decision**: `kit.report_folders` + `GET /api/kit/reports` list, per non-orphan workspace, the report folders of its repos: a directory directly inside a repo root (no symlink followed) whose lowercase name contains `report` (`reports`, `smoke-reports`), at most 8 per repo, with file count (cap 2000), count of `session-*.json` files, and newest file time. The Kit view shows them as plain text under each workspace row of "Kit by workspace". Reference only: nothing is opened, parsed, stored or acted on. The read is its own route, so `/api/kit/workspaces` keeps its no-file-read guarantee.
**Decision (FR-090, HUN-242, 2026-11-08)**: each folder entry also carries `repo_no`, the 0-based place of the repo in the workspace's repo list, so one repo can be named when two share a folder name. Two more read-only `GET` routes under the same Guard: `/api/kit/report-files` lists the regular files of one report folder to depth 3, newest first, 200 at most, with a relative `path`, `size`, nullable `mtime_utc` and an `openable` flag; `/api/kit/report-file` returns one file's bytes. The server resolves workspace + `repo_no` + `folder` itself, so no request ever carries an absolute path. Every miss is 404 with the fixed body `{"error":"not found"}`, a file over 10 MB is 413. Each path level is opened with `dir_fd` + `O_NOFOLLOW`, which closes the check-then-open window a `realpath` check would leave open. The content type comes from a fixed extension table, never from the content (`svg` deliberately absent), with `nosniff`, `no-store` and `inline`. A `.html` report is served as `text/html`, settled by a 3-Advocate Debate: a route-set `Content-Security-Policy` is useless here because `Guard.send_csp` overwrites it (`src/easy_personal/surfaces/api.py:98-99`) and a route CSP never governs a `blob:` document the page builds, so the script defence is the dashboard CSP the blob tab inherits, gated by HUN-243's AC 8 real run.
**Why**: owner, 2026-11-07: "you can depend on the path of projects in the workspace, and mapping the folder reports, report, or something like that to the dashboard, we just show to easy refer, not manage it in anyway". Owner choice (asked): show the relative path `<repo folder>/<report folder>`, not the full path, so NFR-002 and the leak tests stay as they are. The substring match and the separate route are the Supervisor's choice (smaller change, can be undone; PRD question 31 stays open for the owner's review of the match).
**Result**: median read 0.03 s on 160 repos / 10 workspaces; review found one P1 (one open `scandir` handle per sub-folder; fixed, test under a low `RLIMIT_NOFILE`); merged after `/verify` (2026-11-07). HUN-242 merged 2026-11-08 after `/verify` passed every Acceptance Criterion: `code-review` found 0 P0 and 0 P1 code findings, `security-review` nothing at or above its bar. The same merge carried a fix for a defect `/verify` found: a file with an out-of-range `mtime` made both report routes answer 500.
**Approved by**: owner asked for the feature, chose the relative path, and asked to merge both tasks.
**Files**: `src/easy_personal/core/kit.py`, `src/easy_personal/surfaces/api.py`, `web/src/KitView.jsx`, `web/src/WorkspaceKit.jsx`, `tests/leak/harness.py`, `tests/core/test_kit_report_files.py`, `tests/surfaces/test_api_kit_report_files.py`, `tests/leak/test_kit_report_files_leak.py`
