**Date**: 2026-11-02
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: Memory note, easy-memory note, Cited file)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `BRAINSTORMING_LOG_EASYMEMORY.md` (Option A), `PRD.md` US-024, FR-041, FR-052, FR-053, and the **Response contract** section of `tasks/TASK_GUIDE_HUN-184.md`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-11-02: show what easy-memory saved in the Memory view. Owner chose (grill): cited files as nodes, stale mark, top 30, 30-day panel, easy-memory-only projects listed; Option A.

**Restated intent**:

> In `web/src/MemoryView.jsx`, draw the `easy_memory` node kind from `/api/memory-graph` on a new outer ring 4 with its own colour, sized by `cited`, and show a stale mark on any node (also a kit / Claude node) whose `stale` is `stale` or `gone`. A click shows path, citation count, newest citing note's date and stale state only. Add a panel with notes saved per day for 30 days (sparkline + total), the hit rate (`injected` ÷ `prompts_on`, with the prompt count, "no prompts" when 0), files not drawn and abstained notes. The project list shows the easy-memory note count beside the kit and Claude counts.

**Out of scope**:

* Any backend change (HUN-184)
* Edges for cited-file nodes; filters; a date range picker (fixed 30 days)
* Any note text (the API never sends it)

**Requirement Refs**: US-024, FR-052, FR-053

### Shared rules (easy-memory layer)

* Hand-made SVG, no layout or chart library, no new npm dependency (`RingsGraph` + a small inline SVG sparkline).
* Colours only from `web/src/tokens.css` `var(--…)`; state never shown by colour alone (stale has a text label / symbol with an accessible name).
* Keep `layout()` deterministic (same nodes → same numbers); update `web/src/test/memory-graph.golden.svg` on purpose and say why in the review.
* Never use `git stash`; refresh `.claude/hooks/.state/active_task` right before every full test run. Build the page (`npm --prefix web run build`) before any real run (learning: static build is gitignored).

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved grill + Option A, 2026-11-02)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: HUN-184 (backend, merged first; its guide holds the Response contract)

**Entry point**: `easy-personal dashboard` → `// work` → Memory → pick a project

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `easy_memory` nodes draw on ring 4 with the easy-memory colour and the legend / `KIND_NAMES` entry "easy-memory file"; radius grows with `cited` within a fixed min/max | FR-052 |
| 2 | A node with `stale` `stale` or `gone` (any kind) shows a mark with an accessible name ("changed since the newest note" / "file gone"); `fresh` and `unchecked` show no mark, `unchecked` says "not checked" in the side panel | FR-052 |
| 3 | Clicking an easy-memory node shows path, `cited` count, `newest` date and stale state only; a merged kit node shows its existing fields plus the same three | FR-052 |
| 4 | Panel: 30-bar sparkline of `days` + total `notes`; hit rate as a percent with "of N prompts"; `prompts_on: 0` → "no prompts"; `files_not_drawn` and `abstained` shown when > 0 | FR-053 |
| 5 | `easy_memory: null` → no panel, no ring 4, the graph looks as before (golden unchanged for a fixture without easy-memory) | FR-052, FR-053 |
| 6 | Project list shows "easy-memory N" beside the kit / Claude counts; an easy-memory-only project is listed and opens | FR-053 |
| 7 | `layout()` stays deterministic with ring 4 (same nodes in any order → same coordinates) | FR-052 |
| 8 | Real run: dashboard on the owner's machine, Memory → easy-personal-mcp shows cited-file nodes, at least one merged kit node, the panel with real numbers; screenshot pasted | US-024 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run MemoryView App neon && cd .. && uv run pytest tests/test_web_toolchain.py tests/test_site_content.py -q
```

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Memory view at 1280×720, fixture project with kit nodes + 30 easy-memory nodes (2 stale, 1 gone, 1 merged) | headless screenshot (learnings: `scratchpad/hun178` UI verify pattern, CDP `shoot.mjs`) | ring 4 readable, labels not overlapping the panel; same look as the existing rings |
| Same, fixture with no easy-memory | golden SVG test | unchanged from today's golden |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors from tokens | `web/src/neon.test.jsx`-style test | new node colour, stale mark and sparkline use only `var(--…)` from `web/src/tokens.css` |
| Fonts / labels | computed style | Plex Mono for counts and dates, as other labels |
| Accessibility | vitest | stale mark has an accessible name; sparkline has a text summary (`aria-label` with total); contrast ≥ 4.5:1 in both themes |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1280×720 | screenshot | as above |
| narrow (rail collapsed) | screenshot | panel moves under the graph, no horizontal page scroll |

---

## Approach

**Vital slice**: ring 4 + kind colour + stale mark → click panel fields → 30-day panel → list count.
**Cut list**: no edges for cited files; no hover sparkline values; no filter by stale.

`RADII` gains a ring 4 radius (grow `SIZE` if needed); keep `MAX_DRAWN`. Sparkline = 30 `<rect>` in a small inline SVG.

---

## Edge Case Checklist

- [ ] `easy_memory` null vs present with 0 notes
- [ ] 30 nodes on ring 4 with long paths → label rule (`LABEL_LIMIT`) still applies
- [ ] Merged kit node keeps its ring and colour; only the mark and fields are added
- [ ] `prompts_on` 0 → "no prompts", never NaN / 0%
- [ ] All-zero `days` → flat sparkline, total 0
- [ ] Easy-memory-only project (no kit / Claude nodes) → ring 4 only, centre empty without errors
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/MemoryView.jsx` | ring 4, kind, stale mark, panel, list count |
| `web/src/app.css` | node colour, mark, panel layout |
| `web/src/MemoryView.test.jsx`, `web/src/App.test.jsx`, `web/src/test/memory-graph.golden.svg` | tests |
| `README.md` | Memory view section: easy-memory layer, what is never shown |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/**` | HUN-184 |
| `web/package.json`, `web/package-lock.json` | No new dependency |
| `web/src/tokens.css` | Use existing tokens; ask the Supervisor if one is missing |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Vitest with fixture API responses following the Response contract (with and without `easy_memory`, merged node, stale / gone / unchecked); determinism test for `layout()`; golden SVG; headless screenshots at 1280×720 and narrow; real run on the owner's machine.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review