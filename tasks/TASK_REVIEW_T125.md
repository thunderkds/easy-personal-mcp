# TASK_REVIEW — T125: Live like Claude 2b — sub-agent logs in the panel

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

## Auto decisions

Mode: auto
1. Sub-agent log opens inside the same panel under a breadcrumb — RULE: `PRD.md:229` "Opening one shows its conversation with the same view as the main thread"
2. Direction Path A of `BRAINSTORMING_LOG_LIVE2.md` — RULE: `docs/claude-md/tracks.md:239` "Take the recommended direction and list the alternatives in the report."
Checks: <filled at Stage 4/5>
Memory: <filled before stop>
Not done: merge (admin: after the auto verify pass); auto verify: <result>
Paused: auto-mode check failed — no `Memory:` line in the report — run the memory pass before you stop and record it there (each entry written, or `Memory: nothing new`); no test run in memory/event-trace/T125.jsonl — write /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d/$CLAUDE_CODE_SESSION_ID with the task ID (minimal-track step 1): `mkdir -p /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d && printf '%s\n%s\n' "T125" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d/$CLAUDE_CODE_SESSION_ID`, then rerun the task's tests so the trace records them under the task, then stop; extended track, but the trace has no `code-review` Skill call (Stage 4 runs in auto mode); Risk Medium, but the trace has no `security-review` Skill call [report 0e4eb120f790]


## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/SubAgents.test.jsx` (21 tests: open log, nesting at depth 3, unknown parent, newest-200 cut, empty list, unknown meta, 5 s poll, breadcrumb back, per-file offsets, session switch, 404 one line, no-route, Live + Past panels, text not HTML); `web/src/LiveAnswer.test.jsx` (its fetch stub now also answers the new `/agents` read) |
| Verification command run | ☑ pass | Stage 4 fix rounds (2026-11-20): three `npm test` runs in a row after the focus fix, each `Test Files  45 passed (45)` / `Tests  1508 passed (1508)` (three times). `npm run build`: `✓ built`; `python3 scripts/check_static.py`: `check_static: ok`. `LiveRename.test.jsx` focus race: under 8 busy CPU loops the `useEffect` version failed 6 of 14 single runs of the test (2 of 16 in an earlier batch; main failed 1 of 6 in the same file); with `useLayoutEffect` in `SessionName.jsx` 0 of 14 and 0 of 16. |
| Negative cases hold | ☑ pass | Mutation: `AGENTS_POLL_MS` 5000 → 2000 made `SubAgents.test.jsx` "reads the list ... every 5 s" fail (`Tests  1 failed \| 20 passed (21)`), then reverted (back to 5000, 21 passed). Before the stub change `LiveAnswer.test.jsx` had 7 red tests (the new `/agents` call was counted as an answer call); Stage 4 fix: the old `useOwners` (git HEAD ef0fb56) run against the new tests: `Tests  2 failed \| 22 passed (24)` (owner never published; read error cached), green with the fix |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | whole web suite 1505 / 1505 after the change (was 1484 before: +21 new) |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | ux-evaluate by hand on a throw-away harness of the real `LiveView` with mocked routes (scratch dir, nothing committed): `main`, `log`, `nested` at 375 and 1280 in `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/b0e607b5-0126-47ad-b65d-c4fbbbdebda6/scratchpad/ux/` (`main-375.png`, `main-1280.png`, `log-*.png`, `nested-*.png`) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | new CSS in `appsession.css` uses only existing tokens (`--space-*`, `--color-*`, `--fs-label` 0.875rem / `--fs-note` 1rem, `--font-mono`); buttons follow the `reply-toggle` pill pattern |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 and 1280 read: breadcrumb and list wrap, a 47-character type name wraps, no horizontal scrollbar at 375; buttons 44 px high at ≤ 40rem |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (Supervisor, before any implementation commit; `main` the branch point):
```
$ date -u; git log -1 --oneline; grep -rln 'Open log\|parent_call' web/src | wc -l
2026-11-19T11:18:19Z
af4aa48 Board + memory: T122, T123 Done (merged); font decision
0
$ (re-run on the branch point after T124 merged)
2026-11-20T01:32:25Z
e39d05f Memory: T124-T128 merged
0
```

**AFTER** (same command, on the branch after the implementation commit):
```
$ date -u; git log -1 --oneline; grep -rln 'Open log\|parent_call' web/src | wc -l
2026-11-20T01:45:09Z
ef0fb56 T125: sub-agent logs in the Live and Past panels (list, breadcrumb, Open log)
4
(web/src/api.js, web/src/Transcript.jsx, web/src/SubAgents.test.jsx, web/src/SubAgents.jsx)
```

**DELTA**: In the Live panel and a past session the owner can click Open log on an Agent / Task call (or a row of the new Sub-agents list) to read that sub-agent's own conversation, move between main and nested sub-agents by a breadcrumb, and see which are running or finished; before, a sub-agent's work was not visible at all.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T125.jsonl`, never the
implementing agent alone]
