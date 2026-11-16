# 0005. Live view shows the question of a waiting session (show, never store)

**Status**: Accepted
**Date**: 2026-11-16
**Deciders**: thunderkds (owner), Supervisor
**Related**: FR-105, FR-106, FR-107, FR-108 · FR-002, FR-047, FR-050, FR-097 · NFR-002 · US-040 · DDR-0004 · PRD question 39

---

## Context

A session that waits on an `AskUserQuestion` call shows only the fixed words "waiting for you: question". The owner asked (2026-11-16) to see the question on the Live view: "For the dashboard let show the question", then "go [admin], with question text". The metadata-only rule (FR-002, NFR-002) bans tool input on every surface.

Gate: (1) hard to reverse — no: nothing is stored, one release removes it; (2) surprising without context — yes: model-written text now reaches the page; (3) genuine trade-off — yes: three widths were weighed. 2 of 3 hold: DDR, not ADR (`docs/claude-md/tracks.md` row 3).

---

## Decision

In the Rust app, the live tail reads `input.questions[].question` of an open `AskUserQuestion` call, and nothing else of that input. At most 4 questions, 300 characters each, control and hidden characters removed, a secret-shaped question dropped whole. The text lives in memory on the open call and is dropped when the result arrives. It is sent only on `GET /api/live`, inside the *waiting for you* state, and shown only on the Live view. `LiveTail::snapshot()`, which the tray and the alert engine read, never holds it.

---

## Alternatives Considered

| Alternative | Pros | Cons | Why not chosen |
|-------------|------|------|----------------|
| Question text only, Live view only | Answers "what does it ask"; smallest exception; follows DDR-0004 | No option labels | **Selected** (Debate 2026-11-16: all three Advocates proposed it) |
| Also `header` and option labels | More context | More model-written text; little the question does not say | Can be added later in one field; narrowing after a leak cannot undo the leak |
| Also the `ExitPlanMode` plan, or text in tray rows and popups | Most detail, visible without the dashboard | A plan is long and holds paths and commands; a popup's text goes to the desktop's notification service, its history and the lock screen, out of the app's control | Breaks FR-097; cannot be undone once shown |

---

## Consequences

- NFR-002 gets a third exception; FR-047 names the two new fields read (`stop_reason`, the question text).
- The leak suite must prove: a planted question reaches `/api/live` and nothing else (store, logs, `/api/alerts`, tray rows, popups, MCP, CLI); a planted `header`, option label, option description and plan never reach `/api/live`.
- The secret check matches known patterns only. A secret written in plain words inside a question reaches the owner's own local page. Accepted: the page is behind the token and Host/Origin guard (NFR-008) and nothing is kept.
- The Python app does not get the text or the *your turn* state. The parity goldens stay as they are; `rust/parity/README.md` names the difference.
