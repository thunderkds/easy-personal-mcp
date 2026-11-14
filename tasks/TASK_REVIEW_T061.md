# TASK_REVIEW — T061: parity run of Python against Rust

> Sibling of `tasks/TASK_GUIDE_T061.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` tag,
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

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its option>; Advocate 2: <its option>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours)>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/parity/test_parity_report.py` (21 tests after the review rounds: AC1 inventory read from Python and a row for every surface, a new surface fails it; AC4 each README item exists and holds its stated words, a rule explains only what it describes, a missing store only when Python made one; AC5 each `differs: new` has Reproduce / Python / Rust / Proposed task; AC6 report and scanner; N1-N3 do not hide a difference). Red first: before `docs/PARITY_T061.md` existed, 5 of 9 tests failed with `docs/PARITY_T061.md does not exist: run parity/run_parity.py first`. Green: `passed: 0 failing` (21 `ok` lines). Round 3 red first: 5 failures (usage-error findings, blind check, bare `saved_utc` mask, counts line, inventory temp folder). Runner: `rust/parity/run_parity.py` (also a negative control: Rust on a store with one more workspace must be a difference, or the run stops). |
| Verification command run | ☑ pass | Run from `rust/` of the worktree (the worktree has no venv: the parity venv of the main checkout, `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/rust/parity/.venv/bin/python`, was used): `cargo build` -> `Finished dev profile ... in 0.13s`; `env -u CLAUDE_CONFIG_DIR <venv python> parity/run_parity.py` -> `87 rows, 179 probes -> .../rust/docs/PARITY_T061.md` and `Counts: equal 25 + equal except JSON key order 29, differs README 27, differs new 6, not run 0.` (after review round 3); `<venv python> parity/test_parity_report.py` -> 21 `ok`, `passed: 0 failing`; `cargo test --workspace --no-fail-fast` -> exit 0, 882 passed, 0 failed, 29 ignored (sum of the 70 `test result:` lines; run before the last edit of `run_parity.py`, which changed only `check=False` arguments). |
| Negative cases hold | ☑ pass | The runner's control probe (Rust on a world with one more workspace) is seen as `new`; unit tests: an exit-code difference, an extra stdout, or a missing-store text with no store made by Python is `new` even when the rule is declared; a changed number stays a difference after N1-N3. |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | `cargo test --workspace --no-fail-fast`: exit 0, 882 passed, 0 failed, 29 ignored. No file of `src`, `tests`, `web`, `rust/crates`, `rust/Cargo.toml`, `rust/Cargo.lock` changed: `git status --short -- src tests web rust/crates rust/Cargo.toml rust/Cargo.lock` printed nothing. |
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

**BEFORE** (captured 2026-11-14T05:08:46Z, branch `T061` at `8d1792a`, before any implementation commit):

```
$ date -u +%FT%TZ && ls rust/parity/run_parity.py rust/docs/PARITY_T061.md rust/parity/test_parity_report.py; echo "exit=$?"
2026-11-14T05:08:46Z
ls: cannot access 'rust/parity/run_parity.py': No such file or directory
ls: cannot access 'rust/docs/PARITY_T061.md': No such file or directory
ls: cannot access 'rust/parity/test_parity_report.py': No such file or directory
exit=2
```

Size of the job, read from Python (argparse tree, Starlette route table, FastMCP tool and prompt managers; `create_app(..., static=<a folder>)` so the static mount is listed; HEAD not counted):

```
cli leaf subcommands: 32        (plus the top-level --version option)
routes (method,path): 40        (39 API method+path pairs and the "/" static mount)
mcp tools: 12 prompts: 1
```

That is 85 surfaces (86 with `--version`) to run on both apps.

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T061.jsonl`, never the
implementing agent alone]
