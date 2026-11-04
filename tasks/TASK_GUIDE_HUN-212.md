**Date**: 2026-11-04
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P0
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Live tail, Attention state; Critical Constraints: read-only sources, metadata only, single-writer lock)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-043, FR-044, FR-047, FR-071, NFR-011, NFR-012, then `src/easy_personal/core/live.py` and `tests/core/test_live.py` (or the existing live tail tests)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-04: "in the live, seem not correct, I have 3 project are running, but just show 2".

Supervisor investigation (read-only, 2026-11-04):

* Two `easy-personal dashboard` processes ran at once (pid 2219224 from 17:36 on 127.0.0.1:43351, pid 2701946 from 17:53 on 127.0.0.1:44399); 6+ projects wrote transcripts in the last 10 minutes.
* Both live tails read from the one store cursor (`store.file_offsets`). `LiveTail._follow` publishes only lines it stored itself. When the other process moved the cursor first, `_follow` returns at `if store.file_offsets(conn, claude_code.SOURCE).get(path) != state: return True`, and on the next poll `_scan` starts at the new cursor (`offset == size`), so the lines are never published by this tail. Each tab shows only the sessions it won.
* The scheduled ingest (`easy-personal-ingest.timer`, daily 00:00) moves the same cursor, so it drops live events the same way.

**Restated intent**:

> `LiveTail` keeps its own in-memory read offset per file (`path -> (inode, offset)`). On first sight of a file the offset is the store cursor (no history replay, as today). Each poll reads complete lines from the tail's own offset. Lines below the store cursor (another process stored them) are parsed and published, not stored again. Lines from the store cursor on are stored through `store.save_file` under the per-batch lock, then published, as today. A changed inode or a file smaller than the tail's offset restarts that file at the store cursor rule. Result: every dashboard shows every session, and the store holds each line once.

**Out of scope**:

* Stopping a second dashboard from starting, or any single-instance lock
* Any change to `store.save_file`, the scheduled ingest, migrations, `GET /api/live` or `web/`
* Attention rules (FR-047..FR-050) beyond keeping them correct with the new read path

**Requirement Refs**: FR-071 (FR-043, FR-044 kept)

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, owner said "yes" to a fix task, 2026-11-04)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None

**Entry point**: `LiveTail.poll_once` → `_scan` / `_follow` in `src/easy_personal/core/live.py`, started by `easy-personal dashboard` and read by `GET /api/live`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Two `LiveTail` instances on one store, polled in turn while 3 sessions in 3 projects write lines: each tail's `snapshot()` lists all 3 sessions and its buffer holds every tool event of every session | FR-071 |
| 2 | A scheduled-ingest-style write (`store.save_file` by another caller) between two polls: the tail still publishes those lines' events | FR-071 |
| 3 | The `events` row count after AC 1 and AC 2 equals the count with a single writer (no line stored twice) | FR-071 |
| 4 | A tail started on a store whose cursor is at the end of a file publishes nothing from that file until a new line is written (no history replay) | FR-071, FR-043 |
| 5 | File replaced (new inode) or truncated: the tail follows the new content without an exception and without publishing old lines twice | FR-071 |
| 6 | Existing live tail and SSE tests stay green (attention states, lock busy, rolled-past snapshot) | FR-043, FR-044 |
| 7 | Real run: start two `easy-personal dashboard` processes, run 3 Claude Code sessions in 3 repos; both `/api/live` streams list all 3 sessions (paste the session lists from each) | FR-071 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests -q -k live && uv run pytest -q
```

## Approach

**Vital slice**: one per-file offset map in `LiveTail`, a split in `_follow` between "publish only" lines (below the store cursor) and "store + publish" lines; tests for AC 1–5.
**Cut list**: no single-instance lock; no new store API unless `save_file` cannot take a batch that starts at the store cursor; no change to the SSE route.

Keep the lock rule: take the ingest lock only for the batch that stores lines, never for publish-only lines. Keep the batch bound `MAX_BATCH_LINES`. Memory stays bounded: drop a file's offset when the file is gone.

---

## Edge Case Checklist

- [ ] Lock busy while a batch needs storing → publish nothing from that batch, retry next poll (no double publish)
- [ ] Store cursor moves backwards (store reset / re-read migration) → tail does not replay history it already published
- [ ] A partial last line → never read until complete (as today)
- [ ] Subagent files (`subagents/*.jsonl`) handled the same way
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log (leak suite stays green)

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/live.py` | own offsets, publish-only lines |
| `tests/core/test_live.py` (existing live tail test file) | AC 1–5 |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/store.py`, `src/easy_personal/core/migrations/` | Out of scope (ask the Supervisor first if `save_file` truly cannot be used) |
| `src/easy_personal/surfaces/`, `web/` | Out of scope |
| `~/.easy-personal/store.sqlite` (live) | Real run uses a copy or the test store override |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Fixture account tree with 3 sessions in 3 projects; two `LiveTail` objects with a fake clock and `poll_once(conn)`; interleave appends, polls and a direct `store.save_file`; assert snapshots, buffers and `events` row count; inode change and truncate cases; real run with two dashboards.

---

## UI / Design Evidence

☐ N/A — backend only.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review