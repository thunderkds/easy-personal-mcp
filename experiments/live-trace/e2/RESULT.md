# HUN-167 E2: native `http` hooks to a local collector: RESULT

Date: 2026-10-31. Claude Code 2.1.316, model `haiku`, account `CLAUDE_CONFIG_DIR=/home/hungnguyenhuu/.claude-hung2`.
Files: `collector.py` (127.0.0.1:4390, `POST /hook`), `settings.json` (async template), `run_e2.py` (driver, analysis), `summarize.py`.
Run records and reduced events (metadata only): `runs/`. Hooks were passed only with `--settings FILE` (file kept in `runs/`); no repo or `~/.claude*/` settings were touched.

## Tried

* Collector keeps only `hook_event_name`, `session_id`, `tool_use_id`, `tool_name`, `agent_id` and the local receive time `recv_ms`; replies 200 with an empty body. The NAMES of the body fields (never values) go to `<events>.fields.json`.
* Settings: one `http` hook (`url` `http://127.0.0.1:4390/hook`, `timeout` 10, `async: true`) on 9 events: SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, PostToolUseFailure, SubagentStart, SubagentStop, Stop, SessionEnd.
* 5 sessions of the standard prompt with the collector up (labels `e2-async-1..5`); 1 session async and 1 session sync (no `async`) with nothing listening on 4390; 1 edge run of 2 sessions at once against a collector that delays each reply by 2000 ms.
* Latency = collector receive time minus the transcript `timestamp` of the record holding the `tool_use` (same machine clock), matched by `tool_use_id`, main and `subagents/` transcripts.
* Sessions used: 9 (5 measured + 2 collector-down + 2 edge). Cap 9. Cut: the collector-down check ran 1 async + 1 sync, not the 2 + 2 the acceptance criterion asks for, because the cap for this task is 9.

## Worked

* Capture: 45 of 45 reference tool_use ids seen by PreToolUse (100%) and 45 of 45 by PostToolUse or PostToolUseFailure (100%), over 5 runs (9 ids per run: 7 main + 2 subagent, includes 5 `SubagentHandback` ids). No id seen by a hook was missing from the transcript (0 extra ids).
* The 8 standard steps completed in all 9 sessions (steps all true, exit 0).
* Events per session, identical in 5 of 5 runs (25 events): UserPromptSubmit 2, PreToolUse 9, PostToolUse 8, PostToolUseFailure 1 (the failing `ls /nonexistent`), SubagentStart 1, Stop 2, SubagentStop 1, SessionEnd 1.
* Subagent events carry `agent_id`: 6 of 6 subagent-side events per run (SubagentStart, SubagentStop, Pre and Post of its Read and of `SubagentHandback`); main-thread tool events carry no `agent_id`.
* Two sessions at once: 2 distinct `session_id` values, 50 events (25 + 25), each session captured 9 of 9 Pre and 9 of 9 Post by its own ids.
* Large payload: a hook body above 1 MB (`tool_response` about 1.2 MB) is accepted, dropped and answered in under 2 s (unit test `test_e2_large_tool_response_dropped_and_fast`).

## Failed

* SessionStart delivered 0 events in 5 of 5 runs, and no error was shown. The hooks doc says SessionStart and Setup "don't support `http`, `prompt`, or `agent` hooks" (only `command` and `mcp_tool`). A live source cannot use `http` for the session start; SessionEnd, UserPromptSubmit and the others do work.
* `async: true` is documented for `type: "command"` hooks only ("This field is only available on `type: "command"` hooks"). Claude Code accepted the field on `http` hooks without error, but the edge run suggests it is not honored: with a 2000 ms reply delay and `async: true`, two concurrent sessions took 64301 ms and 65513 ms, against a median 26276 ms with the same hooks and a fast collector (about +38 s, over 6 times the 5768 ms noise band). Each 2 s reply held the session, as a blocking hook would. Caveat: the two sessions ran at once, so some of the gap may be CPU contention; the gap is far larger than the contention seen elsewhere. Open question: which events block (we did not run a clean one-session async-vs-sync pair with a slow collector).

## Numbers

Standard session, no tracing baseline (HUN-165): median 28264 ms, spread 5768 ms (noise).

| Run | Session (8 chars) | Wall-clock ms | Pre captured | Post captured |
| -- | -- | -- | -- | -- |
| e2-async-1 | 96893973 | 28990 | 9/9 | 9/9 |
| e2-async-2 | aba56bcd | 25650 | 9/9 | 9/9 |
| e2-async-3 | 2c4eb308 | 25714 | 9/9 | 9/9 |
| e2-async-4 | 04a041e6 | 26278 | 9/9 | 9/9 |
| e2-async-5 | 74b83120 | 26276 | 9/9 | 9/9 |

