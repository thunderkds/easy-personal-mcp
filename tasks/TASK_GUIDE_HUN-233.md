**Date**: 2026-11-06
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Live tail, Attention state; Critical Constraints: metadata only and its 2026-11-06 Live exception)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-043, FR-044, FR-047, FR-071, FR-081..FR-083, NFR-011, NFR-012; `docs/ddr/0004-live-call-labels-show-never-store.md`; `BRAINSTORMING_LOG_LIVELABELS.md` (Option A + Supervisor defaults); then `src/easy_personal/core/live.py`, `src/easy_personal/core/jobs.py` (`command_name`), `tests/core/test_live.py`, `tests/leak/test_live_leak.py`, `tests/leak/harness.py`, `tests/leak/canaries.py`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-06: "from the tools logs for the live, it just show the simple, don't know what is agent doing, I mean could we know the exactly agent do, call which tools, which steps of the kits agent are in will be better".

Owner choices (AskUserQuestion, 2026-11-06): **Safe labels only**; stage = **newest kit skill, fixed map**; sub-agent = **type only** (`agentType`); display = **row chip + panel feed** (frontend task); path = **Option A** (tail works it out, page renders).

**Restated intent**:

> In `core/live.py`, give each buffered `tool_use` event a `label` (FR-081): `Skill` → input `skill`; `Agent`/`Task` → input `subagent_type`; `Bash` → first word of input `command` via `jobs.command_name`, after skipping a leading `cd <dir> &&` / `cd <dir>;` (live-only wrapper; `jobs.py` unchanged); `mcp__<server>__<tool>` → `{server, tool}` split at the first two `__` (no input read). Any label not matching `^[A-Za-z0-9][A-Za-z0-9:._-]{0,63}$` → no label. Other tools → no label. Each buffered event also gets `agent`: `null` for the main thread, else the stream's `agentType` from `subagents/agent-<id>.meta.json` beside its `.jsonl` (read once per stream, ≤ 4 KB, only `agentType`; retried on the stream's next items until read; missing/unreadable/bad after 30 s → `"sub-agent"`) (FR-082). Each `_Session` keeps `stage` + `stage_skill`: the stage of the newest `Skill` call (any stream) whose skill (part after the last `:`) is in the fixed `STAGE_MAP` (FR-083); `snapshot()` sends both (null until known); a change bumps `_state_version` so the stream re-sends the snapshot only then (NFR-012). Labels, agent type and stage never reach the store, logs, MCP or CLI.

**Out of scope**:

* `web/` (separate frontend task, depends on this one)
* `sources/claude_code.py` parser output, store, migrations, MCP/CLI surfaces, `core/jobs.py`
* Typed slash commands (message text stays banned); history replay; reading any other input or meta field

**Requirement Refs**: FR-081, FR-082, FR-083 (FR-044, FR-047, NFR-011, NFR-012 kept)

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner choices recorded above)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None

**Entry point**: `LiveTail._follow` → `_item` → `_publish` → `_append` / `snapshot` in `src/easy_personal/core/live.py`, read by `GET /api/live` (`surfaces/api.py`) while `easy-personal dashboard` runs

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `Skill {skill: "code-review", args: CANARY}` → event `label == "code-review"`; `Agent`/`Task {subagent_type: "qa-expert", prompt: CANARY, description: CANARY}` → `label == "qa-expert"`; no CANARY in any event or snapshot | FR-081 |
| 2 | Bash: `cd /x && uv run pytest` → `uv`; `FOO=1 pytest -q` → `pytest`; `bash -c "git status"` → `git`; empty/missing command → no label; `git status CANARY` → `git` | FR-081 |
| 3 | `mcp__easy-verifier__score` → `label == {"server": "easy-verifier", "tool": "score"}` (or the agreed flat shape, documented in the docstring); `mcp__a__b__c` → server `a`, tool `b__c` | FR-081 |
| 4 | A label failing the pattern (space, 65 chars, leading `-`, non-string) → no label; `Read`/`Edit` → no label | FR-081 |
| 5 | Sub-agent stream with meta `{"agentType": "backend-developer", "description": CANARY}` → its events carry `agent == "backend-developer"`; main thread → `agent is None`; missing meta → retried, then `"sub-agent"` after 30 s (fake clock); invalid JSON / > 4 KB / bad pattern → `"sub-agent"`; meta read once per stream (count opens) | FR-082 |
| 6 | Stage: `Skill tdd` → snapshot `stage "3"`, `stage_skill "tdd"`; then `wake` → unchanged; then `easy-kit:code-review` → `"4"`; a mapped skill in a sub-agent stream also counts; `_state_version` bumps on each change only, not on unmapped skills or ordinary calls | FR-083, NFR-012 |
| 7 | Store unchanged: after the tail reads planted transcripts, no store row, log line, MCP response or CLI output holds a label, agent type, stage or CANARY (leak suite: add `Skill`/`Agent` blocks and a sub-agent meta file to the planted fixtures; `/api/live` may show labels but never CANARY) | FR-081..FR-083, DDR-0004 |
| 8 | NFR-011 latency check (existing test or spike harness) still passes; full `uv run pytest` passes | NFR-011 |
| 9 | Real run: `easy-personal dashboard` with a running session that calls a Skill and spawns a sub-agent; pasted `/api/live` lines show `label`, `agent` and a snapshot `stage` | FR-081..FR-083 |

