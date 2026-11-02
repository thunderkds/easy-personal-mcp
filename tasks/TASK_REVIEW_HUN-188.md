# TASK_REVIEW — HUN-188: [Short Title]

> Sibling of `tasks/TASK_GUIDE_HUN-188.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | web/src/format.test.js (compact, cost figure, abstained line); web/src/UsageView.test.jsx (describe 'one-line figures, cost note, model names'); web/src/MemoryView.test.jsx (describe 'default project and gone line'); web/src/KitView.test.jsx (describe 'recent projects first, the rest folded'); golden SVG test in MemoryView.test.jsx unchanged and passing — pass |
| Verification command run | ☑ pass | `cd web && npx vitest run`: Test Files 18 passed (18), Tests 395 passed (395); `uv run pytest tests/test_web_toolchain.py tests/test_site_content.py -q`: 49 passed in 3.84s — pass |
| Negative cases hold | ☑ pass | Figures < 10,000 stay exact; 999,600 → "1 M" (no "1000 K"); usd null → "no total"; no abstained → no note line; all Kit projects idle → 10 newest shown; `gone` missing/0 → no line; Stage 4 P1 regression: a Claude-folder-only key with 40 notes vs a real project with 1 note → real project is the default (failed on f8976e6, passes on 4c12a30) — pass |
| verify | ☑ pass | Stage 5 2026-11-02 @ 4c12a30 (Supervisor): `npm --prefix web run build`, `uv run easy-personal dashboard` from the worktree on the owner's store, driver `scratchpad/v188/v.mjs`. 1440x900 and 390x844: Usage figures `3.13 B` / `21.5 K` / `$2153.10` / `97.3%` each 1 line, full value in `title` + sr-only text; abstained line under the cost figure; By model names `claude-opus-5-5`, `claude-sonnet-5-5`, `claude-opus-5`, `claude-haiku-5-5`, `(unknown)` all distinct, 0 clipped; Memory opens on `personal-agentic-claude`; Kit 24 cards + "Show 141 more" (`aria-expanded` false → Enter → 165 cards, true); no page-wide horizontal scroll on any view; 0 console errors (besides the known favicon 404) — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed `web/src/format.js`, `Widgets.jsx`, `UsageView.jsx`, `Breakdown.jsx`, `MemoryView.jsx`, `KitView.jsx`, `app.css` (new block) and their tests, plus `core/memgraph.py` `project_list`/`_display_name` (key shape). `lane: C2, 11 files, 316 lines, FITS; unpredicted: 4 test files` (tests only). Stage 4 code-review: 0 P0, 2 P1 — AC 4 not met (folder-only key could be the Memory default) fixed 4c12a30 with a test that failed first; process (role guide read to line 30) fixed, read in full — 0 P2/P3 — pass |
| Full smoke suite still green (no regression) | ☑ pass | @ 4c12a30: `cd web && npx vitest run` → Test Files 18 passed (18), Tests 397 passed (397); memory golden SVG unchanged; no Python code changed — pass |
| **Companion score (easy-verifier)** | ☑ N/A | Not run: learnings 2026-11-01 — easy-verifier refused every repo path |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | screenshots under /tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun188/after/ viewed: figures on one line, distinct model names, Memory on a real project, Kit short; golden SVG file unchanged (git status clean for web/src/test) — pass |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | UsageView.test.jsx CSS test: new `/* Usage, Memory, Kit UX */` block has no hex/rgb/hsl colours (only var(--…)); Plex Mono kept for .kpi-value; sr-only text for full value, Kit fold button has aria-expanded — pass |
| **UI: Responsiveness at target viewports** | ☑ pass | 1440x900 and 390x844: documentElement.scrollWidth <= innerWidth on Usage, Memory, Kit; KPI figures 1 line each; no clipped model name — pass |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-02T08:21:55Z from main code de96ca4 (worktree HUN-188, no edits), real run `BROWSER=true setsid uv run easy-personal dashboard` on the owner's data, probe `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun188/probe.mjs` (headless Chrome), raw output `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun188/before/probe.json`, screenshots `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun188/before/{wide,narrow}-{Usage,Memory,Kit}.png`:
- Usage 1440x900: "Total tokens" figure `3,111,681,853` renders on 2 lines (wraps); cost card figure text `$2143.03 + abstained: (unknown), claude-opus-5` on 3 lines. At 390 px the cost figure is 2 lines.
- Usage By model, visible names (cell clientWidth 96 px, scrollWidth larger = clipped with an ellipsis): claude-opus-5-5 (121 px), claude-sonnet-5-5 (136), claude-opus-5 (106), claude-haiku-5-5 (128), (unknown). All four long names are cut off by the ellipsis (scrollWidth > clientWidth), so claude-opus-5-5 and claude-opus-5 lose their distinguishing end; the full-page screenshot was too tall to read, so the clipped text itself was not eyeballed.
- Memory default project: `-home-hungnguyenhuu-workspace-pets-per-agentic-claude` (48 options), graph "2 notes · 1 link · 1 unresolved"; no "gone folders hidden" text.
- Kit: heading "Kit by project (162)", 162 cards, page height 29779 px at 1440 (30598 px at 390); no "gone folders hidden" text.
- No page-wide horizontal scroll in any of the six captures.

**AFTER**: 2026-11-02T08:30:24Z, same probe on the changed build (uv run easy-personal dashboard from the HUN-188 worktree), raw `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun188/after/probe.json`, screenshots `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun188/after/{wide,narrow}-{usage-kpis,usage-bymodel,memory-view,kit-top,kit-fold}.png` (full pages: `{wide,narrow}-{Usage,Memory,Kit}.png`):
- Usage: Total tokens `3.13 B` (full `3,125,382,288` as title and sr-only text), 1 line at 1440 and 390; Messages `21.4 K`; cost figure `$2148.43` alone on 1 line, muted line under it `abstained: (unknown), claude-opus-5` (kpis-crop screenshots).
- By model: names wrap at hyphens, none clipped (scrollWidth == clientWidth for all 5 rows); opus-5-5 and opus-5 read `claude-opus-5-5` / `claude-opus-5`; Total/Output compact in that table (`2.39 B`, `10.9 M`); cost column visible with a muted abstained line.
- Memory default: `personal-agentic-claude` (21 notes, 10 links), no longer the Claude folder key.
- Kit: heading "Kit by project (165)", 24 cards shown, button "Show 141 more" (aria-expanded false), page height 6600 px at 1440 (7303 at 390), was 29779 / 30598.
- No page-wide horizontal scroll in any capture. "N gone folders hidden" not visible in the real run because the backend `gone` field (HUN-186) is not in this worktree; covered by vitest fixtures.

**DELTA**: The owner can read the Usage figures on one line, tell the opus models apart, open Memory on a real project and see a 24-card Kit page instead of a 165-card one.

**WITNESS**: Supervisor, 2026-11-02, Stage 5 verify run @ 4c12a30 (`scratchpad/v188/v.mjs`, screenshots `scratchpad/v188/{wide,narrow}-usage.png`), independent of the implementing agent.
