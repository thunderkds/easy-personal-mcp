# BRAINSTORMING_LOG.md — Write sessions (the app stops being read-only toward Claude Code)
**Generated**: 2026-11-17
**Task / Context**: owner, 2026-11-17: "we should support the write session in the app, it means that, the app will break the readonly rules" · "we can accept all of write actions like call the claude sub for analyze and update, you can input the text to the ask question in the app and send to claude session are running, ..." · scope now: **plan only** (no code).
**Skill**: `Skill({ skill: "brainstorming" })`, tier **Deep**. It reverses a project-wide constraint, adds a code-changing agent, and opens a new input path into live sessions. Risk **High**.

---

## The Problem Space

The Rust app watches every Claude Code session. Since T094/T096 it can show the question a waiting session asks (FR-106), but the owner still has to find the terminal to answer it. The agent run (FR-101..104, `rust/crates/app/src/agent_run.rs`) can start one fixed read-only job, `investigate`, with five MCP tools (`agent_run.rs:55`), no built-in tool, and a 10-minute limit (`agent_run.rs:46`).

The owner asks for two write capabilities:

1. **App sessions ("call the claude sub for analyze and update").** The app starts the owner's own `claude` on a chosen repo with a prompt the owner types. The run may change files (update), not only read and report (analyze). The owner can keep talking to it from the app.
2. **Reply to a running session ("input the text to the ask question … send to claude session are running").** The owner answers a waiting session's `AskUserQuestion` in the Live view, and perhaps types the next prompt for a *your turn* session, without opening the terminal.

### What Claude Code supports (claude-code-guide research of the docs, 2026-11-17; to be verified in a spike)

| Mechanism | Works on an already-open terminal session? | Documented? |
|---|---|---|
| A PreToolUse or PermissionRequest hook on `AskUserQuestion` returns `allow` with `updatedInput.answers = {"<question>": "<label>"}` | **Yes**, if a user-scope hook is installed before the session starts | Yes (hooks.md, tools-reference.md) |
| A Stop hook returns `{"decision":"block","reason":"…"}`, so the session carries on | Yes (same condition) | Pattern documented; how `reason` reaches Claude is not confirmed |
| `claude --resume <id> -p "text"` | **No.** Two writers to one transcript lose messages, per the docs. `--fork-session` makes a copy instead. | Yes |
| Background sessions (`--bg`, `claude attach`, `--resume <bg> "prompt"`) | Only sessions started with `--bg` | Yes |
| Channels (`notifications/claude/channel`) | No. The flag must be set at launch; custom channels need a dev flag that `-p` ignores. Research preview. | Yes, as a preview |
| tmux `send-keys` / terminal injection | Only inside tmux, and fragile | No |
| A CLI command that installs hooks | none exists: hooks are added by editing `~/.claude/settings.json` | n/a |

Also from the docs: the JSONL transcript format "can break on any release". The live tail already depends on it, and this feature depends on it more.

### Rules this feature breaks (each needs an owner-approved amendment and DDR-0006)

| Rule | Where | What changes |
|---|---|---|
| "Source files are read-only; nothing under `~/.claude/projects/` or any repo is modified." | `PRD.md` NFR-003 | An update run edits repos. The app itself still never writes `~/.claude/projects/`; Claude Code does. |
| "Writes allowed only to `~/.easy-personal/`, the OS keychain … and user scope via `claude mcp add-json`" | `PROJECT_SPEC.md` Critical Constraints | Path 2 must add user-scope hooks to `~/.claude/settings.json`, and no CLI exists for it |
| Metadata only (NFR-002), with three named exceptions (DDR-0003/0004/0005) | `PROJECT_SPEC.md`, `docs/ddr/` | Owner prompt text, owner answers and Claude's reply text pass through the app: a fourth exception, *show, never store* |
| Agent run: one fixed job, five tools, no built-in tool, 10 min, an empty work folder | FR-102, `agent_run.rs:55`, `:46` | A free prompt in a real repo with built-in tools |
| Out of scope: "starting, stopping or editing … agents", "MCP tools that create or change …" | `PRD.md:257-258` | Line 258 changes; the MCP tool surface stays frozen (no new tool) |
| Question 38: no runs the app starts by itself | `PRD.md` | Unchanged. Every write starts from an owner click. |

