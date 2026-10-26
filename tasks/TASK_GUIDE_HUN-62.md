**Date**: 2026-10-26
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P0
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

Agent accounts must be auto-detected from `~/.claude*` and `~/.codex*` folders (plus folders added by hand) and stay *detected* until the owner confirms and names them; the Agents view groups them by provider or by account (PRD FR-032, US-015).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Add the `agent_accounts` table, a detector, and CLI + guarded API to list, confirm (with label), ignore and add-by-path accounts; the default `~/.claude` account is confirmed automatically as "Claude default" so today's ingest keeps working.

**Out of scope** (what this task explicitly does NOT do):

* Ingesting other accounts (HUN-63)
* Picking workspaces (HUN-65)
* Writing anything into any account folder
* Dashboard page (HUN-68)

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-032
* US-015
* NFR-009
* NFR-008

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-26)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (requirement grill 2026-10-26)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-61 — Store foundation

**Entry point**: `easy-personal account list|confirm|ignore|add`; `GET /api/accounts`, `POST /api/accounts/{id}/confirm|ignore`, `POST /api/accounts`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Detector finds directories `~/.claude*` containing `projects/` or `settings.json` and `~/.codex*` containing `config.toml`; files like `~/.claude.json(.backup)` are never accounts | FR-032 |
| 2 | New folders appear as `detected`; `confirm --label` makes them `confirmed`; `ignore` hides them from re-detection | FR-032 |
| 3 | `~/.claude` is confirmed automatically as "Claude default" with `config_dir_env` NULL; other Claude accounts store their folder for `CLAUDE_CONFIG_DIR` | FR-032, FR-033 |
| 4 | A folder added by hand outside HOME is accepted only if it passes the same content check | FR-032 |
| 5 | A confirmed folder that disappears is reported `missing`, not deleted | FR-032 |
| 6 | List output gives provider, label, folder, status; grouping by provider or by account is available in CLI (`--by provider|account`) and API | FR-032 |
| 7 | Account writes via API need the dashboard token + Host/Origin + JSON content type | NFR-008 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Temp HOME with `.claude/`, `.claude-work/projects/`, `.claude.json`, `.codex/config.toml`, `.claude-junk/` (empty) | 3 accounts (default confirmed, work + codex detected); no `.claude.json`, no `.claude-junk` | automated |
| 2 | `account confirm <folder> --label 'Claude work'` then list | status confirmed, label shown | automated |
| 3 | Cross-origin POST | 403 | automated |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core tests/surfaces tests/leak -q && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-62.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-62.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

N/A — no UI in this task (backend/QA only). All three UI Evidence rows are ☐ N/A.

---

## Approach

**Pattern reference**: `connectors/writer.py` confirm flow; dashboard write guard (HUN-49/52)

**Vital slice**: detect + confirm + list via CLI
**Cut list**: account rename after confirm (re-confirm with new label covers it); account delete

Shared design for the workspace feature (Supervisor, 2026-10-26; source: `BRAINSTORMING_LOG_WORKSPACES.md`, Option A):

* **Desired state + reconcile.** The store holds desired state (workspaces, connectors, account→workspace picks). Claude configs are changed only by one `plan(account)` / `apply(plan)` pair that goes through `connectors/writer.py` (`claude mcp add-json|remove --scope user`). Every user action = one DB transaction + plan/apply for affected accounts; the plan lines are the confirmation text.
* **Default account** = folder `~/.claude`; its Claude config is `~/.claude.json`, so `claude` runs with `CLAUDE_CONFIG_DIR` **unset**. Every other Claude account runs `claude` with `CLAUDE_CONFIG_DIR=<folder>` (its config is `<folder>/.claude.json`). Brainstorm fact HUN-62.
* **Connector id / alias.** id is fixed: keychain service `easy-personal/<id>`, launcher `easy-personal-launch exec <id>` (launcher unchanged: it already treats its argument as the keychain key). Alias = the Claude Code name; default `<workspace>-<provider>`; `alias_custom` = 1 keeps an owner-typed alias. Legacy connectors: id = alias = old name. New ids: `c` + 8 lowercase hex chars. All ids/aliases match `launch._NAME`.
* **Store rules (NFR-009).** Schema changes only as numbered migrations on `PRAGMA user_version`, one transaction each, tested from the previous schema with data; `foreign_keys=ON` + `busy_timeout` on every connection; business rules as constraints (UNIQUE / partial unique indexes / CHECK / triggers) as well as code. No secret value in any table.
* **Tests** never touch real accounts: temp HOME / `EASY_PERSONAL_AGENT_ROOT`, fake keychain + fake `claude` (`tests/fakes`, `tests/connectors/conftest.py`).

Task design (Supervisor, 2026-10-26):

* Migration: `agent_accounts(id INTEGER PK, provider TEXT CHECK(provider IN ('claude','codex')), folder TEXT UNIQUE NOT NULL, label TEXT, status TEXT CHECK(status IN ('detected','confirmed','ignored')), config_dir_env TEXT)`; seed/upsert the default `~/.claude` row confirmed when the folder exists.
* Scan root = `EASY_PERSONAL_AGENT_ROOT` env or `Path.home()` (tests set a temp dir). Detection runs on `account list`, on dashboard load and before ingest; it only inserts new rows, never changes confirmed/ignored ones.
* `missing` is computed at read time (folder not a directory), not stored.
* New module `src/easy_personal/connectors/accounts.py` (or `core/accounts.py` if ingest needs it without importing connectors — core must not import connectors; put detection in core).

---

## Edge Case Checklist

- [ ] Symlinked account folder → resolve to real path; same real path twice → one account
- [ ] Label empty or > 60 chars → refused
- [ ] Folder name with spaces or unicode
- [ ] `~/.claude-*` folder that is a backup copy (has projects/) — detected, owner may ignore
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log; every new CLI command / API route is registered in `tests/leak/harness.py` (`CLI_CALLS` / `API_CALLS`).

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/accounts.py` (new) | detector + queries |
| `src/easy_personal/core/migrations/` | accounts migration |
| `src/easy_personal/surfaces/cli.py` | `account` commands |
| `src/easy_personal/surfaces/api.py` | account routes |
| `tests/leak/harness.py` | register new surfaces |

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

Detector tests on temp HOME; CLI tests; API guard tests; leak suite registration.

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