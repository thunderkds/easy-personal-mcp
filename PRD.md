# PRD — easy-personal-mcp
**Last updated**: 2026-11-02
**Status**: Approved (Phase 0 recheck + requirement grill, 2026-10-21)
**Owner**: thunderkds

> **Scope of this document**: *What* to build and *why* — product intent, user stories, requirements, success metrics.
> Technical decisions, architecture, agent config, and task state live in `PROJECT_SPEC.md`.
> If Out of Scope here conflicts with Critical Constraints there, resolve the conflict before Stage 2.
> Source of intent: Phase 0 recheck answers (2026-10-21). Supersedes the earlier draft built from
> `BRAINSTORMING_LOG.md` Option B; that log's 50% cut (one source, static dashboard, connectors in v1.1)
> is reversed by owner decision — v1 now carries all of it, delivered in internal milestones M1–M4.

---

## Overview

An individual who works with coding agents every day has no clear picture of *how* they work: which
projects eat the budget, which sessions run away, whether caching works, whether habits are getting
better or worse — and no single view of what their Easy Kit tools (kernel, easy-verifier, easy-context,
easy-ui) actually did for each project. Claude Code deletes transcripts after 30 days, so history is
lost. Personal tool connectors (email, chat, task tools) are configured by hand with secrets pasted
into plaintext config files.

`easy-personal-mcp` is the Easy Kit plug-in whose subject is the person, not a repo. It reads the
person's local agent and kit activity, turns it into deterministic metrics, warnings and an improvement
plan, and shows them on a live local dashboard and through MCP tools. It also manages personal tool
connectors from a provider catalog for Claude Code, keeping every secret in the OS keychain and
injecting it only at launch. Nothing leaves the person's machine.

---

## Personas

| ID | Name | Role | Pain Point |
|----|------|------|-----------|
| P1 | Solo Builder | One person (the owner) running Claude Code with the Easy Kit across many repos on one Linux machine | Cannot see spend, runaway sessions, habits or kit results across projects; transcripts vanish after 30 days; connector secrets sit in plaintext config. |
| P2 | Supervisor Agent | The Easy Kit Supervisor or any agent session | Has no factual view of the person's recent usage or kit results when asked "how am I doing / what should I change"; would otherwise guess. |

---

## User Stories

Milestones are internal delivery order inside the single v1 release.

