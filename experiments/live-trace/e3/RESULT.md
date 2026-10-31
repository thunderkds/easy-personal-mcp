# E3: Claude Code OpenTelemetry to a local OTLP receiver (HUN-168)

Measured 2026-10-31 on Claude Code with `--model haiku`, account `~/.claude-hung2`, the HUN-165 standard
session (3 Read, 2 Bash with 1 failing, 1 Edit, 1 subagent). Metadata only: the receiver keeps attribute
NAMES for every record and attribute VALUES only for an allow-list (ids, names, token counts, cost, model,
timings). Content-logging variables (`OTEL_LOG_*`) were never set. Env names re-checked against
https://code.claude.com/docs/en/monitoring-usage.md before the runs (all used names are on that page).
Numbers come from `e3/runs/*.json`; `python3 e3/aggregate.py` reprints them.

## Tried

* `e3/receiver.py`: stdlib `ThreadingHTTPServer` on 127.0.0.1:4318, `POST /v1/logs`, `/v1/metrics`, `/v1/traces`, protocol `http/json` (gzip accepted). No SDK, no collector binary.
* `e3/run_e3.py`: one traced session per call, telemetry env only in the child env: `CLAUDE_CODE_ENABLE_TELEMETRY=1`, `OTEL_LOGS_EXPORTER=otlp`, `OTEL_METRICS_EXPORTER=otlp`, `OTEL_EXPORTER_OTLP_PROTOCOL=http/json`, `OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4318`. The shared session lock was held for each whole run.
* Configurations (sessions): default intervals (2), `OTEL_LOGS_EXPORT_INTERVAL=1000` + `OTEL_METRIC_EXPORT_INTERVAL=5000` (5), receiver down with lowered intervals (1), traces beta `CLAUDE_CODE_ENHANCED_TELEMETRY_BETA=1` + `OTEL_TRACES_EXPORTER=otlp` + `OTEL_TRACES_EXPORT_INTERVAL=1000` (1). Total 9 sessions of the 10 allowed (the spare was not needed). The guide asked for 3 default runs; the cap of 2 default sessions applied, so n=2.
* `e3/analyze.py`: latency, token and cost comparison, attribute inventory.

## Worked

* All 8 sessions with a receiver delivered logs and metrics; 72 of 72 `tool_result` events were matched to a transcript `tool_use` by `tool_use_id` (0 unmatched, 0 duplicate ids).
* Token accuracy: OTel token sums equal the transcript `message.usage` sums in 8 of 8 sessions (diff 0.000% for input, output, cacheRead and cacheCreation, both from `api_request` events and from the `claude_code.token.usage` metric). Transcript sums were de-duplicated by message id; API request count equals transcript message count in every run (11/11, 12/12, 10/10).
* Cost accuracy: sum of `api_request` `cost_usd` equals the `claude_code.cost.usage` metric sum in 8 of 8 sessions (diff 0.000%), and equals the `total_cost_usd` of the stream-json result (for example 0.00821692 USD in all three, run 267d2823). Metric temporality is delta (value 1): summing every data point of every export reproduces the totals, so deltas are summed per export.
* Subagent events carry the same `session.id` as the main session: 15 of 15 subagent-tagged events (those with `agent_id`) in each of 8 runs had the main session id. They are told apart by `agent_id` (also on `tool_result`, `tool_decision`, `api_request`) and by `subagent_completed`.
* Receiver down: the session completed (exit 0, 5 of 5 steps, cost 0.00816518 USD).
* Traces beta works over `http/json`: 41 spans in 10 `/v1/traces` posts in 1 session.

## Failed

* Default metrics are not live: with the default 60 s metric interval and a 25-27 s session, the metrics arrive in one `/v1/metrics` post at process exit (26385 ms and 25073 ms after start, 0.6 s and 0.5 s before exit); no metric arrived during the session.
* `tool_result` latency against the transcript reference includes the tool's own run time and the export wait, so it cannot get below the export interval (min 671 ms at lowered; median about 1007 ms).
* The harness's `num_turns`/`cc_duration_ms` open question (HUN-165) was not touched.

## Numbers

Latency = receive time at the receiver minus the transcript timestamp of the `tool_use` (same machine clock), ms. "Export lag" = receive time minus the event's own `event.timestamp`.

| Configuration | Sessions | `tool_result` n | Median ms | p95 ms | Max ms | Export lag median / p95 ms |
| -- | -- | -- | -- | -- | -- | -- |
| Default (logs 5000 ms, metrics 60000 ms) | 2 | 18 | 3388 | 7263 | 8132 | 2706 / 5004 |
| Lowered (logs 1000 ms, metrics 5000 ms) | 5 | 45 | 1007 | 3253 | 6220 | 983 / 1004 |
| Traces beta (lowered + spans 1000 ms) | 1 | 9 | 1006 | 2102 | 2102 | 969 / 995 |

