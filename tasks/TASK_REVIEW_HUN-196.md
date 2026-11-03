# TASK_REVIEW — HUN-196: Cache reuse card follows the app-wide period, with written-to-cache and lowest-model lines (FR-064)

> Sibling of `tasks/TASK_GUIDE_HUN-196.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/Rail.test.jsx` (new describe 'cache reuse follows the app-wide period (HUN-196, FR-064)': 11 tests, AC1, AC3, AC4 x3, AC5, edges: unknown group, zero-prompt group, failing breakdown, failing summary, scope change refetches breakdown); `web/src/App.test.jsx` (period survives a view switch, AC2); `web/src/UsageView.test.jsx` (harness `WithPeriod`, AC6: the existing period tests pass unchanged). Red first: 12 failed before the change, then `Tests  487 passed (487)`. |
| Verification command run | ☑ pass | `cd web && npx vitest run` → `Test Files  18 passed (18)`, `Tests  487 passed (487)`; `npm --prefix web run build` → `✓ built in 113ms`; `uv run pytest tests/test_web_toolchain.py -q` → `16 passed in 1.47s` (2026-11-03). |
| Negative cases hold | ☑ pass | 19-message group ignored; fewer than 2 qualifying groups and `(unknown)` group give no Lowest line; zero prompt tokens give ring "no data", no extra lines, no NaN/Infinity; failing breakdown keeps ring and written line; failing summary gives "no data". |
| verify | ☑ pass | Stage 5 2026-11-03 05:27 UTC @ 685aeda (Supervisor, worktree): `cd web && npx vitest run` → `Test Files 18 passed (18)`, `Tests 487 passed (487)`; `uv run pytest tests/test_web_toolchain.py -q` → `16 passed` (static build fresh). Viewed `scratchpad/hun196/shots/after-usage-30d.png`: card = ring 97.4 %, "Share of prompt tokens read from cache, last 30 days.", "Written to cache 2.6%", "Lowest: claude-haiku-5-5 82.5%", period button "Last 30 days" pressed — matches the owner mockup (period named in the note line, not a separate caption; accepted). Read cost on the store copy (395,029 events, load 0.45): summary 0.30 / 0.47 / 0.42 s and by-model 0.23 / 0.34 / 0.40 s for 7 d / 30 d / all — each < 2 s (NFR-013); the cold-start "Loading" is queueing behind the Usage view's first requests — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed `web/src/App.jsx` (period state), `web/src/UsageView.jsx` (props, `PERIODS` export), `web/src/Rail.jsx` (`CacheReuse`, `lowestModel`), one `app.css` line, tests; callers: every view gets `period` via App, Rail is the only other consumer. No backend change. Code review: 0 P0, 0 P1, 0 P2, 1 P3 (rail card makes one extra by-model request per period change, cheap at < 0.5 s; no action). Security: text nodes only, model names come from the existing API — 0 findings (Low risk). Startup reads complete. |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q -x` → `1244 passed, 6 deselected, 1 xfailed, 1 warning in 146.73s`; vitest 487 passed. |
| **Companion score (easy-verifier)** | ☑ N/A | Not run by the implementer. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | 1280x720 on the v8 store copy, read by vision: card matches the mockup (ring, period note, "Written to cache", "Lowest"). Numbers (card text read from the DOM): 7 days `97.4%`, written 2.6%, Lowest claude-haiku-5-5 82.3%; 30 days `97.4%`, 2.6%, 82.5%; All time `97.5%`, 2.5%, 82.5%. Board view after picking 30 days still says "last 30 days". Screenshots: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/54000ba3-d7c0-4535-8376-908a80d33c6d/scratchpad/hun196/shots/after-usage-7d.png`, `after-usage-30d.png`, `after-usage-all.png`, `after-board-30d.png`; before: `before-usage-30d.png` (card says "last 7 days" on the 30-day picker). |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Reuses `.rail-card`, `.muted`, `.rail-note`, `Ring`; no new colour or token. One CSS declaration added: `overflow-wrap: anywhere` on `.rail-note` (long model names wrap). Accessibility: ring `aria-label` names the period (vitest `Cache reuse 91.8%, last 30 days`); the two lines are plain `<p>` text. |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280x720: page scrollWidth/clientWidth 1265/1265; a 70-character model name injected in the Lowest line wraps inside the card (card 243/243, rail 245/245, page 1265/1265), `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/54000ba3-d7c0-4535-8376-908a80d33c6d/scratchpad/hun196/shots/after-longname.png`. 375x812: page 375/375 on Usage and Board; the right rail is `display: none` below 80rem by existing CSS (`.rail`), so the card is not shown on phones (unchanged behaviour); `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/54000ba3-d7c0-4535-8376-908a80d33c6d/scratchpad/hun196/shots/after-375-usage-30d.png`. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (2026-11-03T05:13:03Z, branch HUN-196 at 5564a82, no implementation commit; dashboard run headless on a copy of the owner store with `BROWSER=true EASY_PERSONAL_HOME=<scratchpad>/hun196/home`, driven over Chrome DevTools Protocol with `<scratchpad>/hun196/tools/shoot.mjs`, 1280x720):

```
[usage-7d] pressed="Last 7 days"  card: Cache reuse | 97.4% | Share of prompt tokens read from cache, last 7 days. || ring: Cache reuse 97.4%, last 7 days
click Last 30 days ok
[usage-30d] pressed="Last 30 days" card: Cache reuse | 97.4% | Share of prompt tokens read from cache, last 7 days. || ring: Cache reuse 97.4%, last 7 days
click All time ok
[usage-all] pressed="All time"     card: Cache reuse | 97.4% | Share of prompt tokens read from cache, last 7 days. || ring: Cache reuse 97.4%, last 7 days
click Board ok
[board-all] pressed=""             card: Cache reuse | 97.4% | Share of prompt tokens read from cache, last 7 days. || ring: Cache reuse 97.4%, last 7 days
```

The card says "last 7 days" while the Usage picker is on "Last 30 days" and "All time": it ignores the picker, and it has no "Written to cache" or "Lowest" line. Screenshots: `<scratchpad>/hun196/shots/before-*.png`.

**AFTER** (2026-11-03T05:17Z, branch HUN-196 at 37f1cd9, same store copy, same script, 1280x720):

```
[usage-7d]  pressed="Last 7 days"  card: Cache reuse | 97.4% | Share of prompt tokens read from cache, last 7 days. | Written to cache 2.6% | Lowest: claude-haiku-5-5 82.3%
[usage-30d] pressed="Last 30 days" card: Cache reuse | 97.4% | Share of prompt tokens read from cache, last 30 days. | Written to cache 2.6% | Lowest: claude-haiku-5-5 82.5%
[usage-all] pressed="All time"     card: Cache reuse | 97.5% | Share of prompt tokens read from cache, all time. | Written to cache 2.5% | Lowest: claude-haiku-5-5 82.5%
[board-30d] (Board view)           card: Cache reuse | 97.4% | Share of prompt tokens read from cache, last 30 days. | Written to cache 2.6% | Lowest: claude-haiku-5-5 82.5%
```

**DELTA**: The owner can pick 7 days, 30 days or All time once and see the rail card follow it on every view, with the share written to cache and the weakest model next to the steady ratio.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
