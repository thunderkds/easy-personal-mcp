# TASK_REVIEW — HUN-126: Restyle dashboard to the Graphite Mint design

> Sibling of `tasks/TASK_GUIDE_HUN-126.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | Tests (written first; App tests were red before the code): `web/src/App.test.jsx` (SC1 header `scope: All workspaces · All accounts`; SC2 collapse `// setup` -> `aria-expanded=false`; AC3 brand, selects `workspace`/`account`, `// family` links, menu button, "Open the plan →"), `web/src/UsageView.test.jsx` (SC3 `0.79` -> `79.0%` + `aria-valuenow="79"`; 4-cell KPI strip; By day latest bar; By model/By project/Outlier layout; cost footnote), `web/src/Rail.test.jsx` (SC4 78% -> `limit-amber`, 92% -> `limit-red`, 42.5% -> `limit-green`; "Bars turn amber at 70%."), `web/src/FeedCard.test.jsx` (amber tone), `web/src/WarningsPlanView.test.jsx` (amber tile on Fix now, no raw rule id), plus `tests/test_web_toolchain.py::test_family_links_are_allowed_but_other_hosts_still_fail`. Result: `Test Files 10 passed (10) / Tests 245 passed (245)`; toolchain test `16 passed`. |
| Verification command run | ☑ pass | `npm --prefix web test && npm --prefix web run build && uv run pytest -q` in the worktree: vitest `Tests 245 passed (245)`; build `✓ built in 212ms` (5 woff2 + css + js in `static/assets`); pytest `815 passed, 5 deselected, 1 xfailed, 1 warning in 146.21s`. After the last two CSS-only fixes (`ol.actions` alignment, notice wrap) I re-ran vitest (245 passed), rebuilt, and re-ran `tests/test_web_toolchain.py tests/surfaces` (`154 passed`) and `python3 scripts/check_static.py` (`check_static: ok`). Success 5 grep `grep -nE '#[0-9a-fA-F]{3,8}\b|rgba?\(' web/src/*.css` (all but tokens.css) and non-test `web/src/*.jsx`: no match. |
| Negative cases hold | ☑ pass | Empty/no-data states keep their text (`no data`, never 0%; null ratio test; abstained cost never `$0`; `no previous data`); a stored scope that no longer exists falls back to `All workspaces` / `All accounts`; any remote URL other than the three family origins still fails `check_static.py` (new negative test); no `fonts.googleapis.com` / `fonts.gstatic.com` request (see UI rows); no horizontal scroll at 375 px on all 5 views (Playwright `scrollWidth 375 == clientWidth 375`). |
| verify | ☑ pass | Stage 5 verify by the Supervisor 2026-10-29 on ce14bd2: `npm --prefix web test` 245 passed, build ok, `uv run pytest -q` 815 passed 1 xfailed, `check_static: ok`; fresh `uv tool install --reinstall .` in a temp dir: `0.2.0`, `easy-personal core ok`, 5 bundled ibm-plex woff2 in the installed static, no fonts.googleapis/gstatic reference, --host 0.0.0.0 and ::1 refused (exit 2); runtime UI checked by the implementing agent with easy-ui (session-575cff17-6589-4a51-b336-6e47cf2f14e6.html) and host Playwright: 5 views at 1440, Usage at 1024/375, no horizontal scroll, Plex fonts from the bundle, 0 font-CDN requests; Supervisor viewed the 1440 Usage/Warnings/Kit shots against the artboards — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | code-review 2026-10-29 on `git diff main...HEAD` (27 files: 9 JSX components, their tests, tokens.css, app.css, fonts, scripts/check_static.py + tests/test_web_toolchain.py) plus direct callers (App.jsx → every view, Rail, Scope); lane_check: `lane: C2, 26 files, 1548 lines, EXCEEDS (files 26 > 14; lines 1548 > 500)` — expected for a full restyle, label kept. Findings: P0 0, P1 0, P2 2, P3 2. P2-1 scripts/check_static.py edited although `scripts/` is in Files Must NOT Touch — Supervisor accepted (three anchor-only family origins, negative test keeps other hosts failing). P2-2 By model drops input/cache/message columns and Outliers moves from a table to rows (deviation 3) — matches the owner's design; day/project keep full tables behind Show table. P3-1 at 1024 px the 4th KPI cell wraps beside an empty slot (same as the artboard's auto-fit). P3-2 header menu button hidden at ≥ 64rem (deviation 5). Security: no API change; family links use target=_blank rel=noopener noreferrer, the token lives in the URL hash and is never sent; Risk Low → no security-review. Startup reads line present and complete. |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q`: `815 passed, 5 deselected, 1 xfailed, 1 warning in 146.21s` (static build present, so the 2 web tests ran). |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | easy-ui (frame-proxy harness, see Notes below) AFTER session report `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-ui-mcp/reports/session-575cff17-6589-4a51-b336-6e47cf2f14e6.html`; screenshots in `reports/HUN-126/`: `easyui-usage-1440.png`, `easyui-warnings-1440.png`, `easyui-kit-1440.png`, `easyui-workspaces-1440.png`, `easyui-agents-1440.png`, BEFORE `before-usage-1440.png`. My verdict against the artboards: shell (232 px sidebar, `// insights` / `// setup`, square dots, mint active item, `// family`, 56 px header with `scope:` line), Usage (note, segmented period, bordered 4-cell KPI strip, footnote, By day with latest bar full mint, By model mono table, By project bars, Outlier raised row with amber value, right rail) match `Main.dc.html`; Warnings cards (amber tile for Fix now, one list, `Not checked (N)` disclosure) and Kit cards match the card/header/count styles of `Warnings.dc.html` / `Kit.dc.html`. Differences by design: full numbers instead of the artboard's sample `4.82M`; Workspaces/Agents have no artboard (same shell and styles). Defects found by looking and fixed: By model cost column clipped; `ol.actions` cards were narrowed and centered by the `.actions` button-row class; Usage note icon wrapped above its text at 375 px. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Colors: Success 5 grep, no match outside `tokens.css`. Fonts, easy-ui `ui_check` (all `Check passed`): body font starts with `"IBM Plex Sans"`; `.kpi-value` starts with `"IBM Plex Mono"`; exactly 5 `assets/ibm-plex-*.woff2` resources loaded; every resource same-origin; `document.fonts.check` true for Plex Mono 500 and Plex Sans 400; no request to `fonts.googleapis.com` / `fonts.gstatic.com` (Success 6). Host Playwright (`reports/HUN-126/pw-results.json`): loaded fonts `IBM Plex Sans 400/500/600`, `IBM Plex Mono 400/500`; font-CDN requests: 0; no console errors. Contrast: red `#EF7B7B` 6.86:1, muted `#8C9590` 6.01:1, secondary 8.62:1 on the page (computed from the token values); `--color-faint` (4.0:1) is used only for disabled button text. |
| **UI: Responsiveness at target viewports** | ☑ pass | easy-ui: `easyui-usage-1024.png` (sidebar + main, rail hidden: check passed), `easyui-usage-375.png` (sidebar hidden until menu, rail hidden, `scrollWidth <= clientWidth`: all checks passed), `easyui-usage-375-menu-open.png` (sidebar open), 1440 px shots show 3 columns with the rail. Host Playwright exact viewports (`reports/HUN-126/pw-results.json`): `scrollWidth == clientWidth` at 1440 (5 views), 1024 (Usage) and 375 (5 views). Observation: at 1024 px the 4th KPI cell wraps to a second row beside an empty slot (3 + 1 grid), as the artboard's `auto-fit` grid does. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-10-29T11:04:06Z on branch `HUN-126` at `aba05a0`, before any implementation commit; fixture-backed dashboard, temp `EASY_PERSONAL_HOME` / `EASY_PERSONAL_AGENT_ROOT`):

