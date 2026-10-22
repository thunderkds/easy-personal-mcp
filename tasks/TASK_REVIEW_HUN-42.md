# TASK_REVIEW — HUN-42: verifier source + kit_summary part

> Sibling of `tasks/TASK_GUIDE_HUN-42.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/sources/test_verifier.py` (33 tests: AC1 fixture JSON stored per project/time; AC2 HTML-only → no data; AC3 unknown schema_version → abstained incl. v1-older/v2-newer — Supervisor confirmed 3 tests FAIL on 3802a3c and PASS on db12d8c; malformed/oversized/symlink/FIFO skipped; worktree folder names ignored; source files unchanged), leak suite canary in `evidence`, `reason_code`, HTML. `uv run pytest -q` → `370 passed, 2 skipped, 1 warning in 27.11s` |
| Verification command run | ☑ pass | `uv run pytest -q` 370 passed, 2 skipped; `uv run ruff check .` → `All checks passed!` |
| Negative cases hold | ☑ pass | v2 file with `INVENTED_V2` → abstained, canary absent; HTML with `INVENTED_HTML` never read; decoy score 99 in `container/proj-a-HUN-9/` ignored; `grep -c INVENTED_` on ingest + kit output = 0; real source JSON sha1 unchanged |
| verify | ☑ pass | Isolated install of db12d8c, temp projects proj-a/proj-b (git repos, fixture transcripts with cwd rewritten), real T120 file `easy-verifier-mcp/reports/evidence-report-project-20261021T124041-825600Z.json` in proj-a/reports, v2 file in container/proj-b: ingest `{'projects': 2, 'skipped': 0, 'status': 'ok'}`; kit: proj-a `{status: ok, scope: project, generated_at: 2026-10-21T12:40:41.000000Z, overall: {value: 48, abstained: False, reason_code: None}}`, proj-b `{status: abstained, reason: unknown_schema_version}` — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | main...HEAD + `kit.kit_summary`, `cli._ingest`, `config`. lane_check: `lane: C1, 13 files, 417 lines, EXCEEDS (files 13 > 6; lines 417 > 250)` — leak-suite + config + store were necessary; label under-sized. code-review: P1 1 fixed (AC3: unknown schema skipped instead of abstained, stale v1 shown; db12d8c). P1 process: implementer first read `agents/backend.md` lines 1–40 only (recurring); re-read in full on follow-up; self-reported gaps: tests written with code (not test-first), MEMORY.md/general template not read — Supervisor ran the fail-before check. P3: `_projects`/`_label`/`_utc` imported as private helpers from sibling sources. security-review (inline, Medium): read-only, O_NOFOLLOW + regular-file + parent-dir checks, 1 MB cap, JSON depth guarded, no HTML parse, no easy-verifier call, no prose stored — no findings. migration-safety: GO — one additive `CREATE TABLE IF NOT EXISTS verifier_scores`, no existing table altered, rebuilt each ingest (delete+insert in one transaction), old code ignores it. |
| Full smoke suite still green (no regression) | ☑ pass | 370 passed, 2 skipped |
| easy-verifier score (advisory) | ☑ recorded | scope=changes: overall `rating_abstention no_dimension_rated`; 13 dimensions + change-risk abstain |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Backend-only task; no UI component |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Backend-only task; no UI component |
| **UI: Responsiveness at target viewports** | ☑ N/A | Backend-only task; no UI component |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-22T08:32:09Z (`date -u`) on branch HUN-42 before any implementation commit. Temp home, a temp project with `reports/evidence-report-project-20261021T124041-825600Z.json` (schema_version 1, overall 48) plus an HTML-only report; ingest of claude_code + kit_trace, then `kit_summary` parts:

```
no verifier source          (import of easy_personal.sources.verifier fails)
{"kernel": {"status": "no data"}, "git": {"status": "no data"}, "context": {"status": "no data"}}
```
No `verifier` part exists.

**AFTER**: captured 2026-10-22T08:34:57Z, same fixture and script: ingest `{'status': 'ok', 'projects': 1, 'skipped': 0}`; parts now include
`"verifier": {"status": "ok", "scope": "project", "generated_at": "2026-10-21T12:40:41.000000Z", "overall": {"value": 48, "abstained": false, "reason_code": null}}` (the HTML-only report was ignored).

**DELTA**: `kit_summary` now shows each project's newest easy-verifier overall score (or abstained for an unknown schema), which before was missing entirely.

**WITNESS**: Supervisor, 2026-10-22 08:37 UTC, isolated install from the HUN-42 worktree with a real T120 JSON file (trace attributed to HUN-42 via the active_task state file).
