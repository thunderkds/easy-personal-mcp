# TASK_REVIEW — T[NNN]: [Short Title]

> Sibling of `tasks/TASK_GUIDE_T[NNN].md`. Everything here is **filled by the reviewer at Stage
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
With `/verify (yours)`, your pass goes in the `verify` row below: Result `☑ pass`, Notes `/verify by the user, session <id>, commit <sha>`.

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its proposal>; Advocate 2: <its proposal>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Memory: <each entry the memory pass wrote, as `file — entry` | nothing new>
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours), with [admin] only after the one retry also ended blocked>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ N/A | spike: no app code or tests; the 7 questions are answered in `tasks/evidence-T130/SPIKE.md` |
| Verification command run | ☑ pass | the guide's command printed `8` (count of GO in SPIKE.md) and `8` for the pgrep count; those 8 are only `zsh` shell wrappers whose command line holds the word claude and the scratchpad path, checked one by one through `/proc/<pid>/cmdline`: no bridge, no `claude` in the scratch repo, no `--resume`, no Chrome profile process (2026-11-20, after 04:12Z) |
| Negative cases hold | ☑ pass | guard cases run against the bridge at 04:03Z: evil Origin 403, rebinding Host 403, wrong token closed with no data, no token 401 (SPIKE.md, routes table) |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | the only repo change is `tasks/evidence-T130/SPIKE.md` (+ this file); `rust/**`, `web/**`, `src/**` untouched; no `~/.claude*` file read or written by the spike itself (only `claude`'s own writes into the account's folders, listed in SPIKE.md Risks) |
| Full smoke suite still green (no regression) | ☑ N/A | no app code changed |
| **Companion score (easy-verifier)** | ☑ N/A | docs/evidence-only spike, no code diff to score |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no UI shipped; throwaway screenshots stay in the scratchpad (`.../scratchpad/t130/shots/`) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no UI shipped |
| **UI: Responsiveness at target viewports** | ☑ N/A | spike only: xterm reflowed at 1280, 768 and 375 px (SPIKE.md Q3) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (Supervisor, before any spike commit; docs-only task: verbatim prior content). `tasks/evidence-T130/SPIKE.md` does not exist; DDR-0006 line 37 reads:
```
| Embedded PTY running the real TUI | Full fidelity, but a terminal emulator in the app; cut list |
```

**AFTER** (verbatim excerpt of `tasks/evidence-T130/SPIKE.md`):
```
| 1 | PTY starts the interactive `claude` and keeps it running | **GO** | ...
| 2 | Typed input reaches it; it answers; its own permission dialog is answered by keystrokes | **GO** | ...
| 3 | Renders correctly in xterm.js over a local bridge: colours, cursor, resize | **GO** | ...
| 4 | Saved as an interactive session; `--resume <id> -p` resumes it | **GO** | ... every `entrypoint` = `cli` ...
| 5 | Live sees it as a terminal session; the answer-bridge hooks fire | **GO** | ... all seven fields present; PermissionRequest, Stop, PreToolUse fired ...
| 6 | Closing the PTY ends `claude` cleanly; nothing left; re-attach from an in-memory buffer | **GO** (conditional) | ...
| 7 | Page cost of `xterm.js` + fit addon | **GO** (conditional) | ... 120,455 B gzip + 657 B ...
```

**DELTA**: The owner can now decide on a build with measured evidence: the real `claude` TUI runs in a Rust PTY and draws correctly in xterm.js (colours, cursor, resize, dialogs answered by keystrokes), Live would see it as a terminal session, and the evidence names what a build needs (WebSocket first-frame token, 1 MiB ring plus a resize nudge, lifecycle with the tray), the risks and six open questions; DDR-0006's cut item is reversed only after a DDR.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T130.jsonl`, never the
implementing agent alone]
