# TASK_REVIEW — T024: A Rust step in the /verify recipe

> Sibling of `tasks/TASK_GUIDE_T024.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` or `[admin]` tag; with `[admin]`, the `Not done:` line names `merge (admin: after the auto verify pass)`,
`docs/claude-md/tracks.md` `## Auto mode`). To use it, copy the block below out of this comment, to this
place, and replace every <...>. Keep `Mode: auto` as its first line, exactly as written: T182's hook reads
it. Write it to this file before the run ends; the chat message is a copy, not the report. The same
block is the Supervisor's final chat message. It is not the `delivery-report` page and not
Evidence. A taste decision (what the user wants) also ends with the flag. Write `DEBATED` only when ≥2
Advocate `Agent` calls ran for that question, naming each Advocate's option. A decision with no Advocates is
labelled `RULE`, `FACT` or `SAFER OPTION`, never `DEBATED`. A decision flagged `⚠ decided for you` is a choice, and is never labelled `FACT`. A
safety action a rule requires (for example the backup before a delete) is `RULE` without the flag.
At Risk Medium or High, a `RULE:` counts only when it names a written rule (`path:line`,
`path § <heading>`, or `docs/claude-md/tracks.md` `### <section>`), and a `FACT:` needs a Citation
(`path:line "quoted text"`); without one it is a choice and must be `DEBATED`. Good: `2. backed up the CSV
— RULE: `docs/claude-md/tracks.md` `### Stops that stay``. Bad: `1. kept recurring.csv — RULE: Surgical Changes`.
`Memory:` records the diff-driven memory pass you run before you stop: each entry written, or `nothing new`.
After an auto verify pass the hook, never you, adds `Closed: row moved to Done <date> — …` and moves the row,
or adds `Paused: done not moved — <reason>`. It moves the row only when this block holds `Mode: auto`, a
numbered decision, `Checks:`, `Memory:`, the `Not done:` auto verify slot with its result, and `Not done:`
naming the merge — no `<...>` left (`docs/claude-md/tracks.md` `### Auto close-out`).

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its proposal>; Advocate 2: <its proposal>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Memory: <each entry the memory pass wrote, as `file — entry` | nothing new>
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours)>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ N/A | text-only change to `skills/verify/SKILL.md`: no test file. The commands in the new section are the test, and each was run (agent, then Supervisor) |
| Verification command run | ☑ pass | guide command: `git diff --stat -- skills/verify/SKILL.md` = `1 file changed, 53 insertions(+)`; removed lines (`grep -c '^-[^-]'`) = 0. Supervisor re-ran the whole new section as written, extracted from the file, 2026-11-11T04:03:13Z: build `Finished`, `--version` `easy-personal-rs 0.1.0`, `store-info` `user_version: 11`, `serve` + `/api/version` `{"version":"0.1.0"}`, no token `403`, `stopped <pid>`, `mcp` ids 1-3 answered, window step `self-test: ok`, `exit=0` |
| Negative cases hold | ☑ pass | no token gives `403` (Supervisor run); no display gives `window step skipped: no display` and never "pass" (agent run under `env -u DISPLAY -u WAYLAND_DISPLAY`); a run under `zsh -e` died at `wait` after `kill` (agent found it, now a Gotchas bullet); after both runs `ps` shows 0 `easy-personal-rs` processes |
| verify | ☑ pass | the thing under test is the recipe itself: Supervisor ran the new Rust section as written (rows above), the Python sections were not touched (0 removed lines). Not run: the `cargo`-not-found path (`cargo` was on PATH) and a cold `target/` build |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | only `skills/verify/SKILL.md` and the BEFORE field changed; read the whole added diff. Lane check against `HEAD` printed `EXCEEDS (76 files)` only because the working tree holds 75 unrelated kit-update files; the T024 change is 1 file, 53 lines. code-review: 0 P0, 0 P1, 2 P2 (stale references in `tasks/TASK_REVIEW_T023.md` and the T023 board card, fixed by the Supervisor), 3 P3. Security review not required (Low risk) |
| Full smoke suite still green (no regression) | ☑ N/A | no `rust/`, `src/`, `web/` or `tests/` file changed (`git status`), so no suite was run; the section's own run exercised build, `store-info`, `serve`, `mcp` and the window |
| **Companion score (easy-verifier)** | ☑ N/A | `score` not run in this session |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no UI |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no UI |
| **UI: Responsiveness at target viewports** | ☑ N/A | no UI |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: verbatim prior content of `skills/verify/SKILL.md` (a skill instruction text; no executable code changes), quoted 2026-11-11 before the first edit of this run. It has no Rust step:

