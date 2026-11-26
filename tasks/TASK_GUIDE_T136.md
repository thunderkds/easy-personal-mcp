# TASK_GUIDE — T136: Git diff 1 — read-only diff routes (server)
**Date**: 2026-11-26
**Complexity Level**: C2
**Risk Level**: High
**Priority**: P1
**Assigned agent**: Supervisor (DDR-0011)
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Read `PROJECT_SPEC.md`, this file, `docs/ddr/0010-git-diff-view-show-never-store.md` and `BRAINSTORMING_LOG_GITDIFF.md`.

---

## Requirement (Pillar 1)

"the git diff for the repos is important, so I want to have this feature in this our application" (owner, 2026-11-26). Owner choices: full compare, Changes tab and Live button, view only, file list then one file.

**Restated intent**:
> The dashboard can ask the app for a repo's changed files and one file's patch, in four compare modes, without the app changing the repo or running anything the repo configures.

**Out of scope**: the page (T137); any git write; fetch; MCP, CLI, Python app; storing anything.

**Requirement Refs**: FR-139, FR-140, NFR-002 (seventh exception), NFR-008.

### Requirement Fidelity Gate
- [x] Restated intent matches the request: auto (Supervisor), from the owner's four answers
- [x] Terms align with the glossary
- [x] Every criterion traces to the requirement
- [x] Refs exist in `PRD.md`

---

## Dependencies & Reachability

**Depends on**: `None`
**Entry point**: `/api/diff/`

---

## Acceptance Criteria

| # | Criterion | Traces to |
|---|-----------|-----------|
| 1 | Uncommitted lists staged, unstaged and untracked files apart, with status and counts; renames, deletions and binary files are marked | FR-139 |
| 2 | Branch compares merge base with HEAD; commit compares with the first parent (root commit included); range compares two refs | FR-139 |
| 3 | One listed file answers its patch; a path that is not listed is refused | FR-139 |
| 4 | Worktrees, branches, default base and commits are listed; `tree` picks a worktree | FR-139 |
| 5 | A repo is reached only by workspace + repo number or session id; every route is behind the guard | FR-139, NFR-008 |
| 6 | Planted filter, external diff, textconv, fsmonitor, hook and signature programs never run; the index bytes do not change | FR-140 |
| 7 | A ref that looks like an option, an unknown ref, an untracked symlink or FIFO give a fixed refusal or no content | FR-140 |
| 8 | Caps cut and say so; hidden characters are visible escapes; no answer holds an absolute path | FR-139, NFR-002 |

---

## Evaluation & Acceptance

### Verification Command

```bash
cd rust && cargo test -p easy-personal-core --test repo_diff && cargo test -p easy-personal-rs diff_routes
```

---

## Approach

**Pattern reference**: `rust/crates/app/src/transcript_route.rs` (show-never-store route), `rust/crates/core/src/report_files.rs` (workspace repo lookup, no-symlink open).
**Vital slice**: file list and one file's patch.
**Cut list**: side-by-side, highlight, search, paging inside one file.

---

## Edge Case Checklist

See `BRAINSTORMING_LOG_GITDIFF.md`.
- [ ] Pin-test pre-flight: none expected (new files; one `merge` line in `api.rs`)

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/repo_diff.rs` | new: git reading |
| `rust/crates/core/tests/repo_diff.rs` | new tests |
| `rust/crates/core/src/{lib,transcript}.rs` | module line; newest folder of a transcript |
| `rust/crates/app/src/diff_routes.rs` | new: routes |
| `rust/crates/app/src/{main,api}.rs` | module and merge lines |

## Files Must NOT Touch

Store schema, ingest, `src/easy_personal/`, `.claude/`, `skills/`.
