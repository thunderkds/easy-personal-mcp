# BRAINSTORMING_LOG.md
**Generated**: 2026-11-15
**Task / Context**: Owner request "in the tray, any notif should be shown" (Rust app `tray` command, after T076/T084/T087)
**Skill**: `Skill({ skill: "brainstorming" })` — tier: Standard

---

## The Problem Space

Verified state (rust/crates/app/src):
- `tray.rs::view` builds the owner-approved 10-row menu (TASK_REVIEW_T076 `## Menu proposal`). It shows counts and
  measures (`1 waiting for you · easy-memory`, limits, machine line). It never shows an alert's text.
- `alerts.rs::Shared` keeps the last `KEEP` = 50 alerts in memory (`GET /api/alerts`, newest first) and the pause flag.
  The `tray` command starts the engine with alerts on (`window.rs:363`, `start(static_dir, true)`).
- Popups: `DBusNotifier` (`org.freedesktop.Notifications.Notify`). At most one per `POPUP_GAP` = 10 s; alerts raised in
  the gap are merged into the next popup ("3 sessions waiting"). While paused, no popup, and what waited is dropped.
- The menu is refreshed every 5 s look; `same_shape` updates texts in place, else the menu is rebuilt (`window.rs:522`).
- GNOME AppIndicator: native menu, text only, no tooltip.

So today an alert can be missed: the popup vanished, was merged into a summary, or was dropped while paused. The
only place that still has it is `GET /api/alerts`, which needs the dashboard.

Non-negotiable: NFR-002 (alert text = fixed keys + cleaned pieces only; already true of `Alert.text`), local only,
nothing written to disk.

---

## Questions for the User

1. Is the goal "I can look back at what I missed" (a history in the tray) or "I must see each one the moment it
   happens" (a popup for each)?
2. Should the menu show alerts that were raised while "Pause alerts" was on? (Recommended: yes — pause is about popups.)
3. Should seen alerts clear (an unread count), or is "the newest N" enough?

---

## Alternative Paths

| Option | Name | Summary | Invasiveness | Code Volume | Regression Risk | Recommended? |
|--------|------|---------|-------------|------------|----------------|--------------|
| A | Recent alerts in the menu | New menu block: newest 5 alerts, `HH:MM text`; `More in dashboard…` row when > 5 | Low | ~60 lines + tests | Low | ✅ Yes |
| B | One popup per alert | Drop `POPUP_GAP` and the summary merge; every alert is its own popup | Low | ~-30 lines, tests rewritten | Medium (popup storms) | |
| C | Unread inbox | A + unread count on row 1 (`Easy Personal · attention · 3 new`), a "Mark all read" action, icon badge state | Medium | ~150 lines | Medium | |

### Option A — Recent alerts in the menu
**Approach**: `Shared` gets `recent(n) -> Vec<Alert>` (newest first). `tray::view` takes it and adds, after the machine
line: separator, `Recent alerts` (info), then up to 5 info rows `14:02 · waiting for you · easy-memory (a1b2c3d4)`,
or one row `No alerts yet`. If the list has more than 5: an action row `All alerts in dashboard…` (opens the window,
like `OPEN`). Local time for `HH:MM`. Popups unchanged.
**Pros**: Nothing missed, even while paused or merged. Reuses the 50-alert list, which is already cleaned (NFR-002).
No new state. Fits the text-only menu.
**Cons**: The rows change shape when the first alert comes → one menu rebuild (already supported). A menu is passive:
you see it only when you open it.
**Why it might fail**: GNOME may truncate long rows (project names) — cap the text width. Times in the menu go stale
between 5 s looks (fine). The list is in memory: a tray restart empties it (same as the API today).

### Option B — One popup per alert
**Approach**: Remove the gap and the merge in `Engine` (alerts.rs ~200-225).
**Pros**: Immediate; less code.
**Cons**: Reverses a T074 decision made to stop bursts (10 sessions finishing → 10 popups). Still lost while paused,
and a popup is still gone once it fades. Does not put anything "in the tray".
**Why it might fail**: GNOME stacks or drops bursts itself; noisy users will turn alerts off entirely.

### Option C — Unread inbox
**Approach**: A plus a `last_seen` mark in `Shared`; row 1 and the icon show `N new`; opening/acting clears it.
**Pros**: Closest to a phone-style notification centre.
**Cons**: AppIndicator gives no "menu opened" event on GNOME, so "seen" needs an explicit "Mark all read" click.
Changes the approved row 1 and the icon state rule (T076 owner answers). More state, more tests.
**Why it might fail**: The unread count is wrong after a restart, and the extra click becomes chore.

---

## 50% Rule Check

Option A at half size: no `More…` row and no `No alerts yet` row — show the block only when there is at least one
alert, newest 3. ~30 lines. Recommended to keep the `No alerts yet` row out too, so a calm machine keeps today's menu.

---

## Recommended Path

**Option A**, with the 50 % trim: a `Recent alerts` block (newest 5) shown only when the list is not empty, shown
also while paused, popups untouched. Option C can come later on top of A if the owner wants an unread count.

---

## Surgical Scope

Touch: `rust/crates/app/src/alerts.rs` (`Shared::recent`), `rust/crates/app/src/tray.rs` (`view`, `Inputs::view`,
tests), `rust/crates/app/src/window.rs` only if a new action id needs a handler (the `More…` row reuses `OPEN`),
`rust/README.md` tray section. Must not touch: the alert engine rules, `POPUP_GAP`, `DBusNotifier`, `/api/alerts`,
the Python app, the store.

---

## Edge Case Checklist for TASK_GUIDE

- 0 alerts: menu equals today's approved menu (no block).
- Exactly 5 / 6 / 50 alerts: 5 rows; the more row only at 6+.
- Paused: alerts still listed in the menu.
- First alert arrives: shape change → rebuild, not a crash or a stale menu (`same_shape` false).
- Long project name / account label: row width capped; text already `clean`ed (no markup, no secrets).
- Clock: `t` is UTC seconds; menu shows local `HH:MM`; a day-old alert shows the date (`11-14 23:50`).
- `app` and `tray` both running: each has its own engine and list (window.rs:146 / :363) — check whether this already
  gives double popups; out of scope to fix here, but record it.

---

## Next Actions

1. Owner picks a path (and answers Q2/Q3 if A or C).
2. Stage 2: write `tasks/TASK_GUIDE_T088.md` (C1, Risk Low, P1, UI → Gate 6 rows), board row.
3. Stage 3: backend-developer, test-first in `tray.rs` tests; Stage 4 code-review; owner visual check on GNOME.

---

## User Selection

2026-11-15, owner (AskUserQuestion): **Option A — Recent alerts in the menu**, as in the preview: block after the
machine line, `Recent alerts` header, newest 5 rows `HH:MM · <text>`, `All alerts in dashboard…` action only when
more than 5. Popups unchanged. Q2 (listed while paused) and Q3 (no unread count) take the recommended defaults.
Open for Stage 2: whether the block shows with 0 alerts (log recommends: hidden).
