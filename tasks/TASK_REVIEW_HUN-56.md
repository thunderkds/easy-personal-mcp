# TASK_REVIEW — HUN-56: README install, usage and deploy guide

> Sibling of `tasks/TASK_GUIDE_HUN-56.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| easy-verifier score (advisory) | ☐ recorded / ☐ N/A | [overall result, abstaining dimensions, report path — or `N/A — <reason>` if the server is down; never blocks Done] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (verbatim prior `README.md`, 16 lines, captured 2026-10-22 08:49:35 UTC via `date -u`, before any implementation commit):

> # easy-personal-mcp
>
> The Easy Kit plug-in whose subject is **you**, not a repo. It reads your local agent activity and turns
> it into deterministic metrics, warnings and an improvement plan, shown on a local dashboard and through
> MCP tools. From v1.1 it also manages your personal tool connectors (email, messaging…) with secrets
> kept in the OS keychain and injected only at launch.
>
> Local-first · no network at runtime · metadata only, never message content · no LLM inside.
>
> Status: planning. See `PRD.md` (what and why), `PROJECT_SPEC.md` (how), `BRAINSTORMING_LOG.md`
> (how the direction was chosen) and `docs/ddr/` (design decisions).
>
> Part of the Easy Kit family: kernel [personal-agentic-claude](https://github.com/thunderkds/personal-agentic-claude),
> plug-ins [easy-ui-mcp](https://github.com/thunderkds/easy-ui-mcp),
> [easy-verifier-mcp](https://github.com/thunderkds/easy-verifier-mcp),
> [easy-context-mcp](https://github.com/thunderkds/easy-context-mcp).

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
