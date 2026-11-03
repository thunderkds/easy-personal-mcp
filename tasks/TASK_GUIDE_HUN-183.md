**Date**: 2026-11-03
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

Owner request 2026-11-02: live attention states as the next version. **Widened 2026-11-03 (owner): one release 0.4.0** from `main` that carries everything merged since 0.3.0 (`1c80450`): live attention (HUN-181..182), the easy-memory layer (HUN-184..185), the dashboard UX pass (HUN-186..191) and kit performance (HUN-192, HUN-193, HUN-195). HUN-194 (a separate 0.5.0) is closed as a duplicate of this task.

**Restated intent**:

> Bump the package to 0.4.0. In `README.md`: (1) Live view — the three attention states, the two settings (`live_failing_streak`, `live_open_call_minutes` under `thresholds:`) and the accepted limit (a call opened before `easy-personal dashboard` started is not seen as open, PRD OQ #17); (2) Memory view — the easy-memory layer (cited-file nodes, stale mark, 30-day panel) if not documented yet; (3) a new "Kit performance" section — what `kit_findings` / `easy-personal kit-findings` return (`findings` with a habit, `other`, `not_enough`), "linked to, not caused", ≥ 10 pooled sessions per side, a gap of at least 5 % for a habit, the good-result rule, the `kit_error_rate` setting under `thresholds:` (default 0.04), and that `wake` reads findings only after XR-2 ships in the kit kernel; replace the placeholder sentence HUN-193 added; (4) "Upgrade and rollback" — store migration 7 → 8 (`m0008_tool_errors`), backup `store.sqlite.bak-v7`, the next ingest re-reads transcripts (about 20–45 s on the owner's store), error counts stay unknown for deleted transcripts; rollback = restore `.bak-v7` with the 0.3.0 binary. Update the site where the drift tests require it (version pill, tool count). Hand the Supervisor what `ship` needs: the changed tasks and the migration + rollback note.

**Out of scope**:

* New features
* Pushing, tagging or reinstalling the tool (owner does it; reinstall migrates the live store)
* The kernel `wake` change (XR-2)

**Requirement Refs**: US-023, FR-049, US-024, US-025, US-026, FR-059, FR-062, FR-063

### Shared rules (v0.4.0)

* No code change under `src/` except the version string. Never use `git stash` (shared across worktrees); refresh `.claude/hooks/.state/active_task` right before every full test run.
* README wording follows ASD-STE100: one term per thing (*attention state*, *open call*, *kit server*, *good result*, *kit finding*, *habit*).

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved grill + Option C, 2026-11-02)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: HUN-195

**Entry point**: `easy-personal --version` → `0.4.0`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `easy-personal --version` prints `0.4.0` | release |
| 2 | README Live view documents the three states, both settings with defaults, and the start limit | US-023, FR-049 |
| 3 | README "Kit performance" documents the tool, the CLI command, `findings` / `other` / `not_enough`, the 10-session and 5 % rules, the good-result rule, `kit_error_rate` with default 0.04, and the XR-2 limit | US-026, FR-062, FR-063 |
| 4 | README "Upgrade and rollback" has the 7 → 8 migration note, the `.bak-v7` backup and the rollback steps | FR-059 |
| 5 | `tests/test_readme.py`, the site drift tests and `tests/test_skeleton.py` pass; full suite green | release |

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
| `README.md` | Live view, Memory view, Kit performance, Upgrade and rollback |
| `RUNBOOK.md` | not touched: `ship` writes the v0.4.0 entry |
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