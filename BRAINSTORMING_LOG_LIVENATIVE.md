# BRAINSTORMING_LOG.md — Live works like Claude itself
**Generated**: 2026-11-18
**Task / Context**: owner 2026-11-18: "should check the live, cause i need the work like claude native, where user can input, choose the option, trace the logs, and manage the sections, [admin], supervisor". Owner answers (AskUserQuestion, 2026-11-18): trace = **full transcript** (messages, Claude's text, every tool call with input and output; shown, never stored); busy input = **queue until turn end**; manage = **one place for all sessions**, **stop a terminal session**, **past sessions + resume**, **filters and names**.
**Skill**: `Skill({ skill: "brainstorming" })`, tier **Deep**: it opens a fifth exception of NFR-002 (tool input and output of every session), and lets the app end a process it did not start. Risk **High**.

---

## The Problem Space

What the app does today (checked in code, 2026-11-18):

| Need | Today | Gap to "like Claude" |
|---|---|---|
| Input | App sessions: message box (`web/src/AppSessions.jsx:107` `MessageBox`). Terminal sessions: only at turn end through the Stop hook hold (FR-121, `web/src/ReplyBox.jsx`) | Nothing can be typed to a terminal session while it works; app sessions live on another tab |
| Choose an option | App sessions: cards (FR-111). Terminal: cards through the answer bridge, 120 s (FR-114, `web/src/PendingAnswer.jsx`) | Works; a modal can hide it (owner test 2026-11-18); it is spread over two tabs |
| Trace the logs | Live: tool-call **labels** only (`web/src/LiveView.jsx:212` `Feed`, FR-081/DDR-0004). App session panel: assistant text + labels, no tool input/output (FR-110) | No conversation text for terminal sessions; no tool input/output anywhere |
| Manage sessions | Live: end an app session, Open in app, Resume/Fork (`LiveView.jsx:331-337`). App sessions tab: Interrupt / Stop / Close (`AppSessions.jsx:235-242`) | Two lists; no stop for a terminal session; nothing older than the live hour; no filter; no names |

Facts that shape the design:
- The live tail already knows each session's main transcript file: `LiveTail::session_origin` (`rust/crates/core/src/live.rs:1037`). A transcript route can read that file on demand, so nothing new is tailed.
- Claude Code 2.1.327 writes one file per running session at `<CLAUDE_CONFIG_DIR>/sessions/<pid>.json` with `pid`, `sessionId`, `cwd`, `procStart`, `kind` (`interactive`), `entrypoint`, `status` (`busy` / idle), `name`. That maps a session to its process for **Stop**, gives busy/idle, and gives Claude's own name. The same folder holds a `<pid>.*.key` file and a `messagingSocketPath`: Claude Code's private peer-messaging channel. **Never read the key or use that socket** (undocumented, a credential: DDR-0001). Format is undocumented and may change, so every field is optional and a missing file only hides **Stop**.
- Past sessions are already in the store: `events` has `session_id`, `project_path`, `git_branch`, `ts_utc`, `entrypoint`, `source_path` (`rust/crates/core/src/schema_v11.sql:68-91`). Listing them needs no schema change; their text is read from the transcript file only when opened.
- A Stop hook can deliver text only at a turn end (T110). A queued message is therefore delivered by `hook ask` on the next Stop of that session at once (no hold), with the FR-121 `{"decision":"block","reason":…}` output.

## The Alternatives

### Path A — Read transcripts on demand, one Live panel (recommended)
Server: `GET /api/live/sessions/{id}/transcript?after=<offset>` reads the session's main transcript file (path from the live tail, or the store for a past session) and returns cleaned entries: user text, assistant text, each tool call (name, full input as FR-115 shows it) with its result (text, cut at 16,000 chars a block, `complete:false` when cut). The page polls it every 2 s while the panel is open (offset makes it cheap). App sessions keep their own stream for live text; the panel shows the transcript the same way for both kinds. Queue, stop, past list and names are four small routes.
- Pros: no new tail loop (NFR-012), nothing stored, works for past sessions with the same code, panel is the same for every kind.
- Cons: 2 s poll lag; a big transcript's first read (cap: last 500 entries, "load earlier" pages back).

### Path B — Stream transcript lines through the live stream
Extend the live tail to keep text of every record and push it over `/api/live` SSE.
- Pros: instant.
- Cons: every session's text sits in memory and goes to every page all the time, even when no panel is open — the opposite of "shown only when looked at"; breaks DDR-0004/0005's narrow snapshot; much larger leak surface.

### Path C — Embed a terminal (tmux / pty attach)
Run terminal sessions inside tmux the app controls and show the terminal itself.
- Pros: exact parity, input mid-turn.
- Cons: only for sessions started that way; the owner's existing terminals are not covered; question 42 already put tmux out.

### 50% less code
Skip past sessions and names (store-backed list + names file) and the stop route; ship transcript + one panel + queue only. Covers input, options and trace; leaves "manage" at today's level. Rejected as the plan because the owner picked all four manage items, but it fixes the **order**: transcript + panel first, manage last.

## Adversarial Review

| Path | Why it might fail |
|---|---|
| A | Transcript format changes on a Claude Code release (docs: "can break on any release") → unknown record types are skipped and counted, never an error; a golden test per known record type. A tool result can be megabytes (a Read of a big file) → per-block cut + `complete:false`, per-response cap. Secrets in tool output (an `.env` read) are shown: the owner chose full parity; the page shows them only in an open panel, never in a list, popup, tray, store or log (leak gate proves it). |
| A queue | The session never ends its turn (long run) → the queued text waits; it is shown with "waits for the turn end" and can be edited or cancelled. The bridge is off for that account → queue is not offered (same `reply_bridge` flag). App restarts → queue lost (shown, never stored) — the page says so. Two queued texts → one per session, edit replaces. |
| A stop | PID reuse → check `/proc/<pid>` start time against `procStart` and that the process's executable is `claude` and owned by the same uid before signalling; else 409. Owner stops the session they are typing in (this Supervisor) → one confirm naming the project and short id. SIGTERM ignored → no SIGKILL escalation for terminal sessions (it is the owner's terminal); say "asked to stop". |
| B | Memory and leak surface; every Live page gets all text. |
| C | Out of scope by question 42. |

