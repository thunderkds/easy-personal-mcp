# RUNBOOK — easy-personal-mcp
**Last updated**: 2026-10-26

> Operational runbook: how to deploy, verify, and recover this service. Written/appended by the `ship` skill after Stage 5 verification, and kept current by whoever last touched the deploy path. This is the document an operator opens at 3am — every command must be copy-pasteable and every check must have a pass condition.

---

## Service Identity

- **Name**: easy-personal-mcp (CLI `easy-personal`, launcher `easy-personal-launch`)
- **Repo**: https://github.com/thunderkds/easy-personal-mcp (local: `~/workspace/pets/hungnguyen111/easy-personal-mcp`)
- **Deployment target**: the owner's Linux machine, native install (`uv tool install`); Docker optional, analytics-only (DDR-0001)
- **Tech**: Python (uv tool), FastMCP, Starlette/uvicorn dashboard on 127.0.0.1, React/Vite static build, SQLite store at `~/.easy-personal/store.sqlite`, OS keychain via `keyring`
- **Owner / on-call**: thunderkds (owner)

---

## Deploy Procedure

Run from the repo root. Schema changes are numbered migrations (`core/migrations/mNNNN_*.py`, `PRAGMA user_version`). The first command that opens the store applies every pending migration, each in one transaction, and leaves `store.sqlite.bak-v<N>` (the store as it was at version N) beside it. v0.2.0 moves a store from version 2 to version 6.

1. **Pre-deploy checks**. The pass condition for each is in brackets:
   ```bash
   cd ~/workspace/pets/hungnguyen111/easy-personal-mcp
   git checkout main && git status --short -- src web tests pyproject.toml uv.lock README.md   # [no output]
   git merge-base --is-ancestor ac64aae HEAD && echo ok              # [ok: the v0.2.0 code commit is on main]
   npm --prefix web ci && npm --prefix web run build                # [✓ built]
   uv run pytest -q && npm --prefix web test                        # [0 failed; 1 xfailed is the known tool_signals P3]
   ```
2. **Smoke test (defined before deploy)**: the post-deploy health checks in step 7. All must pass.
3. **Back up the store** (the rollback point). Stop any running `easy-personal dashboard` first:
   ```bash
   systemctl --user stop easy-personal-ingest.timer 2>/dev/null; true
   cp -p ~/.easy-personal/store.sqlite ~/.easy-personal/store.sqlite.pre-v0.2.0
   ```
4. **Install** (`--no-cache` avoids a stale build):
   ```bash
   uv tool install --reinstall --no-cache .
   ```
5. **Upgrade the store and adopt connectors from before workspaces**:
   ```bash
   easy-personal account list                # opens the store: migrates v2 -> v6; lists "Claude default" (confirmed) and the detected folders
   easy-personal connector adopt-legacy      # shows the plan; answer y only if it lists your old connectors. It removes only entries that start easy-personal-launch.
   ```
   Then confirm the other agent folders you use (`easy-personal account confirm <folder> --label "<name>"`), add workspaces, and pick workspaces per account (`easy-personal account use <account> <workspace>...`), then `easy-personal connector sync`.
6. **Tag the release**:
   ```bash
   git tag -a v0.2.0 ac64aae -m "v0.2.0: workspaces" && git push github main v0.2.0
   ```
7. **Post-deploy health check** (all must pass):
   ```bash
   easy-personal --version                    # pass: prints 0.2.0
   python3 -c "import sqlite3;print(sqlite3.connect('file:'+__import__('os').path.expanduser('~/.easy-personal/store.sqlite')+'?mode=ro',uri=True).execute('pragma user_version').fetchone()[0])"
                                              # pass: 6
   easy-personal usage | head -5            # pass: totals print (the store kept its 575 events of 2026-10-26)
   easy-personal workspace list               # pass: includes "orphan"
   easy-personal account list                 # pass: "Claude default" confirmed
   easy-personal doctor; echo "rc=$?"         # pass: rc=0 (no FAIL lines)
   printf '%s\n' \
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"hc","version":"0"}}}' \
    '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
    '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
    | (cat; sleep 2) | timeout 10 easy-personal mcp 2>/dev/null | grep -o '"name":"[a-z_]*"' | wc -l
                                              # pass: 10
   easy-personal dashboard                    # pass: prints "dashboard: http://127.0.0.1:<port>/#<token>"; tabs Usage, Warnings & plan, Kit, Workspaces, Agents; Ctrl-C to stop
   ```
8. **Optional**: restart the timer if you stopped it: `systemctl --user start easy-personal-ingest.timer`.

---

## Rollback Procedure

