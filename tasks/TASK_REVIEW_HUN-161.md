# TASK_REVIEW — HUN-161: Typed vs agent usage split (entrypoint migration)

> Sibling of `tasks/TASK_GUIDE_HUN-161.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` tag,
`docs/claude-md/tracks.md` `## Auto mode`). To use it, copy the block below out of this comment, to this
place, and replace every <...>. Keep `Mode: auto` as its first line, exactly as written: T182's hook reads
it. Write it to this file before the run ends; the chat message is a copy, not the report. The same
block is the Supervisor's final chat message. It is not the `delivery-report` page and not
Evidence. A taste decision (what the user wants) also ends with the flag. Write `DEBATED` only when ≥2
Advocate `Agent` calls ran for that question, naming each Advocate's option. A decision with no Advocates is
labelled `RULE`, `FACT` or `SAFER OPTION`, never `DEBATED`. A decision flagged `⚠ decided for you` is a choice, and is never labelled `FACT`. A
safety action a rule requires (for example the backup before a delete) is `RULE` without the flag.
At Risk Medium or High, a `RULE:` counts only when it names a written rule (`path:line`,
`path § <heading>`, or `docs/claude-md/tracks.md` `### <section>`), and a `FACT:` needs a Citation
(`path:line "quoted text"`); without one it is a choice and must be `DEBATED`. Good: `2. backed up the CSV
— RULE: `docs/claude-md/tracks.md` `### Stops that stay``. Bad: `1. kept recurring.csv — RULE: Surgical Changes`.

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its option>; Advocate 2: <its option>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours)>

-->

## Migration-safety (Supervisor, 2026-10-31, `m0007_entrypoint.py` at HUN-161 954ffc4)

Verdict: **GO**. Reversible: yes (restore `<store>.bak-v6`, or `ALTER TABLE events DROP COLUMN entrypoint` + `PRAGMA user_version = 6`; tested). Downtime: none. Data-loss risk: none.

- [x] Managed migration (`store.migrate`, numbered `MIGRATIONS`, own `BEGIN IMMEDIATE` + `user_version` bump); no ad-hoc SQL.
- [x] In scope (FR-039, NFR-009). Guide said "every source file"; reset limited to `source = 'claude_code'` — accepted (only Claude carries `entrypoint`; kit_trace re-read gains nothing).
- [x] Destructive ops: none (nullable `ADD COLUMN`; offset reset only).
- [x] Rollback tested up → down → up on a copy of the owner's store: v6 → v7 → v6 (column gone, 267,708 events) → v7 (267,708 events).
- [x] Expand-only; single local process, no multi-version deploy; old program refuses v7 store with a clear message.
- [x] Zero-downtime: additive only.
- [x] Locks: SQLite `ADD COLUMN` is metadata-only — migrate took **147 ms** on the 236 MB copy (267,708 events, 688 Claude files).
- [x] Backfill separate from the schema change (next ingest), per-file commits, idempotent, resumable (`test_interrupted_backfill_resumes_without_duplicates`, `test_backfill_twice_gives_the_same_rows`).
- [x] Dry run on a production-like copy: migrate 147 ms; backfill ingest **16.94 s, maxrss 69,684 KB**; `integrity_check` ok; `foreign_key_check` 0 rows; `.bak-v6` written; kit/other file offsets unchanged (445 files, sum 15,040,956). After backfill (copy also took in 2 days of new sessions): 314,096 events; entrypoint `cli` 291,164 · `sdk-cli` 18,546 · `claude-desktop` 521 · NULL 3,865.
- [x] No silent data loss. Note (P3): with `--retention-days` set, a re-read can bring back rows pruned earlier until the next pruning ingest; default retention is forever, and Claude deletes transcripts after 30 days, so the window is small.

Finding from the dry run → owner decision 2026-10-31: `claude-desktop` is interactive; classification changed to *agent = sidechain or entrypoint starting with `sdk`*. Sent back to the implementer.

