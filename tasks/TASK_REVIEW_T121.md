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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | AC 3 (drops, F5-F11, F20): `web/src/LiveDrop.test.jsx` (12 tests: working -> idle -> needs you -> ended on one open panel, same panel node, typed text and focus kept; a quiet session's panel survives another session's change; id -> `app:<id>` keeps the panel node and text; order held under the pointer and under the focus; Working filter keeps a hovered row; new row goes last; need line same node; footnote in closed details; card click opens; 2 negative focus/text cases). Green: `Tests  12 passed (12)`. Other ACs: [other slices] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | AC 3 red run of `LiveDrop.test.jsx` on `2306a30` (before any drop fix): `Tests  10 failed (10)` — panel closed on leave ("Unable to find ... region Session in"), F10 "expected <section id=live-panel> to be <section id=live-panel>" (remounted), keyboard hold "expected [aaaaaaaa, …] to deeply equal [bbbbbbbb, …]" (re-sorted), need line / details / card click null or not expanded; full output in the section *AC 3 red run* below. The 2 negative tests (focus moved out is not taken back; no 'text is kept' claim with no text) pass on the fixed tree. Other ACs: [other slices] |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☑ fail / ☐ N/A | ux-evaluate after-run, 44 shots + 4 live-change shots in `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t121/shots-after/`: B-a..B-i table in *Demonstration AFTER*; Overall fail (open N1, N3, N4, N5, N7; N2 Supervisor call). F1-F20: 17 fixed, 3 partial, 0 not fixed |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass / ☐ fail / ☐ N/A | measured in the page (computed styles, `after-final.log`): body 16 px, smallest text 13 px, no text under 12 px on any of 44 shots; one accent green for actions, one button style; no source edited by this run. Caveat N2 (most text 13 to 15.5 px) |
| **UI: Responsiveness at target viewports** | ☐ pass / ☑ fail / ☐ N/A | 375 and 1280, every view: no horizontal page scroll (`hscroll: false` on 44 of 44 shots); 375 targets under 44 px: 155 -> 44 log entries, left checkbox labels 97x20 (Agents), memory nodes 27-33 px, drawer family links 19 px; 375 chip row scrolls sideways and hides the plan-limit chip (N1) |
| **AC 2: size (tokens, floor, B-e at 375)** | ☑ pass (AC wording) / ☐ fail | `body` 16 px on 44 of 44 shots; smallest text 13 px, `under12` empty everywhere (`/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t121/after-final.log`). Literal B-e reading on running text is open: N2 |
| **AC 4: every P1 and P2 audit finding fixed or written up** | ☐ pass / ☑ fail | Of the audit's P1/P2: F1 partial (N2), F4 partial (N3-N5), F2-F3, F5-F7, F8-F14 fixed (F8, F10 by test only). New P2 open: N1 (plan-limit chip off screen at 375), N2, N3 |
| **AC 5: ux-evaluate after-run, every view at 375 and 1280** | ☐ pass / ☑ fail | table in *Demonstration AFTER*: no horizontal page scroll; touch targets not all >= 44 px at 375 (N3, N4, N5); B-f/B-g not checked (read-only mock) |

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


**AFTER** (ux-evaluate after-run, AC 5; tree `4438247` = layout slice + drops slice combined; run 2026-11-19, no source edited):

