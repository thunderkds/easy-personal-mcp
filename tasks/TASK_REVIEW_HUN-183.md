# TASK_REVIEW — HUN-183: Release 0.4.0: version, README live attention + limits, site (after HUN-182)

> Sibling of `tasks/TASK_GUIDE_HUN-183.md`. Everything here is **filled by the reviewer at Stage
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

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/test_readme.py`: `test_status_names_v0_4_0_not_planning` (AC 1), `test_live_view_documents_attention_states_settings_and_start_limit` (AC 2), `test_kit_performance_is_documented` (AC 3), `test_upgrade_documents_migration_7_to_8_and_rollback` (AC 4); written first, 4 failed before the README change |
| Verification command run | ☑ pass | `uv run pytest tests/test_readme.py tests/test_site_content.py tests/test_skeleton.py -q` -> `59 passed in 1.46s` (2026-11-03 03:49 UTC) |
| Negative cases hold | ☑ pass | `test_checker_rejects_unknown_command`, site drift negative tests (version pill 9.9.9, extra tool) still pass; README has no host path, email or token (`test_no_host_paths_emails_or_tokens`) |
| verify | ☑ N/A | docs + version string only; `uv run easy-personal --version` prints `0.4.0` (pass); no fresh `uv tool install` (owner reinstalls). Supervisor runs Stage 5 |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Self-review of README.md, tests/test_readme.py, pyproject.toml, uv.lock (version line), site/index.html (pill). No `src/` change. Facts checked against `core/rules.py` (LIVE_DEFAULTS 3/10, THRESHOLD_KEYS), `core/kitperf.py` (MIN_SIDE 10, result keys), `core/migrations/m0008_tool_errors.py`, `surfaces/cli.py` (kit-findings flags). 5 % gap taken from HUN-195 (8bd491e, not in this branch) |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` -> `1239 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 172.53s`; `uv run ruff check .` -> `All checks passed!` |
| **Companion score (easy-verifier)** | ☑ N/A | not run (docs/version release task) |
| **UI: Visual regression** | ☑ N/A | no UI change; only the site version pill text |
| **UI: Design-system compliance** | ☑ N/A | no UI change; only the site version pill text |
| **UI: Responsiveness** | ☑ N/A | no UI change; only the site version pill text |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured Tue Nov  3 03:48:18 UTC 2026, worktree HUN-183 at 5031ed2, before any implementation commit):

```
$ date -u; uv run easy-personal --version
Tue Nov  3 03:48:18 AM UTC 2026
0.3.0
```

README "Upgrade and rollback", prior migration paragraph (verbatim): "The first command that opens the store applies the pending migrations. Version 0.3.0 moves a store from version 6 to 7: it adds the `entrypoint` column for the You and Agents split, and the next ingest fills it in. The migration leaves `store.sqlite.bak-v6` beside the store. The Live view needs no config change."

**AFTER** (2026-11-03 03:49:05 UTC): `uv run easy-personal --version` prints `0.4.0`. README "Upgrade and rollback" now reads: "Version 0.4.0 moves a store from version 7 to 8 (`m0008_tool_errors`). It adds the column `tool_error_count` ... The migration leaves `store.sqlite.bak-v7` beside the store." plus a new "Kit performance" section and Live view attention states.

**DELTA**: A user can read how 0.4.0 differs from 0.3.0 (attention states and their settings, easy-memory layer, kit findings and `kit_error_rate`, store 7 -> 8 with `.bak-v7` rollback), and the package reports 0.4.0.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
