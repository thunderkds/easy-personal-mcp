# BRAINSTORMING_LOG_LIVE.md — Live trace, v0.3.0

**Date**: 2026-10-31 · **Tier**: Standard · **Input**: owner "build them as the next version" after
`experiments/live-trace/REPORT.md` (HUN-165..170).

## The Problem Space

The dashboard shows usage only after the daily ingest timer (`easy-personal-ingest.timer`,
`OnCalendar=daily`) or a manual `easy-personal dashboard` start, which ingests once. The owner wants to
watch Claude Code sessions live: active sessions, a feed of tool calls as they happen, a running token
and cost total. Constraints that hold: metadata only (FR-002, NFR-002), 127.0.0.1 only (NFR-001),
source files read-only (NFR-003), connector writes only through `claude mcp` (DDR-0001), dashboard
on demand (decision 2026-10-21).

Measured facts that shape the options (REPORT.md, combined run, 46 tool calls):

| Source | Median | p95 | Needs a write to the owner's Claude config? |
| -- | -- | -- | -- |
| Transcript tail | 214 ms | 1926 ms | No |
| http hooks | 4.7 ms | 7.0 ms | Yes: `hooks` in each account's `settings.json` |
| OpenTelemetry | 1005 ms | 2897 ms | Yes: `env` block in each account's `settings.json` |

Two facts change the earlier recommendation:
- OpenTelemetry tokens equalled the transcript's `message.usage` to 0.000%, and the product already
  turns tokens into a labelled API-equivalent cost (`core/prices.py` `cost()`, HUN-35). OTel adds no
  number the tail cannot give. Its records also carry the account email and ids.
- The tail is already live by the plan's own bar (< 2 s) for 44 of 46 tool calls (95.7%), with zero
  setup. Hooks only matter if sub-second matters.

## Questions for the User

1. Which path (A, B or C below)?

## Alternative Paths

### Option A — Tail-only live view (recommended)
A live tail of every confirmed account's `projects/` tree runs inside `easy-personal dashboard`
(reusing `sources/claude_code.py` `read_complete_lines` / `parse_record`). New events go into the
existing store through the normal ingest path, then out to the page over one Server-Sent Events route
(`GET /api/live`, Starlette `StreamingResponse`). A "Live" view shows active sessions (records in the
last N minutes), a per-session tool feed and a running token + API-equivalent cost total.
- Pros: no write to any Claude config; nothing to install or undo; works for every session, terminal
  or headless, every account; one new route, one view; reuses the existing parser, store and prices.
- Cons: p95 about 2 s, worst seen 3.1 s; depends on the internal transcript format (already true today).

### Option B — Full stack: tail + http hooks + OpenTelemetry
Option A plus a hook collector (`POST /hook`) and an OTLP receiver (`/v1/logs`) inside the dashboard
process, and an `easy-personal live install|uninstall` command that writes the `hooks` and `env`
blocks into each account's `settings.json` with a backup.
- Pros: tool starts at about 5 ms; Claude Code's own cost figure.
- Cons: new write class into the owner's Claude config (outside DDR-0001); a slow or stuck collector
  slows every session (`async` is ignored on http hooks); OTel puts email/ids on every record; three
  sources to dedupe; the env vars only take effect for sessions started after install; about 3x the
  code of A.

### Option C — Tail + http hooks, no OpenTelemetry
Option A plus the hook collector and an opt-in `live install` for hooks only.
- Pros: about 5 ms tool starts; no email-bearing telemetry.
- Cons: still a write into the owner's Claude config and the slow-collector risk; about 2x the code of A.

## Adversarial Review (why each might fail)

- **A**: Claude Code changes the transcript format → the live view degrades exactly like today's
  ingest (abstain + count, FR-003), so the risk is not new. The 8 record types new in 2.1.316 are
  abstained today; the live view inherits that. Polling hundreds of files: measured 2–3 ms per scan
  over ~410 files, p95 negligible.
- **B**: the owner closes the dashboard → hooks hit a closed port (fast, non-blocking, measured) but
  OTel env stays on and retries silently; a half-dead collector (accepts, never replies) blocks every
  tool call until the hook timeout; uninstall must restore settings the owner may have edited since.
- **C**: same settings-write and half-dead-collector risks as B, minus the telemetry ones.

## 50% Rule Check

Option A is the 50%-less-code version of B: one source instead of three, no installer, no settings
writes, no dedupe. It keeps the user-visible goal (a live view under 2 s for ~96% of events).

## Recommended Path

**Option A.** It delivers the live view with no change to the owner's Claude config and the least
code. Hooks (Option C) can be added later as an opt-in if sub-second latency proves needed in use.

## Surgical Scope

| Should touch | Must not touch |
| -- | -- |
| `src/easy_personal/sources/claude_code.py` (expose a follow/tail entry, no parser change) | `~/.claude*/` (read-only, NFR-003) |
| new `src/easy_personal/core/live.py` (tail loop, active sessions) | any `settings.json` / `~/.claude.json` |
| `src/easy_personal/surfaces/api.py` (`GET /api/live` SSE + token check) | `experiments/live-trace/` (spike stays as evidence) |
| new `web/src/LiveView.jsx` + rail entry, `tests/leak/harness.py` route entry, `site/index.html` views stat | kit framework files |

## Edge Case Checklist for TASK_GUIDE

- [ ] Partial last line while Claude writes: never parsed (reuse `read_complete_lines`)
- [ ] Subagent file appears mid-session under `<session>/subagents/`
- [ ] Two sessions at once, two accounts at once
- [ ] Daily ingest timer and the live tail both write: single-writer lock (HUN-38) respected
- [ ] Browser tab closed / reconnect: SSE resumes without duplicating events
- [ ] Dashboard idle for hours: no unbounded memory (bounded feed buffer)
- [ ] Planted secret/text fixtures never reach the SSE stream, store, page or log

## Next Actions

1. Owner picks a path.
2. PRD: new user story + FRs for the live view; `memory/decisions.md` entry.
3. Stage 2: task breakdown (likely: live tail core + store write, SSE route, Live view, transcript
   record-type fix for 2.1.316) → owner approval → board.

## User Selection

Owner chose **Option A — Tail-only live view** on 2026-10-31 (AskUserQuestion: "A: Transcript tail only (Recommended)"). Rejected: B and C (writes into the owner's Claude config; slow-collector risk). Hooks stay a possible later opt-in.
