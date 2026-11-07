# TASK_REVIEW — HUN-241: [Short Title]

> Sibling of `tasks/TASK_GUIDE_HUN-241.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` tag,
`docs/claude-md/tracks.md` `## Auto mode`). To use it, copy the block below out of this comment, to this
place, and replace every <...>. Keep `Mode: auto` as its first line, exactly as written: T182's hook reads
it. Write it to this file before the run ends; the chat message is a copy, not the report. The same
block is the Supervisor's final chat message. It is not the `delivery-report` page and not
Evidence. A taste decision (what the user wants) also ends with the flag. Write `DEBATED` only when ≥2
Advocate `Agent` calls ran for that question, naming each Advocate's option. A decision with no Advocates is
labelled `RULE`, `FACT` or `SAFER OPTION`, never `DEBATED`. A decision flagged `⚠ decided for you` is a choice, and is never labelled `FACT`. A
safety action a rule requires (for example the backup before a delete) is `RULE` without the flag.
At Risk Medium or High, a `RULE:` counts only when it names a written rule (`path:line`,
`path § <heading>`, or `docs/claude-md/tracks.md` `### <section>`), and a `FACT:` needs a Citation
(`path:line "quoted text"`); without one it is a choice and must be `DEBATED`. Good: `2. backed up the CSV
— RULE: `docs/claude-md/tracks.md` `### Stops that stay``. Bad: `1. kept recurring.csv — RULE: Surgical Changes`.

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its option>; Advocate 2: <its option>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours)>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/WorkspaceKit.test.jsx` (describe `report folders (HUN-241)`, 14 tests: AC1 lines/singular/0 sessions left out, AC2 capped/no newest/`folders []`/missing by name/no folders list, AC3 no link/button/tab stop, AC4 pending/failed, AC6 title+ellipsis class/planted markup/tokens-only CSS) and `web/src/KitView.test.jsx` (describe `report folders in the by-workspace block (HUN-241)`, 6 tests: AC1, AC5 `GET /api/kit/reports` no query + token header, no refetch on period/scope change, AC4 pending/500/started in parallel/order). Written first, 18 failed red, then green. |
| Verification command run | ☑ pass | `cd web && npx vitest run` -> `Test Files 24 passed (24)`, `Tests 771 passed (771)` (2026-11-07T08:23:53Z; was 749 before). `npm --prefix web run build` -> `✓ built in 130ms`. `uv run pytest -q` -> `1676 passed, 6 deselected, 1 xfailed, 1 warning in 189.04s (0:03:09)`. |
| Negative cases hold | ☑ pass | Failed read (HTTP 500) -> `report folders unavailable` in both rows, no `role=alert`, no `role=status`, block rows and project list still shown. Pending read -> no line, no pill. Planted `<img onerror>` / `<script>` folder text rendered as text (`__pwned2` undefined). Workspace missing from the answer -> `no report folders`. No link, button or tabindex in the lines. |
| verify | ☑ N/A | `/verify` is the Supervisor's, not run by this agent. Agent real run in headless Chrome (see Demonstration AFTER): lines show under each row at 1280x720 and 375x812 - pass (auto-checked, not /verify). |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Consumers of `WorkspaceKit`: only `KitView.jsx` (the new `reports` prop has a default, so any other render is unchanged) and `KitView.test.jsx` (two existing page-call filters widened to also skip `/api/kit/reports`). `fixtures.js` only gained a route and a fixture. No backend, no `WorkspacesView.jsx`. |
| Full smoke suite still green (no regression) | ☑ pass | vitest 771/771, pytest 1676 passed (see Verification row). |
| **Companion score (easy-verifier)** | ☑ N/A | Not run by the implementing agent; the Supervisor runs the Stage 4 `score`. | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | BEFORE `tasks/evidence-HUN-241/before-kit-by-workspace-1280.png` / `-375.png`; AFTER `tasks/evidence-HUN-241/after-kit-by-workspace-1280.png` / `-375.png` (headless Chrome, dashboard from this branch on a store copy + temp tree). Verdict: each row keeps its columns and gains one muted monospace line per report folder under it (`app/reports · 12 files · 3 easy-ui sessions · newest 2026-11-05`, `app/smoke-reports · 1 file · newest 2026-11-07`), `no report folders` for `empty-ws`; rows, order and the project list unchanged. The long folder name ends in an ellipsis. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | vitest `AC6: the report styles use tokens only`: the 3 `.ws-report*` CSS lines hold no `#hex` / `rgb()`; values are `--fs-note`, `--space-1`, `--font-mono`, and the `muted` class (`--color-muted`). Accessibility (vitest): lines are a `<ul aria-label="Report folders of <name>">` inside the row, text only, no link/button/tabindex; the unavailable text is a muted `<div>`, the same pattern as the row's `no data` / `not used`. |
| **UI: Responsiveness at target viewports** | ☑ pass | Headless Chrome, `document.scrollingElement`: 1280x720 `scrollWidth 1265 = clientWidth 1265`; 375x812 `scrollWidth 375 = clientWidth 375`. Lines are one-line, ellipsised, full text in `title`; rows stack to one column at 375 as before. No horizontal scroll. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-07T08:17:23Z to 08:21:16Z on branch `worktree-agent-ac0f2ca7910d4d564`, after the allowed fast-forward merge of HUN-240 (07df97d), before any implementation commit.