Per tool at lowered (median / p95 / n): Read 1006 / 1031 / 20; Edit 1006 / 1025 / 5; SubagentHandback 1005 / 1006 / 5; Bash 1996 / 3705 / 10 (the slowest Bash call had `duration_ms` 1551, which is in its latency); Agent 2114 / 6220 / 5 (the Agent `tool_result` waits for the subagent). After removing the event's own `duration_ms`, lowered latency is median 999 ms, p95 2105 ms (n=54, lowered plus traces runs): about one export interval.

Post cadence in a lowered run (run 267d2823, ms after start): first logs post 1395, then logs every 1.5-2 s; metrics posts at 5376, 10378, 15377, 20377, 25377 and a final logs + metrics flush at 27351/27352, 0.5 s before exit (wall 27864 ms). Default run: logs posts at 5537, 11043, 17443, 23429, then the final flush at 26382 with the single metrics post. 0 posts arrived after process exit in all 8 runs; the shutdown flush works.

Logs events are real-time at lowered interval (about 1 s behind) and about 2.7 s behind (median, up to 5 s) at the default. Metrics are about 5 s behind at lowered and exit-only at default for sessions shorter than 60 s.

Overhead against the HUN-165 baseline (median 28264 ms, noise about 5768 ms):

| Configuration | Wall-clock ms per session | Median | Difference | Inside noise |
| -- | -- | -- | -- | -- |
| Default | 26978, 25587 | 26283 | -1981 | yes |
| Lowered | 27865, 25249, 28619, 28534, 27716 | 27864 | -400 | yes |
| Receiver down | 26858 | 26858 | -1406 | yes |
| Traces beta | 24633 | 24633 | -3631 | yes |

Receiver down: 1 run, exit 0, all 5 steps done, wall-clock 26858 ms (baseline median 28264 ms, noise 5768 ms): no measurable slowdown, no hang. With a single run this shows no failure; it does not measure a receiver that accepts and stalls (open question).

Record counts per session (lowered, run 267d2823): 115 events and 81 metric data points over 15 logs posts and 6 metrics posts. Event counts in that run: `hook_execution_start` 23, `hook_execution_complete` 23, `hook_registered` 17, `mcp_server_connection` 12, `tool_decision` 9, `tool_result` 9, `api_request` 11, `user_prompt` 3, `plugin_loaded` 4, `assistant_response` 2, `subagent_completed` 1, `managed_settings_resolved` 1. Hook and plugin events come from the account's own hooks and plugins, not from the test.

### Attribute table (names seen, pooled over 8 sessions; `user.*`, `organization.id` and `terminal.type` are on every record and listed once below)

Always on every record of every event and metric: `user.account_id`, `user.account_uuid`, `user.email`, `user.id`, `organization.id`, `terminal.type`, `session.id`. Resource attributes: `host.arch`, `os.type`, `os.version`, `service.name`, `service.version`. Events also carry `event.name`, `event.sequence`, `event.timestamp`.

| Event name (count in 8 sessions) | Attribute names beyond the common ones |
| -- | -- |
| `tool_result` (72) | `tool_use_id` (72/72), `tool_name`, `prompt.id`, `request_id`, `success`, `duration_ms`, `tool_input_size_bytes`, `tool_result_size_bytes` (64/72), `error_type` (8/72), `agent_id` (16/72) |
| `tool_decision` (72) | `tool_use_id` (72/72), `tool_name`, `decision`, `source`, `permission_mode`, `tool_source`, `prompt.id`, `request_id`, `classifier_verdict` (32/72), `agent_id` (16/72) |
| `api_request` (88) | `model`, `cost_usd`, `cost_usd_micros`, `input_tokens`, `output_tokens`, `cache_read_tokens`, `cache_creation_tokens`, `cache_creation_5m_tokens`, `cache_creation_1h_tokens`, `duration_ms`, `ttft_ms`, `request_id`, `client_request_id`, `query_source`, `speed`, `effort`, `prompt.id`, `agent_id` and `agent.name` (16/88) |
| `user_prompt` (22) | `prompt.id`, `message.uuid`, `prompt_length`, and the content-bearing names `prompt`, `prompt_text` (values never stored) |
| `assistant_response` (15) | `message.uuid`, `request_id`, `model`, `query_source`, `prompt.id`, `response_length`, and the content-bearing name `response` (value never stored) |
| `subagent_completed` (8) | `agent_id`, `agent_type`, `agent.source`, `is_async`, `is_built_in`, `model`, `final_model`, `model_swapped`, `duration_ms`, `total_tokens`, `total_tool_uses`, `prompt.id` |
| `hook_execution_start` / `_complete` (183 each) | `hook_event`, `hook_name`, `hook_source`, `num_hooks`, `managed_only`, `safe_mode`; complete adds `num_success`, `num_blocking`, `num_cancelled`, `num_non_blocking_error`, `total_duration_ms`, `stdout_chars`, and more; `prompt.id` (175/183), `agent_id` (32/183) |
| `hook_registered` (136), `plugin_loaded` (32), `mcp_server_connection` (96), `managed_settings_resolved` (8) | session-start inventory events: hook/plugin/MCP names, status, `duration_ms`, counts |

