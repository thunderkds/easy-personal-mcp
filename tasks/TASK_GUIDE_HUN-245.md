> Board ID: HUN-245. Created from the draft on 2026-11-09.
> Split out of HUN-230 note 3, "button to send command to claude", at Stage 2 on 2026-11-08.

**Date**: 2026-11-08
**Complexity Level**: C2
**Risk Level**: High
**Priority**: P2
**Assigned agent**: Backend-Implementer (the button is a second, dependent Frontend task — see Step 0)
**Agent guide**: `agents/backend.md`

---

## Step 0 — STOP: the execution model is not decided (do this before any code)

This is the first time the dashboard would run another program. Three models were on the table when
this draft was written, and the owner has not chosen:

1. **Run it** — the server runs `claude -p "<command>"` headless in a chosen workspace repo and shows
   the output. Does what the owner literally asked; gives a loopback HTTP surface the power to start a
   process.
2. **Copy it** — the button copies the command to the clipboard and runs nothing. Safest by far, but it
   does not "send" anything, so it may not be what the owner wants at all.
3. **Queue it** — the button appends the command to a file that the owner's own live session picks up.
   Nothing is executed by the dashboard, but a consumer must be built in the Easy Kit kernel, which is
   another repo (a cross-repo dependency, XR).

**Before writing any code**, the Supervisor must settle this with the owner, or — in auto mode — by a
Debate with 3 Advocates (Risk High, `docs/claude-md/tracks.md` `### Debate size by Risk`), one per model.
Record the answer in PRD question 34 and in this guide's Questions Log, then rewrite the Requirement
below to match it and re-sign the Requirement Fidelity Gate. The Requirement below assumes model 1,
because that is the Supervisor's default recorded in FR-092; **it is not an owner decision.**

Once the model is settled, split this draft at Stage 2 into a backend task (the route) and a dependent
frontend task (the button), the way HUN-242 and HUN-243 are split.

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (Critical Constraints: metadata only, no traffic leaves the machine; the glossary term Workspace)
2. Read the memory slice in your spawn prompt. Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely, Step 0 first
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-092, US-038, NFR-002, NFR-008, open question 34, and the Non-Goals line for FR-092; `src/easy_personal/surfaces/api.py` (the `Guard` middleware, every existing `POST` route and how it validates a body); `src/easy_personal/core/workspaces.py` (`list_all`, `project_paths` — the only source of a repo path); `src/easy_personal/core/lock.py` (the single-writer lock, FR-038); `src/easy_personal/core/launcher.py` or the module that already shells out to the `claude` CLI, and how it passes env; `tests/leak/harness.py` (`API_CALLS`)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-08, note 3 of HUN-230: "button to send command to claude".

**Restated intent** (assumes execution model 1; see Step 0):

> **(a)** `POST /api/claude/run`, under the same `Guard` (Host, Origin, header token) as every other
> `/api/*` route, token in the header only. No change to the `Guard`. The body carries exactly two
> fields: `command_key` and `workspace`. Nothing else in the body is read.
>
> **(b)** `command_key` names one entry of a **fixed allow-list held in code** — not a free-text
> command, and never a string from the request. Each entry holds the exact argument list to pass. An
> unknown key is 400 with a fixed body and no echo of the input.
>
> **(c)** `workspace` names a non-orphan stored workspace; the repo path comes from the stored
> workspaces, never from the request. An unknown or orphan workspace is 404 with a fixed body.
>
> **(d)** The server runs the `claude` CLI once, with the entry's argument list passed as a list —
> **no shell, ever** (`shell=False`, no `shell=True`, no string command). It runs in the resolved repo
> directory, with a time limit, and one run at a time under the existing single-writer lock (FR-038). A
> second request while one runs is 409 with a fixed body.
>
> **(e)** The answer carries the exit code, whether the time limit was hit, and the captured output,
> capped at a fixed size. The page shows it read-only.
>
> **Hard rules**: the command text, its output and the repo path are never stored and never logged
> (NFR-002); the output goes only to the response of the request that asked for it. No store write, no
> migration. No secret is passed in an argument or an environment variable the entry does not name. The
> run is killed when the time limit passes, and a killed run reports `timed_out`, never a traceback.
> `GET` on this path is 405.

**Out of scope**: a free-text command box; a shell; more than one run at a time; storing or logging the
command or its output; running any program other than the `claude` CLI; a schedule or a timer; streaming
the output live; cancelling a run from the page; an MCP tool; a CLI command.

**Requirement Refs**: FR-092, US-038, NFR-002, NFR-008

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [ ] **BLOCKED** — Restated intent cannot be confirmed until Step 0 settles the execution model. Do not sign this gate, and do not write code, before then.
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [ ] Every Acceptance Criterion below traces to a line in the Requirement — recheck after Step 0
- [X] All Requirement Refs exist in `PRD.md`

---

## Dependencies & Reachability

**Depends on**: Step 0 (the owner's or a Debate's choice of execution model). The frontend button is a
second task that depends on this route being merged.

