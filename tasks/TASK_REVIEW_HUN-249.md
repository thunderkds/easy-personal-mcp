# TASK_REVIEW — HUN-249: Landing page and README show the dashboard changes since 0.4.0

> Sibling of `tasks/TASK_GUIDE_HUN-249.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/test_site_content.py`: 7 new tests, 67 -> 74 in the three site files. `test_dashboard_lists_what_it_does_one_item_per_change` (AC1, AC2 no FR-091/092), `test_dashboard_list_is_inside_the_existing_section_and_adds_no_section_or_image` (AC1), `test_readme_uses_the_same_four_terms_as_the_page` (AC3), `test_page_and_readme_promise_no_version_after_0_4_0` (AC4), `test_a_removed_dashboard_item_fails`, `test_a_dashboard_item_that_names_a_feature_not_built_fails`, `test_a_dashboard_item_of_three_sentences_or_a_release_promise_fails` (AC5 negatives). Red first: 4 of them failed before `site/index.html` and `README.md` changed. |
| Verification command run | ☑ pass | 2026-11-09T02:52:41Z to 02:56:25Z, `uv run pytest tests/test_site_content.py tests/test_site_theme.py tests/test_site_deploy_config.py -q && uv run pytest -q`: first half `74 passed in 0.78s`; second half `1764 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 222.23s (0:03:42)`. An earlier full run (02:48) had 2 failures in `tests/test_readme.py` (`test_live_view_is_documented`, `test_live_view_documents_attention_states_settings_and_start_limit`): my first README wording dropped two phrases that those tests lock. I restored both phrases truthfully (no test changed); that run is not the one reported above. |
| Negative cases hold | ☑ pass | A removed item, an item with `FR-091`, `store health check`, `send a command`, three sentences, or `next release` each make `check_dashboard_changes` raise (3 tests; each asserts the edited HTML differs from the page, so none passes vacuously). |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | `1764 passed, 2 skipped, 6 deselected, 1 xfailed` (above). |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | BEFORE `tasks/evidence-HUN-249/before-dashboard-1280x720.png` (section 905x790) vs AFTER `tasks/evidence-HUN-249/after-dashboard-1280x720.png` (905x1222) and the real-viewport `tasks/evidence-HUN-249/after-list-viewport-1280x720.png`. Only change: the heading `What the dashboard does` and 5 items between the intro line and the four unchanged screenshots. I read the images: text is readable, no overlap. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | 3 new CSS rules (`.changes`, `.changes li`, `.changes strong`) use only `var(--color-*)` tokens, no hex; `tests/test_site_theme.py` (`test_colors_only_via_variables`, `test_dark_only`, `test_text_contrast_is_aa_on_the_page_background`) passes in the 74. No existing class changed. |
| **UI: Responsiveness at target viewports** | ☑ pass | Real headless Chrome 157 over `python3 -m http.server --bind 127.0.0.1` from `site/`, CDP `Emulation.setDeviceMetricsOverride`. 1280x720: `docScrollWidth` 1265 <= 1280 (15 px is the scrollbar). 390x844: `docScrollWidth` 375 <= 390; each `<li>` right edge 359 and `scrollWidth == clientWidth` (no clipped text). Screenshots: `tasks/evidence-HUN-249/after-list-viewport-390x844.png`, `after-list-viewport-1280x720.png`. Trap: an element-clip capture (`captureBeyondViewport` + clip) at 390 px showed text cut at the right edge; it was a capture artifact (the metrics show no overflow and the real-viewport shot is clean), so I deleted that file rather than keep a misleading one. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-09, before any HUN-249 implementation commit; branch tip `0057b38`, the
worktree was fast-forwarded to `main` first because it started at a stale `d79c100`):

Command 1, the guide's measured-gap check, 2026-11-09T02:43:29Z to 02:43:32Z:

```
$ git log d11c075..HEAD --oneline -- README.md
b24d831 Merge github/main (PR #4: README kernel link)
466e4f1 docs(readme): kernel link names thunderkds/easy-kit
```

Command 2, the first half of the guide's Verification Command, same run:

```
$ uv run pytest tests/test_site_content.py tests/test_site_theme.py tests/test_site_deploy_config.py -q
...................................................................      [100%]
67 passed in 1.40s
```

Command 3, the page and README say nothing about the changes (2026-11-09T02:44:16Z; `grep -ci "<term>"` counts in
`site/index.html` / `README.md`):

```
session row: site=0 README=0
kit stage: site=0 README=0
Kit by workspace: site=0 README=0
report folder: site=0 README=2   (both lines are the config keys ui:/verifier: reports_dir, not the Kit view)
Scratch: site=0 README=1         (the CLI/MCP usage line only, not Usage opening Scratch or the Workspaces page)
Saved report: site=0 README=0    (site has only the tool row `save_report` in #tools)
```

