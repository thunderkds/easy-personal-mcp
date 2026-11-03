# TASK_REVIEW — HUN-203: Repos folder browser + gap above Save

> Sibling of `tasks/TASK_GUIDE_HUN-203.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/surfaces/test_api_dirs.py` (23 tests: AC1-5 listing, order, repo flag, parent/home/root, 400/403, 500 cap, symlinks, unicode, token/Host/Origin, no path in logs, read-only); `tests/leak/harness.py` (`API_CALLS['/api/dirs']` + planted trees listed in `collect_api`; `tests/leak/test_leak_suite.py` scans them); `web/src/FolderPicker.test.jsx` (18 tests: AC6-8 dialog open/go in/Up/Home/Choose/dup/Cancel/Esc/focus return/error alert/loading/truncated, CSS gap + gutter + tokens-only); `web/src/WorkspacesView.test.jsx` (existing test updated for the renamed button) |
| Verification command run | ☑ pass | `uv run pytest -q`: 1331 passed, 2 failed, 6 deselected, 1 xfailed in 221.77s. The 2 failures were `tests/test_web_toolchain.py::test_real_repo_build_is_current` and `::test_real_wheel_contains_static_assets` ("static build stale": the pytest run started before the final `npm run build` of the edited sources). After the build: `uv run pytest -q tests/test_web_toolchain.py` = 16 passed. `cd web && npx vitest run`: Test Files 21 passed (21), Tests 566 passed (566). `npm --prefix web run build`: `built in 715ms`, exit 0. Leak suite alone: `uv run pytest tests/leak -q` = 35 passed. Logs: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/54000ba3-d7c0-4535-8376-908a80d33c6d/scratchpad/pytest203.txt`, `vitest203.txt`, `build203.txt` |
| Negative cases hold | ☑ pass | Real dashboard, curl: no token -> 403 `{"error":"forbidden"}`; wrong Host -> 403; `?path=etc` -> 400 `Path must be absolute`; `/nope-xyz` -> 400 `Not an existing folder`; `/etc/hostname` (a file) -> 400 same text; `/root` -> 403 `Cannot read this folder`; `/` -> `parent: null`; server log holds 0 lines with the path. Full output: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/54000ba3-d7c0-4535-8376-908a80d33c6d/scratchpad/after203.txt` |
| verify | ☑ pass | Owner-run `/verify HUN-203` (2026-11-03), executed by the Supervisor. Isolated `uv tool install .` from the branch: `0.4.0`, `easy-personal core ok`, MCP `tools/list` 11 tools. Dashboard from the branch on a store copy, started in a test folder holding `alpha/`, `beta/.git`, `.secret/`, `notes-CANARYNAME.txt`: `GET /api/dirs` → 200 `dirs` = alpha (repo false), beta (repo true) — no hidden folder, no file name; no token 403; relative 400; a file 400; `/` parent null; dashboard log holds no path or canary. Headless Chrome 1280×720 and 375×812: Browse… opens at the start folder (`:modal`, focus inside), into beta, Up, Choose adds one row, choosing again adds none, + Type a path adds an empty row, Esc closes and focus returns to Browse…, Up disabled at `/`; gap Repos buttons → Save 16 px at both widths; dialog 17–358 px at 375 (16 px gutter); no horizontal scroll. Screenshots `verify203.*/shots/verifyb-*.png` — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Headless Chrome on the branch dashboard (copy of the store): dialog open at the start folder, inside a git repo, at `/`, rows after Choose, 1280x720 and 375x812. Esc closed the dialog and focus returned to Browse…. Choose added `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp` once; choosing it again added no second row; `+ Type a path` added an empty row. Screenshots in `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/54000ba3-d7c0-4535-8376-908a80d33c6d/scratchpad/shots203/`: `after-1280-dialog-cwd.png`, `after-1280-dialog-inside.png`, `after-1280-rows.png`, `after-1280-dialog-root.png`, `after-375-dialog-cwd.png`, `after-375-dialog-inside.png`, `after-375-rows.png`, `after-375-dialog-root.png`, `after-*-form.png` (BEFORE: `before-*-form.png`) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `FolderPicker.test.jsx` `styles` test: own CSS block (`/* Folder browser (HUN-203)`) holds no `#hex` or `rgb()`; all values from `tokens.css` variables (`--color-*`, `--space-*`, `--radius*`, `--font-mono`, `--fs-note`). Accessibility (vitest + real Chrome): `<dialog>` opened with `showModal()` (`:modal` true), `aria-labelledby` the title, focus inside on open, Esc/Cancel close and focus returns to Browse…, error is `role=alert`, loading is `role=status`, list entries are real buttons |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280x720: scrollWidth 1280/1280 with dialog open (list scrolls inside: scrollHeight > clientHeight at 505 px). 375x812: dialog left 17, right 358 (16 px gutter), scrollWidth 375/375, list scrolls inside. Gap between the Repos buttons and Save: 16 px at both widths (BEFORE 0 px). The 1265/1280 figure in `after203.txt` for the 1280 rows shot is the vertical scrollbar, not horizontal overflow |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-03T15:35:33Z on branch HUN-203 at `fd2ff94` (no implementation commit yet), dashboard run from the worktree on a copy of the store (`easy-personal dashboard`, port 56473), headless Chrome:

