# TASK_REVIEW — HUN-70: Usage page Workspace and Account filters

> Sibling of `tasks/TASK_GUIDE_HUN-70.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | pass | `web/src/UsageView.test.jsx` describe 'workspace and account filters (HUN-70)', 8 tests: options, no default params, workspace=, both params, header, notice, no-repos hint, options failure. Run 2026-10-26: `Test Files 6 passed (6), Tests 58 passed (58)` |
| Verification command run | pass | `npm --prefix web test && npm --prefix web run build`: 58 passed; `vite build` ok (25 modules, index-1KtoM2Sy.js 240.87 kB); `uv run pytest tests/test_web_toolchain.py -q`: 15 passed |
| Negative cases hold | ☑ pass | Vitest: options request fails → dropdowns hidden, usage unfiltered; workspace with no repos → hint; filters cleared → no params (implementer tests, Supervisor re-run 58 passed) |
| verify | ☑ pass | Supervisor 2026-10-26 ~10:20Z: isolated install (`scratchpad/demo.sh`: fake agent root with 2 Claude accounts + Codex, 3 workspaces, fixtures shifted to today, fake claude + file keyring), `easy-personal dashboard`, headless Chrome over CDP: Usage tab shows Workspace ▾ + Account ▾; picking company + Claude work → scope line "Workspace: company · Account: Claude work · Last 7 days", totals 1,273 tokens / 3 messages (unfiltered 2,546 / 6); only `127.0.0.1` requests; 0 console errors — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed `UsageView.jsx` (filters, params on all 7 usage calls, options failure path, no-repos hint), `app.css` `.filters` (tokens only), fixtures + tests. code-review: P0 0 / P1 0 / P2 0 / P3 1 (`workspace=` carries the name — matches the API, which resolves names; guide SC said id — guide wording wrong, accepted). security-review: N/A (Low risk; read-only GETs, no new write). |
| Full smoke suite still green (no regression) | ☑ pass | Supervisor re-run: `npm --prefix web test` → Tests 58 passed (58); `uv run pytest -q` → 667 passed, 4 deselected |
| easy-verifier score (advisory) | ☑ N/A | N/A — easy-verifier MCP not loaded in this Supervisor profile |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Screenshots `reports/HUN-70/Usage-{375,768,1280}.png` (local, gitignored) read with LLM vision: filters row under the notice, scope line in the period header, existing panels unchanged in layout; matches AC 1–3 and the approved sketch (Workspace ▾, Account ▾ on Usage) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | CSS audit: new `.filters` rules use only `var(--…)` tokens from `tokens.css`; computed fonts over all elements: one family `system-ui, -apple-system, "Segoe UI", sans-serif` |
| **UI: Responsiveness at target viewports** | ☑ pass | CDP at 375 / 768 / 1280: `scrollWidth == clientWidth` at every width (375/375, 753/753, 1265/1265) — no horizontal scroll; dropdowns wrap at 375 |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**:
```
Mon Oct 26 10:16:04 AM UTC 2026
$ grep -n "workspace\|account" web/src/UsageView.jsx
(exit 1; no matches: no filters today)
```

**AFTER**: `grep -n "workspace\|account" web/src/UsageView.jsx` now matches the filter state, query params (`workspace=`, `account=`), options loader and the two dropdowns; Vitest proves each usage call carries them (58 passed).

**DELTA**: The owner can narrow every Usage number to one workspace and one Claude account from two dropdowns.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
