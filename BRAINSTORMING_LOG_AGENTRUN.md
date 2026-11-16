# BRAINSTORMING_LOG_AGENTRUN.md
**Generated**: 2026-11-16
**Task / Context**: Agent run — the app starts the owner's own Claude Code for a fixed job (PRD US-039, FR-101..FR-104, questions 35-37). Owner request 2026-11-16: "I want each time the app want using agent to analyze or do a job, let use claude subscribetion, not need for the users type the message in claude and call through MCP, application will do that all for user", then "yes, plan the run model, opt-in and off by default [admin]".
**Skill**: `brainstorming` (tier: Deep — it reverses part of an owner decision, PRD question 34, and it spends the owner's subscription). Run in `[admin]` auto mode: no question was put to the owner.

---

## The Problem Space

Today an `investigate` report needs the owner to open a Claude Code session and type the prompt (FR-075); that session calls the easy-personal MCP tools and ends with `save_report` (FR-076, FR-077). The owner wants the app to do this itself, billed to the Claude subscription, not to an API key.

Non-negotiable:
- Off by default, opt-in (owner, 2026-11-16).
- The subscription login is never handled by the app. Anthropic's legal page: "developers may not collect, store, or intermediate Claude.ai credentials or session tokens"; a user may sign in to "the unmodified Claude Code binary with their own Claude subscription". So the app may only start the installed `claude` program.
- NFR-002: no message text or tool output in a store row, log or page. The report text that `save_report` stores is the existing exception.
- NFR-004: the server runs no model. The `claude` program does the reasoning.
- NFR-008: the dashboard guard (Host, Origin, per-launch token) stands in front of every route.
- Rust app only: the Python app is being retired (`rust/PROJECT_KANBAN.md` T062, T063).

Facts checked in the repo (2026-11-16):
- `rust/crates/app/src/claude_cli.rs` already runs `claude` with an argument list, no shell, stdin closed, its own process group and a 60 s limit (`TIMEOUT`, line 23). It keeps 1 MiB of stdout for parsing. A run needs a longer limit and no kept output.
- `rust/crates/app/src/mcp.rs` holds the fixed prompt (`investigate_text`, line 752) and `save_report`.
- `web/src/SavedReports.jsx` shows saved reports on the Warnings & plan page.
- The installed `claude --help` lists `--mcp-config`, `--strict-mcp-config`, `--allowedTools`, `--tools`, `--permission-mode`, `--output-format`, `--setting-sources`, `--max-budget-usd`, `--bare`.
- Claude Code docs (read 2026-11-16): `claude -p` uses the `/login` subscription by default; "In non-interactive mode (`-p`), the key is always used when present" (`ANTHROPIC_API_KEY`); `--bare` "never reads OAuth credentials"; without `--bare`, `-p` runs a folder's hooks and `.mcp.json` "even in a folder you've never trusted".

---

## Questions for the User

None asked (`[admin]`). The decisions made in the owner's place are in **Auto decisions** at the end. One stays open for the owner: PRD question 37 (Anthropic's terms for a distributed app).

---

## Alternative Paths

| Option | Name | Summary | Invasiveness | Code Volume | Regression Risk | Recommended? |
|--------|------|---------|-------------|------------|----------------|--------------|
| A | Start the installed `claude -p` | The app starts the owner's `claude` program for one fixed job, with only its own MCP tools allowed; the report arrives through `save_report` | Medium | ~900 lines with tests | Medium | ✅ Yes |
| B | Agent SDK or Messages API inside the app | The app calls the model itself | High | ~1,500 lines | High | |
| C | Queue file picked up by a Claude session | The app writes a job file; a hook or a scheduled `claude` run of the owner's picks it up | Medium | ~600 lines | Medium | |

