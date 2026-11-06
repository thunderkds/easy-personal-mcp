# TASK_REVIEW — HUN-235: Ingest stores secret-looking Bash first words in tool_signals (no secret check)

> Sibling of `tasks/TASK_GUIDE_HUN-235.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
Owner instruction (not a decision): fix the `tool_signals` secret leak first, auto mode ("bugfix first [auto]", 2026-11-06).
1. Mental model signed `auto (Supervisor)`, not by the owner — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` row 2
2. no migration to delete stored rows; ingest check only — DEBATED: (A) ingest check only (Advocate 1: (A) ingest check only; Advocate 2: (B) also ship m0012 delete migration) ⚠ decided for you — please review
   Trace note: both Advocate `Agent` calls ran at 2026-11-06T11:39:12Z ("Advocate 1: ingest check only", "Advocate 2: ingest check + m0012 purge") but were logged to `memory/event-trace/HUN-233.jsonl`, because the active-task file still named HUN-233 then; the auto-mode check reads only HUN-235's trace. Records left unedited (audit trail).
3. spawned on claude · C1 → claude-sonnet-5-5 — FACT: PROJECT_SPEC.md:165 "| claude | `claude` | `claude-haiku-5-5` | `claude-sonnet-5-5` |"
Checks: tests `uv run pytest tests/sources tests/connectors/test_mcp_tools.py tests/leak -q` — 207 passed; full suite — 1612 passed, 2 skipped (web/ not built), 1 xfailed; ruff clean (Supervisor re-run on 6d2b809); reviews code-review — 0 P0, 0 P1, 0 P2, 1 P3 (`live._safe` still reaches the alias `reports._SECRET`; move to `secretlike.SECRET` later), lane `C1, 6 files, 141 lines, FITS`; security-review (built-in Skill, `origin/HEAD` refreshed to main 9b44ac0) — no high-confidence vulnerabilities (pattern byte-identical, FR-076 and live labels unchanged, `_bash_word` only gets stricter); easy-verifier — N/A (MCP refuses worktree paths, see HUN-233); real run: `easy-personal ingest` on a temp home with `sk-ant-api03-VERIFYSECRETKEY run` + `cd /x && git status` → `tool_signals` = `bash|cd` only, VERIFYSECRET in dump 0 (auto-checked, not /verify)
Not done: merge (yours); memory update; Done

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/sources/test_claude_code.py` (AC 1-2, edge cases), `tests/leak/test_leak_suite.py::test_secret_looking_bash_first_word_is_not_in_the_store_dump` (AC 4) |
| Verification command run | ☑ pass | `uv run pytest tests/sources tests/connectors/test_mcp_tools.py tests/leak -q` -> 207 passed, 1 deselected; `uv run pytest -q` -> 1612 passed, 2 skipped, 6 deselected, 1 xfailed (2026-11-06) |
| Negative cases hold | ☑ pass | `git`, `uv`, `python3`, `task-runner`, `bash -c "sk-…"` → `bash` still stored; domain signals and other words of the same message kept; `sk-learn` dropped (accepted) |
| verify | ☑ pass | owner-invoked `/verify HUN-235` 2026-11-06 on 931774a, temp install (0.4.0): `ping` ok; MCP stdio 12 tools + ping ok; `--host 0.0.0.0/localhost/::1` exit 2; installed `easy-personal ingest` of `sk-ant-api03-VERIFYSECRETKEY run`, `ghp_VERIFYSECRET… x`, `git status`, WebFetch `https://api.slack.com/x` → `tool_signals` = `bash|git`, `domain|api.slack.com`; VERIFYSECRET in store dump 0 — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `main...HEAD` diff + callers of the pattern (`reports._check_text`, `live._safe`) and of `_bash_word` (`_signals` → `parse_record`); suggestions/store untouched |
| Full smoke suite still green (no regression) | ☑ pass | `1612 passed, 2 skipped, 6 deselected, 1 xfailed` (2 skips = web/ not built) |
| **Companion score (easy-verifier)** | ☑ N/A | not scored: the MCP refuses worktree paths (`target repository path does not exist`, HUN-233) |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | pure backend bugfix |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | pure backend bugfix |
| **UI: Responsiveness at target viewports** | ☑ N/A | pure backend bugfix |
| Repro loop | ☑ pass | `uv run pytest tests/sources/test_claude_code.py -q -k "bash"` (2 failed before the fix, 2026-11-06T11:43:33Z) |
| Regression test | ☑ pass | `tests/sources/test_claude_code.py` (two `_bash_` tests), `tests/leak/test_leak_suite.py` |
| Smoke suite | ☑ pass | real `easy-personal ingest` on a temp home: `tool_signals` = `bash|cd`, no `sk-ant…` row, VERIFYSECRET in dump 0 |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: see Phase 1 repro loop. Captured 2026-11-06T11:43:33Z, before any fix edit or commit
(`uv run pytest tests/sources/test_claude_code.py -q -k "bash"`):

```
>       assert _bash_words(conn) == []
E       AssertionError: assert ['AKIAABCDEFG...b-1234567890'] == []
E         Left contains 5 more items, first extra item: 'AKIAABCDEFGHIJKL'
>       assert _bash_words(conn2) == ["git"]
E       AssertionError: assert ['git', 'sk-ant-api03-abcdef'] == ['git']
FAILED tests/sources/test_claude_code.py::test_secret_looking_bash_first_word_stores_no_signal_row
FAILED tests/sources/test_claude_code.py::test_ordinary_bash_first_words_are_stored_as_before
2 failed, 21 deselected in 0.10s
```

**AFTER**: the same Phase 1 repro loop, re-run on 6d2b809 by the Supervisor: `uv run pytest tests/sources/test_claude_code.py tests/leak/test_leak_suite.py -q` → 55 passed (both `_bash_` tests green); real `easy-personal ingest` → `tool_signals` = `bash|cd`, no `sk-ant…` row.

**DELTA**: a key pasted as the first word of a Bash command (e.g. `sk-ant-…`) is no longer saved in the store; ordinary command names still are.

**WITNESS**: Supervisor (main session), 2026-11-06 — re-ran the repro loop, the full suite and a real ingest on 6d2b809 (trace `memory/event-trace/HUN-235.jsonl`)