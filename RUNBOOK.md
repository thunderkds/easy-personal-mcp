# RUNBOOK — easy-personal-mcp
**Last updated**: 2026-11-01

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

Run from the repo root. Schema changes are numbered migrations (`core/migrations/mNNNN_*.py`, `PRAGMA user_version`). The first command that opens the store applies every pending migration, each in one transaction, and leaves `store.sqlite.bak-v<N>` (the store as it was at version N) beside it. v0.3.0 moves a store from version 6 to 7 (`m0007_entrypoint`, HUN-161: adds `events.entrypoint`, resets claude_code offsets to 0; the next ingest re-reads and backfills, adding no rows). On the owner's machine the store is already at 7 (a main build migrated it 2026-10-31 21:50; `store.sqlite.bak-v6` exists), so no migration runs there.

v0.3.0 (approved for `1c80450`):

1. **Pre-deploy checks**. The pass condition for each is in brackets:
   ```bash
   cd ~/workspace/pets/hungnguyen111/easy-personal-mcp
   git checkout main && git status --short -- src web tests pyproject.toml uv.lock README.md   # [no output]
   git merge-base --is-ancestor 1c80450 HEAD && echo ok              # [ok: the approved commit is on main]
   npm --prefix web ci && npm --prefix web run build                # [✓ built]
   uv run pytest -q && npm --prefix web test                        # [0 failed; 1 xfailed is the known tool_signals P3]
   ```
2. **Smoke test (defined before deploy)**: the post-deploy health checks in step 6. All must pass.
3. **Back up the store** (the rollback point). Stop any running `easy-personal dashboard` first:
   ```bash
   systemctl --user stop easy-personal-ingest.timer 2>/dev/null; true
   cp -p ~/.easy-personal/store.sqlite ~/.easy-personal/store.sqlite.pre-v0.3.0
   ```
4. **Install** (`--no-cache` avoids a stale build):
   ```bash
   uv tool install --reinstall --no-cache .
   easy-personal ingest | head -3             # opens the store: migrates 6 -> 7 if still at 6, then ingests
   ```
5. **Tag the release**:
   ```bash
   git tag -a v0.3.0 1c80450 -m "v0.3.0: live view" && git push github v0.3.0
   ```
6. **Post-deploy health check** (all must pass):
   ```bash
   easy-personal --version                    # pass: prints 0.3.0
   python3 -c "import sqlite3;print(sqlite3.connect('file:'+__import__('os').path.expanduser('~/.easy-personal/store.sqlite')+'?mode=ro',uri=True).execute('pragma user_version').fetchone()[0])"
                                              # pass: 7
   easy-personal usage | head -5              # pass: totals print, not lower than before the deploy
   easy-personal account list                 # pass: "Claude default" confirmed
   easy-personal doctor; echo "rc=$?"         # pass: rc=0 (no FAIL lines)
   printf '%s\n' \
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"hc","version":"0"}}}' \
    '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
    '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
    | (cat; sleep 2) | timeout 10 easy-personal mcp 2>/dev/null | grep -o '"version":"0.3.0"\|"name":"[a-z_]*"' | sort -u | wc -l
                                              # pass: 11 (serverInfo version 0.3.0 + 10 tools)
   easy-personal dashboard                    # pass: prints "dashboard: http://127.0.0.1:<port>/#<token>"; the Live tab
                                              # shows "Live stream connected"; run any short Claude Code session and its
                                              # tool calls appear in the Live feed within a few seconds; Ctrl-C stops it at once
   ```
   Live stream probe without a browser (with the dashboard running, `<port>` and `<token>` from its printed URL):
   ```bash
   curl -s -N -m 3 "http://127.0.0.1:<port>/api/live?token=<token>" | head -2   # pass: "event: snapshot" then a "data: {...}" line
   curl -s -o /dev/null -w '%{http_code}\n' -m 2 "http://127.0.0.1:<port>/api/live"   # pass: 403 (no token)
   ```
7. **Optional**: restart the timer if you stopped it: `systemctl --user start easy-personal-ingest.timer`.

---

## Rollback Procedure

- **Trigger conditions**: any step-6 health check fails; the install or the first ingest errors; usage totals drop; the dashboard does not stop on Ctrl-C; the Live tab shows message or tool content (metadata-only breach — roll back at once).
- **Reverse steps** (in order). v0.3.0 adds no schema change of its own, and the schema-7 store also works with the rollback build, so no store restore is needed unless the store itself is damaged:
  1. Install the previous known-good build (last pushed `main` before the Live work, schema 7): `git checkout 1550317 && npm --prefix web ci && npm --prefix web run build && uv tool install --reinstall --no-cache . && git checkout main`.
  2. Only if the store is damaged: `cp -p ~/.easy-personal/store.sqlite.pre-v0.3.0 ~/.easy-personal/store.sqlite && rm -f ~/.easy-personal/store.sqlite-wal ~/.easy-personal/store.sqlite-shm`.
  3. If the tag was pushed: `git push github :refs/tags/v0.3.0 && git tag -d v0.3.0`.
  4. Do **not** install the `v0.2.0` tag: its code refuses a schema-7 store. Going back to v0.2.0 needs `store.sqlite.bak-v6` (2026-10-31 21:50), which loses events ingested since then.
- **Verify rollback**: `easy-personal ping` exits 0; `easy-personal dashboard` has no Live tab; `pragma user_version` is still 7.

---

## Health Checks & Dashboards

| Check | Command / URL | Pass condition |
|-------|---------------|----------------|
| Version | `easy-personal --version` | `0.3.0` |
| Store schema | `pragma user_version` (step 6) | `7` |
| Core | `easy-personal ping` | exit 0 |
| Connectors | `easy-personal doctor` | exit 0 |
| MCP surface | `tools/list` over stdio (step 6) | 10 tools, serverInfo version `0.3.0` |
| Dashboard | `easy-personal dashboard` | URL on `127.0.0.1` printed; Live, Board, Scheduled, Memory, Usage, Warnings & plan, Kit, Workspaces, Agents render |
| Live stream | `curl -N .../api/live?token=<token>` | `event: snapshot` first; 403 without token |

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
| Live tab says "reconnecting" and never recovers | Dashboard stopped or restarted (new port and token) | Open the new URL the dashboard printed; the old page cannot reconnect |
| Live feed empty while Claude Code runs | Account not confirmed, or the session is in a folder of an unconfirmed account | `easy-personal account list`; `easy-personal account confirm <folder>` |
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
| v0.3.0 @ 1c80450 (tag at deploy step 5) | 2026-11-01 | HUN-175–179 (Live view: 2.1.316 record types, live tail, `GET /api/live` SSE, Live tab, release prep); also since v0.2.0: HUN-83, HUN-113–121, HUN-126–132 (dashboard look and Kit cards), HUN-160–164 (Work view: Board, Scheduled, Memory; You/Agents split + m0007), HUN-165–170 (live-trace spike, experiments only) | thunderkds (operator) | owner / 2026-11-01 / "Approve v0.3.0 @ 1c80450 (Recommended)" | Pending — operator to run |
