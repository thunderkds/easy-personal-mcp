# HUN-165 harness: RESULT

Date: 2026-10-31. Claude Code 2.1.316, model `haiku`, account `CLAUDE_CONFIG_DIR=/home/hungnguyenhuu/.claude-hung2`.
Run records (metadata only): `harness/runs/baseline-*.json`.

## Tried

* `make_scratch.py`: scratch git repo from `mktemp -d` style folder under `/tmp` (outside the repo), 3 text files + `check.sh`.
* `run_session.py`: `claude -p <standard prompt> --model haiku --output-format stream-json --verbose --allowed-tools Read Bash Edit Agent Task`, stdin closed, stream parsed line by line for ids, names and types only.
* `reference.py`: reads `{tool_use_id: (tool_name, timestamp)}` from `<config>/projects/*/<session>.jsonl` and `<session>/subagents/*.jsonl`.
* Trial sessions to get the prompt right: 1 (worked first time; prompt not changed). Baseline sessions: 5. Total sessions used: 6 (budget 7).
* The standard session: read 3 files, run `sh check.sh`, run `ls /nonexistent` (fails), edit 1 line, start 1 subagent that reads 1 file, end.

## Worked

* 5 of 5 baseline runs did every standard step: read 3, bash 2 with 1 failing, edit 1, subagent 1, normal end (exit 0, result `success`).
* Session id was taken from `system/init` in 5 of 5 runs; 0 unparsed stream lines.
* Subagent transcript sat under `<session>/subagents/` and `reference.py` returned its tool_use ids (checked on trial session `bfa61da1`: 9 ids = 7 main + 2 subagent).
* Tool counts, identical in all 5 runs: main {Read 3, Bash 2, Edit 1, Agent 1}; subagent {Read 1, SubagentHandback 1}.
* 12 unit tests pass on synthetic stream-json and transcript fixtures (`uv run pytest experiments/live-trace/tests -q -k harness`).

## Failed

* Nothing failed in the baseline. The `result` event fields `num_turns` (0 in 5 of 5) and `duration_ms` (5 to 12 ms) do not look like whole-session values; they are kept in the run record as `num_turns` / `cc_duration_ms` but NOT used for any number below. Open question: what these two fields count in stream-json (plan doc says result field names are unconfirmed).

## Numbers

Wall-clock of the whole `claude -p` process (start of spawn to exit), no tracing configured:

| Run | Session id (first 8) | Wall-clock ms | Exit | Read 3 | Bash 2, 1 failing | Edit 1 | Subagent 1 | Normal end |
| -- | -- | -- | -- | -- | -- | -- | -- | -- |
| 1 | 705c4650 | 31291 | 0 | yes | yes | yes | yes | yes |
| 2 | 1372de80 | 32094 | 0 | yes | yes | yes | yes | yes |
| 3 | cb39c1a6 | 26326 | 0 | yes | yes | yes | yes | yes |
| 4 | d257c722 | 27755 | 0 | yes | yes | yes | yes | yes |
| 5 | d0c953b9 | 28264 | 0 | yes | yes | yes | yes | yes |

Median 28264 ms; min 26326 ms; max 32094 ms (spread 5768 ms, 20% of median). p95 with n=5 equals the max (32094 ms). API-equivalent cost per run (from `total_cost_usd`): 0.0081 to 0.0083 USD.

Overhead rule for E1-E4: a traced configuration costs overhead = its median wall-clock minus 28264 ms; a difference under about 5800 ms (the baseline spread) is inside the noise.

## Edge cases

* Haiku skipped no step in 5 runs; `steps_done()` records a skipped step as `false` (unit test `test_harness_skipped_step_is_recorded_not_faked`).
* The subagent tool is named `Agent` in this version (`Task` in older ones); both are allowed and counted.
* The subagent also calls a tool named `SubagentHandback`; it appears in the subagent bucket and in the transcript reference. E1-E4 must not treat it as a test step.
* Content-logging env vars (`OTEL_LOG_USER_PROMPTS`, `OTEL_LOG_TOOL_DETAILS`, `OTEL_LOG_TOOL_CONTENT`, `OTEL_LOG_RAW_API_BODIES`) are refused by `run_session()` (ValueError) and removed from the inherited env.
* The planted fake API-key string and a fake prompt sentence (see the test file): 0 occurrences in `harness/runs/`, this file and `NOTES.md` (unit test `test_harness_planted_secret_absent_from_stored_files`, plus the fake-claude run test).
* Each session leaves a transcript folder under the account's `projects/-tmp-live-trace-*` (written by Claude Code, read-only for the spike).

## Verdict

keep. The harness gives E1-E4 one fair yardstick: median 28264 ms for the standard session without tracing, 5 of 5 steps complete.
