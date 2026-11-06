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
Checks: tests `cd web && npx vitest run` — 682 passed, `npm run build` ok (Supervisor re-run on 48111a8); full Python suite — 1614 passed, 1 xfailed, 0 skipped (web build present); reviews code-review — 0 P0, 0 P1, 0 P2, 2 P3 (MCP row shows the raw tool name and the `server · tool` label; implementer skipped the DDR-0004 extra read), lane `C1, 3 files, 207 lines, FITS`; security-review not required (Risk Low); easy-verifier — N/A (MCP refuses worktree paths); real run by the implementer, screenshots checked by the Supervisor: chip `Stage 4 · code-review` → `Stage 3 · tdd` live, feed `main Skill tdd`, `main Agent backend-developer`, `backend-developer mcp__easy-verifier__score easy-verifier · score`, no horizontal scroll at 1280 and 375 (auto-checked, not /verify)
Not done: merge (yours); memory update; Done

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/LiveView.test.jsx` — new `describe('kit stage chip and call labels (FR-084)')` (15 tests: AC1 chip / null / half stage / live event before snapshot, AC2 chip update + memo, AC3 main / sub-agent / MCP / no label / error, AC4 planted fields, AC5 footnote, edge cases, CSS tokens) plus 2 adapted sidechain tests. 84 pass (69 before, 15 were failing). |
| Verification command run | ☑ pass | 2026-11-06T12:11:19Z `cd web && npx vitest run src/LiveView.test.jsx && npx vitest run && npm run build` → `Test Files 1 passed (1) / Tests 84 passed (84)`; full suite `Tests 682 passed (682)`; build `✓ built in 136ms` (`index-DY_TcM5v.js 327.08 kB │ gzip: 98.74 kB`, before: 326.54 kB). |
| Negative cases hold | ☑ pass | null stage → no chip; live event before a snapshot → no chip; unexpected label shapes → no label; `agent` null + `is_sidechain` → "sub-agent"; planted text in other fields never on the page (`web/src/LiveView.test.jsx`) |
| verify | ☑ pass | owner-invoked `/verify HUN-234` 2026-11-06 on 65fc852, temp install (0.4.0) with the built page: `ping` ok; MCP stdio 12 tools + ping ok; `--host 0.0.0.0/localhost/::1` exit 2; dashboard on temp homes, headless Chrome over CDP, calls appended with the panel open: row chip `Stage 4 · code-review`, then `Stage 3 · tdd` after the `tdd` call; feed `main Agent qa-expert`, `main Bash uv`, `main Skill tdd`, `qa-expert mcp__easy-verifier__score easy-verifier · score`; no horizontal scroll (1280: 1265/1265, 375: 375/375); planted VERIFYSECRET not on the page; one 375 px run missed the row click (script timing), re-run opened it (`aria-expanded = true`) — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `main...HEAD` diff (`LiveView.jsx`, its test, `app.css` `.live-*`) + entry `App.jsx:25`; server untouched (HUN-233) |
| Full smoke suite still green (no regression) | ☑ pass | vitest `682 passed`; pytest `1614 passed, 6 deselected, 1 xfailed` (Supervisor, 48111a8) |
| **Companion score (easy-verifier)** | ☑ N/A | not scored: the MCP refuses worktree paths (`target repository path does not exist`, HUN-233) |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Real run, headless Chrome 157, throwaway store, 2 fabricated sessions (planted `SECRET-planted` in every tool input and meta `description`; `document.body.textContent.includes('SECRET')` = false at both widths). BEFORE `reports/HUN-234/before-rows-1280x720.png`, `before-panel-1280x720.png`. AFTER `reports/HUN-234/after-rows-1280x720.png`, `after-panel-1280x720.png`, `after-rows-375x812.png`, `after-panel-375x812.png` (worktree `reports/`, gitignored). Observed: row shows chip "Stage 3 · tdd" beside id/project (it showed "Stage 4 · code-review" until a `tdd` Skill call arrived while the page was open, then updated); row height unchanged at 1280 (chip fits the header line); panel feed rows: `main Skill tdd`, `main Agent backend-developer`, `main Bash pytest`, `backend-developer mcp__easy-verifier__score` + `easy-verifier · score`, `backend-developer Bash uv`; plain session has no chip. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `.live-stage`, `.live-who`, `.live-label` in `web/src/app.css` use only `var(--…)` (`--color-accent-line`, `--color-secondary`, `--color-accent`, `--font-mono`, `--fs-label`, `--radius-sm`, `--space-2`); vitest `design` block (no hex/rgb in `.live-` rules, 4.5:1 contrast for every `.live-` text colour on bg/raised/selected) and the new chip-token test pass. Chip is real text; feed stays one `aria-live` region (test). |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280×720 and 375×812 in the real run: `documentElement.scrollWidth/clientWidth` = 1265/1265 and 375/375 with the panel open (no horizontal scroll). At 375 the chip stays on the project line (it fits; the row is `flex-wrap`, so it wraps under the name when the project name is long); long MCP tool name wraps in the feed (`overflow-wrap: anywhere`). |

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

**AFTER** (2026-11-06T12:11:19Z, same command): `Test Files 1 passed (1) / Tests 84 passed (84)`; full suite `Tests 682 passed (682)`. Real run 2026-11-06T12:11:27Z: page text of the open panel `main Skill tdd || main Agent backend-developer || main Bash pytest || backend-developer mcp__easy-verifier__score easy-verifier · score || backend-developer Bash uv`; row `kitrun01 demo-app Stage 3 · tdd active · just now`; footnote "Only tool names, skill, sub-agent, MCP and command names, counts and times are shown, never message or tool content."; screenshots under `reports/HUN-234/after-*.png`.

**DELTA**: The owner can see on each Live row which kit stage a session is in, and in the panel feed who made each call (main or the sub-agent type) and what it was (skill, sub-agent, command, MCP server · tool).

**WITNESS**: Supervisor (main session), 2026-11-06 — re-ran vitest, the build and the full Python suite on 48111a8 and checked the AFTER screenshots `reports/HUN-234/after-*.png` in the task worktree (trace `memory/event-trace/HUN-234.jsonl`)