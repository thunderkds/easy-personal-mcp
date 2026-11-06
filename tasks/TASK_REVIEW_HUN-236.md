# TASK_REVIEW — HUN-236: Live view opens a session inline with one-line feed rows (FR-085)

> Sibling of `tasks/TASK_GUIDE_HUN-236.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
Owner instruction (not a decision): "look good now, but the line is too long with the logs, and the UX for click to show and hide is not good, let investigate and update [auto]" (2026-11-06).
1. inline expand under the row replaces the FR-072 side panel (new FR-085; PRD open question 29) — SAFER OPTION: fixes both complaints with one change (full-width feed; no layout shift; feed next to the tapped row on phones); the side panel was the owner's 2026-11-04 choice ⚠ decided for you — please review
2. MCP feed rows show only `server · tool` (raw name moves to the row `title`); time `HH:MM:SS` 24 h; one line per row with ellipsis — RULE: same Supervisor choice as 1 (owner: "the line is too long") ⚠ decided for you — please review
3. the expanded area shows the feed only (row bars already show the session's totals) — SAFER OPTION: smaller change, no lost data (totals stay on the row) ⚠ decided for you — please review
4. Complexity C2 (restructure of the Live view open/close), Risk Low (UI only, no data) — RULE: `agents/frontend.md` Complexity matrix
5. spawned on claude · C2 → claude-sonnet-5-5 — FACT: PROJECT_SPEC.md:165 "| claude | `claude` | `claude-haiku-5-5` | `claude-sonnet-5-5` | `claude-sonnet-5-5` |"
Checks: tests vitest 695 passed, build ok, pytest 1614 passed (Supervisor re-run on 6854a37); reviews code-review — 0 P0, 0 P1, 0 P2, 3 P3 (stale "side panel" CSS comments; who column cut on phones with no hover; page scrollbar narrows the list 15 px at 1280), lane `C2, 3 files, 544 lines, EXCEEDS (lines 544 > 500)` (mostly tests); security-review not required (Risk Low); easy-verifier — N/A; real run by the implementer, screenshots checked by the Supervisor (auto-checked, not /verify)
Not done: merge (yours); memory update; Done

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/LiveView.test.jsx` (inline expand, Escape/close/focus, one open, no side panel, arrow, one-line rows, MCP `server · tool`, `HH:MM:SS`, session leaving); BEFORE 37 failed / 60 passed (97), AFTER 97 passed |
| Verification command run | ☑ pass | `cd web && npx vitest run src/LiveView.test.jsx && npx vitest run && npm run build` → 97 passed, 695 passed (21 files), built (Supervisor re-run on 6854a37) |
| Negative cases hold | ☑ pass | Escape with nothing open does nothing; opening another row closes the first; the open session leaving resets the open id and draws nothing; unexpected label shapes show no label; `agent` null + `is_sidechain` → sub-agent |
| verify | ☑ pass | owner-invoked `/verify HUN-236` 2026-11-06 on 24c26a4, temp install (0.4.0): `ping` ok; MCP stdio 12 tools + ping ok; `--host 0.0.0.0/localhost/::1` exit 2; dashboard on temp homes, 2 sessions, calls appended after opening the row; headless Chrome at 1280×720 and 375×812: closed ▸ / open ▾ (`aria-hidden="true"`), `aria-expanded` true, region inside the row's `li`, 1 `.live-feed`, 0 `.live-panel`/`[data-panel]`; other row class and width unchanged on open (1280: same; 375: `live-session` 309 px both); feed rows one line, 25 px each: `21:32:51 qa-expert easy-verifier · score`, `main Skill tdd`, `main Bash uv`, `main Agent qa-expert` (raw `mcp__easy-verifier__score` only in `title`); Escape → collapsed, focus on row; Close → collapsed, focus on row; click again → collapsed; opening the other row closes the first (1 feed); scrollWidth = clientWidth (1265/1265, 375/375); VERIFYSECRET on page false, in dashboard log 0; screenshots `reports/HUN-236/verify-{closed,open}-{1280,375}.png` (gitignored) — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `main...HEAD` diff (`LiveView.jsx`, its test, `app.css` `.live-*`) + entry `App.jsx:25`; server untouched |
| Full smoke suite still green (no regression) | ☑ pass | vitest 695 passed; pytest `1614 passed, 6 deselected, 1 xfailed` (Supervisor, 6854a37) |
| **Companion score (easy-verifier)** | ☑ N/A | not scored: the MCP refuses worktree paths (`target repository path does not exist`, HUN-233) |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | BEFORE/AFTER screenshots in `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/worktrees/agent-ae2e9dc40b73c81aa/reports/HUN-236/` (before-w1280-open.png → after-w1280-open.png, before-w375-open.png → after-w375-open.png); Supervisor verdict: feed under the open row at full width, one line per call (rows 25 px, were 73/49/25 px), MCP rows `easy-verifier · score` only, arrow ▸/▾ |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | new rules `.live-arrow`, `.live-expanded`, `.live-feed > li` grid, cells use `var(--…)` tokens only (diff read by the Supervisor); one `aria-live` feed; arrow `aria-hidden`; row `aria-expanded`/`aria-controls` |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280×720 and 375×812: no page-wide horizontal scroll (375/375); row widths unchanged on open at 375 (335 → 335); at 1280 a long feed adds the page scrollbar (−15 px, no re-layout, P3) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-06T12:45:28Z (UTC) on branch `HUN-236`, before any change to `web/src/LiveView.jsx` / `app.css`. AC 1–7 tests written first (`web/src/LiveView.test.jsx`), run against the unchanged side-panel code:

