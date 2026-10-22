# RUNBOOK — easy-personal-mcp
**Last updated**: 2026-10-22

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

Run from the repo root. There is no separate migration step: `store.connect` applies the additive `CREATE TABLE IF NOT EXISTS` schema on first open.

1. **Pre-deploy checks** — pass condition for each in brackets:
   ```bash
   cd ~/workspace/pets/hungnguyen111/easy-personal-mcp
   git fetch github && git checkout main && git status --short     # [no output]
   git rev-parse --short HEAD                                       # [30e21a3 for v0.1.0]
   npm --prefix web ci && npm --prefix web run build                # [✓ built]
   uv run pytest -q && npm --prefix web test                        # [all passed, 0 failed]
   ```
2. **Smoke test (defined before deploy)** — the post-deploy health checks in step 6; all must pass.
3. **Back up the store** (rollback point):
   ```bash
   cp -p ~/.easy-personal/store.sqlite ~/.easy-personal/store.sqlite.pre-v0.1.0 2>/dev/null || echo "no store yet"
   ```
4. **Install** (replaces the existing install of the same version; `--no-cache` avoids a stale build of 0.1.0):
   ```bash
   uv tool install --reinstall --no-cache .
   ```
5. **Tag the release**:
   ```bash
   git tag -a v0.1.0 30e21a3 -m "v0.1.0" && git push github v0.1.0
   ```
6. **Post-deploy health check** (all must pass):
   ```bash
   easy-personal --version                    # pass: prints 0.1.0
   command -v easy-personal-launch            # pass: prints a path under ~/.local/bin
   easy-personal ping                         # pass: exit 0
   easy-personal connector list               # pass: Email (gmail, sendgrid), Messaging (slack, teams, discord)
   easy-personal doctor; echo "rc=$?"         # pass: rc=0 (keychain backend + claude CLI found)
   printf '%s\n' \
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"hc","version":"0"}}}' \
    '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
    '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
    | (cat; sleep 2) | timeout 10 easy-personal mcp 2>/dev/null | grep -o '"name":"[a-z_]*"' | wc -l
                                              # pass: 10 (verified against 30e21a3 on 2026-10-22)
   easy-personal dashboard                    # pass: prints "dashboard: http://127.0.0.1:<port>/#<token>"; page opens with tabs Usage, Warnings & plan, Kit, Connectors; Ctrl-C to stop
   ```
7. **Optional — daily ingest timer**: `easy-personal schedule install`, then `systemctl --user list-timers | grep easy-personal` → pass: one timer listed.

---

## Rollback Procedure

- **Trigger conditions**: any step-6 health check fails; `easy-personal ingest` or the dashboard errors on the real store; a connector added after deploy fails to start in Claude Code.
- **Reverse steps** (in order). There is no earlier tagged release, so rollback removes the install and restores the store:
  1. For each connector added since deploy: `easy-personal connector remove <id>` (deletes the keychain item and runs `claude mcp remove <id> --scope user`).
  2. If the timer was installed: `easy-personal schedule remove`.
  3. `uv tool uninstall easy-personal-mcp`
  4. Restore the store: `cp -p ~/.easy-personal/store.sqlite.pre-v0.1.0 ~/.easy-personal/store.sqlite` (the new tables are additive; restoring only drops derived data that a re-ingest rebuilds).
  5. To return to a specific earlier commit instead of uninstalling: `git checkout <commit> && npm --prefix web ci && npm --prefix web run build && uv tool install --reinstall --no-cache .`
- **Verify rollback**: `command -v easy-personal` → no output (uninstalled), and `claude mcp list` shows no `easy-personal`-launched entry.

---

## Health Checks & Dashboards

| Check | Command / URL | Pass condition |
|-------|---------------|----------------|
| Version | `easy-personal --version` | `0.1.0` |
| Core | `easy-personal ping` | exit 0 |
| Connector prerequisites | `easy-personal doctor` | exit 0 |
| MCP surface | `tools/list` over stdio (step 6) | includes `connector_catalog`, `connector_status`, `connector_suggestions`, `usage_summary`, `kit_summary` |
| Dashboard | `easy-personal dashboard` | URL on `127.0.0.1` printed; 4 tabs render |

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