### Option A — Start the installed `claude -p`
**Approach**: A CLI switch (`agent-run on|off|status`) writes one flag file. When on, a Run button (or `agent-run now`) starts `claude -p "<investigate text>"` with `--mcp-config` naming only this binary's `mcp` command, `--strict-mcp-config`, an allow-list of the five tools the prompt names, no built-in tool, an empty working folder the app owns, and the billing variables removed. One run at a time, 10 minute limit, output dropped. The dashboard reads the outcome and reloads Saved reports.
**Pros**: Uses the subscription with no credential handling. Reuses `claude_cli.rs`, the fixed prompt and `save_report`: the report path is already tested. Nothing new is stored.
**Cons**: Needs Claude Code installed and logged in. Spends plan limits without a session on screen. The terms are not explicit for a distributed app (question 37).
**Why it might fail**: (1) A set `ANTHROPIC_API_KEY` bills the API silently — the variable list must be complete and tested. (2) User-level hooks and user MCP servers of `~/.claude` still load unless the flags shut them out; a tool outside the allow-list must be refused, not asked for (no one answers a prompt in `-p`). (3) A flag name differs between Claude Code versions: the run fails closed, and `claude_found` alone does not prove it. The first task starts with a spike on the real binary. (4) The model does not call `save_report`: the outcome is `ok` with no report id, and the page must say so.

### Option B — Agent SDK or Messages API inside the app
**Approach**: Link an SDK, hold the prompt loop in the app, call the model.
**Pros**: No dependency on an installed CLI; full control of tools and cost.
**Cons**: Needs an API key: "Unless previously approved, Anthropic does not allow third party developers to offer claude.ai login or rate limits for their products, including agents built on the Claude Agent SDK." It is not what the owner asked for (subscription). It breaks NFR-004 and adds a network client and a key store.
**Why it might fail**: The owner pays twice (subscription and API); a key in the app is a new secret to guard.

### Option C — Queue file picked up by a Claude session
**Approach**: The app writes `investigate` into a queue file. A `SessionStart` hook or a cron line of the owner's runs it.
**Pros**: The app starts no process. The owner's own automation spends the subscription.
**Cons**: Still needs the owner to open a session, or to install a cron line or hook: the app does not "do that all for user". Two moving parts; a stale queue runs late.
**Why it might fail**: The job runs hours later against other numbers, or never; the hook edits the owner's Claude settings, which the kit avoids.

---

## 50% Rule Check

Option A at half the code: no dashboard at all — `easy-personal-rs agent-run now` from the CLI or a cron line, and the report shows up in the existing Saved reports list. That is the first task (T090) and it is demoable alone. The API and the button (T091, T092) are what the owner asked for ("application will do that all for user"), so they stay, as later slices.

---

## Recommended Path

**Option A — Start the installed `claude -p`**

It is the only path that uses the subscription without the app touching the login, it reuses three tested parts, and every step can be undone (`agent-run off`; delete the flag file). The vital slice is one job, one run at a time, started by the owner's click.

