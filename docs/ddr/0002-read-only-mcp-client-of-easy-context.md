# 0002. Read-only MCP client of easy-context

**Status**: Accepted
**Date**: 2026-10-21
**Deciders**: thunderkds
**Related**: PRD FR-012, FR-025, NFR-001 · docs/GRILL_requirement.md N3, N4 · BRAINSTORMING_LOG.md F11

---

## Context

easy-personal shows easy-context index freshness per project. easy-context keeps its data in two places:
SQLite files under `~/.easy-context/indexes/<id>/`, and its MCP service on `127.0.0.1:8767` (`list_indexes`,
`get_stats`; header `X-Easy-Context-Root`). Until now no Easy Kit plug-in called another at runtime, and the
first plan said "no network at runtime". DDR gate: surprising without context ✅, genuine trade-off ✅,
hard to reverse — partly (it shapes the source interface and the network rule). 2 of 3 → DDR.

---

## Decision

1. The `context` source is an **MCP client** of the easy-context service on `127.0.0.1` only, using the MCP SDK's HTTP client (no new dependency).
2. It may call only an **allow-list** of read tools: `list_indexes`, `get_stats`. A test asserts no other tool name can be sent; `index_path` is never called.
3. Short timeout; on refusal, timeout or error the source reports **"no data"** for that ingest and never fails the run or hangs the page.
4. NFR-001 becomes "no traffic leaves the machine": calls to `127.0.0.1` are allowed; anything else is refused.
5. easy-personal does not keep indexes in sync. It shows freshness and fires `context_index_stale`; easy-context's own timer does the work.

---

## Alternatives Considered

| Alternative | Pros | Cons | Why not chosen |
|-------------|------|------|----------------|
| MCP client on 127.0.0.1 (chosen) | Public, versioned tool contract | Data only while the service runs; per-repo header | **Selected by owner** |
| Read `~/.easy-context/indexes/*/stats.sqlite` read-only | Works with the service down; keeps "no network" strict | Couples to easy-context's private schema | Owner preferred the public contract |
| Trigger `index_path` for stale repos | Fully automatic sync | Writes another plug-in's state; job queue and error handling | Breaks the read-only kit rule (PRD Open Q 1) |

---

## Consequences

### Positive
- Depends only on easy-context's public tools; its internal file layout can change freely.
- The read-only kit promise stays testable (tool allow-list).

### Negative (accepted trade-offs)
- When the container is down, index freshness shows "no data" for that period.
- First runtime plug-in-to-plug-in call in the Easy Kit; the network rule is relaxed to `127.0.0.1`.

### Follow-up
- [ ] M2 spike: does one `list_indexes` call return every repo, or is one call per repo (header) needed?
