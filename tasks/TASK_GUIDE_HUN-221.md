**Date**: 2026-11-05
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Rule, Abstained, Saved report, Rule snapshot, Before / now; Critical Constraints: metadata only + its named exceptions, deterministic, tool surface)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` US-032, FR-073, FR-075, FR-076, FR-077, FR-078, NFR-002, NFR-004; `docs/ddr/0003-store-owner-saved-investigate-reports.md`; `BRAINSTORMING_LOG_REPORTS.md` (Option A, Edge Case Checklist); then `src/easy_personal/core/rules.py` (`_measure`, `evaluate`), `src/easy_personal/core/store.py`, `src/easy_personal/core/migrations/` (m0008, m0009 as models), `src/easy_personal/surfaces/mcp.py`, `src/easy_personal/surfaces/api.py` (`_query`, `_who`, routes), `tests/leak/` (README, `harness.py`, `canaries.py`)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-05, after running `/mcp__easy-personal__investigate`: "the result appear, but we should log this report to the dashboard also, for easy refer, or trace the improvement or enhance if any". Owner choices 2026-11-05: report text + the server's own number snapshot; improvement = before / now per rule; narrow NFR-002 exception with limits and a secret check (DDR-0003); section on the Warnings & plan page; brainstorm path Option A ("as you recommend").

**Restated intent**:

> Add store migration `m0010_investigate_reports` with one table (id, saved_utc, server_version, workspace, account, summary, fixes_json, snapshot_json). Add an opt-in `checked` list to `rules.evaluate` (every measured rule: `rule_id`, `title`, `measured`, `threshold`, `fired`); the default output stays byte-identical. New `core/reports.py`: `validate` (FR-076 limits + secret check), `save` (takes the rule snapshot itself: `checked` + fired hits' `causes`/`diagnosis`/`summary` + abstained reasons, for the given scope), `list_reports` (newest 50, summary first line, fired count), `get_report` (full report + before / now joined by `rule_id` against `evaluate(checked=True)` for the saved scope, computed on each call). MCP tool `save_report(summary, fixes, workspace=None, account=None)` returns `{id, saved_utc}`; refusals raise a clear error naming the field and store nothing. The `investigate` prompt gains a final step (FR-077). `GET /api/reports` and `GET /api/reports/{id}` behind the dashboard token. The server writes no text of its own (NFR-004).

**Out of scope**:

* The page section (separate task, FR-078 UI)
* Delete / edit report, "tried" ticks, CLI command, charts across reports (PRD Out of Scope)
* Any change to rule maths, thresholds, which rules fire, or the default `warnings` / `improvement_plan` output

**Requirement Refs**: FR-076, FR-077, FR-078 (API part), NFR-002 (exception), NFR-004

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner "as you recommend", 2026-11-05)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None

**Entry point**: MCP `save_report` and prompt `investigate` (`surfaces/mcp.py`); `GET /api/reports`, `GET /api/reports/{id}` (`surfaces/api.py`); store migration runs on any `store.connect`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Migration m0010 creates the table; v9 → v10 on a fixture store keeps every existing row; `.bak-v9` is written; migration tests use `max(MIGRATIONS)` | FR-076 |
| 2 | `save_report` with a valid summary, 2 fixes and no scope stores one row and returns `{id, saved_utc}`; the stored snapshot holds every declared rule (fired, not fired, abstained) with measured / threshold / fired, and fired rules' `causes` + `diagnosis` | FR-076 |
| 3 | Refusals store nothing and name the field: summary > 2,000 chars; > 5 fixes; a fix text > 300 chars; unknown `workspace` / `account`; text with `sk-…`, `ghp_…`, `xox…`, `AKIA…`, a `-----BEGIN` header, or a 32+ char letter-and-digit run. A UUID session id and an absolute file path in the text are **accepted** | FR-076, NFR-002 |
| 4 | `rules.evaluate` default output is identical to before on the existing rules fixtures (existing goldens unchanged); `checked=True` adds the `checked` list only | FR-076, FR-073 |
| 5 | `GET /api/reports/{id}`: before / now per rule; a rule that fires then and is clear now shows both measured values; a rule abstaining now shows its reason; a rule missing from the current config shows `not measured now`; a changed threshold shows each side's own threshold. Unknown id → 404 | FR-078 |
| 6 | `GET /api/reports` lists newest first, at most 50, with save time, scope, fired count, summary first line; both routes refuse without the dashboard token | FR-078 |
| 7 | The `investigate` prompt text ends with the FR-077 step (call `save_report` once, same scope, tell the owner the id; if the tool is missing, tell the owner to reconnect the server in `/mcp`) | FR-077 |
| 8 | Leak suite: `save_report` registered in `MCP_CALLS`, both routes in `API_CALLS`; a planted secret sent to `save_report` is refused and appears in no store row, response or log; fixture message canaries never reach `/api/reports`; README / site / skeleton tool lists updated so their tests pass | NFR-002 |
| 9 | Real run on a **copy** of the owner store (`EASY_PERSONAL_HOME` pointed at the copy): call `save_report` over MCP stdio, then `GET /api/reports/{id}`; paste both outputs. `GET /api/reports/{id}` < 2 s median of 3 (NFR-013) | FR-076, FR-078, NFR-013 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests -q -k "report or rules or migration or leak or mcp or api" && uv run pytest -q
```

## Approach

**Vital slice**: m0010 + `evaluate(checked=True)` + `core/reports.py` (validate / save / list / get) + one MCP tool + prompt step + two read routes; tests per AC.
**Cut list**: no delete / edit route, no CLI, no pagination past 50, no trend query, no dedupe of repeated saves.

Reuse `rules.evaluate` for both snapshot and "now"; do not copy rule code. Store JSON with `json.dumps(..., sort_keys=True)`. The write goes through `store.opened()` (WAL + busy timeout) like other store writes.

---

## Edge Case Checklist

- [ ] Unknown scope → `ValueError` from `evaluate` → refusal, no row
- [ ] Secret regex does not match a UUID (hyphen-split) or a `/home/...` path
- [ ] Rule removed from config since save → `not measured now`; threshold changed → both thresholds shown
- [ ] Two saves in a row → two rows (accepted)
- [ ] Store older than v10 opened by a running server → migrates on next connect (no crash)
- [ ] Planted secret/text fixtures never appear in any store row, MCP/CLI/API response, page or log (leak suite green)

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/migrations/m0010_investigate_reports.py` | new table |
| `src/easy_personal/core/rules.py` | opt-in `checked` in `evaluate` |
| `src/easy_personal/core/reports.py` | new: validate / save / list / get |
| `src/easy_personal/surfaces/mcp.py` | `save_report` tool; prompt step |
| `src/easy_personal/surfaces/api.py` | two GET routes |
| `tests/core/test_reports.py`, `tests/core/test_migrations.py`, `tests/leak/harness.py`, surface tests, README / site tool lists | AC 1–8 |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `web/` | page is another task |
| rule maths / `src/easy_personal/data/rules.yaml` | FR-073 values must not move |
| `~/.easy-personal/store.sqlite` (live) | real run on a copy only (`sqlite3 ~/.easy-personal/store.sqlite ".backup <copy>/store.sqlite"`) |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Fixture stores (two 7-day windows) for snapshot + before / now; refusal table test; migration test; default-output identity test for `evaluate`; MCP stdio test for `save_report` and the prompt text; API tests with / without token; leak suite; timing on a store copy.

---

## UI / Design Evidence

☐ N/A — backend only.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review