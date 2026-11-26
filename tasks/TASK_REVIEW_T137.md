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

A second commit follows the server's own review fixes (T136 `1cc683b`): the page says when branches, tags or work trees
were left out, shows `base_shown` for an unlisted default base, and finds a work tree's branch by its shown name. Three
tests failed first; web tests then 1666 passed. Chrome checks repeated against a `serve` built from `1cc683b`. T137 does
not contain T136's fix commit: the merge gate refused that branch merge while both tasks are in progress, and the two
change different files. The commit to review is the branch tip.

Real server (not stubbed), temp data folder, `serve` from `d3215f5` with this commit's built page: a remote default base
showed `origin/trunk` and compared against it; the select kept focus; a removed work tree showed `no such work tree` and
the button brought the list back.

---

## Review findings (Stage 4, independent reviewer, 2026-11-26, commit `7ec73f9`)

**Verdict: fail.** 0 P0, 2 P1, 0 P2, 5 new P3. No confirmed security finding. No source or test file was changed, so no fix was applied.

Reviewed: commit `7ec73f9ac81c819fedc6d059b719b7b6c5662b17`, diff `d3215f5...7ec73f9`, with close reading of `a3eb611..7ec73f9`. Tests, build, probes and mutants ran in `git archive` copies in the session scratch folder (`node_modules` linked from the worktree). Chrome checks used the built page against a **stub API** on `127.0.0.1`. **Nothing was checked against the real server**: this reviewer did not start the Rust app (the earlier review records a permission denial for that). The server contract was read from `1cc683b`.

Lane check (warn only): `lane: C2, 12 files, 957 lines, EXCEEDS (lines 957 > 500); unpredicted: README.md, tasks/TASK_GUIDE_T137.md, web/src/App.jsx, web/src/App.test.jsx, web/src/ChangesView.test.jsx, web/src/LivePanel.test.jsx, web/src/LiveView.jsx, web/src/SessionPanel.jsx, web/src/api.js, web/src/changes.css, web/src/tokens.css`.

Entry point `ChangesView`: reachable (`web/src/App.jsx:5`, `:33`). `1cc683b` and this task's diff change no common file.

### P1

| # | File:line | Finding and failure scenario | How confirmed | Conf. | Action |
|---|-----------|------------------------------|---------------|-------|--------|
| 1 | `web/src/ChangesView.jsx:192`, `:274-280` | *Commit* mode shows one commit and reads another. The owner picks a commit; it is amended or rebased away (or falls out of the newest 100); *Refresh*. The picked id stays in the request, but the select has no option for it, so it shows the first, newest commit. The old commit's files and patch are read as the new one's, and picking the shown commit fires no change. Same failure as earlier finding 2: the guard went into `RefSelect` only. This code was also in `a3eb611`; the earlier review missed it. | Probe test: select `9999999 Add the page (amended)`, request `head=1111111...`. Headless Chrome at 375 and 1280: select `fee1dad add the parser (amended)`, list `from_commit_0ddc0de.txt` (`amend-after-*.png`) | 100 | Offer the unlisted picked commit as itself, or drop the pick when it is no longer listed. Add a test that refreshes with the commit gone. |
| 2 | `web/src/ChangesView.jsx:125`, `web/src/changes.css:9`, `:15` | **New in the fix for earlier finding 9.** The words `cannot open: the name is not text` are drawn on top of the status word, so neither can be read. They sit in the `auto` grid column, which takes their width and squeezes column 1 (the status) to almost nothing. Fails AC 5 and ux check B-c. The test reads `textContent` only, so it passes. | Chrome: the text boxes of status and words intersect at 320, 375, 1024, 1280 and 1920 (not at 640 or 1023); images `badname-375.png`, `badname-1280.png` | 100 | Give these words their own grid row across both columns; pin the rule in the CSS test and look at 375 and 1280. |

### P2

None.

### P3

