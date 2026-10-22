# TASK_REVIEW — HUN-56: README install, usage and deploy guide

> Sibling of `tasks/TASK_GUIDE_HUN-56.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/test_readme.py` (13 tests: AC1 section order + v0.1.0 status; AC2 commands/flags walked through `build_parser()` + launcher usage, every top-level subcommand documented; AC3 MCP tools = `create_app().list_tools()`; AC4 install/upgrade = RUNBOOK; AC5 no host path/email/token). Agent's run on the OLD README: `9 failed, 4 passed`; after: `13 passed in 0.39s`. Supervisor: `uv run pytest -q` → `385 passed, 1 warning in 27.93s` |
| Verification command run | ☑ pass | `uv run pytest tests/test_readme.py -q && uv run pytest -q` → 13 passed; 385 passed; ruff `All checks passed!` |
| Negative cases hold | ☑ pass | test fails when a README command/flag is not in the parser, a tool list differs, or a host path/email/token appears (demonstrated by the 9 failures on the old README) |
| verify | ☑ pass | Supervisor followed the README in an isolated env (temp EASY_PERSONAL_HOME/UV_TOOL_*, fixtures, failing keyring, no `claude` on PATH): `npm --prefix web ci && npm --prefix web run build` → `✓ built`; `uv tool install --reinstall --no-cache .`; rc=0 for `--version`, `ping`, `ingest`, `usage`, `usage --by project`, `outliers`, `warnings`, `plan`, `kit`, `connector list`, `schedule install --dry-run`; `easy-personal-launch` installed; `doctor` rc=1 with `FAIL keychain`/`FAIL claude` as documented; `dashboard` printed `http://127.0.0.1:<port>/`. `claude mcp add --scope user easy-personal -- easy-personal mcp` matches `claude mcp add --help` usage (`claude mcp add [options] <name> <commandOrUrl> [args...]`); not executed (never registers on the owner's config) — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | README.md + tests/test_readme.py. lane_check: `lane: C1, 2 files, 366 lines, EXCEEDS (lines 366 > 250)` — prose length, acceptable. code-review: P0/P1 0 on content; facts checked vs code (requires-python >=3.11, `thresholds:` + `cache_hit_low` rule id, cron fallback, retention flags, config keys/env vars). P1 process: implementer read `PROJECT_SPEC.md` and `agents/backend.md` partly (lines 56–80 of backend.md unread, tool output truncation) — recurring; conformance checked by Supervisor. P3: rollback example names `slack`; backup name differs from RUNBOOK (`store.sqlite.backup` vs `.pre-v0.1.0`, deliberate). Low risk docs → security-review N/A beyond the safety test; no DB change. |
| Full smoke suite still green (no regression) | ☑ pass | 385 passed |
| easy-verifier score (advisory) | ☑ recorded | scope=changes: overall `rating_abstention no_dimension_rated`; all dimensions abstain |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Documentation task; no UI component |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Documentation task; no UI component |
| **UI: Responsiveness at target viewports** | ☑ N/A | Documentation task; no UI component |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (verbatim prior `README.md`, 16 lines, captured 2026-10-22 08:49:35 UTC via `date -u`, before any implementation commit):

> # easy-personal-mcp
>
> The Easy Kit plug-in whose subject is **you**, not a repo. It reads your local agent activity and turns
> it into deterministic metrics, warnings and an improvement plan, shown on a local dashboard and through
> MCP tools. From v1.1 it also manages your personal tool connectors (email, messaging…) with secrets
> kept in the OS keychain and injected only at launch.
>
> Local-first · no network at runtime · metadata only, never message content · no LLM inside.
>
> Status: planning. See `PRD.md` (what and why), `PROJECT_SPEC.md` (how), `BRAINSTORMING_LOG.md`
> (how the direction was chosen) and `docs/ddr/` (design decisions).
>
> Part of the Easy Kit family: kernel [personal-agentic-claude](https://github.com/thunderkds/personal-agentic-claude),
> plug-ins [easy-ui-mcp](https://github.com/thunderkds/easy-ui-mcp),
> [easy-verifier-mcp](https://github.com/thunderkds/easy-verifier-mcp),
> [easy-context-mcp](https://github.com/thunderkds/easy-context-mcp).

**AFTER**: `README.md` at the HUN-56 head — sections: pitch · Status: v0.1.0 · What it reads · Install · Quick start (First ingest, Register the MCP server, MCP tools, Dashboard) · Schedule · Connectors · Configuration · Upgrade and rollback (→ RUNBOOK.md) · Development · Easy Kit; `tests/test_readme.py` 13 passed.

**DELTA**: A developer can install, run, register, schedule, connect, upgrade and roll back easy-personal from the README alone, where before it said "Status: planning" and had no commands.

**WITNESS**: Supervisor, 2026-10-22 08:52 UTC, isolated install following the README from the HUN-56 worktree (trace attributed to HUN-56).
