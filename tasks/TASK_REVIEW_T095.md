# TASK_REVIEW — T095: Waiting sessions 2 — Live view shows *your turn*, the question text, and the session the app asks for

> Sibling of `tasks/TASK_GUIDE_T095.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. The click mechanism is a query on the window address, `/?live=<session id>#<token>`; the page reads it once and strips it — RULE: tasks/TASK_REVIEW_T095.md § Auto decisions (the Debate was not settled and the run paused: `Paused: unsure — how does a click on a tray row or a popup tell the page which session to show?; options: a query on the window address / the address fragment / a fixed script run in the window`; the Supervisor put its recommendation, the query, to the owner as option 1; the owner's reply of 2026-11-16 was "continue your work [admin]", with no option named, and the Supervisor read it as taking the recommendation)
2. The top-bar chip keeps its count and label and now counts a *your turn* session, as it counts every state — FACT: web/src/StatusChips.jsx:56 "const waiting = sessions?.filter((s) => statesOf(s).length > 0).length"
3. `blast-radius` not run: the page stores and sends nothing — RULE: `docs/claude-md/pipeline-stages.md` § Stage 4: Review (item 3)
4. Companion score written as the run gave it, `fail` — RULE: `skills/score/SKILL.md` § Never ("A run that happened is binding")
Checks: tests `cd web && npm test` — `Test Files 27 passed (27)`, `Tests 957 passed (957)` (Supervisor, 2026-11-16, commit c70f26d); `python3 scripts/check_static.py` — `check_static: ok`; reviews code-review — no P0, two selection races and one visible flaw fixed in two fix rounds (`5d7ea0b`, `c70f26d`), 1 P1 process finding (startup reads, partly remedied), `reports/code-review_T095_20261116T180800.html`; security-review — no finding, `reports/security-review_T095_20261116T180900.html`; ux-evaluate (reviewer pass on 4 screenshots at 375 px and 1280 px) — UX rows 1-4 pass, row 5 (empty state) not checked by image, baseline 7 pass and 2 fail (older); real run: the page on a fixture stream in headless Chrome, `tasks/evidence-T095/after-selected-375.png` — shows the `your turn` badge, two questions as text in the open panel, the selected row (auto-checked, not /verify)
Memory: `memory/learnings.md` — "2026-11-16 — T095 Live view: a wanted session from the address, and review habits"; `memory/MEMORY.md` — its index line
Not done: merge (yours: reply `T095 [admin]` or merge it yourself; the merge gate does not accept a request that names no task); auto verify: ☑ pass (session 4bc64e43-a18b-4f22-aeae-59fbf25ea545, commit f5f839fef1273039ec7969ad6e735cb136f094b7, log .claude/verify-runs/T095-f5f839fef127.log), after T098 gave the recipe its verdict line; not seen in a browser: a `focus` message from the stream (unit tests only; T096 sends it)
Paused: auto-mode check failed — no `Memory:` line in the report — run the memory pass before you stop and record it there (each entry written, or `Memory: nothing new`); no test run in memory/event-trace/T095.jsonl — write /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d/$CLAUDE_CODE_SESSION_ID with the task ID (minimal-track step 1): `mkdir -p /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d && printf '%s\n%s\n' "T095" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d/$CLAUDE_CODE_SESSION_ID`, then rerun the task's tests so the trace records them under the task, then stop [report 503938760e18]
Paused: done not moved — Gate 5: no test run in memory/event-trace/T095.jsonl; report incomplete: auto verify slot

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/LiveView.test.jsx` (describe "your turn and the question text (T095, FR-108)", AC1-4, footnote AC6, redraw on questions only, wrap CSS rule; design test now lists `your_turn`) and `web/src/App.test.jsx` (describe "the session the page is asked for (T095, FR-108)", AC5: listed id, 64-char id, later snapshot within 10 s, give-up after 10 s, select once, leaves the list, no id / empty / unknown / script / 65-char / non-hex, no parameter, `readToken()` = `tok`) |
| Verification command run | ☑ pass | Reviewer (Supervisor), 2026-11-16, commit c70f26d: `cd web && npm test` → `Test Files  27 passed (27)` / `Tests  957 passed (957)`; `python3 scripts/check_static.py` → `check_static: ok`; implementing agent: `npm run build` → `built in 192ms`, `exit=0` |
| Negative cases hold | ☑ pass | Red before the code: LiveView.test.jsx `9 failed, 100 passed (109)`; App.test.jsx `-t T095`: `12 failed, 2 passed`. Green after: LiveView + App `170 passed`. Not a mutation run. |
| verify | ☑ pass | auto-run by hook, session 4bc64e43-a18b-4f22-aeae-59fbf25ea545, commit f5f839fef1273039ec7969ad6e735cb136f094b7, log .claude/verify-runs/T095-f5f839fef127.log — pass; done not moved: Gate 5: no test run in memory/event-trace/T095.jsonl |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed: `web/src/App.jsx`, `web/src/LiveView.jsx`, `web/src/app.css`, their three test files, and the callers `web/src/StatusChips.jsx` (reads `statesOf`) and `web/src/api.js` (`readToken`). Skipped: every other view, `rust/`, Python. `lane_check`: `lane: C2, 16 files, 569 lines, EXCEEDS (files 16 > 14; lines 569 > 500)` — 10 of the 16 files are evidence |
| Full smoke suite still green (no regression) | ☑ pass | all 27 web test files, 957 tests, green after the two fix rounds; Python tests not run (no Python changed) |
| **Companion score (easy-verifier)** | ☑ fail | overall 63 (floor missed: 63 < 80), change_risk 0 (feature): failed rule `new_code_without_test_share` 0.33 > 0.2, computed from `tasks/evidence-T095/fixture_server.py`, scope changes, ref `main...HEAD`, abstentions: solution-fit (rated only at the evaluate gate), reliability, supply-chain, api, data, operations, infrastructure, gates: change_kind and test_pairs answered in the report call (3 pairs), solution-fit left abstained, findings not sent, contributors: architecture 100, test-strategy 72, code-quality 67, security 59, blast-radius 46, requirement-fidelity 33. Reviewer's reading, not a change of the result: the failed rule counts the screenshot fixture server as untested new code. report: `/home/hungnguyenhuu/.easy-verifier-reports/T095/evidence-report-changes-20261116T110731-793118Z.html` |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | BEFORE `tasks/evidence-T095/before-1280.png`; AFTER `tasks/evidence-T095/after-1280.png`, `after-selected-1280.png`, `after-selected-375.png`, `after-375-long.png`, `after-unknown-1280.png`, `after-unknown-375.png`: the `your turn` badge, the question lines and the selected row use the existing pill, text and card styles; nothing else moved (read by LLM vision) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `grep -rnE "#[0-9a-fA-F]{3,8}\b or rgba?\(" web/src --include=*.jsx` outside tests: only the pre-existing `placeholder="#0b5fd6"` in WorkspacesView.jsx; new CSS rules use only `var(--…)`; computed question line: `14px rgb(220, 224, 221) "IBM Plex Sans"` = `--fs-body` / `--color-text`; design test pins `your_turn` rules |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 and 1280: `scrollWidth == innerWidth` (375/375, 1280/1280) with a 300-character unbroken question (`after-375-long.png`) and with the panel open |

### Stage 4 fix round

P2 (the open row showed its questions twice): `web/src/LiveView.test.jsx` "Stage 4 fix: an open row holds each question once (in the panel); closing it shows them in the row again". Red before the fix (`expected [ 'A?', 'B?', 'A?', 'B?' ] to deeply equal [ 'A?', 'B?' ]`), green after (`Tests 947 passed (947)`, build ok, `check_static: ok`, exit=0). `after-selected-1280.png` and `after-selected-375.png` retaken: the questions show once, in the panel.

Round 2 (independent review). Each test was run red before its code (7 red for the right reasons: `expected [ 'T?' ] to deeply equal []`, `expected [ '1?', ..., '6?' ] to deeply equal [ '1?', '2?', '3?', '4?' ]`, `expected "vi.fn()" to not be called at all, but actually been called 1 times`, and for fixes 1-3 `aria-current`/`aria-expanded` not as expected), then green: `Tests 957 passed (957)`, build ok, `check_static: ok`, exit=0.
1. The 10 s limit runs from the first snapshot (`loaded`), App.test.jsx: "Stage 4 round 2 (1): the 10 s limit runs from the first snapshot: one that arrives at 11 s with the id selects it", "(1): the id first listed 9 s after a first snapshot (that came at 11 s) is selected; ...", "(1): 10.5 s after a first snapshot ... selects nothing". Red: the first two; green: all three.
2. A user toggle ends the wish, App.test.jsx "(2): the user opening a row ends the wish ...". Red, then green.
3. Leaving Live drops a pending wish, App.test.jsx "(3): leaving Live drops a pending wish ...". Red, then green.
4. Scroll only for the address selection: LiveView.test.jsx "(4): a normal click that opens a row does not scroll" (red: called 1 times) and App.test.jsx "AC5: the row is scrolled into view, once" (`toHaveBeenCalledTimes(1)`, also after a later snapshot). Green.
5. Questions only from a waiting state, at most 4, each 300 characters, blanks dropped: LiveView.test.jsx "(5): questions are read only from a waiting state" and "(5): at most the first 4 strings show, each cut to 300 characters, and an empty or blank one is dropped". Red, then green.
6. Missing tests: LiveView.test.jsx "(6): a later snapshot that drops the questions, or the key, leaves no question line" (passed at once, a guard); StatusChips.test.jsx "T095: a session whose only state is your_turn is counted in the chip, and is never idle" (passed at once, a guard); the `scrollIntoView` stubs are now removed in `finally`.

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (2026-11-16T17:54:28+07:00, main code at 7f3aaef built, before any implementation commit): `tasks/evidence-T095/before-1280.png`, the Live view fed by `tasks/evidence-T095/fixture_server.py full` (a `your_turn` session `cccccccc3`, a `waiting` session `bbbbbbbb2` with 2 `questions`). Real measure output: `{"scrollW":1280,"innerW":1280,"view":"Live","pills":["waiting for you: question"],"questions":[],"open":[],"head":"1 session needs you"}`. It shows one pill (`waiting for you: question`) with no question text, and the `your_turn` session is drawn as idle (`idle · 30 min age`) with no badge.

**AFTER** (2026-11-16, same fixture server, built page): `tasks/evidence-T095/after-1280.png`, `after-selected-1280.png`, `after-selected-375.png`, `after-375-long.png`. Measure output of the same feed: `{"view":"Live","pills":["waiting for you: question","your turn"],"questions":["Which database should the migration targ…","Keep the old column or drop it?"],"head":"2 sessions need you","scrollW":1280,"innerW":1280}`; on `/?live=bbbbbbbb2#tok`: `"current":["live-row"],"open":["beta"],"url":"http://127.0.0.1:<port>/#tok"` (query stripped, fragment kept); on `?live=deadbeef`: `"current":[],"open":[]`; at 375 `scrollW 375` also with a 300-character unbroken question.

**DELTA**: The Live view now shows a `your turn` badge, shows the question a waiting session asks, and opens on the session the page is asked for.

**WITNESS**: BEFORE and AFTER screenshots were taken by the implementing agent (trace `memory/event-trace/T095.jsonl`); the Supervisor ran the page tests on commit c70f26d on 2026-11-16, read the diff and opened four of the screenshots; an independent read-only reviewer traced the text, address and token paths.
