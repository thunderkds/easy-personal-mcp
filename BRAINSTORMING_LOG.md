# BRAINSTORMING_LOG.md
**Generated**: 2026-10-21 (rerun after the Phase 0 recheck and requirement grill)
**Task / Context**: easy-personal-mcp v1 — one release, internal milestones M1 usage → M2 kit → M3 connectors → M4 live page. Inputs: `PRD.md` (Approved, FR-001…FR-027, NFR-001…NFR-008), `docs/GRILL_requirement.md` (gate PASS).
**Skill**: `Skill({ skill: "brainstorming" })` — tier **Deep** (v1 scope ~3× the first plan; a live page with write actions; first plug-in that calls another plug-in at runtime)
**Supersedes**: the first log (commit `d9fcae0`), whose Option B and 50% cut (one source, static HTML, connectors in v1.1) the owner reversed. Its verified facts F1–F9 and decisions D1–D5 still hold except where noted below.

---

## The Problem Space

The subject is the person, not a repo. v1 must, on one Linux machine, with nothing leaving the machine:
1. Keep Claude Code usage beyond the 30-day transcript cleanup and turn it into tokens-first metrics, 7 declared rules and an improvement plan (M1).
2. Show what the Easy Kit did per discovered project — kernel traces, verifier scores, easy-context index freshness, local git — and easy-ui checks overall (M2).
3. Manage connectors for Claude Code with secrets only in the Secret Service keychain, injected at launch (M3).
4. Serve all of it on an on-demand live page on `127.0.0.1`, including connector forms (M4).

Non-negotiables: metadata only (NFR-002), read-only on sources (NFR-003), deterministic/no LLM (NFR-004),
`127.0.0.1` only (NFR-001), dashboard write guard (NFR-008), launcher < 200 ms with only `keyring` (NFR-006).

### Decisions locked in this session

| # | Question | Answer |
|---|---|---|
| B1 | ASCII mockup of the dashboard? | No — layout is a Stage 2 frontend decision. |
| B2 | How does the live page run? | **On demand**: `easy-personal dashboard` ingests, serves `127.0.0.1`, opens the browser; Ctrl-C stops it. A systemd user timer runs ingest on schedule. No always-on service. |

### Verified facts added this run

| # | Fact | Source |
|---|---|---|
| F10 | The MCP Python SDK already depends on `starlette`, `uvicorn`, `httpx`, `sse-starlette` — a web server and an HTTP MCP client come with no new dependency. | `easy-context-mcp/uv.lock`, package `mcp` |
| F11 | easy-context serves MCP at `http://127.0.0.1:8767/mcp`, expects header `X-Easy-Context-Root`, exposes `list_indexes` / `get_stats`. | `easy-context-mcp/README.md` |
| F12 | easy-verifier writes HTML only (`evidence-report-*.html`) to `<repo>/reports/` or `~/.easy-verifier-reports/<repo>/`; MCP on `127.0.0.1:8766` runs a *new* scoring. | `easy-verifier-mcp/README.md`, `~/.easy-verifier-reports/` |
| F13 | easy-ui writes `session-<id>.json` (`id`, `target`, `status`, `startedAt`, `endedAt`, `actions`) into its server cwd `reports/`; 206 of 222 sessions sit in `easy-ui-mcp/reports/`. | `easy-ui-mcp/src/server.ts`, file scan |
| F14 | Kernel trace lines: `timestamp`, `tool_name`, `summary` (raw tool input), `is_error`; files named by task id, plus `_untagged.jsonl`. | `easy-verifier-mcp/memory/event-trace/T001.jsonl` |

### Changes to earlier decisions

- **D3 (config writers)**: Codex and Cursor writers removed (PRD Out of Scope). Only `claude mcp add-json --scope user`, snippet fallback.
- **D6 (dashboard form)**: live page in v1, on demand (B2) — replaces "static HTML v1 → live server v1.1".
- **D1 (data)**: still transcripts only for usage; OTel stays a later option.

---

## Questions for the User

All resolved (B1, B2 above; PRD grill G1–G15, N1–N7). One choice remains: the path below.

---

## Alternative Paths

| Option | Name | Summary | Invasiveness | Code Volume | Regression Risk | Recommended? |
|--------|------|---------|-------------|------------|----------------|--------------|
| A | One package, server-rendered page | Python only. Starlette page from the MCP SDK's deps, HTML built in Python, small vanilla JS, vendored chart lib; isolated launcher module | Low | ~5–6k lines | Low–Med | Supervisor's pick |
| B | Python core + SPA frontend | Same core; dashboard is a React/Vite app built into the package, talking to a JSON API | Medium | ~8–9k lines + Node toolchain | Medium | ✅ **Owner's selection** |
| C | Read-only page, CLI-only writes | Same as A, but the page never writes; connectors are set up only with `easy-personal connect` in the terminal | Low | ~4.5–5k lines | Low | |

