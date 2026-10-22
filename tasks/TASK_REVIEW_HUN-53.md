# TASK_REVIEW — HUN-53: Session timeline read: `metrics.session_timeline` + `GET /api/usage/sessions`

> Sibling of `tasks/TASK_GUIDE_HUN-53.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☒ pass / ☐ fail | `tests/core/test_session_timeline.py` (AC1 `test_sessions_points_gaps_and_order`, AC2 dup `m1` rows, AC3 `test_gap_threshold` 29.9 vs 30.0 min, AC4 `test_period_since_inclusive_until_exclusive`, AC7 `test_empty_store_and_determinism`, edges `test_tool_only_null_model_abstained_and_separate_sessions`); `tests/surfaces/test_api.py` (AC4 400 in `test_bad_input_is_400`, AC5 `test_sessions_route_is_guarded_and_returns_timeline`, parity in `test_parity_with_core_and_mcp_numbers`); AC6 `tests/leak/harness.py` `API_CALLS["/api/usage/sessions"]` (3 variants). Supervisor re-run 2026-10-22T03:45:24Z: `43 passed, 1 warning in 7.84s` |
| Verification command run | ☒ pass / ☐ fail | `uv run pytest tests/core/test_session_timeline.py tests/surfaces/test_api.py tests/leak -q` → `43 passed, 1 warning in 7.84s` (warning = Starlette TestClient httpx deprecation); `ruff check src tests` → `All checks passed!` |
| Negative cases hold | ☒ pass / ☐ fail | no token 403, `Host: evil.example` 403, foreign Origin 403, `?since=bad` 400; 29.9-min pause → no gap; abstained rows ignored; empty store → `sessions: []`; `since=2099-01-01` → `sessions: []` |
| verify | ☒ pass / ☐ fail / ☐ N/A | verify skill recipe 2026-10-22T03:44Z, isolated `uv tool install`, fixtures `tests/fixtures/claude_code`: `easy-personal dashboard` on `127.0.0.1:44347` only; `/api/usage/sessions` no token 403, evil Host 403, `since=bad` 400; with token 200, `gap_minutes 30`, 4 sessions with the 12 contract keys, token totals = sum of points (asserted), project `demo-app` / `(unknown)` only; full path or `INVENTED_` count 0; `since=2099-01-01` → empty list; Ctrl-C exit 0 — feature confirmed working — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☒ pass / ☐ fail | `main...HUN-53`: `metrics.py` (+`session_timeline`, `_parse_utc`), `api.py` (1 Route), tests; callers: `api.create_app` only. lane_check `lane: C1, 5 files, 243 lines, FITS`. code-review: 0 P0/P1/P2, 1 P3 (a message crossing `since` is placed at its earliest row time — intended, documented). Entry point `/api/usage/sessions` found at `api.py:99`. Startup reads complete. security (inline, Medium risk): route behind existing Guard (tested), SQL `where` from internal `_period` with bound params, tokens/times only, `ts_utc` format guaranteed by the sole writer `claude_code._utc` — no findings. |
| Full smoke suite still green (no regression) | ☒ pass / ☐ fail | `uv run pytest -q` → `194 passed, 2 skipped, 1 warning in 18.17s` (2026-10-22T03:45Z) |
| easy-verifier score (advisory) | ☒ recorded / ☐ N/A | 2026-10-22, MCP `score` repo=HUN-53 worktree, scope=changes, ref=`main...HUN-53`: overall **66** (architecture 100, code-quality 100, test-strategy 75, requirement-fidelity 67, blast-radius 65, reliability 36, security 18); abstained: solution-fit, supply-chain, api, data, operations, infrastructure. Security = `sink_hits_observed` 11 (f-string SQL with internal `_period` clause + bound params, same pattern as `session_outliers`) + `redaction_hits_observed` 122 (known false positive); reliability = `swallowed_errors_observed` 1 in `api.py` (pre-existing; this change adds one Route line). Raw JSON /home/hungnguyenhuu/.claude-personal/projects/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/75d1f65a-8da1-4f4c-8245-0270482d2af1/tool-results/mcp-easy-verifier-score-1792640674895.txt. Advisory only. |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: 2026-10-22T03:41:48Z, in worktree HUN-53 before any implementation commit:
`grep -n "session_timeline\|usage/sessions" -r src tests` printed nothing;
`uv run python -c "from easy_personal.core import metrics; print(hasattr(metrics,'session_timeline'))"` printed `False`.

**AFTER**: `hasattr(metrics,'session_timeline')` is True; `GET /api/usage/sessions` returns 200 with the token.
Verification command: `43 passed, 1 warning in 8.02s`. Full suite: `194 passed, 2 skipped, 1 warning in 17.51s`.

**DELTA**: The dashboard can now read each session's start/end, models, per-message token points and idle gaps from one guarded route.

**WITNESS**: Supervisor — test re-runs 2026-10-22T03:45Z and verify recipe 2026-10-22T03:44Z in the HUN-53 worktree (attributed to HUN-53 via the active_task state file; `memory/event-trace/HUN-53.jsonl`), independent of the implementing agent.
