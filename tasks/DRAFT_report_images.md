# TASK_GUIDE — DRAFT: an opened report shows its embedded screenshots (FR-094)

> **Draft.** The board is unreachable and `LINEAR_API_KEY` is not set, so no task ID exists yet.
> A new task's ID comes from the board and a create cannot be queued
> (`scripts/board_sync.py:374-376`). When the board is reachable, run
> `python3 scripts/board_sync.py create --guide tasks/DRAFT_report_images.md`.
> Owner request 2026-11-08: "the screenshot from report refer, example liek the screenshot for the
> report list from orderly" — screenshots embedded inside an opened report do not display.
> Owner, 2026-11-08, on the proposed `img-src 'self' data:` fix: "merge".

**Date**: 2026-11-08
**Complexity Level**: C1
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Read `PRD.md` FR-090, FR-093, FR-094, NFR-002, NFR-008; `src/easy_personal/surfaces/api.py` (the
   `CSP` constant, its comment, and the whole `Guard` class); `tests/surfaces/test_api.py` (both
   `test_csp_*` tests), `tests/surfaces/test_api_live.py:184`,
   `tests/surfaces/test_api_kit_report_files.py:54`
6. Read `tasks/DRAFT_report_css.md` — the task that shipped FR-093 earlier today. This
   task is its direct follow-up and inherits its Mental Model.

---

## Mental Model (measured, not assumed — Supervisor run, 2026-11-08)

FR-093 shipped `style-src 'self' 'unsafe-inline'` and merged as `a2458f0`, so an opened report now
applies its own CSS. Its **images** are still blank. Measured in real headless Chrome, a report
holding one `<img src="data:image/png;base64,...">` opened through a live dashboard:

```
state: {"sheets":1,"imgComplete":true,"naturalWidth":0,"naturalHeight":0}
security log entries: 1
  [security] Loading the image 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAHgAAAA8...
RESULT: the embedded data: screenshot did NOT render
```

`sheets:1` is FR-093 working. `naturalWidth:0` with `imgComplete:true` is the image refused: the
element finished its load attempt with no bytes. Cause: `img-src` is **unset**, so it falls back to
`default-src 'self'`, and a `data:` URL is not `'self'`.

Reach: this is not one fixture. Every one of the 7 session reports in the owner's `orderly` project
(`/home/hungnguyenhuu/workspace/project/phanpy/orderly/smoke-reports/`) embeds its screenshots as
`data:image/png` — 11 image references in all, every one blank today. This project's own
`templates/report_template.html` has no image, so the project's reports are unaffected; the owner's
are the case to fix.

**Why `data:` does not undo what FR-093's guard test pins** (state this in the security review):

- A `data:` URL carries its own bytes in the document that already holds them. It makes **no**
  network request, so it cannot carry a value out. The egress invariant FR-093 established is about
  a *remote* destination.
- `font-src` and `connect-src` stay absent and keep falling back to `default-src 'self'`, so the
  `@font-face src: url()` and attribute-selector leaks that `security-review` probed on FR-093 stay
  closed. Do **not** widen either one "while you are there".
- `img-src` must name `'self'` and `data:` and nothing else. `*`, a host, `https:` or `blob:` would
  each restore a real egress channel through `background-image: url(...)`, which CSS can build from
  a matched secret.
- `script-src` stays unset, so a report's inline `<script>` stays blocked. That is a hard
  Acceptance Criterion here too, re-proved by a real browser run, not assumed.

**The test that must be deliberately amended, not quietly dropped.**
`tests/surfaces/test_api.py::test_csp_keeps_script_and_egress_on_default_src` currently asserts:

```python
    for egress in ("img-src", "font-src", "connect-src"):
        assert egress not in directives
```

`img-src` now exists, so that loop would fail. Do **not** delete the test and do **not** drop
`img-src` from the loop without replacing the cover: keep `font-src` and `connect-src` in the
absent-loop, and add an assertion that pins the new directive to exactly `["'self'", "data:"]`, so a
later widening to `*` or a host still fails this test. The whole point of the test is that widening
egress is a decision, so it must stay able to catch one.

---

## Requirement (Pillar 1 — Adapt the requirement)

