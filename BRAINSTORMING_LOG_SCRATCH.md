# BRAINSTORMING_LOG.md
**Generated**: 2026-11-04
**Task / Context**: Workspace management — FR-069 (Scratch row), FR-070 (unassigned repos + Add to), US-029; PRD open question 24
**Skill**: `Skill({ skill: "brainstorming" })` — tier **Standard**

---

## The Problem Space

The store holds 181 distinct project paths; `usage --by project` shows 127 groups, but 11–12 real git repos carry 99.6 % of the cost. The rest are folders that agents and tests opened: ~40 `live-trace-*`, `/tmp/...` verify sandboxes, scratchpads (`/tmp/claude-1000/.../scratchpad/...`), gone worktrees that FR-057 cannot place (`wt-t195`, `wt-t148`, `_spike3-repo`), and non-repo folders (`~`, `~/.config/herdr`, `~/workspace/pets`, `~/workspace/pets/hungnguyen111`). Separately, only 2 of the 11 real repos sit in a workspace, so the workspace filter (FR-035) hides most work, and the Workspaces page shows no sign of it.

Non-negotiable: read-time only (never write or delete events); totals unchanged; FR-057 grouping runs first; no new write route; the Workspaces page must not wait on a slow read (HUN-113 lesson).

Verified facts (2026-11-04):
- `owners.owner_map(conn)` (`src/easy_personal/core/owners.py:87`) is the single FR-057 grouping; callers: `metrics.usage_breakdown` (`metrics.py:279`, only when `by == "project"`), `metrics.tokens_by_project` (`metrics.py:364`, feeds the `project_dominates` rule via `rules._top_project_share`), `kit.py:47`, `kitperf.py:214`, `workspaces.py:201`.
- Project groups are keyed by folder name (`_folder(...)`), not path.
- `web/src/Breakdown.jsx` renders project rows (`ProjectBars`); `web/src/Scope.jsx` has workspace + account selects only — no project picker.
- `PATCH /api/workspaces/{id}` (`surfaces/api.py:641`) edits a workspace's repos — the `Add to` write reuses it.
- Board/Kit/Memory already hide gone folders (FR-054, `kit.split_gone`).

---

## Questions for the User

None blocking: the owner chose the Scratch group, gone → Scratch, and the list + `Add to` button (PRD open question 24). The path below is the remaining choice.

---

## Alternative Paths

| Option | Name | Summary | Invasiveness | Code Volume | Regression Risk | Recommended? |
|--------|------|---------|-------------|------------|----------------|--------------|
| A | One grouping map | Widen FR-057's map to `project_map(conn)`: path → owner repo, or `SCRATCH`. Breakdown adds a `folders` list on the Scratch group. New read route for unassigned repos. | Low–Medium | ~250 lines + tests | Low | ✅ Yes |
| B | Separate scratch dimension | Keep `owner_map`; add `scratch` flag per group and a new `by=folder&scratch=1` query for opening Scratch. | Medium | ~350 lines | Medium | |
| C | Store-time kind | Ingest writes `project_kind` (repo / scratch) per path; reads group by it. | High (migration 8 → 9) | ~300 lines | Medium | |

### Option A — One grouping map
**Approach**: In `owners.py`, add `project_map(conn) -> dict[path, key]` built on `owner_map`: owner repo for FR-057 paths, `SCRATCH` for paths matching FR-069 (a)–(d), the path itself otherwise. `usage_breakdown(by="project")` and `tokens_by_project` switch to it. The Scratch group carries `folders: [{key, tokens…, cost}]` (sub-totals by folder name), so the UI opens it with no second request. FR-070: `core/workspaces.unassigned(conn, prices)` → real project paths with usage not covered by any workspace (reuse `_under`), with sessions, cost, last used; served at `GET /api/workspaces/unassigned`, fetched by `WorkspacesView` in parallel with the page.
**Pros**: one rule in one place; every caller of the old map that should see Scratch gets it; no migration; no extra request to open Scratch.
**Cons**: `usage_breakdown` MCP output gains a `Scratch` key and a `folders` field (README tool docs must say so); the top-project rule may name `Scratch` (only if scratch is > 60 % of tokens — today 0.4 %).
**Why it might fail**: git checks on every existing path could slow the first read (181 paths; (a) skips `/tmp` before any git call; the 300 s cache caps repeats). Kit/kitperf/workspaces must NOT switch to Scratch blindly (kitperf compares owner projects; workspaces filter by path) — the TASK_GUIDE pins which callers change.

