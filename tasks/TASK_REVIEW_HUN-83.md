# TASK_REVIEW — HUN-83: Dashboard dark theme, sidebar shell and Usage widget cards

> Sibling of `tasks/TASK_GUIDE_HUN-83.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/Widgets.test.jsx` (new, 11 tests: trend, sparkline 1/0/zero values, ring/progress null), `web/src/App.test.jsx` (sidebar, aria-current, top-bar title, menu aria-expanded, filters in top bar), `web/src/UsageView.test.jsx` (+12: 3 KPIs, 1,500 vs 1,000 → ↑ 50.0%, previous 0 → no previous data, All time no previous request, since/until window + filters, failed previous fetch, sparkline label, cache null, abstained cost). Supervisor run 2026-10-29T07:06:52Z: `Test Files  8 passed (8)` / `Tests  122 passed (122)` |
| Verification command run | ☑ pass | `npm --prefix web test && npm --prefix web run build && uv run pytest -q` (Supervisor, worktree, 2026-10-29T07:06:52Z): `Tests  122 passed (122)`; `✓ built in 134ms`; `748 passed, 4 deselected, 1 xfailed, 1 warning in 210.93s` |
| Negative cases hold | ☑ pass | previous 0 → `no previous data` (no Infinity/NaN); null cache ratio → `no data` in progress + ring, never 0%; failed previous fetch → view renders; no day groups → no sparkline; abstained cost named, never $0 |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | code-review on `main..HUN-83`: App.jsx, UsageView.jsx, Widgets.jsx, tokens.css, app.css + their tests; other views only for the selector edits and screenshots. lane_check: `C2, 11 files, 770 lines, EXCEEDS (lines 770 > 500)` — 600+ of those lines are tests + CSS; C2 kept. Findings: 0 P0, 0 P1, 1 P2 (agent report named `-top.png` crops that were never written — process), 1 P3 (cost ↑ shown green like the other KPIs). security-review: not required (Risk Low, no API/input/secret change) |
| Full smoke suite still green (no regression) | ☑ pass | full pytest `748 passed, 4 deselected, 1 xfailed`; all 8 Vitest files pass |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier MCP is not registered in this session's profile (local scope `~/.claude-personal` only); not run |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Supervisor LLM-vision review of `reports/HUN-83/after-usage-1280.png`, `after-usage-375.png`, `after-kit-768.png` vs the reference image and AC 1–7: dark gradient shell, sidebar with active marker, 3 KPI cards with sparklines + ↑/↓ badges, cache progress + ring card, existing tables below as dark cards; no light leftovers. Agent viewed Usage 375/768/1280, Workspaces 1280, Agents 768, Warnings 375, Kit 1280, menu-open 375. BEFORE: `reports/HUN-83/before-usage-1280.png` |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `grep -rnE "#[0-9a-fA-F]{3,6}|rgba?\(" web/src/app.css web/src/*.jsx` (non-test) → no output: every color is a `tokens.css` token; one font token `--font`; contrast from tokens: text on surface 15.86:1, muted 8.15:1, badge text 5.65–7.73:1, white on accent 5.17:1 (all ≥ 4.5:1) |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 / 768 / 1280 for all 5 views (15 captures, `reports/HUN-83/after-*-{375,768,1280}.png`): `scrollWidth == clientWidth` at all 15; 375 one column + sidebar behind menu button; 768 two columns; 1280 sidebar visible, KPIs 3 across |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-10-29 04:56 UTC, worktree `HUN-83` at HEAD before any implementation commit):

```
$ date -u; npm --prefix web test
Thu Oct 29 04:56:09 AM UTC 2026
 RUN  v5.0.5 .../easy-personal-mcp-HUN-83/web
 Test Files  7 passed (7)
      Tests  92 passed (92)
   Duration  4.23s
```

Screenshot of the current light, tab-based Usage view at 1280px on synthetic fixture data (temp `EASY_PERSONAL_HOME` + `EASY_PERSONAL_CLAUDE_PROJECTS`, 2 projects x 16 days): `reports/HUN-83/before-usage-1280.png` (gitignored). It shows a light page, pill tabs at the top, no sidebar, no widget cards, no sparklines.

**AFTER** (2026-10-29, implementing agent; Supervisor/reviewer re-runs for Evidence):

```
$ npm --prefix web test
 Test Files  8 passed (8)
      Tests  122 passed (122)
$ npm --prefix web run build && uv run pytest -q
 built in 136ms
 748 passed, 4 deselected, 1 xfailed, 1 warning in 154.66s (0:02:34)
```

Screenshots on synthetic fixture data, `reports/HUN-83/after-<view>-<375|768|1280>.png` for usage, warningsplan, kit, workspaces, agents (+ `menuopen-usage-375.png`). No horizontal overflow (scrollWidth == clientWidth) at any of the 15 captures. 0 off-origin requests.

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
