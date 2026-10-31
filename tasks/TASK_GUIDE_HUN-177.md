**Date**: 2026-10-31
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P0
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `BRAINSTORMING_LOG_LIVE.md` and `PRD.md` US-022, FR-044, NFR-002, NFR-008

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-31: "build them as the next versio n" (the live-trace stack, as v0.3.0); owner chose Option A (transcript tail only) and approved this breakdown ("approve").

**Restated intent**:

> Add `GET /api/live` that streams the live tail's new events as Server-Sent Events (session id, project, account, time, kind, tool name, token counts, API-equivalent cost), plus a snapshot of active sessions (a record in the last 5 minutes), under the same Host/Origin/token guard as every `/api/*` route; because the browser `EventSource` cannot send the `x-dashboard-token` header, the token travels another way that keeps NFR-008 (e.g. a query parameter checked with `hmac.compare_digest`, never logged), and a reconnect with `Last-Event-ID` resumes without duplicates.

**Out of scope**:

* The Live view (next task)
* Any write route

**Requirement Refs**: US-022, FR-044, NFR-002, NFR-008

### Shared rules (v0.3.0 live view)

* Direction: `BRAINSTORMING_LOG_LIVE.md` Option A (owner-selected 2026-10-31) and `memory/decisions.md` "v0.3.0 live view = transcript tail only". Transcripts only: no hooks, no OpenTelemetry, no write to any Claude config (`settings.json`, `~/.claude.json`).
* Metadata only (FR-002, NFR-002): message text, tool input and tool output never reach the store, the SSE stream, the page or a log. The leak suite (`tests/leak/`) must cover every new route.
* 127.0.0.1 only (NFR-001); source files read-only (NFR-003); determinism (NFR-004); migrations only through the ordered migration path if a schema change is unavoidable (NFR-009, `migration-safety`).
* Spike evidence to reuse (do not import from `experiments/`): `experiments/live-trace/e1/tail.py` and `e1/RESULT.md` (poll 100 ms, \~2–3 ms per scan over \~410 files, p95 latency 1.35–1.93 s caused by Claude Code's own write delay), `REPORT.md`.
* Never use `git stash` (shared across worktrees); refresh `.claude/hooks/.state/active_task` right before every full test run.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-31)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: HUN-176 (live tail)

**Entry point**: `GET /api/live` in `src/easy_personal/surfaces/api.py`

---

### Event contract (fixed by the Supervisor 2026-10-31; HUN-177 serves it, HUN-178 consumes it)

URL: `GET /api/live?token=<dashboard token>` (the token also accepted in `x-dashboard-token`). `Content-Type: text/event-stream`. Keep-alive `: ping` comment every 15 s.

* `event: snapshot` (first message, and again after a buffer reset) — `data: {"active_window_s": 300, "last_id": <n>, "sessions": [{"session_id", "account", "project", "last_ts", "tool_calls", "tokens_in", "tokens_out", "cost_usd"}]}`
* `event: live` with `id: <n>` (monotonic) — `data: {"id": n, "session_id", "account", "project", "ts", "kind", "tool", "is_error", "is_sidechain", "tokens_in", "tokens_out", "cost_usd"}`
  * `kind` ∈ `tool_use`, `tool_result`, `usage`; `tool` is the tool name or `null`; `ts` is ISO-8601 UTC; `tokens_*` integers or `null`; `cost_usd` API-equivalent number or `null` (unknown model price).
  * `account` and `project` are the same labels the Usage view uses (account label, project folder name), never a full path.
* `last_id` is the id of the newest event already counted in the snapshot's totals. The server sends only `live` events with `id > last_id` after a snapshot, and ids never restart while the dashboard runs; the client drops any `live` event with `id <= last_id` (amended 2026-10-31 after HUN-178 review).
* Resume: a reconnect with `Last-Event-ID: n` gets only events with `id > n`; if `n` is older than the buffer, the server sends a fresh `snapshot` first.

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | A request without the token, or with a wrong Host/Origin, is refused (same status codes as other `/api/*` routes) | NFR-008 |
| 2 | With the token, the stream sends a snapshot event then one event per new buffer entry; fields are metadata only | FR-044, NFR-002 |
| 3 | Reconnect with `Last-Event-ID: N` sends only events after N | FR-044 |
| 4 | The route is in `tests/leak/harness.py` `API_CALLS` and the leak suite passes on it | NFR-002 |
| 5 | The token never appears in any log line | NFR-008 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/surfaces -q -k live && uv run pytest tests/leak -q
```

## Approach

**Vital slice**: guarded SSE stream with resume
**Cut list**: No WebSocket; no server-side filtering beyond account/workspace scope if trivial

Starlette `StreamingResponse` with `text/event-stream`, reading the tail's buffer by event id; keep-alive comment every 15 s; cost from `core/prices.py` `cost()`.

---

## Edge Case Checklist

- [ ] Client disconnects mid-stream: generator exits, no leak
- [ ] Buffer rolled past Last-Event-ID: send a reset snapshot
- [ ] Two tabs open
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, SSE event, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/surfaces/api.py` | route + guard token path |
| `tests/surfaces/test_api_live.py` | tests |
| `tests/leak/harness.py` | API_CALLS entry |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude*/` (any file), `~/.claude.json`, any `settings.json` | Read-only sources; no Claude config writes (Option A) |
| `experiments/live-trace/` | Spike evidence stays as is |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |
| Sibling repos (`../easy-*`) | Conventions only |

---

## Test Plan

Starlette TestClient streaming tests for guard refusals, snapshot + events, resume, metadata-only payload; leak suite.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review