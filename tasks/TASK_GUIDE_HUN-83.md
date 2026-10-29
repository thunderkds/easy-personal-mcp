**Date**: 2026-10-29
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: *Workspace*, *Agent account*, *API-equivalent cost*)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read `agents/frontend.md` with the Read tool and no `limit` (learning: partial role-guide reads recur)
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Look at the reference image `/home/hungnguyenhuu/workspace/image/dashboard.png` (owner-supplied, look only — see Approach)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-29: "following this image, update the dashboard" — reference image of a dark admin template (gradient navy/purple background, left sidebar, glowing rounded cards, KPI cards with sparkline area charts and % badges, goal progress bars, small gauge/ring/bar widgets).

Owner decisions 2026-10-29 (AskUserQuestion):

* Scope = **Reskin + Usage widgets**: dark theme, sidebar nav and card styling on all 5 views; on Usage the top becomes image-style widget cards; tables and existing panels stay below. **No new API.**
* Theme = **Dark only** (one token set).
* Top bar = **Title + filters only** — no search box, flag, bell, cart or avatar.

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Replace the light tab page with a dark shell: a left sidebar listing the 5 views (Usage, Warnings & plan, Kit, Workspaces, Agents) and a top bar with the view title. Restyle every view's sections as dark rounded cards from new dark tokens. On Usage, add a widget grid above the existing Totals/Breakdown/Outliers/Timeline: three KPI cards (Total tokens, Messages, API-equivalent cost) with a daily sparkline and a % change badge vs the previous equal period, one cache-hit-ratio progress card, and one cache-hit-ratio ring gauge. All numbers come from the existing `/api/usage/*` responses.

**Out of scope** (what this task explicitly does NOT do):

* Backend / API changes (`src/easy_personal/**`, except the rebuilt static bundle)
* A light-mode toggle
* Decorative top-bar icons (search, flag, notifications, cart, avatar)
* New widget cards on Warnings, Kit, Workspaces, Agents (reskin only there)
* The template's name, logo, or any sales/e-commerce wording
* Any chart library: SVG by hand, as `Breakdown.jsx` does today

**Requirement Refs**: owner request 2026-10-29 (no PRD FR yet; dashboard FRs for the 5 views stay satisfied unchanged)

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner's 3 answers 2026-10-29)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] Requirement Refs: owner request; no PRD FR changes

---

## Dependencies & Reachability

**Depends on**: none (HUN-70 filters and HUN-73 release are merged)