Method: the same fixture mock as BEFORE (`/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t121/mock.py`, served from `src/easy_personal/dashboard/static` of this tree on port 8793, stopped after the run), the same shoot script (`shoot_ui.py`, headless Chrome through DevTools, full-page, 375 and 1280, every view). 44 screenshots plus 2 live-change shots: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t121/shots-after/<view>-<375|1280>.png`; measurement log `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t121/after-final.log` (same JSON line per shot as `before.log`). Two extra probes (no code touched): `live_change.py` (opens the busy session's panel, types text, focuses the box, then drives `/__state?phase=` 2 -> 0 -> 3 -> 2 -> 4 -> 0 and reads the DOM after each) and `probe_after.py` / `probe_after2.py` (rect measurements of the leftovers, a real mouse click on a card). Screenshots read through montages in `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t121/mont/`.

Measured, before -> after (`before.log` vs `after-final.log`):

| Measure | BEFORE | AFTER |
|---|---|---|
| `body` font size, every shot | 15 px | **16 px** |
| smallest text on any view | 12.5 px | **13 px** (nothing under 12 px: `under12` empty on all 44 shots) |
| text nodes under 15 px: usage-1280 / live-list-1280 / past-list-375 | 194/239, 98/127, 189/191 | 179/239, 77/128, 84/191 |
| text nodes under 16 px (same shots) | 222/239, 120/127, 189/191 | 194/239, 98/128, 189/191 (see N2) |
| horizontal page scroll (43 and 44 shots) | none | **none** (`hscroll: false` on every shot) |
| elements past the right edge at 375 | n/a | only the status-chip row (scrolls inside its own box, see N1) |
| top bar height at 375 | about 150 px | 113 px (139 px with the scope line); chips on one row, 44 px each |
| targets under 44 px at 375, entries in the log (cap 12 per shot) | 155 | 44 (left: see below) |

Live state-change run (panel open on session `bbbbbbbb`, typed `half typed reply` into its box, focus in the box; the mock moves sessions working -> your turn -> idle -> ended -> back). Result at 375 and at 1280, read from the DOM after every change:

```
phase 0 -> 2 (needs you)   panel kept, same node (marker kept), text kept, focus kept (TEXTAREA), Stop still there; chips "1 waiting" -> "2 waiting", top bar height 113 -> 113 (375) / 56 -> 56 (1280)
phase 2 -> 0 -> 3 -> 2     the same; "N sessions need you" line changes 2 -> 3 -> 2 in place
phase 4 (session ended)    panel kept, same node, says "This session is no longer active", no Stop, row removed from the list
phase 4 -> 0 (returns)     panel kept, same node; the box is empty again (the composer is hidden while the session is ended, see N9)
list order across 0/2/0/3/2 (1280): app:ap1, aaaaaaaa, bbbbbbbb, cccccccc, dddddddd every time; no row moved
```
Shots: `shots-after/livechange-ph2-375.png`, `livechange-ph4-375.png`, `livechange-ph2-1280.png`, `livechange-ph4-1280.png` (1280 ph4 read in `mont/d4.png`: "This session is no longer active: it left the live list. Go back to sessions to close this panel."). The scroll offset moved by 12 to 120 px between changes only because the mock re-sends the same transcript on every poll and the page grows (BEFORE note: mock artefact), not through the panel.

Card as the target (F20): idle row `dddddddd` card is 343 x 112; a real mouse click at the card's bottom edge (`Input.dispatchMouseEvent`) opened the panel (`aria-expanded="true"`); `elementFromPoint` at three points of the card returns the row button each time. The button's own text line is still 19 px high (`BUTTON.live-row 317x19`); the card is the target.

#### ux-evaluate verdict table (no UX rows in the guide: baseline checks B-a..B-i, per view, 375 / 1280)

Each cell is the nine letters a..i as `P` pass, `F` fail, `-` not applicable at that width. B-d is judged at 375 only (`-` at 1280). Common to every view, not repeated below: B-b P, B-c P, B-i P (one green action colour; amber and red only mean state; one button style and radius), B-e P on the token definition of AC 2 (body 16 px, floor 13 px, contrast clearly readable on the dark ground; running text is 14.5 to 15.5 px, see N2), B-f and B-g `not checked: the mock has data on every view and no mutation was run, so the empty, loading, error and success states and the feedback after an action were not reached; only the live-stream states (connected, working, needs you, idle, ended -> "no longer active") were seen`, which is why those two letters read `N` below.

| View | 375 | 1280 | Notes (screenshot in `shots-after/`) |
|---|---|---|---|
| Usage | a P, d P, h F, rest as common | all P except f, g N | h at 375 is N1 (chip row clipped); `usage-375.png` |
| Warnings and plan | `h F`, rest as common | pass | `warnings-375.png`; Run investigate and Copy are 44 px |
| Kit | `h F` | pass | `kit-375.png`; Show 2 more / Show details 44 px |
| Workflow | `h F`, d P | pass | stage names whole (F13 fixed) but gate labels cut: N6 |
| Workspaces | `h F`, d P | pass | |
| Agents | **`d F`**, `h F` | pass | checkbox labels 97 x 20 (input 13 x 13): N3. Rename / Ignore / By account 44 px |
| First run | `h F`, d P | pass | copy command wraps whole, Copy and Check again 44 px |
| Settings | `h F`, d P | pass | |
| Watch | `h F`, d P | pass | graphs short: fixture has 5 samples |
| Board | `h F`, d P | pass | `Done (41)` shows a chevron and a border (F16) |
| Scheduled | `h F`, d P | pass | table scrolls in its box with a right-edge fade (F18, partial) |
| Memory | **`d F`**, `h F` | pass | graph labels 12 px effective (was about 7); node hit areas 27 to 33 px high (circle 13 to 19 px): N4 |
| App sessions | `h F`, d P | pass | New session button in view at both widths (B-a P) |
| New session | `h F`, d P | pass | worktree checkbox 13 x 13 inside a 4-line label (tall enough); selects 44 px |
| Live list | `h F`, d P, **a P** | pass | first card starts at about 845 px of 900 (was about 1000), footnote folded (F11); filters 44 px; N8 |
| Live panel (busy / your turn / app / ended) | **`a F`**, `h F`, d P | a P | Queue / Reply box sits 1500 to 1800 px down at 375: N7 (older). 1280 panel is beside the list, in view |
| Past list | `h F`, d P | pass | row buttons 42 to 43 px, whole card is the target (same component as Live, F20 verified on Live only) |
| Past panel | `h F`, d P | pass | Resume / Fork 44 px |
| Menu drawer (375) | **`d F`**, h P | - | Easy Kit / Easy Memory / Easy Verifier links 183 x 19: N5; drawer scrolls (1325 in 900) |

ux-evaluate (letters judged across 19 view rows x 2 widths): B-a 1 view fail (Live panel at 375), B-b/B-c/B-i pass everywhere, B-d fail on 3 views at 375 (Agents, Memory, menu drawer), B-e pass on the AC 2 definition (N2 raises the literal reading), B-f/B-g not checked (mock), B-h fail at 375 on every view with the chip row (N1), pass at 1280. Overall: **fail** (open: N1, N3, N4, N5, N7; N2 is a Supervisor call).

#### Audit findings F1 to F20, after

| ID | Verdict | Evidence |
|---|---|---|
| F1 size | **fixed at token level, partial in effect** | body 16 px on all 44 shots, floor 13 px; most text nodes still 13 to 15.5 px (N2) |
| F2 chips on 3 rows | **fixed** | one row, 44 px each, top bar 113 px (was about 150); `live-list-375.png` vs `shots-before/live-list-375.png`. New side effect N1 |
| F3 *N waiting* chip mounts | **fixed** | chip always there; top bar 113 -> 113 px at 375 and 56 -> 56 px at 1280 between 1 and 2 waiting (live-change run) |
| F4 targets under 44 px | **mostly fixed** | menu button 44 x 44, chips, period and view switches, filters, Agents / Copy / Check again / Rename / Ignore / Review buttons all 44 px; log entries 155 -> 44. Left: checkbox labels (N3), memory nodes (N4), drawer family links (N5) |
| F5 composer swap loses text and focus | **fixed** | working -> your turn: first box value `half typed reply`, focus on it, same panel node (live-change run, both widths) |
| F6 panel closes on its own | **fixed** | ended: panel stays, says no longer active, Stop hidden; also kept while another session changed (phase 2 -> 0 -> 3) |
| F7 re-sort under the pointer | **fixed (order held)** | order identical across five changes at 1280; the pointer-hold and focus-hold rules are covered by `LiveDrop.test.jsx`, not re-shot here (no pointer was moved over the list) |
| F8 Working / Idle filter row vanishes | **fixed by test only** | not re-shot; `LiveDrop.test.jsx` covers it |
| F9 *need you* line mounts | **fixed** | the line stays; no jump at 375 across 2 -> 3 -> 2 |
| F10 panel key `id` -> `app:<id>` | **fixed by test only** | `app:ap1` row present from the first paint in the shots; `LiveDrop.test.jsx` covers the key change |
| F11 list under a 12-line footnote | **fixed** | footnote folded under "What these numbers and this page show"; first card at about 845 px |
| F12 inner 70vh scroll box at 375 | **fixed at 375** | `live-list-375.png` has no inner scrollbar; at 1280 the list is its own scroll column beside the panel (desktop two-column, by design) |
| F13 Workflow names cut | **fixed** | `Stage 0 · Define`, `Stage 1 · Set up` whole at 375 and 1280. Gate labels still cut: N6 |
| F14 memory labels 7 px | **fixed** | text 15 px x 0.8 = 12 px effective at 375 (log probe); labels still end in an ellipsis for long names |
| F15 copy command cut | **fixed** | `$ easy-personal ingest` wraps whole with Copy beside it (`first-run-375.png`) |
| F16 Done (41) looks plain | **fixed** | bordered button with a chevron at both widths |
| F17 period switch wraps | **fixed** | `Last 7 days | Last 30 days | All time` on one row at 375 |
| F18 wide tables | **partial** | still scroll in their card (By tool, Scheduled, Effectiveness); a right-edge fade now shows there is more; last columns still off screen until scrolled |
| F19 sidebar scrolls, Settings under fold | **fixed at 1280 x 900** | `sidebar` scrollHeight = clientHeight = 900 on every 1280 shot, Settings and the family links visible. At 375 the drawer scrolls (1325 in 900) by design, see N5 |
| F20 idle Live row 18 px | **fixed (card target)** | card 343 x 112, real click at its bottom edge opened the panel; the text line alone is still 19 px high |

Counts: fixed 17 (F2, F3, F5-F17, F19, F20; F8 and F10 by test only), partial 3 (F1, F4, F18), not fixed 0.

#### New findings (not fixed, per the brief)

| ID | View | Width | What is wrong | Evidence | Sev |
|---|---|---|---|---|---|
| N1 | top bar, every view | 375 | The chip row is 588 px of content in a 343 px box that scrolls sideways: the red plan-limit chip (`5h 92%`, left 489, right 604 on a 359 px screen) is entirely off screen and the ingest chip is cut; a scrollbar thumb is drawn over the bottom of the first chip. The most urgent chip is the one hidden. B-h (clipped controls) fails on every view | `probe_after.py` CHIPS: `scrollWidth 588, clientWidth 343, overflow-x auto`; `shots-after/live-list-375.png`, `usage-375.png` (grey bar under "3 active") | P2 |
| N2 | all views | 375 / 1280 | Only the `body` element is 16 px. Running text is 14.5 to 15.5 px (`main p`) and 85 percent of the text nodes are 13 to 15.5 px (usage-375 162 of 188 under 16, past-list-375 189 of 191, usage-1280 194 of 239). The owner said "the size is small"; on the screen it still reads close to before (usage-1280 under 15 px: 194 -> 179 of 239). The AC 2 wording is met, the B-e literal reading ("body at least 16 px", paragraph text included) is not. Supervisor call: raise the secondary tokens or accept | `after-final.log` `sizes` per shot, `mainP` | P2 |
| N3 | Agents, any checkbox list | 375 | The checkbox is 13 x 13 inside a label 97 x 20: B-d (44 px) fails; ten of them on Agents (`company`, `side` per account) | `probe_after.py` CHECKBOX; `agents-375.png` | P2 |
| N4 | Memory | 375 | Node hit areas are 27 to 33 px high (the dot is 13 to 19 px); the graph canvas is 640 px wide in a 343 px box, so nodes at the edges (`tokens.cs…`, `0001.md`, `app`) are cut until panned | `probe_after.py` MEMNODES (`g` 86x33, 65x27, 58x27 ...); `memory-375.png` | P3 |
| N5 | menu drawer | 375 | `Easy Kit`, `Easy Memory`, `Easy Verifier` links are 183 x 19 (the rest of the drawer is 44 px) | `probe_after.py` SIDEBAR; `menu-open-375.png` | P3 |
| N6 | Workflow | 375 / 1280 | Gate labels are still cut with an ellipsis (`No TASK_GUIDE,…`, `Ready for Review start…`) | `workflow-375.png`, `workflow-1280.png` (`mont/d1.png`) | P3 |
| N7 | Live panel (terminal) | 375 | The Queue / Reply box is 1500 to 1800 px down the page (after the metadata list and the whole transcript), so the primary action is far below the fold (B-a). Older than this task (BEFORE shots show the same order) | `live-panel-busy-375.png`, `live-panel-yourturn-375.png` | P3 (older) |
| N8 | Live list | 375 | The four filter chips wrap 3 + 1: `Idle (2)` sits alone and centred on a second row | `live-list-375.png` | P3 |
| N9 | Live panel | 375 / 1280 | When the session ends the composer is hidden, so the typed text is gone when the session comes back (box empty again at phase 4 -> 0). By design of the F6 rule (the panel says no longer active and hides the composer) but the text of a half written reply is lost with it | live-change run phase 4 -> 0: `ta ""` | P3 |
| N10 | top bar | 375 | Still 113 to 139 px of a 900 px screen (13 to 15 percent) with a sticky bar; 139 px on the views that add the scope line | `after-final.log` `topH` | P3 |

Not findings (mock artefacts, as in BEFORE): the transcript is drawn several times in the panel shots; the Watch graphs are short.
Not covered: B-f / B-g on empty, loading, error and success states and on mutating actions (the mock is read-only); the F7 and F8 pointer-hold behaviours were not re-shot (tests only); no `rust/` or real-data run.


**DELTA**: Text is one step larger (body 16 px, floor 13 px), the top bar is one 44 px row instead of three, every main button and row is a 44 px target, and an open Live panel (typed text, focus, scroll, row order) now stays put while sessions move working -> your turn -> idle -> ended, saying *no longer active* instead of closing; what is left is the plan-limit chip hidden on the phone bar (N1), small checkbox and memory-node targets (N3, N4) and the text-size call (N2).

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T121.jsonl`, never the
implementing agent alone]

