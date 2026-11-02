**Date**: 2026-11-02
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: Gone project, Owner project; Critical Constraints: read-only, metadata only)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-009, FR-054, FR-057, NFR-013, Open Questions 20–21

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner report 2026-11-02: "the usage by project, you are running by the folder, not focus on the project, so it included a lot of worktree also". Owner chose (forced choice): path rule + branch rule, at read time, store unchanged.

Facts (Supervisor, read-only on the owner's store, 2026-11-02): `discovery.resolve_project` maps a worktree to its main repo only when the folder still exists at ingest. 173 stored `events.project_path` values; 44 gone non-`/tmp` paths: 12 under `<repo>/.claude/worktrees/<x>`, 25 sibling folders like `~/workspace/pets/wt-t165` (branch `feat/t165-site-scan-first`, found only in `personal-agentic-claude` merge subjects, `git log --all --grep`), 7 subfolders such as `orderly/.claude/tasks/...`, `orderly/RestaurantUI/e2e/test-results/...`. 93 `/tmp` paths stay as they are. Usage by project keys on `metrics._folder(project_path)` (the basename), so `wt-t165` shows as its own row.

**Restated intent**:

> Add one core function that maps a stored project path to its *owner project* (FR-057), and use it everywhere a project is grouped or listed: usage by project (`metrics._keys` "project", `tokens_by_project`), the top-project warning (`rules._top_project_share`, via `tokens_by_project`), and `kit.scoped_projects` (feeds Kit, Board and the Memory list, so their gone counts drop and the owner repo shows once). Rules in order: an existing folder → unchanged; (1) a gone path whose nearest existing parent folder is inside a git repo → that repo's top level (same as `discovery.resolve_project` on that parent); (2) else the gone path's distinct `git_branch` values from its events (excluding empty, `main`, `master`, `HEAD`) matched against the stored repos that exist — a repo matches when a branch is a ref (`refs/heads`, `refs/remotes/*`) or appears in a merge commit subject (`Merge branch '<b>'`, `Merge pull request #N from <owner>/<b>`); exactly one matching repo → that repo; (3) else unchanged. Nothing is written to the store.

**Out of scope**:

* Rewriting stored events, a migration, any ingest change
* `/tmp` paths (they stay gone projects)
* The page (no `web/` change: owner rows arrive under the existing keys)
* The Live view and session timeline grouping

**Requirement Refs**: US-025, FR-054, FR-057, NFR-013

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, owner forced choice "Path + branch", 2026-11-02)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: none (builds on HUN-186, merged)

**Entry point**: `easy-personal dashboard` → `GET /api/usage/breakdown?by=project` (`surfaces/api.py`) → `core/metrics.py`; `GET /api/kit`, `/api/board`, `/api/memory-graph` → `kit.scoped_projects`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | A gone `<repo>/.claude/worktrees/x` path and a gone `<repo>/sub/dir` path map to `<repo>` (temp git repo fixture) | FR-057 |
| 2 | A gone sibling path whose branch `feat/x` is a ref in exactly one stored repo, or in a merge subject of exactly one stored repo, maps to that repo | FR-057 |
| 3 | Branch found in two repos, in none, or only `main`/`master`/`HEAD`/empty → path unchanged; a gone `/tmp/x` → unchanged; an existing folder → unchanged with no git call | FR-057 |
| 4 | Usage by project merges the owner's rows: the gone worktree key disappears, the owner's tokens = its own + the worktree's; the sum over all projects is unchanged (golden: total equal before/after) | FR-057 |
| 5 | Top-project warning uses owner projects (the share is computed on merged totals) | FR-057 |
| 6 | `kit.scoped_projects` returns each owner once and no mapped gone path; Kit / Board / Memory `gone` counts drop by the mapped paths | FR-054, FR-057 |
| 7 | Workspace and account filters still work: a workspace with repo R includes R's mapped gone worktree events | FR-035, FR-057 |
| 8 | Speed: git calls happen only for gone paths and only ≤ 2 per stored repo per process (cached); every changed `/api/*` read < 2 s median of 3 on the owner's store (NFR-013) | NFR-013 |
| 9 | Store untouched (mtime + row count equal after the calls); a branch or path string is passed to git only as a list argument, never through a shell | NFR-003 |
| 10 | Real run on the owner's machine: usage by project before/after — `wt-*` and `.claude/worktrees/*` rows gone, `personal-agentic-claude` / `easy-verifier-mcp` grew; paste both lists (top 15) and the timings | US-025 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core -q -k "owner or metrics or kit" && uv run pytest tests/surfaces -q && uv run ruff check .
```

---

## Approach

**Vital slice**: owner map (path rule → branch rule) with a per-process cache → `_keys` / `tokens_by_project` → `scoped_projects` dedupe.
**Cut list**: no store write, no config switch, no UI note "N worktrees merged", no cross-machine mapping.

Build the map once per read: collect the distinct stored paths (one query), keep the existing ones as candidates, run the rules for gone paths only. For the branch rule read each candidate repo once (`for-each-ref` + `log --all --merges --format=%s`, hardened `discovery.run_git`), cached with `lru_cache` or a module dict keyed by repo path. Keep `_folder` as is; apply the owner map before it.

---

## Edge Case Checklist

- [ ] Nearest existing parent is not in a git repo (`~/workspace/pets`) → not rule 1
- [ ] Parent repo is itself the worktree's main repo vs a different nested repo
- [ ] Branch names with `/`, regex characters, quotes → exact match, no regex injection
- [ ] Same branch name in two repos (e.g. `fix/typo`) → unchanged
- [ ] A repo with no merges / not a git repo / git missing / git timeout → no match, no error
- [ ] Owner path itself gone → never map to a gone path
- [ ] Chains: never map to another mapped path (one step only)
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/discovery.py` or new `src/easy_personal/core/owners.py` | owner map |
| `src/easy_personal/core/metrics.py` | apply in `_keys` "project" and `tokens_by_project` |
| `src/easy_personal/core/kit.py` | apply in `scoped_projects` (dedupe) |
| `src/easy_personal/core/workspaces.py` | only if AC7 needs it |
| `tests/core/test_owners.py` (new), `tests/core/test_metrics.py`, `tests/core/test_kit.py`, `tests/surfaces/...` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/store.py`, `core/migrations/`, `src/easy_personal/sources/` | No store or ingest change |
| `web/` | No page change |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Temp git repos (main + `git worktree add` then remove; a sibling worktree with a merged branch; two repos sharing a branch name) and a temp store with events on those paths; assert the map, the breakdown sums, scoped_projects, filters; a count of git calls; store unchanged; real run on the owner's store (read-only).

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review