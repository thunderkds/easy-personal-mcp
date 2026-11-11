# TASK_REVIEW — T026: Ingest oracle — Python ingest over fixtures, tables frozen as goldens

> Sibling of `tasks/TASK_GUIDE_T026.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | the deliverable is the oracle itself: `rust/parity/gen_ingest.py` (with `--check`), `rust/parity/make_fixtures_ingest.py`, 21 scenarios under `rust/parity/ingest/golden/`; self-tests in the generator: determinism, real-home refusal, the secret/path scanner (probed directly), `--check` negative (a flipped byte in `S09_lock_busy.json` gives rc=1, restored gives rc=0; agent run) |
| Verification command run | ☑ pass | Supervisor 2026-11-11T05:27:39Z: `gen_ingest.py --check` = `24 files, 0 different`, exit 0; agent: two generator runs gave identical `manifest.json` sha256 `dfae684b…650522`; `git status --short -- src tests web rust/crates`, counted with `wc -l` = 0. Python's own suite with the parity venv (Supervisor re-run): `tests/core tests/sources tests/ops` = `1065 passed, 6 deselected, 1 warning` |
| Negative cases hold | ☑ pass | agent-reported: real-home guard refuses (`EASY_PERSONAL_HOME` inside a real-looking home: `refused …`, rc=1, nothing created); a changed golden byte makes `--check` exit 1 naming the file; scanner probes hit `@`, `sk-`, `/home/`. Supervisor re-ran only the positive `--check` |
| verify | ☑ N/A | test tooling with no runtime feature: the generator runs the real Python CLI and the Python tests were re-run (above). Not driven through the `/verify` recipe |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | new files only under `rust/parity/` plus the appended README section and this review file; Python and `rust/crates` untouched (0 entries). Safety scan of both scripts: no shell, network or `rmtree`; deletes only its own outputs and one file inside a temp copy. code-review `reports/code-review_main_20261111T131108.html`: 0 P0, 0 P1, 1 P2 (tracebacks in 5 goldens are CPython/src specific), 3 P3. security-review `reports/security-review_main_20261111T131109.html`: nothing at confidence 8 or above. Privacy scan of `rust/parity/ingest` clean |
| Full smoke suite still green (no regression) | ☑ pass | Python `tests/core tests/sources tests/ops`: 1065 passed (Supervisor re-run); Rust code unchanged by this task |
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

**BEFORE**: captured by qa-expert before any T026 file was created:

```
$ date -u +%Y-%m-%dT%H:%M:%SZ; ls rust/parity/ingest rust/parity/gen_ingest.py; git status --short -- rust/parity | wc -l
2026-11-11T05:00:59Z
ls: cannot access 'rust/parity/ingest': No such file or directory
ls: cannot access 'rust/parity/gen_ingest.py': No such file or directory
0
```

**AFTER**: the verification command after the change, Supervisor run 2026-11-11T05:27:39Z: `24 golden files in rust/parity/ingest/golden (21 scenarios)`, `--check: 24 files, 0 different`, `0` changed files in `src tests web rust/crates`.

**DELTA**: the Rust ingest can now be proven against the real Python `ingest` (21 scenarios, a fresh-store schema snapshot and every recorded odd case), byte for byte and repeatably.

**WITNESS**: Supervisor re-ran `--check` and Python's tests after the implementing agent; tool calls recorded in `memory/event-trace/T026.jsonl`.

## Findings the plan must use (from the odd-case table)

1. **Python's ingest can be stuck for good by one bad input** (S06a-d): a deeply nested line (`RecursionError`), an integer beyond int64 (`OverflowError`), a directory named `x.jsonl` (`IsADirectoryError`) and a `type` that is a list (`TypeError`) each crash the run with exit 1, roll back that whole file, never read the files after it, and crash again on every later run. **Supervisor decision, `[admin]` mode, ⚠ decided for you — please review:** the Rust ingest must **not** copy these crashes. It treats such a line as an *abstained row* (all columns NULL, like invalid JSON), skips a non-file entry, and carries on with the other files. This is the one place Rust is allowed to differ from Python, because the Python behaviour is a bug that blocks all later data. T028 records each case as "Rust differs: Python crashes".
2. A store newer than v11 makes Python exit **1 with a traceback**; Rust exits **2** with a clean message (decided in T027).
3. Rust tests must compare `exit_code` and the exception class for S06a-d and S11, never the traceback text; goldens must be regenerated after any change under `src/`.
4. `<WORK>` is padded to exactly 240 characters in the oracle; a Rust test that recreates `repos.json` must pad its temp folder the same way.
