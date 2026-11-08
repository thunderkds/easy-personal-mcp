# Minimal-track Decision records

> One Decision record per minimal-track task (`docs/claude-md/tracks.md`), **newest first**: insert each
> new record directly under this note. The same block is the Supervisor's final chat message.

## DRAFT-mtime-500 — a file with an out-of-range mtime made the report routes answer 500 · minimal
**Asked:** "fix the mtime bug then merge HUN-242"
**Decisions:**
- Minimal track — C1, Risk Low, one session, no UI component. The one worry was that `newest_utc` becoming null would break the page, and it cannot: `kit.py:333` already returned null for an empty folder, `web/src/WorkspaceKit.jsx:53` guards it with `if (f.newest_utc)`, and `web/src/WorkspaceKit.test.jsx:200` already tests the null case.
- Fixed on a branch off `HUN-242`, not off `main` — the flaw has two call sites and only one of them (`_report_stats`) is on `main`; the other (`report_files`) exists only on HUN-242's branch. Branching off HUN-242 fixes both at once, so no merge ever puts the bug into a new route on `main`.
- One helper `_file_utc(mtime)` used by both call sites, rather than two local try/excepts — the module had the same formatting twice and would have grown a third copy.
- It catches `OSError` as well as `OverflowError` and `ValueError`: the real failure was `OSError: [Errno 75] Value too large for defined data type`, which is what the C library returns, not the `ValueError` the reviews predicted.
- `_report_stats` now tracks the newest *readable* time and keeps its formatted stamp, instead of taking `max()` over every mtime and formatting at the end. Taking the max first would let one absurd time hide every real one, so the folder would report no time at all.
- Assumed: a null `mtime_utc` is right for FR-090's own page, which is not built yet — its guide (`tasks/TASK_GUIDE_HUN-243.md`) now says to render a dash for it, so HUN-243 cannot be written against a non-null assumption.
- Known and left alone: a file with an absurd mtime still sorts first in `report_files`, because the sort uses the raw mtime. It is listed with a null time, so the page shows a dash. Sorting unreadable times last would be a second behaviour change for no real gain.
**Checked:**
- New tests, all red before the change and green after: `test_unreadable_mtime_is_listed_with_a_null_time`, `test_unreadable_mtime_does_not_break_the_sort_or_the_other_files`, `test_unreadable_mtime_keeps_the_folder_listed_with_the_newest_readable_time`, `test_newest_utc_is_null_when_no_file_has_a_readable_time`, `test_an_unreadable_mtime_does_not_make_either_route_answer_500`. Before: `OSError: [Errno 75] Value too large for defined data type` at `kit.py:440`. Three more tests passed before and after (`test_zero_and_negative_mtime_are_either_a_time_or_null`, `test_unreadable_mtime_does_not_break_reading_a_file`, and the read path).
- Suite: `uv run pytest -q` — `1756 passed, 2 skipped, 6 deselected, 1 xfailed in 187.52s` (1749 before, plus the 7 new tests).
- Real run: a temp `EASY_PERSONAL_HOME`, a fixture workspace, `easy-personal dashboard`, then `touch`ing one report file to `st_mtime` 1e17. Before the fix both routes printed `500 Internal Server Error` with a 48-line traceback on the dashboard log. After: `/api/kit/reports` -> `200`, `/api/kit/report-files` -> `200` with `{"path":"sub/b.md","size":4,"openable":true,"mtime_utc":null}` beside `{"path":"a.html",...,"mtime_utc":"2026-11-08T00:08:44Z"}`, the folder's `newest_utc` still a real time, reading an unaffected file still `200`, and `0` new lines on the dashboard log, so no traceback.
**Files:** `src/easy_personal/core/kit.py`, `tests/core/test_kit_report_files.py`, `tests/core/test_kit_reports.py`, `tests/surfaces/test_api_kit_report_files.py`, `tasks/TASK_GUIDE_HUN-243.md`
**Not done:** nothing — the user asked for the merge in the same request, so this branch merges to `main` together with HUN-242.

## HUN-217 — Warnings page "new" cause chip shows amber, not grey · minimal
**Asked:** "fix small"
**Decisions:**
- Selector `.chip-new` → `.chip.chip-new` — `.chip` is declared later in `app.css` with equal specificity, so it overrode the amber colours; raising specificity is a one-word change and does not move rules around.
- Assumed: amber is the intended colour (the HUN-216 rule already named `--color-amber-soft` / `--color-amber`) — the fix only makes that rule apply.
**Checked:**
- New test `WarningsPlanView.test.jsx` › `HUN-217: the "new" cause chip` › `wins over the base .chip colours (selector .chip.chip-new, amber)` — failed before the change (`1 failed | 26 passed (27)`), passes after.
- Suite: `cd web && npx vitest run` — `Test Files 21 passed (21) / Tests 600 passed (600)`; `npm run build` — `✓ built in 184ms`
- Real run: headless Chrome on the built CSS bundle with `<span class="chip">` and `<span class="chip chip-new">` — printed `plain rgb(140, 149, 144) | new rgb(227, 182, 107) bg rgba(227, 182, 107, 0.12)` (`--color-amber:#e3b66b`).
**Files:** `web/src/app.css`, `web/src/WarningsPlanView.test.jsx`
**Not done:** merge (the user decides)

