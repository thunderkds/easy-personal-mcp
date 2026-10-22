# TASK_REVIEW — HUN-50: Dashboard usage view: summary, breakdowns, session timeline, "not included" notice

> Sibling of `tasks/TASK_GUIDE_HUN-50.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☒ pass / ☐ fail | `web/src/UsageView.test.jsx` (AC1 totals/breakdowns equal fixture API values, AC3 every cost header/label "API-equivalent cost" + abstained text never $0, AC4 `role="note"` notice, AC6 tokens first / folder name / two same-name `app` rows both render), `web/src/Timeline.test.jsx` (AC2 one mark per point, one band per gap, gap minutes listed, threshold stated, one-point session no NaN, 50-session cap); fixtures `web/src/test/fixtures.js` match the real API shape (Supervisor compared `/api/usage/breakdown?by=day` keys on a live server). Supervisor re-run 2026-10-22T03:49:41Z: `Test Files 2 passed (2)`, `Tests 22 passed (22)` |
| Verification command run | ☒ pass / ☐ fail | `npm --prefix web test && npm --prefix web run build` → 22 passed; `✓ built in 111ms` (check_static ok). `uv run pytest -q` → `190 passed, 1 warning in 17.61s` |
| Negative cases hold | ☒ pass / ☐ fail | empty period → "No usage recorded in this period." / "No sessions in this period."; abstained cost shown as `abstained: <models>`, never $0; API error → `role="alert"` message; no token in URL → API 403 → alert (HUN-49 guard) |
| verify | ☒ pass / ☐ fail / ☐ N/A | 2026-10-22T03:47Z: throwaway checkout = HUN-50 + HUN-53 code commit `38b8696`, page built, `easy-personal dashboard` with date-shifted fixtures; headless Chrome loaded `http://127.0.0.1:57131/#<token>`: page shows notice, totals (1,273 tokens, cost `abstained: claude-fixture-model-1`, price table 2026-10-21), 4 breakdowns, outliers, 4 timeline sessions. Combined suite `196 passed`. AC5 Chrome net log: page requests only `127.0.0.1:57131` (`/`, JS, CSS, 7 `/api/...` calls, favicon); other hosts in the log are Chrome background services with no page initiator — feature confirmed working — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☒ pass / ☐ fail | `main...HUN-50` (web/ only). lane_check: `lane: C2, 15 files, 865 lines, EXCEEDS (files 15 > 14; lines 865 > 500); unpredicted: web/package.json, web/vitest.config.js` — bulk is tests/fixtures + test toolchain; raised to owner, not re-labelled. code-review: 1 P1 found and fixed in `d9997ca` (grid tracks `min-width:auto` → `.panels` overflowed viewport at 375/1280 px; now `minmax(0,1fr)`), P3: `fetch` without timeout (loopback only), single-day bar is wide. Startup reads complete. security (inline, Medium): no `innerHTML`/`eval`/storage/remote URL in sources; bundle URLs only React error link + SVG namespace; token sent only as `X-Dashboard-Token`; 4 devDependencies (vitest, jsdom, @testing-library/react, @testing-library/jest-dom) pinned, 78 lockfile entries all from registry.npmjs.org, none in the shipped bundle; P3: token stays in URL fragment/history (HUN-49 design). |
| Full smoke suite still green (no regression) | ☒ pass / ☐ fail | `uv run pytest -q` → `190 passed, 1 warning in 17.61s` (HUN-50 worktree, page built: the 2 web tests run, none skipped); combined with HUN-53 → `196 passed` |
| easy-verifier score (advisory) | ☒ recorded / ☐ N/A | 2026-10-22, MCP `score` repo=HUN-50 worktree, scope=changes, ref=`main...HUN-50`: overall **77** (architecture 100, blast-radius 100, code-quality 87, test-strategy 75, requirement-fidelity 67, security 59, reliability 50); abstained: solution-fit, supply-chain, api, data, operations, infrastructure. Reliability = `outbound_calls_without_timeout_observed` 1 (`web/src/api.js`, loopback fetch, P3); security = `redaction_hits_observed` 119 (known false positive). Raw JSON /home/hungnguyenhuu/.claude-personal/projects/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/75d1f65a-8da1-4f4c-8245-0270482d2af1/tool-results/mcp-easy-verifier-score-1792641015943.txt. Advisory only. |
| **UI: Visual regression (diff or verdict pasted)** | ☒ pass / ☐ fail / ☐ N/A | easy-ui MCP not available in this session; headless Chrome (`google-chrome --headless=new`) screenshots + LLM vision review against the AC: notice, totals, By day chart + tables, outliers, timeline bars/lanes render; no broken layout after `d9997ca`. Before fix: `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/reports/HUN-50/before-fix-*.png` (panels overflow); after: `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/reports/HUN-50/usage-375.png`, `usage-768.png`, `usage-1280.png`. Gap bands are not visible in the fixture data (no 30-min pause); covered by `Timeline.test.jsx`. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☒ pass / ☐ fail / ☐ N/A | CSS audit: 0 hex colors outside `web/src/tokens.css`; one `font-family: var(--font)`; sizes and spacing from `--text-*`/`--space-*`; only raw values are 1–2px borders/outline and `72rem` page width. SVG uses classes only (no inline style; CSP `default-src 'self'` holds). |
| **UI: Responsiveness at target viewports** | ☒ pass / ☐ fail / ☐ N/A | 375 px: single column, every card inside the viewport, wide tables scroll inside `.table-wrap`; 768 px: single column, readable charts; 1280 px: two-column breakdown panels inside the 72rem page. Screenshots `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/reports/HUN-50/usage-{375,768,1280}.png`. First run failed (overflow) → fixed in `d9997ca` → re-shot pass. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-22T03:41:47Z in the HUN-50 worktree, before any implementation commit:

```
$ npm --prefix web test
npm error Missing script: "test"
$ cat web/src/main.jsx
import { createRoot } from 'react-dom/client'

createRoot(document.getElementById('root')).render(<p>easy-personal dashboard placeholder</p>)
```

**AFTER**: 2026-10-22, same worktree:

```
$ npm --prefix web test
 Test Files  2 passed (2)
      Tests  22 passed (22)
$ npm --prefix web run build   -> built; check_static: ok
```

**DELTA**: the owner can open the dashboard page and see usage totals, breakdowns by day/model/project/tool, outlier sessions and a session timeline, with cost labelled API-equivalent and a "not included" notice.

**WITNESS**: Supervisor — test re-runs 2026-10-22T03:49Z in the HUN-50 worktree and headless-Chrome verify 2026-10-22T03:47–03:49Z on a throwaway HUN-50 + HUN-53 checkout (attributed to HUN-50 via the active_task state file; `memory/event-trace/HUN-50.jsonl`), independent of the implementing agent.
