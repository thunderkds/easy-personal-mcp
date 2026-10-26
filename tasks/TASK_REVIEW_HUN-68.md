# TASK_REVIEW — HUN-68: Dashboard Agents page

> Sibling of `tasks/TASK_GUIDE_HUN-68.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | pass | `web/src/AgentsView.test.jsx` (14 tests: AC1-AC6 + edge cases) |
| Verification command run | pass | `npm --prefix web test`: Test Files 7 passed (7), Tests 64 passed (64) after UI-review fixes (font inherit, short heading); `npm --prefix web run build`: built in 606ms; `uv run pytest tests/surfaces -q tests/test_web_toolchain.py`: 126 passed, 1 warning |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☑ pass | Supervisor 2026-10-26: isolated install (`scratchpad/demo.sh`: agent root with .claude, .claude-work (confirmed), .claude-side (detected), .codex; workspaces company/personal/side; connectors synced by fake claude), `easy-personal dashboard`, headless Chrome over CDP: Agents tab lists 4 accounts; By provider groups claude/codex; Claude work: untick personal → Review changes → plan "Claude work: remove personal-gmail (exec cead0f59a)" with Confirm/Cancel; detected rows offer Name + Confirm & name / Ignore; only `127.0.0.1` requests, 0 console errors — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed `AgentsView.jsx` (toggle, rows per status, pick → PUT → GET plan → POST confirm, cancel restores picks, double-confirm guard), `App.jsx` one tab, `app.css` (`.picks`, `font: inherit`), fixtures + 14 tests. code-review: P0 0 / P1 0 / P2 0 / P3 2 FIXED (form controls rendered in Arial — `font: inherit` added; unnamed rows printed the long folder path twice — heading is now the folder's last segment). security-review (inline): all writes via `sendJson` (token header asserted in a test), no secret on this page, plan lines only alias + id; no findings. |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| easy-verifier score (advisory) | ☑ N/A | N/A — easy-verifier MCP not loaded in this Supervisor profile |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Screenshots `reports/HUN-68/{Agents,provider-Agents}-{375,768,1280}.png` + `plan-375.png` (local, gitignored), read with LLM vision against AC 1–6 and the approved sketch: toggle, confirmed/detected/Codex rows, workspace checkboxes without orphan, plan box with Confirm/Cancel — match |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | CSS audit: new rules use `var(--…)` tokens only; computed fonts over all elements after the fix: one family `system-ui, -apple-system, "Segoe UI", sans-serif` (before the fix also `Arial` on form controls) |
| **UI: Responsiveness at target viewports** | ☑ pass | CDP at 375 / 768 / 1280: scrollWidth == clientWidth (375/375, 753/753, 1265/1265) in both views; long paths wrap |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**:
```
$ date -u
Mon Oct 26 10:16:16 AM UTC 2026
$ grep -n "Agents" web/src/App.jsx
(exit 1: no match, no Agents tab)
$ ls web/src/AgentsView.jsx
ls: cannot access 'web/src/AgentsView.jsx': No such file or directory
```

**AFTER**: `grep -n Agents web/src/App.jsx` now shows the import and the `{ id: 'agents', label: 'Agents', View: AgentsView }` tab; `web/src/AgentsView.jsx` exists; Vitest 63 passed.

**DELTA**: The owner can see agent accounts by account or by provider, confirm or ignore detected folders, and pick workspaces for a Claude account with a plan shown before anything is written.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
