# BRAINSTORMING_LOG.md — Live attention states
**Generated**: 2026-11-02
**Task / Context**: PRD US-023, FR-047..FR-050, FR-044 (amended), NFR-012; Open Questions #15-17. Reference idea source: github.com/agentblazo/blazo (ideas only).
**Skill**: `Skill({ skill: "brainstorming" })` — tier **Standard** (scope locked; open choice = where the states are computed)

---

## The Problem Space

The Live tab (v0.3.0) shows what each Claude Code session does, but not which session needs the owner. Goal: mark a live session *failing* (3 tool failures in a row in one stream), *waiting for you* (open `AskUserQuestion` / `ExitPlanMode`), or *needs attention* (another call open ≥ 10 min while the whole session is quiet).

Non-negotiable: metadata only (FR-002, NFR-002) — block ids are read, held in memory, never stored or sent; no hooks, no OTel, no new thread/loop/route (NFR-012); source files read-only (NFR-003).

Measured on the owner's history (2026-11-02, 1,101 transcript files, 47,011 tool results): 3+ failure streaks 88, 5+ 18; non-question call open ≥ 10 min ≈ 80 of 42,921; `AskUserQuestion` median wait 68 s. Rejected as noise: same-tool-N-times (292/550 streams), tool-name loops (11 streams, all planned batches).

Verified code facts:
- `core/live.py` `_item`: `tool_result` events carry `"tool": None` (no pairing today); feed shows a failed result as `tool_result`.
- Subagent records share the parent `sessionId` (checked on a real `subagents/agent-*.jsonl`) → streams must be keyed by file path; `_follow` knows `path`.
- `_follow` returns early when "the scheduled ingest moved the cursor first" → those lines never reach the tail → a `tool_result` can be skipped.
- `snapshot()` drops sessions with `last_ts` older than 300 s; `_publish` forgets sessions after `FORGET_AFTER_S` (1 h).
- `surfaces/api.py` whitelists fields: `_SESSION_KEYS`, `_EVENT_KEYS`; the stream sends `snapshot` first, then `live` events.
- `web/src/LiveView.jsx` reducer already handles a repeated `snapshot` (keeps feeds unless ids restarted); `TICK_MS = 15_000`; `idle` worked out on the page from `last_ts` and `windowS`.
- `core/rules.py` `load_rules` raises `thresholds for unknown rules` for any `thresholds:` key that is not a rule id.

---

## Questions for the User

All resolved:
1. Badge look → coloured pill in the card head + border colour; header "N sessions need you"; cards with a state sort first in their project.
2. Long foreground subagent → *needs attention* requires open ≥ 10 min **and** the session quiet for that time (PRD FR-049 updated).

---

## Alternative Paths

| Option | Name | Summary | Invasiveness | Code Volume | Regression Risk | Recommended? |
|--------|------|---------|-------------|------------|----------------|--------------|
| A | Event-carried state | Tail computes states; every `live` event carries its session's current `attention`; snapshot carries it too | Medium | ~300 lines | Medium | |
| B | Page-derived state | Tail only adds tool name + open/closed facts to events; the page counts streaks and open calls | Medium | ~350 lines | High | |
| C | Snapshot-pushed state | Tail computes states into `snapshot()`; the stream re-sends a `snapshot` whenever any session's state changes | Low | ~200 lines | Low | ✅ Yes |

### Option A — Event-carried state
**Approach**: `live.py` keeps per-stream pairing + streaks; each buffered event gets the session's `attention` list; `_EVENT_KEYS` and the page reducer copy it.
**Pros**: state arrives with the event that changed it; no extra messages.
**Cons**: every event grows; reducer + event schema + api whitelist all change; state duplicated on every event.
**Why it might fail**: a state that changes with no event (the 10-min timer, a 24 h drop) has no event to ride on → still needs a second mechanism.

### Option B — Page-derived state
**Approach**: server sends `tool` on results and a call-open flag; the page counts streaks and open-call ages.
**Pros**: server stays a dumb pipe.
**Cons**: logic in JS, tested twice; the page only sees events since it opened (bounded buffer, FEED_MAX 50).
**Why it might fail**: a reload or a reconnect past the buffer loses the streak/open-call history → wrong states; the snapshot would need the server-side state anyway → becomes A plus duplicate logic.

