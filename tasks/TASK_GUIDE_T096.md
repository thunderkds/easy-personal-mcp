# TASK_GUIDE — T096: Waiting sessions 3 — tray rows per session, the *your turn* alert, a popup you can click, and the focus request
**Date**: 2026-11-16
**Complexity Level**: C3
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Attention state, Your turn, Waiting session) and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/backend.md`
5. Read `BRAINSTORMING_LOG_WAITING.md` (Option A, Edge Case Checklist), `BRAINSTORMING_LOG_TRAYALERTS.md`, the `PRD.md` rows FR-097, FR-099, FR-105, FR-107, NFR-001, NFR-002, and `tasks/TASK_REVIEW_T076.md` `## Menu proposal`
6. Read `rust/crates/app/src/tray.rs` (`view`, `state_line`, `pick`, `Row`, `same_shape`, tests), `rust/crates/app/src/alerts.rs` (`Engine::sessions`, `popup`, `Notifier`, `DBusNotifier`, `send`, `clean`, the tests), `rust/crates/app/src/window.rs` (`show_dashboard`, `open_window`, the menu build and update at ~l.400-540, `on_menu_event`), `rust/crates/app/src/live_feed.rs` (`LiveStream`, `sse`, `read_first_snapshot`), `rust/crates/app/src/server.rs` (how the feed is shared), and the T094 and T095 guides (the state JSON and the `focus` message the page accepts)
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder (use fixtures and temp folders). Never run a `#[ignore]` keychain test. Never send a real desktop notification from a test: use the fake notifier and a private bus address.

---

## Requirement

> **OPEN — do not start this task.** How a click tells the page which session to show is not decided: the Debate of 2026-11-16 ended with three different proposals (`tasks/TASK_REVIEW_T095.md` `Paused:`). The text below still describes the Supervisor's draft, a `focus` message on `GET /api/live`; the Supervisor rewrites this guide after the owner's answer.

**Original request** (owner, 2026-11-16): "from the tray icon, or notification, with the HITL let show the notif for users to click on" · "currently, I don't see the notif in the tray icon can show the HITL here ... just a list of action to click on, no text input".

