**Date**: 2026-10-21
**Complexity Level**: C2
**Risk Level**: High
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read `agents/backend.md`
5. Note the **Complexity Level** above and apply the matching process (brainstorm / decompose / verify depth) from the Complexity matrix in your role guide (`agents/backend.md`)
6. **C2/C3 or multi-file tasks only**: read `memory/codebase-map.md` for directory layout, entry points, and blast-radius hotspots — skip if the task is C0/C1 and touches a single known file

---

## Requirement (Pillar 1 — Adapt the requirement)

Pick a provider, fill its form, keep secrets in the keychain, and have the connector registered in Claude Code without hand-editing config (PRD US-008–US-010, FR-016, FR-019; DDR-0001).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> End to end for one provider (Slack): declarative catalog entry → CLI prompts (secrets hidden) → keychain → `claude mcp add-json <name> --scope user` with command `easy-personal-launch exec <name>` (owner decision 2026-10-22: the separate launcher script, NFR-006); snippet when `claude` is missing; `doctor` checks keychain, CLI and entries.

**Out of scope** (what this task explicitly does NOT do):

* More providers (catalog breadth task)
* Dashboard forms
* Project-scope files

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-016: catalog files
* FR-019: claude mcp add-json, snippet fallback
* DDR-0001

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-21)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (`grill-with-docs` terminology run 2026-10-21)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-44 — Launcher

**Entry point**: `easy-personal connector add`, `easy-personal connector remove`, `easy-personal doctor`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Catalog entry validated against a schema (category, provider, fields, secret flags, launch, [claude.ai](<http://claude.ai>) flag) | FR-016 |
| 2 | `connector add slack` stores secrets in the keychain and runs `claude mcp add-json ... --scope user` pointing at the launcher; shows what will change and asks to confirm | FR-019, DDR-0001 |
| 3 | `claude` missing → prints the exact snippet, writes nothing | FR-019 |
| 4 | Existing entry with the same name → refuse and show both | edge |
| 5 | `connector remove` deletes keychain items and runs `claude mcp remove --scope user` | DDR-0001 |
| 6 | `doctor` reports keychain backend, `claude` on PATH, and that each connector entry points at the launcher | DDR-0001 follow-up |
| 7 | No secret appears in any output or log (leak suite) | NFR-002 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Fake `claude` binary on PATH recording args | exact add-json call; no secret in args | automated |
| 2 | No `claude` on PATH | snippet, exit 0, nothing written | automated |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/connectors -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-45.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-45.md` (review sections are never taken from the board).

## Approach

**Pattern reference**: None — no comparable prior art; follow DDR-0001

**Vital slice**: one provider end to end
**Cut list**: Only Slack in this task

Catalog as YAML in the package; writer shells out to `claude` with an argument list (no shell); JSON for add-json references only the launcher, never secret values.

---

## Edge Case Checklist

- [ ] `claude` returns non-zero → keychain write rolled back
- [ ] User aborts at confirm → nothing written
- [ ] Secret field empty
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/connectors/{catalog,writer}.py` | Catalog + writer |
| `src/easy_personal/catalog/slack.yaml` | Entry |
| `src/easy_personal/surfaces/cli.py` | connector + doctor commands |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Fake `claude` binary; keychain fake; rollback test; leak suite.

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