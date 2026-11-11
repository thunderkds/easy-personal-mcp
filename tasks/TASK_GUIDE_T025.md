# TASK_GUIDE — T025: Fix the stale "no CLI/API yet" wording in `rust/README.md`
**Date**: 2026-11-11
**Complexity Level**: C0
**Risk Level**: Low
**Priority**: P2
**Assigned agent**: common-infrastructure
**Agent guide**: `agents/common-infrastructure.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/common-infrastructure.md`
5. Read `rust/README.md` completely
6. Do not open `~/.easy-personal` or any `~/.claude*` folder

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (Stage 4 open note on T022, owner's `[admin]` close-out, 2026-11-11): the README heading `## Ported library functions (core crate, no CLI/API yet)` is stale: the core crate now has a CLI, an HTTP API and an MCP server, and a window.

**Restated intent**:
> Every sentence in `rust/README.md` that says something is "not yet" available, "no CLI/API yet", or "not ported yet" is true of the code in this repo today, or is reworded to be true.

**Out of scope**: any change outside `rust/README.md` (code, comments, other docs); adding new sections; reflowing text that is already true.

**Requirement Refs**: none (documentation).

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] No Requirement Refs to cover

---

## Dependencies & Reachability

**Depends on**: None
**Entry point**: Standalone — N/A: a documentation file, not called from code

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | The heading `## Ported library functions (core crate, no CLI/API yet)` no longer says "no CLI/API yet" (for example `## Ported library functions (core crate)`) | the stale heading |
| 2 | Every other "yet" / "not ported" / "no browser is opened yet" / "Not ported yet" claim in `rust/README.md` is checked against the code (`rust/crates/app/src/main.rs` subcommands, `mcp.rs` tool list, `api.rs` routes, `window.rs`) and is left as is when true, or corrected when false. Your report lists each one with TRUE / CORRECTED and the file:line you checked | no other stale claim |
| 3 | `git diff` touches only `rust/README.md` and `tasks/TASK_REVIEW_T025.md`; the diff is small (heading plus only the corrections criterion 2 forced) | surgical |
| 4 | No file link, command or code fence in the README is changed | surgical |

---

## Evaluation & Acceptance

### Success Criteria

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | `grep -n "no CLI/API yet" rust/README.md` | no match | run it |
| 2 | `git diff --stat` | `rust/README.md` plus the review file only | run it |

### Verification Command

```bash
# ci: skip — documentation only
grep -c "no CLI/API yet" rust/README.md; git diff --stat -- rust/README.md
```

---

## Approach

**Vital slice**: grep the README for "yet", "not ported", "not implemented"; check each against the code; change only what is false.
**Cut list**: no rewrite, no new sections, no tidy-up of text that is true.

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/README.md` | the heading and any claim criterion 2 finds false |
| `tasks/TASK_REVIEW_T025.md` | Demonstration BEFORE only (verbatim prior text of what you change) |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/crates/**`, `rust/parity/**`, `rust/spikes/**`, `src/**`, `web/**`, `tests/**` | not part of this task |
| `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Run the two Success Criteria commands and paste the output. Paste the list of checked claims.

## Completion Checklist

- [ ] BEFORE (verbatim prior text) filled in `tasks/TASK_REVIEW_T025.md` before the edit
- [ ] Implementation done
- [ ] Every "yet"/"not ported" claim checked and listed
- [ ] `git diff --stat` pasted
- [ ] Supervisor notified: task ready for Stage 4 review
