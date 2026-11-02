**Date**: 2026-11-02
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P0
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

Owner request 2026-11-02: investigate [github.com/agentblazo/blazo](<http://github.com/agentblazo/blazo>) for ideas → live per-session attention states; owner locked the requirements in a grill, chose Option C, said "let go".

**Restated intent**:

> In `core/live.py`, pair each `tool_use` block with its `tool_result` by block id per stream (one transcript file), fill the tool name on `tool_result` events, keep per-stream failure streaks and open calls, and work out each live session's attention states (`failing`, `waiting`, `needs_attention`) inside the tail — the time-based one in the existing poll loop. `snapshot()` carries the states; a `state_version` counter changes only when a session's set of states changes; `GET /api/live` re-sends `snapshot` when it changed. The two thresholds load from `~/.easy-personal/config.yaml` `thresholds:` beside the rule ids.

**Out of scope**:

* The Live view badge / page (next task)
* MCP, CLI or notification surfaces (cut list)
* Telling a hung command from a permission prompt; any error text or tool input
* Re-reading files at dashboard start (accepted limit, PRD OQ #17)

**Requirement Refs**: US-023, FR-002, FR-044, FR-047, FR-048, FR-049, NFR-002, NFR-012

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

**Depends on**: none (builds on HUN-176/HUN-177, merged)

**Entry point**: `easy-personal dashboard` → `live.LiveTail` → `GET /api/live` (`surfaces/api.py` `live_stream`)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | A failed `tool_result` event carries the tool name of its `tool_use` (paired by id, also across two polls); a result with no known `tool_use` carries `tool: null` | FR-047 |
| 2 | Pairing and streaks are per stream: 3 failures in a subagent file and 0 in the main file of the same session mark `failing` with `is_sidechain: true`; a success in the main file does not clear it | FR-047, FR-048 |
| 3 | 3 failed results in a row (default) in one stream → `failing` with the last tool; the next success in that stream clears it; `live_failing_streak: 5` in `thresholds:` changes the bound | FR-048 |
| 4 | An open `AskUserQuestion` or `ExitPlanMode` → `waiting` at once; its result clears it | FR-049 |
| 5 | Another call open ≥ `live_open_call_minutes` (default 10) with no record in the whole session for that time → `needs_attention` within 2 s (fake clock); a record in any stream of the session (e.g. a subagent) prevents it | FR-049, NFR-012 |
| 6 | A later assistant message (new `message_id`) in the stream closes earlier open calls; parallel `tool_use` blocks of one message stay open until each result | FR-049 |
| 7 | A session with an open call stays in `snapshot()` past 300 s and is not forgotten at 1 h; the call is dropped at 24 h | FR-044, FR-049 |
| 8 | `state_version` is unchanged after 100 ordinary tool calls (open → success); it changes once per state set change | NFR-012 |
| 9 | `GET /api/live` sends a fresh `snapshot` (with `attention` per session and `open_call_minutes`) after a state change, with no duplicate or lost `live` event (ids continuous) | FR-044, NFR-012 |
| 10 | `thresholds:` with the two live keys loads; a non-number is refused with the existing message; an unknown key is still refused | FR-049 |
| 11 | Planted secret text in `input` / `content` / `text` and planted block ids never appear in any SSE event, snapshot, store row or log | FR-002, NFR-002 |
| 12 | Real run: dashboard up with `live_open_call_minutes: 1`; a headless session that runs `false` 3 times shows `failing` in `/api/live`; a `sleep 90` call shows `needs_attention` | US-023 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core -q -k "live or rules" && uv run pytest tests/surfaces -q -k live && uv run pytest tests/leak -q
```

## Approach

**Vital slice**: per-stream pairing + streak + open calls → `attention` in `snapshot()` → `state_version` → stream re-send.
**Cut list**: no MCP/CLI surface; no per-tool thresholds; no "unpaired results" metric beyond one log line by type.

`_follow` passes `path` into the items; `_item` adds the block ids it reads; `_Session` holds `streams: dict[path, _Stream]`. Evaluate time-based `needs_attention` in `poll_once` (O(open calls)). Keep `_EVENT_KEYS` as is; add `attention` to `_SESSION_KEYS`.

---

## Edge Case Checklist

- [ ] `tool_use` and `tool_result` in different polls/batches
- [ ] Scheduled ingest read the lines first (tail skips them) → no call left open forever (new-message rule)
- [ ] Parallel tool calls in one assistant message
- [ ] Subagent stream with the parent's session id; foreground `Agent` call while its subagent writes → no `needs_attention`
- [ ] `tool_result` whose `tool_use` came before the tail started → `tool: null`, still counts in the streak
- [ ] Denied permission / user interrupt (`is_error` true) → counts as failure (OQ #16)
- [ ] Format drift: no `id` / `tool_use_id` → no state shown (fail quiet), one log line by type
- [ ] Re-sent snapshot: `last_id` monotonic, cursor continuous, no duplicate events
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, SSE event, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/live.py` | pairing, streams, attention, `state_version`, session keep rules |
| `src/easy_personal/surfaces/api.py` | `_SESSION_KEYS` += `attention`; snapshot `open_call_minutes`; re-send on `state_version` |
| `src/easy_personal/core/rules.py` | accept + read the two live threshold keys |
| `tests/core/test_live.py`, `tests/core/test_rules*.py`, `tests/surfaces/` live tests, `tests/leak/test_live_leak.py` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/sources/claude_code.py` | High-risk parser; pairing reads blocks in `live.py` |
| `src/easy_personal/core/store.py`, `core/migrations/` | Nothing new is stored |
| `src/easy_personal/data/rules.yaml` | Attention states are not weekly rules |
| `src/easy_personal/surfaces/mcp.py`, `surfaces/cli.py` | Cut list |
| `web/` | Next task |
| `~/.claude*/` (any file), `~/.claude.json`, any `settings.json` | Read-only sources |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Temp-dir fixture transcripts (main + `subagents/` file, same session id) appended across polls; fake clock for the 10-min and 24 h rules; assert events, `snapshot()` `attention`, `state_version` counts, stream messages via the existing SSE test client; planted-secret and planted-id lines never in any output.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review