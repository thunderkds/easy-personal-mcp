# TASK_REVIEW — T137: Git diff 2 — Changes tab and Live button

> Sibling of `tasks/TASK_GUIDE_T137.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` or `[admin]` tag; with `[admin]`, the `Not done:` line names `merge (admin: after the auto verify pass)`,
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
`Memory:` records the diff-driven memory pass you run before you stop: each entry written, or `nothing new`.
After an auto verify pass the hook, never you, adds `Closed: row moved to Done <date> — …` and moves the row,
or adds `Paused: done not moved — <reason>`. It moves the row only when this block holds `Mode: auto`, a
numbered decision, `Checks:`, `Memory:`, the `Not done:` auto verify slot with its result, and `Not done:`
naming the merge — no `<...>` left (`docs/claude-md/tracks.md` `### Auto close-out`).
With `/verify (yours)`, your pass goes in the `verify` row below: Result `☑ pass`, Notes `/verify by the user, session <id>, commit <sha>`.

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its proposal>; Advocate 2: <its proposal>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Memory: <each entry the memory pass wrote, as `file — entry` | nothing new>
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours), with [admin] only after the one retry also ended blocked>

-->

## Review findings (Stage 4, independent reviewer, 2026-11-26)

**Verdict: fail.** 0 P0, 2 P1, 5 P2, 6 P3. No confirmed security finding. No source or test file was changed, so no fix was applied.

Reviewed: commit `a3eb611d992f80eeab7a9443e278d4dac949729e`, diff `d3215f5...a3eb611`. Tests, build and probes ran in a `git archive` copy of that commit in the session scratch folder. Screenshots come from the built page against a **stub API** on `127.0.0.1`. Starting the Rust binary to seed an isolated store was denied by the permission system, so **nothing was checked against the real server**.

Lane check (warn only): `lane: C2, 11 files, 786 lines, EXCEEDS (lines 786 > 500); unpredicted: tasks/TASK_GUIDE_T137.md, web/src/App.jsx, web/src/App.test.jsx, web/src/ChangesView.test.jsx, web/src/LivePanel.test.jsx, web/src/LiveView.jsx, web/src/SessionPanel.jsx, web/src/api.js, web/src/changes.css, web/src/tokens.css` (the guide does predict most of these, in brace form).

Entry point `ChangesView`: reachable (`web/src/App.jsx`).

### P1

| # | File:line | Finding and failure scenario | How confirmed | Conf. | Action |
|---|-----------|------------------------------|---------------|-------|--------|
| 1 | `web/src/changes.css:3` | The whole page scrolls sideways and selects are cut off. `.field` has no width limit, so a select grows to its longest option and `max-width: 100%` follows the grown parent. At 375px: Branch and Two refs give a 600px page and Commit 706px with this repo's own kind of names (branch `T137`, this commit's subject); a long work tree name gives 781px on the first screen. At 1280px: a long commit subject in Commit mode gives scrollWidth 1324. Breaks both UX rows. | Headless Chrome over CDP, `scrollWidth` read at 375 and 1280, images read | 100 | Verified in the stub at 375 and 1280: `.changes .field { min-width: 0; max-width: 100% }` removes the overflow. Add a test that pins the rule. |
| 2 | `web/src/ChangesView.jsx:177` (`full`), `:179` | Base and From show the wrong branch. With no `main`/`master`, the server's default base is `origin/<name>` (`default_base`, `symbolic-ref --short`). `full()` maps local branches only, so the value matches no option and the select shows the first option. Seen: asked `base=origin/trunk`, Base and From showed `a-feature`. The owner reads a diff as "against a-feature" when it is not, and picking `a-feature` fires no change because it already looks picked. | Probe test on the page (`base.value` = `refs/heads/a-feature`, request `base=origin/trunk`); a fixture clone prints `origin/trunk` for that git command | 100 | Map a remote name to `refs/remotes/<name>` too; add a test with a remote default base. |

### P2

