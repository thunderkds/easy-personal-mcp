# TASK_REVIEW — HUN-221: Saved investigate reports: store m0010, MCP save_report + rule snapshot, prompt step, /api/reports (FR-076..FR-078 API)

> Sibling of `tasks/TASK_GUIDE_HUN-221.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` tag,
`docs/claude-md/tracks.md` `## Auto mode`). To use it, copy the block below out of this comment, to this
place, and replace every <...>. Keep `Mode: auto` as its first line, exactly as written: T182's hook reads
it. Write it to this file before the run ends; the chat message is a copy, not the report. The same
block is the Supervisor's final chat message. It is not the `delivery-report` page and not
Evidence. A taste decision (what the user wants) also ends with the flag. Write `DEBATED` only when ≥2
Advocate `Agent` calls ran for that question, naming each Advocate's option. A decision with no Advocates is
labelled `RULE`, `FACT` or `SAFER OPTION`, never `DEBATED`. A decision flagged `⚠ decided for you` is a choice, and is never labelled `FACT`. A
safety action a rule requires (for example the backup before a delete) is `RULE` without the flag.
At Risk Medium or High, a `RULE:` counts only when it names a written rule (`path:line`,
`path § <heading>`, or `docs/claude-md/tracks.md` `### <section>`), and a `FACT:` needs a Citation
(`path:line "quoted text"`); without one it is a choice and must be `DEBATED`. Good: `2. backed up the CSV
— RULE: `docs/claude-md/tracks.md` `### Stops that stay``. Bad: `1. kept recurring.csv — RULE: Surgical Changes`.

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its option>; Advocate 2: <its option>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours)>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_reports.py` (AC 2, 3, 4, 5, 6), `tests/core/test_migrations.py` (AC 1: `test_migration_10_*`), `tests/surfaces/test_mcp_save_report.py` (AC 2, 3, 7 incl. real stdio), `tests/surfaces/test_api_reports.py` (AC 5, 6), `tests/leak/test_report_leak.py` + `tests/leak/harness.py` + `tests/leak/test_leak_suite.py::test_saved_reports_really_ran` (AC 8), `tests/core/test_store.py`, `tests/test_skeleton.py` (tool list). Verification command below: 483 passed in the filtered run, 1479 passed in the full run. |
| Verification command run | ☑ pass | `uv run pytest tests -q -k "report or rules or migration or leak or mcp or api"` -> `483 passed, 1005 deselected, 1 warning in 131.43s`; `uv run pytest -q` -> `1479 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 208.30s`. After the last edit (404 for ids over 18 digits) re-ran `tests/surfaces/test_api_reports.py tests/surfaces/test_mcp_save_report.py tests/core/test_reports.py tests/test_skeleton.py tests/test_readme.py tests/test_site_content.py` -> `147 passed`; `ruff check src tests` -> `All checks passed!` |
| Negative cases hold | ☑ pass | 24-row refusal table in `test_reports.py` (every limit, scope, `sk-`/`ghp_`/`xox`/`AKIA`/`-----BEGIN`/32-run, wrong types): error starts with the field name, says `nothing was saved`, row count stays 0, text never echoed. UUID + absolute path accepted. `test_report_leak.py`: 7 secret forms x 6 places refused in-process and over real stdio; canary absent from store dump/WAL/SHM, responses, log records, stderr. Unknown id / `abc` / `-1` / `1.5` / 30 digits -> 404; no token -> 403. |
| verify | ☐ pass / ☐ fail / ☐ N/A | Not run by the implementer (no `Skill` tool in this agent session). Real run on a copy of the owner store, MCP stdio `save_report` then `GET /api/reports/{id}`: see Demonstration AFTER — pass. Supervisor runs `verify` at Stage 5. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Self-review of the diff only (no `code-review` skill available to this agent): `rules.evaluate` (hub: warnings, plan, kit, 3 surfaces) got one opt-in keyword; default output compared equal to `checked=True` minus `checked` in `test_checked_is_opt_in_*`, and the existing scope golden (`tests/surfaces/test_scope_warnings.py::test_no_scope_equals_golden`) still passes. Other touched files: new `core/reports.py`, new `m0010`, `surfaces/mcp.py`, `surfaces/api.py`, tests, README/site tool lists. Not touched: `web/`, `rules.yaml`, rule maths, `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/`, live store. Migration-safety (read from `skills/migration-safety/SKILL.md`, all boxes): migration tool only (`core/migrations`); additive `CREATE TABLE IF NOT EXISTS`, no DROP/RENAME/NOT NULL on existing tables; no row of another table touched, no backfill; reversible: `store.migrate` writes `<store>.bak-v9` before the first pending migration (tested: `test_migration_10_adds_the_reports_table_keeps_every_row_and_leaves_a_backup`, backup opens as v9 with all events); code older than v10 refuses a v10 store (as for every earlier migration), so rollback = restore `.bak-v9`; idempotent (`test_migration_10_is_idempotent_*`); tiny new table, no lock or rewrite of `events`; dry run on a copy of the owner store (450,723 events, v9 -> v10) passed in the real run. Verdict GO. |
| Full smoke suite still green (no regression) | ☑ pass | `1479 passed, 2 skipped, 6 deselected, 1 xfailed` (full `uv run pytest -q`). |
| **Companion score (easy-verifier)** | ☑ N/A | Not run: the easy-verifier MCP is not available to this sub-agent; the Supervisor runs it at Stage 4. [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Backend only (guide: UI / Design Evidence N/A). |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Backend only. |
| **UI: Responsiveness at target viewports** | ☑ N/A | Backend only. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-05T02:13:00Z on the unchanged code (branch base `da35790`), empty temp store, script `scratchpad/demo.py` (MCP stdio client: `tools/list`, `prompts/get investigate`; Starlette TestClient: `GET /api/reports` with a valid token):

```
tools: ['ping', 'usage_summary', 'usage_breakdown', 'session_outliers', 'warnings', 'improvement_plan', 'kit_summary', 'kit_findings', 'connector_catalog', 'connector_status', 'connector_suggestions']
--- investigate prompt (last 6 lines) ---
3. Call `session_outliers` for the last 7 days.
4. For each warning that fired, explain in plain words what caused it. Read the hit's causes when it has them. Tie it to the projects, models or sessions you found in steps 2 and 3.
5. Suggest at most 5 fixes, ranked by expected saving (largest first). Cite one number from a tool result for each fix, and name the tool it came from.
6. Say plainly when a change looks intended rather than wasteful (for example, one big project that I am working on right now). Do not suggest a fix for it.

