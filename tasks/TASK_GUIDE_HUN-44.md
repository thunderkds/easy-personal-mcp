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

Connector secrets live only in the OS keychain and reach the provider server only at launch (PRD FR-017, FR-018, NFR-006; DDR-0001).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> A keychain wrapper (Secret Service via `keyring`, service prefix `easy-personal/`) that refuses without a backend, and `launch.py` commands that read the secrets and exec the provider server with them in its environment, or print JSON headers.

**Out of scope** (what this task explicitly does NOT do):

* Catalog and config writing (next task)
* Any plaintext fallback

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-017: keychain only, refuse otherwise
* FR-018: exec / headers
* NFR-006: <200 ms, stdlib + keyring only

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

**Entry point**: `easy-personal exec <connector>` and `easy-personal headers <connector>`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | No keychain backend / locked → setup refuses with guidance; `exec` exits non-zero with one stderr line; nothing written to disk | FR-017 |
| 2 | `exec` replaces itself with the provider command (`os.execvpe`) with secrets only in env; secrets never in argv | FR-018 |
| 3 | `headers` prints JSON headers to stdout and nothing else | FR-018 |
| 4 | Import-graph test: `launch.py` imports only stdlib + `keyring` | NFR-006 |
| 5 | Start overhead < 200 ms (timing test with a fake keyring backend) | NFR-006 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Fake keyring with a secret | child env has it; `ps` argv does not | automated |
| 2 | No backend | refusal, exit ≠ 0 | automated |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/connectors/test_launch.py tests/connectors/test_keychain.py -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-44.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-44.md` (review sections are never taken from the board).

## Approach

**Pattern reference**: None — no comparable prior art; follow DDR-0001

**Vital slice**: exec path with env-only secrets + refusal
**Cut list**: No external secret managers (1Password etc.)

Separate console entry or fast path that avoids importing the core; keyring backend chosen by keyring defaults; a test backend for CI.

---

## Edge Case Checklist

- [ ] Secret Service locked in a headless session
- [ ] Connector name not found
- [ ] Provider command missing on PATH
- [ ] Unicode/newlines in secrets
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/launch.py` | exec/headers |
| `src/easy_personal/connectors/keychain.py` | Store/refuse |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Fake keyring backend; subprocess test inspecting child env/argv; import-graph test; timing test.

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