Prior content of the `#dashboard` section of `site/index.html` (lines 507-516, verbatim):

```html
<section id="dashboard">
  <h2>The dashboard</h2>
  <p>Run <code>easy-personal dashboard</code> to open it. These screenshots use test fixtures, not real usage. The projects <code>demo-app</code> and <code>notes-site</code> are made up.</p>
  <div class="shots">
    <figure><img src="dashboard-usage.png" ... alt="Usage view of the dashboard for the last 7 days: ..."><figcaption>Usage: totals, cost, and breakdowns by day, model and project.</figcaption></figure>
    <figure><img src="dashboard-warnings.png" ...><figcaption>Warnings and plan: a rule that fires shows its measured value. Here none fired.</figcaption></figure>
    <figure><img src="dashboard-kit.png" ...><figcaption>Kit: what each kit part left for each project.</figcaption></figure>
    <figure><img src="dashboard-workspaces.png" ...><figcaption>Workspaces: the list of workspaces, with repos and connectors for each.</figcaption></figure>
  </div>
</section>
```

(`...` marks only the long `width/height/alt` attributes, which this task does not change.) There is no list
and no heading "What the dashboard does". The page pill says `usage dashboard · v0.4.0`.

BEFORE screenshot, `#dashboard` section at 1280x720 viewport (section clip 905x790, no horizontal scroll,
`docScrollWidth` 1265 < 1280):
`/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/worktrees/agent-aeeafa91022c13746/tasks/evidence-HUN-249/before-dashboard-1280x720.png`

**AFTER** (2026-11-09T02:52:41Z, same Verification Command): `74 passed in 0.78s` for the three site files (was 67),
`1764 passed, 2 skipped, 6 deselected, 1 xfailed` for the full suite. New `#dashboard` content (verbatim):

```html
<h3>What the dashboard does</h3>
<ul class="changes">
  <li data-change="live"><strong>Live:</strong> It shows one session row for each active session: bars for tokens, cost and tool calls, and a chip for its kit stage, such as Stage 4 &middot; code-review. A row opens in place to its tool feed, which labels each call, and the list stays complete after a restart or with two dashboards open.</li>
  <li data-change="kit-workspace"><strong>Kit by workspace:</strong> For each workspace it shows the easy-verifier scores (repos scored, lowest score, mean, newest report) and the easy-ui use (sessions, calls, last call). The easy-ui check results stay overall, not split by workspace.</li>
  <li data-change="report-folders"><strong>Report folders:</strong> Under each workspace row, Kit by workspace lists the report folders of its repos, as a reference: any folder in a repo root with report in its name. A folder opens to its files and a file opens in a new tab, with its own CSS and embedded screenshots, but a script in a report does not run.</li>
  <li data-change="scratch"><strong>Scratch and unassigned repos:</strong> Usage opens its Scratch row to list the throwaway folders in it. The Workspaces page lists the repos that are in no workspace, each with an Add to control.</li>
  <li data-change="saved-reports"><strong>Saved reports:</strong> The investigate prompt in Claude Code can save its report with <code>save_report</code>, and the server adds its own numbers for every rule. The Warnings &amp; plan page lists the saved reports, and an opened one shows each rule as before and now.</li>
</ul>
```

AFTER screenshots: `tasks/evidence-HUN-249/after-dashboard-1280x720.png`, `after-list-viewport-1280x720.png`, `after-list-viewport-390x844.png`.

**DELTA**: A developer who reads the landing page or the README now learns what the dashboard does today (live session rows, Kit by workspace, report folders, Scratch and unassigned repos, saved reports), where before the page held only four screenshots and the README had no text for any of it.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]


---

## AC 2: claim to proof (FR + one test or file per item)

Each claim was checked against the FR text in `PRD.md` and against `main` (this branch is fast-forwarded to `main` `0057b38`, plus the HUN-249 commits).

