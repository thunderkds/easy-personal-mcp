# TASK_REVIEW — HUN-179: Release 0.3.0: version bump, README and site updates, release plan

> Sibling of `tasks/TASK_GUIDE_HUN-179.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/test_readme.py`: `test_status_names_v0_3_0_not_planning` (version in README + pyproject, tab list) and new `test_live_view_is_documented`; both failed first (`2 failed, 13 passed`), then passed. `tests/test_site_content.py::test_header_has_brand_version_pill_and_github_button` pins the site pill to pyproject; `tests/test_skeleton.py` pins CLI/MCP version to the installed package |
| Verification command run | ☑ pass | `uv run pytest tests/test_readme.py tests/test_site_content.py tests/test_skeleton.py -q` -> `56 passed in 1.06s` |
| Negative cases hold | ☑ pass | Site drift negatives (`test_wrong_version_fails`, drifted stats) and README checker negatives still pass; README has no host path / email / token (`test_no_host_paths_emails_or_tokens`) |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` -> `1049 passed, 6 deselected, 1 xfailed, 1 warning in 124.82s`; `uv run ruff check .` -> `All checks passed!` |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Only change to `site/index.html` is the header pill text `v0.2.0` -> `v0.3.0` (the drift test pins it to pyproject); no layout, style or screenshot changed. No dashboard (`web/`) change |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Text-only change, see row above |
| **UI: Responsiveness at target viewports** | ☑ N/A | Text-only change, see row above |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-01T02:50:54Z from `uv tool install --reinstall --no-cache .` of the unmodified worktree (main 8988140) into a scratch tool dir:

```
$ easy-personal --version
0.2.0
$ easy-personal ping
easy-personal core ok
$ grep -n '^version' pyproject.toml
3:version = "0.2.0"
```

README before (verbatim): `Status: v0.2.0. It runs natively on Linux. ...`, `The first command prints `0.2.0`.`, dashboard paragraph ends `The tabs are Usage, Warnings & plan, Kit, Workspaces and Agents.`; no Live view text anywhere in README. site/index.html: `<span class="pill">usage dashboard · v0.2.0</span>`.

**AFTER**: captured 2026-11-01T03:10:09Z, same install command, fresh scratch tool dir, branch HUN-179:

```
$ easy-personal --version
0.3.0
$ easy-personal ping
easy-personal core ok
$ MCP stdio initialize -> "serverInfo":{"name":"easy-personal","version":"0.3.0"}
```

README now has `Status: v0.3.0`, `The first command prints `0.3.0``, the tab list `Live, Board, Scheduled, Memory, Usage, Warnings & plan, Kit, Workspaces and Agents, plus a First run checklist`, a `### Live view` section (runs only while the dashboard is open, nothing to install, `GET /api/live`, active sessions / last 50 tool calls / token + API-equivalent cost totals, never message text, tool input or tool output) and an Upgrade paragraph (store version 6 -> 7, `store.sqlite.bak-v6`, no config change for the Live view). Site pill: `usage dashboard · v0.3.0`.

**DELTA**: a user installing this commit gets `0.3.0` from the CLI and MCP and can read in the README how the Live view works and that it needs nothing installed.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
