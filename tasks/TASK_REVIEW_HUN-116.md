# TASK_REVIEW — HUN-116: Agents account cards read like the status line (email, model, tokens, 5h/7d limit bars)

> Sibling of `tasks/TASK_GUIDE_HUN-116.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Task breakdown and labels approved — FACT: owner message 2026-10-29 "do all by parallel with tag [auto], merge to main and push" after the Stage 2 plan in `PROJECT_SPEC.md` § Tasks
2. Requirement Fidelity Gate signed auto (Supervisor) — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (row 13)
3. Spawn model from the CLI Model Table — RULE: `PROJECT_SPEC.md` § CLI Model Table
4. Waves (stacked branches) instead of all 8 from main: tasks that edit the same files start from their predecessor's branch — RULE: `PROJECT_SPEC.md` § Tasks ("Waves: 1 = HUN-113, 114, 115, 117 …")
5. Merge to main and push after `/verify` passes — FACT: owner message 2026-10-29 "merge to main and push"; `docs/claude-md/tracks.md` `### Edge cases` "the explicit ask wins for X"
Checks: pending
Not done: merge (owner asked for it; runs after verify); auto verify: running (T182)


## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/AgentsView.test.jsx` (describe "Agents status card (HUN-116)", 10 tests: AC1-AC6, 70/90 boundaries, edge cases, actions kept) + fixtures in `web/src/test/fixtures.js`. Run 2026-10-29T08:32:14Z: `npm --prefix web test` -> `Test Files  8 passed (8)`, `Tests  144 passed (144)` (134 before, +10). Red first: 9 of the 10 failed before the implementation (see Demonstration BEFORE). |
| Verification command run | ☑ pass | `npm --prefix web test && npm --prefix web run build && uv run pytest -q` -> vitest `144 passed (144)`; build `✓ built in 200ms` (index-DwmsBY5S.js 264.49 kB, gzip 80.56 kB); pytest `750 passed, 4 deselected, 1 xfailed, 1 warning in 159.83s`. (Run before a one-line CSS tweak that dropped the card shadow; vitest and build re-run after it, both green.) |
| Negative cases hold | ☑ pass | Tests: `limits: null` -> README hint only for Claude, none for Codex; `bare` account with all identity null -> no 👤 line, "last active never"; window with `used_percentage` null -> no bar; stale window -> "as of <time>, reset since", no bar; reset time passed with `stale: false` -> "resets soon"; boundaries 69 green / 70 amber / 89 amber / 90 red; existing accounts without `status_line` still render (all 134 older tests pass). |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` -> `750 passed, 4 deselected, 1 xfailed` after `npm --prefix web run build`; vitest 144/144. |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Screenshots `reports/HUN-116/agents-375.png`, `agents-768.png`, `agents-1280.png` (headless Chrome against the built page served by a stub server that answers `/api/accounts` and `/api/workspaces` with fixture data in the HUN-115 shape; the real API is not on this branch; no real `~/.easy-personal`/`~/.claude*` read). Verdict (own vision review): each card reads top to bottom as name + chip, 👤 email · plan · org, model · last active, tokens today / 7 days, then green (43%), amber (75%) and red (92%, 91%) bars with reset countdown; the account with no snapshot shows the muted README hint; Codex shows no hint. Actions and workspace picker stay below. Note: the screenshots are not committed (`reports/` is not tracked); they are on disk in the worktree. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Colors only from `tokens.css`: `git diff` added lines in `app.css`, `AgentsView.jsx`, `format.js` grep for `#hex`/`rgb(` -> none. Fill colors: `--chart-green`, `--chart-amber`, `--color-error`; bar is the existing `Progress`. Contrast computed: text/surface 15.86:1, muted/surface 8.15:1 (both >= 4.5). Bar colors are not the only signal: each bar has the percentage in text and an `aria-label`. |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 px: one card per row, bars full width, long email wraps (no horizontal overflow seen). 768 px: one card per row at 22rem min (content area is narrow with the sidebar hidden). 1280 px: 2 cards per row with the 15rem sidebar. Grid: `repeat(auto-fill, minmax(min(100%, 22rem), 1fr))`. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-29T08:26:19Z in the worktree, before any implementation commit (new tests written first; `AgentsView.jsx` unchanged, so the card has no email, model, tokens or limit bars):

```
$ npm --prefix web test -- AgentsView
 ❯ src/AgentsView.test.jsx (33 tests | 9 failed)
     × AC1: shows email, readable plan and organization; omits the line when all are null
     × AC2: shows last model and relative last-active time, "never" when null
     × AC3: shows tokens today and last 7 days with thousands separators
     × AC4: limit bars use Progress, tone by threshold, text with reset countdown, aria-label with numbers
     × AC4: boundaries are 70 and 90
     × AC5: a stale snapshot says "as of <time>, reset since" and draws no bar for that window
     × AC5: no snapshot shows the muted README hint (Claude accounts only)
     × edge: a window without a percentage is hidden; a reset already passed says "resets soon"
     × AC6: works in By provider
```

**AFTER**: same command, 2026-10-29T08:32:14Z, after the change:

```
$ npm --prefix web test
 Test Files  8 passed (8)
      Tests  144 passed (144)
 ✓ Agents status card (HUN-116) > AC1 ... AC2 ... AC3 ... AC4 ... AC4 boundaries ... AC5 stale ... AC5 hint ... edge ... AC6 By provider ... actions stay (10 of 10)
```

Rendered card (screenshot `reports/HUN-116/agents-1280.png`): "Claude company · confirmed ✓", "👤 owner@company.test · subscription · Company Org", "claude-opus-5-5 · last active 2 h ago", "Tokens: 1,234,567 today · 56,789,012 last 7 days", "5h 43% · resets in 1h 19m" (green bar), "7d 75% · resets in 3d 3h" (amber bar).

**DELTA**: The owner can read each agent account on the Agents page like the status line: who, which model, last activity, tokens, and 5h/7d plan-limit bars coloured by use.

**WITNESS**: implementing agent (frontend-developer) ran the commands on 2026-10-29 in worktree `easy-personal-mcp-wt/HUN-116`; the reviewer confirms against `memory/event-trace/HUN-116.jsonl` (not checked by the implementing agent).
