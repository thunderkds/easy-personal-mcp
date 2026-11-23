# TASK_REVIEW — T135: Terminal 3 — roomier terminal panel, Expand, and Pop out as part of the app

> Sibling of `tasks/TASK_GUIDE_T135.md`. Everything here is filled at Stage 4/5.

---

## Review findings

- Reviewer `Reviewer: T135` on `7f1e2ad`: PASS, 0 P0/P1; 1 P2 (no Chat-split test) + 5 P3, all fixed in `0e16ae4`.
- security-review on `7f1e2ad`: no finding at confidence >= 8 (URL gate, token in hash, opener, `?term=`, log lines checked).
- Reviewer on `e1d5098` (owner's UX change): PASS; P2 (Dock back closed the window with no Live listening) + 2 P3, fixed in `30dcd22`.
- Reviewer on `30dcd22`: PASS; 2 P3 (timer order, AC wording), fixed in `49676d7`.

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/TermPanel.test.jsx` (compact head, Pop out, blocked pop-up, app flag, popped, focus resize, forced resize kept, Expand: AC 2, 4, 6, 9), `web/src/TermWindow.test.jsx` (AC 5, 10, 11: bar, open/who/closed, Dock back answered and unanswered, channel carries no token or screen byte), `web/src/LiveTerm.test.jsx` (AC 1, 4, 11: full width, Chat keeps split, Pop out closes the panel, row mark, second window keeps it, Dock back answered only for a listed terminal), `rust/crates/app/src/window.rs` `only_a_terminal_pop_out_opens_a_window` (AC 7). AC 3 = screenshots below. |
| Verification command run | ☑ pass | `cd web && npx vitest run` — `Tests  1637 passed (1637)` (three runs in a row); `npm run build` ok; `python3 scripts/check_static.py` — `check_static: ok`; `cargo test -p easy-personal-rs --bin easy-personal-rs -- window::` — 14 passed; `cargo clippy -p easy-personal-rs --features window --all-targets -- -D warnings` — clean. |
| Negative cases hold | ☑ pass | Pop-up blocked → alert, panel stays; Dock back with no Live answer → window stays and says to open Live; Dock back for an unlisted id → no answer, no panel; malformed channel messages ignored; `is_term_popout` refuses 16 other URLs (other port, host, scheme, path, extra query, `user@`, over-long id). |
| verify | ☐ pass / ☐ fail | [the owner's `/verify`, or the auto verify run by the hook] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | The 13 changed files plus callers (`api.js` readToken, `window.rs` open paths, `term_routes.rs` auth/resize, `app.css` base rules). |
| Full smoke suite still green (no regression) | ☑ pass | Whole web suite 1637/1637; Rust app crate tests pass (479 + window 14). |
| **Companion score (easy-verifier)** | ☑ N/A | Not run. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Real Chrome against a real `serve` (temp store, stub `claude`): see Demonstration. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Existing tokens only (`--space-*`, `--color-*`, `--radius-sm`, `--fs-*`, `--header-height`); existing classes `session-title`, `session-state`, `live-pill`, `danger`, `field-error`. |
| **UI: Responsiveness at target viewports** | ☑ pass | 375x812: panel fits under the two-row top bar, keys 44 px in two rows; 1280x800 and 1440x900: full width, slim key row; popped 1100x720. |

---

## Demonstration

**BEFORE** (`main` @ `2ed3035`, page built to a scratch folder, real `serve` with a stub `claude`, Chrome 1280x800, 2026-11-23):

```
.term-box  width 361 px, top 668 px (below the 800 px fold), height 480 px
stub claude size: ~44 columns; panel = right 3/5 of Live beside the list, plus the right rail; three-row facts list above the screen
```

**AFTER** (branch `T135`, same server and stub):

```
in Live, 1280x800:    .term-box width 907 px, top ~140 px -> "stub claude · 114 cols × 33 rows"
Expand, 1280x800:     .term-box width 1240 px          -> "stub claude · 157 cols × 39 rows"
own window, 1100x720: .term-box width 1058 px          -> "stub claude · 133 cols × 35 rows"; bar "Easy Personal · Terminal — Closing this window does not stop the session." + Dock back
Live row with the pop-out open in a second tab: "in its own window" true; after that tab closed: false
Pop out without a user gesture (headless): "The browser blocked the new window. Allow pop-ups for this page, then try Pop out again."
```

**DELTA**: the terminal gets the whole Live width (2.5x the columns) and the window height; Expand gives it the whole app window; Pop out opens a window that names the app and docks back.

**WITNESS**: Supervisor session, real-browser runs above; the desktop-app window path (AC 7 at runtime) by the owner after installing the `.deb`.
