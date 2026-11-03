# TASK_GUIDE_HUN-201 — Easy Kit link → easy-kits.vercel.app; landing page flow made visual (SVG system diagram + quick-start strip)

**Date**: 2026-11-03
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC_SITE.md` in full (scope of `site/`, drift constraint, stack: plain HTML + CSS, no external hosts, no scripts library) and `PROJECT_SPEC.md` glossary
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-067, `site/index.html` in full, `tests/test_site_content.py`, `tests/test_site_theme.py`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-03: "the easy kit just update the link to https://easy-kits.vercel.app/, let update from our app. by the way, update the index page more visualize, will be easy to catch with the flow, diagram also. let check and update"

**Restated intent**:

> **Part A — Easy Kit link.** Replace every `https://personal-agentic-claude.vercel.app/` with `https://easy-kits.vercel.app/`:
> `web/src/App.jsx` (family list, `Easy Kit`), `web/src/App.test.jsx`, `site/index.html` (header family nav + footer), `scripts/check_static.py` (`ALLOWED_URL_PREFIXES`), `tests/test_site_content.py` (`ALLOWED_HOSTS`, `FAMILY_ORDER`, line ~477), `tests/test_web_toolchain.py` (fixture string). The README GitHub repo link `https://github.com/thunderkds/personal-agentic-claude` is a repo link, not the site link: keep it (test `test_easy_kit_links_kept`).
>
> **Part B — Landing page `site/index.html` easier to catch.** A first-time visitor must get the whole idea in a few seconds by looking, not reading:
>
> * **System diagram** in `#flow`: one inline `<svg>` (no library, no script) showing the full picture left → right: *sources* (Claude Code transcripts, kit parts: easy-memory, easy-verifier, easy-ui, easy-context, git) → `ingest` (metadata only, text dropped) → local SQLite store → three outputs (MCP tools for Claude, CLI, dashboard on 127.0.0.1) → you / Claude. A second, lower lane: catalog form → OS keychain → `claude mcp add-json` → launcher → provider server. Short plain-word labels, arrows, a visible "stays on your machine" boundary around the local part. `role="img"` + `<title>`/`<desc>` (or `aria-labelledby`) describing it. Uses the page's existing CSS colour tokens (`currentColor` / CSS variables) so it reads in dark and light. On a phone (≤ 600 px) it must stay readable: either the SVG stacks to a vertical variant, or it scales with a `viewBox` and the existing step lists stay as the readable text fallback below.
> * Keep the existing two lane step lists (`data-lane` / `data-step` attributes the tests read) as the captioned text version under the diagram; restyle them as numbered connected cards/steps (CSS only) so the order is visual.
> * **Quick-start strip** right under the hero: 4 numbered steps with an icon each — Install → Ingest → Register with Claude Code → Open the dashboard — each with its one verbatim README command (commands must stay verbatim in README, as the drift test checks). Links to `#install` for the full guide.
> * Keep section order, section IDs, the drift rules, no external hosts, CSP-free static page, `prefers-reduced-motion` respected for any animation (animation optional; none required).

**Out of scope**:

* Dashboard (`web/`) changes other than the link
* New sections beyond the quick-start strip; new screenshots
* Any backend change; Vercel config change
* Changing the README repo links

**Requirement Refs**: FR-067; `PROJECT_SPEC_SITE.md` Page content §1, §2, §7

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, 2026-11-03)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None

**Entry point**: `site/index.html` `#flow` (system diagram `svg`), quick-start strip after `#hero`; dashboard sidebar family link `Easy Kit` in `web/src/App.jsx`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `git grep personal-agentic-claude.vercel.app` returns nothing outside `memory/`, `tasks/`, `docs/`; the dashboard `Easy Kit` link and both site family lists point to `https://easy-kits.vercel.app/`; `scripts/check_static.py` allows the new URL and the built bundle passes it | FR-067 Part A |
| 2 | `#flow` contains one inline `svg` with `role="img"` and an accessible name/description, nodes for every Read-lane step and Connect-lane step named in `READ_STEPS` / `CONNECT_STEPS` (test derives from those lists), and a "your machine" boundary label | FR-067 Part B |
| 3 | The existing `data-lane` / `data-step` step lists remain and still pass the current flow tests | FR-067 |
| 4 | A quick-start strip after `#hero` has 4 numbered steps; each command in it appears verbatim in `README.md` (test) | FR-067 |
| 5 | No external host, no `<script src>`, no new file types beyond `site/`; existing site theme/content/deploy tests pass | PROJECT_SPEC_SITE.md Stack |
| 6 | The SVG uses colour tokens / `currentColor` only (no hard-coded hex outside `:root` token blocks), test-checked | FR-067 (dark + light) |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/test_site_content.py tests/test_site_theme.py tests/test_site_deploy_config.py tests/test_readme.py tests/test_web_toolchain.py -q && cd web && npx vitest run && cd .. && npm --prefix web run build && python3 scripts/check_static.py
```

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| `site/index.html` top + `#flow`, 1280×800 | headless screenshot BEFORE and AFTER | quick-start strip visible above the fold; diagram shows both lanes and the machine boundary |
| Dashboard sidebar family list | vitest | `Easy Kit` → new URL |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors / fonts | reuse the page's `:root` tokens | dark and light both readable (screenshot each with `prefers-color-scheme`) |
| Accessibility | test + manual | SVG has accessible name; text fallback lists kept; contrast ≥ 4.5:1 for labels |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1280×800 and 375×812 | screenshot + `document.documentElement.scrollWidth <= innerWidth` | no horizontal page scroll; diagram readable or stacked on 375 px |

---

## Approach

**Vital slice**: link swap (all 7 places + tests) → system SVG in `#flow` → step lists restyled as numbered connected steps → quick-start strip → screenshots.
**Cut list**: no animation required, no JS, no new screenshots, no dark/light toggle, no change to other sections' content.

---

## Edge Case Checklist

- [ ] SVG text does not overflow its boxes at the smallest supported width
- [ ] Reduced motion: any animation off
- [ ] Print / no-CSS: the text step lists still tell the full flow
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `site/index.html` | links, quick-start strip, system SVG, step styling |
| `web/src/App.jsx`, `web/src/App.test.jsx` | Easy Kit URL |
| `scripts/check_static.py` | allowlist URL |
| `tests/test_site_content.py`, `tests/test_web_toolchain.py` | URL + new tests (diagram, quick-start) |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/` (except rebuilt gitignored static) | No backend change |
| `README.md` repo links | GitHub repo links stay |
| `.claude/`, `agents/`, `skills/`, `templates/` | Kit framework files |

---

## Test Plan

Pytest site tests (new: SVG presence + accessible name + nodes from READ_STEPS/CONNECT_STEPS; quick-start commands verbatim in README; no old URL; SVG no hard-coded hex), vitest App family link, build + `check_static.py`, headless screenshots BEFORE/AFTER at 1280×800 and 375×812, dark and light.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled (Hard-Stop Gate 6)
