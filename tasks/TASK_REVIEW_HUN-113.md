# TASK_REVIEW — HUN-113: Workspaces page renders without waiting for the legacy scan; legacy checks run in parallel

> Sibling of `tasks/TASK_GUIDE_HUN-113.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Task breakdown and labels approved — FACT: owner message 2026-10-29 "do all by parallel with tag [auto], merge to main and push" after the Stage 2 plan in `PROJECT_SPEC.md` § Tasks
2. Requirement Fidelity Gate signed auto (Supervisor) — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (row 13)
3. Spawn model from the CLI Model Table — RULE: `PROJECT_SPEC.md` § CLI Model Table
4. Waves (stacked branches) instead of all 8 from main: tasks that edit the same files start from their predecessor's branch — RULE: `PROJECT_SPEC.md` § Tasks ("Waves: 1 = HUN-113, 114, 115, 117 …")
5. Merge to main and push after `/verify` passes — FACT: owner message 2026-10-29 "merge to main and push"; `docs/claude-md/tracks.md` `### Edge cases` "the explicit ask wins for X"
Checks: tests `npm --prefix web test && npm --prefix web run build && uv run pytest -q` — 125 Vitest passed, 749 pytest passed; reviews code-review — 0 findings (Risk Low: no security-review); real run fixture `/api/connectors/legacy` 0.747 s for 5 ids vs ~3.5 s before (auto-checked, not /verify)
Not done: merge (owner asked for it; runs after verify); auto verify: running (T182)


## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/connectors/test_legacy.py::test_detect_runs_claude_checks_concurrently_in_catalog_order` (AC3, AC4: fake `claude` sleeps 0.5 s, 5 ids, catalog order, < 1.5 s); `web/src/WorkspacesView.test.jsx` 3 new tests in "legacy connectors banner" (AC1 pending legacy shows workspaces, AC2 banner on late answer, unmounted view sets no state). Red first: pytest `AssertionError: 5 checks took 2.56s`; Vitest `3 failed \| 32 passed`. Green: `18 passed` (test_legacy + test_api_legacy), Vitest `Tests  125 passed (125)`. |
| Verification command run | ☑ pass | `npm --prefix web test && npm --prefix web run build && uv run pytest -q` at 2026-10-29T08:13:14Z: Vitest `Test Files  8 passed (8)`, `Tests  125 passed (125)`; build `✓ built in 165ms`; pytest `749 passed, 4 deselected, 1 xfailed, 1 warning in 124.34s (0:02:04)`. |
| Negative cases hold | ☑ pass | Failed legacy call (503) still shows no banner and no page error (existing test "is absent when the legacy check fails"); no keychain backend still gives an error not a crash (`test_get_without_a_keychain_backend_is_an_error_not_a_crash`, `test_missing_keychain_backend_refuses_with_guidance`: `LaunchError` re-raises from the worker through `pool.map`); legacy answer after unmount sets no state (console.error not called). |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | code-review (Supervisor, 2026-10-29): `legacy.py` + callers `api.connectors_legacy`, `legacy.adopt`; `WorkspacesView.jsx` + tests. lane: C1, 4 files, 104 lines, FITS. Findings after gate: P0 0, P1 0, P2 0, P3 0 — pass |
| Full smoke suite still green (no regression) | ☑ pass | Same run as the verification command: `749 passed, 4 deselected, 1 xfailed`. |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Headless Chrome (CDP) on a fixture store (temp `EASY_PERSONAL_HOME`, in-memory keyring, fake `claude`). Legacy still pending (fake `claude` 30 s): `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/HUN-113/reports/HUN-113/pending-1280-2500ms.png` shows the 3 workspace cards, no banner, no error, 2.5 s after opening. Legacy answered (fake `claude` 0.7 s): `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/HUN-113/reports/HUN-113/ready-1280-2500ms.png` shows the same page with the banner in today's style. Verdict: list first, banner later, same style. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `git diff -U0 -- web/src \| grep '^+' \| grep -ciE '#[0-9a-f]{3,6}\b\|rgb\('` printed `0`; no CSS file changed. |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280 px and 375 px: `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/HUN-113/reports/HUN-113/pending-375-2500ms.png`, `ready-375-2500ms.png`, `pending-1280-2500ms.png`, `ready-1280-2500ms.png`. No overflow. When the banner appears it pushes the cards down; no content is hidden or cut. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: 2026-10-29T08:12:23Z, `uv run pytest -q tests/connectors/test_legacy.py -k concurrently` (new test, before any implementation edit; 5 catalog ids, fake `claude` sleeping 0.5 s per call, in-memory keyring):
```
>       assert took < 1.5, f"{len(ids)} checks took {took:.2f}s"
E       AssertionError: 5 checks took 2.56s
FAILED tests/connectors/test_legacy.py::test_detect_runs_claude_checks_concurrently_in_catalog_order
1 failed, 14 deselected in 3.37s
```
The page side before the change: `WorkspacesView` waited for `Promise.all` of 4 requests including `/api/connectors/legacy`; the 3 new Vitest tests failed (`Tests  3 failed | 32 passed (35)`, page stuck on "Loading workspaces").

**AFTER**: same test: green (`18 passed` with `tests/surfaces/test_api_legacy.py`). Real API on a fixture store with a 0.7 s fake `claude`: `curl /api/connectors/legacy` answered `200` in `0.746967` s for 5 ids (sequential would be about 3.5 s). Browser with a 30 s legacy call: workspace cards visible at 2.5 s with no banner (`reports/HUN-113/pending-1280-2500ms.png`); with 0.7 s legacy the banner appears (`ready-1280-2500ms.png`).

**DELTA**: The owner sees the Workspaces list at once; the legacy banner joins when its scan ends, and that scan takes about one `claude mcp get` call (0.75 s for 5 ids) instead of five.

**WITNESS**: Backend-Implementer agent, 2026-10-29 08:12 to 08:19 UTC; trace attribution by the `active_task` state file (HUN-113).
