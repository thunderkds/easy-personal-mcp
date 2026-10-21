# PROJECT_KANBAN.md
**Last updated**: 2026-10-21

> Compact task board. Full context lives in `PROJECT_SPEC.md`. Update this file whenever a task status changes.

---

## Board

> Task line format: **Txxx** — [title] | [agent] | C[0–3] | Risk: Low/Med/High | P[0–2]
> Board mode `online` (Linear team HUN, project [easy-personal-mcp](https://linear.app/thunderkds/project/easy-personal-mcp-a6ec3c36d1ae)): the board is the source of truth; this file is its cache.

### Todo
- [ ] **HUN-21** — Spike: inventory Claude Code transcript shapes → synthetic fixtures per version | QA-Automation-Agent | C1 | Risk: Med | P0
- [ ] **HUN-22** — Repo skeleton: package layout, CLI + MCP entry points sharing one core | Backend-Implementer | C1 | Risk: Low | P0
- [ ] **HUN-23** — `claude_code` source: streaming, incremental, metadata-only parser with abstain counts | Backend-Implementer | C2 | Risk: High | P0
- [ ] **HUN-24** — Store + metrics by day/model/project/tool; dated price table | Backend-Implementer | C2 | Risk: Med | P0
- [ ] **HUN-28** — Leak test suite: planted secrets/text never reach any output or store row | QA-Automation-Agent | C1 | Risk: High | P0
- [ ] **HUN-25** — Rules engine + v1 rule pack + improvement plan | Backend-Implementer | C2 | Risk: Med | P1
- [ ] **HUN-26** — MCP tools (6) + CLI commands over the core | Backend-Implementer | C1 | Risk: Low | P1
- [ ] **HUN-27** — Static HTML dashboard (charts, flows, warnings, plan) | Frontend-Implementer | C2 | Risk: Low | P1
- [ ] **HUN-29** — Scheduled ingest so the 30-day transcript cleanup loses nothing | Backend-Implementer | C1 | Risk: Med | P1

### In Progress

### Ready for Review

### Done

---

## Blocked

| Task | Reason | Waiting on |
|------|--------|-----------|

---

## Stage Tracker

| Stage | Status |
|-------|--------|
| 0.5 Grilling → Brainstorming | 🔄 In Progress (requirement grill running) |
| 1 Environment Setup | ⬜ Not Started (board connected early) |
| 1.5 Sub-Agent Architecture | ⬜ Not Started |
| 2 Planning (/plan) | ⬜ Not Started |
| 3 Execution | ⬜ Not Started |
| 4 Review | ⬜ Not Started |
| 5 Integration & Verify | ⬜ Not Started |