- **Trigger conditions**: any step-7 health check fails; the migration in step 5 errors; usage totals drop; a connector stops starting in Claude Code after `connector sync`.
- **Reverse steps** (in order):
  1. If `connector sync` ran: unregister every alias it added, keeping the keychain items. For each connector id from `easy-personal connector list`: `easy-personal connector move <id> orphan --yes`.
  2. Install the previous version: `git checkout 30e21a3 && npm --prefix web ci && npm --prefix web run build && uv tool install --reinstall --no-cache . && git checkout main`.
  3. Restore the store: `cp -p ~/.easy-personal/store.sqlite.pre-v0.2.0 ~/.easy-personal/store.sqlite && rm -f ~/.easy-personal/store.sqlite-wal ~/.easy-personal/store.sqlite-shm`.
  4. If `adopt-legacy` removed old entries, add them again with v0.1.0: `easy-personal connector add <name>` (it asks for the secret again).
- **Verify rollback**: `easy-personal --version` prints `0.1.0`; `easy-personal ping` exits 0; `claude mcp list` shows no `<workspace>-<provider>` alias.

---

## Health Checks & Dashboards

| Check | Command / URL | Pass condition |
|-------|---------------|----------------|
| Version | `easy-personal --version` | `0.2.0` |
| Store schema | `pragma user_version` (step 7) | `6` |
| Core | `easy-personal ping` | exit 0 |
| Connectors | `easy-personal doctor` | exit 0 |
| MCP surface | `tools/list` over stdio (step 7) | 10 tools |
| Dashboard | `easy-personal dashboard` | URL on `127.0.0.1` printed; 5 tabs render |

- **Dashboards**: the local dashboard itself (`easy-personal dashboard`); no remote monitoring.

---

## Common Failure Modes & Remediation

| Symptom | Likely cause | Remediation |
|---------|-------------|-------------|
| Dashboard page blank / 404 on `/` | Static build missing from the wheel | `npm --prefix web run build`, then `uv tool install --reinstall --no-cache .` |
| Old behaviour after reinstall of the same version | uv reused a cached build | Always use `--reinstall --no-cache` |
| `doctor` reports no keychain backend | No Secret Service (e.g. headless session) | Unlock/start the keyring daemon; connectors cannot be added without it |
| `static build stale` in `tests/test_web_toolchain.py` | `web/` changed since the last build | `npm --prefix web run build` |
| Timer runs but data stale | Timer has no shell env | Put settings in `~/.easy-personal/config.yaml`, not env vars |
| `doctor` notes "legacy connectors not adopted" | Connectors from v0.1.0 | `easy-personal connector adopt-legacy`, then `connector move <id> <workspace>` |
| `connector sync` refuses a step "already exists" | An MCP server with that alias was made by hand | Rename yours, or give the connector another alias: `easy-personal connector alias <id> <alias>` |
| Migration fails on open | Disk full or store locked | Store is unchanged (one transaction per migration); free space or stop other `easy-personal` processes, then retry. Older copies: `store.sqlite.bak-v<N>` |

---

## On-Call / Escalation

1. **First responder**: thunderkds (owner)
2. **Escalate to**: none (personal tool)
3. **Comms**: Linear project easy-personal-mcp

---

## Release Log

| Version / Tag | Date | Scope (Task IDs) | Deployer | Approved (by / when / answer) | Outcome |
|---------------|------|------------------|----------|-------------------------------|---------|
| v0.1.0 @ 30e21a3 (tag to be created at deploy step 5) | 2026-10-22 | HUN-32–41, HUN-43–53 (HUN-42 excluded: blocked by XR-1) | thunderkds (operator) | owner / 2026-10-22 / "Approve v0.1.0 @ 30e21a3 (Recommended)" | Pending — operator to run |
| v0.2.0 @ ac64aae (tag at deploy step 6) | 2026-10-26 | HUN-42, HUN-56–58, HUN-61–73 (workspaces, agent accounts, connector id/alias + sync, Workspaces/Agents pages, multi-account ingest) | thunderkds (operator) | owner / 2026-10-26 / "Approve v0.2.0 @ ac64aae (Recommended)" | Deployed 2026-10-26 23:4x +07 by Supervisor on owner request ("run all for me"): backup store.sqlite.pre-v0.2.0; installed 0.2.0; store migrated v2 -> v6 (575 events kept, bak-v2 written); adopt-legacy: nothing to adopt; health checks all pass (version 0.2.0, schema 6, ping 0, doctor rc=0, 10 MCP tools, dashboard 5 tabs, no console errors); tag v0.2.0 pushed with main to github |
