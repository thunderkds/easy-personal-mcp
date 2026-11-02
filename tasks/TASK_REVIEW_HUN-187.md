# TASK_REVIEW — HUN-187: [Short Title]

> Sibling of `tasks/TASK_GUIDE_HUN-187.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/BoardView.test.jsx` (32 tests, AC1-AC5 + edge cases); AC6/AC8 by real run below. Implementer evidence; Supervisor to confirm. |
| Verification command run | ☑ pass | `cd web && npx vitest run`: Test Files 17 passed (17), Tests 387 passed (387); `uv run pytest tests/test_web_toolchain.py tests/test_site_content.py -q`: 49 passed in 3.65s (after `npm --prefix web run build`). |
| Negative cases hold | ☑ pass | No board anywhere → "No project on this machine has a board file." + toggle; `gone` missing/0 → no line; duplicate folder names → distinct picker labels; `snake_case` and `2*3*4` titles kept; scope change resets the choice (tests in `web/src/BoardView.test.jsx`). Real run: 0 raw `**`/`__`/backtick titles of 113 cards — pass |
| verify | ☑ pass | Stage 5 2026-11-02 @ 5fd082d (Supervisor): `npm --prefix web run build`, `uv run easy-personal dashboard` from the worktree on the owner's store, headless Chrome driver `scratchpad/v187/v.mjs`. 1440x900: picker 11 options (10 projects + All), opens on index 0, 5 cards, no project names on cards, height 900, no overflow; All projects: 113 cards, 113 with project names, no overflow; 0 raw markdown titles; `-webkit-line-clamp` 3, max title box 51 px; toggle reached by keyboard (focus + Enter) → `aria-expanded="true"`; 0 console errors (besides the known favicon 404). 390x844: same, height 1274, no overflow — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed `web/src/BoardView.jsx`, `web/src/app.css` (Board block), `web/src/BoardView.test.jsx`, and the producer `core/board.py` response shape (unchanged). `lane: C2, 3 files, 393 lines, FITS`. Stage 4 code-review: 0 P0, 1 P1 (process: `agents/frontend.md` read only to line 40 — fixed 5fd082d, agent read it in full, no code change needed), 0 P2/P3 code findings — pass |
| Full smoke suite still green (no regression) | ☑ pass | @ 5fd082d: `cd web && npx vitest run` → Test Files 17 passed (17), Tests 387 passed (387); no Python code changed — pass |
| **Companion score (easy-verifier)** | ☑ N/A | Not run: learnings 2026-11-01 — easy-verifier refused every repo path |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | after: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun187/after/wide-Board.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun187/after/narrow-Board.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun187/after/wide-All.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun187/after/narrow-All.png` (before: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun187/before/*.png`). One project's columns above the fold at 1440x900; no legend wall; All projects shows project names on cards. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Existing token test in BoardView.test.jsx passes (no colour literals in .board/.project-dot rules); computed h3 font is IBM Plex Mono. Picker has label `project`; toggle has aria-expanded; article accessible name = id + full title (tested). |
| **UI: Responsiveness at target viewports** | ☑ pass | 1440x900 and 390x844 real run: scrollWidth 1440/1440 and 390/390, no overflow (before 736 > 390). |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-02T08:21:44Z (wide) and 08:21:52Z (narrow) from current main de96ca4, worktree build, owner's real store, `VIEWS=Board node all.mjs <url> /tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun187/before`:

```
1440x900: page height 19580 px, horizontal overflow false (scrollWidth 1440), projects listed 149 (ul[aria-label=Projects] li), cards 113
390x844 : page height 21128 px, horizontal overflow TRUE (scrollWidth 736 > innerWidth 390), projects listed 149, cards 113
GET /api/board: 149 projects (139 status "no data", 10 ok, 3 with a board kind), no top-level "gone" key
```

Screenshots: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun187/before/wide-Board.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun187/before/narrow-Board.png`; raw numbers `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun187/before/info.json`.

**AFTER**: captured 2026-11-02T08:26:13Z (wide) and 08:26:23Z (narrow), worktree build of this branch, same owner store:

```
1440x900: page height 900 px (was 19580), horizontal overflow false (scrollWidth 1440), picker lists 10 projects with a board and opens on 1; 143 others behind the "143 without a board" toggle (was 149 listed in a wall), cards 5 (was 113)
390x844 : page height 1274 px (was 21128), horizontal overflow false (scrollWidth 390 vs innerWidth 390; was 736), cards 5
All projects, 1440x900: scrollWidth 1440, page 13442 px with the 143 names expanded; longest title box 51 px = 3 lines
```

**DELTA**: the owner opens the Board on one project, picks another or All projects, and reads clamped plain-text cards on a phone without sideways scroll.

**WITNESS**: Supervisor, 2026-11-02, Stage 5 verify run @ 5fd082d (`scratchpad/v187/v.mjs`, screenshots `scratchpad/v187/wide.png`, `narrow.png`), independent of the implementing agent.