### Option A — One package, server-rendered page
**Approach**:
```
src/easy_personal/
  sources/     claude_code · kit_trace · verifier · context (MCP client) · ui · git_local   → normalized events
  core/        store (SQLite WAL, single-writer lock) · discovery (projects, worktree grouping)
               metrics · rules (7, declared) · plan · price table
  surfaces/    mcp (stdio default, HTTP opt-in) · cli · dashboard (Starlette, on demand)
  connectors/  catalog/*.yaml · keychain · writer (claude mcp add-json) · suggestions
  launch.py    `exec` / `headers` — imports only stdlib + keyring (import test enforces it)
```
The dashboard returns server-built HTML; charts use one pinned, vendored JS library shipped inside the
package (no CDN). Connector forms POST with a per-launch token and a `Host`/`Origin` check.
**Pros**: one language, one test runner; no new dependencies (F10); the vendored asset keeps the browser
off the internet; matches the siblings' Python layout.
**Cons**: hand-built HTML is plainer; interactive charts limited to what the chosen library gives.
**Why it might fail**: the page grows into an ad-hoc template mess — mitigate with one small render module
and HTML snapshot tests; the launcher drifts into importing the core and breaks NFR-006 — mitigate with an
import-graph test and a start-time test.

### Option B — Python core + SPA frontend
**Approach**: Option A's core and API, plus `web/` (React + Vite + a chart library), built at release time
into `src/easy_personal/dashboard/static/`.
**Pros**: richest interactivity; component tests (Vitest); easier to grow toward a v2 "life" dashboard.
**Cons**: two toolchains (uv + npm), a build step in release, a lockfile with hundreds of npm packages for a
single-user local page; more supply-chain surface for a tool that holds the keychain.
**Why it might fail**: the build step breaks `uv tool install` from source; npm dependency churn creates
maintenance with no user-visible gain at v1 scope.

### Option C — Read-only page, CLI-only writes
**Approach**: Option A, except the page has no forms. `easy-personal connect` runs an interactive terminal
wizard (pick category → provider → fields; secrets typed with hidden input) and the page only shows
connector status and suggestions.
**Pros**: the page has no write surface, so NFR-008 and its CSRF/DNS-rebinding risk disappear; the terminal
is a natural place to type secrets; least code.
**Cons**: contradicts FR-023 "connector forms" on the page (needs a PRD change); less guided for the owner.
**Why it might fail**: the owner expects to click through providers on the page; the wizard becomes the
second-class path nobody uses.

---

## 50% Rule Check

