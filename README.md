# easy-personal-mcp

The Easy Kit plug-in about you, not a repo: it turns your local Claude Code activity into usage numbers, warnings and an improvement plan, and manages your personal tool connectors.

Local-first. No network at runtime. Metadata only, never message content. No LLM inside.

Status: v0.4.0. It runs natively on Linux. See `PRD.md` (what and why), `PROJECT_SPEC.md` (how) and `docs/ddr/` (design decisions).

## What it reads

easy-personal reads metadata only. It never stores message text, tool input or output, file contents or secrets.

- Claude Code transcripts in `~/.claude/projects/`: token counts, model, time, project folder and tool names.
- For the Agents page, from each Claude account folder: the account email, plan and organization (three keys of `<folder>/.claude.json`, no other key) and the plan-limit snapshot `<folder>/easy-personal-limits.json` that your status line writes. A missing file shows as empty.
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

The first command prints `0.4.0`. The install adds two commands: `easy-personal` and `easy-personal-launch`.

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
easy-personal kit-findings --json
easy-personal store-check
```

`store-check [--json] [--full]` is a read-only health check of the store: file and `-wal` size, the schema version beside the newest migration the program knows, the verdict of `PRAGMA integrity_check` and `PRAGMA foreign_key_check` (the verdict only, never a row), the row count of every table, the newest record time and abstained count per source (`no data` when a source never ran) and the `store.sqlite.bak-v<N>` backups by name. It writes nothing and runs no migration. The default run stops after 1.5 seconds. On a store over 32 MB it runs `PRAGMA quick_check` and then usually reports `not fully checked`: run `easy-personal store-check --full` to get the integrity verdict (no time limit, it can take minutes, Ctrl-C stops it). Exit code: 0 healthy, 1 a problem was found, 3 not fully checked, 130 stopped with Ctrl-C. The dashboard serves the same answer at `GET /api/store/health` (never `--full`; `healthy` is `null` when not fully checked). `kit-findings` compares sessions with and without each Easy Kit MCP server (see Kit performance below; `--limit`, `--since`). `usage --by` takes `day`, `model`, `project` or `tool`. `usage --by project` and the MCP tool `usage_breakdown` put every scratch folder into one group named `Scratch` (with `scratch: true` and a `folders` list of the same numbers per folder name). A scratch folder is a project path under the system temp folder, a folder that no longer exists, a folder outside any git repo, or one inside a hidden folder; a gone worktree is still counted under its repo. The totals do not change. `usage` and `outliers` take `--since` and `--until` (ISO dates). `ingest --retention-days N` deletes events older than N days.

Group repos into workspaces and filter usage by one. A workspace counts events whose project path equals or sits under one of its repo paths. The name is a slug (`orphan` is reserved for the built-in workspace). The email is information only. The dashboard also serves the real projects that have usage but sit in no workspace at `GET /api/workspaces/unassigned` (all stored time, newest use first; read only).

```bash
easy-personal workspace add company --repo ~/work/app --repo ~/work/api
easy-personal workspace list
easy-personal workspace edit company --rename acme --add-repo ~/work/web
easy-personal usage --workspace acme
easy-personal workspace delete acme   # its connectors move to orphan
```

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

The server has 12 tools. None of them writes config or returns a secret. Only `save_report` writes, and only a report to the store (see the prompt below).

- `ping`
- `usage_summary`
- `usage_breakdown`
- `session_outliers`
- `warnings`
- `improvement_plan`
- `kit_summary`
- `kit_findings`
- `connector_catalog`
- `connector_status`
- `connector_suggestions`
- `save_report`

### MCP prompt

The server also offers one prompt, `investigate`. In Claude Code, type `/mcp__easy-personal__investigate` (optional arguments `workspace` and `account`). It asks your own Claude session to read the warnings, the plan, usage by project and model for the last 7 days, and the session outliers. Then it explains what caused each warning and suggests at most 5 fixes, each with a number from a tool result. The last step asks your session to call `save_report` once with its summary (at most 2,000 characters) and fixes (at most 5, each at most 300 characters). The server refuses the whole call, naming the field, when a limit is passed, the scope is unknown or the text looks like a secret (a key prefix such as `sk-` or `AKIA`, a private-key header, or a run of 32 or more letters and digits). It then stores the report with its own numbers for every rule at that moment, and `GET /api/reports` and `GET /api/reports/{id}` (dashboard token needed) serve them with a before / now line per rule. The Warnings & plan tab shows them under **Saved reports**, newest first. An opened report shows its summary, its fixes and each rule as the value saved then next to the value measured now. The tab cannot edit or delete a report. This report text is the one text the store keeps, and it stays on your machine. The prompt text is fixed. The server calls no model and no network; your Claude Code session does the reasoning.

### Dashboard

```bash
easy-personal dashboard
```

This runs an ingest, then starts the page on `127.0.0.1` with a free port. It prints a URL like `http://127.0.0.1:<port>/#<token>` and opens your browser. The token is new on every launch. Press Ctrl-C to stop. The tabs are Live, Board, Scheduled, Memory, Usage, Warnings & plan, Kit, Workspaces and Agents, plus a First run checklist.

