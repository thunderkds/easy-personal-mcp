# BRAINSTORMING_LOG.md — Waiting sessions (tray rows, popup click, question text)
**Generated**: 2026-11-16
**Task / Context**: owner, 2026-11-16: "from the tray icon, or notification, with the HITL let show the notif for users to click on. For the dashboard let show the question, or stop point to the live section" · "just a list of action to click on, no text input" · "go [admin], with question text". PRD US-040, FR-105..FR-108, question 39.
**Skill**: `Skill({ skill: "brainstorming" })` — tier **Standard** (the requirement is fixed; two mechanisms are open)

---

## The Problem Space

The Rust `tray` process already marks a session *waiting for you* and raises a popup, but the owner cannot act on it:

1. **Most stops are not seen.** Only an open `AskUserQuestion` / `ExitPlanMode` call is a state (`rust/crates/core/src/live.rs:64`). A main thread that ended its turn and waits for the next prompt gives no state.
2. **The tray shows a count, not a list.** One row `N waiting for you · <project>` (`rust/crates/app/src/tray.rs:293`); the *Recent alerts* rows are `Row::Info`, not clickable.
3. **A click goes nowhere useful.** The popup is sent with an empty action list and the D-Bus connection is dropped (`rust/crates/app/src/alerts.rs:375-410`). A tray row opens the window on its last view (`rust/crates/app/src/window.rs:431`). The page's address holds only the token (`web/src/api.js:4`), so there is no address for "Live view, this session".
4. **The Live view cannot say what is asked.** It prints the fixed words `waiting for you: question` (`web/src/LiveView.jsx:53`); FR-002 / NFR-002 forbid reading the input.

Fixed by the owner and the Debate (question 39): only `questions[].question` is read, kept in memory, sent on `GET /api/live` and shown on the Live view only. The tray and the popup stay text-free and have no text input.

Constraints that do not move: NFR-001 (loopback only), NFR-008 (guard on every route), NFR-012 (no new loop, thread or route for a state), the `tray` build adds no IPC command, the Python app is not changed (`rust/PROJECT_SPEC.md` Critical Constraints), the parity goldens are never edited by hand.

Two mechanisms are open: (a) how the question text is kept away from the tray and the alert engine, which read the same `LiveTail::snapshot()` (`alerts.rs:465`, `tray.rs:339`) as the stream; (b) how a click tells the page which session to show.

---

## Questions for the User

None is open. Under `[admin]` the Supervisor decided these, and question 39 of `PRD.md` lists them for review:

1. *Your turn* holds until the session leaves the live list (one hour without a record).
2. The *your turn* popup waits 60 s, so a turn answered at once raises none.
3. At most 5 session rows in the tray menu.

---

## Alternative Paths

| Option | Name | Summary | Invasiveness | Code Volume | Regression Risk | Recommended? |
|--------|------|---------|-------------|------------|----------------|--------------|
| A | Focus event, text beside the snapshot | The text is added only where the stream builds its snapshot; a click sets a *focus request* that the open stream sends to the page as an SSE `focus` message | Medium | ~700 lines with tests | Low | ✅ Yes |
| B | Address deep link, text in the snapshot | The text is a key of `LiveTail::snapshot()`; a click loads `/#<token>&session=<id>` in the window | Medium | ~600 lines | Medium | |
| C | Open the Live view only | A click opens the Live view with no session selected; the text is a key of the snapshot | Low | ~350 lines | Medium | |

### Option A — Focus event, text beside the snapshot
**Approach**: `LiveTail::snapshot()` keeps its keys. The live tail holds the question text on the open call and gives it through a second read that only `live_feed.rs` calls when it builds the stream's snapshot (key `questions` in the *waiting for you* state). `alerts.rs` and `tray.rs` cannot see it, by construction. A click (tray row, popup action) writes a *focus request* `{session_id | null}` into a slot the server shares with the stream; each open stream sends one `event: focus` message for a request newer than itself, and a stream that starts within 10 s after a request sends it too (the window was closed and has just opened). The page switches to the Live view and selects the session.
**Pros**: the text cannot reach the tray or a popup through a later refactor; no change of the address or the token read; no script run in the window; works for an open window and for a new one; the `focus` message is ids only.
**Cons**: one new SSE message kind, which the page and the self-test reader must accept; a slot shared by the tray thread and the stream.
**Why it might fail**: a stream that reconnects inside the 10 s re-sends the focus (harmless: it selects the same session); the stream's allow-list of keys (`SESSION_KEYS`) must pass `questions` through without passing anything else.

### Option B — Address deep link, text in the snapshot
**Approach**: add `questions` to the snapshot JSON and trust `alerts.rs` / `tray.rs` to keep reading fixed keys. A click navigates the window to `/#<token>&session=<id>`; `web/src/api.js` learns to split the fragment.
**Pros**: no shared slot; the selection survives a reload.
**Cons**: the token read changes on every page load (NFR-008 surface); a navigation of an open window reloads the page and drops its state; the text sits one key away from the popup.
**Why it might fail**: any code that formats the snapshot for a log or a row leaks the text; a session id in the fragment next to the token invites a parsing bug that breaks sign-in.

### Option C — Open the Live view only
**Approach**: as B for the text; a click only opens the Live view, where the waiting sessions are already sorted first.
**Pros**: least code.
**Cons**: does not do what the owner asked ("click on" a HITL item); with several waiting sessions the owner still has to find the right row.
**Why it might fail**: FR-107 is not met.

---

## 50% Rule Check