```
$ cd web && npx vitest run src/LiveView.test.jsx
 FAIL  ... inline expand (FR-085) > AC1: a row is a native button; a click expands its feed inside its own card; aria-expanded and aria-controls follow
 FAIL  ... inline expand (FR-085) > AC1: opening another row closes the first; at most one .live-feed and one aria-live region
 FAIL  ... inline expand (FR-085) > AC2: Escape on the row collapses it and the focus stays on the row button
 FAIL  ... inline expand (FR-085) > AC2: Escape from inside the expanded area, and its Close control, collapse it and put the focus on that row button
 FAIL  ... inline expand (FR-085) > AC3: no side panel and no two-column grid; opening a row leaves a sibling row's markup unchanged and does not draw it again
 FAIL  ... inline expand (FR-085) > AC4: the row shows an arrow, closed then open, hidden from assistive technology
 FAIL  ... inline expand (FR-085) > AC5: an MCP call shows only "server · tool", never the raw name; the raw name is in the row title; the time is HH:MM:SS 24 h
 FAIL  ... inline expand (FR-085) > AC6: every feed cell is one line with an ellipsis, and the feed row is a grid; the feed scrolls inside a max height
 FAIL  ... rows (FR-072) > AC7: when the open session leaves the list nothing is drawn for it and the open id resets
 FAIL  ... design > has no side-panel grid: the list keeps one width whether a session is open or not (FR-085)
 (37 failing in all, incl. the adapted feed/label tests)
 Test Files  1 failed (1)
      Tests  37 failed | 60 passed (97)
   Start at  19:45:28
```

BEFORE screenshots of the old side panel (session open, 2 sessions; calls appended after opening; the feed in the panel wraps to 2-3 lines, MCP row prints the raw name and the label, time is 12 h "7:42:53 PM", list squeezed to a narrow column): `reports/HUN-236/before-w1280-closed.png`, `before-w1280-open.png`, `before-w375-closed.png`, `before-w375-open.png` (under the worktree; `/reports/` is gitignored). Measured in headless Chrome at 375 px: feed row heights 73,25,73,25,25,25 px (wrapped rows), at 1280 px: 73,25,49,25,25,25.

**AFTER**: 2026-11-06T12:49:35Z, same command after the change:

```
$ cd web && npx vitest run src/LiveView.test.jsx && npx vitest run && npm run build
 Test Files  1 passed (1)
      Tests  97 passed (97)
 Test Files  21 passed (21)
      Tests  695 passed (695)
✓ built in 204ms
```

Real run (`easy-personal dashboard` on temp homes; transcript line written first, row opened, `aria-expanded` true, then Skill/Agent/Bash/MCP calls and a sub-agent file call appended; two sessions), headless Chrome over CDP:

```
w1280 expanded: true
w1280 feed inside open card: true | aria-controls target exists: true
w1280 arrows: ▸▾        feed rows: 6        feed row heights: 25,25,25,25,25,25
w1280 feed text: 19:48:05 backend-developer claude_ai_Linear · save_issue | ... main easy-verifier · score | main Bash uv | main Agent backend-developer | main Skill code-review
w1280 feed titles: 19:48:05 · main · mcp__easy-verifier__score · easy-verifier · score | ...
w1280 second row opened, first closed: false,true feeds=1
w1280 after Escape: false,false feeds=0 focusIsRow=true
w375  scrollWidth/clientWidth: 375/375     row widths closed -> open: 335,335 -> 335,335
w375  feed row heights: 25,25,25,25,25,25  (cells cut with an ellipsis: live-who, live-call)
w375  after Escape: false,false feeds=0 focusIsRow=true
```

Screenshots (worktree, gitignored): `reports/HUN-236/after-w1280-closed.png`, `after-w1280-open.png`, `after-w1280-second-open.png`, `after-w375-closed.png`, `after-w375-open.png`, `after-w375-second-open.png`.

**DELTA**: The owner opens a Live session under its own row at full list width (arrow, Close, Escape, one open at a time), and each call is one 24 h `HH:MM:SS` line that ends with an ellipsis instead of wrapping to 2-3 lines.

**WITNESS**: Supervisor (main session), 2026-11-06 — re-ran vitest, the build and the full Python suite on 6854a37 and checked the AFTER screenshots in the task worktree's `reports/HUN-236/` (trace `memory/event-trace/HUN-236.jsonl`)