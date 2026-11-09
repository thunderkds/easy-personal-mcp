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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

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

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
