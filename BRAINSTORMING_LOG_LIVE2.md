# BRAINSTORMING_LOG.md — Live like Claude, round 2: everything from Live, sub-agent logs included
**Generated**: 2026-11-19
**Task / Context**: owner 2026-11-19, after asking how to trace app-session logs and sub-agents: "yes, I want from the live, I can do all the things, prompt, choose the option, start new session, trace the agents logs include sub-agents, ..., let investigate and update all" + `[admin]`.
**Skill**: `Skill({ skill: "brainstorming" })`, tier **Deep**: it widens DDR-0007's NFR-002 exception to sub-agent transcripts and lets an answer change permission rules ("allow always"). Risk **High**. Under `[admin]` the direction is taken by `docs/claude-md/tracks.md` row 4 (the recommended one).

---

## The Problem Space

Round 1 (T115–T119, DDR-0007) made Live the one panel for every session. A capability map of `main` @ `af4aa48` (2026-11-19, checked in code) shows what is still not "like Claude":

| Need | Today | Gap |
|---|---|---|
| Trace sub-agents | An `Agent` call is one folded tool entry (`Agent · <type>`: input = prompt, result = final text). The parser skips every `isSidechain` record (`rust/crates/core/src/transcript.rs:226`); no route or field lists a session's `subagents/agent-<id>.jsonl` | No sub-agent conversation, no nested agents, no link from the call to its log |
| Start a new session | Form on the **App sessions** tab (`web/src/NewSession.jsx`); Live has only Resume/Fork, which navigate there | No **New session** in Live; no effort; models fixed to `default/opus/sonnet/haiku` (`rust/crates/app/src/app_session.rs:61`) |
| Choose an option | App cards: question / plan / tool (FR-111). Terminal cards via the answer bridge (FR-114) | No "allow always" (`permission_suggestions` / `updatedPermissions` unused anywhere); terminal `ExitPlanMode` shows as a generic permission card (`web/src/PromptCard.jsx:50` `readPrompt`); plan approve has no mode choice |
| Steer a running app session | Interrupt / Stop / Close (FR-110) | Model and permission mode are read-only; no context-use figure |
| Read tool calls | Every tool call is pretty JSON | `TodoWrite` is a JSON blob, not a checklist |
| Terminal sessions | Reply at turn end, queue while busy, Stop (FR-121/124/125) | No interrupt; an idle session with no hold takes no text — limits of documented hooks (T110), kept |

