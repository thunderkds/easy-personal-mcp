# TASK_REVIEW — T034

> Sibling of `tasks/TASK_GUIDE_T034.md`. Everything here is **filled by the reviewer at Stage
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

**BEFORE** (captured 2026-11-11T11:33:45Z, before any T034 file existed; debug binary built from `e2ba53d` + the guide commit; temp `HOME` and `XDG_CONFIG_HOME`, `env -i`, no `systemctl` run; Python run via the parity venv with `PYTHONPATH=<worktree>/src`, whose `easy-personal` is not on `PATH`, so Python falls back to `<python> -m easy_personal.surfaces.cli`):

```
--- RUST: easy-personal-rs --help
Rust port of easy-personal

Usage: easy-personal-rs <COMMAND>

Commands:
  store-info  Show the store's version and row counts (opens it read-only; never writes)
  ingest      Copy new Claude Code transcript metadata into the store (events only; takes the ingest lock; never migrates)
  serve       Serve the dashboard API on 127.0.0.1 (a free port, a per-run token) until Ctrl-C
  mcp         Serve the MCP tools over stdio (JSON-RPC on stdout, nothing else)
  app         Open the dashboard in a window (needs a build with `--features window` and a display)
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
exit=0
--- RUST: easy-personal-rs schedule install --dry-run
error: unrecognized subcommand 'schedule'

Usage: easy-personal-rs <COMMAND>

For more information, try '--help'.
exit=2
--- PYTHON: easy-personal schedule install --dry-run
# easy-personal-ingest.service
[Unit]
Description=easy-personal ingest (copy new Claude Code usage metadata to the store)

[Service]
Type=oneshot
ExecStart=/home/thunderkds/workspace/pets/easy-personal-mcp/rust/parity/.venv/bin/python -m easy_personal.surfaces.cli ingest

# easy-personal-ingest.timer
[Unit]
Description=Run easy-personal ingest daily

[Timer]
OnCalendar=daily
Persistent=true

[Install]
WantedBy=timers.target

exit=0
--- files written under temp xdg/home: (none)
```

**AFTER**: Agent real run (2026-11-11T11:52:17Z, temp HOME and XDG_CONFIG_HOME, PATH with only the fake systemctl): `--help` lists `schedule`; `schedule install --retention-days 30` wrote both unit files and the fake logged `--user show-environment`, `--user daemon-reload`, `--user enable --now easy-personal-ingest.timer`, exit 0; `schedule remove` logged `disable --now` then `daemon-reload` and deleted both files, exit 0; with no systemctl on PATH `install` printed the two lines and `17 3 * * * <binary> ingest`, exit 1. Red run before the code: 15 failed, 3 passed. Mutations (`enable --now` to `enable`; `'` counted safe) failed 4 oracle tests and the quoting test.
Supervisor re-runs on main, 2026-11-11: `gen_schedule.py --check` 12 files, 0 different; `cargo test -p easy-personal-rs --test schedule_cli` 18 passed, 0 failed; `cargo test --workspace` TZ unset / Pacific/Kiritimati / Pacific/Pago_Pago: 461 passed, 0 failed, 26 ignored each; clippy default and `--features window` clean; fmt clean; `git status -- src tests web` empty. The real `~/.config/systemd/user` does not exist on this machine: nothing was written there.

**DELTA**: A Rust-only machine can run its ingest daily with `easy-personal-rs schedule install` (a systemd user timer, same units as Python, command line runs the Rust binary), and remove it again.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
