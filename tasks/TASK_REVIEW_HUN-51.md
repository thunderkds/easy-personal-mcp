# TASK_REVIEW — HUN-51: Dashboard warnings, improvement plan and kit panel

> Sibling of `tasks/TASK_GUIDE_HUN-51.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☒ pass / ☐ fail | `web/src/WarningsPlanView.test.jsx` (AC1 label, rule id, measured, threshold, fix line; unmeasured rules with reason; AC2 plan grouped by priority), `web/src/KitView.test.jsx` (AC3 "no data" never 0 incl. missing verifier key, abstained git, `stale` highlighted / `null` → unknown, missing on disk, generic extra part, sort with `aria-sort`, easy-ui overall), `web/src/App.test.jsx` (tabs), `tests/surfaces/test_api.py::test_kit_route_guarded_and_matches_core` (AC4: 403 without token, parity with `kit.kit_summary`), `tests/leak/harness.py` `API_CALLS["/api/kit"]`. Supervisor re-run 2026-10-22T04:25:11Z: web `Test Files 5 passed (5)`, `Tests 38 passed (38)`; `40 passed, 1 warning in 8.43s` |
| Verification command run | ☒ pass / ☐ fail | `npm --prefix web test && uv run pytest tests/surfaces/test_api.py tests/leak -q` → 38 passed; `40 passed, 1 warning in 8.43s`; `npm --prefix web run build` → `✓ built in 118ms`; `uv run ruff check src tests` → `All checks passed!` |
| Negative cases hold | ☒ pass / ☐ fail | `/api/kit` without token → 403 (test); missing parts/keys → "no data"; project missing on disk marked; no error alert on any of 9 screens; empty states covered by component tests |
| verify | ☒ pass / ☐ fail / ☐ N/A | 2026-10-22T04:26Z, HUN-51 worktree page built, `easy-personal dashboard` with a transcript pointed at this repo (real git + real easy-context index) and a vanished project, easy-ui folder via `config.yaml`; headless Chrome driven over CDP (Node 26 `WebSocket`, script `shoot.mjs`) clicked each tab at 375/768/1280 px: Kit shows easy-personal-mcp kernel `986 tool calls, 0 errors (0.0%); 18 kit tasks`, verifier `no data`, context `fresh · indexed …`, git `126 commits`, gone-project `(missing on disk)` all `no data`, easy-ui overall 203/110/93/0/4,740; Warnings shows `records_unreadable` (label aware, measured 0.25, threshold 0.02, fix line) + 4 unmeasured rules with reasons; Plan grouped under Priority 2. Page requests (CDP Network): only `127.0.0.1:44305` (`/`, assets, 7 `/api/*` incl. `/api/kit`) — feature confirmed working — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☒ pass / ☐ fail | `main...HUN-51`: `surfaces/api.py` (+1 route, `kit` import), `web/src/*` (App tabs, WarningsPlanView, KitView, useLoad, css), tests + leak registration. lane_check `lane: C2, 13 files, 574 lines, EXCEEDS (lines 574 > 500)` — tests/fixtures ~230 lines. code-review: 0 P0, 0 code P1; 1 P1 process (startup reads: `agents/frontend.md` read 5 lines only, despite the prompt) closed by conformance check (framework from PROJECT_SPEC, accessible tabs `role=tab`/`aria-selected`/`aria-sort`, no memory writes, no new deps, scope = guide files); process note: components and tests written in one pass, not red-first (only the route test seen failing first). P3: measured/threshold shown without unit (payload has none); `useLoad` effect deps empty by design (static loader). security (inline, Low): no `innerHTML`/inline style/remote URL; route behind the existing Guard (tested). |
| Full smoke suite still green (no regression) | ☒ pass / ☐ fail | `uv run pytest -q` → `245 passed, 1 warning in 22.58s` (page built: web tests run, none skipped) |
| easy-verifier score (advisory) | ☒ recorded / ☐ N/A | 2026-10-22, MCP `score` repo=HUN-51 worktree, scope=changes, ref=`main...HUN-51`: overall **81** (architecture 100, blast-radius 100, code-quality 87, test-strategy 75, requirement-fidelity 67, security 59); security = `redaction_hits_observed` 124 (known false positive); requirement-fidelity = `requirement_items_traced_share` 0. Raw JSON /home/hungnguyenhuu/.claude-personal/projects/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/75d1f65a-8da1-4f4c-8245-0270482d2af1/tool-results/mcp-easy-verifier-score-1792643221328.txt. Advisory only. |
| **UI: Visual regression (diff or verdict pasted)** | ☒ pass / ☐ fail / ☐ N/A | CDP-driven headless Chrome screenshots of all 3 tabs at 3 widths + LLM vision review against AC1–AC3: tabs, warnings list, plan by priority, kit table (no data / fresh / missing on disk), easy-ui overall render; no broken layout; no error alert. Files: `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/reports/HUN-51/{usage,warnings,kit}-{375,768,1280}.png`, `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/reports/HUN-51/report.json`. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☒ pass / ☐ fail / ☐ N/A | CSS audit: 0 hex colors in `app.css` (all in `tokens.css`); one `font-family: var(--font)`; new rules use `--space-*`/`--text-*`; raw values only 1–2px borders and the existing 72rem page width; no inline style in any `.jsx`. |
| **UI: Responsiveness at target viewports** | ☒ pass / ☐ fail / ☐ N/A | CDP measured `scrollWidth == clientWidth` (no horizontal page scroll) on all 9 screens (3 tabs × 375/768/1280); kit table scrolls inside `.table-wrap` at 375 px (same accepted pattern as HUN-50); tabs wrap. `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/reports/HUN-51/report.json`. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: Captured 2026-10-22T04:22:19Z (TestClient on create_app, empty store, before any HUN-51 change; web has no Warnings or Kit view either):

```
Using CPython 3.13.2
Creating virtual environment at: .venv
   Building easy-personal-mcp @ file:///home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-51
      Built easy-personal-mcp @ file:///home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-51
Installed 43 packages in 5ms
<stdin>:3: StarletteDeprecationWarning: Using `httpx` with `starlette.testclient` is deprecated; install `httpx2` instead.
/api/warnings 200
/api/plan 200
/api/kit 404
```

**AFTER**: 2026-10-22T04:25Z, HUN-51 worktree: `npm --prefix web test` → `Test Files 5 passed (5)`, `Tests 38 passed (38)`; `uv run pytest tests/surfaces/test_api.py tests/leak -q` → `40 passed`; `GET /api/kit` with token → 200 = `kit.kit_summary`; the page shows tabs Usage / Warnings & plan / Kit (screenshots in `reports/HUN-51/`).

**DELTA**: The owner can now open the Warnings & plan tab (each warning with label, rule, measured value, threshold and fix; the plan by priority) and the Kit tab (per-project kernel, context freshness and git, "no data" never shown as zero, plus easy-ui checks overall) in the dashboard; before, only the Usage view existed.

**WITNESS**: Supervisor — test re-runs 2026-10-22T04:25Z and CDP-driven browser verify 04:26Z in the HUN-51 worktree (attributed to HUN-51 via the active_task state file; `memory/event-trace/HUN-51.jsonl`), independent of the implementing agent.