### Memory view

The Memory tab draws one project's memory notes as rings: `memory/MEMORY.md` in the centre, then the other kit notes, then Claude notes, then unresolved `[[links]]`. It reads the files live and stores nothing. A note body is never sent to the page.

If you use easy-memory, a fourth, outer ring draws the files its notes cite (the 30 most cited, bigger when cited more). A file that changed since the newest note that cites it, or is gone, gets a mark. A cited file that is also a kit memory note is drawn once, as that note. Click a node to see its path, how many notes cite it, the date of the newest one and its state. A panel beside the graph shows notes saved per day for the last 30 days, how often notes were injected into prompts, files not drawn and notes that could not be read. The project list shows the easy-memory note count beside the kit and Claude counts.

easy-memory is read as files in `$EASY_MEMORY_HOME` (else `~/.easy-memory`), never through its MCP server, and never written. From a note only its time and the file paths it cites are read. Its prompt, answer and session id are never read out, and neither is its `trash/` folder.

### Live view

The Live tab shows what Claude Code is doing now. It runs only while `easy-personal dashboard` is open. There is nothing to install. It adds no hooks and no OpenTelemetry, and it changes no Claude config file.

While the dashboard runs, a background tail follows the transcripts of every confirmed Claude account and checks for new lines about every 100 ms. It reads them through the normal ingest path. The tail and the scheduled ingest share one read position, so no event is counted twice. The page gets the new events over `GET /api/live` (Server-Sent Events). That route uses the same `127.0.0.1` and token guard as the rest of the API.

The tab shows:

- Active sessions, one **session row** for each session that wrote a record in the last 5 minutes (or has an open call), grouped by account. A row shows the project, a short session id, the attention badges below, active or idle with the time since the last record, and three bars for tokens (in + out), API-equivalent cost and tool calls, each with its number. A full bar is the largest value of that measure among the listed rows. An unknown model price shows "unknown price", never a bar for $0.
- The **kit stage** of a session as a chip on its row, such as `Stage 4 · code-review`. It is the stage of the newest `Skill` call whose skill is in a fixed map of the kit stages (0.5 to 5). There is no chip until the tail has seen such a call, live or in the warm start replay below. A slash command that you type is not seen.
- A feed of tool calls for one session at a time. Click a row, or press Enter or Space on it, to open the feed in place under the row. Click the row again, use its Close control or press Escape to close it. The feed lists the newest call first, the last 50, one line each: the time, who made the call (`main` or the sub-agent type) and the call with its label. A label is the skill name, the sub-agent type, the first word of a Bash command, or the MCP server and tool. Nothing else of a call's input is shown.
- Running totals of tokens and API-equivalent cost. An unknown model price is named, never counted as $0.

Each session can carry an **attention state**. The tail works it out, and the page only shows it as a badge that names the tool, a border and a count in the Live header. Active sessions with a state sort first. There are three states:

