**Date**: 2026-10-21
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Common-Infrastructure-Agent
**Agent guide**: `agents/common-infrastructure.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read `agents/common-infrastructure.md`
5. Note the **Complexity Level** above and apply the matching process (brainstorm / decompose / verify depth) from the Complexity matrix in your role guide (`agents/common-infrastructure.md`)
6. **C2/C3 or multi-file tasks only**: read `memory/codebase-map.md` for directory layout, entry points, and blast-radius hotspots — skip if the task is C0/C1 and touches a single known file

---

## Requirement (Pillar 1 — Adapt the requirement)

The dashboard is a React + Vite app whose built assets ship inside the Python package so install needs no Node; npm supply chain pinned (BRAINSTORMING_LOG Option B; PROJECT_SPEC Critical Constraints).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> A minimal `web/` app (one placeholder page) with exact-version lockfile, `npm ci`, a build that writes into the package's static folder, and a check that fails a release when the build is missing or stale.

**Out of scope** (what this task explicitly does NOT do):

* Real dashboard views (frontend tasks)
* Server (backend task)

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-023 (prerequisite)
* NFR-001: no remote assets

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-21)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (`grill-with-docs` terminology run 2026-10-21)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-33 — Skeleton

**Entry point**: `npm --prefix web run build` output in `src/easy_personal/dashboard/static/`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `npm ci && npm run build` produces assets in the package static folder | Option B |
| 2 | Built HTML/JS reference no remote URL (test greps for `http`) | NFR-001 |
| 3 | `uv build` wheel contains the static assets | install without Node |
| 4 | Release check fails when static is missing or older than `web/src` | release check |
| 5 | No analytics/telemetry packages in the lockfile (allow-list check) | supply chain |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Fresh clone | build + wheel contains assets | automated script |

### Verification Command (exact, runnable)

```bash
npm --prefix web ci && npm --prefix web run build && uv build && python3 scripts/check_static.py
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-48.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-48.md` (review sections are never taken from the board).

## Approach

**Pattern reference**: `easy-verifier-mcp/site/` (a sibling web build) — conventions only — never import or vendor sibling code (PROJECT_SPEC Critical Constraints)

**Vital slice**: pinned build into the package
**Cut list**: No component library or design system yet

Vite `build.outDir` → package static dir, `base: './'`; commit `package-lock.json`; small Python check script.

---

## Edge Case Checklist

- [ ] Vite injects modulepreload polyfill URLs
- [ ] Source maps leaking absolute paths → disable in prod
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/**` | App + config + lockfile |
| `src/easy_personal/dashboard/static/` | Build output |
| `scripts/check_static.py` | Release check |
| `pyproject.toml` | Include static files |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Script-based checks run in pytest.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Security review: `Skill({ skill: "security-review" })` run (if Medium/High risk)
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md`'s Evidence table (Hard-Stop Gate 5)
- [ ] `Skill({ skill: "verify" })` run — feature confirmed working in running app
- [ ] `memory/MEMORY.md` updated (if new patterns or feedback learned)
- [ ] Supervisor notified: task ready for Stage 4 review