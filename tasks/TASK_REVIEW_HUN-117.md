# TASK_REVIEW — HUN-117: Warnings, plan and kit accept workspace/account scope (API, MCP, CLI)

> Sibling of `tasks/TASK_GUIDE_HUN-117.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Task breakdown and labels approved; run all tasks in auto mode — FACT: PROJECT_SPEC.md:175 "do all by parallel with tag [auto], merge to main and push"
2. Requirement Fidelity Gate signed auto (Supervisor) — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (row 13)
3. Spawn model from the CLI Model Table — RULE: `PROJECT_SPEC.md` § CLI Model Table
4. Waves (stacked branches) instead of all 8 from main: tasks that edit the same files start from their predecessor's branch — RULE: `PROJECT_SPEC.md` § Tasks ("Waves: 1 = HUN-113, 114, 115, 117 …")
5. Merge to main and push after `/verify` passes (the owner's explicit ask overrides the auto-mode no-merge rule for this one step) — FACT: docs/claude-md/tracks.md:365 "the explicit ask wins for X"
6. security-review done inline by the Supervisor; the built-in command failed with `fatal: ambiguous argument 'origin/HEAD...'` (this repo's remote is `github`, there is no `origin`) — FACT: memory/MEMORY.md:41 "security-review inline"
Checks: tests `uv run pytest -q` — 766 passed, leak 26 passed; reviews code-review — 0 P0–P2, security-review (inline) — 0; real run fixture CLI `easy-personal warnings --account` / `kit --workspace` BEFORE "unrecognized arguments", AFTER scoped output (auto-checked, not /verify)
Not done: merge (owner asked for it; runs after verify); auto verify: running (T182)


## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/surfaces/test_scope_warnings.py` (18 tests; fixture `tests/surfaces/scope_fixture.py`, golden `tests/surfaces/scope_golden.json` captured from the pre-change code). `uv run pytest -q tests/surfaces/test_scope_warnings.py` -> `18 passed`. AC1 golden (`test_no_scope_equals_golden`, `test_api_without_scope_is_the_golden_shape`); AC2 `test_account_scopes_usage_rules`; AC3 `test_workspace_scopes_usage_rules`, `test_kit_summary_filters_projects`; AC4 same test (account); AC5 `test_unknown_scope_raises_even_on_an_empty_store`, `test_api_unknown_scope_is_400_like_the_usage_routes`, `test_cli_unknown_scope_exits_2`; AC6 `test_mcp_tools_accept_scope_and_match_cli`, `test_cli_scope_flags` + `tests/leak/harness.py` (`MCP_CALLS`, `CLI_CALLS`, `API_CALLS`); edge cases `test_tokens_jump_scopes_both_weeks`, `test_kernel_error_rate_follows_scope_projects`, project_dominates abstain in `test_workspace_scopes_usage_rules`. |
| Verification command run | ☑ pass | `uv run pytest -q` (no `EASY_PERSONAL_*` vars set, built web) -> `766 passed, 4 deselected, 1 xfailed, 1 warning in 140.49s (0:02:20)`. `uv run ruff check src tests/surfaces/test_scope_warnings.py tests/surfaces/scope_fixture.py tests/leak` -> `All checks passed!` |
| Negative cases hold | ☑ pass | Unknown workspace/account: API answers 400 with the same JSON as `/api/usage/summary` and never echoes the bad value; CLI exits 2 with stderr only, listing valid labels; core raises even on an empty store (before the history abstain). Empty `?workspace=` = no filter. Workspace with no matching projects selects nothing (`hits == []`, kit `projects == []`). Leak suite: `uv run pytest -q tests/leak` -> `26 passed`. |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | code-review (Supervisor, 2026-10-29): rules/plan/kit/metrics/context scope args + callers (api, mcp, cli), leak harness, tests. lane: C2, 12 files, 640 lines, EXCEEDS (golden fixture + needed helper args; label note). Findings after gate: P0 0, P1 0, P2 0, P3 1 (DISTINCT project scan, pre-existing). security-review inline: all values bound, f-string only builds placeholders/internal clause, unknown scope → 400 — no findings — pass |
| Full smoke suite still green (no regression) | ☑ pass | Same run as above: `766 passed, 4 deselected, 1 xfailed`. |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression** | ☑ N/A | Pure backend task (API, MCP, CLI); no UI change. The selector UI is a follow-up task. |
| **UI: Design-system compliance** | ☑ N/A | Pure backend task (API, MCP, CLI); no UI change. The selector UI is a follow-up task. |
| **UI: Responsiveness** | ☑ N/A | Pure backend task (API, MCP, CLI); no UI change. The selector UI is a follow-up task. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-10-29T08:12:43Z, before any implementation commit; fixture store in a temp `EASY_PERSONAL_HOME`: account 1 `Claude default` has cache reads, account 2 `Claude work` has none; workspace `wa` = repo `/work/a`):

```
$ easy-personal warnings            -> runs, hits = [project_dominates]
$ easy-personal warnings --account 'Claude work'
easy-personal: error: unrecognized arguments: --account Claude work
$ easy-personal warnings --workspace wa
easy-personal: error: unrecognized arguments: --workspace wa
$ easy-personal kit --workspace wa
easy-personal: error: unrecognized arguments: --workspace wa
$ easy-personal plan --account 2
easy-personal: error: unrecognized arguments: --account 2
```

(full capture incl. the usage line: scratchpad `demo117/before.txt`). The API routes `/api/warnings|plan|kit` ignore `workspace`/`account` (`lambda c, q: rules.evaluate(c)`); the MCP tools `warnings`, `improvement_plan`, `kit_summary` take no arguments.

**AFTER** (2026-10-29T08:19:58Z, same fixture store, same commands):

```
$ easy-personal warnings                          hits: [project_dominates 0.9854]      (unchanged from BEFORE)
$ easy-personal warnings --account 'Claude work'  hits: [cache_hit_low 0.0]; project_dominates abstains
$ easy-personal warnings --account 'Claude default'  hits: []
$ easy-personal warnings --workspace wa           hits: []; project_dominates abstains (one project)
$ easy-personal kit --workspace wa                kit projects: ['a']
$ easy-personal plan --account 2                  plan: [cache_hit_low 0.0]
$ easy-personal warnings --account nope           easy-personal: unknown account; valid labels: Claude default, Claude work   (exit 2)
```

(full capture: scratchpad `demo117/after.txt`). `/api/warnings|plan|kit` read `?workspace=&account=`; MCP `warnings`, `improvement_plan`, `kit_summary` take optional `workspace`, `account`.

**DELTA**: The owner can now see warnings, the improvement plan and the kit panel for one account or one workspace (API query, MCP argument, CLI flag); with no argument the output is unchanged.

**WITNESS**: Backend-Implementer agent ran both captures in the HUN-117 worktree on 2026-10-29 (BEFORE 08:12:43Z, before any implementation commit; AFTER 08:19:58Z); the pytest runs are attributed to HUN-117 by the trace hook (`memory/event-trace/HUN-117.jsonl`).
