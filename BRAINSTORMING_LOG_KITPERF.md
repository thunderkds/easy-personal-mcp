# BRAINSTORMING_LOG_KITPERF.md
**Generated**: 2026-11-03
**Task / Context**: Kit performance, first release — `PRD.md` US-026, FR-059..FR-063, OQ 22 (grill resolved 2026-11-03)
**Skill**: `Skill({ skill: "brainstorming" })` — tier **Standard** (requirements locked; 3 viable build shapes; one cross-repo question)

---

## The Problem Space

Give Claude, at session start, up to 3 findings of the form "sessions that used kit server X are *linked to* N% fewer tokens / shorter sessions / more good results, like with like", each with one habit. Inputs already in the store, plus one new per-record count (failed tool results). No LLM text, no message text, metadata only (FR-002, NFR-002).

Verified facts (2026-11-03, `~/.easy-personal/store.sqlite`, read-only):
- 625 sessions. A per-session aggregate over `events` (tokens, first/last time, server used) runs in 0.18 s → computing on read is fast enough.
- Sessions per kit server (projects): easy-ui 29 (2), easy-verifier 15 (10), easy-context 25 (5), easy-memory 18 (8). No `plugin_*easy*` tool names → the name rule `mcp__<server>__*` is enough.
- Tool results live in `user` records (55,707 stored, `record_type = user`). `sources/claude_code.py` `parse_record` returns early for non-`assistant` records and never reads their blocks → FR-059 must count `tool_result` blocks with `is_error` in **user** records, not "per message". `core/live.py` already reads only `name` / `is_error` of these blocks (same privacy line).
- `sources/git_local.py` counts commits per day over all refs; it has no per-branch or time-window read.
- `skills/wake/` is kit-managed: it is listed in `.claude/harness-lock.json`, and a kit update overwrites it (`.claude/update-log.md`, 2026-10-29). Editing `wake` in this repo does not last.

---

## Questions for the User

1. **Group size rule.** With strict like-with-like groups (same project + model + size band), almost no group reaches 10 sessions. easy-verifier has 15 sessions across 10 projects. Proposal: match each kit session with sessions without that server from its own group, then pool all matched sessions over groups; the "≥ 10" rule counts the pooled sessions on each side. Accept?
2. **Where `wake` changes.** `wake` belongs to the kit kernel repo. Proposal: record a cross-repo dependency XR-2 ("`wake` calls `kit_findings` when the easy-personal MCP is connected; at most 3 lines; silent on failure") on the kernel repo's own board. Until XR-2 ships, Claude reaches findings only by calling `kit_findings`. Accept?

---

## Alternative Paths

| Option | Name | Summary | Invasiveness | Code Volume | Regression Risk | Recommended? |
|--------|------|---------|-------------|------------|----------------|--------------|
| A | Compute on read | Migration adds one column `tool_error_count`; `core/kitperf.py` builds session rows + findings by SQL on each call; git window check per branch with a small in-process cache | Low–Medium | ~350 lines + tests | Low | ✅ Yes |
| B | Session facts at ingest | Migration adds a `session_facts` table filled by ingest (servers used, tokens, cost, length, branch, errors, commit result); findings read the table | Medium–High | ~600 lines + tests | Medium | |
| C | No migration | Count tool errors by re-reading transcripts at call time (reuse the live-tail parser); no store change | Low | ~250 lines | Low | |

### Option A — Compute on read
**Approach**: m0008 adds `events.tool_error_count INTEGER` (NULL = unknown) and marks every source file for re-read (pattern of m0007). `parse_record` counts `tool_result` blocks with `is_error is True` in `user` records. New `core/kitperf.py`: one SQL pass → session rows (owner project via `core/owners.py`, model = most-used, size band = token tertile, length, tool calls from `tool_names`, errors, servers used); git result per (repo, branch) via `git rev-list --timestamp <branch>` plus merged-branch lookup, cached for the call; matched pooling; fixed habit table. `kit_findings` in `surfaces/mcp.py` (+ CLI `easy-personal kit-findings` for verify).
**Pros**: one column; findings always reflect the latest ingest; follows the existing core → surfaces pattern (FR-008); easy to reverse (drop the tool, the column stays NULL-safe).
**Cons**: each call runs git per (repo, branch) — ~290 branch sessions; needs a cache and a timeout budget.
**Why it might fail**: git calls make `kit_findings` slow on a loaded machine (load average ~60 seen 2026-11-02) → `wake` stalls. Mitigation: a total time budget (for example 3 s); a branch past the budget counts as "unknown"; NFR row with the target.

