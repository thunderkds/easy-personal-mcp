# 0007. Live like Claude: transcripts shown on demand, never stored

**Status**: Accepted
**Date**: 2026-11-18
**Deciders**: thunderkds (owner), Supervisor
**Related**: FR-123..FR-128 · NFR-002, NFR-008, NFR-012 · FR-115, FR-121 · US-043 · DDR-0004, DDR-0005, DDR-0006 · PRD question 45 · `BRAINSTORMING_LOG_LIVENATIVE.md`

## Context
The owner asked on 2026-11-18 for the Live view to "work like claude native, where user can input, choose the option, trace the logs, and manage the sections". They chose the full transcript (tool input and output included), a queue for busy terminal sessions, one place for all sessions, stopping a terminal session, past sessions with resume, and filters and names.

## Decision
1. **Fifth exception of NFR-002.** A session's transcript is read from Claude Code's own file only when the owner opens that session's panel (`GET /api/live/sessions/{id}/transcript`). It is shown, never stored, logged, cached or pushed on `/api/live`. Nothing is redacted (owner: parity); per-block and per-answer cuts bound the size.
2. **On demand, not streamed.** The page polls the route while a panel is open. The live tail keeps no text (NFR-012, DDR-0004 stay).
3. **Process files, read-only.** Claude Code's `sessions/<pid>.json` gives pid, busy/idle and name. The app never opens the `*.key` files or the peer messaging socket next to them. Stop sends SIGTERM only after checking uid, executable and start time.
4. **Queue through the Stop hook.** Text typed to a busy terminal session waits in memory and leaves on its next `Stop` through FR-121's block output.

## Alternatives considered
- Stream every record's text through `/api/live` (every page gets all text all the time). Rejected.
- tmux or an embedded terminal. Rejected by question 42.
- Claude Code's private peer socket for mid-turn input. Rejected: undocumented, needs a key file.

## Consequences
The transcript format may change on any Claude Code release: unknown records are counted as `skipped`, never an error. Secrets inside tool output are visible in an open panel, as in the terminal; the leak gate proves they go nowhere else.
