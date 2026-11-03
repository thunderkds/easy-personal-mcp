**Date**: 2026-11-03
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: Kit server, Good result, Kit finding, Habit)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `BRAINSTORMING_LOG_KITPERF.md` and `PRD.md` US-026, FR-059..FR-063, NFR-014, XR-2

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-03: kit performance is the main goal; "let go" on the Stage 2 plan.

**Restated intent**:

> Bump the package to 0.5.0 (after 0.4.0 from HUN-183). In `README.md` add a "Kit performance" section: what `kit_findings` / `easy-personal kit-findings` return, the "linked to, not caused" rule, the ≥ 10 pooled sessions rule, the good-result rule, the `kit_error_rate` setting under `thresholds:` with its default, and the upgrade note (store migration 7 → 8, backup `store.sqlite.bak-v7`, the next ingest re-reads transcripts; error counts stay unknown for deleted transcripts). State that `wake` reads findings only after XR-2 ships in the kit kernel. Update the site where drift tests require it, and hand the Supervisor what `ship` needs (changed tasks; migration + rollback note: restore `.bak-v7` with the 0.4.0 binary).

**Out of scope**:

* New features
* Pushing or tagging (owner does it)
* The kernel `wake` change (XR-2)

**Requirement Refs**: US-026, FR-059, FR-063

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, 2026-11-03)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: HUN-193

**Entry point**: `easy-personal --version` → `0.5.0`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `easy-personal --version` prints `0.5.0` | release |
| 2 | README "Kit performance" documents the tool, the CLI command, the three rules, the setting with its default, the migration note and the XR-2 limit; `tests/test_readme.py` passes | US-026, FR-059, FR-063 |
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

- [ ] README wording follows ASD-STE100 (one term per thing: *kit server*, *good result*, *kit finding*, *habit*)
- [ ] If HUN-183 is not merged yet, stop and report `Status: blocked` (versions must go 0.4.0 → 0.5.0)
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `pyproject.toml`, `src/easy_personal/__init__.py` | version 0.5.0 |
| `README.md` | Kit performance section + upgrade note |
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