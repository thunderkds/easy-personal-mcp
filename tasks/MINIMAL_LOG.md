# Minimal-track Decision records

> One Decision record per minimal-track task (`docs/claude-md/tracks.md`), **newest first**: insert each
> new record directly under this note. The same block is the Supervisor's final chat message.

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