```
$ curl -s -w "\nHTTP %{http_code} %{content_type}\n" -H "X-Dashboard-Token: <token>" http://127.0.0.1:56473/api/dirs
Not Found
HTTP 404 text/plain; charset=utf-8

1280x720  workspace form: button "+ Add repo" bottom=612, Save top=612  -> gapPx 0   (scrollWidth 1280/1280)
375x812   workspace form: button "+ Add repo" bottom=798, Save top=798  -> gapPx 0   (scrollWidth 375/375)
```
Screenshots: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/54000ba3-d7c0-4535-8376-908a80d33c6d/scratchpad/shots203/before-1280-form.png`, `.../before-375-form.png`. Full log: `.../scratchpad/before203.txt`.

**AFTER**: captured 2026-11-03T15:48:00Z on branch HUN-203 at `1089df7`, same method (dashboard from the worktree on a copy of the store, started in `/home/hungnguyenhuu/workspace/pets/hungnguyen111`):

```
$ curl -H "X-Dashboard-Token: <token>" http://127.0.0.1:60299/api/dirs
{"path": "/home/hungnguyenhuu/workspace/pets/hungnguyen111", "parent": "/home/hungnguyenhuu/workspace/pets", "home": "/home/hungnguyenhuu", "dirs": [{"name": "deep-agent-example", "repo": true}, {"name": "easy-context-mcp", "repo": true}, ...]}
HTTP 200
1280x720  FORM GAP {"gapPx":16}     375x812  FORM GAP {"gapPx":16}      (BEFORE: 0 and 0)
Browse… -> dialog (modal, focus inside) -> easy-personal-mcp -> Choose this folder -> repo row "/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp"
```
Full log: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/54000ba3-d7c0-4535-8376-908a80d33c6d/scratchpad/after203.txt`.

**DELTA**: The owner can click Browse… in the workspace form, walk the folders from where the dashboard was started and add a repo path with one click (typing still works), and the Repos buttons now sit 16 px above Save instead of touching it.

**WITNESS**: frontend-developer agent captured BEFORE (`/api/dirs` 404; gap 0 px) and AFTER; Supervisor re-ran the full pytest suite (1333 passed), reviewed at Stage 4 and ran `/verify` at the owner's command (2026-11-03).

---

## Stage 4 code-review (HUN-203)

Scope: `git diff main...HEAD` (10 files); callers: `create_app` route table, `WorkspaceForm`. `lane_check`: `lane: C2, 9 files, 691 lines, EXCEEDS (lines 691 > 500)` — 468 of those lines are tests; label kept C2. Entry points `Route("/api/dirs", dirs_list)` (`src/easy_personal/surfaces/api.py`) and `Browse…` (`web/src/WorkspacesView.jsx`) found. Startup reads line complete (PROJECT_SPEC.md, guide, agents/frontend.md). Conditional reviewers: security (input handling, filesystem), adversarial (≥ 50 lines), api (new route), over-engineering (new module `core/folders.py`: one caller, but keeps `api.py` thin like the other core helpers — accepted).

Checks re-run by the Supervisor: `uv run pytest -q` → `1333 passed, 6 deselected, 1 xfailed` (the agent's 2 stale-build failures are gone after its rebuild); symlink-loop path → 400 `Not an existing folder` (Python 3.13.2).

## P0 — Must Fix
None.

## P1 — Should Fix
None.

## P2 — Recommended
None.

## P3 — Optional
| File | Line | Finding | Confidence | Action |
| -- | -- | -- | -- | -- |
| src/easy_personal/core/folders.py | 27 | Hidden folders are left out of a listing, but an explicit `?path=/home/u/.ssh` still lists that hidden folder's sub-folder names (names only, token-gated) | 75 | Accept (the UI never builds such a path); or refuse a path with a hidden segment if the owner wants it stricter |
| web/src/app.css | 576 | Adds a shared `main button.primary` style (only used by "Choose this folder"); not in the guide, but it answers the HUN-200 suggestion "Save has no primary style" | 75 | Keep; apply to Save in a later task if wanted |

Verdict: P0 0, P1 0, P2 0, P3 2. No fixes applied.

## Stage 4 security review (HUN-203, Risk Medium)

The built-in `security-review` skill failed to start (`git diff --name-only origin/HEAD...`: this repo's remote is `github`, there is no `origin`), so the Supervisor reviewed by hand against the same checklist:

| Area | Finding | Result |
| -- | -- | -- |
| Access control | `/api/dirs` sits behind `Guard` (Host, Origin, `X-Dashboard-Token`); query-string token not accepted. Tests `test_needs_the_token`, `test_wrong_host_and_foreign_origin_are_refused`; real run: no token 403, wrong Host 403 | pass |
| Data exposure | Sub-folder names + `repo` flag only; no file names, sizes, contents, symlink targets. Leak suite lists 5 planted trees whose file names and bodies hold canaries: `tests/leak` 35 passed | pass |
| Input handling | Relative / missing / file / NUL → 400 with fixed text, path never echoed; symlink loop → 400; unreadable → 403 fixed text | pass |
| Writes | `os.scandir` + `is_dir` + `lexists` only; nothing written | pass |
| Logging | uvicorn `access_log=False`; the route logs nothing | pass |
| Network | binds 127.0.0.1 (unchanged); no new outbound call | pass |
| Page | folder names rendered as React text (escaped); dialog buttons `type="button"` outside the form | pass |
| Residual | Anyone holding the per-launch token can list folder names the user can read — same trust as the dashboard itself. Hidden-folder note above (P3) | Low, accepted |

Verdict: no High/Medium findings.
