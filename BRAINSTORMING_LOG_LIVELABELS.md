# BRAINSTORMING_LOG.md — Live call labels + kit stage

**Date**: 2026-11-06 · **Tier**: Standard · **Requirements**: FR-081..FR-084, US-033, DDR-0004 (locked by the 2026-11-06 grill)

---

## The Problem Space

The Live view feed shows tool names only (`web/src/LiveView.jsx` `Feed`, `live-tool`). The owner cannot tell what the agent does or which kit stage it is in. The grill locked *what* to show: call labels (`Skill.skill`, `Agent`/`Task` `subagent_type`, Bash first word, MCP server/tool), the sub-agent `agentType` from `agent-<id>.meta.json`, and a kit stage from a fixed map. All of it is shown live and never stored. Open now: *where* the labels and the stage are worked out, and how they reach the page.

Verified code facts:
- `core/live.py` `_item()` reads only `type`, `id`, `tool_use_id`, `name`, `is_error` from content blocks. Events go to the buffer in `_append()`. The store gets `parsed` rows from `claude_code.parse_record`, never `_item` output. So a label added in `_item` cannot reach the store.
- `_Session` keeps per-session state; `snapshot()` sends it; `_state_version` bumps (line 481) re-send the snapshot only when attention changes (NFR-012).
- `_Stream` is keyed by transcript path; a sub-agent stream is a path under `subagents/`.
- `core/jobs.command_name` gives the first word (handles `bash -c`, env prefixes). `cd /x && uv run pytest` → `cd`.
- Real meta file: `{"agentType":"backend-developer", "description": ..., "toolUseId": ..., ...}`.
- Leak fixtures plant `@@tool_input@@` canaries in a Bash `command` (`git status @@tool_input@@ ...`); no `Skill`/`Agent` fixtures exist yet.

## Questions for the User

1. Which path (A, B or C)? Recommendation: A.

## Alternative Paths

### Option A — Tail works it out, page renders (recommended)
`_item()` adds `label` to each `tool_use` event; `_Stream` caches `agent_type` (meta read once per stream); `_Session` keeps `stage`/`stage_skill`; a stage change bumps `_state_version`, so the snapshot carries it. `_append()` puts `label` and `agent` on buffered events. The page adds a row chip and feed labels.
- Pros: one place for the rules; reconnect/resume gets stage from the snapshot and labels from the buffer; matches FR-084 "page only renders"; leak suite already reads `/api/live`.
- Cons: touches the hot file `live.py` (FR-043..FR-072 history); stage re-send adds snapshot pushes (one per stage change only).

### Option B — Tail sends labels, page works out the stage
The tail adds `label`/`agent` only; the page maps skill → stage in JS from the feed.
- Pros: less Python; no snapshot change.
- Cons: stage lost on page reload (the feed starts empty); map in JS, breaks FR-083/FR-084 (stage is the tail's); each tab recomputes.

### Option C — Labels in the parser (`claude_code.parse_record`)
Add label fields to the shared parser, so ingest and live both see them.
- Pros: one parser for future surfaces (usage by skill).
- Cons: parsed rows go to `store.save_file` — one slip stores tool input; breaks DDR-0004 "never store"; wider blast radius (ingest, re-read, leak suite).

## Adversarial Review

- **A fails if** a label slips into a stored row or a log: guard = labels only exist in `_item`/`_append`, never in `parsed`; leak test plants canaries in `Skill.args`, `Agent.prompt`/`description`, meta `description`, Bash arguments and checks store + logs + MCP + CLI + `/api/live`. Fails if the meta read blocks the poll loop: guard = one small `stat`+read per stream, cached, size cap (e.g. 4 KB).
- **B fails** the first time the owner reloads mid-review: the chip disappears until the next skill call.
- **C fails** on the first refactor of `parse_record` that forwards a field to `save_file`.

## 50% Rule Check

Half the code: drop the meta read (FR-082) and show "agent" as today; drop the row chip and show stage only in the panel. Rejected: the owner chose both on 2026-11-06. Code still small in A: about 60 Python lines + 30 JSX lines + tests.

## Recommended Path

**Option A.** Supervisor defaults (owner may change):
- Bash label skips a leading `cd <dir> &&` / `cd <dir>;` so `cd /repo && uv run pytest` shows `uv`, not `cd`. Live-only wrapper; `jobs.command_name` stays unchanged (FR-022 suggestions use it).
- Skill names with a plugin prefix (`easy-kit:tdd`) look up the map by the part after the last `:`; the label keeps the full name.
- `Task` is treated as `Agent` (older Claude Code name).
- A missing meta file is retried on the stream's next item until found, then cached; still missing after 30 s → "sub-agent".

## Surgical Scope

Must touch: `src/easy_personal/core/live.py`, `web/src/LiveView.jsx`, `web/src/app.css` (chip only), tests (`tests/test_live*.py`, `web/src/LiveView.test.jsx`, `tests/leak/` canaries + fixtures with `Skill`/`Agent` blocks and a sub-agent meta file).
Must not touch: `core/jobs.py` (`command_name` reused as is), `sources/claude_code.py` parser output, store/migrations, MCP/CLI surfaces, `/api/live` route code beyond what the event dict already carries.

## Edge Case Checklist for TASK_GUIDE

- [ ] Label pattern `^[A-Za-z0-9][A-Za-z0-9:._-]{0,63}$`; fail → no label (never truncated text).
- [ ] `Skill` with `args` / `Agent` with `prompt`, `description`: canaries never reach `/api/live`, store, logs.
- [ ] Bash `cd x && pytest` → `pytest`; `bash -c "..."`; empty command → no label.
- [ ] MCP name with `__` in the tool part (`mcp__a__b__c`) → server `a`, tool `b__c`.
- [ ] Meta file missing, unreadable, invalid JSON, oversized, `agentType` failing the pattern → "sub-agent".
- [ ] Stage: unmapped skill leaves it unchanged; plugin prefix; newest wins across streams; snapshot re-sent only on change (NFR-012).
- [ ] Resume/reconnect keeps stage (snapshot) and labels (buffer).
- [ ] Page: no chip without stage; "main" vs sub-agent type in feed; MCP shown "server · tool"; 375 px no horizontal scroll.
- [ ] NFR-011 2 s latency still holds with the meta read.

## Next Actions

1. Owner picks a path.
2. Stage 2: two tasks — backend (live.py labels, agent type, stage, leak suite) then frontend (chip + feed labels, after backend).

## User Selection

**Option A** — owner, 2026-11-06 ("A: tail works it out"), with the Supervisor defaults listed under Recommended Path.
