**Date**: 2026-11-02
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P0
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: Gone project)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` US-025, FR-037, FR-038, FR-054, FR-055

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-11-02: "I check the board, but the component UX is not good at all, could you test all and update" → owner chose three parallel tasks (this one = the Board view).

Supervisor screenshots 2026-11-02 of the real dashboard (`web/src/BoardView.jsx`): the project legend lists 148 projects, 138 "no data" (temp and test folders), as a wall of text above the board; the board merges all projects (Todo 104, Done 656), page \~19,700 px tall; card titles show raw markdown (`**Replies to the user read plainly…**`) and run 10+ lines; the four columns are narrow in a wide page; at 375 px the page scrolls sideways.

**Restated intent**:

> Rework `web/src/BoardView.jsx` so it opens on one project (the one with the newest `last_pull`; else the first with tasks), with a project picker that lists projects with a board first, each with its open-task count, plus "All projects". Projects with `status: "no data"` are not listed in the main list; a "N without a board" toggle shows their names. If the response has `gone` > 0, say "N gone folders hidden". Card titles are plain text (strip markdown emphasis/code marks), clamped to 3 lines with the full title available (accessible name / expand). The project name on a card shows only in "All projects". Columns fill the width; at 375 px no page-wide horizontal scroll (columns stack or scroll inside their own container).

**Out of scope**:

* Any backend change (the parallel backend task adds `gone`; treat it as optional, missing = 0)
* New statuses, drag and drop, editing tasks, links to Linear
* Other views (parallel task)

**Requirement Refs**: US-025, FR-054, FR-055

### Shared rules

* Colours only from `web/src/tokens.css` `var(--…)`; no new npm dependency; keep the kit's status names (FR-038).
* Parallel task HUN-(Usage/Memory/Kit) also edits `web/src/app.css`: add your rules in one block under a `/* Board view */` comment to keep the merge simple.
* Never use `git stash`; refresh `.claude/hooks/.state/active_task` right before every full test run; run `npm --prefix web run build` before any real run (static build is gitignored).

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, owner chose "All three tasks in parallel", 2026-11-02)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: none (works with and without the backend `gone` field)

**Entry point**: `easy-personal dashboard` → `// work` → Board (`web/src/BoardView.jsx`)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Opens on the project with the newest `last_pull` (else the first with tasks); picker lists board projects first with open-task counts, plus "All projects" | FR-055 |
| 2 | `no data` projects are not in the main list; "N without a board" toggle (button with `aria-expanded`) reveals their names | FR-055 |
| 3 | `gone` > 0 → "N gone folders hidden"; missing `gone` → nothing shown, no error | FR-054 |
| 4 | Titles: `**x**`, `__x__`, `` `x` `` shown as plain `x`; clamped to 3 lines; full title reachable (accessible name or expand) | FR-055 |
| 5 | Card shows the project name only in "All projects" | FR-055 |
| 6 | 375 px: `document.documentElement.scrollWidth <= innerWidth` on the Board view with 6 columns of data | FR-055 |
| 7 | Existing Board tests still pass or are updated on purpose (say why per changed test) | FR-038 |
| 8 | Real run on the owner's machine: screenshots at 1440×900 and 390×844, page height before/after pasted | US-025 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run && cd .. && uv run pytest tests/test_web_toolchain.py tests/test_site_content.py -q
```

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Board at 1440×900, owner's real data | headless screenshot (learnings: `scratchpad/hun178` pattern; playwright-core from `../easy-ui-mcp/node_modules`, `/usr/bin/google-chrome`; nav items are buttons, narrow needs the `Menu` button first) | one project's columns visible above the fold; no legend wall |
| Board "All projects" | screenshot | cards show project names; Done collapsed as today |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors from tokens | `web/src/neon.test.jsx`-style test | only `var(--…)` |
| Fonts | computed style | Plex Mono for ids and counts, as today |
| Accessibility | vitest | picker has a label; toggle has `aria-expanded`; clamped title has the full text as accessible name |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1440×900 | screenshot | columns fill the width |
| 390×844 | screenshot + scrollWidth check | no page-wide horizontal scroll |

---

## Approach

**Vital slice**: default project + picker → hide no-data list behind toggle → plain clamped titles → width/phone layout.
**Cut list**: no search box; no per-column sort; no persistence of the chosen project beyond the page (optional `localStorage` only if trivial).

---

## Edge Case Checklist

- [ ] No project has a board → "No project on this machine has a board file" + the toggle
- [ ] Two projects with the same folder name → picker entries still distinct (append the board kind or an index)
- [ ] Project with a board but 0 tasks
- [ ] Workspace/account scope change resets the chosen project if it is no longer listed
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/BoardView.jsx` | picker, toggle, titles, layout |
| `web/src/app.css` | `/* Board view */` block only |
| `web/src/BoardView.test.jsx` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/**` | backend task |
| `web/src/UsageView.jsx`, `MemoryView.jsx`, `KitView.jsx`, `Widgets.jsx`, `Timeline.jsx` | parallel frontend task |
| `web/package.json`, `web/package-lock.json`, `web/src/tokens.css` | no new dependency / tokens |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Vitest with fixture `/api/board` responses (several projects, no-data ones, `gone` present/absent, markdown titles, duplicate names); a 375 px layout check in the real-run screenshot script.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review