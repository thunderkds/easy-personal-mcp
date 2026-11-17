# TASK_REVIEW — T105: Write sessions 3, Resume in app, fork, Continue (backend half)

> Sibling of `tasks/TASK_GUIDE_T105.md`. Stage 3 parts only (the Supervisor fills Stage 4/5). This file covers AC 1, AC 2 and the Rust part of AC 4; the page buttons (AC 3) come in a second step.

## Demonstration

**BEFORE** (captured 2026-11-17T15:39:30Z on branch T105 at commit e34dfc0, before any implementation commit; default-feature debug build; temp `EASY_PERSONAL_HOME`, a stand-in `claude` on `PATH`, agent runs on):

```
$ easy-personal-rs serve      # dashboard: http://127.0.0.1:33413/#<token>
$ POST /api/app-sessions resume (quiet session)    {"resume":"aaaa1111-quiet","model":"default","mode":"default","prompt":"hi","confirmed":true}
{"error":"unknown field"} [400]
$ POST /api/app-sessions continue                  {"continue":true,"repo":"<quiet-repo>","workspace":"acme",...,"confirmed":true}
{"error":"unknown field"} [400]
```
