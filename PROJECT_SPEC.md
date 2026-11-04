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
- **Connector id / alias** — a connector's fixed id keys its keychain item and its launcher command (`exec <id>`); its alias is the name registered in Claude Code, default `<workspace>-<provider>`, owner-editable. Renames and moves change only the alias. *(planned, 2026-10-26)*
- **Launcher** — `easy-personal-launch exec|headers <connector>` (separate console script, stdlib + `keyring` only): the command Claude Code runs, which injects secrets at launch.
- **Workspace** — a named group the owner creates (e.g. "Acme", "Personal"): identity (name, color/icon, description, main email), a plain list of repo paths, and its own connectors. Has a fixed id; its name can be renamed. Holds no agent settings. At most one connector per provider, except the built-in **orphan** workspace. Deleting a workspace never deletes connectors: they move to orphan, which is active in no agent account; from orphan they can be assigned to another workspace. Connectors are deleted one at a time only. Pre-workspace connectors (`slack`, …) start in orphan. *(planned, 2026-10-26)*
- **Agent account** — one agent config folder on this machine (Claude: `~/.claude*`; Codex: `~/.codex*`), auto-detected and confirmed + named by the owner (e.g. "Claude company"). A Claude agent account picks one or more workspaces whose connectors are written into it under their aliases; a Codex agent account is listed only. *(planned, 2026-10-26)*
- **Milestone (M1–M4)** — internal delivery order inside the single v1 release.
- **Work view** — the dashboard's `// work` section: Board, Scheduled, Memory. Read-only; inspired by Javis OS (ideas only). *(planned, 2026-10-31)*
- **Board task** — one task row read live from a kit project's `PROJECT_KANBAN.md`: project + task id + title + **board status** (`Todo`, `In Progress`, `Ready for Review`, `Done`, `Closed (will not do)`, `Blocked`). Not a **kit task id** (trace file name), though the ids often match. Never stored.
- **Last board pull** — the mtime of a project's `PROJECT_KANBAN.md` when its `.claude/board.json` names a board; shows how fresh the Linear side is. easy-personal never calls a board API.
- **Typed work / Agent work** — the usage split. *Agent work* = a sidechain record, or any record of a session whose **entrypoint** starts with `sdk` (headless). *Typed work* = the rest (`cli`, `claude-desktop`, IDE, …). A session with no entrypoint yet (pre-backfill) is **unknown** (abstained). Dashboard labels: "You" / "Agents".
- **Entrypoint** — the transcript's `entrypoint` field (`cli`, `sdk-cli`, …), stored per event from the Work view migration on.
- **Scheduled job** — a systemd user timer or crontab line whose command's first word (basename) is on the agent allow-list (`claude`, `easy-personal`, `easy-personal-launch`, kit tools). Shown: name, kind, schedule, next run, last run, last exit code. Arguments are never read out. Not a /loop job or a cloud routine.
- **Memory note** — one `.md` file under `<repo>/memory/` (**kit memory**) or `~/.claude*/projects/*/memory/` (**Claude memory**). Its graph identity is the frontmatter `name` slug, else the file stem.
- **easy-memory note** — one `notes/<id>.json` in the easy-memory store (`$EASY_MEMORY_HOME`, else `~/.easy-memory/<key>/`), saved by easy-memory's Stop hook. Only its `time` and `files` map are read out; its `prompt`, `answer` and `session_id` never leave `core/easymem.py`. Not a **Memory note** (no frontmatter, no links). *(planned, 2026-11-02)*
- **Cited file** — a repo-relative path in an easy-memory note's `files` map, drawn as one graph node per project (top 30 by citation count, FR-052). **Stale** when the file's current SHA-256 differs from the hash in the newest note that cites it, or the file is gone. *(planned, 2026-11-02)*
- **Kit server** — one of the four Easy Kit MCP servers compared for kit performance: easy-ui, easy-verifier, easy-context, easy-memory. A session *used* a kit server when any of its events (sub-agent records included) calls a tool named `mcp__<server>__*` (FR-060). easy-personal itself is not a compared server. *(built HUN-193, 2026-11-03)*
- **Tool error count** — `events.tool_error_count`: the number of `tool_result` blocks with `is_error` true in one `user` record (FR-059, store v8). NULL = unknown (not a user record with tool results, or ingested before v8). *(built HUN-192, 2026-11-03)*
- **Good result** — a session with a commit on its branch between its first event and last event + 30 min, and a tool-error rate (failed tool results ÷ tool calls) ≤ `kit_error_rate`. Sessions on `main`/`master`/`HEAD`/no branch, or with unknown error counts, have an *unknown* result, not a bad one (FR-061). *(built HUN-193, 2026-11-03)*
- **Kit finding** — one pooled like-with-like comparison (strata = owner project × main model × tool-call tertile) of one kit server on one measure (tokens per call, cost per call, good-result share), shown only with ≥ 10 pooled sessions per side and always worded "linked to" (FR-062). Returned by the MCP tool `kit_findings` (FR-063): `findings` = the ones with a habit, `other` = the rest. *(built HUN-193, 2026-11-03)*
- **Habit** — the fixed text a kit finding carries when the server side is better by at least 5 % (`kitperf.MIN_GAP_PCT`); Claude follows it in that session only and edits no file. *(built HUN-193, 2026-11-03)*
- **Gone project** — a stored project path (`events.project_path`) whose folder no longer exists on disk (e.g. a deleted `/tmp` test folder). Hidden from the Board, Kit and Memory lists (FR-054); its usage history is still counted.
- **Owner project** — the stored repo that a gone worktree or subfolder belongs to (FR-057): by path when a parent folder is still a git repo, else by its git branch name found in exactly one stored repo. Usage by project and the project lists count it there; stored events keep their own path.
- **Scratch folder** — a stored project path that is under the system temp folder, gone, not inside a git repo, or inside a hidden folder, and has no owner project. Every project list shows all scratch folders as one `Scratch` row that opens to list them (FR-069). A *real project* is every other path (a git repo). *(HUN-206, 2026-11-04; UI HUN-207)*
- **Unassigned repo** — a real project with stored usage whose path is not equal to or under any workspace repo path; listed on the Workspaces page with an `Add to` control (FR-070). *(planned HUN-206/HUN-207, 2026-11-04)*
- **Dangling link** — a `[[name]]` in a memory note that resolves to no memory note in the same project; drawn as an empty node.
- **Live tail** — the background reader `core/live.py` that follows transcripts while `easy-personal dashboard` runs (FR-043). Not the scheduled ingest, though both share one cursor.
- **Stream** — one transcript file of a session: the main thread or one subagent (`subagents/*.jsonl`). Subagent records share the parent's session id, so failure streaks and open calls are kept per stream. *(planned, 2026-11-02)*
- **Open call** — a `tool_use` block whose `tool_result` has not arrived yet, paired by block id (held in memory only, never stored or sent). A later assistant message in the same stream also closes it. *(planned, 2026-11-02)*
- **Attention state** — one of three marks on a live session (FR-048/049): **failing** (a stream's last N results all failed, default 3), **waiting for you** (an open `AskUserQuestion` / `ExitPlanMode`), **needs attention** (another open call ≥ N min, default 10, while the whole session wrote nothing). Worked out in the live tail; the page only renders it. Not a **Rule** (no weekly window, not in `rules.yaml`). *(planned, 2026-11-02)*
- **First-run checklist** — the `// system` panel that shows done / not done for: first ingest, ingest timer, agent accounts confirmed, first workspace. Links to existing commands and buttons; adds no write.

## Kit performance (HUN-193, FR-060..FR-063, NFR-014)

Rules of `core/kitperf.py`, computed on every call from `events` (nothing stored). MCP `kit_findings(limit=3, since=None)` and CLI `easy-personal kit-findings [--json] [--limit N] [--since D]` call the same function.

- **Compared sessions** (revised 2026-11-03): `abstained = 0`, at least one `assistant` event with a model, **at least one tool call**, inside `since` (default: whole store). Sessions with no tool call are not compared. Tokens count once per `message.id`. Main model = the model with the most output tokens. Owner project = FR-057 (`core/owners.owner_map`). Cost = price table; one unpriced model makes the session's cost unknown for that measure.
- **Kit server used**: any event (sub-agent records included) with a tool name that starts with `mcp__<server>__`, server in `easy-ui`, `easy-verifier`, `easy-context`, `easy-memory`. `mcp__claude_ai_*` and other names never count.
- **Strata**: (owner project, main model, size band). Size band = tertile of **tool-call count** over the compared sessions (cut points = sorted counts at 1/3 and 2/3; a tie on a cut point falls into the higher band); it is never a compared measure. A stratum counts for a measure only if it has at least 1 session with a known value on each side and the non-server mean is not 0.
- **Pooled delta** (per server and measure) = Σ over counted strata (server sessions x (server mean - other mean)) / Σ (server sessions x other mean), in percent. Measures: `tokens_per_call` (total tokens / tool calls), `cost_per_call` (API-equivalent cost / tool calls), `good_result` (share; known results only). Raw tokens, raw cost and session length are not compared.
- **Finding**: a comparison passes when its pooled "with" and "without" session counts are each at least 10 for that measure; otherwise the server is listed under `not_enough` with its `tokens_per_call` counts (0 vs 0 when it was never used). Passing comparisons are ranked by absolute delta (ties: server, measure). `findings` = those where the server side is better (lower tokens or cost per call, higher good-result share) by at least 5 % (`MIN_GAP_PCT`, owner 2026-11-03; a smaller gap goes to `other`), each with the server's fixed habit, top `limit` (default 3, max 10). `other` = every other passing comparison, same ranking, `habit` null, not cut by `limit`. Text is always "linked to" (e.g. "Sessions that used easy-ui are linked to 12.3% fewer tokens per tool call than similar sessions without it (27 vs 62 sessions).").
- **Good result** (FR-061, with the amendments below): branch not in `main`/`master`/`HEAD`/none, one branch per session (the branch of most events; a tie = unknown), the owner project is a git repo, and a local commit (committer time) on `refs/heads/<branch>` in [first event, last event + 30 min]. Ref gone: commits found through a merge commit whose subject uses an FR-057 form for that branch (`<merge>^1..<merge>^2`); none found = unknown. A branch with commits but none in the window is not good. Error rate = failed tool results / tool calls (no calls = 0); a session with tool calls and no `user` record that carries a count (`SUM(tool_error_count)` is NULL) is unknown, never 0. Good = commit found and rate <= `kit_error_rate`. A "typed prompt" user record has a NULL count by design (FR-059), so unknown means "no user record of the session was re-read".
- **`kit_error_rate`**: `thresholds:` key in `~/.easy-personal/config.yaml` (a non-number or unknown key is refused with the same message as the rule thresholds). **Default 0.04** (unchanged by the 2026-11-03 revision) = the median session tool-error rate on the owner's store after HUN-192's re-read (590 sessions with tool calls and a known rate: median 0.0433, rounded to 2 decimals; 0.0403 over all 602 sessions with a known rate; 2026-11-03).
- **Git**: `git rev-list` only, local refs, never a fetch; branch text is never passed as an option (names with spaces, `..`, `@{`, `^`, `~`, `:`, a leading `-` are unknown). One total budget of 2 s per call (`kitperf.GIT_BUDGET_SECONDS`); whatever is not checked in time is unknown. One lookup per (repo, branch) per call, limited to the commit window the sessions on it need. Two sessions on one branch at the same time both get the commit (accepted).
- **Not built (cut list)**: dashboard page, `/api` route, per-account or per-workspace filter, confidence intervals or p-values, kernel skill comparison, stored findings. `wake` reads the tool through XR-2.

---

## Critical Constraints

- Read-only on every source: never modify `~/.claude/projects/`, kit traces, report folders, `~/.easy-context/`, the easy-memory store (`~/.easy-memory/`) or any repo. easy-memory is read as files, never through its MCP (FR-051), and its `trash/` is never read.
- Store and emit **metadata only**: no message text, tool input/output, trace `summary`, easy-ui `target`, file contents or secrets in the store, MCP/API responses, page, or logs. Named exception (owner, 2026-10-31): the Work view may *show, never store* board task titles (FR-037) and memory note frontmatter `name` + `description` (FR-041); note bodies and job arguments stay banned.
- No traffic leaves the machine: local calls to `127.0.0.1` only; every HTTP surface binds `127.0.0.1`, never `0.0.0.0`; the page loads no remote asset (CSP `default-src 'self'`).
- The only runtime call to another plug-in is the read-only easy-context MCP client (allow-list: `list_indexes`, `get_stats`). Never `index_path`; never call easy-verifier or easy-ui at runtime (DDR-0002).
- Deterministic: no LLM in the server; every warning cites rule id + measured value.
- Kit findings say "linked to", never "caused"; habit texts come from a fixed table in `core/kitperf.py`. `skills/wake` is kit-managed (`.claude/harness-lock.json`): never edit it here; its call to `kit_findings` is XR-2 on the kernel repo.
- Writes allowed only to `~/.easy-personal/`, the OS keychain (service prefix `easy-personal/`) and Claude Code user scope **via `claude mcp add-json|remove --scope user`** (DDR-0001). Never `~/.claude.json` by direct edit; never project-scope config or hooks (kernel-owned).
- No MCP tool may write config, create a connector or return a secret value. Dashboard writes need the per-launch token and a matching `Host`/`Origin`.
- `launch.py` imports only stdlib + `keyring`; secrets reach the child process only through its environment, never argv.
- Sibling repos are conventions-only references; never import or vendor their code. Cross-repo needs go to that repo's board (XR-1).
- Tool surface is frozen once approved; new tools need owner approval. Approved 2026-10-31: `usage_summary` / `usage_breakdown` gain typed / agent / unknown token fields (FR-039); no new tools.
- npm: exact versions in `package-lock.json`, `npm ci` only; the built page ships in the package so install needs no Node.

---

## Known Risk Areas

| Area | Risk Level | Reason | Files |
|------|-----------|--------|-------|
| Transcript parsing | High | Undocumented, changing format; silent mis-parse = wrong numbers | `sources/claude_code.py` |
| Content leakage | High | Transcripts, trace `summary`, easy-ui `target` and easy-memory notes (`prompt`, `answer`) hold private text | `sources/`, `core/store.py`, `core/easymem.py`, all surfaces |
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
| HUN-56 | README: install, usage and deploy guide for v0.1.0 | Todo | Backend-Implementer | C1 | Low | P1 |
| HUN-57 | Landing page `site/` (idea, flow, install & deploy guide); scope `PROJECT_SPEC_SITE.md` | Todo | Frontend-Implementer | C2 | Low | P1 |

**Workspaces** (Stage 2, 2026-10-26, owner-approved; brainstorm `BRAINSTORMING_LOG_WORKSPACES.md`, PRD FR-028…FR-036, NFR-009):

| ID | Title | Status | Assigned Agent | Complexity | Risk | Priority |
|----|-------|--------|---------------|-----------|------|----------|
| HUN-61 | Store foundation: versioned migrations, FK + busy_timeout, time-first usage query + indexes, 1M-event benchmark | Todo | Backend-Implementer | C2 | Medium | P0 |
| HUN-62 | Agent accounts: detect `~/.claude*` / `~/.codex*`, confirm/name/ignore (CLI + API) | Todo | Backend-Implementer | C2 | Medium | P0 |
| HUN-63 | Ingest every confirmed Claude account; events tagged by account; `--account` filter | Todo | Backend-Implementer | C2 | High | P1 |
| HUN-64 | Workspaces CRUD + repos + built-in orphan; `--workspace` usage filter | Todo | Backend-Implementer | C2 | Medium | P0 |
| HUN-65 | Connector id + alias in workspaces; accounts pick workspaces; plan/apply sync per Claude account | Todo | Backend-Implementer | C3 | High | P0 |
| HUN-66 | Move / alias / workspace rename / delete → orphan | Todo | Backend-Implementer | C2 | High | P1 |
| HUN-67 | One-time adoption of pre-workspace connectors into orphan | Todo | Backend-Implementer | C1 | High | P1 |
| HUN-68 | Dashboard Agents page | Todo | Frontend-Implementer | C2 | Medium | P1 |
| HUN-69 | Dashboard Workspaces page + form + legacy banner | Todo | Frontend-Implementer | C2 | High | P1 |
| HUN-70 | Usage page Workspace / Account filters | Todo | Frontend-Implementer | C1 | Low | P2 |
| HUN-71 | QA end-to-end across fake accounts | Todo | QA-Automation-Agent | C2 | Medium | P1 |

Owner run request 2026-10-29: "do all by parallel with tag [auto], merge to main and push".

**Dashboard fixes** (Stage 2, 2026-10-29, owner issues #1–#7; owner decisions: global scope selector, status-line card with plan-limit snapshot, no change to the owner's store now). Waves: 1 = HUN-113, 114, 115, 117 (parallel); 2 = HUN-116 (after 114+115), HUN-118 (after 114+117); 3 = HUN-119, HUN-120 (after 118).

| ID | Title | Status | Assigned Agent | Complexity | Risk | Priority |
|----|-------|--------|---------------|-----------|------|----------|
| HUN-113 | Workspaces page renders without waiting for the legacy scan; legacy checks run in parallel | Todo | Backend-Implementer | C1 | Low | P0 |
| HUN-114 | Agents: re-choose ignored accounts, rename/ignore confirmed ones; Usage explains ignored accounts | Todo | Frontend-Implementer | C1 | Medium | P0 |
| HUN-115 | Account status-line data in /api/accounts: email, plan, last model, tokens, plan-limit snapshot | Todo | Backend-Implementer | C2 | Medium | P1 |
| HUN-116 | Agents account cards read like the status line (email, model, tokens, 5h/7d limit bars) | Todo | Frontend-Implementer | C1 | Low | P1 |
| HUN-117 | Warnings, plan and kit accept workspace/account scope (API, MCP, CLI) | Todo | Backend-Implementer | C2 | Medium | P1 |
| HUN-118 | Global Workspace/Account selector in the sidebar for Usage, Warnings and Kit | Todo | Frontend-Implementer | C2 | Medium | P1 |
| HUN-119 | Kit tab: plain project cards with health, recent activity and a details toggle | Todo | Frontend-Implementer | C2 | Low | P1 |
| HUN-120 | Warnings & plan: plain titles, why it matters, numbered steps, one ordered action list | Todo | Backend-Implementer | C2 | Low | P1 |
| HUN-121 | Restyle dashboard to reference image 2: indigo + monospace, grouped sidebar, right rail, timeline feed cards (added mid-run 2026-10-29; wave 4, after HUN-116, 118, 119, 120) | Todo | Frontend-Implementer | C2 | Low | P1 |
| HUN-126 | Restyle dashboard to the Graphite Mint design (owner canvas GGoKuwVrYPQAVGcT6e7Ptq); plain Warnings/Kit content kept (owner choice 2026-10-29); UI checked with easy-ui MCP | Todo | Frontend-Implementer | C2 | Low | P1 |
| HUN-128 | Dashboard shell in a centered max-width container (1440 px, owner request 2026-10-29) | Todo | Frontend-Implementer | C1 | Low | P1 |
| HUN-129 | Usage "By model" card: short headers, no clipped cost column (owner "fix", 2026-10-29) | Todo | Frontend-Implementer | C1 | Low | P1 |
| HUN-160 | Work view rail regroup (`// work` `// usage` `// kit` `// connect` `// system`) + first-run checklist (FR-042) | Done (merged 2026-10-31) | Frontend-Implementer | C2 | Low | P0 |
| HUN-161 | Typed vs agent usage split: `entrypoint` migration + backfill, MCP split fields (FR-039; migration-safety) | Done (merged 2026-10-31) | Backend-Implementer | C2 | Medium | P0 |
| HUN-162 | Work view: read-only cross-project board from `PROJECT_KANBAN.md` (FR-037/038, NFR-010; after HUN-160) | Done (merged 2026-10-31) | Frontend-Implementer | C2 | Low | P0 |
| HUN-163 | Work view: scheduled jobs, systemd timers + cron, first word only (FR-040; after HUN-160) | Done (merged 2026-10-31) | Frontend-Implementer | C2 | Medium | P1 |
| HUN-164 | Work view: memory graph, names + links read live, rings layout (FR-041, NFR-010; after HUN-160) | Done (merged 2026-10-31) | Frontend-Implementer | C2 | Medium | P1 |
| HUN-165 | Live trace spike: harness + baseline for the standard test session (`experiments/live-trace/`, owner-approved 2026-10-31) | Done (merged 2026-10-31) | QA-Automation-Agent | C1 | Medium | P1 |
| HUN-166 | Live trace E1: live tail of transcripts with the existing reader (after HUN-165) | Done (merged 2026-10-31) | QA-Automation-Agent | C1 | Medium | P1 |
| HUN-167 | Live trace E2: native async http hooks to a local collector (after HUN-165) | Done (merged 2026-10-31) | QA-Automation-Agent | C1 | Medium | P1 |
| HUN-168 | Live trace E3: OpenTelemetry to a local OTLP receiver (after HUN-165) | Done (merged 2026-10-31) | QA-Automation-Agent | C2 | Medium | P1 |
| HUN-169 | Live trace E4: stream-json capture from headless runs (after HUN-165) | Done (merged 2026-10-31) | QA-Automation-Agent | C1 | Low | P1 |
| HUN-170 | Live trace report: combined run, E5 docs-only (no Admin key), REPORT.md (after HUN-166..169) | Done (merged 2026-10-31) | QA-Automation-Agent | C1 | Low | P1 |
| HUN-175 | Transcript reader: 8 record types new in Claude Code 2.1.316 (FR-046; v0.3.0) | Done (merged 2026-11-01) | Backend-Implementer | C1 | Medium | P1 |
| HUN-176 | Live tail while the dashboard runs, lock-friendly (FR-043; after HUN-175) | Done (merged 2026-11-01) | Backend-Implementer | C2 | Medium | P0 |
| HUN-177 | `GET /api/live` SSE stream, guarded, resumable (FR-044; after HUN-176) | Done (merged 2026-11-01) | Backend-Implementer | C2 | Medium | P0 |
| HUN-178 | Live view: active sessions, tool feed, running totals (FR-045; after HUN-177) | Done (merged 2026-11-01) | Frontend-Implementer | C2 | Low | P0 |
| HUN-179 | Release 0.3.0: version, README, site, ship plan (after HUN-178) | Done (merged 2026-11-01) | Backend-Implementer | C1 | Low | P1 |
| HUN-181 | Live attention states in the live tail + stream re-send (FR-047..049, NFR-012; v0.4.0) | Done (merged 2026-11-02) | Backend-Implementer | C2 | Medium | P0 |
| HUN-182 | Live view attention badge, border, header count, sort (FR-050; after HUN-181) | Done (merged 2026-11-02) | Frontend-Implementer | C2 | Low | P0 |
| HUN-183 | Release 0.4.0 (widened 2026-11-03): live attention + easy-memory + UX pass + kit performance, v8 upgrade note | Done (merged 2026-11-03) | Backend-Implementer | C1 | Low | P1 |
| HUN-184 | easy-memory store reader + cited-file nodes and panel data in /api/memory-graph (FR-051..053 backend) | Done (merged 2026-11-02) | Backend-Implementer | C2 | Medium | P1 |
| HUN-185 | Memory view easy-memory layer: cited-file nodes, stale mark, 30-day panel, list count (FR-052..053 page; after HUN-184) | Done (merged 2026-11-02) | Frontend-Implementer | C2 | Medium | P1 |
| HUN-186 | Dashboard speed + gone-project filter (FR-054, NFR-013) | Done (merged 2026-11-02) | Backend-Implementer | C2 | Medium | P0 |
| HUN-187 | Board view UX: one project at a time, hide no-board projects, plain clamped titles, phone width (FR-055) | Done (merged 2026-11-02) | Frontend-Implementer | C2 | Low | P0 |
| HUN-188 | Usage, Memory and Kit view UX: one-line numbers, distinct model names, Memory default, Kit fold (FR-054, FR-056) | Done (merged 2026-11-02) | Frontend-Implementer | C2 | Low | P1 |
| HUN-189 | Owner project: count gone worktrees under their main repo in usage by project, top-project warning and project lists (FR-057) | Done (merged 2026-11-02) | Backend-Implementer | C2 | Medium | P1 |
| HUN-190 | Usage lists show top 10 with "Show N more" (By project, model, tool, outlier sessions) (FR-058) | Done (merged 2026-11-02) | Frontend-Implementer | C1 | Low | P1 |
| HUN-191 | Session timeline shows 10 sessions with "Show N more"; sessions past the 50 with chart detail listed without a chart (FR-058) | Done (merged 2026-11-02) | Frontend-Implementer | C1 | Low | P1 |
| HUN-192 | Store failed tool results per user record: migration 7 → 8 + re-read (FR-059) | Done (merged 2026-11-03) | Backend-Implementer | C2 | Medium | P0 |
| HUN-193 | kit_findings: like-with-like session comparison per kit server, MCP tool + CLI (FR-060..FR-063, NFR-014; after HUN-192) | Done (merged 2026-11-03) | Backend-Implementer | C2 | Medium | P0 |
| HUN-195 | kit_findings: a finding needs a gap of at least 5% to carry a habit (FR-062, owner 2026-11-03) | Done (merged 2026-11-03, owner approved) | Supervisor | C1 | Low | P1 |
| HUN-196 | Cache reuse card follows the app-wide period, with written-to-cache and lowest-model lines (FR-064) | Done (merged 2026-11-03) | Frontend-Implementer | C2 | Low | P1 |
| HUN-198 | Memory panel: memory on vs off block from easy-memory report.json (FR-066) | Done (merged 2026-11-03) | Supervisor | C2 | Low | P2 |
| HUN-200 | Dashboard form UX, floating toasts, floating loading pill (FR-065) | Done (merged 2026-11-03) | Frontend-Implementer | C2 | Low | P1 |
| HUN-202 | Toasts inside the page box; loading pill in the content middle (FR-065 follow-up) | Done (merged 2026-11-03) | Frontend-Implementer | C1 | Low | P1 |
| HUN-203 | Repos folder browser + gap above Save (FR-068) | Done (merged 2026-11-03) | Frontend-Implementer | C2 | Medium | P1 |
| HUN-205 | Easy Kit update 75040d9 → 04b019a (minimal track) | Done (merged 2026-11-04) | Supervisor | C1 | Low | P1 |
| HUN-206 | Scratch row in project lists + `GET /api/workspaces/unassigned` (FR-069, FR-070 read side) | Done (merged 2026-11-04) | Backend-Implementer | C2 | Medium | P1 |
| HUN-207 | Usage opens Scratch; Workspaces page lists repos in no workspace + Add to (FR-069 UI, FR-070; after HUN-206) | Todo | Frontend-Implementer | C2 | Medium | P1 |
| HUN-209 | Store migration 9: clear stale `abstained` on Claude Code metadata rows (false `records_unreadable`; FR-003, FR-026) | Done (merged 2026-11-04) | Backend-Implementer | C1 | Medium | P1 |
| HUN-212 | Live tail publishes lines another process stored first (FR-071; two dashboards / scheduled ingest) | Done (merged 2026-11-04) | Backend-Implementer | C2 | Medium | P0 |
| HUN-213 | Live view session rows with bars + one session panel, batched render (FR-072, NFR-015) | Done (merged 2026-11-04) | Frontend-Implementer | C2 | Low | P1 |
| HUN-214 | Warning hits carry causes from the owner's data + diagnosis-picked fixes (FR-073) | Done (merged 2026-11-04) | Backend-Implementer | C2 | Medium | P1 |
| HUN-215 | MCP prompt `investigate`: owner's Claude Code session investigates usage (FR-075) | Done (merged 2026-11-04) | Backend-Implementer | C1 | Low | P1 |
| HUN-216 | Warnings & plan page: one list, "Why this happened", matching fixes, investigate hint (FR-074; after HUN-214) | Done (merged 2026-11-04) | Frontend-Implementer | C2 | Low | P1 |
| HUN-194 | Release 0.5.0 kit performance: version, README, site (after HUN-193 and HUN-183) | Closed (duplicate of HUN-183, 2026-11-03) | Backend-Implementer | C1 | Low | P1 |

Cross-repo: **XR-1** = easy-verifier-mcp **T120** (`../easy-verifier-mcp/tasks/TASK_GUIDE_T120.md`, its local board) — score JSON sidecar next to each HTML report — **Done 2026-10-21**. Needed by FR-011 / HUN-42 (M2).

Cross-repo: **XR-2** = Easy Kit kernel (owner of `skills/wake`) — `wake` calls `kit_findings` when the easy-personal MCP is connected, ≤ 3 lines, silent on failure — **not filed yet**: goes on the kernel repo's own board. Needed by FR-063.

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
