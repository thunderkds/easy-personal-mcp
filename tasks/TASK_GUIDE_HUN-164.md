**Date**: 2026-10-31
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
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
6. Read `memory/codebase-map.md`, `PRD.md` US-020, FR-041, NFR-002, NFR-010, and `BRAINSTORMING_LOG_WORKVIEW.md` (sketches, Edge Case Checklist, Surgical Scope)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-31: "wanna build the personal like that [https://github.com/blogminhquy/javis-os](<https://github.com/blogminhquy/javis-os>) ... core vision still go with ours, but the idea is good to following — the data showing, the UX, the board". Grill + brainstorm decisions: `memory/decisions.md` 2026-10-31 "Work view inspired by Javis OS, read-only"; `BRAINSTORMING_LOG_WORKVIEW.md` Option A (owner-selected; sketches there are the layout reference).

**Restated intent**:

> Add a memory-graph reader that, live on request and never into the store, reads **memory notes** — `<repo>/memory/**/*.md` (**kit memory**) of every kit project and `~/.claude*/projects/*/memory/*.md` (**Claude memory**) of every confirmed agent account — grouped by project. Nodes = file stem + frontmatter `name` + `description`; edges = `[[name]]` links; unresolved links = **dangling link** nodes. Serve at `GET /api/memory-graph?project=…`; show a Memory view under `// work` with a project picker, a deterministic rings layout in SVG (MEMORY.md centre, cold files ring 1, learning records + Claude notes ring 2, dangling outermost), colour by kind, and a side panel with name, description, path, links in/out. Note bodies are never sent to the page.

**Out of scope**: note bodies or previews; storing anything; editing notes; force-directed layout; cross-project 'all' merged view beyond a simple per-project list; MCP tool.

**Requirement Refs**: PRD US-020, FR-041, NFR-002, NFR-010.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's request — Supervisor, 2026-10-31, owner approved PRD answers, Option A and this 5-task breakdown
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: HUN-160 (rail `// work` group). **Entry point**: route `/api/memory-graph`; Memory tab in `// work`.

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | Response holds only path (relative to project/account), kind, `name`, `description`, link targets; a leak test plants a secret in a note body and asserts it is absent | FR-041, NFR-002 |
| 2 | Claude memory is mapped to its project via the Claude project folder → stored project path (same mapping ingest uses); unmapped folders show under their folder name | FR-041, FR-009 |
| 3 | Link parsing handles `[[name]]`, `[[name|alias]]`, `[[name#anchor]]`; self-links ignored; cycles fine | FR-041 |
| 4 | Rings layout is deterministic: same input → byte-identical SVG (golden test) | NFR-004 |
| 5 | Side panel shows name, description, path, in/out counts — never body | FR-041 |
| 6 | 50 fixture projects × 40 notes: `/api/memory-graph` per project < 1 s and the project list < 1 s | NFR-010 |
| 7 | Nothing written; file mtimes unchanged | NFR-003 |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | fixture project with MEMORY.md, decisions.md, 2 LRs, a Claude note, a dangling `[[ghost]]` | 6 nodes incl. 1 dangling, correct edges | pytest |
| 2 | note body containing `SECRET_Y` | `SECRET_Y` nowhere in response or page | pytest + vitest |
| 3 | same fixture twice | identical SVG | vitest golden |

### Verification Command (exact, runnable)

```bash
uv run pytest -q && npm --prefix web ci && npm --prefix web test -- --run && npm --prefix web run build && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-164.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-164.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Memory at 1280×720 | easy-ui MCP (`http://localhost:8765/mcp`) screenshot on fixture data vs the sketch in `BRAINSTORMING_LOG_WORKVIEW.md` | rings graph + side panel + legend as in the sketch |

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

**Pattern reference**: `accounts.list_accounts` (confirmed accounts and folders), `discovery.resolve_project`, hand-drawn SVG in `Widgets.jsx`/`Timeline.jsx`. New `core/memgraph.py`; frontmatter parsed with a small line reader (no YAML dependency) — only `name:` and `description:` keys.
**Vital slice**: reader + route + rings SVG + side panel + project picker.
**Cut list**: force layout; zoom/pan beyond SVG viewBox scroll; search; body preview; graph history.

---

## Edge Case Checklist

- [ ] note without frontmatter → name = file stem
- [ ] two notes with the same `name` in one project → both nodes, links resolve to the first by path order, counted as duplicate
- [ ] 1,000+ notes in one account → cap rendered nodes (e.g. 300) with a 'N more' note; API still returns counts
- [ ] symlinked memory dirs / loops → resolve once, skip repeats
- [ ] non-UTF-8 or > 1 MB file → abstained, not read fully
- [ ] description longer than \~200 chars → truncated in panel

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/memgraph.py (new)` | reader |
| `src/easy_personal/surfaces/api.py` | `/api/memory-graph` |
| `web/src/MemoryView.jsx (new) + test` | view + rings layout |
| `web/src/App.jsx` | append Memory tab |
| `tests/core/test_memgraph.py, tests/leak/…, tests/fixtures/…` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/store.py, migrations/` | graph is never stored |
| `src/easy_personal/connectors/**`, `src/easy_personal/launch.py` | connectors and keychain are out of scope |
| any repo's `PROJECT_KANBAN.md` / `memory/**`, `~/.claude*/**` | read-only sources (NFR-003) |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |

---

## Test Plan

pytest: parser + mapping + leak + timing; vitest: rings golden, panel, picker; easy-ui screenshot.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled or N/A with reason (Hard-Stop Gate 6)
- [ ] `Skill({ skill: "verify" })` run
- [ ] Supervisor notified: task ready for Stage 4 review