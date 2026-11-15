# TASK_REVIEW — T085: Leak gate hardening: run `serve --alerts` over the planted transcripts; store-check 'no file beside the store' stable under load

> Sibling of `tasks/TASK_GUIDE_T085.md`. Everything here is **filled by the reviewer at Stage
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

**BEFORE** (captured 2026-11-15, branch `T085` at main b4c8469, no implementation commit yet; command: `cd rust && env -u CLAUDE_CONFIG_DIR .../parity/.venv/bin/python parity/leak_rust.py; echo exit=$?`):

Idle-ish machine (load average 32 at start, other agents building; `cargo build -q` had just finished), 2026-11-15T09:55:09Z to 09:55:56Z. The gate passes and has no alert surface: `/api/alerts` is read from a `serve` WITHOUT `--alerts`, so it is the empty list:
```
exit=0
202:surface api GET /api/alerts: 246 bytes
910:check store-check made no file beside the store (healthy)  ok (4 files)
914:check store-check made no file beside the store (text past the cut)  ok (1 files)
918:check store-check made no file beside the store (blob source)  ok (1 files)
...
981:leak check: 0 hits over 42 canaries, 818 surfaces, 1801725 bytes
```
The same command with 24 busy loops of mine running (my PIDs only, later killed by PID), 3 runs in a row, 09:56:17Z to 09:58:15Z, load average 21 / 35 / 40:
```
2026-11-15T09:56:17Z  load average: 21.42  -> exit=1
910:check store-check made no file beside the store (healthy)  FAILED (6 files)
983:leak check: 0 hits over 42 canaries, 818 surfaces, 1802202 bytes; 1 failed checks
2026-11-15T09:57:19Z  load average: 35.17  -> exit=1
918:check store-check made no file beside the store (blob source)  FAILED (3 files)
983:leak check: 0 hits over 42 canaries, 818 surfaces, 1802245 bytes; 1 failed checks
2026-11-15T09:58:15Z  load average: 40.58  -> exit=1
914:check store-check made no file beside the store (text past the cut)  FAILED (3 files)
918:check store-check made no file beside the store (blob source)  FAILED (3 files)
983:leak check: 0 hits over 42 canaries, 818 surfaces, 1802555 bytes; 2 failed checks
```
The kept folders show what was beside the store: `store.sqlite-shm` (32768 bytes) and `store.sqlite-wal` (0 bytes). Probe (a `serve --no-ingest` over a copy of the store, SIGINT after the first line, folder listed after exit):
```
hold 0.04s (stopped 66 ms after the first line): ['store.sqlite']
hold 0.06s (stopped 91 ms after the first line): ['store.sqlite', 'store.sqlite-shm', 'store.sqlite-wal']
hold 0.3s (stopped 323 ms after the first line): ['store.sqlite', 'store.sqlite-shm', 'store.sqlite-wal']
```

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
