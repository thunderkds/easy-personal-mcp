# BRAINSTORMING_LOG_WORKVIEW.md
**Generated**: 2026-10-31
**Task / Context**: Work view feature set — PRD US-017..US-021, FR-037..FR-042, NFR-010; decision log 2026-10-31 ("Work view inspired by Javis OS, read-only")
**Skill**: `Skill({ skill: "brainstorming" })` — tier **Standard**

---

## The Problem Space

The owner liked how Javis OS (github.com/blogminhquy/javis-os, ideas only) manages work: a task board,
background-job visibility, a knowledge graph, grouped navigation. easy-personal keeps its own vision —
local, read-only, metadata-only, never starts agents. The job is to bring the *display* side of those
ideas into the existing dashboard without new write paths and without storing note content.

Non-negotiables: NFR-001 (no off-machine calls), NFR-002 (no content/args), NFR-003 (read-only
sources), FR-037 (no board API), FR-041 (graph never stored, no bodies), NFR-010 (< 1 s, 50 projects).

Verified codebase facts (claim gate):
- Rail is already grouped: `GROUPS` in `web/src/App.jsx:12` (`// insights`, `// setup`). FR-042 = regroup + checklist.
- API routes live in `src/easy_personal/surfaces/api.py:504-528`, each a thin `_query(lambda c, q: core.fn(...))`.
- Project list for scoping: `kit.scoped_projects()` (`src/easy_personal/core/kit.py:22`) — reuse for board and graph.
- `is_sidechain` is stored (`core/store.py:24`, `sources/claude_code.py:147`); `entrypoint` is not.
- Our own timer is written by `core/schedule.py` (`easy-personal-ingest.timer`) — the schedules view must show it.
- `web/package.json` has only `react` + `react-dom`; all charts are hand-drawn SVG.
- The kit's Linear pull writes into `PROJECT_KANBAN.md` itself (learnings: "Board pull overwrites `## Blocked`").

---

## Layout Sketches (for owner review)

### Work › Board
```
┌ Work › Board ────────────────────────── scope: [workspace ▾] [project ▾] ─┐
│ 4 projects · 2 on Linear (last pull 2h ago) · 1 no data                  │
├──────────────┬──────────────┬──────────────────┬──────────────┬─────────┤
│ Todo  (6)    │ In Progress(2)│ Ready for Review(1)│ Done (41) ▸ │Blocked(1)│
├──────────────┼──────────────┼──────────────────┼──────────────┼─────────┤
│┌────────────┐│┌────────────┐│┌────────────────┐ │ collapsed;   │┌───────┐│
││HUN-140     │││HUN-133     │││HUN-131         │ │ shows last 5 ││HUN-90 ││
││Work board  │││Neon pass   │││Landing family  │ │ on expand    ││XR-1   ││
││● easy-pers │││● easy-ctx  │││● easy-pers     │ │              │└───────┘│
│└────────────┘│└────────────┘│└────────────────┘ │              │         │
└──────────────┴──────────────┴──────────────────┴──────────────┴─────────┘
  card = id · title · project dot (workspace colour) · "Linear" tag if synced
```

### Work › Scheduled
```
┌ Work › Scheduled ─────────────────────────────────────────────────────────┐
│ Job                         Kind     Schedule     Next     Last   Result │
│ easy-personal-ingest.timer  systemd  hourly       in 48m   12m ago  ✓ 0 │
│ (crontab) claude            cron     0 7 * * 1-5  Mon 7:00 Fri     ✗ 1 │
│ 1 unrelated timer hidden · /loop jobs and cloud routines not included    │
└───────────────────────────────────────────────────────────────────────────┘
```

### Work › Memory graph
```
┌ Work › Memory ─────────────── project: [easy-personal-mcp ▾] [all] ───────┐
│                                                ┌ selected ──────────────┐ │
│        (decisions)───(glossary)                │ local-merge-push       │ │
│          │      \                              │ "No per-task PR; ..."  │ │
│   (MEMORY)──(learnings)───(LR-0004)            │ memory/decisions.md    │ │
│          \                                     │ links in 3 · out 2     │ │
│        ((feedback-tone))   ◌ dangling-name     └────────────────────────┘ │
│  ● kit memory  ● Claude memory  ◌ unresolved [[link]]  · 38 notes 61 links│
└───────────────────────────────────────────────────────────────────────────┘
  panel shows name + description line + path only (FR-041), never the body
```

### Rail
```
// work      Board · Scheduled · Memory
// usage     Usage · Warnings & plan
// kit       Kit
// connect   Workspaces · Agents
// system    First run  (badge "2 left" until the checklist is done)
```

---

## Questions for the User