Facts that shape the design (verified 2026-11-19):
- Claude Code 2.1.328 writes `agent-<id>.meta.json` beside each sub-agent stream with `agentType`, `description`, `model`, `toolUseId` (the parent `Agent` call's id), `spawnDepth`, `finished`, and for worktree agents `worktreePath`/`worktreeBranch` (400 meta files sampled). A sub-agent's records carry `isSidechain: true`, `agentId`, and the parent's `sessionId`. `toolUseId` links a stream to the call that started it, in the main thread or (nested) in another sub-agent's stream.
- The binary handles the stream-json control requests `set_model`, `set_permission_mode`, `set_max_thinking_tokens`, `get_context_usage`, `apply_flag_settings` (grep of `~/.local/share/claude/versions/2.1.328`); `--effort low|medium|high|xhigh|max` is a start flag. No `set_effort` subtype: a mid-session effort change is only possible if `apply_flag_settings {effortLevel}` works — a spike decides.
- `permission_suggestions` (input) and `updatedPermissions` (answer) exist in the binary for both the permission-prompt tool and the `PermissionRequest` hook. Claude Code itself writes the rule where its suggestion says (session / local / project / user settings) — the app never writes a settings file for this.

## The Alternatives

### Path A — Extend round 1 in place: one route family, one panel (recommended)
- **Sub-agents**: `GET /api/live/sessions/{id}/agents` lists the session's sub-agent streams from its own `subagents/` folder (id, type, parent `toolUseId`, depth, finished, size; *superseded by FR-129: `description` and `model` are never read*); `GET /api/live/sessions/{id}/transcript?agent=<id>&after=` reads one with the same parser (sidechain records kept for that file) and the same caps. The panel puts **Open log** on each `Agent` call and a breadcrumb (main › agent › nested agent). Works for live and past sessions.
- **App session controls**: Model / Mode / Effort pickers in the panel header sending `set_model` / `set_permission_mode` (and effort if the spike passes); context use from `get_context_usage` on demand. `--effort` and a wider model list on the start form.
- **Cards**: the CLI's own `permission_suggestions` shown as "Allow and don't ask again (<where>)" buttons, sent back unchanged as `updatedPermissions` (app sessions and terminal `PermissionRequest`); a plan card for terminal `ExitPlanMode`; plan approve with "and auto-accept edits".
- **Start**: **New session** button in Live opening the same form in place; Resume/Fork open it there too.
- **Read**: `TodoWrite` drawn as a checklist.
- Pros: reuses T115's guard, parser, caps and leak gate; nothing new is stored or streamed (NFR-012 holds); one panel.
- Cons: sub-agent logs poll like the main log (2 s lag).

### Path B — Fold sub-agent records into the main transcript
Merge every `subagents/*.jsonl` into one time-ordered transcript, indented by depth.
- Pros: one scroll, no navigation.
- Cons: parallel agents interleave into noise; paging by one file offset breaks (offsets per file); big sessions (dozens of agents) blow the 500-entry/2 MB page caps.

### Path C — `--forward-subagent-text` for app sessions only
Start app sessions with the CLI's `--forward-subagent-text` so sub-agent text arrives on stdout.
- Pros: instant for app sessions.
- Cons: terminal and past sessions get nothing (the owner's main case is "sessions that start from the app" *and* terminal); text would sit in the app session's memory — a second, differently-shaped log.

### 50% less code
Ship only the `agent=` parameter and an **Open log** button (no listing route: derive agent ids from the `Agent` call's result `agentId`). Rejected as the whole plan (owner asked for "all the things"), but fixes the **order**: sub-agent logs first; the listing is needed anyway for nested and still-running agents whose call has no result yet.

## Adversarial Review

| Path | Why it might fail |
|---|---|
| A sub-agents | Agent id from the page joins a path → strict `^[A-Za-z0-9]{1,64}$` (*as decided in FR-129*), the file must be `<main file stem>/subagents/agent-<id>.jsonl` under the same confirmed root, opened no-follow (`live::open_regular`); else 404. Meta `description` is model-written text → shown under the same exception, React text only, cut at 300 chars. Nested agents: parent found by `toolUseId` in any stream; a missing parent shows the agent at top level. Hundreds of sub-agents in one session → listing capped (newest 200) with a count. A sub-agent still running → its log grows; same 2 s poll. Worktree agents (`worktreePath`) → their own file still sits in the session folder. |
| A controls | `set_model` with a model the account cannot use → the CLI's error comes back and is shown; the picker reverts. Mode `bypassPermissions` mid-session → same second confirm as the start form. Control request with no answer → 10 s timeout, error shown, session untouched. |
| A cards | `updatedPermissions` lets a rule persist beyond the session → only suggestions the CLI itself offered, sent back unchanged, never a page-made rule; the button names where it lands ("this session" / "this project" / "you, everywhere"). Terminal hook output with `updatedPermissions` on an older Claude Code → feature-check by version; fall back to plain allow. |
| B | Interleaving and paging break. |
| C | Covers app sessions only. |

## Edge Case Checklist (for every TASK_GUIDE)
- Sub-agent file gone, unreadable, symlinked, or outside the root → 404 / one line, never a 500.
- `meta.json` missing or invalid → type "sub-agent", no description, parent unknown; never an error.
- A past session (no live tail) → resolve from the store's main `source_path`, same root check.
- `isSidechain` filtering must stay for the **main** file (T115 goldens unchanged).
- Leak gate: planted secret in a sub-agent file appears only in the transcript answer, never in `/api/live`, store, logs, MCP or CLI.
- Page at 375 px: breadcrumb wraps; pickers fit; cards stay above the composer.
- Hidden/control characters shown as visible escapes (FR-115 rule), React text only.

## Debate (High risk: 3 Advocates) — what may a sub-agent log show?
See the final report and DDR-0008 for the Advocates' answers and the decision.

## Recommended Path
**Path A.** Tasks:
1. **T124** (backend, High): sub-agent listing + `agent=` transcript, parser flag, leak gate, goldens.
2. **T125** (frontend, Medium; after T124): **Open log** on `Agent` calls, sub-agent list with nesting, breadcrumb, works in Past.
3. **T126** (full stack, High): app session steering — Model / Mode / Effort mid-session, context use; `--effort` and wider model list at start (spike first).
4. **T127** (full stack, High): cards — "allow always" from the CLI's own suggestions (app + terminal), terminal plan card, plan approve with mode.
5. **T128** (frontend, Medium): **New session** in Live (form in place, Resume/Fork land there), `TodoWrite` checklist.

## Surgical Scope
Should touch: `rust/crates/core/src/transcript.rs`, `rust/crates/app/src/{transcript_route.rs, api.rs, app_session.rs, app_session_routes.rs, answer_bridge.rs}`, `rust/parity/leak_rust.py`, `web/src/{Transcript.jsx, SessionPanel.jsx, PromptCard.jsx, PendingAnswer.jsx, NewSession.jsx, LiveView.jsx, App.jsx, api.js}`, `PRD.md`, new `docs/ddr/0008-*`.
Must not touch: any Claude settings file (Claude Code writes its own rules), `*.key` files or the messaging socket, the store schema, `src/easy_personal/**` (Python app), the live tail's state (NFR-012).

## Cut list (not in this plan)
Terminal interrupt (no documented way; SIGINT ends the process — T102); text to an idle terminal session with no hold (hook limit — Resume/Fork in app covers it); images/attachments in the composer; thinking blocks; edit diffs; full-text search in transcripts; background Bash output viewer.
