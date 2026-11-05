# TASK_REVIEW — HUN-227: Share repeated store computations across dashboard reads (NFR-013 server)

> Sibling of `tasks/TASK_GUIDE_HUN-227.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Extended track — RULE: `AGENTS.md` § Tracks ("The extended track applies if any of: C2 or above; Risk Medium or High")
2. Risk Medium (not Low): the change wraps functions in `metrics.py` and `owners.py`, used by most reads — RULE: `docs/claude-md/pipeline-stages.md:137` "A hub touch raises Risk a level even when the edit itself is small"
3. Fix path: a single-flight memo in core for `_messages` and `_stored_branches`, keyed by `PRAGMA data_version` on a watcher connection, not by `MAX(events.id)` — DEBATED: option A (Advocate 1: A, shared computations in core, with a key that catches updates, deletes and scope-table edits; Advocate 2: B, whole-response cache in `api.py`, which conceded B alone cannot meet NFR-013 on an active store) ⚠ decided for you — please review
4. Brainstorm direction taken without owner approval — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (row 4: "Take the recommended direction and list the alternatives in the report"); alternatives B and C in `BRAINSTORMING_LOG_PERF.md`
5. Requirement Fidelity Gate signed `auto (Supervisor)` — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (row 13)
6. Spawn model `claude-sonnet-5-5` — FACT: `PROJECT_SPEC.md:165` "| claude | `claude` | `claude-haiku-5-5` | `claude-sonnet-5-5` | `claude-sonnet-5-5` | `claude-opus-5-5` |"
7. One-time setup (`fewer-permission-prompts`, `update-config`) skipped — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (row 5)
Checks: (filled at Stage 4)
Not done: merge (yours); auto verify: running (T182)


## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_memo.py` (25 tests; AC 1-6). `uv run pytest tests/core/test_memo.py -q` -> `25 passed in 0.96s`. Mutation check: with the data version pinned to a constant, 11 of the 25 fail (the never-stale, bound and fallback tests). |
| Verification command run | ☑ pass | `uv run pytest tests -q -k "memo or metrics or owners or usage or api or leak"` -> `321 passed, 1225 deselected, 1 warning in 142.76s`; `uv run pytest -q` -> `1537 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 236.94s` (AC 7: no existing test or golden edited). |
| Negative cases hold | ☑ pass | `test_unknown_workspace_and_account_raise_as_before` (ValueError / AccountError, error never cached); `test_a_failing_computation_reaches_the_waiters_and_caches_nothing`; `test_watcher_cannot_be_opened` / `test_watcher_cannot_be_read` (no exception, no cache); `test_uncommitted_writes_of_the_readers_connection_are_never_cached`; `test_a_store_recreated_at_the_same_path_is_not_served_stale`; `test_an_evicted_store_is_not_served_stale`; in-memory store computed without cache. |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` -> `1537 passed, 2 skipped, 6 deselected, 1 xfailed` (2026-11-05, after the last code edit; `ruff check` clean on the 4 files). |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | backend only. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | backend only. |
| **UI: Responsiveness at target viewports** | ☑ N/A | backend only. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: page-load timing (AC 8) on the branch before any change (identical to `main` code, HEAD 3eca522), copy of the owner store (`sqlite3 ~/.easy-personal/store.sqlite ".backup <dir>/store.sqlite"`), `EASY_PERSONAL_HOME=<dir>`, 3 runs, each run = cold page + warm page; top 6 reads shown. Captured 2026-11-05T08:41Z (UTC), load average ~3.

```
 15:41:13 up  9:25,  1 user,  load average: 3.20, 2.53, 2.48
=== BEFORE run 1 2026-11-05T08:41:13Z
cold: page wall 6.23s
     6223 ms 200 /api/usage/sessions?since=2026-10-29T08:41:16.433358Z
     5885 ms 200 /api/usage/breakdown?by=project&since=2026-10-29T08:41:16.43
     5707 ms 200 /api/usage/breakdown?by=tool&since=2026-10-29T08:41:16.43335
     5595 ms 200 /api/usage/breakdown?by=model&since=2026-10-29T08:41:16.4333
     5217 ms 200 /api/usage/breakdown?by=day&since=2026-10-29T08:41:16.433358
     4749 ms 200 /api/warnings
