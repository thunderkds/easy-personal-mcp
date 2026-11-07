# TASK_GUIDE — HUN-230: Epic — three owner notes, split at Stage 2

**Date**: 2026-11-08
**Complexity Level**: C3
**Risk Level**: High
**Priority**: P2
**Assigned agent**: none — this row is an Epic and is never picked up
**Agent guide**: n/a

> **Epic, not a task.** A task above C3 must be split at Stage 2 before pickup
> (`AGENTS.md` § Permanent Rules). This row held three unrelated scopes, so it was split on
> 2026-11-08 and no agent works this row.

---

## The owner's note, verbatim

```
1. push online
2. check db effectively
3. button to send command to claude
```

---

## How it was split

| Note | Where it went now | Why |
| -- | -- | -- |
| 1. "push online" | **No task.** It is an operational step, not code: `main` holds 84 commits that are not on `github/main` (last pushed `700c273`). The remote is `git@github.com:thunderkds/easy-personal-mcp.git`, named `github`, and `main` tracks `github/main`. The owner pushes with `git push github main`. | The project's recorded git flow is "local merge to main, push in batches" (`memory/decisions.md` § 2026-10-21 — Stage 3–5 git flow). Nothing needs building. A push is also never done in auto mode (`docs/claude-md/tracks.md` § Never in auto mode), so it stays the owner's action. |
| 2. "check db effectively" | **`tasks/DRAFT_store_health.md`** — FR-091, a read-only store health check on the CLI and the dashboard. C2, Risk Medium, P1. | Read as a health check, not a database browser and not a repair tool: the smaller change, and it writes nothing, so it can be undone. Recorded as PRD question 33, open for owner review. |
| 3. "button to send command to claude" | **`tasks/DRAFT_claude_command_button.md`** — FR-092, a dashboard button that sends a command to Claude Code. C2, Risk High, P2. Its Step 0 blocks all code until the execution model is chosen. | This would be the first time the dashboard runs another program, so the execution model (run it / copy it / queue it) is an owner decision. Recorded as PRD question 34, open and needing an owner decision. |

---

## Why the two new tasks have no ID yet

The board is unreachable and `LINEAR_API_KEY` is not set in the session's shell, so no issue could be
created. A new task's ID comes from the board and a create cannot be queued:

> `scripts/board_sync.py:374-376` — "A new task's ID comes from the board, so a create cannot be
> queued; nothing was written. If it timed out, check the board for the issue before retrying"

**To finish the split**, with `LINEAR_API_KEY` exported and the board reachable:

```bash
python3 scripts/board_sync.py create --guide tasks/DRAFT_store_health.md
python3 scripts/board_sync.py create --guide tasks/DRAFT_claude_command_button.md
```

Each writes `tasks/TASK_GUIDE_<new ID>.md` from the board's copy. Then delete the two drafts, put the
two new rows in `### Todo` on `PROJECT_KANBAN.md`, and close this Epic row.

Both drafts were checked against `parse_draft` and are complete (title, Complexity, Risk, Priority).

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Every one of the three notes is either a task with its own guide, or recorded here as needing no task, with the reason | `AGENTS.md` § Permanent Rules (Epics split at Stage 2) |
| 2 | This row is closed once the two drafts have board IDs and rows of their own | `AGENTS.md` § Hard-Stop Gate 3 |

## Questions Log

* 2026-11-08 Supervisor split of HUN-230 into note 1 (no task, an owner push), FR-091 and FR-092, in auto mode, with no owner input. PRD questions 33 and 34 record the two readings ⚠ decided for you — please review
