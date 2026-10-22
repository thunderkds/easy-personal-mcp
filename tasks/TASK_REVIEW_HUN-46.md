# TASK_REVIEW — HUN-46: Catalog breadth (Gmail, SendGrid, Teams, Discord)

> Sibling of `tasks/TASK_GUIDE_HUN-46.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/connectors/test_catalog.py` (claude_ai_link schema: required with yes, refused with no, https only; catalog load = 5 providers, 2 categories, all pinned; per-entry command + secret flags = approved servers) — AC1/AC2. `tests/surfaces/test_connector_list.py` (grouping, connector-first ordering, links, planted secret absent) — AC2/AC3. `uv run pytest -q` → `304 passed, 2 skipped, 1 warning in 23.10s` |
| Verification command run | ☑ pass | `uv run pytest tests/connectors/test_catalog.py -q` within full run: 304 passed, 2 skipped; `uv run ruff check .` → `All checks passed!` |
| Negative cases hold | ☑ pass | http:// and javascript: links refused; link with `no` refused; missing link with `yes` refused; unpinned npx still refused (existing guard); planted `SENDGRID_API_KEY=INVENTED_SG_CANARY` → `grep -c INVENTED_` on list output = 0 |
| verify | ☑ pass | Isolated `uv tool install --reinstall --no-cache .` (temp EASY_PERSONAL_HOME/UV_TOOL_DIR/UV_TOOL_BIN_DIR), `easy-personal connector list` rc=0: Email (gmail with link, sendgrid), Messaging (slack, teams with links, discord); `connector --help` shows `{list,add,remove}` — pass. No provider server, npx, real claude CLI or keychain used. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | main...HEAD (13 files) + callers of `catalog.parse/load` (writer, launch, cli). lane_check: `lane: C1, 12 files, 138 lines, EXCEEDS (files 12 > 6); unpredicted: tests/connectors/conftest.py, tests/connectors/test_writer.py` — test-fixture scoping for the fake `claude mcp get`, justified. code-review: P0 0, P1 0, P2 1 (lane exceeds size label, test files), P3 1 (`connector` parser help still says "add or remove"). Low risk → security-review not required; no DB change → migration-safety N/A. |
| Full smoke suite still green (no regression) | ☑ pass | 304 passed, 2 skipped (main was 294 + 10 new) |
| easy-verifier score (advisory) | ☑ recorded | scope=changes: overall `rating_abstention no_dimension_rated`; all 13 dimensions + change-risk abstain (below_coverage_floor / insufficient_measured_weight / no_static_rule) — change too small to rate |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Backend/CLI-only task; no UI component |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Backend/CLI-only task; no UI component |
| **UI: Responsiveness at target viewports** | ☑ N/A | Backend/CLI-only task; no UI component |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured before any implementation commit):

```
Thu Oct 22 07:42:30 AM UTC 2026
$ uv run easy-personal connector list
Using CPython 3.13.2
Creating virtual environment at: .venv
   Building easy-personal-mcp @ file:///home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-46
      Built easy-personal-mcp @ file:///home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-46
Installed 43 packages in 5ms
usage: easy-personal connector [-h] {add,remove} ...
easy-personal connector: error: argument action: invalid choice: 'list' (choose from add, remove)
$ uv run pytest tests/connectors/test_catalog.py -q
.........................                                                [100%]
25 passed in 0.03s
```

**AFTER** (Supervisor, isolated install, Thu Oct 22 07:45:13 AM UTC 2026):

```
$ easy-personal connector list
Email
  gmail Gmail | claude.ai connector: yes (https://claude.ai/settings/connectors)
    fields: OAuth client keys file (gcp-oauth.keys.json)
  sendgrid SendGrid | claude.ai connector: no
    fields: API key (SG....)
Messaging
  slack Slack | claude.ai connector: yes (https://claude.ai/settings/connectors)
    fields: Bot token (xoxb-...)
  teams Teams | claude.ai connector: yes (https://claude.ai/settings/connectors)
    fields: Tenant ID
  discord Discord | claude.ai connector: no
    fields: Bot token
$ uv run pytest -q
304 passed, 2 skipped, 1 warning in 23.10s
```

**DELTA**: The owner can list five providers by category, with the official claude.ai connector offered first, where before only Slack existed and `connector list` was an invalid choice.

**WITNESS**: Supervisor, 2026-10-22 07:45 UTC, isolated install from the HUN-46 worktree (trace attributed to HUN-46 via the active_task state file).