warm: page wall 5.86s
     5860 ms 200 /api/usage/sessions?since=2026-10-29T08:41:16.433358Z
     5484 ms 200 /api/usage/breakdown?by=project&since=2026-10-29T08:41:16.43
     5330 ms 200 /api/usage/breakdown?by=day&since=2026-10-29T08:41:16.433358
     5055 ms 200 /api/usage/breakdown?by=model&since=2026-10-29T08:41:16.4333
     4923 ms 200 /api/usage/breakdown?by=tool&since=2026-10-29T08:41:16.43335
     4787 ms 200 /api/warnings
=== BEFORE run 2 2026-11-05T08:41:28Z
cold: page wall 6.17s
     6171 ms 200 /api/usage/sessions?since=2026-10-29T08:41:31.081725Z
     5828 ms 200 /api/usage/breakdown?by=project&since=2026-10-29T08:41:31.08
     5608 ms 200 /api/usage/breakdown?by=tool&since=2026-10-29T08:41:31.08172
     5505 ms 200 /api/usage/breakdown?by=model&since=2026-10-29T08:41:31.0817
     5300 ms 200 /api/usage/breakdown?by=day&since=2026-10-29T08:41:31.081725
     4742 ms 200 /api/warnings
warm: page wall 6.50s
     6493 ms 200 /api/usage/sessions?since=2026-10-29T08:41:31.081725Z
     6135 ms 200 /api/usage/breakdown?by=project&since=2026-10-29T08:41:31.08
     6021 ms 200 /api/usage/breakdown?by=tool&since=2026-10-29T08:41:31.08172
     5909 ms 200 /api/usage/breakdown?by=model&since=2026-10-29T08:41:31.0817
     5815 ms 200 /api/usage/breakdown?by=day&since=2026-10-29T08:41:31.081725
     4478 ms 200 /api/warnings
=== BEFORE run 3 2026-11-05T08:41:43Z
cold: page wall 6.12s
     6120 ms 200 /api/usage/sessions?since=2026-10-29T08:41:46.247972Z
     5699 ms 200 /api/usage/breakdown?by=project&since=2026-10-29T08:41:46.24
     5493 ms 200 /api/usage/breakdown?by=model&since=2026-10-29T08:41:46.2479
     5325 ms 200 /api/usage/breakdown?by=tool&since=2026-10-29T08:41:46.24797
     5322 ms 200 /api/warnings
     4806 ms 200 /api/usage/breakdown?by=day&since=2026-10-29T08:41:46.247972
warm: page wall 6.56s
     6557 ms 200 /api/usage/sessions?since=2026-10-29T08:41:46.247972Z
     6205 ms 200 /api/usage/breakdown?by=project&since=2026-10-29T08:41:46.24
     6065 ms 200 /api/usage/breakdown?by=day&since=2026-10-29T08:41:46.247972
     5860 ms 200 /api/usage/breakdown?by=model&since=2026-10-29T08:41:46.2479
     5559 ms 200 /api/usage/breakdown?by=tool&since=2026-10-29T08:41:46.24797
     4764 ms 200 /api/warnings
```

**AFTER**: same script, same store copy, same branch after the change, 3 runs, captured 2026-11-05T08:58Z (UTC), load average ~4.4 (the full test suite had just finished; BEFORE was ~3.2). Median of 3 cold page wall: 6.17 s -> 2.46 s. Median warm: 6.50 s -> 1.67 s.

```
15:58:10 up  9:42,  1 user,  load average: 4.42, 5.14, 4.18
=== AFTER run 1 2026-11-05T08:58:10Z
cold: page wall 2.44s
     2433 ms 200 /api/warnings
     2314 ms 200 /api/usage/sessions?since=2026-10-29T08:58:13.300080Z
     1852 ms 200 /api/usage/breakdown?by=day&since=2026-10-29T08:58:13.300080
     1851 ms 200 /api/usage/breakdown?by=model&since=2026-10-29T08:58:13.3000
     1836 ms 200 /api/usage/breakdown?by=tool&since=2026-10-29T08:58:13.30008
     1794 ms 200 /api/usage/breakdown?by=project&since=2026-10-29T08:58:13.30