**Entry point**: `easy-personal dashboard` → every view; Usage view for the widgets

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | The shell has a `<nav>` sidebar with the 5 views as buttons (same ids/labels as today's tabs); clicking one shows that view; the active one has `aria-current="page"`; the top bar shows the active view's title | sidebar, top bar |
| 2 | Below 64rem the sidebar collapses behind a menu button (`aria-expanded`); no horizontal page scroll at 375px | sidebar, responsiveness |
| 3 | `tokens.css` holds one dark token set (gradient background, card surface, text, muted, border, accent + 4 chart accents: green, amber, cyan, pink); every color in `app.css` and in JSX `fill`/`stroke` comes from a token; text on card surface meets WCAG AA (4.5:1) | dark only, design tokens |
| 4 | Usage shows 3 KPI cards — Total tokens, Messages, API-equivalent cost — each with the period value, a sparkline of the `by=day` groups (area + line), and a badge with % change vs the previous equal period (up/down arrow + color; "no previous data" when the previous value is 0 or the period is All time) | Usage widgets |
| 5 | Usage shows a cache-hit-ratio card with a progress bar and a ring gauge with the same ratio; `null` ratio shows "no data", never 0% | Usage widgets |
| 6 | Cost KPI follows the existing rule: abstained models are named, never shown as $0 (`costText`) | API-equivalent cost |
| 7 | Every sparkline/gauge SVG has `role="img"` and an `aria-label` with its numbers | accessibility |
| 8 | Existing Totals, Breakdown, Outliers, Timeline, filters, period buttons and the 'not included' notice still render with their current text; all existing Vitest tests pass (update only selectors that the tab→sidebar change breaks) | reskin only |
| 9 | No new npm dependency | no chart library |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Click "Kit" in the sidebar | Kit view renders; Kit button `aria-current="page"`; top bar title "Kit" | Vitest (`App.test.jsx`) |
| 2 | Last 7 days; fixture current total tokens 1,500, previous 1,000 | Total tokens badge "↑ 50.0%" | Vitest (`UsageView.test.jsx`) |
| 3 | Previous period has 0 tokens | badge "no previous data", no Infinity/NaN | Vitest |
| 4 | Period All time | no previous-period request sent; badges "no previous data" | Vitest (request log) |
| 5 | 7 day groups | sparkline `aria-label` names 7 days and the peak | Vitest |
| 6 | `cache_hit_ratio: null` | progress + gauge show "no data" | Vitest |
| 7 | Cost with an abstained model | KPI text names the model | Vitest |
| 8 | Previous-period request: `since = now − 2×days`, `until = now − days`, same workspace/account | URL params | Vitest (request log) |

### Verification Command (exact, runnable)

```bash
npm --prefix web test && npm --prefix web run build && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-83.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-83.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

> UI evidence via headless Chrome on fixture data (learning: UI evidence without easy-ui) — screenshots at 375/768/1280 of every view.

### 1. Visual Regression

| Screen / Component | Verification method | Expected result |
| -- | -- | -- |
| Usage view (widgets + tables) | headless Chrome screenshot + LLM vision review against the reference image and AC 1–7 | dark gradient shell, sidebar, 3 sparkline KPI cards, progress + gauge card; reads like the reference |
| Warnings, Kit, Workspaces, Agents | headless Chrome screenshot | dark cards, no light leftovers, no unreadable text |

### 2. Design-System Compliance

| Criterion | Verification method | Expected result |
| -- | -- | -- |
| Colors match design tokens | grep `app.css` + `web/src/*.jsx` for hex/rgb literals outside `tokens.css` | none |
| Typography matches spec | computed style | one font family, sizes from tokens |
| Contrast | computed colors, contrast ratio | text ≥ 4.5:1 on card surface |

### 3. Layout / Responsiveness

| Viewport | Verification method | Expected result |
| -- | -- | -- |
| Mobile (375px) | screenshot | sidebar collapsed behind menu button; cards one column; no horizontal scroll |
| Tablet (768px) | screenshot | cards 2 columns; no overlap |
| Desktop (1280px) | screenshot | sidebar visible; KPI cards 3 across |

---

## Approach

**Pattern reference**: `web/src/Breakdown.jsx` `DayChart` (hand SVG, `viewBox`, `role="img"`); `web/src/UsageView.jsx` `load()` / `query()`

**Vital slice**: dark tokens + sidebar shell + 3 sparkline KPI cards with % badges + cache-hit progress/gauge card.
**Cut list**: gradient-stroke animations; kebab (⋮) menus on cards; the bar/line mini-widgets of the image's bottom row; a Settings button; widget cards on the other 4 views; light mode.

Task design (Supervisor, 2026-10-29):

* The reference image is a third-party commercial template. Copy the **look** (colors, card shape, glow, layout), never its name, logo, icons or text.
* `App.jsx` keeps in-page state (no routing — the URL fragment holds the dashboard token). The tab list becomes a sidebar `<nav aria-label="Views">` of buttons.
* New `web/src/Widgets.jsx`: `Sparkline({ values, label, tone })`, `Ring({ ratio, label })`, `Progress({ ratio })`, `Trend({ current, previous })`. Pure SVG/CSS; tone maps to a token class.
* `UsageView.load()` adds one previous-period `summary` fetch (and `breakdown?by=day` is already loaded for sparklines) only when the period has `days`; reuse `query()` with an `until` param.
* Sparkline sources (all from `breakdown?by=day` groups, already loaded): Total tokens = `totalTokens(g)`; Messages = `g.messages`; Cost = `g.api_equivalent_cost.usd` (null → 0 in the line; the `aria-label` names the abstained days). Day groups have no `sessions` field, so the KPI is Messages, not Sessions; Sessions stays in Totals.
* Rebuild the static bundle (`npm --prefix web run build`) before `uv run pytest` — the static build is gitignored but web tests skip unbuilt.

---

## Edge Case Checklist

- [ ] Period with 0 day groups → KPI cards show values, no sparkline, "No usage recorded in this period."
- [ ] 1 day group → sparkline draws a flat line, no NaN in `d`
- [ ] Previous-period fetch fails → badges "no previous data"; the view still renders (not an error state)
- [ ] `cache_hit_ratio` null → "no data"
- [ ] Workspace with no repos → existing hint still shows
- [ ] Planted secret/text fixtures never appear in the page (no new API route, so no leak-suite registration needed)

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/tokens.css` | dark token set |
| `web/src/app.css` | shell, sidebar, cards, widgets |
| `web/src/App.jsx` + `App.test.jsx` | sidebar + top bar |
| `web/src/Widgets.jsx` + `Widgets.test.jsx` (new) | sparkline, ring, progress, trend |
| `web/src/UsageView.jsx` + `UsageView.test.jsx` | widget grid, previous-period fetch |
| `web/src/test/fixtures.js` | previous-period fixtures |
| `web/src/Breakdown.jsx` | chart colors via token classes only if needed |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/**` (except rebuilt static output) | no API change |
| real `~/.claude*/`, `~/.easy-personal/` | fixtures only |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit framework files |
| `web/package.json` dependencies | no new dependency (AC 9) |

---

## Test Plan

Vitest for AC 1–9 (success criteria 1–8); headless Chrome screenshots at 375/768/1280 for all 5 views on fixture data.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] `Skill({ skill: "verify" })` run on fixtures
- [ ] Supervisor notified: task ready for Stage 4 review