**Restated intent**:

> Add `img-src 'self' data:` to the one `CSP` constant, and nothing else changes:
>
> ```python
> CSP = "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'"
> ```
>
> An opened report then shows the screenshots embedded in it. A remote image, a remote font and any
> `connect-src` destination stay refused, and a report's inline `<script>` stays blocked.

**Out of scope**: a per-route CSP or any change to `Guard._allowed`, `Guard.send_csp` or the token
check; a one-time link route; `font-src`, `connect-src`, `script-src` or any other directive; a
remote host or `*` in `img-src`; the page (`web/`); serving reports anywhere other than the existing
`blob:` tab; sanitising report HTML; anything in the owner's `orderly` project (it is a fixture to
read, never to edit).

**Requirement Refs**: FR-094, FR-093, FR-090, NFR-008

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner report and "merge", 2026-11-08)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered

---

## Dependencies & Reachability

**Depends on**: FR-093 / `DRAFT-report-css`, merged on `main` as `a2458f0`

**Entry point**: `CSP` in `src/easy_personal/surfaces/api.py`, applied by `Guard.send_csp` to every response

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Every response carries `content-security-policy: default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'` — assert the exact string on a page response, an `/api/` response and both 403 paths (missing token and bad Host), because `send_csp` serves those too | FR-094, NFR-008 |
| 2 | The three existing CSP assertions (`tests/surfaces/test_api.py:77`, `test_api_live.py:184`, `test_api_kit_report_files.py:54`) pass; the two that compare to `api.CSP` need no edit, the literal one does. None is deleted to make it green | FR-094 |
| 3 | `test_csp_keeps_script_and_egress_on_default_src` is amended as the Mental Model describes: `font-src` and `connect-src` stay in the absent-loop, and a new assertion pins `img-src` to exactly `["'self'", "data:"]`. Prove the guard still guards: patch `api.CSP` in memory to `img-src *`, to `img-src 'self' data: https://evil.example`, and to a widened `connect-src`, and show each one FAILS the test. Paste that output | FR-094, FR-093 |
| 4 | **The embedded screenshot now renders.** A real run, headless Chrome, a fixture report `shot.html` carrying one `<img src="data:image/png;base64,...">` of known pixel size, opened from the Kit view: `naturalWidth` and `naturalHeight` equal the PNG's real dimensions (not 0), and no security log entry names the image. Capture the log with a CDP session (`Log.enable`, then `Log.entryAdded` where `entry.source === "security"`) — a CSP violation does NOT reach `page.on('console')`. Take a screenshot of the tab where the image is visible to the eye, not only to a measurement | FR-094 |
| 5 | **A remote image is still refused.** Same real run, a fixture report `leak.html` whose CSS and markup try a remote destination four ways: `<img src="http://127.0.0.1:<sink>/i.png">`, `background-image: url(http://127.0.0.1:<sink>/b.png)`, `@font-face { src: url(http://127.0.0.1:<sink>/f.woff2) }`, and an attribute-selector rule that matches a planted value and fetches it. Run a real listening TCP sink on `127.0.0.1:<sink>` and show its connection log recorded **zero** connections. Measure, do not reason: the CSS exfiltration primitive genuinely fires, and only the fallback to `default-src 'self'` stops it. If the sink records one connection, STOP and report `Status: blocked` | FR-094, FR-093, NFR-002 |
| 6 | **The script defence is unchanged.** Same real run, the HUN-243 fixture shape: a report `evil.html` whose inline `<script>` sets `document.title = "PWNED"` and calls `fetch("/api/workspaces")` opens with the tab title NOT `PWNED`, makes no `/api/*` request, and the security log still reports the inline **script** blocked. If the script runs, STOP and report `Status: blocked` | FR-094, FR-093, NFR-002 |
| 7 | **One of the owner's real reports renders.** Copy one file from `/home/hungnguyenhuu/workspace/project/phanpy/orderly/smoke-reports/` into the fixture repo's `reports/` (copy — never edit the owner's project) and open it through the dashboard: its stylesheet applies and every `<img>` in it has a non-zero `naturalWidth`. Screenshot it | FR-094 |
| 8 | The dashboard's own pages are unchanged: `cd web && npx vitest run` and the full `uv run pytest -q` pass, and the Usage view and the Kit view at 1280x720 look as they do today. **Do not run `npx --prefix web vitest run`** — `--prefix` is an npm flag that npx ignores, so vitest then runs from the repo root with a mismatched root and reports false failures | FR-094 |
| 9 | Existing tests pass; `uv run pytest -q` passes | — |

---

## UI / Design Acceptance Criteria

The dashboard's own look does not change — this task alters a response header, not a component.
AC 8 pins that with the full suites plus a before/after screenshot of two views. Fill the three UI
Evidence rows ☑ N/A with that reason, and put the AC 4 and AC 7 screenshots in the Demonstration
instead.

---

## Evaluation & Acceptance

### Verification Command (exact, runnable)

```bash
uv run pytest tests/surfaces -q && uv run pytest -q && cd web && npx vitest run
```

Plus a real run: build the page (`npm --prefix web run build` — this one is a real npm flag), then
install into a temp tree and start the dashboard there, so the owner's own install and data stay
untouched:

```bash
T=$(mktemp -d)
export EASY_PERSONAL_HOME=$T/home UV_TOOL_DIR=$T/tools UV_TOOL_BIN_DIR=$T/bin
uv tool install -q --refresh .      # --refresh is NOT optional: with a cached wheel,
                                    # uv tool install reuses it and ships stale source
$T/bin/easy-personal workspace add acme --repo <fixture-repo>
```

The fixture workspace must be **registered**: a repo created on disk but never registered gets 404
on every route, and the negative checks then "pass" for the wrong reason. The fixture repo's
`reports/` holds `shot.html`, `leak.html`, `evil.html` and one copied `orderly` report. Drive real
Chrome (puppeteer-core against system `/usr/bin/google-chrome`) for AC 4–7; screenshots under
`tasks/evidence-DRAFT-report-images/`.

Gotchas that cost the last task time, so they are written down here:

- `page.goto` returns `null` on a second `goto` to the same URL — only the fragment changes, so no
  navigation occurs. Capture the first `goto`'s response.
- A command ending in `| tail -N` reports `tail`'s exit code, not the suite's, and clips the summary
  line. Never gate a pass on that.
- Do not run `pkill -f "easy-personal dashboard"` — it would kill the owner's own running dashboard.
  Kill your own dashboard by its specific PID.
- `--host` belongs to `easy-personal mcp`, not to `easy-personal dashboard`; on `dashboard` it exits
  2 with `unrecognized arguments`, which reads like a refusal but tests nothing.

## Approach

**Vital slice**: one directive added to one constant, one literal test assertion updated, the guard
test amended so it still catches a widening, and a real browser run that proves a `data:` image now
renders while a remote one, a remote font and a script still do not. **Cut list**: no per-route CSP,
no new route, no sanitiser, no other directive, no page change.

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/surfaces/api.py` | the `CSP` constant and the comment above it (the comment names the egress invariant and must now say why `data:` is outside it) |
| `tests/surfaces/test_api.py` | AC 1, AC 2 (the literal string at line 77), AC 3 (the guard test) |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `Guard._allowed`, `Guard.send_csp` | The token and Host/Origin checks are out of scope |
| `web/` | The page does not change |
| `src/easy_personal/core/kit.py` | The routes do not change |
| `templates/report_template.html` | The project's own reports have no image; this task changes a header |
| `/home/hungnguyenhuu/workspace/project/phanpy/orderly/` | The owner's own project — read and copy from it, never write to it |
| `.claude/`, `agents/`, `skills/`, `scripts/` | Kit framework files |

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table
- [ ] UI Evidence rows ☑ N/A with the reason above
- [ ] Supervisor notified: task ready for Stage 4 review

## Questions Log

* 2026-11-08 The owner reported the defect and approved the fix in one word ("merge") after being
  shown the measured cause, the proposed directive, and the reason `data:` does not reopen the
  egress FR-093 closed. No open question. The alternative not taken: re-write each report's
  `data:` image into a file served by a new route, which needs a route, a cache and a lifecycle for
  bytes the report already carries.
