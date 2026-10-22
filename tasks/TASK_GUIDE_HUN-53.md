**Date**: 2026-10-22
**Complexity Level**: C1
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read `agents/backend.md`
5. Note the **Complexity Level** above and apply the matching process (brainstorm / decompose / verify depth) from the Complexity matrix in your role guide (`agents/backend.md`)
6. **C2/C3 or multi-file tasks only**: read `memory/codebase-map.md` for directory layout, entry points, and blast-radius hotspots — skip if the task is C0/C1 and touches a single known file

---

## Requirement (Pillar 1 — Adapt the requirement)

FR-023: the dashboard shows the session timeline — per session: start→end, model, token use over time, gaps. No core function or API route returns this data today (`/api/usage/outliers` gives only session id, project, tool calls, peak context). HUN-50 (Usage view) renders the timeline and depends on this task. Owner chose a separate backend task on 2026-10-22.

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> A read-only core function and a guarded dashboard API route return, for a period, each session with its project folder name, start/end UTC, models, token totals, per-message token points and idle gaps — tokens only, from the store, deterministic.

**Out of scope** (what this task explicitly does NOT do):

* Any UI (HUN-50)
* MCP tool or CLI command for the timeline (cut list; FR-023 is a dashboard requirement)
* Cost per session (no cost in this response; cost stays in summary/breakdown)
* Schema change or migration (all needed columns exist in `events`)

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-023: session timeline data (start→end, model, token use over time, gaps)
* FR-004: tokens primary, project folder name only, times stored in UTC
* NFR-001: route served only behind the existing loopback Guard (token + Host/Origin)
* NFR-002: no text or secret content in the response (metadata only)
* NFR-004: deterministic output for the same store

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor; owner chose option 1 "new backend task first", 2026-10-22)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-49 — dashboard server and `/api/*` Guard (merged to main 2026-10-22)

**Entry point**: `/api/usage/sessions`

---

## Response contract (fixed — HUN-50 builds against it in parallel)

`GET /api/usage/sessions?since=<ISO>&until=<ISO>` (same `since`/`until` parsing and 400 errors as `/api/usage/summary`) returns `metrics.session_timeline(conn, since, until)`:

```json
{
  "period": {"since_utc": "...|null", "until_utc": "...|null"},
  "gap_minutes": 30,
  "sessions": [
    {
      "session_id": "abc",
      "project": "demo-app",
      "start_utc": "2026-10-20T08:00:00.000000Z",
      "end_utc": "2026-10-20T09:10:00.000000Z",
      "models": ["claude-opus-5-5"],
      "messages": 12,
      "input_tokens": 0, "output_tokens": 0,
      "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0,
      "points": [
        {"ts_utc": "...", "model": "claude-opus-5-5",
         "input_tokens": 0, "output_tokens": 0,
         "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0}
      ],
      "gaps": [{"start_utc": "...", "end_utc": "...", "minutes": 42.5}]
    }
  ]
}
```