---

## UI / Design Acceptance Criteria

N/A — pure backend task. The page change is the frontend task.

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence. UI Evidence rows: ☐ N/A (pure backend).

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core/test_live.py tests/surfaces/test_api_live.py tests/leak -q && uv run pytest -q
```

## Approach

**Vital slice**: in `live.py` — `STAGE_MAP` (dict skill → stage string), `_call_label(name, input)` (reads only the four allowed fields), `_cd_skip(command)` wrapper before `jobs.command_name`, label pattern check; `_item` adds `label` to `tool_use` events (input read only for `Skill`/`Agent`/`Task`/`Bash`); `_Stream.agent_type` + `_meta_agent_type(path)`; `_Session.stage`/`stage_skill`; `_append` adds `label` and `agent`; `snapshot()` adds `stage`/`stage_skill`; bump `_state_version` on stage change.
**Cut list**: no new module unless `live.py` grows > \~650 lines; no config for the stage map; no history scan for stage.

Keep the `_item` docstring accurate: update it to list exactly which input fields are now read and why (DDR-0004).

---

## Edge Case Checklist

- [ ] `input` not a dict, or field not a string → no label
- [ ] `skill` with plugin prefix → map by part after last `:`; label keeps the full name
- [ ] Meta file written after the first `.jsonl` line → retried, not cached as "sub-agent" at once
- [ ] Meta path from the stream path only (`<dir>/agent-<id>.jsonl` → `<dir>/agent-<id>.meta.json`); never follow other paths; symlink → treat as unreadable
- [ ] A session dropped after 24 h / idle drops its stage with it
- [ ] Resume (`events_since`) returns labels from the buffer; reconnect snapshot has stage
- [ ] Leak canaries in `Skill.args`, `Agent.prompt`, `Agent.description`, meta `description`, Bash arguments

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/live.py` | labels, agent type, stage map, snapshot fields |
| `tests/core/test_live.py` | AC 1–6 |
| `tests/surfaces/test_api_live.py` | event + snapshot carry the new fields |
| `tests/leak/` (harness, canaries, planted fixture) | AC 7 |
| `tests/fixtures/claude_code/...` | `Skill`/`Agent` blocks + a sub-agent meta file, if the leak suite plants from there |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/sources/claude_code.py`, `src/easy_personal/core/store.py`, `core/migrations/` | Never store (DDR-0004) |
| `src/easy_personal/core/jobs.py` | `command_name` reused unchanged (FR-022) |
| `web/` | Frontend task |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Unit tests in `tests/core/test_live.py` with hand-made transcript lines (existing helpers): labels per tool, Bash cases, MCP split, pattern rejects, meta read once / retry / fallback with a fake clock, stage changes and `state_version`. API test: SSE event and snapshot fields. Leak suite: new planted blocks + meta file with canaries; assert no canary anywhere and no label in store/logs/MCP/CLI. Real run with pasted `/api/live` output.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] UI Evidence rows ☐ N/A (pure backend)
- [ ] Real run done and output pasted
- [ ] Supervisor notified: task ready for Stage 4 review