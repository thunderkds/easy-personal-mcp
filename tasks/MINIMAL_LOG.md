# Minimal-track Decision records

> One Decision record per minimal-track task (`docs/claude-md/tracks.md`), **newest first**: insert each
> new record directly under this note. The same block is the Supervisor's final chat message.

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
