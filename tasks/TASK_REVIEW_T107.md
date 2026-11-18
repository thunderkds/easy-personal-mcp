# TASK_REVIEW — T107: Write sessions 5 — Settings bridge block and the Live view answer card

> Sibling of `tasks/TASK_GUIDE_T107.md`. Stage 3 parts only (frontend agent, 2026-11-17). The Stage 4/5 rows are not filled here.

---

## Auto decisions

Mode: auto
1. Allow is offered only when `complete === true` and the input is a non-empty string; any other card can only be denied or handed to the terminal. RULE: `PRD.md:212` FR-115 "a cut prompt is marked `complete: false` and can only be denied or handed to the terminal"
2. Turn on shows the server's exact `entries_text`, the target file and the backups note, and sends `confirmed: true` only after **Add these entries**. RULE: `PRD.md:210` FR-113 "shows the exact entries it will add … and adds them only after the owner confirms"
3. Question, label, header and input text are React text only, held in component state, never in `localStorage`. RULE: `PRD.md:212` FR-115 "Held in memory only"
4. The permission input block is in the page flow (no inner scroll), so its tail cannot sit below a scroll edge. RULE: `PRD.md:212` FR-115 "a cut or one-line summary let the owner allow a command whose tail they never saw"
Checks: tests `cd web && npx vitest run` — Test Files 31 passed (31), Tests 1086 passed (1086); reviews code-review + security (independent, 0 P0, 1 P1 + 2 P2 + 2 P3, all fixed in `2c68f40`), security-review — T106 skill run covered the bridge; real run: not done — answering a real terminal session from the app needs the owner (AC 5)
Memory: nothing new (the card decision is in memory/decisions.md, T106 entry)
Not done: merge (admin: after the auto verify pass); auto verify: ☑ pass (session 4b233003-7ced-4346-937c-4fbb84cf870b, commit c93395df3dfa9b629e72d1e791d84c56df2be753, log .claude/verify-runs/T107-c93395df3dfa.log); owner real run (AC 5)

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass (AC 1-4; AC 5 is the owner's real run) | `web/src/AnswerBridge.test.jsx` (AC 1: block text and the not-covered note, per-account on / off / changed by hand / binary moved, buttons per state, Turn on shows the server's exact `entries_text` and the settings file and writes nothing until confirmed, PUT body `{account, on, confirmed: true}`, 409 and failed-change texts, 404 draws nothing, `listening: false` note, no `localStorage`); `web/src/LiveAnswer.test.jsx` (AC 2: question card by option position, multi-select + Other, secret-hidden question answerable, permission Allow / Deny + note, Answer in terminal, 409 -> *Already answered.*, 404 and a prompt that leaves the stream -> *The terminal asks now.*, 400 keeps the card with the server's reason, network failure, double click dropped, text never HTML, token only in the header; the permission card: `pre-wrap` monospace block, line breaks and visible escapes kept, *Too long to show in full* banner and no Allow for `complete: false`, a refused allow (other 409, T106's exact text) keeps the card; Stage 4 fix round: `readPrompt fails closed`, `fails closed: no Allow when complete is missing, or the input is empty`, `in the page flow: no max-height and no inner scroll`, `a new prompt starts clean (key by prompt id)`, AnswerBridge 503 reason and `other_copy` tests); `web/src/LiveView.test.jsx` `T095 AC6, T107 AC3` (footnote names the new texts, now the tool's full input) |
| Verification command run | ☑ pass (lint: no script) | 2026-11-17, T107 worktree, `cd web`. `npm run lint` -> `npm error Missing script: "lint"` (`web/package.json` has only `build` and `test`; no eslint or other linter is installed, and adding one is a new dependency, so none was added). `npx vitest run`: `Test Files  31 passed (31)`, `Tests  1081 passed (1081)` (before: 29 files, 1043 tests). Stage 4 fix round (2026-11-17, same worktree): `cd web && npx vitest run` -> `Test Files  31 passed (31)`, `Tests  1086 passed (1086)`; the guide's Verification Command is now this one (web/ has no lint script). |
| Negative cases hold | ☑ pass | RED first for the footnote (`T095 AC6` failed on the old text until the footnote changed). Found while testing: the three `AnswerBridge` tests that overrode the account list timed out (test stub bug, fixed); a second `open()` in one test duplicated the heading (split into two tests). Cases: 400 / 409 / network failure keep the card; 404 and an unknown prompt shape; a session that leaves the list with its prompt shows no line; a new prompt replaces the last line; `unreadable` offers no button |
| verify | ☑ pass | auto-run by hook, session 4b233003-7ced-4346-937c-4fbb84cf870b, commit c93395df3dfa9b629e72d1e791d84c56df2be753, log .claude/verify-runs/T107-c93395df3dfa.log — pass; done not moved: Gate 5: no test run in memory/event-trace/T107.jsonl |
| Review scope bounded to the change's blast radius | ☐ pass / ☐ fail / ☐ N/A | Stage 4 |
| Full smoke suite still green (no regression) | ☑ pass | whole web suite 1086 passed after the fix round (1081 before), 0 failed; the 29 older files unchanged except one assertion in `LiveView.test.jsx` (the footnote) |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass (see ux-evaluate table) | scratch harness (a Vite dev server outside the repo, fetch and EventSource stubbed with fixtures), headless Chrome, shots in `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/fbba027f-5d7f-408e-ba27-4a16ceab8c1f/scratchpad/harness/shots/` (`live-375.png`, `live-1280.png`, `settings-375.png`, `settings-1280.png`, `confirm-375.png`, `confirm-1280.png`) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | only `tokens.css` variables in the new CSS (`--color-accent*`, `--color-amber`, `--space-*`, `--radius-sm`, `--font-mono`, `--fs-*`); existing `.card`, `.actions`, `.field`, `button.primary`, `.muted`, `.folder`, `.field-error` reused; no new colour, font or dependency |
| **UI: Responsiveness at target viewports** | ☑ pass (375, 1280) | no horizontal scroll at 375; long command wraps inside its block; touch targets set to 44 px by CSS under 40rem (`.prompt-card button`, option rows, `.settings button`) — not measured with a probe |

### ux-evaluate (no UX rows in the guide: baseline checks only)

| # | Flow / state | Width | Verdict | Reason |
|---|---|---|---|---|
| B-a | Live card; Settings | 375 / 1280 | pass | question card: green **Send answer** (disabled until every question is answered); permission: green **Allow**; cut input: green **Deny**; Settings: **Turn on** / **Add these entries** |
| B-b | all | 375 / 1280 | pass | session row -> card -> bars; block title -> text -> rows |
| B-c | Live, Settings | 375 / 1280 | pass | first shot at 375 had status and buttons wrapping unevenly in the Settings rows; fixed (name on its own line under 40rem), re-shot `settings-375.png`, `confirm-375.png` |
| B-d | buttons, options | 375 | pass (CSS, not probed) | min-height 44 px under 40rem; option rows 44 px; seen in the shots |
| B-e | text | 375 / 1280 | not checked | 375 body is 14 px (existing app size, `--fs-body`), below the skill's 16 px; the app's own style, not changed here |
| B-f | states | 375 / 1280 | pass | on, off, changed by hand, binary moved, not listening, confirm, error, cut banner, hidden question hint, notices (tests and shots) |
| B-g | feedback | 375 / 1280 | pass | *Answer sent…*, *Already answered.*, *The terminal asks now.* lines in the row; the state word in Settings changes |
| B-h | 375 | 375 | pass | no sideways scroll, no clipped control, the entries block wraps |
| B-i | all | 375 / 1280 | pass | one accent, one button style, one radius (existing classes) |

`ux-evaluate: 8 pass, 0 fail, 1 not checked (B-e, app-wide 14 px; B-d by CSS and eyes). Overall: not pass until B-e is decided.` Not shot: the 409 / 404 notice lines and the Answer-in-terminal outcome (covered by tests, not by a screenshot).

---

## Demonstration

**BEFORE** (captured 2026-11-17T12:17:10Z on the T107 worktree at `481b9f1`, before any T107 implementation commit): the web app knows no bridge route and no `pending_answer`:

```
$ grep -rlE "api/hooks|api/answer|pending_answer" web/src; echo "grep exit=$?"
grep exit=1        # no file
$ grep -n "<h2" web/src/SettingsView.jsx
235:      <h2 id="h-agent-run-account" ref={heading} tabIndex={-1}>       # the only Settings block
$ cd web && npx vitest run     # Test Files 29 passed, Tests 1043 passed
```

**AFTER** (2026-11-17T12:27Z, same worktree): Settings has a second block, *Answer terminal sessions from the app*, and a Live row with a `pending_answer` draws a prompt card:

```
$ cd web && npm run lint
npm error Missing script: "lint"
$ npx vitest run
 Test Files  31 passed (31)
      Tests  1081 passed (1081)
```

Screens: `confirm-1280.png` (Turn on opened: the exact entries and the file), `live-375.png` (question card, permission card, cut-input card with the *Too long to show in full* banner and no Allow).

**DELTA**: new files `PromptCard.jsx` (reusable card), `PendingAnswer.jsx` (Live wrapper and the notice line), `AnswerBridge.jsx` (Settings block), three test files; `api.js` gains `getHooks`, `setHooks`, `answerPrompt`, `releasePrompt`; `LiveView.jsx` carries `pending` per session and draws the card; `SettingsView.jsx` mounts the block; `app.css` gains the card and block styles. AC 5 (a real terminal session) is the owner's gate and was not run.

---

## Stage 4 fix round (frontend agent, 2026-11-17)

`web/src/PromptCard.jsx` is now byte-identical to T104's; the `.prompt-*` and `.live-notice` rules moved from `app.css` to `web/src/appsession.css` (copied from T104, imported in `main.jsx`).

| # | Finding | Fix | Test |
|---|---|---|---|
| 1 | P1 Allow offered for a missing `complete` | T104's card: Allow only for `complete === true` and a non-empty string input | `readPrompt` > `fails closed: only complete: true is complete...`; `fails closed: no Allow when complete is missing, or the input is empty` |
| 2 | P2 footnote said "one line" | footnote: "a permission's tool name and its full tool input (the whole command, path or plan)" | `LiveView.test.jsx` `T095 AC6, T107 AC3` |
| 3 | P2 inner scroll on the input block | no `max-height` / `overflow-y` for the prompt rules in `appsession.css`, `app.css`, `watch.css` | `...in the page flow: no max-height and no inner scroll, in any CSS file` |
| 4 | P3 error carried to the next prompt | `<PendingAnswer key={s.pending.id} ...>` in `LiveView.jsx` (test fails without the key) | `a new prompt starts clean (key by prompt id)` |
| 5 | P3 503 on `/api/hooks` showed the URL text | `getHooks` shows the server's `error`, else fixed wording | `AnswerBridge.test.jsx`: `a 503 on load shows the server's reason...`, `a 503 without a reason...` |
| 6 | 409 texts, `other_copy` | `ALREADY_SERVER` stays T106's `already answered`; the incomplete-allow test uses T106's `INCOMPLETE` text; `other_copy` shown as "Another copy of the app" (Turn on replaces, Turn off removes) | `a 409 that is not "already answered"...`; `shows another copy of the app as such...` |
