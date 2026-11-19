# TASK_REVIEW — T121: UX audit and fix pass, whole app

> Sibling of `tasks/TASK_GUIDE_T121.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | [paste the red run against the prior content, or the mutation diff (`git diff --stat`, non-empty) before each verdict] |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
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

**BEFORE** (the audit, AC 1; unchanged tree at `570de70`, captured 2026-11-19T08:30:01Z, before any implementation commit):

Method: `npm --prefix web run build` of the unchanged tree, copied to `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t121/static-before/`; served by a fixture mock of the app routes (`/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t121/mock.py`, data from `web/src/test/fixtures.js` + the view tests via `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t121/dump.mjs`; `GET /__state?phase=N` re-sends the live snapshot with sessions in another state); headless Chrome through DevTools (`/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t121/shoot.py`) at 375 and 1280, every view, full-page. 43 screenshots: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t121/shots-before/<view>-<375|1280>.png`; slices for reading in `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t121/crops/`. Measurement log (one JSON line per shot: body font size, count of text nodes per font size, elements past the right edge, targets under 44 px at 375): `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t121/before.log`. Excerpt:

```
SHOT usage 375  {"hscroll": false, "bodyFs": "15px", "sizes": {"12.5px": 36, "13px": 21, "13.5px": 94, "14px": 10, "14.5px": 1, "15px": 9, "15.5px": 2, "16px": 7, ...}, "tapUnder44": ["BUTTON.menu-button \"Menu\" 36x36", "BUTTON.status-chip \"3 active\" 97x28", "BUTTON. \"Last 7 days\" 120x32", "SUMMARY. \"Show table\" 301x18", ...]}
SHOT live-list 1280 {"hscroll": false, "bodyFs": "15px", "sizes": {"12.5px": 47, "13px": 14, "13.5px": 16, "14px": 21, "15px": 22, "16px": 2, ...}}
```

No view scrolls sideways at 375 or 1280 (`hscroll: false` on all 43 shots; wide tables sit in their own scroll boxes). No text under 12 px in HTML (smallest 12.5 px, `--fs-label`); SVG labels of the memory graph are scaled down below that at 375 (F16).

