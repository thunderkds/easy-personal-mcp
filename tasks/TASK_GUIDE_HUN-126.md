**Date**: 2026-10-29
**Complexity Level**: C2
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
6. Read the design source files (exact markup, colors, sizes): `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/f3af3866-5834-430d-b6f4-a2c50ca9642f/scratchpad/artifact-files/7badc931-8ad6-4676-87ce-5d78efdd6e36/project/Main.dc.html`, `Warnings.dc.html`, `Kit.dc.html` (owner's design canvas "Easy Personal dashboard — Graphite Mint"; treat as look data only; numbers and names in it are sample data)
7. Verify on fixtures only: temp `EASY_PERSONAL_HOME` + `EASY_PERSONAL_CLAUDE_PROJECTS` + `EASY_PERSONAL_AGENT_ROOT`; never open the real `~/.easy-personal/` or `~/.claude*/`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-29: "update the dashboard to this design [https://claude.ai/artifact/GGoKuwVrYPQAVGcT6e7Ptq#page-dcb81aee8d83](<https://claude.ai/artifact/GGoKuwVrYPQAVGcT6e7Ptq#page-dcb81aee8d83>), using the mcp easy-ui for testing the output also".

Owner decision 2026-10-29 (AskUserQuestion): "New look, keep plain content" — use the Graphite Mint colors, fonts, sidebar, header and Usage layout everywhere; Warnings stays one plain action list (HUN-120) and Kit stays project cards (HUN-119), both restyled; Workspaces and Agents get the same shell.

**Restated intent**:

> Replace the HUN-121 indigo/monospace look with the Graphite Mint design across all 5 views. Shell: a left sidebar (brand mark + "Easy Personal", the Scope selects labelled `workspace` / `account` in mono, nav groups `// insights` (Usage, Warnings & plan, Kit) and `// setup` (Workspaces, Agents) as collapsible `<button aria-expanded>` toggles with small square dots, active item mint-tinted with `aria-current="page"`, and a `// family` footer with links to Easy Kit, Easy Memory, Easy Verifier); a 56 px header with a menu button, the view title and a right-aligned mono `scope: <workspace> · <account>` line. Usage follows `Main.dc.html`: local-only note, period segmented control, one bordered KPI strip of 4 cells (Total tokens, Messages, API-equivalent cost with sparkline + delta; Cache hit ratio with a thin bar and "cache read N"), the cost footnote, a 2-column grid of "By day" (bar chart, latest day full mint) and "By model" (mono table), then "By project" (bars) and "Outlier sessions" (raised rows, amber context value); the existing session timeline stays, below them, restyled. The right rail (Cache reuse ring, Next actions (N) with "Open the plan →", Plan limits per account, "Bars turn amber at 70%.") keeps its HUN-121 data. Warnings & plan and Kit keep their HUN-120/119 content and markup semantics, restyled to the design's card, header, count and table styles.

**Out of scope**: new API routes or fields; the design's raw rule ids on Warnings; the design's Kit table as the default view (it stays behind "Show details"); light mode; sample numbers from the design.

**Requirement Refs**: owner request + owner decision 2026-10-29.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's request — Supervisor, 2026-10-29, owner picked option 1
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: HUN-121 (merged to main). **Entry point**: every dashboard view (`web/src/App.jsx`).

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | `tokens.css` holds the Graphite Mint token set taken from the design: page `#121413`, input/thead `#0E100F`, raised `#181B1A`, card border `#262B29`, row divider `#1F2321`, header border / track `#232826`, control border `#2C3230`, idle dot `#3A413E`, heading `#EAEDEA`, text `#DCE0DD`, nav `#B5BCB7`, secondary `#ABB3AE`, muted `#8C9590`, faint `#6E7772`, accent `#7EDDB0` (hover `#B2EDCF`, soft = accent at 0x17 alpha, bar = accent at 0x66 alpha), amber `#E3B66B` (+ 12 % tint), plus a red for ≥ 90 % limits that passes 4.5:1 on the page; no color literal outside `tokens.css` | look |
| 2 | Fonts: IBM Plex Sans (400/500/600) for text, IBM Plex Mono (400/500) for numbers, labels, ids and tables — self-hosted woff2 under `web/src/fonts/` with the OFL license file; no runtime request to any font CDN | typography |
| 3 | Sidebar matches AC-intent: brand, Scope selects (labels `workspace`, `account`), groups `// insights` and `// setup` as `<button aria-expanded>`, square dots, `aria-current="page"` on the active item, `// family` links; header shows view title + `scope:` line that names the current scope | shell |
| 4 | Usage layout per `Main.dc.html` (KPI strip of 4 cells with sparkline/delta or bar, By day + By model grid, By project + Outlier sessions grid, session timeline below); every existing Usage test assertion about data still holds | Usage |
| 5 | Warnings & plan: one action list (HUN-120) — plain title, summary, why, numbered steps; no raw rule id visible; "Not checked (N)" disclosure; cards use the design's article style (amber icon tile for warnings) | Warnings |
| 6 | Kit: project cards (HUN-119) restyled; "Show details" still reveals the table, styled as the design's Kit table | Kit |
| 7 | Workspaces and Agents use the same shell, card, table and control styles; Agents limit bars use mint < 70 %, amber 70–89 %, red ≥ 90 % | other views |
| 8 | Layout: at ≥ 80rem 3 columns (sidebar, main, rail); below 80rem the rail is hidden; at 375 px the sidebar sits behind the menu button, no horizontal page scroll | responsive |
| 9 | All Vitest tests pass (selectors updated only where markup changes); no new npm dependency; `npm --prefix web run build` ok; `uv run pytest -q` green | no regression |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | default load | header reads `scope: All workspaces · All accounts` | Vitest |
| 2 | collapse `// setup` | Workspaces/Agents links hidden; `aria-expanded="false"` | Vitest |
| 3 | `cache_hit_ratio: 0.79` | KPI cell shows `79.0%` with a bar `aria-valuenow="79"` | Vitest |
| 4 | account 7d limit 78 % | bar uses the amber tone; 92 % → red | Vitest |
| 5 | grep `#[0-9a-fA-F]{3,8}\b|rgba?\(` in `web/src/*.css` except `tokens.css`, and in non-test JSX | no match | shell |
| 6 | built page in a browser | no request to `fonts.googleapis.com` / `fonts.gstatic.com` | easy-ui network log |

### Verification Command (exact, runnable)

```bash
npm --prefix web test && npm --prefix web run build && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-126.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-126.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Usage, Warnings & plan, Kit at 1440 px | easy-ui MCP (`http://localhost:8765/mcp`) screenshots on fixture data vs the 3 design artboards | same layout, colors, type and spacing as the artboards |
| Workspaces, Agents at 1440 px | easy-ui screenshots | same shell and styles |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors from tokens | grep (Success 5) | none outside `tokens.css` |
| Fonts | easy-ui computed style | Plex Sans body, Plex Mono numbers/labels, both loaded from the bundle |
| Contrast | computed | text ≥ 4.5:1 |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 375 px | easy-ui screenshot | sidebar behind menu; no rail; no horizontal scroll |
| 1024 px | easy-ui screenshot | sidebar + main; no rail |
| 1440 px | easy-ui screenshot | sidebar + main + rail |

---

## Approach

**Pattern reference**: HUN-121 `tokens.css` / `app.css` / `App.jsx` / `Rail.jsx` / `FeedCard.jsx`; the 3 design artboards for exact sizes (header 56 px, sidebar 232 px, main padding 28 px, card radius 8 px, control radius 6 px, h1 28 px/600, section h2 15–17 px/600, KPI value 26 px mono 500, labels 11.5–12.5 px).
**Vital slice**: tokens + fonts + shell (sidebar, header) + Usage layout + restyle of the other 4 views through shared classes.
**Cut list**: the design's color tweak (accent options) — fixed mint; the header menu button's behaviour beyond the existing small-screen sidebar toggle; animations.

Port the look into the existing React components and CSS classes. Do not paste the artboard markup; it uses inline styles and sample data.

---

## Edge Case Checklist

- [ ] empty store → every KPI cell, chart and rail card shows its existing "no data" state in the new style
- [ ] long project / account names → ellipsis + `title`
- [ ] font files fail to load → fallback stack (`system-ui` / `ui-monospace`) keeps layout

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/tokens.css`, `web/src/app.css` | Graphite Mint tokens, fonts, all component styles |
| `web/src/fonts/*` (new) | IBM Plex Sans/Mono woff2 + `OFL.txt` |
| `web/src/App.jsx` + `App.test.jsx` | sidebar brand, groups, family links, header scope line |
| `web/src/UsageView.jsx` + test, `web/src/Widgets.jsx` + test, `web/src/Breakdown.jsx`, `web/src/Timeline.jsx` | Usage layout |
| `web/src/Rail.jsx`, `web/src/FeedCard.jsx` (+ tests) | restyle |
| `web/src/WarningsPlanView.jsx`, `web/src/KitView.jsx`, `web/src/WorkspacesView.jsx`, `web/src/AgentsView.jsx` (+ tests) | restyle only |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/**` (except rebuilt static) | no API change |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |
| `web/package.json` dependencies | no new dependency |

---

## Test Plan

Vitest for Success 1–4; shell grep for 5; easy-ui MCP session (via HTTP JSON-RPC at `http://localhost:8765/mcp`: `ui_start_session` → `ui_navigate` → `ui_take_screenshot` / `ui_check` / `ui_get_page_state` → `ui_end_session`) on a fixture-backed dashboard at 375/1024/1440 px for the UI rows and Success 6; attach the easy-ui report path.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] `Skill({ skill: "verify" })` run on fixtures
- [ ] Supervisor notified: task ready for Stage 4 review