# TASK_REVIEW — HUN-47: MCP connector tools (catalog, status, suggestions)

> Sibling of `tasks/TASK_GUIDE_HUN-47.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/connectors/test_mcp_tools.py` (AC1 tool surface: no args, no write, no secret; AC2 status yes/no; AC3 counters only, canary in text never counted; claude.ai servers count as available — Supervisor confirmed this test FAILS on cea117d and PASSES on cd81616), `tests/leak/` (3 tools in `MCP_CALLS`, memory keyring with canary + fake `claude`), `tests/core/test_store.py`. `uv run pytest -q` → `314 passed, 2 skipped, 1 warning in 51.72s` |
| Verification command run | ☑ pass | `uv run pytest tests/connectors/test_mcp_tools.py tests/leak -q` → 33 passed (agent); full suite 314 passed, 2 skipped; ruff `All checks passed!` |
| Negative cases hold | ☑ pass | Fixture WebFetch `https://api.slack.com/methods?token=INVENTED_URL_CANARY` → stored `('domain','api.slack.com')` only; Bash `curl -H 'Authorization: INVENTED_BASH_CANARY' x` → `('bash','curl')` only; `grep -c INVENTED_` on MCP out/err = 0; `grep -rl INVENTED_URL_CANARY` on the store dir = none; `mcp__claude_ai_Gmail__*` → gmail not suggested |
| verify | ☑ pass | Isolated install (temp EASY_PERSONAL_HOME/UV_TOOL_*, fixtures + one extra fixture line, `PYTHON_KEYRING_BACKEND=keyring.backends.fail.Keyring`, no `claude` on PATH), `ingest`, MCP stdio: catalog 5 entries; status all `configured:false`; suggestions `discord` (mcp_calls 1 via `mcp__discord_bot__send`) and `slack` (webfetch_calls 1), gmail excluded, note present — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | main...HEAD + callers of `store.save_file`/`prune`, `parse_record`. lane_check: `lane: C2, 10 files, 346 lines, FITS` (store.py unpredicted, required). code-review: P1 1 fixed (claude.ai servers suggested as missing; cd81616), P3 1 done (`_bash_word` docstring), P2 open: `_configured` uses private `writer._keychain_item/_entry_exists`, duplicated in HUN-52's api.py — follow-up. security-review (inline, Medium): read-only tools, no args, no secret path, counters store host / first word only, regex-gated — no findings. migration-safety: GO — additive `CREATE TABLE IF NOT EXISTS tool_signals` + index via `store.connect`, no existing table altered, reversible (drop loses only derived counters), idempotent `INSERT OR IGNORE`, prune covers it; old events get counters only on re-ingest (stated in tool note). |
| Full smoke suite still green (no regression) | ☑ pass | 314 passed, 2 skipped |
| easy-verifier score (advisory) | ☑ recorded | scope=changes: overall `rating_abstention no_dimension_rated`; all 13 dimensions + change-risk abstain (below_coverage_floor / insufficient_measured_weight / no_static_rule) |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | MCP/backend-only task; no UI component |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | MCP/backend-only task; no UI component |
| **UI: Responsiveness at target viewports** | ☑ N/A | MCP/backend-only task; no UI component |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-22 (`date -u` at run time) in the HUN-47 worktree before any implementation commit. Command: MCP stdio `initialize` + `tools/list` against `python -m easy_personal.surfaces.cli mcp` (temp `EASY_PERSONAL_HOME`, PATH=/usr/bin):

```
['improvement_plan', 'kit_summary', 'ping', 'session_outliers', 'usage_breakdown', 'usage_summary', 'warnings']
{'connector_catalog': False, 'connector_status': False, 'connector_suggestions': False}
```

**AFTER**: same command after the change:

```
Processing request of type ListToolsRequest
['connector_catalog', 'connector_status', 'connector_suggestions', 'improvement_plan', 'kit_summary', 'ping', 'session_outliers', 'usage_breakdown', 'usage_summary', 'warnings']
{'connector_catalog': True, 'connector_status': True, 'connector_suggestions': True}
```

**DELTA**: An MCP client can now read the connector catalog, which connectors are configured (yes/no), and usage-based suggestions for providers not yet set up — none of which existed before.

**WITNESS**: Supervisor, 2026-10-22 08:05 UTC, isolated install + MCP stdio from the HUN-47 worktree (trace attributed to HUN-47 via the active_task state file).
