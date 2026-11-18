# TASK_REVIEW — T118: Live like Claude 4 — one panel for every session

> Sibling of `tasks/TASK_GUIDE_T118.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` or `[admin]` tag; with `[admin]`, the `Not done:` line names `merge (admin: after the auto verify pass)`,
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
`Memory:` records the diff-driven memory pass you run before you stop: each entry written, or `nothing new`.
After an auto verify pass the hook, never you, adds `Closed: row moved to Done <date> — …` and moves the row,
or adds `Paused: done not moved — <reason>`. It moves the row only when this block holds `Mode: auto`, a
numbered decision, `Checks:`, `Memory:`, the `Not done:` auto verify slot with its result, and `Not done:`
naming the merge — no `<...>` left (`docs/claude-md/tracks.md` `### Auto close-out`).
With `/verify (yours)`, your pass goes in the `verify` row below: Result `☑ pass`, Notes `/verify by the user, session <id>, commit <sha>`.

-->

## Auto decisions

Mode: auto
1. App sessions move into Live; App sessions tab keeps New session — FACT: owner AskUserQuestion answer 2026-11-18 "One place for all sessions"
2. The FR-104 one-time account prompt waits while a card is pending — FACT: owner test 2026-11-18, the dialog covered a question card
3. The FR-104 one-time dialog is suppressed on the whole Live view, not only while a card is pending — RULE: a card can arrive at any moment and app-session cards are known only inside Live (agent finding, accepted)
Checks: tests `npx vitest run` — 39 files, 1347 passed; `npm run build` ok; `check_static.py` ok; reviews code-review — 0 P0/P1, 2 P2 + 3 P3 fixed (`1cbbfc4`), re-check 1 P2 fixed (`ed7b34a`); security-review — no finding ≥ 8; ux-evaluate — 8 pass, 1 fail (older: 14 px body, app-wide setting)
Memory: memory/decisions.md — "2026-11-18 — Live like Claude: transcript on demand, queue, Stop/Force stop, past sessions, one panel" (+ MEMORY.md index line), `5c38bf4` on main
Not done: merge (admin: after the auto verify pass; after T115 and T116, whose routes it calls); owner smoke test (open a terminal session, read its tool output, queue a message); auto verify: pass
Paused: done not moved — Gate 5: no test run in memory/event-trace/T118.jsonl; report incomplete: auto verify slot

<!-- template:
Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its proposal>; Advocate 2: <its proposal>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Memory: <each entry the memory pass wrote, as `file — entry` | nothing new>
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours), with [admin] only after the one retry also ended blocked>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/LivePanel.test.jsx` (panel, 2 s poll, Load earlier via `start`, merge by id incl. orphans and front merge, MAX_KEPT trim, 404 stops polling, queue edit/cancel/409, Stop + Force stop, kind, Esc, late answer for another session), `web/src/LiveAppPanel.test.jsx` (45 app-session tests ported: send, Interrupt/Stop/Close, permission/question/plan cards), `App.test.jsx` (FR-104 dialog waits on Live), `AppSessions.test.jsx`, `AppSessionEntry.test.jsx`, `LiveView.test.jsx`, `LiveAnswer.test.jsx`, `ReplyBox.test.jsx` |
| Verification command run | ☑ pass | at `13e7717`: `npx vitest run` → `Test Files  39 passed (39)` / `Tests  1347 passed (1347)`; `npm run build` → `✓ built in 284ms`; `python3 scripts/check_static.py` → `check_static: ok` |
| Negative cases hold | ☑ pass | the FR-104 dialog test fails without the change; the openId re-sync test failed with the fix reverted; the joinTool offset test failed with the old merge (`expected '?before=60000' to be '?before=100'`); hostile markup renders as text; no `setItem`; no token in any request URL |
| verify | ☑ pass | auto-run by hook, session 238f982e-95f4-401b-a752-d4b699656303, commit b69704e223fca07afe8929a732126e3c66aea929, log .claude/verify-runs/T118-b69704e223fc.log — pass; done not moved: Gate 5: no test run in memory/event-trace/T118.jsonl |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | code-review and security-review on `main...T118` (`web/src` Live, panel, transcript, composer, App, AppSessions, api.js, css) + re-check of `1cbbfc4` |
| Full smoke suite still green (no regression) | ☑ pass | web 1347 / 1347 (baseline 1294) |
| **Companion score (easy-verifier)** | ☑ N/A | not run: independent code and security reviews used instead | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | ux-evaluate on `tasks/evidence-T118/{list,terminal,card,app}-{375,1280}.png` (mock server, real build); Supervisor read `terminal-1280.png` (list + panel: header, busy, Stop, conversation, tool input/result in own scroll boxes, queued text with Edit/Cancel) and `card-375.png` (Back to sessions, question card with options above the composer, nothing covers it). B-e 14 px body: fail (older, app-wide) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `appsession.css` / `app.css` use `tokens.css` variables only; one accent; open row has an accent bar |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 and 1280: `scrollW == clientW` in all 8 captures; at 375 the panel replaces the list with Back; controls ≥ 44 px at 375 |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (Supervisor, before any T118 implementation commit; `main` at `ddb947f`, the branch point):
```
$ date -u; git log -1 --oneline; grep -ln 'transcript\|queueReply\|stopLiveSession' web/src/*.jsx web/src/api.js | wc -l; ls web/src/SessionPanel.jsx web/src/Transcript.jsx
2026-11-18T10:15:50Z
ddb947f Plan: Live like Claude (question 45, FR-123..FR-128, DDR-0007, T115-T119)
1
ls: cannot access 'web/src/SessionPanel.jsx': No such file or directory
ls: cannot access 'web/src/Transcript.jsx': No such file or directory
```
(The one hit is `web/src/FirstRun.jsx`, unrelated wording; no transcript call, queue or Live stop exists. App sessions live on their own tab; Live shows tool labels only, no conversation, no Stop for terminal sessions, no queue.)
first implementation commit] OR [verbatim excerpt of the prior content, for non-executable changes]

**AFTER** (`13e7717`, 2026-11-18T11:56:22Z):
```
$ ls web/src/SessionPanel.jsx web/src/Transcript.jsx web/src/Composer.jsx; grep -c 'getTranscript\|stopSession\|queueReply' web/src/api.js
web/src/Composer.jsx
web/src/SessionPanel.jsx
web/src/Transcript.jsx
3
```
Screenshots `tasks/evidence-T118/terminal-1280.png`, `card-375.png`.

**DELTA**: in one Live panel the owner reads any session's conversation with tool input and output, answers its question cards, types to it (send, reply, or queue until the turn end), and stops it.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
