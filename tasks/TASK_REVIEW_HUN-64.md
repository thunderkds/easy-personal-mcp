# TASK_REVIEW — HUN-64: Workspaces CRUD, repos, orphan, usage filter

> Sibling of `tasks/TASK_GUIDE_HUN-64.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | pass (implementer) | tests/core/test_workspaces_schema.py (AC1, AC2, AC3 raw SQL constraints; v2->v4 upgrade; gap + m0003-after-v4 limitation), tests/core/test_workspaces.py (AC1-5 CRUD, filter incl. `/a/b` vs `/a/bc`, zero repos, perf 1M events), tests/surfaces/test_workspace_cli.py (AC1-5), tests/surfaces/test_api_workspaces.py (AC5, AC7 guard), tests/leak/ (registration + canary scan). AC6 (workspace AND account) NOT testable on this branch: no account filter exists until HUN-63; `metrics._scope` returns an AND-able clause for it. |
| Verification command run | pass (implementer, 2026-10-26) | ```
-- Docs: https://docs.pytest.org/en/stable/how-to/capture-warnings.html
235 passed, 2 deselected, 1 warning in 14.85s
perf usage_summary 30d over 1M events: [236, 231, 232] ms
.perf workspace usage_summary 30d over 1M events: [225, 221, 220] ms
2 passed, 531 deselected, 1 warning in 20.34s
-- Docs: https://docs.pytest.org/en/stable/how-to/capture-warnings.html
529 passed, 2 skipped, 2 deselected, 1 warning in 30.41s
```
`uv run ruff check .` -> All checks passed! |
| Negative cases hold | pass (implementer) | orphan add/rename/edit/delete refused (CLI exit 2, API 403/400, trigger via raw SQL); duplicate/rename-to-existing 409 with nothing changed; bad slug/colour/relative repo refused; errors do not echo input; guard: no token/wrong Host/foreign Origin 403, non-JSON 415, bad JSON 400 |
| verify | ☑ pass | verify by Supervisor 2026-10-26T09:27:14Z: isolated install + fixtures → ingest; `workspace add company --repo /work/demo-app` ok; `workspace add orphan` → reserved, exit 2; rename company→acme ok; usage all 3 messages, --workspace acme 3 (fixture has one project), --workspace empty 0; delete --yes ok; list shows orphan; user_version 4, integrity ok — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed: m0004 (constraints + triggers), `core/workspaces.py` (normalise, CRUD, `project_paths`), `metrics._scope` + 4 usage functions, cli/api routes, leak registration, tests. code-review: P0 0 / P1 0 / P2 1 deferred (AC6 workspace AND account filter cannot be tested before HUN-63 — moved to HUN-63 guide AC 8) / P3 2 (`resolve_project` runs git per repo on each filtered query — few repos, accepted; `session_contexts`/rules not workspace-scoped — out of FR-035 scope). Known limitation noted: runner skips a migration numbered below the store's `user_version` — merge order HUN-62 (m0003) before HUN-64 (m0004) on main; no store ran HUN-64 alone outside tests. security-review (inline): params bound (`IN (?,…)`), names validated by regex + CHECK, errors never echo input, write routes guarded; no findings. migration-safety: GO — additive; dry run on .backup copy of owner store v2→v4: events 575 / kit 11564 / tool_signals 17 equal, fk_check empty, integrity ok, 0.05 s, bak-v2. |
| Full smoke suite still green (no regression) | pass | `uv run pytest -q`: 529 passed, 2 skipped, 2 deselected (see above) |
| easy-verifier score (advisory) | ☑ N/A | N/A — easy-verifier MCP is registered only in the `~/.claude-personal` profile; this Supervisor session runs another profile, tool not loaded |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Pure backend task; no UI file changed |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Pure backend task; no UI file changed |
| **UI: Responsiveness at target viewports** | ☑ N/A | Pure backend task; no UI file changed |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (2026-10-26, temp `EASY_PERSONAL_HOME`, before any implementation commit):

```
Mon Oct 26 09:14:55 AM UTC 2026
$ uv run easy-personal workspace list
Using CPython 3.13.2
Creating virtual environment at: .venv
   Building easy-personal-mcp @ file:///home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-64
      Built easy-personal-mcp @ file:///home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-64
Installed 43 packages in 4ms
usage: easy-personal [-h] [--version]
                     {ping,mcp,ingest,schedule,usage,outliers,warnings,kit,plan,dashboard,exec,headers,connector,doctor} ...
