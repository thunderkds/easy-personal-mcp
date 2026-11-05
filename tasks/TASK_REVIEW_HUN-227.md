# TASK_REVIEW — HUN-227: Share repeated store computations across dashboard reads (NFR-013 server)

> Sibling of `tasks/TASK_GUIDE_HUN-227.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Extended track — RULE: `AGENTS.md` § Tracks ("The extended track applies if any of: C2 or above; Risk Medium or High")
2. Risk Medium (not Low): the change wraps functions in `metrics.py` and `owners.py`, used by most reads — RULE: `docs/claude-md/pipeline-stages.md:137` "A hub touch raises Risk a level even when the edit itself is small"
3. Fix path: a single-flight memo in core for `_messages` and `_stored_branches`, keyed by `PRAGMA data_version` on a watcher connection, not by `MAX(events.id)` — DEBATED: option A (Advocate 1: A, shared computations in core, with a key that catches updates, deletes and scope-table edits; Advocate 2: B, whole-response cache in `api.py`, which conceded B alone cannot meet NFR-013 on an active store) ⚠ decided for you — please review
4. Brainstorm direction taken without owner approval — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (row 4: "Take the recommended direction and list the alternatives in the report"); alternatives B and C in `BRAINSTORMING_LOG_PERF.md`
5. Requirement Fidelity Gate signed `auto (Supervisor)` — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (row 13)
6. Spawn model `claude-sonnet-5-5` — FACT: `PROJECT_SPEC.md:165` "| claude | `claude` | `claude-haiku-5-5` | `claude-sonnet-5-5` | `claude-sonnet-5-5` | `claude-opus-5-5` |"
7. One-time setup (`fewer-permission-prompts`, `update-config`) skipped — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (row 5)
Checks: (filled at Stage 4)
Not done: merge (yours); auto verify: running (T182)


## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: [pasted timestamped command output showing the thing absent/failing, captured before the
first implementation commit] OR [verbatim excerpt of the prior content, for non-executable changes]

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
