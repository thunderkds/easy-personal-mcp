# TASK_REVIEW — HUN-207: Usage opens Scratch; Workspaces lists repos in no workspace

> Sibling of `tasks/TASK_GUIDE_HUN-207.md`. Everything here is **filled by the reviewer at Stage
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
1. Started HUN-207 now: its precondition (HUN-206, the Scratch group and `GET /api/workspaces/unassigned`) is merged — FACT: memory/MEMORY.md:123 "merged 2026-11-04 (FR-069, FR-070 read side)"
2. Left the code-review P2 (the "Repos not in a workspace" list is not re-read after another workspace edit on the page) unfixed, reported as a suggestion — RULE: `.claude/skills/code-review/SKILL.md` § Phase 4 ("Do not apply P2/P3 fixes automatically")
3. Did not run `/verify` (only the user can) — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule`
4. Did not merge, push or move the task to Done — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule`
Checks: tests `cd web && npx vitest run && cd .. && npm --prefix web run build && uv run pytest -q` — vitest 720/720, build ok, pytest 1627 passed (agent run 2026-11-06T23:49:57Z); reviews code-review — 0 P0, 0 P1, 1 P2, 2 P3; security-review — no finding at confidence 8 or above; easy-verifier changes score — overall 73, change_risk 100 (advisory); real run — dashboard from this branch on a COPY of the owner store, headless Chrome at 1280x720 and 375x812, Scratch opened (77 folders) and one repo added to a workspace (auto-checked, not /verify)
Not done: merge (yours); /verify (yours); the P2 refresh of the unassigned list after other workspace edits
Paused: auto-mode check failed — Risk Medium, but the trace has no `security-review` Skill call

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/Breakdown.test.jsx` (AC1-3: 9 tests), `web/src/UnassignedRepos.test.jsx` (AC4-7: 15 tests); 24 new tests, red before the code (22 failed / 2 passed), green after. `web/src/test/fixtures.js` +1 line (fake route `/api/workspaces/unassigned`, default `{repos: []}`). |
| Verification command run | ☑ pass | 2026-11-06T23:49:57Z `cd web && npx vitest run` -> `Test Files 23 passed (23) / Tests 720 passed (720)`; `npm --prefix web run build` -> `index-XoOnF7g7.js 331.36 kB ... built in 115ms`; `uv run pytest -q` -> `1627 passed, 6 deselected, 1 xfailed, 1 warning in 163.45s`. |
| Negative cases hold | ☑ pass | Tests: no Scratch group = no button, one row per group; Scratch without `folders` stays plain; opening sends no fetch; same-name folders both listed; failed unassigned read = inline alert only (1 alert on page); PATCH error = error toast, row stays, Add re-enabled, repo count unchanged; double click = 1 PATCH; fresh read keeps a repo added in another tab; no duplicate path; only orphan = hint, no select/button; empty list text; planted secret not on page. |
| verify | ☑ pass | /verify run 2026-11-07 on commit c940171 (branch `worktree-agent-afe9f263323fa335d`; no `HUN-207` branch exists), isolated temp home. vitest 720/720, build ok, pytest 1627 passed, `easy-personal --version` 0.4.0, `ping` ok. Real dashboard on a synthetic store (4 git repos + 6 gone folders), headless Chrome 1280x720 and 375x812: Scratch row "Scratch (6 folders)" closed, `aria-expanded` false→true→false, 0 requests on open, 6 folder rows listed, no planted secret on the page. Workspaces card "Repos not in a workspace (3)": select lists `main`, `side` only (orphan excluded); double-click on Add → toast "Added gamma-cli to main", row gone, count 1→2 repos, store has the repo once. `scrollWidth` = `clientWidth` on all three screens at both sizes (1265/1265, 375/375). Not exercised: failed-PATCH path and the "Add a workspace first" hint (covered by vitest only). The earlier hook-run "blocked" row was a log-format miss, not a failure. Result: pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed the 8 changed web source/test files plus `useWorkspaceData`/`sendJson` callers (`WorkspacesView.jsx`, `api.js`); skipped the HUN-237 files that appear in the diff against the stale `origin/HEAD` (already reviewed with HUN-237) and the 8 PNG evidence files. Lane check: `EXCEEDS (files 17 > 14; lines 546 > 500)` — from the PNGs and review file; code is 7 web files. Findings: P2 `UnassignedRepos.jsx:15` list not re-read after other workspace edits; P3 `web/src/test/fixtures.js` touched outside the guide's file list (needed by an existing test). |
| Full smoke suite still green (no regression) | ☑ pass | Same run as the verification command: vitest 720/720, pytest 1627 passed (no Python file touched). |
| **Companion score (easy-verifier)** | ☑ pass | overall 73 (advisory, no gate), change_risk 100, scope changes, ref `main...HEAD`; the tool asked for change_kind/test_pairs/picks (needs_input) and was used as returned, report: /home/hungnguyenhuu/.easy-verifier-reports/agent-afe9f263323fa335d/evidence-report-changes-20261106T235603-300716Z.html |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Headless Chrome (CDP), dashboard from this branch on a COPY of the owner store. BEFORE: `tasks/evidence-HUN-207/before-1280-usage-by-project.png` (Scratch a plain row), `before-1280-workspaces.png` (no card). AFTER 1280x720: `after-1280-usage-scratch-closed.png` ("▸ Scratch (77 folders)", 10 rows, 7-day period), `after-1280-usage-scratch-open.png` (77 indented folder rows, `aria-expanded=true`), `after-1280-workspaces-card.png`, `after-1280-workspaces-after-add.png` (toast "Added idea-ladder to bep-work", card 6 -> 5, bep-work 1 -> 2 repos). AFTER 375x812: `after-375-usage-scratch-closed.png`, `after-375-usage-scratch-open.png`, `after-375-workspaces-card.png`. Verdict: matches the sketch; the "~11 rows + Scratch instead of 127" is 10 rows (top-10 list) with Scratch as one row. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | New CSS block (`/* Scratch group in By project ... */` in `web/src/app.css`) uses only `var(--...)` tokens; vitest `Breakdown.test.jsx` "CSS: the HUN-207 block uses tokens only" passes (regex finds no `#hex`/`rgb(`/`hsl(`). Disclosure is a real `<button type="button" aria-expanded>`; select has `aria-label="Add to workspace"` inside a named `li`; toasts reuse `useNotify` (error = `role=alert`, success = `role=status`). |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280x720 and 375x812: `document.documentElement.scrollWidth` = `clientWidth` (1265/1265 and 375/375) with Scratch closed, open (77 folders), and on the Workspaces card; long paths ellipsised (`.mono-ellipsis`, full path in `title`); at 375 the select + Add wrap under the path. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-06T23:45:31Z..23:46:23Z UTC, worktree HEAD 06ee898, no implementation commit yet; dashboard from this branch on a COPY of the owner store, `EASY_PERSONAL_HOME` = scratchpad copy, http://127.0.0.1:48329):
```
$ curl .../api/usage/breakdown?by=project   -> groups 13; scratch [('Scratch', 115)]   (the server already sends the 115 folders, HUN-206)
$ curl .../api/workspaces/unassigned        -> unassigned 6   (idea-ladder 18 sessions 2026-11-01, idea-radar 42, kitchd 9, ...)
$ headless Chrome 1280x720, Usage -> By project: {"rows":10,"toggles":[],"scrollW":1265,"clientW":1265}
   Scratch is one plain row ("Scratch  23,419,381"), not a button, cannot be opened; the folders (115 distinct paths all-time in the API answer) are never shown.
$ headless Chrome 1280x720, Workspaces: {"headings":["orphan","bep-work","personal"],"card":false,...}
   No "Repos not in a workspace" card: the 6 unassigned repos are not visible on the page.
```
Screenshots: `tasks/evidence-HUN-207/before-1280-usage-by-project.png`, `tasks/evidence-HUN-207/before-1280-workspaces.png`.

