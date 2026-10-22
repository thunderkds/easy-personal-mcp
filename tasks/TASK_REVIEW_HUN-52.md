# TASK_REVIEW — HUN-52: Dashboard connector forms + guarded write API

> Sibling of `tasks/TASK_GUIDE_HUN-52.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/surfaces/test_api_connectors.py` (22 tests: list order + configured, guard 403 on GET/POST/DELETE, 415/400, preview writes nothing, confirm writes via `writer.add` with no echo, double submit, empty value, claude failure rollback, missing claude/keychain, remove, non-catalog DELETE 404 with no claude call, broken keychain → 200 configured false) — AC3/4/5/6; `web/src/ConnectorsView.test.jsx` (AC1 password inputs from catalog, AC2 link first, AC6 confirmation, cancel/clear, refusal, double click) ; leak suite `API_WRITE_CALLS` + scanned-routes check — AC4. Supervisor run on 659e6e4: web `Tests 50 passed (50)`; `uv run pytest -q` → `329 passed, 1 warning in 27.05s` |
| Verification command run | ☑ pass | `npm --prefix web test && npm --prefix web run build && uv run pytest ...` → 50 web tests passed, `✓ built in 133ms`, 329 passed; ruff `All checks passed!` |
| Negative cases hold | ☑ pass | foreign Origin → 403, non-JSON → 415 (implementer live run); DELETE non-catalog → 404 (test); planted `SG.INVENTED_UI_CANARY` typed in the browser: page text after preview and after apply has no `INVENTED_`, inputs cleared (`inputValues: []`), `grep -c INVENTED_` on fake-claude argv log and dashboard log = 0 |
| verify | ☑ pass | Isolated install of 659e6e4 (temp home/tool dirs), dashboard with fake `claude` on PATH + in-memory keyring (`PYTHON_KEYRING_BACKEND=fakekr.Keyring`), driven by headless Chrome over CDP: Connectors tab → Set up SendGrid → password input → Review changes (plan with `SENDGRID_API_KEY=***`) → Apply → `connector 'sendgrid' is configured.`; fake claude argv: `mcp add-json sendgrid {"type": "stdio", "command": "…/easy-personal-launch", "args": ["exec", "sendgrid"]} --scope user` (no secret). Requests: 127.0.0.1 only — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | main...HEAD + `Guard` middleware + `writer.add/remove`. lane_check: `lane: C2, 13 files, 835 lines, EXCEEDS (lines 835 > 500)` — tests are ~half; noted. code-review: P0 0, P1 0; P2 2 fixed (cdeba5a: DELETE limited to catalog ids → 404; KeyringError on GET → configured false); P2 open: `_configured` duplicated with HUN-47 `suggestions._configured`, both use private writer helpers — follow-up; P3: preview ends with writer's "aborted; nothing was written." line; links use browser default color. security-review (inline, High): Guard covers all `/api/` (token header, Host, Origin), JSON content-type blocks form CSRF, body schema validated, unknown provider generic 400, writes serialized by lock, secrets only to keychain, masked `***` in plan — no open findings. blast-radius: T1 credentials (provider API tokens: `SENDGRID_API_KEY`, `DISCORD_TOKEN`, `SLACK_MCP_XOXB_TOKEN`) stored only in OS keychain, never in store/log/response; population 1 (single owner, local tool); top vectors (all Low): (1) token from URL fragment via local browser history → attacker can add catalog connectors with fixed pinned commands or delete them, cannot read secrets; (2) same-user local process — already has keychain access, no new exposure; (3) DNS rebinding/CSRF — blocked by Host/Origin + token + JSON type. Regulatory: personal/household processing (GDPR Art. 2(2)(c)); no fine exposure estimated; not legal advice. Hardening candidate: none required. |
| Full smoke suite still green (no regression) | ☑ pass | 329 passed; web 50 passed |
| easy-verifier score (advisory) | ☑ recorded | scope=changes: overall `rating_abstention no_dimension_rated`; 13 dimensions + change-risk abstain |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | `reports/HUN-52/connectors-{375,768,1280}.png`, `plan-1280.png`, `applied-1280.png` (local, gitignored). Supervisor vision review vs AC: grouped Email/Messaging, claude.ai link shown before the setup button for Gmail/Slack/Teams (AC2), password input (`inputType: password`), plan shown before write (AC6), no broken layout |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | CSS audit of the `web/src/app.css` diff: every color/spacing/radius/size is a `var(--…)` token; no hex/rgb literals in app.css or ConnectorsView.jsx; inherited font (`font: inherit`). P3: `a` links keep the browser default color (pre-existing, no link token) |
| **UI: Responsiveness at target viewports** | ☑ pass | `report.json`: 375 → scrollWidth 375 = clientWidth 375; 768 → 753 = 753; 1280 → 1265 = 1265 (no horizontal scroll); single column at 375 |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-10-22 07:49:13 UTC, before any implementation commit; running `easy-personal dashboard` with a temp `EASY_PERSONAL_HOME`, HEAD of branch HUN-52 = main 63757e7):

```
Thu Oct 22 07:49:13 AM UTC 2026
GET /api/connectors:            -> HTTP 404
POST /api/connectors (json {}): -> HTTP 404 "Not Found"
tabs in web/src/App.jsx:  Usage, Warnings & plan, Kit   (no Connectors tab)
```

**AFTER** (2026-10-22 08:03:09 UTC, implementer run; running dashboard, temp home, `PYTHON_KEYRING_BACKEND=keyring.backends.fail.Keyring`, no `claude` on PATH):

```
GET  /api/connectors                      -> 200 {"connectors":[{"id":"gmail",...,"configured":false},...
POST /api/connectors (no JSON type)       -> 415 {"error":"content-type must be application/json"}
POST /api/connectors (Origin evil.example)-> 403 {"error":"forbidden"}
POST confirm:true, claude missing         -> 200 {"status":0,"lines":["the `claude` CLI is not on PATH; nothing was written. ...claude mcp add-json slack ..."],"pending":false}
tabs: Usage, Warnings & plan, Kit, Connectors
```

**DELTA**: The owner can set up or remove a catalog connector from the dashboard, seeing the exact keychain and `claude mcp` change before it is written, where before only the CLI could.

**WITNESS**: Supervisor, 2026-10-22 08:07 UTC, isolated install + headless Chrome flow from the HUN-52 worktree (trace attributed to HUN-52 via the active_task state file).
