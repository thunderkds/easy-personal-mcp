**Date**: 2026-10-31
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Frontend-Implementer (owns the view) — the API route is small enough to stay in the same task; Backend-Implementer only if the Supervisor splits the spawn
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: Work view terms, Critical Constraints incl. the 2026-10-31 named exception)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `memory/codebase-map.md`, `PRD.md` US-019, FR-040, and `BRAINSTORMING_LOG_WORKVIEW.md` (sketches, Edge Case Checklist, Surgical Scope)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-31: "wanna build the personal like that [https://github.com/blogminhquy/javis-os](<https://github.com/blogminhquy/javis-os>) ... core vision still go with ours, but the idea is good to following — the data showing, the UX, the board". Grill + brainstorm decisions: `memory/decisions.md` 2026-10-31 "Work view inspired by Javis OS, read-only"; `BRAINSTORMING_LOG_WORKVIEW.md` Option A (owner-selected; sketches there are the layout reference).

**Restated intent**:

> Add a `schedules` reader that lists **scheduled jobs**: systemd user timers (`systemctl --user list-timers --all` + each unit's `ExecStart` and last exit status) and crontab lines, keeping only those whose command's first word (basename) is on the agent allow-list (`claude`, `easy-personal`, `easy-personal-launch`, kit tools — list in config). Serve at `GET /api/jobs`; show a Scheduled view under `// work` with name, kind, schedule, next run, last run, last exit code, and a footer with the hidden-count and the 'not included: /loop jobs, cloud routines' note.

**Out of scope**: creating, editing, starting or stopping jobs; showing command arguments; /loop and cloud routines; MCP tool.

**Requirement Refs**: PRD US-019, FR-040.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's request — Supervisor, 2026-10-31, owner approved PRD answers, Option A and this 5-task breakdown
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: HUN-160 (rail `// work` group). **Entry point**: route `/api/jobs`; Scheduled tab in `// work`.

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | Only the command's first word (basename) is kept; arguments are never stored, returned or logged (leak test with a planted secret in a fixture crontab/unit) | FR-040, NFR-002 |
| 2 | Allow-list filter; non-matching jobs are counted as hidden, not listed | FR-040 |
| 3 | Our own `easy-personal-ingest.timer` shows with next/last run and exit code | US-019 |
| 4 | No user bus / no `systemctl` / no `crontab` → that kind is no data; the view still renders the other kind | no data rule |
| 5 | Reader only runs read commands (`list-timers`, `show`, `crontab -l`) with a timeout; a test asserts no other subcommand is ever invoked | NFR-003 |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | fake `systemctl`/`crontab` on PATH (tests/fakes) with 1 agent timer, 1 unrelated, 1 cron `claude -p "SECRET_X"` | 2 rows, 1 hidden, `SECRET_X` nowhere in response | pytest |
| 2 | no systemctl on PATH | systemd = no data, cron rows still listed | pytest |
| 3 | view | rows + footer note | vitest |

### Verification Command (exact, runnable)

```bash
uv run pytest -q && npm --prefix web ci && npm --prefix web test -- --run && npm --prefix web run build && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-163.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-163.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Scheduled at 1280×720 | easy-ui MCP (`http://localhost:8765/mcp`) screenshot on fixture data vs the sketch in `BRAINSTORMING_LOG_WORKVIEW.md` | table + footer note as in the sketch |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors from tokens | `web/src/neon.test.jsx`-style test | only `var(--…)` from `web/src/tokens.css`, no raw literals |
| Fonts / labels | easy-ui computed style | Plex Sans body, Plex Mono labels, same card style as Kit/Usage views |
| Accessibility | vitest + easy-ui `ui_check` | landmarks, `aria-label` on SVG, keyboard focus visible, contrast ≥ 4.5:1 |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1280×720 | easy-ui screenshot (fixed viewport, see learnings "easy-ui + Graphite Mint") | matches sketch |
| narrow (rail collapsed) | easy-ui screenshot | no horizontal page scroll; wide content scrolls inside its box |

---

## Approach

**Pattern reference**: `core/schedule.py` (`systemctl` wrapper, `unit_dir`), `tests/fakes/` (fake binaries). Prefer `--output=json` when the installed systemd supports it; fall back to `show -p` properties. New `core/jobs.py`.
**Vital slice**: reader + route + table view.
**Cut list**: run history from journal; headless session runs per job; per-job detail page.

---

## Edge Case Checklist

- [ ] crontab env lines (`PATH=…`), comments, `@reboot`, `@hourly`
- [ ] command as absolute path (`/home/x/.local/bin/claude`) → basename; `env VAR=… cmd` → `cmd`; `sh -c '…'` / `bash -c '…'` → first word of the quoted string; anything else → basename of the first word
- [ ] timer with no matching service / service never ran → last run "never"
- [ ] systemd versions without `--output=json`
- [ ] timezone: show local time, store nothing

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/jobs.py (new)` | reader |
| `src/easy_personal/core/config.py` | allow-list setting |
| `src/easy_personal/surfaces/api.py` | `/api/jobs` |
| `web/src/JobsView.jsx (new) + test` | view |
| `web/src/App.jsx` | append Scheduled tab |
| `tests/fakes/…, tests/core/test_jobs.py, tests/leak/…` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/schedule.py` | install/remove logic unchanged (read helpers may be imported) |
| `src/easy_personal/connectors/**`, `src/easy_personal/launch.py` | connectors and keychain are out of scope |
| any repo's `PROJECT_KANBAN.md` / `memory/**`, `~/.claude*/**` | read-only sources (NFR-003) |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |

---

## Test Plan

pytest with fake binaries incl. a leak test (grep response for the planted secret); vitest; easy-ui screenshot.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled or N/A with reason (Hard-Stop Gate 6)
- [ ] `Skill({ skill: "verify" })` run
- [ ] Supervisor notified: task ready for Stage 4 review