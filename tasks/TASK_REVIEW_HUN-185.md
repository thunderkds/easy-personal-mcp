# TASK_REVIEW — HUN-185: Memory view easy-memory layer: cited-file nodes, stale mark, 30-day panel, list count (FR-052..053 page; after HUN-184)

> Sibling of `tasks/TASK_GUIDE_HUN-185.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/MemoryView.test.jsx` (55 tests, 33 new, 1 existing picker test updated on purpose; was 22): AC1 ring 4/kind/radius, AC2 mark + accessible name + "not checked", AC3 click panel, AC4 panel/sparkline/hit rate/"no prompts"/0-total flat line, AC5 `easy_memory` null or missing, AC6 list count + easy-memory-only project, AC7 determinism with ring 4. Run 2026-11-02T10:19:57Z: `Test Files 3 passed (3)  Tests 105 passed (105)` (MemoryView App neon) |
| Verification command run | ☑ pass | `cd web && npx vitest run MemoryView App neon && cd .. && uv run pytest tests/test_web_toolchain.py tests/test_site_content.py -q` → `Tests 105 passed (105)` then `49 passed in 4.23s` (2026-11-02T10:19:57Z, after `npm --prefix web run build`; before the rebuild 2 toolchain tests fail with `static build stale`, as the learning predicts) |
| Negative cases hold | ☑ pass | `easy_memory: null` and field missing → no panel, 3 rings, `viewBox="0 0 640 640"`, no legend entry; `prompts_on: 0` → "no prompts" (no NaN / 0%); all-zero `days` → 30 equal-height bars, "0 notes"; easy-memory-only project; planted `SECRET_P/A/S` fields on nodes and block never reach the DOM; ring 4 kept when the other nodes hit `MAX_DRAWN`; golden `web/src/test/memory-graph.golden.svg` unchanged (`git diff --stat` empty) |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | `cd web && npx vitest run` → `Test Files 18 passed (18)  Tests 446 passed (446)`; `uv run pytest -q -x --ignore=tests/e2e` → `1116 passed, 6 deselected, 1 warning in 113.86s`; `tests/e2e` not run (nothing it covers changed; page-only task) |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Real run, Chrome headless via playwright-core, owner's machine, dashboard on a read-only copy of the real store (`EASY_PERSONAL_HOME` = scratch copy; the real `~/.easy-personal/store.sqlite` and `~/.easy-memory/**` were only read), Memory → easy-personal-mcp: 32 nodes, 28 easy-memory on ring 4, 3 merged kit nodes with a mark, 13 stale marks, all "changed since the newest note". Screenshots: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun185/shots/memory-wide.png` (1280×720 viewport, full page), `memory-wide-click.png` (an easy-memory node selected), `graph-only.png`, `memory-narrow.png`. Read by eye: ring 4 readable, long names cut to 10 chars with every second label above its node, a few same-prefix names (BRAINSTOR…) repeat and sit close; panels sit under the graph so no label meets a panel. No `gone` node in the real data: `gone` is covered by vitest (mark ×, name "file gone"). Without easy-memory: golden SVG test unchanged |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | vitest `design tokens and contrast for the easy-memory layer`: node = `var(--color-heading)`, mark = `var(--color-red)`, sparkline = `var(--color-accent)`, counts and dates in `var(--font-mono)`; the existing "no raw colour literals on `.memory` lines" test still passes. Computed from `tokens.css`: mark symbol on its disc 6.86:1, shapes on the graph background 7.08–16.19:1 (need 3:1), panel text on the page 5.9–14.7:1 (need 4.5:1). Real run: panel font computed `"IBM Plex Mono", ui-monospace…`, node fill `rgb(234, 237, 234)` = `--color-heading`. Stale mark has an accessible name (`role="img"`, `aria-label` "changed since the newest note" / "file gone") and a symbol (!, ×), so never colour alone; sparkline `aria-label` "Notes saved per day, last 30 days: 116 in the window, 116 notes in total". `tokens.css` has a dark theme only, so "both themes" is one theme. No token for a blue/violet exists: the easy-memory node uses the neutral `--color-heading` (ask the Supervisor if another hue is wanted) |
| **UI: Responsiveness at target viewports** | ☑ pass | Real run measured in Chrome: 1280×720 `scrollWidth 1280 = innerWidth 1280`, graph 702 px wide; 375×800 (clicked `Menu` first) `scrollWidth 375 = innerWidth 375`, graph 341 px, panels stacked under it. Screenshots as above. At 1280 the 18 rem side column would leave the graph 394 px wide (ring 4 text ~6 px), so with ring 4 the panels go under the graph in two columns (`.memory-body[data-wide]`) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-02T10:13Z UTC, worktree HUN-185 at main 9a47d59, before any implementation commit):

1. Web tests of the files this task touches, current page code:
```
$ cd web && npx vitest run MemoryView App neon        # started 2026-11-02T10:13:21Z
 Test Files  3 passed (3)
      Tests  72 passed (72)
```
2. The page code has no easy-memory layer. `grep -n "easy" web/src/MemoryView.jsx` prints only the two lines
that count easy-memory notes for the default project (HUN-188), nothing that draws a node, mark or panel:
```
145:const noteCount = (p) => (p.kit_notes ?? 0) + (p.claude_notes ?? 0) + (p.easy_memory_notes ?? 0)
149:/** Real project paths first, then the most notes (kit + Claude + easy-memory), then the lower name. */
```
3. The data the page ignores (HUN-184 backend, real read-only call on the owner's machine, store opened `mode=ro`,
`2026-11-02T10:13:45Z`, `uv run python real_graph.py <this repo>`):
```
nodes 32 easy_memory nodes 27 merged 3
stale: ['fresh', 'stale']
{"notes": 115, "abstained": 0, "days": [... "2026-10-29": 49, "2026-10-31": 26, "2026-11-01": 11, "2026-11-02": 29],
 "prompts_on": 12, "injected": 5, "files_cited": 46, "files_not_drawn": 16, "outside": 0}
top node: {'path': 'PROJECT_KANBAN.md', 'cited': 11, 'newest': '2026-11-02', 'stale': 'stale'}
```
So the API sends 27 `easy_memory` nodes (ring 4) and a panel block, but `RingsGraph` has no ring 4 (`RADII` has 4 entries)
and `KIND_NAMES` has no `easy_memory` entry: such a node would be labelled "<name>, undefined".

**AFTER** (2026-11-02T10:22Z, real run on the owner's machine, dashboard on a read-only copy of the store, `node real.mjs`):
```
wide   {"scrollW":1280,"innerW":1280,"nodes":32,"easy":28,"marks":13,
        "panelText":"easy-memory | 116 notes | saved per day, last 30 days | Hit rate 41.7% of 12 prompts | 16 files not drawn"}
click  board.json | easy-memory file | .claude/board.json | cited by 1 note | newest note 2026-11-02 | unchanged since the newest note
list   ['hungnguyenhuu · easy-memory 2', 'plugins · easy-memory 1', 'ghostty · easy-memory 2', 'easy-personal-mcp · kit 4 · easy-memory 116' ...]
```

**DELTA**: Memory → easy-personal-mcp now draws the files easy-memory notes cite on a fourth ring with a stale mark, shows notes per day, hit rate, files not drawn and unreadable notes, and the project list shows the easy-memory count.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
