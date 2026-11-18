# TASK_GUIDE — T110: Reply to idle sessions 0 — spike: prove the Stop-hook reply on the installed `claude`
**Date**: 2026-11-18
**Complexity Level**: C2
**Risk Level**: High (the feature changes how every turn of every terminal session ends; this spike merges nothing into the app)
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md` (Critical Constraints, DDR-0006 line)
2. Read `PRD.md` question 42 (decided 2026-11-18), FR-105 (*your turn*), FR-113..FR-115 (the answer bridge), `docs/ddr/0006-write-sessions-cli-parity.md`
3. Read `tasks/evidence-T102/SPIKE.md` (how the T102 spike probed hooks: temp settings, scratch repo, the interactive probe) and `rust/spikes/write-sessions/p6_hooks.py`, `hook.py`
4. Read `rust/crates/app/src/answer_bridge.rs` (how `hook ask` talks to the app) — read only
- Never edit any real Claude Code settings file and never copy login files. Give hooks to `claude` only through `--settings <temp file>`, and run each probe in a scratch git repo under `/tmp`. Use the owner's default login as it is. Each probe costs a few plan tokens: keep prompts tiny ("say ok"). Interactive probes run `claude` under a PTY (as T102's `p6_hooks.py` does), never in the owner's terminal.

## Requirement
Owner 2026-11-18: "if the agent is idle, I can input the request to the live session also, can we do that" → "plan it and update [admin]" (PRD question 42, option A): when a terminal session ends its turn, a `Stop` hook may hold it and return the owner's typed request as `{"decision": "block", "reason": "<text>"}`, so Claude goes on in the same terminal session. Before any build task, prove this on the installed `claude` (record `claude --version`) and measure what the terminal can do while the hook holds.

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | **Block with reason**: a Stop hook that prints `{"decision":"block","reason":"say the word banana"}` makes Claude start a new turn that acts on the text (the reply contains "banana"); write down how the text appears in the transcript (record type, role, any prefix such as "Stop hook feedback") with text replaced by `<text>` | Q42 |
| 2 | **Loop guard**: the Stop hook input fields (record the full key list); `stop_hook_active` is `true` on the Stop that follows a blocked one; a hook that blocks only when `stop_hook_active` is false runs the session exactly one extra turn | Q42 |
| 3 | **No reply**: a Stop hook that exits 0 with no output ends the turn as usual (terminal back at its prompt), and its added latency with the hook holding 0 s is measured (< 100 ms expected) | Q42 |
| 4 | **Terminal while the hook holds** (interactive, PTY): with a Stop hook that sleeps 20 s then exits 0, write down what the terminal shows; whether text typed meanwhile is (a) accepted into the input box, (b) sent as the next user message after the hook returns, or (c) lost; and whether **Esc** or **Ctrl+C** ends the hook early and what Claude Code does then (hook killed? turn ends? message sent?) | Q42 |
| 5 | **Hook timeout**: with `timeout: 30` and a hook that sleeps 60 s, what happens at 30 s (turn ends as with no output? an error line in the terminal?) | Q42 |
| 6 | **Text shape**: a reason of 4,000 characters with line breaks, and one with a leading `/` (`/help`), each reach Claude as plain text (slash commands are not run) or the difference is written down | Q42, FR-115 |
| 7 | **Headless**: in `claude -p` the same Stop hook block also continues the run (one extra turn) — so a `-p` session is not a target the app must exclude — or the difference is written down | Q42 |
| 8 | Findings in `tasks/evidence-T110/SPIKE.md` (versions, exact commands, JSON samples with any text replaced by `<text>`), a go/no-go, and a recommendation for the hold rule with its reason, chosen from: (i) hold every turn end while a Live page is watching, up to a cap; (ii) hold only for a session the owner marked "reply from the app" in the Live view; (iii) another rule the facts suggest. Any blocker → `Status: blocked` with the question for the owner | all |

## Approach
Throwaway scripts under `rust/spikes/reply-sessions/` (Python or shell, as T102). No change to `rust/crates/` or `web/`.

## Files to change (predicted)
| File | Change |
|------|--------|
| `rust/spikes/reply-sessions/*` | probe scripts |
| `tasks/evidence-T110/SPIKE.md` | findings |

## Files must NOT touch
| File | Reason |
|------|--------|
| `~/.claude*/settings.json`, `~/.claude.json`, any real config dir | spike runs with temp settings only |
| `rust/crates/**`, `web/**` | spike only |
| `src/easy_personal/**` | Python app untouched |

## Test plan
The spike is its own evidence: each AC has a command and its output sample in `SPIKE.md`.

## Verification Command
```bash
test -s tasks/evidence-T110/SPIKE.md && grep -cE '^\| *[0-9]+ ' tasks/evidence-T110/SPIKE.md && ls rust/spikes/reply-sessions/
```

## Completion checklist
- [ ] Each AC has its command and recorded output in `SPIKE.md`
- [ ] Go/no-go and the hold-rule recommendation written
- [ ] No real settings file changed (`git status` of the owner's config dirs untouched; the spike used `--settings` only)
- [ ] Supervisor notified: task ready for Stage 4 review
