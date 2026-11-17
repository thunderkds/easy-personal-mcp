# TASK_REVIEW — T104: Write sessions 2 — app session page: New session form, session panel, prompt cards

> Sibling of `tasks/TASK_GUIDE_T104.md`. Stage 3 parts only (frontend agent, 2026-11-17). The Stage 4/5 rows are not filled here.

---

## Auto decisions

Mode: auto
1. Allow (tool) and Approve (plan) are offered only when `complete === true` and the shown text is non-empty; otherwise only Deny / Keep planning. RULE: `PRD.md:212` FR-115 "a cut prompt is marked `complete: false` and can only be denied or handed to the terminal"
2. A pending prompt card is drawn in page flow below the conversation log, never inside its scroll box. RULE: `PRD.md:212` FR-115 "a cut or one-line summary let the owner allow a command whose tail they never saw"
3. `bypassPermissions` needs a second, explicit confirm step after the Review step. RULE: `tasks/TASK_GUIDE_T104.md` § Acceptance Criteria AC 1 "*bypass permissions* mode needs a second, explicit confirm"
4. Every write sends only the form fields (checked against fixed lists and stored repos) plus `confirmed: true`; the token is only in a header. RULE: `PRD.md` § Functional Requirements FR-116 "No route lets the page name the program or free arguments"
Checks: tests `cd web && npx vitest run` — Test Files 33 passed (33), Tests 1141 passed (1141); reviews code-review + security (independent, 0 P0/P1/P2, 4 P3 + 5 minor, all fixed in `f686d9f`; 2 backend items handed to T103 F7/F8); real run: not done — a real app session needs the owner (AC 7)
Memory: nothing new (card rules recorded in memory/decisions.md, T106 entry)
Not done: merge (admin: after the auto verify pass) — merge after T103 and T107, taking T104's `web/src/PromptCard.jsx`; auto verify: ☑ pass (session eb1c3e48-4d7f-41c8-9933-0b2fbc019c30, commit e2481b465e20e050cc3c027582e710dc82b43b4c, log .claude/verify-runs/T104-e2481b465e20.log); owner real run (AC 7)

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass (AC 1-6; AC 7 is the owner's real run) | `web/src/NewSession.test.jsx` (AC 1: fixed model and mode lists, stored repos only, confirm names account and mode, bypass needs a second step, one POST; Stage 4 fix: size limit counted in UTF-8 bytes); `web/src/AppSessions.test.jsx` (AC 2-4: header, stream, Enter / Shift+Enter, Interrupt / Stop / Close, Allow / Deny + note, question, plan, 409, no token in a URL, no localStorage; Stage 4 fixes: a slow poll cannot bring back an answered card, a pending card is outside the log, Safari keyCode 229, text typed during a send kept, byte limit on a message, plan without `complete` has no Approve); `web/src/PromptCard.test.jsx` (AC 3: both sources to one shape, Allow only when `complete === true` and a non-empty input, Approve gated the same way); `web/src/AppSessionEntry.test.jsx` (Workspaces New session, Live Open in app, Stage 4 fix: a later read gives Open in app to a row whose session id arrived late, at most one read per 5 s) |
| Verification command run | ☑ pass (lint: no script) | `npm run lint` -> `Missing script: "lint"`; `cd web && npx vitest run` after the Stage 4 fix round: `Test Files  33 passed (33)`, `Tests  1141 passed (1141)` (Stage 3: 1128 tests). The stale-poll test was run with its guard removed and fails, with it passes |
| verify | ☑ pass | auto-run by hook, session eb1c3e48-4d7f-41c8-9933-0b2fbc019c30, commit e2481b465e20e050cc3c027582e710dc82b43b4c, log .claude/verify-runs/T104-e2481b465e20.log — pass; done not moved: Gate 5: the tests row is not `☑ pass` with pasted output; Gate 5: no test run in memory/event-trace/T104.jsonl |
| Review scope | Stage 4 | |
| **UI: Visual regression** | ☑ pass (see table) | scratch harness outside the repo (Vite dev server, `fakeFetch` + app session stub), headless Chrome; shots in `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/fbba027f-5d7f-408e-ba27-4a16ceab8c1f/scratchpad/h104/shots/` (`panel`, `question`, `plan`, `cut`, `ended`, `form`, `confirm`, `bypass`, `live`, `off`, `workspaces`, each -375 and -1280) |
| **UI: Design-system compliance** | ☑ pass | `appsession.css` uses `tokens.css` variables only; existing `.card`, `.actions`, `.field`, `button.primary`, `.danger`, `.muted`, `.mono`; no new colour, font or dependency |
| **UI: Responsiveness** | ☑ pass (375, 1280) | no sideways scroll; long command wraps in page flow; 44 px targets by CSS under 40rem (not probe-measured) |

### ux-evaluate (no UX rows in the guide: baseline checks only)

| # | Verdict | Reason |
|---|---|---|
| B-a | pass | first shot had two green buttons with a prompt waiting; New session is now plain once a session exists, so Allow / Approve / Send answer is the one primary |
| B-b | pass | title, session header, conversation, card, message box |
| B-c | pass | the session tab was a tall box at 375; centred (not re-shot) |
| B-d | pass by CSS and eyes | min-height 44 px under 40rem; not probed |
| B-e | not checked | body 14 px is the app's own size |
| B-f | pass | form, confirm, bypass, waiting, ended, not available (shot); off, no claude, limit, load error (tests) |
| B-g | pass | state line is a polite status; errors are alerts |
| B-h | pass | `panel-375`, `question-375`, `cut-375`, `bypass-375`, `live-375` |
| B-i | pass | one accent, one button style |

Not read: `workspaces-375/1280.png` were shot but the tool-call step limit stopped the run before they were opened (the button is covered by tests). `off-*` shows "not available" because the harness did not serve `/api/agent-run`; the off state is a test. The two B-a / B-c edits were made after the shots and were not shot again.

---

## Demonstration

**BEFORE** (captured 2026-11-17T12:37:05Z on the T104 worktree at `4eef19f`, before any T104 implementation commit): the web app knows no app session route and has no session page:

```
$ grep -rlE "api/app-sessions" web/src; echo "grep exit=$?"
grep exit=1        # no file
$ ls web/src | grep -iE "session|prompt"
(no output)        # no AppSessions page, no PromptCard
$ cd web && npm run lint
npm error Missing script: "lint"          # web/package.json has only build and test
$ npx vitest run
 Test Files  29 passed (29)
      Tests  1043 passed (1043)
```

**AFTER** (2026-11-17, same worktree): `cd web && npx vitest run` -> `Test Files  33 passed (33)  Tests  1128 passed (1128)`. New page `App sessions` under `// work`; New session on a workspace card; Open in app on a Live row.

**DELTA**: new `AppSessions.jsx`, `NewSession.jsx`, `PromptCard.jsx` (same exports as T107), `appsession.css`; `api.js` + 8 app session helpers; `LiveView.jsx`, `WorkspacesView.jsx`, `App.jsx`, `main.jsx` small edits. AC 7 (real run) not run.