Do not guess. If a tool returns no data or abstains, say so. Do not change any file.
GET /api/reports -> 404 Not Found
```

No `save_report` tool, the prompt ends at step 6, and `/api/reports` is 404.

**AFTER**: captured 2026-11-05T02:28Z on the changed code, on a **copy** of the owner store (`sqlite3 ~/.easy-personal/store.sqlite ".backup <copy>/store.sqlite"`, 450,723 events, v9; `EASY_PERSONAL_HOME` pointed at the copy). Same script plus a `save_report` call over MCP stdio and `GET /api/reports/{id}` (the `rules` array is cut to two entries below; the store migrated v9 -> v10 on the first connect):

```
tools: ['ping', 'usage_summary', 'usage_breakdown', 'session_outliers', 'warnings', 'improvement_plan', 'kit_summary', 'kit_findings', 'connector_catalog', 'connector_status', 'connector_suggestions', 'save_report']
--- investigate prompt (last 6 lines) ---
5. Suggest at most 5 fixes, ranked by expected saving (largest first). ...
6. Say plainly when a change looks intended rather than wasteful ...

Do not guess. If a tool returns no data or abstains, say so. Do not change any file.

7. Save the report: call `save_report` once, with a summary of what you found (at most 2,000 characters), the fixes from step 5 (at most 5, each with its text and the tool and number it cites) and the same scope you used above. Do not put keys, tokens or message text in it; the tool refuses them. Then tell me the report id it returns. If `save_report` is not available, tell me to reconnect the easy-personal server in /mcp, and do not retry.
save_report -> False { "id": 1, "saved_utc": "2026-11-05T02:28:24Z" }
GET /api/reports -> 200 {"reports":[{"id":1,"saved_utc":"2026-11-05T02:28:24Z","workspace":null,"account":null,"fired_count":1,"summary_first_line":"Token use rose because of more sessions."}]}
GET /api/reports/1 -> 200 {"id":1,"saved_utc":"2026-11-05T02:28:24Z","server_version":"0.4.0","workspace":null,"account":null,"summary":"Token use rose because of more sessions.\nSecond line.","fixes":[{"number":"645301","text":"Start a new session per task","tool":"session_outliers"}],"as_of_utc":"2026-11-05T02:28:24.258967+00:00","now_as_of_utc":"2026-11-05T02:28:26.170168+00:00","rules":[ ...
  {"rule_id":"cache_hit_low","title":"Low cache reuse","before":{"state":"clear","measured":0.9743,"threshold":0.5,"fired":false},"now":{"state":"clear","measured":0.9743,"threshold":0.5,"fired":false}},
  {"rule_id":"tokens_jump","title":"Token use jumped","before":{"state":"fired","measured":2.1853,"threshold":0.5,"fired":true,"summary":"Token use rose 218.5% ...","diagnosis":"more_sessions","causes":[ ...3 projects... ]},"now":{"state":"fired","measured":2.1851,"threshold":0.5,"fired":true,"summary":"Token use rose 218.5% ..."}}, ... 7 rules in all]}
GET /api/reports/999 -> 404 {"error":"no such report"}
GET /api/reports (no token) -> 403
GET /api/reports/{id} seconds x3: [1.346, 1.552, 1.701] median 1.552
```

NFR-013 median of 3 = 1.55 s (< 2 s); the cost is the same `rules.evaluate` as `GET /api/warnings` on 450k events, so it is close to the limit.

**DELTA**: A Claude Code session can now save its `investigate` report with one MCP call, and `GET /api/reports/{id}` returns that report with the server's own before / now number per rule, which was impossible before (no tool, no table, 404).

**WITNESS**: Backend-Implementer (sub-agent) ran both captures on 2026-11-05 (BEFORE 02:13Z, AFTER 02:28Z); the independent witness line from `memory/event-trace/HUN-221.jsonl` is for the Supervisor to add.
