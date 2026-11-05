# BRAINSTORMING_LOG_PERF.md — Slow dashboard data loading

**Date:** 2026-11-05 · **Tier:** Lightweight · **Traces:** NFR-013, FR-064 · **Mode:** auto

## The Problem Space

Owner, 2026-11-05: "the data loading take so long, investigate the root cause and propose the fix", then "yes, investigate and fix them [auto]".

Measured on a copy of the owner store (462k events; the last 7 days hold ~170k events, ~28k messages). Scripts: Supervisor scratchpad `perf/`.
- The Usage page opens with ~14 parallel `/api` reads. Page wall time is 5.1–6.0 s. Each read takes 0.3–1.4 s alone but 4–5 s in parallel. The work is mostly Python, so threads hardly overlap. Sequential total: 5.7 s.
- A warm second load is the same (5.1 s), so the 300 s git cache in `owners.py` is not the bottleneck.
- `metrics._messages` (`src/easy_personal/core/metrics.py:183`) takes ~0.31 s and runs ~7 times per page. `owners._stored_branches` (`src/easy_personal/core/owners.py:94`) takes ~0.26 s and runs in every `project_map` / `owner_map` caller.
- Index choice does not matter: the planner default 244 ms, `events_ts` 292 ms, `events_ts_message` 247 ms.
- The page waits for all of its reads (`Promise.all` in `web/src/UsageView.jsx` `load`).
- The rail computes its own `since`, so its URLs differ by milliseconds from the Usage view's.
- Owner machine: three dashboards paused with Ctrl-Z were still running, 16 GB swap was in use and 23% of CPU time was I/O wait. The Supervisor stopped the paused dashboards (owner said "yes").

## Alternative Paths

- **A — Shared computations in core (recommended).** A single-flight memo for `_messages` and `_stored_branches`, keyed by every input plus a store version. The page renders each part as its data arrives. Prototype: page 5.6–6.0 s → 2.7 s cold / 2.0 s warm, with no change to any result.
- **B — Whole-response cache in `api.py` `_query`.** It helps only repeat identical URLs. Live ingest changes the version every few seconds, so cold page opens stay ~4 s. MCP gets no benefit.
- **C — One combined `/api/usage/page` endpoint** that computes everything from one pass. It is the largest change (new API plus a page rewrite).

50% rule: B is the least code (~15 lines), but it misses the cold case that NFR-013 measures. A is about 60 lines plus tests.

## Adversarial Review (Debate, 2 Advocates)

- Advocate 1 (A): a `MAX(events.id)` key is unsafe. Upserts (`store.py` `ON CONFLICT DO UPDATE`), `UPDATE events`, retention deletes and workspace / account edits from the CLI do not change it. Cached values must be immutable.
- Advocate 2 (B): B fixes nothing cold on an active store. B alone cannot meet NFR-013, so it is at most a later second layer.
- Supervisor check: `PRAGMA data_version` on one long-lived connection changes on every commit by any other connection, in any process (tested: insert, update, other-process delete; stable when nothing changes). It needs no write-path change.

## Recommended Path

Option A with a `PRAGMA data_version` key on a dedicated watcher connection, immutable cached values, a small bound, and per-part rendering on the page with one minute-rounded `since`. Two tasks run in parallel: a server task (C2, Risk Medium, because `metrics.py` and `owners.py` are hub files) and a page task (C2, Risk Low).

Cut list: no whole-response cache (B), no combined endpoint (C), no browser cache, no new index, no write-path version counter.
