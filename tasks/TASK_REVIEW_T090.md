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
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
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

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
