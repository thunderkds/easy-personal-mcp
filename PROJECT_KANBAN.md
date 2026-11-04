# PROJECT_KANBAN.md
**Last updated**: 2026-11-04

> Board cache, pulled from the board by `python3 scripts/board_sync.py pull`. Do not edit it by hand:
> the board is the source of truth, and the next pull overwrites this file.

---

## Board

### Todo
- [ ] **HUN-207** — Usage opens the Scratch row; Workspaces page lists repos in no workspace with Add to (FR-069 UI, FR-070) | Frontend-Implementer | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-207.md`

### In Progress

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
- [x] **HUN-57** — Landing page: the idea, the flow, the install and deploy guide of easy-personal-mcp | Frontend-Implementer | C2 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-57.md`
- [x] **HUN-58** — Bugfix: ingest crashes on pre-HUN-42 stores (verifier_scores 6 vs 7 columns) | Backend-Implementer | C1 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-58.md`
- [x] **HUN-61** — Store foundation: versioned migrations, FK + busy_timeout, time-first usage query + indexes, 1M-event benchmark | Backend-Implementer | C2 | Risk: Medium | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-61.md`
- [x] **HUN-62** — Agent accounts: detect ~/.claude\* and ~/.codex\* folders, confirm/name/ignore (CLI + API) | Backend-Implementer | C2 | Risk: Medium | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-62.md`
- [x] **HUN-63** — Ingest usage from every confirmed Claude account; events tagged by account; usage filter --account | Backend-Implementer | C2 | Risk: High | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-63.md`
- [x] **HUN-64** — Workspaces: add/edit/rename/delete, repos, built-in orphan; usage filter --workspace (CLI + API) | Backend-Implementer | C2 | Risk: Medium | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-64.md`
- [x] **HUN-65** — Connectors get id + alias and belong to a workspace; accounts pick workspaces; plan/apply sync into each Claude account | Backend-Implementer | C3 | Risk: High | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-65.md`
- [x] **HUN-66** — Move connector, change alias, rename workspace, delete workspace → connectors to orphan, assign out of orphan | Backend-Implementer | C2 | Risk: High | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-66.md`
- [x] **HUN-67** — One-time adoption: pre-workspace connectors (slack, gmail, …) move to orphan | Backend-Implementer | C1 | Risk: High | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-67.md`
- [x] **HUN-68** — Dashboard Agents page: detected/confirmed accounts, by account / by provider, pick workspaces with plan confirmation | Frontend-Implementer | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-68.md`
- [x] **HUN-69** — Dashboard Workspaces page + workspace form (repos, connectors, alias, move, delete) + legacy banner | Frontend-Implementer | C2 | Risk: High | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-69.md`
- [x] **HUN-70** — Usage page: Workspace and Account filters | Frontend-Implementer | C1 | Risk: Low | P2 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-70.md`
- [x] **HUN-71** — QA end-to-end: several fake accounts + workspaces, no secret leaks, no real config touched | QA-Automation-Agent | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-71.md`
- [x] **HUN-72** — Fix workspace review defects (connector remove ownership, default-account env, doctor after adoption, plan abort line, add EOF) | Backend-Implementer | C2 | Risk: High | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-72.md`
- [x] **HUN-73** — Release prep v0.2.0 (version bump, README for Workspaces/Agents tabs and remove rule) | Backend-Implementer | C1 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-73.md`
- [x] **HUN-83** — Dashboard dark theme, sidebar shell and Usage widget cards | Frontend-Implementer | C2 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-83.md`
- [x] **HUN-113** — Workspaces page renders without waiting for the legacy scan; legacy checks run in parallel | Backend-Implementer | C1 | Risk: Low | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-113.md`
- [x] **HUN-114** — Agents: re-choose ignored accounts, rename/ignore confirmed ones; Usage explains ignored accounts | Frontend-Implementer | C1 | Risk: Medium | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-114.md`
- [x] **HUN-115** — Account status-line data in /api/accounts: email, plan, last model, tokens, plan-limit snapshot | Backend-Implementer | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-115.md`
- [x] **HUN-116** — Agents account cards read like the status line (email, model, tokens, 5h/7d limit bars) | Frontend-Implementer | C1 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-116.md`
- [x] **HUN-117** — Warnings, plan and kit accept workspace/account scope (API, MCP, CLI) | Backend-Implementer | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-117.md`
- [x] **HUN-118** — Global Workspace/Account selector in the sidebar for Usage, Warnings and Kit | Frontend-Implementer | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-118.md`
- [x] **HUN-119** — Kit tab: plain project cards with health, recent activity and a details toggle | Frontend-Implementer | C2 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-119.md`
- [x] **HUN-120** — Warnings & plan: plain titles, why it matters, numbered steps, one ordered action list | Backend-Implementer | C2 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-120.md`
- [x] **HUN-121** — Restyle dashboard to reference image 2: indigo + monospace, grouped sidebar, right rail | Frontend-Implementer | C2 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-121.md`
- [x] **HUN-126** — Restyle dashboard to the Graphite Mint design (all 5 views; plain Warnings/Kit content kept) | Frontend-Implementer | C2 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-126.md`
- [x] **HUN-128** — Dashboard shell in a centered max-width container | Frontend-Implementer | C1 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-128.md`
- [x] **HUN-129** — Usage "By model" card: no clipped cost column | Frontend-Implementer | C1 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-129.md`
- [x] **HUN-131** — Dashboard neon mint pass: brighter accent and glow per the updated Graphite Mint design | Frontend-Implementer | C1 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-131.md`
- [x] **HUN-132** — Landing page (site/index.html) in the Easy Kit family Graphite Mint layout | Frontend-Implementer | C2 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-132.md`
- [x] **HUN-160** — Work view rail regroup and first-run checklist | Frontend-Implementer | C2 | Risk: Low | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-160.md`
- [x] **HUN-161** — Typed vs agent usage split (entrypoint migration) | Backend-Implementer | C2 | Risk: Medium | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-161.md`
- [x] **HUN-162** — Work view: read-only cross-project board | Frontend-Implementer | C2 | Risk: Low | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-162.md`
- [x] **HUN-163** — Work view: scheduled jobs (systemd timers + cron) | Frontend-Implementer | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-163.md`
- [x] **HUN-164** — Work view: memory graph (names + links, read live) | Frontend-Implementer | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-164.md`
- [x] **HUN-165** — Live trace spike: test harness and baseline for the standard test session | QA-Automation-Agent | C1 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-165.md`
- [x] **HUN-166** — Live trace E1: live tail of Claude Code transcripts | QA-Automation-Agent | C1 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-166.md`
- [x] **HUN-167** — Live trace E2: native async http hooks to a local collector | QA-Automation-Agent | C1 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-167.md`
- [x] **HUN-168** — Live trace E3: Claude Code OpenTelemetry to a local OTLP receiver | QA-Automation-Agent | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-168.md`
- [x] **HUN-169** — Live trace E4: stream-json capture from headless runs | QA-Automation-Agent | C1 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-169.md`
- [x] **HUN-170** — Live trace report: combined run, E5 docs-only result, REPORT.md | QA-Automation-Agent | C1 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-170.md`
- [x] **HUN-175** — Transcript reader recognises the record types new in Claude Code 2.1.316 (FR-046) | Backend-Implementer | C1 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-175.md`
- [x] **HUN-176** — Live tail: follow every account's transcripts while the dashboard runs (FR-043) | Backend-Implementer | C2 | Risk: Medium | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-176.md`
- [x] **HUN-177** — GET /api/live: stream live events to the page as Server-Sent Events (FR-044) | Backend-Implementer | C2 | Risk: Medium | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-177.md`
- [x] **HUN-178** — Live view: active sessions, per-session tool feed, running token and cost totals (FR-045) | Frontend-Implementer | C2 | Risk: Low | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-178.md`
- [x] **HUN-179** — Release 0.3.0: version bump, README and site updates, release plan | Backend-Implementer | C1 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-179.md`
- [x] **HUN-181** — Live attention states in the live tail + stream re-send (FR-047..049, NFR-012; v0.4.0) | Backend-Implementer | C2 | Risk: Medium | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-181.md`
- [x] **HUN-182** — Live view attention badge, border, header count, sort (FR-050; after HUN-181) | Frontend-Implementer | C2 | Risk: Low | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-182.md`
- [x] **HUN-183** — Release 0.4.0: version, README live attention + easy-memory + kit performance + v8 upgrade note, site (after HUN-195) | Backend-Implementer | C1 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-183.md`
- [x] **HUN-184** — easy-memory store reader + cited-file nodes and panel data in /api/memory-graph (FR-051..053 backend) | Backend-Implementer | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-184.md`
- [x] **HUN-185** — Memory view easy-memory layer: cited-file nodes, stale mark, 30-day panel, list count (FR-052..053 page; after HUN-184) | Frontend-Implementer | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-185.md`
- [x] **HUN-186** — Dashboard speed + gone-project filter: fast kit / usage sessions / warnings / plan / memory list reads, hide gone projects (FR-054, NFR-013) | Backend-Implementer | C2 | Risk: Medium | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-186.md`
- [x] **HUN-187** — Board view UX: one project at a time, hide projects without a board, plain clamped titles, no sideways scroll on phones (FR-055) | Frontend-Implementer | C2 | Risk: Low | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-187.md`
- [x] **HUN-188** — Usage, Memory and Kit view UX: one-line numbers, distinct model names, Memory opens on the busiest project, Kit folds idle projects (FR-054, FR-056) | Frontend-Implementer | C2 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-188.md`
- [x] **HUN-189** — Owner project: count gone worktrees under their main repo in usage by project and project lists (FR-057) | Backend-Implementer | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-189.md`
- [x] **HUN-190** — Usage lists show top 10 with "Show N more" (FR-058) | Frontend-Implementer | C1 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-190.md`
- [x] **HUN-191** — Session timeline shows 10 sessions with "Show N more" (FR-058) | Frontend-Implementer | C1 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-191.md`
- [x] **HUN-192** — Store failed tool results per user record: migration 7 → 8 + re-read (FR-059) | Backend-Implementer | C2 | Risk: Medium | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-192.md`
- [x] **HUN-193** — kit_findings: like-with-like session comparison per kit server, MCP tool + CLI (FR-060..FR-063, NFR-014; after HUN-192) | Backend-Implementer | C2 | Risk: Medium | P0 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-193.md`
- [x] **HUN-195** — kit_findings: a finding needs a gap of at least 5% to carry a habit (FR-062) | Supervisor | C1 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-195.md`
- [x] **HUN-196** — Cache reuse card follows the app-wide period, with written-to-cache and lowest-model lines (FR-064) | Frontend-Implementer | C2 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-196.md`
- [x] **HUN-198** — Memory panel: "memory on vs off" card from easy-memory report.json | ? | C2 | Risk: Low | P2 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-198.md`
- [x] **HUN-200** — Dashboard forms UX + floating notifications (toasts) + floating loading indicator (FR-065) | Frontend-Implementer | C2 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-200.md`
- [x] **HUN-201** — Easy Kit link → easy-kits.vercel.app; landing page flow made visual (SVG system diagram + quick-start strip) | ? | ? | Risk: ? | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-201.md`
- [x] **HUN-202** — Toasts stay inside the page box; loading indicator in the middle of the content area (FR-065 follow-up) | Frontend-Implementer | C1 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-202.md`
- [x] **HUN-203** — Repos folder browser in the workspace form + gap above Save (FR-068) | Frontend-Implementer | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-203.md`
- [x] **HUN-205** — Commit the Easy Kit update 75040d9 → 04b019a (kit files only) | Supervisor | C1 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-205.md`
- [x] **HUN-206** — Scratch row in project lists + unassigned-repos read route (FR-069, FR-070 read side) | Backend-Implementer | C2 | Risk: Medium | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-206.md`

### Closed (will not do)
- [~] **HUN-194** — Release 0.5.0 kit performance: version, README kit_findings + migration note, site (after HUN-193 and HUN-183) | Backend-Implementer | C1 | Risk: Low | P1 | @hungnh1110 | Guide: `tasks/TASK_GUIDE_HUN-194.md`

## Blocked

---

## Stage Tracker
