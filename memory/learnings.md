# learnings.md — Cold Tier: Clarifications, Patterns & Gotchas

> **Rules**: Supervisor-only writes. Each entry dated (`YYYY-MM-DD`) and citing the file/task it came from (the diff-driven pass greps this file by changed file path).

## Requirement Clarifications

### 2026-10-21 — Ignore Claude Code's `cost-state.totalCostUSD`
**Learning**: Owner confirmed: cost is only the labelled "API-equivalent cost" from the dated price table (FR-005); the transcript's own `totalCostUSD` is never read.
**Files**: `src/easy_personal/sources/claude_code.py`, `tasks/TASK_GUIDE_HUN-34.md`

## Patterns

### 2026-10-21 — Merge gate blocks while ANY task is In Progress
**Learning**: `.claude/hooks/pre_bash_block_unsafe_merge.py` refuses every `git merge`/`push` while any KANBAN row is In Progress, and needs a `verify` row containing "pass" for Ready for Review rows. With no review status in Linear (decision: In Progress → Done), move a task to Done only after its Evidence is complete, then merge. Parallel tasks must all reach Done before any of them merges.
**Files**: `.claude/hooks/pre_bash_block_unsafe_merge.py`, `PROJECT_KANBAN.md`

### 2026-10-21 — Skills the Supervisor cannot run itself
**Learning**: `/verify` is user-invoked only (disable-model-invocation); built-in `security-review` diffs `origin/HEAD` (remote here is `github`) on the current checkout, so the Supervisor reviews worktree diffs inline. Sub-agents have no Skill tool: write tdd red-green steps into spawn prompts instead of naming the skill.
**Files**: `skills/verify/SKILL.md`, `skills/craft-spawn-prompt/SKILL.md`

### 2026-10-21 — easy-context registered for this repo (local scope)
**Learning**: `claude mcp add --transport http easy-context http://127.0.0.1:8767/mcp --header 'X-Easy-Context-Root: ${PWD}'` (local scope, `~/.claude-personal/.claude.json`); tools load only after a session restart. Indexed 2026-10-21 (177 files, 1646 slices); retrieval rule in `CLAUDE.md → Context Retrieval`; re-index (`index_path`, incremental) after each merge to main.
**Files**: `PROJECT_SPEC.md`

### 2026-10-21 — `board_sync.py pull` overwrites the `## Blocked` table
**Learning**: The Blocked section of `PROJECT_KANBAN.md` is a fixed footer rendered on every pull; manual rows vanish. Record external blockers in the task's guide (`Depends on` external note) and in `PROJECT_SPEC.md` § Tasks instead (e.g. XR-1 → easy-verifier T120 in `tasks/TASK_GUIDE_HUN-42.md`).
**Files**: `PROJECT_KANBAN.md`, `scripts/board_sync.py`, `tasks/TASK_GUIDE_HUN-42.md`

### 2026-10-21 — Git remote is `github`, not `origin`
**Learning**: Push with `git push -u github <branch>`; `origin` does not exist in this repo.
**Files**: `.git/config`

### 2026-10-21 — Linear escapes backticks in issue titles
**Learning**: Titles with inline code come back as `` \` `` in the KANBAN cache; cosmetic only, the guide bodies are intact.
**Files**: `PROJECT_KANBAN.md`

## Gotchas
