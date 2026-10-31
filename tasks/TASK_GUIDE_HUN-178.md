**Date**: 2026-10-31
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P0
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `BRAINSTORMING_LOG_LIVE.md` and `PRD.md` US-022, FR-045, NFR-002, NFR-011

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-31: "build them as the next versio n" (the live-trace stack, as v0.3.0); owner chose Option A (transcript tail only) and approved this breakdown ("approve").

**Restated intent**:

> Add a Live view under `// work` that opens an `EventSource` on `GET /api/live`, shows active sessions grouped by account and project, a per-session feed of tool calls (newest first, bounded), and a running token and API-equivalent cost total per session and overall, labelled like the Usage view's cost; it shows no message or tool content.

**Out of scope**:

* Any backend change beyond what the route already offers
* Charts

**Requirement Refs**: US-022, FR-045, NFR-002, NFR-011

### Shared rules (v0.3.0 live view)

* Direction: `BRAINSTORMING_LOG_LIVE.md` Option A (owner-selected 2026-10-31) and `memory/decisions.md` "v0.3.0 live view = transcript tail only". Transcripts only: no hooks, no OpenTelemetry, no write to any Claude config (`settings.json`, `~/.claude.json`).
* Metadata only (FR-002, NFR-002): message text, tool input and tool output never reach the store, the SSE stream, the page or a log. The leak suite (`tests/leak/`) must cover every new route.
* 127.0.0.1 only (NFR-001); source files read-only (NFR-003); determinism (NFR-004); migrations only through the ordered migration path if a schema change is unavoidable (NFR-009, `migration-safety`).
* Spike evidence to reuse (do not import from `experiments/`): `experiments/live-trace/e1/tail.py` and `e1/RESULT.md` (poll 100 ms, \~2–3 ms per scan over \~410 files, p95 latency 1.35–1.93 s caused by Claude Code's own write delay), `REPORT.md`.
* Never use `git stash` (shared across worktrees); refresh `.claude/hooks/.state/active_task` right before every full test run.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-31)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: HUN-177 (GET /api/live)

**Entry point**: Rail entry `{ id: 'live', label: 'Live', View: LiveView }` in `web/src/App.jsx`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | With a mocked stream, new events appear in the right session's feed, newest first, capped at the configured length | FR-045 |
| 2 | Totals per session and overall update from token/cost fields; cost labelled API-equivalent | FR-045 |
| 3 | Stream drop shows a 'reconnecting' state and resumes (EventSource auto-reconnect) | FR-044 |
| 4 | The `views` stat in `site/index.html` is bumped and `tests/test_site_content.py` passes | site drift test |
| 5 | Real run: dashboard up + one headless Haiku session → its tool calls show on the page | NFR-011 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run LiveView && cd .. && uv run pytest tests/test_site_content.py -q
```

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Live at 1280×720 | headless screenshot (learnings: `scratchpad/shoot.mjs` pattern) on a fake stream | session list + feed + totals, same card style as Usage/Memory |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors from tokens | `web/src/neon.test.jsx`-style test | only `var(--…)` from `web/src/tokens.css` |
| Fonts / labels | computed style | Plex Sans body, Plex Mono labels |
| Accessibility | vitest | landmarks, live region for new feed items (`aria-live="polite"`), keyboard focus visible, contrast ≥ 4.5:1 |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1280×720 | screenshot | as above |
| narrow (rail collapsed) | screenshot | no horizontal page scroll; long lists scroll inside their box |

---

## Approach

**Vital slice**: session list + feed + totals
**Cut list**: No filters, no history scroll-back beyond the buffer

Follow the existing view pattern (`MemoryView.jsx`, `JobsView.jsx`); tokens from `web/src/tokens.css`; reuse the Usage view's cost formatter.

---

## Edge Case Checklist

- [ ] No active sessions: empty state
- [ ] 50+ sessions: list scrolls inside its box
- [ ] Long project names truncate
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, SSE event, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/LiveView.jsx` | new view |
| `web/src/LiveView.test.jsx` | tests |
| `web/src/App.jsx` | rail entry |
| `web/src/App.test.jsx` | views list |
| `site/index.html` | views stat |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude*/` (any file), `~/.claude.json`, any `settings.json` | Read-only sources; no Claude config writes (Option A) |
| `experiments/live-trace/` | Spike evidence stays as is |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |
| Sibling repos (`../easy-*`) | Conventions only |

---

## Test Plan

vitest with a fake EventSource: event routing, ordering, cap, totals, reconnect state; site stat test.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review