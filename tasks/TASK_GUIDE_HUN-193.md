**Date**: 2026-11-03
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P0
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: Kit server, Good result, Kit finding, Habit, Owner project, Abstained; Critical Constraints: read-only, metadata only, no LLM text)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `BRAINSTORMING_LOG_KITPERF.md` (Option A, Edge Case Checklist) and `PRD.md` US-026, FR-008, FR-014, FR-057, FR-059..FR-063, NFR-002, NFR-014, Open Question 22

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-02/03: the main job of easy-personal-mcp is Easy Kit performance; Claude reads findings by itself. Grill + brainstorm locked: unit = session; like with like, pooled matched sessions, ≥ 10 per side, "linked to"; good result = commit on branch + low tool-error rate; `kit_findings` MCP tool, ≤ 3 findings, one fixed habit each; first scope = 4 kit MCP servers; Option A compute on read.

**Restated intent**:

> New `src/easy_personal/core/kitperf.py` builds one row per session from `events` (owner project via `core/owners.owner_map`; main model = the model with the most output tokens; total tokens; API-equivalent cost via `core/prices`; length = first to last event; tool calls = names in `tool_names`; failed tool results = `SUM(tool_error_count)`, NULL-aware; kit servers used = tool names `mcp__<server>__*` for `easy-ui`, `easy-verifier`, `easy-context`, `easy-memory`; sub-agent records count toward the parent session id). It adds a *good result* per session (FR-061) from git and the `kit_error_rate` threshold. For each server and each measure (tokens, cost, length, good-result share) it compares sessions that used the server with sessions that did not, inside strata of (owner project, main model, size band), pools the strata, and returns findings. `surfaces/mcp.py` adds `kit_findings(limit=3, since=None)`; `surfaces/cli.py` adds `easy-personal kit-findings [--json]`. Both call the same core function (FR-008).

**Comparison rules (fixed here; record them in** `PROJECT_SPEC.md` **under Kit performance)**:

* Sessions compared: `abstained = 0`, at least one `assistant` event and **at least one tool call**, inside `since` (default: whole store).
* Size band = tertile of **tool-call count** over the compared sessions (3 bands). Never a compared measure. (Revised 2026-11-03 after the first real run.)
* Measures (revised 2026-11-03): `tokens_per_call` (total tokens ÷ tool calls), `cost_per_call` (API-equivalent cost ÷ tool calls), `good_result` (share). Raw tokens, raw cost and session length are no longer compared.
* A stratum counts only if it has ≥ 1 session on each side. Pooled sizes = sums over counted strata. A finding needs ≥ 10 pooled sessions on **each** side (FR-062); otherwise the server is listed under `not_enough` with both counts.
* Pooled delta for a measure = Σ over strata (kit sessions in stratum × (kit mean − non-kit mean)) ÷ Σ (kit sessions × non-kit mean), as a percentage. Good-result share uses only sessions whose result is known, with its own ≥ 10 check.
* Ranking: by absolute pooled delta; ties by server name then measure. `findings` = only comparisons that carry a habit (server side better), top `limit` (default 3, max 10); `other` = every remaining comparison that passed the ≥ 10 rule, same ranking, `habit` always `null`, not cut by `limit`.
* Wording: `text` = "Sessions that used <server> are linked to <N>% <fewer|more> <measure> than similar sessions without it (<a> vs <b> sessions)." Never "caused". `habit` = the server's fixed habit text from a table in `kitperf.py` when the server side is better on that measure, else `null`.
* Good result (FR-061): branch not in (`main`, `master`, `HEAD`, NULL) and the project is a git repo; ≥ 1 commit on that branch with committer time in \[first event, last event + 30 min\] → commit known; branch ref gone → look for a merge commit whose subject uses an FR-057 form for that branch and use `<merge>^1..<merge>^2`; nothing found → unknown. Error rate = failed tool results ÷ tool calls; tool calls 0 → rate 0; any NULL `tool_error_count` in a session that has tool calls → unknown. Good = commit known and rate ≤ `kit_error_rate`. Unknown error rate = no user record of the session carries a count (`SUM(tool_error_count)` NULL) — revised 2026-11-03.
* `kit_error_rate`: a new key under `thresholds:` in `~/.easy-personal/config.yaml`, read like `rules.live_thresholds` (unknown keys still refused). Default: compute the median session error rate on the owner's store after HUN-192's re-read, round to 2 decimals, put that number in code and in `PROJECT_SPEC.md`.
* Git work has a total budget of 2 s per call (`git rev-list` only, local refs, never fetch); a branch not checked in budget → unknown. Cache per (repo, branch) inside one call.

**Response contract** (`kit_findings`, CLI `--json`):

```json
{"as_of": "<UTC>", "sessions": 625, "findings": [{"server": "easy-memory", "measure": "tokens_per_call", "delta_pct": -21.4, "with": 14, "without": 61, "text": "…", "habit": "…"}],
 "other": [{"server": "easy-ui", "measure": "tokens_per_call", "delta_pct": 41.0, "with": 27, "without": 62, "text": "…", "habit": null}],
 "not_enough": [{"server": "easy-ui", "with": 3, "without": 40}], "unknown_results": 312, "note": "linked to, not caused"}
```

