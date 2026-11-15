# TASK_GUIDE — T089: Landing page + READMEs: the Rust `.deb` is the main install path (app window, tray, Recent alerts); Python `uv` install second
**Date**: 2026-11-15
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P2
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md`, `PROJECT_SPEC_SITE.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/frontend.md`
5. Read `site/index.html` (hero install box, `section.quickstart`, `#install`), `README.md` (`## Install`, `## Rust app (default from T062)`, `## Quick start`), `rust/packaging/README.md` (the source of truth for the `.deb` steps), `rust/README.md` around line 240, `rust/docs/CUTOVER.md` (skim)
6. Read `tests/test_site_content.py` (install box, `QUICKSTART_COMMANDS`, `check_install`, `check_quickstart`), `tests/test_site_theme.py`, `tests/test_site_deploy_config.py`
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder

---

## Requirement

**Original request** (owner, 2026-11-15): "did we update the guide to run the app into the index page" → Supervisor: no, the page shows only the Python `uv` install → owner: "yes, should be" (the Rust `.deb` becomes the main install path; Python `uv` stays as the second option).

**Restated intent**:
> 1. `site/index.html`: the hero install box, the Quick start strip and the `#install` "Install and deploy" section lead with the Rust app on Ubuntu: clone, `rust/packaging/build-deb.sh`, `sudo apt install ./rust/target/deb/easy-personal-rs_*_amd64.deb`, then the per-user steps of `rust/packaging/README.md` (`easy-personal-rs schedule install`, `claude mcp add --scope user easy-personal -- /usr/bin/easy-personal-rs mcp`, dashboard window, tray with `--autostart on`).
> 2. The page says in one or two sentences what the tray does: icon state, the menu rows, and the "Recent alerts" block (newest 5 alerts, "All alerts in dashboard…" when more; listed even while alerts are paused). Plus the tray-host note (Ubuntu AppIndicators extension).
> 3. The Python `uv` install stays on the page and in `README.md` as a clearly labelled second option ("Python app"), not deleted.
> 4. `README.md`: the same commands, verbatim, so the drift tests hold (every install-box / quick-start command is a README line). `README.md` `## Install` leads with the `.deb`; the Python block follows under its own heading.
> 5. `rust/README.md` line ~240: drop "packaging" and "a tray or menu" from "Not done" and point to `rust/packaging/README.md` and the `tray` command.
> 6. Drift tests (`tests/test_site_content.py`) updated to the new commands (`QUICKSTART_COMMANDS`, install box) and one new test: the first install command block on the page is the `.deb` path (Rust before Python).

**Out of scope**: version pill and release numbers (do not change `0.4.0` / `0.3.0` claims; use the `*` glob in the `.deb` file name so no version is written), new screenshots, the "Twelve MCP tools" / connectors sections, macOS/Windows, publishing a `.deb` download (none exists: the user builds it), Vercel config.

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner ("yes, should be", 2026-11-15)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: PROJECT_SPEC_SITE.md (landing site), T087 (`.deb`), T088 (Recent alerts)

---

## Dependencies & Reachability
**Depends on**: T087, T088 merged
**Entry point**: `site/index.html` `#install`

## Dependency rule
No new dependency. The page stays plain HTML + CSS, no external hosts (PROJECT_SPEC_SITE.md).

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Hero install box lines are the `.deb` path, each a README line (existing test, updated data) | intent 1, 4 |
| 2 | `QUICKSTART_COMMANDS` = the Rust four (install `.deb`, schedule/ingest, `claude mcp add … /usr/bin/easy-personal-rs mcp`, dashboard or tray); `check_quickstart` passes; the negative test still fails on an edited README line | intent 1, 4 |
| 3 | New test: in `#install`, the `.deb` block comes before the `uv tool install` block, and both are present | intent 1, 3 |
| 4 | New test: the page and README name the tray's "Recent alerts" block and the AppIndicators tray-host note | intent 2 |
| 5 | `rust/README.md` no longer lists packaging or a tray as "Not done" (grep test or assertion) | intent 5 |
| 6 | `uv run pytest tests/test_site_content.py tests/test_site_theme.py tests/test_site_deploy_config.py -q` all pass; full `uv run pytest -q` no new failure | all |
| 7 | Page screenshots at desktop and 375 px width of the hero and `#install` (headless browser if available) — owner visual check | intent 1 (Gate 6) |