- **failing**: one stream of the session ended its last 3 tool results in a row with an error. The next successful result in that stream clears it. A denied permission counts as an error. A stream is the main thread or one subagent.
- **waiting for you**: the session has an open `AskUserQuestion` or `ExitPlanMode` call. It shows at once. An open call is a tool call whose result has not arrived yet.
- **needs attention**: another open call is older than 10 minutes and the whole session (main thread and subagents) wrote nothing in that time. It means the call is hung or waits for a permission. A long subagent that keeps writing is not flagged.

Two settings change the limits. Put them under `thresholds:` in `~/.easy-personal/config.yaml`:

```yaml
thresholds:
  live_failing_streak: 3
  live_open_call_minutes: 10
```

`live_failing_streak` is the number of failed results in a row (default 3). `live_open_call_minutes` is the age of an open call in minutes (default 10). Only the Live view shows attention states. The MCP tools, the CLI and notifications do not.

The tail pairs a call with its result in memory only. It stores and sends no call id. An open call is dropped after 24 hours.

Two dashboards can run at once, and the scheduled ingest can run beside them. Each tab shows every new line, even when another process stored it first. A line that another process already stored is shown but not stored again, so the event count is the same as with one writer.

Warm start: when the dashboard starts, the tail replays each transcript file (main thread and subagents) that changed in the last hour, up to the saved read position. It replays them into session state only: totals, project, kit stage, open calls, failure streaks and sub-agent types. A session that is active, or has an open call, is then listed at once after a restart, with its whole-session totals. The replay adds nothing to the feed and stores nothing again, so the feed still shows only calls since the page opened.

Known limit: the warm start reads only files changed in the last hour, and only the last part of a very large file. A call that opened before `easy-personal dashboard` started, outside that replay, is not seen as open. Its session gets no state from that call.

The totals add up the listed session rows. The tab never shows message text, tool input or tool output, except the labels above. It shows only tool names, those labels, counts and times. Claude Code itself writes some records a little late, so an event can show up a second or two after it happened.

### Kit view and report folders

The Kit tab has a **Kit by workspace** block, with one row for each workspace except `orphan`. It reads only what is already stored, and it follows the account and the period that you picked.

- easy-verifier: how many repos of the workspace have a score, the lowest score with its repo folder name, the mean of the scored repos and the time of the newest report. A repo with no score counts as no score, never as 0. A workspace with no scored repo says "no data".
- easy-ui: the sessions in the period that called an `mcp__easy-ui__*` tool, the number of those calls and the time of the last one. A workspace with no call says "not used". The easy-ui check results (passed, failed, actions) stay overall, labelled "not split by workspace", because a session file names no project.

Under each workspace row the block lists the **report folders** of its repos, as a reference only. A report folder is a folder directly inside a workspace repo (for example `reports` or `smoke-reports`) with `report` in its name. A line shows `<repo folder>/<report folder>`, the number of files in it, how many of them are `session-*.json` files and the newest file time. It never shows an absolute path. A workspace with no such folder says "no report folders".

Click a report folder line to list its files, newest first. Click a file to open it in a new tab. Only `html`, `htm`, `md`, `txt`, `json`, `png`, `jpg`, `jpeg` and `pdf` files open, up to 10 MB. The page fetches the file with the dashboard token in a header and opens it as a `blob:` page, so the token is never in a URL. The report keeps its own CSS and shows its embedded `data:` screenshots. A script in a report does not run. Nothing is written, moved, deleted or stored.

### Scratch folders and unassigned repos

In Usage, the project lists put every throwaway folder into one `Scratch` row: a folder under the system temp folder, a folder that is gone and belongs to no repo, a folder that is not inside a repo, or one in a hidden folder. Click the row to list its folders with their own numbers. The Workspaces page lists every repo with usage that is in no workspace, with its sessions, cost and last-used date. Pick a workspace in the `Add to` control of a row to add the repo to it. The row then leaves the list.

### Kit performance

