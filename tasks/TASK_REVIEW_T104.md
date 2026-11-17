# TASK_REVIEW — T104: Write sessions 2 — app session page: New session form, session panel, prompt cards

> Sibling of `tasks/TASK_GUIDE_T104.md`. Stage 3 parts only (frontend agent, 2026-11-17). The Stage 4/5 rows are not filled here.

---

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria** | pass (AC 1-6; AC 7 is the owner's real run) | `web/src/NewSession.test.jsx` (AC 1: fixed model and mode lists, stored repos only, confirm names account and mode, bypass needs a second step, one POST); `web/src/AppSessions.test.jsx` (AC 2-4: header, stream, Enter / Shift+Enter, Interrupt / Stop / Close, Allow / Deny + note, question, plan, 409, no token in a URL, no localStorage); `web/src/PromptCard.test.jsx` (AC 3: both sources to one shape, Allow only when `complete === true` and a non-empty input); `web/src/AppSessionEntry.test.jsx` (Workspaces New session, Live Open in app) |
| Verification command run | pass (lint: no script) | `npm run lint` -> `Missing script: "lint"`; `cd web && npx vitest run`: `Test Files  33 passed (33)`, `Tests  1128 passed (1128)` (before: 29 files, 1043 tests). Two assertions in `App.test.jsx` gained the new `App sessions` tab |
| verify | leave | the auto verify hook writes this row |
| Review scope | Stage 4 | |
| **UI: Visual regression** | pass (see table) | scratch harness outside the repo (Vite dev server, `fakeFetch` + app session stub), headless Chrome; shots in `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/fbba027f-5d7f-408e-ba27-4a16ceab8c1f/scratchpad/h104/shots/` (`panel`, `question`, `plan`, `cut`, `ended`, `form`, `confirm`, `bypass`, `live`, `off`, `workspaces`, each -375 and -1280) |
| **UI: Design-system compliance** | pass | `appsession.css` uses `tokens.css` variables only; existing `.card`, `.actions`, `.field`, `button.primary`, `.danger`, `.muted`, `.mono`; no new colour, font or dependency |
| **UI: Responsiveness** | pass (375, 1280) | no sideways scroll; long command wraps in page flow; 44 px targets by CSS under 40rem (not probe-measured) |

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
