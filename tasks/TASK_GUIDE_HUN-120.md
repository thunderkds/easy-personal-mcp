**Date**: 2026-10-29
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: *Workspace*, *Agent account*, *API-equivalent cost*)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Verify on fixtures only: temp `EASY_PERSONAL_HOME` + `EASY_PERSONAL_CLAUDE_PROJECTS` + `EASY_PERSONAL_AGENT_ROOT`; never open the real `~/.easy-personal/` or `~/.claude*/`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner issue 2026-10-29 #7: "the warning and improvement plan is useless too, let make the information good to see, easy to follow".

Today (`web/src/WarningsPlanView.jsx`): each item shows the raw `rule_id` (e.g. `cache_hit_low`), "Measured 0.31 (threshold 0.5)", a fix sentence and a `key: value` detail dump. Rules that could not be measured are listed as `rule_id: reason` in machine words. The plan repeats the warnings grouped as "Priority 1/2/3". On the owner's machine everything abstains because accounts are ignored, so the page shows only the abstained list.

**Restated intent**:

> Every rule in `rules.yaml` gains a plain `title` (e.g. "Low cache reuse"), a one-sentence `why` (why it matters), and its `fix` becomes 1–3 numbered `steps`. The API returns them on hits, plan items and abstained entries; measured values come with a human `summary` (e.g. "31% of prompt tokens came from cache; the target is 50% or more, last 7 days"). The page becomes one action list: a top line "2 things to fix, 1 to watch" (or "All clear — nothing to fix in the last 7 days"), then one card per hit, ordered by priority, with a chip ("Fix now" for warning, "Improve" for improve, "Watch" for aware), the title, the summary, "Why it matters", and the numbered steps. Rules that could not be measured collapse into "Not checked (5)" with plain reasons and, when the reason is "no data", a hint pointing to Agents ("Confirm the accounts you use"). The separate "Improvement plan" section is merged into the ordered list (no duplicate cards); the plan API stays for MCP.

**Out of scope**: new rules; threshold changes; the MCP output shape beyond added keys.

**Requirement Refs**: owner issue #7; FR-006, FR-026.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's issue (Supervisor, 2026-10-29)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: the scope task (edits `rules.py`) and the global selector task (edits `WarningsPlanView.jsx`). **Entry point**: dashboard → Warnings & plan; MCP `warnings`, `improvement_plan`.

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | All 7 rules in `rules.yaml` have `title`, `why`, `steps` (1–3); `load_rules` refuses a rule without them; owner `thresholds:` overrides still work | plain words |
| 2 | Hits, plan items and abstained entries carry `title`; hits/plan carry `why`, `steps`, `summary`; existing keys unchanged (additive) | API shape |
| 3 | `summary` per metric kind: ratios as %, token counts with thousands separators, the window ("last 7 days") and the target ("target 50% or more") | human numbers |
| 4 | Page header line counts by chip; "All clear…" when no hits | easy to follow |
| 5 | One card per hit, priority order; chip text Fix now / Improve / Watch; title, summary, Why it matters, numbered steps; no raw `rule_id` visible (it stays in a `data-rule` attribute) | good to see |
| 6 | "Not checked (N)" is a `<details>` closed by default; reasons in plain words; "no data"-type reasons add the Agents hint | abstained clear |
| 7 | No separate Improvement plan section on the page | no duplicate |
| 8 | MCP `warnings` / `improvement_plan` outputs only gain keys; leak suite still passes | compatibility |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | rule without `title` in a test pack | `load_rules` raises | pytest |
| 2 | `cache_hit_low` measured 0.31 | summary "31% of prompt tokens came from cache; target 50% or more, last 7 days" | pytest |
| 3 | 1 warning + 1 aware hit | header "1 thing to fix, 1 to watch"; 2 cards | Vitest |
| 4 | 0 hits, 5 abstained | "All clear" + closed "Not checked (5)" | Vitest |
| 5 | page text | contains no `_`-joined rule id | Vitest |

### Verification Command (exact, runnable)

```bash
npm --prefix web test && npm --prefix web run build && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-120.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-120.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Warnings with hits; all clear; all abstained | headless Chrome + LLM vision | reader knows what to do first in under 10 s |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors from tokens | grep | none outside `tokens.css` |
| Contrast | computed | ≥ 4.5:1 |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 375 / 768 / 1280 px | screenshot | cards stack, steps readable, no horizontal scroll |

---

## Approach

**Pattern reference**: `src/easy_personal/data/rules.yaml` flat `key=value … fix=` format; `rules._parse_rule`.
**Vital slice**: extend the rule line with `title=` / `why=` / `steps=` (pipe-separated), keep `fix` as the joined steps for old readers; one `summary()` per metric kind.
**Cut list**: per-rule "learn more" pages; dismiss/snooze.

---

## Edge Case Checklist

- [ ] title/why text containing `=` → parser keeps it (parse from the known keys, like `fix=` today)
- [ ] owner config with old-format rules overrides → still loads
- [ ] `detail` lists (e.g. stale projects) shown as a plain list under the steps

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/data/rules.yaml` | title/why/steps |
| `src/easy_personal/core/rules.py`, `plan.py` + tests | parse + summary |
| `web/src/WarningsPlanView.jsx` + `.test.jsx` | action list |
| `web/src/app.css` | cards |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| thresholds in `rules.yaml` | out of scope |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |

---

## Test Plan

pytest for AC 1–3, 8; Vitest for AC 4–7; screenshots.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] `Skill({ skill: "verify" })` run on fixtures
- [ ] Supervisor notified: task ready for Stage 4 review