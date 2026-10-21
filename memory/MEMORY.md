# MEMORY.md — Hot-Tier Memory Index

> **Rules**: Supervisor-only writes. Max 42,000 characters — a ratchet: `/compact-memory` may lower
> it, never raise it to fit growth. One-line summaries + links to cold files.
> Each spawn prompt carries a slice of these lines (craft-spawn-prompt's memory_slice.py) and this
> path to read in full only when the slice falls short; the whole file is not pasted.
> Updated by the Supervisor — prompted by the PostToolUse hook on `git push` / `git merge` (diff-driven pass), or via the `/compact-memory` skill.

---

## Memory Architecture

- [decisions.md](decisions.md) — code + infra architectural decisions (the "why")
- [glossary.md](glossary.md) — canonical biz domain terms and core domain models
- [learnings.md](learnings.md) — specs/requirement clarifications, patterns, gotchas

---

## Index

<!-- Format: - [Title](cold-file.md#section) — one-line summary.
     Target ≤150 chars/entry. Advisory: reported by the size test, never enforced.
     The enforced gate is the 42,000-character whole-file budget above. -->

### Decisions
- [Local merge to main, push in batches](decisions.md#2026-10-21--stage-35-git-flow-local-merge-to-main-push-in-batches) — no per-task PR; board goes In Progress → Done (no review status in Linear).
- [v1 = Python core + React/Vite page, on demand](decisions.md#2026-10-21--v1--python-core--reactvite-page-on-demand) — Option B; page on 127.0.0.1 only while `easy-personal dashboard` runs; systemd timer ingests.
- [Read-only MCP client of easy-context](decisions.md#2026-10-21--read-only-mcp-client-of-easy-context) — only list_indexes/get_stats on 127.0.0.1:8767; never index_path; DDR-0002.
- [Native runtime; writes via `claude mcp` user scope](decisions.md#2026-10-21--native-runtime-connector-writes-only-via-claude-mcp-user-scope) — keychain + Claude Code only; no Codex/Cursor; DDR-0001.
- [Board scoped to Linear project P-HUN-3](decisions.md#2026-10-21--board-scoped-to-linear-project-easy-personal-mcp-p-hun-3) — empty `project` in board.json pulls the whole team.

### Learnings
- [Merge gate blocks while any task In Progress](learnings.md#2026-10-21--merge-gate-blocks-while-any-task-is-in-progress) — Done after evidence, then merge; parallel tasks finish together.
- [Skills the Supervisor cannot run](learnings.md#2026-10-21--skills-the-supervisor-cannot-run-itself) — /verify is user-only; security-review inline; no Skill tool in sub-agents.
- [easy-context registered (local scope)](learnings.md#2026-10-21--easy-context-registered-for-this-repo-local-scope) — restart to load tools; index the repo next session.
- [Ignore cost-state.totalCostUSD](learnings.md#2026-10-21--ignore-claude-codes-cost-statetotalcostusd) — cost only from the labelled price table.
- [Board pull overwrites `## Blocked`](learnings.md#2026-10-21--board_syncpy-pull-overwrites-the--blocked-table) — keep external blockers in the guide + PROJECT_SPEC Tasks.
- [Git remote is `github`](learnings.md#2026-10-21--git-remote-is-github-not-origin) — `git push -u github <branch>`.
- [Linear escapes backticks in titles](learnings.md#2026-10-21--linear-escapes-backticks-in-issue-titles) — cosmetic in KANBAN cache.
