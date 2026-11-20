**Date**: 2026-11-09
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `PROJECT_SPEC_SITE.md` (the site spec: section order, drift constraint, safety rules)
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/frontend.md` with the Read tool and no `limit`
5. Read `site/index.html` (`#dashboard`, `#hero`), `tests/test_site_content.py` (all of it: the page is test-locked) and the README sections named below

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-09: "I just care about the information from the index page includes the change for improvement."

**Measured gap.** `site/index.html` was last edited on 2026-11-05 and says `v0.4.0`. Since then `main` gained
the changes below. The page's `#dashboard` section shows four screenshots and no list of what the
dashboard does, and the README has no text for any of them (`git log d11c075..HEAD -- README.md` shows
only a kernel-link fix).

**Restated intent**:

> A developer who reads the landing page learns what the dashboard does today, including every
> user-visible change merged since 0.4.0, in plain words. The README says the same things, so the
> page and the README do not drift apart.

**The changes to describe** (all merged on `main`; read each FR in `PRD.md` before you write its line —
describe what the FR says, no more):

| Change | Requirement |
| -- | -- |
| Live view: one row per active session with bars, a kit-stage chip, a tool label for each call, rows that open inline, and a warm start that lists active sessions after a restart | FR-071, FR-072, FR-083, FR-084, FR-085, FR-086 |
| Kit view "Kit by workspace": easy-verifier scores and easy-ui use for each workspace | FR-087, FR-088 |
| Kit view report folders under each workspace row; a folder opens to its files; a file opens in a new tab; the report keeps its own CSS and shows its embedded screenshots; nothing runs from a report | FR-089, FR-090, FR-093, FR-094 |
| Usage opens the Scratch row (repos in no workspace); the Workspaces page lists those repos with "Add to" | FR-069, FR-070 |
| Saved investigate reports, with the server's own numbers (`save_report`) | FR-076, FR-077, FR-078 |

**Do not describe** FR-091 (store health check) or FR-092 (send-command button): they are not built
(HUN-244, HUN-245). The page states only what `main` does.

**Out of scope**: a version bump or a release (the owner decides; keep the pill and README status line at
`v0.4.0` and say nothing like "0.5"); new screenshots (the `#dashboard` section holds at most four
and has four); a new page section or any change to section order, ids, the flow diagram, the install
box, the tool table or the connector table; changing `web/`, `src/` or `PRD.md`.

**Requirement Refs**: FR-067, FR-071, FR-072, FR-076, FR-083 to FR-090, FR-093, FR-094

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [ ] Restated intent confirmed to match the user's request
- [ ] Domain terms align with `PROJECT_SPEC.md` glossary and `PROJECT_SPEC_SITE.md`
- [ ] Every Acceptance Criterion below traces to a line in the Requirement
- [ ] All Requirement Refs exist in `PRD.md` and are fully covered

---

## Dependencies & Reachability

**Depends on**: nothing. **Entry point**: `site/index.html` served from `main` by the Vercel Git integration; `README.md` on GitHub.

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `#dashboard` holds a list headed "What the dashboard does" (plain HTML list, no new section, no new id on `<section>`), one item per row of the table above, each item at most two short sentences | FR-071 to FR-094 |
| 2 | Every claim in the list is true of `main`: for each item the implementer cites the FR and one test or file that proves it, in the review file. No claim about FR-091 or FR-092 | all |
| 3 | The README gains matching text in its dashboard part (after "Memory view" or in the Live view part, wherever each topic already lives), using the same terms as the page: *session row*, *kit stage*, *Kit by workspace*, *report folder* | all |
| 4 | The page and the README say no version other than `v0.4.0` and promise no release | — |
| 5 | `uv run pytest tests/test_site_content.py tests/test_site_theme.py tests/test_site_deploy_config.py -q` passes with no test deleted or loosened. Add one test that fails if an item of the new list is removed or if the list names FR-091 or FR-092 | — |
| 6 | The safety scan still passes: no host path, email, token-shaped string or external host on the page | NFR-002 |
| 7 | **Real run.** Serve `site/` (`python3 -m http.server` from `site/`, bound to 127.0.0.1) and open it in headless Chrome at 1280x720 and at 390x844: the new list is visible, readable, with no horizontal scroll and no overlap; dark neon theme only. Screenshot both, saved under `tasks/evidence-<ID>/` | — |

---

## UI / Design Acceptance Criteria

Plain text list in the page's existing style. Reuse existing CSS classes; add none unless a list style is
missing. Visual regression: before/after screenshot of `#dashboard` at 1280x720. Design-system
compliance: tokens only, no hex fill. Responsiveness: AC 7.

---

## Evaluation & Acceptance

### Verification Command (exact, runnable)

```bash
uv run pytest tests/test_site_content.py tests/test_site_theme.py tests/test_site_deploy_config.py -q && uv run pytest -q
```

## Approach

**Vital slice**: one list in `#dashboard`, matching README text, one drift test. **Cut list**: no new
screenshots, no new section, no release, nothing about FR-091 or FR-092.

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `site/index.html` | the list in `#dashboard` |
| `README.md` | dashboard text for the changes above |
| `tests/test_site_content.py` | AC 5 test |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `web/`, `src/`, `PRD.md`, `pyproject.toml` | Not part of this task; no release |
| `site/*.png`, the flow SVG, install box, tool and connector tables | Out of scope |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table
- [ ] UI Evidence rows filled
- [ ] Supervisor notified: task ready for Stage 4 review

## Questions Log

* 2026-11-09 The owner's words do not say "page" or "README". Read "index page" as `site/index.html` (the only index page of this project that the owner sees on the web); the README is included because the site's drift constraint makes README the source for its text.