## AC 3 — drops on status change (slice: Composer / LiveView / SessionPanel)

Fixed: F5, F6, F7, F8, F9, F10, F11, F20. Full suite after: `Test Files  43 passed (43)`, `Tests  1433 passed (1433)`; `npm run build` ok; `check_static: ok`.

**F6 rule change (Supervisor decision, approved; FR-072 "the panel says it is no longer active").** The open panel no longer closes when its session leaves the snapshot: it keeps the last known entry, says *This session is no longer active*, hides Stop / Interrupt / Close, the cards and the composer, and Back closes it; an app session the owner closes with *Close* still closes the panel. Earlier tests that pinned the old rule, rewritten to the new one:
- `web/src/LiveView.test.jsx` — `AC7: when the open session leaves the list nothing is drawn…` -> `AC7 (T121, FR-072): … its panel stays and says it is no longer active; Back closes it and resets the open id`; `AC7: the last session leaving draws no expanded area…` -> `AC7 (T121): the last session leaving keeps its panel saying no longer active, and the empty notice shows`.
- `web/src/App.test.jsx` — `AC5: a selected session that leaves the list clears the selection…` -> `AC5 (T121, FR-072): … has no row left; its panel says it is no longer active; no error`.
- `web/src/LivePanel.test.jsx` — `a session that leaves the list within 5 s never offers Force stop`: now asserts the panel says no longer active and has no Stop, instead of the panel being gone.

