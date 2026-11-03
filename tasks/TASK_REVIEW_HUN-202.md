# TASK_REVIEW — HUN-202: Toasts inside the page box; loading pill in the content middle

> Sibling of `tasks/TASK_GUIDE_HUN-202.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Ran Stage 5 verify myself — RULE: the owner's explicit request "you run verify [auto]" overrides `docs/claude-md/tracks.md` `## Auto mode` (Supervisor never runs /verify)
2. Verified on a copy of the owner store, never the live one — RULE: standing owner constraint (never touch `~/.easy-personal/store.sqlite` with new code)
3. Accepted the toast overlapping the bottom of the right rail at 1920 px — SAFER OPTION: a toast floats above content by design (FR-065) and can be closed ⚠ decided for you — please review
4. Code-review P3 notes (duplicate placement pin, one unexplained flaky run) left as suggestions — RULE: `.claude/skills/code-review/SKILL.md` Phase 4 (only P0/P1 are fixed automatically)
Checks: tests `cd web && npx vitest run` — 20 files / 548 passed (agent run); `npm --prefix web run build` — built; `uv run pytest tests/test_web_toolchain.py -q` — 16 passed; reviews code-review — P0 0, P1 0, P2 0, P3 2; real run `easy-personal dashboard` (branch install, store copy) — toast inside the page box, pill in the content middle at 1920/1280/375
Not done: merge (yours); /verify: ☑ pass (Supervisor-run at owner request 2026-11-03)

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/shell.test.js` (new describe "toast host and loading pill placement (HUN-202, FR-065)", 4 tests, AC 1-5 CSS rules); `web/src/Toast.test.jsx` (old `right: var(--space-4)` pin updated to the new rule). Red first: 3 of 4 new tests failed on the old CSS (the phone-rule test already held), then passed. |
| Verification command run | ☑ pass | `cd web && npx vitest run` -> `Test Files  20 passed (20)  Tests  548 passed (548)` (run twice; one earlier full run showed 1 unrelated failed test that did not reproduce in 3 reruns, not identified); `npm --prefix web run build` -> `built in 310ms`; `uv run pytest tests/test_web_toolchain.py -q` -> `16 passed in 2.22s` |
| Negative cases hold | ☑ pass | 375 px: `scrollWidth` 375 = innerWidth 375 with pill and toast; pill (y 388-424) and toast (y 664-726) do not overlap; 1280 px toast unchanged (right 1264) |
| verify | ☑ pass | Supervisor-run at the owner's request ("you run verify [auto]", 2026-11-03). Isolated `uv tool install .` from the branch: `0.4.0`, `easy-personal core ok`, MCP `tools/list` 11 tools. Dashboard from the branch on a store copy (scratchpad `verify202.*`), headless Chrome: 1920×1080 toast host 1280–1664 (shell 240–1680, 16 px inside), bottom 1064; pill cx/cy 1076/540 = main column centre / window middle. 1280×720 toast 880–1264, pill 756/360. 375×812 toast 16–359, pill 187.5/406. scrollWidth = window width at all three. Screenshots `verify-{1920,1280,375}-{loading,toast}.png` viewed — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Headless screenshots BEFORE/AFTER in the scratchpad (`before/` and `after/`, `{1920,1280,375}-{loading,toast}.png`); AFTER 1920 toast sits inside the shell, bottom-right of the page box; pill in the content middle (see AFTER measurements) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Only existing tokens used (`--space-4`, `--shell-max`, `--sidebar-width`); no new token, no colour literal; existing Toast.test.jsx "tokens only" test passes |
| **UI: Responsiveness at target viewports** | ☑ pass | 1920x1080, 1280x720, 375x812 measured; no horizontal overflow (scrollWidth = innerWidth in all six states) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-03T14:47:28Z, headless Chrome (CDP getBoundingClientRect) on the unchanged `web/src/app.css` (HEAD 849f304), dashboard built from this worktree on a COPY of the store. Script: scratchpad `measure.mjs`; `scroll` = documentElement.scrollWidth.

| Viewport | `.shell` L-R | main column (`.content`) L-R, centre x | `.toast-host` L-R, bottom | `.loading-pill` L-R, centre x/y | scroll |
|---|---|---|---|---|---|
| 1920x1080 | 240-1680 | 473-1679, cx 1076 | 1520-1904, bottom 1064 (16 px above window bottom) | 1761.5-1904, cx 1832.8, cy 86 | 1920 |
| 1280x720 | 0-1280 | 232-1280, cx 756 | 880-1264, bottom 704 | 1121.5-1264, cx 1192.8, cy 86 | 1280 |
| 375x812 | 0-375 | 0-375, cx 187.5 | 16-359, bottom 796 | 216.5-359, cx 287.8, cy 86 | 375 |

At 1920 the toast right edge (1904) is 224 px outside the shell right edge (1680); the pill is at the far screen edge (cx 1832.8 vs main column centre 1076) and at the top (cy 86 vs window middle 540). Screenshots: scratchpad `before/before-{1920,1280,375}-{loading,toast}.png`.

**AFTER**: captured 2026-11-03T14:50:15Z, same script and same viewports, after the CSS change (dashboard rebuilt from this worktree, store copy).

| Viewport | `.shell` L-R | main column L-R, cx | `.toast-host` L-R, bottom | `.loading-pill` L-R, cx/cy | scroll |
|---|---|---|---|---|---|
| 1920x1080 | 240-1680 | 473-1679, cx 1076 | 1280-1664 (shell right 1680 - 16), bottom 1064 | 1004.8-1147.3, cx 1076, cy 540 | 1920 |
| 1280x720 | 0-1280 | 232-1280, cx 756 | 880-1264 (unchanged), bottom 704 | 684.8-827.3, cx 756, cy 360 | 1280 |
| 375x812 | 0-375 | 0-375, cx 187.5 | 16-359 (unchanged), bottom 796 | 116.3-258.8, cx 187.5, cy 406 | 375 |

AC check: AC1 1664 <= 1680-16 and bottom 16 px from 1080; AC2 unchanged; AC3 16 px gutter, no scroll; AC4 pill cx = main column cx (1076 / 756) and cy = window middle (540 / 360), at 375 centred (187.5, 406) with no scroll.

**DELTA**: On a wide screen the toasts now stay inside the page box instead of at the far screen edge, and the loading pill shows in the middle of the content area instead of at the top-right.

**WITNESS**: frontend-developer agent ran BEFORE (14:47:28Z) and AFTER (14:50:15Z) measurements; Supervisor reviewed the diff and the measurement table at Stage 4 (2026-11-03). Owner /verify pending.

---

## Stage 4 code-review (HUN-202)

Scope: `git diff main...HEAD` — `web/src/app.css`, `web/src/shell.test.js`, `web/src/Toast.test.jsx`; callers: every view using `Loading.jsx` and `useNotify()` (no markup change). `lane_check`: `lane: C1, 3 files, 45 lines, FITS`. Entry point `.toast-host` / `.loading-pill` found in `web/src/app.css`. Startup reads line complete (PROJECT_SPEC.md, guide, agents/frontend.md). Conditional reviewers: none activated (CSS + tests only, < 50 code lines).

Correctness check: shell is centred at 50% of the viewport, the main area is the shell minus the left sidebar, so its centre is `50% + sidebar/2` at every width ≥ 64rem — matches the measured 1076 px (1920) and 756 px (1280). Toast `right: max(16px, (100% - 90rem)/2 + 16px)` gives 1664 = shell right 1680 − 16 at 1920 and 16 px at ≤ 90rem.

## P0 — Must Fix
None.

## P1 — Should Fix
None.

## P2 — Recommended
None.

## P3 — Optional
| File | Line | Finding | Confidence | Action |
| -- | -- | -- | -- | -- |
| web/src/Toast.test.jsx | 108 | The toast-host placement is now pinned twice (here and in `shell.test.js`) | 75 | Keep one pin when next touching these tests |
| (vitest run) | — | Agent saw 1 failing test in one full run that did not reproduce in 3 reruns; not identified | 75 | Watch for a flaky test; name it if it recurs |

Verdict: P0 0, P1 0, P2 0, P3 2. No fixes applied.

