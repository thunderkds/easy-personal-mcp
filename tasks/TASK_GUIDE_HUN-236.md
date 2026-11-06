**Date**: 2026-11-06
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Live tail, session row, session panel; Critical Constraints: metadata only)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-072, FR-084, FR-085, NFR-015; then `web/src/LiveView.jsx`, `web/src/LiveView.test.jsx`, the `.live-*` rules in `web/src/app.css`, `web/src/tokens.css`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-06 (after HUN-234 merged): "look good now, but the line is too long with the logs, and the UX for click to show and hide is not good, let investigate and update \[auto\]".

Supervisor investigation (read-only, 2026-11-06), from `web/src/LiveView.jsx`, `web/src/app.css` and the HUN-234 screenshots:

* Long lines: the side panel is at most `22rem` wide (`.live-body[data-panel]` grid `minmax(0, 1fr) minmax(16rem, 22rem)`); each feed `li` is `flex-wrap`, so time + who + tool + label wraps to 2–3 lines; an MCP row prints the raw `mcp__easy-verifier__score` **and** the label `easy-verifier · score`; time is `toLocaleTimeString()` ("7:11:36 PM").
* Open/close: opening a session turns the list into a narrower column, so every row's bars jump to the stacked `@container (max-width: 28rem)` layout (layout shift); under 768 px the panel opens below **all** rows, away from the tapped row, with no scroll; no open/closed affordance; toggling by clicking the same row is hidden; Escape does nothing; the panel repeats the totals the row bars already show.

**Restated intent**:

> Replace the side panel with an inline expand (FR-085). Activating a session row (click, Enter, Space) expands that session's feed directly under the row inside the same `.live-session` card, at full list width; activating it again, a small close control in the expanded area, or Escape collapses it, and focus returns to / stays on the row button. One open at a time (opening another closes the first). The list never changes width when a session opens (remove the `data-panel` two-column grid). The row shows an arrow (▸ closed / ▾ open, `aria-hidden`) and keeps `aria-expanded` + `aria-controls` pointing at the expanded region's id. The expanded region shows only the feed (the row bars already carry the totals) — keep the "no longer active" case: if the open session leaves the list, nothing is drawn and `openId` resets. Each feed row is one line, laid out as a grid: time `HH:MM:SS` 24 h (local time, zero-padded), who (FR-084, unchanged rules), then the call: an MCP call (label is `{server, tool}`) shows only `server · tool`; any other call shows the tool name and, if present, its label; error tag kept. Every cell is `white-space: nowrap` with ellipsis; the row's `title` holds the full text (time, who, raw tool name, label). The feed keeps one `aria-live` region and FEED_MAX rows, scrolls inside a max height.

**Out of scope**:

* Any server change; other views; stage chip (unchanged, FR-084)
* Remembering the open session across reloads; filters; history

**Requirement Refs**: FR-085 (FR-072 amended; FR-084, NFR-015 kept)

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (auto (Supervisor), 2026-11-06 — owner complaint quoted above; inline-expand choice flagged in the review file)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None (HUN-233/HUN-234 merged)

**Entry point**: `LiveView` default export in `web/src/LiveView.jsx`, reached from the `Live` tab in `web/src/App.jsx`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Click, Enter and Space on a row expand its feed inside that row's `.live-session` (the feed's region is a descendant of the same `li`); `aria-expanded` true and `aria-controls` = the region id; activating again collapses it; opening another row collapses the first; at most one `.live-feed` in the DOM | FR-085, NFR-015 |
| 2 | Escape (with focus on the row or inside the expanded area) and the close control collapse it; focus ends on that row's button | FR-085 |
| 3 | No side panel: no `.live-panel`, no `data-panel` grid; opening a session does not change the other rows' markup or classes (render spy / snapshot of a sibling row unchanged); existing memo test still passes | FR-085, NFR-015 |
| 4 | The row shows ▸ when closed and ▾ when open (`aria-hidden="true"`) | FR-085 |
| 5 | Feed row text: an MCP event (`label: {server:"easy-verifier", tool:"score"}`) shows `easy-verifier · score` and not `mcp__easy-verifier__score` in visible text; the raw name is in the row `title`; a Bash call shows `Bash` + `uv`; time reads `HH:MM:SS` 24 h | FR-085, FR-084 |
| 6 | Feed rows are one line: each cell `white-space: nowrap` + `text-overflow: ellipsis` (CSS rule test, as HUN-234 did); a long label/who never widens the page (375 px: no horizontal scroll) | FR-085 |
| 7 | The open session leaving the list draws no expanded area and resets the open id (adapt the old "no longer active" test) | FR-085 |
| 8 | Existing Live tests adapted, not deleted; full `npx vitest run` and `npm run build` pass | FR-072, FR-084 |
| 9 | Real run: `easy-personal dashboard` on temp homes, calls appended **after** opening the row (the feed shows only events received while the page is open); screenshots at 1280×720 and 375×812, closed and open | FR-085 |

---

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Live, 2+ sessions, one open, 1280×720 | headless screenshot BEFORE/AFTER | feed under the open row, full list width, one line per call |
| Live, one open, 375×812 | headless screenshot | feed directly under the tapped row; no horizontal scroll |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Tokens | `web/src/tokens.css` variables only | no hard-coded colours or sizes |
| Accessibility | vitest + keyboard | row button `aria-expanded`/`aria-controls`; Escape closes; one `aria-live` feed; arrow `aria-hidden` |

### 3. Responsiveness

| Width | Expected |
| -- | -- |
| ≥ 768 px | rows full width; opened feed under its row; rows never re-layout on open |
| 375 px | same; long cells end with an ellipsis; no page-wide horizontal scroll |

---

## Evaluation & Acceptance

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`.

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run src/LiveView.test.jsx && npx vitest run && npm run build
```

## Approach

**Vital slice**: move the feed into `SessionRow` (rendered only when `open`), delete `SessionPanel` and the `data-panel` grid; Escape handler on the row `li`; arrow span; feed `li` as a CSS grid (`time who call`) with `minmax(0, …)` columns and nowrap/ellipsis cells; `HH:MM:SS` formatter; MCP rows show the label only. Keep `memo` working: pass `open` and the session's own `feed` so only the open row redraws on feed events.
**Cut list**: no animation library; no new component file unless `LiveView.jsx` passes \~450 lines; no stored open state.

---

## Edge Case Checklist

- [ ] Opening a row whose session then gets new events: only that row redraws (memo)
- [ ] Server restart snapshot clears the feed, as today
- [ ] Escape when nothing is open does nothing
- [ ] `agent` null + `is_sidechain` → "sub-agent" (FR-084 rule kept)
- [ ] Label of unexpected shape → no label (kept)

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/LiveView.jsx` | inline expand, arrow, Escape, one-line feed rows, time format |
| `web/src/LiveView.test.jsx` | AC 1–8 (adapt panel tests) |
| `web/src/app.css` | remove panel grid; `.live-feed` row grid + ellipsis; arrow |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/` | No server change |
| Other views in `web/src/` | Out of scope |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |
