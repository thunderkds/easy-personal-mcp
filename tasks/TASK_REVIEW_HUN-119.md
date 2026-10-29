# TASK_REVIEW — HUN-119: Kit tab: plain project cards with health, recent activity and a details toggle

> Sibling of `tasks/TASK_GUIDE_HUN-119.md`. Everything here is **filled by the reviewer at Stage
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
Checks: tests `npm --prefix web test && npm --prefix web run build && uv run pytest -q` — 178 Vitest passed, 750 pytest passed; reviews code-review — P3 1 (Risk Low: no security-review); real run stub fixture server, headless Chrome 375/768/1280 in reports/HUN-119/ (auto-checked, not /verify)
Not done: merge (owner asked for it; runs after verify); auto verify: running (T182)
Paused: auto-mode check failed — extended track, but the trace has no `code-review` Skill call (Stage 4 runs in auto mode)

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/KitView.test.jsx` (39 tests: `projectHealth`, `commitStats`, cards, chip, summary, sparkline, part lines, not-set-up line, details toggle, table, intro, empty state; fixtures in `web/src/test/fixtures.js`). `npm --prefix web test -- KitView` -> `Test Files  1 passed (1)  Tests  39 passed (39)`. Red first: with the cards absent, the same file gave `Tests  33 failed \| 6 passed (39)`. |
| Verification command run | ☑ pass | 2026-10-29T08:44:34Z, `npm --prefix web test && npm --prefix web run build && uv run pytest -q` in a clean shell (no `EASY_PERSONAL_*`): `Test Files  8 passed (8)` / `Tests  178 passed (178)`; `✓ built in 217ms`; `750 passed, 4 deselected, 1 xfailed, 1 warning in 111.05s`; `exit=0`. |
| Negative cases hold | ☑ pass | Tested: error rate exactly 10% is not flagged; `error_rate` null gives no failure text; unknown `stale` (null) is never shown as fresh; no-data and abstained parts never show a zero; 30 days ago is outside the window, 29 days ago inside; commits from a year ago excluded; 0 projects shows the empty message and no toggle; unknown extra part `ui` listed by key; verifier `abstained` reads "could not be scored"; `missing_on_disk` shows "(folder missing)". |
| verify | ☑ pass | owner ran /verify 2026-10-29; headless `claude -p '/verify HUN-119'` run, log .claude/verify-runs/HUN-119-d7736e2b12c2.log; hook record reads blocked only because the run's summary had no single verdict line: Kit tab card in headless Chrome at 375/1280 px: Healthy chip, summary, sparkline label, Not set up here line, Show/Hide details, no console errors; vitest 178/178; run flagged the chip stretching to card width, fixed in fa3fcac (dropped flex-basis:100%) and rechecked: vitest 178 passed — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | code-review (Supervisor, 2026-10-29): KitView ProjectCard/projectHealth/commitStats, details toggle, app.css, fixtures + tests, base HUN-118. lane: C2, 4 files, 464 lines, FITS. Findings after gate: P0 0, P1 0, P2 0, P3 1 (card id from folder name with spaces) — pass |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` -> `750 passed, 4 deselected, 1 xfailed, 1 warning in 111.05s` (same run as the verification command). Web suite: 8 files, 178 tests pass. |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Headless Chrome (`google-chrome --headless=new`) against a fixture dashboard (stub server with fixture `/api/kit` JSON for 3 projects; no real `~/.easy-personal/` or `~/.claude*/`): `reports/HUN-119/kit-1280.png`, `kit-768.png`, `kit-375.png`, `kit-375-details.png`. Verdict (my own reading of the screenshots): a non-technical reader sees at once that `billing-service` needs attention, and why ("Needs attention: 20% of tool calls failed; code index is out of date", amber chip plus an amber "stale" tag), that `easy-personal-mcp` is Healthy (green chip), and that `old-experiment` has no kit data (muted chip, "Not set up here: ..."). Sparklines and part lines are legible. No independent LLM-vision pass was run. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `grep -nE "#[0-9a-fA-F]{3,8}\b\|rgba?\(" web/src/app.css web/src/KitView.jsx` -> no match (exit 1): no color outside `tokens.css`. Chip pairs from tokens, computed WCAG contrast: Healthy `--color-up` on `--color-up-bg` 7.73; Needs attention `--color-gap` on `--color-gap-soft` 6.58; No kit data `--color-muted` on `--color-surface-2` 6.97; muted text on surface 8.15; body text on surface 15.86 (all >= 4.5). Sparkline uses `Sparkline` from `Widgets.jsx` with `tone="cyan"`. |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 px: 1 card per row, no page overflow, details table scrolls inside its `.table-wrap` (existing behavior). 768 px: 2 per row. 1280 px (viewport, minus the 15rem sidebar): 3 cards per row (guide asks 2-3). Files in `reports/HUN-119/`. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-29T08:37:45Z at commit 122d175 (before any implementation commit), on the fixture `kit`. Command: a throwaway probe test (`web/src/probe.test.jsx`, deleted after) that renders `<KitView />` and prints the table rows; plus `grep -c "Show details\|Healthy\|Needs attention" web/src/KitView.jsx` (08:35:33Z) which printed `0`.

```
BEFORE-ROWS: alpha40 tool calls, 0 errors (0.0%); 2 kit tasksno datastale · indexed 10/1/2026, 7:00:00 AM7 commits || beta (missing on disk)no datano datano dataabstained || gamma5 tool calls, 2 errors (40.0%); 1 kit task; 1 abstainedno dataunknown · indexed 10/21/2026, 7:00:00 AM0 commits
BEFORE-HAS-SHOW-DETAILS: false
$ npm --prefix web test -- KitView   ->  Tests  13 passed (13)
```

The kit tab is only a technical table (raw "40 tool calls, 0 errors", "no data", all-time commit counts), with no health verdict, no recent-activity text and no toggle.

**AFTER**: same probe idea on the same fixture after the change (2026-10-29T08:44Z run of the test file; page in `reports/HUN-119/kit-1280.png`). The Kit tab now renders one card per project, newest first, e.g. for the fixture `delta`: chip `Healthy`; `Last active today. 1,200 agent tool calls, none failed. 3 commits today, 5 in the last 30 days.`; `Agent kit (tool calls the kit traced) — 9 kit tasks`; `Code index (easy-context) — indexed 2 days ago, fresh`; `Verifier score — 82/100`. `gamma`: chip `Needs attention: 40% of tool calls failed`. `beta`: chip `No kit data`, `Not set up here: ...`. The intro line is shown and a `Show details` button reveals the unchanged table (`BEFORE-HAS-SHOW-DETAILS` is now true: `getByRole('button', { name: 'Show details' })` passes in `KitView.test.jsx`).

**DELTA**: The owner can now see which project needs attention and why, what is recent, and what each kit part means, without reading the table; the table is one click away.

**WITNESS**: frontend-developer sub-agent, BEFORE at 2026-10-29T08:37:45Z and the final verification run at 2026-10-29T08:44:34Z, both in worktree `HUN-119`; Bash calls attributed to HUN-119 through the active-task state file. The Supervisor confirms against `memory/event-trace/HUN-119.jsonl`.
