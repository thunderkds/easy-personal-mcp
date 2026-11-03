# TASK_REVIEW — HUN-193: kit_findings: like-with-like session comparison per kit server, MCP tool + CLI (FR-060..FR-063, NFR-014; after HUN-192)

> Sibling of `tasks/TASK_GUIDE_HUN-193.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_kitperf.py` (35 tests: AC1 server use incl. sub-agent and `mcp__claude_ai_*`; AC2 commit window, merged+deleted branch, main/NULL/not-a-repo; AC3 error rate 1/10 and 3/10 at 0.2 and 0.3, NULL count = unknown; AC4 `kit_error_rate` config, non-number refused, missing file = default; AC5 hand-computed pooling 8.333% with one-sided strata excluded; AC6 9 vs 10 sessions; AC7 wording and habit; AC9 slow fake git; AC10 canary), `tests/surfaces/test_kit_findings.py` (7 tests: AC8 MCP = CLI `--json`, limit cap), `tests/leak/harness.py` (registers `kit_findings` and `kit-findings`; AC10) |
| Verification command run | ☑ pass | `uv run pytest tests/core -q -k kitperf` → `35 passed, 490 deselected`; `uv run pytest tests/surfaces -q -k "kit_findings or mcp or cli"` → `47 passed, 169 deselected`; `uv run pytest tests/leak -q` → `34 passed in 47.86s`; `uv run pytest -q` → `1231 passed, 2 skipped, 6 deselected, 1 xfailed in 144.14s` (2026-11-03, worktree HUN-193; log `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/54000ba3-d7c0-4535-8376-908a80d33c6d/scratchpad/hun193/verify.log`) |
| Negative cases hold | ☑ pass | 9 pooled sessions → `not_enough`; branch `main`/`master`/`HEAD`/NULL, not a repo, gone folder, branch texts `--all`, `a..b`, `x@{1}`, tie of two branches → unknown; NULL `tool_error_count` with tool calls → unknown, never 0; git missing on PATH → all unknown; empty store → no findings, no error; slow git → returns within budget + 1 s |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` → `1231 passed, 2 skipped, 6 deselected, 1 xfailed`. Four doc/registry tests needed the new tool listed (`tests/test_skeleton.py`, `README.md` tool list + command + count 11, `site/index.html` row + stat 11): minimal edits, HUN-194 owns the real prose |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured Tue Nov 3 03:06:58 UTC 2026, branch HUN-193, before any implementation commit, on the v8 store copy `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/54000ba3-d7c0-4535-8376-908a80d33c6d/scratchpad/hun193/store.sqlite`:

```
$ date -u; EASY_PERSONAL_HOME=<hun193 dir> uv run easy-personal kit-findings --json
usage: easy-personal [-h] [--version]
                     {ping,mcp,ingest,schedule,usage,outliers,workspace,warnings,kit,plan,dashboard,exec,headers,connector,account,doctor} ...
easy-personal: error: argument command: invalid choice: 'kit-findings' (choose from ping, mcp, ingest, schedule, usage, outliers, workspace, warnings, kit, plan, dashboard, exec, headers, connector, account, doctor)

$ MCP tool names from surfaces/mcp.create_app().list_tools():
['connector_catalog', 'connector_status', 'connector_suggestions', 'improvement_plan', 'kit_summary', 'ping', 'session_outliers', 'usage_breakdown', 'usage_summary', 'warnings']
```
(no `kit_findings` tool, no `kit-findings` command)

**AFTER**: Tue Nov 3 03:15:59 UTC 2026, same store copy, after the change:

```
$ EASY_PERSONAL_HOME=<hun193 dir> uv run easy-personal kit-findings --json
{
  "as_of": "2026-11-03T03:16:01Z",
  "findings": [
    {
      "delta_pct": 236.2,
      "habit": null,
      "measure": "tokens",
      "server": "easy-ui",
      "text": "Sessions that used easy-ui are linked to 236.2% more tokens than similar sessions without it (27 vs 62 sessions).",
      "with": 27,
      "without": 62
    },
    {
      "delta_pct": 207.1,
      "habit": null,
      "measure": "cost",
      "server": "easy-ui",
      "text": "Sessions that used easy-ui are linked to 207.1% more dollars of API-equivalent cost than similar sessions without it (23 vs 59 sessions).",
      "with": 23,
      "without": 59
    },
    {
      "delta_pct": 196.2,
      "habit": null,
      "measure": "length",
      "server": "easy-ui",
      "text": "Sessions that used easy-ui are linked to 196.2% more minutes of session time than similar sessions without it (27 vs 62 sessions).",
      "with": 27,
      "without": 62
    }
  ],
  "not_enough": [],
  "note": "linked to, not caused",
  "sessions": 621,
  "unknown_results": 396
}
$ median of 3 runs (`uv run easy-personal kit-findings --json`, includes uv start-up): 1.89 s, 1.92 s, 1.90 s -> median 1.90 s (< 3 s, NFR-014)
```
MCP tool list now includes `kit_findings` (`tests/test_skeleton.py` lists it after `kit_summary`).

**DELTA**: Claude (MCP `kit_findings`) and the owner (`easy-personal kit-findings`) can now read, from real history, which Easy Kit servers are linked to cheaper, shorter or better sessions, like with like.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