**AFTER** (2026-11-06T23:53:29Z (375x812) and 23:53:49Z (1280x720), commit e50e670, same dashboard-on-store-copy setup, headless Chrome):
```
Usage -> By project, Scratch closed: {"label":"Scratch (77 folders)23,419,381","expanded":"false","rows":10,"scrollW":1265,"clientW":1265}
click Scratch (no request):         {"expanded":"true","rows":87,"nested":77,"scrollW":1265,"clientW":1265}
Workspaces, before Add:             {"heading":"Repos not in a workspace (6)","rows":["idea-ladder | 18 sessions · $121.35 · last used 2026-11-01", ...5 more],"workspaces":["orphan: -","bep-work: 1 repo","personal: 5 repos"]}
choose bep-work, click Add:         toast "Added idea-ladder to bep-work"
Workspaces, after Add:              {"heading":"Repos not in a workspace (5)", idea-ladder gone,"workspaces":["orphan: -","bep-work: 2 repos","personal: 5 repos"]}
```
(The Add ran only on the COPY store in the scratchpad; the owner store was never opened by the dashboard.) Screenshots: `tasks/evidence-HUN-207/after-*.png`.

**DELTA**: The owner can open the Scratch row in Usage By project to see its folders, and can add each repo that is in no workspace to a workspace from the Workspaces page.

**WITNESS**: frontend-developer agent, 2026-11-06 23:45Z-23:54Z UTC; the Bash calls are attributed to HUN-207 in `memory/event-trace/HUN-207.jsonl` by the trace hook (the Supervisor derives the final witness at Stage 4).
