# TASK_REVIEW — T072: Watch 1: machine status reader + 60 s in-memory sampler, `GET /api/machine`, CLI `machine`

> Sibling of `tasks/TASK_GUIDE_T072.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/machine.rs` (17 tests: AC1 readers on a fake `/proc` + `/sys`, CPU delta, null cases; AC2 `the_ring_keeps_1440_samples_drops_the_oldest_and_keeps_the_order`); `rust/crates/app/src/api_tests.rs` (3 `machine_*` tests: shape, 403 with no token, 405 on POST/PUT/PATCH/DELETE, store never asked); `rust/crates/app/src/sampler.rs` (4 tests); `rust/crates/app/src/cli_cmds.rs` (`the_machine_text_is_one_line_per_measure_and_no_data_where_null`); `rust/crates/app/tests/machine.rs` (3 tests on the real binary: `machine`, `machine --json`, `serve` + `GET /api/machine`). `cargo test --workspace` 2026-11-15T02:57Z: 925 passed, 0 failed, 29 ignored (incl. `ctrl_c_stops_the_server_frees_the_port_and_leaks_no_token`). |
| Verification command run | ☑ pass | `cargo test --workspace` exit 0, see above. Real run 2026-11-15T03:06:47Z (needs `--bin easy-personal-rs`, two binaries): `CPU: 25.0 %` / `RAM: 6.8 GiB of 31.0 GiB` / `Disk: 352.3 GiB of 454.4 GiB` / `Load (1 min): 5.36` / `Battery: no data` / `Claude processes: 6`, exit 0. |
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

**BEFORE** (2026-11-15T02:46:33Z and 02:46:36Z, branch `T072` at `244e983`, before any implementation commit):

```
$ cd rust && cargo run -q -p easy-personal-rs -- machine
error: `cargo run` could not determine which binary to run. Use the `--bin` option to specify a binary, or the `default-run` manifest key.
available binaries: easy-personal-launch, easy-personal-rs

$ cargo run -q -p easy-personal-rs --bin easy-personal-rs -- machine
error: unrecognized subcommand 'machine'

Usage: easy-personal-rs <COMMAND>

For more information, try '--help'.
```

(The guide's verification command needs `--bin easy-personal-rs`: the package has two binaries.)

**AFTER** (2026-11-15T03:06:47Z, same branch after the commit `T072: machine status readers...`):

```
$ cargo run -q -p easy-personal-rs --bin easy-personal-rs -- machine
CPU: 25.0 %
RAM: 6.8 GiB of 31.0 GiB
Disk: 352.3 GiB of 454.4 GiB
Load (1 min): 5.36
Battery: no data
Claude processes: 6

$ ... -- machine --json
{
  "battery_pct": null,
  "charging": null,
  "claude_procs": 6,
  "cpu_pct": 27.0,
  "disk_total": 487863541760,
  "disk_used": 378326298624,
  "load1": 5.36,
  "ram_total": 33256267776,
  "ram_used": 7294607360,
  "t": 1794712008
}
```

**DELTA**: a user can now read this machine's CPU, RAM, disk, load, battery and `claude` process count with `machine` (or `GET /api/machine`, which also gives the last 24 h in memory), with "no data" instead of 0 where the OS gives nothing.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T072.jsonl`, never the
implementing agent alone]
