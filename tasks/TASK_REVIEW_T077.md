# TASK_REVIEW — T077: Watch 6: dashboard Watch view

> Sibling of `tasks/TASK_GUIDE_T077.md`. Everything here is **filled by the reviewer at Stage
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

**BEFORE**: captured 2026-11-15T06:29:02Z on branch `T077` at `892aa99` (no implementation commit yet), in `web/`:

```
$ ls src/WatchView.jsx
ls: cannot access 'src/WatchView.jsx': No such file or directory
$ grep -c "Watch" src/App.jsx
0
$ npm test 2>&1 | grep -E "Test Files|Tests "
 Test Files  25 passed (25)
      Tests  838 passed (838)
```

The dashboard has no Watch view: no `WatchView.jsx` and no "Watch" in the nav (`App.jsx`).

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T077.jsonl`, never the
implementing agent alone]


---

## Layout proposal

No Graphite Mint artboard covers the Watch view. The only "watch" hits in `docs/design/graphite-mint/` are the tray artboards; `rust/docs/UI_REDESIGN_PLAN.md` lists tray (G6/G7) but no dashboard Watch screen. So this is a proposal for the owner to confirm. It reuses existing pieces only: `.card`, `.kpi-strip`/`.kpi`, `.table-frame`, `Sparkline` look (mint line, `vector-effect: non-scaling-stroke`), the tokens in `web/src/tokens.css`, no new colour.

**Nav**: a new tab `Watch` in the `// work` group, right after `Live` (both are about "now"). Not scoped (machine and limits have no workspace; effectiveness is shown for all). Alternative: the `// usage` group, last.

**Order, top to bottom** (one column, cards stack; same page width as Usage):

```
Watch
Last 24 h, one sample a minute. Kept in memory: the graphs start empty after a restart.

// machine                                                    (card)
+-------------+-------------+-------------+
| CPU         | RAM         | Disk        |      grid: auto-fit, min 11.9rem
| 23 %        | 6.1 / 15 GB | 71 %        |      (like .kpi-strip: 3 per row at 1440,
| ~~/~~/~~~ | ___/--\____ | ------------|       2 at 1024, 1 at 375)
+-------------+-------------+-------------+
| Load (1 min)| Battery     | Claude      |
| 0.42        | 88 % charg. | processes 3 |
| ...         | no data     | ...         |
+-------------+-------------+-------------+
  each tile: name, latest value (mono, large), 24 h sparkline, "24 h ago ... now" in muted small text
  null sample = a GAP in the line (the path breaks), never 0; no data at all = value "no data", empty sparkline
  desktop with no battery: Battery tile says "no data"

// plan limits                                                (one card per account)
+-----------------------------------------------+
| work account                  5 h 41 %   7 d 12 % |   label as h3; latest values (red at 90 % and more, as Warnings)
| 100 %  - - - - - - - - - - - - - (90 % line)  |
|        5 h = solid mint line, 7 d = amber line |   fixed 0..100 scale; a null round = a gap
|   0 %  ______/\_____/--\____                  |
|        24 h ago                          now  |
+-----------------------------------------------+
  one account: one card; no account: "No Claude account yet" line; limits file missing or stale: "no data" instead of values

// effectiveness                                              (card, .table-frame inside .table-wrap)
| Task      | Tokens | Cost  | Sessions | Good | Tokens / good | Done       | Tokens / task done |
| T075      | 1.2 M  | $4.10 |  6       |  4   | 300 k         | 2026-11-14 | 1.2 M              |
| HUN-245   | 800 k  | $2.90 |  3       |  0   | no data       | not done   | no data            |
| Unattributed (last, muted) ...                                                      |
  "oldest stored day" note under the table (the API's retention_note), only when given
  "unknown price" shown for a cost with unknown price (never $0); "unknown" for a done state without git history
  long list: scrolls inside the card (table-wrap), no page-wide horizontal scroll at 375 px; first column sticky is NOT planned
  empty: "No tasks yet in this period."
```

**States**: loading line per part (as Usage's `Part`), a part that fails shows its own error line and the others still render; machine with no samples (just after start) shows "No samples yet. The first one comes within a minute."

**Cut list (not built)**: totals row, per-task branches/project column, period picker (fixed to All time), tooltips on the graphs, alerts list.

**Questions for the owner**: (1) nav place: `// work` after Live (recommended) or `// usage`? (2) Effectiveness period: all time (recommended) or last 30 days? (3) OK with the 6 machine tiles and one card per account?

**Owner answer (2026-11-15, relayed by the Supervisor)**: "Approve as proposed". Watch tab in `// work` after Live; effectiveness over all time; 6 machine tiles; one limits card per account; the cut list stands.
