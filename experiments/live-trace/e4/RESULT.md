# HUN-169 E4: stream-json capture, RESULT

Date: 2026-10-31. Claude Code 2.1.316, model `haiku`, account `CLAUDE_CONFIG_DIR=/home/hungnguyenhuu/.claude-hung2`.
Code: `e4/capture.py` (tests: `tests/test_e4.py`). Run records (metadata only): `e4/runs/e4-measured-*`, `e4-partial-*`, `e4-resume-*`.
Docs read: https://code.claude.com/docs/en/headless.md , https://code.claude.com/docs/en/agent-sdk/streaming-output.md , https://code.claude.com/docs/en/agent-sdk/typescript.md (`SDKResultMessage`), https://code.claude.com/docs/en/cli-reference.md .

## Tried

* `capture_session()` runs `claude -p <standard prompt> --model haiku --output-format stream-json --verbose --allowed-tools Read Bash Edit Agent Task` (plus `--include-partial-messages` or `--resume <id>`), under an exclusive `flock` for the whole run, in its own process group (kill reaches children). Each stdout line gets a local receive time (`time.time()` at `readline`) and is cut at once to: type/subtype, key names, tool_use ids and names, tool_result ids, usage numbers, stream event/delta type names, and for `result` lines every field (numbers, bools, a short enum allow-list; every other string becomes `"<str>"`). The harness `parse_stream` supplies the tool counts and step checks.
* `evaluate()` compares ids with `harness/reference.py` (main + `subagents/` transcripts). Latency per id = stream receive time minus the transcript `timestamp` of the record holding that `tool_use`.
* Sessions: 5 measured + 2 with `--include-partial-messages` + 1 `--resume` = 8 (cap 8). No trial session.
* Docs only for the interactive question (no TUI session was started).

## Worked

* Capture: 45 of 45 reference tool_use ids seen in the stream over 5 runs = 100 % (9 of 9 in every run). Subagent included: each run's 9 ids = 7 main (Read 3, Bash 2, Edit 1, Agent 1) + 2 subagent (Read 1, SubagentHandback 1); the 2 subagent ids arrived in the parent stream with `parent_tool_use_id` set to the `Agent` tool_use id (2 of 2 per run, 10 of 10 total). 0 ids in the stream that were not in the reference.
* Partial-messages runs: 18 of 18 ids = 100 %.
* All 5 measured runs did every standard step (read 3, bash 2 with 1 failing, edit 1, subagent 1, normal end); exit 0 in 8 of 8 sessions.
* 24 tests pass in `experiments/live-trace/tests` (12 are E4 tests): metadata-only record from a fixture holding the planted secret, partial last line, killed run (timeout kill, no `result`), two `result` lines, lock held for the run, capture % and latency maths.

## Failed

