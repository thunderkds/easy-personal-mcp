# 0004. Live view shows call labels and kit stage (show, never store)

**Status**: Accepted
**Date**: 2026-11-06
**Deciders**: thunderkds (owner), Supervisor
**Related**: FR-081, FR-082, FR-083, FR-084 · FR-044, FR-047, FR-072 · NFR-012 · US-033 · DDR-0003

---

## Context

The Live view feed shows tool names only ("Bash", "Read"). The owner asked (2026-11-06) to see what the agent does: which tools, which steps of the kit. The metadata-only rule (PROJECT_SPEC Critical Constraints) bans tool input on every surface, so a skill name or a Bash command name cannot be shown as the rule is written.

Gate: (1) hard to reverse — no: nothing is stored, the labels can be removed in one release; (2) surprising without context — yes: tool input now reaches the page; (3) genuine trade-off — yes: three options (safe labels / full detail / no change). 2 of 3 hold: DDR, not ADR.

---

## Decision

We will read four input-derived *call labels* in the live tail and send them on `GET /api/live` only: `Skill.skill`, `Agent`/`Task` `subagent_type`, the Bash command's first word (`core/jobs.command_name`), and the MCP server/tool split from the tool name. Sub-agent calls also carry `agentType` from the stream's `agent-<id>.meta.json`. Each label must match a short name pattern or it is dropped. A fixed stage map in code turns the newest mapped `Skill` call into the session's kit stage. Labels and stage live in memory only: never in the store, logs, MCP or CLI.

---

## Alternatives Considered

| Alternative | Pros | Cons | Why not chosen |
|-------------|------|------|----------------|
| Safe labels only (names, no arguments or paths) | Shows the steps; names carry no secrets | Read/Edit show no file; typed slash commands not seen | **Selected** (owner, 2026-11-06) |
| Full step detail (paths, command lines, descriptions) | Most detail | Commands and paths can hold secrets; a wide exception | Owner chose safe labels |
| Keep tool names only | No rule change | Owner cannot tell what the agent does | Owner asked for more |

---

## Consequences

- The metadata-only rule gets a second *show, never store* exception (after FR-037/FR-041). The leak suite must prove no label reaches the store, logs, MCP or CLI, and that planted `args`/`prompt`/`description`/command arguments never reach `/api/live`.
- The stage map is code: a kit skill rename needs a code change.
- A session that began before the dashboard shows no stage until its next mapped `Skill` call (no history replay, FR-071).
