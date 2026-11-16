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
Checks: <filled at Stage 4/5>
Memory: <filled before the run ends>
Not done: merge (admin: after the auto verify pass); auto verify: <not started>
Paused: auto-mode check failed — no `Memory:` line in the report — run the memory pass before you stop and record it there (each entry written, or `Memory: nothing new`); no test run in memory/event-trace/T095.jsonl — write /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d/$CLAUDE_CODE_SESSION_ID with the task ID (minimal-track step 1): `mkdir -p /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d && printf '%s\n%s\n' "T095" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d/$CLAUDE_CODE_SESSION_ID`, then rerun the task's tests so the trace records them under the task, then stop [report 503938760e18]

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/LiveView.test.jsx` (describe "your turn and the question text (T095, FR-108)", AC1-4, footnote AC6, redraw on questions only, wrap CSS rule; design test now lists `your_turn`) and `web/src/App.test.jsx` (describe "the session the page is asked for (T095, FR-108)", AC5: listed id, 64-char id, later snapshot within 10 s, give-up after 10 s, select once, leaves the list, no id / empty / unknown / script / 65-char / non-hex, no parameter, `readToken()` = `tok`) |
| Verification command run | ☑ pass | `cd web && npm test` ->  Test Files  27 passed (27) /       Tests  946 passed (946) /  / ✓ built in 154ms / check_static: ok / exit=0 (static check prints `check_static: ok`, exit=0) |
| Negative cases hold | ☑ pass | Red before the code: LiveView.test.jsx `9 failed, 100 passed (109)`; App.test.jsx `-t T095`: `12 failed, 2 passed`. Green after: LiveView + App `170 passed`. Not a mutation run. |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | left to the reviewer (code-review at Stage 4); blast radius: `statesOf` is also read by `web/src/StatusChips.jsx` (the "N waiting" chip now counts a `your_turn` session too) |
| Full smoke suite still green (no regression) | ☑ pass | all 27 web test files, 946 tests, green; Python tests not run (no Python changed) |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | BEFORE `tasks/evidence-T095/before-1280.png`; AFTER `tasks/evidence-T095/after-1280.png`, `after-selected-1280.png`, `after-selected-375.png`, `after-375-long.png`, `after-unknown-1280.png`, `after-unknown-375.png`: the `your turn` badge, the question lines and the selected row use the existing pill, text and card styles; nothing else moved (read by LLM vision) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `grep -rnE "#[0-9a-fA-F]{3,8}\b or rgba?\(" web/src --include=*.jsx` outside tests: only the pre-existing `placeholder="#0b5fd6"` in WorkspacesView.jsx; new CSS rules use only `var(--…)`; computed question line: `14px rgb(220, 224, 221) "IBM Plex Sans"` = `--fs-body` / `--color-text`; design test pins `your_turn` rules |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 and 1280: `scrollWidth == innerWidth` (375/375, 1280/1280) with a 300-character unbroken question (`after-375-long.png`) and with the panel open |

### Stage 4 fix round

P2 (the open row showed its questions twice): `web/src/LiveView.test.jsx` "Stage 4 fix: an open row holds each question once (in the panel); closing it shows them in the row again". Red before the fix (`expected [ 'A?', 'B?', 'A?', 'B?' ] to deeply equal [ 'A?', 'B?' ]`), green after (`Tests 947 passed (947)`, build ok, `check_static: ok`, exit=0). `after-selected-1280.png` and `after-selected-375.png` retaken: the questions show once, in the panel.

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (2026-11-16T17:54:28+07:00, main code at 7f3aaef built, before any implementation commit): `tasks/evidence-T095/before-1280.png`, the Live view fed by `tasks/evidence-T095/fixture_server.py full` (a `your_turn` session `cccccccc3`, a `waiting` session `bbbbbbbb2` with 2 `questions`). Real measure output: `{"scrollW":1280,"innerW":1280,"view":"Live","pills":["waiting for you: question"],"questions":[],"open":[],"head":"1 session needs you"}`. It shows one pill (`waiting for you: question`) with no question text, and the `your_turn` session is drawn as idle (`idle · 30 min age`) with no badge.

**AFTER** (2026-11-16, same fixture server, built page): `tasks/evidence-T095/after-1280.png`, `after-selected-1280.png`, `after-selected-375.png`, `after-375-long.png`. Measure output of the same feed: `{"view":"Live","pills":["waiting for you: question","your turn"],"questions":["Which database should the migration targ…","Keep the old column or drop it?"],"head":"2 sessions need you","scrollW":1280,"innerW":1280}`; on `/?live=bbbbbbbb2#tok`: `"current":["live-row"],"open":["beta"],"url":"http://127.0.0.1:<port>/#tok"` (query stripped, fragment kept); on `?live=deadbeef`: `"current":[],"open":[]`; at 375 `scrollW 375` also with a 300-character unbroken question.

**DELTA**: The Live view now shows a `your turn` badge, shows the question a waiting session asks, and opens on the session the page is asked for.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