---

## Evaluation & Acceptance

### Verification Command
```bash
uv run pytest tests/test_site_content.py tests/test_site_theme.py tests/test_site_deploy_config.py -q 2>&1 | tail -3
```

### Evidence (filled by reviewer at Stage 4/5)
| Item | Result | Evidence |
|---|---|---|
| New test(s) cover acceptance criteria | ☑ | new tests in tests/test_site_content.py (deb-before-uv on page and README, Recent alerts + AppIndicators named, rust/README Not done, apt-line negative); Supervisor rerun `uv run pytest tests/test_site_content.py tests/test_site_theme.py tests/test_site_deploy_config.py tests/test_readme.py -q` → `98 passed in 0.97s` |
| Verification command | ☑ | full `uv run pytest -q` (Supervisor, after Stage 4 fixes) → `1856 passed, 2 skipped, 7 deselected, 1 xfailed, 1 warning in 165.40s` |
| code-review | ☑ | Supervisor inline 2026-11-15: 1 P1 fixed (`014f7eb`: `easy-personal-rs dashboard` is an alias of `serve` and opens no window → `app`, as the .desktop entry runs), 1 pre-existing failure fixed in scope (views stat 9 → 10 after T077); 1 P1 process (startup reads partial) |
| security-review | ☐ N/A | Risk Low: static docs; `test_site_tree_has_no_host_path_email_or_token` still runs |
| UI: visual regression | ☑ | tasks/T089-screens/hero-desktop.png + hero-375.png retaken after fixes (headless Chrome); install-*.png from the agent (pre-fix text, same layout) |
| UI: design-system compliance | ☑ | test_site_theme passes; one new CSS rule `h3.sub { margin-top: 1rem; }`, no new colours |
| UI: responsiveness | ☑ | 375 px: scrollWidth == clientWidth (agent iframe check); commands scroll inside `pre` |

## Demonstration
> BEFORE/AFTER/DELTA/WITNESS go in `tasks/TASK_REVIEW_T089.md`. BEFORE = the current hero install box and `#install` text, quoted.

## UI / Design Acceptance Criteria

### 1. Visual Regression
| Screen / Component | Verification method | Expected result |
|---|---|---|
| hero install box, quick start, `#install` | screenshot desktop + 375 px | same look as today; only text and step count change |

### 2. Design-System Compliance
| Criterion | Verification method | Expected result |
|---|---|---|
| Colours / classes | `tests/test_site_theme.py`; reuse existing classes (`install-box`, numbered steps) | no new hex colours, no new CSS unless needed |

### 3. Layout / Responsiveness
| Viewport | Verification method | Expected result |
|---|---|---|
| 375 px | screenshot | no horizontal scroll; long commands scroll inside their `pre` |

---

## Approach
Test first: change the test data/new tests, see them fail, then edit README, page, rust README. Copy commands from `rust/packaging/README.md` exactly. Writing rules: AGENTS.md Writing Principles (ASD-STE100) for the page and README text. Record cuts.

## Edge Case Checklist
- Commands with `sudo` and the `*` glob must appear verbatim in README (drift test)
- `~/.local/bin` cutover copy comes before `/usr/bin` on PATH: use `/usr/bin/easy-personal-rs` in `claude mcp add`, as `rust/packaging/README.md` does
- Hero box must stay short (copy button copies all lines)
- No host path, email or token in `site/` (existing test)
- Python steps still correct and complete

## Files to Change (Predicted)
`site/index.html`, `README.md`, `rust/README.md`, `tests/test_site_content.py`

## Files Must NOT Touch
`rust/crates/**`, `src/easy_personal/**`, `web/**`, `vercel.json`, `.vercelignore`, `rust/packaging/**` (source of truth; report a mistake there instead of fixing it)

## Test Plan
ACs 1-6 as automated tests with pasted output; AC 7 screenshots.

## Completion Checklist
- [x] Every AC with pasted output in `tasks/TASK_REVIEW_T089.md`
- [x] Commits on branch `T089` only; final report with `Status:`