- easy-ui screenshot of the current Usage view at 1440 px (HUN-121 indigo, all-monospace look; easy-ui's viewport is fixed at 1280x720, so the 1440 px page is an iframe of width 1440 scaled by 0.8889 inside a same-origin test proxy): `reports/HUN-126/before-usage-1440.png` (copy of `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-ui-mcp/reports/screenshot-1793271837904.png`); easy-ui session report: `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-ui-mcp/reports/session-a7b456b8-eb23-45d9-af96-e3a66847ccba.html`.
- `grep -c -- '--font' web/src/tokens.css` -> `1`; the line is `--font: ui-monospace, 'SF Mono', 'Cascadia Mono', 'DejaVu Sans Mono', Menlo, Consolas, 'Liberation Mono', monospace;` (one system monospace stack, no webfont).
- Accent tokens in `web/src/tokens.css`: `--accent-start: #3b6be0;` `--accent-end: #9140dc;` `--color-accent: #3b6be0;` `--color-accent-text: #a9c0ff;` `--color-accent-soft: #3a3382;` (blue-to-purple gradient accent; page background `--color-bg: #1a1540`).

**AFTER** (2026-10-29, same fixture-backed dashboard, built from `HUN-126`): `reports/HUN-126/easyui-usage-1440.png` (easy-ui; Graphite Mint: `#121413` page, Plex Sans text, Plex Mono numbers, mint accent, KPI strip, rail) with the AFTER easy-ui session report `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-ui-mcp/reports/session-575cff17-6589-4a51-b336-6e47cf2f14e6.html`. `grep -c -- '--font' web/src/tokens.css` -> `2` (`--font-sans: 'IBM Plex Sans', system-ui, sans-serif;`, `--font-mono: 'IBM Plex Mono', ui-monospace, ...`); accent tokens now `--color-accent: #7eddb0;` `--color-accent-hover: #b2edcf;` `--color-accent-soft: #7eddb017;` `--color-accent-bar: #7eddb066;`; the indigo gradient tokens are gone.

**DELTA**: the dashboard now looks like the owner's Graphite Mint design on all 5 views, with self-hosted Plex fonts, a new sidebar/header with a visible scope line, and the Usage KPI strip and charts, and no request leaves the machine for fonts.

**WITNESS**: Frontend-Implementer (Claude Opus 5.5 sub-agent) ran the vitest, build, pytest, grep, easy-ui and Playwright commands above in the worktree on 2026-10-29; see `memory/event-trace/HUN-126.jsonl` for the hook trace.

**Evidence tools**: easy-ui produced all screenshots named `easyui-*.png` and the font/network/no-scroll `ui_check` results; its browser viewport is fixed at 1280x720 and the tools have no viewport setting, so each width is an iframe of that width (scaled 0.8889 for 1440) in a same-origin test proxy (`frame-ancestors` rewritten to `'self'` in the proxy only, `Origin`/`Referer` stripped). Host Playwright (chromium from the easy-ui-mcp install) produced `pw-results.json` (exact-viewport `scrollWidth`, computed fonts, request list) and its own full-page shots kept outside the repo.

**Deviations from the guide** (accepted by the Supervisor for Stage 4 review):
1. Edited `scripts/check_static.py` and `tests/test_web_toolchain.py` (outside the guide's file list; `scripts/` was on the must-not-touch list): the three `// family` anchor origins (`personal-agentic-claude.vercel.app`, `easy-memory.vercel.app`, `easy-verifier.vercel.app`) are allowed in the built bundle; other remote URLs still fail (new negative test). The links are plain `<a target=_blank rel="noopener noreferrer">`.
2. Removed `ScopeLine` ("Showing: ...") from the views; the header `scope:` line carries it at every width. Its tests moved to App-level header assertions.
3. By model shows 4 columns (Model, Total tokens, Output, API-equivalent cost) as on the artboard; per-model input/cache/messages columns are not shown. By day and By project keep their full tables behind "Show table"; By tool keeps its full table.
4. The "Cache hit gauge" KPI cell is removed (the ring is in the right rail); the Totals section stays below the grids.
5. The header menu button shows only below 64rem (the artboard shows it at 1440, where it would do nothing).
6. Trend badges are neutral secondary text, as on the artboard; `chip-warning` / `label-warning` are amber, and red is used only for limits of 90 % and more and for errors.
