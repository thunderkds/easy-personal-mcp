# TASK_GUIDE — T120: Mono font and one step larger text, whole app
**Date**: 2026-11-19
**Complexity Level**: C1
**Risk Level**: Medium (page only, app-wide visual blast radius)
**Priority**: P1
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `agents/frontend.md`
2. `web/src/tokens.css` (all font and size tokens live here), `web/src/app.css`, `web/src/appsession.css`
3. `skills/ux-evaluate/SKILL.md`
- Tests write only to temp dirs; never the real `~/.claude*` or `~/.easy-personal`. Kill only processes your test started.

## Requirement
Owner 2026-11-19, after installing the T119 package: "for the font, and the content, let we use the mono font and the make the text size larger". Owner answers the same day: scope **whole app**; size **one step, body 14 → 15 px**.

## Dependencies & Reachability
**Depends on**: T119 (merged)
**Entry point**: Standalone — N/A (every page; style tokens only)

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Every page's text renders in IBM Plex Mono: `body` (and any element that set `var(--font-sans)`) uses `var(--font-mono)`; no hard-coded font family anywhere in `web/src/*.css` | owner |
| 2 | Body text is 15 px (`--fs-body: 0.9375rem`); every other `--fs-*` token is scaled by the same ratio (15/14), rounded to the nearest 1/32 rem, keeping the size order unchanged; no component hard-codes a px font size | owner |
| 3 | Headings and other weight-600 text do not use a browser-faked bold: either a bundled Plex Mono 600 `@font-face` (same licence, OFL, file in `web/src/fonts/`) or the 600 uses are mapped to the mono 500 face — say which and why | ux |
| 4 | `ux-evaluate` at 375 and 1280 on at least Usage, Live (list + panel + Past) and Settings: no horizontal page scroll, no clipped or overlapping text, tables/cards still fit; B-e (text size) should now pass | ux |
| 5 | `cd web && npx vitest run && npm run build`; `python3 scripts/check_static.py` ok; existing tests that pin font tokens updated only where the rule itself changed | all |

## Approach
Options weighed by the Supervisor (Medium risk, compact brainstorm):

| Option | Invasiveness | Regression risk | Pick |
|---|---|---|---|
| A. Change the tokens: `--font-sans` users → mono, `--fs-*` scaled once in `tokens.css` | Low (one file + any stray hard-coded rules) | wrapping/overflow on narrow cards and tables | **Yes** |
| B. Root `font-size` bump (`html { font-size: 107% }`) | Lowest | also scales spacing/rem layout everywhere, not just text | No |
| C. Per-component overrides | High | drift, many files | No |

Keep the `--font-sans` token defined (point it at the mono stack or leave it unused) so nothing breaks on a stale reference; fix any layout that the larger mono text breaks, surgically, with tokens.

## Files to change (predicted)
| File | Change |
|------|--------|
| `web/src/tokens.css` | font tokens, `--fs-*` scale, maybe a Plex Mono 600 face |
| `web/src/app.css`, `web/src/appsession.css` | only where the bigger text breaks layout |
| `web/src/fonts/` | maybe `ibm-plex-mono-latin-600-normal.woff2` |
| `web/src/*.test.jsx` | only tests that pin the changed rules |

## Files must NOT touch
| File | Reason |
|------|--------|
| `rust/**`, `src/easy_personal/**` except the build output | server code |
| component logic (`*.jsx` behaviour) | style-only task |

## Test plan
vitest + build; ux-evaluate screenshots before/after at 375 and 1280.

## Verification Command
```bash
cd web && npx vitest run 2>&1 | tail -5 && npm run build && cd .. && python3 scripts/check_static.py
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed (owner: reinstall the .deb and read Live + Usage)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
