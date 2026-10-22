# easy-personal-mcp

The Easy Kit plug-in about you, not a repo: it turns your local Claude Code activity into usage numbers, warnings and an improvement plan, and manages your personal tool connectors.

Local-first. No network at runtime. Metadata only, never message content. No LLM inside.

Status: v0.1.0. It runs natively on Linux. See `PRD.md` (what and why), `PROJECT_SPEC.md` (how) and `docs/ddr/` (design decisions).

## What it reads

easy-personal reads metadata only. It never stores message text, tool input or output, file contents or secrets.

- Claude Code transcripts in `~/.claude/projects/`: token counts, model, time, project folder and tool names.
- Easy Kit parts, when you have them: kernel traces, easy-verifier score JSON, easy-ui reports and the easy-context index.
- Local git history, for index freshness.

It reads these files and never changes them. Everything goes into one SQLite store in `~/.easy-personal/`. Cost is shown as API-equivalent cost: tokens times a dated price table. It is not what your subscription charges.

## Install

You need Python 3.11 or newer, [uv](https://docs.astral.sh/uv/) and Node.js. The dashboard page is built from `web/` and is not in git, so build it before you install.

```bash
git clone https://github.com/thunderkds/easy-personal-mcp.git
cd easy-personal-mcp
npm --prefix web ci && npm --prefix web run build
uv tool install --reinstall --no-cache .
```

Check the install:

```bash
easy-personal --version
easy-personal ping
```

The first command prints `0.1.0`. The install adds two commands: `easy-personal` and `easy-personal-launch`.

## Quick start

### First ingest

Copy the metadata of your Claude Code sessions into the store. Claude Code deletes old transcripts after 30 days, so ingest often.

```bash
easy-personal ingest
```

Then read the numbers:

```bash
easy-personal usage
easy-personal usage --by project
easy-personal outliers
easy-personal warnings
easy-personal plan
easy-personal kit
```

`usage --by` takes `day`, `model`, `project` or `tool`. `usage` and `outliers` take `--since` and `--until` (ISO dates). `ingest --retention-days N` deletes events older than N days.

### Register the MCP server

Add easy-personal to Claude Code with the `claude` CLI:

```bash
claude mcp add --scope user easy-personal -- easy-personal mcp
```

Restart Claude Code. The server speaks stdio by default. For streamable HTTP, run the server yourself:

```bash
easy-personal mcp --transport http
```

It binds `127.0.0.1` and port 8765 unless you pass `--port`.

### MCP tools

The server has 10 tools. None of them writes config or returns a secret.

- `ping`
- `usage_summary`
- `usage_breakdown`
- `session_outliers`
- `warnings`
- `improvement_plan`
- `kit_summary`
- `connector_catalog`
- `connector_status`
- `connector_suggestions`

### Dashboard

```bash
easy-personal dashboard
```

This runs an ingest, then starts the page on `127.0.0.1` with a free port. It prints a URL like `http://127.0.0.1:<port>/#<token>` and opens your browser. The token is new on every launch. Press Ctrl-C to stop. The tabs are Usage, Warnings & plan, Kit and Connectors.

## Schedule

Run ingest every day with a systemd user timer:

```bash
easy-personal schedule install
```

Preview the unit files first with `easy-personal schedule install --dry-run`. Without a systemd user session, the command prints a cron line and exits with 1. Add `--retention-days N` to prune old events on each run. Check the timer and remove it:

```bash
systemctl --user list-timers | grep easy-personal
easy-personal schedule remove
```

The timer has no shell environment. Put settings in `~/.easy-personal/config.yaml`, not in environment variables.

## Connectors

A connector starts a provider's MCP server for Claude Code and keeps its secrets in the OS keychain. Claude Code runs `easy-personal-launch exec <id>`. The launcher reads the keychain and passes the secrets to the server as environment variables. No secret is in `~/.claude.json` or in a command line.

You never run the launcher by hand. This is the command that Claude Code starts for a connector named `slack`:

```bash
easy-personal-launch exec slack
```

The catalog has five providers:

- Email: `gmail`, `sendgrid`
- Messaging: `slack`, `teams`, `discord`

```bash
easy-personal connector list
easy-personal connector add slack
easy-personal connector remove slack
easy-personal doctor
```

- `connector list` shows the catalog by category. It never shows secret values.
- `connector add` asks for each field, shows what it will change, then asks you to confirm. Add `--yes` to skip the question. It writes the keychain item and registers the entry with `claude mcp add-json --scope user`. It refuses to overwrite an existing connector.
- `connector remove` deletes the keychain item and runs `claude mcp remove --scope user`.
- `doctor` checks the keychain backend, the `claude` CLI and the connector entries. It exits 0 when all pass.

`connector add`, `connector remove` and `doctor` need the `claude` CLI on `PATH` and a keychain backend such as the Secret Service. Without `claude`, `add` and `remove` write nothing and print the command to run later. Restart Claude Code after a change.

## Configuration

Settings live in `~/.easy-personal/config.yaml`. The file is optional. An environment variable wins over the file.

```yaml
ui:
  reports_dir: ~/path/to/easy-ui/reports
verifier:
  reports_dir: ~/path/to/easy-verifier-reports
thresholds:
  cache_hit_low: 0.4
```

- `ui: reports_dir:` is the easy-ui report folder. Without it, UI checks show as no data.
- `verifier: reports_dir:` is the easy-verifier report folder. The default is `~/.easy-verifier-reports`.
- `thresholds:` overrides a rule threshold by rule id. The rules are in `src/easy_personal/data/rules.yaml`.

| Variable | Meaning |
| -- | -- |
| `EASY_PERSONAL_HOME` | Data folder instead of `~/.easy-personal` |
| `EASY_PERSONAL_CLAUDE_PROJECTS` | Transcript folder instead of `~/.claude/projects` |
| `EASY_PERSONAL_UI_REPORTS` | Same as `ui: reports_dir:` |
| `EASY_PERSONAL_VERIFIER_REPORTS` | Same as `verifier: reports_dir:` |
| `EASY_PERSONAL_CONTEXT_URL` | easy-context MCP URL. The default is `http://127.0.0.1:8767/mcp` |

## Upgrade and rollback

To upgrade, pull the new code, rebuild the page and reinstall. Always use `--reinstall --no-cache`. Without it, uv can reuse an old build of the same version.

```bash
git pull
npm --prefix web ci && npm --prefix web run build
uv tool install --reinstall --no-cache .
```

Before an upgrade, back up the store so you can roll back:

```bash
cp -p ~/.easy-personal/store.sqlite ~/.easy-personal/store.sqlite.backup
```

To roll back, remove the connectors you added since the deploy and the timer, uninstall the tool and restore the store:

```bash
easy-personal connector remove slack
easy-personal schedule remove
uv tool uninstall easy-personal-mcp
cp -p ~/.easy-personal/store.sqlite.backup ~/.easy-personal/store.sqlite
```

The full deploy, health check and rollback steps, and the common failures, are in [`RUNBOOK.md`](RUNBOOK.md).

## Development

```bash
uv run pytest -q
uv run ruff check .
npm --prefix web test
```

Tests never touch your real `~/.claude/`, keychain or `~/.easy-personal/`. The tests use `EASY_PERSONAL_HOME` and fixtures.

## Easy Kit

Part of the Easy Kit family: kernel [personal-agentic-claude](https://github.com/thunderkds/personal-agentic-claude),
plug-ins [easy-ui-mcp](https://github.com/thunderkds/easy-ui-mcp),
[easy-verifier-mcp](https://github.com/thunderkds/easy-verifier-mcp),
[easy-context-mcp](https://github.com/thunderkds/easy-context-mcp).
