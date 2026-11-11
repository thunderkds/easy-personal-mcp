# TASK_REVIEW — T025: Fix the stale "no CLI/API yet" wording in rust/README.md

> Sibling of `tasks/TASK_GUIDE_T025.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ N/A | documentation only: no test file; the check is the grep and diff below |
| Verification command run | ☑ pass | Supervisor re-ran: `grep -c "no CLI/API yet" rust/README.md` = 0; `git diff --stat -- rust/README.md` = `1 file changed, 1 insertion(+), 1 deletion(-)`: `-## Ported library functions (core crate, no CLI/API yet)` / `+## Ported library functions (core crate)` |
| Negative cases hold | ☑ pass | the agent checked every other "yet"/"not ported" claim against code and left the true ones: README:36 no browser opened yet (TRUE, `main.rs:65`), :91 other six tools, `investigate` prompt, `--http` (TRUE, `mcp.rs:188-193`), :166 `checked` list and `config.yaml` override (TRUE, `rules.rs:7-10`), :73 other Python routes absent (TRUE, `api.rs:74-81`). Code fences 6 at HEAD, 6 now |
| verify | ☑ N/A | documentation only, no runtime feature |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | the one changed line of `rust/README.md` and the code files its claims were checked against (read only). Open P3, not changed: README:3 says five commands while `main.rs` has four subcommands; the README counted `--version` before ("four commands"), so it is ambiguous, not false |
| Full smoke suite still green (no regression) | ☑ N/A | no code changed |
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

**BEFORE**: verbatim prior text of `rust/README.md` (T025 is documentation only), captured 2026-11-11 before the first edit. Every line the grep `yet|not ported|not implemented|no CLI|no browser` matched, as it stood:

- `rust/README.md:36`: `and `Ctrl-C to stop.` (no browser is opened yet). The token is 32 random bytes, new per run, and only appears in that URL fragment. The routes`
- `rust/README.md:91`: `unknown tool is a JSON-RPC error; a line that is not JSON is answered with `-32700` and a null id. Not ported yet: the other six tools, the`
- `rust/README.md:139`: `## Ported library functions (core crate, no CLI/API yet)`
- `rust/README.md:166`: `... Not ported yet: the `checked` list, the `config.yaml` threshold override.`  (the sentence sits at the end of the paragraph that begins `easy_personal_core::rules::evaluate`)

**AFTER**: `rust/README.md:139` now reads `## Ported library functions (core crate)`; `grep -c "no CLI/API yet" rust/README.md` prints 0.

**DELTA**: a reader of `rust/README.md` is no longer told there is no CLI or API when the Rust app has four subcommands, nine HTTP routes and six MCP tools.

**WITNESS**: Supervisor re-ran the grep and diff at close-out 2026-11-11; tool calls recorded in `memory/event-trace/T025.jsonl`.