---

## Questions for the User

1. **Path**: B as recommended (app sessions first, then the AskUserQuestion bridge), or another path?
2. **Update power**: in an update run, may Claude run shell commands (`Bash`), or only `Read/Grep/Glob/Edit/Write`? And always on a new git branch or worktree, or in place?
3. **Reply text**: may the app show Claude's reply text (it shows only metadata today)? Proposed: show, never store, like DDR-0005.
4. **Hook install**: may the app add one hook block to `~/.claude/settings.json` (add-only, shown before writing, with an undo command)? Without it, a terminal session cannot be answered from the app.
5. **Who answers when both are open**: while the app waits for the owner, the terminal shows no question. Proposed: wait up to 120 s, then fall back to the terminal; Live view offers **Answer in terminal** to release it at once.

---

## Alternative Paths

| Option | Name | Summary | Invasiveness | Code Volume | Regression Risk | Recommended? |
|--------|------|---------|-------------|------------|----------------|--------------|
| A | App sessions only | The app starts and talks to its own `claude` sessions (stream-json stdin/stdout). Terminal sessions stay read-only. | Medium | ~1,500 Rust + ~600 web | Medium | |
| B | App sessions + AskUserQuestion bridge (phased) | A first. Then a user-scope PreToolUse hook (this binary's `hook ask` command) asks the running app over an owner-only unix socket for the owner's answer, and falls back to the terminal. A Stop-hook reply is a later, optional phase. | High (in phases) | A + ~900 Rust + ~300 web | Medium-High | ✅ Yes |
| C | Resume / fork | The Live view sends text with `claude --resume <id> -p` to sessions that are not open, and `--fork-session` for ones that are | Low | ~500 Rust + ~300 web | Low in code, **High** in data loss | |
| D | tmux injection | `tmux send-keys` into the pane that runs the session | Low | ~300 | High (keystrokes into the wrong pane) | ❌ rejected |
| E | Channels | The app's MCP server pushes text as a channel | Medium | ~600 | High (preview; launch flag; dev flag) | ❌ later, once Channels is out of preview |

### Option A — App sessions only
**Approach**: Extend the agent run into an **app session**: an owner-typed prompt; a repo from the workspace's stored repos; a profile, *analyze* (read tools only) or *update* (edit tools, on a new branch in a worktree the app makes); `claude -p --input-format stream-json --output-format stream-json` with the agent run account's `CLAUDE_CONFIG_DIR` and the same environment scrub as FR-102. The app owns stdin, so a follow-up message and an `AskUserQuestion` answer are just stdin lines. The session appears in the Live view like any other (`sdk-cli` entrypoint).
**Pros**: Uses only documented interfaces. No Claude Code config is touched. Builds on the guarded `agent_run.rs` (account choice, one at a time, scrubbed environment, lock).
**Cons**: Does not answer the sessions the owner already runs in terminals, which is the second half of the request.
**Why it might fail**: The stream-json input protocol is less documented than `-p` (spike needed). The NFR-008 guard does not prove a human clicked (question 36), and an update run with built-in tools turns a page fault into code changes in a repo. Long sessions break the 10-minute limit, which needs a new limit and a **Stop** that works.

### Option B — App sessions + AskUserQuestion bridge (phased) ✅
**Approach**: Phase 1 = Option A. Phase 2: `easy-personal-rs hooks install` (also a button in Settings) adds one PreToolUse hook with matcher `AskUserQuestion` to `~/.claude/settings.json`. It adds only, shows the diff first, keeps a backup, and `hooks remove` undoes it. The hook runs `easy-personal-rs hook ask`, which reads the hook JSON on stdin and connects to `~/.easy-personal/answer.sock` (mode 0600). With no app running or no socket, it exits 0 at once and changes nothing, so the terminal works as today. With the app running, the Live view shows the question **with its option labels** and an **Other** text box. The owner's pick goes back as `updatedInput.answers`. A timeout or **Answer in terminal** returns nothing, and the terminal asks as usual. Phase 3 (optional, owner gate): a Stop hook for *your turn* replies.
**Pros**: Covers both halves. Uses only documented hook fields. Fails open: with no app, the session behaves as today. Opt-in, and can be undone.
**Cons**: It writes Claude Code's user config by direct edit, against today's rule. It needs option labels, which FR-106 now forbids reading. Only sessions started after the install are covered (hooks load at start).
**Why it might fail**: A hook bug blocks every session's questions on the machine (mitigation: hard timeout, fail open, a test of the no-app path). Another local process could answer through the socket (mitigation: owner-only socket, peer-uid check, a per-question nonce shown only in the guarded page). The `answers` format may change. Concurrent edits to `settings.json` by Claude Code could lose a write (mitigation: write-temp-and-rename, re-read and compare).

### Option C — Resume / fork
**Approach**: A text box on a *your turn* session runs `claude --resume <id> -p "<text>"`, or `--fork-session` when the session is still open.
**Pros**: Smallest. No hook, no config write.
**Cons**: The docs say two writers to one transcript lose messages. The open terminal never sees the reply, and the fork is a different session. It cannot answer `AskUserQuestion`.
**Why it might fail**: The owner types in the terminal at the same time and loses history: silent data loss, the worst kind.

---

## 50% Rule Check

Ship **Phase 2 before Phase 1, with no update profile**: the AskUserQuestion bridge only (about 900 + 300 lines). It meets the clearest pain (answer a waiting question in the app) and touches no repo. App sessions with *analyze* only reuse `agent_run.rs` almost as it is (a free prompt instead of the fixed job): about 400 more lines. *Update* is then a separate, later decision. If the owner wants half the work, this is the order.

---

## Recommended Path

**Option B, in phases with an owner gate per phase**:

| Phase | Task (proposed ids) | Risk | Breaks |
|---|---|---|---|
| 0 | T102 spike: prove `stream-json` stdin and the PreToolUse `updatedInput.answers` on the installed `claude`, in a temp home | Medium | nothing (spike) |
| 1a | T103 app session *analyze*: free prompt, repo picker, read tools, show-never-store reply text (DDR-0006) | High | NFR-002 (4th exception), FR-102 |
| 1b | T104 app session *update*: worktree + branch, edit tools, Stop, diff summary | High | NFR-003 |
| 2a | T105 `hooks install/remove` + the `hook ask` command + socket | High | write-scope rule |
| 2b | T106 Live view answer form (options + Other), **Answer in terminal** | Medium | FR-106 (option labels read) |
| 3 | T107 Stop-hook reply for *your turn* (owner gate; only after 2a has run for a week) | High | — |

It covers both halves of the request, uses only documented interfaces, every phase fails open, and each phase can be undone alone. Options C and D can lose data or misfire silently, and E is a preview.

---

## Owner correction (2026-11-17): CLI parity

Owner: "I means what we can do with the claude CLI, we can do with our application". This replaces the narrow profiles above (*analyze* = read tools; *update* = edits on a worktree, no `Bash`). The goal is **parity**: the app is a second front end to the owner's own `claude` binary, with the same powers and the same safety prompts as the terminal.

| What the CLI does | How the app does it (documented route) | Phase |
|---|---|---|
| Start a session in a folder, type prompts, follow-ups, slash commands (`/wake`, `/verify` …) | The app owns the process: `claude -p --input-format stream-json --output-format stream-json --verbose` in the chosen repo; each owner message is one stdin line | P1 |
| Any tool (`Bash`, `Edit`, MCP servers, sub-agents) under the owner's own settings | No `--allowedTools` narrowing: the owner's `settings.json` permissions apply, as in the terminal | P1 |
| Permission prompts ("allow this Bash command?") | `--permission-prompt-tool mcp__easy-personal__permission_prompt`: Claude Code asks this app's MCP server, and the app shows Allow / Deny in the session panel (to verify in the spike) | P1 |
| `AskUserQuestion` / plan approval | App-owned session: through the same permission-prompt tool, with `updatedInput.answers`. Terminal session: the hook bridge (Option B phase 2) | P1 / P2 |
| Choose model, permission mode, `--resume` a closed session, `--continue` | Fields in the *New session* form (fixed lists, never free argv); resume only a session that is not open anywhere (live tail says no record in the last N s) | P1 |
| Interrupt (Esc) / stop | Stop button: SIGINT, then SIGTERM to the process group | P1 |
| Answer a session that is **open in a terminal** | Hook bridge (PreToolUse `AskUserQuestion`, PermissionRequest); Stop-hook reply for *your turn* later | P2 / P3 |
| Account (`CLAUDE_CONFIG_DIR`) | The agent run account (FR-101), or a picker over confirmed Claude accounts | P1 |

What stays from the earlier plan: the environment scrub of FR-102 (no API key billing), the NFR-008 guard plus `confirmed` on every write route, nothing starts without an owner click, text shown and never stored (DDR-0006), loopback only, no MCP tool that starts sessions. A worktree becomes an **option** in the New session form, not a rule.

New risk this brings: a page fault, or another local page that passes the guard, can now run any command the owner's settings allow. Mitigation: permission prompts still reach the owner (the app never answers *allow* by itself, and never passes `--dangerously-skip-permissions` unless the owner picks that mode in the form, as they could in the terminal), one route-level confirm per session start, and the token never leaves the window.

---

## Surgical Scope

Files that **should** be touched (Rust app only; the Python app is retired-in-place):
- `rust/crates/app/src/agent_run.rs`, `agent_routes.rs`: app session runner and routes
- new `rust/crates/app/src/hooks_cmd.rs`, `answer_bridge.rs`: hook install/remove, `hook ask`, socket
- `rust/crates/core/src/live.rs`: option labels for an open `AskUserQuestion` (in memory only, same rules as `call_questions`, `live.rs:257`)
- `web/src/LiveView.jsx`, Settings and Warnings views: answer form and app-session panel
- `PRD.md` (FR-109+, NFR-003 / FR-102 / FR-106 amendments, questions 40+), `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md` constraints, `docs/ddr/0006-write-sessions.md`, `rust/README.md`

Files that **must not** be touched:
- anything under `~/.claude/projects/` (the app never writes transcripts; Claude Code does)
- `~/.claude.json` (connectors still go through `claude mcp add-json`)
- project-scope `.claude/settings.json` and hooks of any repo (kernel-owned)
- `skills/wake` (kit-managed), parity goldens, the Python app under `src/easy_personal`
- the MCP tool surface (no new MCP tool; the writes go through the dashboard only)

---

## Edge Case Checklist for TASK_GUIDE

- [ ] App not running, socket missing or stale: `hook ask` exits 0 in < 50 ms with no output; the terminal asks as today
- [ ] App and terminal both answer: the first answer wins; the other side gets "already answered"
- [ ] Hook timeout (Claude Code's 600 s default): our own limit is lower (120 s proposed), and we return nothing
- [ ] Multi-select answers: labels joined by `,` as the docs say; a label containing `,`
- [ ] **Other** free text: control and bidi characters removed (FR-092 list); a length cap
- [ ] Two sessions ask at once: answers keyed by session id + tool_use id, never mixed up
- [ ] Another local user or process opens the socket: peer uid check, mode 0600, a nonce per question
- [ ] `settings.json` missing, invalid JSON, or has other hooks: add-only, never reorder or drop; refuse on invalid JSON
- [ ] `hooks remove` after the owner edited the block by hand: remove only our exact entry
- [ ] Sessions started before install are not covered: the UI says so
- [ ] Update run: dirty repo, detached HEAD, branch name taken, worktree path exists
- [ ] Update run Stop: the process group is killed; the worktree stays for review; nothing is merged by the app
- [ ] Reply text and owner text never reach the store, logs, MCP, tray or a popup (leak gate extended)
- [ ] Agent run account turned off mid-session: the session stops (FR-101 rule kept)
- [ ] Environment scrub of FR-102 kept for every app session (no API key billing)
- [ ] A page fault posts to the write routes: per-action confirm, `confirmed: true`, one session at a time
