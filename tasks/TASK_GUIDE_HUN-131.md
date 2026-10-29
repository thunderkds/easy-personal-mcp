**Date**: 2026-10-29
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: *Workspace*, *Agent account*, *API-equivalent cost*)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read the updated design source files (exact values): `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/5a654c3b-f981-4de5-b6fd-f21813c8d2f1/scratchpad/artifact-files/7badc931-8ad6-4676-87ce-5d78efdd6e36/project/Main.dc.html`, `Warnings.dc.html`, `Kit.dc.html` (owner's canvas "Easy Personal dashboard — Graphite Mint", version 1793286424-4ac1, saved 2026-10-29 22:07 +07; look data only; numbers and names are sample data). The previous version that HUN-126 built from is at `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/f3af3866-5834-430d-b6f4-a2c50ca9642f/scratchpad/artifact-files/7badc931-8ad6-4676-87ce-5d78efdd6e36/project/` — `diff` the two to see every change
7. Verify on fixtures only: temp `EASY_PERSONAL_HOME` + `EASY_PERSONAL_CLAUDE_PROJECTS` + `EASY_PERSONAL_AGENT_ROOT`; never open the real `~/.easy-personal/` or `~/.claude*/`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-29: "design is updated, let check [https://claude.ai/artifact/GGoKuwVrYPQAVGcT6e7Ptq](<https://claude.ai/artifact/GGoKuwVrYPQAVGcT6e7Ptq>) for the dashboard and the index here [https://claude.ai/artifact/5jKFUHLaGJTPK4KgazQ2o1#page-dcb81aee8d83](<https://claude.ai/artifact/5jKFUHLaGJTPK4KgazQ2o1#page-dcb81aee8d83>)".

Supervisor diff of the canvas (HUN-126 copy → 22:07 copy): layout, copy and sizes are unchanged. Only the look changed: the accent moved from `#7EDDB0` to a brighter neon mint `#3CF5A8`, and a glow layer was added to accent elements.

**Restated intent**:

> Apply the design's neon pass to the existing Graphite Mint dashboard (HUN-126). Change the accent to `#3CF5A8` and its derived tints (soft `0x14`, line `0x59`, bar `0x4D` alpha). Add four glow tokens taken from the design's `renderVals()`: neon text `0 0 12px <accent>59`, neon glow `0 0 0 1px <accent>40, 0 0 18px <accent>40`, dot glow `0 0 8px <accent>`, line glow `drop-shadow(0 0 3px <accent>99)`. Apply them where the design does: active nav item (accent text + neon text, inset 1px accent-line ring, dot glow on its square dot); active period button (soft bg, accent text, inset accent-line ring — it was neutral grey); KPI sparklines (line glow); Cache hit ratio value (accent + neon text) and its bar (dot glow); latest "By day" bar (neon glow); top "By project" bar (dot glow); Cache reuse ring stroke (line glow) and its centre text (accent); Warnings & plan step numbers (neon text); Kit "checks passed" value (accent + neon text) and its pass bar (dot glow), where those elements exist. Link colour stays `#7EDDB0` / hover `#B2EDCF` as in the design's `a` rule.

**Out of scope**: layout, copy or data changes; the design's accent picker and glow on/off tweak (fixed: `#3CF5A8`, glow on); the landing page (separate task); light mode.

**Requirement Refs**: owner request 2026-10-29 + canvas diff above.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's request — Supervisor, 2026-10-29, from the canvas diff
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: HUN-126, HUN-128, HUN-129 (merged to main). **Entry point**: every dashboard view (`web/src/App.jsx`).

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | `tokens.css`: `--color-accent: #3cf5a8`; soft = accent + `14`, a new accent-line = accent + `59`, bar = accent + `4d`; link `#7eddb0` / hover `#b2edcf` kept as their own tokens; four glow tokens (neon text, neon glow, dot glow, line glow) with the exact values in the Restated intent; no color literal outside `tokens.css` | look |
| 2 | Every element listed in the Restated intent uses the matching glow token; nothing else gains a glow | look |
| 3 | Active period button uses soft bg + accent text + inset accent-line ring (not the neutral `#1F2321`) | Usage |
| 4 | Text on an accent fill (if any) stays `#0E100F`; accent `#3CF5A8` on page `#121413` and on soft bg passes 4.5:1 | contrast |
| 5 | `prefers-contrast: more` turns the glow tokens to `none` (readability; the design's glow switch) | a11y |
| 6 | All Vitest tests pass; no new npm dependency; `npm --prefix web run build` ok; `uv run pytest -q` green | no regression |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | `tokens.css` | the accent, tint and 4 glow values equal the design's | Vitest (read the CSS file) |
| 2 | Usage at default load | active nav item and active period button carry the active classes styled with the accent-line ring | Vitest |
| 3 | grep `#[0-9a-fA-F]{3,8}\b\|rgba?\(` in `web/src/*.css` except `tokens.css`, and in non-test JSX | no match | shell |
| 4 | built page, 1440 px | glow visible on each element of AC 2 | easy-ui screenshot vs artboards |

### Verification Command (exact, runnable)

```bash
npm --prefix web test && npm --prefix web run build && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-131.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-131.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Usage, Warnings & plan, Kit at 1440 px | easy-ui MCP (`http://localhost:8765/mcp`) screenshots on fixture data vs the 3 updated artboards | same accent and glow as the artboards; layout unchanged from HUN-129 |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors from tokens | grep (Success 3) | none outside `tokens.css` |
| Contrast | computed | text ≥ 4.5:1 |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 375 / 1024 / 1440 px | easy-ui screenshots | layout identical to before; glow does not cause overflow or horizontal scroll |

---

## Approach

**Pattern reference**: HUN-126 `web/src/tokens.css` / `web/src/app.css`; the design's `renderVals()` for the exact values.
**Vital slice**: tokens + the glow on the active nav, period button, KPI strip and charts.
**Cut list**: accent picker; glow toggle UI; any animation or pulsing.

CSS only where possible: change tokens, add `box-shadow` / `text-shadow` / `filter` to the existing classes. Touch JSX only if an element in AC 2 has no class to hook.

---

## Edge Case Checklist

- [ ] empty store → "no data" states have no glow
- [ ] `prefers-contrast: more` → no glow
- [ ] SVG sparkline `filter` does not clip at the SVG edge (add `overflow: visible` if it does)

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/tokens.css` | accent, tints, glow tokens, `prefers-contrast` override |
| `web/src/app.css` | glow on the listed classes; period button active style |
| `web/src/*.test.jsx` (one) | token / active-state assertions |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/**` (except rebuilt static) | no API change |
| `site/**` | landing page is a separate task |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |
| `web/package.json` dependencies | no new dependency |

---

## Test Plan

Vitest for Success 1–2; shell grep for 3; easy-ui MCP session over HTTP JSON-RPC (`ui_start_session` → `ui_navigate` → `ui_take_screenshot` / `ui_check` → `ui_end_session`) on a fixture-backed dashboard at 375/1024/1440 px for Success 4 and the UI rows.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] `Skill({ skill: "verify" })` run on fixtures
- [ ] Supervisor notified: task ready for Stage 4 review