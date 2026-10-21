# TASK_REVIEW — HUN-35: usage_breakdown + session_outliers

> Sibling of `tasks/TASK_GUIDE_HUN-35.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☒ pass / ☐ fail | New: `tests/core/test_metrics.py` (AC1 totals = usage_summary per dimension, AC2 Berlin DST across 2026-03-29, AC3 labelled cost + per-model abstain, AC4 folder name, AC5 outliers ≥90 calls / ≥200k context with measured values), `tests/core/test_prices.py` (AC3 dated table, abstain), `tests/surfaces/test_usage_breakdown.py` (MCP/CLI parity, AC4 at surface); leak-suite registration in `tests/leak/harness.py`. Supervisor 2026-10-21T10:58Z: `29 passed` |
| Verification command run | ☒ pass / ☐ fail | 2026-10-21T10:58Z `uv run pytest tests/core/test_metrics.py tests/core/test_prices.py tests/surfaces -q` → `29 passed`; `uv run ruff check` → `All checks passed!` |
| Negative cases hold | ☒ pass / ☐ fail | Unknown `by` → ValueError (core), MCP `isError: true` "by must be one of day, model, project, tool", CLI `--by bogus` exit 2; empty period → empty groups; unpriced model → `usd: null`, listed in `abstained_models`; leak suite with the new surfaces registered → `18 passed` (no canary in any output) |
| verify | ☒ pass / ☐ fail / ☐ N/A | 2026-10-21T10:57Z Supervisor, verify recipe (temp `EASY_PERSONAL_HOME`, `uv tool install .`, fixtures in temp `EASY_PERSONAL_CLAUDE_PROJECTS`): `ingest` → files 5, new_events 31; `usage --by day|model|project|tool` totals in/out 15/8 = `usage` summary 15/8 for every dimension; project key `demo-app` (folder only); tool groups `Read` 10/5 + `(none)` 5/3; cost `usd: null` (fixture model unpriced); `outliers` → sessions [], thresholds 90 / 200000; MCP stdio `tools/list` → `['ping', 'usage_summary', 'usage_breakdown', 'session_outliers']`, `usage_breakdown(by=model)` isError False; 0 `INVENTED_` hits — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☒ pass / ☐ fail | 10 files in `main...HUN-35` + callers (`surfaces/cli.py`, `surfaces/mcp.py`). lane_check: `C2, 9 files, 571 lines, EXCEEDS (lines 571 > 500)` — 208 lines are tests, accepted at C2. code-review 0 P0/P1, 3 P2 (placeholder prices; 200k context threshold is implementer's choice; same folder name → duplicate group keys, carry to HUN-50), 2 P3/process — `reports/code-review_HUN-35_20261021T175649.html`; security (inline) pass — `reports/security-review_HUN-35_20261021T175649.html` (main checkout) |
| Full smoke suite still green (no regression) | ☒ pass / ☐ fail | 2026-10-21T10:58Z Supervisor, after merging main (HUN-36): `uv run pytest -q` → `97 passed, 2 skipped` |
| easy-verifier score (advisory) | ☐ recorded / ☒ N/A | N/A — easy-verifier MCP is not registered in this session's Claude profile (`CLAUDE_CONFIG_DIR=~/.claude-wife`; registered only in `~/.claude-personal`), so the `score` tool is not loaded. Server itself is up on 127.0.0.1:8766. |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task (no UI component) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task (no UI component) |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task (no UI component) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-21T10:52:01Z in the HUN-35 worktree, before any implementation commit:

```
$ uv run pytest tests/core/test_metrics.py tests/core/test_prices.py tests/surfaces -q
ERROR: file or directory not found: tests/core/test_metrics.py
no tests ran in 0.00s
$ uv run easy-personal usage --by day
easy-personal: error: unrecognized arguments: --by day
$ MCP tool list (create_app().list_tools())
['ping', 'usage_summary']
```

**AFTER**: captured 2026-10-21T10:53:34Z, same worktree, fixtures ingested into a temp store:

```
$ uv run pytest tests/core/test_metrics.py tests/core/test_prices.py tests/surfaces -q
29 passed in 0.59s
$ uv run easy-personal usage --by model   # (abridged)
groups[0]: key claude-fixture-model-1, input_tokens 15, output_tokens 8, messages 3,
  api_equivalent_cost {usd: null, price_table_date: "2026-10-21", abstained_models: ["claude-fixture-model-1"]}
total: same tokens as usage_summary
$ uv run easy-personal outliers   -> {"sessions": [], "thresholds": {"context_tokens": 200000, "tool_calls": 90}}
MCP tools: ping, usage_summary, usage_breakdown, session_outliers
```

**DELTA**: the owner can now see tokens (and a dated, labelled API-equivalent cost) per local day, model, project folder or tool, and list tool-heavy or long-context sessions, from the CLI (`usage --by`, `outliers`) or MCP (`usage_breakdown`, `session_outliers`).

**WITNESS**: Supervisor, 2026-10-21T10:57–10:58Z — verify recipe and test runs in the HUN-35 worktree (attributed to HUN-35 via the active_task state file; `memory/event-trace/HUN-35.jsonl`).
