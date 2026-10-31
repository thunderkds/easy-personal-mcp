**Date**: 2026-10-31
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P0
**Assigned agent**: Backend-Implementer (core, migration, MCP), then Frontend-Implementer for the Usage series
**Agent guide**: `agents/backend.md` (then `agents/frontend.md`)

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: Work view terms, Critical Constraints incl. the 2026-10-31 named exception)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `memory/codebase-map.md`, `PRD.md` US-018, FR-039, FR-008, NFR-009, OQ 14, and `BRAINSTORMING_LOG_WORKVIEW.md` (sketches, Edge Case Checklist, Surgical Scope)
7. Read `src/easy_personal/core/migrations/m0005_event_accounts.py` and `tests/` for its migration test pattern

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-31: "wanna build the personal like that [https://github.com/blogminhquy/javis-os](<https://github.com/blogminhquy/javis-os>) ... core vision still go with ours, but the idea is good to following — the data showing, the UX, the board". Grill + brainstorm decisions: `memory/decisions.md` 2026-10-31 "Work view inspired by Javis OS, read-only"; `BRAINSTORMING_LOG_WORKVIEW.md` Option A (owner-selected; sketches there are the layout reference).

**Restated intent**:

> Store each event's session **entrypoint**, split usage into **typed work**, **agent work** and **unknown** (PROJECT_SPEC glossary), expose the split in `usage_summary` / `usage_breakdown` (MCP, CLI, API — one core) and show a You / Agents series per day in the Usage view. A new versioned migration adds the column and marks every source file for re-read so the next normal ingest backfills; until then old rows count as unknown.

**Out of scope**: new MCP tools; a separate backfill command; splitting cost by price differently; per-agent-type breakdown.

**Requirement Refs**: PRD US-018, FR-039, FR-008, NFR-009, OQ 14.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's request — Supervisor, 2026-10-31, owner approved PRD answers, Option A and this 5-task breakdown
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: None — can start immediately. **Entry point**: `usage_summary` MCP tool; route `/api/usage/summary`; Usage view chart.

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | Migration `m0007_*` adds `entrypoint TEXT` to `events` (nullable) and resets ingest offsets so the next ingest re-reads every file; it is idempotent and tested from the m0006 schema | FR-039, NFR-009 |
| 2 | Re-read does not duplicate events: event count after backfill equals count before; only `entrypoint` changes | FR-001 idempotent |
| 3 | Classification: `is_sidechain = 1` → agent; else session entrypoint `cli` → typed; other known entrypoint → agent; missing → unknown (abstained, counted) | FR-039, FR-003 |
| 4 | `usage_summary` and `usage_breakdown(by=day)` return `typed`, `agent`, `unknown` token totals that sum to the existing total; MCP, CLI and API give the same numbers | FR-008, FR-039 |
| 5 | Usage view shows You / Agents (and Unknown when > 0) per day with legend and accessible labels; workspace/account scope still applies | US-018, FR-035 |
| 6 | Workspace-filtered 30-day query over 1,000,000 events stays < 500 ms (`EXPLAIN QUERY PLAN` uses an index) | NFR-009 |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | fixture: 1 cli session, 1 `sdk-cli` session, 1 cli session with sidechain records | typed = cli main-thread tokens; agent = sdk-cli + sidechain | pytest |
| 2 | store migrated from m0006 with old rows | all old rows unknown until next ingest; after ingest none unknown, counts unchanged | pytest |
| 3 | same period via MCP, CLI `--json`, API | identical split numbers | pytest |

### Verification Command (exact, runnable)

```bash
uv run pytest -q && npm --prefix web ci && npm --prefix web test -- --run && npm --prefix web run build && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-161.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-161.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Usage view at 1280×720 | easy-ui MCP (`http://localhost:8765/mcp`) screenshot on fixture data vs the sketch in `BRAINSTORMING_LOG_WORKVIEW.md` | You / Agents series readable, legend, same chart style |

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

**Pattern reference**: `m0005_event_accounts.py` (column add + backfill path), `sources/claude_code.py:147` (`is_sidechain`), `metrics.usage_summary`. Run `Skill({ skill: "migration-safety" })` before the migration lands (Supervisor runs it at Stage 3/4). Session entrypoint = first non-null `entrypoint` seen in the session.
**Vital slice**: column + backfill + 3-way totals in summary/breakdown + one stacked/paired series in Usage.
**Cut list**: per-model split; split in warnings/rules; split on the timeline.

---

## Edge Case Checklist

- [ ] session whose first records have no `entrypoint` but later ones do
- [ ] sidechain records inside a headless session → agent, counted once
- [ ] new entrypoint value (e.g. `ide`) → agent (only `cli` is typed, per FR-039); expose per-value counts so a wrong call is visible
- [ ] ingest interrupted mid-backfill → resumes, no duplicates
- [ ] retention prune runs during backfill

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/migrations/m0007_entrypoint.py (new)` | column + offset reset |
| `src/easy_personal/core/store.py` | column list |
| `src/easy_personal/sources/claude_code.py` | read `entrypoint` (metadata only) |
| `src/easy_personal/core/metrics.py` | split totals |
| `src/easy_personal/surfaces/mcp.py, cli.py` | split fields |
| `web/src/UsageView.jsx + test` | You / Agents series |
| `tests/core, tests/sources, tests/surfaces, tests/fixtures` | fixtures + tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `web/src/App.jsx` | rail is another task |
| `src/easy_personal/core/rules.py` | rules unchanged |
| `src/easy_personal/connectors/**`, `src/easy_personal/launch.py` | connectors and keychain are out of scope |
| any repo's `PROJECT_KANBAN.md` / `memory/**`, `~/.claude*/**` | read-only sources (NFR-003) |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |

---

## Test Plan

pytest: migration from m0006, idempotent re-ingest, classification table, MCP/CLI/API parity, perf with EXPLAIN; vitest: series + legend; easy-ui screenshot of Usage.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled or N/A with reason (Hard-Stop Gate 6)
- [ ] `Skill({ skill: "verify" })` run
- [ ] Supervisor notified: task ready for Stage 4 review