## Edge Case Checklist (for every TASK_GUIDE)
- Transcript of a session whose file is gone or unreadable (symlink, hostile file rules `live.rs:19`) → 404 / one line, never a 500.
- A resumed session (T105): the main file may be a new file; use `session_origin`.
- Sub-agent streams: shown folded under the Task call that started them, or skipped in v1 (cut list).
- `isMeta` records (`Stop hook feedback:`) show as "reply from the app" entries, not as owner prompts.
- Hidden/control characters: shown as visible escapes (FR-115 rule), React text only, no HTML.
- Page at 375 px: transcript wraps, code blocks scroll inside themselves.
- Session files of other accounts: read per account config dir; a missing `sessions/` folder = no Stop.
- Names file: owner-only mode, temp + rename, max 200 names, 80 chars, FR-081 cleaning.

## Recommended Path
**Path A.** Order:
1. **T115** (backend, High): transcript route + NFR-002 fifth exception (DDR-0007) + leak gate.
2. **T116** (backend, High): session process files (busy/idle, Claude's name, pid) → **Stop** a terminal session; **queue** for busy terminal sessions delivered by `hook ask` on Stop.
3. **T117** (backend, Medium): past sessions list (store, filters by project/branch/date/kind), own names file + routes.
4. **T118** (frontend, Medium): one Live panel for every session — conversation view from T115, app session controls moved in (App sessions tab keeps New session), composer that sends / replies / queues by the session's state, Stop with one confirm; question cards stay on top, never under a dialog.
5. **T119** (frontend, Medium): filters, names, Past sessions list with Resume / Fork and read-only transcript.

## Surgical Scope
Should touch: `rust/crates/core/src/live.rs` (read-only helpers), new `rust/crates/core/src/transcript.rs`, `rust/crates/app/src/{api.rs, live_feed.rs, answer_bridge.rs, hooks_cmd.rs}`, new `rust/crates/app/src/session_procs.rs`, `rust/parity/leak_rust.py`, `web/src/{LiveView.jsx, AppSessions.jsx, ReplyBox.jsx, api.js}`, new web components, `PRD.md`, `docs/ddr/0007-*`.
Must not touch: `~/.claude.json`, any `*.key` or messaging socket of Claude Code, store schema, `src/easy_personal/**` (Python app).

## Cut list (not in this plan)
Full-text search inside past transcripts; sub-agent transcripts in v1; SIGKILL of terminal sessions; mid-turn delivery (impossible through documented hooks).
