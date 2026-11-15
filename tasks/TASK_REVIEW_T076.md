# TASK_REVIEW — T076: Watch 5: `tray` command

> Sibling of `tasks/TASK_GUIDE_T076.md`. Everything here is **filled by the reviewer at Stage
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

**BEFORE**: default build of branch `T076` at 892aa99 (no implementation commit yet), temp `XDG_CONFIG_HOME`; captured by backend-developer:

```text
2026-11-15T08:08:06Z
892aa99 Board: T063 note: re-point connector entries at the Rust launcher before removing Python (from T062)
$ easy-personal-rs tray
error: unrecognized subcommand 'tray'

Usage: easy-personal-rs <COMMAND>

For more information, try '--help'.
exit 2
$ easy-personal-rs tray --autostart on
error: unrecognized subcommand 'tray'

Usage: easy-personal-rs <COMMAND>

For more information, try '--help'.
exit 2
$ ls $XDG_CONFIG_HOME/autostart
ls: cannot access '/tmp/tmp.qUaeGVdD0E/autostart': No such file or directory
```

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T076.jsonl`, never the
implementing agent alone]

---

## Menu proposal

Native Linux menu (AppIndicator via Tauri `tray-icon`), rebuilt on each 5 s alert-engine look. Source: `docs/design/graphite-mint/TrayPopover.dc.html`, reduced to the rows named in the guide's Restated intent 2. Example values are the canvas's `attention` sample. "info" = disabled row (text only); "action" = clickable.

| # | Row (exact label) | Kind | Shown when | Example / other forms |
|---|---|---|---|---|
| 1 | `Easy Personal · <state>` | info | always | `Easy Personal · attention` (state = `idle` / `active` / `attention` / `problem`) |
| — | separator | | | |
| 2 | live line | info | always | `2 sessions active`, `1 session active`, `No sessions running` |
| 3 | `<n> waiting for you · <project>` | action: opens the dashboard (the canvas's "Open" button) | at least 1 session waiting | `1 waiting for you · easy-memory`; several projects: `3 waiting for you · easy-memory +2` |
| 4 | `<n> failing · <project>` | action: opens the dashboard | at least 1 session failing | `1 failing · easy-personal-mcp` (NOT in the canvas; explains a `problem` state. Owner choice: keep or drop) |
| 5 | `5h limit · <p> % · 7d · <p> %` | info | always; one row per Claude account when there are several, prefixed `<label>: ` | `5h limit · 78 % · 7d · 41 %`; missing/stale window: `5h limit · no data · 7d · 41 %` |
| 6 | machine line | info | always | `CPU 12 % · RAM 64 % · disk 71 % · battery 80 %`; no battery: the battery part is left out (never 0 %); no sample: `Machine · no data` |
| — | separator | | | |
| 7 | `Open dashboard` | action: opens the existing dashboard window (`app` window), focuses it if already open | always | |
| 8 | `Pause alerts` | check item (action): toggles the same in-memory flag as `POST /api/alerts/pause` | always | ticked while paused |
| — | separator | | | |
| 9 | `127.0.0.1 · local only` | info | always | canvas footer |
| 10 | `Quit` | action: stops the server, the feed, the samplers and the alerts, removes the icon, exit 0 | always | |

Differences from the canvas (proposed cuts; owner to confirm):
- No ingest row and no "Run ingest now": not in the Restated intent 2 (the tray does not run ingest).
- No "resets in 2h 10m": the plan-limit sampler (`LimitPercents`) carries no reset time; adding it is extra reading.
- "Pause alerts", not "Pause alerts for 1 hour": T074's pause is a plain flag with no timer (Restated intent 2 says toggle).
- No colours, dots or progress bars: a native AppIndicator menu shows text only. The state colour is on the icon.
- State rule follows the Restated intent 1 (attention = waiting or a limit >= 90 %; problem = a failing or machine-pressure alert active), not the canvas's 70 % / ingest-failed wording.
- Tooltip `Easy Personal · <live line>` is set, but AppIndicator does not show tooltips on GNOME.


### Owner answers (2026-11-15, relayed by the Supervisor via AskUserQuestion)

1. Menu rows: "Approve as listed". All 10 rows are approved; row 4 (`<n> failing · <project>`) is KEPT. The listed differences from the canvas are accepted (no ingest row and no "Run ingest now", no reset time, "Pause alerts" with no timer, text only).
2. State rule: "Approved plan". Restated intent 1 wins over the canvas: attention = a session waiting or a plan limit >= 90 %; problem = a failing session or a machine-pressure alert.
