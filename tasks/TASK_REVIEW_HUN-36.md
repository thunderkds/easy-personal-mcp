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
| verify | ☒ pass / ☐ fail / ☐ N/A | 2026-10-21T10:55Z Supervisor, HUN-36 worktree: `uv run pytest tests/leak -q` → `18 passed in 3.46s`. Independent mutation (not the implementer's): `sources/claude_code.py:140` `tool_names=str(blocks)[:500]` → `LEAKS: {'store .dump': [('assistant_text','raw'), ('assistant_text','json'), ('thinking','raw'), ('tool_input','raw'), ('file_path','raw'), ...]}`, `1 failed, 17 passed`; reverted with `git checkout`, re-run `18 passed` — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☒ pass / ☐ fail | 9 files in `main...HUN-36` (tests/leak, fixture template, pyproject pytest config) + the surfaces they drive (`surfaces/cli.py`, `surfaces/mcp.py`, `sources/claude_code.py`, read only). lane_check: `C2, 8 files, 495 lines, FITS; unpredicted: pyproject.toml`. code-review 0 P0/P1/P2, 1 P3 (flag-level variants are not meta-tested; registering tasks add them) — `reports/code-review_HUN-36_20261021T175604.html`; security (inline) pass, no findings — `reports/security-review_HUN-36_20261021T175604.html` (main checkout) |
| Full smoke suite still green (no regression) | pass | `uv run pytest -q` -> `77 passed, 2 skipped in 5.96s`; `uv run ruff check .` -> All checks passed |
| easy-verifier score (advisory) | ☒ recorded / ☐ N/A | 2026-10-21T11:01Z, server at 127.0.0.1:8766 called directly over HTTP (MCP tool not loaded in this profile), `score` repo=main checkout, scope=changes, ref=`6781187...1f480dc`: overall **66** (4 of 13 rated: requirement-fidelity 67, security 30, test-strategy 100, blast-radius 65); abstained: api, architecture, code-quality, data, infrastructure, operations, reliability, supply-chain, change-risk (no changed source code). Security 30 = known false positive `redaction_hits_observed` 121 (lockfile pack + the suite's own planted canaries/README). Report `~/.easy-verifier-reports/easy-personal-mcp/evidence-report-changes-20261021T110126-172674Z.html`. Advisory only. |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☒ N/A | Test-only task (no UI component) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☒ N/A | Test-only task (no UI component) |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☒ N/A | Test-only task (no UI component) |

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

**WITNESS**: Supervisor, 2026-10-21T10:55Z — re-ran `uv run pytest tests/leak -q` and the independent mutation above in the HUN-36 worktree (calls attributed to HUN-36 via the active_task state file; `memory/event-trace/HUN-36.jsonl`).
