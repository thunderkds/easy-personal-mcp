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
6. Read `tests/e2e/test_workspaces_flow.py` and `tests/e2e/e2e_support.py` (HUN-71): three of the defects below are pinned there as `xfail(strict=False)` tests

---

## Requirement (Pillar 1 — Adapt the requirement)

Fix the defects found in the HUN-69 / HUN-71 reviews of the workspace feature, before the feature ships. Each defect is a bug: write (or un-xfail) a test that reproduces it, then make it pass.

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> easy-personal must never remove a Claude Code entry it did not create, must act on the default account with `CLAUDE_CONFIG_DIR` unset on every code path, must report a healthy post-adoption state as healthy, must show plans without a misleading "aborted" line, and must not print a traceback when a prompt hits end of input.

**Defects in scope**:

| \# | Sev | Defect | Found by |
| -- | -- | -- | -- |
| D1 | P2 | CLI `easy-personal connector remove <any name> --yes` falls back to the pre-workspace `writer.remove` and runs `claude mcp remove <name> --scope user` in the default account, deleting a hand-made server of that name. The API answers 404 for the same call. | HUN-71 `test_cli_remove_of_an_unknown_name_leaves_a_hand_made_server` |
| D2 | P2 | After legacy adoption and assignment, `easy-personal doctor` exits 1: `FAIL discord: keychain item present but no Claude Code entry`, `FAIL sendgrid: entry does not point at easy-personal-launch`. The catalog-id loop in `writer.doctor` still checks ids that are now stored connectors by their bare names. | HUN-71 `test_doctor_is_clean_after_the_flow` |
| D3 | P2 | Code paths call `claude` without removing an inherited `CLAUDE_CONFIG_DIR`, so with that variable set in the owner's shell they act on the wrong account: `writer.remove` (`_claude("mcp","remove",...)` with no env), `writer.doctor` (`_entry_exists(name)`), `surfaces/api.py::_configured`, `connectors/suggestions.py::_configured`. Every call that means the default account must pass `writer.account_env(None)`. | Supervisor, HUN-69 verify (fake `claude` log showed `cfg=~/.claude-wife` for `mcp get discord`) |
| D4 | P3 | API plan answers (`pending: true`) end with the writer's abort line, e.g. `aborted; nothing was changed.`, shown under the plan next to Confirm on the Workspaces and Agents pages. `api._run_writer` must drop the lines written after `confirm()` returned False when it answers `pending: true`. CLI output is unchanged. | Supervisor, HUN-69 screenshots |
| D5 | P3 | `easy-personal connector add <provider>` with stdin at EOF during the field/secret prompt exits with a Python traceback ending in `EOFError`. Must print one line (no traceback), exit non-zero, write nothing. | HUN-65 review; HUN-71 `test_connector_add_prompt_at_stdin_eof_is_a_clean_error` |

**Out of scope** (what this task explicitly does NOT do):

* HUN-63 P3 `tool_signals` double count for a transcript copied into two accounts (`test_copied_transcript_counts_tool_signals_once` stays xfail): it needs a dedupe-key decision for `tool_signals`, likely a migration — separate task.
* Any UI change (D4 is fixed in the API only; the pages already render `lines` as given).
* Changing what `sync` plans.

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-031
* FR-034
* DDR-0001

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, under the owner's standing instruction 2026-10-26 "run auto for all, if you have any question, let decide and note in the final report")
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-71 (e2e suite with the xfail tests) merged to main — work on a branch from main after HUN-69 and HUN-71 merge.

**Entry point**: `easy-personal connector remove <name>`, `easy-personal doctor`, `easy-personal connector add <provider>`; `POST/PUT/PATCH/DELETE` dashboard routes that go through `api._run_writer`; `GET /api/connectors` (`configured`); suggestions.

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | D1: `connector remove <name>` for a name that is not a stored connector id/alias runs `claude mcp remove` only if `<name>` is a catalog id AND the default account's entry `points_at_launcher(got, name)`; otherwise it changes nothing and exits non-zero with a one-line message. The HUN-71 xfail test passes with its `xfail` marker removed. | D1, DDR-0001 |
| 2 | D2: after the HUN-71 flow (adopt legacy, assign, sync), `easy-personal doctor` exits 0. Ids that have a `connectors` row are not checked by the bare-name catalog loop. A real fault (keychain item for a stored connector missing) still reports FAIL. The HUN-71 xfail test passes with its marker removed. | D2, FR-034 |
| 3 | D3: with `CLAUDE_CONFIG_DIR=/some/other` in the environment, every `claude` call that targets the default account (remove, doctor, both `_configured`) is made with `CLAUDE_CONFIG_DIR` unset — asserted from the fake `claude` log. | D3, DDR-0001 |
| 4 | D4: an API call that answers `pending: true` has no line written after the confirmation prompt (no `aborted; ...` line); the plan lines are unchanged otherwise. CLI output still prints the abort line on "no". | D4 |
| 5 | D5: `connector add slack` and `connector add gmail` with stdin at EOF print one line, no `Traceback`, exit non-zero, and write nothing to keychain or store. The HUN-71 xfail test passes with its marker removed. | D5 |
| 6 | Full suite green; `test_copied_transcript_counts_tool_signals_once` is the only remaining xfail. | all |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.
> Oracle: the HUN-71 xfail tests (written by QA, signed off by the Supervisor) for D1, D2, D5.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Default account holds hand-made `notes` (not our launcher) and our launcher entry `slack` with no `connectors` row | `connector remove notes --yes` → exit≠0, `notes` still present; `connector remove slack --yes` → removed (legacy path) | automated |
| 2 | HUN-71 flow end state | `doctor` exit 0 | automated (e2e) |
| 3 | `CLAUDE_CONFIG_DIR=/x` exported | fake `claude` log shows `<unset>` for every default-account call | automated |
| 4 | `PUT /api/connectors/<id>/alias` with `confirm: false` | `pending: true`, last line is a plan line, not `aborted; ...` | automated |
| 5 | `connector add slack < /dev/null` | one line, no traceback, exit≠0, keychain + store unchanged | automated |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/connectors tests/surfaces tests/e2e tests/leak -q -rx && uv run pytest -q && uv run ruff check .
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-72.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-72.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

