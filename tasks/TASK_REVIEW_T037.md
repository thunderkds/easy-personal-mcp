# TASK_REVIEW — T037

> Sibling of `tasks/TASK_GUIDE_T037.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | [paste the red run against the prior content, or the mutation diff (`git diff --stat`, non-empty) before each verdict] |
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

**BEFORE** (verbatim from `skills/verify/SKILL.md` at `548bd15`, taken before any edit: the frontmatter `description` line, then the Rust step, lines 42-88):

`````text
description: How to build, launch and drive easy-personal-mcp for runtime verification (CLI, MCP stdio, MCP HTTP). Read by /verify before a cold start.

## Rust (easy-personal-rs)

For a change under `rust/`. No `cargo` on the machine: skip this section and say so in the report. Every command runs from `rust/`. If `cargo` is not found, run `. "$HOME/.cargo/env"` first (rustup installed without editing the shell profile). The temp dir keeps the owner's store untouched: the recipe reads a copy of the committed synthetic store, never `EASY_PERSONAL_HOME`'s default.

```bash
T=$(mktemp -d); mkdir $T/home
export EASY_PERSONAL_HOME=$T/home
cp parity/store-v11.sqlite $T/home/store.sqlite
cargo build                                       # default build: no window, no display needed
B=$PWD/target/debug/easy-personal-rs
$B --version                                      # easy-personal-rs <Cargo version>
$B store-info                                     # user_version 11, row counts, no rows, no path
```

`serve` on the fixture page; read the token into a variable and never echo it. Stop by PID:

```bash
$B serve --static-dir crates/app/tests/fixtures/self-test >$T/serve.out 2>$T/serve.err & P=$!
for i in 1 2 3 4 5; do grep -q '^dashboard: ' $T/serve.out && break; sleep 1; done
U=$(grep -m1 '^dashboard: ' $T/serve.out); U=${U#dashboard: }
TOKEN=${U##*#}; BASE=${U%%/#*}                    # URL is http://127.0.0.1:<port>/#<token>
curl -s -w '\n' -H "x-dashboard-token: $TOKEN" $BASE/api/version    # {"version":"<Cargo version>"}
curl -s -o /dev/null -w '%{http_code}\n' $BASE/api/version  # no token: 403
kill $P; wait $P 2>/dev/null; kill -0 $P 2>/dev/null && echo "STILL RUNNING $P" || echo "stopped $P"
```

`mcp` over the temp store; the server ends when stdin closes:

```bash
printf '%s\n' \
 '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"verify","version":"0"}}}' \
 '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
 '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
 '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"usage_summary","arguments":{}}}' \
 | (cat; sleep 2) | timeout 10 $B mcp 2>$T/mcp.err | cut -c1-160
```

Window step, last (it replaces the default binary). Run it only when `DISPLAY` or `WAYLAND_DISPLAY` is set and the Tauri system packages (`rust/docs/spike-T001.md`) are installed; otherwise skip it and write "window step skipped: no display", never "pass":

```bash
if [ -n "$DISPLAY$WAYLAND_DISPLAY" ]; then
  cargo build --features window
  timeout 60 $B app --self-test --static-dir crates/app/tests/fixtures/self-test; echo "exit=$?"   # self-test: ok, exit 0
else echo "window step skipped: no display"; fi
```

macOS and Windows are not covered yet (the window and signal handling are Ubuntu only).
`````

**AFTER**: Added to the Rust step of `skills/verify/SKILL.md` (+18 lines, 106 -> 124): an `ingest` block (fixture `parity/ingest/fixtures/s3`, temp data and agent folders, ingest twice, `store-info`) and the leak-gate command with its pass condition; the frontmatter `description` now mentions the Rust port.
Supervisor run of both blocks from the recipe text, 2026-11-11: ingest 1 `files 2, new_events 3, skipped 0` exit 0; ingest 2 `new_events 0`; `store-info` `user_version: 11, known: 11, pending: no`; `leak check: 0 hits over 42 canaries, 116 surfaces, 786773 bytes`. (The agent also ran the ingest block under zsh and bash.)

**DELTA**: `/verify` now exercises the Rust second writer (ingest twice, second adds 0) and the privacy gate, not only the read side.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