**Out of scope**:

* Dashboard page or `/api` route for findings
* Kernel skills/hooks/stages; easy-personal's own MCP tools as a server
* Editing `skills/wake` (kit-managed; XR-2 on the kernel repo)
* Storing findings; any LLM-written text; any statistics library

**Requirement Refs**: US-026, FR-060, FR-061, FR-062, FR-063, NFR-002, NFR-014

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved grill + brainstorm, 2026-11-03)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: HUN-192

**Entry point**: MCP tool `kit_findings` (`surfaces/mcp.py`) and CLI `easy-personal kit-findings` → `core/kitperf.kit_findings`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | A session with any `mcp__easy-memory__*` call (also only in a sub-agent record) counts as used for easy-memory; `mcp__claude_ai_*` and other servers never count | FR-060 |
| 2 | Good result: commit on the branch inside the window → known; commit only before the first event or after last + 30 min → no commit; branch on `main` / NULL → unknown; branch ref deleted but merged with `Merge <b>: <text>` → commits found via the merge | FR-061 |
| 3 | Error rate: 1 failed of 10 calls with `kit_error_rate` 0.2 → good (with commit); 3 of 10 → not good; a NULL `tool_error_count` with tool calls → unknown | FR-061 |
| 4 | `thresholds: {kit_error_rate: 0.3}` in config changes the result; a non-number is refused with the existing error style; missing file → default | FR-061 |
| 5 | Strata and pooling: a hand-computed fixture (3 strata, known means) gives the exact `delta_pct`; a stratum with only one side is excluded from both pooled counts | FR-062 |
| 6 | 9 pooled sessions on one side → no finding, server in `not_enough` with both counts; 10 on each side → finding | FR-062 |
| 7 | `text` contains "linked to" and never "caused"/"because"; `habit` is the table text only when the server side is better, else `null` | FR-062 |
| 8 | MCP `kit_findings` and CLI `--json` return the same dict for the same store (shared core, FR-008); `limit` above 10 is capped | FR-063 |
| 9 | Git budget: with a fake git that sleeps, the call returns within budget + 1 s and the slow branches count as unknown | NFR-014 |
| 10 | No prompt text, tool input, file content or error text in the response; the leak suite covers the new tool | NFR-002 |
| 11 | Real run on the owner's store after HUN-192's re-read: paste `easy-personal kit-findings --json`, the median-of-3 time (< 3 s, NFR-014), and the chosen `kit_error_rate` default with the median it came from | NFR-014, US-026 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core -q -k kitperf && uv run pytest tests/surfaces -q -k "kit_findings or mcp or cli" && uv run pytest tests/leak -q && uv run pytest -q
```

## Approach

**Vital slice**: one SQL pass → session rows → git result with budget → strata + pooled deltas → ranked findings → MCP + CLI.
**Cut list**: no page; no `/api` route; no per-account or per-workspace filter; no confidence intervals or p-values; no kernel skill comparison; no stored results.

Reuse `core/owners.py` (owner project + FR-057 merge subject forms; do not copy the regexes — import or extract a shared helper inside `owners.py`), `core/prices.py` for cost, `core/rules.py` threshold reading. Plain functions returning dicts.

---

## Edge Case Checklist

- [ ] Session spans 2 branches → use the branch of most events; ties → result unknown
- [ ] Gone project folder → owner project; not a repo → result unknown
- [ ] Two sessions on one branch at once → both get the commit (accepted; documented)
- [ ] Session with no model (all abstained assistant rows) → excluded
- [ ] Non-kit mean 0 for a measure → that stratum skipped for that measure
- [ ] Store empty → `findings: []`, `sessions: 0`, no error
- [ ] Rows from before HUN-192's re-read (NULL errors) → unknown, never 0
- [ ] Git missing on PATH → every result unknown, no exception
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/kitperf.py` (new) | session rows, good result, pooling, findings, habit table |
| `src/easy_personal/core/rules.py` | `kit_error_rate` threshold reader |
| `src/easy_personal/core/owners.py` | expose the merge-subject branch helper (no behaviour change) |
| `src/easy_personal/surfaces/mcp.py`, `src/easy_personal/surfaces/cli.py` | `kit_findings` tool, `kit-findings` command |
| `tests/core/test_kitperf.py` (new), `tests/surfaces/…`, `tests/leak/harness.py` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/migrations/`, `core/store.py`, `sources/` | HUN-192 owns the schema |
| `web/`, `src/easy_personal/surfaces/api.py` | No page in this release |
| `skills/wake/`, `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files (XR-2) |

---

## Test Plan

Fixture store with hand-built sessions across 3 strata and both sides; temp git repos with branches, a deleted merged branch and commits at chosen times (`GIT_COMMITTER_DATE`); fake slow git for the budget; config file variants; MCP/CLI parity; leak suite; real run on the owner's store with timings.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review