N/A — no UI in this task (backend only). All three UI Evidence rows are ☐ N/A.

---

## Approach

**Pattern reference**: `connectors/legacy.py::detect` (catalog id + `points_at_launcher` with `writer.account_env(None)`); `writer.account_env`.

**Vital slice**: D1 and D3 (the two that can change the wrong Claude config), then D2, D4, D5.
**Cut list**: `tool_signals` dedupe (separate task); no refactor of the three JSON-body helpers in `api.py`.

Shared design for the workspace feature (Supervisor, 2026-10-26; source: `BRAINSTORMING_LOG_WORKSPACES.md`, Option A):

* **Default account** = folder `~/.claude`; its Claude config is `~/.claude.json`, so `claude` runs with `CLAUDE_CONFIG_DIR` **unset**. Every other Claude account runs `claude` with `CLAUDE_CONFIG_DIR=<folder>`.
* **Never remove an entry we did not create**: only entries recorded in `account_registrations`, or (pre-workspace only) a catalog-id entry that points at `easy-personal-launch exec <id>`.
* **Tests** never touch real accounts: temp HOME / `EASY_PERSONAL_AGENT_ROOT`, fake keychain + fake `claude` (`tests/fakes`, `tests/connectors/conftest.py`).

Task design (Supervisor, 2026-10-26):

* D4: in `_run_writer`, remember `len(lines)` when `confirm()` is called; if `asked`, answer with `lines[:mark]`.
* D5: catch `EOFError` (and `KeyboardInterrupt`) at the prompt in the CLI layer, like the HUN-65 confirm-prompt fix.

---

## Edge Case Checklist

- [ ] `connector remove <name>` where `<name>` is a stored alias but not an id → existing id/alias path, unchanged
- [ ] Legacy catalog id with keychain item but no Claude entry → keychain removed as before, no `claude` call
- [ ] `claude` not on PATH → existing messages unchanged
- [ ] `doctor` with a stored connector whose keychain item is missing → still FAIL
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/connectors/writer.py` | `remove`: ownership check + `account_env(None)`; `doctor`: skip stored ids, `account_env(None)` |
| `src/easy_personal/surfaces/cli.py` | D1 fallback rule; D5 EOF handling |
| `src/easy_personal/surfaces/api.py` | `_run_writer` (D4); `_configured` env (D3) |
| `src/easy_personal/connectors/suggestions.py` | `_configured` env (D3) |
| `tests/connectors/`, `tests/surfaces/`, `tests/e2e/test_workspaces_flow.py` | new tests; remove 3 `xfail` markers |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| real `~/.claude*/`, `~/.claude.json`, `~/.codex*/` | Never read or written by tests or verify: fixtures + temp HOME only. Product code writes Claude config only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| `~/.easy-personal/` | The owner's real store — tests use the autouse `_isolated_data_dir` fixture |
| `src/easy_personal/launch.py` | NFR-006 |
| `src/easy_personal/core/migrations/` | No schema change in this task |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Reproduce each defect with a failing test first (D1, D2, D5 already exist as xfail in `tests/e2e`; add focused unit tests for D1, D3, D4 in `tests/connectors` / `tests/surfaces`), then fix.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Security review: `Skill({ skill: "security-review" })` run (Medium/High risk)
- [ ] Lint passes (`uv run ruff check .`)
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md`'s Evidence table (Hard-Stop Gate 5)
- [ ] `Skill({ skill: "verify" })` run — feature confirmed working in running app (fixtures only)
- [ ] Supervisor notified: task ready for Stage 4 review