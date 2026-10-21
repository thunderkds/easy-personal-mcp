# Spike HUN-32: Claude Code transcript shapes and real volume

Date: 2026-10-21. Method: `experiments/scan_transcripts.py` (read-only, prints aggregate counts only).
No transcript text, path or real session id is stored in the repo or in this note.

## Sample size (read this first)

The owner's `~/.claude/projects/` holds only **5 `.jsonl` files** in 3 project folders, the oldest
modified 6.7 days ago. So "30 days" is really **about 7 days**. All percentages below come from
575 lines. Treat them as a floor on variety, not as proof a shape never occurs. Also found in the
folder: `.md` and `.json` files (not scanned, not transcripts).

## Volume (last 30 days = everything present)

| Measure | Value |
|---|---|
| Files | 5 (largest 951 KB, smallest 100 KB) |
| Bytes | 2,208,688 (2.2 MB) |
| Lines | 575 (about 3.8 KB per line) |
| Distinct `sessionId` | 5 |
| Nested / `subagents/` transcript files | 0 |
| Files with a truncated last line | 0 |
| Unparseable lines | 0 |
| Bare `json.loads` speed on this data | about 240 MB/s (0.009 s total) |

Rate: 2.2 MB in about 7 days is about 0.3 MB per day, so 30 days is about 10 MB. The provisional
"30 days = 500 MB" in NFR-005 is about 50 times the measured value.

## Record types per `version` (line counts)

| `version` | assistant | user | attachment | system | other |
|---|---|---|---|---|---|
| 2.1.293 | 13 | 7 | 36 | 3 | |
| 2.1.295 | 78 | 46 | 192 | 7 | |
| 2.1.302 | 2 | 2 | 27 | 0 | |
| no `version` field | | | | | 162 (types below) |

Types with **no `version` field** (metadata lines mixed into the same files; none has `message`):
`agent-name` 25, `artifact-autoreact-ledger` 7, `artifact-comment-monitor` 2, `atis-latch` 29,
`cost-state` 5, `custom-title` 28, `file-history-delta` 1, `file-history-snapshot` 10,
`frame-link` 2, `last-prompt` 29, `queue-operation` 24.
(`artifact-autoreact-ledger` and `file-history-delta` have no fixture; `file-history-delta` and `artifact-autoreact-ledger` are rare and metadata-only.)

## Fields FR-002 needs (presence % within the type)

| FR-002 item | Where | Presence |
|---|---|---|
| timestamp | `timestamp` (24-char ISO UTC string) | 100% on user/assistant/attachment/system; **absent** on `agent-name`, `custom-title`, `last-prompt`, `atis-latch`, `cost-state`(has `startTime` ms), `file-history-snapshot` |
| session id | `sessionId` | 100% on all versioned types and most metadata types (not on `file-history-*`) |
| project path | `cwd` | 100% on user/assistant/attachment/system; absent on metadata types |
| branch | `gitBranch` | 100% on same set as `cwd` |
| model | `message.model` | 100% of assistant |
| record type | `type` | 100% of lines |
| tool names | `message.content[].name` where block `type` = `tool_use` | 31% (2.1.293) / 50% (2.1.295) of assistant records |
| token fields | `message.usage.{input_tokens,output_tokens,cache_read_input_tokens,cache_creation_input_tokens}` | 100% of assistant; all ints |
| sidechain flag | `isSidechain` | 100% on versioned types; always `false` in this sample |

Other useful fields: `usage.cache_creation.{ephemeral_5m,ephemeral_1h}_input_tokens`,
`usage.server_tool_use.{web_search_requests,web_fetch_requests}`, `usage.service_tier`,
`attributionMcpServer` / `attributionMcpTool` (54% of 2.1.295 assistant; the MCP call attribution).

## Findings that change the parser design

1. **One API message is split over several `assistant` records.** 93 assistant records carry only
   40 distinct `message.id`; 33 ids repeat (one record per content block: thinking, text, tool_use).
   In all 33 cases `usage` is **identical** on every record. Summing per record would
   over-count tokens about 2.3 times here. Dedupe key: `message.id` (take usage once).
2. **Metadata types carry no `version`** and no `cwd`. A parser keyed on `version` alone would
   abstain on 162 of 575 lines (28%). Decide: map by `type` first, `version` is a label only.
3. **Types appear at different Claude Code versions inside one file**: all five files are mixed,
   so "per file version" does not exist; use the per-record `version`.
4. **API-error assistant records** (2.1.302): `error`, `isApiErrorMessage: true`, zero usage, text in
   `content`. Count as a record, not as usage.
5. `cost-state` holds `totalCostUSD` and per-model usage. It is Claude Code's own cost figure.
   Do not read it into the store without an owner decision (it would conflict with "API-equivalent cost").
6. Free-text fields to **never** read: `message.content`, `toolUseResult`, `attachment`,
   `queue-operation.content`, `last-prompt.lastPrompt`, `custom-title.customTitle`,
   `frame-link.title/path/frameUrl`, `file-history-*` paths, `thinking`.
7. Not observed in the sample (fixtures are **assumptions**, mark as such): `isSidechain: true`
   records, `subagents/` nested transcript files, a truncated last line. These are common
   in live Claude Code use, so fixtures include them.

## Fixtures (`tests/fixtures/claude_code/`)

| Path | Purpose |
|---|---|
| `2.1.293/session.jsonl`, `2.1.295/session.jsonl` | all versioned types, split `message.id`, `tool_use` + `tool_result`, one unknown type, truncated last line |
| `2.1.295/<sid>/subagents/agent-fixture0001.jsonl` | nested sidechain file (assumed layout) |
| `2.1.302/session.jsonl` | API-error assistant, two unknown types, truncated last line |
| `unversioned/meta.jsonl` | the types with no `version`; truncated last line |

All values are invented: `sessionId` is `00000000-0000-4000-8000-<n>`, `cwd` is `/work/demo-app`,
text is `INVENTED_*`, and the planted token is `sk-FAKE-0000000000`.
Check: `python3 -m pytest tests/fixtures_check -q`. With the optional env var `EPM_SCAN_IDS`
(a file of real ids, kept outside the repo) it also proves no real id is present.

## Proposed NFR-005

> Cold ingest of the owner's 30 days of transcripts (measured about 10 MB; design ceiling 100 MB, ten times
> the measurement) completes in **< 10 s**, and a re-run with no changed files completes in **< 2 s**.
> Ingest streams line by line (bounded memory, no whole-file read).

Why: bare JSON parse runs near 240 MB/s, so 100 MB costs about 0.5 s of parsing. The store write
dominates and is the thing to measure in HUN-34. The old 500 MB / 60 s figure stays valid as a
stress case, not as the target. Re-measure when the owner has more history (the sample is 7 days).

## Open questions for the owner

- Is the `~/.claude/projects/` folder pruned by Claude Code (cleanup period)? Only 7 days exist, which suggests yes.
- Should `cost-state.totalCostUSD` be used at all? (see finding 5)
