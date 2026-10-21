# TASK_REVIEW — HUN-44: Launcher — keychain store + refusal path, exec/headers, env-only secrets

> Sibling of `tasks/TASK_GUIDE_HUN-44.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☒ pass / ☐ fail | `tests/connectors/test_launch.py` (AC1 `test_exec_refuses_without_backend`, `test_headers_refuses_without_backend`; AC2 `test_exec_puts_secret_in_env_not_argv`, `test_exec_uses_os_execvpe`; AC3 `test_headers_prints_json_only`; AC4 `test_imports_only_stdlib_and_keyring`; AC5 `test_start_overhead_under_200ms`), `tests/connectors/test_keychain.py` (AC1 `test_store_refuses_without_backend`). In-memory keyring backend only. Supervisor re-run 2026-10-21T14:55:58Z: `uv run pytest tests/connectors/test_launch.py tests/connectors/test_keychain.py -q` → `15 passed in 0.76s` |
| Verification command run | ☒ pass / ☐ fail | `15 passed in 0.76s` (Supervisor re-run, worktree HUN-44). Import cost measured by Supervisor (`python -X importtime`): `easy_personal.launch` ≈ 47–54 ms; `easy_personal.surfaces.cli` ≈ 299–315 ms → the registered command must be `easy-personal-launch`, not `easy-personal exec` (raised against HUN-45's guide) |
| Negative cases hold | ☒ pass / ☐ fail | no backend, unknown connector, provider command missing on PATH, invalid connector name, bad usage (exit 2); refusal = one fixed stderr line, no secret, nothing written to disk. Not tested: locked Secret Service on a real headless session (maps to `KeyringError` → same refusal) |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☒ pass / ☐ fail | 9 files in `main...HUN-44` + `cli.main` dispatch. lane_check: `C2, 9 files, 344 lines, FITS`. code-review 0 P0/P1 code, 1 P1 process (partial read of `agents/backend.md`, closed by conformance check), P1 contract raised on HUN-45 (`easy-personal exec` breaks NFR-006). security (inline) pass. blast-radius: T1 connector tokens only in keychain/env/stdout(headers); follow-up candidates: `headers` refuses on a TTY, catch-all in `launch.main` (no traceback). |
| Full smoke suite still green (no regression) | ☒ pass / ☐ fail | `uv run pytest -q` → `157 passed, 2 skipped in 17.05s` (2026-10-21T14:56Z) |
| easy-verifier score (advisory) | ☒ recorded / ☐ N/A | 2026-10-21, MCP `score` repo=HUN-44 worktree, scope=changes, ref=`main...HUN-44`: overall **65** (architecture 100, test-strategy 75, requirement-fidelity 67, blast-radius 65, code-quality 53, security 30); abstained: solution-fit, reliability, supply-chain, api, data, operations, infrastructure. Security 30 = `redaction_hits_observed` 126 (known false positive: lockfiles + test secrets in the fake backend); code-quality: `functions_over_ccn_10_share` 0.13 (`launch.load`), `format_config_missing`. Raw JSON ~/.claude-personal/projects/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/dbd9c4da-edf5-49bc-80a5-cb2b8c04f640/tool-results/mcp-easy-verifier-score-1792594497490.txt. Advisory only. |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured before any implementation commit):

```
2026-10-21T12:32:11Z
$ uv run pytest tests/connectors/test_launch.py tests/connectors/test_keychain.py -q
ERROR: file or directory not found: tests/connectors/test_launch.py


no tests ran in 0.00s
$ uv run easy-personal exec slack
usage: easy-personal [-h] [--version]
                     {ping,mcp,ingest,schedule,usage,outliers,warnings,plan} ...
easy-personal: error: argument command: invalid choice: 'exec' (choose from ping, mcp, ingest, schedule, usage, outliers, warnings, plan)
```

**AFTER**: `uv run pytest tests/connectors/test_launch.py tests/connectors/test_keychain.py -q` -> `15 passed`; full suite `157 passed, 2 skipped`; `uv run easy-personal exec slack` -> `easy-personal: connector 'slack' is not configured` (exit 1). Launcher time to exec/print (5 runs, fake backend): exec 67-81 ms, headers 65-87 ms.

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
