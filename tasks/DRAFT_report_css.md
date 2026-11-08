# TASK_GUIDE — DRAFT: an opened report keeps its own CSS (FR-093, PRD question 32)

> **Draft.** The board is unreachable and `LINEAR_API_KEY` is not set, so no task ID exists yet.
> A new task's ID comes from the board and a create cannot be queued
> (`scripts/board_sync.py:374-376`). When the board is reachable, run
> `python3 scripts/board_sync.py create --guide tasks/DRAFT_report_css.md`.
> Owner request 2026-11-08: "when I open the report, it does not contain the CSS related, should update that".

**Date**: 2026-11-08
**Complexity Level**: C1
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Read `PRD.md` FR-090, FR-093, NFR-002, NFR-008; `src/easy_personal/surfaces/api.py` (the `CSP` constant and the whole `Guard` class); `tests/surfaces/test_api.py`, `tests/surfaces/test_api_live.py`, `tests/surfaces/test_api_kit_report_files.py` (the three files that assert the CSP today)

---

## Mental Model (measured, not assumed — `/verify HUN-243`, 2026-11-08)

HUN-243 opens a report file in a new tab as a `blob:` URL. A `blob:` document is a local scheme, so
it **clones the creating page's policy container** — it inherits the dashboard's CSP rather than any
header the route sets. That inherited policy is the single constant
`src/easy_personal/surfaces/api.py:57`:

```python
CSP = "default-src 'self'; frame-ancestors 'none'"
```

`style-src` is not set, so it falls back to `default-src 'self'`, and an inline `<style>` element is
refused. Measured in real headless Chrome: a report reaches the tab with its `<style>` element intact
(`document.querySelectorAll('style').length === 1`) but `document.styleSheets.length === 0`, and the
browser's security log says:

> `Applying inline style violates the following Content Security Policy directive 'default-src 'self''. ... The action has been blocked.`

So the report is readable but unstyled. This is not one unlucky fixture: `templates/report_template.html`
keeps its CSS in an inline `<style>` block (line 6) and the `html-report` skill requires "pure inline
CSS only", so **every report this project generates** opens plain, including its own code-review and
security-review pages.

**Why the fix is this small, and what bounds its risk** (checked 2026-11-08, state these in the review):

- The dashboard has **no** `dangerouslySetInnerHTML` and **no** `innerHTML` anywhere in `web/src/`, so
  React escapes every value it renders. No attacker-controlled markup reaches a dashboard page, which
  is what would make `'unsafe-inline'` styling dangerous there.
- The dashboard's own bars, swatches and dots (`style={{ width: ... }}` in `Breakdown.jsx`,
  `LiveView.jsx`, `UsageView.jsx`, `WorkspacesView.jsx`, `WorkspaceKit.jsx`) are **not** affected by
  this change either way: React sets those through the CSSOM, which CSP does not govern. Do not cite
  them as a reason for the change.
- `templates/report_template.html` and every file in `reports/` contain no `<img>`, no `data:image`,
  no `@font-face` and no `url(` — so `style-src` alone is enough. Do **not** widen `img-src` or
  `font-src` "while you are there".

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-08: "when I open the report, it does not contain the CSS related, should update that".
Owner choice, 2026-11-08 (PRD question 32): allow inline CSS dashboard-wide, rather than build a
separate sandboxed route.

**Restated intent**:

> Add `style-src` to the one `CSP` constant so that it permits inline styles, and nothing else
> changes:
>
> ```python
> CSP = "default-src 'self'; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'"
> ```
>
> An opened report then applies its own `<style>` block and looks the way its author designed it. A
> report's `<script>` must stay blocked exactly as it is today, because `script-src` still falls back
> to `default-src 'self'` and an inline script is not `'self'`.

**Out of scope**: a per-route CSP or any change to `Guard._allowed`, `Guard.send_csp` or the token
check; a one-time link route; `img-src`, `font-src`, `connect-src` or any other directive; the page
(`web/`); serving reports anywhere other than the existing `blob:` tab; sanitising report HTML.

**Requirement Refs**: FR-093, FR-090, NFR-008

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner message and owner choice, 2026-11-08)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered

---

## Dependencies & Reachability

**Depends on**: nothing — HUN-243 (the blob tab) is merged on `main` as `900ee1a`

**Entry point**: `CSP` in `src/easy_personal/surfaces/api.py`, applied by `Guard.send_csp` to every response

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Every response carries `content-security-policy: default-src 'self'; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'` — assert the exact string on a page response, an `/api/` response and a 403 response (the 403 path uses `send_csp` too) | FR-093, NFR-008 |
| 2 | The three existing CSP assertions (`tests/surfaces/test_api.py`, `test_api_live.py`, `test_api_kit_report_files.py`) are updated to the new string and pass; none is deleted to make it green | FR-093 |
| 3 | **The script defence is unchanged.** A real run, headless Chrome, same fixture shape as HUN-243 AC 8: a report `evil.html` whose inline `<script>` sets `document.title = "PWNED"` and calls `fetch("/api/workspaces")` is opened from the Kit view. The tab's title is NOT `PWNED`, the tab makes no `/api/*` request, and the browser's security log still reports the inline **script** blocked by CSP. Capture the log entry with a CDP session (`Log.enable`, then `Log.entryAdded` where `entry.source === "security"`) — a CSP violation does NOT reach `page.on('console')`. If the script runs, STOP and report `Status: blocked` | FR-093, NFR-002 |
| 4 | **The styling now applies.** Same real run: a report `score.html` with an inline `<style>` block opens with its CSS in effect — `document.styleSheets.length === 1` and the computed `body` background is the colour its CSS sets, not `rgba(0, 0, 0, 0)`. No `Applying inline style ... blocked` entry appears in the security log | FR-093 |
| 5 | The dashboard's own pages are unchanged: vitest and the full pytest suite pass, and a screenshot of the Usage view and the Kit view at 1280x720 looks as it does today | FR-093 |
| 6 | Existing tests pass; `uv run pytest -q` passes | — |

