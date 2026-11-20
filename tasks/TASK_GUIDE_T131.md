# TASK_GUIDE — T131: Board Run — one click starts an app session on a task
**Date**: 2026-11-20
**Complexity Level**: C2
**Risk Level**: Medium (starts a real `claude` process from one click; page-only change)
**Priority**: P2
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` FR-038 (board), FR-109 (New session form)
2. `BRAINSTORMING_LOG_BOARDRUN.md` (T131 section: Path A, edge cases, prompt text)
3. `web/src/BoardView.jsx`, `web/src/NewSessionSheet.jsx`, `web/src/NewSession.jsx` (the POST body it sends), `web/src/App.jsx` (how a view opens Live on a session), `web/src/api.js`
4. `rust/crates/app/src/app_session_routes.rs` `start`, `form`, `stored_repo` (what the route accepts and refuses) — read only

## Requirement
Owner 2026-11-20: "from the board section, let add the button the can execute the task from board". Owner decisions: **start immediately** (no form), follow the existing styles, screenshots at review. This is a deliberate exception to FR-109's confirm step, for the board button only.

## Dependencies & Reachability
**Depends on**: none (app session routes are on `main`)
**Entry point**: Board tab → a task card's **Run** button

## Acceptance Criteria
| # | Criterion | Evidence |
|---|---|---|
| 1 | Each card in Todo, In Progress, Ready for Review and Blocked has a **Run** button; Done and Closed cards have none | vitest |
| 2 | The card's project maps to the one workspace repo (from `GET /api/workspaces`) whose folder basename equals `task.project`; Run POSTs `/api/app-sessions` with `{workspace, repo, account: null, model: "default", mode: "default", worktree: false, prompt, confirmed: true}` (match the exact keys `NewSession.jsx` sends; never `bypassPermissions`) | vitest asserts the body |
| 3 | Prompt is exactly `Work on task <ID>: <plain title>. Follow this project's workflow; its guide is tasks/TASK_GUIDE_<ID>.md if that file exists.` (title through the existing `plain()`, whole prompt ≤ 100 000 UTF-8 bytes) | vitest |
| 4 | After 202 the app opens Live on the new session (same path `NewSessionSheet`'s `onStarted(id)` takes) | vitest |
| 5 | Disabled with visible reason text (not only a tooltip) when: app sessions off (`/api/agent-run` `enabled: false`) → the `OFF` text + Settings link; no matching repo → "Add this repo to a workspace to run it"; two repos match → "Two workspace repos share this name"; app sessions unavailable (route 404) → button hidden | vitest per case |
| 6 | One click = one session: the button is busy while the POST is out; a server error (403/409/4xx/5xx) shows its message in a toast, nothing is retried | vitest |
| 7 | Same id in two projects stays tied to its own card's project ("All projects" view) | vitest |
| 8 | UX: existing button style; card layout unchanged at 375 px and 1280 px | `ux-evaluate` screenshots |
| 9 | Full web suite + build pass (web/ has no lint script) | `npm test`, `npm run build` in `web/` |
| 10 | (Owner 2026-11-20: "highligh the button run") The Run button uses the app's existing accent style `className="primary"` (`main button.primary` in `app.css`: accent border, soft accent fill, accent text; disabled keeps the existing faint look). No new CSS rule or colour | vitest asserts the class; screenshot at 1280 |

## Approach
Path A (page-only): `BoardView` loads `/api/workspaces` and `/api/agent-run` beside `/api/board`; a small pure helper `repoFor(project, workspaces)` → `{workspace, repo} | {reason}`; `Card` gets a Run button; `BoardView` takes an `onOpenSession(id)` prop that `App.jsx` wires to the Live view the same way Live's New session sheet does.

## Files to change (predicted)
| File | Change |
|------|--------|
| `web/src/BoardView.jsx` | Run button, repo match, start call |
| `web/src/BoardView.test.jsx` | tests for ACs 1–7 |
| `web/src/App.jsx` | pass the open-Live callback to Board |
| `web/src/app.css` | only if the card footer needs a rule |

## Files must NOT touch
| File | Reason |
|------|--------|
| `rust/**`, `rust/parity/**` | `/api/board` is a byte-exact parity golden |
| `web/src/NewSession.jsx` | FR-109 confirm rule stays for the form |

## Test plan
Vitest for every AC with mocked fetch; `ux-evaluate` at 375/1280; `/verify` drives the real app: Run on a card in a temp workspace repo starts a session visible in Live (stop it after).

**How `/verify` clicks Run without a real account or tokens** (used by the Supervisor's `/verify` 2026-11-20, PASS):
1. Temp dir `$T`: `EASY_PERSONAL_HOME=$T/home` (copy of `rust/parity/store-v11.sqlite`), `EASY_PERSONAL_AGENT_ROOT=$T/agents` with `.claude/settings.json` = `{}`, and a git repo `$T/repos/demo` holding a `PROJECT_KANBAN.md` with one Todo task (e.g. `T900`) and one Done task.
2. A stub `claude` first on `PATH` (`$T/bin/claude`): writes `"$@"` to `$T/claude-args`, then `exec cat > $T/claude-stdin`. The app finds `claude` on `PATH` only, so no real `claude` and no real account run.
3. Store: point 3 fixture `events.project_path` rows at `$T/repos/demo` (the board lists projects with usage) and insert a workspace with that repo (`workspaces`, `workspace_repos`).
4. `serve --no-ingest --static-dir ../src/easy_personal/dashboard/static` (the page builds there, not `web/dist`), run under `env -u CLAUDE_CONFIG_DIR`; then `POST /api/accounts {"path": "$T/agents/.claude"}` and `PUT /api/agent-run/setting {"account": "<id>", "confirmed": true}`.
5. Drive the page in headless Chrome (CDP; no Playwright here): Board → `Run T900`. Expect: only the Todo card has Run; the stub's stdin holds the exact AC 3 prompt; its args carry no bypass mode; the app is on Live; `GET /api/app-sessions` lists the session. Then `POST /api/app-sessions/<id>/stop`, stop `serve`, and confirm no stub process is left.

## Verification Command
```bash
cd web && npm test -- --run && npm run build   # web/ has no lint script (found at T131 Stage 4)
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes (N/A: web/ has no lint script)
- [ ] Tests pass
- [ ] Smoke test confirmed
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
