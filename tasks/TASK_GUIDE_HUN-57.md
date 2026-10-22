**Date**: 2026-10-22
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` and `PROJECT_SPEC_SITE.md` (the site's spec — governs this task)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read `agents/frontend.md`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-10-22: "compose the index page to deploy the guide also"; plan approved "Yes, as described" (sibling pattern from easy-context-mcp HUN-20).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> A single static page in `site/`, deployed by Vercel from `main`, where a developer from GitHub gets the idea of easy-personal at a glance and can install, run and deploy it: a two-lane flow diagram (Read: transcripts/kit reports → ingest → local store → MCP · CLI · dashboard; Connect: form → keychain → `claude mcp add-json` → launcher), dashboard screenshots taken from fixtures, the install & deploy guide, the MCP tools and the connector catalog — with tests that fail when the page drifts from the README, the MCP server's tool list or the catalog.

**Out of scope** (what this task explicitly does NOT do):

* Connecting the Vercel project (operator action) and any CI deploy workflow
* Any change to `src/` behaviour or to `README.md` content (HUN-56 owns the README)
* Live data from the page; analytics; external fonts/CDNs/scripts
* The owner's real usage data in any screenshot

**Requirement Refs**: none in `PRD.md` by design (site scope); ACs trace to `PROJECT_SPEC_SITE.md`.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner answer "Yes, as described", 2026-10-22)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary and `PROJECT_SPEC_SITE.md`
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] Requirement Refs: none in `PRD.md` by design; ACs trace to `PROJECT_SPEC_SITE.md`

---

## Dependencies & Reachability

**Depends on**: HUN-56 — README install, usage and deploy guide (the page quotes its pitch and commands verbatim)

**Entry point**: `site/index.html`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `site/index.html` has, in this order, sections with ids `hero`, `flow`, `dashboard`, `install`, `tools`, `connectors`, and a `footer`; the hero contains the README pitch line verbatim | PROJECT_SPEC_SITE content + order |
| 2 | `#flow` holds one inline diagram (SVG or HTML/CSS, no library) with two labelled lanes *Read* and *Connect* as in `PROJECT_SPEC_SITE.md`; each step has a one-line caption | flow |
| 3 | `#tools` lists exactly the tools `src/easy_personal/surfaces/mcp.py` registers; the test derives the set from the server, not a hard-coded list | drift |
| 4 | `#connectors` lists exactly the providers in `src/easy_personal/catalog/*.yaml`, grouped by category, with the claude.ai-connector flag; the test derives them from the catalog | drift |
| 5 | Every command shown in `#install` appears verbatim in `README.md`; the test fails if one does not | drift |
| 6 | `#dashboard` shows 2–4 screenshots committed under `site/` (PNG/WebP, each ≤ 400 kB), captured from fixture data, each with alt text; a committed script under `scripts/` or the review notes document how they were produced | screenshots, fixtures only |
| 7 | Nothing under `site/` contains `/home/`, `/Users/`, `/workspace`, `/tmp/`, an email address, a token-shaped string or the dashboard token; no `<script src=`, no external stylesheet/font host; external links only to the GitHub repo and the two sibling sites | safety |
| 8 | `vercel.json` is exactly `{"outputDirectory": "site"}`; `.vercelignore` is the allowlist `*`, `!site/`, `!site/**`, `!vercel.json` | Vercel from `main` |
| 9 | Footer links GitHub repo, README, [https://personal-agentic-claude.vercel.app/](<https://personal-agentic-claude.vercel.app/>) and [https://easy-verifier.vercel.app/](<https://easy-verifier.vercel.app/>) | Easy family links |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | The committed page | sections, drift, safety, deploy config hold | `tests/test_site_content.py`, `tests/test_site_theme.py`, `tests/test_site_deploy_config.py` |
| 2 | The page opened in headless Chrome at 375 / 768 / 1280 | no horizontal scroll, diagram readable, no external request | Supervisor screenshots + net log |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/test_site_content.py tests/test_site_theme.py tests/test_site_deploy_config.py -q && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-57.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-57.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen / Component | Verification method | Expected result |
| -- | -- | -- |
| Whole page (hero, flow, dashboard, install, tools, connectors, footer) | headless Chrome screenshot + LLM vision review against this guide's AC | matches AC; no broken layout |

### 2. Design-System Compliance

| Criterion | Verification method | Expected result |
| -- | -- | -- |
| Colors match the Easy Kit dark neon theme | CSS audit vs `../easy-context-mcp/site/index.html` custom properties | colors via CSS custom properties defined once |
| Typography | CSS audit | system font stack, no external font host |
| Spacing / layout | screenshot review | consistent spacing scale |

### 3. Layout / Responsiveness

| Viewport | Verification method | Expected result |
| -- | -- | -- |
| Mobile (375px) | headless Chrome | single column, no horizontal scroll, diagram stacks or scrolls inside its own box |
| Tablet (768px) | headless Chrome | readable |
| Desktop (1280px) | headless Chrome | full layout |

---

## Approach

**Pattern reference**: `../easy-context-mcp/site/index.html`, `../easy-context-mcp/tests/test_site_content.py`, `../easy-context-mcp/tests/test_site_theme.py`, `../easy-context-mcp/tests/test_site_deploy_config.py`, `../easy-context-mcp/vercel.json`, `../easy-context-mcp/.vercelignore` (conventions only — write this repo's own files; do not import sibling code)

**Vital slice**: hero + flow diagram + install & deploy + drift tests + deploy config
**Cut list**: no light theme toggle, no search, no per-rule docs, no copy-to-clipboard script

Screenshots: produce them from the dashboard running on fixtures (`tests/fixtures/claude_code`, temp `EASY_PERSONAL_HOME`, fixture `cwd` rewritten, `BROWSER=true`), with headless Chrome (`google-chrome --headless=new --screenshot=… --window-size=1280,900 "<url>"`). The URL fragment holds the per-launch token: never write it into any committed file. Connectors tab screenshot: run with no `claude` on PATH and `PYTHON_KEYRING_BACKEND=keyring.backends.fail.Keyring` so nothing is ever written.

---

## Edge Case Checklist

- [ ] A screenshot shows no absolute path, token, email or real project name (fixture project `demo-app` only)
- [ ] Page works with JavaScript disabled (no script needed)
- [ ] Planted secret/text fixtures never appear on the page

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `site/index.html`, `site/favicon.svg`, `site/*.png` | New page |
| `vercel.json`, `.vercelignore` | Deploy config |
| `tests/test_site_content.py`, `tests/test_site_theme.py`, `tests/test_site_deploy_config.py` | Drift + safety tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `README.md`, `src/**` | Owned elsewhere |
| `~/.claude.json`, real keychain, `~/.claude/projects/`, `~/.easy-personal/` | Never in screenshots or verify |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/` | Kit framework files |

---

## Test Plan

Three pytest files mirroring the sibling site tests; Supervisor headless Chrome evidence.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md`
- [ ] Supervisor notified: task ready for Stage 4 review