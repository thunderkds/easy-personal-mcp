# PRD — easy-personal-mcp
**Last updated**: 2026-10-21
**Status**: Draft
**Owner**: thunderkds

> **Scope of this document**: *What* to build and *why* — product intent, user stories, requirements, success metrics.
> Technical decisions, architecture, agent config, and task state live in `PROJECT_SPEC.md`.
> Source of intent: `BRAINSTORMING_LOG.md` (2026-10-21; Option B, D1–D6 approved).

---

## Overview

An individual who works with coding agents every day has no clear picture of *how* they work: which
projects eat the budget, which sessions run away, whether caching works, whether habits are getting
better or worse. Their personal tool connections (email, chat, task tools) are configured by hand,
per agent, with secrets pasted into plaintext config files.

`easy-personal-mcp` is the Easy Kit plug-in whose subject is the person, not a repo. It reads the
person's local agent activity, turns it into deterministic metrics, warnings and an improvement plan,
and shows them on a local dashboard and through MCP tools. From v1.1 it also manages personal tool
connectors from a provider catalog, keeping every secret in the OS keychain and injecting it only at
launch. Everything stays on the person's machine.

---

## Personas

| ID | Name | Role | Pain Point |
|----|------|------|-----------|
| P1 | Solo Builder | One person (the owner) running Claude Code, Codex and Cursor across many repos | Cannot see spend, runaway sessions or habits across projects; transcripts vanish after 30 days; connector secrets sit in plaintext config. |
| P2 | Supervisor Agent | The Easy Kit Supervisor or any agent session | Has no factual view of the person's recent usage when asked "how am I doing / what should I change"; would otherwise guess. |

---

## User Stories

| ID | Story | Persona | Phase |
|----|-------|---------|-------|
| US-001 | As P1, I want my Claude Code usage (sessions, tokens, cache, estimated cost) summarised by day, model and project, so I know where my agent time and money go. | P1 | v1 |
| US-002 | As P1, I want usage history kept beyond Claude Code's 30-day transcript cleanup, so trends survive. | P1 | v1 |
| US-003 | As P1, I want warnings labelled *improve / aware / warning*, each citing the rule and the measured value, so I trust why it fired. | P1 | v1 |
| US-004 | As P1, I want an improvement plan ordered by priority with a concrete fix per item, so I know what to change first. | P1 | v1 |
| US-005 | As P1, I want a dashboard with charts and flows that I open locally, so I can review my week at a glance. | P1 | v1 |
| US-006 | As P2, I want MCP tools that return the same numbers as the dashboard, so I can answer the person with facts. | P2 | v1 |
| US-007 | As P1, I want to pick a category (Email, Messaging…) and a provider (Gmail, SendGrid, Slack, Teams, Discord…) and get the right config form, so connecting a tool is guided. | P1 | v1.1 |
| US-008 | As P1, I want connector secrets kept in my OS keychain and never written to any config file, so a leaked dotfile leaks nothing. | P1 | v1.1 |
| US-009 | As P1, I want one connector set up once to work in Claude Code, Codex and Cursor, so I don't repeat it per agent. | P1 | v1.1 |
| US-010 | As P1, I want connector suggestions based on what I actually do in sessions, so I add the tools that would help. | P1 | v1.1 |
| US-011 | As P1, I want verifier scores, kit traces and git activity alongside usage, so I see output, not just spend. | P1 | v1.2 |
| US-012 | As P1, I want life sources (calendar, finance, health) through the same source interface. | P1 | v2 |

---

## Functional Requirements

