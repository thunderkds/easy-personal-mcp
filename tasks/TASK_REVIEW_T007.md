# TASK_REVIEW — T007: A richer parity oracle

> Sibling of `tasks/TASK_GUIDE_T007.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/rich_parity.rs` (8 + 1 child: 21 unscoped usage entries, 2 warnings entries, `bad_since` by status and `error` key, a rule-hit recount, the scoped entries listed as not compared, two mutation tests). Generator: `rust/parity/gen_golden.py --rich`, fixtures `rust/parity/make_fixtures_rich.py` + `fixtures-rich/` (62 files), store `store-rich/store-v11.sqlite`, 145 goldens in `golden-rich/` |
| Verification command run | ☑ pass | Supervisor ran the whole `cargo test`: lib 2, git_timeout 1, rich_parity 8, store_info 12, usage_parity 22, usage_tz_parity 9, warnings_parity 15, app 3+6+15+13, all ok; clippy `-D warnings` and `cargo fmt --check` clean; `ruff check rust/parity` (venv) all checks passed |
| Negative cases hold | ☑ pass | Supervisor: default generator run `diff -r` against `rust/parity/golden` empty and store `cmp` identical; `--rich` run under `PYTHONHASHSEED=5 LC_ALL=C TZ=Asia/Bangkok` equals the committed `golden-rich` and `store-rich` byte for byte; leak grep over `golden-rich` and `fixtures-rich` 0 matches; a `--out` under `~/.easy-personal` refuses ("nothing was opened", no folder made). Agent: a changed fixture value changed 49 of 145 goldens; Rust mutations fail both usage and warnings comparisons. Unscoped warnings golden has 6 hits and 0 abstentions (cache_hit_low, records_unreadable, session_huge_no_compact, tool_errors_high, project_dominates, tokens_jump). `git status --short -- src web tests pyproject.toml uv.lock` empty |
| verify | ☑ pass | the Rust core's `usage_*` and `rules::evaluate` equal the rich goldens (no differences found) — pass. Not testable in Rust yet: 77 entries with `workspace` / `account`, 7 report-folder entries, and the routes the core does not serve (kit, board, accounts, workspaces, reports, memory graph, sessions, outliers, plan, kit_findings, kit_summary) |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `rust/parity/**` (generator, fixtures, store, goldens, README), `rust/crates/core/tests/rich_parity.rs`, one line of `rust/README.md`; `rust/crates/core/src/**` and the locked oracles untouched (hashes equal before and after) |
| Full smoke suite still green (no regression) | ☑ pass | all of the above, incl. T012's refactor and T009's zone tests |
| **Companion score (easy-verifier)** | ☑ N/A | not run: easy-verifier is not installed in this session |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no UI |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no UI |
| **UI: Responsiveness at target viewports** | ☑ N/A | no UI |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-10T10:34:42Z by the Supervisor, before the agent's first command:
```
$ ls rust/parity; python3 -c "import json;m=json.load(open('rust/parity/golden/manifest.json'));print(len(m['entries']))"
README.md  gen_golden.py  golden  golden-tz  store-v11.sqlite
52
$ grep -c '"hit"\|measured' rust/parity/golden/api__warnings__all.json  # one hit only (records_unreadable); the other five rules abstain
(no golden-rich/, no store-rich/, no fixtures-rich/ exists)
```
(the oracle holds one day of data; warnings and plan only abstain.)

**AFTER**: captured 2026-11-10 by the Supervisor:
```
$ python rust/parity/gen_golden.py --rich --out <scratch>
145 rich golden entries, 18 skipped; 6 of 7 rules fire somewhere; store v11, 319488 bytes
$ diff -r <scratch>/golden-rich rust/parity/golden-rich   -> empty
```

**DELTA**: the oracle now holds 18 days of invented history, 6 of the 7 rules firing, outliers, two accounts, workspaces, reports and kit data, and the filter parameters, so a wrong Rust port of those paths can no longer pass on empty answers; the Rust usage and warnings code already matches it.

**WITNESS**: Supervisor, 2026-11-10, re-ran the generator (default and `--rich`) and the Rust suites independently of the implementing agent.
