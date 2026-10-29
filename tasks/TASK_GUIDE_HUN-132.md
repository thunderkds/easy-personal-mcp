**Date**: 2026-10-29
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` and `PROJECT_SPEC_SITE.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read the family design source files (exact markup, colors, sizes): `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/5a654c3b-f981-4de5-b6fd-f21813c8d2f1/scratchpad/artifact-files/264d0a25-034b-4f7b-bd3c-8bdd90e7a318/project/Memory-Mint.dc.html` (closest template) and `Graphite-Mint.dc.html` (Easy Kit: install box + Copy button, stat strip) — owner's canvas "Easy Kit family — calm dark theme", version 1793286407-941f; look data only; their numbers and text are about other products
7. Read the current `site/index.html` and its tests `tests/test_site_content.py`, `tests/test_site_theme.py`, `tests/test_site_deploy_config.py`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-29: "design is updated, let check [https://claude.ai/artifact/GGoKuwVrYPQAVGcT6e7Ptq](<https://claude.ai/artifact/GGoKuwVrYPQAVGcT6e7Ptq>) for the dashboard and the index here [https://claude.ai/artifact/5jKFUHLaGJTPK4KgazQ2o1#page-dcb81aee8d83](<https://claude.ai/artifact/5jKFUHLaGJTPK4KgazQ2o1#page-dcb81aee8d83>)".

The family canvas has landing boards for Easy Kit, Easy Memory and Easy Verifier, none for Easy Personal. Owner decision 2026-10-29 (AskUserQuestion): "Follow the family layout" — use the Easy Memory / Easy Kit layout, keep the current site's text and screenshots, in the new neon mint look.

**Restated intent**:

