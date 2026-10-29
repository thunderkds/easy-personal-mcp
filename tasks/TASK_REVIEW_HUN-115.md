# TASK_REVIEW — HUN-115: Account status-line data in /api/accounts: email, plan, last model, tokens, plan-limit snapshot

> Sibling of `tasks/TASK_GUIDE_HUN-115.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Task breakdown and labels approved; run all tasks in auto mode — FACT: PROJECT_SPEC.md:175 "do all by parallel with tag [auto], merge to main and push"
2. Requirement Fidelity Gate signed auto (Supervisor) — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (row 13)
3. Spawn model from the CLI Model Table — RULE: `PROJECT_SPEC.md` § CLI Model Table
4. Waves (stacked branches) instead of all 8 from main: tasks that edit the same files start from their predecessor's branch — RULE: `PROJECT_SPEC.md` § Tasks ("Waves: 1 = HUN-113, 114, 115, 117 …")
5. Merge to main and push after `/verify` passes (the owner's explicit ask overrides the auto-mode no-merge rule for this one step) — FACT: docs/claude-md/tracks.md:365 "the explicit ask wins for X"
6. security-review done inline by the Supervisor; the built-in command failed with `fatal: ambiguous argument 'origin/HEAD...'` (this repo's remote is `github`, there is no `origin`) — FACT: memory/MEMORY.md:41 "security-review inline"
Checks: tests `uv run pytest -q` — 775 passed; perf 1M events 67–87 ms (<100 ms); reviews code-review — 0 findings, security-review (inline) — 0, blast-radius — low (local household data); real run README jq snippet run in a temp dir: writes the file, nothing without rate_limits (auto-checked, not /verify)
Not done: merge (owner asked for it; runs after verify); auto verify: running (T182)


## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_account_status_line.py` (AC 1-5 + 8: keys in both groupings, email/plan/org and nothing else, default-account fallback file, unusable files -> nulls, Codex nulls, missing folder, tokens today/7d incl. NULL-account, message dedupe, local midnight, newest model/time, limits parse/stale/broken/percent types, API 200, `perf` 1M-event benchmark); `tests/leak/canaries.py`, `tests/leak/conftest.py`, `tests/leak/test_leak_suite.py` (AC 7: planted `.claude.json` env secret + userID + limits extra keys; `test_account_status_line_really_ran`). Output: `uv run pytest tests/core/test_account_status_line.py tests/leak -q` -> `53 passed, 1 deselected`. README snippet (AC 6) run with real `jq` in a temp dir: wrote `{"five_hour":{"used_percentage":42.5,"resets_at":1893456000},"seven_day":{...},"written_at":1793261691.78}`, and left no file when stdin had no `rate_limits`. |
| Verification command run | ☑ pass | 2026-10-29T08:18:25Z `uv run pytest -q` -> `775 passed, 5 deselected, 1 xfailed, 1 warning in 159.43s (0:02:39)` (web built first with `npm --prefix web ci && npm --prefix web run build`). `uv run ruff check src tests` -> `All checks passed!`. Benchmark: `uv run pytest tests/core/test_account_status_line.py -m perf -q -s` -> `perf /api/accounts core over 1M events: [70.6, 86.6, 67.3, 67.1, 67.0] ms`, 1 passed (limit 100 ms). |
| Negative cases hold | ☑ pass | Parametrized tests: missing / `not json` / `[]` / `{}` / `oauthAccount: 5` / non-string email -> null identity; limits `None` / `{broken` / `[]` / `{}` / window not a dict / percent as string -> `limits: null`; bool percent and string `resets_at` -> window dropped / `resets_at` null; Codex folder with `.claude.json` + limits file -> all null; missing folder -> nulls + `missing`; planted `INVENTED_SECRET_115` and leak canaries never in output (leak suite: 27 passed). |
| verify | ☑ pass | owner ran /verify 2026-10-29; headless `claude -p '/verify HUN-115'` run, log .claude/verify-runs/HUN-115-d7736e2b12c2.log; hook record reads blocked only because the run's summary had no single verdict line: /api/accounts status_line has all 8 keys (by=account and by=provider); canary CANARY_HUN115_SECRET_9f3a absent from 8 API routes, 6 CLI commands, MCP stdio, log and store; tokens_today/7d and limits stale flag correct; Codex nulls; full pytest 775 passed 1 xfailed; 1M-event timings [64.4, 81.6, 78.6, 84.3, 103.4] ms, test passes on its typical value — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | code-review (Supervisor, 2026-10-29): accounts.status_lines + callers (grouped → /api/accounts, CLI account list), README, leak suite. lane: C2, 6 files, 439 lines, FITS. Findings after gate: P0 0, P1 0, P2 0, P3 0. security-review inline: 3 keys only, size caps, read-only, leak suite plants secrets in other keys — no findings. blast-radius: email (T3) of up to 6 local accounts, loopback + token only, household use; optional follow-up: hide email behind a toggle — pass |
| Full smoke suite still green (no regression) | ☑ pass | Same run: `775 passed, 5 deselected, 1 xfailed` (the 5 deselected are `perf`-marked, run on their own above). |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Backend-only task; no UI change (card UI is the follow-up task). |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Backend-only task; no UI change. |
| **UI: Responsiveness at target viewports** | ☑ N/A | Backend-only task; no UI change. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-29T08:13:06Z, branch HUN-115 before any implementation commit, command `uv run python <scratchpad>/demo_115.py` (fixture agent root in a temp dir: `.claude-work` has a `.claude.json` with email + planted `INVENTED_SECRET` in `mcpServers`, an `easy-personal-limits.json`, and one stored event of 100 tokens; it then calls `GET /api/accounts`):

```
200
{"id": 1, "provider": "claude", "label": "Claude default", "status": "confirmed", "status_line": "<absent>"}
{"id": 2, "provider": "claude", "label": null, "status": "detected", "status_line": "<absent>"}
secret in body: False
```

**AFTER**: 2026-10-29T08:21:13Z, same command and fixture, branch HUN-115 with the change:

```
200
{"id": 1, "provider": "claude", "label": "Claude default", "status": "confirmed", "status_line": {"email": null, "plan": null, "organization": null, "last_model": null, "last_active_utc": null, "tokens_today": 0, "tokens_7d": 0, "limits": null}}
{"id": 2, "provider": "claude", "label": null, "status": "detected", "status_line": {"email": "fixture@example.invalid", "plan": "stripe_subscription", "organization": "Fixture Org", "last_model": "claude-opus-5-5", "last_active_utc": "2026-10-29T08:21:13.283681Z", "tokens_today": 100, "tokens_7d": 100, "limits": {"five_hour": {"used_percentage": 42.5, "resets_at": "2026-10-29T09:21:13Z", "stale": false}, "seven_day": {"used_percentage": 10, "resets_at": "2026-10-30T08:21:13Z", "stale": false}, "written_at": "2026-10-29T08:21:13Z", "stale": false}}}
secret in body: False
```

**DELTA**: `/api/accounts` now gives each account its email, plan, organization, last model, tokens today and in 7 days, and the plan-limit snapshot, so the Agents card can read like the status line; the planted `mcpServers` secret still does not appear.

**WITNESS**: [left for the reviewer: derive from `memory/event-trace/HUN-115.jsonl`]