Half the code of A: drop the 10 s rule and the `null` request (a click on a summed popup would then do nothing), and reuse the existing `snapshot` message with one extra top-level key `focus` in place of a new message kind. Rejected: the snapshot is compared with Python's by the parity tests, and a closed window would lose the click. Kept from this check: the focus request has no queue, no history and no acknowledgement — one slot, the newest request wins.

---

## Recommended Path

**Text beside the snapshot: settled. The click mechanism: open.**

Debate 2026-11-16 (3 Advocates). All three proposed the same for the text: `LiveTail::snapshot()` never holds it, and the stream gets it through a separate read. That part of Option A is taken.

For the click, no Advocate proposed the `focus` message of Option A (their reasons: it reaches every client of the stream, browser tabs included, and it needs a pending state for a window that is not open yet). Each proposed another mechanism:

- Advocate 1: a query on the window address, `/?live=<session id>#<token>`, with a navigation of an open window (the page reloads) and a page that reads the query once and strips it. The token read does not change.
- Advocate 2: the fragment, `#<token>&live=<session id>`, read on load and on `hashchange` (no reload). `readToken` must split the fragment.
- Advocate 3: a fixed script run in the window (an event for an open window, an initialization script for a new one). No reload, no address change; the id is checked and JSON-encoded.

No written rule or Citation picks one. Under `[admin]` this is a stop: `tasks/TASK_REVIEW_T095.md` holds the `Paused:` line. T094 does not depend on it and goes on.

Delivered in three tasks, in this order (each can be verified alone):

1. **T094** core and stream: the *your turn* state, the question text, the `questions` key on `GET /api/live`, the leak proofs.
2. **T095** page: the `your turn` badge, the question text, the `focus` message.
3. **T096** tray and popup: session rows, the 60 s *your turn* alert, the popup's default action, the focus request.

---

## Surgical Scope

Files that **should** be touched:
- `rust/crates/core/src/live.rs`, `rust/crates/core/src/ingest/claude_code.rs` — `stop_reason`, the state, the text on the open call (T094)
- `rust/crates/app/src/live_feed.rs` — `questions` in the stream snapshot (T094); the `focus` message (T096)
- `rust/parity/leak_rust.py`, `rust/parity/README.md` — leak proofs, the documented difference from Python (T094)
- `web/src/LiveView.jsx`, `web/src/App.jsx`, their tests, `web/src/app.css` — badge, text, focus (T095); the built page under `src/easy_personal/dashboard/static` as `scripts/check_static.py` requires
- `rust/crates/app/src/tray.rs`, `alerts.rs`, `window.rs`, `server.rs` — rows, alert, popup action, focus request (T096)
- `docs/ddr/0005-live-question-text-show-never-store.md`, `PRD.md`, `PROJECT_SPEC.md` — written at Stage 0 by the Supervisor

Files that **must not** be touched:
- `src/easy_personal/**` Python code and `tests/**` — the Python app is the parity reference and does not get the feature
- `rust/parity/golden*/**` by hand — a golden changes only through its generator, and this work needs no golden change
- `rust/crates/app/src/keychain.rs`, `launch.rs`, the store schema — no secret path, no migration

---

## Edge Case Checklist for TASK_GUIDE

- [ ] A headless session (`entrypoint` starts with `sdk`) and a session with no known entrypoint never get *your turn*
- [ ] A subagent stream's `end_turn` never gives *your turn*; an `end_turn` followed by any `assistant` or `user` record of the main thread ends it
- [ ] `stop_reason` of any other value (`tool_use`, `stop_sequence`, `max_tokens`, `null`, a number, an object) gives no state
- [ ] A warm start (FR-086 replay) of a session that ended its turn before the app started shows *your turn*, and raises no popup burst
- [ ] `questions` that is not a list, an item that is not an object, a `question` that is not a string, 5 or more questions, a 10,000-character question, an empty string
- [ ] A question that holds a secret-shaped string is dropped whole; control, bidi and zero-width characters are removed
- [ ] The text is gone from the stream on the first snapshot after the call's result arrives
- [ ] A planted `header`, option `label`, option `description` and `ExitPlanMode` `plan` never reach `/api/live`; a planted question never reaches `/api/alerts`, a tray row, a popup, the store, a log, MCP or the CLI
- [ ] Popup click when the notification service has no `actions`, has gone away, or answers after the 3 s bound: no panic, no hang, the alert stays in the list
- [ ] An `ActionInvoked` signal for a notification id this process did not send is ignored
- [ ] A click for a session that left the list: the Live view opens, nothing is selected, no error
- [ ] Six or more waiting sessions: 5 rows and `+N more in dashboard…`; the menu is rebuilt without leaking menu items
- [ ] `Pause alerts`: no popup, the rows and the state stay
- [ ] Long question text wraps at 375 px and never widens the page; it is React text, never HTML

---

## Next Actions

1. Write `docs/ddr/0005-live-question-text-show-never-store.md` (gate: surprising without context, and a real trade-off).
2. Write `tasks/TASK_GUIDE_T094.md`, `T095`, `T096` and their board rows in `rust/PROJECT_KANBAN.md`.
3. Run the tasks one at a time, in order; each ends with its own review, auto verify run and merge.

---

## User Selection

> **Approved direction**: the text part of Option A (text beside the snapshot, stream only) — DEBATED 2026-11-16, all three Advocates. ⚠ decided for you — please review.
> **Open**: the click mechanism (query / fragment / script). Waiting for the owner's answer; see `## Recommended Path`.
