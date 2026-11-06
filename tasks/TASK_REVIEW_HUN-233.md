# TASK_REVIEW — HUN-233: Live tail call labels, sub-agent type and kit stage, shown never stored (FR-081..FR-083)

> Sibling of `tasks/TASK_GUIDE_HUN-233.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
Owner instruction (not a decision): commit the plan and start HUN-233 in auto mode ("yes, commit and start HUN-233 [auto]", 2026-11-06).
1. Requirement Fidelity Gate stays signed by the owner (four AskUserQuestion choices + path A, 2026-11-06) — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` row 13
2. spawned on claude · C2 → claude-sonnet-5-5 — FACT: PROJECT_SPEC.md:165 "| claude | `claude` | `claude-haiku-5-5` | `claude-sonnet-5-5` | `claude-sonnet-5-5` |"
3. touched `src/easy_personal/surfaces/api.py` (two whitelist tuples) though the guide's Files table did not list it — FACT: tasks/TASK_GUIDE_HUN-233.md:68 "pasted `/api/live` lines show `label`, `agent` and a snapshot `stage`"; src/easy_personal/surfaces/api.py:129 "_EVENT_KEYS = (" (the route copies only these keys)
4. Stage 4 security fix applied: a label that looks like a secret is dropped, with the FR-076 check `reports._SECRET` (fdcc0e1) — RULE: PROJECT_SPEC.md:118 "no message text, tool input/output, trace `summary`, easy-ui `target`, file contents or secrets in the store, MCP/API responses, page, or logs"
5. security review done inline: the built-in `security-review` failed (`fatal: ambiguous argument 'origin/HEAD...'`) — RULE: memory/learnings.md:31 "built-in `security-review` diffs `origin/HEAD` (remote here is `github`) on the current checkout, so the Supervisor reviews worktree diffs inline"
6. board not moved to Ready for Review — FACT: memory/MEMORY.md:55 "Linear has no review state → push `done`, pull, merge" (the owner moves it to Done after /verify)
Checks: tests `uv run pytest tests/core/test_live.py tests/surfaces/test_api_live.py tests/leak -q` — 176 passed (Supervisor re-run after the fix); full suite — 1609 passed, 2 skipped (web/ not built), 1 xfailed (Supervisor re-run on fdcc0e1); ruff clean; reviews code-review — 0 P0, 0 P1, 1 P2 (guide Files table missed `api.py`), 2 P3 (meta read under `_mutex`; sidechain line in a main file shows `agent: null` → handle in HUN-234), lane `C2, 6 files, 606 lines, EXCEEDS (lines 606 > 500); unpredicted: src/easy_personal/surfaces/api.py` (339 of the lines are tests); security-review (inline) — 1 Medium fixed (secret-looking Bash first word reached `/api/live`), probes clean (pattern + 64-char cap, `O_NOFOLLOW` 4 KB meta read, `cd` regex ≈ 1.5 ms on 4 KB, `RecursionError` caught, no new route); easy-verifier — N/A (`target repository path does not exist` for the worktree path); real run by the implementer: dashboard on temp homes, `/api/live` carried `"label":"easy-kit:tdd"`, `"label":"uv"` (input `cd /work/demo-app && uv run pytest SECRET-arg`), `"agent":"backend-developer"`, `{"server":"easy-verifier","tool":"score"}`, snapshot `"stage":"3","stage_skill":"tdd"`, `SECRET in output: False` (auto-checked, not /verify)
Not done: merge (yours); /verify (yours); memory update; Done; HUN-234 (frontend, after merge)

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_live.py` (AC 1–6 + secret-label test), `tests/surfaces/test_api_live.py` (wire keys, resume), `tests/leak/test_live_leak.py` + `tests/leak/canaries.py` (AC 7): `176 passed, 1 warning in 59.58s` |
| Verification command run | ☑ pass | `uv run pytest tests/core/test_live.py tests/surfaces/test_api_live.py tests/leak -q && uv run pytest -q` → `176 passed` and `1609 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 162.21s` (Supervisor, fdcc0e1) |
| Negative cases hold | ☑ pass | pattern rejects (65 chars, leading `-`/`.`, newline, non-string), other tools no label, secret-looking labels dropped, bad/missing/oversized/symlinked meta → `sub-agent`, canaries in `args`/`prompt`/`description`/meta `description`/Bash args never on any surface |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `main...HEAD` diff (live.py, api.py, tests) + callers `surfaces/api.py` `/api/live`, `surfaces/cli.py:399` tail start; web/ skipped (unchanged, HUN-234) |
| Full smoke suite still green (no regression) | ☑ pass | `1609 passed, 2 skipped, 6 deselected, 1 xfailed` (2 skips = `tests/test_web_toolchain.py`, web/ not built) |
| **Companion score (easy-verifier)** | ☑ N/A | not scored: the MCP refused the worktree path (`target repository path does not exist`), known since 2026-11-01 |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | pure backend task; the page change is HUN-234 |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | pure backend task; the page change is HUN-234 |
| **UI: Responsiveness at target viewports** | ☑ N/A | pure backend task; the page change is HUN-234 |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (command: `uv run pytest tests/core/test_live.py -q -k "label or stage or agent_type"` with the new AC 1–6 tests written, before any implementation change):

```
$ date -u +%Y-%m-%dT%H:%M:%SZ
2026-11-06T11:03:26Z
$ uv run pytest tests/core/test_live.py -q -k "label or stage or agent_type"
...
>       resumed = [(e["label"], e["agent"]) for e in tail.events_since(0) if e["kind"] == "tool_use"]
E       KeyError: 'label'
tests/core/test_live.py:1249: KeyError
FAILED tests/core/test_live.py::test_label_skill_and_agent_calls_carry_the_safe_field_and_no_canary
FAILED tests/core/test_live.py::test_label_bash_is_the_first_word_after_a_leading_cd[cd /x && uv run pytest-uv]
FAILED tests/core/test_live.py::test_label_mcp_tool_is_split_at_the_first_two_double_underscores_without_reading_input
FAILED tests/core/test_live.py::test_agent_type_of_a_subagent_stream_comes_from_its_meta_file_and_the_main_thread_has_none
FAILED tests/core/test_live.py::test_agent_type_meta_file_is_read_once_per_stream
FAILED tests/core/test_live.py::test_stage_follows_the_newest_mapped_skill_and_keeps_unmapped_ones_out
FAILED tests/core/test_live.py::test_stage_map_is_the_fixed_prd_table - Attri...
FAILED tests/core/test_live.py::test_stage_changes_bump_state_version_once_each_and_other_calls_do_not
FAILED tests/core/test_live.py::test_label_and_agent_reach_the_buffer_for_resume_and_the_snapshot_carries_the_stage
(... 51 failed in total, listed in full by pytest)
51 failed, 1 passed, 48 deselected in 2.43s
```

**AFTER**: same command, post-change (backend-developer run, 2026-11-06): `uv run pytest tests/core/test_live.py -q -k "label or stage or agent_type"` passes; verification command `uv run pytest tests/core/test_live.py tests/surfaces/test_api_live.py tests/leak -q` gave `170 passed`; full `uv run pytest -q` gave `1603 passed, 2 skipped, 6 deselected, 1 xfailed`. Real run: `/api/live` event `{"kind":"tool_use","tool":"Skill",...,"label":"easy-kit:tdd","agent":null}`, a sub-agent event `"label":{"server":"easy-verifier","tool":"score"},"agent":"backend-developer"`, snapshot `"stage":"3","stage_skill":"tdd"`.

**DELTA**: The `/api/live` stream now tells which skill, sub-agent type, Bash command name or MCP tool each call is, which sub-agent made it, and which kit stage the session is in, with nothing stored.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
