# TASK_GUIDE — T123: Inter for text, Plex Mono only for ids, code and labels
**Date**: 2026-11-19
**Complexity Level**: C1
**Risk Level**: Medium (page only; app-wide visual blast radius)
**Priority**: P1
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `agents/frontend.md`, `skills/ux-evaluate/SKILL.md`
2. `web/src/tokens.css`, `web/src/app.css`, `web/src/shell.test.js` (T120/T121 font + size rules), `memory/decisions.md` entry "2026-11-19 — T120"
3. How the design assigned fonts before T120: `git show 1abbd39^1:web/src/tokens.css` and `git diff 1abbd39^1 1abbd39 -- web/src/app.css web/src/appsession.css` (the four `var(--font-sans)` → `var(--font-mono)` swaps T120 made)
- Fixture/mock data only; never the real `~/.claude*` or `~/.easy-personal`. Port 8765 is taken. Reusable mock + shoot scripts: Supervisor scratchpad `ux-t121/`.

## Requirement
Owner 2026-11-19, after T121: "let suggest another font, the current look hard to the normal users" — then "how about ubuntu mono". Supervisor explained that any monospace font keeps the readability problem (and Ubuntu Mono runs ~10% small); owner chose **Inter + Plex Mono**: Inter for all text, IBM Plex Mono only for ids, commands, code and the design's mono labels; numbers in Inter's tabular figures.

## Dependencies & Reachability
**Depends on**: T121 (merged). Runs in parallel with T122 (Live filter bar) — T122 owns `web/src/appsession.css` and `LiveFilters.jsx`; do not edit them (T122 already uses `var(--font-sans)` in its controls).
**Entry point**: Standalone — N/A (every page; style tokens)

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Inter is self-hosted (latin subset, woff2, weights 400/500/600) from `@fontsource/inter` 5.3.0 (OFL-1.1): tarball fetched with `npm pack`, its sha512 checked against `npm view … dist.integrity`, licence text added to `web/src/fonts/` (OFL notice for Inter); no runtime dependency, no network request at run time | owner, supply chain |
| 2 | `--font-sans` = `'Inter', system-ui, sans-serif` and `body` uses it; `--font-mono` stays Plex Mono. The rules T120 swapped from sans to mono go back to `var(--font-sans)` (in `app.css`); every rule that was mono before T120 stays mono (labels, ids, tables' id columns, code, chips as designed) | owner |
| 3 | Numbers that sit in columns or change live (KPIs, table number cells, token/cost/tool-call counts, ring text, chips) use `font-variant-numeric: tabular-nums` (a token or one shared rule), so digits don't jiggle | owner, ux |
| 4 | Sizes unchanged from T121 (`--fs-*` tokens untouched unless Inter's metrics make a size clearly off — then state why); nothing under 12 px; no hard-coded family or size (`shell.test.js` rules updated to the new family split, keep its intent) | T121 |
| 5 | Screenshots read at 375 and 1280 on Usage, Live list + panel, Past, Settings, Memory, Workflow, Board: no clipping/overflow introduced, no horizontal page scroll, mono only where intended | ux |
| 6 | `cd web && npx vitest run && npm run build`; `python3 scripts/check_static.py` ok | all |

## Approach
| Option | Invasiveness | Regression risk | Pick |
|---|---|---|---|
| A. Swap `--font-sans` to Inter, restore T120's four sans→mono swaps, tabular-nums rule | Low (tokens.css, app.css, fonts/, shell.test.js) | width changes in tight cards (Inter is narrower than mono — mostly gains room) | **Yes** |
| B. Inter everywhere incl. ids/code | Low | ids/code lose alignment and clarity | No |
| C. System font stack only | Lowest | looks different per OS; not what owner picked | No |

## Files to change (predicted)
| File | Change |
|------|--------|
| `web/src/tokens.css` | Inter `@font-face` ×3, `--font-sans`, tabular-nums token |
| `web/src/app.css` | restore sans on body/inputs etc., tabular-nums on number elements |
| `web/src/fonts/` | `inter-latin-{400,500,600}-normal.woff2`, Inter licence notice |
| `web/src/shell.test.js` | font rules |

## Files must NOT touch
| File | Reason |
|------|--------|
| `web/src/appsession.css`, `web/src/LiveFilters.jsx` | T122 owns them (parallel) |
| `rust/**`, `src/easy_personal/**` (except build output) | server |

## Test plan
vitest (family split + no literal families + tabular-nums rule present + font files are real woff2); screenshots before/after.

## Verification Command
```bash
cd web && npx vitest run 2>&1 | tail -5 && npm run build && cd .. && python3 scripts/check_static.py
```

## Completion checklist
- [ ] Implementation done
- [ ] Tests pass
- [ ] Screenshots read at 375 and 1280
- [ ] Smoke test confirmed (owner: reinstall, read Live and Usage)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