| # | File:line | Finding and failure scenario | How confirmed | Conf. | Action |
|---|-----------|------------------------------|---------------|-------|--------|
| 3 | `web/src/ChangesView.jsx:169`, `:191` | A session's list and patch are asked without `tree`. The server finds the folder again on every request from the newest transcript record (`diff_routes.rs` `picked`). If the session moves to another work tree while the tab is open, a clicked file can answer from the new tree under the old tree's list and selector. | Probe: files and file queries carry only `session`; server code read | 75 | Once the overview answers, send `tree: overview.tree` with files and file. |
| 4 | `web/src/ChangesView.jsx:198` | If the picked work tree is removed (normal after a task merge), Refresh answers `no such work tree`, every picker disappears and later Refreshes send the same dead `tree`. The owner must leave the tab or pick another repository; with one repository only leaving works. | Probe: two Refreshes, same error, only the Repository select left | 100 | On that error drop `tree`, or keep the work tree picker. |
| 5 | `web/src/ChangesView.test.jsx:148-151` | AC 2 gap: Two refs is tested with base equal to head and From is never picked. Replacing `base: values.base` with `values.head` (line 188) passes all tests. | Mutation run: survived | 100 | Pick From and To with different refs and assert both. |
| 6 | `web/src/ChangesView.test.jsx` | AC 5 and safety gaps. These mutants also pass: show another path's answer (`ChangesView.jsx:62`); remove the patch loading state (`:88`); hide the file list error (`:267`); keep the *Changes* button on a gone session (`SessionPanel.jsx:494`). | Mutation run: 4 survived | 100 | One assertion for each. |
| 7 | `README.md:167` | The README sentence about the Changes tab is only in the work tree, not in the reviewed commit. A merge of `a3eb611` ships without it. | `git status`, `git diff` | 100 | Commit it and review that commit. |

### P3

| # | File:line | Finding | Conf. |
|---|-----------|---------|-------|
| 8 | `web/src/ChangesView.jsx:205` | Changing the work tree unmounts the select while the overview loads, so keyboard focus is lost (code read, not run in a browser). | 75 |
| 9 | `web/src/ChangesView.jsx:120` | A file with a non-text name is disabled with no words saying why. | 100 |
| 10 | `web/src/changes.css` | At 375px *Two refs* wraps alone onto a second row of the compare control. The B-c fail below is carried at P1 by its cut-off selects (finding 1); the wrap alone is rated here. | 100 |
| 11 | `web/src/ChangesView.jsx:186-188` | A repository with no commit: Branch asks with no base and Two refs shows empty selects and asks for a ref that cannot exist, so the owner gets an error where "no commit yet" (as in Commit mode) would help (server answer inferred from code). | 75 |
| 12 | `web/src/App.jsx:33` | The Python dashboard shares this page and has no `/api/diff/*`, so its Changes tab can only show a status-code error (code read). | 75 |
| 13 | `tests/test_readme.py:79` | Older, not from this diff: the pinned tab sentence is absent from README at `d3215f5` and `a3eb611` (checked by search; the test was not run). | 100 |

Process note: the guide says it was written after the first draft of the page code (before any commit). Hard-Stop Gate 1 asks for it first.

### ux-evaluate (steps followed from `skills/ux-evaluate/SKILL.md`; stub data)

