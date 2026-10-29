**Date**: 2026-10-29
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Verify on fixtures only: temp `EASY_PERSONAL_HOME` + `EASY_PERSONAL_CLAUDE_PROJECTS` + `EASY_PERSONAL_AGENT_ROOT`; never open the real `~/.easy-personal/` or `~/.claude*/`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-29: "scope the dashboar in the container, currently it wide to the screensize".

**Restated intent**:

> The whole dashboard shell (sidebar, header + main content, right rail) sits in one horizontally centered container of at most 1440 px (`--shell-max: 90rem`, the width of the Graphite Mint artboards). On screens wider than that, the page background (`--color-bg`) fills both sides and the container has a 1 px `--color-divider` border on its left and right. At 1440 px and below nothing changes from HUN-126. The sticky sidebar, sticky header and sticky rail stay inside the container; the small-screen sidebar overlay (< 64rem) is unchanged.

**Out of scope**: changing column widths, the design, or any view's content; a user setting for the width.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's request — Supervisor, 2026-10-29 (assumption: 1440 px, the artboard width)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: HUN-126 (merged to main). **Entry point**: `web/src/App.jsx` `.shell`.

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | `tokens.css` defines `--shell-max: 90rem`; `.shell` has `max-width: var(--shell-max)` and `margin-inline: auto` | container |
| 2 | At a 1920 px viewport the shell's bounding box is 1440 px wide and centered (left offset 240 px ± 1); side borders visible | centered |
| 3 | At 1440, 1024 and 375 px the layout matches HUN-126 (no horizontal scroll; rail at ≥ 80rem only; sidebar overlay < 64rem still opens over the page from the left edge of the viewport) | no regression |
| 4 | Sticky sidebar, header and rail stay within the container while scrolling at 1920 px | sticky |
| 5 | No color literal outside `tokens.css`; all Vitest tests pass; build ok; `uv run pytest -q` green | standards |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | viewport 1920×1080 | `.shell` rect width 1440, x 240 | easy-ui / Playwright |
| 2 | viewport 1920, scroll 1500 px | sidebar and rail rects keep the same x as before scrolling | easy-ui / Playwright |
| 3 | viewport 375 | `scrollWidth <= clientWidth`; menu opens the sidebar | easy-ui / Playwright |

### Verification Command (exact, runnable)

```bash
npm --prefix web test && npm --prefix web run build && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-128.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-128.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Usage at 1920 and 1440 px | screenshot | centered 1440 container at 1920; unchanged at 1440 |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors and sizes from tokens | grep | no literal outside `tokens.css` |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1920 / 1440 / 1024 / 375 px | screenshot + rect checks | as Success Criteria |

---

## Approach

**Pattern reference**: `web/src/app.css` `.shell`, `.sidebar`, `.topbar`, `.rail` (HUN-126).
**Vital slice**: one token + `.shell` max-width/margin/borders; check sticky and the < 64rem overlay still work.
**Cut list**: none.

---

## Edge Case Checklist

- [ ] the fixed small-screen sidebar overlay is not clipped by the container
- [ ] borders do not add horizontal scroll at exactly 1440 px (use `box-sizing` or borders only above 90rem)

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/tokens.css` | `--shell-max` |
| `web/src/app.css` | `.shell` container rules |
| `web/src/App.test.jsx` (optional) | none expected; jsdom has no layout |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/**` (except rebuilt static) | no API change |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |
| `web/package.json` dependencies | no new dependency |

---

## Test Plan

Hard-Stop Gate 5 needs a new automated test: add a Vitest (e.g. `web/src/shell.test.js`) that reads `tokens.css` and `app.css` as text and asserts AC 1 (`--shell-max: 90rem`; `.shell` rule holds `max-width: var(--shell-max)` and `margin-inline: auto`). Layout is CSS only (jsdom computes no layout), so the Success Criteria are checked in a real browser: one batched host-Playwright or easy-ui script at 1920/1440/1024/375 px on a fixture dashboard, writing rects and screenshots to `reports/<this task ID>/`. Vitest + pytest must stay green.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] Supervisor notified: task ready for Stage 4 review