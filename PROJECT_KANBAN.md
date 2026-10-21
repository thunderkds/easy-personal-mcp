# PROJECT_KANBAN.md
**Last updated**: 2026-10-21

> Compact task board. Full context lives in `PROJECT_SPEC.md`. Update this file whenever a task status changes.

---

## Board

> Task line format: **Txxx** — [title] | [agent] | C[0–3] | Risk: Low/Med/High | P[0–2]

### Todo
- [ ] **T001** — Spike: inventory Claude Code transcript shapes → synthetic fixtures per version | QA-Automation-Agent | C1 | Risk: Med | P0
- [ ] **T002** — Repo skeleton: package layout, CLI + MCP entry points sharing one core | Backend-Implementer | C1 | Risk: Low | P0
- [ ] **T003** — `claude_code` source: streaming, incremental, metadata-only parser with abstain counts | Backend-Implementer | C2 | Risk: High | P0
- [ ] **T004** — Store + metrics by day/model/project/tool; dated price table | Backend-Implementer | C2 | Risk: Med | P0
- [ ] **T008** — Leak test suite: planted secrets/text never reach any output or store row | QA-Automation-Agent | C1 | Risk: High | P0
- [ ] **T005** — Rules engine + v1 rule pack + improvement plan | Backend-Implementer | C2 | Risk: Med | P1
- [ ] **T006** — MCP tools (6) + CLI commands over the core | Backend-Implementer | C1 | Risk: Low | P1
- [ ] **T007** — Static HTML dashboard (charts, flows, warnings, plan) | Frontend-Implementer | C2 | Risk: Low | P1
- [ ] **T009** — Scheduled ingest so the 30-day transcript cleanup loses nothing | Backend-Implementer | C1 | Risk: Med | P1

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
| 0.5 Brainstorming | ✅ Done |
| 1 Environment Setup | 🔄 In Progress |
| 1.5 Sub-Agent Architecture | ⬜ Not Started |
| 2 Planning (/plan) | ⬜ Not Started |
| 3 Execution | ⬜ Not Started |
| 4 Review | ⬜ Not Started |
| 5 Integration & Verify | ⬜ Not Started |
