# TASK_REVIEW — HUN-33: Skeleton package

> Sibling of `tasks/TASK_GUIDE_HUN-33.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | pass | `tests/test_skeleton.py` (6 tests: version, CLI==MCP core value, stdio no stdout noise, 0.0.0.0 refused, default host loopback, data dir 0700) |
| Verification command run | pass | 2026-10-21T09:01Z: `6 passed in 0.61s`; `All checks passed!`; `Installed 1 executable: easy-personal`; `easy-personal --version` -> `0.1.0` |
| Negative cases hold | pass | `easy-personal mcp --transport http --host 0.0.0.0` -> `error: refusing host '0.0.0.0': HTTP must bind 127.0.0.1 only`, exit 2 |
| verify | ☑ pass | /verify 2026-10-21, cold start (no verifier skill): installed into a temp uv tool dir; `easy-personal --version` → 0.1.0; `ping` → `easy-personal core ok`; real MCP stdio handshake initialize → tools/list `['ping']` → tools/call → `easy-personal core ok`; HTTP on 127.0.0.1:18765 listens on 127.0.0.1 only and answers initialize; probes 0.0.0.0 / localhost / ::1 refused exit 2, bad --port / --transport rejected exit 2 — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | pass | 11 files in `main..feat/HUN-33-skeleton` + their callers (none outside the diff); kit files out of scope. code-review 2026-10-21: 0 P0/P1, 2 P2, 2 P3 — `reports/code-review_feat-HUN-33-skeleton_20261021T160239.html` (main checkout) |
| Full smoke suite still green (no regression) | pass | 6 passed |
| **UI: Visual regression (diff or verdict pasted)** | ☐ N/A | No UI in this task (pure infra skeleton). |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ N/A | No UI in this task (pure infra skeleton). |
| **UI: Responsiveness at target viewports** | ☐ N/A | No UI in this task (pure infra skeleton). |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-21T09:00:30Z-09:00:34Z in the worktree, before any code exists (no pyproject.toml, no src/):

```
$ uv run pytest -q
no tests ran in 0.03s            # exit=5, so the && chain stops here
$ which easy-personal
easy-personal not found          # exit=1
```

**AFTER**: 2026-10-21T09:01:12Z

```
$ uv run pytest -q && uv run ruff check && uv tool install --force . && easy-personal --version
6 passed in 0.61s
All checks passed!
Installed 1 executable: easy-personal
0.1.0
$ easy-personal ping
easy-personal core ok
$ printf <initialize, initialized, tools/list> | easy-personal mcp
{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"ping", ...
```

**DELTA**: The owner can install `easy-personal` with `uv tool install .`, run it, and serve a stdio MCP server whose one tool calls the same core as the CLI.

**WITNESS**: common-infrastructure agent, 2026-10-21T09:01Z (reviewer sign-off pending).