Metrics (all with the common attributes): `claude_code.token.usage` (`type` input/output/cacheRead/cacheCreation, `model`, `query_source`, `effort`, `agent.name` sometimes), `claude_code.cost.usage` (`model`, `query_source`, `effort`, `agent.name` sometimes), `claude_code.session.count` (`start_type`), `claude_code.active_time.total` (`type`), `claude_code.lines_of_code.count` (`type`, `model`), `claude_code.code_edit_tool.decision` (`decision`, `language`, `source`, `tool_name`). Metrics carry `session.id` but never `prompt.id`/`tool_use_id`.

Dedupe key: `tool_use_id` is present on 72 of 72 `tool_result` and 72 of 72 `tool_decision` events, and it is the same id as the transcript `tool_use` (72 of 72 matched), so it joins OTel events to transcripts and hooks. `request_id` joins `api_request` to `tool_result`.

Traces beta (1 session, 41 spans): span names `claude_code.interaction` (3), `claude_code.llm_request` (11), `claude_code.tool` (9), `claude_code.tool.blocked_on_user` (9), `claude_code.tool.execution` (9). `claude_code.tool` and `.execution` carry `tool_use_id` and `gen_ai.tool.call.id`; `claude_code.tool` has `bash_argv0`/`bash_command_class` on 2 of 9; `claude_code.interaction` carries the content-bearing name `user_prompt` (value never stored).

## Edge cases

* Delta temporality: yes, `aggregationTemporality` 1 on all metrics; summing per export gives 0.000% diff to the transcript.
* Subagent `session.id`: same as the main session (15 of 15 per run, 8 of 8 runs).
* Prompt-like text in an attribute value: never stored. Unit tests `test_e3_allow_list_drops_prompt_like_text_secret_email_and_body`, `test_e3_allow_listed_key_with_sentence_value_is_dropped` plant a fake API-key string, a fake prompt sentence and a fake email in non-allow-listed attributes, in an allow-listed attribute and in a record body; the stored rows hold none of them. A record whose body is prompt text gets `event_name` None.
* The test `test_e3_planted_secret_absent_from_stored_files` greps every file under `e3/` (including `runs/` and `runs/sessions/`): 0 occurrences of the planted secret, the fake sentence and the fake email. A separate grep over `e3/` for the key prefix, the sentence's distinctive word and a personal mail domain found 0 files.
* The account's real email and ids (`user.email`, `user.id`, `user.account_uuid`, `organization.id`) are on every record; their values are NOT on the allow-list and are not stored. Any later source built on OTel must keep them off the allow-list.
* Port 4318 was free before every run; the receiver binds 127.0.0.1 only (a non-loopback host raises ValueError, unit test).
* Late posts: 0 after process exit (8 of 8 runs).
* Open questions: (1) whether the values of `prompt`, `prompt_text`, `response` and the span `user_prompt` are `<REDACTED>` on this account; the values were deliberately not read, and no `OTEL_LOG_*` variable was set by the spike, but the account's own settings could set one. (2) behaviour with a receiver that accepts and stalls (only a closed port was tested). (3) latency with protobuf or gRPC (cut). (4) the default-interval n is 2, not 3.

## Verdict

keep with limits.

* Keep for: token and cost totals (0.000% diff in 8 of 8 sessions) and per-tool, per-session event attribution with `tool_use_id` and `session.id`, including subagents.
* Limits: it is live only when the owner lowers `OTEL_LOGS_EXPORT_INTERVAL` (about 1.0 s median, 3.3 s p95 against the transcript `tool_use`, in which about 1 s is the export wait; at the default 5 s interval 3.4 s median, 7.3 s p95); metrics are exit-only at the default 60 s; it needs the owner's environment to carry the OTel variables for every `claude` start (a setup step, nothing a transcript reader needs); the stream carries the account email and ids on every record, so a product source needs the allow-list; hook, plugin, MCP and settings inventory events are 70% of the records (638 of 915) and can be dropped at the receiver.
* Overhead and failure: no measurable slowdown at any configuration (all differences inside the 5768 ms baseline noise); receiver down did not break or slow the session in 1 run.

## Interface for HUN-170 (combined run)

```python
sys.path[:0] = ["experiments/live-trace/e3", "experiments/live-trace/harness"]
from receiver import OtlpReceiver
from run_e3 import BASE_ENV, CONFIGS, run_once   # run_once(config, grace_quiet, grace_max, lock_path, config_dir, runs_dir, port)
from analyze import analyze_run                  # analyze_run(snapshot, session_id, config_dir, start_epoch_ms)

with OtlpReceiver(port=4318) as rx:                 # 127.0.0.1 only; port=0 for a free port
    rec = run_session(extra_env={**BASE_ENV, **CONFIGS["lowered"]}, ...)   # harness; holds no lock itself
    rx.wait_quiet(quiet_s=3, max_s=10)
    snap = rx.snapshot()   # {"events","metrics","spans","posts"}: sanitized dicts with recv_ms
```

`run_once("lowered")` does all of this under the shared session lock and writes `e3/runs/lowered-<stamp>-<session8>.json`.