The MCP tool `kit_findings` and the command `easy-personal kit-findings` answer one question: which Easy Kit servers are linked to cheaper or better sessions on your own history. The four kit servers are easy-ui, easy-verifier, easy-context and easy-memory. A session used a kit server when one of its tool calls is named `mcp__<server>__...`. Subagent calls count for their session.

```bash
easy-personal kit-findings
easy-personal kit-findings --json --limit 5 --since 2026-10-01
```

The comparison is like with like. A session that used a kit server is compared with sessions that did not, in the same project, with the same main model and a similar number of tool calls. Sessions with no tool call are not compared. Three measures are compared: tokens per tool call, API-equivalent cost per tool call and the share of good results. Raw totals are not compared, because a long session always has more tokens.

A **good result** is a session that has a commit on its own git branch between its first event and 30 minutes after its last event, and a tool-error rate of at most `kit_error_rate` (failed tool results divided by tool calls). A session on `main`, `master` or `HEAD`, or with no branch, has an unknown result. It is not a bad result. A session whose errors were never counted also has an unknown result.

The result has three lists:

- `findings`: comparisons where the server side is better by at least 5 %. Each one has a fixed habit text, such as use the server before you start. At most 3 are returned (`--limit`, up to 10).
- `other`: every other comparison that has enough sessions. It has the same numbers and no habit. A gap smaller than 5 % goes here.
- `not_enough`: servers that have fewer than 10 pooled sessions on either side, with their session counts.

A comparison needs at least 10 pooled sessions on each side. The text always says "linked to", never "caused": the numbers show a link in your history, not a cause. Nothing is stored: the result is worked out from the event store on every call. No LLM writes any text.

Set the limit for a good result under `thresholds:`:

```yaml
thresholds:
  kit_error_rate: 0.04
```

The default 0.04 is the median tool-error rate of the owner's sessions. The git check reads local branches only, never fetches, and stops after 2 seconds. Whatever it did not check in time is unknown.

The kit kernel's `wake` skill will read `kit_findings` at session start and add at most 3 lines from `findings`. That change belongs to the kernel repo (XR-2) and is not shipped yet. Until then, ask Claude to call `kit_findings`, or run the command.

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

You never run the launcher by hand. This is the command that Claude Code starts for a connector with the id `slack`:

```bash
easy-personal-launch exec slack
```

Each connector belongs to one workspace and has two names. Its **id** (for example `c1a2b3c4d`) is fixed: it names the keychain item and the launcher argument. Its **alias** is the name in Claude Code. The default alias is `<workspace>-<provider>`, for example `company-slack`. A workspace holds at most one connector per provider.

The catalog has five providers:

- Email: `gmail`, `sendgrid`
- Messaging: `slack`, `teams`, `discord`

```bash
easy-personal connector list
easy-personal connector add slack --workspace company
easy-personal connector add gmail --workspace company --alias work-mail
easy-personal connector sync
easy-personal connector sync --account 2 --yes
easy-personal connector move c1a2b3c4d side
easy-personal connector alias c1a2b3c4d team-chat
easy-personal connector alias c1a2b3c4d --default
easy-personal connector remove c1a2b3c4d
easy-personal doctor
```

