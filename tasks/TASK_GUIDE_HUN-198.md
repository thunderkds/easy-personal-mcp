**Date**: 2026-11-03
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P2
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary; Critical Constraints: metadata only, CSP `default-src 'self'`, core is stdlib only)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-053, FR-065 and Open Questions row 23
7. Read `src/easy_personal/core/easymem.py` in full, `src/easy_personal/core/memgraph.py` around `easymem.summary`, and `web/src/MemoryView.jsx` `EasyMemoryPanel`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-03, in easy-memory-mcp: "as a memory mcp, you can analyze, collect and give the easy-personal the number and analyze number to show on the dashboard"; plan approved "yes, let go"; then "do all". easy-memory now writes `~/.easy-memory/<repo-key>/report.json` (its FR-015, HUN-197, merged `261c6f6`). Real file on 2026-11-03:

```json
{"v": 1, "generated": "2026-11-03T10:06:31Z", "min_sessions": 10, "sessions_logged": 9, "no_data": 0,
 "no_claude_log": 1, "forced": 0,
 "groups": {"on":  {"sessions": 3, "tokens_all_median": 18658261, "tokens_io_median": 129194, "turns_median": 177},
            "off": {"sessions": 5, "tokens_all_median": 16553412, "tokens_io_median": 100808, "turns_median": 169}},
 "note_opened": {"opened": 0, "indexed": 3}, "too_few": true, "on_vs_off_pct": null,
 "store": {"notes": 139, "bytes": 275959}}
```

`on_vs_off_pct` = 100 × (on median − off median) ÷ off median, rounded to a whole number: negative means memory on uses fewer tokens.

Mockup (Supervisor default, PRD row 23; inside the existing easy-memory panel, under the hit-rate line):

```
Memory on vs off            updated 2 h ago
On    3 sessions   18.7M tokens / session
Off   5 sessions   16.6M tokens / session
too few sessions to compare (3 on / 5 off, need 10 each)
```

With `too_few` false the last line is "Saves 12%" (pct −12) or "Uses 8% more" (pct 8) or "No difference" (0).

**Restated intent**:

> Backend: `easymem.report(folder) -> dict` reads `<folder>/report.json` (size cap 64 KB) and returns only the FR-065 fields, validated as numbers / bools: `{"state": "ok", "generated", "min_sessions", "too_few", "on_vs_off_pct", "on": {"sessions", "tokens_all_median"}, "off": {...}}`; `{"state": "unsupported"}` when `v` > 1; `{"state": "none"}` when the file is missing, unreadable, not JSON, not an object, or a needed field has the wrong type. It never raises. `memgraph` puts it in the panel as `easy_memory.report`. Frontend: `EasyMemoryPanel` shows the block from the mockup for `state: "ok"`, "report format not supported" for `unsupported`, "no report yet" for `none`.

**Out of scope**:

* Any write to `~/.easy-memory/` (read-only rule of `easymem.py`)
* Calling easy-memory or running its `report` (DDR-0002)
* `tokens_io_median`, `turns_median`, `note_opened`, `no_data`, `store` (not shown in this block)
* A new card, view or MCP tool; a chart

**Requirement Refs**: FR-065, FR-053

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, 2026-11-03; owner "do all")
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None in this repo. Producer: easy-memory-mcp HUN-197 (merged).

**Branch**: `HUN-198` in worktree `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/HUN-198`.

**Entry point**: `easy-personal dashboard` → Memory view → `EasyMemoryPanel` (`web/src/MemoryView.jsx`), data from `/api/memory-graph?project=...` `easy_memory.report`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `easymem.report` on the real-shape fixture above returns `state: "ok"` with exactly the restated fields and values; no other key from the file (e.g. `store`, `note_opened`) is returned | FR-065 |
| 2 | Missing file, empty file, `[]`, `{`, a 65 KB file, `groups.on.sessions = "3"` → `state: "none"`, no exception; `v: 2` → `state: "unsupported"` | FR-065 |
| 3 | `/api/memory-graph?project=<p>` returns `easy_memory.report` for a project with an easy-memory store; a store folder holding only `report.json` (no `notes/`) does not crash the project list or the graph | FR-065, FR-053 |
| 4 | Panel with `too_few` true shows both group rows and "too few sessions to compare (3 on / 5 off, need 10 each)", and no "Saves"/"Uses" text | FR-065 |
| 5 | `too_few` false: pct −12 → "Saves 12%"; pct 8 → "Uses 8% more"; pct 0 → "No difference"; pct null → no saving line | FR-065 |
| 6 | `unsupported` → "report format not supported", no numbers; `none` → "no report yet" | FR-065 |
| 7 | "updated N ago" from `generated` (fixed clock in the test): 2 h → "updated 2 h ago"; tokens use the existing compact formatter (18,658,261 → "18.7M") | FR-065 |
| 8 | Leak suite still passes; a planted prompt/answer/session id canary inside `report.json` never reaches the API response | FR-065, metadata-only |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_HUN-198.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core/test_easymem.py tests/core/test_memgraph.py tests/surfaces/test_api_memory_graph.py tests/leak -q && cd web && npx vitest run && cd .. && npm --prefix web run build && uv run pytest -q
```

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Memory view, easy-memory-mcp project, owner store copy, 1280×720 | headless screenshot | block matches the mockup under the hit-rate line |
| Same with a fixture `too_few: false`, pct −12 | headless screenshot | "Saves 12%" line |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors / fonts | reuse `.memory-easy`, `.muted`, existing number formatters | no new colour or token |
| Accessibility | vitest | block is a labelled group (`aria-label="memory on vs off"`); numbers are plain text |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1280×720 and 375×812 | screenshot + `scrollWidth` | no horizontal page scroll; the two group rows stay inside the panel |

---

## Approach

**Vital slice**: `easymem.report` → panel field → three lines in `EasyMemoryPanel`.
**Cut list**: no chart, no in+out tokens, no turns, no note-opened rate, no refresh button.

1. Tests first: `tests/core/test_easymem.py` (AC1, AC2), `tests/surfaces/test_api_memory_graph.py` (AC3), leak canary (AC8), `web/src/MemoryView.test.jsx` (AC4–AC7).
2. `easymem.report(folder)`; call it in `memgraph` where `summary` is called; add it to `panel`.
3. `EasyMemoryPanel` block; rebuild `src/easy_personal/dashboard/static/`.

---

## Edge Case Checklist

- [ ] `generated` in the future or unparseable → no "updated" text, block still shows
- [ ] Booleans are not numbers (`True` must not pass as a session count)
- [ ] Negative or float session counts → `state: "none"`
- [ ] `report.json` is a symlink → read it only if it resolves inside the store folder, else `none`
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/easymem.py` | `report(folder)` |
| `src/easy_personal/core/memgraph.py` | panel gets `report` |
| `tests/core/test_easymem.py`, `tests/surfaces/test_api_memory_graph.py`, `tests/leak/*` | tests |
| `web/src/MemoryView.jsx`, `web/src/MemoryView.test.jsx`, `web/src/app.css` (only if needed) | block + tests |
| `src/easy_personal/dashboard/static/` | rebuilt page |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| anything under `~/.easy-memory/` | read-only rule |
| the easy-memory-mcp repo | producer, other board |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit framework files |

---

## Test Plan

pytest for the reader (fixture folders in `tmp_path`, `EASY_MEMORY_HOME` set), the API and the leak canary; vitest for the panel with fixed clock; build; headless screenshots on a copy of the owner store.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] Not merged into `main`
- [ ] Report: `Status: done` with commits, or `Status: blocked` + `Question:`
