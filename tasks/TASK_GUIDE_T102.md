# TASK_GUIDE — T102: Write sessions 0 — spike: prove the three Claude Code routes on the installed `claude`
**Date**: 2026-11-17
**Complexity Level**: C2
**Risk Level**: Medium (spike; nothing merged into the app)
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md` (Critical Constraints, DDR-0006 line)
2. Read `BRAINSTORMING_LOG_WRITESESSIONS.md` (all, especially "Owner correction"), `docs/ddr/0006-write-sessions-cli-parity.md`, `PRD.md` FR-109..FR-116
3. Read `rust/crates/app/src/agent_run.rs` (how a `claude -p` run is built, scrubbed and stopped today), `rust/crates/app/src/mcp.rs`, `rust/crates/app/src/mcp_http.rs`
- Never edit any real Claude Code settings file and never copy login files. Give hooks and the permission tool to `claude` only through `--settings <temp file>` and `--mcp-config <temp file>` (with `--strict-mcp-config`), and run each probe in a scratch git repo under `/tmp`. Use the owner's default login as it is. Each probe costs a few plan tokens: keep prompts tiny ("say ok"). Interactive probes (AC 3, 6) run under `script`/a PTY or tmux that you start yourself; if that is not possible headless, say so and give the owner the exact commands.

## Requirement
Owner 2026-11-17: "what we can do with the claude CLI, we can do with our application". Before any build task, prove on the installed `claude` (record `claude --version`) the routes FR-109..FR-114 depend on.

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | **stream-json input**: a script starts `claude -p --input-format stream-json --output-format stream-json --verbose`, sends two user messages one after the other on stdin, and receives two assistant results; the exact stdin line shape and the output event types (`system`/`assistant`/`user`/`result`, session id field) are written down | FR-109, FR-110 |
| 2 | A slash command sent as a user message (`/help` or a project skill) behaves as in the CLI, or the difference is written down | FR-110 |
| 3 | **SIGINT** during a long answer: does the process stop the turn and wait for the next stdin line (Esc behaviour), or exit? Written down | FR-110 |
| 4 | **`--permission-prompt-tool`**: with a tiny MCP server exposing one tool, a `Bash` call that needs permission reaches the tool; the exact input shape and the accepted result shapes (`allow` + `updatedInput`, `deny` + `message`) are written down. Same for `AskUserQuestion` (does it reach the tool? is `updatedInput.answers` honoured?) and `ExitPlanMode` | FR-111 |
| 5 | Which transport the permission tool can use so that it reaches the **already running app**: (a) the app's MCP HTTP server (`mcp_http.rs`) given in `--mcp-config` as a URL, or (b) a stdio child (`<binary> mcp`) that forwards over a socket. Recommend one with the reason | FR-111 |
| 6 | **Hooks** in the temp config dir: a PreToolUse entry (matcher `AskUserQuestion`) whose command prints `updatedInput.answers` answers an interactive session's question with no terminal UI shown; a PermissionRequest entry that allows/denies a `Bash` prompt; a command that exits 0 with no output leaves the terminal prompt as usual. Measure: does the terminal show anything while the hook waits? | FR-113, FR-114 |
| 7 | `--resume <id>` with `--fork-session` produces a new session id and leaves the original transcript unchanged | FR-112 |
| 8 | Findings in `tasks/evidence-T102/SPIKE.md` (versions, exact commands, JSON samples with any text replaced by `<text>`), and a go/no-go per route; any route that fails → `Status: blocked` with the question for the owner | all |

## Approach
Throwaway scripts under `rust/spikes/write-sessions/` (Python or shell is fine for a spike). No change to `rust/crates/`.

## Files to change (predicted)
| File | Change |
|------|--------|
| `rust/spikes/write-sessions/*` | probe scripts |
| `tasks/evidence-T102/SPIKE.md` | findings |

## Files must NOT touch
| File | Reason |
|------|--------|
| `~/.claude/settings.json`, `~/.claude.json`, any real config dir | spike runs in temp homes only |
| `rust/crates/**` | spike only |
| `src/easy_personal/**` | Python app untouched |

## Test plan
The spike is its own evidence: each AC has a command and its output sample in `SPIKE.md`.

## Verification Command
```bash
test -s tasks/evidence-T102/SPIKE.md && grep -cE '^\| *[0-9]+ ' tasks/evidence-T102/SPIKE.md && ls rust/spikes/write-sessions/
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes (n/a for spike scripts)
- [ ] Tests pass (n/a)
- [ ] Smoke test confirmed (each probe re-run once)
- [ ] `docs/` updated (FR-111 wiring fixed in PRD if the spike changes it)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
