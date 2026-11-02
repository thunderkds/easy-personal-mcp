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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_owners.py` (23 tests: AC1-9 and the hidden-folder tie-break) and `tests/surfaces/test_owner_api.py` (2 tests: API breakdown, kit/board gone). `uv run pytest tests/core/test_owners.py tests/surfaces/test_owner_api.py -q` -> `25 passed`. |
| Verification command run | ☑ pass | `uv run pytest tests/core -q -k "owner or metrics or kit"` -> `38 passed, 443 deselected`; `uv run pytest tests/surfaces -q` -> `209 passed`; `uv run ruff check .` -> `All checks passed!` |
| Negative cases hold | ☑ pass | Two visible repos or two hidden repos share the branch (unchanged), hidden+visible tie (visible wins), single hidden match (used), no repo has it, `main`/`master`/`HEAD`/empty, non-repo parent, existing folder (zero git calls), regex/shell text as a branch, git timeout, plain folder without `.git`, owner never a gone path: all unchanged (tests in `test_owners.py`). |
| verify | ☑ pass | Stage 5 2026-11-02 @ 344a170 (Supervisor, own run): worktree web build, `easy-personal dashboard` on the owner's store, `curl` with the token, 3 runs each: `/api/usage/breakdown?by=project` 1.06/0.60/0.67 s, `/api/kit` 0.70–0.72 s, `/api/board` 0.38–0.49 s, `/api/memory-graph` 0.44–0.57 s, `/api/warnings` 1.11–1.16 s, `/api/plan` 1.13–1.35 s (all < 2 s, NFR-013). Usage by project: 123 keys; worktree keys left only `wt-t148`, `wt-t150`, `wt-t195`, `wt-t198` (no ref, never merged — rule 3 as designed); sum of groups == total (True). Gone counts: kit 102, board 102, memory 26 — same as the agent's AFTER. Dashboard stopped by its own process group — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed `core/owners.py` (new), `core/discovery.py` (`repo_of` extract, `resolve_project` same behaviour), `core/metrics.py` (`usage_breakdown` project, `tokens_by_project`), `core/kit.py` (`scoped_projects` / `_stored_projects`, `kernel_error_rate` scope unchanged), `core/workspaces.py` (`project_paths`), callers `rules._top_project_share`, `board.board`, `memgraph.project_list`, API routes. `lane: C2, 7 files, 482 lines, FITS; unpredicted: tests/surfaces/test_owner_api.py` (test only). Code review: 0 P0, 0 P1, 0 P2, 2 P3 (`owner_map` runs a DISTINCT scan on every call, no per-request memo — +0.1–0.15 s, within NFR-013; the `Merge <b>: <text>` pattern can read a word such as `Merge two lists` as a branch — harmless because rule 2 needs a gone path with that branch and a single match). Owner decisions applied: keep the third merge form; hidden-folder tie-break (PRD FR-057, d141886 on main). Security review (manual; built-in needs `origin/HEAD`): git runs via `discovery.run_git` with list args, timeout and no optional locks, only `-C <existing stored folder>`; branch text never reaches git (Python set compare); read-only store; no new route; no text leaves metadata — 0 findings. Startup reads complete (role guide in full) |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` -> `1165 passed, 6 deselected, 1 xfailed, 1 warning in 223.30s` (after `npm --prefix web run build`). |
| **Companion score (easy-verifier)** | ☑ N/A | Not run: learnings 2026-11-01 — easy-verifier refused every repo path |
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

**AFTER** (same script and read-only connection; code at the HUN-189 tie-break commit, 2026-11-02 11:01Z; live ingest adds tokens, so the equality check is on one snapshot; first `usage_breakdown` run includes the cold git reads):

```
captured 2026-11-02T11:01:17Z store /home/hungnguyenhuu/.easy-personal/store.sqlite
usage_breakdown(project) all-time: 123 keys, total tokens 7275249849; median 0.586s runs [0.979, 0.547, 0.586]
    2,323,295,082  orderly
    1,421,717,836  personal-agentic-claude
    1,245,986,095  easy-verifier-mcp
      745,826,859  easy-personal-mcp
      376,132,068  idea-radar
      309,276,993  easy-context-mcp
      293,230,753  easy-memory-mcp
      216,479,110  kitchd
      144,361,835  idea-ladder
       83,782,283  easy-ui-mcp
       78,227,869  java-training
        4,565,152  hungnguyen111
        3,326,165  proj
        2,895,569  tapchat-apis
        1,556,900  kit
kit.scoped_projects: 139 paths (37 alive, 102 gone); median 0.347s
kit_summary(hide_gone): 37 projects, gone=102; median 0.636s runs [0.636, 0.63, 0.639]
board: 37 projects, gone=102; median 0.369s
memory project_list: 34 projects, gone=26; median 0.401s
tokens_by_project: 123 keys; median 0.357s
same-snapshot total tokens, owners on/off: 7275249849 7275249849 equal
sum of groups == total: True ; keys on/off: 123 158
```

Rule 2 tie-break (owner choice "Prefer non-hidden repo"): when 2+ stored repos match, repos with a hidden path component (here `~/workspace/pets/.spawn/t182-live/repo`, a clone) are dropped; exactly one left -> it wins. Result: `wt-t146-t147`, `wt-t165`, `wt-t169`, `wt-t170`, `wt-t180-auto` and `wt-t183` now map to `personal-agentic-claude`. Of 21 top-level sibling `wt-*` paths, 17 map and 4 match no stored repo (`wt-t148`, `wt-t150`, `wt-t195`, `wt-t198`: branch never merged, no ref). 35 stored paths map in total (172 -> 139 listed paths). No `wt-*` row is left in the top 15. `rules.evaluate` (/api/warnings) median 0.977 s, `plan.improvement_plan` (/api/plan) median 1.005 s.

**DELTA**: Usage by project, the top-project warning and the Kit / Board / Memory lists count a gone worktree under its owner repo: 157 project keys became 123 and the Kit/Board gone count fell from 137/125 to 102/102 (reads under 2 s), with total tokens unchanged.

**WITNESS**: backend-developer agent, 2026-11-02 (real-store reads at 10:21Z BEFORE and 11:01Z AFTER); event trace `memory/event-trace/HUN-189.jsonl` for the Supervisor to confirm.
