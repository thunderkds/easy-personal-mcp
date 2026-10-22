# TASK_REVIEW — HUN-40: `git_local` source + `kit_summary` git part

> Sibling of `tasks/TASK_GUIDE_HUN-40.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☒ pass / ☐ fail | `tests/sources/test_git_local.py` (7 tests: AC1 per-day counts vs `git log` on a temp repo, all branches, worktree counted once, shallow clone; AC2 `subprocess.run` spy — only `log`, every call has `core.fsmonitor=false`, `GIT_OPTIONAL_LOCKS=0` and a timeout; AC3 non-repo / missing folder / no commits → `no data`; timeout → `abstained`; commit-message canary absent from `kit_summary`); `tests/sources/test_kit_trace.py` expected parts updated. Supervisor re-run 2026-10-22T04:00:30Z: `7 passed in 0.10s` |
| Verification command run | ☒ pass / ☐ fail | `uv run pytest tests/sources/test_git_local.py -q` → `7 passed in 0.10s`; `uv run ruff check src tests` → `All checks passed!` |
| Negative cases hold | ☒ pass / ☐ fail | non-git folder → `{"status": "no data"}` (live); git timeout → `{"status": "abstained"}` (test); no network subcommand (spy test); canary in commit messages count 0 in `easy-personal kit` |
| verify | ☒ pass / ☐ fail / ☐ N/A | verify recipe 2026-10-22T04:00Z, isolated `uv tool install`, fixture transcripts with `cwd` rewritten to a temp git repo (3 commits: 2 on 2026-10-20, 1 on 2026-10-21, message `CANARY_HUN40_msg`) and a non-git folder: `easy-personal kit` → `demo-app {"commits": 3, "commits_by_day": {"2026-10-20": 2, "2026-10-21": 1}, "status": "ok"}` = `git log` local-day counts; `notgit {"status": "no data"}`; canary count 0; `kit` 372 ms end to end; `git_part` on this repo (103 commits) 47 ms — feature confirmed working — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☒ pass / ☐ fail | `main...HUN-40`: `sources/git_local.py` (new), `core/discovery.py` (`run_git` hardened, `_git` wraps it → `resolve_project` callers covered by existing ingest tests), `core/kit.py` (+`git` part); callers of `kit_summary`: MCP `kit_summary`, CLI `kit`. lane_check `lane: C1, 5 files, 201 lines, FITS; unpredicted: src/easy_personal/core/discovery.py, tests/...` (discovery.py = the guide's HUN-34 carry-over). code-review: 0 P0/P1; P2 (conf 75): `--remotes` counts remote-tracking refs, so fetched commits by others count as this owner's "local git activity" (FR-014 intent) — owner decision, recorded; P3: `commits_by_day` is all-time, unbounded by period (response grows with repo age). Startup reads complete. security (inline, Low): argv list (no shell), fixed subcommand `log`, hardened env/config, timeout, reads commit times only. |
| Full smoke suite still green (no regression) | ☒ pass / ☐ fail | `uv run pytest -q` → `201 passed, 2 skipped, 1 warning in 16.07s` (2026-10-22T04:00Z) |
| easy-verifier score (advisory) | ☒ recorded / ☐ N/A | 2026-10-22, MCP `score` repo=HUN-40 worktree, scope=changes, ref=`main...HUN-40`: overall **81** (architecture 100, blast-radius 100, reliability 100, code-quality 87, requirement-fidelity 67, security 59, test-strategy 55); abstained: solution-fit, supply-chain, api, data, operations, infrastructure. test-strategy = `source_files_without_covering_test_share` 1 (engine's test pairing unconfirmed; `tests/sources/test_git_local.py` covers `git_local.py`); security = `redaction_hits_observed` 118 (known false positive). Raw JSON /home/hungnguyenhuu/.claude-personal/projects/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/75d1f65a-8da1-4f4c-8245-0270482d2af1/tool-results/mcp-easy-verifier-score-1792641656922.txt. Advisory only. |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (2026-10-22T03:57:42Z, `kit_summary` on an in-memory store with one known project, before any change):

```
{"projects": [{"project": "easy-personal-mcp", "missing_on_disk": true, "parts": {"kernel": {"status": "no data"}}}]}
```

No `git` part under `parts`.

**AFTER**: `kit_summary` now returns `parts.git` = `{"status": "ok", "commits": 2, "commits_by_day": {...}}` for a git repo and `{"status": "no data"}` otherwise (tests/sources/test_git_local.py::test_kit_summary_git_part).

**DELTA**: The owner can now see each project's local commits per day in `kit_summary` (MCP and `easy-personal kit`), with "no data" for non-git folders; before, kit_summary showed only the kernel part.

**WITNESS**: Supervisor — test re-runs and verify recipe 2026-10-22T04:00Z in the HUN-40 worktree (attributed to HUN-40 via the active_task state file; `memory/event-trace/HUN-40.jsonl`), independent of the implementing agent.
