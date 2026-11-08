# TASK_REVIEW — T[NNN]: [Short Title]

> Sibling of `tasks/TASK_GUIDE_T[NNN].md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` tag,
`docs/claude-md/tracks.md` `## Auto mode`). To use it, copy the block below out of this comment, to this
place, and replace every <...>. Keep `Mode: auto` as its first line, exactly as written: T182's hook reads
it. Write it to this file before the run ends; the chat message is a copy, not the report. The same
block is the Supervisor's final chat message. It is not the `delivery-report` page and not
Evidence. A taste decision (what the user wants) also ends with the flag. Write `DEBATED` only when ≥2
Advocate `Agent` calls ran for that question, naming each Advocate's option. A decision with no Advocates is
labelled `RULE`, `FACT` or `SAFER OPTION`, never `DEBATED`. A decision flagged `⚠ decided for you` is a choice, and is never labelled `FACT`. A
safety action a rule requires (for example the backup before a delete) is `RULE` without the flag.
At Risk Medium or High, a `RULE:` counts only when it names a written rule (`path:line`,
`path § <heading>`, or `docs/claude-md/tracks.md` `### <section>`), and a `FACT:` needs a Citation
(`path:line "quoted text"`); without one it is a choice and must be `DEBATED`. Good: `2. backed up the CSV
— RULE: `docs/claude-md/tracks.md` `### Stops that stay``. Bad: `1. kept recurring.csv — RULE: Surgical Changes`.

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its option>; Advocate 2: <its option>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours)>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/surfaces/test_api.py::test_csp_on_every_response` now asserts the exact new string on page, 404, 403 (token), 200 and bad-Host responses; failed before the change (`1 failed, 17 deselected`), passes after. `test_api_live.py` and `test_api_kit_report_files.py` compare to `api.CSP`, so they follow the constant unchanged. |
| Verification command run | ☑ pass | `uv run pytest tests/surfaces -q` -> `339 passed, 1 warning in 8.14s`; `uv run pytest -q` -> `1758 passed, 6 deselected, 1 xfailed, 1 warning in 156.37s`; vitest -> `Test Files 24 passed (24)`, `Tests 786 passed (786)` |
| Negative cases hold | ☑ pass | Regression guard `tests/surfaces/test_api.py::test_csp_keeps_script_and_egress_on_default_src` (script-src, 'unsafe-inline' only in style-src, img/font/connect absent); probe: adding `img-src *` to the constant made it fail (`2 failed, 17 deselected`), probe reverted, `uv run pytest tests/surfaces -q` -> `340 passed, 1 warning in 7.80s`. AC 3 real Chrome: evil.html title stays `safe` (not `PWNED`), 0 `/api/workspaces` requests from the tab, security log: `Executing inline script violates ... directive 'default-src 'self''. ... 'script-src' was not explicitly set, so 'default-src' is used as a fallback. The action has been blocked.` AC 4: score.html `styleSheets.length 1`, body `rgb(1, 2, 3)`, no `Applying inline style ... blocked` entry after the change (it was present before). |
| verify | ☑ N/A | Independent `/verify` not run by the implementer; the real-browser run above is the implementer's own check (pass for AC 3 and AC 4). |
| Stage 4 reports (html-report) | ☑ pass | `reports/code-review_DRAFT-report-css_20261108T120019.html` (Risk 2, Quality 88, Effort 10 — Healthy) and `reports/security-review_DRAFT-report-css_20261108T120019.html` (Risk 0, Quality 95, Effort 0 — Healthy). `reports/` is gitignored, so the pages stay on this machine and only their paths are recorded. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | One line in `api.py`; every response's CSP header changes; the three test files that assert CSP checked. `Guard`, `web/`, `kit.py` untouched. |
| Full smoke suite still green (no regression) | ☑ pass | Supervisor re-ran both suites on the branch at Stage 4, after the guard test landed (8b0fe87): `uv run pytest -q` -> `1759 passed, 6 deselected, 1 xfailed, 1 warning in 154.37s` (1758 on `main` plus the one new guard test); `npx vitest run` -> `Test Files 24 passed (24)`, `Tests 786 passed (786)`. |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier is not loaded in this session, so no score was run. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Header change, no component change. 1280x720, before = file reverted to `main`'s `api.py` (printed CSP `default-src 'self'; frame-ancestors 'none'`), after = this change (printed CSP with `style-src`). Usage view: `before/usage.png` and `after/usage.png` are byte-identical (`cmp`). Kit view: `before/kit.png` and `after/kit.png` look identical (bytes differ only by the random temp repo name). Styled-report screenshot: `after/score.html.png`. All under `tasks/evidence-DRAFT-report-css/`. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | No dashboard component changes. |
| **UI: Responsiveness at target viewports** | ☑ N/A | No dashboard component changes. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (2026-11-08T04:40:03Z, real headless Chrome via CDP, unchanged `CSP = "default-src 'self'; frame-ancestors 'none'"` at `src/easy_personal/surfaces/api.py:57`; harness run on the Kit view -> report folder -> file click; screenshots `tasks/evidence-DRAFT-report-css/before/`):

```
{
 "out": {
  "evil.html": {
   "url": "blob:http://127.0.0.",
   "title": "safe",
   "styles": 0,
   "sheets": 0,
   "bg": "rgba(0, 0, 0, 0)"
  },
  "score.html": {
   "url": "blob:http://127.0.0.",
   "title": "",
   "styles": 1,
   "sheets": 0,
   "bg": "rgba(0, 0, 0, 0)"
  }
 },
 "security": [
  "Executing inline script violates the following Content Security Policy directive 'default-src 'self''. Either the 'unsafe-inline' keyword, a hash ('sha256-fMGsF4nWWkfnz2nZkEKQYe4XgbPtz/EdkODcXCVTxP4='), or a nonce ('nonce-...') is required to enable inline execution. Note also that 'script-src' was not explicitly set, so 'default-src' is used as a fallback. The action has been blocked.",
  "Applying inline style violates the following Content Security Policy directive 'default-src 'self''. Either the 'unsafe-inline' keyword, a hash ('sha256-5l8arhEAuNopJs/rtr0uNH1iWDOH6Ik4hqu91zQJAzQ='), or a nonce ('nonce-...') is required to enable inline execution. Note also that 'style-src' was not explicitly set, so 'default-src' is used as a fallback. The action has been blocked."
 ],
 "apiRequestsFromTabs": []
}
```

score.html opens with `styles: 1, sheets: 0` and a transparent body: its `<style>` is blocked.

**AFTER** (2026-11-08T04:43:23Z, same harness, `CSP = "default-src 'self'; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'"`; screenshots `tasks/evidence-DRAFT-report-css/after/`):

```
{
 "out": {
  "evil.html": {
   "url": "blob:http://127.0.0.",
   "title": "safe",
   "styles": 0,
   "sheets": 0,
   "bg": "rgba(0, 0, 0, 0)"
  },
  "score.html": {
   "url": "blob:http://127.0.0.",
   "title": "",
   "styles": 1,
   "sheets": 1,
   "bg": "rgb(1, 2, 3)"
  }
 },
 "security": [
  "Executing inline script violates the following Content Security Policy directive 'default-src 'self''. Either the 'unsafe-inline' keyword, a hash ('sha256-fMGsF4nWWkfnz2nZkEKQYe4XgbPtz/EdkODcXCVTxP4='), or a nonce ('nonce-...') is required to enable inline execution. Note also that 'script-src' was not explicitly set, so 'default-src' is used as a fallback. The action has been blocked."
 ],
 "apiRequestsFromTabs": []
}
```

**DELTA**: An opened report now applies its own inline `<style>` (`sheets: 1`, body `rgb(1, 2, 3)`), while an inline `<script>` is still blocked.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
