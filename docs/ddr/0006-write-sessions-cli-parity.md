# 0006. Write sessions: the app becomes a second front end to the owner's Claude Code (CLI parity)

**Status**: Accepted · item 4 "an embedded PTY" amended by DDR-0009 (2026-11-20)
**Date**: 2026-11-17
**Deciders**: thunderkds (owner), Supervisor
**Related**: FR-109..FR-116 · NFR-002, NFR-003, NFR-008 · FR-101, FR-102, FR-106 · US-041, US-042 · DDR-0001, DDR-0005 · PRD questions 36-43 · `BRAINSTORMING_LOG_WRITESESSIONS.md`

---

## Context

Since its first release the app has been read-only toward Claude Code. It reads transcripts, never writes into a session, and starts `claude` only for one fixed, five-tool job (FR-102). The owner asked on 2026-11-17 to break that rule: "we should support the write session in the app, it means that, the app will break the readonly rules", then "what we can do with the claude CLI, we can do with our application".

Claude Code documents three routes the app can use: driving its own `claude -p` process over stream-json; a `--permission-prompt-tool` for the prompts of that process; and user-scope hooks (PreToolUse `updatedInput.answers` for `AskUserQuestion`, PermissionRequest) for sessions open in a terminal. It documents that `--resume` on a session open elsewhere loses messages. There is no documented way to type into an open terminal session.

Gate: (1) hard to reverse — partly: the hook entries written to a user's settings file outlive the app unless removed; (2) surprising without context — yes; (3) genuine trade-off — yes (five paths weighed). 3 of 3: DDR.

---

## Decision

1. **App sessions with CLI parity.** The app starts the owner's own `claude` in a stored repo (or a worktree it makes on request). It applies the owner's own settings, permissions, tools, MCP servers and slash commands, and narrows no tool. Prompts and questions reach the app through `--permission-prompt-tool`; the app never answers them by itself. The FR-102 environment scrub stays.
2. **Answer bridge for terminal sessions.** Opt-in per account: the app adds, add-only and after showing the diff, a PreToolUse (`AskUserQuestion`) entry and a PermissionRequest hook entry to that account's user `settings.json`. The hook asks the running app over an owner-only socket and fails open to the terminal.
3. **Text shown, never stored.** This is the fourth exception to NFR-002. Prompts, messages, answers, assistant text, question options, plan text and a one-line permission summary live in memory and appear only on guarded routes and the page.
4. **Not done**: typing into an open terminal, `--resume` of an open session without fork, Channels, an embedded PTY, any MCP tool that drives a session, any start without an owner click.

---

## Alternatives Considered

| Alternative | Why not chosen |
|---|---|
| Narrow profiles (read tools; edits on a worktree, no `Bash`) | Chosen first the same day, then replaced by the owner: "what we can do with the claude CLI, we can do with our application" |
| `--resume` / `--fork-session` into open sessions | Documented message loss; the terminal never sees the reply |
| tmux `send-keys` | Only inside tmux; keystrokes can go to the wrong pane |
| Channels | Research preview; flag at launch; dev flag ignored under `-p` |
| Embedded PTY running the real TUI | Full fidelity, but a terminal emulator in the app; cut list |

---

## Consequences

- NFR-003 is amended. The app's own readers stay read-only, but an app session changes what the owner's settings allow, as a terminal session does.
- The `PROJECT_SPEC.md` write-scope constraint gains `<CLAUDE_CONFIG_DIR>/settings.json` (hook entries only, add-only, opt-in). `~/.claude.json` stays DDR-0001.
- The guard (NFR-008) now protects a route that can run any command the owner's settings allow. Kept against it: `confirmed` on every write route, fixed-list form fields only, permission prompts always reach the owner, and the token never goes in a URL. A page fault that passes the guard could start a session; it cannot skip a permission prompt unless the owner picked such a mode.
- The leak suite must cover every FR-115 text: present on the guarded routes, absent from the store, logs, MCP, CLI, `/api/alerts`, tray and popups.
- A bug in `hook ask` touches every session on the machine. It must fail open in < 50 ms with no app running, and that path is a hard test.
- The JSONL transcript format is documented as internal and liable to change. App sessions read the documented stream-json instead.
- The Python app is not changed.
