# 0010. Git diff view: a repo's changes shown in the app, never stored

**Status**: Accepted
**Date**: 2026-11-26
**Deciders**: thunderkds (owner), Supervisor
**Related**: FR-139..FR-141 · NFR-002, NFR-008 · DDR-0007 · PRD question 48 · `BRAINSTORMING_LOG_GITDIFF.md`

---

## Context

The owner asked on 2026-11-26 for git diffs of their repos in the app. A diff is file contents, which NFR-002 keeps off every surface unless a named exception says otherwise. Git also runs programs named in a repo's own config; a probe (git 2.53.0) showed that `--no-ext-diff --no-textconv` still runs a repo's clean filter in a working-tree diff.

Gate: (1) hard to reverse: no; (2) surprising without context: yes (file contents on the page, git config overridden); (3) genuine trade-off: yes. 2 of 3: DDR.

---

## Decision

1. **Full compare, view only** (owner): uncommitted (staged, unstaged, untracked), branch vs base, one commit, any two refs. No stage, commit, discard, checkout or fetch.
2. **Two entry points** (owner): a *Changes* tab (workspace repos and their worktrees) and a *Changes* button on a Live session.
3. **Seventh NFR-002 exception**: file contents, file paths, ref names, commit subjects and author names of the selected repo are read on demand and sent to the dashboard page only. Never stored, logged, cached, or sent to MCP, the CLI, `/api/alerts`, the tray or a popup. Nothing is redacted: the page shows what `git diff` shows.
4. **File list, then one file** (owner chose path B).
5. **No repo program runs**: external diff, textconv, fsmonitor, hooks and signature checks are off; every configured filter is overridden by name; a filter name that cannot be overridden refuses the working-tree compare. Submodule work trees are not entered. A repo's diff order file is not read (git loads all of it, whatever its size).
6. **The request never names a folder**: it names a workspace repo by number, or a session id; a worktree by an id the server made.

---

## Alternatives Considered

| Alternative | Why not chosen |
|---|---|
| One patch for the whole compare | A large change is cut with no way to reach the rest |
| Uncommitted only | Does not meet the owner's full compare |
| Stage and commit from the app | Owner chose view only |
| A diff or git library crate | New dependency; the installed `git` already gives the owner's view |

---

## Consequences

- With filters off, a file kept through a filter (for example Git LFS) can show as changed when it is not; the answer says that filters are off.
- A repo config change between reading the filter names and running the diff is not covered (the owner's own machine).
- Accepted by the owner 2026-11-26 (AskUserQuestion, after the third review of T136): in a repo that uses git's split index, git renews the timestamp of `.git/sharedindex.<id>` when it reads the index. No byte changes; `git status` does the same. Refusing such repos was the other choice.