| ID | Requirement | Traces to |
|----|-------------|-----------|
| FR-001 | Must ingest `~/.claude/projects/**/*.jsonl` incrementally (file + byte offset), idempotently, into a local store under `~/.easy-personal/`. | US-001, US-002 |
| FR-002 | Must extract only metadata: timestamps, session id, project path, branch, model, record type, tool names, `message.usage` token fields. Message and tool content must never be stored. | US-001, NFR-002 |
| FR-003 | Records of an unknown type or shape must be counted as *abstained* per Claude Code version and shown, never coerced into a metric. | US-001, US-003 |
| FR-004 | Must compute usage summaries and breakdowns by day, model, project and tool for a requested period. | US-001 |
| FR-005 | Cost must come from a dated local price table; a model missing from the table makes cost abstain for that model. | US-001 |
| FR-006 | Warnings must come from declared rules (id, threshold, label `improve`/`aware`/`warning`, fix line) over measured facts; each hit cites rule id and measured value. | US-003 |
| FR-007 | The improvement plan must list unmet rules grouped by priority, each with its static fix line. | US-004 |
| FR-008 | `report` must write a self-contained static HTML dashboard + JSON to `~/.easy-personal/reports/` (or `EASY_PERSONAL_REPORTS_DIR`). | US-005 |
| FR-009 | MCP and CLI must share one core; v1 MCP tools: `usage_summary`, `usage_breakdown`, `session_outliers`, `warnings`, `improvement_plan`, `render_report`. | US-006 |
| FR-010 | *(v1.1)* Provider catalog as declarative files: category, provider, fields, which fields are secret, how the provider server is launched, whether an official claude.ai connector exists. | US-007 |
| FR-011 | *(v1.1)* Secrets must be stored only via the OS keychain; with no keychain backend the setup must refuse with guidance, never fall back to plaintext. | US-008 |
| FR-012 | *(v1.1)* `easy-personal exec <connector>` must read secrets from the keychain and exec the provider server with them in its environment; `easy-personal headers <connector>` must print JSON headers for HTTP servers. | US-008, US-009 |
| FR-013 | *(v1.1)* Agent config must be written through official CLIs (`claude mcp add-json --scope user`, Codex CLI) and, for Cursor, a diffed, backed-up, atomic edit of `~/.cursor/mcp.json`; when a CLI is missing, show the snippet instead. Project-scope files are never written. | US-009 |
| FR-014 | *(v1.1)* When a provider has an official claude.ai connector, the dashboard must offer the link as the first option. | US-007 |
| FR-015 | *(v1.1)* No MCP tool may create or modify a connector or return a secret value; MCP exposes only `connector_catalog`, `connector_status` (configured yes/no), `connector_suggestions`. | US-008 |
| FR-016 | *(v1.1)* Suggestions must come from counters over metadata (tool names, command names, link domains), never from stored text. | US-010 |

---

## Non-Functional Requirements

| ID | Requirement | Category |
|----|-------------|----------|
| NFR-001 | No network calls at runtime; HTTP mode binds `127.0.0.1` only. | Security |
| NFR-002 | No secret, message text or tool output appears in any tool response, report, log or store row (fixture with planted secrets proves it). | Security / Privacy |
| NFR-003 | Source files are read-only; nothing under `~/.claude/projects/` or any repo is modified. | Safety |
| NFR-004 | Deterministic: the same inputs give the same numbers; no LLM inside the server. | Correctness |
| NFR-005 | Ingest of 30 days of transcripts (≈ 500 MB) completes in < 60 s on a laptop and streams (bounded memory). | Performance |
| NFR-006 | `exec` launcher adds < 200 ms to connector start and has no third-party imports beyond `keyring`. | Performance / Reliability |
| NFR-007 | Python 3.11+, native install (`uv tool install`); Docker optional and analytics-only. | Portability |

---

## Success Metrics / KPIs

| Metric | Baseline | Target | How measured |
|--------|----------|--------|--------------|
| Usage history retained | 30 days (Claude Code default) | ≥ 180 days | Oldest day in store |
| Unknown-record share | unknown | < 2% of records per version | `abstained` count / total |
| Secrets in plaintext agent config | today: some | 0 after v1.1 | Scan of `~/.claude.json`, `~/.codex/config.toml`, `~/.cursor/mcp.json` for connector env values |
| Weekly review time | n/a | < 5 min | Self-reported |

---

## Out of Scope

- Calling provider APIs directly (no SendGrid/Slack client in personal-mcp) — future brainstorm.
- Creating claude.ai connectors (not possible from a local app; link only).
- Writing project-scope agent config or hooks (owned by the kernel's `setup.sh`).
- Sharing or publishing reports; multi-user.
- LLM-written narratives or "AI judgment" in reports.
- Life sources before v2.

---

## Open Questions / Assumptions

| # | Question / Assumption | Owner | Due |
|---|----------------------|-------|-----|
| 1 | Repo name `easy-personal-mcp` (server id `easy-personal`) | thunderkds | before repo creation |
| 2 | Assumed: the owner signs in to Claude Code with a claude.ai subscription (needed for FR-014 links to matter) | thunderkds | v1.1 |
| 3 | Exact Codex CLI command for adding an MCP server — verify at v1.1 | Supervisor | v1.1 |
| 4 | Cursor `${env:}` in remote headers reported broken on the forum — verify before relying on it | Supervisor | v1.1 |