Also changed for F9: the *need you* line keeps one node; it carries `role="status"` only while the count is above 0 (LiveView.test AC4 counts one status region at zero).

### AC 3 red run

`cd web && npx vitest run src/LiveDrop.test.jsx` on `2306a30`, before the fixes:

```
     × working -> idle -> needs you -> ended: the same panel node; the typed text and the focus are kept; ended says no longer active 212ms
     × another session changing state does not close or redraw the open panel of a quiet one 83ms
     × the app list answering after the stream (id -> app:<id>) keeps the same panel node and text 65ms
     × while the pointer is on the list a re-sort or a new state keeps the order; leaving it lets the list sort again 12ms
     × the keyboard focus on a row holds the order too, until the focus leaves the list 17ms
     × with Working set, a hovered row whose session goes idle stays listed until the pointer leaves 13ms
     × a row added while the pointer is on the list goes at the end, the others keep their place 10ms
     × the "need you" line is always drawn: the same node with 0 and with 1 7ms
     × the footnote of the totals is folded in a closed details 8ms
     × a click anywhere on the row card opens it, not only on its first line 9ms
⎯⎯⎯⎯⎯⎯ Failed Tests 10 ⎯⎯⎯⎯⎯⎯⎯
 FAIL  src/LiveDrop.test.jsx > the open panel stays put (F5, F6) > working -> idle -> needs you -> ended: the same panel node; the typed text and the focus are kept; ended says no longer active
TestingLibraryElementError: Unable to find an element with the text: /Your text is kept/. This could be because the text is broken up by multiple elements. In this case, you can provide a function for your text matcher to make your matcher more flexible.
 FAIL  src/LiveDrop.test.jsx > the open panel stays put (F5, F6) > another session changing state does not close or redraw the open panel of a quiet one
TestingLibraryElementError: Unable to find an accessible element with the role "region" and name `/^Session in /`
 FAIL  src/LiveDrop.test.jsx > the panel is not remounted when its row key changes (F10) > the app list answering after the stream (id -> app:<id>) keeps the same panel node and text
AssertionError: expected <section id="live-panel" …(2)>…(6)</section> to be <section id="live-panel" …(2)>…(9)</section> // Object.is equality
 FAIL  src/LiveDrop.test.jsx > rows do not move or vanish under the pointer (F7, F8) > while the pointer is on the list a re-sort or a new state keeps the order; leaving it lets the list sort again
Error: Unable to fire a "pointerenter" event - please provide a DOM element.
 FAIL  src/LiveDrop.test.jsx > rows do not move or vanish under the pointer (F7, F8) > the keyboard focus on a row holds the order too, until the focus leaves the list
AssertionError: expected [ 'aaaaaaaa-1111-2222', …(1) ] to deeply equal [ 'bbbbbbbb-3333-4444', …(1) ]
 FAIL  src/LiveDrop.test.jsx > rows do not move or vanish under the pointer (F7, F8) > with Working set, a hovered row whose session goes idle stays listed until the pointer leaves
Error: Unable to fire a "pointerenter" event - please provide a DOM element.
 FAIL  src/LiveDrop.test.jsx > rows do not move or vanish under the pointer (F7, F8) > a row added while the pointer is on the list goes at the end, the others keep their place
Error: Unable to fire a "pointerenter" event - please provide a DOM element.
 FAIL  src/LiveDrop.test.jsx > the head and the list do not jump (F9, F11, F20) > the "need you" line is always drawn: the same node with 0 and with 1
AssertionError: expected null not to be null
 FAIL  src/LiveDrop.test.jsx > the head and the list do not jump (F9, F11, F20) > the footnote of the totals is folded in a closed details
AssertionError: expected null not to be null
 FAIL  src/LiveDrop.test.jsx > the head and the list do not jump (F9, F11, F20) > a click anywhere on the row card opens it, not only on its first line
Error: [2mexpect([22m[31melement[39m[2m).toHaveAttribute([22m[32m[32m"aria-expanded"[32m[39m[2m, [22m[32m[32m"true"[32m[39m[2m) // element.getAttribute("aria-expanded") === "true"[22m
      Tests  10 failed (10)
```


