# TASK_REVIEW — HUN-34: Tracer bullet claude_code source -> store -> usage_summary

> Sibling of `tasks/TASK_GUIDE_HUN-34.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☒ pass / ☐ fail | Implementer-filled, reviewer to confirm. New: `tests/sources/test_claude_code.py` (AC2, AC3, AC4, AC5, AC8), `tests/core/test_store.py` (AC1, AC6, SC2 dump grep), `tests/surfaces/test_usage_summary.py` (SC1 Supervisor oracle, SC3, AC7 MCP/CLI parity, period); `tests/test_skeleton.py` +2 (carried HUN-33 fixes). `uv run pytest -q` → `46 passed in 0.88s` (2026-10-21T10:23:40Z) |
| Verification command run | ☒ pass / ☐ fail | 2026-10-21T10:23:40Z `uv run pytest tests/sources/test_claude_code.py tests/core/test_store.py tests/surfaces/test_usage_summary.py -q` → `33 passed in 0.43s`; `uv run ruff check` → `All checks passed!` |
| Negative cases hold | ☒ pass / ☐ fail | Implementer-filled: truncated last line never consumed; bad JSON / non-dict / string token / usage without `message.id` abstain and feed no metric; free text in `gitBranch` / tool name dropped; planted `sk-FAKE-0000000000` and `INVENTED_*` absent from `iterdump()`; bad `--since` → exit 2; missing source root → `{"files": 0, "new_events": 0}`. Mutation checks A–H in the log below each turn a test red. |
| verify | ☒ pass / ☐ fail / ☐ N/A | 2026-10-21 Supervisor /verify (temp `EASY_PERSONAL_HOME`, `uv tool install .`, fixtures copied into a temp `EASY_PERSONAL_CLAUDE_PROJECTS`): `--version` → `0.1.0`; `ingest` → `{"files": 5, "new_events": 31}`, second run `new_events: 0` (idempotent); `usage` → sessions 4, messages 3, input 15, output 8, cache_read 1000, cache_hit_ratio 0.7905, abstained 3 by version; `--since 2026-01-15` → `since_utc 2026-01-14T17:00:00Z` (local day); `--since notadate` → exit 2; MCP stdio `tools/list` → `['ping', 'usage_summary']`, `usage_summary` isError False with same keys; grep `INVENTED_\|planted-secre` over CLI/MCP stdout+stderr and the store dir → no leak — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | pass | 14 files in `main...feat/HUN-34-claude-code-tracer` + callers; code-review 0 P0/P1, 1 P2 (bounded-memory test → HUN-38), 2 P3 — `reports/code-review_feat-HUN-34-claude-code-tracer_20261021T172530.html`; security (inline) pass — `reports/security-review_feat-HUN-34-claude-code-tracer_20261021T172531.html` (main checkout) |
| Full smoke suite still green (no regression) | ☒ pass / ☐ fail | 2026-10-21 Supervisor: `uv run pytest -q` → `46 passed in 0.90s` |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task (no UI component) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task (no UI component) |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task (no UI component) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured by the implementing agent at HEAD `50c240c` (no implementation commit yet), temp `EASY_PERSONAL_HOME` under the session scratchpad:

```
2026-10-21T10:13:43Z
$ uv run easy-personal ingest
usage: easy-personal [-h] [--version] {ping,mcp} ...
easy-personal: error: argument command: invalid choice: 'ingest' (choose from ping, mcp)
exit=2
2026-10-21T10:13:44Z
$ uv run easy-personal usage
usage: easy-personal [-h] [--version] {ping,mcp} ...
easy-personal: error: argument command: invalid choice: 'usage' (choose from ping, mcp)
exit=2
```

**AFTER**: implementing agent, branch `feat/HUN-34-claude-code-tracer`, temp `EASY_PERSONAL_HOME`, `EASY_PERSONAL_CLAUDE_PROJECTS=tests/fixtures/claude_code`:

```
2026-10-21T10:22:35Z
$ uv run easy-personal ingest
{
  "files": 5,
  "new_events": 31
}
exit=0
$ uv run easy-personal ingest   # second run
{
  "files": 5,
  "new_events": 0
}
exit=0
2026-10-21T10:22:35Z
$ uv run easy-personal usage
{
  "abstained": {
    "by_version": {
      "2.1.293": 1,
      "2.1.295": 1,
      "2.1.302": 1
    },
    "total": 3
  },
  "cache_creation_input_tokens": 250,
  "cache_hit_ratio": 0.7905,
  "cache_read_input_tokens": 1000,
  "input_tokens": 15,
  "messages": 3,
  "output_tokens": 8,
  "period": {
    "since_utc": null,
    "until_utc": null
  },
  "sessions": 4
}
exit=0
```

**DELTA**: the owner can now copy Claude Code transcript metadata into `~/.easy-personal/store.sqlite` incrementally and read sessions, tokens and cache-hit ratio for a period from the CLI (`easy-personal usage`) or the MCP tool `usage_summary`, with the same numbers.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]

---

## Test-first log (implementer, RED/GREEN)

Steps 2-4 passed on first run because the step-1 GREEN code already carried offsets, abstain and
discovery. A true RED was therefore not available for them. Each guard was mutated instead, and the
matching test was shown to fail (mutations reverted from a backup copy, then GREEN re-run).

```
### Step 1 RED 2026-10-21T10:16:10Z — uv run pytest tests/surfaces/test_usage_summary.py -q
E   ImportError: cannot import name 'metrics' from 'easy_personal.core' (/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-34/src/easy_personal/core/__init__.py)
=========================== short test summary info ============================
ERROR tests/surfaces/test_usage_summary.py
!!!!!!!!!!!!!!!!!!!! Interrupted: 1 error during collection !!!!!!!!!!!!!!!!!!!!
1 error in 0.08s

