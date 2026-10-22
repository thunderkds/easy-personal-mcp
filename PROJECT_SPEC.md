# PROJECT_SPEC.md
**Last updated**: 2026-10-21
**Version**: 0.2 (rebuilt after the Phase 0 recheck; Option B approved)

> **Scope of this document**: *How* to build it safely — architecture, agent config, constraints, risk areas, task state, and accumulated learnings.
> Product intent (personas, user stories, FR/NFR, success metrics) lives in `PRD.md`.
> If Critical Constraints here conflict with Out of Scope in `PRD.md`, resolve before Stage 2.

---

## Project Identity

- **Name**: easy-personal-mcp (MCP server id `easy-personal`, CLI `easy-personal`)
- **Repo**: `thunderkds/easy-personal-mcp`
- **Primary tech**: Python ≥ 3.11, official MCP Python SDK (FastMCP; brings `starlette`, `uvicorn`, `httpx`), SQLite (stdlib), `keyring`, `uv`; dashboard: React + Vite in `web/`, built assets shipped in the package
- **Type**: MCP server (stdio default, streamable HTTP opt-in) + CLI + on-demand local dashboard (JSON API + SPA)
- **Deployment target**: the owner's Linux machine, native install (`uv tool install`); Docker optional, analytics-only (DDR-0001)
- **Board**: Linear team HUN, project [easy-personal-mcp](https://linear.app/thunderkds/project/easy-personal-mcp-a8282bde25b5) (`.claude/board.json`, online mode)
- **Key stakeholders**: thunderkds (owner)

---

## Architecture Summary

One Python package plus one SPA (BRAINSTORMING_LOG.md, Option B). **Sources** are read-only adapters that
turn local data into normalized metadata events. **Core** owns project discovery, the store
(`~/.easy-personal/store.sqlite`, WAL, single writer), metrics, declared rules, warnings and the improvement plan.
**Surfaces** — MCP tools, CLI, and the dashboard's JSON API — call only the core, so every surface shows the same
numbers. **Connectors** (catalog, keychain, writer, suggestions) sit beside the core; core and sources never import
them. **`launch.py`** is the connector launcher and imports only stdlib + `keyring`.

```
src/easy_personal/
  sources/     claude_code · kit_trace · verifier · context (read-only MCP client of easy-context) · ui · git_local
  core/        discovery · store · metrics · rules · plan · prices
  surfaces/    mcp · cli · api (Starlette JSON API + static file serving, on demand)
  connectors/  catalog/*.yaml · keychain · writer (claude mcp add-json) · suggestions
  launch.py    exec / headers
  dashboard/static/   built output of web/
web/           React + Vite app (dev-time only; never served by Vite at runtime)
```

Runtime shape (B2): `easy-personal dashboard` runs ingest, starts the API + page on `127.0.0.1` (free port,
per-launch token), opens the browser; Ctrl-C stops it. A systemd user timer runs `easy-personal ingest` on schedule.

Milestones inside v1: **M1** usage → **M2** kit → **M3** connectors → **M4** dashboard.

---

## Glossary

- **Source** — a read-only adapter producing events from one local data origin (`claude_code`, `kit_trace`, `verifier`, `context`, `ui`, `git_local`).
- **Event** — one normalized metadata record (time, machine, session, project, model, kind, token counts, tool name). Never content.
- **Project** — where usage is counted: the git top-level of a session's `cwd` (worktrees → main repo; subfolders → their repo), else the folder itself. Resolved at ingest and stored, so a deleted folder keeps its history (shown *missing on disk*). Kit sources read only projects that still exist.
- **Kit part** — one of the four Easy Kit components easy-personal reads: kernel (traces), easy-verifier, easy-context, easy-ui.
- **Kit task id** — the task id in a kernel trace file name (`T001.jsonl`); not a task on this project's board.
- **Session** — one Claude Code session (`sessionId`). An easy-ui `session-*.json` is a **UI check**, never a session.
- **Report** — only a file left by a kit part (verifier HTML/JSON, easy-ui JSON). easy-personal writes no reports; its outputs are **surfaces** (MCP, CLI, dashboard).
- **Dashboard** — the on-demand React page plus its JSON API served by `easy-personal dashboard`. (Not "live page".)
- **Abstained** — a record or metric the engine could not interpret; counted and shown, never scored.
- **No data** — a source that has nothing to read for a project (file missing, service down); shown explicitly, never zero-filled.
- **Rule** — a declared check: id, metric, threshold, label (`improve` / `aware` / `warning`), priority, fix line. v1 pack: PRD FR-026.
- **Index freshness** — easy-context `last_index_at` for a project compared with its latest local commit; *stale* fires `context_index_stale`.
- **API-equivalent cost** — tokens × dated price table; labelled so, never presented as what a subscription pays.
- **Connector** — a configured provider (e.g. Slack) = catalog entry + keychain refs + a Claude Code user-scope MCP entry pointing at the launcher.
- **Launcher** — `easy-personal-launch exec|headers <connector>` (separate console script, stdlib + `keyring` only): the command Claude Code runs, which injects secrets at launch.
- **Milestone (M1–M4)** — internal delivery order inside the single v1 release.

---

## Critical Constraints

- Read-only on every source: never modify `~/.claude/projects/`, kit traces, report folders, `~/.easy-context/` or any repo.
- Store and emit **metadata only**: no message text, tool input/output, trace `summary`, easy-ui `target`, file contents or secrets in the store, MCP/API responses, page, or logs.
- No traffic leaves the machine: local calls to `127.0.0.1` only; every HTTP surface binds `127.0.0.1`, never `0.0.0.0`; the page loads no remote asset (CSP `default-src 'self'`).
- The only runtime call to another plug-in is the read-only easy-context MCP client (allow-list: `list_indexes`, `get_stats`). Never `index_path`; never call easy-verifier or easy-ui at runtime (DDR-0002).
- Deterministic: no LLM in the server; every warning cites rule id + measured value.
- Writes allowed only to `~/.easy-personal/`, the OS keychain (service prefix `easy-personal/`) and Claude Code user scope **via `claude mcp add-json|remove --scope user`** (DDR-0001). Never `~/.claude.json` by direct edit; never project-scope config or hooks (kernel-owned).
- No MCP tool may write config, create a connector or return a secret value. Dashboard writes need the per-launch token and a matching `Host`/`Origin`.
- `launch.py` imports only stdlib + `keyring`; secrets reach the child process only through its environment, never argv.
- Sibling repos are conventions-only references; never import or vendor their code. Cross-repo needs go to that repo's board (XR-1).
- Tool surface is frozen once approved; new tools need owner approval.
- npm: exact versions in `package-lock.json`, `npm ci` only; the built page ships in the package so install needs no Node.

---

## Known Risk Areas

| Area | Risk Level | Reason | Files |
|------|-----------|--------|-------|
| Transcript parsing | High | Undocumented, changing format; silent mis-parse = wrong numbers | `sources/claude_code.py` |
| Content leakage | High | Transcripts, trace `summary` and easy-ui `target` hold private text | `sources/`, `core/store.py`, all surfaces |
| Launcher | High | In the start path of every connector; handles live secrets | `launch.py` |
| Dashboard write path | High | Browser-reachable `127.0.0.1` endpoint that stores secrets (CSRF, DNS rebinding) | `surfaces/api.py`, `web/` |
| Claude Code config writes | Med | A wrong entry breaks a connector; must go through the official CLI | `connectors/writer.py` |
| easy-context client | Med | Cross-plug-in call; service down/slow must not hang the page | `sources/context.py` |
| Incremental ingest | Med | Offsets, partial last line, files deleted by cleanup, timer vs. page concurrency | `sources/`, `core/store.py` |
| npm supply chain | Med | Many transitive packages in a tool that handles the keychain | `web/package-lock.json` |
| Cost estimates | Low | Price table goes stale | `core/prices.py` |

---

## Sub-Agent Team

All agents run in Claude Code (`claude`); spawned with the `Agent` tool. Final roster is locked in Stage 1.5.

| Agent | Role | CLI Spawn Command |
|---|---|---|
| Common-Infrastructure-Agent | Repo skeleton, `pyproject.toml`, `web/` toolchain, build-into-package, systemd timer, worktrees | `Agent({ subagent_type: "common-infrastructure", prompt: "..." })` |
| Backend-Implementer | Sources, core, MCP/CLI, JSON API, connectors, launcher | `Agent({ subagent_type: "backend-developer", prompt: "..." })` |
| Frontend-Implementer | `web/` React + Vite dashboard | `Agent({ subagent_type: "frontend-developer", prompt: "..." })` |
| QA-Automation-Agent | Fixtures, leak tests, parser tests, UI evidence via easy-ui (dev-time) | `Agent({ subagent_type: "qa-expert", prompt: "..." })` |

---

## CLI Model Table

| CLI | Run command | C0 | C1 | C2 | C3 |
|---|---|---|---|---|---|
| claude | `claude` | `claude-haiku-5-5` | `claude-sonnet-5-5` | `claude-sonnet-5-5` | `claude-opus-5-5` |

---

## Tasks

Created 2026-10-21 in Stage 2 (`to-issues`, owner-approved). Milestones: M1 = HUN-32…38, M2 = HUN-39…43, M3 = HUN-44…47, M4 = HUN-48…52.
The first plan's HUN-21…HUN-29 are archived with the trashed project and do not carry over. Live status: `PROJECT_KANBAN.md`.

| ID | Title | Status | Assigned Agent | Complexity | Risk | Priority |
|----|-------|--------|---------------|-----------|------|----------|
| HUN-32 | Spike: inventory Claude Code transcript shapes and real volume → synthetic fixtures | Todo | QA-Automation-Agent | C1 | Medium | P0 |
| HUN-33 | Skeleton: package, pyproject, `easy-personal` CLI + MCP stdio entry sharing one core, test runner | Todo | Common-Infrastructure-Agent | C1 | Low | P0 |
| HUN-34 | Tracer bullet: `claude_code` source → store (project, machine_id, UTC) → `usage_summary` in MCP and CLI | Todo | Backend-Implementer | C3 | High | P0 |
| HUN-35 | `usage_breakdown` + `session_outliers`: tokens-first, labelled API-equivalent cost, local day, folder names | Todo | Backend-Implementer | C2 | Medium | P0 |
| HUN-36 | Leak test suite: planted secrets and text never reach store, MCP, CLI or logs | Todo | QA-Automation-Agent | C2 | High | P0 |
| HUN-37 | Rules engine + 5 usage rules + `warnings` + `improvement_plan` | Todo | Backend-Implementer | C2 | Medium | P1 |
| HUN-38 | Scheduled ingest: single-writer lock, systemd user timer, `retention_days` | Todo | Common-Infrastructure-Agent | C1 | Medium | P1 |
| HUN-39 | `kit_trace` source (never `summary`) + `tool_errors_high` rule + `kit_summary` (kernel part) | Todo | Backend-Implementer | C2 | High | P1 |
| HUN-40 | `git_local` source + `kit_summary` git part | Todo | Backend-Implementer | C1 | Low | P1 |
| HUN-41 | `context` read-only MCP client of easy-context + index freshness + `context_index_stale` | Todo | Backend-Implementer | C2 | Medium | P1 |
| HUN-42 | `verifier` source reads easy-verifier score JSON + `kit_summary` part | Todo | Backend-Implementer | C1 | Medium | P2 |
| HUN-43 | `ui` source: easy-ui UI checks overall (never `target`) + `kit_summary` part | Todo | Backend-Implementer | C1 | Medium | P2 |
| HUN-44 | Launcher: keychain store + refusal path, `exec`/`headers`, env-only secrets, <200 ms | Todo | Backend-Implementer | C2 | High | P1 |
| HUN-45 | Connector tracer: Slack catalog entry + `connector add/remove` CLI → keychain → `claude mcp add-json` (snippet fallback) + `doctor` | Todo | Backend-Implementer | C2 | High | P1 |
| HUN-46 | Catalog breadth: Gmail, SendGrid, Teams, Discord + claude.ai-connector flag | Todo | Backend-Implementer | C1 | Low | P2 |
| HUN-47 | MCP `connector_catalog` / `connector_status` / `connector_suggestions` (metadata counters) | Todo | Backend-Implementer | C2 | Medium | P2 |
| HUN-48 | `web/` toolchain: Vite + React, pinned lockfile, build copied into the package, release check | Todo | Common-Infrastructure-Agent | C2 | Medium | P1 |
| HUN-49 | Dashboard server: `easy-personal dashboard` on demand, free port, token + Host/Origin guard, CSP, read API | Todo | Backend-Implementer | C2 | High | P1 |
| HUN-50 | Dashboard usage view: summary, breakdowns, session timeline, "not included" notice | Todo | Frontend-Implementer | C2 | Medium | P1 |
| HUN-51 | Dashboard warnings, improvement plan and kit panel | Todo | Frontend-Implementer | C2 | Low | P2 |
| HUN-52 | Dashboard connector forms + guarded write API (secrets never echoed) | Todo | Frontend-Implementer | C2 | High | P2 |
| HUN-53 | Session timeline read: `metrics.session_timeline` + `GET /api/usage/sessions` (added 2026-10-22, blocks HUN-50 AC2) | Todo | Backend-Implementer | C1 | Medium | P1 |

Cross-repo: **XR-1** = easy-verifier-mcp **T120** (`../easy-verifier-mcp/tasks/TASK_GUIDE_T120.md`, its local board) — score JSON sidecar next to each HTML report — **Done 2026-10-21**. Needed by FR-011 / HUN-42 (M2).

---

## Memory / Insights

| Date | Insight | Source Task |
|------|---------|------------|
| 2026-10-21 | Claude Code deletes local transcripts after 30 days (`cleanupPeriodDays`); ingest must be incremental and scheduled. | brainstorm |
| 2026-10-21 | The connector problem is secret *delivery*, not storage: Claude Code spawns a command, so a launcher injects keychain secrets without any plaintext config. | brainstorm |
| 2026-10-21 | `~/.claude.json` also holds sign-in and trust; write MCP entries through `claude mcp add-json --scope user`, not by editing the file. | brainstorm |
| 2026-10-21 | Kernel trace `summary` holds raw tool input; easy-ui `target` is caller text — both are content, never stored. | requirement grill |
| 2026-10-21 | easy-verifier leaves HTML only and easy-ui writes into its own server cwd; per-project kit data needs XR-1 (verifier) and is overall-only for easy-ui. | requirement grill |
| 2026-10-21 | An empty `project` in `.claude/board.json` makes `board_sync.py pull` fetch the whole team; the old project had been trashed. | Stage 1 |
