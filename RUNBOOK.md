# RUNBOOK — easy-personal-mcp
**Last updated**: 2026-11-03

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

v0.4.0 (approved for `385dbed`, owner 2026-11-03; first approved at `373d007`, re-approved to add HUN-196). It moves a store from version 7 to 8 (`m0008_tool_errors`, HUN-192: adds `events.tool_error_count`, resets claude_code offsets to 0; the next ingest or the dashboard's live tail re-reads every transcript on disk, adding no rows, about 20–45 s). **On the owner's machine this already happened**: the owner reinstalled from `main` at 2026-11-03 10:54 +07 (before the 0.4.0 version bump, so `--version` still says 0.3.0) and started the dashboard; the store is at 8 (393,188 events, 47,614 user rows with a count) and `store.sqlite.bak-v7` (386,360 events, 10:54:23) exists. So no migration runs at step 4; the reinstall only brings the 0.4.0 code (HUN-183 docs, HUN-195 5 % gap rule, HUN-196 cache reuse card) and version.

1. **Pre-deploy checks**. The pass condition for each is in brackets:
   ```bash
   cd ~/workspace/pets/hungnguyen111/easy-personal-mcp
   git checkout main && git status --short -- src web tests pyproject.toml uv.lock README.md   # [no output]
   git merge-base --is-ancestor 385dbed HEAD && echo ok              # [ok: the approved commit is on main]
   npm --prefix web ci && npm --prefix web run build                # [✓ built]
   uv run pytest -q && npm --prefix web test                        # [0 failed; 1 xfailed is the known tool_signals P3]
   ```
2. **Smoke test (defined before deploy)**: the post-deploy health checks in step 6. All must pass.
3. **Back up the store** (the rollback point). Stop any running `easy-personal dashboard` first (Ctrl-C in its terminal; never `pkill -f "easy-personal dashboard"`):
   ```bash
   systemctl --user stop easy-personal-ingest.timer 2>/dev/null; true
   cp -p ~/.easy-personal/store.sqlite ~/.easy-personal/store.sqlite.pre-v0.4.0
   ```
4. **Install** (`--no-cache` avoids a stale build):
   ```bash
   uv tool install --reinstall --no-cache .
   easy-personal ingest | head -3             # migrates 7 -> 8 if a store is still at 7 (writes .bak-v7), then ingests
   ```
5. **Tag the release** (v0.3.0 was never tagged; its row below is superseded):
   ```bash
   git tag -a v0.4.0 385dbed -m "v0.4.0: live attention, easy-memory layer, dashboard UX, kit performance, cache reuse card" && git push github main v0.4.0
   ```
6. **Post-deploy health check** (all must pass):
   ```bash
   easy-personal --version                    # pass: prints 0.4.0
   python3 -c "import sqlite3;print(sqlite3.connect('file:'+__import__('os').path.expanduser('~/.easy-personal/store.sqlite')+'?mode=ro',uri=True).execute('pragma user_version').fetchone()[0])"
                                              # pass: 8
   sqlite3 -readonly ~/.easy-personal/store.sqlite "select count(*) from events where record_type='user' and tool_error_count is not null"
                                              # pass: > 0 (about 47,600 on the owner's store)
   easy-personal usage | head -5              # pass: totals print, not lower than before the deploy
   easy-personal kit-findings                 # pass: exits 0 within 3 s; every line says "linked to"; habits only on gaps >= 5 %
   easy-personal doctor; echo "rc=$?"         # pass: rc=0 (no FAIL lines)
   # HUN-196, manual: easy-personal dashboard -> Usage, pick "Last 30 days" (wide window, >= 80rem):
   #   pass: the rail's Cache reuse card note says "last 30 days" and shows "Written to cache" and "Lowest: <model>"
   printf '%s\n' \
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"hc","version":"0"}}}' \
    '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
    '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
    | (cat; sleep 2) | timeout 10 easy-personal mcp 2>/dev/null | grep -o '"version":"0.4.0"\|"name":"[a-z_]*"' | sort -u | wc -l
                                              # pass: 12 (serverInfo version 0.4.0 + 11 tools, incl. kit_findings)
   easy-personal dashboard                    # pass: prints the 127.0.0.1 URL; Live tab connects; a failing tool streak
                                              # shows a "failing" badge; Memory view shows easy-memory cited-file nodes;
                                              # Usage lists show 10 rows + "Show N more"; Ctrl-C stops it at once
   ```
7. **Restart** the timer if you stopped it: `systemctl --user start easy-personal-ingest.timer`. Restart Claude Code sessions (or `/mcp` reconnect) so the `easy-personal` MCP server runs the 0.4.0 code and lists `kit_findings`.

v0.3.0 (approved for `1c80450`; superseded by v0.4.0, never tagged):

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

- **v0.4.0 reverse steps** (in order). The 0.3.0 code refuses a schema-8 store ("schema v8, newer than this program (v7): upgrade easy-personal" — tested 2026-11-03), so restore the store first:
  1. Stop the dashboard (Ctrl-C) and the timer: `systemctl --user stop easy-personal-ingest.timer`.
  2. Restore schema 7: `cp -p ~/.easy-personal/store.sqlite.bak-v7 ~/.easy-personal/store.sqlite && rm -f ~/.easy-personal/store.sqlite-wal ~/.easy-personal/store.sqlite-shm` (events ingested after the migration come back on the next ingest from transcripts still on disk).
  3. Install the 0.3.0 code: `git checkout 1c80450 && npm --prefix web ci && npm --prefix web run build && uv tool install --reinstall --no-cache . && git checkout main`.
  4. If the tag was pushed: `git push github :refs/tags/v0.4.0 && git tag -d v0.4.0`.
  5. Verify: `easy-personal --version` prints 0.3.0; `pragma user_version` is 7; `easy-personal usage` prints totals.
- **Trigger conditions** (v0.4.0 adds: `kit-findings` errors or takes > 3 s; any finding text without "linked to"; any tool-result or error text in the store, MCP or CLI output — metadata-only breach, roll back at once): any step-6 health check fails; the install or the first ingest errors; usage totals drop; the dashboard does not stop on Ctrl-C; the Live tab shows message or tool content (metadata-only breach — roll back at once).
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
| Version | `easy-personal --version` | `0.4.0` |
| Store schema | `pragma user_version` (step 6) | `8` |
| Kit findings | `easy-personal kit-findings` | exit 0 in < 3 s; "linked to" only |
| Core | `easy-personal ping` | exit 0 |
| Connectors | `easy-personal doctor` | exit 0 |
| MCP surface | `tools/list` over stdio (step 6) | 11 tools (incl. `kit_findings`), serverInfo version `0.4.0` |
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
| `.bak-v7` exists but `--version` says 0.3.0 | Reinstalled from `main` between the v8 merge and the version bump (owner, 2026-11-03) | Harmless: code is v8-aware; reinstall at the tagged commit (step 4) |
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
| v0.3.0 @ 1c80450 (never tagged; superseded by v0.4.0) | 2026-11-01 | HUN-175–179 (Live view: 2.1.316 record types, live tail, `GET /api/live` SSE, Live tab, release prep); also since v0.2.0: HUN-83, HUN-113–121, HUN-126–132 (dashboard look and Kit cards), HUN-160–164 (Work view: Board, Scheduled, Memory; You/Agents split + m0007), HUN-165–170 (live-trace spike, experiments only) | thunderkds (operator) | owner / 2026-11-01 / "Approve v0.3.0 @ 1c80450 (Recommended)" | Superseded — not tagged; the owner's install moved past it (store v8 on 2026-11-03) |
| v0.4.0 @ 385dbed (tag at deploy step 5) | 2026-11-03 | HUN-181–182 (live attention states + badge), HUN-184–185 (easy-memory layer), HUN-186–191 (dashboard speed, gone projects, owner project, board/usage/memory/kit UX, top-10 lists), HUN-192 (store v8 `tool_error_count`), HUN-193 (`kit_findings` MCP + CLI), HUN-195 (5 % habit gap), HUN-183 (release docs), HUN-196 (cache reuse card follows the period, written-to-cache + lowest model); HUN-194 closed as duplicate | thunderkds (operator) | owner / 2026-11-03 / "Approve v0.4.0 @ 385dbed (Recommended)" (supersedes the earlier approval of 373d007) | Pending — operator to run steps 3–7 (store already migrated to v8 by the owner's 10:54 reinstall) |
