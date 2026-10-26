# TASK_REVIEW — HUN-65: Connector id + alias, workspace picks, per-account sync

> Sibling of `tasks/TASK_GUIDE_HUN-65.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☒ pass / ☐ fail | Implementer-written (Supervisor to sign off the oracle): `tests/core/test_connectors_schema.py` (AC1, 21 tests, raw SQL + v4→v6 upgrade keeps every row), `tests/core/test_account_picks.py` (AC7, 7), `tests/connectors/test_workspace_connectors.py` (AC2/AC8, 18), `tests/connectors/test_sync.py` (AC3/4/5/8 + Success Criteria 1–3, 14: `test_plan_for_two_accounts`, `test_apply_writes_each_account_with_its_own_config_dir`, `test_failure_mid_plan_keeps_what_succeeded_and_rerun_finishes`, `test_unpick_removes_only_from_that_account`, `test_ignored_account_gets_its_registered_aliases_removed`), `tests/connectors/test_sync_cli.py` (AC6, remove <id> incl. `test_remove_works_when_the_holder_account_is_ignored`, doctor, 12), `tests/surfaces/test_api_sync.py` (AC6 API + guard, 6), `tests/surfaces/test_api_connectors.py` (updated: workspace required), `tests/leak/test_connector_leak.py` + `tests/leak/harness.py` (AC8). AC9: no MCP tool added — `tests/leak` meta-test lists tools unchanged. 2026-10-26 09:54 UTC: `422 passed, 2 deselected` / `637 passed, 2 deselected`. |
| Verification command run | ☒ pass / ☐ fail | Implementer run, `Mon Oct 26 09:54:15 AM UTC 2026` (after the Supervisor's ignored-account fix; web built via `npm --prefix web ci && npm --prefix web run build`): `uv run pytest tests/connectors tests/core tests/surfaces tests/leak -q` → `422 passed, 2 deselected, 1 warning in 34.14s`; `uv run pytest -q` → `637 passed, 2 deselected, 1 warning in 53.17s`; `uv run ruff check .` → `All checks passed!`; `npm --prefix web test` → `Test Files 6 passed (6) / Tests 50 passed (50)` (ConnectorsView Vitest still green). Reviewer to re-run. |
| Negative cases hold | ☒ pass / ☐ fail | Implementer tests: hand-made entry with our alias → `refused`, never overwritten; unregistered names never removed; folder missing → `skipped`; ignored account with 2 registrations → 2 `remove` steps with its own `CLAUDE_CONFIG_DIR`, registrations cleared, `connector remove` succeeds (Supervisor decision 2026-10-26); `claude` missing → snippets, registrations unchanged; 2nd add fails → 1st registered only, re-run adds the rest; orphan / Codex / unconfirmed picks refused (code + trigger); bad alias / unknown workspace refused without echo; one provider per workspace + alias uniqueness (index + code); `connector add` without `--workspace` → exit 2; API guard 403 on all new routes. |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| easy-verifier score (advisory) | ☐ recorded / ☐ N/A | [overall result, abstaining dimensions, report path — or `N/A — <reason>` if the server is down; never blocks Done] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-26 by backend-developer on branch `HUN-65` at `5b43554` (no implementation commit yet). Isolated: temp `EASY_PERSONAL_HOME` + `EASY_PERSONAL_AGENT_ROOT` (`.claude/`, `.claude-work/`), fake `claude` first on PATH logging argv + `CLAUDE_CONFIG_DIR`, file keyring via `PYTHON_KEYRING_BACKEND=fakekr.Keyring` (scratchpad).

```
$ date -u
Mon Oct 26 09:31:21 AM UTC 2026
$ which claude
/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/0a221257-03c2-404a-adc3-6706867aa89a/scratchpad/hun65/bin/claude
$ uv run easy-personal connector add slack --workspace company
Using CPython 3.13.2
Creating virtual environment at: .venv
   Building easy-personal-mcp @ file:///home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-65
      Built easy-personal-mcp @ file:///home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-65
Installed 43 packages in 5ms
usage: easy-personal [-h] [--version]
                     {ping,mcp,ingest,schedule,usage,outliers,workspace,warnings,kit,plan,dashboard,exec,headers,connector,account,doctor} ...
