# TASK_REVIEW — HUN-224: Opened saved report uses Warnings & plan cards + Now line (FR-079 UI)

> Sibling of `tasks/TASK_GUIDE_HUN-224.md`. Everything here is **filled by the reviewer at Stage
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

**BEFORE** (captured 2026-11-05T04:09:11Z, before any implementation commit; HEAD `7cb1159`):

- Command: `node h224-shoot.mjs "http://127.0.0.1:56641/#<token>" .../scratchpad/h224-before-1280.png 1280 720 fired` (headless Chrome, CDP) against `easy-personal dashboard` run from this worktree's source on a store copy (`EASY_PERSONAL_HOME=.../scratchpad/h224home`, made with `sqlite3 ~/.easy-personal/store.sqlite ".backup ..."`), Warnings & plan, saved report 1 open.
- Output: `nav: clicked`, `row: clicked`, `hscroll: 1265 vs 1265`, `has Saved reports: true`, `saved /tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h224-before-1280.png 2026-11-05T04:09:11.616Z`
- Observed in the screenshot: opened report = Summary, Fixes (cite as grey text in brackets, `(session_outliers: 645,301)`), then a "Before / now (now = last 7 days)" table with 6 rule rows (`97.5% clear -> 97.5% clear`, `222.5% fired -> 239.8% fired`, ...). No warning card, no "Now:" line, no "Other rules" disclosure.
- Vitest before: `cd web && npx vitest run` at 2026-11-05T04:07Z printed `Tests  617 passed (617)`.

**AFTER** (2026-11-05T04:13:16Z, same command and store copy; server is pre-HUN-223, so `before` has the old shape, which is AC 7):

- `hscroll: 1265 vs 1265` (1280 px), `hscroll: 375 vs 375` (375 px); screenshots `.../scratchpad/h224-after-open-1280.png`, `.../scratchpad/h224-after-open-375.png` (scratchpad = `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/`).
- Observed: Summary; Fixes with chips `session_outliers · 645,301`, `usage_breakdown · $2,261.62`; heading "1 to watch then"; one `Token use jumped` card (same markup as the list card above, with "Why this happened"); `Now: 242.1% · still firing`; closed "Other rules (6)". On the old-shape snapshot the card has no label chip and no steps (the server sends no `label`/`why`/`fix` before HUN-223), so the chip row is empty. That gap closes when HUN-223 merges.
- Patching the store copy's `snapshot_json` to the full-hit shape changed nothing on the page: the pre-HUN-223 server builds `before` from `checked` plus only causes/summary/diagnosis of `fired`. The full-hit card is therefore proven by vitest fixtures only (`reportOne`, HUN-223 shape), not by a real run. Re-shoot after HUN-223 merges.
- Vitest after: `cd web && npx vitest run` printed `Test Files  21 passed (21)`, `Tests  630 passed (630)` (before: 617); `npx vitest run src/WarningsPlanView.test.jsx` printed `Tests  57 passed (57)`; `npm run build` printed `built in 233ms`.

**DELTA**: An owner opening a saved report now reads it as the Warnings & plan page does: summary, fixes with cited-number chips, a warning card per rule that fired then with a "Now:" line, and the other rules in a closed list.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