| ID | Story | Persona | Milestone |
|----|-------|---------|-----------|
| US-001 | As P1, I want my Claude Code usage (sessions, tokens, cache, estimated cost) summarised by day, model and project, so I know where my agent time and money go. | P1 | M1 |
| US-002 | As P1, I want usage history kept beyond Claude Code's 30-day transcript cleanup, so trends survive. | P1 | M1 |
| US-003 | As P1, I want warnings labelled *improve / aware / warning*, each citing the rule and the measured value, so I trust why it fired. | P1 | M1 |
| US-004 | As P1, I want an improvement plan ordered by priority with a concrete fix per item, so I know what to change first. | P1 | M1 |
| US-005 | As P2, I want MCP tools that return the same numbers as the dashboard, so I can answer the person with facts. | P2 | M1 |
| US-006 | As P1, I want my projects found automatically from the folders Claude Code has worked in, so I set nothing up per repo. | P1 | M2 |
| US-007 | As P1, I want to see per project what the Easy Kit did — kernel traces, verifier scores, easy-context index freshness, local git activity — plus my easy-ui checks overall, next to my usage, so I see output, not just spend. | P1 | M2 |
| US-008 | As P1, I want to pick a category (Email, Messaging…) and a provider (Gmail, SendGrid, Slack, Teams, Discord…) and get the right config form, so connecting a tool is guided. | P1 | M3 |
| US-009 | As P1, I want connector secrets kept in my OS keychain and never written to any config file, so a leaked dotfile leaks nothing. | P1 | M3 |
| US-010 | As P1, I want a connector set up once to be registered in Claude Code for me, so I don't edit config by hand. | P1 | M3 |
| US-011 | As P1, I want connector suggestions based on what I actually do in sessions, so I add the tools that would help. | P1 | M3 |
| US-012 | As P1, I want a local dashboard with charts, flows, warnings, the plan, kit results and connector forms, so I review my week at a glance and act on it. | P1 | M4 |
| US-013 | As P1, I want life sources (calendar, finance, health) through the same source interface. | P1 | v2 |
| US-014 | As P1, I want to group my connectors into workspaces (personal, company, side…) created from an Add button, each with its email and repos, so work and personal tools never mix. | P1 | Workspaces |
| US-015 | As P1, I want my agent accounts (Claude personal, Claude company, Codex…) found for me and to pick which workspaces each Claude account uses, so each account sees only its own tools. | P1 | Workspaces |
| US-016 | As P1, I want to filter my usage by workspace, so I see what each part of my life costs. | P1 | Workspaces |
| US-017 | As P1, I want one read-only board of the tasks in all my kit projects, grouped by status, so I see what is in flight, waiting for review or blocked without opening each repo. | P1 | Work view |
| US-018 | As P1, I want my usage split into work I typed and work agents ran (sub-agents, headless runs), so I see how much runs without me. | P1 | Work view |
| US-019 | As P1, I want to see my agent-related scheduled jobs with next run, last run and last result, so I know my background work is alive. | P1 | Work view |
| US-020 | As P1, I want each project's memory notes drawn as a graph of names and links, so I see what my agents remember and what is orphaned. | P1 | Work view |
| US-021 | As P1, I want the dashboard views grouped into sections and a first-run checklist, so I find views fast and know what setup is still missing. | P1 | Work view |
| US-022 | As P1, I want a Live view that shows my active Claude Code sessions, a feed of their tool calls as they happen and a running token and cost total, so I can watch my agents work without waiting for the next ingest. | P1 | Live trace (v0.3.0) |
| US-023 | As P1, I want the Live view to mark a session that keeps failing, waits for my answer, or has a tool call that has not returned for a long time, so I know which of my sessions needs me without watching each feed. | P1 | Live attention (v0.4.0) |
| US-024 | As P1, I want the memory graph to also show what easy-memory saved for each project (which files its notes cite, which of those changed since, how often notes were injected), so I see what the auto-memory knows and whether it is going stale, without reading any note text. | P1 | easy-memory layer |

---

## Functional Requirements

