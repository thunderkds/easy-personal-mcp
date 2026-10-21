# PROJECT_SPEC.md
**Last updated**: 2026-10-21
**Version**: 0.1 (draft from BRAINSTORMING_LOG.md)

> **Scope of this document**: *How* to build it safely — architecture, agent config, constraints, risk areas, task state, and accumulated learnings.
> Product intent (personas, user stories, FR/NFR, success metrics) lives in `PRD.md`.
> If Critical Constraints here conflict with Out of Scope in `PRD.md`, resolve before Stage 2.

---

## Project Identity

- **Name**: easy-personal-mcp (MCP server id `easy-personal`, CLI `easy-personal`)
- **Repo**: `thunderkds/easy-personal-mcp`
- **Primary tech**: Python ≥ 3.11, official MCP Python SDK (FastMCP), SQLite (stdlib), `keyring` (v1.1), `uv`
- **Type**: MCP server (stdio default, streamable HTTP opt-in) + CLI + static HTML dashboard (v1) / local web dashboard (v1.1)
- **Deployment target**: the owner's machine, native install (`uv tool install`); Docker optional, analytics-only (DDR-0001)
- **Key stakeholders**: thunderkds (owner)

---

## Architecture Summary

Three layers in one package. **Sources** are read-only adapters that turn local files into normalized
metadata events (v1: Claude Code transcripts). **Core** owns the store (`~/.easy-personal/store.sqlite`, WAL),
metrics, declared rules, warnings and the improvement plan. **Surfaces** — MCP tools, CLI, dashboard — call
only the core, so every surface shows the same numbers. In v1.1 a separate **connectors** module (catalog,
keychain, `exec`/`headers` launcher, config writer) is added; core and sources never import it, so it can be
split into its own repo later.

```
src/easy_personal/
  sources/claude_code.py     versioned transcript parser → events (metadata only)
  core/{store,metrics,rules,plan,prices}.py
  surfaces/{mcp,cli,dashboard}.py
  connectors/                (v1.1) catalog/*.yaml, keychain.py, launcher.py, writers/{claude,codex,cursor}.py
```

---

## Glossary

- **Source** — a read-only adapter producing events from one local data origin.
- **Event** — one normalized metadata record (time, session, project, model, kind, token counts, tool name). Never content.
- **Abstained** — a record or metric the engine could not interpret; shown, never scored.
- **Rule** — a declared check: id, metric, threshold, label (`improve` / `aware` / `warning`), priority, fix line.
- **Connector** — (v1.1) a configured provider (e.g. Slack) = catalog entry + keychain refs + agent config entries.
- **Launcher** — (v1.1) `easy-personal exec|headers <connector>`: the command agent clients run, which injects secrets at launch.

---

## Critical Constraints

- Read-only on every source: never modify `~/.claude/projects/`, kit traces or any repo.
- Store and emit **metadata only**: no message text, tool input/output, file contents or secrets in the store, responses, reports or logs.
- No network at runtime; HTTP binds `127.0.0.1` only, never `0.0.0.0`.
- Deterministic: no LLM in the server; every warning cites rule id + measured value.
- Writes allowed only to `~/.easy-personal/` and the reports dir — plus, from v1.1, exactly the targets in DDR-0001 (keychain, official agent CLIs, `~/.cursor/mcp.json`). Never project-scope config or hooks (kernel-owned).
- No MCP tool may write config, create a connector or return a secret value.
- Sibling repos are conventions-only references; never import or vendor their code.
- Tool surface is frozen once approved; new tools need user approval.

---

## Known Risk Areas

| Area | Risk Level | Reason | Files |
|------|-----------|--------|-------|
| Transcript parsing | High | Undocumented, changing format; silent mis-parse = wrong numbers | `sources/claude_code.py` |
| Content leakage | High | Transcripts hold private text and pasted secrets | `sources/`, `core/store.py`, all surfaces |
| Launcher (v1.1) | High | In the start path of every connector; handles live secrets | `connectors/launcher.py` |
| Agent config writes (v1.1) | High | A bad write breaks Claude Code / Codex / Cursor | `connectors/writers/` |
| Incremental ingest | Med | Offsets, truncated last line, files deleted by cleanup mid-read | `sources/claude_code.py`, `core/store.py` |
| Cost estimates | Low | Price table goes stale | `core/prices.py` |

---

## Sub-Agent Team

| Agent | Role | CLI Spawn Command |
|---|---|---|
| Backend-Implementer | Sources, core, MCP/CLI | `Agent({ subagent_type: "backend-developer", prompt: "..." })` |
| Frontend-Implementer | Dashboard | `Agent({ subagent_type: "frontend-developer", prompt: "..." })` |
| QA-Automation-Agent | Fixtures, leak tests, parser tests | `Agent({ subagent_type: "qa-expert", prompt: "..." })` |

---

## CLI Model Table

| CLI | Run command | C0 | C1 | C2 | C3 |
|---|---|---|---|---|---|
| claude | `claude` | haiku | sonnet | sonnet or opus | opus |

---

## Tasks

| ID | Title | Status | Assigned Agent | Complexity | Risk | Priority |
|----|-------|--------|---------------|-----------|------|----------|
| HUN-21 | Spike: inventory Claude Code transcript shapes on the owner's machine → synthetic fixtures per version | Todo | QA-Automation-Agent | C1 | Med | P0 |
| HUN-22 | Repo skeleton: package layout, CLI + MCP entry points sharing one core, config dirs | Todo | Backend-Implementer | C1 | Low | P0 |
| HUN-23 | `claude_code` source: streaming, incremental, metadata-only parser with abstain counts | Todo | Backend-Implementer | C2 | High | P0 |
| HUN-24 | Store + metrics: summaries and breakdowns by day/model/project/tool; dated price table | Todo | Backend-Implementer | C2 | Med | P0 |
| HUN-25 | Rules engine + v1 rule pack (~8 rules) + improvement plan | Todo | Backend-Implementer | C2 | Med | P1 |
| HUN-26 | MCP tools (6) + CLI commands over the core | Todo | Backend-Implementer | C1 | Low | P1 |
| HUN-27 | Static HTML dashboard (charts, flows, warnings, plan) | Todo | Frontend-Implementer | C2 | Low | P1 |
| HUN-28 | Leak test suite: planted secrets/text in fixtures never reach any output or store row | Todo | QA-Automation-Agent | C1 | High | P0 |
| HUN-29 | Scheduled ingest (cron/launchd snippet or `SessionEnd` hook doc) so the 30-day cleanup loses nothing | Todo | Backend-Implementer | C1 | Med | P1 |

v1.1 (connectors, keychain, launcher, writers, live dashboard) is planned after v1 ships.

---

## Memory / Insights

| Date | Insight | Source Task |
|------|---------|------------|
| 2026-10-21 | Claude Code deletes local transcripts after 30 days (`cleanupPeriodDays`); ingest must be incremental and scheduled. | brainstorm |
| 2026-10-21 | The connector problem is secret *delivery*, not storage: every agent client spawns a command, so a launcher injects keychain secrets without any plaintext config. | brainstorm |
| 2026-10-21 | `~/.claude.json` also holds sign-in and trust; write MCP entries through `claude mcp add-json --scope user`, not by editing the file. | brainstorm |