- `connector list` shows the catalog by category, then your connectors (id, alias, provider, workspace). It never shows secret values.
- `connector add` asks for each field, shows what it will change, then asks you to confirm. Add `--yes` to skip the question. It writes the keychain item and stores the connector. It does not change Claude Code. `--workspace` is required. `--alias` sets the Claude Code name.
- `connector sync` shows the plan for each confirmed Claude account: which aliases it will add or remove. Then it asks you to confirm and runs `claude mcp add-json` or `claude mcp remove` with `--scope user` in that account. `--account` limits it to one account. It only removes aliases that it added itself. After `account ignore`, sync removes every alias it added to that account. If an MCP server with the same name already exists, sync refuses that step and changes nothing.
- `connector move` puts a connector in another workspace (`orphan` allowed). It is refused if the target already has that provider. A default alias follows the new workspace name; a custom alias stays. `connector alias` sets a custom alias, or `--default` resets it to `<workspace>-<provider>`. `workspace edit --rename` changes every default alias of that workspace; `workspace delete` moves its connectors to `orphan` and removes them from every account. Each of these shows the plan, asks you to confirm (`--yes` skips it) and then changes only the Claude Code entries (remove the old alias, add the new alias, same id). The keychain is never touched. A connector in `orphan` is active in no account; `connector move <id> <workspace>` assigns it again.
- `connector remove` removes the alias from every account, then deletes the keychain item and the connector. A connector from before workspaces (for example `slack`) is removed only if its Claude Code entry starts `easy-personal-launch`; any other name is refused and nothing is removed.
- `doctor` checks the keychain backend, the `claude` CLI and the connector entries. For each account it checks that every alias it added is still there and still starts the launcher. It reports drift and never removes anything. It exits 0 when all pass.

`connector sync`, `connector remove` and `doctor` need the `claude` CLI on `PATH`. `connector add` and `connector remove` need a keychain backend such as the Secret Service. Without `claude`, `sync` and `remove` write nothing and print the commands to run later. Restart Claude Code after a change.

## Agent accounts

`easy-personal` finds your agent config folders: `~/.claude*` that hold `projects/` or `settings.json`, and `~/.codex*` that hold `config.toml`. Files such as `~/.claude.json` are never accounts. A new folder starts as `detected`. `~/.claude` is confirmed on its own as "Claude default".

```bash
easy-personal account list --by provider
easy-personal account confirm ~/.claude-work --label "Claude work"
easy-personal account ignore ~/.codex
easy-personal account add /mnt/backup/claude-old
easy-personal account use 2 company personal
```

- `account list` finds new folders, then prints every account with its provider, label, folder and status. `--by provider` groups by provider; `--by account` (default) gives one flat list. A folder that is gone shows as `missing`; its row stays.
- `account confirm` names an account (1 to 60 characters). Run it again with a new label to rename.
- `account ignore` hides an account from re-detection. It refuses the default account.
- `account add` accepts a folder anywhere, only if it passes the same content check.
- `account use` sets the workspaces that a confirmed Claude account uses. Give no workspace to clear the list. Codex accounts and the `orphan` workspace are refused. Then run `connector sync` to write the change into that account. The default account `~/.claude` writes to `~/.claude.json`; every other account writes to `<folder>/.claude.json`.

`account ignore` refuses the default `~/.claude` account. The dashboard serves the same data at `/api/accounts`.

### Plan limits on the Agents page

Claude Code never saves your plan limits (5-hour and 7-day usage) to disk. They exist only in the JSON that Claude Code sends to your status-line script on stdin. To show them on the Agents page, add this one line to your status-line script. Put it after the line that reads stdin into `$input`:

```bash
f="${CLAUDE_CONFIG_DIR:-$HOME/.claude}/easy-personal-limits.json"; printf '%s' "$input" | jq -c 'select(.rate_limits) | {five_hour: (.rate_limits.five_hour | {used_percentage, resets_at}), seven_day: (.rate_limits.seven_day | {used_percentage, resets_at}), written_at: now}' > "$f.tmp" && [ -s "$f.tmp" ] && mv "$f.tmp" "$f"; rm -f "$f.tmp"
```

The line needs `jq`. It writes to a temporary file, then moves it, so the dashboard never reads a half-written file. `/api/accounts` returns the snapshot as `limits` in each account's `status_line`: per window `used_percentage`, `resets_at` (UTC) and `stale`, plus `written_at`. A window is `stale` once its reset time has passed. A missing or broken file gives `limits: null`. The dashboard only reads this file; it never changes your status line.

## Configuration

Settings live in `~/.easy-personal/config.yaml`. The file is optional. An environment variable wins over the file.

```yaml
ui:
  reports_dir: ~/path/to/easy-ui/reports
verifier:
  reports_dir: ~/path/to/easy-verifier-reports
thresholds:
  cache_hit_low: 0.4
  live_failing_streak: 3
  live_open_call_minutes: 10
  kit_error_rate: 0.04
```

