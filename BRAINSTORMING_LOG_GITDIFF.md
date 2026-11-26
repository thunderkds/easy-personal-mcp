# BRAINSTORMING_LOG_GITDIFF.md
**Generated**: 2026-11-26
**Task / Context**: Git diff view in the app (Rust app + web dashboard)
**Skill**: `Skill({ skill: "brainstorming" })`
**Tier**: Standard. **Risk**: High (file contents reach the page; git runs inside repos whose config can start programs).

---

## The Problem Space

The owner wants to read a repo's changes in the app (owner, 2026-11-26: "the git diff for the repos is important").

Decided by the owner (AskUserQuestion, 2026-11-26):

- **Compare**: uncommitted (staged, unstaged, untracked), branch vs main, one commit, any two refs.
- **Where**: a new *Changes* tab for workspace repos and their worktrees, and a *Changes* button on a Live session.
- **Actions**: view only.
- **Privacy**: file contents are shown on the local dashboard only, never stored or logged (new NFR-002 exception).

Fixed constraints: read-only on every repo; `127.0.0.1` only; NFR-008 guard; CSP `default-src 'self'`; no new MCP tool; Rust app only (as for the write sessions and the terminal).

Today there is no git diff view or route. `web/package.json` holds no diff or highlight library.

---

## Evidence: what a repo can run during `git diff` (git 2.53.0, probe on a scratch repo)

| Command | Repo-configured program ran? |
|---|---|
| `git diff` | yes: clean filter and `diff.external` |
| `git diff --no-ext-diff --no-textconv` | **yes: clean filter** |
| the same plus `-c filter.<name>.clean= -c filter.<name>.process= -c filter.<name>.required=false` | no |
| `git diff --cached --no-ext-diff --no-textconv` | no |
| `git diff --no-ext-diff --no-textconv <commit> <commit>` | no |

So the two safety flags are not enough for a working-tree diff. The existing `git_cmd` helper
(`rust/crates/app/src/app_session.rs`) switches off `core.fsmonitor` and the untracked cache, not filters.

---

## Alternative Paths

| Option | Name | Summary | Invasiveness | Code Volume (estimate) | Regression Risk | Recommended? |
|--------|------|---------|--------------|------------------------|-----------------|--------------|
| A | One patch | One route returns the whole patch text; the page splits and colours it | Low | ~600 lines | Low | |
| B | File list, then one file | Route 1: changed files with counts. Route 2: one file's hunks on demand. Route 3: refs and commits | Medium | ~1100 lines | Low | yes |
| C | Uncommitted text only | One route, one `<pre>`; compare modes later | Low | ~300 lines | Low | |

### Option A — One patch
**Approach**: `GET /api/repo/diff?...` runs one `git diff -p` and returns text.
**Pros**: fewest routes; one git run, so the answer is one consistent moment.
**Cons**: a lock file or a generated file fills the size cap and hides later files.
**Why it might fail**: a branch vs main compare on a large branch is cut in the middle, with no way to reach the rest.

### Option B — File list, then one file
**Approach**: the list comes from `--numstat -z` / `--name-status -z`; a file's patch is read when it is opened.
**Pros**: large changes stay usable; caps apply per file; the list is cheap to refresh.
**Cons**: more routes and UI state.
**Why it might fail**: the tree changes between the list and the file read; the page must say so and refresh.

### Option C — Uncommitted text only
**Approach**: smallest read-only slice.
**Pros**: quick.
**Cons**: does not meet the owner's *full compare* choice.
**Why it might fail**: it narrows a decided requirement. Rejected unless the owner re-scopes.

---

## 50% Rule Check

Option B at half the code: no side-by-side view, no syntax highlight, no ref search box (two plain
selects filled from route 3), no live auto-refresh (a *Refresh* button). One shared view component serves
the tab and the Live panel.

---

## Recommended Path

**Option B — File list, then one file.** It is the only option that meets *full compare* and stays usable on large changes.

Cut list: side-by-side view, syntax highlight, word-level diff, auto-refresh, any write action, Python app, MCP/CLI surface.

---

## Surgical Scope

Files that **should** be touched:

- `rust/crates/app/src/` — a new diff module and its routes; one line in `api.rs` `routes()`
- `web/src/` — a new Changes view; one tab entry in `App.jsx`; one button in `SessionPanel.jsx`
- `PRD.md`, `PROJECT_SPEC.md` (NFR-002 exception), `docs/ddr/0010-*.md`

Files that **must not** be touched:

- `rust/crates/core` store schema and ingest — nothing is stored
- `src/easy_personal/` — Rust app only
- `.claude/`, `skills/`, `templates/` — kit-managed

---

## Edge Case Checklist for TASK_GUIDE

- [ ] Planted programs in repo config never run: clean/process filter, `diff.external`, textconv, `core.fsmonitor`, hooks, submodule config (`--ignore-submodules=all`)
- [ ] A filter name that cannot be overridden with `-c` (holds `=` or a line break): refuse the working-tree compare
- [ ] Switching filters off can show an LFS file as changed: say that filters are off
- [ ] The request never carries a path of a repo: workspace + repo number + worktree number, or a session id
- [ ] A ref is resolved with `rev-parse --verify --end-of-options` to a commit id; only the id reaches `git diff`
- [ ] A file path must be one from the list answer; passed after `--` as `:(literal)`
- [ ] Untracked files: no symlink followed, regular files only, a FIFO never blocks
- [ ] `GIT_OPTIONAL_LOCKS=0` (index never written), `GIT_NO_LAZY_FETCH=1` (no network)
- [ ] Time limit and caps per answer (files, bytes, lines); a cut answer says it is cut
- [ ] Binary files, non-UTF-8 bytes, renames, mode changes, very long lines, names with spaces or line breaks
- [ ] Empty repo (no commit), detached HEAD, no `main` branch (`master` or none), merge commit, unrelated histories
- [ ] Hidden and bidi characters shown as visible escapes; text is never put in the page as HTML
- [ ] Planted-secret test: diff text appears in no store row, log, MCP or CLI answer

---

## Next Actions

1. PRD: user story, FR and the new NFR-002 exception; DDR-0010.
2. Two tasks: T136 server routes (C2, High), then T137 Changes tab and Live button (C2, Medium).
3. `security-review` on T136.

---

## User Selection

> **Approved direction**: Option B — File list, then one file.
> Approved by user on 2026-11-26.
