# TASK_REVIEW — HUN-69: Dashboard Workspaces page, form, legacy banner

> Sibling of `tasks/TASK_GUIDE_HUN-69.md`. Everything here is **filled by the reviewer at Stage
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
| | **New test(s) cover Acceptance Criteria (file paths pasted)** | pass | `web/src/WorkspacesView.test.jsx` (31 tests: cards AC1, form AC2, add connector + planted secret AC3/AC7, move/alias/delete/assign AC4, delete workspace AC5, legacy banner AC6); `web/src/test/fixtures.js`. `web vitest`: Test Files 7 passed (7), Tests 91 passed (91) |
| | Verification command run | pass | `npm --prefix web test && npm --prefix web run build && uv run pytest tests/surfaces tests/leak -q` -> vitest 91 passed; vite built (index-C520mTQ-.js 255.83 kB); pytest `144 passed, 1 warning in 24.18s`. Also `uv run pytest tests/test_web_toolchain.py tests/test_site_content.py tests/test_readme.py -q` -> `50 passed` |
| | Negative cases hold | pass | invalid name shows API message and keeps form; refused move shows API message, no Confirm; refusal clears secret; double click sends one request; legacy check failure shows no banner |
| verify | ☑ pass | Supervisor 2026-10-26 21:5x +07: isolated install (`uv tool install` into mktemp dir, EASY_PERSONAL_HOME/AGENT_ROOT temp, fake `claude` + file keyring) with 3 workspaces, 1 orphan connector, 1 legacy keychain item; dashboard Workspaces tab drove: cards + orphan card + legacy banner render; Open company → move company-slack to personal → plan shown (Claude default add, Claude work remove/add, keychain untouched) with Confirm/Cancel; Add connector → provider form with password field. Console errors: none; requests only to 127.0.0.1. `npm --prefix web test` 92 passed; `uv run pytest -q` 711 passed — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed: WorkspacesView.jsx (all), App.jsx tab swap, app.css (selector scoped to `main.workspaces`), fixtures.js additions, test file. Visual check of Usage/Agents tabs for CSS regression: unchanged. Skipped backend: no Python change in this branch. Findings: P3 button style outside <section> (fixed ccef37d), P3 banner wording (fixed ccef37d, 9e6649e). Out-of-scope findings routed to a follow-up: API plan lines end with "aborted; nothing was changed." (`api._run_writer`); env-less `claude` calls in `_configured`/`doctor`/`writer.remove`. Open question decided: workspace create has no Confirm (writes no Claude config) — pass |
| | Full smoke suite still green (no regression) | pass | AgentsView/UsageView/App tests unchanged and green (91/91 vitest) |
| easy-verifier score (advisory) | ☐ N/A | N/A — easy-verifier MCP server not loaded in this profile |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Headless Chrome (CDP) screenshots in `reports/HUN-69/` (cards-, form-, add-, usage-, agents- × 375/768/1280). Verdict: cards, orphan card, legacy banner, edit form with move plan and add-connector form render as designed; Usage and Agents tabs unchanged — pass |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Computed font families on every element: `system-ui, -apple-system, "Segoe UI", sans-serif` (+ `monospace` for plan `<pre>`), no Arial; new CSS uses only `var(--space-*)`, `var(--color-*)`, `var(--radius)`, `var(--text-sm)` tokens; all buttons share the section button style after ccef37d — pass |
| **UI: Responsiveness at target viewports** | ☑ pass | 375/768/1280: `scrollWidth <= clientWidth` at every width for cards, form-with-plan, add-connector, usage, agents (noHorizontalScroll true ×15); cards stack in one column at 375 — pass |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-10-26, before any implementation commit):

```
$ date -u
Mon Oct 26 10:39:04 AM UTC 2026
$ grep -n "Workspaces\|Connectors" web/src/App.jsx
3:import ConnectorsView from './ConnectorsView.jsx'
12:  { id: 'connectors', label: 'Connectors', View: ConnectorsView },
$ ls web/src/WorkspacesView.jsx
ls: cannot access 'web/src/WorkspacesView.jsx': No such file or directory
```

**AFTER** (2026-10-26, after implementation, worktree HUN-69):

```
$ ls web/src/WorkspacesView.jsx web/src/ConnectorsView.jsx
web/src/WorkspacesView.jsx   (ConnectorsView.jsx: removed)
$ grep -n "Workspaces\|Connectors" web/src/App.jsx
5:import WorkspacesView from './WorkspacesView.jsx'
12:  { id: 'workspaces', label: 'Workspaces', View: WorkspacesView },
```

**DELTA**: The owner can create, edit, rename and delete workspaces, add, alias, move and delete connectors per workspace, assign orphan connectors and adopt legacy ones from the Workspaces tab, each write showing its plan before Confirm.

**WITNESS**: Supervisor, 2026-10-26 ~22:00 +07 — re-ran `npm --prefix web test` (92 passed) and `uv run pytest -q` (711 passed) at 9e6649e, and drove the built dashboard in headless Chrome on an isolated install (screenshots `reports/HUN-69/`).