| # | File:line | Finding | How confirmed | Conf. |
|---|-----------|---------|---------------|-------|
| 3 | `web/src/ChangesView.jsx:200` | With no default base, *Branch* shows the checked-out branch under Base, while the request names no base and the answer says `no base branch was found; pick one`. Picking the shown branch fires no change. In `a3eb611` too. | Probe: shown `main`, no `base` in the request | 100 |
| 4 | `web/src/ChangesView.jsx:195`, `:263` | On a branch with no commit in a repository that has commits (`git checkout --orphan`), *Branch* and *Commit* say `This repository has no commit yet.` *Two refs* asks for the unborn branch name as To (an error). The *Branch* wording came with the fix for finding 11. | Probe with that overview shape (the shape is inferred from `repo_diff.rs` `overview`, not from a real repository) | 75 |
| 5 | `web/src/ChangesView.jsx:239` | New with the fix for finding 8: the work tree select takes the focus back even when the owner moved it elsewhere during the read. | Probe: focus on *Refresh*, answer arrives, focus is on the select | 100 |
| 6 | `web/src/ChangesView.jsx:137` | A picked ref that is gone after *Refresh* is labelled with its full raw ref (`refs/heads/topic`). The list and request still agree. The raw label also skips the escaped form for hidden characters. | Probe for the label; the hidden-character part is a code read | 75 |
| 7 | `web/src/ChangesView.test.jsx` | Three edits pass all 166 tests of the three files: remote looked up before local (`ChangesView.jsx:183`); *Back to the default work tree* keeps the picked refs (`:212`); *Two refs* in a repository with branches but no commit on `HEAD` counted as empty (`:195`). | Mutation run | 100 |

Earlier P3 findings 12 (Python dashboard, accepted as not changed) and 13 stand. Finding 13 re-run: `tests/test_readme.py` gives `1 failed, 18 passed` at both `d3215f5` and `7ec73f9`.

### Earlier findings (review of `a3eb611`)

| # | State at `7ec73f9` | Check by this reviewer |
|---|--------------------|------------------------|
| 1 | closed | Page width equals the window in every captured state at 375; no sideways scroll at 1280. Removing `min-width: 0` fails a test |
| 2 | closed for Base, From, To | Chrome shows `origin/trunk`; both mutants (local only, option removed) fail. The same class remains in the Commit select: new finding 1 |
| 3 | closed | Dropping `tree` fails 3 tests; probe: a moved session asks and shows the new tree |
| 4 | closed | Chrome: error, button, list back, focus on the select |
| 5 | closed | `base: values.head` mutant fails |
| 6 | closed | All four mutants now fail |
| 7 | closed | The README sentence is in the commit |
| 8 | closed | Chrome and mutant. Side effect: new finding 5 |
| 9 | **not closed** | The words are in the row but unreadable: new finding 2 |
| 10 | closed | Two columns at 375 |
| 11 | closed for an empty repository | Chrome and mutants. Side effect: new finding 4 |

### Test runs

The first full web run had `1 failed | 1665 passed (1666)`. Only the tail of its output was kept; the code frame ends with lines that match `web/src/LiveTerm.test.jsx:140-141` (not changed by this task), so line 138 or 139 is a likely but **unproven** source. Seven later full runs each passed 1666, and 30 parallel runs of that file passed.

### ux-evaluate (steps followed from `skills/ux-evaluate/SKILL.md`; stub data)

