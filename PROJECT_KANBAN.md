# PROJECT_KANBAN.md
**Last updated**: 2026-10-22

> Board cache, pulled from the board by `python3 scripts/board_sync.py pull`. Do not edit it by hand:
> the board is the source of truth, and the next pull overwrites this file.

---

## Board

### Todo

### In Progress
- [ ] **HUN-57** — Landing page: the idea, the flow, the install and deploy guide of easy-personal-mcp | Frontend-Implementer | C2 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-57.md`

### Ready for Review

### Done
- [x] **HUN-32** — Spike: inventory Claude Code transcript shapes and real volume → synthetic fixtures | QA-Automation-Agent | C1 | Risk: Medium | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-32.md`
- [x] **HUN-33** — Skeleton: package, pyproject, \`easy-personal\` CLI + MCP stdio entry sharing one core, test runner | Common-Infrastructure-Agent | C1 | Risk: Low | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-33.md`
- [x] **HUN-34** — Tracer bullet: \`claude_code\` source → store (project, machine_id, UTC) → \`usage_summary\` in MCP and CLI | Backend-Implementer | C3 | Risk: High | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-34.md`
- [x] **HUN-35** — \`usage_breakdown\` + \`session_outliers\`: tokens-first, labelled API-equivalent cost, local day, folder names | Backend-Implementer | C2 | Risk: Medium | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-35.md`
- [x] **HUN-36** — Leak test suite: planted secrets and text never reach store, MCP, CLI or logs | QA-Automation-Agent | C2 | Risk: High | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-36.md`
- [x] **HUN-37** — Rules engine + 5 usage rules + \`warnings\` + \`improvement_plan\` | Backend-Implementer | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-37.md`
- [x] **HUN-38** — Scheduled ingest: single-writer lock, systemd user timer, \`retention_days\` | Common-Infrastructure-Agent | C1 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-38.md`
- [x] **HUN-39** — \`kit_trace\` source (never \`summary\`) + \`tool_errors_high\` rule + \`kit_summary\` (kernel part) | Backend-Implementer | C2 | Risk: High | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-39.md`
- [x] **HUN-40** — \`git_local\` source + \`kit_summary\` git part | Backend-Implementer | C1 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-40.md`
- [x] **HUN-41** — \`context\` read-only MCP client of easy-context + index freshness + \`context_index_stale\` | Backend-Implementer | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-41.md`
- [x] **HUN-42** — \`verifier\` source reads easy-verifier score JSON + \`kit_summary\` part | Backend-Implementer | C1 | Risk: Medium | P2 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-42.md`
- [x] **HUN-43** — \`ui\` source: easy-ui UI checks overall (never \`target\`) + \`kit_summary\` part | Backend-Implementer | C1 | Risk: Medium | P2 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-43.md`
- [x] **HUN-44** — Launcher: keychain store + refusal path, \`exec\`/\`headers\`, env-only secrets, <200 ms | Backend-Implementer | C2 | Risk: High | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-44.md`
- [x] **HUN-45** — Connector tracer: Slack catalog entry + \`connector add/remove\` CLI → keychain → \`claude mcp add-json\` (snippet fallback) + \`doctor\` | Backend-Implementer | C2 | Risk: High | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-45.md`
- [x] **HUN-46** — Catalog breadth: Gmail, SendGrid, Teams, Discord + claude.ai-connector flag | Backend-Implementer | C1 | Risk: Low | P2 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-46.md`
- [x] **HUN-47** — MCP \`connector_catalog\` / \`connector_status\` / \`connector_suggestions\` (metadata counters) | Backend-Implementer | C2 | Risk: Medium | P2 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-47.md`
- [x] **HUN-48** — \`web/\` toolchain: Vite + React, pinned lockfile, build copied into the package, release check | Common-Infrastructure-Agent | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-48.md`
- [x] **HUN-49** — Dashboard server: \`easy-personal dashboard\` on demand, free port, token + Host/Origin guard, CSP, read API | Backend-Implementer | C2 | Risk: High | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-49.md`
- [x] **HUN-50** — Dashboard usage view: summary, breakdowns, session timeline, "not included" notice | Frontend-Implementer | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-50.md`
- [x] **HUN-51** — Dashboard warnings, improvement plan and kit panel | Frontend-Implementer | C2 | Risk: Low | P2 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-51.md`
- [x] **HUN-52** — Dashboard connector forms + guarded write API (secrets never echoed) | Frontend-Implementer | C2 | Risk: High | P2 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-52.md`
- [x] **HUN-53** — Session timeline read: \`metrics.session_timeline\` + \`GET /api/usage/sessions\` | Backend-Implementer | C1 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-53.md`
- [x] **HUN-56** — README: install, usage and deploy guide for v0.1.0 | Backend-Implementer | C1 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-56.md`

### Closed (will not do)

## Blocked

---

## Stage Tracker
