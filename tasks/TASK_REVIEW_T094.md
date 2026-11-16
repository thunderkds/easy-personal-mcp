# TASK_REVIEW — T094: Waiting sessions 1 — the *your turn* state and the question text on `GET /api/live`

> Sibling of `tasks/TASK_GUIDE_T094.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Stage 0 questions answered from the request and the repo: the request is the owner's three messages of 2026-11-16 — FACT: PRD.md:313 "Decided by the owner 2026-11-16 (\"go [admin], with question text\")"
2. Width of the question-text exception: only `questions[].question`, in memory, on `GET /api/live` and the Live view only — DEBATED: question text only, Live view only (Advocate 1: question text only, Live view only; Advocate 2: question text only, Live view only; Advocate 3: question text only, Live view only, 4 × 300 characters) ⚠ decided for you — please review
3. The text is given to the stream by a separate read; `LiveTail::snapshot()` never holds it — DEBATED: separate read, stream only (Advocate 1: separate read, stream only; Advocate 2: separate read, stream only; Advocate 3: separate read, stream only) ⚠ decided for you — please review
4. *Your turn* is given only to a session of typed work — FACT: PROJECT_SPEC.md:74 "*Agent work* = a sidechain record, or any record of a session whose **entrypoint** starts with `sdk` (headless)."
5. *Your turn* adds no timer of its own: it ends with the session's place in the live list — RULE: PRD.md § Non-Functional Requirements (NFR-012: "Live attention states add no new polling loop, thread or SSE route")
6. A DDR (0005), not an ADR — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (row 3)
7. Base team, own task breakdown (T094, T095, T096), Requirement Fidelity Gate signed `auto (Supervisor)` — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (rows 11-13)
Checks: <filled at Stage 4/5>
Memory: <filled before the run ends>
Not done: merge (admin: after the auto verify pass); auto verify: <not started>



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
