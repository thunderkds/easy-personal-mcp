# TASK_REVIEW — T068: Top bar status chips (live, waiting, last ingest, 5h limit)

> Sibling of `tasks/TASK_GUIDE_T068.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/StatusChips.test.jsx` (24 new tests; AC 1-3 incl. one-stream test against `<App/>`) |
| Verification command run | ☑ pass | Supervisor re-run 2026-11-14: `Test Files  25 passed (25)`, `Tests  810 passed (810)`; `npm run build` `✓ built in 243ms` — pass |
| Negative cases hold | ☑ pass | 0 sessions → grey `0 active`, no waiting chip; no limits → `5h no data`; health fail → `ingest no data`; stale limits = no data — pass |
| verify | ☐ pass / ☐ fail / ☐ N/A | not run yet: Stage 5 `/verify` before merge |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | diff + direct callers only, see Stage 4 section |
| Full smoke suite still green (no regression) | ☑ pass | full web suite above — pass |
| **Companion score (easy-verifier)** | ☑ N/A | advisory run: overall 69, change_risk 0 (needs_input not answered: change_kind, test_pairs); no report path written |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | `tasks/evidence-T068/after-1440.png` viewed by the Supervisor: chip order, colours, 28 px height match `Main.dc.html` header — pass |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | grep for hex/`rgb(` in new CSS: none; amber border via `color-mix` on `--color-amber` (no amber-line token) — pass |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 (wrap, scrollWidth 375), 768 and 1024 one row (scope text ellipsised), 1440 one row — pass |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-14T16:15:21Z on branch `T068` at main `042997b`, before any implementation commit. Command: `npm run build` in `web/`, then `scratchpad/shot.sh full 1440 300 before-1440.png` (a throwaway fixture server that serves the built page and canned `/api/*` JSON plus an SSE `/api/live` snapshot with 2 sessions, 1 waiting; then `google-chrome --headless=new --window-size=1440,300 --screenshot=...`). Output: `47706 bytes written to file .../before-1440.png`. Screenshot: `tasks/evidence-T068/before-1440.png`. Observed: the top bar holds the menu button (hidden at 1440), title `Usage` and the right-aligned text `scope: All workspaces · All accounts`, and nothing else (no chips), although the fixture has 2 live sessions, 1 waiting, a 4-minute-old ingest and 5h limit 42 %.

**AFTER**: same fixture server and page, rebuilt from branch `T068` (`npm run build`, `check_static: ok`), captured through Chrome DevTools (`tasks/evidence-T068/cdp_shot.mjs`, wait until the chips and the Usage cards have drawn). 1440 px: one row, chips right-aligned after the scope text, each 28 px tall, 6 px radius, `IBM Plex Mono` 12 px, gap 6 px, padding `0px 10px`: `2 active` (mint dot, glow), `1 waiting` (amber, triangle), `ingest 4m ago` (muted), `5h 42%` + 36 px bar. Screenshots: `tasks/evidence-T068/after-1440.png`, `after-1024.png` (one row, scope text cut with an ellipsis), `after-375.png` (chips wrap under the title and scope, `scrollWidth` 375 = `innerWidth`, no horizontal scroll), `variants.png` (top to bottom: 72 % amber, 91 % red, `5h no data`, `0 active` grey with no waiting chip, `ingest 4m ago · no timer` amber with triangle). `npm test`: `Test Files  25 passed (25)`, `Tests  810 passed (810)`.

**DELTA**: every view's top bar now shows live-session, waiting, last-ingest and 5-hour-limit status at a glance, and the Live and Waiting chips open the Live view.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T068.jsonl`, never the
implementing agent alone]

## Stage 4 — code-review (Supervisor, 2026-11-14)

Scope: `main...T068` production files (`App.jsx`, `LiveView.jsx`, `Rail.jsx`, `StatusChips.jsx`, `app.css`) + callers (`LiveView` consumers, `format.js` `limitTone`/`until`). Lane: `C2, 14 files, 571 lines, EXCEEDS (lines 571 > 500)` — excess is the evidence folder (4 PNG + 3 scripts), not code. Reachability: entry `web/src/App.jsx` topbar renders `StatusChips` — reachable. Startup reads line present with all Permanent-Rule reads.

P0: none. P1: none. P2: none.
P3 (optional):
| File | Finding | Confidence | Action |
|---|---|---|---|
| `web/src/LiveView.jsx` reducer | With one stream for the page's life, ended sessions stay in `state.sessions` until a reconnect (decision 5); the Live view now lists every session since the window opened, not since the view opened | 75 | Owner call; later: drop idle sessions older than N hours |
| `tasks/evidence-T068/` | Evidence PNGs and throwaway fixture scripts committed in the task branch | 75 | Keep or move to scratch before merge (owner call) |
| `web/src/StatusChips.jsx` | `tone-green` class (from `limitTone` < 70) has no CSS rule; falls back to the default colour as intended | 100 | None needed, or a comment |

0 safe fixes applied.
