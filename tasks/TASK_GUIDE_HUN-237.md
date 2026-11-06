**Date**: 2026-11-06
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Live tail, open call, store cursor; Critical Constraints: metadata only, read-only sources)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-043, FR-044, FR-047, FR-049, FR-071, FR-081..FR-083, FR-086; then `src/easy_personal/core/live.py` in full, `tests/core/test_live.py` (fixtures and FR-071 tests), `tests/surfaces/test_api_live.py`, `tests/leak/test_live_leak.py`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-06 (after HUN-236 merged and a dashboard restart at 21:37): "can not see the active session". Then: "yes, start HUN-237 \[auto\]".

Supervisor diagnosis (2026-11-06, reproduced on a temp store with the owner's `.claude-hung2` account, metadata only): `LiveTail` starts every file of its first scan at the store cursor (`_scan`: `own = (inode, _cursor(...))`, [live.py](<http://live.py>) \~416–423; FR-071 "no history replay"). `_sessions` is filled only by `_publish` of lines read after start. So after a restart a session that has not written a new line yet is not in `snapshot()`; once it writes, it shows with totals counted only since the restart (seen: `a2420a60` listed with 1 tool call, 320 tokens, real session hundreds of calls) and no kit stage. The server tail itself works (store WAL written within 50 ms of each transcript line).

**Restated intent**:

> Warm start (FR-086). On the tail's first scan only (`self._primed` is False), for each `.jsonl` file whose mtime is within `FORGET_AFTER_S` of the tail clock, replay its complete lines from byte `max(0, size_at_scan - WARM_MAX_BYTES)` (advanced to the next line start when not 0) up to the file's store cursor into session state — the same `parse_record` + `_item` + `_publish` logic (session totals and message dedupe, project, account label, `_set_stage`, `_pair` open calls and streaks, `_close_on_new_message`, `_agent_of` meta read) — but append **no** event to `_buffer` (no ids consumed, `last_id` unchanged by the replay) and store nothing (those lines are below the cursor). Lines from the cursor on follow today's path unchanged (stored + published). Then `_evaluate()` once. Files older than the window, and every file first seen after the first scan, behave exactly as today. Result: right after start, `snapshot()` lists every session that wrote within `ACTIVE_WINDOW_S` or has an open call, with whole-session totals (capped by `WARM_MAX_BYTES` for huge files) and its stage; `state_version` reflects them. The page needs no change.

**Out of scope**:

* Page changes; feed history (the feed still shows only events received while the page is open)
* Any store schema change; storing labels/stage (still never stored, DDR-0004)
* Seeding from the store's events table (it has no block ids, labels or stage: cannot rebuild FR-047/FR-083 state)

**Requirement Refs**: FR-086 (amends FR-071, FR-083 for state only; FR-043, FR-044, FR-047, FR-049, FR-082 kept)

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (auto (Supervisor), 2026-11-06 — owner said "yes" to "load the sessions that wrote in the last 5 minutes, with their full totals" on start; replay-not-store-seed choice flagged in the review file)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None (HUN-233..HUN-236 merged)

**Entry point**: `LiveTail._scan` (first scan) in `src/easy_personal/core/live.py`, started by `easy-personal dashboard` (`surfaces/cli.py`) and read by `GET /api/live`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | A transcript fully stored before the tail starts (cursor = file end), last written 1 min ago (tail clock), with N tool calls and usage: after the first `poll_once`, `snapshot()["sessions"]` lists it with `tool_calls == N` and the whole file's tokens/cost — no new line needed | FR-086 |
| 2 | Replay appends nothing to the buffer: `events_since(0) == []` and `last_id == 0` after a first poll over stored-only files; a line appended afterwards is published with id 1 as today | FR-086, FR-044 |
| 3 | Replay stores nothing: the store event count and `file_offsets` are identical before and after the first poll for already-stored files; a file partly stored (cursor mid-file) replays \[0, cursor) into state and stores + publishes \[cursor, end) exactly once (store count = one-writer count) | FR-086, FR-071, FR-043 |
| 4 | State rebuilt: a replayed `Skill` `code-review` call gives `stage "4"`/`stage_skill "code-review"`; a replayed unanswered `AskUserQuestion` gives a `waiting` attention state (session listed even if its last line is 20 min old, FR-044); a replayed sub-agent stream gets its `agentType`; a tool_use answered later in the file is not open | FR-086, FR-083, FR-047, FR-049, FR-082 |
| 5 | Window: a file with mtime older than `FORGET_AFTER_S` is not replayed (its session is absent until it writes; then totals count from that line, as today); a file first seen after the first scan behaves as today (read from 0, stored, published) | FR-086, FR-071 |
| 6 | Byte cap: a file larger than `WARM_MAX_BYTES` (monkeypatch small) is replayed from its last cap bytes, starting at the first line start (no half line parsed; no error) | FR-086 |
| 7 | Leak: canaries in message text, tool input and output of replayed lines appear in no snapshot, event or log (extend `tests/leak/test_live_leak.py`) | FR-002, PROJECT_SPEC.md:118 |
| 8 | Existing live / API / leak tests pass unchanged except those that assert "no history replay" for state, which are adapted (name each in the report); full `uv run pytest -q` passes | FR-071 |
| 9 | Real run: temp homes; write a transcript (Skill code-review + Bash + usage), run `easy-personal ingest`, then start `easy-personal dashboard`; `/api/live` first snapshot lists the session with full `tool_calls` and `stage "4"`, before any new line; then append a line and see it as event id 1. Measure first-scan time on a \~8 MB synthetic transcript and paste it | FR-086 |

---

## UI / Design Acceptance Criteria

N/A — no page change (the page renders the snapshot it already receives). UI Evidence rows ☐ N/A.

---

## Evaluation & Acceptance

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core/test_live.py tests/surfaces/test_api_live.py tests/leak -q && uv run pytest -q
```

## Approach

**Vital slice**: in `_scan`, when not `self._primed` and the file is new to `_offsets` and its mtime is in the window, call a `_warm(conn, account_id, label, path, inode, size, cursor)` that reads lines from the capped start to `cursor` in batches (`claude_code.read_complete_lines`), builds items with `_item`, and applies them through `_publish` with a flag that skips `_append` (or split `_publish` into state update + append). `_jsonl_files` must give mtime (or `os.stat` once per candidate). Add `WARM_MAX_BYTES` (suggest 64 MiB) next to the other constants. Update the module docstring, FR-071 comment near `_scan`, and the `_cursor`/first-scan comment.
**Cut list**: no store reads for seeding; no progress UI; no config option for the window or cap.

---

## Edge Case Checklist

- [ ] `_primed` stays False when the first scan returns early (lock busy / stop): the next poll must not replay a file twice (replay only when the path is not yet in `_offsets`)
- [ ] Message dedupe: a message id seen in replay and again after the cursor is not double-counted (existing `session.messages` dict)
- [ ] A truncated/replaced file during replay (`FileNotFoundError`, size shrink) is skipped, as in `_scan`
- [ ] `_evaluate` after replay drops sessions older than `FORGET_AFTER_S` and expired open calls
- [ ] Meta file read for replayed sub-agent streams follows `META_WAIT_S` rules (use the tail clock, not wall time, in tests)

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/live.py` | warm start on first scan; constant; docstrings |
| `tests/core/test_live.py` | AC 1–6, 8 |
| `tests/leak/test_live_leak.py` | AC 7 |
| `tests/surfaces/test_api_live.py` | only if a test asserts the old first snapshot |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/store.py`, `src/easy_personal/core/migrations/` | No store change |
| `src/easy_personal/sources/claude_code.py` | Parser unchanged |
| `web/` | No page change |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

## Questions Log

* 2026-11-06 DEBATE Warm-start replay window and size: files modified within FORGET_AFTER_S (1 h), last 64 MiB each, vs ACTIVE_WINDOW_S (5 min), whole file → 1 h window + `WARM_MAX_BYTES = 64 MiB` (Advocate 1: 1 h + cap — an open-call session quiet > 5 min must be listed, [PRD.md:132](<http://PRD.md:132>); Advocate 2: 5 min, whole file — exact totals, smallest change; conceded the open-call gap) ⚠ decided for you — please review