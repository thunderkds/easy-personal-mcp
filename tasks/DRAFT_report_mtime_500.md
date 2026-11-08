# TASK_GUIDE — DRAFT: a file with an out-of-range mtime makes the report routes answer 500 (FR-089, FR-090)

> **Draft.** The board is unreachable and `LINEAR_API_KEY` is not set, so no task ID exists yet.
> A new task's ID comes from the board and a create cannot be queued
> (`scripts/board_sync.py:374-376`). When the board is reachable, run
> `python3 scripts/board_sync.py create --guide tasks/DRAFT_report_mtime_500.md`.
> Found by `/verify HUN-242` on 2026-11-07. Both Stage 4 reviews had flagged it below their bar;
> the real run confirmed it.

**Date**: 2026-11-08
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Read `PRD.md` FR-089, FR-090, NFR-002; `src/easy_personal/core/kit.py` (`_report_stats`, `report_files`, and every other `datetime.fromtimestamp` call in the module); `src/easy_personal/surfaces/api.py` (`_query`, `_report_call`)

---

## Mental Model (confirmed by a real run)

`kit._report_stats` (FR-089, merged in HUN-240) and `kit.report_files` (FR-090, HUN-242) both turn a
file's `st_mtime` into text with `datetime.fromtimestamp(mtime, UTC)`. That call raises `ValueError`
or `OverflowError` for a timestamp outside the year range `datetime` can represent. Neither `_query`
nor `_report_call` catches those, so the exception escapes as a 500 with a traceback on the
dashboard's stderr.

One file with such an `mtime` therefore takes out **the whole workspace's** report-folder listing,
not just that file's row — and `/api/kit/reports` feeds the "Kit by workspace" block (HUN-241), so
the Kit view loses its report folders entirely.

**It is not a regression from HUN-242.** Measured 2026-11-07:

| Checkout | Fixture | `/api/kit/reports` | `/api/kit/report-files` |
| -- | -- | -- | -- |
| `main` @ 75a2d08 | normal mtimes | 200 | 404 (route absent) |
| `main` @ 75a2d08 | one file at `st_mtime` 1e17 | **500** | 404 (route absent) |
| `HUN-242` @ 78fe629 | normal mtimes | 200 | 200 |
| `HUN-242` @ 78fe629 | one file at `st_mtime` 1e17 | **500** | **500** |

So the flaw is in the merged FR-089 code, and FR-090 repeats it. Fixing it is out of HUN-242's scope
(Surgical Changes: it would change already-merged code), which is why it is its own task.

**Reproduce**:

```bash
# in a temp EASY_PERSONAL_HOME with a workspace whose repo has reports/sub/b.md
python3 -c "import os,sys; os.utime(sys.argv[1], (10**17, 10**17))" <repo>/reports/sub/b.md
curl -s -o /dev/null -w '%{http_code}\n' -H "X-Dashboard-Token: $TOK" \
  "$BASE/api/kit/reports"                                             # 500
curl -s -o /dev/null -w '%{http_code}\n' -H "X-Dashboard-Token: $TOK" \
  "$BASE/api/kit/report-files?workspace=acme&repo_no=0&folder=reports" # 500
```

How a real file gets such an `mtime`: an archive extracted with absurd timestamps (tar and zip both
carry them), a build or sync tool writing a far-future time, a corrupt inode, or a plain `touch -d`.
Report folders are written by other tools, so the dashboard does not control them.

---

## Requirement

> A file whose `mtime` cannot be represented must not break the answer. One unreadable time degrades
> that one file's field; the listing still returns 200 with every other file.
>
> Fix it in **one** place: a small helper in `kit.py` that turns an `st_mtime` into the
> `%Y-%m-%dT%H:%M:%SZ` string and returns `None` on `ValueError` or `OverflowError`. Use it for both
> `report_files`'s `mtime_utc` and `_report_stats`'s `newest_utc`, and for any other
> `datetime.fromtimestamp` on a file time in that module. `mtime_utc` and `newest_utc` become
> nullable: `null` means "the file's time cannot be read", which the page shows as a dash.
>
> No traceback reaches the log for this case, matching the FR-090 hard rule that read errors are
> never a traceback. Nothing is written; no migration.

**Out of scope**: catching these exceptions in `_query` or `_report_call` instead (that would turn one
bad file into a 404 or 400 for the whole folder, which is the same outage in a different suit); any
other `datetime` use outside file times; changing the `Guard`; the page.

**Requirement Refs**: FR-089, FR-090

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | A report folder holding one file at `st_mtime` 1e17 and two normal files: `report_files` returns 200 and all three files, the bad one with `mtime_utc` `null` and the others with their real times | FR-090 |
| 2 | Same fixture: `report_folders` returns 200; the folder's `newest_utc` is the newest *representable* time, or `null` when no file has one; `files` still counts every file | FR-089 |
| 3 | A negative `st_mtime` (before 1970) and `st_mtime` 0 are handled: either a real time or `null`, never an exception | FR-089, FR-090 |
| 4 | Sorting still works with an unrepresentable time present: `report_files` returns newest first and does not raise while sorting | FR-090 |
| 5 | No traceback: with the bad fixture, the dashboard's captured stderr holds no `Traceback` line for these two routes | FR-090 |
| 6 | Both routes answer 200 for the bad fixture through the real HTTP surface, not only at the core level (a `tests/surfaces/` test) | FR-089, FR-090 |
| 7 | Existing tests pass; `uv run pytest -q` passes. Real run: the Reproduce block above now prints 200 twice; paste the real output | — |

---

## UI / Design Acceptance Criteria

N/A for the backend fix. `mtime_utc` and `newest_utc` become nullable, so the page must render a
dash rather than `Invalid Date` — if the Kit view does not already handle `null` there, that is a
one-line follow-up on the frontend, noted here but not part of this task. UI Evidence rows ☐ N/A.

---

## Evaluation & Acceptance

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core tests/surfaces -q && uv run pytest -q
```

## Approach

**Vital slice**: one `_utc_or_none(mtime)` helper in `kit.py`, used by both call sites; make the two
fields nullable; tests with the bad fixture at both the core and the HTTP level. **Cut list**: no
change to `_query` or `_report_call`, no clamping to a sentinel date (a wrong time is worse than no
time), no page change.

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/kit.py` | the helper; `report_files` `mtime_utc`; `_report_stats` `newest_utc` |
| `tests/core/test_kit_report_files.py` | AC 1, 3, 4 |
| `tests/core/test_kit_reports.py` | AC 2 |
| `tests/surfaces/test_api_kit_report_files.py` | AC 6 |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `Guard` class in `api.py` | Unrelated |
| `src/easy_personal/core/migrations/`, `store.py` | No store change |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table
- [ ] UI Evidence rows ☐ N/A
- [ ] Supervisor notified: task ready for Stage 4 review

## Questions Log

* 2026-11-07 Found by `/verify HUN-242`. Both Stage 4 reviews had noted it below their own bar (`security-review` classed it as excluded DoS, correctly, since no path or file name leaks in the traceback); the real run showed it also breaks the already-merged FR-089 route, which raised it to P1.
