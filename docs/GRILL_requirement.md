# Stage 0.5a — Requirement grill of PRD.md (easy-personal-mcp)
**Date**: 2026-10-21 · **Skill**: `grill-with-docs` `mode=requirement` · **PRD gate**: FAIL until every item below is resolved

Checks run: FR → US traceability · scope clarity · ambiguity ("would two engineers build it differently?") ·
missing NFRs · open questions without owner/date · assumptions that trace only to a supervisor guess.

Traceability: FR-001…FR-016 each trace to a US ✅. US-012 (life sources) has no FR — correct for v2, kept out of v1 ✅.

| # | Finding | Where | Kind | Recommended answer | Status |
|---|---|---|---|---|---|
| G1 | Board/workflow (Phase 0 Section D) never asked | Process | Missing | Linear `online`, team HUN, project `easy-personal-mcp` | ✅ Resolved 2026-10-21 (Thunder) |
| G2 | "Cost" is ambiguous: on a subscription plan, $ per token is notional; the real limit is the plan's usage windows | FR-005, US-001 | Ambiguity | Show tokens as the primary unit; $ as "API-equivalent cost", labelled; usage-window view if the data allows | Open |
| G3 | Cloud sessions (claude.ai/code, like this project's threads) leave no local transcript, so v1 cannot see them | FR-001, US-001 | Scope gap | State it in Out of Scope for v1; revisit with OTel/export later | Open |
| G4 | Several machines (laptop + desktop) each have their own `~/.claude` | FR-001, US-002 | Missing NFR | v1 = one machine; store carries a `machine_id` so a later merge is possible | Open |
| G5 | "Flows" on the dashboard is undefined (session timeline? tool sequence? kit stage flow?) | US-005, FR-008 | Ambiguity | Session timeline + tool-call sequence per session (no content) | Open |
| G6 | The v1 rule pack is only examples in the log; the PRD names no actual rules or thresholds | FR-006, FR-007 | Ambiguity | Lock ~8 rules in the PRD (list proposed in the question) | Open |
| G7 | Contradiction: suggestions from "pasted Slack links" read message text, but FR-002 says metadata only | FR-016 vs FR-002 | Conflict | Suggestions only from tool names, Bash command names and WebFetch/MCP target domains; never from message text | Open |
| G8 | Persona P1 uses Codex and Cursor, but v1 reads Claude Code only | P1, US-001 | Assumption | Confirm Claude Code only in v1; Codex/Cursor in v1.2 | Open |
| G9 | Retention target 180 days is a supervisor guess | Success metrics, US-002 | Assumption | Keep forever by default; `retention_days` setting to prune | Open |
| G10 | NFR-005 (500 MB in 60 s) is a guess, not measured | NFR-005 | Assumption | Mark provisional; T001 measures real volume and resets it | Open |
| G11 | Success metrics don't measure whether the improvement plan works | Success metrics | Missing | Add: number of rule hits per week falls over 4 weeks | Open |
| G12 | How an agent reaches the MCP: standalone only, or an optional kernel skill (e.g. `weekly-review`)? | US-006, P2 | Scope | Standalone in v1; kernel skill is a separate kernel brainstorm later | Open |
| G13 | Project names in reports: full paths or short names? | FR-002, FR-008 | Ambiguity | Repo folder name only (no full path) in reports; full path kept in the store | Open |
| G14 | "Per day" uses which timezone? | FR-004 | Ambiguity | OS local timezone, stored in UTC | Open |
| G15 | Open Questions table has owners but no due dates | Open Questions | Missing | Due = before the task that needs it (named per row) | Open |

Only G2–G8 change what gets built in a way the user can feel; they are asked one at a time.
G9–G15 have a reversible default; they are listed so they can be overridden, and become tracked assumptions if accepted.
