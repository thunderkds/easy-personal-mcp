# TASK_REVIEW — HUN-45: Connector tracer: Slack catalog entry + `connector add/remove` CLI → keychain → `claude mcp add-json` (snippet fallback) + `doctor`

> Sibling of `tasks/TASK_GUIDE_HUN-45.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☒ pass / ☐ fail | `tests/connectors/test_catalog.py` (AC1 schema: unknown/repeated/missing key, bad ENV name, bad flag, duplicate field, bad id; pin guard: unpinned, `@latest`, range refused, exact pin accepted; packaged Slack entry), `tests/connectors/test_writer.py` (AC2 summary with `***` + confirm + keychain + add-json argv via fake `claude`; AC3 `claude` missing → snippet, nothing written; AC4 refuse existing, show only Scope/Type/Command; AC5 remove; AC6 doctor; edges: rollback on `claude` failure, abort at confirm, empty field, no keychain backend), `tests/leak/test_connector_leak.py` (AC7 canary via `getpass` scanned in stdout/stderr/logs/fake `claude` argv/files; leak-test failure proven by a temporary `out(str(env))`). Supervisor re-run 2026-10-22T04:48:56Z: `62 passed in 1.47s`; leak `24 passed` |
| Verification command run | ☒ pass / ☐ fail | `uv run pytest tests/connectors -q` → `62 passed in 1.47s`; `uv run pytest tests/leak -q` → `24 passed, 1 warning`; `uv run ruff check src tests` → `All checks passed!` |
| Negative cases hold | ☒ pass / ☐ fail | live: second `add` refused (existing entry shown as Scope/Type/Command only + "keychain item present (values hidden)"); `claude` not on PATH → snippet only, nothing written; probes: `npx -y slack-mcp-server`, `@latest`, `@scope/x@^1.2.3`, `uvx tool@1.2` → `must be pinned`; tests: rollback when `claude` exits non-zero, abort at confirm, empty field exit 2 |
| verify | ☒ pass / ☐ fail / ☐ N/A | 2026-10-22T≈04:50Z, fresh `uv tool install --reinstall --no-cache` of the HUN-45 worktree (a cached build first served the old catalog — recorded as a learning); fake `claude` on PATH (records argv, simulates get/add-json/remove) and a throwaway file keychain via `PYTHON_KEYRING_BACKEND` — the owner's real `claude` config and keychain untouched. `connector add slack` (random `xoxb-VERIFY2-*` secret) → exit 0, summary `SLACK_MCP_XOXB_TOKEN=***`, `claude mcp add-json slack {"type": "stdio", "command": "<abs>/easy-personal-launch", "args": ["exec", "slack"]} --scope user`; secret count 0 in output and 0 in `claude` argv; keychain item command `['npx', '-y', 'slack-mcp-server@1.3.0', '--transport', 'stdio']`, env keys `['SLACK_MCP_XOXB_TOKEN']`; `doctor` → `ok keychain`, `ok claude`, `ok launcher`, `ok slack: entry points at the launcher`; launcher `headers` 74 ms; `connector remove slack --yes` → keychain `{}` — feature confirmed working — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☒ pass / ☐ fail | `main...HUN-45`: `catalog/slack.yaml`, `connectors/catalog.py`, `connectors/writer.py` (new), `surfaces/cli.py` (connector + doctor), tests + leak. lane_check `lane: C2, 9 files, 733 lines, EXCEEDS (lines 733 > 500)` — tests ~430 lines. code-review: 1 P1 found and fixed in `fd9e3e9` (catalog launched deprecated, unpinned `npx -y @modelcontextprotocol/server-slack` with the token in env → owner-approved pinned `slack-mcp-server@1.3.0 --transport stdio`, single `SLACK_MCP_XOXB_TOKEN` field; new guard refuses unpinned npx/uvx/pipx-run packages). P2: `writer.add` and `catalog.parse` above complexity 10 (score `functions_over_ccn_10_share` 0.16). P3: pin guard takes the first non-option arg as the package (`npx -p a@1.0.0 b` would pass; catalog is maintainer-authored); `remove` with `claude` missing keeps the keychain item. Startup reads complete (in full). security (inline, High): add-json JSON holds only the launcher path + args (`json.dumps`, argv list, no shell); secrets via `getpass`, shown as `***`; existing entries shown without Args/Environment; `~/.claude.json` never opened; keychain rollback on failure. |
| Full smoke suite still green (no regression) | ☒ pass / ☐ fail | `uv run pytest -q` → `292 passed, 2 skipped, 1 warning in 25.53s` (2026-10-22T04:49Z) |
| easy-verifier score (advisory) | ☒ recorded / ☐ N/A | 2026-10-22, MCP `score` repo=HUN-45 worktree, scope=changes, ref=`main...HUN-45`: overall **71** (architecture 100, blast-radius 100, requirement-fidelity 67, test-strategy 60, security 59, code-quality 38). code-quality = `functions_over_ccn_10_share` 0.16 (`catalog.py`, `writer.py`) + `max_function_ccn` 17 (`cli.py`) → P2 above; test-strategy = `tests_without_assertions_share` 0.16 (mostly `pytest.raises` tests); security = `redaction_hits_observed` 6 (known false-positive class). Raw JSON /home/hungnguyenhuu/.claude-personal/projects/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/75d1f65a-8da1-4f4c-8245-0270482d2af1/tool-results/mcp-easy-verifier-score-1792644756273.txt. Advisory only. |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☒ N/A | CLI/backend task: no UI component (Hard-Stop Gate 6 N/A) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☒ N/A | CLI/backend task: no UI component (Hard-Stop Gate 6 N/A) |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☒ N/A | CLI/backend task: no UI component (Hard-Stop Gate 6 N/A) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-22T04:36:55Z in worktree easy-personal-mcp-HUN-45 before any implementation commit:

```
$ uv run easy-personal connector add slack
easy-personal: error: argument command: invalid choice: 'connector' (choose from ping, mcp, ingest, schedule, usage, outliers, warnings, kit, plan, dashboard, exec, headers)
rc=2
$ uv run easy-personal doctor
easy-personal: error: argument command: invalid choice: 'doctor' (choose from ping, mcp, ingest, schedule, usage, outliers, warnings, kit, plan, dashboard, exec, headers)
rc=2
```

**AFTER**: `uv run pytest tests/connectors -q` -> 50 passed; `uv run pytest tests/leak -q` -> 24 passed; `uv run pytest -q` -> 280 passed, 2 skipped (fake `claude` on PATH and in-memory keychain; the real ones are never run).

**DELTA**: `easy-personal connector add|remove slack` and `easy-personal doctor` now exist: secrets go to the keychain and `claude mcp add-json --scope user` registers `easy-personal-launch exec slack`.

**WITNESS**: Supervisor — test re-runs 2026-10-22T04:40Z and 04:49Z, live connector verify 04:41Z and (pinned entry, fresh install) ≈04:50Z in the HUN-45 worktree (attributed to HUN-45 via the active_task state file; `memory/event-trace/HUN-45.jsonl`), independent of the implementing agent.
