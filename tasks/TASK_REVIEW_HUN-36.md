# TASK_REVIEW — HUN-36: Leak test suite

> Sibling of `tasks/TASK_GUIDE_HUN-36.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | pass | `tests/leak/test_leak_suite.py` (+ helpers `tests/leak/canaries.py`, `harness.py`, `conftest.py`; fixture `tests/fixtures/leak/session.jsonl.tmpl`). AC1 canaries/fixture: test_fixture_plants_every_canary, test_canaries_are_unique_per_run. AC2: test_no_canary_in_any_output_log_or_store. AC3: test_every_mcp_tool_is_registered, test_unregistered_new_tool_fails_meta_test. AC4: `tests/leak/README.md`. Self-test: test_leaking_tool_fails_the_suite, test_scan_detects_every_encoding. |
| Verification command run | pass | `uv run pytest tests/leak -q` -> `18 passed in 3.90s` |
| Negative cases hold | pass | Leaking tool, log, store row, 6 encodings, 3 base64 alignments detected by self-tests. Mutation: temporarily set `model=message.get("model")` in sources/claude_code.py -> `LEAKS: {'store .dump': [('model_text', 'raw'), ('model_text', 'json')], 'store file store.sqlite': [...]}`, `1 failed, 17 passed`; change reverted (git checkout). |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | pass | `uv run pytest -q` -> `77 passed, 2 skipped in 5.96s`; `uv run ruff check .` -> All checks passed |
| easy-verifier score (advisory) | ☐ recorded / ☐ N/A | [overall result, abstaining dimensions, report path — or `N/A — <reason>` if the server is down; never blocks Done] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-21T10:51:31Z in the worktree, before any implementation commit.
Command `uv run pytest tests/leak -q`:

```
ERROR: file or directory not found: tests/leak

no tests ran in 0.00s
```
`ls tests/leak tests/fixtures/leak` -> `No such file or directory` for both.

**AFTER**: same command after the change (2026-10-21):

```
$ uv run pytest tests/leak -q
..................                                                       [100%]
18 passed in 3.90s
```

**DELTA**: A developer can now run one command that plants canary secrets in transcripts and fails if any appear in the store, any MCP/CLI output or logs.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/HUN-36.jsonl`, never the
implementing agent alone]
