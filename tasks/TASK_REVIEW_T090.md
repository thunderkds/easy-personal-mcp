# TASK_REVIEW — T090: Agent run 1: the setting, the runner and the `agent-run on|off|status|now` command

> Sibling of `tasks/TASK_GUIDE_T090.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Track extended, Risk High — RULE: `AGENTS.md` § Tracks
2. The setting is changed only by the CLI; the dashboard shows it — DEBATED: CLI only (Advocate 1: CLI only, dashboard read-only plus Run; Advocate 2: CLI only, an off-only browser switch may come later; Advocate 3: CLI only, `agent-run on|off|status`) ⚠ decided for you — please review
3. Accepted the Step 0 spike result: `--restricted` in place of `--setting-sources` (the help lists no value for "no source"), and stdout and stderr on `/dev/null` in place of read-and-drop; both keep the requirement (no settings file loaded, no output kept). The guide leaves the exact flags to the spike; AC 9 must still prove `--restricted` keeps the subscription login — RULE: `tasks/TASK_GUIDE_T090.md` § Step 0 — Spike
4. Code review (Supervisor, inline, scope `27c4abe..c5d5fbf`): 0 P0, 0 P1, 0 P2; P3: a run with no `--account` uses the default Claude folder even when this shell has `CLAUDE_CONFIG_DIR` set (as `claude_cli.rs` does; by the guide) — RULE: `skills/code-review/SKILL.md` § Workflow
5. Security review (T090 diff only): no High or Medium finding; needs validation at AC 9: `--restricted` keeps the `/login` subscription, and a tool outside the allow-list is refused without a prompt — RULE: `AGENTS.md` § Permanent Rules
6. An agent run keeps its Claude Code session on disk (no `--no-session-persistence`): the run's tokens show in the usage numbers; the transcript stays in the owner's Claude folder, written by Claude Code, never read by the app beyond the FR-002 metadata. Decided by the owner 2026-11-16 ("keep the run's claude session") after an unsettled Debate (Advocate 1: keep; Advocate 2: `--no-session-persistence`; Advocate 3: keep) — RULE: `skills/resolve-question/SKILL.md` § Debate
Checks: tests `cd rust && cargo test -p easy-personal-rs --bin easy-personal-rs -- agent_run claude_cli` — `test result: ok. 20 passed; 0 failed` (Supervisor, 2026-11-16); agent: `cargo test --workspace --no-fail-fast` — app unit tests 199 passed, 1 target failed (`connector_cli_parity`, 5 problems, same without the change: the installed .deb puts `easy-personal-launch` on `PATH`); `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --check` ok; reviews code-review 0 P0/P1/P2, security-review no finding; real run: none — AC 9 spends plan limits and is the owner's (auto-checked, not /verify)
Memory: `memory/learnings.md` — "2026-11-16 — T090 agent run: child process gotchas" (PDEATHSIG thread rule, flock and fork in tests, `connector_cli_parity` with the .deb installed, hook `Paused:` lines); `memory/MEMORY.md` — its index line
Not done: merge (admin: after the auto verify pass); auto verify: ☐ blocked (session 2b39b2f7-c674-4516-8c57-88725450638a, commit a695b04417835e206082d8fdecbacb0c8f02cc76, log .claude/verify-runs/T090-a695b0441783.log); AC 9 real run (yours: it spends plan limits); `html-report` pages and the companion score
AC 9 real run (2026-11-16, Supervisor, at the owner's request "make sure the claude invoke work", `main` `7f3aaef` debug build): `agent-run on` printed the notice; `agent-run now` with the default account printed `investigate: failed, started 2026-11-16T10:56:27Z, ended 2026-11-16T10:56:30Z, report none` (cause, seen once through a diagnostic wrapper on `PATH`: `Failed to authenticate: OAuth session expired and could not be refreshed` for `~/.claude`, the owner's login, not the app); `agent-run now --account hung2` printed `investigate: ok, started 2026-11-16T10:57:24Z, ended 2026-11-16T10:58:17Z, report 6`, exit 0, child `CLAUDE_CONFIG_DIR=/home/hungnguyenhuu/.claude-hung2`; `agent-run off` then `agent run: off`. Validates decision 5: `--restricted` keeps the `/login` subscription. The wrapper's output files were deleted.

## Step 0 — Spike input (Supervisor, 2026-11-16, from `claude --help` only; no model run)

Claude Code `2.1.325`. Flags the help lists (the agent confirms each against the fake-`claude` tests and records the final argv):
- `--mcp-config <configs...>` "Load MCP servers from JSON files or strings (space-separated)"
- `--strict-mcp-config` "Only use MCP servers from --mcp-config, ignoring all other MCP configurations"
- `--allowedTools, --allowed-tools <tools...>` "Comma or space-separated list of tool names to allow"
- `--tools <tools...>` "Use \"\" to disable all tools"
- `--permission-mode <mode>` choices: "acceptEdits", "auto", "bypassPermissions", "manual", "dontAsk", "plan" — `dontAsk` is the candidate for "refuse, never ask"
- `--permission-prompts <target>` "Who answers permission prompts"
- `--setting-sources <sources>` "Comma-separated list of setting sources to load (user, project, local)"
- `--no-session-persistence` (only with `--print`)
- `--output-format` (only with `--print`): text, json, stream-json
- Not used: `--bare` ("OAuth and keychain are never read"), `--dangerously-skip-permissions`, `bypassPermissions`

### Step 0 — confirmed by the agent (backend-developer, 2026-11-16, `claude --version` and `claude --help` only; no model run)

`claude --version` → `2.1.325 (Claude Code)`. Every flag above is listed. Choices for the runner:
- MCP: `--mcp-config <one inline JSON string>` + `--strict-mcp-config`.
- Allow-list: `--allowedTools <one comma-separated argument of the five mcp__easy-personal__* names>`.
- No built-in tool: `--tools ""` (help: `Use "" to disable all tools`).
- Refuse, never ask: `--permission-mode dontAsk` plus `--permission-prompts none` (help: `"none" (nobody: anything that would prompt is denied automatically; the permission mode still decides everything else)`). A listed flag gives "refuse, never ask", so this is not blocked.
- Settings files: **not** `--setting-sources`. The help gives no documented value for "no source" (unlike `--tools ""`); an empty value could be read as "not given" and load the user file, whose `env` or `apiKeyHelper` could bill an API key. The help lists `--restricted`: "ignores user, project and local settings files (managed settings and --settings still apply; add --strict-mcp-config to skip MCP servers too)" and removes the code-running built-in tools. The runner uses `--restricted`. It does not name OAuth (`--bare` is the flag that "never reads OAuth credentials"), so the `/login` subscription is expected to stay; only AC 9 (the one real run) proves it.
- Not used: `--no-session-persistence` (the run's session is kept, so the next ingest counts it like any other session), `--output-format` (the output is dropped), `--setting-sources`, `--bare`, `--dangerously-skip-permissions`, `--allow-dangerously-skip-permissions`.

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☐ blocked | auto-run by hook, session 2b39b2f7-c674-4516-8c57-88725450638a, commit a695b04417835e206082d8fdecbacb0c8f02cc76, log .claude/verify-runs/T090-a695b0441783.log — blocked (no single verdict line) |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (backend-developer, worktree `T090` at `1cfb16a`, before any implementation commit; `-p easy-personal-rs` alone
is refused by cargo because the package has two binaries, so `--bin easy-personal-rs` is added):

```
$ cd rust && date -u; cargo run -q -p easy-personal-rs --bin easy-personal-rs -- agent-run status; echo "exit=$?"
Mon Nov 16 07:20:45 AM UTC 2026
error: unrecognized subcommand 'agent-run'

