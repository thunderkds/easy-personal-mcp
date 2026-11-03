# TASK_REVIEW — HUN-200: Dashboard forms UX + floating notifications + floating loading indicator

> Sibling of `tasks/TASK_GUIDE_HUN-200.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Track extended, Risk Low, C2 — RULE: `AGENTS.md` § Tracks (a UI component puts the task on the extended track); UI only, no stored data or API change
2. "The form" read as every dashboard form: the workspace form, Add connector and the agent accounts form — SAFER OPTION: `WorkspacesView.jsx` holds the only `<form>` elements, but Agents has the same result/error pattern; covering all keeps one style ⚠ decided for you — please review
3. Toasts in the bottom-right corner (full width at the bottom on a phone); success hides after 5 s, errors stay until closed, max 3 — RULE ⚠ (taste) decided for you — please review
4. The Confirm/Cancel plan stays inline, moved next to the form's buttons, not a toast — SAFER OPTION: a decision the owner must take should not disappear on a timer ⚠ decided for you — please review
5. Failed page loads stay inline; the Rail card's own loading and the Live link status stay inline — SAFER OPTION: they are page or card state, not notifications ⚠ decided for you — please review
6. Stage 0.5 grill/brainstorm answered from the request and code: PRD US-028 + FR-065 written by the Supervisor; brainstorming took the one recommended direction — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` rows 2 and 4 ⚠ (taste) decided for you — please review
7. Sub-agent spawned on this session's CLI and model — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` row 14
8. Stage 4 code-review: no P0/P1; P2/P3 left as suggestions, not applied — RULE: `.claude/skills/code-review/SKILL.md` Phase 4 (only P0/P1 are fixed automatically)
9. html-report for the review not rendered — SAFER OPTION: findings are recorded here; the page adds no check ⚠ decided for you — please review
Checks: tests `cd web && npx vitest run` — 20 files / 529 tests passed (was 18 / 487); `npm --prefix web run build` — built; `uv run pytest tests/test_web_toolchain.py -q` — 16 passed (agent run 2026-11-03T12:35Z); reviews code-review — P0 0, P1 0, P2 1, P3 3; real run: dashboard from the branch on a copy of the owner store, screenshots `after-{1280,375}-*.png` in the session scratchpad `shots/` — field errors next to fields, toast bottom-right, loading pill floating (auto-checked, not /verify)
Not done: merge (yours); /verify: ☑ pass (owner-run 2026-11-03)

### Stage 4 code-review (HUN-200)

Scope: 18 files (`git diff main...HEAD`), callers = `App.jsx` provider. `lane_check`: `lane: C2, 17 files, 1041 lines, EXCEEDS (files 17 > 14; lines 1041 > 500)` — the loading pill touches 10 views by design; label kept C2. Entry point `ToastProvider` found in `web/src/App.jsx:99`. Startup reads line complete.

## P0 — Must Fix
None.

## P1 — Should Fix
None.

## P2 — Recommended
| File | Line | Finding | Confidence | Action |
| -- | -- | -- | -- | -- |
| web/src/WorkspacesView.jsx | 109 | A long multi-line success result (e.g. a connector plan answer) now hides after 5 s; before it stayed on the page | 75 | Keep success toasts with more than ~3 lines open until closed, or longer timeout |

## P3 — Optional
| File | Line | Finding | Confidence | Action |
| -- | -- | -- | -- | -- |
| web/src/app.css | 522 | At 375 px the loading pill overlaps the wrapped scope line under the header (`after-375-loading-usage.png`) | 100 | Place the pill lower on phones or at the bottom-centre |
| web/src/app.css | 516 | Save is first but has the same style as the other buttons; no visual primary | 75 | Add a primary button style from tokens |
| web/src/WorkspacesView.test.jsx | — | Form tests were written with the implementation, not seen failing alone (agent report) | 100 | Process note only; tests do assert the ACs |


## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/Toast.test.jsx` (AC5, AC6, styles), `web/src/Loading.test.jsx` (AC7, all views), `web/src/WorkspacesView.test.jsx` (describe blocks 'workspace form UX (HUN-200)' and 'add connector form UX (HUN-200)': AC1-AC4, AC6), `web/src/AgentsView.test.jsx` ('Agents toasts and form hints (HUN-200)'). `npx vitest run` 2026-11-03T12:35:25Z: `Test Files  20 passed (20)` / `Tests  529 passed (529)`. Existing tests changed only where they queried the inline result/alert (WorkspacesView 2, AgentsView 2) or the old label 'Color (#rrggbb)', and they now render the view inside `ToastProvider`. |
| Verification command run | ☑ pass | 2026-11-03T12:35:25Z: vitest `20 passed (20)` files, `529 passed (529)` tests; `npm --prefix web run build`: `index-CYI4i1jm.css 33.65 kB`, `index-D7E3XSij.js 312.04 kB`, `built in 160ms`; `uv run pytest tests/test_web_toolchain.py -q`: `16 passed in 2.31s`. |
| Negative cases hold | ☑ pass | Tests: empty and whitespace-only Name, bad colour and bad email each send no request (`writes(fn)` length 0) and set `aria-invalid`; API error shows a toast, not an inline alert; a typed connector secret never appears in a toast; toast text is rendered as text (`<b>` not parsed); `useNotify` outside a provider throws; unmount with a timer running does not warn. |
| verify | ☑ pass | Owner ran `/verify HUN-200` 2026-11-03: branch build installed in an isolated tool dir, `--version` 0.4.0, `ping` "core ok", MCP stdio tools/list 11 tools; dashboard from the branch on a `.backup` copy of the owner store, driven in headless Chrome at 1280×720 and 375×812: empty submit → "Name is required" next to Name; bad colour/email → field errors; save → success toast bottom-right; duplicate name → "Could not complete: a workspace with this name exists" error toast that stays; Usage loading → floating pill; scrollWidth = innerWidth (no horizontal scroll). Test dashboard stopped; owner dashboard untouched — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Before/after pairs at 1280x720 and 375x812 in `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/54000ba3-d7c0-4535-8376-908a80d33c6d/scratchpad/shots/` (`before-*` vs `after-*`: ws-empty-submit, ws-bad-values, ws-result, ws-error, loading-usage). After: field errors next to fields, grid, toast floats bottom-right and the page does not move, loading shows as a fixed pill with a spinner and nothing in the flow. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | All new CSS uses `tokens.css` variables only; `Toast.test.jsx` asserts no hex/rgb in the toast/pill/spinner rules, fixed position with `--space-4` gutter, full width on phone, `animation: none` for `.spinner` and `.toast` under `prefers-reduced-motion: reduce`. Labels, `aria-describedby` hints and errors, `aria-invalid`, `role=status`/`role=alert` and the close button (a native button) are asserted in vitest. Not done: a manual keyboard walk in a real browser, and a light theme check (the page is dark only, `tokens.css`). |
| **UI: Responsiveness at target viewports** | ☑ pass | 375x812 and 1280x720 in headless Chrome: `document.documentElement.scrollWidth` / `innerWidth` = 375/375 and 1280/1280 (1265/1280 where the page has a vertical scrollbar), no horizontal scroll in any of the 10 after-shots; at 375 the form is one column and toasts span the width with a 16 px gutter. An error toast stays until closed and can cover the form's buttons until then (success hides after 5 s). |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (unmodified `HEAD` 3474b5c source built to `scratchpad/oldstatic` and served from this worktree; screenshots re-captured 2026-11-03T12:34:18Z, before any implementation commit; first capture 12:15:30Z had a stale 375 px pass because Chrome did not reload the same URL, so it was replaced; dashboard on a `sqlite3 .backup` copy of the owner store, never the live store):

`cd web && npx vitest run` (2026-11-03T12:16:24Z, before any change):
```
 Test Files  18 passed (18)
      Tests  487 passed (487)
```
Headless Chrome (CDP script `scratchpad/shot.mjs`) screenshots in `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/54000ba3-d7c0-4535-8376-908a80d33c6d/scratchpad/shots/`, 1280x720 and 375x812:
- `before-1280-ws-empty-submit.png`, `before-375-ws-empty-submit.png`: Workspaces > New workspace, Save with empty Name. A request is sent; "Could not complete: name must match [a-z0-9][a-z0-9-]{0,39}" shows inline ABOVE the form and pushes the fields down. No hints, no field error, colour is a plain text box (label "Color (#rrggbb)"), no required mark.
- `before-*-ws-bad-values.png`: colour `#12345` and email `a@` get no client check.
- `before-*-ws-result.png`: result "Workspace 'demo-x-1280' created." is an inline block in the page flow above the form.
- `before-*-ws-error.png`: failed action (duplicate name) is an inline red line above the form.
- `before-*-loading-usage.png`: Usage with `/api/usage/*` held: the plain line "Loading usage" sits in the page flow under the period buttons.

**AFTER** (2026-11-03T12:34:44Z screenshots, 12:35:25Z checks, same dashboard and store copy, same script, branch HUN-200 working tree):

`cd web && npx vitest run`: `Test Files  20 passed (20)`, `Tests  529 passed (529)` (487 before, +42 new/updated tests, 2 new test files).
Screenshots in `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/54000ba3-d7c0-4535-8376-908a80d33c6d/scratchpad/shots/`: `after-1280-ws-empty-submit.png` (Name required error under the field, red border, focus on Name, two-column grid, hints under Color/Icon/Email/Repos, colour picker), `after-1280-ws-bad-values.png` (colour and email errors next to their fields), `after-1280-ws-result.png` (success toast bottom-right with a close button, page did not move), `after-1280-ws-error.png`, `after-1280-loading-usage.png` (floating "Loading usage" pill top-right with a spinner, no line in the flow), and the same five at `after-375-*.png` (one column, toasts full width with a 16 px gutter).

**DELTA**: The owner can see what is wrong with a workspace form before anything is sent (labels, hints, colour picker, errors next to fields), sees a busy Save button, and gets action results, errors and loading as floating toasts and a pill that never push the page.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
