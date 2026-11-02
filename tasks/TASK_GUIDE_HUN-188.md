**Date**: 2026-11-02
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: Gone project, Memory note)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` US-025, FR-023, FR-041, FR-054, FR-056

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-11-02: "I check the board, but the component UX is not good at all, could you test all and update" → owner chose three parallel tasks (this one = Usage, Memory, Kit).

Supervisor screenshots 2026-11-02 of the real dashboard at 1440×900:

* Usage "Total tokens" wraps mid-number (`3,107,421,8` / `01`).
* Usage "API-equivalent cost" card spills `$2138.61 + abstained: (unknown), claude-opus-5` across 4 lines in the big figure.
* Usage "By model" cuts names to `claude-opu…`, so two rows (`claude-opus-4-…` vs `claude-opus-5…`) look the same.
* Memory opens on `-home-hungnguyenhuu-workspace-pets-per-…` (a Claude folder name that sorts first because `-` < `/`), a near-empty graph, while real projects have many notes.
* Kit lists 160 projects (page \~29,000 px), most idle or test folders.

**Restated intent**:

> In the Usage view, keep each big figure on one line (compact form like `3.11 B`, full value in `title` and accessible name), move abstained notes out of the cost figure into a muted line under it, and show model names so no two rows have the same visible text (wrap or a smarter shortening that keeps the distinguishing part). In the Memory view, open on the project with the most notes (`kit_notes + claude_notes + easy_memory_notes`, ties by name) and show "N gone folders hidden" when the list response has `gone` > 0. In the Kit view, list projects active in the last 30 days first and fold the rest behind "Show N more"; show "N gone folders hidden" when `gone` > 0.

**Out of scope**:

* Any backend change (the parallel backend task adds `gone`; treat it as optional, missing = 0)
* The Board view (parallel task) and the HUN-185 easy-memory graph layer (Todo)
* New charts, new filters, the right rail content

**Requirement Refs**: US-025, FR-054, FR-056

### Shared rules

* Colours only from `web/src/tokens.css` `var(--…)`; no new npm dependency.
* Parallel Board task also edits `web/src/app.css`: add your rules in one block under a `/* Usage, Memory, Kit UX */` comment.
* Keep `MemoryView` `layout()` and its golden SVG unchanged (HUN-185 builds on them).
* Never use `git stash`; refresh `.claude/hooks/.state/active_task` right before every full test run; run `npm --prefix web run build` before any real run.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, owner chose "All three tasks in parallel", 2026-11-02)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: none

**Entry point**: `easy-personal dashboard` → `// usage` → Usage (`web/src/UsageView.jsx`, `Widgets.jsx`); `// work` → Memory (`web/src/MemoryView.jsx`); `// kit` → Kit (`web/src/KitView.jsx`)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Usage big figures: `3107421801` renders compact on one line; full value in `title` and accessible name; no wrap at 1440 and 390 px | FR-056 |
| 2 | Cost card: the figure is the dollar amount only; abstained models listed in a muted line under it | FR-056 |
| 3 | By model: rows for `claude-opus-4-1-20250805` and `claude-opus-5` show different visible text; full name in `title` | FR-056 |
| 4 | Memory opens on the project with the most notes (ties by name); a Claude-folder-only key is never the default when a real project has notes | FR-056 |
| 5 | Kit: projects active in the last 30 days first; others folded behind "Show N more" (button with `aria-expanded`) | FR-056 |
| 6 | Memory and Kit show "N gone folders hidden" when `gone` > 0; nothing when missing | FR-054 |
| 7 | Memory golden SVG unchanged | FR-041 |
| 8 | Real run on the owner's machine: screenshots of Usage, Memory, Kit at 1440×900 and 390×844 pasted | US-025 |

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
| Usage, Memory, Kit at 1440×900 on the owner's data | headless screenshot (playwright-core from `../easy-ui-mcp/node_modules`, `/usr/bin/google-chrome`; nav items are buttons; narrow needs the `Menu` button first) | figures on one line; distinct model names; Memory shows a real project; Kit short |
| Memory graph, fixture | golden SVG | unchanged |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors from tokens | `web/src/neon.test.jsx`-style test | only `var(--…)` |
| Fonts | computed style | Plex Mono for figures, as today |
| Accessibility | vitest | compact figures carry the full value as accessible name; fold button has `aria-expanded` |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1440×900 | screenshot | no wrapped figure |
| 390×844 | screenshot + scrollWidth check | no page-wide horizontal scroll |

---

## Approach

**Vital slice**: compact number formatter in `format.js` → cost card abstained line → model name display → Memory default → Kit fold.
**Cut list**: no new Usage widgets; no Kit sorting controls; no Memory search.

---

## Edge Case Checklist

- [ ] Figures < 10,000 stay exact (no "9.9 K" for small numbers)
- [ ] Cost with no abstained models → no muted line
- [ ] All projects idle > 30 days in Kit → show the 10 most recent, fold the rest
- [ ] Memory list empty → current empty text
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/UsageView.jsx`, `web/src/Widgets.jsx`, `web/src/Breakdown.jsx`, `web/src/format.js` | figures, cost line, model names |
| `web/src/MemoryView.jsx` | default project, gone line (not `layout()`) |
| `web/src/KitView.jsx` | order + fold, gone line |
| `web/src/app.css` | `/* Usage, Memory, Kit UX */` block only |
| tests for each | vitest |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/**` | backend task |
| `web/src/BoardView.jsx`, `BoardView.test.jsx` | parallel Board task |
| `web/src/test/memory-graph.golden.svg` | must stay unchanged |
| `web/package.json`, `web/package-lock.json`, `web/src/tokens.css` | no new dependency / tokens |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Vitest with fixture responses (large token totals, abstained models, two opus model ids, a Claude-folder-only memory key with fewer notes, Kit with idle and active projects, `gone` present/absent); real-run screenshots.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review