**Date**: 2026-11-03
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt. Read `memory/MEMORY.md` in full only if your work reaches a file the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Apply the C1 process from your role guide
6. Read `PRD.md` FR-065, `web/src/tokens.css` (`--shell-max`, `--sidebar-width`, `--header-height`) and the toast/loading block at the end of `web/src/app.css`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-03, after HUN-200 merged: "the toast and loading indicator got the wrong place, the loading indicator should be in the middle and the toast should stick with the scope of page, not run by the screen wide"

**Restated intent**:

> The shell (`.shell`) is centred with `max-width: var(--shell-max)` (90rem). Today `.toast-host` is fixed at `right: var(--space-4)` of the **viewport**, so on a screen wider than 90rem the toasts sit outside the page box, at the far screen edge. Move it so its right edge is `--space-4` inside the shell's right edge at every width (same as today when the screen is ≤ 90rem). Bottom stays `--space-4`. Phone (< 40rem) rule unchanged: full width inside a 16 px gutter.
>
> Today `.loading-pill` is fixed at the top-right under the header. Move it to the **middle of the content area**: horizontally centred on the main column (the shell minus the sidebar when the sidebar is shown, i.e. ≥ 64rem; the whole shell below 64rem), vertically at the middle of the window. It stays fixed (the page does not move), keeps `role=status`, its label and spinner. On a phone it must not cause horizontal scroll.
>
> CSS only if possible (e.g. a `--shell-w: min(100vw, var(--shell-max))` helper and `calc()`); no JS layout measurement, no new dependency.

**Out of scope**: toast timing, toast count, form changes, the three open HUN-200 suggestions, anything outside `web/src/app.css` (and tests).

**Requirement Refs**: FR-065

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, 2026-11-03; "middle" read as the content area's centre — stated to the owner)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria

---

## Dependencies & Reachability

**Depends on**: HUN-200 (merged)

**Entry point**: `.toast-host` / `.loading-pill` in `web/src/app.css`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | At 1920×1080 the toast host's right edge is within the shell (`toast.right <= shell.right - 16px ± 1`) and its bottom is 16 px from the window bottom | FR-065 |
| 2 | At 1280×720 the toast host is unchanged (right 16 px from the window edge, which is the shell edge) | FR-065 |
| 3 | At 375×812 toasts span the width with a 16 px gutter; no horizontal scroll | FR-065 |
| 4 | At 1920×1080 and 1280×720 the loading pill's horizontal centre equals the main column's centre (shell minus sidebar) ± 2 px and its vertical centre equals the window middle ± 2 px; at 375×812 it is centred on the screen and causes no horizontal scroll | FR-065 |
| 5 | Existing vitest suite still passes; a test pins the new placement rules in CSS (e.g. reads `app.css` and checks the `.toast-host` / `.loading-pill` rules), since jsdom does no layout | FR-065 |

---

## Evaluation & Acceptance

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run && cd .. && npm --prefix web run build && uv run pytest tests/test_web_toolchain.py -q
```

Plus headless-Chrome measurements (getBoundingClientRect of `.shell`, `.toast-host`, `.loading-pill`, main column) at 1920×1080, 1280×720, 375×812 on a copy of the owner store — never the live store.

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Usage loading + a toast, 1920×1080 | headless screenshot BEFORE/AFTER | toast inside the page box; pill in the content middle |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Tokens | reuse `tokens.css` variables | no hard-coded widths except via existing tokens |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1920×1080, 1280×720, 375×812 | measurements + `scrollWidth` | AC 1–4; no horizontal scroll |

---

## Approach

**Vital slice**: two CSS rules (`.toast-host`, `.loading-pill`) + a CSS-rule test.
**Cut list**: no JS positioning, no portal changes, no new tokens unless one helper variable is clearly simpler.

## Edge Case Checklist

- [ ] Screen exactly 90rem wide → same as ≤ 90rem
- [ ] Vertical scrollbar width (100vw includes it) → off by the scrollbar at most; acceptable within the shell
- [ ] Sidebar hidden below 64rem → centre on the whole shell
- [ ] Pill and an open toast at the same time → do not overlap at 375 px

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/app.css` | `.toast-host`, `.loading-pill` placement |
| `web/src/*.test.jsx` | CSS placement test |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/` | No backend change |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table
- [ ] UI Evidence rows filled
- [ ] Supervisor notified: task ready for Stage 4 review