| ID | Requirement | Traces to |
|----|-------------|-----------|
| FR-001 | Must ingest `~/.claude/projects/**/*.jsonl` incrementally (file + byte offset), idempotently, into a local store under `~/.easy-personal/`. | US-001, US-002 |
| FR-002 | Must extract only metadata: timestamps, session id, project path, branch, model, record type, tool names, `message.usage` token fields. Message and tool content must never be stored. | US-001, NFR-002 |
| FR-003 | Records of an unknown type or shape must be counted as *abstained* per source version and shown, never coerced into a metric. Applies to every source. | US-001, US-003, US-007 |
| FR-004 | Must compute usage summaries and breakdowns by day, model, project and tool for a requested period. Tokens are the primary unit. "Day" is the OS local timezone; all times are stored in UTC. Every surface shows the project folder name only; the store keeps the full path. | US-001 |
| FR-005 | Cost must come from a dated local price table and be labelled "API-equivalent cost" everywhere (it is not what a subscription pays); a model missing from the table makes cost abstain for that model. | US-001 |
| FR-006 | Warnings must come from declared rules (id, threshold, label `improve`/`aware`/`warning`, fix line) over measured facts; each hit cites rule id and measured value. Thresholds live in config. | US-003 |
| FR-007 | The improvement plan must list unmet rules grouped by priority, each with its static fix line. | US-004 |
| FR-008 | MCP, CLI and dashboard must share one core. MCP usage tools: `usage_summary`, `usage_breakdown`, `session_outliers`, `warnings`, `improvement_plan`. | US-005 |
| FR-009 | Must assign each session to a project at ingest: the git top-level of its `cwd` (worktrees grouped under the main repo, subfolders under their repo), else the folder itself; the result is stored, so a deleted folder keeps its history, marked *missing on disk*. Kit sources read every project that still exists. | US-006 |
| FR-010 | `kit_trace` source must read kernel trace files (`<repo>/memory/event-trace/*.jsonl`) and keep only `timestamp`, `tool_name`, `is_error` and the task id from the file name (`_untagged` → no task). The `summary` field holds raw tool input and must never be read into the store. | US-007, NFR-002 |
| FR-011 | `verifier` source must read only the score JSON that easy-verifier writes next to each HTML report (`<repo>/reports/` or `~/.easy-verifier-reports/<repo>/`). HTML-only reports count as "no data". Depends on XR-1. | US-007 |
| FR-012 | `context` source must act as an MCP client of the easy-context service on `127.0.0.1` (`list_indexes`, `get_stats`; read-only tools only) and record per repo `last_index_at` and counts. When the service is down, the source reports "no data" and never fails or guesses. easy-personal must never call `index_path`. | US-007, NFR-001 |
| FR-013 | `ui` source must read easy-ui `session-*.json` files from easy-ui's report folder (path configurable) and keep only `id`, `status`, `startedAt`, `endedAt` and the action count — shown for the owner overall, not per project. The caller-authored `target` field must never be stored. | US-007, NFR-002 |
| FR-014 | `git_local` source must read local commit counts per repo from local git only (no remote, no hosting API). | US-007 |
| FR-015 | MCP must expose `kit_summary` (per project for kernel traces, verifier, context freshness and git; overall for easy-ui; "no data" shown explicitly). | US-005, US-007 |
| FR-016 | Provider catalog as declarative files: category, provider, fields, which fields are secret, how the provider server is launched, whether an official claude.ai connector exists. | US-008 |
| FR-017 | Secrets must be stored only via the OS keychain (Linux Secret Service); with no keychain backend the setup must refuse with guidance, never fall back to plaintext. | US-009 |
| FR-018 | `easy-personal-launch exec <connector>` must read secrets from the keychain and exec the provider server with them in its environment; `easy-personal-launch headers <connector>` must print JSON headers for HTTP servers. | US-009, US-010 |
| FR-019 | Connectors must be registered in Claude Code through `claude mcp add-json --scope user` pointing at `easy-personal-launch exec <connector>` (the separate launcher entry point, so start-up stays under NFR-006's 200 ms); when the `claude` CLI is missing, show the snippet instead. Project-scope files are never written. | US-010 |
| FR-020 | When a provider has an official claude.ai connector, the dashboard must offer the link as the first option. | US-008 |
| FR-021 | No MCP tool may create or modify a connector or return a secret value; MCP exposes only `connector_catalog`, `connector_status` (configured yes/no), `connector_suggestions`. | US-009 |
| FR-022 | Suggestions must come from counters over metadata only — tool names, Bash command names (first word), WebFetch/MCP target domains — never from message text. | US-011 |
| FR-023 | `easy-personal dashboard` must serve the dashboard on `127.0.0.1` showing usage charts, the session timeline (per session: start→end, model, token use over time, gaps), warnings, the plan, kit results and connector forms. It must state that cloud sessions and other machines are not included; connector forms are the only write actions and run only from this local page. | US-012, US-008 |
| FR-024 | Ingest must be schedulable on Linux (cron or systemd user timer snippet) so the 30-day transcript cleanup loses nothing. | US-002 |
| FR-025 | A declared rule must mark a repo's easy-context index *stale* when its `last_index_at` is older than the repo's latest local commit, with the fix line "call `index_path`". | US-003, US-007 |
| FR-026 | The v1 rule pack must contain exactly these rules (defaults shown): `cache_hit_low` (warning, cache-hit ratio < 50% over 7 days); `session_huge_no_compact` (improve, median session > 150k tokens without `/compact`); `tokens_jump` (aware, week-over-week tokens + 50%); `project_dominates` (aware, one project > 60% of weekly tokens); `tool_errors_high` (warning, > 10% of kernel-trace tool calls with `is_error` over 7 days); `context_index_stale` (FR-025); `records_unreadable` (aware, > 2% abstained per source version). | US-003, US-004 |
| FR-027 | History must be kept forever by default; a `retention_days` setting prunes older rows. Every stored row carries a `machine_id`. | US-002 |
| FR-028 | Workspaces must be created, listed, edited (including rename) and deleted from the dashboard (Add button) and the CLI (`easy-personal workspace add|list|edit|delete`). Each has a fixed internal id; fields: name (slug `[a-z0-9][a-z0-9-]*`, unique, `orphan` reserved), color/icon, description, main email, repo paths. Email is information only. A repo path may belong to several workspaces. | US-014 |
| FR-029 | Every connector has a fixed id (keychain service `easy-personal/<id>`, launcher `easy-personal-launch exec <id>`) and an alias — the name registered in Claude Code. The alias defaults to `<workspace>-<provider>` and follows workspace renames and moves; an owner-set alias is kept as typed. Aliases are unique among non-orphan connectors. Every connector belongs to exactly one workspace; a workspace holds at most one connector per provider, except `orphan`. | US-014 |
| FR-030 | Deleting a workspace must move its connectors to the built-in `orphan` workspace (never deletable or renamable), keeping id, alias and keychain item, and remove them from every agent account. Connectors are deleted only one at a time. | US-014 |
| FR-031 | Moving a connector, renaming its workspace or changing its alias must never touch the keychain: only the Claude Code entries change (remove old alias, add new alias pointing at the same id) in every affected agent account. A move is refused if the target already has that provider. Orphan is active in no agent account. | US-014, US-015 |
| FR-032 | Agent accounts must be auto-detected from `~/.claude*` and `~/.codex*` folders (plus folders added by hand) and stay *detected* until the owner confirms and names them. The Agents view groups them by provider or by account. | US-015 |
| FR-033 | A confirmed Claude agent account picks zero or more workspaces (never `orphan`); their connectors are written or removed only via `claude mcp add-json|remove --scope user` run with `CLAUDE_CONFIG_DIR` set to that account's folder (FR-019, DDR-0001). Codex accounts are listed only. Every write shows its plan and needs confirmation. | US-015 |
| FR-034 | On first run with workspaces, pre-workspace connectors (`slack`, `gmail`, …) must be placed in `orphan` with their current name as both id and alias (keychain item untouched), and — after one confirmation — removed from the Claude config they were registered in. | US-014 |
| FR-035 | Dashboard usage views and the CLI usage commands (`--workspace <name>`) must filter to events whose project path equals or sits under one of the workspace's repo paths. | US-016 |
| FR-036 | Ingest must read the `projects/` folder of every confirmed Claude agent account; every stored event carries its agent account; dashboard and CLI usage can filter by agent account. | US-016, US-015 |
| FR-037 | The `kanban` source must read each kit project's `PROJECT_KANBAN.md` (`## Board` subsections `Todo`, `In Progress`, `Ready for Review`, `Done`, `Closed (will not do)`, plus the `## Blocked` table) live on request, keeping only task id, title, status and project. When `<repo>/.claude/board.json` names a `board`, tasks are tagged with that board kind and the file's mtime is shown as "last board pull" — the Linear side comes from the kit's own pull into this file; easy-personal never calls the board's API. A missing or unparsable file is "no data" for that project. Nothing is written. | US-017, NFR-001, NFR-003 |
| FR-038 | The Work view must show the FR-037 tasks as columns named by those statuses (the kit's terms, not new ones), filterable by project and workspace, with a count per column and the "last board pull" age per project. | US-017, FR-035 |
| FR-039 | Ingest must store each session's `entrypoint` (new migration; the old rows get it by re-reading their source files from offset 0). A record is *agent* work when `is_sidechain` is true or its session's entrypoint starts with `sdk` (headless: `sdk-cli`, SDK); any other entrypoint (`cli`, `claude-desktop`, IDE, …) is *you* (owner, 2026-10-31, after real data showed `claude-desktop`). Usage views and `usage_summary`/`usage_breakdown` add this split per day; an unknown entrypoint is abstained per FR-003, never guessed. | US-018, FR-003, NFR-009 |
| FR-040 | The `schedules` source must list systemd user timers (`systemctl --user list-timers --all` + the unit's `ExecStart`) and the owner's crontab lines whose command's first word is `claude`, `easy-personal`, `easy-personal-launch` or a kit tool on a declared allow-list; it shows name, schedule, next run, last run and last exit code. Only the command's first word is kept, never its arguments. It never creates, edits, starts or stops a job. | US-019, NFR-002, NFR-003 |
| FR-041 | The memory graph must read, live on request and never into the store, `<repo>/memory/**/*.md` of every kit project and `~/.claude*/projects/*/memory/*.md` of every confirmed agent account, grouped by project. Nodes = file name plus frontmatter `name` and `description`; edges = `[[name]]` links resolved by `name` slug; an unresolved link is drawn as a dangling node. Note bodies are never sent to the page; a click shows the path only. Node colour = kind (kit / Claude). | US-020, NFR-002, NFR-003 |
| FR-042 | The dashboard rail must group views into sections: Work (board, scheduled, memory graph), Usage (usage, timeline, warnings and plan), Kit, Connections (connectors, workspaces, agent accounts), System. A first-run checklist must show, each with done/not done and the existing command or button: first ingest, ingest timer installed, agent accounts confirmed, first workspace. It adds no new write action. | US-021, FR-023 |
| FR-043 | While `easy-personal dashboard` runs, a live tail must follow every confirmed agent account's `projects/` tree (main and `subagents/` files), polling about every 100 ms, reading only complete lines from stored byte offsets with the existing `claude_code` parser, and write new events through the normal store path. Nothing is written to any Claude config; source files stay read-only (NFR-003). It must not hold the single-writer lock between writes, so the scheduled ingest still runs. | US-022, FR-002, NFR-003 |
| FR-044 | `GET /api/live` must stream new metadata events to the page as Server-Sent Events (session id, project, account, time, kind, tool name, token counts, API-equivalent cost), under the same Host/Origin/token guard as other `/api/*` routes (NFR-008), resuming after a reconnect without duplicates. A session is *active* when it wrote a record in the last 5 minutes, or when it has an *open call* (FR-047) — amended 2026-11-02 (owner, grill) so a stuck or waiting session stays listed. | US-022, US-023, NFR-002, NFR-008 |
| FR-045 | The Live view must show active sessions grouped by account and project, a per-session feed of tool calls (newest first, bounded length), and a running token and API-equivalent cost total per session and overall; it never shows message or tool content. | US-022, FR-005 |
| FR-046 | The `claude_code` source must recognise the record types first seen in Claude Code 2.1.316 (`api-request`, `api-request-blob`, `api-request-shape`, `tools-used`, `ai-title`, `mode`, `permission-mode`, `bridge-session`) as metadata types, keeping only type, session and time, so they are no longer counted as abstained. | FR-002, FR-003 |
| FR-047 | The live tail must pair each `tool_use` block with its `tool_result` by the block ids (`id` / `tool_use_id`), so a result carries its tool name and a call with no result yet is an *open call*. The ids are held in memory only, never stored or sent to the page; no other block field is read (FR-002). Pairing and streaks are kept per *stream* (one transcript file: the main thread or one subagent), because subagent records share their parent's session id. | US-023, FR-002, FR-043 |
| FR-048 | A live session is *failing* while one of its streams has ended its last `live_failing_streak` (default 3) tool results in a row with `is_error`, any tool; the next successful result in that stream clears it. A denied permission counts as a failure (the transcript holds no other signal). | US-023, FR-006 |
| FR-049 | A live session is *waiting for you* while it has an open `AskUserQuestion` or `ExitPlanMode` call (shown at once), and *needs attention* while any other open call is older than `live_open_call_minutes` (default 10) **and** the whole session (main thread and subagents) wrote no record in that time, worded "hung or waiting for permission" (quiet rule: owner, 2026-11-02, so a long foreground subagent is not flagged). An open call also closes when a later assistant message appears in its stream (a result the tail skipped must not stay open). Both thresholds live in `~/.easy-personal/config.yaml` under `thresholds:`, and the threshold reader must accept them beside the rule ids. An open call keeps its session in memory for at most 24 h, then the call is dropped. | US-023, FR-006, FR-044 |
| FR-050 | The Live view must show each session's states (*failing*, *waiting for you*, *needs attention*) as a badge naming the tool and, for a subagent stream, that it is a subagent, plus a count of sessions with any state in the Live header. All states, time-based ones too, are worked out in the live tail; the page only renders them. No other surface (MCP, CLI, notifications) shows them in this version. | US-023, FR-045 |
| FR-051 | The memory graph must read the easy-memory store (`$EASY_MEMORY_HOME`, else `~/.easy-memory/`) live on request, never into the store and never through the easy-memory MCP. A store folder belongs to a project when its `meta.json` `path`, without a trailing `/.git`, equals the stored project path; a folder with no `meta.json` belongs to the project whose easy-memory key (first 16 hex characters of the SHA-256 of the project's git common dir, else its real path) equals the folder name. From each note (`notes/*.json`, format `v` 1) only `time` and the `files` map (relative path → SHA-256) are kept; `prompt`, `answer` and `session_id` are never sent to the page. From `prompts.log` only `time`, `effective` and `injected` are kept. `trash/` is never read. A corrupt or newer-version note is abstained and counted. | US-024, NFR-002, NFR-003 |
| FR-052 | The memory graph must draw, per project, one *cited-file* node for each of the 30 files most cited by easy-memory notes (ties by path), sized by citation count. A cited-file node is *stale* when the file's current SHA-256 differs from the hash in the newest note that cites it, or the file is gone; a cited path that resolves outside the project folder is never opened and counts as not drawn. A cited file that is also a kit or Claude memory node is shown as that node, with the count and stale mark added, not as a second node. Node colour = a third kind (easy-memory). A click shows path, citation count, newest citing note's date and stale state only. | US-024, FR-041 |
| FR-053 | The Memory view must show a panel per project: notes saved per day for the last 30 days (sparkline + total), injection hit rate for the same 30 days (prompts with `injected` true ÷ prompts with `effective` on, with the prompt count), the number of cited files not drawn, and abstained notes. The project list must include projects that have only easy-memory notes and show an easy-memory note count beside the kit and Claude counts. | US-024, FR-041 |

---

## Non-Functional Requirements

| ID | Requirement | Category |
|----|-------------|----------|
| NFR-001 | No traffic leaves the machine at runtime: easy-personal makes no off-machine calls; local calls go to `127.0.0.1` only; HTTP surfaces bind `127.0.0.1` only. (Provider servers launched by `exec` reach their own services — that is their job, not easy-personal's.) | Security |
| NFR-002 | No secret, message text or tool output appears in any tool response, dashboard, log or store row (fixtures with planted secrets prove it). | Security / Privacy |
| NFR-003 | Source files are read-only; nothing under `~/.claude/projects/` or any repo is modified. | Safety |
| NFR-004 | Deterministic: the same inputs give the same numbers; no LLM inside the server. | Correctness |
| NFR-005 | Cold ingest of all confirmed agent accounts (≈ 960 MB on the owner's machine, 2026-10-26) completes in < 2 min, one time; 30 days of one account (≈ 10 MB, HUN-32) in < 10 s; a re-run with no changed files completes in < 2 s; ingest streams line by line with bounded memory. 500 MB stays as a stress case, not a target. | Performance |
| NFR-006 | `exec` launcher adds < 200 ms to connector start and has no third-party imports beyond `keyring`. | Performance / Reliability |
| NFR-007 | Linux only; Python 3.11+; native install (`uv tool install`); Docker optional and analytics-only (DDR-0001). | Portability |
| NFR-008 | The live dashboard must reject any write request that lacks the per-launch token or whose `Host`/`Origin` is not the dashboard's own `127.0.0.1` address (defends against other browser tabs and DNS rebinding). | Security |
| NFR-009 | Store scales with all agent accounts and forever history: schema changes go through ordered, versioned migrations (`PRAGMA user_version`), each in one transaction and tested from the previous released schema; `foreign_keys=ON` and a `busy_timeout` on every connection; workspace rules (unique names, one provider per workspace except `orphan`) enforced by constraints, not only code; usage queries filter by time before grouping and are covered by indexes, checked with `EXPLAIN QUERY PLAN` in tests. Target: workspace-filtered 30-day usage over 1,000,000 events returns in < 500 ms (owner delegated the number, 2026-10-26). | Performance / Reliability |
| NFR-010 | Live Work view reads (FR-037 board, FR-041 memory graph with the FR-051 easy-memory layer) for 50 projects return in < 1 s on the owner's machine; add an in-memory cache only if a measured read misses this. | Performance |
| NFR-011 | Live view latency: a tool call appears on the page within 2 s of its transcript record for at least 95% of events in the standard test session (spike baseline: 95.7%, median 214 ms, HUN-170); the tail adds no measurable wall-clock to the session. | Performance |
| NFR-012 | Live attention states add no new polling loop, thread or SSE route: they come from the existing tail and stream. A *failing* or *waiting for you* state reaches the page within the NFR-011 2 s bound; *needs attention* within 2 s after its threshold (worked out in the tail's existing poll loop). The stream re-sends state only when a session's set of states changes, never for an ordinary tool call. Memory stays bounded (per-stream open-call map, dropped with the session or after 24 h). | Performance |

---

## Success Metrics / KPIs

| Metric | Baseline | Target | How measured |
|--------|----------|--------|--------------|
| Usage history retained | 30 days (Claude Code default) | ≥ 180 days | Oldest day in store |
| Unknown-record share | unknown | < 2% of records per source version | `abstained` count / total |
| Secrets in plaintext agent config | today: some | 0 | Scan of `~/.claude.json` for connector env values |
| Kit coverage | none | Kernel traces, verifier (JSON era) and context freshness show data for every discovered project that uses them; easy-ui shows overall | `kit_summary` vs. presence of that part's files/index per project |
| Plan effectiveness | first 4 weeks | Rule hits per week fall over 4 weeks | `warnings` count per ISO week |
| Weekly review time | n/a | < 5 min | Self-reported |

---

## Out of Scope

- Calling provider APIs directly (no SendGrid/Slack client in personal-mcp) — future brainstorm.
- Creating claude.ai connectors (not possible from a local app; link only).
- Writing project-scope agent config or hooks (owned by the kernel's `setup.sh`); installing or configuring other Easy Kit plug-ins.
- Writing connector config into Codex or Cursor (v1 targets Claude Code only); reading Codex or Cursor activity.
- Cloud sessions (claude.ai/code) and other machines — no local transcript; merging is possible later via `machine_id`.
- Starting easy-context indexing or easy-ui checks at runtime.
- Calling kernel skills from easy-personal (it stays a standalone plug-in the Supervisor calls).
- macOS and Windows.
- Sharing or publishing reports; multi-user.
- LLM-written narratives or "AI judgment" in reports.
- Life sources before v2.
- Workspaces: writing connectors into Codex accounts (listed only, DDR-0001); new providers (GitHub, Linear/Jira, Google Calendar, Notion); MCP tools that create or change workspaces (FR-021 stays read-only).
- Work view: creating, moving or editing board tasks; calling Linear or any board API; starting, stopping or editing scheduled jobs or agents (the board and jobs views are read-only, unlike the reference Javis OS); storing or showing memory note bodies; /loop jobs (they live only inside a running session) and claude.ai cloud routines.
- Live attention (v0.4.0): "same tool N times in a row" and tool-name loop rules (measured as noise on the owner's history, 2026-11-02: 292 of 550 streams have an 8+ same-tool run, mostly normal Bash work; 2-tool loops hit 11 streams, all planned batches); attention states in MCP, CLI or browser notifications (cut list, later); telling a hung command from a permission prompt; reading error text or tool input; hooks or OpenTelemetry (reference: github.com/agentblazo/blazo, ideas only).
- easy-memory layer: showing note prompt or answer text, session ids or trashed notes; one node per note; calling the easy-memory MCP; forgetting, restoring or editing notes; switching easy-memory on or off; a page date range for the panel (fixed 30 days).

---

## Open Questions / Assumptions

| # | Question / Assumption | Owner | Due |
|---|----------------------|-------|-----|
| 1 | Assumed: kit integration is read-only — easy-personal never installs/configures other plug-ins or touches the kernel's `setup.sh`. | thunderkds | confirmed 2026-10-21 |
| 2 | Resolved (grill 2026-10-21): verifier writes HTML only → XR-1; easy-ui writes `session-*.json` into its own `reports/`, with no project field → shown overall; easy-context serves MCP on `127.0.0.1:8767` and needs an `X-Easy-Context-Root` header. | Supervisor | done |
| 3 | Default: a source with no data reports "no data" (never fails, never guesses). | Supervisor | revisit Stage 2 |
| 4 | Default: git source counts local commits only. | Supervisor | revisit Stage 2 |
| 5 | Default: worktree folders grouped under their main repo. | Supervisor | revisit Stage 2 |
| 6 | Default: on Linux with no Secret Service running, connector setup refuses with guidance. | Supervisor | revisit Stage 2 |
| 7 | Default: v1 delivered in internal milestones M1 usage → M2 kit → M3 connectors → M4 dashboard; one release. | Supervisor | revisit Stage 2 |
| 8 | Assumed: the owner signs in to Claude Code with a claude.ai subscription (needed for FR-020 links to matter). | thunderkds | Stage 2 |
| 9 | Dev-time only: QA uses the easy-ui container to screenshot/check easy-personal's own dashboard as UI Evidence. easy-personal never calls easy-ui at runtime. | thunderkds | confirmed 2026-10-21 |
| 10 | Resolved 2026-10-26: usage is ingested from every confirmed Claude agent account (each account's `projects/` folder); every event carries its agent account, and usage can be filtered by account as well as by workspace (FR-036). | thunderkds | done |
| 11 | Default: workspace and agent-account data live in the existing local store; no new MCP tools in the first version. | Supervisor | revisit Stage 2 |
| 12 | Resolved 2026-10-31 (grill): Work view inspired by github.com/blogminhquy/javis-os, ideas only. Board = read-only, PROJECT_KANBAN.md + kit's local Linear pull; agent split = sub-agents + `sdk*` entrypoints (revised 2026-10-31 from non-`cli`); schedules = agent-related timers + cron; graph = names + links, live, both memory kinds, per project; first run = checklist, no writes. | thunderkds | done |
| 13 | Resolved 2026-10-31: board and memory graph (FR-037/FR-041) for 50 projects return in < 1 s on the owner's machine (NFR-010). | thunderkds | done |
| 14 | Resolved 2026-10-31: the FR-039 migration marks every source file for re-read; the next normal ingest fills `entrypoint`; until then old rows count as "unknown" (abstained). No separate command. | thunderkds | done |
| 15 | Resolved 2026-11-02 (grill): live attention = FR-047..FR-050. Owner chose: open call keeps a session active; failing = 3 failures in a row, any tool, one setting; question shown at once, other calls at 10 min; Live tab only. Evidence (owner history, 1,101 files): 3+ failure streaks 88, 5+ 18; non-question call open ≥ 10 min ≈ 80 of 42,921. | thunderkds | done |
| 16 | Resolved 2026-11-02: streams = one transcript file; denied permission counts as failure; open call kept ≤ 24 h (Supervisor defaults, owner confirmed). | thunderkds | done |
| 17 | Resolved 2026-11-02 (owner): accepted limit — a call opened before `easy-personal dashboard` started, whose record the scheduled ingest already read, is not seen as open (the tail starts from stored offsets). Documented in the README Live view section; no re-read at start. | thunderkds | done |
| 18 | Resolved 2026-11-02 (grill): easy-memory layer = FR-051..FR-053. Owner chose: cited files as nodes (not one node per note), stale = newest citing note's hash ≠ disk, top 30 nodes, 30-day panel, list easy-memory-only projects. Read store files directly, no MCP (keeps DDR-0002 and the metadata-only rule). Measured 2026-11-02: 861 notes in 34 store folders (50 more hold only `prompts.log`), kit repo 288 notes / 109 cited files, this repo 98 / 43. | thunderkds | done |
| 19 | Assumed: easy-memory keeps store format `v` 1 (`meta.json`, `notes/<id>.json`, `prompts.log`). A newer `v` is abstained, never guessed; no change is asked of easy-memory-mcp. | Supervisor | revisit Stage 2 |

---

## Cross-Repo Dependencies

Work that lives in another repo goes on that repo's own board (one project per KANBAN), not this one.

| ID | Repo | Change | Needed by |
|----|------|--------|-----------|
| XR-1 | easy-verifier-mcp | Write score JSON next to each HTML evidence report (same folder, same base name). | FR-011 (M2) |
