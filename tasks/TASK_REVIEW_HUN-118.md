# TASK_REVIEW — HUN-118: Global Workspace/Account selector in the sidebar for Usage, Warnings and Kit

> Sibling of `tasks/TASK_GUIDE_HUN-118.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Task breakdown and labels approved; run all tasks in auto mode — FACT: PROJECT_SPEC.md:175 "do all by parallel with tag [auto], merge to main and push"
2. Requirement Fidelity Gate signed auto (Supervisor) — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (row 13)
3. Spawn model from the CLI Model Table — RULE: `PROJECT_SPEC.md` § CLI Model Table
4. Waves (stacked branches) instead of all 8 from main: tasks that edit the same files start from their predecessor's branch — RULE: `PROJECT_SPEC.md` § Tasks ("Waves: 1 = HUN-113, 114, 115, 117 …")
5. Merge to main and push after `/verify` passes (the owner's explicit ask overrides the auto-mode no-merge rule for this one step) — FACT: docs/claude-md/tracks.md:365 "the explicit ask wins for X"
6. security-review done inline by the Supervisor; the built-in command failed with `fatal: ambiguous argument 'origin/HEAD...'` (this repo's remote is `github`, there is no `origin`) — FACT: memory/MEMORY.md:41 "security-review inline"
Checks: tests `npm --prefix web test && npm --prefix web run build && uv run pytest -q` — 152 Vitest passed, 750 pytest passed; reviews code-review — P2 1 carried to HUN-121, security-review (inline) — 0; real run fixture dashboard headless Chrome 375/768/1280 in reports/HUN-118/ (auto-checked, not /verify)
Not done: merge (owner asked for it; runs after verify); auto verify: running (T182)


## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/App.test.jsx` (new `global scope selector (HUN-118)` block, 13 tests, AC1-AC7 + failing options + localStorage throws + stale scope), `web/src/UsageView.test.jsx` (scope props, no own filters), `web/src/WarningsPlanView.test.jsx`, `web/src/KitView.test.jsx` (request params, refetch, "Showing:", stale answer ignored). `npm --prefix web test` 2026-10-29T08:32:48Z: `Test Files  8 passed (8)` / `Tests  152 passed (152)` (was 134 before) |
| Verification command run | ☑ pass | `npm --prefix web test && npm --prefix web run build && uv run pytest -q`: vitest `152 passed (152)`; build `✓ built in 201ms`; pytest `750 passed, 4 deselected, 1 xfailed, 1 warning in 160.32s` |
| Negative cases hold | ☑ pass | Tests: stored workspace `gone` / account `99` fall back to All and no `workspace=`/`account=` is sent; `localStorage.getItem`/`setItem` throw: page works and the choice still applies; options request fails: selects hidden, views load unscoped even with a stored scope; slow answer for an older scope is ignored (KitView); orphan workspace and detected/Codex accounts not listed |
| verify | ☑ pass | owner ran /verify 2026-10-29; headless `claude -p '/verify HUN-118'` run, log .claude/verify-runs/HUN-118-d7736e2b12c2.log; hook record reads blocked only because the run's summary had no single verdict line: global Workspace/Account selector scopes Usage, Warnings and Kit (needs HUN-117 merged first — merge order keeps it); pytest 750 passed 1 xfailed; vitest 1 unexplained flake in 7 runs, then 152/152 x6; flake fixed by HUN-121 5f7e8a2 (3 s Testing Library wait) — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | code-review (Supervisor, 2026-10-29): App scope state, Scope.jsx, useLoad deps, Usage/Warnings/Kit props + tests, base HUN-114. lane: C2, 11 files, 604 lines, EXCEEDS (tests + shared helper; label note). Findings after gate: P0 0, P1 0, P2 1 (scope options not refreshed after an Agents confirm — fix carried to HUN-121), P3 0. security-review inline: localStorage holds names/ids only, invalid values fall back to All, React-escaped — no findings — pass |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q`: `750 passed, 4 deselected, 1 xfailed, 1 warning in 160.32s (0:02:40)` (built static present); vitest 152 passed |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Headless Chrome 155 against a fixture dashboard (temp `EASY_PERSONAL_HOME`/`EASY_PERSONAL_AGENT_ROOT`/`EASY_PERSONAL_CLAUDE_PROJECTS`, workspaces acme/personal, accounts Claude company/Claude personal). Shots (not committed: `reports/` is gitignored): `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/HUN-118/reports/HUN-118/usage-1280.png`, `warnings-1280.png`, `kit-1280.png`. Verdict: the Scope card (Workspace and Account labelled selects) sits right under the brand on every tab, and each view says `Showing: acme · Claude company`; the owner sees where to choose at first glance. Warnings shot also read: same card, `Showing: acme · All`. (Note: this branch's backend ignores the params until HUN-117 merges, so the fixture shows unscoped data.) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `grep -nE "#[0-9a-fA-F]{3,8}\b|rgb\(|hsl\(" web/src/app.css web/src/Scope.jsx web/src/App.jsx` finds nothing (exit 1); all colors are `var(--...)` tokens. Contrast from `tokens.css` values: `--color-text` on `--color-surface-2` 13.56:1, `--color-muted` on `--color-surface-2` 6.97:1, `--color-muted` on `--color-bg` 8.81:1 (all >= 4.5:1) |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280 px `usage-1280.png`: scope block in the sidebar, no top-bar text. 768 px `usage-768.png` and 375 px `closed-375.png`: menu closed, top bar shows `Scope: acme · All` under the title, no horizontal overflow. 375 px `menu-open-375.png`: Scope block is at the top of the opened sidebar menu above the nav. All under `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/HUN-118/reports/HUN-118/` |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-29T08:25:44Z in the worktree (branch HUN-118, HEAD 49f845b, no implementation commit yet):

```
$ grep -c "Scope\|scope" web/src/App.jsx
0
$ grep -n "Filters\|createPortal\|slot" web/src/UsageView.jsx | head
(Filters component, createPortal into the top-bar slot; WarningsPlanView.jsx and KitView.jsx take no scope)
$ npm --prefix web test
 Test Files  8 passed (8)
      Tests  134 passed (134)
```

The sidebar has no Scope block. The Workspace/Account selects exist only on the Usage tab, portaled into the top bar. Warnings & plan and Kit send no `workspace=` / `account=`.

**AFTER**: captured 2026-10-29T08:32:48Z, same worktree after the change:

```
$ npm --prefix web test
 Test Files  8 passed (8)
      Tests  152 passed (152)
```

Screenshot `reports/HUN-118/usage-1280.png`: the sidebar shows a Scope card with "Workspace: acme" and "Account: Claude company" selects, and Usage says "Showing: acme · Claude company". `kit-1280.png` shows the same card kept on the Kit tab with "Showing: personal · Claude personal". `closed-375.png` shows "Scope: acme · All" in the top bar; `menu-open-375.png` shows the card at the top of the opened menu.

**DELTA**: The owner can choose Workspace and Account in one sidebar block on any tab, the choice stays across tabs and reloads, and Usage, Warnings & plan and Kit filter by it and say what they show.

**WITNESS**: frontend-developer agent, HUN-118 run, 2026-10-29T08:25Z to 08:33Z (vitest, build, pytest and Chrome screenshots on a fixture dashboard). The Supervisor's trace `memory/event-trace/HUN-118.jsonl` is the independent record.
