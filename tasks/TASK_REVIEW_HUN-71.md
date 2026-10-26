# TASK_REVIEW — HUN-71: QA end-to-end across fake accounts

> Sibling of `tasks/TASK_GUIDE_HUN-71.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | pass | tests/e2e/test_workspaces_flow.py, tests/e2e/e2e_support.py, tests/e2e/e2e_filekeyring.py |
| Verification command run | pass | `uv run pytest tests/e2e tests/leak -q && uv run pytest -q`: 47 passed, 4 xfailed, 1 warning in 81.83s (0:01:21) / 730 passed, 2 skipped, 4 deselected, 4 xfailed, 1 warning in 150.55s (0:02:30) |
| Negative cases hold | pass | refusals (dup name, orphan pick, Codex pick, move clash, bad alias, wrong token) asserted on CLI and API |
| verify | ☑ pass | Supervisor re-ran 2026-10-26 ~22:10 +07 at 392bac3: `uv run pytest tests/e2e tests/leak -q -rx` -> 47 passed, 4 xfailed (the 4 defects below); `uv run ruff check .` clean. Real `~/.easy-personal/store.sqlite` mtime unchanged before/after (17:47:16 +07). The e2e flow itself is the runtime verification (CLI subprocesses + API TestClient on an isolated machine) — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Test-only change (tests/e2e/*, review file). Reviewed isolation: REAL_HOME used only for top-level lstat/listdir in the no-touch guard and audit-hook prefixes; fake keychain + FAKE_CLAUDE; HOME patched to temp. Open questions decided: (1) ~/.claude* dirs covered by the audit hook only, mtime skipped (live session rewrites them) — accepted; (2) keep the scratch account; (3) in-process ingest via `cli._ingest` for the API world — accepted; (4) ~80 s runtime, not marked slow — accepted. Defects routed to follow-up HUN-72: P2 CLI `connector remove <unknown>` deletes a hand-made server; P2 `doctor` FAIL on adopted legacy ids; P3 `connector add` EOF traceback; P3 tool_signals double count — pass |
| Full smoke suite still green (no regression) | pass | full suite: 730 passed, 2 skipped, 4 deselected, 4 xfailed, 1 warning in 150.55s (0:02:30); ruff check: All checks passed |
| easy-verifier score (advisory) | ☐ N/A | N/A — easy-verifier MCP server not loaded in this profile |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | N/A — test-only task, no UI change |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | N/A — test-only task, no UI change |
| **UI: Responsiveness at target viewports** | ☑ N/A | N/A — test-only task, no UI change |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured before the first commit):

```
$ date -u
Mon Oct 26 10:38:50 AM UTC 2026
$ ls tests/e2e
ls: cannot access 'tests/e2e': No such file or directory
$ uv run pytest -q | tail -1
709 passed, 2 skipped, 4 deselected, 1 warning in 51.75s
```

**AFTER**: `uv run pytest tests/e2e tests/leak -q` -> see Evidence (tests/e2e now exists).

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: Supervisor, 2026-10-26 ~22:10 +07 — re-ran `uv run pytest tests/e2e tests/leak -q -rx` (47 passed, 4 xfailed) at 392bac3 and checked the real store was untouched.