easy-personal: error: argument command: invalid choice: 'workspace' (choose from ping, mcp, ingest, schedule, usage, outliers, warnings, kit, plan, dashboard, exec, headers, connector, doctor)
$ uv run easy-personal usage --workspace x
usage: easy-personal [-h] [--version]
                     {ping,mcp,ingest,schedule,usage,outliers,warnings,kit,plan,dashboard,exec,headers,connector,doctor} ...
easy-personal: error: unrecognized arguments: --workspace x
```

**AFTER** (same temp-home setup, `tests/fixtures/claude_code`):

```
Mon Oct 26 09:25:05 AM UTC 2026
$ easy-personal workspace add demo --repo /work/demo-app --repo /work/other
{
  "color": null,
  "description": null,
  "email": null,
  "icon": null,
  "id": 2,
  "is_orphan": false,
  "name": "demo",
  "repos": [
    "/work/demo-app",
    "/work/other"
  ]
}

(exit 0)
$ easy-personal workspace add orphan
easy-personal: the name 'orphan' is reserved

(exit 2)
$ easy-personal workspace edit demo --rename company
{
  "color": null,
  "description": null,
  "email": null,
  "icon": null,
  "id": 2,
  "is_orphan": false,
  "name": "company",
  "repos": [
    "/work/demo-app",
    "/work/other"
  ]
}

(exit 0)
$ easy-personal workspace list
{
  "workspaces": [
    {
      "color": null,
      "description": null,
      "email": null,
      "icon": null,
      "id": 1,
      "is_orphan": true,
      "name": "orphan",
      "repos": []
    },
    {
      "color": null,
      "description": null,
      "email": null,
      "icon": null,
      "id": 2,
      "is_orphan": false,
      "name": "company",
      "repos": [
        "/work/demo-app",
        "/work/other"
      ]
    }
  ]
}

(exit 0)
$ easy-personal usage --workspace company
{
  "abstained": {
    "by_version": {},
    "total": 0
  },
  "cache_creation_input_tokens": 250,
  "cache_hit_ratio": 0.7905,
  "cache_read_input_tokens": 1000,
  "input_tokens": 15,
  "messages": 3,
  "output_tokens": 8,
  "period": {
    "since_utc": null,
    "until_utc": null
  },
  "sessions": 3
}

(exit 0)
$ easy-personal usage --workspace x
easy-personal: unknown workspace

(exit 2)
$ easy-personal usage
{
  "abstained": {
    "by_version": {
      "2.1.293": 1,
      "2.1.295": 1,
      "2.1.302": 1
    },
    "total": 3
  },
  "cache_creation_input_tokens": 250,
  "cache_hit_ratio": 0.7905,
  "cache_read_input_tokens": 1000,
  "input_tokens": 15,
  "messages": 3,
  "output_tokens": 8,
  "period": {
    "since_utc": null,
    "until_utc": null
  },
  "sessions": 4
}

(exit 0)
```

**DELTA**: The owner can create, list, rename, edit and delete workspaces (repo paths, colour, email) from the CLI and the guarded dashboard API, and filter usage to one workspace's repos with `--workspace` / `workspace=`.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]

## Migration safety (self-applied checklist; no Skill tool in this run)

- Additive only: two new tables, three triggers, one partial unique index, one seed row. No existing table, column or index touched.
- Idempotent: every statement `IF NOT EXISTS` / `INSERT OR IGNORE` (like m0001), so a store whose `user_version` was reset still upgrades.
- One transaction with the `user_version` bump (runner); the runner copies a non-empty store to `store.sqlite.bak-v2` first. Restore = put the `.bak-v2` file back.
- Upgrade test with data: `test_v2_store_upgrades_to_v4_keeping_every_row` (events count unchanged, backup has the rows and `user_version` 2, `orphan` seeded).
- Gap: runner sorts by number and sets `user_version` to each number, so m0001, m0002, m0004 works (`test_runner_tolerates_a_gap_in_numbers`).
- KNOWN LIMITATION (reported, runner NOT changed): a store already at v4 never runs a later-merged m0003, because the runner compares each migration number only with `user_version` (`test_known_limitation_m0003_arriving_after_v4_is_skipped`). Safe when both migrations merge before any real store upgrades; unsafe for a store opened on this branch alone.
- Existing test edited for the new tables: `tests/core/test_store.py` table list.
- Rollback: no destructive step; dropping the two tables and triggers and setting `user_version` back to 2 reverses it.
