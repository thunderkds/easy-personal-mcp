**Date**: 2026-11-02
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P0
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: Live tail, Stream, Open call, Attention state)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `BRAINSTORMING_LOG_ATTENTION.md` (Option C, incl. the 2026-11-02 performance revision) and `PRD.md` US-023, FR-002, FR-044, FR-047, FR-048, FR-049, NFR-002, NFR-012

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-11-02: live attention states; owner chose Option C and the "badge in card head" mockup, then "let go".

**Restated intent**:

> In `web/src/LiveView.jsx`, copy each session's `attention` list from the snapshot and render it: a pill in the card head (`failing: Edit ×3 · agent`, `waiting for you: question`, `needs attention: Bash · hung or waiting for permission`), a card border in the state's token colour, cards with a state first in their project, and a header count "N sessions need you". A session with a state is never shown as idle. The page computes no state itself.

Owner-selected mockup (2026-11-02):

```
Live          2 sessions need you

┌─ 81475b70 ── [failing: Edit ×3 · agent] ─ active · 12s ┐
│ Tool calls 41  Tokens in 1.2M  Cost $3.10              │
│ 14:02:11  Edit   agent error                           │
└────────────────────────────────────────────────────────┘
┌─ 4fd3699d ── [waiting for you: question] ─ active · 3m ┐
```

**Out of scope**:

* Any state logic (it is in the live tail, previous task)
* Browser notifications, MCP, CLI (cut list)

**Requirement Refs**: US-023, FR-045, FR-050

### Shared rules (v0.4.0 live attention)

* Direction: `BRAINSTORMING_LOG_ATTENTION.md` Option C and `memory/decisions.md` "v0.4.0 live attention states = snapshot-pushed state (Option C)". No hooks, no OpenTelemetry, no Claude config write, no new thread, loop or route, no new dependency, nothing new stored (no migration).
* Metadata only (FR-002, NFR-002): from content blocks read only `type`, `id`, `tool_use_id`, `name`, `is_error`. Never `input`, `content`, `text`. Block ids stay in memory: never in the store, the SSE payload, the page or a log. Extend `tests/leak/test_live_leak.py`.
* Only an attention-state change bumps `state_version`; an ordinary tool call (open → success) never does.
* Never use `git stash` (shared across worktrees); refresh `.claude/hooks/.state/active_task` right before every full test run.
* The page renders `attention` only; it never derives a state from the feed or the clock.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved grill + Option C, 2026-11-02)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: HUN-181 (live tail attention states + stream re-send)

**Entry point**: `easy-personal dashboard` → rail `// work` › Live

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | A snapshot session with `attention: [{state: "failing", tool: "Edit", count: 3, is_sidechain: true}]` renders the pill `failing: Edit ×3 · agent` in the card head | FR-050 |
| 2 | `waiting` renders `waiting for you: question`; `needs_attention` renders `needs attention: <tool> · hung or waiting for permission`; `tool: null` renders "unknown tool" | FR-050 |
| 3 | A re-sent snapshot without the state removes the pill and border; feeds are kept | FR-050 |
| 4 | The header shows "N sessions need you" (N = sessions with any state) in a `role="status"` region; hidden when N = 0 | FR-050 |
| 5 | Within a project, cards with a state sort before cards without; ties keep newest first | FR-050 |
| 6 | A session with a state never shows "idle" | FR-044, FR-050 |
| 7 | A failed result row in the feed shows its tool name (`Edit error`), not `tool_result` | FR-045 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run LiveView neon && cd .. && uv run pytest tests/test_site_content.py -q
```

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Live at 1280×720 with one card per state + one without | headless screenshot (learnings: `scratchpad/hun178` UI verify pattern) on a fake stream | matches the owner mockup; same card style as v0.3.0 |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors from tokens | `web/src/neon.test.jsx`-style test | pill and border use only `var(--…)` from `web/src/tokens.css`; state never shown by colour alone (text label always present) |
| Fonts / labels | computed style | Plex Mono for the pill, as other labels |
| Accessibility | vitest | header count in `role="status"`; pill text readable by screen readers; contrast ≥ 4.5:1 in both themes |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1280×720 | screenshot | as above |
| narrow (rail collapsed) | screenshot | pill wraps under the session id, no horizontal page scroll |

---

## Approach

**Vital slice**: `fromSnapshot` copies `attention` → pill + border + sort + header count.
**Cut list**: no filter "only sessions that need me"; no sound or notification.

---

## Edge Case Checklist

- [ ] Several states on one session (e.g. failing + waiting) → one pill per state, failing first
- [ ] Snapshot re-sent while events stream → no duplicate feed rows (existing `restarted` check)
- [ ] Long tool names (`mcp__claude_ai_Linear__save_issue`) → pill truncates with title attribute
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, SSE event, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/LiveView.jsx` | copy `attention`, pill, sort, header count, idle rule |
| `web/src/app.css` | pill + border styles from tokens |
| `web/src/LiveView.test.jsx` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/` | Backend is the previous task |
| `web/src/tokens.css` | Use existing tokens; ask before adding one |
| `web/package.json`, `web/package-lock.json` | No new dependency |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Vitest with a fake `EventSource` sending snapshots with and without `attention`; assert pill text, border attribute, order, header count, idle label; screenshot script for the three UI Evidence rows.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] All three UI Evidence rows filled with pasted evidence (Hard-Stop Gate 6)
- [ ] Supervisor notified: task ready for Stage 4 review