### Option B — Separate scratch dimension
**Approach**: Breakdown groups keep folder names but get `scratch: true`; the UI collapses them; opening calls a new `by=folder` query.
**Pros**: MCP keys unchanged.
**Cons**: two places apply the rule (API + UI); CLI/MCP users still see 127 rows, which is the problem; one more query shape.
**Why it might fail**: the CLI/MCP lists stay noisy — fails the "every project list" part of FR-069.

### Option C — Store-time kind
**Approach**: classify at ingest and store it.
**Pros**: fastest reads.
**Cons**: a store migration; classification goes stale (a folder deleted later stays "repo"); FR-069 says read-time.
**Why it might fail**: breaks the read-time rule and needs a re-read of 180 k events.

---

## 50% Rule Check

Drop the `folders` sub-list and let Scratch be a closed row: saves ~40 lines in API + UI, but the owner asked that Scratch can be opened. Drop the separate unassigned route and compute it inside `GET /api/workspaces`: saves ~30 lines, but makes the Workspaces page wait on a cost query (HUN-113 regression). Kept both; cut everything else (below).

---

## Recommended Path

**Option A — One grouping map**

Scratch rule lives once in `owners.py`; the two token-by-project readers use it; the Workspaces page gets a parallel read for unassigned repos and writes through the existing PATCH.

### UI sketch

```
By project                                  tokens
orderly                    ███████████████  2.46 B
personal-agentic-claude    ██████████       1.59 B
...
▸ Scratch (116 folders)    ▏                 4.1 M     ← click to open
    live-trace-aqym18uj                      …
    t234-verify.qJWc                         …

Workspaces page
┌ Repos not in a workspace (9) ─────────────────────────────────┐
│ personal-agentic-claude  ~/workspace/pets/personal-agentic-…   │
│   130 sessions · $1,110 · last used 2026-11-03  [Add to ▾][Add]│
│ easy-verifier-mcp …                                             │
└────────────────────────────────────────────────────────────────┘
```

### Cut list (deliberately not built)
- No CLI or MCP command for unassigned repos (dashboard only, as FR-070 says).
- No "hide" button or owner-edited ignore list (owner chose the automatic rule).
- Session timeline / Live view keep raw folder names (not project lists).
- No bulk "add all" and no workspace guess from the folder.

---

## Edge Case Checklist (for every TASK_GUIDE)

1. `/tmp` symlink (macOS `/private/tmp`): compare `realpath` of both the path and `tempfile.gettempdir()`.
2. A real repo inside a hidden folder (`~/workspace/pets/.spawn/t182-live/repo`) → Scratch by rule (d), unless it has an FR-057 owner.
3. `~` (home) and `~/.config/*` are not repos → Scratch.
4. Gone path with an FR-057 owner → its owner, never Scratch (owner first, then rule).
5. Two scratch folders with the same name (`proj` ×N) → one line each in `folders`, summed by name like today's groups.
6. Totals: sum of all groups (Scratch included) = `total`, for every period.
7. `Add to`: PATCH sends the workspace's current repos + the new path (fetch the workspace fresh); a path already present is not duplicated; orphan never offered.
8. No workspace except orphan → hint, no control.
9. Unassigned list empty → "Every repo with usage is in a workspace."
10. Unassigned read fails or is slow → the rest of the Workspaces page still renders (inline error for that card only).
11. Top-project rule naming `Scratch`: allowed, wording unchanged.
12. MCP `usage_breakdown`: README drift test must match the new `Scratch` key / `folders` field.

---

## Surgical Scope

**Should touch**: `src/easy_personal/core/owners.py`, `src/easy_personal/core/metrics.py` (`usage_breakdown`, `tokens_by_project`), `src/easy_personal/core/workspaces.py` (`unassigned`), `src/easy_personal/surfaces/api.py` (one GET route), `src/easy_personal/surfaces/mcp.py` (tool description), `web/src/Breakdown.jsx`, `web/src/WorkspacesView.jsx`, `web/src/api.js`, `README.md`, tests.
**Must not touch**: the store schema / migrations, ingest (`sources/claude_code.py`), `kitperf.py` grouping, `workspaces.project_paths` filter semantics, connectors.

---

## Next Actions (Stage 2)

1. HUN-A (Backend, C2, Medium, P1): `owners.project_map` + Scratch group with `folders` in `usage_breakdown` / `tokens_by_project`; MCP description + README (FR-069).
2. HUN-B (Backend, C1, Low, P1): `workspaces.unassigned` + `GET /api/workspaces/unassigned` (FR-070 read side).
3. HUN-C (Frontend, C2, Medium, P1): Breakdown opens Scratch; Workspaces page "Repos not in a workspace" card + `Add to` via PATCH (FR-069 UI, FR-070 UI). After A and B.