### Step 1 GREEN 2026-10-21T10:17:55Z
2 passed in 0.02s

### Step 2 (incremental) — tests passed on first run (offset code landed in step 1). Mutation checks instead of RED:
#### Mutation A 2026-10-21T10:18:12Z: partial-line guard removed ('if not line.endswith(b"
")' -> 'if not line')
151:            if not line:
FAILED tests/sources/test_claude_code.py::test_truncated_last_line_not_consumed_until_complete
FAILED tests/sources/test_claude_code.py::test_fixture_truncated_lines_never_consumed
FAILED tests/surfaces/test_usage_summary.py::test_oracle_totals_over_all_fixtures
3 failed, 6 passed in 0.06s
#### Mutation B 2026-10-21T10:18:12Z: inode/shrink restart removed
190:        if False:
FAILED tests/sources/test_claude_code.py::test_replaced_file_new_inode_restarts_offset
1 failed, 6 passed in 0.05s
#### Mutation C 2026-10-21T10:18:12Z: offsets ignored (always start at 0, no UNIQUE dedupe would hide it -> check)
9 passed in 0.04s
#### Restored 2026-10-21T10:18:13Z
7 passed in 0.04s
#### Mutation C re-run with new test_rerun_resumes_from_offset_without_reparsing 2026-10-21T10:18:26Z
FAILED tests/sources/test_claude_code.py::test_rerun_resumes_from_offset_without_reparsing
1 failed, 7 passed in 0.05s
#### Step 2 GREEN 2026-10-21T10:18:26Z
8 passed in 0.03s

### Step 3 (schema/privacy/abstain) — passed on first run; mutation checks:
#### Mutation D 2026-10-21T10:19:13Z: 'frame-link' removed from METADATA_TYPES
FAILED tests/sources/test_claude_code.py::test_unversioned_metadata_types_are_known_not_abstained
FAILED tests/sources/test_claude_code.py::test_unknown_types_abstained_per_version
FAILED tests/surfaces/test_usage_summary.py::test_oracle_totals_over_all_fixtures
3 failed, 20 passed in 0.09s
#### Mutation E 2026-10-21T10:19:13Z: label validation disabled (_label returns any str)
FAILED tests/sources/test_claude_code.py::test_free_text_in_label_fields_is_dropped
1 failed, 22 passed in 0.08s
#### Mutation F 2026-10-21T10:19:13Z: per-message.id dedupe removed (GROUP BY id instead of message_id)
FAILED tests/sources/test_claude_code.py::test_split_message_counted_once - a...
FAILED tests/surfaces/test_usage_summary.py::test_oracle_totals_over_all_fixtures
2 failed, 21 passed in 0.08s
#### Step 3 GREEN 2026-10-21T10:19:14Z
23 passed in 0.07s

### Step 4 (FR-009 project assignment) — passed on first run; mutation checks:
#### Mutation G 2026-10-21T10:19:40Z: worktree grouping removed (show-toplevel only)
E       AssertionError: assert {'s-0': '/tmp...0/plain', ...} == {'s-0': '/tmp...0/plain', ...}
E         
E         Omitting 4 identical items, use -vv to show
E         Differing items:
E         {'s-2': '/tmp/pytest-of-hungnguyenhuu/pytest-99/test_project_assignment_per_fr0/repo-wt'} != {'s-2': '/tmp/pytest-of-hungnguyenhuu/pytest-99/test_project_assignment_per_fr0/repo'}
E         Use -v to get more diff
#### Mutation H 2026-10-21T10:19:40Z: git skipped (cwd is always its own project)
FAILED tests/sources/test_claude_code.py::test_project_assignment_per_fr009
1 failed, 15 deselected in 0.04s
#### Step 4 GREEN 2026-10-21T10:19:40Z
16 passed in 0.06s

### Step 5 RED 2026-10-21T10:20:12Z — uv run pytest tests/surfaces/test_usage_summary.py tests/test_skeleton.py -q
FAILED tests/surfaces/test_usage_summary.py::test_mcp_and_cli_return_identical_numbers
FAILED tests/surfaces/test_usage_summary.py::test_ingest_source_flag_overrides_env
FAILED tests/surfaces/test_usage_summary.py::test_cli_usage_rejects_bad_date
FAILED tests/test_skeleton.py::test_cli_and_mcp_return_same_core_value - Asse...
FAILED tests/test_skeleton.py::test_data_dir_leaves_existing_dir_mode_alone
FAILED tests/test_skeleton.py::test_mcp_server_reports_package_version - Asse...
6 failed, 11 passed in 1.09s

### Step 5 GREEN 2026-10-21T10:21:07Z
17 passed in 0.73s

### Step 6 RED 2026-10-21T10:21:44Z — usage without message.id must abstain
E       AssertionError: assert {'total': 0, 'by_version': {}} == {'total': 1, ...'2.1.295': 1}}
E         
E         Differing items:
E         {'by_version': {}} != {'by_version': {'2.1.295': 1}}
### Step 6 GREEN 2026-10-21T10:22:11Z
45 passed in 0.82s

### Step 7 RED 2026-10-21T10:23:27Z — file deleted by cleanup between listing and reading
E           FileNotFoundError: /tmp/pytest-of-hungnguyenhuu/pytest-112/test_file_vanishing_mid_ingest0/projects/a.jsonl
FAILED tests/sources/test_claude_code.py::test_file_vanishing_mid_ingest_is_skipped
1 failed, 17 deselected in 0.05s
### Step 7 GREEN 2026-10-21T10:23:38Z
46 passed in 0.88s
```

NFR-005 spot check (not a test): synthetic 10,000,686-byte file, 2,477 lines → cold ingest 0.07 s, re-run 0.000 s, 0 new rows.