For Option A at half the code: drop the `ui` source and the `kit_summary` easy-ui part (owner sees easy-ui
reports directly), drop connector suggestions (FR-022), make the page read-only (Option C's cut), and keep
`git_local` to "commits in period". That roughly halves M2–M4. It is **not** recommended because the owner
explicitly chose all four kit parts and connector forms; it is recorded as the cut list if v1 slips.

---

## Recommended Path

**Selected: Option B — Python core + SPA frontend** (owner, 2026-10-21). The Supervisor had recommended
Option A; the reasoning is kept below for the record. Everything else in this section (milestones, XR-1) applies to B unchanged,
except M4 builds the page as `web/` (React + Vite) over a JSON API served by the on-demand Starlette process.

**Supervisor's original recommendation: Option A — One package, server-rendered page.**

It meets every FR with one language and no new dependency, keeps the browser and the server on the
machine, and keeps the secret-handling launcher small and testable on its own. B buys interactivity the
owner did not ask for at the cost of a second toolchain; C is safer but reverses a requirement the owner set.

### Milestone shape (input to Stage 2 `to-issues`)

| Milestone | Vertical slices (candidates) |
|---|---|
| M1 usage | spike: transcript shapes + real volume (resets NFR-005) · skeleton (package, CLI + MCP share core) · `claude_code` source · store + metrics (tokens-first, local-day) · rules engine + 7 rules + plan · MCP usage tools + CLI · scheduled ingest (systemd timer) · leak test suite |
| M2 kit | project discovery + worktree grouping · `kit_trace` · `context` MCP client + `context_index_stale` · `verifier` JSON (after XR-1) · `ui` overall · `git_local` · `kit_summary` |
| M3 connectors | catalog files · keychain store + refusal path · `launch.py` exec/headers · Claude Code writer + snippet fallback · `connector_*` MCP tools · suggestions |
| M4 live page | on-demand Starlette page + token/Origin guard · usage + session timeline + warnings/plan · kit panel · connector forms · UI evidence via easy-ui (dev-time) |

Cross-repo: **XR-1** (easy-verifier writes score JSON) goes on easy-verifier-mcp's own board.

---

## Surgical Scope

Files that **should** be touched (this repo):
- `src/easy_personal/{sources,core,surfaces,connectors}/`, `src/easy_personal/launch.py`
- `web/` — React + Vite app (chart library chosen in Stage 2), `package-lock.json` committed
- `src/easy_personal/dashboard/static/` — the built `web/` output served by the on-demand Starlette process
- `catalog/*.yaml` — provider catalog
- `tests/` with `tests/fixtures/<source>/<version>/` — synthetic data only, never real transcripts or traces
- `PRD.md`, `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `docs/ddr/` — planning artifacts
- Board-sync config — limit Linear sync to the `easy-personal-mcp` project (Stage 1)

Files that **must not** be touched:
- `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` — read-only sources
- `~/.claude.json` by direct edit — only through `claude mcp add-json` / `claude mcp remove`
- Project-scope `.mcp.json`, `.claude/settings.json`, hooks — kernel-owned
- Sibling repos' code — XR-1 is done on easy-verifier's own board, not from here

---

## Edge Case Checklist for TASK_GUIDE

Ingest and store
- [ ] Transcript or trace file being written during ingest → consume only up to the last complete line; the offset never points mid-line.
- [ ] Source file deleted by the 30-day cleanup → stored rows stay; its offset entry is retired, not reset.
- [ ] Timer ingest and dashboard ingest at the same time → one writer (lock + WAL); the second waits or skips with a message.
- [ ] Unknown record shape or unknown source schema version → counted as abstained, shown, never coerced.
- [ ] Planted secrets/text in every source fixture (transcript content, trace `summary`, easy-ui `target`) never reach any store row, response, page or log.
- [ ] Sessions spanning midnight and DST changes → assigned by local day consistently; stored in UTC.
- [ ] Model missing from the price table → cost abstains for that model; tokens still shown.

Kit sources
- [ ] easy-context down, slow or refusing → short timeout, "no data", never a crash or a hang of the page.
- [ ] easy-context's `X-Easy-Context-Root` header vs. `list_indexes` scope — confirm in the M2 spike whether one call lists all repos or one call per repo is needed.
- [ ] easy-personal never calls `index_path` or any non-read tool (tool allow-list test).
- [ ] Worktrees (`repo-T025`) group under the main repo via `git rev-parse --git-common-dir`; a deleted worktree path drops out of discovery without errors.
- [ ] Verifier HTML without JSON → "no data"; JSON with an unknown schema version → abstain.
- [ ] easy-ui folder missing or moved → "no data"; path configurable.

Connectors and launcher
- [ ] Secret Service missing or locked → setup refuses with guidance; `exec` exits non-zero with a one-line stderr message; never plaintext.
- [ ] Secrets go to the child only through the environment (`os.execvpe`), never in argv (visible in `ps`).
- [ ] `claude` CLI missing → show the snippet, write nothing; name already exists in user scope → refuse and show both entries.
- [ ] `launch.py` imports only stdlib + `keyring` (import test); start overhead < 200 ms (timing test).

Live page
- [ ] Every write needs the per-launch token and a matching `Host`/`Origin`; requests from another origin or a rebinding host get 403.
- [ ] The token never appears in logs (server access log off or redacted); secret fields are never echoed back in HTML.
- [ ] All assets are served locally (CSP `default-src 'self'`); no CDN request from the browser.
- [ ] Port in use → pick a free port and print the URL; bind `127.0.0.1` only (test refuses `0.0.0.0`).
- [ ] (Option B) The JSON API is the only data path: every API response passes the same leak test as MCP responses.
- [ ] (Option B) `uv tool install` works without Node on the owner's machine → the built assets ship in the package; a missing build fails the release check, not the install.
- [ ] (Option B) npm supply chain: exact versions in `package-lock.json`, `npm ci` only, no runtime CDN, no analytics/telemetry packages; dependency count reviewed at Stage 4.
- [ ] (Option B) Dev server (Vite) never used at runtime; proxy/CORS settings from dev never ship.
- [ ] Page states that cloud sessions and other machines are not included.

---

## Next Actions

1. Owner selects a path (A recommended).
2. Update DDR-0001: drop the Codex/Cursor writers, note the on-demand live page in v1.
3. Write DDR-0002: easy-personal acts as a read-only MCP client of easy-context on `127.0.0.1` (first runtime plug-in-to-plug-in call; allow-list of read tools; "no data" on failure).
4. Stage 1: fix the board sync to the `easy-personal-mcp` project; restore the board; set aside the 20 foreign task guides in `tasks/`.
5. Open XR-1 on easy-verifier-mcp's board.
6. Stage 2: run `to-issues` on the milestone table above; rebuild `PROJECT_SPEC.md` tasks and KANBAN from it.

---

## User Selection

> **Approved direction**: Option B — Python core + SPA frontend (React + Vite over a JSON API), on-demand page (B2), milestones M1–M4.
> Approved by user on 2026-10-21.
