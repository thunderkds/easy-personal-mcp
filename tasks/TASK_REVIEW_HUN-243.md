# TASK_REVIEW — HUN-243: report folder lines open a file list and files open in a tab (FR-090)

> Sibling of `tasks/TASK_GUIDE_HUN-243.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/WorkspaceKit.test.jsx` (describe 'report files (HUN-243)', 17 tests: AC1-AC7 + edges incl. null mtime dash, markup/encoding, popup blocked, failed fetch, unmount), `web/src/KitView.test.jsx` (period change refetches no report files); fixtures in `web/src/test/fixtures.js`. Web: Test Files 24 passed, Tests 786 passed. |
| Verification command run | ☑ pass | `cd web && npx vitest run` -> `Tests 786 passed (786)`; `npm --prefix web run build` -> `built in 189ms`; `uv run pytest -q` -> `1758 passed, 6 deselected, 1 xfailed, 1 warning in 227.42s` |
| Negative cases hold | ☑ pass | popup blocked -> message and no fetch; file fetch 404 -> tab closed + 'cannot open this file'; HTTP 500 -> 'files unavailable'; null mtime -> dash; markup name -> text only; unmount mid-load -> no console error |
| verify | ☑ pass | **pass** — `/verify HUN-243` run by the Supervisor on 2026-11-08, independently of the implementer, on an own fixture (temp `EASY_PERSONAL_HOME`, `uv tool install` from this worktree, workspace `acme` registered with `workspace add acme --repo`, a repo holding `reports/` with 10 files and `smoke-reports/`). Real headless Chrome (`/usr/bin/google-chrome`) driven by puppeteer-core 24.9.0 against the live dashboard. All 9 Acceptance Criteria re-checked end to end: 32 of 32 assertions pass. Suites: vitest `Test Files 24 passed, Tests 786 passed (786)`; `npm --prefix web run build` -> `built in 143ms`; `uv run pytest -q` -> `1758 passed, 6 deselected, 1 xfailed in 221.39s`. AC 8 is now proven **with its mechanism**, not only by its effect: Chrome's security log says `Executing inline script violates the following Content Security Policy directive 'default-src 'self''. ... The action has been blocked.` An independent in-tab probe that injected a fresh inline script into the blob document returned `BLOCKED`, and a relative `fetch('/api/workspaces')` inside the blob document failed with `Failed to parse URL from /api/workspaces` (a blob document has no base URL), so the API is unreachable from there by a second, independent barrier. No request URL carried the token; the only match was the dashboard page's own fragment, which is never sent to the server. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed: the 6 source and test files of `git diff main...HEAD` (`web/src/WorkspaceKit.jsx`, `web/src/api.js`, `web/src/app.css`, `web/src/WorkspaceKit.test.jsx`, `web/src/KitView.test.jsx`, `web/src/test/fixtures.js`) plus 7 evidence PNGs, and the direct context of the changed code: `web/src/useLoad.js` (read, not changed — confirmed its `current` flag drops a late resolve after unmount), `web/src/api.js` `readToken` (confirmed the token is read from the URL fragment, which is why `win.opener = null` matters), and the server side the page calls, `src/easy_personal/surfaces/api.py` `Guard.send_csp` and the `/api/kit/report-file` route (read, not changed). Skipped: the Python backend (HUN-242, already merged and separately reviewed) and the rest of `web/src`, because nothing outside the diff imports the new components and the only new export, `getBytes`, has one caller. `scripts/lane_check.py`: `lane: C2, 13 files, 417 lines, FITS` — the 7 `unpredicted` entries are the evidence screenshots the guide itself asks for under `tasks/evidence-HUN-243/`. |
| Full smoke suite still green (no regression) | ☑ pass | same pytest and vitest runs as above, all green |
| **Companion score (easy-verifier)** | ☑ N/A | The easy-verifier MCP server is not loaded in this session (no `score` tool available; `127.0.0.1:8766` not reachable), so no scoring session ran. Per the row's own rule the merge gate blocks on ☑ fail only, never on an N/A row. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | `tasks/evidence-HUN-243/kit-closed-desktop.png`, `kit-open-desktop.png`, `kit-closed-phone.png`, `kit-open-phone.png`, plus opened tabs `tab-score_html.png`, `tab-evil_html.png`, `tab-sub_notes_md.png`. Verdict: folder line is a muted mono text row with a small chevron; open list is indented under it, files in link colour, size/date muted; rows and project list unchanged. No BEFORE screenshot (BEFORE is the terminal capture in Demonstration; plain text rows). |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | vitest 'styles use tokens only' (no #hex/rgb in `.ws-report/.ws-file/.ws-files/.ws-chevron` lines); colours are `var(--color-*)`, spacing `var(--space-*)`; `:focus-visible` ring from tokens (`outline: 2px solid var(--color-accent)`) applies to the native buttons; `aria-expanded`, native buttons, `role="status"` asserted in vitest |
| **UI: Responsiveness at target viewports** | ☑ pass | real Chrome: 1280x720 `scrollWidth 1265 <= innerWidth 1280` (vertical scrollbar), 375x812 `scrollWidth 375 == innerWidth 375`; long folder text ellipsised with full text in `title` |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-08T00:17:46Z on base a979835 (no implementation commit yet):
```
2026-11-08T00:17:46Z
a979835
57:/** Report folders (FR-089) of one workspace, as plain reference text: nothing to click. Shows nothing while the read is pending. */
 RUN  v5.0.5 /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/HUN-243/web


 Test Files  1 passed (1)
      Tests  1 passed | 35 skipped (36)
   Start at  07:17:47
   Duration  1.58s (environment 57%, tests 17%, transform 14%, setup 8%, import 4%)
```
The existing test `AC3: plain text only: no link, no button, no tab stop` passes: report folder lines are plain `<li>` text, no button.