**Cut list (not built)**: a schedule or automatic runs; more jobs than `investigate`; a free-text prompt; a dashboard switch for the setting; a run history in the store; showing progress text; cost or token numbers of the run itself (the next ingest counts the run's session like any other); macOS and Windows; the Python app.

---

## Surgical Scope

Files that **should** be touched:
- `rust/crates/app/src/agent_run.rs` (new) — setting, lock, runner, outcome
- `rust/crates/app/src/claude_cli.rs` — share `which` and the process-group kill; no change to the 60 s path
- `rust/crates/app/src/main.rs`, `mcp.rs` (expose the prompt text), `api.rs`, `server.rs`, `api_tests.rs`
- `rust/parity/leak_rust.py` — the two write routes and the read route
- `web/src/AgentRun.jsx` (new), `web/src/WarningsPlanView.jsx`, `web/src/app.css`, tests
- `rust/README.md`, `README.md` — how to turn it on, what it spends

Files that **must not** be touched:
- `rust/crates/core/src/schema_v11.sql` and the store — no schema change
- `rust/crates/core/src/reports.rs` — `save_report` rules stay
- `web/src/CopyCommand.jsx` — FR-092 stays
- `src/easy_personal/**` — the Python app
- any file under `~/.claude*`, `~/.codex*`, `~/.easy-personal`

---

## Edge Case Checklist for TASK_GUIDE

- [ ] `ANTHROPIC_API_KEY` (and the others) set in the app's environment: the child never sees them
- [ ] `claude` not on `PATH`, or not logged in: outcome says so, no hang
- [ ] The setting is turned off while a run is going: the run is stopped
- [ ] Two starts at once (two dashboards, or CLI and dashboard): one runs, the other gets 409 / "a run is going"
- [ ] The app is killed during a run: the child's process group dies or is cleaned at the next start; a stale lock does not block for ever
- [ ] The run asks for a tool outside the allow-list: refused, the run goes on or ends; never a wait for an answer
- [ ] The model never calls `save_report`: outcome `ok`, no report id, the page says "no report was saved"
- [ ] The run saves 2 reports: the page shows the newest
- [ ] Planted secret in a fake `claude`'s stdout and stderr: in no response, log, store row or page
- [ ] A workspace or account name with a quote, a newline or a leading `-`: passed as data inside the prompt text, one argument, no shell
- [ ] Plan limit reached: `claude` exits non-zero; outcome `failed`, no retry
- [ ] 10 minute limit: the whole group is killed; outcome `timed out`

---

## Next Actions

1. T090 (backend): spike on the real `claude`, the setting, the runner, `agent-run on|off|status|now`.
2. T091 (backend): the three routes, guard and leak gate.
3. T092 (frontend): the agent run block on Warnings & plan, README text.
4. Owner: PRD question 37 before the feature is named on the landing page.

---

## User Selection

> **Approved direction**: Option A — Start the installed `claude -p`
> Taken by the Supervisor on 2026-11-16 in `[admin]` auto mode (`docs/claude-md/tracks.md` row 4), not approved by the owner. The owner's own words fix the frame: "yes, plan the run model, opt-in and off by default".

## Auto decisions (planning run, 2026-11-16)
Mode: auto

1. Track extended, Risk High — RULE: `AGENTS.md` `## Tracks` (a UI component; Risk Medium or High).
2. Stage 0 step Requirements: answered from the request and the repo, no question asked. PRD US-039, FR-101..FR-104, questions 35-37 written; `Approved by` not written — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` row 1.
3. Rust app only — FACT: `rust/PROJECT_KANBAN.md:12` "**T063** — Retire Python 6: remove the Python app".
4. One job, `investigate` — FACT: `PRD.md:168` "The MCP server must offer a prompt `investigate`"; it is the only agent job the product defines. More jobs: cut list.
5. The setting is changed only by the CLI; the dashboard shows it — DEBATED: CLI only (Advocate 1: CLI only, dashboard read-only plus Run; Advocate 2: CLI only, an off-only browser switch may come later; Advocate 3: CLI only, `agent-run on|off|status`) ⚠ decided for you — please review.
6. Step Grilling, PRD gate: PASS — 4 FRs validated (each traces to US-039), 1 open question with an owner (37), 0 items flagged. RULE: row 2.
7. Direction: Option A; B and C listed above — RULE: row 4 ⚠ decided for you — please review (taste).
8. Stage 1: nothing new to set up; the base team; `settings-setup`, `companion-setup`, `fewer-permission-prompts`, `update-config` not run — RULE: rows 5 and 11.
9. Breakdown into T090, T091, T092 with their labels — RULE: row 12.
10. Requirement Fidelity Gate of the three guides signed `auto (Supervisor)` — RULE: row 13.

Checks: none — a plan; no code changed, no test run.
Memory: nothing new (the decision is recorded in `PRD.md` questions 35-37 and this log; the memory pass runs at the first merge).
Not done: commit (yours); implementation of T090-T092 (not asked: the request was "plan"); `thinking-report` HTML page; PRD question 37 (yours); readiness `Approved by` for this feature (yours).