| Measure (5 async runs, fast local collector, n = 45 ids) | Median ms | p95 ms | Max ms |
| -- | -- | -- | -- |
| PreToolUse latency (receive time minus transcript timestamp) | 5.9 | 10.1 | 12.7 |
| PostToolUse or PostToolUseFailure latency (includes the tool's run time) | 41.2 | 2295.2 | 2518.5 |

Post latency by tool (median over runs): Read 25.3 ms, Edit 22.0 ms, Bash success 2295.2 ms, Bash failure 680.0 ms, Agent 594.7 ms, SubagentHandback 627.7 ms. The long Post values are tool run time (the first Bash call took about 2.3 s), not hook delay; Pre latency is the cleaner delivery number. Post minus Pre per tool: median 24.9 to 36.1 ms across runs for most calls (max 2.3 to 2.5 s for Bash). The Agent PostToolUse arrived about 0.6 s after its transcript record, while SubagentStop came about 4 s later: the Agent tool seems to return when the subagent starts (open question).

Overhead (async template, collector up): median wall-clock 26276 ms vs baseline 28264 ms = -1988 ms (-7.0%), range 25650 to 28990 ms. That is inside the 5768 ms noise band: no measurable overhead.

Collector down (nothing listening on 4390; connection refused):

| Config | Wall-clock ms | vs baseline 28264 | Exit | Steps | Events captured |
| -- | -- | -- | -- | -- | -- |
| async template | 25060 | -3204 ms | 0 | 5 of 5 true | 0 |
| sync (no `async`) | 26820 | -1444 ms | 0 | 5 of 5 true | 0 |

Both finished normally, with no visible change in wall-clock (inside noise). Connection errors are non-blocking, as the docs say. One run per config, not two.

Slow collector edge run (2 sessions at once, reply delay 2000 ms, `async: true`): wall-clock 64301 ms and 65513 ms, 50 events, all 18 reference ids captured (9 + 9).

## Confirmed stdin (POST body) field names

Union over 5 runs (names only; values never stored). Common to all: `session_id`, `hook_event_name`, `transcript_path`, `cwd`, `prompt_id`.

* `PreToolUse`: `agent_id`, `agent_type`, `cwd`, `effort`, `hook_event_name`, `permission_mode`, `prompt_id`, `session_id`, `tool_input`, `tool_name`, `tool_use_id`, `transcript_path`
* `PostToolUse`: same plus `duration_ms`, `tool_response`
* `PostToolUseFailure`: `cwd`, `duration_ms`, `effort`, `error`, `hook_event_name`, `is_interrupt`, `permission_mode`, `prompt_id`, `session_id`, `tool_input`, `tool_name`, `tool_use_id`, `transcript_path` (no `agent_id` seen: the failing call was on the main thread)
* `SubagentStart`: `agent_id`, `agent_type`, `cwd`, `hook_event_name`, `prompt_id`, `session_id`, `transcript_path`
* `SubagentStop`: `agent_id`, `agent_transcript_path`, `agent_type`, `background_tasks`, `cwd`, `effort`, `hook_event_name`, `permission_mode`, `prompt_id`, `session_crons`, `session_id`, `stop_hook_active`, `transcript_path`
* `Stop`: `background_tasks`, `cwd`, `effort`, `hook_event_name`, `last_assistant_message`, `permission_mode`, `prompt_id`, `session_crons`, `session_id`, `stop_hook_active`, `transcript_path`
* `UserPromptSubmit`: `cwd`, `hook_event_name`, `permission_mode`, `prompt`, `prompt_id`, `session_id`, `transcript_path`
* `SessionEnd`: `cwd`, `hook_event_name`, `prompt_id`, `reason`, `session_id`, `transcript_path`
* `SessionStart`: no event arrived (see Failed).

`prompt`, `last_assistant_message`, `tool_input`, `tool_response` and `error` hold content: the collector drops them before writing. `prompt_id` matches the OTel `prompt.id` (docs), so it can join E2 with E3. `duration_ms` (tool run time) is sent by the hook but is not in the collector's allowed keys; add it if tool run time is wanted.

## Edge cases

* Large `tool_response` above 1 MB: dropped, 200 in under 2 s (unit test).
* Subagent events carry `agent_id`: 6 of 6 per run (see Worked).
* Two sessions at once: separated by `session_id`, 50 of 50 events attributed (see Worked).
* The planted fake API-key string and a fake prompt sentence (see the test file): 0 occurrences in `e2/` files (unit tests `test_e2_planted_secret_and_prompt_never_stored`, `test_e2_collector_delay_knob_and_no_content_in_source_folder`, which greps every `.json/.jsonl/.md/.txt` under `e2/`).
* Bad body (not JSON, not an object) gets 400, wrong path 404, nothing stored.
* Two Stop and two UserPromptSubmit events per session (one extra of each): not explained; open question (likely the subagent's own turn; not checked).

## Verdict

keep with limits. The built-in `http` hook is an accurate, low-latency live source for tool events: 100% capture (45 of 45 Pre, 45 of 45 Post), Pre latency median 5.9 ms / p95 10.1 ms, no measurable session overhead, and a down collector does not fail or slow the session. Limits: SessionStart cannot use `http`; `async: true` is documented for command hooks only and appears not honored for `http` (a slow collector stalls the session by about the reply delay per blocking hook), so the collector must answer at once and do its work after replying; Post latency is not a delivery number because it includes the tool's run time.
