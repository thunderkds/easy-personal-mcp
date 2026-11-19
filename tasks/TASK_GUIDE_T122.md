# TASK_GUIDE — T122: Live filter bar — Project, Kind and Clear filters
**Date**: 2026-11-19
**Complexity Level**: C1
**Risk Level**: Medium (page only; 2–4 files, the filter bar shared by Live and Past)
**Priority**: P1
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `agents/frontend.md`, `skills/ux-evaluate/SKILL.md`
2. `web/src/LiveFilters.jsx`, `web/src/LiveFilters.test.jsx`, `web/src/PastSessions.jsx` (its own Project/Kind controls), `web/src/appsession.css` (`.live-filters` rules and the `/* T121 drops */` block), `web/src/app.css` (`.periods`, `.board-filter`, line ~173 select rule, the ≤40rem block)
- Fixture/mocked data only; never the real `~/.claude*` or `~/.easy-personal`. Port 8765 is taken. Reusable mock + shoot scripts: Supervisor scratchpad `ux-t121/` (`mock.py <static> <port>`, `shoot_ui.py <out> <port> live-list,past-list`).

## Requirement
Owner 2026-11-19, after installing the T121 package: "from the live, the project, kind and clear filter is not good".
Supervisor's reading of the current screen (screenshots `ux-t121/shots-now/live-list-{375,1280}.png`):
1. Project and Kind are native `<select>`s with the OS arrow, 36 px high next to ~40 px State buttons, different widths, labels stacked above — they sit lower and look unrelated to State.
2. Kind has only three values (terminal / app / headless), hidden behind a dropdown with its counts.
3. *Clear filters* mounts only when a filter is set (`isFiltered && …`), as a link at the end of a wrapped row — it pops in and out and shifts the row (the T121 "drop" class of bug).
4. At 375 Project and Kind each take a full-width row with a label above (~170 px before the first session).

## Dependencies & Reachability
**Depends on**: T119, T121 (merged)
**Entry point**: Live view filter bar (`FilterBar` in `web/src/LiveFilters.jsx`), and the Past filters that share its styles

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | **Kind** is a segmented button group like State: All / Terminal / App / Headless, each with its count, `aria-pressed` on the chosen one, same look and height as State | owner |
| 2 | **Project** stays a select (the list can be long) but is styled to match: same height as the segmented buttons, same border/radius/background tokens, a token-coloured chevron (`appearance: none` + background chevron or an inline SVG), label on the same line (`Project` before the box, or a visually-hidden label with the "All projects" option saying it) | owner |
| 3 | **Clear filters** is always rendered as a button styled like the segmented controls; `disabled` (muted) when nothing is filtered; never mounts/unmounts; clicking it resets state, project and kind; a test proves the same DOM node with and without a filter | owner, T121 rule |
| 4 | Layout: at 1280 one tidy row (State · Kind · Project · Clear), wrapping cleanly on narrower desktop widths, all controls the same height and baseline-aligned; at 375: State grid, then Kind, then a row Project (flex 1) + Clear; every control ≥ 44 px at 375; no horizontal page scroll | owner, ux |
| 5 | Past keeps working with the restyled controls (its own Project/Kind; the `app` "kept from the Live list" option stays as T119 built it); existing LiveFilters / PastSessions / LiveDrop tests stay green, updated only where the Kind control changed from a select to buttons | regression |
| 6 | ux-evaluate (screenshots read) at 375 and 1280 for Live list with no filter, with a filter set, and Past list; tokens only; `cd web && npx vitest run && npm run build`; `python3 scripts/check_static.py` ok | all |

## Approach
| Option | Invasiveness | Regression risk | Pick |
|---|---|---|---|
| A. Kind → segmented buttons; Project → restyled select; Clear always rendered + disabled; CSS in the `.live-filters` rules | Low (LiveFilters.jsx + appsession.css, tests) | Past shares `.live-filters` styles — re-shoot Past | **Yes** |
| B. Custom dropdown component for Project and Kind | Medium (new component, keyboard handling) | a11y bugs; not needed | No |
| C. CSS-only restyle, keep Kind a select and Clear conditional | Lowest | leaves points 2 and 3 | No |

## Files to change (predicted)
| File | Change |
|------|--------|
| `web/src/LiveFilters.jsx` | Kind buttons, Clear always rendered |
| `web/src/appsession.css` | filter bar layout and select styling (edit the `.live-filters` rules in place) |
| `web/src/LiveFilters.test.jsx`, maybe `PastSessions.test.jsx` / `LiveDrop.test.jsx` | updated selectors, new tests |
| `web/src/PastSessions.jsx` | only if it must follow the new Kind control to stay consistent |

## Files must NOT touch
| File | Reason |
|------|--------|
| `rust/**`, `src/easy_personal/**` (except build output) | server |
| `web/src/tokens.css` | sizes were just set by T121 |

## Test plan
vitest (Kind buttons + counts, Clear same node + disabled + reset, localStorage unchanged shape `{state, project, kind}`); screenshots at 375/1280.

## Verification Command
```bash
cd web && npx vitest run 2>&1 | tail -5 && npm run build && cd .. && python3 scripts/check_static.py
```

## Completion checklist
- [ ] Implementation done
- [ ] Tests pass
- [ ] Screenshots read at 375 and 1280 (Live no filter, Live filtered, Past)
- [ ] Smoke test confirmed (owner: reinstall, use the Live filters)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
