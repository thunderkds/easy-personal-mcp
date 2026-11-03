**Date**: 2026-11-03
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P0
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: Abstained, Kit server, Tool error count; Critical Constraints: read-only, metadata only)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `BRAINSTORMING_LOG_KITPERF.md` (Option A) and `PRD.md` US-026, FR-002, FR-059, NFR-002, Open Question 22
7. This task changes the store schema: run `Skill({ skill: "migration-safety" })` before writing the migration

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-03 (kit performance grill, Q3 + error-data answer): a session's result quality uses its tool-error rate; the store has no tool-error signal today, so "add the error count to the store".

**Restated intent**:

> Add store migration 8 (`core/migrations/m0008_tool_errors.py`): `events.tool_error_count INTEGER` (nullable, no backfill) and reset the stored offset of every `claude_code` source file to 0, exactly like m0007, so the next normal ingest re-reads each transcript and fills the column into the rows it already has. `sources/claude_code.py` `parse_record` counts, for `user` records only, the `content` blocks with `type == "tool_result"` and `is_error is True`, and stores that integer (0 when the record has tool results and none failed; NULL when the record has no `tool_result` block or is not a `user` record). Only `type` and `is_error` of a block are read; never `content`, `tool_use_id` or any text.

**Out of scope**:

* Any reader of the new column (next task: `core/kitperf.py`, `kit_findings`)
* Changes to `core/live.py` (it keeps its own in-memory pairing)
* Kernel trace `is_error` (all 0 today; not used)
* Any page, route or MCP change

**Requirement Refs**: US-026, FR-059, FR-002, NFR-002

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved grill + Option A, 2026-11-03)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None

**Entry point**: `easy-personal ingest` → `sources/claude_code.parse_record` → `core/store.save_file` (column `tool_error_count`); migration via `core/store.migrate` (`MIGRATIONS`)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | A v7 store opened by the new code becomes v8: `events.tool_error_count` exists, every `claude_code` row in `source_files` has `byte_offset = 0`, other sources keep their offsets, and `<store>.bak-v7` exists | FR-059 |
| 2 | Running the migration twice changes nothing more (column check first; repeat offset reset only causes one harmless re-read) | FR-059 |
| 3 | After one ingest over the same transcripts, the event row count is unchanged and existing rows get `tool_error_count` filled (no duplicate rows) | FR-059 |
| 4 | A `user` record with 3 `tool_result` blocks, 2 with `is_error: true` → 2; with none failing → 0; a `user` record with only text → NULL; an `assistant` record → NULL; `is_error` `"true"` (string) or missing → not counted | FR-059 |
| 5 | Planted `INVENTED_` text inside a failing `tool_result` `content` never reaches any store row, log or exception (leak suite) | FR-002, NFR-002 |
| 6 | A transcript file deleted before the re-read keeps its rows with `tool_error_count` NULL ("unknown"), never 0 | FR-059 |
| 7 | Real run on the owner's store (copy first): migrate + ingest; paste the time taken, `SELECT COUNT(*), SUM(tool_error_count IS NOT NULL), SUM(tool_error_count) FROM events WHERE record_type='user'`, and the event count before and after | FR-059 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core/test_migrations.py tests/sources -q && uv run pytest tests/leak -q && uv run pytest -q
```

## Approach

**Vital slice**: m0008 (copy m0007's shape) → one counter in `parse_record` for `user` records → `EVENT_COLUMNS` + `save_file` update path fills the column on re-read.
**Cut list**: no separate "tool result count" column (the next task uses tool calls from `tool_names` as the denominator); no backfill command; no live-tail change.

Check how m0007 made `save_file` write `entrypoint` into existing rows on re-read and do the same for `tool_error_count`. Run the real migration on a **copy** of `~/.easy-personal/store.sqlite` (set `EASY_PERSONAL_HOME` or the store override the tests use), never on the live store.

---

## Edge Case Checklist

- [ ] `content` not a list (string user prompt) → NULL
- [ ] Block not a dict, or `is_error` not exactly `True` → not counted
- [ ] Sidechain (sub-agent) `user` records → counted the same; they keep `is_sidechain = 1`
- [ ] Huge `tool_result` content → never decoded beyond `json.loads` of the line (already done); nothing kept
- [ ] Store already at v8 → no-op
- [ ] Old binary against a v8 store → existing version guard refuses (check and test)
- [ ] Ingest lock held by the timer during the real run → wait or retry, never break the lock
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/migrations/m0008_tool_errors.py` (new) | column + offset reset |
| `src/easy_personal/core/migrations/__init__.py`, `src/easy_personal/core/store.py` | register migration, `EVENT_COLUMNS`, re-read update path |
| `src/easy_personal/sources/claude_code.py` | count failed `tool_result` blocks in `user` records |
| `tests/core/test_migrations.py`, `tests/sources/…`, `tests/leak/harness.py` | tests + planted text in a failing tool result |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/live.py` | Out of scope |
| `src/easy_personal/surfaces/`, `web/` | No reader in this task |
| `~/.easy-personal/store.sqlite` (live) | Real run on a copy only |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Fixture transcripts with user records holding 0 / 1 / several failing `tool_result` blocks, string `is_error`, text-only content and planted `INVENTED_` text; a v7 fixture store migrated to v8; ingest twice for idempotence; deleted-file case; leak suite; real run on a store copy with timings.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review