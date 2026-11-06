# TASK_REVIEW — HUN-234: Live view kit stage chip and call labels in the session feed (FR-084)

> Sibling of `tasks/TASK_GUIDE_HUN-234.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
Owner instruction (not a decision): merge HUN-235, then start HUN-234 in auto mode ("merge, then start HUN-234 [auto]", 2026-11-06).
1. Requirement Fidelity Gate stays signed by the owner (row chip + panel feed, 2026-11-06) — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` row 13
2. guide gained the HUN-233 wire shapes and the "`agent` null + `is_sidechain` → sub-agent" edge case — FACT: tasks/TASK_REVIEW_HUN-233.md code-review P3 "sidechain line in a main file shows `agent: null` → handle in HUN-234"
3. spawned on claude · C1 → claude-sonnet-5-5 — FACT: PROJECT_SPEC.md:165 "| claude | `claude` | `claude-haiku-5-5` | `claude-sonnet-5-5` |"
Checks: pending
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

**BEFORE** (2026-11-06T12:08:20Z, `cd web && npx vitest run src/LiveView.test.jsx`, AC 1–5 tests written first, no implementation yet; local clock shows 19:08):

```
 FAIL  ... > attention (FR-050) > AC7: a failed result row shows its tool name, not tool_result   (adapted: now expects "sub-agent")
 FAIL  ... > feed > shows an error result and an agent (sidechain) call, and counts only tool_use as a call   (adapted)
 FAIL  ... > kit stage chip and call labels (FR-084) > AC1: a session with a stage shows "Stage 4 · code-review" ...; no stage, no chip
 FAIL  ... > AC1: a half stage and a stage without a skill name read as the server sent them
 FAIL  ... > AC2: a new snapshot with a new stage updates the chip, and a snapshot that clears the stage removes it
 FAIL  ... > AC2: a snapshot that changes only one session's stage draws only that row; live events do not touch the chip
 FAIL  ... > AC3: a call from a sub-agent shows its type, the tool and the label
 FAIL  ... > AC3: a failed result keeps its error tag next to who, tool and label
 FAIL  ... > AC3: an MCP label reads "server · tool"
 FAIL  ... > AC3: a Skill call from the main thread reads "main", the tool and the label
 FAIL  ... > AC4: text from any other event field never appears, in the row or the panel
 FAIL  ... > AC5: the footnote names the shown names and still says no message or tool content
 FAIL  ... > a long label and a long sub-agent type wrap instead of widening the panel
 FAIL  ... > edge: agent null but is_sidechain true reads "sub-agent", never "main"
 FAIL  ... > the chip is drawn from tokens only, in the label font, and the feed stays one aria-live region
 Test Files  1 failed (1)
      Tests  15 failed | 69 passed (84)
```

BEFORE screenshots (real `easy-personal dashboard` run on the unchanged page, throwaway store, 2 fabricated metadata-only sessions, taken 2026-11-06T12:10Z; HUN-233 server already sends `label`, `agent`, `stage`): `reports/HUN-234/before-rows-1280x720.png`, `before-panel-1280x720.png`, `before-rows-375x812.png`, `before-panel-375x812.png` in the task worktree (`reports/` is gitignored). Observed: the row has no stage chip although session kitrun01 ran a `code-review` Skill call; the open panel's feed reads "Bash agent", "mcp__easy-verifier__score agent", "Bash", "Agent", "Skill" — no label, no sub-agent type, no "main"; the footnote says "Only tool names, counts and times are shown".

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/HUN-234.jsonl`, never the
implementing agent alone]