* `start_utc`/`end_utc`: min/max `ts_utc` of the session's non-abstained events in the period (all record types).
* `points`: one per message (deduplicated by `message_id`, same rule as `metrics._messages`), sorted by `ts_utc`, then `message_id`. Token totals = sums over `points`.
* `models`: distinct non-null models of the points, sorted.
* `gaps`: each stretch between two consecutive event timestamps of the session that is `>= gap_minutes`; `minutes` rounded to 1 decimal.
* `project`: `metrics._folder(project_path)` — folder name only.
* `sessions` sorted by `start_utc` descending, then `session_id`.
* Same format for all UTC strings as the existing metrics functions.

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Seeded store → each session has the start/end, models, message count, token totals and points of the contract; totals equal the sum of points | FR-023, FR-004 |
| 2 | A message stored as several rows (same `message_id`) counts once in points and totals | FR-004 |
| 3 | A pause of ≥ `gap_minutes` between consecutive events gives one gap with its start, end and minutes; a pause just below it gives none | FR-023 |
| 4 | `since`/`until` limit events by `ts_utc` (since inclusive, until exclusive); a bad value → 400, same as `/api/usage/summary` | FR-023 |
| 5 | Route refuses without token / wrong Host / foreign Origin (403) and carries the CSP header — via the existing Guard, no new auth code | NFR-001 |
| 6 | `project` is the folder name only; no full path, message text or planted canary in the response (route registered in `tests/leak/harness.py` `API_CALLS`, leak suite passes) | FR-004, NFR-002 |
| 7 | Empty store → `{"sessions": []}` with period and `gap_minutes`; abstained rows are ignored; same store → identical JSON twice | NFR-004 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_HUN-53.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Store with 2 sessions: A has 3 messages (one duplicated across 2 rows) at 08:00, 08:10, 09:00 on one model; B has 1 message at 07:00 | A: `messages` 3, start 08:00, end 09:00, one gap 08:10→09:00 of 50.0 min; order A then B (start descending); totals = sum of points | `tests/core/test_session_timeline.py` |
| 2 | Pause of 29.9 min with `gap_minutes` 30 | no gap | automated test |
| 3 | `GET /api/usage/sessions` with token | 200, JSON equal to `metrics.session_timeline` on the same store | `tests/surfaces/test_api.py` |
| 4 | No token / `Host: evil.example` / `?since=bad` | 403 / 403 / 400 | `tests/surfaces/test_api.py` |
| 5 | Leak fixtures ingested | canary count 0 in the route response | `tests/leak` |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core/test_session_timeline.py tests/surfaces/test_api.py tests/leak -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-53.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-53.md` (review sections are never taken from the board).

## Approach

**Pattern reference**: `src/easy_personal/core/metrics.py` — `session_outliers` (period filter via `_period`/`_utc_bound`, `_folder`) and `_messages` (dedup by `message_id`); `src/easy_personal/surfaces/api.py` — the `Route("/api/usage/outliers", _query(...))` line.

**Vital slice**: `metrics.session_timeline` + one `Route` line + leak registration.
**Cut list**:

* MCP tool / CLI command (dashboard-only requirement; add later if wanted)
* Point downsampling for very long sessions (local JSON; revisit if a real week is slow)
* Configurable `gap_minutes` (module constant `TIMELINE_GAP_MINUTES = 30`; owner to confirm the number)

Add `TIMELINE_GAP_MINUTES = 30` next to the `OUTLIER_*` constants. Read with plain SQL like the other metrics; no new dependency. The route reuses `_query` and `_period`, so the Guard and error handling come for free.

---

## Edge Case Checklist

- [ ] Session with events but no message rows (tool-only) → listed with `messages` 0, empty points, start/end from events
- [ ] Event rows with null `ts_utc` or `abstained = 1` ignored
- [ ] Null `model` on a point → `"model": null`, not in `models`
- [ ] Session crossing `since`/`until` → only in-period events count
- [ ] Two projects with the same folder name keep separate sessions (keyed by `session_id`, not `project`)
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/metrics.py` | `TIMELINE_GAP_MINUTES`, `session_timeline()` |
| `src/easy_personal/surfaces/api.py` | one `Route("/api/usage/sessions", ...)` |
| `tests/core/test_session_timeline.py` | new: AC1–4, AC7 |
| `tests/surfaces/test_api.py` | route parity, guard, 400 |
| `tests/leak/harness.py` | `API_CALLS["/api/usage/sessions"]` variants |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `web/**` | HUN-50 owns the UI, in parallel |
| `src/easy_personal/core/store.py` (schema) | No schema change in this task |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

pytest, test-first: unit tests on a seeded in-memory store for the core function (AC1–4, AC7), TestClient tests for the route (AC4–5), leak suite registration (AC6).

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Security review: `Skill({ skill: "security-review" })` run (if Medium/High risk)
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_HUN-53.md`'s Evidence table (Hard-Stop Gate 5)
- [ ] `Skill({ skill: "verify" })` run — feature confirmed working in running app
- [ ] `memory/MEMORY.md` updated (if new patterns or feedback learned)
- [ ] Supervisor notified: task ready for Stage 4 review