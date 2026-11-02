**Date**: 2026-11-02
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

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

Owner request 2026-11-02: live attention states as the next version; owner chose Option C and said "let go".

**Restated intent**:

> Bump the package to 0.4.0. In `README.md` Live view: describe the three attention states, the two settings (`live_failing_streak`, `live_open_call_minutes` under `thresholds:`), and the accepted limit (a call opened before `easy-personal dashboard` started is not seen as open, PRD OQ #17). Update the site where the drift tests require it, and hand the Supervisor what `ship` needs (changed tasks; upgrade note: no migration, no config change needed).

**Out of scope**:

* New features
* Pushing or tagging (owner does it)

**Requirement Refs**: US-023, FR-049

### Shared rules (v0.4.0 live attention)

* Direction: `BRAINSTORMING_LOG_ATTENTION.md` Option C and `memory/decisions.md` "v0.4.0 live attention states = snapshot-pushed state (Option C)". No hooks, no OpenTelemetry, no Claude config write, no new thread, loop or route, no new dependency, nothing new stored (no migration).
* Metadata only (FR-002, NFR-002): from content blocks read only `type`, `id`, `tool_use_id`, `name`, `is_error`. Never `input`, `content`, `text`. Block ids stay in memory: never in the store, the SSE payload, the page or a log. Extend `tests/leak/test_live_leak.py`.
* Only an attention-state change bumps `state_version`; an ordinary tool call (open → success) never does.
* Never use `git stash` (shared across worktrees); refresh `.claude/hooks/.state/active_task` right before every full test run.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved grill + Option C, 2026-11-02)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: HUN-182 (Live view badge)

**Entry point**: `easy-personal --version` → `0.4.0`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `easy-personal --version` prints `0.4.0` | release |
| 2 | README Live view documents the three states, both settings with defaults, and the start limit; `tests/test_readme.py` passes | US-023, FR-049 |
| 3 | Site drift tests pass | site |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/test_readme.py tests/test_site_content.py tests/test_skeleton.py -q
```

## Approach

**Vital slice**: version + README section + site pill.
**Cut list**: no changelog file beyond what `ship` writes.

---

## Edge Case Checklist

- [ ] README wording follows ASD-STE100 (short sentences, one term per thing: *attention state*, *open call*, *stream*)
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, SSE event, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `pyproject.toml`, `src/easy_personal/__init__.py` | version 0.4.0 |
| `README.md` | Live view: attention states, settings, limit |
| site files named by `tests/test_site_content.py` | version pill |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/`, `surfaces/`, `web/src/` | No feature change |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Existing README, site and skeleton tests; `easy-personal --version` from a fresh `uv tool install`.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Supervisor notified: task ready for Stage 4 review