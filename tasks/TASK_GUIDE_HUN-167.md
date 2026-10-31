**Date**: 2026-10-31
**Complexity Level**: C1
**Risk Level**: Medium
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

> Measure Claude Code's built-in `http` hook type as a live source: hooks configured only in the scratch session folder POST each event to a tiny local collector; find latency, capture vs the transcript reference, overhead on the session, and what happens when the collector is down.

**Out of scope**:

* Command hooks or any hook in the real repo or account settings
* Changing `.claude/hooks/post_tool_trace.py`

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

**Depends on**: HUN-165 (harness)

**Entry point**: Standalone — N/A: spike code under `experiments/live-trace/`, run by hand; nothing in `src/` calls it

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `e2/collector.py`: stdlib HTTP server on 127.0.0.1:4390, `POST /hook`, keeps only hook_event_name, session_id, tool_use_id, tool_name, agent_id, local receive time; returns 2xx empty body | collector |
| 2 | `e2/settings.json` template: `http` hooks with `async: true` for SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, PostToolUseFailure, SubagentStart, SubagentStop, Stop, SessionEnd | hooks source |
| 3 | 5 runs async: capture % of reference tool_use ids, median/p95 latency (PreToolUse and PostToolUse), wall-clock overhead % vs baseline | pass targets |
| 4 | Collector down: 2 runs async and 2 runs sync (no `async`) with nothing listening — session completes? wall-clock change? | failure mode |
| 5 | `e2/RESULT.md` with verdict and the confirmed stdin field names | deliverable |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest experiments/live-trace/tests -q -k e2
```

## Approach

**Vital slice**: latency, capture %, overhead, collector-down behaviour
**Cut list**: No auth on the collector; no retries

Confirm the http hook JSON shape against [https://code.claude.com/docs/en/hooks.md](<https://code.claude.com/docs/en/hooks.md>) before writing settings. Drop `tool_input`/`tool_response` at the collector before anything is written.

---

## Edge Case Checklist

- [ ] Hook payload with large `tool_response` (>1 MB): collector still drops it and stays fast
- [ ] Subagent events carry `agent_id`
- [ ] Two sessions at once: events separated by session_id
- [ ] Planted secret: a fixture/session line holding `sk-ant-FAKE-SECRET-123` and a fake prompt sentence never appears in any stored file, RESULT.md or log of this task (grep check in tests).

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `experiments/live-trace/e2/*` | collector, settings template, RESULT.md |
| `experiments/live-trace/tests/test_e2.py` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/`, `web/`, `pyproject.toml`, `uv.lock` | Product code; the spike may IMPORT from `src/easy_personal` read-only, never change it |
| `~/.claude*/` (any file), `~/.claude.json` | Live accounts; transcripts are read-only, settings never edited |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/`, `memory/` | Kit framework files, not spike code |
| Sibling repos (`../easy-*`) | Out of scope |

---

## Test Plan

Start the collector on a free port in-test, POST a fixture payload holding the planted secret in `tool_input`/`tool_response`: stored line has only allowed keys; 2xx response.

---

## Completion Checklist

- [ ] Implementation done; RESULT.md written with numbers
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Planted-secret grep clean
- [ ] Session count used reported to the Supervisor
- [ ] Supervisor notified: task ready for Stage 4 review