**Restated intent** (FR-107):
> 1. **Session rows.** The tray menu lists one clickable row per *waiting session*: a session whose states hold `waiting`, `your_turn` or `failing`. Newest state first (by the state's `since`), at most 5 rows, then one clickable row `+N more in dashboard…` when there are more. A row's text is `<kind> · <project> · <short id>`: the kind in fixed words (`question` for an open `AskUserQuestion`, `plan to approve` for an open `ExitPlanMode`, `your turn`, `failing`), the project folder name through `alerts::clean`, the 8-character session id. A session with several states gets one row, named by its first state in the order `failing`, `waiting`, `your_turn`. The rows replace the two count rows (`N waiting for you`, `N failing`). No row holds question text. The menu has no text input.
> 2. **Icon.** The icon state *attention* also covers a session that is `your_turn`. *Problem* (failing) stays more urgent.
> 3. **Alert for your turn.** The alert engine raises one alert of kind `session_your_turn` per stay in the state, but only once the state has held for 60 s, measured by the engine on its monotonic clock from the first tick that saw the state (never from the record time in `since`: a clock jump or an old idle session must not fire it); a session that leaves the state before that raises nothing, and its timer is cleared. Text: `Your turn, project <project>, session <short id>`. Re-armed when the session leaves the state. The existing kinds, the 10 s popup gap, the summed popup (`N sessions waiting`, now also counting your-turn sessions in fixed words) and `Pause alerts` keep their rules. A start of the app with several sessions already in the state raises nothing for 60 s, then at most one summed popup.
> 4. **Popup action.** A popup is sent with one default action (`["default", "Open"]`) when the notification service lists the capability `actions`; else with no action, as today. The process keeps what it needs to hear `ActionInvoked` for the notifications it sent (today each `Notify` opens and drops a connection). A click on a popup for one session makes a *focus request* for that session; a click on a summed popup makes one with no session. A signal for a notification id this process did not send is ignored. Fail soft in every case: no bus, a service that goes away, a slow service (the existing 3 s bound) — no panic, no hang, the alert stays in `GET /api/alerts`.
> 5. **Focus request.** One slot shared by the server and its streams: the newest request `{session_id | null}` with its time. A click on a session row, on `+N more in dashboard…` (no session), or on a popup writes it, then the window is shown (`show_dashboard`: focus an open window, else open one). Each open `GET /api/live` stream sends one message `event: focus` with data `{"session_id": "<id>"}` or `{"session_id": null}` for a request that is newer than the stream's start; a stream that starts at most 10 s after a request sends it once, after its first `snapshot`. No queue, no history: the newest request wins. The message holds an id only. It is not a new route, and it needs no IPC command and no script run in the window.
> 6. **Only the `tray` and `app` commands make focus requests.** `serve` has no window: it never sends `focus`, and its popups carry no action.
> 7. Nothing is written to disk; no token and no session text in a log line.

**Out of scope**: answering from the tray or the popup; question text in a row, a popup or `/api/alerts`; a sound; a setting for the 60 s, the 5 rows or the 10 s; macOS and Windows notifications; the Python app; the page (T095).

### Requirement Fidelity Gate
- [x] Restated intent confirmed: auto (Supervisor), 2026-11-16, `[admin]`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: `PRD.md` US-040, FR-107, FR-097, FR-099, FR-105, NFR-001, NFR-002

---

## Dependencies & Reachability
**Depends on**: T094, T095
**Entry point**: `tray::view` (`rust/crates/app/src/tray.rs`) and the `tray` command's menu (`rust/crates/app/src/window.rs` `on_menu_event`)

## Dependency rule
No new crate: `zbus` 4.4.0 is already a dependency. No change to `Cargo.lock`. If hearing `ActionInvoked` needs a `zbus` feature that is not on, stop and report (`Status: blocked`) with the exact feature.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | `tray::view` tests: one row per waiting session with the text of intent 1 for each of the four kinds; newest first; 6 sessions → 5 rows and `+1 more in dashboard…`; a session with `failing` and `waiting` → one row `failing`; the two count rows are gone; a calm machine's menu is as before | intent 1 |
| 2 | A row's id carries the session (for example `session:<id>`), and `same_shape` / the menu update rebuild the menu when the set of sessions changes without leaking items (test on the plain `Row` list; the Tauri part is checked in the real run) | intents 1, 5 |
| 3 | Icon: a `your_turn` session alone → *attention*; with a failing session → *problem* | intent 2 |
| 4 | Alert engine tests with the fake notifier and a set clock: `your_turn` held 59 s → nothing; 60 s → one `session_your_turn` with the fixed text; still in the state → no second; leaves and returns → one more after another 60 s; leaves at 30 s → nothing | intent 3 |
| 5 | Summed popup: 2 waiting, 1 your-turn, 1 failing in one gap → one popup whose text counts each group in fixed words; an app start with 3 your-turn sessions → nothing before 60 s, then one popup | intent 3 |
| 6 | No text: with a planted question (T094) and a planted project name holding markup, no row, alert text, `/api/alerts` JSON or popup holds the question, and the project passes `clean` | intents 1, 7 |
| 7 | Popup action: against a fake notification service on a private bus (or a `Notifier` test double plus a unit test of the signal filter), `Notify` is sent with the `default` action when `actions` is listed and without it when not; an `ActionInvoked` for a sent id writes the focus request for that alert's session (none for a summed popup); an unknown id writes nothing | intent 4 |
| 8 | Fail soft: no session bus, a service that never answers, a service that drops mid-wait — no panic, the start is not blocked (extend `a_hung_notifier_never_blocks_the_start`), `Running::stop` still returns within its bound | intent 4 |
| 9 | Stream tests: a request made after a stream started gives exactly one `event: focus` with the right data on that stream; a stream started 5 s after a request sends it once after its first `snapshot`; started 11 s after → none; two requests → the stream ends on the newest; `serve` (no window) never sends one. The self-test reader (`read_first_snapshot`) still accepts the stream | intents 5, 6 |
| 10 | Leak gate `rust/parity/leak_rust.py` still exits 0 (the `focus` message and the new rows hold ids and fixed words only) | intent 7 |
| 11 | `cd rust && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p easy-personal-rs --features window --all-targets -- -D warnings && cargo test --workspace` pass; `cargo build --features window` builds | all |
| 12 | Real run, in your report: `tray` (window build) on a temp agent root with one transcript in each kind; a screenshot or a text dump of the menu rows; one click on a row opens the window on the Live view with that session selected (screenshot). If the sandbox has no display, say so plainly and give the exact commands for the reviewer | intents 1, 5 |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test -p easy-personal-rs --bin easy-personal-rs -- tray:: alerts:: live_feed:: 2>&1 | tail -6 && cargo build --features window 2>&1 | tail -2 && cargo test --workspace 2>&1 | tail -5
```

### Evidence (filled by reviewer at Stage 4/5)
> Moved to `tasks/TASK_REVIEW_T096.md`.

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T096.md`. BEFORE (before your first implementation commit): the `tray::view` rows for a snapshot with one `waiting`, one `your_turn` and one `failing` session (a test that prints them, or the menu of a real run): one count row, no row per session. AFTER: the same snapshot gives three session rows.

## UI / Design Acceptance Criteria
N/A — no page change. The tray menu is a native menu: its rows are checked as text (AC 1) and in the real run (AC 12).

---

## Approach

**Pattern reference**: `rust/crates/app/src/tray.rs` `view` + `Row` (T076, T088) — rows built as plain data with no Tauri code, tested as data; `rust/crates/app/src/alerts.rs` `cross` and the `Rig` test helper — one alert per state change with a set clock.

**Vital slice**: session rows that open the Live view on the session, and the *your turn* alert. The popup action comes after them: if `ActionInvoked` cannot be heard within the dependency rule, report it and deliver the rest (the popup then keeps no action).

Test first (`tdd`). Order: rows and icon (AC 1-3), alert (AC 4-6), focus slot and stream (AC 9), menu click wiring (AC 2, 12), popup action (AC 7-8).

Sketch (change it if you find a simpler one that meets the ACs):
- `Row::Action` today holds a `&'static str` id; a session row needs an owned id. Keep every `tauri` use in `window.rs`.
- The focus slot: a small `Arc<Mutex<Option<(Instant or micros, Option<String>)>>>` owned by the server state and handed to the tray; `LiveStream` reads it in its `step`.
- Hearing `ActionInvoked`: one long-lived connection in the alert thread's runtime with a match rule on `org.freedesktop.Notifications`, a map `notification id → Option<session id>` bounded to the last `KEEP` alerts; entries are dropped on `NotificationClosed`.

## Edge Case Checklist
- Six or more waiting sessions: 5 rows and `+N more in dashboard…`; the menu is rebuilt without leaking menu items
- A session id shorter than 8 characters, an empty project, a project that fails `clean`: the row leaves the piece out, never panics
- Popup click when the notification service has no `actions`, has gone away, or answers after the 3 s bound: no panic, no hang, the alert stays in the list
- An `ActionInvoked` signal for a notification id this process did not send is ignored; the `action_key` must be `default`
- `Pause alerts`: no popup, the rows and the state stay; a paused engine still tracks the 60 s so that un-pausing raises no burst
- A wall-clock jump must not fire or delay the 60 s: the timer is the engine's monotonic first-seen time per (session, `your_turn`), as `POPUP_GAP` is monotonic; a test jumps the wall clock and sees no change
- The timer is cleared when the state ends: a test flips the state at 30 s, lets 60 s pass, and sees no late popup
- A click for a session that left the list: the Live view opens, nothing is selected (the page does this; the app sends the id as it is)
- Two windows or a reconnecting stream: each stream sends a request once; a re-send inside 10 s after a reconnect is allowed and harmless
- The tray lock, the autostart file and `Quit` are unchanged

## Files to Change (Predicted)
`rust/crates/app/src/tray.rs`, `rust/crates/app/src/alerts.rs`, `rust/crates/app/src/window.rs`, `rust/crates/app/src/live_feed.rs`, `rust/crates/app/src/server.rs`, `rust/crates/app/src/main.rs` (only if the slot must be passed), `rust/crates/app/tests/live.rs`, `rust/crates/app/tests/app.rs`, `rust/README.md` (the tray part: rows, click, the 60 s), `tasks/TASK_REVIEW_T096.md`, `tasks/evidence-T096/**`

## Files Must NOT Touch
`rust/crates/core/**` (T094 owns the state; if it lacks something, stop and report), `src/easy_personal/**`, `tests/**`, `web/**`, `rust/parity/golden*/**`, `rust/Cargo.toml`, `rust/Cargo.lock`, `rust/crates/app/Cargo.toml` (except a `zbus` feature approved through `Status: blocked`), `rust/crates/app/src/keychain.rs`, `PRD.md`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `memory/**`, `.claude/**`, `docs/**`

## Test Plan
AC 1-9: unit tests in the app crate, output pasted. AC 10: the leak gate's last lines. AC 11: the commands' last lines. AC 12: the real run's rows and screenshots, or the plain statement that no display was available with the commands for the reviewer.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T096.md` (Evidence and Demonstration)
- [ ] Commits on branch `T096` only; final report with `Status:`

## Questions Log
- 2026-11-16 DEBATE When is the *your turn* popup raised: at once, or after a wait, and how long? → after the state has held 60 s on the engine's monotonic clock (Advocate 1: hold 60 s, monotonic first-seen time; Advocate 2: hold 60 s, monotonic first-seen time) ⚠ decided for you — please review
