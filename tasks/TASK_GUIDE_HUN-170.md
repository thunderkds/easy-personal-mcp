**Date**: 2026-10-31
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: QA-Automation-Agent
**Agent guide**: `agents/qa.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `experiments/live-trace/NOTES.md` (Phase 0 source table) once the harness task has written it; `PRD.md` FR-002, NFR-002

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-31: "do all the experimental" for the plan doc "Claude Live Trace — Experimental Plan" ([https://claude.ai/artifact/EVYWZrCSD34Cn35cAaVusB](<https://claude.ai/artifact/EVYWZrCSD34Cn35cAaVusB>)); breakdown approved the same day ("approve, with the key you need in any service, if you don't have, dont test that").

**Restated intent**:

> Compare the sources on the same runs and write the recommendation: one combined configuration (E1 tail + E2 hooks + E3 OTel + E4 stream-json all on) over 5 standard runs, an E5 section from the Phase 0 docs only (no Admin API key exists — owner: do not test it), and `REPORT.md` in the plan's template.

**Out of scope**:

* Calling any Admin API endpoint (no key; owner instruction 2026-10-31)
* The collector + dashboard prototype (next review stop)

**Requirement Refs**: FR-002 and NFR-002 (metadata only, no content) — the spike's only PRD constraints; the rest traces to the plan doc.

### Shared spike rules (all live-trace tasks)

* Plan: the owner's doc "Claude Live Trace — Experimental Plan" ([https://claude.ai/artifact/EVYWZrCSD34Cn35cAaVusB](<https://claude.ai/artifact/EVYWZrCSD34Cn35cAaVusB>)). Phase 0 (docs check, 2026-10-31) is done; its source table is copied into `experiments/live-trace/NOTES.md` by the harness task. Everything in the plan below Phase 0 is a hypothesis to measure, not a requirement: a source may end with verdict "drop".
* Folder: `experiments/live-trace/` only (`harness/`, `e1/`..`e4/`, `report/`). Python 3 stdlib only — no new dependency in `pyproject.toml`. Tests for spike code live in `experiments/live-trace/tests/` and run with `uv run pytest experiments/live-trace/tests -q`.
* Metadata only (PRD FR-002 / NFR-002 hold for the spike): never store, print or commit prompt text, assistant text, tool input or tool output. Keep ids, names, timestamps, counts, token fields, cost numbers, event and attribute NAMES. Never set `OTEL_LOG_USER_PROMPTS`, `OTEL_LOG_TOOL_DETAILS`, `OTEL_LOG_TOOL_CONTENT`, `OTEL_LOG_RAW_API_BODIES`. The plan's `--capture-content` flag is dropped.
* Never read, print or log credentials, OAuth tokens or API keys. Never edit any `~/.claude*/` file or global settings; hooks and env vars go only into the scratch session folder's `.claude/settings.json` or the child process env.
* Test sessions run headless with the harness (`harness/run_session.py`) in a scratch folder made by `mktemp -d` OUTSIDE the repo, model Haiku (`--model haiku`). The live account is `CLAUDE_CONFIG_DIR=/home/hungnguyenhuu/.claude-hung2` (transcripts under its `projects/`); read transcripts read-only.
* Latency reference: the `timestamp` Claude Code writes on the transcript record holding a `tool_use` (matched by `tool_use_id`); a source's latency = its local receive time minus that reference, same machine clock. Report median and p95 in ms.
* Every claim in a RESULT.md carries a number (ms, count, %) or is listed as an open question. RESULT.md sections: Tried / Worked / Failed / Numbers / Edge cases / Verdict (keep | keep with limits | drop).
* Run the standard session 5 times per measured configuration. Usage budget for the whole spike is about 25 short Haiku sessions; stay inside your task's share and say how many you used.
* Collector ports: 4318 is reserved for the OTLP receiver (E3); the hook collector (E2) uses 4390.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-31)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: HUN-166 (E1), HUN-167 (E2), HUN-168 (E3), HUN-169 (E4)

**Entry point**: Standalone — N/A: spike code under `experiments/live-trace/`, run by hand; nothing in `src/` calls it

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `report/combined.py` starts the E2 collector, E3 receiver and E1 tail, runs 5 standard sessions with E4 capture, then joins all sources by tool_use_id: per-source capture %, median/p95 latency on the SAME events, overhead vs baseline | fair comparison |
| 2 | `report/e5.md`: Admin usage/cost and Claude Code analytics endpoints, key type, freshness and granularity from NOTES.md; Pro/Max: no usage API, what `/usage` shows; marked 'docs only, not tested — no Admin key' | E5 |
| 3 | `REPORT.md` (plan template): Recommendation, Comparison table (latency, completeness, setup effort, overhead, verdict per source), Account usage findings, Risks and open questions, v2 ideas | deliverable |
| 4 | A check script fails when a RESULT.md lacks a required section or REPORT.md lacks a row per source | report integrity |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest experiments/live-trace/tests -q -k report
```

## Approach

**Vital slice**: combined comparison + recommendation
**Cut list**: No charts; no screen recording (prototype stop)

Reuse each experiment's collector/receiver as-is. Latency numbers in REPORT.md come from the combined run; per-experiment numbers stay in their RESULT.md.

---

## Edge Case Checklist

- [ ] A source missed events in the combined run: show n/N, never hide
- [ ] Planted secret: a fixture/session line holding `sk-ant-FAKE-SECRET-123` and a fake prompt sentence never appears in any stored file, RESULT.md or log of this task (grep check in tests).

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `experiments/live-trace/report/*` | combined run, e5.md |
| `experiments/live-trace/REPORT.md` | report |
| `experiments/live-trace/tests/test_report.py` | integrity check |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/`, `web/`, `pyproject.toml`, `uv.lock` | Product code; the spike may IMPORT from `src/easy_personal` read-only, never change it |
| `~/.claude*/` (any file), `~/.claude.json` | Live accounts; transcripts are read-only, settings never edited |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/`, `memory/` | Kit framework files, not spike code |
| Sibling repos (`../easy-*`) | Out of scope |

---

## Test Plan

Integrity test over the real RESULT.md / REPORT.md files (sections + rows) and a planted-secret grep over everything under `experiments/live-trace/`.

---

## Completion Checklist

- [ ] Implementation done; RESULT.md written with numbers
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Planted-secret grep clean
- [ ] Session count used reported to the Supervisor
- [ ] Supervisor notified: task ready for Stage 4 review