---

## UI / Design Acceptance Criteria

The dashboard's own look does not change — this task alters a response header, not a component.
AC 5 pins that with the full suites plus a before/after screenshot of two views. Fill the three UI
Evidence rows ☑ N/A with that reason, and put the styled-report screenshot from AC 4 in the
Demonstration instead.

---

## Evaluation & Acceptance

### Verification Command (exact, runnable)

```bash
uv run pytest tests/surfaces -q && uv run pytest -q && cd web && npx vitest run
```

Plus a real run: build the page (`npm --prefix web run build`), start the dashboard on a temp
`EASY_PERSONAL_HOME` with a workspace registered by `easy-personal workspace add <name> --repo <path>`
(a fixture that is created on disk but never registered gets 404 on every route and the negative
checks then "pass" for the wrong reason), whose repo has `reports/` holding `score.html` (inline
`<style>`) and `evil.html` (inline `<script>`). Drive real Chrome for AC 3 and AC 4; screenshots under
`tasks/evidence-<ID>/`.

## Approach

**Vital slice**: one directive added to one constant, the three existing CSP assertions updated, and a
real browser run that proves style now applies while script still does not. **Cut list**: no per-route
CSP, no new route, no sanitiser, no other directive, no page change.

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/surfaces/api.py` | the `CSP` constant (one line) |
| `tests/surfaces/test_api.py` | AC 1, AC 2 |
| `tests/surfaces/test_api_live.py` | AC 2 |
| `tests/surfaces/test_api_kit_report_files.py` | AC 2 |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `Guard._allowed`, `Guard.send_csp` | The token and Host/Origin checks are out of scope |
| `web/` | The page does not change |
| `src/easy_personal/core/kit.py` | The routes do not change |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table
- [ ] UI Evidence rows ☑ N/A with the reason above
- [ ] Supervisor notified: task ready for Stage 4 review

## Questions Log

* 2026-11-08 PRD question 32 settled by the owner: allow inline CSS dashboard-wide. The alternatives offered were a separate sandboxed route with a one-time link (stronger isolation, much bigger: Guard change + new route + ticket lifecycle) and stripping the report's CSS to restyle it in the dashboard's own look (loses the author's design, adds a sanitiser). The owner chose the small reversible change.
* 2026-11-08 Stage 4 `code-review`: 0 P0, 0 P1, 1 P2, 0 P3. The P2 was a test gap, now closed by `test_csp_keeps_script_and_egress_on_default_src`. `lane_check`: `lane: C1, 10 files, 9 lines, EXCEEDS (files 10 > 6)` — warn-only, and all 8 unpredicted files are the before/after screenshots this guide's own Evaluation section asks for. `ruff format --check` fails on both touched files, but it fails identically on `main` and everything it wants to reformat is unrelated code, so it is pre-existing and was deliberately not "fixed" (Surgical Changes). `ruff check` passes.
* 2026-11-08 Stage 4 `security-review`: no findings at or above the reporting bar. The built-in skill reads the checked-out branch and this task's code lives in a sibling worktree, so the skill's own run captured `main`'s diff instead; it was re-run as a scoped sub-review against the real `main...HEAD` diff. It verified empirically, using a second local origin as the attacker's server so the ground truth was that server's request log rather than the browser's own violation report: five CSS egress vectors (cross-origin `@import`, `background-image: url()`, a `style="..."` attribute, `@font-face src: url()`, and the attribute-selector leak) produced **zero** requests to the attacker. Key nuance it surfaced, which drove the extra assertions: the CSS exfiltration **primitive** does now work — an attribute-selector rule matched a secret value and attempted its fetch — and only the `default-src 'self'` fallback for `img-src`/`font-src`/`connect-src` closes the egress. That protection was implicit, so the new test pins it. Also found: `style-src-attr` falls back too, so inline `style="..."` attributes are permitted as well as `<style>` elements (the docstring now says both). Recorded, not a finding: `form-action` is absent and does not fall back to `default-src`, so a report could submit a form cross-origin on a user click — unchanged by this task, and the token stays unreachable.
* 2026-11-08 Supervisor check of the guard, independent of the implementer's probe: `api.CSP` was patched in memory to four regressions and the new test was called directly. Baseline passes; `img-src *`, `script-src 'unsafe-inline'`, `'unsafe-inline'` moved into `default-src`, and a widened `connect-src` each FAIL. The guard guards.
