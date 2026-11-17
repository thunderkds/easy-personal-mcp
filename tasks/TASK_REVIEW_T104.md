# TASK_REVIEW — T104: Write sessions 2 — app session page: New session form, session panel, prompt cards

> Sibling of `tasks/TASK_GUIDE_T104.md`. Stage 3 parts only (frontend agent, 2026-11-17). The Stage 4/5 rows are not filled here.

---

## Evidence

(filled at the end of Stage 3)

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
