# TASK_REVIEW — HUN-49: Dashboard server — on demand, free port, token + Host/Origin guard, CSP, read API

> Sibling of `tasks/TASK_GUIDE_HUN-49.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☒ pass / ☐ fail | `tests/surfaces/test_api.py` (AC1 `test_bind_free_port_is_loopback`, `test_serve_prints_url_opens_browser_and_hides_token_from_logs`; AC2 `test_missing_or_wrong_token_is_403`, `test_dns_rebinding_host_is_403`, `test_other_origin_is_403`, `test_websocket_connect_is_refused`; AC3 `test_csp_on_every_response` incl. `frame-ancestors 'none'`; AC4 `test_parity_with_core_and_mcp_numbers`; AC5 logging test above); AC6 `tests/leak/test_leak_suite.py::test_every_api_route_is_registered`, `test_api_routes_were_scanned`. Supervisor re-run 2026-10-21T14:56:17Z: `uv run pytest tests/surfaces/test_api.py tests/leak -q` → `36 passed, 1 warning in 7.41s` |
| Verification command run | ☒ pass / ☐ fail | `36 passed, 1 warning in 7.41s` (warning = Starlette TestClient httpx deprecation, from the dependency) |
| Negative cases hold | ☒ pass / ☐ fail | no/wrong token 403; `Host: evil.example[:port]` 403; foreign `Origin` 403; websocket refused; `by=bad` / bad `since` 400; CSP on 403/404/400; lock busy → exit 75, no serve; no static dir → API only |
| verify | ☒ pass / ☐ fail / ☐ N/A | verify skill run 2026-10-22T03:20Z, isolated `uv tool install` (temp EASY_PERSONAL_HOME, fixtures `tests/fixtures/claude_code`, BROWSER=true): `easy-personal dashboard` printed `http://127.0.0.1:52675/#<43-char token>`; `ss` shows `127.0.0.1:52675` only; `/api/usage/summary` no token 403, wrong `X-Dashboard-Token` 403, good token + `Host: evil.example` 403, good token + foreign `Origin` 403, good token + same Origin 200 `{"sessions":4,"messages":3,"input_tokens":15,...}`; `breakdown?by=bad` 400; CSP `default-src 'self'; frame-ancestors 'none'` on `/` and on 403; token not in stderr; Ctrl-C exit 0, port closed — feature confirmed working — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☒ pass / ☐ fail | `main...HUN-49` (8 files) + `cli._ingest`. lane_check: `C2, 6 files, 356 lines, FITS` (before fix commit). code-review 0 P0/P1 code, 1 P1 process (partial read of `agents/backend.md`, closed by conformance check). security (inline): 2 findings fixed in `9e864f9` (websocket scopes skipped the guard; no `frame-ancestors`). blast-radius: low — T4 folder names, T5 counts, short-lived T3 token; no hardening required. |
| Full smoke suite still green (no regression) | ☒ pass / ☐ fail | `uv run pytest -q` → `162 passed, 1 warning in 15.88s` (2026-10-21T14:56Z) |
| easy-verifier score (advisory) | ☒ recorded / ☐ N/A | 2026-10-21, MCP `score` repo=HUN-49 worktree, scope=changes, ref=`main...HUN-49`: overall **76** (architecture 100, code-quality 87, test-strategy 75, requirement-fidelity 67, blast-radius 65, security 59); abstained: solution-fit, reliability, supply-chain, api, data, operations, infrastructure. Security = `redaction_hits_observed` 122 (known false positive). Raw JSON ~/.claude-personal/projects/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/dbd9c4da-edf5-49bc-80a5-cb2b8c04f640/tool-results/mcp-easy-verifier-score-1792594499767.txt. Advisory only. |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: Captured 2026-10-21T12:31:56Z in the HUN-49 worktree, before any implementation commit.
`uv run easy-personal dashboard` ->
```
usage: easy-personal [-h] [--version]
                     {ping,mcp,ingest,schedule,usage,outliers,warnings,plan} ...
easy-personal: error: argument command: invalid choice: 'dashboard' (choose from ping, mcp, ingest, schedule, usage, outliers, warnings, plan)
```
`uv run pytest tests/surfaces/test_api.py -q` -> `no tests ran in 0.00s` (file absent).

**AFTER**: Runtime check with fixture transcripts (BROWSER=true, 6 s timeout):
`easy-personal dashboard` printed `http://127.0.0.1:43661/#<token>`; `GET /` -> 200 with `content-security-policy: default-src 'self'`; `/api/usage/summary` without token -> 403; with `Host: evil.example` -> 403; with token -> 200 `{"sessions":4,"messages":3,...}`. Token appears only in the printed URL (stdout), not stderr. `uv run pytest tests/surfaces/test_api.py tests/leak -q` -> 35 passed.

**DELTA**: The owner can now run `easy-personal dashboard` to serve the dashboard page and a token-guarded read API on 127.0.0.1 until Ctrl-C; before, the command did not exist.

**WITNESS**: Supervisor — Stage 4 re-runs 2026-10-21T14:56Z (`memory/event-trace/HUN-49.jsonl`); Stage 5 verify recipe 2026-10-22T03:20Z in the HUN-49 worktree (Supervisor session, not attributed in the trace: no active_task state file was set).