## HUN-205 — Commit the Easy Kit update 75040d9 → 04b019a (kit files only) · minimal
**Asked:** "yes" — answer to: "Should I start with option 1?" (commit the kit update on its own branch, check the 3 `.bak-75040d9` backups against the new versions, delete the backups, run the tests; merge waits for the owner).
**Decisions:**
- Re-applied one project edit the update dropped: Stage 4 "easy-verifier MCP `score` (`scope="changes"`, advisory) always" in `AGENTS.md`, and step 4b in `docs/claude-md/pipeline-stages.md` — the backups are labelled "your edits"; memory says never skip the score.
- Step 4b now points at the kit's new TASK_REVIEW row "Companion score (easy-verifier)" and its shape, and says the merge gate blocks on a ☑ fail row — the new kit's `pre_bash_block_unsafe_merge.py` reads that row; the old "never blocks merge" is no longer true.
- Every other line in the 3 backups is old upstream text that the update reworded (CLAUDE.md → `@AGENTS.md` import, auto-mode notes, `templates/general-agent-template.md` path) — nothing else lost; backups deleted.
- Left out of the commit: `.claude/verify-runs/*.log` (old `/verify` runtime output, HUN-113..121) and `tasks/TASK_GUIDE_HUN-198.md` / `HUN-201.md` (board-pull cache changes) — not part of the kit update.
- Assumed: the working-tree kit files are exactly the 04b019a update (`.claude/harness-lock.json` `kit_commit` and `.claude/update-log.md` say so) — safe because the full suite and the hooks run clean.
**Checked:**
- New test `tests/test_kit_local_edits.py::test_stage4_always_runs_the_easy_verifier_score` — failed before the re-applied edit (`AssertionError` at line 17), passes after (`1 passed in 0.10s`).
- Suite: `uv run pytest -q` — `1334 passed, 6 deselected, 1 xfailed, 1 warning in 282.19s (0:04:42)`
- Real run: the 3 Stop hooks wired in `.claude/settings.json` (`stop_review_reminder.py`, `stop_auto_mode_check.py`, `stop_auto_verify.py`) and `pre_bash_block_unsafe_merge.py`, each fed a sample hook event — all printed nothing and exited 0.
**Files:** 37 modified kit files + new `.claude/hooks/lib/auto_mode.py`, `.claude/hooks/lib/verify_record.py`, `.claude/hooks/stop_auto_mode_check.py`, `.claude/hooks/stop_auto_verify.py`, `.claude/update-log.md`, `docs/claude-md/tracks.md`, `skills/resolve-question/`, `templates/MINIMAL_LOG_template.md`, `templates/general-agent-template.md`; deleted `agents/general-agent-template.md`; `tests/test_kit_local_edits.py`, `tasks/TASK_GUIDE_HUN-205.md`, `tasks/MINIMAL_LOG.md`
**Not done:** merge and `/verify` (the user decides); `.claude/verify-runs/` is not gitignored (left as is)

## HUN-195 — kit_findings: a finding needs a gap of at least 5% to carry a habit · minimal
**Asked:** "Yes, at least 5% (Recommended)" — answer to: "The first real findings show small gaps (3.6% to 9.4%) on 12 to 24 kit sessions. Should a finding need a minimum gap before it carries a habit?"
**Decisions:**
- Fixed constant `kitperf.MIN_GAP_PCT = 5.0`, not a config key — the owner set one number; no speculative setting.
- The gap is checked on the rounded `delta_pct` (the number the text shows) — what the reader sees decides.
- A better comparison under 5 % goes to `other` with `habit` null — keeps every comparison visible, as FR-063 requires.
- PRD FR-063, `PROJECT_SPEC.md` (Habit, Kit performance) and the MCP tool description name the 5 % rule.
**Checked:**
- New test `test_a_better_gap_under_5_percent_goes_to_other_without_a_habit` (gaps −4.0 / −4.5 / −5.0 %) — failed before the change (`Left contains one more item: {... 'delta_pct': -4.0 ...}`), passes after.
- Suite: `uv run pytest -q` — `1239 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 165.23s`
- Real run: `EASY_PERSONAL_HOME=<v8 store copy> uv run easy-personal kit-findings` (2026-11-03 03:47 UTC) — printed 1 finding with a habit (easy-ui +9.4 % good results, 12 vs 21), then 7 `other` lines; easy-context −4.4 %, easy-ui −3.6 % and easy-memory −1.2 % moved to `other`.
**Files:** `src/easy_personal/core/kitperf.py`, `src/easy_personal/surfaces/mcp.py`, `tests/core/test_kitperf.py`, `PRD.md`, `PROJECT_SPEC.md`
**Not done:** merge (the user decides)