Usage: easy-personal-rs <COMMAND>

For more information, try '--help'.
exit=2
```

**AFTER** (backend-developer, same command on branch `T090`; `EASY_PERSONAL_HOME` set to a temp folder so `~/.easy-personal` is not opened):

```
$ cd rust && export EASY_PERSONAL_HOME=$(mktemp -d)/data; date -u; cargo run -q -p easy-personal-rs --bin easy-personal-rs -- agent-run status; echo "exit=$?"; cargo run -q -p easy-personal-rs --bin easy-personal-rs -- agent-run now; echo "exit=$?"
Mon Nov 16 07:51:39 AM UTC 2026
agent run: off
claude: found
run going: no
exit=0
easy-personal-rs: agent runs are off; turn them on with `easy-personal-rs agent-run on`
exit=1
```

**DELTA**: The owner can turn agent runs on with `easy-personal-rs agent-run on` and start the `investigate` job with `agent-run now`, with no prompt typed in a Claude session. The real model run is AC 9, not done here.

### Agent notes (backend-developer)

**The app is killed during a run** (checked with the built binary and a fake `claude` that starts `sleep 30 &`; no model run):

```
Mon Nov 16 07:48:43 AM UTC 2026
Agent runs use the Claude subscription login of Claude Code on this machine and count against your plan limits.
agent run: on
status while the run goes:
agent run: on
claude: found
run going: yes
kill -9 the app (pid 3782916)
pid 3782918: gone
grandchild 3782919: still runs
status after the kill:
agent run: on
claude: found
run going: no
a new run takes the lock (fake ends at once this time):
investigate: ok, started 2026-11-16T07:48:44Z, ended 2026-11-16T07:48:44Z, report none
exit=0
agent run: off
```

- `claude` itself dies with the app: the child is started with `PR_SET_PDEATHSIG = SIGKILL`. This also covers Ctrl-C on `agent-run now`, because the child is in its own process group and gets no SIGINT from the terminal.
- A process that `claude` started lives on after a `kill -9` of the app (the fake's `sleep 30`). In a real run that is the `easy-personal-rs mcp` server; it is expected to exit when its stdin closes, but no real run proved this.
- The lock is `flock`: the kernel frees it when the app dies, so no stale lock blocks the next run.
- `PR_SET_PDEATHSIG` follows the thread that started the child. T091 must start a run from a thread that lives until the run ends.

**The output is dropped** by giving the child `/dev/null` for stdout and stderr, not pipes that are read and dropped. Nothing reaches this process, and a full pipe cannot block the child.

**Pre-existing failure, not from T090**: `cargo test -p easy-personal-rs --test connector_cli_parity` fails on this machine with 5 problems in the `*_no_launcher` cases. The `easy-personal-rs` .deb installed `/usr/bin/easy-personal-launch`, so the launcher is found on `PATH`. The same test fails the same way with the T090 changes stashed.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]

**Leak gate fix (follow-up, backend-developer, 2026-11-16)**: `rust/parity/leak_rust.py` `CLI_CALLS` now lists `agent-run status` (0), `agent-run on` (0), `agent-run status` (0), `agent-run off` (0), then `agent-run now --workspace <key>` (1: the setting is off, nothing starts) and `agent-run nope` (2). The calls run in list order with the gate's own environment (`EASY_PERSONAL_HOME` = its temp `data` folder), so `on` writes no flag in the owner's folder. `agent-run now` with the setting on is never listed. An unknown subcommand uses a fixed name: clap echoes the name in its error, and `{key}` there was a canary hit (3 hits in the first try).

```
$ cd rust && env -u CLAUDE_CONFIG_DIR /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/rust/parity/.venv/bin/python parity/leak_rust.py; echo "exit=$?"
Mon Nov 16 08:30:36 AM UTC 2026
check cli commands all registered   ok (23 commands)
check cli exit codes as expected    ok (103 calls, one process each)
check cli connector/doctor exit codes as expected      ok (29 calls, one process each)
check planted sources unchanged      ok (sha256 of the tree)
leak check: 0 hits over 42 canaries, 840 surfaces, 1798257 bytes
exit=0
```
