# TASK_REVIEW — HUN-181: Live attention states in the live tail + stream re-send (FR-047..049, NFR-012; v0.4.0)

> Sibling of `tasks/TASK_GUIDE_HUN-181.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` tag,
`docs/claude-md/tracks.md` `## Auto mode`). To use it, copy the block below out of this comment, to this
place, and replace every <...>. Keep `Mode: auto` as its first line, exactly as written: T182's hook reads
it. Write it to this file before the run ends; the chat message is a copy, not the report. The same
block is the Supervisor's final chat message. It is not the `delivery-report` page and not
Evidence. A taste decision (what the user wants) also ends with the flag. Write `DEBATED` only when ≥2
Advocate `Agent` calls ran for that question, naming each Advocate's option. A decision with no Advocates is
labelled `RULE`, `FACT` or `SAFER OPTION`, never `DEBATED`. A decision flagged `⚠ decided for you` is a choice, and is never labelled `FACT`. A
safety action a rule requires (for example the backup before a delete) is `RULE` without the flag.
At Risk Medium or High, a `RULE:` counts only when it names a written rule (`path:line`,
`path § <heading>`, or `docs/claude-md/tracks.md` `### <section>`), and a `FACT:` needs a Citation
(`path:line "quoted text"`); without one it is a choice and must be `DEBATED`. Good: `2. backed up the CSV
— RULE: `docs/claude-md/tracks.md` `### Stops that stay``. Bad: `1. kept recurring.csv — RULE: Surgical Changes`.

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its option>; Advocate 2: <its option>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours)>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | New tests written in this task: `tests/core/test_live.py` (AC 1–8 + mixed-streak), `tests/core/test_rules.py` (AC 10), `tests/surfaces/test_api_live.py` (AC 9 + resume re-snapshot), `tests/leak/test_live_leak.py` (AC 11). Supervisor rerun 2026-11-02 @ 66520b1: `81 passed, 330 deselected` / `27 passed, 160 deselected` / `32 passed` |
| Verification command run | ☑ pass | `uv run pytest tests/core -q -k "live or rules"` → `81 passed, 330 deselected, 1 warning in 5.10s`; `uv run pytest tests/surfaces -q -k live` → `27 passed, 160 deselected, 1 warning in 0.83s`; `uv run pytest tests/leak -q` → `32 passed, 1 warning in 45.46s` |
| Negative cases hold | ☑ pass | 9m59s open → no `needs_attention`; record in another stream prevents it; 100 ordinary calls → `state_version` unchanged; unknown / non-number threshold refused; planted ids + secret text absent from events, snapshot, SSE text, store, logs |
| verify | ☐ pass / ☐ fail / ☐ N/A | [Stage 5 — pending] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed `core/live.py`, `core/rules.py`, `surfaces/api.py` + their tests and direct callers (`live_stream`, `load_rules` users). Lane: `C2, 7 files, 646 lines, EXCEEDS (lines 646 > 500)` — ~400 lines are tests; kept C2. Stage 4 code-review: P1 stale attention after resume (fixed 66520b1), P2 mixed-tool streak named last tool (fixed 66520b1), P1 process: agent's startup reads partial (`agents/backend.md` head only, PRD refs not read) — Supervisor re-checked every AC against PRD, no code impact. Security pass (manual; built-in `security-review` failed: no `origin/HEAD`): block reads limited to `type`/`name`/`id`/`tool_use_id`/`is_error` via `_label`; `_append` fixed key list excludes `block_id`; logs carry kind/exception type only; `/api/live` guard unchanged; memory bounded — no findings |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` → `1070 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 143.09s`; `uv run ruff check src tests` → `All checks passed!` |
| **Companion score (easy-verifier)** | ☑ N/A | Not run: learnings 2026-11-01 — easy-verifier refused every repo path |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Backend only; the page is HUN-182 |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Backend only; the page is HUN-182 |
| **UI: Responsiveness at target viewports** | ☑ N/A | Backend only; the page is HUN-182 |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-02T03:08:37Z on branch `HUN-181` @ 5af6a25, before any implementation commit. Command: `uv run python <scratchpad>/demo181.py` (temp home; a fixture session `s1` with 3 `tool_use` + 3 failed `tool_result` records, read by `live.LiveTail.poll_once()`):

```
2026-11-02T03:08:37.763572+00:00 BEFORE
snapshot session keys: ['account', 'cache_creation', 'cache_read', 'cost_usd', 'last_ts', 'project', 'session_id', 'tokens_in', 'tokens_out', 'tool_calls']
has attention: False | snapshot keys: ['active_window_s', 'last_id', 'sessions']
tool_result events: [('tool_result', None, True), ('tool_result', None, True), ('tool_result', None, True)]
```

**AFTER**: same script, 2026-11-02T03:24:19Z (the script's `BEFORE` label is only its print text), after the change:

```
snapshot session keys: ['account', 'attention', 'cache_creation', 'cache_read', 'cost_usd', 'last_ts', 'project', 'session_id', 'tokens_in', 'tokens_out', 'tool_calls']
has attention: True | snapshot keys: ['active_window_s', 'last_id', 'open_call_minutes', 'sessions', 'state_version']
tool_result events: [('tool_result', 'Bash', True), ('tool_result', 'Bash', True), ('tool_result', 'Bash', True)]
```

Real run (AC 12, headless `claude -p --model claude-haiku-5-5`, dashboard with `live_open_call_minutes: 1`): `/api/live` showed `failing` (Bash, count 4) at 03:20:40 after `false` x3, and `needs_attention` (Bash, count 1) at 03:23:08 for a `python3 -c 'import time; time.sleep(90)'` call, cleared at its result (03:23:38).

**DELTA**: A live session now carries `attention` states (`failing`, `waiting`, `needs_attention`) in `snapshot()` and `GET /api/live`, and failed results name their tool.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
