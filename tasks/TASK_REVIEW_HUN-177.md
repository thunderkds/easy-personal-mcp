# TASK_REVIEW — HUN-177: GET /api/live: stream live events to the page as Server-Sent Events (FR-044)

> Sibling of `tasks/TASK_GUIDE_HUN-177.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/surfaces/test_api_live.py` (25 tests: AC1 refusals x6 + query-token scope; AC2 snapshot then live events, pass-through values, no-tail; AC3 resume, rolled buffer, stale/bad id; AC5 token absent from caplog + `access_log=False`; disconnect, shutdown flag, two tabs). `tests/leak/harness.py` (`API_CALLS['/api/live']`, `_collect_live`) + `tests/leak/test_live_leak.py::test_live_stream_really_ran_over_planted_transcripts` (AC4). Output: `25 passed, 160 deselected, 1 warning in 0.66s` |
| Verification command run | ☑ pass | `uv run pytest tests/surfaces -q -k live && uv run pytest tests/leak -q` -> `25 passed, 160 deselected, 1 warning in 0.66s` then `31 passed, 1 warning in 40.89s` |
| Negative cases hold | ☑ pass | no token / wrong query token / wrong header token / wrong Host / wrong Origin -> 403 `{"error":"forbidden"}`; `?token=` on `/api/first-run` -> 403 (query token is read on `/api/live` only). Real run: curl gave 403 x5 (see AFTER). |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` -> `1046 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 125.51s (0:02:05)`; `uv run ruff check src tests` -> `All checks passed!` |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-01T02:18:39Z on branch HUN-177 at 23b0b3a (before any implementation commit):

```
$ uv run python -  # TestClient on api.create_app("tok", 5555), GET /api/live?token=tok with the token header
GET /api/live -> 404 Not Found
$ uv run pytest tests/surfaces -q -k live
160 deselected, 1 warning in 0.56s     (no live API test exists; exit code 5)
```

**AFTER**: real run 2026-11-01 ~02:30Z: installed wheel in a temp venv, `easy-personal dashboard` on fixture transcripts copied to a temp `EASY_PERSONAL_CLAUDE_PROJECTS`, temp `EASY_PERSONAL_HOME`, temp `EASY_PERSONAL_AGENT_ROOT`, port 42639:

```
no token: 403 | wrong token (query): 403 | token ok, Host: evil.example: 403 | token ok, Origin: http://evil.example: 403
/api/first-run?token=<token>: 403
$ curl -sN -D hdr.txt "$B/api/live?token=$TOKEN"   (then one fixture line appended to projects/2.1.316/session.jsonl)
HTTP/1.1 200 OK
cache-control: no-store
content-type: text/event-stream; charset=utf-8
content-security-policy: default-src 'self'; frame-ancestors 'none'

event: snapshot
data: {"active_window_s":300,"last_id":0,"sessions":[]}

event: live
id: 1
data: {"id":1,"session_id":"00000000-0000-4000-8000-0000000000aa","account":"Claude default","project":"demo-app","ts":"2026-11-01T02:32:11.000000Z","kind":"tool_use","tool":"Bash","is_error":false,"is_sidechain":false,"tokens_in":null,"tokens_out":null,"cost_usd":null}

event: live
id: 2
data: {"id":2,... "kind":"usage","tool":null,... "tokens_in":11,"tokens_out":22,"cost_usd":null}

: ping            (every 15 s; 2 seen after ~30 s)

$ curl -sN -H "Last-Event-ID: 1" ...   -> only `event: live` id 2, no snapshot
$ curl -sN -H "Last-Event-ID: 2" ...   -> nothing (until a new event)
$ curl -sN -H "Last-Event-ID: 999" ... -> fresh snapshot, last_id 2, sessions: [{... "tool_calls":1,"tokens_in":11,"tokens_out":22,"cost_usd":null}]
$ kill -INT <dashboard pid>  (two curl -N streams open)  -> dashboard stopped after 0.21 s; both curls closed
token hits in dash.log outside the CLI's own "dashboard: URL#token" line: 0
INVENTED_ in dash.log: 0; in the stream output: 0
```

(`cost_usd` is null because `claude-fixture-model-1` has no price. The store holds `INVENTED_LIVE_INPUT` in `tool_signals`: that is the existing one-word Bash program-name signal of `claude_code._signals`, from my one-word fixture command, not new in this task.)

**DELTA**: A dashboard page can now open `/api/live?token=...` with `EventSource` and get a snapshot plus every new tool call and usage record as it is written, resume after a drop with `Last-Event-ID`, and Ctrl-C still stops the dashboard at once.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
