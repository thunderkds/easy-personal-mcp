# BRAINSTORMING_LOG.md
**Generated**: 2026-11-07
**Task / Context**: Kit view by workspace — easy-verifier scores (FR-087) and easy-ui use (FR-088); PRD open question 30
**Skill**: `Skill({ skill: "brainstorming" })` — tier **Standard** (auto mode: nothing asked; the Supervisor's answers are logged as Decision 30)

---

## The Problem Space

Owner, 2026-11-07: "for verifier mcp, collect and show the report by workspace to the dashboard" and "for ui mcp, do the same stuffs".

Today the Kit view lists projects, one verifier score each. A workspace filter exists (`Scope.jsx`), but the owner must pick one workspace at a time and read the project rows. There is no side-by-side view of all workspaces.

The two sources differ:
- **easy-verifier** writes per-repo score JSON; ingest keeps the newest overall score per project in `verifier_scores`. A repo maps to a workspace by path, so a rollup is possible.
- **easy-ui** writes session files with no project. Ingest keeps `id, status, startedAt, endedAt, action_count` in `ui_sessions` and never stores `target` (FR-013, NFR-002). A check cannot be put under a workspace without a guess.

Non-negotiable: read-only; no new file read; no HTML opened; no secret or caller text on the page (NFR-002); an abstained or missing score is "no score", never 0.

Verified facts (2026-11-07):
- `verifier_scores(project_path PK, status, scope, generated_at_utc, overall_value, overall_abstained, reason_code)` — `core/migrations/m0001_baseline.py:71`.
- `ui_sessions` holds five columns only — `sources/ui.py:12-41`; `ui_part` is overall (`sources/ui.py:ui_part`).
- `events.tool_names` is a comma list; `mcp__<server>__*` is matched by `GLOB` — `sources/context.py:195` (`in_use`), `core/kitperf.py:_tool_servers`.
- `events.session_id` and `events.project_path` exist — `m0001_baseline.py:21-40`.
- `workspaces.project_paths(conn, name, since, until)` resolves a workspace to stored project paths, path-aware, with FR-057 owners — `core/workspaces.py:175`.
- `kit.kit_summary` already takes `workspace` and `account`; route `GET /api/kit` — `surfaces/api.py:657`.
- `WorkspacesView.jsx` is being edited by HUN-207 at the time of writing.

---

## The Alternatives

### Path A — One new read, one new Kit block (recommended)
`kit.workspace_rollup(conn, since, until, account)` returns one row per non-orphan workspace: verifier (scored ÷ total, lowest + folder name, mean, newest time) from `verifier_scores`; easy-ui (sessions, calls, last call) from `events`; plus one overall `ui` entry as today. Route `GET /api/kit/workspaces`. A `WorkspaceKit` block in `KitView.jsx`.
- Pros: no migration; both sources in one read and one block; easy to test; no change to ingest.
- Cons: easy-ui check results (passed/failed) cannot be split — shown once as overall.
- 50 % less code: reuse `workspaces.project_paths` and one `GROUP BY` per workspace; no new table.

### Path B — Add a `workspace` field to each existing Kit project row
Group client-side: the page takes `GET /api/kit` and groups project rows by workspace.
- Pros: no new route.
- Cons: `GET /api/kit` has no workspace per row; the page would need `GET /api/workspaces` and path matching in JS (duplicates FR-035 logic); easy-ui events are not in the Kit rows.
- 50 % less code: same as A, so no gain.

### Path C — Split easy-ui checks by workspace (time overlap or hash of `target`)
Attribute each `ui_sessions` row to the workspace of the Claude session that overlapped it, or store a hash of `target`.
- Pros: shows passed/failed per workspace.
- Cons: overlap is a guess (two workspaces open at once); a `target` hash changes FR-013 and NFR-002 and needs a migration.
- Verdict: rejected in auto mode as the unsafe option (Decision 30b).

---

## Adversarial Review

| Path | Why it might fail |
|---|---|
| A | A repo in two workspaces counts in both rows (allowed: a repo may be listed twice, as in the usage filter). Workspace with 40 repos and 1 score reads as "1 of 40" — correct but may look bad: show the count, not a verdict. Gone folders: `kit_summary` hides them; the rollup must also skip repos whose folder is gone, or the denominator is wrong. |
| B | Duplicates path matching in JS; drifts from `project_paths`. |
| C | A wrong number the owner trusts. |

## Edge Case Checklist

- [ ] Workspace with no repos → "no data" row, not an error
- [ ] Orphan workspace is left out
- [ ] Repo in two workspaces → shown in both
- [ ] Verifier `status = unknown` (other schema) or `overall_abstained = 1` → counts as "no score", never 0
- [ ] Mean uses scored repos only; lowest shows its folder name only (no full path)
- [ ] Gone folders (FR-054) skipped in the repo count
- [ ] Two repos with the same folder name → both listed by folder name, no merge
- [ ] easy-ui: a session with several calls counts once in "sessions", all calls in "calls"
- [ ] easy-ui: subagent/sidechain events count (same as FR-060)
- [ ] Period and account scope follow the page, as other Kit parts do
- [ ] A planted secret in `target` / report fields never appears (NFR-002)
- [ ] Reads in < 2 s on 160 projects (NFR-013); memo by `data_version` if needed (HUN-227/228 pattern)

## Surgical Scope

Should touch: `src/easy_personal/core/kit.py` (+ a small helper), `src/easy_personal/surfaces/api.py` (one route), `web/src/KitView.jsx`, `web/src/app.css`, tests, `tests/leak/harness.py` (`API_CALLS`), README/site only if a drift test requires it.
Must not touch: `sources/verifier.py`, `sources/ui.py`, migrations, `WorkspacesView.jsx` (HUN-207), MCP tools and CLI (dashboard only), `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/`.

## Questions for the User

None blocking (auto mode). Open for owner review: Decision 30 (a) Kit view as the place; (b) easy-ui checks stay overall; (c) mean beside lowest.

## Recommended Path

**Path A**, as two tasks so the backend can finish before the page:
1. Backend read + route (`workspace_rollup`, FR-087/FR-088 data) — Backend-Implementer, C2, Medium.
2. Kit view block — Frontend-Implementer, C2, Medium; depends on 1.
