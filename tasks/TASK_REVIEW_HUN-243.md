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
| verify | ☐ N/A | Supervisor's stage; real run done by implementer (see AC8 below): headless Chrome 157 against a live dashboard on a temp EASY_PERSONAL_HOME, real mouse clicks via CDP |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | same pytest and vitest runs as above, all green |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
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

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
