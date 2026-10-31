**Date**: 2026-10-31
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `BRAINSTORMING_LOG_LIVE.md` and `PRD.md` US-022

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-31: "build them as the next versio n" (the live-trace stack, as v0.3.0); owner chose Option A (transcript tail only) and approved this breakdown ("approve").

**Restated intent**:

> Bump the package to 0.3.0, describe the Live view in `README.md` and the site where the drift tests require it, and hand the Supervisor what `ship` needs (changed tasks, upgrade note: no config change needed).

**Out of scope**:

* New features
* Pushing or tagging (owner does it)

**Requirement Refs**: US-022

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

**Depends on**: HUN-178 (Live view)

**Entry point**: `easy-personal --version` → `0.3.0`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `easy-personal --version` prints `0.3.0` | release |
| 2 | README documents the Live view (what it shows, that it needs nothing installed, that content is never shown); `tests/test_readme.py` passes | docs |
| 3 | Site drift tests pass | site |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/test_readme.py tests/test_site_content.py tests/test_skeleton.py -q
```

## Approach

**Vital slice**: version + docs
**Cut list**: No changelog tooling

Change `pyproject.toml` version and any version constant; update docs only where tests or users need it.

---

## Edge Case Checklist

- [ ] Version shown in MCP `ping`/CLI matches
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, SSE event, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `pyproject.toml` | version |
| `README.md` | Live view section |
| `site/index.html` | if a drift test needs it |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude*/` (any file), `~/.claude.json`, any `settings.json` | Read-only sources; no Claude config writes (Option A) |
| `experiments/live-trace/` | Spike evidence stays as is |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |
| Sibling repos (`../easy-*`) | Conventions only |

---

## Test Plan

Existing README/site/skeleton drift tests, plus a version assertion if none exists.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review