1. Do the sketches above match what you liked in Javis OS? (Board columns = the kit's own status names.)
2. Path choice below — A, B or C.

---

## Alternative Paths

| Option | Name | Summary | Invasiveness | Code Volume | Regression Risk | Recommended? |
|--------|------|---------|-------------|------------|----------------|--------------|
| A | Live-read modules | 3 new core readers (board, schedules, memgraph) read on request; 1 migration for `entrypoint`; 4 views + rail regroup | Low | ~1,400 lines (incl. tests) | Low | ✅ Yes |
| B | Snapshot store | Ingest board + schedules into the store each run (history: time-in-column, job miss streaks); graph still live | Medium | ~2,300 lines | Medium | |
| C | Minimalist | Board + usage split only; schedules = a card inside Kit; graph = a table of notes with broken-link counts, no drawing | Low | ~700 lines | Low | |

### Option A — Live-read modules
**Approach**: `core/board.py` (parse `PROJECT_KANBAN.md` by `## Board` subsections + `## Blocked`), `core/jobs.py`
(`systemctl --user list-timers --all --output=json` + `systemctl --user show -p ExecStart,ExecMainStatus`, `crontab -l`, allow-list on first word),
`core/memgraph.py` (frontmatter + `[[...]]` regex, per project). Routes `/api/board`, `/api/jobs`, `/api/memory-graph`
in `api.py`; MCP stays unchanged except the usage split fields. Migration `m000N_entrypoint` adds the column and
resets file offsets so the next ingest backfills. Graph drawn by a small deterministic force layout in SVG (fixed seed, ~150 lines).
**Pros**: matches FR-037/FR-041 "live, never stored" exactly; no new dependency; each reader is independent → parallel tasks.
**Cons**: no history (can't show "stuck in review 5 days"); graph layout code is ours to maintain.
**Why it might fail**: kanban files drift in format across repos (hand edits, older kit versions) → parser must abstain per file, not crash;
`systemctl --user` absent (container, no user bus) → "no data"; 50 repos × `memory/**` on a slow disk could miss 1 s → measure first, cache only if needed.

### Option B — Snapshot store
**Approach**: A's readers become sources run by ingest; tables `board_snapshot`, `job_run`. Adds trends: days in column, missed runs.
**Pros**: history and "stuck task" warnings become possible; dashboard reads are pure SQL (fast).
**Cons**: PRD said board is read live; snapshots are only as fresh as the last ingest (hourly); two more migrations; scope beyond what was asked (Simplicity First).
**Why it might fail**: task titles in the store are repo-authored text — closer to the "content" line NFR-002 draws; owner never asked for history.

### Option C — Minimalist
**Approach**: Ship FR-037/038 board + FR-039 split; jobs as a card in KitView; graph as a sortable table (note, kind, links in/out, dangling).
**Pros**: half the code; no layout algorithm.
**Cons**: drops the visual graph and the Work section the owner explicitly chose; table does not show clusters.
**Why it might fail**: owner picked the graph for the picture — a table would likely be sent back at review.

---

## 50% Rule Check

For Option A: drop the custom force layout and lay the graph out deterministically by kind in rings
(MEMORY.md at centre, cold files ring 1, LRs/Claude notes ring 2) — ~60 lines instead of ~150, stable
between reloads, testable as golden SVG. Skip the separate Scheduled view by putting jobs in a Work-section
card on the Board page. Together this cuts ~35%; the rings layout is adopted, the merged page is not (owner chose a jobs view).

---

## Recommended Path

**Option A — Live-read modules, with the rings layout for the graph.** Selected by owner 2026-10-31 (mockups accepted).

## Edge Case Checklist (copy into each TASK_GUIDE)

- Kanban: missing file; no `## Board`; unknown subsection names (count as abstained, show); duplicate task ids across repos (key = project + id); tables vs bullet lists; titles with `|` or markdown links.
- Board scope: deleted project folder (FR-009 *missing on disk*) → "no data", not an error; worktree folders must not double-count.
- Usage split: session whose first record has no `entrypoint`; sidechain records inside a headless session (still agent, count once); old rows during backfill → "unknown".
- Jobs: no user systemd bus; timer unit with no `ExecStart`; crontab with env lines and `@reboot`; command path `/home/x/.local/bin/claude` → first word basename; never show arguments (they may hold secrets).
- Graph: notes without frontmatter (name = file stem); `[[link|alias]]` and `[[link#anchor]]`; self-links; cycles; 1,000+ notes in one account; symlinked memory dirs (resolve once, avoid loops); non-UTF-8 files → abstain.
- Perf: NFR-010 measured on 50 fixture projects in a test, not assumed.

## Surgical Scope

**Touch**: `src/easy_personal/core/{board,jobs,memgraph}.py` (new), `core/metrics.py` (split), `core/migrations/` (new step), `sources/claude_code.py` (store entrypoint), `surfaces/api.py` (3 routes), `surfaces/mcp.py` (split fields only), `web/src/App.jsx` (GROUPS), new `web/src/{BoardView,JobsView,MemoryView,FirstRun}.jsx` + tests, `web/src/UsageView.jsx` (split series), `tests/`.
**Must not touch**: `connectors/`, `launch.py`, keychain code, `core/workspaces.py` write paths, any repo's `PROJECT_KANBAN.md` or memory files (read-only), `scripts/board_sync.py`.

## Next Actions (Stage 2)

1. Stage 2 `/plan`: `PROJECT_SPEC.md` glossary — *Work view*, *agent work*, *kit memory* / *Claude memory*, *dangling link*.
2. Split into ~6 tasks: entrypoint migration + split (C2, Medium — migration-safety); board reader + view; jobs reader + view; memory graph reader + view; rail regroup + first-run checklist; Work view perf test (NFR-010). UI tasks need the three design Evidence rows.
