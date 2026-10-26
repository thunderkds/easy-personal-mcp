**Date**: 2026-10-26
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: *Workspace*, *Agent account*, *Connector id / alias*)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely, and `BRAINSTORMING_LOG_WORKSPACES.md` (verified facts W1–W10, edge cases)
4. Read `agents/backend.md`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide

---

## Requirement (Pillar 1 — Adapt the requirement)

Prepare the workspace milestone (HUN-61..HUN-72) for release as v0.2.0: the package version and the user docs must match what ships.

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Bump the package version to 0.2.0 and make README (and anything drift-tested against it, e.g. the landing site tests) state v0.2.0, the dashboard tabs Usage, Warnings & plan, Kit, Workspaces and Agents, and the HUN-72 rule for removing a pre-workspace connector.

**Out of scope** (what this task explicitly does NOT do):

* Tagging, installing or pushing (the Supervisor's ship plan; the owner runs it)
* `RUNBOOK.md` (the Supervisor updates it in the ship step)
* Any code behavior change

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* US-014
* FR-028

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, Stage 5 ship, under the owner's standing instruction 2026-10-26)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-72 merged (main 8bd3cf4)

**Entry point**: `easy-personal --version`; `README.md`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `pyproject.toml` version = `0.2.0` and `uv.lock` updated (`uv lock`); after `uv sync`, `uv run easy-personal --version` prints `0.2.0` | version |
| 2 | README states `Status: v0.2.0`, says the first command prints `0.2.0`, and lists the tabs `Usage, Warnings & plan, Kit, Workspaces and Agents` | docs |
| 3 | README `connector remove` line says a pre-workspace connector is removed only if its Claude Code entry starts `easy-personal-launch`; any other name is refused and nothing is removed | docs (HUN-72 D1) |
| 4 | `tests/test_readme.py` checks `v0.2.0` (not `v0.1.0`); site/readme tests pass | docs |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | main after this task | `uv run easy-personal --version` → `0.2.0` | automated (add a test if none checks the version string against pyproject) |
| 2 | README | readme/site tests green with v0.2.0 | automated |

### Verification Command (exact, runnable)

```bash
uv lock && uv run pytest tests/test_readme.py tests/test_site_content.py -q && uv run pytest -q && uv run ruff check . && uv run easy-personal --version
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-73.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-73.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

N/A — no UI in this task (docs + version only). All three UI Evidence rows are ☐ N/A.

---

## Approach

**Pattern reference**: HUN-56 (README for v0.1.0), `tests/test_readme.py`

**Vital slice**: version + README status/tabs/remove line.
**Cut list**: no CHANGELOG file (release notes live in `RUNBOOK.md` / the ship plan).

---

## Edge Case Checklist

- [ ] The landing site (HUN-57) quotes README text: if a site test fails, update the site text the same way, nothing more
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any output

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `pyproject.toml`, `uv.lock` | version 0.2.0 |
| `README.md` | status, version line, tabs, remove rule |
| `tests/test_readme.py` | v0.2.0 |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| real `~/.claude*/`, `~/.claude.json`, `~/.codex*/`, `~/.easy-personal/` | Never read or written by tests or verify |
| `RUNBOOK.md` | Supervisor-owned in the ship step |
| `src/` | No behavior change |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Update the README version test to v0.2.0 (fails first against the old README), then change README/pyproject.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Lint passes (`uv run ruff check .`)
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md`'s Evidence table (Hard-Stop Gate 5)
- [ ] Supervisor notified: task ready for Stage 4 review