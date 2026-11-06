**Date**: 2026-11-07
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Workspace, orphan workspace, project, kit part; Critical Constraints: metadata only, read-only sources)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-011, FR-013, FR-035, FR-054, FR-057, FR-060, FR-087, FR-088, NFR-002, NFR-013, open question 30; `BRAINSTORMING_LOG_KITWS.md` (Path A, Edge Case Checklist); `src/easy_personal/core/kit.py`, `src/easy_personal/core/workspaces.py` (`project_paths`, `_under`, `list_all`), `src/easy_personal/sources/verifier.py` (`verifier_part`), `src/easy_personal/sources/ui.py` (`ui_part`), `src/easy_personal/sources/context.py` (`in_use`: the tool-name `GLOB` pattern), `src/easy_personal/core/metrics.py` (`_scope`, `_period`, `_utc_bound`), `src/easy_personal/core/memo.py`, `src/easy_personal/surfaces/api.py` (route list, `_query`, `_period`, `_who`), `tests/leak/harness.py` (`API_CALLS`)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-07: "for verifier mcp, let collect and show the report by workspace to the dashboard" and "for ui mcp, do the same stuffs with 1". Run in auto mode: the Supervisor chose the placement and the easy-ui limit (PRD open question 30, owner review open).

**Restated intent**:

> Add `kit.workspace_rollup(conn, since=None, until=None, account=None)` and route `GET /api/kit/workspaces` (query: `since`, `until`, `account`; same token and host guard as the other `/api/*` routes). It returns:
>
> ```
> {"workspaces": [
>    {"name": str, "color": str|None, "repos": int,
>     "verifier": {"status": "ok"|"no data", "repos_scored": int, "repos_total": int,
>                  "mean": float|None, "lowest": {"project": <folder name>, "value": float}|None,
>                  "newest_utc": str|None},
>     "ui": {"status": "ok"|"not used", "sessions": int, "calls": int, "last_utc": str|None}}, ...],
>  "ui_checks": <the existing sources.ui.ui_part(conn) dict, unchanged>}
> ```
>
> One entry per non-orphan workspace, in the order of `workspaces.list_all`. `repos` = the workspace's repo count; `repos_total` = its repos whose folder exists (FR-054).
>
> **Verifier** (FR-087): read the stored `verifier_scores` rows only. A row counts for a workspace repo when its project path is equal to or under the repo path (FR-035 rule, `_under`, owner map FR-057 applied, as `workspaces.project_paths` does). `status "ok"` rows with a number are *scored*; `unknown`, abstained and missing rows are *no score* and never count as 0. `mean` is over scored repos only; `lowest.project` is the folder name only (never a full path); `newest_utc` is the newest `generated_at_utc` among scored. No scored repo → `status "no data"`, `mean`/`lowest`/`newest_utc` null. The verifier part ignores `since`/`until`/`account` (newest score is not a period figure).
>
> **easy-ui** (FR-088): from stored events of the workspace's project paths (`claude_code` source, abstained = 0, inside `since`/`until`, `account` limit through `metrics._scope`): `sessions` = distinct `session_id` with at least one `mcp__easy-ui__*` call; `calls` = the number of tool names that start with `mcp__easy-ui__` over those events (the `tool_names` comma list); `last_utc` = newest event time with such a call. Sub-agent (sidechain) events count. None → `status "not used"`, zeros, null. `ui_checks` stays overall; no check is ever put under a workspace.
>
> No file is read, nothing is written, no migration. Unknown `account` raises like the other kit reads (400 from the route).

**Out of scope**: the page (HUN-239); an MCP tool or CLI command; any change to `sources/verifier.py`, `sources/ui.py` or the store; attributing check results to a workspace.

**Requirement Refs**: FR-087, FR-088 (data side), FR-011, FR-013, FR-035, FR-054, FR-057, FR-060, NFR-002, NFR-013

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (auto (Supervisor), 2026-11-07 — see PRD open question 30)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria

---

## Dependencies & Reachability

**Depends on**: None (HUN-207 touches only `web/`)

