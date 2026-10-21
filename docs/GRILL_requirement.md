# Stage 0.5a — Requirement grill of PRD.md (easy-personal-mcp)
**Date**: 2026-10-21 (rerun after the Phase 0 recheck) · **Skill**: `grill-with-docs` `mode=requirement` · **PRD gate**: PASS

Checks run: FR → US traceability · scope clarity · ambiguity ("would two engineers build it differently?") ·
missing NFRs · open questions without owner/date · assumptions that trace only to a supervisor guess.
Sibling repos read to answer facts instead of asking: `easy-verifier-mcp`, `easy-ui-mcp`, `easy-context-mcp`,
`~/.easy-context/`, `~/.easy-verifier-reports/`.

Traceability: FR-001…FR-027 each trace to a US ✅. US-013 (life sources) has no FR — correct for v2 ✅.
No `<!-- FLAGGED -->` items remain in `PRD.md`.

## New findings (this run)

| # | Finding | Where | Resolution | Decided by |
|---|---|---|---|---|
| N1 | Kernel trace `summary` field holds raw tool input (file contents) | FR-010 | Read only `timestamp`, `tool_name`, `is_error`, task id from file name; never `summary` | Supervisor (tightens NFR-002) |
| N2 | Live dashboard has write actions on `127.0.0.1`; any browser tab can send requests there | NFR-008 | Per-launch token + `Host`/`Origin` check on every write | Supervisor (security) |
| N3 | easy-context: stats exist on disk and via MCP on `127.0.0.1:8767` | FR-012 | MCP client of the service, read-only tools only | Owner |
| N4 | Owner asked about keeping easy-context's index in sync | FR-012, FR-025 | Show freshness + `context_index_stale` rule; never call `index_path` | Owner |
| N5 | easy-verifier writes HTML reports only; its MCP tools run a new scoring | FR-011, XR-1 | easy-verifier writes score JSON next to each HTML (task on its own board); HTML-only = "no data" | Owner |
| N6 | easy-ui writes `session-*.json` into its own `reports/` (206/222 files), with no project field; `target` is caller-authored text | FR-013 | Show UI checks overall, not per project; never store `target` | Owner |
| N7 | Owner wants easy-ui to check UIs | Open Q 9 | Dev-time only: QA checks our own dashboard with easy-ui as UI Evidence | Owner |

## Carried over from the first run

| # | Finding | Resolution | Decided by |
|---|---|---|---|
| G1 | Board/workflow never asked | Linear, project `easy-personal-mcp` only | Owner (Phase 0 D) |
| G2 | "Cost" ambiguous on a subscription | Tokens primary; $ labelled "API-equivalent cost" (FR-004, FR-005) | Owner |
| G3 | Cloud sessions leave no local transcript | Out of Scope; dashboard says so (FR-023) | Owner |
| G4 | Several machines | This machine only; `machine_id` on every row (FR-027) | Owner |
| G5 | "Flows" undefined | Session timeline only (FR-023) | Owner |
| G6 | No actual rules named | 7-rule pack locked (FR-026) | Owner |
| G7 | Suggestions vs. metadata-only | Tool names, Bash first word, WebFetch/MCP domains only (FR-022) | Owner (Phase 0 A privacy) |
| G8 | Persona uses Codex/Cursor | Claude Code only; Codex/Cursor Out of Scope | Owner (Phase 0 C) |
| G9 | Retention 180 days was a guess | Keep forever; `retention_days` prunes (FR-027) | Owner (accepted default) |
| G10 | NFR-005 not measured | Marked provisional; first spike resets it | Owner (accepted default) |
| G11 | No metric for plan effectiveness | Rule hits/week fall over 4 weeks | Owner (accepted default) |
| G12 | Agent access: standalone or kernel skill | Standalone; kernel skill calls Out of Scope | Owner (Phase 0 A) |
| G13 | Full paths in reports | Folder name in reports; full path in store (FR-004) | Owner (accepted default) |
| G14 | Timezone for "per day" | OS local; stored UTC (FR-004) | Owner (accepted default) |
| G15 | Open Questions lacked due dates | Every row has a due stage or "confirmed/done" | Supervisor |

## Gate
No requirement traces only to an unconfirmed guess. Remaining supervisor defaults (Open Questions 3–7)
were confirmed by the owner in the Phase 0 Project Context Document and carry a Stage 2 revisit flag.
**PRD gate: PASS.**
