# TASK_REVIEW — T055: Rust wave-3 follow-ups (one path resolver, write_session create-race, shared helpers, test fixes)

> Sibling of `tasks/TASK_GUIDE_T055.md`. Everything here is **filled by the reviewer at Stage
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

**BEFORE** (captured by the implementing agent on `main` at `0ad72fb`, worktree
`/home/thunderkds/workspace/pets/easy-personal-mcp/.claude/worktrees/agent-a60889f929c8fa520`, before any change; times UTC; Python 3.14.4):

(a) `memgraph_parity` with `TMPDIR` on the disk volume (an empty folder `/home/thunderkds/t055-b`, outside every repo), 2026-11-12T12:01:08Z:

```
$ TMPDIR=/home/thunderkds/t055-b cargo test -p easy-personal-core --test memgraph_parity
test the_hostile_tree_gives_python_answers ... FAILED
thread 'the_hostile_tree_gives_python_answers' panicked at crates/core/tests/memgraph_parity.rs:350:5:
assertion `left == right` failed: list
  left (Rust):  projects = [proj, proj600, projcap, zz(/nonexistent-t043/zz), claude-bare]
  right (golden): projects = [zz(/nonexistent-t043/zz), proj, proj600, projcap, claude-bare]
test result: FAILED. 26 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.93s
```
(the two rows are shortened to the project names; every field of every row is equal, only the order differs: `zz` is the 5th row on the Rust side and the 1st in the golden. Under `TMPDIR` unset, or `/var/tmp/t055-before-a` (2026-11-12T12:00:11Z), the same command prints `test result: ok. 27 passed; 0 failed`.)

(b) the three resolvers on a fixture made by `mkfx.py` (a 3-link loop `a -> b -> c -> a`; a chain `l1 -> l2 -> ... -> l41 -> d`, 41 links, `d` a directory), a throwaway in-crate test (not committed), 2026-11-12T12:01:46Z, next to Python 3.14.4 on the same fixture (`<F>` = `/var/tmp/t055-fx`):

```
python os.path.realpath(<F>/a/x)  = <F>/a/x          python os.path.realpath(<F>/l1/x) = <F>/d/x     python os.path.realpath(<F>/l1) = <F>/d
rust metrics::realpath("a/x")          = <F>/b/x     ("l1/x") = <F>/l41/x    ("l1") = <F>/l41
rust workspaces::resolve_path("a/x")   = <F>/a/x     ("l1/x") = <F>/d/x      ("l1") = <F>/d
rust accounts::resolve("a/x")          = <F>/a/x     ("l1/x") = <F>/l1/x     ("l1") = <F>/l1
```
`metrics::realpath` is wrong on both (the 3-link loop is walked round to `b`; the 41-link chain stops after 40 links at `l41`); `accounts::resolve` is wrong on the chain (`fs::canonicalize` gives up past 40 links and the whole path is kept as typed); `workspaces::resolve_path` equals Python.

(c) temp folders left by one `cargo test --workspace --no-fail-fast`, run with `TMPDIR=/var/tmp/t055-c` (an empty folder on the disk volume, outside the worktree: a `TMPDIR` inside the real home or inside a git repo makes the coexist tests refuse to run and the memgraph fixtures resolve to the enclosing repo), 2026-11-12T12:03:39Z to 12:07:27Z:

```
$ ls /var/tmp/t055-c | wc -l
9
easy-personal-rs-t041-edge-<pid>-{0,1,3}   easy-personal-rs-t041-golden-<pid>-2   easy-personal-rs-t041-golden-rich-<pid>-5
easy-personal-rs-t041-hand-<pid>-{...}  (3)   ep-t033-cfg-<pid>  (1)
workspace totals: passed 661, failed 1 (workspaces_parity::edge_store_python_values: the known failure inside a worktree under .claude/), ignored 26
```

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