| ID | View | Width | What is wrong | Evidence | Sev |
|---|---|---|---|---|---|
| F1 | all views | 375 / 1280 | Text is small: body 15 px and most text uses the smaller tokens (12.5 to 14.5 px): usage-1280 194 of 239 text nodes under 15 px, live-list-1280 98 of 127, past-list-375 189 of 191. B-e (16 px) fails | before.log `sizes` per shot | P1 |
| F2 | top bar | 375 | Status chips wrap onto three rows; the top bar takes about 150 px of a 900 px screen on every view | live-list-375.png, usage-375.png (top) | P2 |
| F3 | top bar | 375 / 1280 | The *N waiting* chip mounts and unmounts as sessions change state; at 375 it adds or removes a chip row, so the whole page jumps | StatusChips.jsx `waiting > 0 &&`; live-panel-busy-375 (1 waiting) vs live-panel-yourturn-375 (2 waiting) | P2 |
| F4 | all views | 375 | Touch targets under 44 px: menu button 36x36, status chips 28 high, period / view switches 32, most `main` buttons 32 (Copy, Check again, Rename, Ignore, Assign, Delete, Save alias, + Add workspace, By account), `Show table` summaries 18, checkboxes 13x13, Kit report toggles 18, Workflow gates 34, sidebar items 35 and group toggles 22, selects 36, an idle Live row button 18 | before.log `tapUnder44` | P2 |
| F5 | Live panel (terminal) | 375 / 1280 | **Drop**: the composer follows the state by swapping components: busy -> idle removes the Queue box, a hold at *your turn* replaces it by the Reply box, the hold ending swaps back. What the owner was typing and the focus are lost on each change | live-panel-busy-375.png (Queue box) vs live-panel-yourturn-1280.png (Reply box); Composer.jsx: each box keeps its own `text` state | P1 |
| F6 | Live panel | 375 / 1280 | **Drop**: the open panel closes by itself when its session leaves the snapshot (no record for 5 min, or it ended); a snapshot is re-sent whenever any session changes state, so a quiet session's panel vanishes when *another* session changes. FR-072 asks the panel to say the session is no longer active | LiveView.jsx effect `setOpenId(null) // the open session left the list` | P1 |
| F7 | Live list | 375 / 1280 | Rows re-sort on every batch of events (newest `last_ts` first) and on every state change: a row moves under the pointer while the owner aims at it | LiveView.jsx `group()` sorts by `last_ts`; live-list-1280 vs live-panel-yourturn-1280 (bbbbbbbb moved from third to second) | P2 |
| F8 | Live list, filter set | 375 / 1280 | With *Working* or *Idle* set, a row disappears under the pointer the moment its session changes state | LiveView.jsx `passes(facts[i], filters)` re-run on every state change | P2 |
| F9 | Live head | 375 | The *N sessions need you* line mounts and unmounts with the count; at 375 it is its own line, so everything below jumps | LiveView.jsx `needing > 0 &&`; live-list-375.png | P2 |
| F10 | Live panel | 375 / 1280 | The panel is keyed by the row key, which changes from the Claude id to `app:<id>` when the app list answers after the stream (and back when an app session leaves that list): the panel remounts, its conversation loads again and typed text is lost | LiveView.jsx `<SessionPanel key={selected.key}` + the effect that follows the key | P2 |
| F11 | Live | 375 | The session list starts at about 720 px, under a 12-line footnote inside the totals card: no row is in view without scrolling (B-a) | live-list-375.png | P2 |
| F12 | Live list | 375 | The list is a 70vh scroll box inside the scrolling page (a second scrollbar beside the filters) | live-list-375.png (bar right of the filters); app.css `.live-sessions { max-height: 70vh; overflow-y: auto }` | P2 |
| F13 | Workflow | 375 / 1280 | Stage names are cut to a few letters (`Stage 0 · De…`, `Stage 3 · Ex…`), at 1280 too | workflow-375.png, workflow-1280.png | P2 |
| F14 | Memory | 375 | Graph labels are drawn about 7 px high (the SVG is scaled down to the phone width) | memory-375.png | P2 |
| F15 | First run | 375 | The copy command box shows `easy-personal ing` with its scrollbar drawn over the text | first-run-375.png | P3 |
| F16 | Board | 375 / 1280 | `Done (41)` looks like a plain column title over an empty card; nothing says it opens | board-1280.png, board-375.png | P3 |
| F17 | Usage | 375 | The period switch wraps `All time` to a second row inside its box | usage-375.png | P3 |
| F18 | Usage, Scheduled | 375 | Wide tables (By tool, Agent jobs) scroll inside their card; the last columns are off screen until scrolled | usage-375.png, scheduled-375.png | P3 |
| F19 | sidebar | 1280 x 900 | The nav is its own scroll box; Settings sits under the fold of the sidebar | board-1280 / workflow-1280 (900 px tall shots) | P3 |
| F20 | Live row | 375 | A row with no state is a 18 px high button inside its card; the rest of the card does not open it | before.log `BUTTON.live-row "dddddddd…" 309x18` | P3 |

Counts: P1 3 (F1, F5, F6), P2 11 (F2-F4, F7-F14), P3 6 (F15-F20). Per view: all views 2, top bar 2, Live 8, Workflow 1, Memory 1, First run 1, Board 1, Usage/Scheduled 1, sidebar 1, Live row 1 (more P3 polish is listed in the cut list if left).
Not findings (mock artefacts): the transcript is drawn twice in the panel shots because the mock answers every poll with the same entries; the Watch graphs are short because the fixture has 5 samples.


**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T121.jsonl`, never the
implementing agent alone]
