# TASK_REVIEW — HUN-114: Agents: re-choose ignored accounts, rename/ignore confirmed ones; Usage explains ignored accounts

> Sibling of `tasks/TASK_GUIDE_HUN-114.md`. Everything here is **filled by the reviewer at Stage
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
Checks: pending
Not done: merge (owner asked for it; runs after verify); auto verify: running (T182)


## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/AgentsView.test.jsx` (9 new: AC1 ignored → Use this account → confirm POST with label; AC2 Rename / Ignore; AC3 default has no Ignore, By provider same buttons; AC5 ingest hint with `easy-personal ingest`; AC7 row alert), `web/src/UsageView.test.jsx` (3 new: AC6 plural note + Open Agents callback, singular, no note), `tests/core/test_accounts.py` (2 new: `is_default`, ignored → confirmed). Output: `Test Files 8 passed (8) / Tests 134 passed (134)`; `tests/core/test_accounts.py` `23 passed in 0.31s`. Red first: 11 vitest + 2 pytest failed before the change. |
| Verification command run | ☑ pass | 2026-10-29T08:18:49Z `npm --prefix web test && npm --prefix web run build && uv run pytest -q`: vitest `Tests 134 passed (134)`; build `✓ built in 477ms`; pytest `750 passed, 4 deselected, 1 xfailed, 1 warning in 154.78s (0:02:34)` |
| Negative cases hold | ☑ pass | Default account: no Ignore button (vitest) and API still refuses (`test_default_account_cannot_be_ignored` passes). Missing folder: no Use / Rename / Ignore. Empty rename label: Save name disabled, Cancel sends no request. Codex ignored account does not trigger the Usage note. Refused action: server text in `role="alert"` on that row only. |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green | ☑ pass | `uv run pytest -q` → `750 passed, 4 deselected, 1 xfailed` (includes leak harness); `npm --prefix web test` → 134 passed |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Headless Chrome 155 on a fixture dashboard (temp `EASY_PERSONAL_HOME`/`AGENT_ROOT`; default + confirmed + 2 ignored Claude + 1 ignored Codex). Screenshots in `reports/HUN-114/`: `agents-{1280,768,375}.png`, `agents-use-form-1280.png`, `usage-ignored-note-{1280,768,375}.png` (gitignored folder). Verdict: ignored rows show "Use this account"; the form shows Name + Confirm & name + Cancel; default row shows Rename only; confirmed row shows Rename + Ignore; Usage shows "2 agent accounts are ignored, so their usage is not counted." with an Open Agents button, above the period buttons; the button opens Agents. Readable at all sizes. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `grep -nE '#[0-9a-fA-F]{3,8}\b|rgb\(' web/src/AgentsView.jsx web/src/UsageView.jsx web/src/App.jsx web/src/app.css` → no output. The only CSS change extends an existing button rule to `.notice button` and adds `.notice p` spacing with `var(--space-2)`. |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280 / 768 / 375 px: `scrollWidth == clientWidth` on every screenshot (1265/1265, 753/753, 375/375), so no horizontal scroll; the action buttons wrap. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-29T08:12:26Z on fixtures (temp `EASY_PERSONAL_HOME`, `EASY_PERSONAL_CLAUDE_PROJECTS`, `EASY_PERSONAL_AGENT_ROOT`), branch `HUN-114`, before any implementation commit. Command: detect two accounts, ignore id 2, list. No `is_default` key exists, so the UI cannot tell which account is the default:

```
$ uv run python /tmp/claude-1000/demo114/demo.py
[{"id": 1, "folder": ".../root/.claude",   "status": "confirmed", "is_default": null},
 {"id": 2, "folder": ".../root/.claude-b", "status": "ignored",   "is_default": null}]
```

UI before (verbatim from `web/src/AgentsView.jsx` `AccountRow`): the Name + Confirm form shows only for `detected`, so an `ignored` row has no action and a confirmed row has no Rename or Ignore:

```
      {status === 'detected' && <Detected account={account} onChanged={onChanged} />}
      {status === 'confirmed' && provider === 'codex' && <p className="muted">listed only</p>}
      {status === 'confirmed' && provider === 'claude' && <Picker account={account} workspaces={workspaces} />}
```

`UsageView.jsx` has no note about ignored accounts.

**AFTER**: same command, 2026-10-29T08:21:38Z, after the change: `[{"id":1,...,"status":"confirmed","is_default":true},{"id":2,...,"status":"ignored","is_default":false}]`. The Agents page on a fixture dashboard now shows "Use this account" on the ignored rows, "Rename" + "Ignore" on the confirmed non-default row, "Rename" only on the default row (`reports/HUN-114/agents-1280.png`); the Usage page shows "2 agent accounts are ignored, so their usage is not counted." with an Open Agents button (`reports/HUN-114/usage-ignored-note-1280.png`).

**DELTA**: The owner can now choose an ignored account again, rename or ignore a confirmed one, and sees on Usage why ignored accounts leave the last 7 days empty.

**WITNESS**: frontend-developer agent, 2026-10-29 (BEFORE 08:12:26Z, final verification 08:18:49Z, AFTER 08:21:38Z); Bash calls attributed to HUN-114 via the active-task state file for the trace hook.