Screenshots: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/59c442dc-75ac-448b-90d3-a8b45e0be458/scratchpad/` + `shots/` (long names and states), `shots2/` (short and usual names; `d` suffix = desktop emulation), `shots3/` (after a pick).

| # | Story / check | Width | Verdict | Reason |
|---|---------------|-------|---------|--------|
| 1 | US-045 pick a file | 375 / 1280 | fail | Grouping by area, mint `+` lines and red `-` lines show; the page scrolls sideways (finding 1) |
| 2 | US-045 two refs | 375 / 1280 | fail | At 375 From and To run past the right edge (600px page); at 1280 they show with the list |
| B-a | primary action | 375 / 1280 | pass | No call to action on a view-only screen; pickers and first files are in view on the default screen |
| B-b | hierarchy | 375 / 1280 | pass | Title, pickers, notes, files, patch |
| B-c | spacing | 375 / 1280 | fail | Finding 10 and cut-off selects at 375 |
| B-d | 44px targets | 375 | pass | Every button and select inside `main` measured at least 44x44 |
| B-e | readable text | 375 / 1280 | pass | Body 16px; paths, counts and patch lines are 14px (the app's small size) |
| B-f | states in words | 375 / 1280 | pass | Loading, "No changes.", server error line, no-repository hint, cut, binary, filters off were seen |
| B-g | feedback | 375 / 1280 | pass | Picked row is highlighted; at 375 the patch can start below the fold |
| B-h | phone layout | 375 | fail | Sideways scroll (finding 1) |
| B-i | one visual language | 375 / 1280 | pass | Existing accent, buttons, radius |

ux-evaluate: 7 pass, 4 fail, 0 not checked, 0 not checkable. Overall: fail

### security-review pass

No confirmed finding and no `needs_validation` lead. Rejected leads:
- Script from repo content: all text goes through React text nodes; no `dangerouslySetInnerHTML`/`innerHTML` in `ChangesView.jsx` or `api.js`; a patch holding `<img onerror>` drew as text (test and screenshot).
- Query injection from a path or ref: `URLSearchParams` encodes it; the server decodes `+` and `%` to match, and takes only a listed path as a literal pathspec (code read).
- Content kept (NFR-002 seventh exception): `ChangesView.jsx` uses component state only; no storage or console call (search).
- Token or folder in a URL: `getDiff` sends the token in the header only; the page sends workspace + repo number or session id, never a folder.
- Server routes themselves are T136, unchanged here, and were not re-reviewed.

---

## Supervisor response (2026-11-26, after the review of `a3eb611`)

Tests first: 10 tests failed on `a3eb611`'s code, then passed. This response is in the commit that holds the fixes;
that commit needs its own reviewer run.

| # | Result | What changed, and the test that pins it |
|---|--------|------------------------------------------|
| 1 | fixed | `.changes .field { min-width: 0; max-width: 100% }`; CSS pin test. Chrome at 375 and 1280 with very long names: page width equals the window in every mode |
| 2 | fixed | A server branch name maps to its local, then remote, ref; a value in neither list is offered as itself. Two tests |
| 3 | fixed | The file list and patch send `tree` from the overview. Query pins in the page tests |
| 4 | fixed | *Back to the default work tree* under the error when a work tree was picked. Test |
| 5 | fixed | From and To picked with different refs |
| 6 | fixed | Tests for the loading state and no other file's patch, the list error, and no button on a gone session. The 5 surviving mutants and a dropped `tree` are now killed |
| 7 | fixed | The README sentence is committed |
| 8 | fixed | The work tree select takes the focus back after the read. Test |
| 9 | fixed | The row says "cannot open: the name is not text". Test |
| 10 | fixed | Compare buttons in two columns below 40rem. CSS pin test |
| 11 | fixed | "This repository has no commit yet." in Branch, Commit and Two refs; nothing is asked. Test |
| 12 | not changed | The Python dashboard already shows *Watch* with no routes behind it (`/api/machine`, `/api/limits/history`, `/api/effectiveness` are not in `surfaces/api.py`); hiding tabs per server is a separate change |
| 13 | not changed | Older than this task |

Real server (not stubbed), temp data folder, `serve` from `d3215f5` with this commit's built page: a remote default base
showed `origin/trunk` and compared against it; the select kept focus; a removed work tree showed `no such work tree` and
the button brought the list back.

---

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☑ fail | `web/src/ChangesView.test.jsx`, `web/src/App.test.jsx`, `web/src/LivePanel.test.jsx`. AC 1, 3, 4 covered; AC 2 (From) and AC 5 (list error, patch loading) have gaps (findings 5, 6); no test for either UX row |
| Verification command run | ☑ pass / ☐ fail | `npx vitest run && npm run build` in a `git archive` copy of `a3eb611`, 2026-11-26T07:54:42Z: `Test Files 50 passed (50)`, `Tests 1654 passed (1654)`; `vite v8.3.5 ... 72 modules transformed ... built in 256ms` |
| Negative cases hold | ☐ pass / ☑ fail | 12 single-edit mutants in the scratch copy against the three test files (154 tests). KILLED 7: unopenable file clickable, session target never cleared, workspace scope ignored, lines outside a hunk marked, server error text dropped, cut note removed, Changes button removed. SURVIVED 5: button kept on a gone session, another path's answer shown, patch loading removed, list error hidden, range base replaced by head. Probes for findings 2, 3, 4: 4 tests passed, each asserting the faulty behaviour |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass / ☐ fail | Reviewed: the 12 changed files; callers and helpers `App.jsx`, `LiveView.jsx`, `SessionPanel.jsx`, `useLoad.js`, `Loading.jsx`, `app.css`; the contract in `diff_routes.rs`, `repo_diff.rs`, `api.rs` `Query`. Skipped: re-review of T136 server code and unrelated views |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | Not ticked: only the web suite ran (1654 passed). Rust and Python suites not run; the diff changes no Rust or Python file |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☑ N/A | Not run in this review |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☑ fail / ☐ N/A | ux-evaluate overall fail (table above); stub data |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass / ☐ fail / ☐ N/A | `changes.css` read: colours and fonts are tokens (the file's own test passes); the one new token `--color-red-soft` is in `tokens.css`; computed line colours at 375 and 1280 match the tokens |
| **UI: Responsiveness at target viewports** | ☐ pass / ☑ fail / ☐ N/A | 375 and 1280: page width 600 to 781px at 375, scrollWidth 1324 at 1280 with a long commit subject (finding 1) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (the page has no Changes view at the branch point, before the first T137 commit):

```
$ date -u +%FT%TZ; git rev-parse --short HEAD; git grep -c ChangesView HEAD -- web/src; echo "grep exit=$?"
2026-11-26T05:59:04Z
d3215f5
grep exit=1
```

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
