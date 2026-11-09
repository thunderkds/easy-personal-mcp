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
| verify | ☑ pass | User-invoked `/verify DRAFT-report-css`, 2026-11-08, run by the Supervisor on an own fixture, independently of the implementer: temp `EASY_PERSONAL_HOME`, `uv tool install --force` from this worktree, workspace `acme` registered, own `score.html` / `evil.html` / `leak.html`, plus a real project report. **pass** — 21 of 21 assertions, all 6 Acceptance Criteria. Full record in the VERIFY block below. |
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

**WITNESS**: the Supervisor, on user-invoked `/verify DRAFT-report-css`, 2026-11-08, against branch
`report-css` at `cd2d0dc`. Not the implementing agent: an own fixture, an own driver
(`verify.mjs`), an own dashboard launch on a random port, and an own real project report. The
implementer's own BEFORE/AFTER above stays as it was written.

---

## VERIFY — user-invoked `/verify DRAFT-report-css`, 2026-11-08

**Verdict: pass. 21 of 21 assertions; all 6 Acceptance Criteria met.** Branch `report-css` at
`cd2d0dc`, diff `main...HEAD` = 8 lines in `src/easy_personal/surfaces/api.py` (docstring, comment,
the constant) and 19 lines in `tests/surfaces/test_api.py`.

### Recipe (`.claude/skills/verify/SKILL.md`)

```
=== version ===     0.4.0
=== ping ===        easy-personal core ok
=== mcp stdio ===   id 1 initialize ok, protocol 2025-06-18
                    id 2 tools: 12
                    id 3 ping: {"content": [{"type": "text", "text": "easy-personal core ok"}], ...}
=== NFR-001 ===     host=0.0.0.0   rc=2 :: refusing host '0.0.0.0': HTTP must bind 127.0.0.1 only
                    host=localhost rc=2 :: refusing host 'localhost': HTTP must bind 127.0.0.1 only
                    host=::1       rc=2 :: refusing host '::1': HTTP must bind 127.0.0.1 only
                    host=10.0.0.5  rc=2 :: refusing host '10.0.0.5': HTTP must bind 127.0.0.1 only
=== argparse ===    bad --port rc=2; bad --transport rc=2
```

Correction to the recipe, found by this run: `--host` belongs to `easy-personal mcp`, not to
`easy-personal dashboard`. Run against `dashboard` the probe exits 2 with
`error: unrecognized arguments: --host 0.0.0.0` — argparse rejecting an unknown flag, which looks
like a pass but tests nothing. The NFR-001 guard above was therefore re-run against `mcp`. The
`dashboard` subcommand takes no host argument at all, so it cannot be pointed off loopback.
MCP HTTP skipped: the recipe marks it opt-in and this task adds no MCP surface.

### AC 1 — the CSP header, live, on five response shapes

Every one returned exactly
`content-security-policy: default-src 'self'; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'`:

```
GET /                                     -> 200
GET /api/workspaces (token)               -> 200
GET /api/workspaces (no token)            -> 403
GET / (Host: evil.example)                -> 403
GET /assets/index-DvUpsqr9.js             -> 200
```

The browser confirmed the same string on the page document it actually loaded.

### AC 2 — the three existing CSP assertions

`tests/surfaces/test_api.py` updates the literal; `test_api_live.py:184` and
`test_api_kit_report_files.py:54` compare against `api.CSP`, so they track the constant. No test is
deleted or weakened: the diff adds 19 lines and removes 4 (the reformatted literal).

### AC 3 — the inline SCRIPT is still blocked (real headless Chrome)

`evil.html` opened from the Kit view, CDP `Log` domain attached before the blob navigation
(`attached: true`, so no race):

```
state: {"title":"evil","out":"script did not run","proto":"blob:"}
security log entries: 1
  [security] Executing inline script violates the following Content Security Policy directive
             'default-src 'self''. Either the 'unsafe-inline' keyword, a hash (...) ...
```

PASS the tab opened as a `blob:` document · PASS the title is not `PWNED` · PASS the script body
never ran · PASS no `/api/*` request from the tab · PASS the security log still names the inline
**script** as blocked · PASS no `Applying inline style` entry any more.

### AC 4 — the report CSS now applies

`score.html` (an inline `<style>` and one `style="..."` attribute):

```
state: {"styleTags":1,"sheets":1,"rules":3,"bg":"rgb(16, 16, 24)",
        "h1":"rgb(255, 138, 216)","attr":"rgb(255, 209, 102)"}
security log entries: 0
```

Before the change the same fixture gave `styleTags: 1, sheets: 0` and a transparent body. The
style **attribute** applies too, which confirms `style-src-attr` falling back to `style-src`.

