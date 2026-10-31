# Claude Live Trace — Results

Spike HUN-165..170, 2026-10-31. Claude Code 2.1.316, model Haiku, one account (`~/.claude-hung2`), one Linux machine.
Plan: the owner's doc "Claude Live Trace — Experimental Plan". Metadata only (PRD FR-002 / NFR-002): ids, names, timestamps, counts, token and cost numbers.
Per-source detail: `harness/RESULT.md`, `e1/RESULT.md` .. `e4/RESULT.md`, `report/e5.md`. Combined-run data: `report/runs/combined-1..5.json`, pooled in `report/runs/summary.json` (`python experiments/live-trace/report/combined.py aggregate`).

## Recommendation

Build the next stage (a live collector plus a dashboard) on three sources joined by `tool_use_id`:

1. **E2 `http` hooks as the live channel.** In the combined run, 45 of 46 tool starts arrived at median 4.7 ms and p95 7.0 ms after the transcript timestamp. The hook needs one settings block and a local server that replies at once (`async: true` is ignored on `http` hooks).
2. **E1 transcript tail as the complete record and the fallback.** It saw 46 of 46 tool calls, at median 213 ms and p95 1926 ms (100 ms poll). It needs no setup, works for any session, and its numbers do not depend on hook or telemetry settings.
3. **E3 OpenTelemetry (logs every 1000 ms) only for cost and exact tokens.** It is the only source of `cost_usd`; tokens and cost matched the transcript and the stream-json result to 0.000% in 5 of 5 combined sessions. It arrives about 1 s behind (median 1005 ms, p95 2897 ms), so show cost as "about a second behind", not as live. It carries the account email and ids on every record, so the receiver needs a value allow-list.

Do not build the product on E4 stream-json: it is the fastest source (median 1.3 ms, p95 2.7 ms, 46 of 46) but it exists only for a `claude -p` process we start ourselves, never for the owner's open terminal. Keep it for launched headless runs. Do not build on E5: its best freshness is about 1 h and it needs an Admin API key the project does not have (docs only, not tested).

**What is live (under 2 s behind the transcript timestamp) and what is not.** Live: tool starts from hooks (45 of 46 = 97.8% under 2 s, all 45 under 8.1 ms) and from the transcript tail (44 of 46 = 95.7%; two Bash/Agent calls in run 1 took 3112 and 3143 ms). Cost and tokens from OTel are 38 of 46 = 82.6% under 2 s, so they are near-live, not live. Not live: metrics at the default 60 s interval (they arrive once at process exit), the Admin and Claude Code analytics APIs (1 h to 24 h), `SessionStart` over `http` hooks (it never fires), and any `claude` session started without the hook and OTel settings in place.

## Comparison

All numbers: combined run, 5 standard Haiku sessions with E1, E2, E3 and E4 on at once, 46 tool calls (45 in the first 4 runs plus 1 extra `Edit` in run 5). Latency = local receive time minus the transcript `timestamp` of the record holding the `tool_use`, same machine clock, matched by `tool_use_id`. p95 is the nearest-rank value used in the earlier RESULT files. Overhead = wall-clock of the process against the HUN-165 baseline (median 28264 ms, noise 5768 ms): the combined sessions took 24580, 25986, 27620, 27841 and 31477 ms, median 27620 ms = -644 ms, worst +3213 ms, all inside the noise. Per-source overhead comes from each source alone (its RESULT.md); the combined run cannot split it.

| Source | Median latency | Completeness | Setup effort | Overhead | Verdict |
| -- | -- | -- | -- | -- | -- |
| E1 transcript tail (100 ms poll) | 213 ms, p95 1926 ms, max 3143 ms (alone: 197 / 1353) | 46 of 46 tool_use (100%); no cost field | none: read-only watch of `projects/**`, reuses the repo reader; format is internal | alone -873 ms, tail CPU about 3% of one core, scan max 7.3 ms in the combined run | keep: the complete record and the fallback |
| E2 `http` hooks to a local collector | PreToolUse 4.7 ms, p95 7.0 ms (alone: 5.9 / 10.1); PostToolUse 30 ms but p95 2692 ms (includes tool run time) | 45 of 46 PreToolUse (97.8%); the missed one is a call rejected before any hook ran; `SessionStart` never fires | one settings block for 9 events plus a collector that replies at once; the owner's settings or a per-project file | alone -1988 ms; down collector did not slow or fail a session | keep: the live channel |
| E3 OpenTelemetry, logs 1000 ms | `tool_result` 1005 ms, p95 2897 ms (alone: 1007 / 3253; default intervals 3388 / 7263) | 45 of 46 `tool_result` (97.8%); tokens and cost exact (0.000%, 5 of 5) | 7 env variables on every `claude` start (5 base + 2 intervals), a receiver with a value allow-list | alone -400 ms; down receiver did not slow or fail a session | keep with limits: cost and tokens, about 1 s behind |
| E4 stream-json (`claude -p`) | 1.3 ms, p95 2.7 ms (alone: 1.8 / 4.4) | 46 of 46 (100%), subagent included | launch every session through our own `-p --output-format stream-json --verbose` | alone -975 ms | keep with limits: launched headless runs only, not the owner's terminal |
| E5 account usage APIs | not live: 1 h to 24 h (docs) | not tested (docs only, not tested — no Admin key) | an Admin API key, Console organizations only | none (no call made) | drop for live use; maybe a daily reconciliation if a key ever exists |