### Option C — Snapshot-pushed state
**Approach**: `live.py` keeps per-stream `open_calls` (id → tool, ts) and `fail_streak` (count, tool); `snapshot()` adds per session `attention` = list of `{state, tool, is_sidechain, since}`; the tail bumps a `state_version` counter only when a session's set of states changes; `live_stream` re-sends `snapshot` when `state_version` moved. *Needs attention* (open ≥ 10 min and quiet ≥ 10 min) is also worked out in the tail, in its existing 100 ms poll loop (O(open calls), no new thread), so the page only renders `attention`.
**Revised 2026-11-02 (performance review)**: the first draft sent the oldest open call to the page for a page-side 10-min check. That makes every ordinary tool call (open → close) a state change → one snapshot re-send per tool call (owner peak: 78 tool calls/min). Rule: an ordinary open call is never part of `attention`; only `failing`, `waiting`, `needs_attention` are, so re-sends happen only on real state changes.
**Pros**: one source of truth (the snapshot); page change = render only + one time check; reuses the existing snapshot path the reducer already handles; no event schema change except `tool` filled on results (FR-047).
**Cons**: a state change re-sends the whole session list (small: active sessions only; state changes are rare — ~88 streaks in all history).
**Why it might fail**: a re-sent snapshot races with events in flight → must keep the stream's cursor = snapshot `last_id` (already how the first snapshot works); a reducer bug could wipe feeds on re-send (covered by the existing `restarted` check — test it).

---

## 50% Rule Check

Option C already removes the event-schema and reducer changes of A. Further 50% cut considered: skip `state_version` and let the page re-fetch nothing — compute *failing* only from the feed it sees. Rejected: breaks on reload (Option B's failure). Kept cut: no new SSE event type, no new route, no MCP/CLI surface, no notification.

---

## Recommended Path

**Option C — Snapshot-pushed state.** Smallest change that stays correct across reload and reconnect, keeps all logic in Python next to the tail (one test suite), and needs no new route or thread (NFR-012).

---

## Surgical Scope

Files that **should** be touched:
- `src/easy_personal/core/live.py` — pairing by block id, per-stream streaks/open calls, `attention` + `quiet_since` in `snapshot()`, `state_version`, keep sessions with open calls (≤ 24 h), `tool` on `tool_result` events.
- `src/easy_personal/surfaces/api.py` — `_SESSION_KEYS` += `attention`, `quiet_since`; re-send `snapshot` on `state_version` change.
- `src/easy_personal/core/rules.py` (or `core/config.py`) — accept `live_failing_streak`, `live_open_call_minutes` under `thresholds:` beside rule ids.
- `web/src/LiveView.jsx`, `web/src/app.css` — badge, border colour, header count, sort; idle rule respects an open call.
- Tests: `tests/` live tail + api stream; `web/src/LiveView.test.jsx`.
- `README.md` Live view section — states, thresholds, accepted start-gap limit (OQ #17).

Files that **must not** be touched:
- `src/easy_personal/sources/claude_code.py` — parser is a High-risk area; pairing reads blocks in `live.py` like `is_error` does today.
- `src/easy_personal/core/store.py`, migrations — nothing new is stored.
- `src/easy_personal/data/rules.yaml` — these are live states, not weekly rules.
- `src/easy_personal/surfaces/mcp.py`, `cli.py` — cut list.

---

## Edge Case Checklist for TASK_GUIDE

- [ ] `tool_use` and its `tool_result` in different polls/batches → pairing state persists per stream across `_follow` calls.
- [ ] Scheduled ingest read the lines first (tail skips them) → a skipped result must not leave a call open: a later assistant message (new `message_id`) in the same stream closes earlier open calls.
- [ ] Parallel tool calls in one assistant message (several `tool_use` blocks) → all open until each result; the new-message rule must not close them early.
- [ ] Subagent stream (`subagents/*.jsonl`, same `sessionId`) → streak and open calls keyed by file path; badge says "agent".
- [ ] Foreground `Agent` call open while its subagent writes → session not quiet → no *needs attention*.
- [ ] `tool_result` whose `tool_use` came before the tail started → tool unknown; counts in the streak, badge says "unknown tool".
- [ ] Denied permission / user interrupt → `is_error` true → counts as failure (OQ #16).
- [ ] Any success in the stream resets its streak; a success in another stream does not.
- [ ] Session with an open call stays in `snapshot()` past 300 s and is not forgotten at 1 h; dropped at 24 h.
- [ ] `thresholds:` with `live_failing_streak` / `live_open_call_minutes` loads; a non-number is refused with the existing message; unknown keys still refused.
- [ ] Re-sent snapshot keeps feeds (no `restarted`), keeps `lastId` monotonic, sends no duplicate events.
- [ ] Block ids never appear in the SSE payload, logs or store (planted-id test, NFR-002 style).
- [ ] *Needs attention* is set by the tail poll loop ≤ 1 s after both 10-min conditions hold; clears when a record arrives.
- [ ] Snapshot re-send count: a session making 100 ordinary tool calls (open → success) causes 0 re-sends (test asserts `state_version` unchanged).
- [ ] Format drift: if `id` / `tool_use_id` are missing (future Claude Code), no state is shown (fail quiet: no badge, never a wrong badge); unpaired results are counted and logged once by type.
