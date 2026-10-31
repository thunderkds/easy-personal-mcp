**Date**: 2026-10-31
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P0
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
6. Read `BRAINSTORMING_LOG_LIVE.md` and `PRD.md` US-022, FR-043, FR-002, NFR-003, NFR-011

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-31: "build them as the next versio n" (the live-trace stack, as v0.3.0); owner chose Option A (transcript tail only) and approved this breakdown ("approve").

**Restated intent**:

> Add `core/live.py`: a background tail that, while `easy-personal dashboard` runs, polls every confirmed agent account's `projects/` tree about every 100 ms, reads only complete new lines from the stored byte offsets with the existing `claude_code` parser, writes them through the normal store path taking the single-writer lock only per short batch, and keeps a bounded in-memory list of recent events (with a monotonically increasing event id) for the SSE route to read.

**Out of scope**:

* The HTTP/SSE route (next task)
* The Live view
* Any hook, OpenTelemetry or Claude config write

**Requirement Refs**: US-022, FR-043, FR-002, NFR-003, NFR-011

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

**Depends on**: HUN-175 (record types)

**Entry point**: `easy-personal dashboard` (`surfaces/cli.py` → `api.serve()`), which starts the tail

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Appending a tool_use line to a fixture transcript makes the tail store it and expose it in the recent-events buffer within 500 ms (poll 100 ms) in a test | FR-043 |
| 2 | A partial last line is never parsed; it is taken once complete | FR-043 |
| 3 | A new `subagents/` file appearing mid-session is followed | FR-043 |
| 4 | While the tail runs, `easy-personal ingest` (the scheduled ingest) completes instead of failing with LockBusy | FR-043 single-writer |
| 5 | The recent-events buffer is bounded (configurable, default 2000) and its events carry only metadata fields | FR-002 |
| 6 | The tail stops cleanly when the dashboard stops (no orphan thread) | FR-043 |
| 7 | Real run: with the dashboard up, one headless `claude -p` Haiku session's tool calls reach the buffer; ≥ 95% within 2 s of the transcript timestamp | NFR-011 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core -q -k live && uv run pytest tests/leak -q
```

## Approach

**Vital slice**: tail → store → bounded buffer, lock-friendly
**Cut list**: No inotify; no per-account tuning

Reuse `read_complete_lines`, `parse_record` and the per-file offsets in the store (`store.file_offsets`/`save_file`) so the tail and the scheduled ingest share one cursor and never double-count. One thread; scan with `os.scandir` + size/inode; sleep 100 ms.

---

## Edge Case Checklist

- [ ] Daily timer ingest and the tail run at the same moment
- [ ] File replaced (new inode) or truncated
- [ ] Two sessions, two accounts at once
- [ ] Dashboard idle for hours: memory stays bounded
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, SSE event, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/live.py` | new: tail + buffer |
| `src/easy_personal/surfaces/cli.py` | start/stop the tail with the dashboard |
| `src/easy_personal/core/store.py` | only if a helper is missing |
| `tests/core/test_live.py` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude*/` (any file), `~/.claude.json`, any `settings.json` | Read-only sources; no Claude config writes (Option A) |
| `experiments/live-trace/` | Spike evidence stays as is |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |
| Sibling repos (`../easy-*`) | Conventions only |

---

## Test Plan

Temp-dir fixture transcripts appended in chunks; assert store rows, buffer contents, latency bound, lock coexistence with a concurrent ingest call, bounded buffer, clean stop; planted-secret line never in buffer.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review