# TASK_GUIDE — T130: Terminal in the app 0 — spike: the real `claude` in a PTY, drawn by xterm.js
**Date**: 2026-11-20
**Complexity Level**: C2
**Risk Level**: Medium (spike: throwaway code in the scratchpad only; runs the owner's real `claude` in a temp repo)
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` FR-109..FR-116, FR-121..FR-128, NFR-002, NFR-008, question 42, the "Write sessions" cut list (line ~296)
2. `docs/ddr/0006-write-sessions-cli-parity.md` (the "Embedded PTY running the real TUI" cut item), `docs/ddr/0007-live-transcript-show-never-store.md`, `BRAINSTORMING_LOG_LIVENATIVE.md` Path C
3. `rust/crates/app/src/app_session.rs` (`child_env`, how a child is started and cleaned up), `rust/crates/app/src/session_procs.rs` (how Live finds a terminal session), `tasks/evidence-T126/SPIKE.md` § How it ran
- The spike writes **no app code**: everything it builds lives in the session scratchpad and is deleted or left there. The only repo change is `tasks/evidence-T130/SPIKE.md` (+ this task's review file).
- Never read the owner's real `~/.claude*` transcripts or files, any `*.key`, or a Claude Code messaging socket. Check only file existence / record `type` counts / `entrypoint` values in a project folder the spike itself created. Kill only processes the spike started; confirm none is left.

## Requirement
Owner 2026-11-20: "how about when the application new session, we can show the terminal inside the app, and it will work like the terminal, it mean that, the app will wrap the terminal, so the normal flow will going in the app, and the app don't care too much, that is my idea, let check that" → "go with both" (T129 fix + Stage 0 for this idea, spike first). This reverses DDR-0006's cut item "Embedded PTY running the real TUI" only after this spike and a DDR.

## Dependencies & Reachability
**Depends on**: none
**Entry point**: Standalone — N/A (spike; evidence document only)

## Acceptance Criteria (spike questions — each answered GO / NO-GO with timestamped evidence)

| # | Question | Evidence |
|---|---|---|
| 1 | A Rust PTY (prefer `portable-pty`; say its version, license and dependency weight) starts the interactive `claude` in a scratchpad temp git repo with an account's `CLAUDE_CONFIG_DIR` (the same env scrub as `child_env`) and keeps running | process up, first screen bytes read |
| 2 | Typed input reaches it and it answers (a tiny prompt, haiku); the TUI's own permission dialog appears and can be answered by keystrokes | bytes in/out log (no conversation text pasted beyond the test prompt and "OK") |
| 3 | It renders correctly in `xterm.js` in a browser over a throwaway local bridge (WebSocket or SSE+POST): colours, cursor, resize (SIGWINCH) | screenshots at 1280 and 375 px in the scratchpad |
| 4 | The run is saved by Claude Code as an interactive session (`entrypoint` value, file under the account's `projects/<munged cwd>/`), and `claude --resume <id> -p "…"` resumes it | entrypoint value, record type counts only |
| 5 | Live would see it as a terminal session: Claude Code writes `<config>/sessions/<pid>.json` for it (name the fields `session_procs.rs` reads), and the answer-bridge hooks (FR-113/FR-121) fire for it when installed in that config | file present + fields; hook fired yes/no |
| 6 | Closing the PTY (and killing the bridge) ends `claude` cleanly; nothing is left; a page reload can re-attach if the server keeps a scrollback buffer in memory (measure a sensible size) | `pgrep` after; buffer bytes for one short session |
| 7 | Cost of the page side: `xterm.js` (+ fit addon) version, license, size gzipped, and whether it passes `scripts/check_static.py` budgets | numbers |

## Approach
Throwaway: a tiny Rust bin (or test) in the scratchpad using `portable-pty`, plus a static HTML page with `xterm.js` from the repo's `web/node_modules` if present, else a pinned tarball checked against the npm registry hash. Answer each question; write `tasks/evidence-T130/SPIKE.md` (like `tasks/evidence-T126/SPIKE.md`): how it ran, answers table, findings → what the build would need (routes, guard, lifecycle, buffer, risks), and open questions for the owner.

## Files to change (predicted)
| File | Change |
|------|--------|
| `tasks/evidence-T130/SPIKE.md` | new: the spike's evidence and findings |
| `tasks/TASK_REVIEW_T130.md` | evidence rows |

## Files must NOT touch
| File | Reason |
|------|--------|
| `rust/**`, `web/**`, `src/**` | spike: no app code |
| any real `~/.claude*` file, any `*.key`, Claude Code's messaging socket | DDR-0001, DDR-0007 |

## Test plan
Each AC answered with evidence; no app tests change.

## Verification Command
```bash
test -s tasks/evidence-T130/SPIKE.md && grep -c "GO" tasks/evidence-T130/SPIKE.md && pgrep -af "claude" | grep -c scratchpad || true
```

## Completion checklist
- [ ] All 7 questions answered GO / NO-GO with evidence
- [ ] No spike process left; nothing outside the scratchpad changed except the evidence file
- [ ] Findings list what the build needs and the risks
- [ ] `memory/` updated (Supervisor)
