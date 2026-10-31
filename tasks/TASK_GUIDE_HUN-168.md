**Date**: 2026-10-31
**Complexity Level**: C2
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

> Measure Claude Code's OpenTelemetry export as a live source: a stdlib OTLP/HTTP JSON receiver on 127.0.0.1:4318 takes logs (events) and metrics from headless runs; find event latency at default and lowered export intervals, token and cost accuracy, and which attributes each event carries.

**Out of scope**:

* Any OTel SDK or collector binary dependency
* Enabling any `OTEL_LOG_*` content flag

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
| 1 | `e3/receiver.py`: `POST /v1/logs` and `/v1/metrics` (protocol `http/json`), keeps per event: event name, [session.id](<http://session.id>), [prompt.id](<http://prompt.id>), tool_use_id, tool name, token counts, cost_usd, model, timestamps, attribute NAMES; per metric: name, attributes names, values | receiver |
| 2 | Runs with `CLAUDE_CODE_ENABLE_TELEMETRY=1`, `OTEL_LOGS_EXPORTER=otlp`, `OTEL_METRICS_EXPORTER=otlp`, `OTEL_EXPORTER_OTLP_PROTOCOL=http/json`, endpoint 127.0.0.1:4318: 3 runs at default intervals, 5 runs with `OTEL_LOGS_EXPORT_INTERVAL=1000` and `OTEL_METRIC_EXPORT_INTERVAL=5000`; latency median/p95 of `tool_result` events vs reference | freshness |
| 3 | Tokens: OTel token sums vs transcript `message.usage` sums per session (% diff); cost: sum of `api_request` cost_usd vs `claude_code.cost.usage` metric | accuracy, 1% target |
| 4 | Attribute table per event name seen; confirm `tool_use_id` present on tool events (dedupe key) | completeness |
| 5 | Receiver down: 1 run — session completes? wall-clock change? | failure mode |
| 6 | Optional: 1 run with traces beta (`CLAUDE_CODE_ENHANCED_TELEMETRY_BETA=1`, `OTEL_TRACES_EXPORTER=otlp`) — span names seen | traces |
| 7 | `e3/RESULT.md` with verdict | deliverable |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest experiments/live-trace/tests -q -k e3
```

## Approach

**Vital slice**: latency at lowered interval, token/cost accuracy, attribute inventory
**Cut list**: No protobuf/gRPC; Prometheus exporter not tested

Re-check env var names at [https://code.claude.com/docs/en/monitoring-usage.md](<https://code.claude.com/docs/en/monitoring-usage.md>) first. Decode OTLP JSON (resourceLogs/scopeLogs/logRecords; resourceMetrics) with stdlib json. Drop any attribute whose name is not on an allow-list before writing.

---

## Edge Case Checklist

- [ ] Delta temporality: metrics summed per export
- [ ] Events from the subagent carry the same [session.id](<http://session.id>)?
- [ ] An attribute value with prompt-like text never stored (allow-list)
- [ ] Planted secret: a fixture/session line holding `sk-ant-FAKE-SECRET-123` and a fake prompt sentence never appears in any stored file, RESULT.md or log of this task (grep check in tests).

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `experiments/live-trace/e3/*` | receiver, run script, RESULT.md |
| `experiments/live-trace/tests/test_e3.py` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/`, `web/`, `pyproject.toml`, `uv.lock` | Product code; the spike may IMPORT from `src/easy_personal` read-only, never change it |
| `~/.claude*/` (any file), `~/.claude.json` | Live accounts; transcripts are read-only, settings never edited |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/`, `memory/` | Kit framework files, not spike code |
| Sibling repos (`../easy-*`) | Out of scope |

---

## Test Plan

POST a fixture OTLP JSON payload (logs + metrics) with the planted secret in a `prompt` attribute and a body: stored rows hold only allow-listed keys; token/cost parsing correct.

---

## Completion Checklist

- [ ] Implementation done; RESULT.md written with numbers
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Planted-secret grep clean
- [ ] Session count used reported to the Supervisor
- [ ] Supervisor notified: task ready for Stage 4 review