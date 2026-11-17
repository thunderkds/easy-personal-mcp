# TASK_REVIEW — T096: Waiting sessions 3 — tray rows per session, the *your turn* alert, a popup you can click, and the focus request

> Sibling of `tasks/TASK_GUIDE_T096.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

> **Not started.** It waits for T095's open question (the click mechanism). The block below becomes the report when the task starts.

<!-- held until the task starts (a task that has not started is not marked as an auto-mode run)
## Auto decisions

Mode: auto
1. The *your turn* popup waits until the state has held 60 s on the engine's monotonic clock — DEBATED: hold 60 s, monotonic first-seen time (Advocate 1: hold 60 s, monotonic first-seen time; Advocate 2: hold 60 s, monotonic first-seen time) ⚠ decided for you — please review
2. At most 5 session rows, as the menu's alert block shows 5 — FACT: rust/crates/app/src/tray.rs:220 "pub(crate) const ALERTS_SHOWN: usize = 5;"
Not done: merge (admin: after the auto verify pass); auto verify: not started (the task waits for T095's answer)
-->

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

**BEFORE** (2026-11-17T02:43:20Z, branch `T096` at `f5f839f`, no implementation commit yet): a throwaway test (not committed) printed `tray::view` for a snapshot with one `waiting` session (`shop`), one `your_turn` session (`blog`) and one `failing` session (`api`): one count row for waiting, one for failing, and no row at all for the your-turn session or for any single session. The alert engine has no `your_turn` kind and `Notifier::notify(summary, body)` has no action.

```
$ cd rust && cargo test -p easy-personal-rs --bin easy-personal-rs -- tray::tests::before_t096_print_rows --nocapture
running 1 test
state: problem
Info("Easy Personal · problem")
Separator
Info("3 sessions active")
Action("waiting", "1 waiting for you · shop")
Action("failing", "1 failing · api")
Info("5h limit · no data · 7d · no data")
Info("Machine · no data")
Separator
Action("open", "Open dashboard")
Check("pause", "Pause alerts", false)
Separator
Info("127.0.0.1 · local only")
Action("quit", "Quit")
test tray::tests::before_t096_print_rows ... ok
```

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
