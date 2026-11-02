# TASK_REVIEW — HUN-189: Owner project for gone worktrees

> Sibling of `tasks/TASK_GUIDE_HUN-189.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_owners.py` (19 tests: AC1-9) and `tests/surfaces/test_owner_api.py` (2 tests: API breakdown, kit/board gone). `uv run pytest tests/core/test_owners.py tests/surfaces/test_owner_api.py -q` -> `21 passed`. |
| Verification command run | ☑ pass | `uv run pytest tests/core -q -k "owner or metrics or kit"` -> `34 passed, 443 deselected`; `uv run pytest tests/surfaces -q` -> `209 passed`; `uv run ruff check .` -> `All checks passed!` |
| Negative cases hold | ☑ pass | Two repos share the branch, no repo has it, `main`/`master`/`HEAD`/empty, non-repo parent, existing folder (zero git calls), regex/shell text as a branch, git timeout, plain folder without `.git`, owner never a gone path: all unchanged (tests in `test_owners.py`). |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` -> `1161 passed, 6 deselected, 1 xfailed, 1 warning in 168.34s` (after `npm --prefix web run build`). |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Backend only, no `web/` change. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Backend only, no `web/` change. |
| **UI: Responsiveness at target viewports** | ☑ N/A | Backend only, no `web/` change. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured before any implementation commit, unchanged `main` code at 2f9dbf0, read-only `?mode=ro` connection to `~/.easy-personal/store.sqlite`; script = usage_breakdown(project), kit.scoped_projects, kit_summary(hide_gone), board, memgraph.project_list, tokens_by_project; each timed as median of 3):

```
captured 2026-11-02T10:21:32Z store /home/hungnguyenhuu/.easy-personal/store.sqlite
usage_breakdown(project) all-time: 157 keys, total tokens 7239133029; median 0.428s runs [0.441, 0.424, 0.428]
    2,300,421,336  orderly
    1,383,952,220  personal-agentic-claude
    1,213,879,909  easy-verifier-mcp
      732,328,896  easy-personal-mcp
      376,132,068  idea-radar
      309,276,993  easy-context-mcp
      293,230,753  easy-memory-mcp
      216,479,110  kitchd
      132,445,400  idea-ladder
       83,782,283  easy-ui-mcp
       78,227,869  java-training
       11,916,435  T005
       11,579,131  wt-t165
        9,623,719  wt-t146-t147
        8,483,812  agent-affa2aecb6624047e
kit.scoped_projects: 172 paths (35 alive, 137 gone); median 0.203s
kit_summary(hide_gone): 35 projects, gone=137; median 0.584s runs [0.584, 0.612, 0.574]
board: 35 projects, gone=125; median 0.224s
memory project_list: 31 projects, gone=27; median 0.297s
tokens_by_project: 157 keys; median 0.246s
```

**AFTER** (same script, same read-only connection, code at the HUN-189 commit; captured later, so new ingest adds tokens: the equality check below is on one snapshot):

```
captured 2026-11-02T10:29:22Z store /home/hungnguyenhuu/.easy-personal/store.sqlite
usage_breakdown(project) all-time: 129 keys, total tokens 7253177192; median 0.545s runs [1.002, 0.545, 0.54]
    2,306,919,580  orderly
    1,396,541,868  personal-agentic-claude
    1,245,986,095  easy-verifier-mcp
      741,445,839  easy-personal-mcp
      376,132,068  idea-radar
      309,276,993  easy-context-mcp
      293,230,753  easy-memory-mcp
      216,479,110  kitchd
      144,361,835  idea-ladder
       83,782,283  easy-ui-mcp
       78,227,869  java-training
       11,579,131  wt-t165
        9,623,719  wt-t146-t147
        4,565,152  hungnguyen111
        3,326,165  proj
kit.scoped_projects: 145 paths (37 alive, 108 gone); median 0.350s
kit_summary(hide_gone): 37 projects, gone=108; median 0.699s runs [0.699, 0.683, 0.704]
board: 37 projects, gone=108; median 0.374s
memory project_list: 34 projects, gone=26; median 0.422s
tokens_by_project: 129 keys; median 0.385s
same-snapshot total tokens, owners on/off: 7253177192 7253177192 equal
sum of groups == total: True ; keys on/off: 129 158
```

Why `wt-t165` and `wt-t146-t147` stay in the AFTER list: their branches match two stored repos (`personal-agentic-claude` and a clone at `~/workspace/pets/.spawn/t182-live/repo`), so rule 2 leaves them unchanged (guide: exactly one repo). 21 top-level sibling `wt-*` paths checked: 11 mapped to `personal-agentic-claude`, 6 ambiguous because of that clone (`wt-t146-t147`, `wt-t165`, `wt-t169`, `wt-t170`, `wt-t180-auto`, `wt-t183`), 4 match no stored repo (`wt-t148`, `wt-t150`, `wt-t195`, `wt-t198`: branch never merged or ref deleted). The gone `.claude/worktrees/*` paths and the `orderly/...` subfolders mapped by rule 1. In total 29 stored paths map to an owner on the final run (172 -> 145 listed paths). `T005` and `agent-affa...` left the list (rule 1).

**DELTA**: Usage by project, the top-project warning and the Kit / Board / Memory lists now count a gone worktree under its owner repo, so 157 project keys became 129 and the gone count in Kit/Board fell from 137/125 to 108/108 (reads still under 2 s), with total tokens unchanged.

**WITNESS**: backend-developer agent, 2026-11-02 (real-store reads at 10:21Z BEFORE and 10:29Z AFTER); event trace `memory/event-trace/HUN-189.jsonl` for the Supervisor to confirm.