- `ui: reports_dir:` is the easy-ui report folder. Without it, UI checks show as no data.
- `verifier: reports_dir:` is the easy-verifier report folder. The default is `~/.easy-verifier-reports`.
- `thresholds:` overrides a rule threshold by rule id. The rules are in `src/easy_personal/data/rules.yaml`. It also takes `live_failing_streak` and `live_open_call_minutes` (see Live view) and `kit_error_rate` (see Kit performance). A value that is not a number is refused.

| Variable | Meaning |
| -- | -- |
| `EASY_PERSONAL_HOME` | Data folder instead of `~/.easy-personal` |
| `EASY_PERSONAL_CLAUDE_PROJECTS` | Transcript folder instead of `~/.claude/projects` |
| `EASY_PERSONAL_UI_REPORTS` | Same as `ui: reports_dir:` |
| `EASY_PERSONAL_VERIFIER_REPORTS` | Same as `verifier: reports_dir:` |
| `EASY_PERSONAL_AGENT_ROOT` | Folder scanned for `.claude*` and `.codex*` accounts (default: your home folder) |
| `EASY_PERSONAL_CONTEXT_URL` | easy-context MCP URL. The default is `http://127.0.0.1:8767/mcp` |

## Upgrade and rollback

To upgrade, pull the new code, rebuild the page and reinstall. Always use `--reinstall --no-cache`. Without it, uv can reuse an old build of the same version.

```bash
git pull
npm --prefix web ci && npm --prefix web run build
uv tool install --reinstall --no-cache .
```

The first command that opens the store applies the pending migrations. Version 0.4.0 moves a store from version 7 to 8 (`m0008_tool_errors`). It adds the column `tool_error_count`, the number of failed tool results in each user record, which the kit performance comparison needs. The migration leaves `store.sqlite.bak-v7` beside the store. The earlier move from version 6 to 7 (0.3.0) left `store.sqlite.bak-v6`.

The next ingest re-reads every Claude Code transcript that is still on disk and fills in the error counts. It adds no row. On a large store this takes about 20 to 45 seconds, once. Error counts of deleted transcripts stay unknown, never 0, and the sessions that need them are not scored. The Live view and the Memory view need no config change.

Before an upgrade, back up the store so you can roll back:

```bash
cp -p ~/.easy-personal/store.sqlite ~/.easy-personal/store.sqlite.backup
```

To roll back, remove the connectors you added since the deploy and the timer, uninstall the tool and restore the store. The 0.3.0 program refuses a version 8 store ("newer than this program"), so restore the store first. Use the backup you made before the upgrade, or the `.bak-v7` file that the migration left, then install the 0.3.0 program:

```bash
easy-personal connector remove slack
easy-personal schedule remove
uv tool uninstall easy-personal-mcp
cp -p ~/.easy-personal/store.sqlite.backup ~/.easy-personal/store.sqlite
```

To restore from the migration's own copy instead, use `cp -p ~/.easy-personal/store.sqlite.bak-v7 ~/.easy-personal/store.sqlite`. Stop the dashboard first. Then install the 0.3.0 program from its tag. A store that went back to version 7 migrates again on the next 0.4.0 run.

The full deploy, health check and rollback steps, and the common failures, are in [`RUNBOOK.md`](RUNBOOK.md).

## Development

```bash
uv run pytest -q
uv run ruff check .
npm --prefix web test
```

Tests never touch your real `~/.claude/`, keychain or `~/.easy-personal/`. The tests use `EASY_PERSONAL_HOME` and fixtures.

## Easy Kit

Part of the Easy Kit family: kernel [easy-kit](https://github.com/thunderkds/easy-kit),
plug-ins [easy-ui-mcp](https://github.com/thunderkds/easy-ui-mcp),
[easy-verifier-mcp](https://github.com/thunderkds/easy-verifier-mcp),
[easy-context-mcp](https://github.com/thunderkds/easy-context-mcp).
