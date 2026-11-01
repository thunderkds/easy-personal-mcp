# TASK_REVIEW — HUN-178: Live view: active sessions, per-session tool feed, running token and cost totals (FR-045)

> Sibling of `tasks/TASK_GUIDE_HUN-178.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/LiveView.test.jsx` (30 tests: AC1 feed order/cap, AC2 totals + API-equivalent label, AC3 reconnect state, last_id resume, server restart, design/a11y), `web/src/App.test.jsx` (views list), `tests/test_site_content.py` (AC4 views stat 9). `cd web && npx vitest run LiveView` -> `Tests  30 passed (30)` |
| Verification command run | ☑ pass | `cd web && npx vitest run LiveView` -> `Test Files  1 passed (1) / Tests  30 passed (30)`; `uv run pytest tests/test_site_content.py -q` -> `33 passed in 0.60s` (2026-11-01) |
| Negative cases hold | ☑ pass | Tests: malformed message ignored; duplicate/old id (`id <= last_id`) not counted twice; unknown price shown as `abstained: unknown price`, never $0; extra server fields (message text) never rendered; server restart (lower `last_id`) clears stale feed rows. Real run: planted `INVENTED_SECRET_PATH` in the transcript tool input appears 0 times in the page text, the SSE stream, `dash.out` or `dash.err`; the token appears 0 times in logs after the start line. |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | `cd web && npx vitest run` -> `Tests  356 passed (356)`; `uv run pytest -q` -> `1048 passed, 6 deselected, 1 xfailed, 1 warning in 129.44s` (static bundle rebuilt first) |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Real dashboard (installed CLI, temp `EASY_PERSONAL_HOME`) + real tail, headless Chrome (Playwright over `/usr/bin/google-chrome`) at 1280x720: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/ffe95e7d-1a2a-4db0-bd19-6595640fcdeb/scratchpad/hun178/after-live-1280.png`. Verdict (viewed): same Graphite Mint card style as Usage/Memory; stream status line, All sessions totals card with the API-equivalent footnote, account heading, project heading, session cards with id, active/idle, totals and feed (newest first). BEFORE (main b289dbe, no Live): `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/ffe95e7d-1a2a-4db0-bd19-6595640fcdeb/scratchpad/hun178/before-work-1280.png`. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | vitest `design` block in `web/src/LiveView.test.jsx`: every `.live-*` rule in `app.css` uses only `var(--…)` colors; text colors >= 4.5:1 on card and page backgrounds; Reconnect button shows the shared focus ring; main landmark + heading per section; feed has `aria-live="polite"`. Computed style in the real page: body `IBM Plex Sans` 14px, `.live-tool`/`.live-time`/`.live-id`/dt labels `IBM Plex Mono` (12.5/12.5/13/11.5px); `.live-feed` `aria-live` = `polite`. |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280x720: `scrollWidth 1280 = clientWidth 1280`; narrow 375x800 (rail collapsed behind the menu button): `scrollWidth 375 = clientWidth 375`, no horizontal page scroll, long project name truncated with an ellipsis (`/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/ffe95e7d-1a2a-4db0-bd19-6595640fcdeb/scratchpad/hun178/after-live-narrow.png`); `.live-sessions` (`overflow-y: auto`, max-height 70vh) scrolled inside its box (scrollHeight 706 > clientHeight 560) and `.live-feed` is its own scroll box. Vitest: 60 sessions scroll inside their own box; long project name has a `title` and a one-line style. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-01T02:41:02Z from main b289dbe (pre-HUN-178 code) after the branch's implementation commits existed (no BEFORE was taken before them; the earlier BEFORE text in this file is replaced). Main was exported with `git archive b289dbe`, built with `npm --prefix web run build`, installed with `uv tool install` into a temp dir, and run as `easy-personal dashboard` on fixtures; headless Chrome read the sidebar:

```
$ grep -c "live" web/src/App.jsx        # in the b289dbe export
0
$ node nav.mjs <dashboard url>          # sidebar buttons, 1280x720
["// work","Board","Scheduled","Memory","// usage","Usage","Warnings & plan","// kit","Kit","// connect","Workspaces","Agents","// system","First run2 left"]
```

Screenshots of the `// work` area without a Live view: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/ffe95e7d-1a2a-4db0-bd19-6595640fcdeb/scratchpad/hun178/before-work-1280.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/ffe95e7d-1a2a-4db0-bd19-6595640fcdeb/scratchpad/hun178/before-work-narrow.png`. The `site/index.html` views stat on main is 8.

**AFTER**: same flow on the HUN-178 branch (installed CLI from the worktree, built bundle, real dashboard with the live tail on; three transcript sessions in 2 projects, 5 tool-call lines appended while the page was open, one with an unpriced model). Page text after the appends:

```
Live / Live stream connected
All sessions: Tool calls 5, Tokens in 5,000, Tokens out 1,250, API-equivalent cost $0.02 + abstained: unknown price
Claude default
  a-very-long-project-folder-name-... : c44d81a0 active · just now  (1 call, $0.01)  feed: Edit
  alpha : b7d20e55 (2 calls, abstained: unknown price) feed: Bash, Grep ; a1f3c9e2 (2 calls, $0.01) feed: Glob, WebFetch
feed rows before/after append (live, no reload): 0 -> 5
```

Sidebar now lists `Live` first under `// work`. Screenshots: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/ffe95e7d-1a2a-4db0-bd19-6595640fcdeb/scratchpad/hun178/after-live-1280.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/ffe95e7d-1a2a-4db0-bd19-6595640fcdeb/scratchpad/hun178/after-live-narrow.png`. Raw stream (curl) opened `event: snapshot` with `last_id` 20 and the three sessions; page and logs contain 0 hits of the planted `INVENTED_SECRET_PATH`.

**DELTA**: The owner can open `// work` > Live and watch each active Claude Code session's tool calls arrive newest-first, with running token and API-equivalent cost totals per session and overall, without reloading and without any message or tool content.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
