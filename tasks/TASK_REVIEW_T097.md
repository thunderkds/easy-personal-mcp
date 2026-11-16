# TASK_REVIEW — T097: app icon on notifications, window and dock; one real-machine smoke script

> Sibling of `tasks/TASK_GUIDE_T097.md`. Everything here is **filled by the reviewer at Stage
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
With `/verify (yours)`, your pass goes in the `verify` row below: Result `☑ pass`, Notes `/verify by the user, session <id>, commit <sha>`.

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its proposal>; Advocate 2: <its proposal>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Memory: <each entry the memory pass wrote, as `file — entry` | nothing new>
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours), with [admin] only after the one retry also ended blocked>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | [paste the red run against the prior content, or the mutation diff (`git diff --stat`, non-empty) before each verdict] |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
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

**BEFORE** (captured 2026-11-16 on branch T097 at d822063, no implementation commit yet, debug build `cargo build --locked -p easy-personal-rs --features window`; probes: notify_check.sh / window_probe.sh from the Supervisor's scratchpad):

```
2026-11-16T11:10:43Z
== notify
--- Notify call seen on the session bus:
   string "easy-personal"
   uint32 0
   string ""
   string "Easy Personal"
   string "Waiting for you, AskUserQuestion, project tmp, session notify-c"
   array [
   ]
--
--- serve stderr:
notify-check: PASS
== window
WM_CLASS(STRING) = "easy-personal-rs", "Easy-personal-rs"
window icon: not set
== desktop
(grep exit 1)
```

Parity test (`cargo test -p easy-personal-rs --test connector_cli_parity`, `/usr/bin/easy-personal-launch` installed):

```
2026-11-16T11:10:59Z
---- every_connector_and_doctor_step_equals_the_python_cli stdout ----
connector cli oracle: 95 cases, 116 steps, 5 different

thread 'every_connector_and_doctor_step_equals_the_python_cli' (1767769) panicked at crates/app/tests/connector_cli_parity.rs:250:5:
5 problems:
sync_no_launcher step 0 ["connector", "sync", "--yes"]: exit 0, Python 1; stdout "Claude default: add acme-discord -> exec discord-acme\nClaude default: add acme-gmail -> exec gmail-acme\nClaude default: add acme-sendgrid -> exec sendgrid-acme\nClaude default: add acme-slack -> exec slack-acme\nClau
=====
sync_no_launcher: claude writes [Object {"argv": Array [String("mcp"), String("add-json"), String("acme-discord"), String("{\"type\": \"stdio\", \"command\": \"/usr/bin/easy-personal-launch\", \"args\": [\"exec\", \"discord-acme\"]}"), String("--scope"), String("user")], "ccd": String("<unset>")}, O
  want []
=====
sync_no_launcher: claude state {"<WORK>/agent-root/.claude-work":{"acme-discord":"Command: <LAUNCHER>\nArgs: exec discord-acme\n","acme-gmail":"Command: /usr/bin/easy-personal-launch\nArgs: exec gmail-acme\n","acme-sendgrid":"Command: /usr/bin/some-server\nArgs: --serve\n","acme-slack":"Command: <LA
=====
sync_no_launcher: table account_registrations [Array [Number(1), String("acme-discord"), String("discord-acme")], Array [Number(1), String("acme-gmail"), String("gmail-acme")], Array [Number(1), String("acme-sendgrid"), String("sendgrid-acme")], Array [Number(1), String("acme-slack"), String("slack-
  want [Array [Number(2), String("acme-gmail"), String("gmail-acme")], Array [Number(2), String("acme-slack"), String("slack-acme")], Array [Number(2), String("acme-teams"), String("teams-acme")], Array [Number(2), String("old-one"), String("slack-personal")], Array [Number(2), String("old-two"), St
=====
doctor_no_launcher step 0 ["doctor"]: exit 0, Python 1; stdout "ok keychain: <BACKEND>\nok claude: on PATH\nok launcher: /usr/bin/easy-personal-launch\n" stderr ""
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    every_connector_and_doctor_step_equals_the_python_cli

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.47s

error: test failed, to rerun pass `-p easy-personal-rs --test connector_cli_parity`
```

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