**Entry point**: `POST /api/claude/run` in `src/easy_personal/surfaces/api.py`

---

## Acceptance Criteria

> These assume execution model 1. Rewrite them if Step 0 chooses another model.

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | A known `command_key` plus a known workspace runs the CLI once, in that workspace's repo directory, and the answer carries the exit code and the captured output | FR-092 |
| 2 | The process is started with an argument **list** and `shell=False`; a test asserts no shell is used and that no value from the request body reaches the argument list except through the allow-list lookup and the stored workspace path | FR-092, NFR-002 |
| 3 | Injection attempts in the body each give 400 or 404 with a fixed body and no echo: an unknown `command_key`; a `command_key` holding `;`, `&&`, a backtick, `$(...)`, a NUL byte or a newline; extra body fields; a `command` or `args` field; a workspace name holding a path, `..` or a NUL byte; an absolute path as the workspace | FR-092, NFR-002 |
| 4 | An unknown workspace, an orphan workspace and a workspace whose repo folder is gone each give 404 with the fixed body | FR-092 |
| 5 | One at a time: a second request while a run is in flight gives 409 with a fixed body, and the first run is unaffected | FR-092 |
| 6 | The time limit: a command that runs past it is killed, the answer says `timed_out`, the exit state is reported and no traceback appears; the process leaves no child behind | FR-092 |
| 7 | Output over the cap is truncated at the cap and marked truncated; the output is never written to the store, to a file or to a log (assert the log output and the store hash) | FR-092, NFR-002 |
| 8 | Route: 200 with the token; 401/403 as the other routes without it, with a wrong Host or a foreign Origin; `GET` → 405; the token is not accepted from a `token` query parameter | FR-092, NFR-008 |
| 9 | Leak: a planted secret in the environment, in the repo path and in the command's own output never appears in any log line, any error body or the store; the route is registered in `tests/leak/harness.py` (`API_CALLS`) | NFR-002 |
| 10 | No store write and no migration: the store file hash is unchanged after a run | FR-092 |
| 11 | Existing tests pass; `uv run pytest -q` passes. Real run: temp `EASY_PERSONAL_HOME`, one stored workspace with a real repo, `easy-personal dashboard`, `curl` with `X-Dashboard-Token` for an allowed key, an unknown key, a second concurrent request, and a no-token try; paste the real output | — |

---

## UI / Design Acceptance Criteria

N/A for this route task — the button is the dependent Frontend task, and the three UI Evidence rows
are filled there, not here. UI Evidence rows ☐ N/A on this task.

---

## Evaluation & Acceptance

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<ID>.md`.
> Risk High: Stage 4 runs `code-review` **and** `security-review`, and `blast-radius` applies because
> the route can start a process.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core tests/surfaces tests/leak -q && uv run pytest -q
```

## Approach

**Vital slice**: the allow-list table in code, the workspace lookup, one `subprocess` call with a list
and no shell, the lock, the size and time caps, and the attacking tests. **Cut list**: no free text, no
streaming, no cancel, no history, no schedule, no second program, no MCP tool, no CLI command.

---

## Edge Case Checklist

- [ ] `claude` not on the PATH → a clear answer, no traceback
- [ ] The CLI prompts for a login → the time limit kills it; the answer says `timed_out`
- [ ] The CLI writes a very large amount of output → capped, truncated, marked
- [ ] The repo folder is gone, or is a symlink, between the lookup and the run → 404, no run
- [ ] Two requests at the same moment → exactly one runs, the other 409
- [ ] The run leaves a child process → killed with the process group, nothing orphaned
- [ ] The dashboard is stopped while a run is in flight → no orphan, no partial write
- [ ] Planted secrets (NFR-002) never in logs, error bodies or the store

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/claude_run.py` (new) | the allow-list, the lookup, the no-shell run, the caps |
| `src/easy_personal/surfaces/api.py` | the `POST /api/claude/run` route |
| `tests/core/test_claude_run.py` (new) | AC 1–7, 10 |
| `tests/surfaces/test_api_claude_run.py` (new) | AC 3, 8 |
| `tests/leak/harness.py`, `tests/leak/` | AC 9 |
| `README.md`, `site/` | only if a drift test requires the new route to be listed |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `Guard` class in `api.py` | The token rule stays: header only |
| `src/easy_personal/sources/`, `src/easy_personal/core/migrations/`, `src/easy_personal/core/store.py` | No ingest or store change |
| `web/` | The button is the dependent Frontend task |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

## Completion Checklist

- [ ] Step 0 settled and the Requirement rewritten to match
- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table
- [ ] UI Evidence rows ☐ N/A (the button task fills them)
- [ ] Supervisor notified: task ready for Stage 4 review

## Questions Log

* 2026-11-08 Supervisor reading of the owner's words "button to send command to claude" → FR-092, an allow-listed headless `claude -p` run in a chosen workspace repo (PRD question 34). The execution model is **not** decided: Step 0 blocks this task until the owner chooses, or a 3-Advocate Debate settles it ⚠ decided for you — please review