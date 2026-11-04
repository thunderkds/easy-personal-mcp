**Date**: 2026-11-04
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (Critical Constraints: deterministic, no LLM in the server)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-008, FR-075, NFR-004; `src/easy_personal/surfaces/mcp.py`; the MCP surface tests; the installed `mcp` SDK's `FastMCP.prompt` decorator

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-04: "any way to add a session that LLM can investigate the data and suggest the good solution? through the claude subcrible". Owner choice (AskUserQuestion): **In Claude Code, via MCP** — the server offers a ready-made prompt; the owner's own Claude Code session (subscription) does the reasoning; the server runs no model (NFR-004).

**Restated intent**:

> Register an MCP prompt `investigate` on the easy-personal FastMCP app, with optional arguments `workspace` and `account`. It returns one fixed user message that tells the model to: call `warnings` and `improvement_plan` (with the same scope), then `usage_breakdown` (by project and by model, last 7 days) and `session_outliers`; explain in plain words what caused each warning; suggest at most 5 fixes ranked by expected saving, each citing a number from a tool result; and say plainly when a change looks intended rather than wasteful. In Claude Code it appears as `/mcp__easy-personal__investigate`. No model call, no network, no new tool.

**Out of scope**:

* Any dashboard button or server-side `claude -p` run (owner chose MCP)
* The dashboard hint text (part of FR-074, another task)
* New tools or changes to existing tool results

**Requirement Refs**: FR-075

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner choice recorded above)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None (works with today's tools; richer once FR-073 causes land)

**Entry point**: MCP `prompts/list` / `prompts/get` name `investigate` in `src/easy_personal/surfaces/mcp.py`; Claude Code `/mcp__easy-personal__investigate`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `prompts/list` over MCP stdio lists `investigate` with optional `workspace` and `account` arguments and a one-line description | FR-075 |
| 2 | `prompts/get investigate` returns one user message naming the four tools, the ≤ 5 ranked fixes rule, the cite-a-number rule and the "intended vs wasteful" rule | FR-075 |
| 3 | With `workspace=X` / `account=Y` the message tells the model to pass the same values to every tool call; without them it names no scope | FR-075 |
| 4 | Every tool name in the message exists in `tools/list` (test guards against renames) | FR-075 |
| 5 | The server makes no model or network call for the prompt (no new dependency; test or code review) | NFR-004 |
| 6 | Real run: paste the stdio `prompts/list` and `prompts/get` responses from an isolated `uv tool install` (verify recipe) | FR-075 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests -q -k mcp && uv run pytest -q
```

## Approach

**Vital slice**: one `@app.prompt("investigate")` function with a fixed text template in `surfaces/mcp.py`; tests in the MCP surface tests.
**Cut list**: no prompt per rule; no localisation; no extra arguments (period etc.).

---

## Edge Case Checklist

- [ ] Workspace or account names with quotes or newlines → inserted as plain values, never breaking the message
- [ ] Unknown workspace → the prompt still returns; the tools report the unknown scope
- [ ] Planted secret/text fixtures never appear in any MCP response (leak suite green)

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/surfaces/mcp.py` | the prompt |
| MCP surface tests (e.g. `tests/surfaces/test_mcp*.py`) | AC 1–5 |
| `README.md` | one line under the MCP tools list: how to run the prompt |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/` | no core change |
| `web/` | other task |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

In-process FastMCP client (as existing MCP tests do): list prompts, get with and without scope, cross-check tool names; stdio real run from an isolated install.

---

## UI / Design Evidence

☐ N/A — no UI.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review