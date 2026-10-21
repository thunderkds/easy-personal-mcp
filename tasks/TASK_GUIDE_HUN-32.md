**Date**: 2026-10-21
**Complexity Level**: C1
**Risk Level**: Medium
**Priority**: P0
**Assigned agent**: QA-Automation-Agent
**Agent guide**: `agents/qa.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read `agents/qa.md`
5. Note the **Complexity Level** above and apply the matching process (brainstorm / decompose / verify depth) from the Complexity matrix in your role guide (`agents/qa.md`)
6. **C2/C3 or multi-file tasks only**: read `memory/codebase-map.md` for directory layout, entry points, and blast-radius hotspots — skip if the task is C0/C1 and touches a single known file

---

## Requirement (Pillar 1 — Adapt the requirement)

M1 needs a trustworthy picture of the undocumented Claude Code transcript format and of the owner's real data volume before any parser or performance target is frozen (PRD NFR-005 is provisional; BRAINSTORMING_LOG F2).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Know every record type/shape that appears in the owner's transcripts per Claude Code version, and the real size of 30 days of data, and capture them as synthetic fixtures that contain no real content.

**Out of scope** (what this task explicitly does NOT do):

* Writing the parser (next task)
* Storing any real transcript text anywhere in the repo

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-001, FR-002, FR-003: ingest, metadata-only, abstain on unknown shapes
* NFR-005: provisional performance target to reset

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

**Entry point**: Standalone — N/A: produces fixtures and a findings note consumed by later tasks' tests

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | A findings note lists, per Claude Code `version`, every record `type` seen and the metadata fields used by FR-002 (with presence %) | inventory of shapes |
| 2 | Synthetic fixtures exist per version under `tests/fixtures/claude_code/<version>/`, including at least one unknown-type record and one truncated last line | fixtures for parser tests |
| 3 | Fixtures contain no real message text, paths or secrets — only invented values; a grep for the owner's home path and real session ids finds nothing | no real content |
| 4 | The note records total bytes, files and lines for the last 30 days and proposes the NFR-005 number | reset NFR-005 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | The owner's `~/.claude/projects/` (read-only) | Findings note + fixtures committed | manual review + grep check |
| 2 | Fixtures dir | No line matches `/home/` or a real `sessionId` from the scan | automated grep in test |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/fixtures_check -q   # asserts fixtures contain no real paths/ids
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-32.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-32.md` (review sections are never taken from the board).

## Approach

**Pattern reference**: `easy-verifier-mcp/tests/` fixture layout — conventions only — never import or vendor sibling code (PROJECT_SPEC Critical Constraints)

**Vital slice**: the per-version record-type inventory and the fixtures the parser will be tested against
**Cut list**: No parser code; No automation of the scan beyond a throwaway script kept out of `src/`

Scan read-only with a throwaway script (keep it under `experiments/`), count `type` × `version` and field presence; never print or copy message/tool content. Hand-write fixtures that reproduce each shape with invented values. Measure volume with `du`/line counts.

---

## Edge Case Checklist

- [ ] Records with no `version` field
- [ ] Undocumented types (`atis-latch`, `cost-state`, `queue-operation`)
- [ ] Truncated/partial last line in a live file
- [ ] Sidechain/sub-agent transcripts in nested folders
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `docs/spikes/transcript-shapes.md` | Findings note |
| `tests/fixtures/claude_code/<version>/*.jsonl` | Synthetic fixtures |
| `tests/fixtures_check/` | No-real-content check |
| `experiments/scan_transcripts.py` | Throwaway scan script |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

A pytest that greps every fixture for the owner's home path and for session ids listed in the (uncommitted) scan output; manual review of the note by the Supervisor.

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