### Option B — Session facts at ingest
**Approach**: ingest writes one row per session into `session_facts`, updated as sessions grow; git result computed at ingest.
**Pros**: fastest read; a history of facts that survives branch deletion.
**Cons**: a second table to keep in sync with re-reads, retention and owner-project regrouping (FR-057 is read-time only, so facts would go stale when grouping rules change); larger migration; timer ingest does git work.
**Why it might fail**: facts drift from events after a rule change, and nobody sees it; migration-safety surface doubles.

### Option C — No migration
**Approach**: at call time, re-read transcripts on disk for error counts.
**Pros**: no store change; ~50% less code.
**Cons**: contradicts the owner's choice (add the count to the store); transcripts are deleted after 30 days, so history loses the error part; reading 1,206 files per call is slow.
**Why it might fail**: `wake` time blows up; old sessions silently lose their quality signal.

**50% rule**: Option A's smallest form drops the CLI command and the matched pooling (raw per-server comparison inside one owner project only). Rejected as default: pooling is the only way to reach 10 sessions (Question 1); the CLI is ~20 lines and gives `/verify` a real entry point.

---

## Edge Case Checklist (for every TASK_GUIDE)

- `user` records written before m0008 re-read: `tool_error_count` NULL → session error rate "unknown", never 0.
- A tool call with no result (session killed): counts as a call, not an error.
- Sub-agent records share the parent `session_id` → server use and errors count toward the parent (FR-060).
- Branch merged and deleted: find commits through the merge commit (`Merge <b>`, FR-057 forms); none found → "unknown", not "bad".
- Two sessions on one branch at the same time: both get the commit; accepted, documented.
- Session on `main`/`master`/`HEAD`/no branch → result "unknown" (FR-061).
- Gone project folder: owner project via `core/owners.py`; no repo → git result "unknown".
- Fewer than 10 pooled sessions on a side → no finding for that server; `kit_findings` says why (counts).
- `kit_findings` must never return paths outside metadata rules, prompt text or session ids beyond what `session_outliers` already returns.
- Migration: back up as `store.sqlite.bak-v7`; old binary vs new store (version guard); re-read is idempotent (UNIQUE offsets).

---

## Surgical Scope

**Should touch**: `src/easy_personal/core/migrations/m0008_*.py`, `src/easy_personal/sources/claude_code.py` (user-record error count), `src/easy_personal/core/store.py` (column list), new `src/easy_personal/core/kitperf.py`, `src/easy_personal/surfaces/mcp.py`, `src/easy_personal/surfaces/cli.py`, `~/.easy-personal/config.yaml` threshold reader (`core/config.py`), tests, README, `PROJECT_SPEC.md`.
**Must not touch**: `skills/wake/` and other kit-managed files in `.claude/harness-lock.json` (XR-2 instead); `web/` (dashboard page is out of scope); `core/live.py` behaviour; stored events other than the new column.

---

## Recommended Path

**Option A — compute on read**, with matched pooling (Question 1) and XR-2 for `wake` (Question 2).

## Next Actions (Stage 2)

1. Add NFR-014: `kit_findings` < 3 s on the owner's machine at normal load; git work bounded by a time budget.
2. Set the `kit_error_rate` default from the owner's history after the m0008 re-read (median session error rate is a candidate).
3. Tasks (draft): T1 m0008 + user-record error count (C2, Medium, `migration-safety`); T2 `core/kitperf.py` + `kit_findings` MCP + CLI (C2, Medium, `security-review`); T3 XR-2 on the kernel board; T4 release note / README.
