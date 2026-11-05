# TASK_REVIEW — HUN-232: Store reads take no write lock in connect(); migration 11 drops the stale events_ts index

> Sibling of `tasks/TASK_GUIDE_HUN-232.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
Owner instruction (not a decision): run HUN-231 and HUN-232 in parallel, auto mode ("yes, do both parallel [auto]", 2026-11-05).
1. Requirement Fidelity Gate stays signed by the owner ("yes", 2026-11-05) — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` row 13
2. spawned on claude · C1 → claude-sonnet-5-5 — FACT: PROJECT_SPEC.md:165 "| claude | `claude` | `claude-haiku-5-5` | `claude-sonnet-5-5` |"
3. built in parallel with HUN-231 but merged after it — FACT: tasks/TASK_GUIDE_HUN-232.md:54 "**Depends on**: HUN-231 — backup cleanup must be merged first"
4. migration 11 dry-run on a copy of the owner's real store (copy deleted afterwards), not on the store itself — RULE: tasks/TASK_GUIDE_HUN-232.md "Files Must NOT Touch" `~/.easy-personal/*`
Checks: tests `uv run pytest tests/core/test_store.py tests/core/test_migrations.py -q` — 34 passed, 1 deselected (Supervisor re-run); perf `-m perf` — 1 passed (implementer); full suite — 1541 passed, 2 skipped, 1 xfailed (implementer); reviews code-review — 0 P0, 0 P1, 0 P2, 1 P3 (implementer read only the first 40 lines of `agents/backend.md`), lane `C1, 4 files, 105 lines, FITS`; migration-safety Stage 4 — GO (only op `DROP INDEX IF EXISTS events_ts`, superseded by `events_ts_message`; restore path `bak-v10`); real run: copy of the owner's v10 store (474,642 events) → `migrate+connect 0.33s`, v11, `events_ts` gone, every table's row count equal, `bak-v10` is v10 with equal counts (auto-checked, not /verify)
Not done: merge (yours); /verify: ☑ pass (owner-requested run, see Evidence); memory update; Done

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | tests/core/test_store.py: AC1 `test_opening_an_up_to_date_store_takes_no_write_lock`, AC2 `test_a_new_store_gets_exactly_one_machine_id`; tests/core/test_migrations.py: AC3 `test_migration_11_drops_events_ts_keeps_other_indexes_and_every_row`, AC4 `test_migration_11_on_a_v10_store_without_events_ts_is_a_noop_and_idempotent`; AC5 existing plan + 1M-event tests pass. `34 passed, 1 deselected in 0.58s` |
| Verification command run | ☑ pass | `uv run pytest tests/core/test_migrations.py tests/core/test_store.py -q` → `34 passed, 1 deselected in 0.58s` (Supervisor, 2026-11-05) |
| Negative cases hold | ☑ pass | v10 store without `events_ts` migrates with no error; second apply changes nothing; connect under a held write lock returns in < 1 s instead of `database is locked` after 5 s |
| verify | ☑ pass | /verify run by the owner's request 2026-11-05 on commit 411321c — pass: isolated `uv tool install`, `--version` 0.4.0, `ping` exit 0, MCP stdio `tools/list` 12 tools + `ping` tool ok; `easy-personal usage` on a v10 store with `events_ts` → exit 0, v11, indexes `events_message events_ts_message events_account_ts`, `store.sqlite.bak-v10` written; `easy-personal usage` while another connection holds `BEGIN IMMEDIATE` → exit 0 in 0.49s (BEFORE: 5.01s then `database is locked`) |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | reviewed `store.connect()`, m0011, its 41 `store.connect/opened` callers by grep (signature unchanged), new tests |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` → `1541 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 200.41s` (implementer run) |
| **Companion score (easy-verifier)** | ☑ N/A | advisory: `score` ran with scope `changes`, ref `main...HEAD`, and returned overall 68, change_risk 100. Both branches got the same overall, so it measures the repo rather than this diff, and no report file was written (no `report:` path).
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | pure backend task, no UI |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | pure backend task, no UI |
| **UI: Responsiveness at target viewports** | ☑ N/A | pure backend task, no UI |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-05T15:26:29Z, before any implementation commit; `uv run python <scratchpad>/demo232.py`, tmp store, `~/.easy-personal/` untouched):

```
(a) connect() raised after 5.01s: database is locked
(b) user_version 10 indexes ['events_account_ts', 'events_message', 'events_ts', 'events_ts_message']
    backups []
```
(a) = second connection holds `BEGIN IMMEDIATE` on an up-to-date store; (b) = v10 store with `events_ts`, after `connect()` it is still v10 and still has `events_ts`.

**AFTER** (captured 2026-11-05T15:30:31Z, same script, after the change):

```
(a) connect() returned in 0.00s
(b) user_version 11 indexes ['events_account_ts', 'events_message', 'events_ts_message']
    backups ['store.sqlite.bak-v10']
```

**DELTA**: Reads (MCP tools, `/api/*`) no longer queue behind the live ingest writer when they open the store, and a store that still has the stale `events_ts` index loses it on the next open.

**WITNESS**: Supervisor re-ran the verification command in this worktree on 2026-11-05 (see Checks); BEFORE/AFTER captured by the implementer.