**AFTER**: real run 2026-11-08, headless Chrome 157, live dashboard on a temp EASY_PERSONAL_HOME, real mouse clicks. Folder button `app/reports` opens to 5 file rows (`blob.bin 100 B · 2026-11-08`, `evil.html 480 B ...`, `sub/notes.md`, `score.html`, ...). Click on `score.html` opens a `blob:http://127.0.0.1:<port>/...` tab titled `Score report`, `window.opener` = null.
AC8 (security, real run): click on `evil.html` (script sets `document.title="PWNED"`, fetches `/api/workspaces`, writes `window.opener`/`localStorage` into body attributes). New tab: title `evil` (not `PWNED`), body attributes `{}` (the script did not run), `performance` resource entries `[]` (no `/api/*` request), `String(window.opener)` = `null`. The blob tab inherits the dashboard CSP: inline script blocked.
Opened HTML report look (PRD question 32): the inline `<style>` did NOT apply. `score.html` (red h1, yellow background, blue pill in its CSS) shows as unstyled browser-default text: h1 `rgb(0, 0, 0)`, body background transparent; see `tab-score_html.png`. Content is readable but plain. The `.md` opens as plain text.

**DELTA**: The owner can click a report folder in the Kit view to list its files and click a file to read it in a new tab (before: the folder line was plain text).

**WITNESS**: the Supervisor, in session `04d37093-a136-4282-93b0-cee973d96aaf`, on 2026-11-08 at
10:40–10:55Z, running the user-invoked `/verify HUN-243`. This is a second, independent witness: the
AFTER above was captured by the implementing agent, and the VERIFY block below was captured by the
Supervisor on a separately built fixture, so no claim here rests on the implementing agent alone.
There is no `memory/event-trace/HUN-243.jsonl`: the `active_task` anchor was cleared when HUN-243's
agent work finished at Stage 4, so the Supervisor's own `Bash` calls in this verify run were not
attributed to the task. The evidence is therefore the pasted output and the screenshots named below,
not a trace file.

**VERIFY (Supervisor, independent run, 2026-11-08)**: fixture `reports/` held 10 files chosen to
attack the edges: `stale.md` with `st_mtime` 1e17 (unrepresentable), `archive.bin` and `diagram.svg`
(outside `REPORT_TYPES`), `we&ird #1 ?x <img onerror=alert(1)>.md`, a 110-character name, `sub/deep.md`
at depth 2, `evil.html`, `score.html`, `notes.md`, `data.json`.

- `GET /api/kit/reports` -> `HTTP 200`, `"files": 10`, `"newest_utc": "2026-11-08T03:45:55Z"` — the
  out-of-range mtime is counted but never becomes the newest, and the route does not 500 (the merged
  `DRAFT-mtime-500` fix, re-confirmed here on a live server).
- `GET /api/kit/report-files?workspace=acme&repo_no=0&folder=reports` -> `HTTP 200`, 10 rows,
  `stale.md` with `"mtime_utc": null`, `archive.bin` and `diagram.svg` with `"openable": false`.
- Page: 2 folder lines, both `aria-expanded="false"`. One click -> exactly 1 `report-files` request,
  `aria-expanded="true"`. A second click closes it; a third reopens it with **no** new request (1 -> 1).
- Rows: `stale.md` renders `46 B · –` (a dash, not `Invalid Date`); no row shows `Invalid Date` or `NaN`;
  `archive.bin`/`diagram.svg` are `SPAN` with `title="this file type is not opened here"`; openable files
  are `BUTTON`; the markup name produced **0** `<img>` elements; the long name carries its full text in `title`.
- Open: `score.html` -> `blob:http://127.0.0.1:34265/...`, `h1` = `Score report`, `window.opener === null`.
- AC 8: `evil.html` -> title stays `evil` (not `PWNED`), the in-page marker still reads
  `script did not run`, `window.opener === null`, and the tab made **0** `/api/*` requests. Mechanism
  captured, see the verify Evidence row.
- AC 5 (real browser, not only jsdom): the file was deleted between listing and click, so the fetch
  404'd -> the blank tab was closed (`tab.isClosed() === true`) and one `role="status"` notice read
  `cannot open this file`. With `window.open` stubbed to return null -> `allow pop-ups to open the file`
  and **0** `report-file` fetches.
- 375x812: `scrollWidth 375 <= clientWidth 375`, no horizontal page scroll.
- Screenshots: `tasks/evidence-HUN-243/v-kit-closed-desktop.png`, `v-kit-open-desktop.png`,
  `v-kit-closed-phone.png`, `v-kit-open-phone.png`, `v-tab-score.png`, `v-tab-evil.png`, `v-fail-notice.png`.

**VERIFY — PRD question 32, measured cause**: the opened HTML report is **unstyled**, and the cause is
now identified. `score.html` reaches the tab with its `<style>` element intact (`document.querySelectorAll('style').length === 1`)
but `document.styleSheets.length === 0`, and Chrome's security log says:
`Applying inline style violates the following Content Security Policy directive 'default-src 'self''. ... The action has been blocked.`
So the very CSP that blocks the attacker's script also blocks the report's own styling — under this
design the two cannot be separated. This matters more than one fixture: `templates/report_template.html`
carries its CSS in an inline `<style>` block (line 6), and the `html-report` skill requires
"pure inline CSS only", so **every report this project generates opens unstyled** in this tab.
Readable, but plain. That is the trade-off PRD question 32 asks the owner to settle.