Other combined-run numbers:

* Same events: the 45 tool calls that all four sources captured give median 217 ms (E1), 4.7 ms (E2), 1005 ms (E3), 1.3 ms (E4).
* Per tool, median of E1 / E2 / E3 / E4 in ms: Read (20) 166 / 4.6 / 1004 / 1.3; Edit (6) 166 / 5.5 / 1005 / 1.1; Bash (10) 1036 / 5.2 / 1582 / 2.2; Agent (5) 1184 / 4.4 / 1025 / 1.7; SubagentHandback (5) 648 / 5.1 / 1005 / 1.1. E1 is late for Bash, Agent and SubagentHandback because Claude Code writes those records late (E1 RESULT.md: tail lag at most 100 ms).
* Cost cross-check: the sum of OTel `api_request` `cost_usd` equals the stream-json `total_cost_usd` of the result line with `result_index` 0 in 5 of 5 sessions (0.00885241, 0.00809928, 0.00831361, 0.00823763, 0.00845497 USD).
* Token cross-check: OTel token sums equal the transcript `message.usage` sums in 5 of 5 sessions (for example 280065 of 280065 tokens in run 1), from `api_request` events and from the token metric.
* E3 `tool_decision` arrives at the same time as `tool_result` (both median 1005 ms): it gives no earlier signal.

## Account usage findings

* The cost figures in OTel and stream-json are a local estimate at list price. Five combined sessions cost 0.0420 USD together (0.0081 to 0.0089 USD each). They are API-equivalent numbers, not plan usage.
* No plan-quota number (the plan bars of `/usage`) appears in any source. The field and attribute names listed in `e3/RESULT.md` and `e4/RESULT.md` hold none; transcripts hold no cost at all (E1: `cost_usd` 0 of 77).
* A Pro or Max account has no usage API in the docs. An Admin API key (Console organizations only) unlocks the usage, cost and Claude Code analytics endpoints; none of them is under 1 h fresh. See `report/e5.md` (docs only, not tested — no Admin key).
* The kind of account behind `~/.claude-hung2` is unknown: the spike never read credentials.
* The spike used 46 headless Haiku sessions: harness 6, E1 9, E2 9, E3 9, E4 8, this combined run 5 (the spare was not used). The plan budget was about 25. About 0.38 USD at list price (46 times about 0.0082 USD; an estimate, not a measured sum).

## Risks and open questions

* **A call rejected early is invisible to E2 and E3.** In run 5 an `Edit` got an error `tool_result` 2 ms after its `tool_use` record (before any permission or hook step): no PreToolUse, no PostToolUseFailure, no OTel `tool_result`. E1 and E4 saw it. Hooks and OTel alone under-count by this kind of call (1 of 46 here). The error text was not read (metadata only), so the cause is unknown.
* **E1 tail got slower with everything on.** p95 1926 ms and max 3143 ms against p95 1353 ms alone. Cause not measured: either Claude Code writes records later with hooks and telemetry on, or the one-process threads (tail, collector, receiver) contend.
* **Hook bodies and OTel records carry content or identity.** The hook POST holds the prompt, tool input and tool output in transit to localhost; OTel carries the account email and ids on every record. The collector must drop them before storing (the spike did, and the planted-secret grep is clean). A product must keep that allow-list.
* **`async: true` is ignored on `http` hooks** (E2: +38 s for a collector that replied in 2 s). A slow collector stalls the session. The collector must reply at once and work afterwards.
* **Transcript format is internal.** `parse_record` abstains on 8 record types new in 2.1.316 (E1: 36 to 67 records per run). The product ingest skips them silently.
* **Headless only.** All 46 sessions ran with `claude -p`. Hooks and OTel behaviour in an interactive terminal was not tested, and hooks there need the owner's settings to change (the spike never edits `~/.claude*`). Open question for the owner.
* **Small sample.** 5 combined runs, n=46 calls, one Haiku session shape, one version, one account. p95 with n=46 is the 44th value.
* **Not measured:** a receiver that accepts and then stalls (E3); a clean one-session async-versus-sync hook pair with a slow collector (E2); a transcript line over 1 MB in a real session (E1); the early tool name from stream-json partial events (E4).
* **Shared lock.** Every session ran under one machine-wide lock, so timings were taken one session at a time. Two live sessions at once were tested for capture only (E1, E2), not for latency.

## v2 ideas

* Prototype the collector: hook receiver plus transcript tail in one process, joined by `tool_use_id`, with OTel logs for cost (next review stop).
* Replace the 100 ms poll of E1 with inotify and measure again; find why Bash and Agent records land 0.7 to 1.5 s late.
* Test one interactive session with hooks and OTel set in a project-level settings file, with the owner's agreement.
* Add `duration_ms` (sent by the PostToolUse hook, not yet kept) so tool run time is exact.
* Derive cost from transcript tokens and a price table, so cost does not need OTel.
* Find why a call is rejected before any hook (the run 5 `Edit`), and whether other hook events cover it.
* If an Admin API key ever exists, run E5 for a daily reconciliation against the live totals.
* Re-run the field and record-type checks on every Claude Code upgrade (transcript record types, stream-json `result` fields, OTel attribute names).
