# TASK_REVIEW — HUN-62: Agent accounts: detect, confirm, name, ignore

> Sibling of `tasks/TASK_GUIDE_HUN-62.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | pass | `tests/core/test_accounts.py` (AC 1-5, migration v2 upgrade, edge cases), `tests/surfaces/test_accounts_surfaces.py` (AC 2, 6, 7: CLI, API, 403/415), `tests/test_real_home_guard.py`, `tests/leak/` registration. 192 + 487 pass; default-account ignore refusal tested (core, CLI, API) |
| Verification command run | pass | `uv run pytest tests/core tests/surfaces tests/leak -q` -> `192 passed, 1 deselected`; `uv run pytest -q` -> `487 passed, 2 skipped, 1 deselected`; `uv run ruff check .` -> `All checks passed!` |
| Negative cases hold | pass | bad labels (empty, 61 chars), unknown account (404), non-folder / empty folder add refused without echoing the path, cross-origin / no-token POST 403, text/plain 415, `.claude.json` and empty `.claude-junk` never accounts |
| verify | ☑ pass | verify by Supervisor 2026-10-26T09:22:28Z: isolated install, temp agent root (.claude, .claude-work/projects, .codex/config.toml, .claude-junk empty, .claude.json file) → list: default confirmed (config_dir_env null), work + codex detected, no .claude-junk / .claude.json; confirm --label 'Claude work' ok; ignore default → 'the default account cannot be ignored' exit 2; --by provider groups; ingest exit 0; store user_version 3, integrity ok — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed: `core/accounts.py`, m0003, `core/config.py` agent_root, cli/api account routes, conftest isolation, leak registration. code-review: P0 0 / P1 1 FIXED (ignore on the default account would stop the fallback ingest — now refused, 3 tests) / P2 0 / P3 1 (`detect` writes on GET list — by design, insert-only). security-review (inline): routes behind token+Host/Origin+JSON guard; errors never echo input; detector only stats marker files, never reads account content; tests isolated via EASY_PERSONAL_AGENT_ROOT autouse + real-home guard test; no findings. migration-safety: GO — additive table; dry run on .backup copy of owner store v2→v3: events 575 / kit 11564 / tool_signals 17 equal, fk_check empty, integrity ok, 0.05 s, bak-v2 written. |
| Full smoke suite still green (no regression) | pass | 487 passed, 2 skipped. Two existing tests updated: `test_store.py` table list gained `agent_accounts`; README gained an Agent accounts section (`test_readme` requires each command documented) |
| easy-verifier score (advisory) | ☑ N/A | N/A — easy-verifier MCP is registered only in the `~/.claude-personal` profile; this Supervisor session runs another profile, tool not loaded |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Pure backend task; no UI file changed |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Pure backend task; no UI file changed |
| **UI: Responsiveness at target viewports** | ☑ N/A | Pure backend task; no UI file changed |

**migration-safety (applied by hand, no Skill tool)**: `m0003_agent_accounts` is additive (one `CREATE TABLE IF NOT EXISTS`, no existing table or row touched); runs in the runner's `BEGIN IMMEDIATE` with the `user_version` bump; the runner copies the store to `.bak-v2` first (restore = put the copy back); `test_v2_store_upgrades_keeping_rows` upgrades a v2 store with ingested events, asserts every event row survives and the backup exists; constraints in the table (provider/status CHECK, folder UNIQUE, label length, confirmed needs label). No secret column.

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured before any implementation commit, temp EASY_PERSONAL_HOME):

```
Mon Oct 26 09:14:59 AM UTC 2026
$ EASY_PERSONAL_HOME=$T uv run easy-personal account list
Installed 43 packages in 5ms
usage: easy-personal [-h] [--version]
                     {ping,mcp,ingest,schedule,usage,outliers,warnings,kit,plan,dashboard,exec,headers,connector,doctor} ...
easy-personal: error: argument command: invalid choice: 'account' (choose from ping, mcp, ingest, schedule, usage, outliers, warnings, kit, plan, dashboard, exec, headers, connector, doctor)
$ easy-personal ping; sqlite3 store.sqlite .tables
context_index     kit_trace_events  source_files      ui_sessions     
events            meta              tool_signals      verifier_scores 
```

**AFTER** (same temp-dir method, after the change):

```
Mon Oct 26 09:19:58 AM UTC 2026
$ account list
claude confirmed Claude default <tmp>/ar/.claude
claude detected None <tmp>/ar/.claude-work
codex detected None <tmp>/ar/.codex
$ account confirm .claude-work --label "Claude work"
  "label": "Claude work",
  "status": "confirmed"
$ .tables
agent_accounts    kit_trace_events  tool_signals    
context_index     meta              ui_sessions     
events            source_files      verifier_scores 
```

**DELTA**: The owner can list detected `~/.claude*` / `~/.codex*` agent accounts and confirm, name, ignore or add them (CLI and guarded API); before, the `account` command and `agent_accounts` table did not exist.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
