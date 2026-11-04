**Date**: 2026-11-04
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (Critical Constraints: metadata only, read-only on sources, 127.0.0.1 only, token + Host/Origin on dashboard routes; glossary: Owner project, Workspace)
2. Read the memory slice in your spawn prompt. Read `memory/MEMORY.md` in full only if your work reaches a file the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Apply the C2 process from your role guide
6. Read `PRD.md` FR-069, FR-070, FR-057, FR-035, FR-054, open question 24; `BRAINSTORMING_LOG_SCRATCH.md` (Option A, Edge Case Checklist); `src/easy_personal/core/owners.py`; `src/easy_personal/core/metrics.py` (`usage_breakdown`, `tokens_by_project`, `_folder`); `src/easy_personal/core/workspaces.py` (`_under`, `project_paths`); `src/easy_personal/surfaces/api.py` (`create_app`, `_query`); `src/easy_personal/surfaces/mcp.py` (`usage_breakdown` description); `README.md` (MCP tools section) and `tests/test_readme.py`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-04: "relate to the workspace, I saw the management is not good also, let evaluate and improve that. by the way, I saw a lot of draft folder and project that I do not work on, seem like the agent just create or open for some actions". Chosen: "Auto 'Scratch' group", "Gone → Scratch", "List + 'Add to' button", brainstorm Option A ("continue").

**Restated intent**:

> **Scratch (FR-069).** One read-time rule, in `owners.py`, maps every stored project path to a *project key*: its FR-057 owner repo when it has one; else `SCRATCH` when it is a *scratch folder*; else the path itself. A scratch folder: (a) `realpath` sits under `realpath(tempfile.gettempdir())` or `/tmp`; (b) the folder no longer exists; (c) it exists but `discovery.repo_of` returns None (not inside a git repo); (d) a path part starts with `.`. Owner first, then the rule. Git checks reuse the 300 s cache in `owners.py`; a `/tmp` path is decided before any git call.
>
> `usage_breakdown(by="project")` and `tokens_by_project` use this map. Scratch paths form one group with `key: "Scratch"` and `scratch: true`, plus `folders`: the same per-group fields (tokens, messages, cost, split) for each scratch folder name, sorted like groups. Totals unchanged. Other `by` values unchanged. CLI `usage --by project` and MCP `usage_breakdown` show the same (one core). Update the MCP tool description and `README.md` so the drift test passes.
>
> **Unassigned repos (FR-070 read side).** `workspaces.unassigned(conn, prices)` → real projects (project key is a path, not Scratch) with stored usage whose path is not equal to or under any workspace repo path (`_under`), each `{path, name, sessions, api_equivalent_cost: {usd, abstained_models, price_table_date}, last_used_utc}`, newest `last_used_utc` first. Served read-only at `GET /api/workspaces/unassigned` (behind the token + Host check like every `/api/*` route) as `{"repos": [...]}`. All stored time (no period). No CLI or MCP surface.

**Out of scope**: the dashboard UI (separate task), any write route, kitperf/kit/workspaces filter grouping (they keep `owner_map`), store schema, ingest, a hide list.

**Requirement Refs**: FR-069, FR-070, US-025, US-029

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, 2026-11-04; owner picked each choice in AskUserQuestion)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (Owner project, Workspace); new terms *scratch folder*, *project key*, *unassigned repo* added at Stage 2
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered

---

## Dependencies & Reachability

**Depends on**: none

**Entry point**: `metrics.usage_breakdown(by="project")` (CLI `easy-personal usage --by project`, MCP `usage_breakdown`, `GET /api/usage/breakdown`); `Route("/api/workspaces/unassigned", ...)` in `src/easy_personal/surfaces/api.py`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | A tmp store with events in: a real git repo, a folder under `tempfile.gettempdir()`, a gone non-repo path, an existing non-repo folder, a repo inside a `.hidden/` folder, and a gone worktree with an FR-057 owner → `usage_breakdown(by="project")` has exactly two groups: the repo (worktree tokens included) and `Scratch` with `scratch: true` and 4 `folders` | FR-069 |
| 2 | Sum of all groups' tokens and cost (Scratch included) equals `total`; `usage_summary` unchanged | FR-069 |
| 3 | `tokens_by_project` uses the same keys (the `project_dominates` rule sees `Scratch` as one project) | FR-069 |
| 4 | A path under `/tmp` triggers no git call (patch `discovery.run_git` and assert not called for it) | FR-069 |
| 5 | `by="day"/"model"/"tool"` output is byte-identical to before for the same store | FR-069 |
| 6 | `workspaces.unassigned`: with workspace repos `[R1]`, events in R1, R1/sub (stored as R1), R2, a scratch folder → only R2 listed, with sessions, cost, last used; newest first; empty list when all covered | FR-070 |
| 7 | `GET /api/workspaces/unassigned` answers `{"repos": [...]}`; no token / wrong Host → refused like other `/api/*` routes | FR-070 |
| 8 | Leak test: planted secret / prompt text in the fixtures never appears in the breakdown, the new route or logs | NFR (metadata only) |
| 9 | `tests/test_readme.py` passes with the README describing the `Scratch` group | FR-069 |
| 10 | Existing tests still pass | — |

---

## Evaluation & Acceptance

### Verification Command (exact, runnable)

```bash
uv run pytest -q
```

Plus a real run on a COPY of the owner store (`EASY_PERSONAL_HOME=<copy>`): `uv run easy-personal usage --by project` → paste group count and the Scratch line (folder count, tokens); `curl` the new route with the dashboard token → paste repo names + count (expected ≈ 9 today).

## Approach

**Vital slice**: `owners.project_map` + tests → wire `usage_breakdown` / `tokens_by_project` → `folders` on Scratch → README + MCP description → `workspaces.unassigned` + route + tests.
**Cut list**: no CLI/MCP for unassigned, no period filter on unassigned, no change to kit/kitperf/workspace filtering, no hide list.

## Edge Case Checklist

- [ ] `/tmp` symlink (`/private/tmp`): compare realpaths
- [ ] Gone path with an FR-057 owner → owner, never Scratch
- [ ] `~` and `~/.config/*` → Scratch
- [ ] Repo inside a hidden folder with no owner → Scratch
- [ ] Two scratch folders with the same name → one `folders` line, summed
- [ ] Relative or empty stored path → Scratch (not a crash)
- [ ] A workspace repo path with a trailing slash still covers its repo (`normalise_repo`)
- [ ] Planted secret/text fixtures never appear in any response or log

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/owners.py` | `project_map`, `SCRATCH`, scratch rule |
| `src/easy_personal/core/metrics.py` | `usage_breakdown` / `tokens_by_project` use `project_map`; Scratch `folders` |
| `src/easy_personal/core/workspaces.py` | `unassigned` |
| `src/easy_personal/surfaces/api.py` | `GET /api/workspaces/unassigned` (register before `/api/workspaces/{id}`) |
| `src/easy_personal/surfaces/mcp.py` | `usage_breakdown` description |
| `README.md` | Scratch group in the tools section |
| `tests/core/`, `tests/surfaces/`, `tests/leak/` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/migrations/`, `src/easy_personal/sources/` | read-time only, no store change |
| `src/easy_personal/core/kitperf.py`, `kit.py` | they keep FR-057 grouping |
| `web/` | UI is a separate task |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table
- [ ] UI Evidence rows ☐ N/A (backend only)
- [ ] Supervisor notified: task ready for Stage 4 review