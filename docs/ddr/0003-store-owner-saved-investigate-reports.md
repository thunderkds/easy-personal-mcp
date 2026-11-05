# 0003. Store owner-saved investigate reports (narrow NFR-002 exception)

**Status**: Accepted
**Date**: 2026-11-05
**Deciders**: thunderkds (owner), Supervisor
**Related**: HUN-221, HUN-222 · FR-076, FR-077, FR-078 · NFR-002, NFR-004 · US-032 · DDR-0001

---

## Context

The `investigate` prompt (FR-075) makes the owner's own Claude Code session write a usage report. The report lives only in that chat. The owner asked (2026-11-05) to keep it on the dashboard "for easy refer, or trace the improvement".

NFR-002 says no message text appears in any store row. Out of Scope listed "LLM-written narratives". Storing the report breaks both as written.

Gate: (1) hard to reverse — stored text stays in the owner's store and backups; (2) surprising without context — a privacy-first, metadata-only store now holds model text; (3) genuine trade-off — three options were on the table. 3 of 3 hold: ADR-eligible, owner asked whether to escalate.

---

## Decision

We will add one MCP tool, `save_report`, as the only path that stores text. It stores what the owner's session sends: a summary of ≤ 2,000 characters and ≤ 5 fixes of ≤ 300 characters. It refuses the whole call when a limit is passed or any text looks like a secret. Next to the text, the server stores its own rule snapshot. Before / now comparisons use only the server's numbers (NFR-004); the text is never parsed into numbers. The server itself still writes no text and runs no model.

---

## Alternatives Considered

| Alternative | Pros | Cons | Why not chosen |
|-------------|------|------|----------------|
| Narrow exception: report text + server snapshot | Owner rereads the written fixes; improvement measured from server numbers | NFR-002 no longer absolute; model text in the store | **Selected** (owner, 2026-11-05) |
| Fixes only, no summary | Less text stored | Loses the "why" the owner wants to reread | Owner chose the full report |
| Numbers only (NFR-002 stays strict) | No rule change | Written fixes stay in the chat; nothing to "refer" to | Owner chose report + numbers |

---

## Consequences

- NFR-002 now has one named exception. Leak-suite fixtures must prove the secret check refuses planted secrets sent to `save_report`.
- `save_report` is the first MCP tool that writes. It goes through the store's single-writer path like ingest does.
- Reports cannot be deleted or edited from the page in this release (cut list, PRD Out of Scope).
