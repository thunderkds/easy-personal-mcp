# TASK_REVIEW — T054: Rust T048 review fixes

> Sibling of `tasks/TASK_GUIDE_T054.md`. Everything here is **filled by the reviewer at Stage
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

**BEFORE**: captured on the unchanged T048 code (HEAD `66e94a6`, `target/debug/easy-personal-rs`, debug build) with a
throwaway script outside the repo that starts the built `serve` binary over a temp copy of `store-rich` and times one request
(no tracked file had been changed). Terminal capture, `date -u` as the host clock printed it:

```
$ date -u +%FT%TZ                          # 2026-11-12T05:14:11Z
$ timeout 60 python3 hostile.py chain 18   # POST /api/workspaces, repos: ["<tmp>/chain/L0"], L_i -> "L_{i+1}/L_{i+1}", L_18 -> "."
depth 18 (201, '{"workspace":{...,"name":"hostile","repos"', 4.04104117299994)        # 4.04 s wall, WRITE_LOCK held
$ timeout 60 python3 hostile.py patch      # PATCH /api/workspaces/2, 1 MiB body, 95,000 distinct keys "k00000":1,...
bytes 1045001 keys 95000 (400, '{"error":"unknown field"}', 36.87778788000014)        # 36.9 s wall on one async worker
$ date -u +%FT%TZ                          # 2026-11-12T05:14:52Z
$ date -u +%FT%TZ                          # 2026-11-12T06:57:30Z
$ timeout 20 python3 hostile.py chain 30
exit=124 (124 = timeout killed it)         # depth 30 never answers (2^30 expansions)
```

**AFTER**: the same script against the rebuilt `serve` binary (debug build), same host, `date -u` as printed:

```
$ date -u +%FT%TZ                          # 2026-11-12T07:21:19Z
$ timeout 60 python3 hostile.py chain 18
depth 18 (201, '{"workspace":{...,"name":"hostile","repos"', 0.005046042000685702)    # was 4.04 s
$ timeout 60 python3 hostile.py chain 30
depth 30 (201, '{"workspace":{...,"name":"hostile","repos"', 0.005360771001505782)    # was: never answered (timeout, exit 124)
$ timeout 60 python3 hostile.py patch
bytes 1045001 keys 95000 (400, '{"error":"unknown field"}', 0.2557988700009446)         # was 36.9 s
$ date -u +%FT%TZ                          # 2026-11-12T07:21:20Z
```

New tests, red on the T048 code and green now (red runs: the three source files restored from `66e94a6` for the integration tests, a
mutation for the two unit tests; every red was run under `timeout`):
`crates/core/tests/writes_parity.rs` (`a_chain_of_links_that_doubles_at_every_hop_is_resolved_in_linear_time`: "no answer in 10 s",
`a_repo_that_resolves_to_a_path_that_is_not_utf8_is_refused`: FAILED), `crates/app/tests/writes.rs`
(`a_hostile_link_chain_and_a_body_of_95000_keys_are_answered_in_under_a_second`: "no status line" after 30.8 s), `crates/app/src/writes.rs`
(`prepare_runs_before_the_write_lock_is_taken`: "the write lock is held while the request prepares" with `prepare()` moved under the lock;
`ninety_five_thousand_keys_are_scanned_in_linear_time`: 42.3 s with `Vec::contains` put back). The exact counts: dropping the last case of
`cases.json` fails `assertion left == right failed: cases  left: 261  right: 262` (reverted with `git checkout`).

**DELTA**: a repo path made of a hostile symlink chain or a PATCH body of 95,000 keys is now answered in milliseconds instead of holding the write lock for seconds to forever, and a non-UTF-8 repo path is a clean 400.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T054.jsonl`, never the
implementing agent alone]
