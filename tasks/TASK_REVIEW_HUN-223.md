# TASK_REVIEW — HUN-223: Saved report snapshot keeps the full warning hit (FR-079 server)

> Sibling of `tasks/TASK_GUIDE_HUN-223.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_reports.py` (6 new: full hit stored, card fields in `before`, clear rule has none, old shape filled without `steps`, new shape not overwritten, rule gone keeps stored fields), `tests/surfaces/test_api_reports.py` (2 new over the API). 6 red before the change. |
| Verification command run | ☑ pass | Supervisor re-run 2026-11-05: `uv run pytest tests -q -k "report or leak"` → `194 passed, 1305 deselected`; agent full run `1490 passed, 2 skipped, 6 deselected, 1 xfailed`. |
| Negative cases hold | ☑ pass | Old shape with rule removed from config → stored fields only, no crash; unreadable config → stored fields (ValueError swallowed, same as `evaluate` path); `steps` never invented for an old snapshot. |
| verify | ☑ pass | Combined real run 2026-11-05T04:20Z: scratch worktree of HUN-224 + HUN-223's `reports.py`, fresh `.backup` copy of the owner store (report 1, old shape), real `easy-personal dashboard`: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/integ-open-1280.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/integ-open-375.png` (hscroll 1265 = 1265, 375 = 375). `GET /api/reports/1` old-shape `before` now carries `label` aware, `priority` 3, `why`, `fix`, `window_days` 7, no `steps` (agent run 04:10Z); the page draws the full card from it (Watch chip, Why it matters, causes, fix line as step 1, `Now: 245.4% · still firing`) — pass. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor `code-review` of `git diff main...HEAD -- src/` (`core/reports.py` only: `save` stores `found["hits"]`, `_fill_from_config`, `_before` returns `(title, state)`) + callers `get_report` → `GET /api/reports/{id}`. P0 0, P1 0, P2 0. Note: new saves store the full hit incl. `steps`/`why` text — all server-written fixed text (NFR-004), no model text added. Startup reads complete for the Permanent-Rule set (PROJECT_SPEC full, guide full, `agents/backend.md` full no limit). |
| Full smoke suite still green (no regression) | ☑ pass | Agent full run `1490 passed, 2 skipped, 6 deselected, 1 xfailed`. |
| **Companion score (easy-verifier)** | ☑ N/A | Advisory; one-file diff, same scope HUN-221 scored 72 with 9 of 13 dimensions abstained. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Backend only. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Backend only. |
| **UI: Responsiveness at target viewports** | ☑ N/A | Backend only. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-05T04:07:07Z (UTC), before any implementation commit. Store: `sqlite3 ~/.easy-personal/store.sqlite ".backup <scratchpad>/copy/store.sqlite"`, `EASY_PERSONAL_HOME=<scratchpad>/copy`; the copy holds report 1 (saved 2026-11-05T03:10:55Z, old shape). Command: `uv run python show.py <copy>` which prints `reports.get_report(conn, 1)` for rule `tokens_jump`, `before`:

```
rule_id: tokens_jump   title: "Token use jumped"
before keys: ['causes', 'diagnosis', 'fired', 'measured', 'state', 'summary', 'threshold']
before: state "fired", fired true, measured 2.2253, threshold 0.5, diagnosis "more_sessions",
        summary "Token use rose 222.5% in the last 7 days against the 7 days before; target 50% or less",
        causes [3 project entries, e.g. "easy-verifier-mcp: 267.9k -> 452.8M tokens, 1 -> 12 sessions", ...]
has label/why/steps: [False, False, False]
```

**AFTER**: same copy, same command, 2026-11-05T04:10:33Z (UTC), after the change:

```
before keys: ['causes', 'diagnosis', 'fired', 'fix', 'label', 'measured', 'priority', 'rule_id', 'state', 'summary', 'threshold', 'why', 'window_days']
has label/why/steps: [True, True, False]
label "aware", priority 3, window_days 7, why "A sudden rise is easy to miss and quickly becomes the new normal, along with its cost.",
fix "Open Usage and find which project or session drove the rise. ..." (from the current config; no `steps`, old shape)
```
`GET /api/reports/1` through the Starlette test client on the copy, 2026-11-05T04:10:47Z: status 200, same fields, `steps in before: False`.

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