---

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_entrypoint.py` (migration 7 registered/upgrade/idempotent, classification table, backfill, perf + EXPLAIN), `tests/sources/test_claude_code_entrypoint.py` (parse, interrupted backfill), `tests/surfaces/test_usage_split.py` (MCP = CLI = API), `web/src/UsageView.test.jsx` (7 split tests). Supervisor run 2026-10-31T15:22:39+07:00 on 07788de: `Tests  263 passed (263)`; `860 passed, 6 deselected, 1 xfailed, 1 warning in 191.53s (0:03:11)` |
| Verification command run | ☑ pass | `npm --prefix web test -- --run` → `Test Files  12 passed (12)` / `Tests  263 passed (263)`; `npm --prefix web run build` → `✓ built in 251ms`; `uv run pytest -q` → `860 passed, 6 deselected, 1 xfailed, 1 warning in 191.53s (0:03:11)`; `uv run ruff check src tests` → `All checks passed!` |
| Negative cases hold | ☑ pass | non-identifier entrypoint dropped (parse test); missing entrypoint → unknown; re-read adds 0 rows (`new_events == 0`); migration re-run idempotent; dry run on store copy: integrity ok, 0 FK errors |
| verify | ☑ pass | Supervisor /verify recipe 2026-10-31T15:19:32+07:00, isolated install on fixtures: `0.2.0`, `easy-personal core ok`, MCP `usage_summary` → `split: {'typed': 1215, 'agent': 58, 'unknown': 0}`, `entrypoints: {'cli': 1273}`; `GET /api/usage/summary` split identical — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor code-review + inline security-review 2026-10-31 on `main...HUN-161` (07788de): 15 files + callers `surfaces/{api,cli,mcp}.py` (pass metrics dicts through), `sources/claude_code._label`, `store.migrate`. lane_check: `lane: C2, 15 files, 725 lines, EXCEEDS (files 15 > 14; lines 725 > 500)` — mostly tests (4 new test files); raised to owner, no relabel. Findings: P0 0. P1 1 (process): implementer's `Startup reads` omits `agents/frontend.md` (named for the Usage part) and PRD/brainstorm extras — not fixable after the fact; mitigated by Supervisor UI review below. P2 2: (a) `.split-unknown` uses `--color-dot` #3a413e on #121413 ≈ 1.6:1, under the 3:1 non-text contrast target — suggest `--color-muted`; (b) `save_file` runs one SELECT + UPDATE per session per file — fine at measured 16.9 s backfill, revisit if NFR-005 slips. P3 1: retention re-read window (see migration-safety). Security (Medium risk): `entrypoint` passes `_label` (identifier-only) so no free text is stored; all SQL parameterized (the f-string parts are constant column lists / `in_file`); no new route, no secret, leak suite green. No safe P0/P1 code fix to apply. |
| Full smoke suite still green (no regression) | ☑ pass | full suites above, no regression vs baseline (829 → 860 passed) |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier MCP not run for this task (advisory, no hook); no report path. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Supervisor 2026-10-31, headless Chrome over CDP, HUN-161 07788de dashboard on a date-shifted copy of `tests/fixtures/claude_code` (incl. the sidechain fixture): `reports/HUN-161/Usage_{1280,768,375}.png`, `crop1280b.png`. Verdict pass: new "You and agents" card, stacked columns per day (You accent, Agents amber), legend `You 1,215 (95.4%)` / `Agents 58 (4.6%)`, help line, Show table; same card style as By day. CLI on the same store: `split {'agent': 58, 'typed': 1215, 'unknown': 0}`. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `.split-*` rules use tokens only (`--color-accent`, `--color-amber`, `--color-dot`, spacing/font tokens); vitest asserts no raw literals; console errors `[]`; request hosts only `127.0.0.1` (NFR-001). P2 noted: Unknown swatch contrast. |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280 / 768 / 375 px: `scrollWidth <= clientWidth` at all three. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (Supervisor, 2026-10-31T14:49:29+07:00, main de84e68, before any implementation commit):
```
$ grep -rn "entrypoint" src
(no output — no entrypoint column)
$ ls src/easy_personal/core/migrations | grep m000
m0001_baseline.py … m0006_connectors.py
$ grep -n "typed\|\"agent\"" src/easy_personal/core/metrics.py
(no output — usage_summary has no typed/agent split)
```

**AFTER** (Supervisor, 2026-10-31T15:19:32+07:00, HUN-161 07788de):
```
$ ls src/easy_personal/core/migrations | grep m0007
m0007_entrypoint.py
$ (MCP) tools/call usage_summary
split: {'typed': 1215, 'agent': 58, 'unknown': 0} | entrypoints: {'cli': 1273}
$ GET /api/usage/summary → split {'typed': 1215, 'agent': 58, 'unknown': 0}
```
On a copy of the owner's store: entrypoint `cli` 291,164 · `sdk-cli` 18,546 · `claude-desktop` 521 · NULL 3,865 after a 16.9 s backfill.

**DELTA**: The owner and the Supervisor agent can see how many tokens went to work the owner typed versus sub-agents and headless (sdk) runs, per day, with the same numbers on MCP, CLI and dashboard.

**WITNESS**: Supervisor, 2026-10-31 (verify recipe, store-copy dry run, full suites with `active_task` = HUN-161).
