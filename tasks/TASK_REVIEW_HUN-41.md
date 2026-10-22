# TASK_REVIEW — HUN-41: `context` read-only MCP client of easy-context + index freshness + `context_index_stale`

> Sibling of `tasks/TASK_GUIDE_HUN-41.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☒ pass / ☐ fail | `tests/sources/test_context.py` (22 tests: AC1 allow-list refuses `index_path` locally before any call; AC2 host guard refuses `localhost`, `0.0.0.0`, https, userinfo, `127.0.0.1.evil`; AC3 service down/slow (bounded timeout) → ingest completes, part `no data`; AC4 stale vs latest commit, skew, unjudgeable cases, rule fires/abstains; AC5 spike shape mirrored by `tests/fakes/mcp_server.py`); `tests/conftest.py` autouse dead-port URL so no test reaches the real service; expectation updates in `tests/core/test_rules.py`, `test_store.py`, `tests/sources/test_kit_trace.py`, `tests/surfaces/test_warnings.py`. Supervisor re-run 2026-10-22T04:09:41Z: `22 passed in 5.49s` |
| Verification command run | ☒ pass / ☐ fail | `uv run pytest tests/sources/test_context.py -q` → `22 passed in 5.49s`; `uv run ruff check src tests` → `All checks passed!` |
| Negative cases hold | ☒ pass / ☐ fail | live: service down (`EASY_PERSONAL_CONTEXT_URL=http://127.0.0.1:9/mcp`) → ingest completes in 425 ms, `context: {'status': 'no data'}`, kit part `no data`; `http://localhost:8767/mcp` → refused → `no data`; tests: `index_path` refused locally, non-loopback hosts refused, timeout bounded |
| verify | ☒ pass / ☐ fail / ☐ N/A | verify recipe 2026-10-22T04:09Z, isolated `uv tool install`, fixture transcript with `cwd` = this repo (indexed by easy-context): ingest against the REAL service (read-only `list_indexes` only) → `context: {'indexes': 1, 'status': 'ok'}`; `easy-personal kit` → `{"last_index_at": "2026-10-22T04:09:44.007000+00:00", "latest_commit_utc": "2026-10-22T04:08:16+00:00", "stale": false, "status": "ok"}`. The 04:09:44 stamp on all 4 indexes is easy-context's own 300 s timed refresh (`EASY_CONTEXT_REFRESH_SECONDS`, `jobs.py` refresh jobs), not caused by easy-personal (DDR-0002 holds). Service down / non-loopback → `no data` — feature confirmed working — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☒ pass / ☐ fail | `main...HUN-41`: `sources/context.py` (new), `core/kit.py` (+context part), `core/rules.py` (+metric), `core/store.py` (+table), `data/rules.yaml` (+rule), `surfaces/cli.py` (ingest step); callers: `kit_summary` (MCP/CLI), `rules.evaluate` (MCP/CLI/API warnings+plan), `cli._ingest` (ingest, dashboard, systemd timer). lane_check `lane: C2, 13 files, 538 lines, EXCEEDS (lines 538 > 500)` — 285 of them tests/fake server. code-review: 0 P0; 1 P1 process (startup reads: `agents/backend.md` read head only) closed by conformance check (no new deps, no memory writes, schema via the store's `connect()` like HUN-39, unpredicted files are required wiring); P2: `latest_commit` uses `--remotes` (same owner check as HUN-40); P3: worktree-keyed index not mapped to its main project; `from pathlib import Path` inside `stale_projects`; `except BaseException` re-raises only KeyboardInterrupt/SystemExit. migration-safety (Supervisor, skill run): GO — additive `CREATE TABLE IF NOT EXISTS context_index` via `store.connect`; dry run on a populated store copy: up (events 31, source_files 5 unchanged; +context_index), old `main` code reads the upgraded store (usage sessions 4, messages 3), down `DROP TABLE context_index` lossless (re-fetched next ingest), up again recreates it; no locks/backfill (≤1 row per project). security (inline, Medium): allow-list before network, exact `127.0.0.1` host check, no redirects followed (httpx default), only root + timestamp stored, `X-Easy-Context-Root` carries a local path to a local service only. |
| Full smoke suite still green (no regression) | ☒ pass / ☐ fail | `uv run pytest -q` → `223 passed, 2 skipped, 1 warning in 21.51s` (2026-10-22T04:09Z) |
| easy-verifier score (advisory) | ☒ recorded / ☐ N/A | 2026-10-22, MCP `score` repo=HUN-41 worktree, scope=changes, ref=`main...HUN-41`: overall **76** (architecture 100, code-quality 100, test-strategy 75, requirement-fidelity 67, blast-radius 65, reliability 64, security 59); abstained: solution-fit, supply-chain, api, data, operations, infrastructure. blast-radius = `changed_files_in_churn_hotspots_share` 0.36 (`kit.py`, `rules.py`, `store.py`); reliability = `queries_in_loops_observed` 1 in `tests/sources/test_context.py` (test code); security = `redaction_hits_observed` 118 (known false positive). Raw JSON /home/hungnguyenhuu/.claude-personal/projects/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/75d1f65a-8da1-4f4c-8245-0270482d2af1/tool-results/mcp-easy-verifier-score-1792642285038.txt. Advisory only. |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |

**Migration-safety checklist (done by hand, HUN-41 adds one table)**: additive only (`CREATE TABLE IF NOT EXISTS context_index (project_path PRIMARY KEY, last_index_at_utc)` in `core/store.py` SCHEMA, the project's migration mechanism; no ad-hoc SQL); no existing table altered; no DROP/RENAME/NOT NULL change/data rewrite; rollback = old code ignores the extra table (`DROP TABLE context_index` is lossless, rows are re-fetched at the next ingest); old and new code coexist (old code never reads it); no backfill, table is tiny (one row per project) so no lock concern; verified by `tests/core/test_store.py` table list and `tests/sources/test_context.py`. Verdict: GO.

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-10-22T04:03:42Z, main code, before any implementation commit):

```
$ uv run python -c "import easy_personal.sources.context"
ModuleNotFoundError: No module named 'easy_personal.sources.context'
$ grep -c context_index_stale src/easy_personal/data/rules.yaml
0
$ kit_summary parts for one project (python -c ...)
['kernel', 'git']
```

**AFTER**: `uv run pytest tests/sources/test_context.py -q` -> `22 passed`; `rules.yaml` has `context_index_stale`; `kit_summary` parts are `kernel`, `git`, `context`.

**DELTA**: The owner sees per project when easy-context last indexed it, and a `context_index_stale` hint (call `index_path`) when commits are newer, without easy-personal ever triggering indexing.

**WITNESS**: Supervisor — test re-runs, migration dry run and verify recipe against the real easy-context service 2026-10-22T04:07–04:10Z in the HUN-41 worktree (attributed to HUN-41 via the active_task state file; `memory/event-trace/HUN-41.jsonl`), independent of the implementing agent.
