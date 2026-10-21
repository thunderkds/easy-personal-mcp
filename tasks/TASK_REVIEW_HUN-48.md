# TASK_REVIEW — HUN-48: web/ toolchain

> Sibling of `tasks/TASK_GUIDE_HUN-48.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | pass | `tests/test_web_toolchain.py` (15 tests). RED first (2026-10-21T10:13:33Z, script absent): `1 failed, 2 skipped, 12 errors in 0.26s` (`check_static.py: No such file`). GREEN after script: `13 passed, 2 skipped`; with build present: `15 passed in 1.36s`. AC1 build, AC2 remote-URL/sourcemap, AC3 wheel (real `uv build --wheel`), AC4 missing/stale, AC5 telemetry deny-list + exact pins |
| Verification command run | pass | 2026-10-21T10:14:49Z: `npm --prefix web ci && npm --prefix web run build && uv build && python3 scripts/check_static.py` -> `built in 296ms`, `Successfully built dist/easy_personal_mcp-0.1.0-py3-none-any.whl`, `check_static: ok` |
| Negative cases hold | pass | stale: appended a line to `web/src/main.jsx` -> `FAIL: static build stale: web/ sources changed since the last build`, exit 1 (then reverted). Real wheel RED before pyproject fix: `FAIL: wheel lacks static assets under easy_personal/dashboard/static/` (gitignored build dropped by hatch; fixed with `artifacts` in wheel and sdist targets). Missing static: `FAIL: static build missing: ...` |
| verify | pass | 2026-10-21 Supervisor /verify: verification command → `found 0 vulnerabilities`, `built in 177ms`, `Successfully built dist/easy_personal_mcp-0.1.0-py3-none-any.whl`, `check_static: ok`; wheel lists `easy_personal/dashboard/static/{.web-stamp,index.html,assets/index-CRJI5cQk.js}`; stale probe (line appended to `web/src/main.jsx`) → `FAIL: static build stale: web/ sources changed since the last build`, exit 1, restored → ok; `uv tool install` of the wheel in a temp dir → `0.1.0`, `easy-personal core ok`, static dir installed; `uv run pytest -q` → `26 passed in 2.34s` — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | pass | 11 files in `main...feat/HUN-48-web-toolchain`; code-review 0 P0/P1, 1 P2 (partial startup read), 2 P3 — `reports/code-review_feat-HUN-48-web-toolchain_20261021T171645.html`; security (inline) pass — `reports/security-review_feat-HUN-48-web-toolchain_20261021T171645.html` (main checkout) |
| Full smoke suite still green (no regression) | pass | `uv run pytest -q` -> `26 passed in 2.67s` |
| **UI: Visual regression (diff or verdict pasted)** | N/A | One placeholder page, no product UI (frontend tasks) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | N/A | No design system yet (cut list) |
| **UI: Responsiveness at target viewports** | N/A | Placeholder page only |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-21T10:12:41Z in the worktree, before any implementation commit.
Command: `npm --prefix web ci && npm --prefix web run build && uv build && python3 scripts/check_static.py`

```
npm error   --workspaces ... (npm ci usage text; no web/package.json or package-lock.json exists)
npm error aliases: clean-install, ic, install-clean, isntall-clean
npm error Run "npm help ci" for more info
```
Result: fails at the first step (`npm ci`); `web/` and `scripts/check_static.py` do not exist.

**AFTER**: 2026-10-21T10:14:49Z, same command: `npm ci` ok (44 locked packages, 0 vulnerabilities); build writes `src/easy_personal/dashboard/static/index.html`, `assets/index-CRJI5cQk.js` (218.83 kB), `.web-stamp`; `uv build` wheel contains all three; `check_static: ok`.

**DELTA**: A release now builds the dashboard page from a pinned lockfile into the package, and `scripts/check_static.py` blocks a release whose build is missing, stale, remote-linked or not in the wheel.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