warm: page wall 1.67s
     1662 ms 200 /api/warnings
     1614 ms 200 /api/usage/sessions?since=2026-10-29T08:58:13.300080Z
     1364 ms 200 /api/usage/outliers?since=2026-10-29T08:58:13.300080Z
      808 ms 200 /api/accounts?by=account
      790 ms 200 /api/usage/summary?since=2026-10-29T08:58:13.300080Z
      681 ms 200 /api/usage/breakdown?by=tool&since=2026-10-29T08:58:13.30008
=== AFTER run 2 2026-11-05T08:58:17Z
cold: page wall 2.46s
     2449 ms 200 /api/warnings
     2404 ms 200 /api/usage/sessions?since=2026-10-29T08:58:19.883985Z
     1934 ms 200 /api/usage/outliers?since=2026-10-29T08:58:19.883985Z
     1894 ms 200 /api/usage/breakdown?by=day&since=2026-10-29T08:58:19.883985
     1875 ms 200 /api/usage/breakdown?by=model&since=2026-10-29T08:58:19.8839
     1873 ms 200 /api/usage/breakdown?by=project&since=2026-10-29T08:58:19.88
warm: page wall 1.67s
     1672 ms 200 /api/usage/sessions?since=2026-10-29T08:58:19.883985Z
     1666 ms 200 /api/warnings
     1465 ms 200 /api/usage/outliers?since=2026-10-29T08:58:19.883985Z
      893 ms 200 /api/accounts?by=account
      788 ms 200 /api/usage/summary?since=2026-10-29T08:58:19.883985Z
      661 ms 200 /api/usage/breakdown?by=project&since=2026-10-29T08:58:19.88
=== AFTER run 3 2026-11-05T08:58:24Z
cold: page wall 2.48s
     2476 ms 200 /api/warnings
     2416 ms 200 /api/usage/sessions?since=2026-10-29T08:58:26.495273Z
     1953 ms 200 /api/usage/breakdown?by=day&since=2026-10-29T08:58:26.495273
     1951 ms 200 /api/usage/breakdown?by=project&since=2026-10-29T08:58:26.49
     1950 ms 200 /api/usage/breakdown?by=tool&since=2026-10-29T08:58:26.49527
     1949 ms 200 /api/usage/breakdown?by=model&since=2026-10-29T08:58:26.4952
warm: page wall 1.61s
     1602 ms 200 /api/warnings
     1590 ms 200 /api/usage/sessions?since=2026-10-29T08:58:26.495273Z
     1459 ms 200 /api/usage/outliers?since=2026-10-29T08:58:26.495273Z
      908 ms 200 /api/accounts?by=account
      780 ms 200 /api/usage/summary?since=2026-10-29T08:58:26.495273Z
      621 ms 200 /api/usage/breakdown?by=project&since=2026-10-29T08:58:26.49
```

Reads still above 2 s in the cold page (not fixed, out of scope): `/api/warnings` 2.43-2.48 s and `/api/usage/sessions` 2.31-2.42 s; the `breakdown` reads are 1.8-1.95 s cold. Each read alone (sequential, cold, cProfile on): `warnings` 1.52 s (20 SQL statements 0.76 s, about 58 `git` subprocess calls 0.32 s, plus two Python functions of 0.20 s and 0.23 s); `sessions` 0.71 s (one Python function 0.39 s over 67k messages, SQL 0.17 s). In the parallel page they stretch because the work is CPU-bound Python sharing one GIL; the page wall (2.45 s) is the sum of that work. Candidates for a follow-up (measure first): the rules evaluation behind `/api/warnings`, and the aggregation in `metrics.session_timeline`.

**DELTA**: the Usage page's reads on a 462k-event store now share one scan of the messages and one scan of the branches while the store is unchanged, so the page opens in about 2.5 s instead of about 6 s, and no read is served stale.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
