# 0008. Live like Claude 2: sub-agent logs, steering, allow always

**Status**: Accepted
**Date**: 2026-11-19
**Deciders**: thunderkds (owner, `[admin]`), Supervisor (3-Advocate Debate)
**Related**: FR-129..FR-133 · US-044 · NFR-002, NFR-008, NFR-012 · FR-082, FR-109..FR-116, FR-123 · DDR-0007 · PRD question 46 · `BRAINSTORMING_LOG_LIVE2.md`

## Context
On 2026-11-19 the owner asked to do everything from the Live view: "prompt, choose the option, start new session, trace the agents logs include sub-agents". DDR-0007 left sub-agent transcripts on the cut list. Claude Code 2.1.328 writes each sub-agent's records to `<session>/subagents/agent-<id>.jsonl` (every record `isSidechain: true`) and a meta file whose `toolUseId` is the id of the `Agent` call that started it.

## Decision
1. **Same exception, same rules.** DDR-0007's fifth NFR-002 exception covers a session's sub-agent transcripts: read on demand through the existing transcript route (`?agent=<id>`), shown, never stored; same caps and paging. Debate: Advocate 1 (same rules) and Advocate 3 (smallest safe: one parameter on the existing route) against Advocate 2 (a narrower sixth exception). Settled by the owner's written rule "parity, not narrowing" (memory `owner-wants-parity-not-narrowing.md`); Advocate 2 conceded its live-only limit.
2. **Meta file: ids only.** The listing reads `agentType`, `toolUseId`, `spawnDepth` and `finished`; `description` stays never read (FR-082; Advocates 2 and 3). The sub-agent's prompt is already the first entry of its own transcript.
3. **Steering uses the CLI's own control requests** (`set_model`, `set_permission_mode`, `get_context_usage`); values from fixed lists only (FR-116).
4. **Allow always passes the CLI's own suggestion back unchanged.** The app never builds or writes a permission rule; Claude Code writes it where its suggestion says.

## Alternatives considered
- Merge all sub-agent records into the main transcript (Path B). Rejected: parallel agents interleave; per-file paging breaks.
- `--forward-subagent-text` on app sessions (Path C). Rejected: terminal and past sessions get nothing.
- A narrower sixth exception (Advocate 2). Rejected by the parity rule.

## Consequences
Sub-agent files are one more path the route opens: the agent id is checked by pattern and joined only under the session's own `subagents/` folder, opened no-follow, inside a confirmed account's `projects` root. Secrets in a sub-agent's tool output are visible in an open panel, as in the main thread; the leak gate covers the new route.
