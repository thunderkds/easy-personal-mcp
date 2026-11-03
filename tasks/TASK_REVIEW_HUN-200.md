# TASK_REVIEW — HUN-200: Dashboard forms UX + floating notifications + floating loading indicator

> Sibling of `tasks/TASK_GUIDE_HUN-200.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Track extended, Risk Low, C2 — RULE: `AGENTS.md` § Tracks (a UI component puts the task on the extended track); UI only, no stored data or API change
2. "The form" read as every dashboard form: the workspace form, Add connector and the agent accounts form — SAFER OPTION: `WorkspacesView.jsx` holds the only `<form>` elements, but Agents has the same result/error pattern; covering all keeps one style ⚠ decided for you — please review
3. Toasts in the bottom-right corner (full width at the bottom on a phone); success hides after 5 s, errors stay until closed, max 3 — RULE ⚠ (taste) decided for you — please review
4. The Confirm/Cancel plan stays inline, moved next to the form's buttons, not a toast — SAFER OPTION: a decision the owner must take should not disappear on a timer ⚠ decided for you — please review
5. Failed page loads stay inline; the Rail card's own loading and the Live link status stay inline — SAFER OPTION: they are page or card state, not notifications ⚠ decided for you — please review
6. Stage 0.5 grill/brainstorm answered from the request and code: PRD US-028 + FR-065 written by the Supervisor; brainstorming took the one recommended direction — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` rows 2 and 4 ⚠ (taste) decided for you — please review
7. Sub-agent spawned on this session's CLI and model — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` row 14
Checks: pending
Not done: merge (yours); /verify (yours)


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
