# TASK_REVIEW — HUN-228: Usage page shows each card when its data arrives (NFR-013 page)

> Sibling of `tasks/TASK_GUIDE_HUN-228.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Extended track — RULE: `AGENTS.md` § Tracks ("a UI component")
2. Each Usage part renders when its own data arrives (per-part loading and error states) — RULE: owner approved the proposal "Page: show each card as its data arrives" ("yes, investigate and fix them [auto]")
3. `since` rounded down to the whole minute in Usage and the rail, so the same question gives the same URL (the 7-day window may start up to 59 s earlier) — SAFER OPTION: no API change; lets HUN-227 share work between the rail and Usage ⚠ decided for you — please review
4. Requirement Fidelity Gate signed `auto (Supervisor)` — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (row 13)
5. Spawn model `claude-sonnet-5-5` (C2) — FACT: `PROJECT_SPEC.md:165`
Checks: (filled at Stage 4)
Not done: merge (yours); auto verify: running (T182)


## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
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

**BEFORE**: unchanged code (commit 3eca522), `npm run build`, real dashboard on a store copy (`sqlite3 ~/.easy-personal/store.sqlite ".backup <dir>/store.sqlite"`, 415 MB, `EASY_PERSONAL_HOME=<dir> uv run easy-personal dashboard`), headless Chrome 1280x720, Usage page (Last 7 days), polled every 25 ms. Script: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h228/measure.mjs`. Captured 2026-11-05T08:40:49Z..08:41:12Z, before any implementation commit.

```
== BEFORE run 1   start 2026-11-05T08:40:49.039Z
  response   910 ms  /api/usage/summary            (first answer)
  response  1423 ms  /api/usage/breakdown?by=day&until=…   (previous)
  response  1975 ms  /api/usage/outliers
  response  2590 ms  /api/usage/summary            (rail)
  response  3636 ms  /api/usage/breakdown?by=day
  ... model 3816, tool 4791, project 5300, rail model 5642
  response  5978 ms  /api/usage/sessions            (last)
marks: {"firstKpi":6020,"totals":6020,"timeline":6020}
== BEFORE run 2   marks: {"firstKpi":6244,"totals":6244,"timeline":6244}   (last response 6218 ms)
== BEFORE run 3   marks: {"firstKpi":6793,"totals":6793,"timeline":6793}   (last response 6748 ms)
```

Time to first KPI card = time to Totals = time to Timeline = 6.0 / 6.2 / 6.8 s (median 6.2 s): nothing is on screen until the last request returns, although the summary answered at ~0.9-1.0 s and the day breakdown at 3.6-4.5 s. The rail's `/api/usage/summary?since=…` and `/api/usage/breakdown?by=model&since=…` URLs differ from the Usage view's by milliseconds in `since`.

BEFORE screenshot (all data loaded, identical-layout reference): `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h228/before-1.png` (also before-2.png, before-3.png).

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
