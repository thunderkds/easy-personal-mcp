# TASK_GUIDE — T068: Top bar status chips (live, waiting, last ingest, 5h limit) from the Graphite Mint prototype
**Date**: 2026-11-14
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P2
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/frontend.md`
5. Read `rust/docs/UI_REDESIGN_PLAN.md` (row G1) and `docs/design/graphite-mint/README.md`
6. Read the design source: the `<header>` of `docs/design/graphite-mint/Main.dc.html` (four chips) and of `FirstRun.dc.html` (no-timer and no-data variants)
7. Read `web/src/App.jsx` (`topbar`), `web/src/LiveView.jsx` (how `/api/live` is opened and reduced), `web/src/Rail.jsx` (plan-limit windows), `web/src/tokens.css`, `web/src/api.js`
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder; fixture stores only

---

## Requirement

**Original request** (owner, 2026-11-14): "Refer to this prototype … to compose the plan to update the UI for the window application"; plan accepted ("yes, just following the guide").

**Restated intent**:
> The top bar of every view shows, right-aligned after the scope text, up to four status chips as in the prototype:
> 1. **Live**: `N active` with a dot (accent + glow when N > 0, `--color-dot` grey when 0). N = sessions the live snapshot counts as active.
> 2. **Waiting**: `N waiting` in amber with the warning triangle; shown only when N > 0. N = live sessions with any attention state (`waiting`, `failing`, `needs_attention`).
> 3. **Last ingest**: `ingest 4m ago` (muted). Amber with the triangle and `· no timer` when the first-run step `ingest_timer` is not done. Time = newest `last_ingest_utc` from `GET /api/store/health` (cheap form, not `full`).
> 4. **5h limit**: `5h 42%` + a 36 px mini bar; the highest `five_hour.used_percentage` over confirmed accounts that have status-line limits. Amber at ≥ 70 %, red at ≥ 90 % (same thresholds as the rail). `5h no data` (faint) when no account has the value.
>
> Live and Waiting chips are links to the Live view. Chips wrap on narrow widths. Each chip has a `title` (as the prototype).

**Out of scope**: new API routes; tray; any other top bar change; the First run nav badge (T069).

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner (plan accepted 2026-11-14)
- [x] Every Acceptance Criterion traces to the Requirement
- [ ] Requirement Refs: FR for the dashboard shell in `PRD.md` (find and cite; add none)

---

## Dependencies & Reachability
**Depends on**: none
**Entry point**: `web/src/App.jsx` `<header className="topbar">`

## Dependency rule
No new npm package.

## Design constraint: one live stream
The page must keep **one** `EventSource` on `/api/live`. Today `LiveView` opens it. Lift the subscription (the existing reducer) to `App` (or a small hook) and pass the state to both `LiveView` and the chips. `LiveView` behaviour and its tests must not change. If lifting is impossible without changing `LiveView` behaviour, stop and report `Status: blocked` with a `Question:` line.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Vitest: chips render from fixture data — `2 active` + `1 waiting` + `ingest 4m ago` + `5h 42%`; 0 sessions → grey `0 active` and no waiting chip | chips 1-2 |
| 2 | Vitest: timer not done → amber ingest chip with `no timer`; no limits → `5h no data`; 72 % → amber class, 91 % → red class | chips 3-4 |
| 3 | Vitest: only one `EventSource` is created while the Live view and the top bar are both shown, and after switching views | one stream |
| 4 | Existing web tests pass (`npm test`), `npm run build` passes (incl. `check_static.py`) | all |
| 5 | UI evidence rows below filled | all |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd web && npm test 2>&1 | tail -5 && npm run build 2>&1 | tail -3
```

### Evidence (filled by reviewer at Stage 4/5)
| Item | Result | Evidence |
|---|---|---|
| New test(s) cover acceptance criteria | ☐ | |
| Web suite and build pass | ☐ | |
| code-review | ☐ | |
| UI: visual regression | ☐ | |
| UI: design-system compliance | ☐ | |
| UI: responsiveness | ☐ | |

## Demonstration
BEFORE: screenshot of the current top bar (title + scope only). AFTER: the same view with the four chips, on fixture data.

## UI / Design Acceptance Criteria

### 1. Visual Regression
| Screen / Component | Verification method | Expected result |
|---|---|---|
| Top bar on Usage at 1440 px | easy-ui MCP screenshot (`http://localhost:8765/mcp`, as in `tasks/TASK_GUIDE_HUN-126.md`) or headless `google-chrome --screenshot`, compared by LLM vision with `Main.dc.html`'s header | same chip order, colours, sizes (28 px tall, 6 px radius, mono 12 px) |

### 2. Design-System Compliance
| Criterion | Verification method | Expected result |
|---|---|---|
| Colours | grep the new CSS for raw hex | `var(--color-*)` / `var(--glow-*)` only |
| Typography | computed style | `--font-mono`, 12 px |
| Spacing | computed style | gap 6 px, padding 0 10px |

### 3. Layout / Responsiveness
| Viewport | Verification method | Expected result |
|---|---|---|
| 375 px | screenshot | chips wrap under the title; no horizontal scroll |
| 1024 px | screenshot | one row |
| 1440 px | screenshot | one row, right-aligned |

---

## Approach
Test-first (`tdd`). Write the chip component tests on fixtures, then a `StatusChips.jsx`; then lift the live subscription and prove one stream; then wire `store/health`, `first-run` and accounts data (re-use what `App` already loads for the rail/options where it can; one fetch each, refreshed with the existing refresh signal).

## Edge Case Checklist
- `/api/store/health` fails or has no `last_ingest_utc` → `ingest no data` (faint), never "0m ago"
- Limits file stale (reset time passed) → treated as no data (rail rule)
- Several accounts → highest percentage; title lists which account
- Live stream reconnecting → keep the last counts, do not flash to 0
- Reduced motion / glow off → no glow (`tokens.css` media rule)

## Files to Change (Predicted)
`web/src/App.jsx`, `web/src/LiveView.jsx` (subscription moved out only), new `web/src/StatusChips.jsx` + test, `web/src/app.css`, maybe `web/src/api.js`.

## Files Must NOT Touch
`rust/**`, `src/easy_personal/**`, `web/src/tokens.css` values.

## Test Plan
Vitest for AC 1-3; build; screenshots for the UI rows.

## Completion Checklist
- [ ] AC 1-5 with pasted output
- [ ] Commits on branch `T068` only; final report with `Status:`
