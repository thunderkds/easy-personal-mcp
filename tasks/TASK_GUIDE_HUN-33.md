**Date**: 2026-10-21
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P0
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

v1 needs one Python package where MCP, CLI (and later the dashboard API) call the same core (PRD FR-008; PROJECT_SPEC Architecture).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> A runnable, installable package skeleton: `uv tool install .` gives an `easy-personal` command, `easy-personal mcp` serves an MCP stdio server, both reaching a shared core, with pytest + ruff wired.

**Out of scope** (what this task explicitly does NOT do):

* Any source, store or metric logic
* The `web/` toolchain (separate task)
* HTTP MCP transport wiring beyond a flag that binds `127.0.0.1` only

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-008: MCP, CLI and dashboard share one core
* NFR-001: HTTP binds 127.0.0.1 only
* NFR-007: Linux, Python 3.11+, uv tool install

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-21)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (`grill-with-docs` terminology run 2026-10-21)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: None

**Entry point**: `easy-personal` (console script) and MCP server id `easy-personal`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `uv tool install .` then `easy-personal --version` prints the package version | installable CLI |
| 2 | `easy-personal mcp` starts an MCP stdio server that lists one placeholder tool calling the core | MCP entry sharing core |
| 3 | CLI and MCP get the same value from the same core function (test) | one core |
| 4 | Starting HTTP transport with host `0.0.0.0` is refused; default host is `127.0.0.1` | NFR-001 |
| 5 | `uv run pytest` and `uv run ruff check` pass in CI-like local run | test runner |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Fresh clone | `uv run pytest -q` green, `ruff check` clean | automated |
| 2 | `--host 0.0.0.0` | exits non-zero with a clear message | automated test |

### Verification Command (exact, runnable)

```bash
uv run pytest -q && uv run ruff check && uv tool install --force . && easy-personal --version
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-33.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-33.md` (review sections are never taken from the board).

## Approach

**Pattern reference**: `easy-context-mcp/pyproject.toml` and `easy-context-mcp/src/easy_context_mcp/server.py` (FastMCP setup, console script) — conventions only — never import or vendor sibling code (PROJECT_SPEC Critical Constraints)

**Vital slice**: console script + MCP stdio entry + shared core module + test runner
**Cut list**: No config file loading yet beyond defaults; No logging framework beyond stdlib logging

src layout `src/easy_personal/{core,surfaces,sources,connectors}/` with empty packages; `surfaces/cli.py` (argparse) and `surfaces/mcp.py` (FastMCP) both call `core`. Runtime data dir `~/.easy-personal/` resolved in one config function (overridable by env for tests).

---

## Edge Case Checklist

- [ ] `~/.easy-personal/` missing → created with 0700
- [ ] Python < 3.11 → clear install error
- [ ] MCP stdio must not print anything to stdout except protocol
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `pyproject.toml` | Package, deps (mcp, keyring), console script, dev group |
| `src/easy_personal/**` | Skeleton packages, cli, mcp, core placeholder |
| `tests/test_skeleton.py` | Shared-core + host guard tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

pytest: CLI and MCP placeholder return the same core value; host guard refuses 0.0.0.0; data dir created with 0700.

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