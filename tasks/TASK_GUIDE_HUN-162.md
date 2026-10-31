**Date**: 2026-10-31
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P0
**Assigned agent**: Frontend-Implementer (owns the view) — the API route is small enough to stay in the same task; Backend-Implementer only if the Supervisor splits the spawn
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: Work view terms, Critical Constraints incl. the 2026-10-31 named exception)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `memory/codebase-map.md`, `PRD.md` US-017, FR-037, FR-038, NFR-010, and `BRAINSTORMING_LOG_WORKVIEW.md` (sketches, Edge Case Checklist, Surgical Scope)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-31: "wanna build the personal like that [https://github.com/blogminhquy/javis-os](<https://github.com/blogminhquy/javis-os>) ... core vision still go with ours, but the idea is good to following — the data showing, the UX, the board". Grill + brainstorm decisions: `memory/decisions.md` 2026-10-31 "Work view inspired by Javis OS, read-only"; `BRAINSTORMING_LOG_WORKVIEW.md` Option A (owner-selected; sketches there are the layout reference).

**Restated intent**:

> Add a `kanban` reader that reads each kit project's `PROJECT_KANBAN.md` live on request and returns **board tasks** (project, task id, title, board status) plus per project `board` kind and **last board pull** (file mtime) when `.claude/board.json` names a board. Serve it at `GET /api/board` (workspace / project scope) and show it as the Board view under `// work`: one column per board status with counts, Done collapsed, cards with id, title, project dot and a board-kind tag; a header line with project count, synced count + pull age, and no-data count.

**Out of scope**: editing or moving tasks; calling Linear or any board API; storing tasks; history / time-in-column; MCP tool.

**Requirement Refs**: PRD US-017, FR-037, FR-038, NFR-010.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's request — Supervisor, 2026-10-31, owner approved PRD answers, Option A and this 5-task breakdown
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: HUN-160 (rail `// work` group). **Entry point**: route `/api/board`; Board tab in `// work`.

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | Reader parses `## Board` subsections `Todo`, `In Progress`, `Ready for Review`, `Done`, `Closed (will not do)` and the `## Blocked` table; unknown subsections are counted as abstained, never coerced | FR-037, FR-003 |
| 2 | Projects = `kit.scoped_projects()` that still exist on disk; a missing or unparsable file = no data for that project | FR-037, FR-009 |
| 3 | Response holds only project folder name, id, title, status, board kind, pull time — nothing else from the file | Critical Constraints exception |
| 4 | Board view: columns by board status with counts, Done collapsed, filter by workspace (global scope) and project | FR-038 |
| 5 | Nothing is written anywhere (test asserts file mtimes unchanged) | NFR-003 |
| 6 | 50 fixture projects × 60 tasks: `/api/board` < 1 s | NFR-010 |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | fixture repo with all 6 sections + an unknown `### Parked` | tasks in the right columns; `Parked` counted abstained | pytest |
| 2 | repo with `.claude/board.json` `{"board":"linear"}` | tasks tagged `linear`, pull age shown | pytest + vitest |
| 3 | 50 fixture projects | < 1 s | pytest timing test |

### Verification Command (exact, runnable)

```bash
uv run pytest -q && npm --prefix web ci && npm --prefix web test -- --run && npm --prefix web run build && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-162.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-162.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Board at 1280×720 | easy-ui MCP (`http://localhost:8765/mcp`) screenshot on fixture data vs the sketch in `BRAINSTORMING_LOG_WORKVIEW.md` | columns, cards, header line as in the sketch |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors from tokens | `web/src/neon.test.jsx`-style test | only `var(--…)` from `web/src/tokens.css`, no raw literals |
| Fonts / labels | easy-ui computed style | Plex Sans body, Plex Mono labels, same card style as Kit/Usage views |
| Accessibility | vitest + easy-ui `ui_check` | landmarks, `aria-label` on SVG, keyboard focus visible, contrast ≥ 4.5:1 |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1280×720 | easy-ui screenshot (fixed viewport, see learnings "easy-ui + Graphite Mint") | matches sketch |
| narrow (rail collapsed) | easy-ui screenshot | no horizontal page scroll; wide content scrolls inside its box |

---

## Approach

**Pattern reference**: `core/kit.py` (per-project reads, no data handling), `KitView.jsx` (scoped view). New `core/board.py`; parse line by line, no markdown library.
**Vital slice**: reader + route + columns view.
**Cut list**: drag & drop; search; card detail panel; linking board tasks to kit traces.

---

## Edge Case Checklist

- [ ] table rows vs bullet lists inside a section (both formats exist across repos)
- [ ] titles with `|`, backticks or markdown links → shown as plain text, escaped
- [ ] same task id in two projects → key = project + id
- [ ] worktree folders under `.claude/worktrees/` must not appear as projects
- [ ] very large Done section (hundreds) → collapsed; count still exact
- [ ] non-UTF-8 file → no data, not a crash

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/board.py (new)` | reader |
| `src/easy_personal/surfaces/api.py` | `/api/board` |
| `web/src/BoardView.jsx (new) + test` | view |
| `web/src/App.jsx` | append Board tab to `// work` |
| `tests/core/test_board.py, tests/fixtures/…` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/metrics.py, migrations/` | no store change |
| `src/easy_personal/connectors/**`, `src/easy_personal/launch.py` | connectors and keychain are out of scope |
| any repo's `PROJECT_KANBAN.md` / `memory/**`, `~/.claude*/**` | read-only sources (NFR-003) |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |

---

## Test Plan

pytest: parser table-driven over fixture kanbans, read-only assertion, timing test; vitest: columns/counts/collapse; easy-ui screenshot.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled or N/A with reason (Hard-Stop Gate 6)
- [ ] `Skill({ skill: "verify" })` run
- [ ] Supervisor notified: task ready for Stage 4 review