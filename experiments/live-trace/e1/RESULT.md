# HUN-166 E1: live tail of transcripts: RESULT

Date: 2026-10-31. Claude Code 2.1.316, model `haiku`, account `CLAUDE_CONFIG_DIR=/home/hungnguyenhuu/.claude-hung2`.
Code: `e1/tail.py` (watcher), `e1/run_e1.py` (driver). Run data (metadata only): `e1/runs/*.json`, `e1/runs/*.events.jsonl`.
Sessions used: 9 of the cap 9 (5 measured + 4 edge: 1 spoiled kill run, 1 kill run, 2 in the two-at-once run).

## Tried

* `tail.py` polls `projects/*/*.jsonl` and `projects/*/<session>/subagents/*.jsonl` with `os.scandir` + inode/size checks every 100 ms (flag `--poll-ms`), reads new complete lines with the repo's `read_complete_lines` (a half-written last line is never yielded) and maps them with `parse_record` (read-only import; `src/` unchanged). It emits one JSON line per tool_use block, per tool_result block and per assistant record with `usage`. Resume by byte offset: `--state FILE`.
* The standard session of HUN-165, 5 times, tail running in the same process as a thread (run data: `runs/measured-*.json`). Latency = tail receive time (`time.time()` right after the line is read) minus the transcript `timestamp` of the record that holds the `tool_use` (HUN-165 `read_reference`).
* Diagnostic added from run 2: `mtime_ms` = file mtime read right after the line. `recv - mtime` bounds the tail's own delay; `mtime - transcript timestamp` bounds Claude Code's write delay (both approximate: mtime can be later than the line landed).
* Four edge cases (below). Every session held the flock on the shared lock file.

## Worked

* Capture: 45 of 45 tool_use ids seen (9 of 9 in each of 5 runs); 0 duplicate tool_use events; 0 bad JSON lines. Subagent files were found while tailing: runs 2 to 5 counted 2, 1, 1, 1 new files (run 2's second file is not explained, open question; run 1's counter was taken before the counter was fixed, but its events include subagent-file lines), and the subagent's `Read` and `SubagentHandback` calls are among the 45.
* Latency over the 45 tool_use records: median 196.5 ms, p95 1352.8 ms, min 115.2 ms, max 1527.3 ms.
* Overhead on the session: whole-session wall-clock of the 5 runs 27391, 27465, 28104, 26151, 25007 ms; median 27391 ms vs the HUN-165 baseline 28264 ms = -873 ms, inside the baseline spread (5768 ms), so no measurable overhead. Tail CPU 0.67 to 0.89 s per ~27 s run (about 3%); one scan of the whole account tree (about 410 files) mean 2.2 to 3.1 ms, max 14.1 ms.
* Metadata only: 0 occurrences of the planted `sk-ant-FAKE-SECRET-123` or the fake prompt sentence in the events, stats, runs and this file (unit test `test_e1_planted_secret_never_in_output_or_stats`; grep of `experiments/` finds them only in the two test files).

## Failed

* The "session killed mid-tool" case failed on the first try (spoiled run, `runs/edge-killed-mid-tool-spoiled.*`): `sleep 120` was refused by Claude Code with an error result at once, then the model ran a second command; nothing was killed mid-tool. Repeated with `python3 -c "import time; time.sleep(100)"` (see Edge cases).
* A tool output over 1 MB could not be produced in a real session. Asked for 1.5 MB on stdout (with `BASH_MAX_OUTPUT_LENGTH=2000000`), the longest transcript line was 153727 bytes. Claude Code cut the output before it reached the file. The 1 MB case is covered only by a synthetic line (unit test `test_e1_line_over_1mb_is_read_whole`, 1.1 MB line read whole, content absent from events). Open question: which tool result writes a line over 1 MB (an image read is untested; no sessions left).
* `cost_usd` cannot be filled: transcripts hold no cost number (`cost-state.totalCostUSD` is deliberately never read by the repo). Field is always `null`.

## Numbers

Per run (n/N = tool_use ids seen / ids in the reference; latency in ms):

| Run | Session (8) | Wall ms | n/N | Median | p95 (= max, n=9) | Tail lag median / max | Write lag median / max | Records read / abstained |
| -- | -- | -- | -- | -- | -- | -- | -- | -- |
| 1 | 215a3b4b | 27391 | 9/9 | 196.5 | 1223.6 | not measured | not measured | 197 / 67 |
| 2 | 6ed42a39 | 27465 | 9/9 | 186.1 | 1374.7 | 44.8 / 93.1 | 122.7 / 1350.8 | 224 / 64 |
| 3 | f3f4981d | 28104 | 9/9 | 194.1 | 1352.8 | 21.1 / 96.2 | 120.3 / 1338.4 | 141 / 40 |
| 4 | 02cb44fd | 26151 | 9/9 | 224.2 | 1527.3 | 61.6 / 99.9 | 127.2 / 1450.8 | 133 / 36 |
| 5 | b597d2f3 | 25007 | 9/9 | 212.9 | 1248.3 | 41.9 / 96.9 | 177.6 / 1226.2 | 151 / 45 |