**Entry point**: `GET /api/kit/workspaces` in `src/easy_personal/surfaces/api.py`, read by HUN-239

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Two workspaces: WS-A with 3 repos (2 scored 80 and 60, 1 no data) → `repos_scored 2`, `repos_total 3`, `mean 70.0`, `lowest {project: <folder of the 60 repo>, value 60.0}`, `newest_utc` = newest scored time. WS-B with no verifier row → `status "no data"`, nulls | FR-087 |
| 2 | An abstained row and an `unknown`-schema row are *no score*: not in `mean`, not 0; a workspace whose only rows are those → `"no data"` | FR-087, FR-011 |
| 3 | The orphan workspace is not listed; a workspace with no repos → entry with `repos 0`, `verifier "no data"`, `ui "not used"` (no error); a repo whose folder is gone is not in `repos_total` | FR-087, FR-054 |
| 4 | A project under a repo path (subfolder) and a gone worktree owned by a repo (FR-057) count for that repo; `/a/b` never matches `/a/bc`; a repo listed in two workspaces appears in both | FR-087, FR-035, FR-057 |
| 5 | easy-ui: 2 sessions, one with 3 `mcp__easy-ui__*` calls in several events, one with 1 call in a sidechain event, plus a session of another server's tools → `sessions 2`, `calls 4`, `last_utc` correct; events outside `since`/`until` and of another workspace not counted; no call → `"not used"` | FR-088, FR-060 |
| 6 | `ui_checks` equals `sources.ui.ui_part(conn)` and is the same for every workspace (one top-level key, not per workspace); no code path reads `ui_sessions` per workspace | FR-088, FR-013 |
| 7 | Route: 200 with the shape above and token; 401/403 as the other routes; unknown `account` → 400; `since`/`until`/`account` honoured; no workspaces → `{"workspaces": [], "ui_checks": ...}` | FR-087, FR-088 |
| 8 | Leak: canaries in verifier `reason_code`/scope, in a workspace description and in easy-ui session `target` appear nowhere in the response or the logs; full repo paths never appear (folder names only); add the route to `API_CALLS` in `tests/leak/harness.py` | NFR-002 |
| 9 | Speed: on a synthetic store of 160 projects and 10 workspaces the read answers in < 2 s (median of 3); paste the timing | NFR-013 |
| 10 | Existing tests pass; `uv run pytest -q` passes. Real run: temp `EASY_PERSONAL_HOME` with fixtures, `easy-personal dashboard`, `curl` the route with the token, paste the JSON | — |

---

## UI / Design Acceptance Criteria

N/A — no page change (HUN-239). UI Evidence rows ☐ N/A.

---

## Evaluation & Acceptance

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core tests/surfaces tests/leak -q && uv run pytest -q
```

## Approach

**Vital slice**: a pure function over the store → route → tests. Resolve each workspace to repo roots once; match `verifier_scores` rows in Python with `_under`; for easy-ui, one `SELECT` per workspace on `events` filtered with `metrics._scope`/`_period` and the `tool_names GLOB 'mcp__easy-ui__*' OR GLOB '*,mcp__easy-ui__*'` pattern of `context.in_use`, then count names in Python. Add the memo (`core/memo.py`) only if AC 9 fails without it.
**Cut list**: no MCP tool, no CLI, no per-repo detail list in the response, no caching unless AC 9 needs it.

---

## Edge Case Checklist

- [ ] Workspace with no repos → entry, not an error
- [ ] Orphan workspace left out
- [ ] Repo in two workspaces → counted in both
- [ ] Verifier `unknown` or abstained → no score, never 0; mean over scored only
- [ ] Lowest shows the folder name only; two repos with one folder name stay two repos
- [ ] Gone folder → not in `repos_total`
- [ ] easy-ui: many calls in one event counted once each; one session counted once
- [ ] `tool_names` NULL or empty → no crash
- [ ] Period bounds are half-open as in `metrics._period`
- [ ] Planted secrets (NFR-002) never in the response

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/kit.py` | `workspace_rollup` (+ small helpers) |
| `src/easy_personal/surfaces/api.py` | the route |
| `tests/core/test_kit_workspaces.py` (new) | AC 1–6, 9 |
| `tests/surfaces/test_api_kit_workspaces.py` (new) | AC 7 |
| `tests/leak/harness.py`, `tests/leak/` | AC 8 (route in `API_CALLS`; canary test) |
| `README.md`, `site/` | only if a drift test requires the new route to be listed |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/sources/verifier.py`, `src/easy_personal/sources/ui.py` | Ingest unchanged (FR-011, FR-013) |
| `src/easy_personal/core/migrations/`, `src/easy_personal/core/store.py` | No store change |
| `web/` | The page is HUN-239 |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

## Questions Log

* 2026-11-07 DEBATE (Supervisor, auto) easy-ui check results by workspace: time overlap, hash of `target`, new easy-ui field, or keep overall → keep overall; show events-based use per workspace (safer-option rule: no guess, no change to FR-013/NFR-002) ⚠ decided for you — please review (PRD open question 30)
