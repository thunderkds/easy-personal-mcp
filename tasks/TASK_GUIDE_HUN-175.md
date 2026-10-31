**Date**: 2026-10-31
**Complexity Level**: C1
**Risk Level**: Medium
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
6. Read `BRAINSTORMING_LOG_LIVE.md` and `PRD.md` FR-046, FR-002, FR-003

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-31: "build them as the next versio n" (the live-trace stack, as v0.3.0); owner chose Option A (transcript tail only) and approved this breakdown ("approve").

**Restated intent**:

> Add `api-request`, `api-request-blob`, `api-request-shape`, `tools-used`, `ai-title`, `mode`, `permission-mode` and `bridge-session` to the `claude_code` source's known metadata types, keeping only type, session and time, so a 2.1.316 transcript no longer counts them as abstained; no new field is read from them.

**Out of scope**:

* Reading any new field from these records (e.g. `ai-title` text, `tools-used` lists)
* The live tail (next task)

**Requirement Refs**: FR-046, FR-002, FR-003

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

**Depends on**: None

**Entry point**: `easy_personal.sources.claude_code.parse_record` (called by `ingest`)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | A synthetic 2.1.316 fixture with one line of each of the 8 types ingests with 0 abstained records for those types | FR-046 |
| 2 | Each such record stores only type, session id and time; title/text-like fields never reach the store (planted values absent) | FR-002 |
| 3 | A still-unknown type is abstained and counted as before | FR-003 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/sources -q && uv run pytest tests/leak -q
```

## Approach

**Vital slice**: the 8 types recognised, nothing new read
**Cut list**: No new metrics from these records

Extend `METADATA_TYPES` in `src/easy_personal/sources/claude_code.py`; add fixtures under `tests/fixtures/claude_code/2.1.316/` with invented values only (HUN-32 rules); extend the leak canaries with an `ai-title` planted string.

---

## Edge Case Checklist

- [ ] `ai-title` holds a generated title (content): never stored
- [ ] Records without `version`/`cwd`
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, SSE event, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/sources/claude_code.py` | METADATA_TYPES |
| `tests/fixtures/claude_code/2.1.316/*.jsonl` | fixtures |
| `tests/sources/` | tests |
| `tests/leak/canaries.py` | planted title |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude*/` (any file), `~/.claude.json`, any `settings.json` | Read-only sources; no Claude config writes (Option A) |
| `experiments/live-trace/` | Spike evidence stays as is |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |
| Sibling repos (`../easy-*`) | Conventions only |

---

## Test Plan

Fixture ingest test (0 abstained for the 8 types, 1 abstained for an invented unknown type) + leak canary for `ai-title`.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review