(The run files `runs/measured-20261031T225132-1`, `...225248-1`, `...225423-2`, `...225618-3`, `...225806-4` are runs 1 to 5 in this order.) Records read and abstained count everything the tail read in the whole account tree during the run, so they include other sessions that wrote to the account at the same time.

Latency by tool (pooled, 45 records):

| Tool | n | Median ms | Min ms | Max ms |
| -- | -- | -- | -- | -- |
| Read | 20 | 159 | 115 | 224 |
| Edit | 5 | 186 | 159 | 213 |
| Agent (subagent start) | 5 | 783 | 703 | 1069 |
| SubagentHandback | 5 | 806 | 621 | 1005 |
| Bash | 10 | 1154 | 686 | 1527 |

Reading of the diagnostic (runs 2 to 5): tail lag medians 21 to 62 ms and max 99.9 ms, never above the 100 ms poll interval; so almost all of the 196 ms median and the whole >600 ms tail for Bash, Agent and SubagentHandback happens before the line reaches the file. Why those three tools land late is an open question (a guess, untested: the record is written when the message with the long tool input ends, not when it starts).

Abstained records (types `parse_record` does not know in version 2.1.316): `api-request` , `api-request-blob`, `api-request-shape`, `tools-used`, `ai-title`, `mode`, `permission-mode`, `bridge-session`: 36 to 67 per run, counted in `stats["abstained_by_type"]`, no event emitted.

### Fields table (E1 can fill, measured over 167 events of the 5 runs: 45 tool_use, 45 tool_result, 77 usage)

| Normalized field | Filled? | Fill rate | Source |
| -- | -- | -- | -- |
| session_id | yes | 167/167 | `sessionId` |
| project | yes | 167/167 | `cwd` (path, via `parse_record`) |
| kind | yes | 167/167 | `tool_use` / `tool_result` / `usage` |
| tool | yes, on tool_use | 45/45 tool_use | `tool_use.name` |
| tokens_in / tokens_out | yes, on usage | 77/77 usage | `message.usage` (+ cache creation / read) |
| cost_usd | no | 0/77 | not in transcripts; could be derived from tokens x price table (not done) |
| tool_use_id, record time, model, is_error, sidechain, file kind | yes | all | extras |

A usage event is emitted per assistant record (77 usage events for 45 tool_use); a consumer must dedupe on `message_id`, as the repo does.

## Edge cases

| Case | Sessions | Result |
| -- | -- | -- |
| Two sessions at once | 2 (one lock around both) | Both seen, events separated by session id: resumed session 1/1 new tool_use (3 usage, 2 tool_result), big-output session 1/1 (2 usage, 1 tool_result); 0 mixed ids, 0 bad JSON. Latency 138.8 ms and 683.9 ms. |
| `--resume` | 1 (in the two-at-once run, of the killed session) | Same session id, same transcript file: new lines were read from the saved end, no repeat of old records. The reference holds 2 ids (1 old, from before the tail started, 1 new); 1/1 new ids seen. On resume, Claude Code wrote a `tool_result` with `is_error` true for the interrupted old id (recv 16:06:14.494Z record time), then the new Read and its result. |
| Session killed mid-tool | 1 good (+1 spoiled) | SIGKILL at 25 s while `python3 -c "import time; time.sleep(100)"` ran: tool_use seen 1/1 (latency 791.6 ms), tool_result 0, exit -9. The orphan `sleep` child kept running to its end. |
| Tool output over 1 MB (line length only) | 1 (in the two-at-once run) | Real session: longest line 153727 bytes (< 1 MB, not reproduced). Synthetic 1.1 MB line: read whole, events correct, no content in events. |
| Truncated last line | unit test | Half line not parsed (0 bad JSON), emitted once when completed. |
| New file mid-session (subagent) | 5 live runs + unit test | Read from byte 0; 1 to 2 subagent files per live run. |
| Unknown record type | live + unit test | Abstained and counted per type, no event. |
| File truncated / replaced; resume by offset | unit tests | Re-read from 0; saved offsets skip old lines. |

## Verdict

keep with limits.

* Keep: 45/45 capture, median 196.5 ms, no measurable session overhead (-873 ms vs a 5768 ms spread), out of process, reuses the repo reader unchanged, works for subagent files and parallel sessions.
* Limits: p95 1352.8 ms because Claude Code writes Bash and subagent tool_use records 0.7 to 1.5 s after their timestamp (the tail itself adds at most the poll interval, 100 ms); no `cost_usd`; the transcript format is internal and `parse_record` already abstains on 8 record types in 2.1.316 (36 to 67 per run); a >1 MB line was not reproduced live.
* Not decided here: whether 0.2 s median (but 1.4 s p95) is "live" enough for a dashboard; that is the owner's call in the HUN-170 report.

## Tail interface for HUN-170

* `start_tail(projects_dir, out_path, poll_ms=100, from_start=False, state_path=None, keep=None) -> handle`; `handle.stop() -> stats`.
* `read_events(out_path)`, then `score(events, read_reference(session_id, config_dir), to_epoch_ms) -> {captured, reference_total, missed, median_ms, p95_ms, latency_ms, ...}`.
* CLI: `python experiments/live-trace/e1/tail.py --projects DIR --poll-ms 100 --out FILE [--state FILE] [--from-start] [--duration SEC]`.