* Nothing failed in a live session. Not measured live (unit tests only): a killed run, and a stream-json run where the tool name arrives first in a `content_block_start` partial event (the stored partial records keep event and delta type names, not the block's tool id; see Open questions).
* The 3 `result` lines are not "wrong values" in HUN-165: see the explanation below.

## Numbers

Latency = stream receive time minus transcript timestamp, ms, same machine clock.

| Config | Runs | ids | Capture % | Latency median ms | Latency p95 ms | Max ms |
| -- | -- | -- | -- | -- | -- | -- |
| Measured (no partials) | 5 | 45 | 100.0 | 1.8 | 4.4 | 5.5 |
| Partial messages | 2 | 18 | 100.0 | 1.6 | 5.4 | 5.5 |

Per-run median / max latency (ms), measured: 2.0 / 3.1, 1.8 / 5.5, 1.9 / 4.4, 1.7 / 3.2, 1.7 / 4.8. By tool (measured, min-max ms): Read 20 ids 0.5-4.3, Bash 10 ids 1.6-5.5, Edit 5 ids 1.0-2.0, Agent 5 ids 1.0-2.3, SubagentHandback 5 ids 0.8-4.8. What the latency means: the assistant stream line carries its own `timestamp` field (same write as the transcript record), so 2 ms is the pipe delivery delay, not model or tool time. The stream line for a `tool_use` arrives when its content block is complete and before the tool runs.

Wall-clock of the process (lock wait excluded; lock waits were 23016 to 89766 ms because other experiments held the lock):

| Config | Wall-clock ms per run | Median | Min-max | vs baseline 28264 ms |
| -- | -- | -- | -- | -- |
| Measured | 26381, 29934, 26106, 30409, 27289 | 27289 | 26106-30409 (spread 4303) | -975 (inside noise 5768) |
| Partial messages | 29246, 24788 | 27017 | 24788-29246 | -1247 (inside noise) |

p95 of 5 runs with the harness formula equals the max (30409 ms). Capture overhead on wall-clock: not measurable (inside the baseline spread); the per-line work is one `json.loads` and a small dict.

Partial messages (`--include-partial-messages`):

| Run | stdout lines | `stream_event` lines | stdout bytes (metadata line sizes) |
| -- | -- | -- | -- |
| partial 1 | 160 | 115 | 88214 |
| partial 2 | 183 | 129 | 118555 |
| measured, same shape as partial 2 (3 `result` lines), runs 1-4 | 45-49 | 0 | 70207-71855 |
| measured, same shape as partial 1 (1 `result` line), run 5 | 42 | 0 | 44470 |

* Event count: 115 and 129 (mean 122). Extra lines vs the same-shape measured run: +118 and about +135. Extra bytes: +43744 (+98 %) and about +47000 (+65 %).
* Event types, run 1: content_block_delta 71, content_block_start 10, content_block_stop 10, message_start 8, message_delta 8, message_stop 8. Delta types: input_json_delta 62, thinking_delta 5, signature_delta 2, text_delta 2 (44 events without a delta).
* All `stream_event` lines belong to the main conversation (`parent_tool_use_id` null in 244 of 244 events): subagent output has no partial events.
* Wall-clock overhead: -1247 ms vs baseline (noise). The complete `assistant` lines still arrive; they are not replaced.

### `result` line: exact top-level field names (Claude Code 2.1.316)

The first `result` line of a turn, identical key set in 7 of 7 sessions that had one (5 measured, 2 partial; the key set is the same also when a session has 1 or 3 result lines):
`api_error_status`, `duration_api_ms`, `duration_ms`, `fast_mode_disabled_reason`, `fast_mode_state`, `first_content_frame_ms`, `is_error`, `modelUsage`, `model_usage_since_last_result`, `num_turns`, `permission_denials`, `queued_turn_count`, `result`, `result_index`, `safety_stops`, `session_id`, `stop_reason`, `subagent_stats`, `subtype`, `terminal_reason`, `time_to_request_ms`, `total_cost_usd`, `ttft_ms`, `ttft_stream_ms`, `type`, `usage`, `uuid`. The `peer` and `task-notification` result lines add `origin` and drop the timing fields (`first_content_frame_ms`, `time_to_request_ms`, `ttft_ms`, `ttft_stream_ms`; the `task-notification` line also lacks `api_error_status` and `terminal_reason`).

| Need | Field | Example (run 1) |
| -- | -- | -- |
| cost | `total_cost_usd` (cumulative for the session, same value on all result lines of a run) | 0.008158845 |
| cost per model | `modelUsage.<model>.costUSD` and token fields `inputTokens`, `outputTokens`, `cacheReadInputTokens`, `cacheCreationInputTokens`, `webSearchRequests`, `thinkingTokens`, `contextWindow`, `maxOutputTokens`, `cacheCreationEphemeral5mInputTokens`, `cacheCreationEphemeral1hInputTokens`; text fields `canonicalModel`, `provider`, `costBasis` | model key `claude-haiku-5-5`, `costUSD` 0.008158845 |
| usage (main loop of that turn only) | `usage.input_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`, `output_tokens`, `output_tokens_details.thinking_tokens`, `server_tool_use.{web_search_requests,web_fetch_requests}`, `service_tier`, `cache_creation.{ephemeral_1h_input_tokens,ephemeral_5m_input_tokens}`, `inference_geo`, `iterations`, `speed`, `fallback_credit` | input 16, output 1085 |
| duration | `duration_ms`, `duration_api_ms`, `ttft_ms`, `ttft_stream_ms`, `time_to_request_ms`, `first_content_frame_ms` | 19351, 23838, 2310, 1726, 166 |
| session | `session_id`, `uuid`, `result_index` | 0, 1, 2 |
| outcome | `subtype`, `is_error`, `terminal_reason`, `num_turns`, `permission_denials` (list), `subagent_stats` (spawned, completed, failed, by_type ...) | `success`, false, `completed`, 8 |

`usage` covers the main loop only: output_tokens summed over the 3 result lines of run 1 = 1085 + 138 + 0 = 1223, while `modelUsage.<model>.outputTokens` = 1450 (includes the subagent). For cost and tokens of the whole session read `modelUsage` and `total_cost_usd`, as the docs say.

### Answer to the HUN-165 open question (`num_turns` 0, `duration_ms` 5 to 12 ms)

A run can write more than one `result` line, and the harness `parse_stream` keeps only the last one. Counted in this task: 4 of 5 measured runs wrote 3 result lines; 1 of 5 wrote 1; partial runs 1 of 2 wrote 3 and 1 of 2 wrote 1. The 3 lines of run 1:

| `result_index` | `origin` | `num_turns` | `duration_ms` | meaning |
| -- | -- | -- | -- | -- |
| 0 | none (the user's prompt) | 8 | 19351 | the whole main task: 8 turns |
| 1 | `kind: peer`, `handback: true` | 1 | 2390 | the turn that answered the subagent's handback |
| 2 | `kind: task-notification` | 0 | 6 | empty follow-up result for the finished background task |

The `Agent` call ran as a background task (`subagent_stats.started_in_background` 1 although `requested.unset` 1), so Claude Code answered its completion in a second turn. Sizes of the third line over runs 1-4: `duration_ms` 6, 19, 8, 5 and `num_turns` 0 in 4 of 4: these are the 5-12 ms values HUN-165 saw. Runs 1-4 `result_index` 0 values: `num_turns` 8 and `duration_ms` 19351, 21825, 18869, 23831. In run 5 (1 result line) `num_turns` 8, `duration_ms` 24711. The docs say the same: all but the last of the completions answered together "produce empty results with `num_turns: 0`" (typescript.md, `SDKResultMessage`, `origin`). Rule for readers of the stream: use the result line with `result_index` 0, or the one without `origin`; never "the last". The 3 lines are written together at the end: result 0 and result 1 arrive 0-8 ms apart, about 0.7 s before process exit (0.66 to 0.72 s in runs 1-4), so the first `result` is not a live end-of-turn signal.

### Interactive session

Answer: no, an interactive session cannot be observed with stream-json.
* Docs: `--output-format` "only works with --print"; `--include-partial-messages` "requires `--print` and `--output-format stream-json`" (cli-reference.md, `claude --help` 2.1.316). The stream is the stdout of the one process that runs `-p`. An interactive TUI has no such stream.
* Test (1 session): `claude -p "<one-word reply>" --resume b4f448dc-... ` in the run-1 scratch folder, stream-json. Result: same `session_id` as the original (1 `init` line, equal); 6 lines; 0 tool_use ids replayed from the earlier 8 turns (history is not re-streamed); `result` `num_turns` 1, `duration_ms` 2107, `total_cost_usd` 0.008690 (run 1 was 0.008159: cost is the conversation total, as docs say); wall-clock 3768 ms. The transcript for that session still held 9 reference ids, since the new turn used no tool.
* So `--resume <id> -p` can observe only the turns it starts itself, from a new process. It cannot attach to a session the owner has open in a terminal, and driving a live session this way would write into the same transcript as the open terminal (not tested; risk, not a measurement).

## Edge cases

* Subagent messages appear in the parent stream: yes, as `assistant` / `user` lines with `parent_tool_use_id` = the `Agent` tool_use id; the subagent's `tool_use` ids (Read, SubagentHandback) are there; its text is omitted by default (docs: `--forward-subagent-text` adds it; not used).
* Killed run (unit test, timeout kill of the process group): last line partial -> stored as `type: unparsed` with no content from the cut line; `has_result` false; `normal_end` false. Docs: SIGTERM exits 143 and records no result for the turn in progress. No live kill was run.
* The planted fake API-key string and a fake prompt sentence (see the test file) in a fixture stream (text, tool input, tool_result, `result` text, unparsed line): 0 occurrences in the stored record (unit test `test_e4_planted_secret_never_stored`), and 0 in `e4/runs/` and this file (`test_e4_planted_secret_absent_from_stored_files`). The stored strings in the 8 run files are ids, timestamps, tool and event type names, field NAMES (e.g. `prompt`, `body` appear only in `keys` lists) and the enum allow-list.
* Lock: each of the 8 sessions held the exclusive lock for its whole run (unit test checks it blocks 600 ms and is released after).
* `system/hook_started` and `hook_response` lines appear before `init` (from the account's own hook config; contents not stored). Three `init` lines appear in runs with a follow-up turn.

## Open questions

* Early tool name from partial events: `content_block_start` for a `tool_use` block should carry the id and name before the complete `assistant` line, but the stored partial records do not keep the block's id and name, so the gain in ms is not measured (needs a new session beyond this task's cap of 8).
* Does the second `result` line's `peer` origin appear for non-background subagents? Run 5 (1 result line) shows the number of result lines is not fixed for the same prompt; 5 of 8 runs had 3 lines, 3 of 8 had 1 line (resume run: 1). The cause is not measured.
* Stream-json schema is "internal" per the Phase 0 table for transcripts and documented for the SDK; fields such as `wire_tool_inputs` and `model_usage_since_last_result` are not in the docs. Re-check the field list on each Claude Code upgrade.

## Verdict

keep with limits. As a live source for sessions we launch ourselves: 100 % of tool_use ids (45 of 45, subagent included), latency median 1.8 ms and p95 4.4 ms behind the transcript write, no measurable wall-clock cost. Limits: it works only for `claude -p` processes we start (not interactive sessions); a run can have 3 `result` lines, so take `result_index` 0 / no `origin`, not the last; `--include-partial-messages` adds about 122 lines and +65 to +98 % stdout bytes per run for no id the complete lines do not already carry (early-name gain unmeasured); the first `result` line arrives only at the end of the run.

Capture interface for HUN-170: `from capture import capture_session, evaluate, summarize_latency` (add `experiments/live-trace/e4` and `harness` to `sys.path`); `cap = capture_session(label=..., scratch=None, extra_env=None, partial_messages=False, resume=None, runs_dir=..., lock_path=DEFAULT_LOCK)`; `evaluate(cap, config_dir)` returns `capture_pct`, `latency_ms` per id, `latency_summary`, `missing_by_name`.
