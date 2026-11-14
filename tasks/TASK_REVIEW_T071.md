# TASK_REVIEW — T071: Icon assets

> Sibling of `tasks/TASK_GUIDE_T071.md`. Everything here is **filled by the reviewer at Stage
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

**BEFORE** (captured before any implementation commit, base `042997b`):

```
2026-11-14T16:13:23Z
$ ls -R rust/crates/app/icons
rust/crates/app/icons:
icon.png
$ file rust/crates/app/icons/icon.png
rust/crates/app/icons/icon.png: PNG image data, 32 x 32, 8-bit/color RGBA, non-interlaced
$ python3 scripts/gen_icons.py
python3: can't open file '/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/T071/scripts/gen_icons.py': [Errno 2] No such file or directory
exit=2
$ git status --short rust/crates/app/icons
```

**AFTER** (same generator command, post-change):

```
2026-11-14T16:40:20Z
$ python3 scripts/gen_icons.py && git status --short rust/crates/app/icons
gen_icons: wrote 28 PNGs under rust/crates/app/icons
(empty = clean)
$ python3 scripts/gen_icons.py --check; echo $?
0
$ pytest tests/test_icons.py -q
.............................                                            [100%]
29 passed in 0.41s
$ ls -R rust/crates/app/icons | head -12; file icon.png
128x128.png
128x128@2x.png
32x32.png
README.md
icon.png
src
tray
rust/crates/app/icons/icon.png:                PNG image data, 512 x 512, 8-bit/color RGBA, non-interlaced
rust/crates/app/icons/32x32.png:               PNG image data, 32 x 32, 8-bit/color RGBA, non-interlaced
rust/crates/app/icons/128x128@2x.png:          PNG image data, 256 x 256, 8-bit/color RGBA, non-interlaced
rust/crates/app/icons/tray/dark/active-16.png: PNG image data, 16 x 16, 8-bit/color RGBA, non-interlaced
```

Contact sheet of all 24 tray PNGs on dark `#1C1F1E` and light `#E8EBE9` panels (left: dark; right: light; rows idle, active, attention, problem; columns 16, 24, 32 px, scaled 4x) plus the app icon at 512 (shown 256), 128 and 32 px: `reports/T071-contact-sheet.png` in the T071 worktree (gitignored). Compared by LLM vision with `TrayIcons.dc.html`: same mark shape, dot place and colours; idle has no dot.

**DELTA**: The app now has the brand-mark icon in four sizes (the window embeds `icon.png`) and 24 state-dot tray icons for dark and light panels, all rebuilt by one command from two SVG files.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T071.jsonl`, never the
implementing agent alone]