| Item | Claim (page) | FR | Proof on `main` |
|---|---|---|---|
| live | one session row per active session; bars for tokens, cost, tool calls | FR-072 | `web/src/LiveView.test.jsx` "AC1: three sessions give three rows with project, 8-char id, active/idle text and three bars" |
| live | kit stage chip, e.g. `Stage 4 · code-review` | FR-083, FR-084 | `web/src/LiveView.jsx:258`; `tests/core/test_live.py::test_stage_follows_the_newest_mapped_skill_and_keeps_unmapped_ones_out` |
| live | row opens in place to its tool feed; each call carries a label | FR-085, FR-084, FR-081 | `web/src/LiveView.test.jsx` "AC1: a row is a native button; a click expands its feed inside its own card"; `tests/core/test_live.py::test_label_skill_and_agent_calls_carry_the_safe_field_and_no_canary` |
| live | list stays complete after a restart (warm start) | FR-086 | `tests/core/test_live.py::test_warm_start_lists_a_stored_session_with_whole_totals_before_any_new_line` |
| live | ... or with two dashboards open | FR-071 | `tests/core/test_live.py::test_two_tails_on_one_store_each_list_every_session_and_every_tool_event_once_in_the_store` |
| kit-workspace | easy-verifier: repos scored, lowest, mean, newest report | FR-087 | `tests/core/test_kit_workspaces.py::test_verifier_counts_scored_repos_mean_lowest_and_newest`; `web/src/WorkspaceKit.test.jsx` "renders one row per workspace with the verifier and easy-ui text" |
| kit-workspace | easy-ui: sessions, calls, last call; check results stay overall | FR-088 | `web/src/WorkspaceKit.test.jsx` "shows the one overall line, once, under the rows"; `tests/surfaces/test_api_kit_workspaces.py` |
| report-folders | folders of a workspace's repos listed under each row, "report" in the name, reference only | FR-089 | `tests/core/test_kit_reports.py`; `web/src/WorkspaceKit.test.jsx` "AC1: one muted line per folder, relative path, files, easy-ui sessions and newest day" |
| report-folders | a folder opens to its files; a file opens in a new tab | FR-090 | `web/src/WorkspaceKit.test.jsx` "AC1: the line is a button with aria-expanded ..." and the `blob:fake-1` new-tab test (lines 372-390); `tests/surfaces/test_api_kit_report_files.py::test_read_route_headers_and_bytes` |
| report-folders | report keeps its own CSS and embedded screenshots; a script does not run | FR-093, FR-094 | `tests/surfaces/test_api.py:78` asserts the CSP `default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'`; `:92` only `style-src` allows `unsafe-inline`; `:97` `img-src` is `'self' data:`; `script-src` is unset, so it falls back to `'self'` |
| scratch | Usage opens its Scratch row to list its folders | FR-069 | `web/src/Breakdown.test.jsx` "shows one closed Scratch (3 folders) row ... click opens 3 indented rows"; `tests/core/test_scratch.py::test_ac1_two_groups_repo_with_worktree_and_scratch_with_four_folders` |
| scratch | Workspaces page lists repos in no workspace, each with Add to | FR-070 | `web/src/UnassignedRepos.test.jsx`; `tests/surfaces/test_api_unassigned.py::test_route_answers_repos_in_no_workspace`; `web/src/WorkspacesView.jsx:713` mounts it |
| saved-reports | the investigate prompt saves a report with `save_report`; server adds its own numbers | FR-076, FR-077 | `tests/surfaces/test_mcp_save_report.py::test_valid_call_stores_one_row_and_returns_id_and_time` |
| saved-reports | Warnings & plan lists saved reports; an opened one shows before and now | FR-078 | `web/src/WarningsPlanView.test.jsx` (region "Saved reports", line 292 on); `tests/surfaces/test_api_reports.py::test_one_report_shows_before_and_now_per_rule` |

No claim about FR-091 or FR-092 (store health check, send-command button): not built. `test_a_dashboard_item_that_names_a_feature_not_built_fails` guards it.

I read these proofs from the test names and the FR text. I did not re-run the `web/` vitest suite or open the dashboard for this task; the Python suite above ran in full.

Notes for the supervisor:
- The guide's change table says "Usage opens the Scratch row (repos in no workspace)". That blurs two things. In FR-069 the Scratch row holds throwaway folders (temp folder, gone with no owner project, not in a repo, hidden folder). "Repos in no workspace" is the separate FR-070 list on the Workspaces page. The page and README state them separately, as the FRs say.
- Two existing README sentences were stale after FR-086, so I changed them (they sit in the Live view part): "Known limit: the tail starts from the positions that the ingest saved ..." became a smaller known limit (the warm start reads only files changed in the last hour, and only the last part of a large file), and "The totals count what the tail saw since the dashboard started" became "The totals add up the listed session rows" (`web/src/LiveView.jsx:383`, `sum(sessions)`). The "never shows ... tool input or tool output" sentence now adds "except the labels above" (labels are the DDR-0004 exception). `tests/test_readme.py` locks the old phrases `before `easy-personal dashboard` started` and `never shows message text, tool input or tool output`; both stay.
- The Live item names FR-071 (two dashboards) inside its second sentence, to keep the two-sentence cap.
- The worktree started at a stale commit `d79c100` with no guide and no FR-093/FR-094; I fast-forwarded the branch to `main` (`git merge --ff-only main`, no merge commit) before reading anything else. The post-bash hook asked for a memory pass after that; I wrote nothing under `memory/` (Supervisor-only).