> Rebuild `site/index.html` in the family layout. Header 64 px: brand mark + "Easy Personal" + mono version pill (`usage dashboard · v<pyproject version>`); family nav with Easy Personal active (accent dot + dot glow) and links to Easy Kit, Easy Memory, Easy Verifier (the same URLs as the dashboard's `// family` footer) and a GitHub button. Left aside 248 px with mono group labels (`// overview`, `// getting started`, `// reference`) linking to the page's own sections; the current section is accent-tinted with the inset accent-line ring. Main (48/56 px padding): mono eyebrow `> …` in accent + neon text, h1 44 px/600, the README pitch verbatim, a primary button (accent fill, `#0E100F` text, neon glow) to GitHub; an install box (input bg `#0E100F`, control border, glow shadow) with the install command and a Copy button; a bordered stat strip whose numbers come from real sources only (MCP tool count = server registration, connector count = catalog, view count = dashboard tabs); then the existing sections — flow, dashboard screenshots, install and deploy, tools, connectors, footer — in the family card, table and numbered-step styles. Colors are the Graphite Mint set with the neon accent `#3CF5A8` and the four glow tokens (same values as HUN-131). Fonts: IBM Plex Sans / Mono, self-hosted under `site/fonts/` (copy of `web/src/fonts/` + `OFL.txt`).

**Out of scope**: new page content or claims; invented stats; any JS beyond the Copy button; a light theme; changes to the dashboard app.

**Requirement Refs**: owner request + owner decision 2026-10-29.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's request — Supervisor, 2026-10-29, owner picked "Follow the family layout"
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: HUN-131 (dashboard neon mint pass) (its build gives the fresh `site/dashboard-*.png` screenshots). **Entry point**: `site/index.html`, served by Vercel (`vercel.json` → `outputDirectory: site`).

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | Layout per Restated intent: header (brand, version pill, family nav with Easy Personal `aria-current="page"`, GitHub), left section nav, hero, install box with Copy, stat strip, then the existing sections in the same order | layout |
| 2 | Every content assertion in `tests/test_site_content.py` still holds (pitch verbatim, terms defined, flow lanes and captions, tools = server registration, connectors = catalog, install commands verbatim from README, runbook link, screenshots, safety scan, allowlisted links, footer) | content kept |
| 3 | `tests/test_site_theme.py` pins the new token set (Graphite Mint colors from HUN-126 AC 1 with accent `#3cf5a8`, tints, 4 glow tokens) instead of the old Kit neon set; colors only via `var(--…)`; contrast ≥ 4.5:1 for text tokens on the page bg | look |
| 4 | Stat strip numbers are computed from the same sources the content tests use; a test fails if a number drifts | no invented stats |
| 5 | Fonts self-hosted under `site/fonts/` with `OFL.txt`; no external stylesheet, font or script (existing "no external asset" rule holds) | assets |
| 6 | Copy button: `<button type="button">` with an inline script using `navigator.clipboard.writeText`; shows "Copied" for \~2 s; without clipboard access the command stays selectable text | install |
| 7 | Responsive: at ≤ 768 px the aside is hidden or stacks above main, header nav wraps, no horizontal page scroll at 375 px; tables scroll in their box | responsive |
| 8 | `site/dashboard-*.png` re-shot from the neon dashboard on fixture data (no real host path, email or token — safety scan holds) | screenshots |
| 9 | `uv run pytest -q` green | no regression |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | `site/index.html` | header nav marks Easy Personal with `aria-current="page"` and links the 3 family sites | pytest |
| 2 | tools / connectors counts | stat strip numbers equal server registration / catalog | pytest |
| 3 | a raw color literal outside `:root` | theme test fails | pytest |
| 4 | page at 375 px | no horizontal scroll | easy-ui |
| 5 | page loaded | no request outside the site origin | easy-ui network log |

### Verification Command (exact, runnable)

```bash
uv run pytest -q tests/test_site_content.py tests/test_site_theme.py tests/test_site_deploy_config.py && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-132.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-132.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Landing at 1280 px | easy-ui MCP (`http://localhost:8765/mcp`) screenshot of `site/index.html` served locally vs `Memory-Mint.dc.html` / `Graphite-Mint.dc.html` | same header, aside, hero, install box, stat strip, card styles, accent and glow |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors from tokens | theme test | none outside `:root` |
| Fonts | easy-ui computed style | Plex Sans body, Plex Mono labels/code, loaded from `site/fonts/` |
| Contrast | theme test | text ≥ 4.5:1 |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 375 px | easy-ui screenshot | aside out of the way, nav wraps, no horizontal scroll |
| 768 px | easy-ui screenshot | readable, no overflow |
| 1280 px | easy-ui screenshot | matches the family boards |

---

## Approach

**Pattern reference**: `Memory-Mint.dc.html` (header, aside, hero, buttons, 3-cell feature strip, numbered steps, stat strip); `Graphite-Mint.dc.html` (install box + Copy); HUN-126 `web/src/tokens.css` for the color names; HUN-57 `site/index.html` for content.
**Vital slice**: header + aside + hero + install box + stat strip + restyle of the existing sections.
**Cut list**: scroll-spy JS for the aside (static `:target` / first-item active is enough); accent picker; animations.

Port the look into semantic HTML + one `<style>` block with `:root` tokens. Do not paste the artboard markup; it uses inline styles and other products' text.

---

## Edge Case Checklist

- [ ] font files fail to load → `system-ui` / `ui-monospace` fallback keeps layout
- [ ] no clipboard permission → Copy fails quietly, command still selectable
- [ ] long connector / tool names → wrap or ellipsis, no overflow

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `site/index.html` | new layout and styles; content kept |
| `site/fonts/*` (new) | Plex woff2 copies + `OFL.txt` |
| `site/dashboard-*.png` | re-shot from the neon dashboard |
| `tests/test_site_theme.py` | new token pins |
| `tests/test_site_content.py` | header/family nav, stat strip, Copy button assertions |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/**`, `web/**` | dashboard is a separate task |
| `vercel.json` | deploy config unchanged |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |

---

## Test Plan

pytest for Success 1–3 and the content/theme pins; easy-ui MCP session over HTTP JSON-RPC (`ui_start_session` → `ui_navigate` to a local `python3 -m http.server` of `site/` → `ui_take_screenshot` / `ui_check` → `ui_end_session`) at 375/768/1280 px for Success 4–5 and the UI rows.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] `Skill({ skill: "verify" })` run
- [ ] Supervisor notified: task ready for Stage 4 review