The owner's real case, checked end to end: this task's own
`reports/code-review_DRAFT-report-css_20261108T120019.html` (generated from
`templates/report_template.html`) copied into the fixture and opened through the dashboard:

```
real project report: {"title":"Code Review Report — report-css","sheets":1,"rules":41,
                      "bg":"rgb(10, 10, 18)"}
security log entries: 0
```

All 41 rules live, the dark theme, the score bars and the badges render as designed.

### EGRESS — the invariant the new comment and the guard test claim

A third fixture, `leak.html`, whose inline CSS attempts three off-origin loads plus an `<img>`,
against a real listening sink on `127.0.0.1:19999`:

```
state: {"sheets":1,"bgImg":"url(\"http://127.0.0.1:19999/leak-bg.png\")","imgW":0}
security log entries: 4
  [security] Loading the image 'http://127.0.0.1:19999/leak-img.png' violates ... "default-src 'self'".
             Note that 'img-src' was not explicitly set, so 'default-src' is used as a fallback.
  [security] Loading the font  'http://127.0.0.1:19999/leak-font.woff2' violates ... 'font-src' ...
  [security] Loading the image 'http://127.0.0.1:19999/leak-bg.png'  violates ... 'img-src' ...
  [security] Loading the image 'http://127.0.0.1:19999/leak-el.png'  violates ... 'img-src' ...
sink log: "sink listening on 127.0.0.1:19999"   (no CONNECTION line — not one TCP connection)
```

The attacking stylesheet **does** load and its `url()` **is** a computed value, so the exfiltration
primitive is real and only the fallback to `default-src 'self'` stops it. The sink never received a
connection. This is the behaviour `test_csp_keeps_script_and_egress_on_default_src` pins.

### AC 5 — the dashboard's own views are unchanged

`npm --prefix web run build` in this worktree produced `index-DvUpsqr9.js`, byte-for-byte the hash
already on `main`, so the page did not change. Usage view and Kit view screenshotted at 1280x720 in
this run and read: the cache-reuse donut, the sparklines and the progress bars all render (React
sets those through the CSSOM, which CSP does not govern). The implementer's
`before/usage.png` and `after/usage.png` are byte-identical; `before/kit.png` and `after/kit.png`
differ only in the random temp repo name (`ep-repo-rv9jCY` vs `ep-repo-CH29bg`) — checked by
opening both, not by trusting the hash.

### AC 6 and the suites

Re-run by the Supervisor on this branch during the verify, not taken from Stage 4:

```
uv run pytest -q
  1759 passed, 6 deselected, 1 xfailed, 1 warning in 220.55s (0:03:40)

cd web && npx vitest run
  Test Files  24 passed (24)
       Tests  786 passed (786)
```

1759 is `main`'s 1758 plus the one new guard test.

Method note: the first vitest attempt used `npx --prefix web vitest run --root web` from the repo
root and reported `Test Files 12 failed | 12 passed (24)`, `Tests 11 failed | 560 passed (571)`.
That is the invocation, not the code: `--prefix` is an npm flag, npx ignores it, so vitest ran from
the repo root with a mismatched root and collected a different set. Run the documented way,
`cd web && npx vitest run`, it is 24 / 786 green. The failure was also masked at first because the
command ended in `tail -6`, so the subshell's exit code was `tail`'s 0 and the summary lines were
clipped — the same clipping trap as the HUN-243 run.

`uv run ruff check src tests` -> `All checks passed!`.
`python3 scripts/lane_check.py tasks/TASK_GUIDE_HUN-247.md main...HEAD` ->
`lane: C1, 11 files, 30 lines, EXCEEDS (files 11 > 6)`; warn-only, and every unpredicted file is an
evidence PNG or the guide itself. 30 changed code lines is well inside C1.

### NFR-008 — the token

One request URL contains the token: the page document's own `http://127.0.0.1:<port>/#<token>`.
A fragment is never transmitted. No request puts the token in a path or a query. Checked with a
dedicated probe after the first, blunter check reported a false positive.

### Single source of policy

`CSP` has one consumer, `Guard.send_csp`, which strips any route-set
`content-security-policy` and writes this one. The built `index.html` has no
`<meta http-equiv>`. So the constant is the whole policy, for the dashboard and for every `blob:`
tab that inherits it.

### Not done

The merge and the post-merge `npm --prefix web run build` on `main` stay the owner's call
(`src/easy_personal/dashboard/static/` is gitignored, so the bundle is built locally, never
committed). The running dashboard must be restarted to serve the new policy.
