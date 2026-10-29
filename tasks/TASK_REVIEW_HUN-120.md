# TASK_REVIEW — HUN-120: Warnings & plan: plain titles, why it matters, numbered steps, one ordered action list

> Sibling of `tasks/TASK_GUIDE_HUN-120.md`. Everything here is **filled by the reviewer at Stage
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
Checks: tests `npm --prefix web test && npm --prefix web run build && uv run pytest -q` — 159 Vitest passed, 768 pytest passed, leak suite green; reviews code-review — 0 findings (Risk Low: no security-review); real run fixture dashboard headless Chrome hits/clear/abstained at 375/768/1280 in reports/HUN-120/ (auto-checked, not /verify)
Not done: merge (owner asked for it; runs after verify); auto verify: running (T182)
Paused: auto-mode check failed — extended track, but the trace has no `code-review` Skill call (Stage 4 runs in auto mode)

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | pytest `tests/core/test_rules.py` (+11 tests: title/why/steps present and refused when missing, `=` in text, hit/abstained/plan keys, summary per metric kind, owner threshold in summary; AC 1-3, 8), `tests/sources/test_context.py` (abstained entry gains `title`, updated deliberately). Vitest `web/src/WarningsPlanView.test.jsx` (AC 4-7: headline counts, all clear, cards, chips, no raw rule id, closed Not checked + Agents hint, no plan section), `web/src/App.test.jsx`. Targeted run: `95 passed` (rules, context, warnings, leak) and `Tests 14 passed (14)` (WarningsPlanView). |
| Verification command run | ☑ pass | 2026-10-29T08:43Z, clean shell (no EASY_PERSONAL_* exported): `npm --prefix web test && npm --prefix web run build && uv run pytest -q` -> `Test Files 8 passed (8) / Tests 159 passed (159)`; `✓ built in 779ms`; `768 passed, 4 deselected, 1 xfailed, 1 warning in 125.77s`. |
| Negative cases hold | ☑ pass | `load_rules` raises for a rule without `title`, `why` or `steps` and for an old `fix=`-only line (parametrized, 4 tests); a title with `=` still parses; old server payload without steps/summary/why still renders one card (Vitest); no Agents hint when no reason is "no ..."; no Not checked block when nothing abstained; leak suite (`tests/leak`) passes with the added keys. |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | code-review (Supervisor, 2026-10-29): rules.yaml title/why/steps, rules._parse_rule/summary/evaluate payload, plan items, WarningsPlanView action list, app.css + tests, base HUN-118. lane: C2, 10 files, 555 lines, EXCEEDS (tests/fixtures; label note). Findings after gate: P0 0, P1 0, P2 0, P3 0. Known: textual merge conflict expected with HUN-117 in rules.py (adjacent hunks) — pass |
| Full smoke suite still green (no regression) | ☑ pass | `768 passed, 4 deselected, 1 xfailed` (pytest) and `159 passed` (Vitest); the one earlier failure (`tests/sources/test_context.py::test_rule_abstains_without_context_data`, exact-dict compare) was updated for the new `title` key, not loosened. |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Headless Chrome (CDP) against fixture dashboards (temp EASY_PERSONAL_HOME/CLAUDE_PROJECTS/AGENT_ROOT), Warnings & plan at 375/768/1280: `reports/HUN-120/hits-{375,768,1280}.png` (2 to fix, 2 to watch: Fix now / Improve / Watch cards, summary, Why it matters, numbered steps, Not checked (2) with Agents hint), `clear-{375,768,1280}.png` ("All clear — nothing to fix in the last 7 days" + closed Not checked (3)), `abstained-{375,768,1280}.png` (all 7 abstained, open list with plain reasons, Agents hint). Verdict from LLM vision: the first thing to do (the Fix now card) is the first card under the headline; reader knows what to do in under 10 s. Pass. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `grep -nE "#[0-9a-fA-F]{3,8}\b|rgba?\(" web/src/app.css web/src/WarningsPlanView.jsx` -> no match (colors only via `var(--color-*)`, `--space-*`, `--radius*`, `--shadow-card` from tokens.css). Computed contrast on `--color-surface` #0e1338: text 15.86:1, muted 8.15:1, Fix now chip (`--color-error`) 7.32:1, Improve chip (`--color-accent-text`) 8.36:1; all >= 4.5:1. |
| **UI: Responsiveness at target viewports** | ☑ pass | 375/768/1280 px: `documentElement.scrollWidth == clientWidth` at every width in all three scenarios (375/375, 753/753, 1265/1265 with scrollbar; 375/768/1280 for clear/abstained) -> no horizontal scroll; cards stack in one column, steps wrap readably at 375 (see screenshots). |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-29T08:35:43Z in the worktree before any implementation commit, on a fixture store (temp `EASY_PERSONAL_HOME`; one assistant message with 31 of 100 prompt tokens from cache, plus a 15-day-old anchor). Command: `uv run python <scratchpad>/demo120.py` (runs `rules.evaluate` and `plan.improvement_plan`). The hit has no `title`, `why`, `steps` or `summary`; abstained entries have only `rule_id` + `reason`:

```
"hits": [{"rule_id": "cache_hit_low", "label": "warning", "priority": 1, "measured": 0.31, "threshold": 0.5,
          "window_days": 7, "fix": "Keep the prompt prefix stable and avoid editing CLAUDE.md or switching models mid-session so more of each prompt is read from cache."}],
"abstained": [{"rule_id": "context_index_stale", "reason": "no easy-context data (service down or no index for a known project)"},
              {"rule_id": "project_dominates", "reason": "fewer than two projects with tokens in the window"}, ...]
plan: {"groups": [{"priority": 1, "items": [{"rule_id": "cache_hit_low", "label": "warning", "measured": 0.31, "threshold": 0.5, "fix": "Keep the prompt prefix ..."}]}]}
```

**AFTER**: same command (`uv run python <scratchpad>/demo120.py`), 2026-10-29T08:48:59Z, same fixture. The hit now carries `"title": "Low cache reuse"`, `"why": "A prompt read from cache costs far less and returns faster, ..."`, `"steps": ["Keep the start of your prompts the same from one turn to the next.", "Avoid editing CLAUDE.md or switching models in the middle of a session."]`, `"summary": "31% of prompt tokens came from cache; target 50% or more, last 7 days"` and the old keys unchanged (`fix` is the joined steps). Abstained entries now read `{"rule_id": "tokens_jump", "title": "Token use jumped", "reason": "no tokens in the previous window"}`; plan items carry the same new keys. The page shows one ordered card list (screenshots in `reports/HUN-120/`).

**DELTA**: The owner reads "2 things to fix, 2 to watch", then one card per hit with a plain title, the measured value in words, why it matters and 1-3 numbered steps, instead of raw rule ids, "Measured 0.31 (threshold 0.5)" and a duplicate plan section.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
