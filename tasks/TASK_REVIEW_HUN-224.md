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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/WarningsPlanView.test.jsx` describe "saved report as warning cards (HUN-224)": 15 tests (order, card `outerHTML` equal to the list card, Now line states, priority sort, closed Other rules, Nothing fired then, old shape without steps, unique ids); red first (15 failing). |
| Verification command run | ☑ pass | Supervisor re-run 2026-11-05: `npx vitest run` → one run `1 failed | 629 passed` (test name not captured; run overlapped a `git worktree remove` and a pytest run), then 3 consecutive runs `Tests 630 passed (630)`; treated as a flaky test under load, not a defect of this diff. `npm run build` → `✓ built`. |
| Negative cases hold | ☑ pass | Abstained / not-measured now → reason text, never `0`; no fired rule → "Nothing fired then"; old-shape `before` without `title`/`steps` → card with top-level title and `fix` line. |
| verify | ☑ pass | Combined real run 2026-11-05T04:20Z: scratch worktree of HUN-224 + HUN-223's `reports.py`, fresh `.backup` copy of the owner store (report 1, old shape), real `easy-personal dashboard`: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/integ-open-1280.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/integ-open-375.png` (hscroll 1265 = 1265, 375 = 375). Opened report: Summary, 4 fixes with `tool · number` chips, "1 to watch then" card identical to the list card above (rank 1, Token use jumped, Watch chip, summary, Why it matters, Why this happened ×3, step), `Now: 245.4% · still firing`, "Other rules (6)" closed — pass. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor `code-review` of 5 web files. P0 0, P1 0, P2 0. P3 1: `SavedReports.jsx` ↔ `WarningsPlanView.jsx` import cycle (safe: render-time use only; moving `Card` to its own file would remove it). Card gained optional `idPrefix`/`children` (defaults keep the list identical; unique-id test). Startup reads: Permanent-Rule set complete (PROJECT_SPEC full, `agents/frontend.md` full no limit); some supporting files read in part, stated by the agent. |
| Full smoke suite still green (no regression) | ☑ pass | `Tests 630 passed (630)` ×3 (617 before + 13 net new); no server file in this diff. |
| **Companion score (easy-verifier)** | ☑ N/A | Advisory; web-only diff. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | BEFORE `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h224-before-1280.png` (table layout); AFTER `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/integ-open-1280.png` (cards). |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Reuses `Card` (imported, not copied); Other rules reuses `details.not-checked`; new `.card-now` rule uses tokens only. |
| **UI: Responsiveness at target viewports** | ☑ pass | 375×812 `integ-open-375.png`: card and chips wrap; no page-wide horizontal scroll (375 = 375). |

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
