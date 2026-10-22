# PROJECT_SPEC — easy-personal Landing Site (v1)

**Scope boundary:** this document governs the public static site under `site/` only. The product stays on `PROJECT_SPEC.md`.

**Board:** site tasks stay on the one Linear-backed board (`PROJECT_KANBAN.md`). User-approved exception to Hard-Stop Gate 4 (2026-10-22, "Yes, as described"), same as easy-context-mcp: `board_sync.py` serves exactly one Linear project, so a second board would drift. See `memory/decisions.md` → "Landing site: sibling pattern, one board".

Pattern reference: `../easy-context-mcp` (`site/index.html`, `vercel.json`, `.vercelignore`, `PROJECT_SPEC_SITE.md`, `tests/test_site_content.py`, `tests/test_site_theme.py`, `tests/test_site_deploy_config.py`, task HUN-20 there).

## Purpose

One short public page that lets a developer arriving from GitHub catch the idea quickly and install, run and deploy it. It is a reference surface, not a project surface: never KANBAN rows, task IDs, memory contents, DDR internals or in-flight work — and never the owner's real usage data.

## Decisions (owner, 2026-10-22)

| Question | Answer |
|---|---|
| Pattern | Copy easy-context / easy-verifier: `site/index.html`, Vercel Git integration from `main`, drift tests |
| Board | Same Linear board (Gate 4 exception, above) |
| Content + order | 1 Hero · 2 Flow diagram · 3 Dashboard screenshots (fixture data only) · 4 Install & deploy guide · 5 MCP tools · 6 Connector catalog · 7 Footer |
| Look | Easy Kit dark neon theme, as the sibling sites |
| Vercel project | Connected by the owner (operator action), not by a task |

## Page content

1. **Hero** (`#hero`) — name, the pitch line verbatim from `README.md`, GitHub link.
2. **Flow** (`#flow`) — one inline diagram (SVG or HTML/CSS, no library), two lanes: *Read* (Claude Code transcripts + kit reports/traces → `ingest` (metadata only) → local SQLite store → MCP tools · CLI · dashboard on 127.0.0.1) and *Connect* (catalog form → secrets to the OS keychain → `claude mcp add-json` registers `easy-personal-launch exec <id>` → the launcher injects secrets at start). One-line caption per step.
3. **Dashboard** (`#dashboard`) — 2–4 screenshots of the dashboard tabs taken from test fixtures only (committed under `site/`), with alt text.
4. **Install & deploy** (`#install`) — install, first run, MCP registration, dashboard, schedule, connectors, upgrade and rollback; every command verbatim from `README.md`; link to `RUNBOOK.md` on GitHub for the full procedure.
5. **MCP tools** (`#tools`) — every tool `surfaces/mcp.py` registers, one line each.
6. **Connectors** (`#connectors`) — the catalog providers grouped by category, claude.ai-connector flag shown.
7. **Footer** — GitHub repo, README, https://personal-agentic-claude.vercel.app/, https://easy-verifier.vercel.app/.

## Stack

Plain HTML + CSS in `site/index.html` (plus `site/favicon.svg`, screenshots). No framework, no build step, no `package.json`, no external hosts (fonts, CDNs, scripts), no analytics. `vercel.json` = `{"outputDirectory": "site"}`; `.vercelignore` allowlist `*`, `!site/`, `!site/**`, `!vercel.json`.

## The drift constraint

| Site content | Source of truth |
|---|---|
| Pitch line | `README.md` (verbatim) |
| Commands in `#install` | must appear verbatim in `README.md` |
| Tool names | tools registered by `src/easy_personal/surfaces/mcp.py` (derived in the test, not hard-coded) |
| Providers | `src/easy_personal/catalog/*.yaml` (derived in the test) |

## Safety constraints

Nothing under `site/` contains an absolute host path (`/home/`, `/Users/`, `/workspace`, `/tmp/`), an email address, a token-shaped string or a dashboard token; screenshots come from fixtures (`tests/fixtures/`), never the owner's store. External links only to the GitHub repo and the sibling sites.