````markdown
---
name: verify
description: How to build, launch and drive easy-personal-mcp for runtime verification (CLI, MCP stdio, MCP HTTP). Read by /verify before a cold start.
---

# Verify recipe — easy-personal-mcp

Run from the task's worktree. Isolate everything in a temp dir so the owner's real install and data stay untouched.

```bash
T=$(mktemp -d)
export EASY_PERSONAL_HOME=$T/home UV_TOOL_DIR=$T/tools UV_TOOL_BIN_DIR=$T/bin
uv tool install -q .
B=$T/bin/easy-personal
$B --version            # package version
$B ping                 # core reachable from the CLI
```

## MCP stdio (the default surface)

Pipe a real handshake; keep stdin open a moment so responses flush:

```bash
printf '%s\n' \
 '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"verify","version":"0"}}}' \
 '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
 '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
 '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ping","arguments":{}}}' \
 | (cat; sleep 2) | timeout 10 $B mcp 2>$T/err
```

## MCP HTTP (opt-in)

```bash
($B mcp --transport http --port 18765 >$T/http.log 2>&1 &); sleep 3
ss -ltnp | grep ':18765 '          # must show 127.0.0.1 only
curl -s -X POST http://127.0.0.1:18765/mcp -H 'content-type: application/json' \
  -H 'accept: application/json, text/event-stream' -d '<initialize JSON as above>'
pkill -f "$B mcp --transport http --port 18765"
```

## Probes worth repeating

- `--host 0.0.0.0`, `localhost`, `::1` → refused, exit 2 (NFR-001).
- Bad `--port` / `--transport` → argparse error, exit 2.
- Planted-canary fixtures (once sources exist): grep every surface's output for the canary (NFR-002).

## Gotchas

- The Bash tool runs zsh: `$B $args` does not word-split a string. Wrap probe loops in `bash -c '...'`.
- `serverInfo.version` in `initialize` is the MCP SDK version, not the package version.
- The MCP server logs `Processing request of type …` to stderr on every request; stdout carries protocol only.
- Never point verification at the owner's real `~/.claude/projects/` with content-printing commands; use the fixtures under `tests/fixtures/`.
````

**AFTER**: the new `## Rust (easy-personal-rs)` section in `skills/verify/SKILL.md` (53 added lines, after `## MCP HTTP (opt-in)` and before `## Probes worth repeating`: four fenced blocks for build + `--version` + `store-info`, `serve` + one authenticated request + PID stop, `mcp` with `usage_summary`, and a display-gated window self-test, plus 5 Gotchas bullets). Running it as written on 2026-11-11T04:03:13Z printed `easy-personal-rs 0.1.0`, `user_version: 11`, `{"version":"0.1.0"}`, `403`, `stopped <pid>`, three MCP answers, `self-test: ok` and `exit=0`.

**DELTA**: `/verify` can now exercise the Rust binary (build, store, server with token guard, MCP, window self-test) from the recipe itself, with a skip line when there is no display.

**WITNESS**: Supervisor re-ran the section 2026-11-11T04:03:13Z after the implementing agent; tool calls recorded in `memory/event-trace/T024.jsonl`.
