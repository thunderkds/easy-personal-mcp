# TASK_REVIEW — HUN-182: Live view attention badge, border, header count, sort (FR-050; after HUN-181)

> Sibling of `tasks/TASK_GUIDE_HUN-182.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/LiveView.test.jsx` (describe 'attention (FR-050)': 12 tests for AC1-AC7, plus 1 design test for token-only pill/border colours). Before the change 11 of them failed (see BEFORE); after: `Tests 52 passed (52)` for `LiveView neon`. |
| Verification command run | ☑ pass | 2026-11-02T04:40Z `cd web && npx vitest run LiveView neon` -> `Test Files 2 passed (2) / Tests 52 passed (52)`; `uv run pytest tests/test_site_content.py -q` -> `33 passed in 1.23s`. |
| Negative cases hold | ☑ pass | No state → no pill, no border, no header count (N=0 renders nothing); `tool: null` → "unknown tool" / "N in a row"; unknown state names dropped; a session with a state is never idle; live events never change `attention`. Stage 4 added: two `failing` items (main + subagent stream, `core/live.py:450` makes one per stream) → two pills, no duplicate React key (test fails before fix 4024f8d with "Encountered two children with the same key") — pass |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed `web/src/LiveView.jsx`, `web/src/app.css` (Live block), `web/src/LiveView.test.jsx`, and the producer contract `core/live.py` `_attention` (direct upstream). `lane: C2, 3 files, 199 lines, FITS`. Startup reads: all 3 Permanent-Rule reads present. Stage 4 code-review: P1 duplicate React key when two streams share a state — fixed 4024f8d + regression test; P0/P2/P3: none. Security: not required (Risk Low); no new data read, page renders server fields only — pass |
| Full smoke suite still green (no regression) | ☑ pass | After fix 4024f8d (Supervisor, 2026-11-02): `cd web && npx vitest run` -> `Test Files 17 passed (17) / Tests 371 passed (371)`; `uv run pytest tests/test_site_content.py tests/test_web_toolchain.py -q` -> `47 passed, 2 skipped` (no Python code changed) — pass |
| **Companion score (easy-verifier)** | ☑ N/A | Not run: learnings 2026-11-01 — easy-verifier refused every repo path |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Fixture stream (fake SSE server + built page, no real store), 1280x720: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/shots/live-wide.png`. Verdict: matches the owner mockup (card head = id, pill, active/idle count; red / green / amber card border by state; header '4 sessions need you'); stateless cards keep the v0.3.0 style. Script: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/shoot.mjs`. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Test 'colors the pill and the card border from tokens only' + existing 'Live rules ... only var(--...) colors' and '4.5:1' tests (the contrast test now also covers the red, accent and amber pill/header colours on --color-bg and --color-raised). Computed style in the screenshot run: pill font-family `"IBM Plex Mono", ...`, pill colour rgb(239,123,123) = --color-red, failing card border rgb(239,123,123). Text label is always in the pill. Only one theme (dark) exists in tokens.css, so 'both themes' = the one theme. |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280x720 and 375x800 (rail collapsed behind Menu): `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/shots/live-wide.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/shots/live-narrow.png`. `document.documentElement.scrollWidth` = innerWidth in both (1280/1280, 375/375): no horizontal page scroll. Pill wraps under/next to the id; long tool name `mcp__claude_ai_Linear__save_issue` is cut with an ellipsis and its full text is in `title`. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured on main @ 16eb0c2 code, with the new attention tests written but no implementation commit; command `cd web && npx vitest run LiveView`, output filtered to results, first line is the UTC time):

```
2026-11-02T04:36:31Z
     × AC1: a failing state in a subagent stream renders the tool, the count and "agent" in the card head 12ms
     × AC2: waiting and needs attention are worded as the guide says 8ms
     × AC2: an unknown tool reads "unknown tool"; a failing streak of mixed tools reads "N in a row" 10ms
     × several states on one session give one pill each, failing first; the border takes the first state 6ms
     × the border state is on the card, one value per state, and absent without a state 11ms
     × a long tool name is cut by CSS but the whole text is in the title 7ms
     × a live event does not change the states of a session 10ms
     × AC4: the header counts sessions with any state in a status region, and hides it at zero 13ms
     × AC5: inside a project a card with a state comes first; ties keep the newest first 19ms
     × AC6: a session with a state is never shown as idle, even when it is quiet 8ms
     × colors the pill and the card border from tokens only, one rule per state, and never by color alone 1ms
⎯⎯⎯⎯⎯⎯ Failed Tests 11 ⎯⎯⎯⎯⎯⎯⎯
 FAIL  src/LiveView.test.jsx > attention (FR-050) > AC1: a failing state in a subagent stream renders the tool, the count and "agent" in the card head
AssertionError: expected [] to deeply equal [ 'failing: Edit ×3 · agent' ]
-   "failing: Edit ×3 · agent",
    358|     expect(pills('aaaaaaaa')).toEqual(['failing: Edit ×3 · agent'])
 FAIL  src/LiveView.test.jsx > attention (FR-050) > AC2: waiting and needs attention are worded as the guide says
 FAIL  src/LiveView.test.jsx > attention (FR-050) > AC2: an unknown tool reads "unknown tool"; a failing streak of mixed tools reads "N in a row"
 FAIL  src/LiveView.test.jsx > attention (FR-050) > several states on one session give one pill each, failing first; the border takes the first state
AssertionError: expected [] to deeply equal [ 'failing: Edit ×3', …(2) ]
-   "failing: Edit ×3",
    386|       'failing: Edit ×3',
 FAIL  src/LiveView.test.jsx > attention (FR-050) > the border state is on the card, one value per state, and absent without a state
 FAIL  src/LiveView.test.jsx > attention (FR-050) > a long tool name is cut by CSS but the whole text is in the title
 FAIL  src/LiveView.test.jsx > attention (FR-050) > a live event does not change the states of a session
AssertionError: expected [] to deeply equal [ 'failing: Edit ×3 · agent' ]
-   "failing: Edit ×3 · agent",
    430|     expect(pills('aaaaaaaa')).toEqual(['failing: Edit ×3 · agent'])
 FAIL  src/LiveView.test.jsx > attention (FR-050) > AC4: the header counts sessions with any state in a status region, and hides it at zero
 FAIL  src/LiveView.test.jsx > attention (FR-050) > AC5: inside a project a card with a state comes first; ties keep the newest first
 FAIL  src/LiveView.test.jsx > attention (FR-050) > AC6: a session with a state is never shown as idle, even when it is quiet
 FAIL  src/LiveView.test.jsx > design > colors the pill and the card border from tokens only, one rule per state, and never by color alone
 Test Files  1 failed (1)
      Tests  11 failed | 33 passed (44)
   Duration  1.39s (tests 62%, environment 30%, setup 5%, import 2%, transform 1%)
```

**AFTER** (2026-11-02T04:40:14Z, same file, after the change): `cd web && npx vitest run LiveView neon` -> `Test Files 2 passed (2)`, `Tests 52 passed (52)`. Screenshot: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/shots/live-wide.png` shows `failing: Edit ×3 · agent`, `waiting for you: question`, `needs attention: mcp__claude_ai_Linear__s…` pills with red/green/amber borders and the header text `4 sessions need you`.

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