Screenshots: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/59c442dc-75ac-448b-90d3-a8b45e0be458/scratchpad/r7ec/ui/shots/` (all 375 and 1280 state images opened, as contact sheets in `../sheets/`).

| # | Story / check | Width | Verdict | Reason |
|---|---------------|-------|---------|--------|
| 1 | US-045 pick a file | 375 / 1280 | pass | Grouped by area; green `+` and red `-` lines; no sideways page scroll |
| 2 | US-045 two refs | 375 / 1280 | pass | From, To and the file list show |
| B-a | primary action | 375 / 1280 | pass | View-only screen; pickers and the first files are in view on the default screen |
| B-b | hierarchy | 375 / 1280 | pass | Title, pickers, notes, files, patch |
| B-c | spacing | 375 / 1280 | fail | Text drawn over text in the unopenable file row (finding 2) |
| B-d | 44px targets | 375 | pass | No button or select in `main` measured under 44x44 |
| B-e | readable text | 375 / 1280 | pass | Readable in the images; no font rule changed since `a3eb611`, whose review measured 16px body and 14px paths and patch (not re-measured). The overlap is counted under B-c |
| B-f | states in words | 375 / 1280 | pass | Loading, no changes, error, no repository, no commit, cut list, cut patch, filters off seen. A binary file's patch state was not opened in Chrome |
| B-g | feedback | 375 / 1280 | pass | Picked row highlighted; at 375 the patch can start below the fold |
| B-h | phone layout | 375 | pass | No sideways scroll; compare buttons in two columns |
| B-i | one visual language | 375 / 1280 | pass | Existing accent, buttons, radius |

ux-evaluate: 10 pass, 1 fail, 0 not checked, 0 not checkable. Overall: fail

### security-review pass

No confirmed finding and no `needs_validation` lead. The new option labels and notes are React text nodes; the fix commits add no storage, log, request route or URL part other than `tree` (an id) through `URLSearchParams`. Server code was not re-reviewed.

### Supervisor response to the review of `7ec73f9`

All fixed in the next commit, each with a test seen failing first (the *Back* case is a mutation guard that passed already).

| # | State | Change |
|---|-------|--------|
| 1 | fixed | One `Picker` for *Base*, *From*, *To* and *Commit*: a picked ref that left the lists is offered as itself, marked `(no longer listed)` |
| 2 | fixed | The unopenable words are `.diff-note`, on their own row across both columns; CSS pinned; Chrome against the real server: no text boxes intersect at 320, 375, 1024, 1280, 1920 |
| 3 | fixed | With no default base, *Branch* shows `Pick a base` and sends none |
| 4 | fixed | `This branch has no commit yet.` when the repository has other branches; *Two refs* starts from the default base, else the first branch |
| 5 | fixed | Focus returns to the work tree select only when the read left it nowhere |
| 6 | fixed | As 1: the label is the one shown when picked |
| 7 | fixed | Tests for all three. The lookup order is gone: a default base is sent as its full ref (`refs/heads/` for `main`/`master`, else `refs/remotes/`) |

The one unnamed failure in a first full web run was not reproduced: one full run here passed 1674.

---

## Review findings (Stage 4, independent reviewer, 2026-11-26, commit `bf281fd`)

**Verdict: fail** for commit `bf281fd3e8641d7d9ba3dccf45df784654444a21`. 0 P0, 2 P1, 2 P2, 5 P3. No confirmed security finding. No source or test file was changed, so no fix was applied.

Diff `d3215f5...bf281fd`, with close reading of `7ec73f9..bf281fd`. Tests, build, probes and 38 single-edit mutants ran in `git archive` copies in the session scratch folder (`node_modules` copied from the worktree). Chrome checks used the built page against a **stub API** on `127.0.0.1`. **Nothing was checked against the real server**: the Rust app was not built or started. The contract was read from `1cc683b`.

Lane check (warn only): `lane: C2, 12 files, 1122 lines, EXCEEDS (lines 1122 > 500); unpredicted: README.md, tasks/TASK_GUIDE_T137.md, web/src/App.jsx, web/src/App.test.jsx, web/src/ChangesView.test.jsx, web/src/LivePanel.test.jsx, web/src/LiveView.jsx, web/src/SessionPanel.jsx, web/src/api.js, web/src/changes.css, web/src/tokens.css`.

Entry point `ChangesView`: reachable (`web/src/App.jsx:5`, `:33`). `1cc683b` and this task's diff change no common file. Startup-reads check: not applicable (the guide names the Supervisor as implementer; there is no implementer spawn or final report).

### P1

| # | File:line | Finding and failure scenario | How confirmed | Conf. | Action |
|---|-----------|------------------------------|---------------|-------|--------|
| 1 | `web/src/ChangesView.jsx:224` | *Two refs* in a repository with no default base: picking **To** also changes **From**. `from = base ?? head` follows the picked To until From is picked. Both start at `trunk`; the owner picks To = `topic`; From jumps to `topic`, the request is `topic..topic` and the page says `No changes.` The trigger is narrow: no local `main` or `master` and no `refs/remotes/origin/HEAD` (also an unborn branch with no default base). The selects do show what is read, and picking From afterwards works. The same expression is in `7ec73f9` (code read); both earlier reviews missed it. | Probe: after picking only To, `base=refs/tags/v1&head=refs/tags/v1`. Chrome 375 and 1280 (`nobase_pickTo-*.png`). Git in a fixture `trunk` repository: all three base lookups of `default_base` fail | 100 | `const from = base ?? defaults.head`. Tried in the copy: From stays, and all 174 tests of the three files pass, so add a test that picks To first. |
| 2 | `web/src/ChangesView.jsx:49-64`, `:401` | *Refresh* gives no visible feedback. The last answer stays while all three reads run, with no loading mark and no busy button. During a slow read the screen is unchanged, and when nothing changed it stays unchanged for good (an error also stays as it was). The server allows 10 s per git call. Fails ux check B-g; `code-review` rates a ux-evaluate `fail` P1. Both earlier reviews judged B-g on the file pick only. | Chrome, stub reads delayed 6 s: the images before and during Refresh are identical at 375 and 1280 (`refresh_before-*`, `refresh_during-*`; JavaScript click, so no focus ring). Probe: 0 `status` elements during Refresh | 100 | Show that a read is running (loading words or a busy *Refresh*) and keep the last answer. Add a test. |

### P2

| # | File:line | Finding and failure scenario | How confirmed | Conf. | Action |
|---|-----------|------------------------------|---------------|-------|--------|
| 3 | `web/src/LiveView.jsx:898-901` (`:916` is the only wiring) | A Live session that runs in the app's Terminal has no *Changes* button: its row opens `TermPanel`, which gets no `onOpenChanges`. FR-141 and AC 4 ask for the button on a Live session's panel. `TermPanel.jsx` is not in the guide's predicted files, so the Supervisor must say whether it is in scope. | Probe: the same live session shows 1 *Changes* button without a terminal entry and 0 with one | 100 that it is absent, 75 that it is required | Add the button to the terminal panel, or record it as a cut that the owner accepts. |
| 4 | `web/src/ChangesView.jsx:190` | No test has `master` as the default base. Dropping `\|\| name === 'master'` passes all 174 tests; every `master` repository would then ask `refs/remotes/master`, which fails in *Branch* and *Two refs*. | Mutant M03 survived | 100 | One test with `base: 'master'`. |

### P3

| # | File:line | Finding | How confirmed | Conf. |
|---|-----------|---------|---------------|-------|
| 5 | `web/src/ChangesView.jsx:140` | `(no longer listed)` is cut off in the closed select when the label is long: after an amend, the Commit select reads `bf281fd T137: second review fixes (gor…` at 375 and 1280, so the mark shows only in the opened list. The commit shown is the one read. | `amended-375.png`, `amended-1280.png` | 100 |
| 6 | `web/src/ChangesView.jsx:38` | Two repositories with the same folder name in one workspace get the same label (`app (acme)` twice). | Probe | 100 |
| 7 | `web/src/ChangesView.jsx:277` | When the overview answers `tree: null`, the Work tree select shows the first tree while the files are read from the server's default one. The server does that only if the picked tree is not in its list (more than 100 work trees; code read). | Probe for the page | 75 |
| 8 | `web/src/ChangesView.test.jsx` | Edits that pass all 174 tests: unborn To ignores the default base (M07); first ref always taken as a local branch (M09); focus flag never cleared (M14); a commit picked in *From*/*To* treated as unlisted (M21); no To for a detached work tree (M22); the word `binary` removed from the file row (M24). | Mutation run | 100 |
| 9 | `site/index.html:322` (older) | Says 10 dashboard views; `App.jsx` has 14 entries (13 at `d3215f5`). `test_stat_strip_numbers_come_from_real_sources` fails at both commits. This diff adds one view. | Test run at both commits | 100 |

Earlier P3 findings 12 (Python dashboard, accepted as not changed) and 13 stand.

### Acceptance Criteria at `bf281fd`

| AC | State | Evidence |
|----|-------|----------|
| 1 | met | Page tests; mutant M28 (orphan repo listed) fails |
| 2 | **not met in one state** | Four modes ask the picked refs; finding 1 changes a ref the owner did not pick |
| 3 | met | Areas, status, counts, marked patch, *Refresh* reads again (M26 fails); feedback is finding 2 |
| 4 | met except the terminal panel | `App.test.jsx`, `LivePanel.test.jsx`; M32, M34, M35 fail; finding 3 |
| 5 | met | Text only (test); empty, loading, error, cut, binary, filters off seen in Chrome |

### Findings of the review of `7ec73f9`

| # | State | Check by this reviewer |
|---|-------|------------------------|
| 1 | closed | Test; M02 and M19 fail; Chrome shows the picked commit after an amend. See finding 5 |
| 2 | closed | Chrome 375 and 1280: no text boxes of a file row intersect; words on their own row; M31 fails |
| 3 | closed | Chrome shows `Pick a base`; no `base` sent; M16 fails |
| 4 | closed | Chrome: `This branch has no commit yet.`; M10, M11 fail |
| 5 | closed | Test; M12, M13 fail (M14 survives, finding 8) |
| 6 | closed | M01 fails |
| 7 | closed | M04, M10, M15 fail |

The response's "each with a test seen failing first" holds: 9 tests of the new test file fail on the `7ec73f9` page and CSS, and the *Back* guard passes there. Its real-server Chrome check at 320 to 1920 was not repeated.

### Runs

- `npm run build`: exit 0, `72 modules transformed` (twice).
- `npx vitest run`: `Test Files 50 passed (50)`, `Tests 1674 passed (1674)` in 3 of 3 full runs (10:24Z to 10:46Z).
- Mutants against `ChangesView.test.jsx`, `App.test.jsx`, `LivePanel.test.jsx` (174 tests): 38 run, 28 failed the tests, 10 survived (M03, M05, M07, M09, M14, M17, M21, M22, M24, M36). M17 is the fix proposed for finding 1; M36 changes no behaviour; M05 touches a full-name base that the server of `1cc683b` does not send for local or remote branches.
- `pytest tests/test_readme.py tests/test_site_content.py`: `4 failed, 69 passed` at `d3215f5` and at `bf281fd`, the same four names.

### ux-evaluate (steps followed from `skills/ux-evaluate/SKILL.md`; stub data)

Screenshots: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/59c442dc-75ac-448b-90d3-a8b45e0be458/scratchpad/rbf2.NrUWI4/ui/shots/` (all state images opened as contact sheets in `../sheets/`; the two Refresh pairs were compared pixel by pixel).

| # | Story / check | Width | Verdict | Reason |
|---|---------------|-------|---------|--------|
| 1 | US-045 pick a file | 375 / 1280 | pass | Grouped by area; green `+` and red `-` lines; page width equals the window |
| 2 | US-045 two refs | 375 / 1280 | pass | From, To and the file list show |
| B-a | primary action | 375 / 1280 | pass | View-only screen; pickers and first files in view on the default screen |
| B-b | hierarchy | 375 / 1280 | pass | Title, pickers, notes, files, patch |
| B-c | spacing | 375 / 1280 | pass | No overlap; one column of pickers, files and patch aligned |
| B-d | 44px targets | 375 | pass | Measured in every captured state: no button or select in `main` under 44x44 |
| B-e | readable text | 375 / 1280 | pass | Body 16px; paths and patch 14px |
| B-f | states in words | 375 / 1280 | pass | Loading, no changes, list error, no such work tree with button, no repository, no commit, cut list, cut patch, filters off, binary |
| B-g | feedback | 375 / 1280 | fail | *Refresh* changes nothing on screen while reading (finding 2) |
| B-h | phone layout | 375 | pass | No sideways page scroll; compare buttons in two columns |
| B-i | one visual language | 375 / 1280 | pass | Existing accent, buttons, radius |

ux-evaluate: 10 pass, 1 fail, 0 not checked, 0 not checkable. Overall: fail

### security-review pass

No confirmed finding and no `needs_validation` lead. Rejected leads: script from repo content (ref names, subjects, authors, labels and patch lines are React text nodes; no `innerHTML` in `ChangesView.jsx` or `api.js`); content kept (component state only; no storage, cookie or console call; the server answers `no-store`, contract test in `1cc683b`); injection from a picked ref (`URLSearchParams`; the server resolves it after `--end-of-options`). The server routes are T136 and were not re-reviewed.

### Not checked

Real server; widths other than 375 and 1280; real keyboard or screen reader; the Python dashboard; the Rust suite; the Python suite beyond the two files; the easy-verifier score.

### Supervisor response to the review of `bf281fd`

Fixed in the next commit unless the row says otherwise.

| # | State | Change |
|---|-------|--------|
| 1 | fixed | With no default base, *From* starts at the default head and picking *To* leaves it. Test |
| 2 | fixed | A status line says `Reading again…`, then `Read at <time>`: the time of the oldest answer on screen. Also under a repository error. Tests |
| 3 | fixed | In scope (FR-141, AC 4: the owner chose a *Changes* button on a Live session), so `TermPanel.jsx` joins the files: the button shows once the terminal session has its Claude id. Tests |
| 4 | fixed | Tests for a `master` and a full-ref default base |
| 5 | fixed | `(no longer listed)` comes first |
| 6 | fixed | Repositories with one folder name show their whole paths. A narrow closed select still cuts a long path at its end; the opened list shows it |
| 7 | fixed | A work tree that is not listed is offered as `This folder (not listed)` |
| 8 | fixed | Tests for each; see the mutation run below |
| 9 | not changed | The site's view count and its failing test are older than this task |

Mutation run on a scratch copy (241 tests in five files): the review's mutants that still apply, plus new ones for these
fixes (M17 turned round, M34 and M37 rewritten for the new text, M01 replaced by the mark-order mutant). All fail the
tests except M36 (changes no behaviour) and two that showed redundant guards (`!popped` and the app-sessions check on the
terminal panel); both guards are removed.

After that: web tests 1684 passed, build and `check_static` ok. Headless Chrome against a `serve` built from T136
`50870c3`: duplicate names, the status line during and after a slowed *Refresh*, *To* leaving *From*, the amended commit's
mark at 375 and 1280, a `master` base, page width equal to the window, no console problem. The terminal panel's button
was **not** checked in a browser (it needs a running agent session); its tests cover it.

---

## Review findings (Stage 4, independent reviewer, 2026-11-26, commit `c43c472`)

Recorded by the Supervisor from the reviewer's hand-back: the step-limit hook stopped the reviewer before it wrote this
file. Condensed wording; the reviewer's ratings are unchanged.

**Verdict: fail** for commit `c43c4724f455130391a5dd4cb1b75b13ca65905e`. 0 P0, 1 P1, 0 P2, 6 P3, no confirmed security
finding. Must fix before merge: finding 1 only; every P3 can be listed for later.

| # | Sev | Where | Finding | Conf. |
|---|-----|-------|---------|-------|
| 1 | P1 | `web/src/appsession.css:218`, `web/src/TermPanel.jsx:245` | New in this commit: the fourth terminal button does not fit. At 375 *Stop* crosses the card's border and *Pop out* breaks into two lines; at 360 the page scrolls sideways; at 320 *Stop* is mostly cut off | 100 |
| 2 | P3 | `web/src/ChangesView.jsx:39` | Two repositories of one folder name: the closed select shows only the shared start of the path at 375 and 1280, so the picked one cannot be told | 100 |
| 3 | P3 | `web/src/ChangesView.jsx:63-64`, `:249-255` | Uncommitted, then Commit (slow), then Uncommitted shows the old list with its old "Read at" and no reading mark while a new read runs; "Read at" also shows above "Loading the files" | 100 |
| 4 | P3 | `web/src/ChangesView.jsx:385`, `:415` | *Refresh* does not read the workspace list again: if its repositories change elsewhere, the same `repo_no` reads another repository under the old label | 100 page probe; server order by code read |
| 5 | P3 | `web/src/ChangesView.jsx:263` | A work tree vanishes on *Refresh*: after *Back to the default work tree*, keyboard focus is on the page body | 100 (jsdom) |
| 6 | P3 | T136 `diff_routes.rs:60-63`, `transcript_route.rs:116` | A new session with no transcript the app can find would answer "no such session" to *Changes* | 75, not run against the real server |
| 7 | P3 | `web/src/ChangesView.test.jsx`, `App.test.jsx` | 15 of 48 single-edit mutants survive; among them: the reading line fed only by the file list (a repository error then reads "Read at Invalid Date"), the older-answer guard removed, the tab's `scoped` flag, the non-JSON 200 guard | 100 |

Earlier findings (review of `bf281fd`): 1, 2, 4, 5, 7, 8 closed; 3 closed but brought finding 1; 6 partly closed (finding
2); 9 older, unchanged.

Runs: `npx vitest run` 1684 passed in 3 of 3 full runs of a `git archive` copy; `npm run build` exit 0; `pytest
tests/test_readme.py tests/test_site_content.py` `4 failed, 69 passed` at `d3215f5` and `c43c472`, same four names.
ux-evaluate (375 and 1280, stub API): 9 pass, 2 fail (B-c and B-h, both finding 1). AC 1 to 5 met on the page with
stubbed data. Lane check (warn only): C2, 14 files, 1277 lines, exceeds 500.

Evidence rows as the reviewer gave them (not entered in the table below): tests cover the AC pass; verification command
pass; negative cases pass with gaps (finding 7); blast radius pass; full smoke suite web pass, not fully ticked; companion
score N/A; visual regression fail and responsiveness fail (finding 1); design system pass.

Limits: the permission system denied the reviewer's command that created synthetic git repositories, so nothing was
checked against the real server. The merge gate blocked a command containing `git merge-tree`; the reviewer ran its other
read-only parts without it. The step limit killed its last read-only search and the Demonstration capture. Note for the
T136 review, from reading code only and not counted: `hidden()` treats a carriage return as hidden, so each line of a
CRLF file's patch would end in `\u{d}`.

### Supervisor response to the review of `c43c472`

| # | State | Change |
|---|-------|--------|
| 1 | fixed in the next commit | The terminal button row wraps, right-aligned. CSS pin test seen failing first; web tests 1685 passed; headless Chrome with the reviewer's stand-in server: all four buttons inside the card with no sideways scroll at 320, 360, 375, 390, 414 and 1280 |
| 2 | open, for later | My earlier response said only a narrow select cuts the path; that was wrong, it also does at 1280 |
| 3-7 | open, for later | Not changed |

---

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☑ fail | At `bf281fd`: `web/src/ChangesView.test.jsx`, `web/src/App.test.jsx`, `web/src/LivePanel.test.jsx`. Open: no test picks To first with no default base (AC 2, finding 1); none for a `master` default base (finding 4); AC 4 has no case for a terminal panel (finding 3). Earlier: At `7ec73f9`: `web/src/ChangesView.test.jsx`, `web/src/App.test.jsx`, `web/src/LivePanel.test.jsx`. Earlier gaps closed. Still open: no test for *Refresh* in *Commit* mode with the picked commit gone (AC 2, 3; finding 1), and the AC 5 test for an unopenable file checks text only, not that it can be read (finding 2) |
| Verification command run | ☑ pass / ☐ fail | `git archive` copy of `bf281fd`, 2026-11-26T10:24:12Z to 10:25:16Z (build first, then tests): `npm run build` exit 0 (`72 modules transformed`, `built in 2.52s`); `npx vitest run`: `Test Files 50 passed (50)`, `Tests 1674 passed (1674)`. Two more full runs by 10:46:29Z: same counts. Earlier: `git archive` copy of `7ec73f9`, 2026-11-26T10:15:41Z to 10:16:09Z: `npm run build` exit 0 (`72 modules transformed`, `built in 241ms`), then `npx vitest run`: `Test Files 50 passed (50)`, `Tests 1666 passed (1666)`. One earlier full run had 1 failure (see *Test runs*) |
| Negative cases hold | ☐ pass / ☑ fail | At `bf281fd`: 38 single-edit mutants against the three test files (174 tests): 28 killed, 10 survived (findings 4 and 8; M17 is the proposed fix, M36 changes nothing). Probes show findings 1, 2, 3, 6, 7. Earlier: At `7ec73f9`: 23 single-edit mutants against the three test files (166 tests): 20 killed (the 5 earlier survivors and a dropped `tree` among them), 3 survived (finding 7). 7 probe tests passed, each recording what the page does now; they show findings 1, 3, 4, 5, 6 |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass / ☐ fail | At `bf281fd`: the changed files; `api.js`, `useLoad.js`, `Loading.jsx`, the panel wiring in `LiveView.jsx`; the contract in `1cc683b` `diff_routes.rs` and `repo_diff.rs`. Skipped: re-review of T136 server code and unrelated views. Earlier: At `7ec73f9`: the changed files; helpers `api.js`, `useLoad.js`, `Loading.jsx`, `app.css` rules used; the contract in `1cc683b` `diff_routes.rs` and `repo_diff.rs`. Skipped: re-review of T136 server code and unrelated views |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | Not ticked. At `bf281fd` the web suite passed 3 of 3 full runs (1674 tests). Python: only `tests/test_readme.py` and `tests/test_site_content.py`, `4 failed, 69 passed`, the same four at `d3215f5`. Rust suite not run (no Rust file changed). Earlier: Not ticked: the web suite passed 7 of 8 full runs (1666 tests), one run had 1 unidentified failure. Rust suite not run (no Rust file changed). Python: only `tests/test_readme.py`, 1 failure that is the same at `d3215f5` |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☑ N/A | Not run in the review of `bf281fd`. |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☑ fail / ☐ N/A | `bf281fd`: ux-evaluate 10 pass, 1 fail (B-g, finding 2); stub data. Earlier: `7ec73f9`: ux-evaluate 10 pass, 1 fail (B-c, finding 2); stub data |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass / ☐ fail / ☐ N/A | `bf281fd`: the one added CSS rule `.diff-note` uses tokens only; the file's token-only test passes. Earlier: `7ec73f9`: the added CSS rules set layout only; the file's token-only test passes |
| **UI: Responsiveness at target viewports** | ☑ pass / ☐ fail / ☐ N/A | `bf281fd`: page width equals the window at 375 and 1280 in every captured state; phone targets 44px or more; no overlapping text in file rows. Earlier: `7ec73f9`: page width fits at 375 and 1280 and phone targets are 44px or more; text overlaps in the unopenable file row at 375 and 1280 (finding 2) |

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
