# TASK_REVIEW — T033: Rust ingest of kit_trace, ui and verifier (P2b)

> Sibling of `tasks/TASK_GUIDE_T033.md`. Everything here is **filled by the reviewer at Stage
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

**BEFORE** (captured 2026-11-11T11:15:27Z, before any T033 file existed; script `scratchpad/before.sh`: one transcript whose `cwd` is a planted project with `memory/event-trace/T001.jsonl` (2 good lines + 1 bad), a `ui/session-a.json`, a `verifier/proj/evidence-report-*.json`; `HOME`, `EASY_PERSONAL_HOME`, `EASY_PERSONAL_AGENT_ROOT`, `EASY_PERSONAL_UI_REPORTS`, `EASY_PERSONAL_VERIFIER_REPORTS` all temp folders; Rust binary `rust/target/debug/easy-personal-rs`, Python `rust/parity/.venv` via the CLI entry):

```
=== Rust ingest
{ "files": 1, "new_events": 1, "not_ingested": ["context","kit_trace","ui","verifier"], "skipped": 0 }   exit 0
=== Python ingest
{ "context": {"status": "no data"}, "files": 1,
  "kit_trace": {"files": 1, "new_events": 3}, "new_events": 1,
  "ui": {"sessions": 1, "skipped": 0, "status": "ok"},
  "verifier": {"projects": 1, "skipped": 0, "status": "ok"} }   exit 0
=== table counts after each run
rust   {'events': 1, 'kit_trace_events': 0, 'ui_sessions': 0, 'verifier_scores': 0, 'source_files': 1}
python {'events': 1, 'kit_trace_events': 3, 'ui_sessions': 1, 'verifier_scores': 1, 'source_files': 2}
```

The three tables stay empty after the Rust run and the Rust output names the three sources under `not_ingested`; Python fills 3, 1 and 1 rows.

**AFTER**: Same script as BEFORE, after the change (agent): Rust printed {"files":1,"kit_trace":{"files":1,"new_events":3},"new_events":1,"not_ingested":["context"],"skipped":0,"ui":{"sessions":1,"skipped":0,"status":"ok"},"verifier":{"projects":1,"skipped":0,"status":"ok"}}; table counts equal Python's (events 1, kit_trace_events 3, ui_sessions 1, verifier_scores 1, source_files 2).
Supervisor re-runs, 2026-11-11, main checkout: `gen_ingest_kit.py --check` 11 files, 0 different; `leak_rust.py` leak check 0 hits over 42 canaries, 116 surfaces, 847452 bytes, `check kit/ui/verifier rows exist ok (2 / 2 / 1 rows)`; `cargo test --workspace` TZ unset / Pacific/Kiritimati / Pacific/Pago_Pago: 439 passed, 0 failed, 26 ignored each; clippy and fmt clean; `git status -- src tests web` empty. Agent (real Python, ignored tests): ingest_kit_python 1 passed (Python after Rust adds 0 and the reverse), ingest_coexist 10 passed, ingest_cli 1 passed. Perf release (agent): A 177.0 MB/s (T032: 176.9), C unchanged re-run 0.05 s (gate 2 s), B +6.0 MiB (gate 8). Negative proof: flipping the newest-report comparison in verifier.rs failed k07 and k08, reverted, 10 of 10 pass.

**DELTA**: After a Rust-only ingest the kit, UI and verifier tables are filled the same as Python's, so those dashboard and `kit_summary` views are no longer empty.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
