**Date**: 2026-11-03
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary; Critical Constraints: metadata only, CSP `default-src 'self'`)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` US-027, FR-023, FR-064 and `memory/decisions.md` "Cache reuse card follows the app-wide period + detail lines"

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-03: "the cache reuse I saw it always 1 number, pls check". Supervisor measured: the ratio is right but steady (\~97 %), and the rail card is fixed to the last 7 days (`web/src/Rail.jsx` `CacheReuse`). Owner chose "Follow period + detail" with this mockup:

```
┌ Cache reuse ─────────────┐
│      ◯ 97.4%             │
│  Last 30 days            │
│  Written to cache  2.1%  │
│  Lowest: haiku-5-5 82.5% │
└──────────────────────────┘
```

**Restated intent**:

> Lift the Usage period choice (`PERIODS` and `period` state in `web/src/UsageView.jsx`) into `web/src/App.jsx` as one app-wide setting (default `'7'`), passed to `UsageView` (its picker sets it) and to `Rail`. `CacheReuse` in `web/src/Rail.jsx` loads `/api/usage/summary` and `/api/usage/breakdown?by=model` for that period (same `since` rule as the Usage view; All time = no `since`) and the current scope, and shows: the ring with the cache hit ratio, the period label (e.g. "Last 30 days"), "Written to cache N%" (`cache_creation_input_tokens ÷ (input_tokens + cache_creation_input_tokens + cache_read_input_tokens)` from the summary), and "Lowest: <model> N%" (lowest `cache_read ÷ prompt tokens` among by-model groups with `messages ≥ 20`; the line is left out when fewer than 2 groups qualify). Ring label and note name the period instead of "last 7 days".

**Out of scope**:

* Any backend or API change
* Persisting the period across reloads
* Other rail cards (Next actions, Plan limits)

**Requirement Refs**: US-027, FR-064

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, owner-approved mockup, 2026-11-03)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None

**Entry point**: `easy-personal dashboard` → right rail `CacheReuse` (`web/src/Rail.jsx`) on every view; period buttons `aria-label="Period"` in `web/src/UsageView.jsx`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Picking "Last 30 days" on the Usage view changes the rail card's request to the 30-day `since` and its label to "Last 30 days"; "All time" sends no `since` | FR-064 |
| 2 | Switching to another view (e.g. Board) keeps the chosen period in the rail card; coming back to Usage shows the same period pressed | FR-064 |
| 3 | "Written to cache" shows `cache_creation ÷ prompt tokens` as a percent with one decimal (fixture: 21 / 1000 → "2.1%") | FR-064 |
| 4 | "Lowest" names the model with the lowest ratio among groups with ≥ 20 messages; a group with 19 messages is ignored; fewer than 2 qualifying groups → no "Lowest" line | FR-064 |
| 5 | No prompt tokens → ring "no data", no extra lines, no NaN/Infinity text | FR-064 |
| 6 | Existing Usage view tests still pass (period buttons, previous-period badges) | FR-023 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run && cd .. && npm --prefix web run build && uv run pytest tests/test_web_toolchain.py -q
```

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Usage with the rail at 1280×720, owner data copy, periods 7 / 30 / All | headless screenshot | card matches the mockup; label and numbers change with the period |
| Board view with the rail after picking 30 days | headless screenshot | card still says "Last 30 days" |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors / fonts | reuse `.rail-card`, `.muted`, `.rail-note`, `Ring` | no new CSS colour or token |
| Accessibility | vitest | ring `label` names the period; the two lines are plain text, readable by a screen reader |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1280×720 and 375×812 | screenshot + `scrollWidth` | no horizontal page scroll; long model names wrap or clamp inside the card |

---

## Approach

**Vital slice**: lift `period` → rail loads summary + by-model for it → two lines.
**Cut list**: no persistence, no new API field, no per-project lowest.

---

## Edge Case Checklist

- [ ] All time (no `since`) for both requests
- [ ] Scope change (workspace / account) still reloads the card
- [ ] Breakdown request fails → ring still shows from the summary; lines left out
- [ ] Model name missing or `(unknown)` group → skipped for "Lowest"
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/App.jsx` | app-wide `period` state |
| `web/src/UsageView.jsx` | use the lifted period (export `PERIODS` or move it) |
| `web/src/Rail.jsx` | `CacheReuse` follows the period + two lines |
| `web/src/*.test.jsx`, `web/src/test/fixtures.js` | tests |
| `src/easy_personal/dashboard/static/` | rebuilt page |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/`, `src/easy_personal/surfaces/` | No backend change |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Vitest with fixtures for summary + by-model (qualifying / non-qualifying groups, zero prompt tokens, failed breakdown); App-level test that the period survives a view switch; build; headless screenshots on a copy of the owner store (never the live store with a different code version).

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] Supervisor notified: task ready for Stage 4 review