easy-personal: error: unrecognized arguments: --workspace company
exit=2
$ uv run easy-personal account use 1 company
usage: easy-personal account [-h] {list,confirm,ignore,add} ...
easy-personal account: error: argument action: invalid choice: 'use' (choose from list, confirm, ignore, add)
exit=2
$ cat claude.log
(no claude call)
```

**AFTER**: captured 2026-10-26 by backend-developer on branch `HUN-65` (head `4e0e74d`), fresh isolated env as BEFORE (new temp `EASY_PERSONAL_HOME` + `EASY_PERSONAL_AGENT_ROOT` with `.claude/` and `.claude-work/projects/`; fake `claude` first on PATH, logging argv + `CLAUDE_CONFIG_DIR` and keeping a per-`CLAUDE_CONFIG_DIR` registry; file keyring via `PYTHON_KEYRING_BACKEND`; runner's own `CLAUDE_CONFIG_DIR` unset). Scratch path replaced by `<tmp>`.

```
$ date -u
Mon Oct 26 09:51:49 AM UTC 2026
$ uv run easy-personal workspace add company
exit=0
$ uv run easy-personal account confirm 2 --label "Claude work"
  "label": "Claude work",
  "status": "confirmed",
$ printf "xoxb-demo-token
" | uv run easy-personal connector add slack --workspace company --yes
  passwd = fallback_getpass(prompt, stream)
Warning: Password input may be echoed.
Bot token (xoxb-...): 
will change (Slack):
  keychain: service easy-personal/c527ebb45, fields SLACK_MCP_XOXB_TOKEN=***
  store: connector c527ebb45, alias company-slack, workspace company
  Claude Code: no change now; `easy-personal connector sync` registers it