## AC 3 — Round 2 (code review P2/P3 + after-run N7/N8)

| Item | Cause | Fix | Test |
|---|---|---|---|
| P2-1 Back/Esc focus held the order | `backTo` focuses the row inside `.live-rows`; `onFocus` set the hold for mouse users too | the hold takes only keyboard focus (`:focus-visible`, a browser that cannot tell counts as yes) and never our own Back focus (`quietFocus` ref) | LiveDrop "Back puts the focus on the row without holding the list…" |
| P2-2 hold stuck after the focused row left | an unmounted row sends no blur; rows were keyed by `e.key`, which flips id ↔ `app:<id>` | rows keyed by `rowId(e)` (session id, else key); an effect on every `groups` change lets go of the focus hold when `document.activeElement` is not in `.live-rows` | LiveDrop "the focused row leaving the list lets go of the hold…" |
| P2-3 summary lost its triangle | `.live-about > summary { display: flex }` | `display: list-item`, `min-height: 44px`, `padding-block: 10px` (triangle back, see shot below) | CSS, shot |
| P3-a card click with a text selection | the card's click opened the row | skip when `getSelection()` is non-empty | LiveDrop "a click that ends a text selection on the card does not open it" |
| P3-b need line role | role only when count > 0 | always `role="status"`. This really clashed with 3 tests that used a single `getByRole('status')`: LiveView.test AC3 drop / closed-stream now read the link status through `linkStatus()` (the `.live-status` region); AC4 now expects 2 status regions at zero (link + empty need line) | LiveDrop "the need line is a status region at every count"; LiveView.test AC3, closed stream, AC4 |
| N7 composer below the fold at 375 | composer is the last child of a long panel | under 768 px `.session-panel > .composer` is `position: sticky; bottom: 0` with `--color-bg`, a top border, and the card padding pulled to the edges; 1280 unchanged | CSS; shot `…/ux-t121/shots-r2b/live-panel-busy-375.png` (full-page shot, so the stick itself shows only when scrolling) |
| N8 filter chips 3+1 at 375 | flex wrap of `.periods` | under 768 px the State group is a 2×2 grid, left aligned | shot `…/ux-t121/shots-r2b/live-list-375.png`: All / Needs me, Working / Idle |
| N9 | — | accepted as the F6 rule, no change | — |

Red run of the 4 new LiveDrop tests on `20a118f`, before the fixes:

```
     × Back puts the focus on the row without holding the list: a row that became the newest moves up (round 2) 35ms
     × the focused row leaving the list lets go of the hold: the list sorts again (round 2) 22ms
     × a click that ends a text selection on the card does not open it (round 2) 14ms
     × the need line is a status region at every count (round 2) 6ms
⎯⎯⎯⎯⎯⎯⎯ Failed Tests 4 ⎯⎯⎯⎯⎯⎯⎯
      Tests  4 failed | 12 passed (16)
```

Green: `npx vitest run` → `Test Files  43 passed (43)`, `Tests  1440 passed (1440)` (one run showed WorkspacesView.test "AC4: the Confirm/Cancel plan…" failing under load; it passes alone, 52/52, and in the next full run); `npm run build` ok; `check_static: ok`. Shots (mock, headless Chrome) in `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t121/shots-r2b/`: no horizontal scroll at 375 or 1280.
