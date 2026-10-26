**Date**: 2026-10-26
**Complexity Level**: C2
**Risk Level**: High
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: *Workspace*, *Agent account*, *Connector id / alias*)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely, and `BRAINSTORMING_LOG_WORKSPACES.md` (verified facts W1–W10, edge cases)
4. Read `agents/backend.md`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. **C2/C3 or multi-file tasks only**: read `memory/codebase-map.md`

---

## Requirement (Pillar 1 — Adapt the requirement)

Ingest must read the `projects/` folder of every confirmed Claude agent account; every stored event carries its agent account; dashboard and CLI usage can filter by agent account (PRD FR-036). Cold ingest of all accounts (≈ 960 MB) < 2 min one time; re-run with no changes < 2 s (NFR-005).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Add `events.agent_account_id` (FK, backfilled to the default account), loop ingest over confirmed Claude accounts, and add an `account` filter to the usage functions, CLI (`--account`) and API (`?account=`).

**Out of scope** (what this task explicitly does NOT do):

* Codex usage (listed only)
* Workspace filter (HUN-64)
* Dashboard filter UI (HUN-70)

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-036
* NFR-005
* NFR-009
* NFR-003

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-26)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (requirement grill 2026-10-26)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-62 — Agent accounts

**Entry point**: `easy-personal ingest`, systemd timer, usage CLI/API

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Migration adds `events.agent_account_id INTEGER REFERENCES agent_accounts(id)` and backfills existing rows to the default account; index `(agent_account_id, ts_utc)` | FR-036, NFR-009 |
| 2 | Ingest reads `<folder>/projects` of every confirmed Claude account (detected/ignored/missing skipped); `EASY_PERSONAL_CLAUDE_PROJECTS` still overrides the default account's projects path | FR-036 |
| 3 | A message id present in two accounts counts once in usage totals | FR-036, NFR-004 |
| 4 | `usage`, breakdown, outliers, sessions accept an account filter in CLI (`--account <id|label>`) and API (`account=`); no filter = all accounts | FR-036 |
| 5 | Ingest commits per file and resumes from `source_files` offsets after a kill | NFR-005 |
| 6 | Throughput: synthetic ≥ 200 MB ingests at ≥ 8 MB/s (≙ 960 MB < 2 min); unchanged re-run < 2 s | NFR-005 |
| 7 | Source files are only read (NFR-003) | NFR-003 |
| 8 | Account filter combines with the HUN-64 workspace filter (AND) in metrics, CLI (`--account` + `--workspace`) and API (`account=` + `workspace=`); test with two accounts × two workspaces (moved here from HUN-64 AC6, which could not be tested before the account filter existed). Reuse `metrics._scope`'s AND-able clause. | FR-035, FR-036 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Two fixture accounts with distinct + one shared message id | per-account totals correct; shared message counted once | automated |
| 2 | Kill ingest mid-run then re-run | same final totals as one clean run | automated |
| 3 | Synthetic 200 MB (perf marker) | ≥ 8 MB/s | automated (`-m perf`) |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/sources tests/core tests/surfaces tests/leak -q && uv run pytest -m perf -q && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-63.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-63.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

N/A — no UI in this task (backend/QA only). All three UI Evidence rows are ☐ N/A.

---

## Approach

**Pattern reference**: `sources/claude_code.py` ingest loop; `core/store.file_offsets`

**Vital slice**: account tag + loop + filter
**Cut list**: per-account charts (only a filter); Codex transcripts

Shared design for the workspace feature (Supervisor, 2026-10-26; source: `BRAINSTORMING_LOG_WORKSPACES.md`, Option A):

* **Desired state + reconcile.** The store holds desired state (workspaces, connectors, account→workspace picks). Claude configs are changed only by one `plan(account)` / `apply(plan)` pair that goes through `connectors/writer.py` (`claude mcp add-json|remove --scope user`). Every user action = one DB transaction + plan/apply for affected accounts; the plan lines are the confirmation text.
* **Default account** = folder `~/.claude`; its Claude config is `~/.claude.json`, so `claude` runs with `CLAUDE_CONFIG_DIR` **unset**. Every other Claude account runs `claude` with `CLAUDE_CONFIG_DIR=<folder>` (its config is `<folder>/.claude.json`). Brainstorm fact HUN-62.
* **Connector id / alias.** id is fixed: keychain service `easy-personal/<id>`, launcher `easy-personal-launch exec <id>` (launcher unchanged: it already treats its argument as the keychain key). Alias = the Claude Code name; default `<workspace>-<provider>`; `alias_custom` = 1 keeps an owner-typed alias. Legacy connectors: id = alias = old name. New ids: `c` + 8 lowercase hex chars. All ids/aliases match `launch._NAME`.
* **Store rules (NFR-009).** Schema changes only as numbered migrations on `PRAGMA user_version`, one transaction each, tested from the previous schema with data; `foreign_keys=ON` + `busy_timeout` on every connection; business rules as constraints (UNIQUE / partial unique indexes / CHECK / triggers) as well as code. No secret value in any table.
* **Tests** never touch real accounts: temp HOME / `EASY_PERSONAL_AGENT_ROOT`, fake keychain + fake `claude` (`tests/fakes`, `tests/connectors/conftest.py`).

Task design (Supervisor, 2026-10-26):

* Ingest stays one `claude_code` source; it now iterates `(account_id, projects_dir)` pairs. `source_files` keys on absolute path, so no collision (brainstorm fact W6).
* Usage functions take an optional `account_ids: list[int] | None`; filter applied in the inner time-first query (HUN-61).
* `--account` accepts id or label (case-insensitive exact); unknown → error listing valid labels.

---

## Edge Case Checklist

- [ ] Account confirmed after its transcripts already exist → first ingest picks them up from offset 0
- [ ] Account goes missing → ingest skips it, keeps its rows
- [ ] Huge single JSONL file (> 100 MB) → streamed, bounded memory
- [ ] Same transcript copied between accounts → message counted once
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log; every new CLI command / API route is registered in `tests/leak/harness.py` (`CLI_CALLS` / `API_CALLS`).

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/sources/claude_code.py` | per-account loop, tag |
| `src/easy_personal/core/migrations/` | events column + index |
| `src/easy_personal/core/metrics.py` | account filter |
| `src/easy_personal/surfaces/cli.py`, `api.py` | `--account`, `account=` |
| `tests/fixtures/` | second account fixture |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| real `~/.claude*/`, `~/.claude.json`, `~/.codex*/` | Never read or written by tests or verify: fixtures + temp HOME only (learnings: verify against fixtures). Product code writes Claude config only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| `~/.claude*/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `src/easy_personal/launch.py` beyond what the task states | NFR-006: stdlib + `keyring` only, < 200 ms |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Multi-account fixture ingest; dedupe test; resume test; perf throughput test; CLI/API filter tests; migration test from HUN-62 schema with rows.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Security review: `Skill({ skill: "security-review" })` run (Medium/High risk)
- [ ] `Skill({ skill: "migration-safety" })` run (schema change) — Stage 3 and Stage 4
- [ ] Lint passes (`uv run ruff check .`)
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md`'s Evidence table (Hard-Stop Gate 5)
- [ ] `Skill({ skill: "verify" })` run — feature confirmed working in running app (fixtures only)
- [ ] `memory/MEMORY.md` updated (if new patterns or feedback learned)
- [ ] Supervisor notified: task ready for Stage 4 review