connector c527ebb45 (company-slack) added to workspace 'company'.
run `easy-personal connector sync` to register it in the accounts that picked this workspace.
exit=0
$ uv run easy-personal account use 1 company
run `easy-personal connector sync` to apply it to the account's Claude config.
  "label": "Claude default",
  "workspaces": [
$ uv run easy-personal account use 2 company
  "label": "Claude work",
  "workspaces": [
$ uv run easy-personal connector sync --yes
Claude default: add company-slack -> exec c527ebb45
Claude work: add company-slack -> exec c527ebb45
done: Claude default: add company-slack -> exec c527ebb45
done: Claude work: add company-slack -> exec c527ebb45
restart Claude Code in the changed accounts to use the change.
exit=0
$ uv run easy-personal connector sync --yes   # again: idempotent
nothing to change.
exit=0
$ uv run easy-personal doctor
ok keychain: Keyring
ok claude: on PATH
ok launcher: /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-65/.venv/bin/easy-personal-launch
ok Claude default company-slack: points at the launcher (exec c527ebb45)
ok Claude work company-slack: points at the launcher (exec c527ebb45)
exit=0
$ cat claude.log  (argv + CLAUDE_CONFIG_DIR per call; writes only)
{"argv": ["mcp", "add-json", "company-slack", "{\"type\": \"stdio\", \"command\": \"/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-65/.venv/bin/easy-personal-launch\", \"args\": [\"exec\", \"c527ebb45\"]}", "--scope", "user"], "CLAUDE_CONFIG_DIR": "<unset>"}
{"argv": ["mcp", "add-json", "company-slack", "{\"type\": \"stdio\", \"command\": \"/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-65/.venv/bin/easy-personal-launch\", \"args\": [\"exec\", \"c527ebb45\"]}", "--scope", "user"], "CLAUDE_CONFIG_DIR": "<tmp>/agent-root/.claude-work"}
$ grep -c xoxb-demo-token store.sqlite* claude.log claude-state.json
<tmp>/claude-state.json:0
<tmp>/claude.log:0
$ grep -c xoxb-demo-token <home>/store.sqlite*; grep -c easy-personal/c keyring.json
<tmp>/home/store.sqlite:
easy-personal/c527ebb45
$ sqlite3 store: account_registrations
[(1, 'company-slack', 'c527ebb45'), (2, 'company-slack', 'c527ebb45')]
[('c527ebb45', 'company-slack', 0, 'slack', 2)]
$ python3: count of b"xoxb-demo-token" in each store file / in the keychain file
<tmp>/home/store.sqlite 0
<tmp>/claude.log 0
<tmp>/claude-state.json 0
<tmp>/keyring.json 1
```

**DELTA**: The owner can now put a connector in a workspace (fixed id `c527ebb45`, alias `company-slack`), pick workspaces per Claude account, and write the alias into each account's own Claude config only after a shown plan — the default account with `CLAUDE_CONFIG_DIR` unset, `Claude work` with it set to its folder — while the secret stays only in the keychain.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]

---

## Implementer self-checks (no Skill tool in the sub-agent; done by hand — Supervisor runs the real skills at Stage 4)

**Design (C3, brainstorming done inline):** Option A from `BRAINSTORMING_LOG_WORKSPACES.md` as the guide prescribes. One choice made here: `plan` adds two bookkeeping actions besides add/remove — `record` (entry already present AND pointing at `exec <same id>`, e.g. a crash after `claude mcp add-json` but before the registration row) and `forget` (a registered alias already gone from Claude Code). Without them a crash in that window would turn our own entry into a "hand-made" refusal. Both write only `account_registrations`, never Claude config.

**migration-safety (m0006_connectors.py):**
- Additive only: 3 new tables (`connectors`, `account_workspaces`, `account_registrations`, all `WITHOUT ROWID`), 2 partial unique indexes, 1 index, 2 triggers. No `ALTER`/`DROP` on an existing table; no existing row is read or written except `SELECT id FROM workspaces WHERE is_orphan = 1`.
- Runs inside the runner's `BEGIN IMMEDIATE` with the `user_version` bump; the runner writes `store.sqlite.bak-v<N>` first (rollback = restore that file; old code refuses a newer store, so rollback needs the backup). Every statement is `IF NOT EXISTS`.
- Upgrade from a v4 store with rows (events from fixtures, an account, a workspace + repo) keeps every row byte-equal: `test_v4_store_with_rows_upgrades_keeping_every_row`; a `.bak-v4` is left.
- Partial-index `WHERE workspace_id <> <orphan id>` uses the literal orphan id read at migration time (partial indexes cannot hold a subquery); the m0004 triggers make that row's id immutable.
- FK effects: `connectors.workspace_id` RESTRICT → `workspaces.delete` of a workspace that still has connectors now returns `WorkspaceError(409)` instead of a raw `IntegrityError` (moving to orphan is HUN-66). `account_workspaces.workspace_id` CASCADE (picks vanish with the workspace; the next sync removes the aliases). `account_registrations.connector_id` RESTRICT → a connector row can only be deleted after it is unregistered everywhere.
- **Merge order:** this branch has no m0005 (HUN-63). The runner skips any migration ≤ `user_version`, so HUN-63 (m0005) must merge before this branch, and no real store may run this branch alone.

**Security self-check:**
- Secrets: only `keychain.store(<id>, ...)` under service `easy-personal/<id>`. No secret in any table (schema has no value column; leak suite scans `.dump` + raw db/WAL bytes), plan line, CLI/API output, `claude` argv or log (leak tests + AFTER count: 0 everywhere except the keyring file).
- Plan lines hold account label, alias, connector id and `exec <id>` only. `claude mcp get` output is parsed for `Command:`/`Args:` internally (`writer.entry_target`) and never printed.
- Env: `writer.account_env` copies `os.environ`, removes `CLAUDE_CONFIG_DIR`, then sets it only for non-default accounts (`config_dir_env`). Tests set a runner `CLAUDE_CONFIG_DIR` and assert it never reaches a call.
- No shell: every `claude` call is `subprocess.run([exe, *args], env=...)`. Snippets printed for a missing `claude` are `shlex.quote`d.
- Ownership: sync removes only aliases in `account_registrations`; an existing unregistered name is `refused`; names not managed are never touched (`test_unregistered_names_are_never_removed`).
- Error messages never echo caller input (unknown workspace / bad alias / bad account id); leak harness plants the canary in `workspace`, `alias`, `workspaces[]`, `account`.
- API: new routes sit behind the existing `Guard` (token + Host/Origin, 403 tested); writes need `confirm: true`; `_WRITE_LOCK` serialises connector writes and sync. MCP: no tool added (FR-021).
- `launch.py` unchanged (NFR-006); `writer` imports `launch._NAME` for the alias rule.

**Cut list (not built):** parallel apply across accounts; auto-sync after `account use` (it prints a hint instead); a separate GET for picks (account rows now carry `workspaces`); dashboard UI (HUN-68/HUN-69); moving/aliasing/orphaning (HUN-66); legacy adoption (HUN-67).

**Supervisor decision applied (2026-10-26, open question 1):** an `ignored` Claude account that still holds registrations gets an empty desired set: `plan` emits `remove` (or `forget`) for each registered alias, and `plan_all` includes it. Its picks no longer count. A missing folder stays `skipped`. An ignored account with no registrations gives an empty plan. So `connector remove <id>` works when the holder is ignored.