```
$ date -u +%FT%TZ && cd web && npx vitest run     # 2026-11-07T08:17:23Z
 Test Files  24 passed (24)
      Tests  749 passed (749)
```

Dashboard from this worktree (`easy-personal dashboard`, port 35929) on a consistent copy of the owner store (sqlite backup) in a temp `EASY_PERSONAL_HOME`, plus two added workspaces over a temp tree (`demo-ws`: `app/reports` 12 files, `app/smoke-reports` 1 file, `lib`, and a repo with a very long folder name; `empty-ws`: `lib` only). Headless Chrome (CDP), Kit view, section "Kit by workspace":

```
$ node shot.mjs <dashboard-url> tasks/evidence-HUN-241/before-kit-by-workspace 6000    # 2026-11-07T08:21:16Z
1280x720 {"found":true,"scrollWidth":1265,"clientWidth":1265,"pills":0,"text":"Kit by workspace (4)\n\n... bep-work / Verifier no data / easy-ui 15 sessions · 1,459 calls · last 2026-11-06 / demo-ws / Verifier no data / easy-ui not used / empty-ws ... / personal ...","lines":[]}
375x812  {"found":true,"scrollWidth":375,"clientWidth":375,"pills":0, ... ,"lines":[]}
```

No row shows any report folder (`lines: []`), although `demo-ws` holds `app/reports` and `app/smoke-reports`. Screenshots: `tasks/evidence-HUN-241/before-kit-by-workspace-1280.png`, `before-kit-by-workspace-375.png`.

**AFTER**: 2026-11-07T08:23:53Z `cd web && npx vitest run` -> `Test Files 24 passed (24)`, `Tests 771 passed (771)`; `npm --prefix web run build` ok; `uv run pytest -q` -> `1676 passed, 6 deselected, 1 xfailed`. Real run, 2026-11-07T08:27:10Z, dashboard from this branch (port 59923), same store copy and temp tree, same headless Chrome script:

```
1280x720 {"found":true,"scrollWidth":1265,"clientWidth":1265,"pills":0,"lines":[
 "orderly/reports · 0 files","orderly/smoke-reports · 14 files · 7 easy-ui sessions · newest 2026-08-13",
 "app/reports · 12 files · 3 easy-ui sessions · newest 2026-11-05","app/smoke-reports · 1 file · newest 2026-11-07",
 "big-project-with-a-really-long-repository-folder-name/Test-Reports-with-a-really-long-folder-name-for-ellipsis · 1 file · newest 2026-11-07",
 "easy-memory-mcp/reports · 3 files · newest 2026-11-05", ..., "easy-ui-mcp/reports · 1,929 files · 345 easy-ui sessions · newest 2026-11-06", ...]}
 and "no report folders" under empty-ws
375x812 {"found":true,"scrollWidth":375,"clientWidth":375,"pills":0, same lines}
```

Screenshots: `tasks/evidence-HUN-241/after-kit-by-workspace-1280.png`, `after-kit-by-workspace-375.png`.

**DELTA**: The Kit view now shows, under each workspace row, where its repos keep report folders (`app/reports · 12 files · 3 easy-ui sessions · newest 2026-11-05`), as plain reference text, with no way to act on them.

**WITNESS**: Frontend-Implementer agent run 2026-11-07T08:17Z to 08:30Z (Bash calls traced to HUN-241 through `.claude/hooks/.state/active_task`; the file was overwritten once by the parallel HUN-240 agent, so part of this run may be traced under HUN-240). The Supervisor's Stage 4/5 re-run is the independent witness.
