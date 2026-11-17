# TASK_REVIEW — T108: Kit workflow

> Sibling of `tasks/TASK_GUIDE_T108.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Direction: the map is one table in the Rust core that feeds `stage_of` and a new read-only `GET /api/kit/workflow`; not a copy in React, not parsed from project files — DEBATED: B (Advocate 1: B, one table + route, cites `PRD.md:180` "fixed *stage map* in code"; Advocate 2: B, same, cites `web/src/LiveView.jsx:31` "the page never works one out") ⚠ decided for you — please review
2. Placement and design: a second `// kit` tab **Workflow** built only from existing `app.css` tokens; no owner design was asked for (the owner said "I will check the final later") — DEBATED: A (Advocate 1: A, cites `docs/claude-md/pipeline-stages.md:201` "With no design and no rows, the baseline checks are its design pass"; Advocate 2: A, cites `memory/decisions.md:282` D7 and the owner's newer instruction) ⚠ decided for you — please review
3. One task (route + view in one worktree, two agents in turn) instead of the two tasks in the chat plan — RULE: `CLAUDE.md` § Karpathy Engineering Principles (Simplicity First: the route is a constant)
4. The agent of each board row is cut (the board route is not changed) — FACT: `rust/PROJECT_SPEC.md:62` "JSON parity with Python | High"
5. The live layer reads the existing stream only (no backend change for it) — FACT: `rust/crates/core/src/live.rs:968` "\"attention\": attention, \"stage\": s.stage, \"stage_skill\": s.stage_skill,"
6. Stage ids are exactly those of `stage_of` (no plain "0"; 0.5 is named Stage 0 · Define) so the two-way table test holds — FACT: `rust/crates/core/src/live.rs:85` "\"brainstorming\" | \"ideate\" => \"0.5\","
7. A Rust-board task edits `web/` (the page is shared) — FACT: `tasks/TASK_GUIDE_T104.md:16` "Read `web/src/LiveView.jsx`, the Settings view and the Workspaces page from T092"


## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | [paste the red run against the prior content, or the mutation diff (`git diff --stat`, non-empty) before each verdict] |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
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
