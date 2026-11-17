# TASK_REVIEW — T096: Waiting sessions 3 — tray rows per session, the *your turn* alert, a popup you can click, and the focus request

> Sibling of `tasks/TASK_GUIDE_T096.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

> **Not started.** It waits for T095's open question (the click mechanism). The block below becomes the report when the task starts.

<!-- held until the task starts (a task that has not started is not marked as an auto-mode run)
## Auto decisions

Mode: auto
1. The *your turn* popup waits until the state has held 60 s on the engine's monotonic clock — DEBATED: hold 60 s, monotonic first-seen time (Advocate 1: hold 60 s, monotonic first-seen time; Advocate 2: hold 60 s, monotonic first-seen time) ⚠ decided for you — please review
2. At most 5 session rows, as the menu's alert block shows 5 — FACT: rust/crates/app/src/tray.rs:220 "pub(crate) const ALERTS_SHOWN: usize = 5;"
Not done: merge (admin: after the auto verify pass); auto verify: not started (the task waits for T095's answer)
-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/src/tray.rs` `tray::tests` (AC 1-3, 9: `a_row_names_its_kind_the_project_and_the_short_id`, `five_session_rows_at_most_and_a_row_for_the_rest`, `a_session_with_several_states_gets_one_row_by_its_first_state`, `a_row_leaves_out_a_piece_it_does_not_have`, `a_your_turn_session_is_attention_and_failing_stays_more_urgent`, `a_menu_item_id_carries_the_session`, `the_session_rows_decide_update_or_rebuild`, `the_address_names_the_session_and_keeps_the_token_in_the_fragment`, `an_id_that_is_not_a_session_id_is_left_out`, `the_token_is_only_ever_in_the_fragment`, `a_row_and_a_popup_give_the_same_address`); `rust/crates/app/src/alerts.rs` `alerts::tests` (AC 4-6: `your_turn_alerts_once_after_sixty_seconds`, `a_state_that_ends_before_sixty_seconds_raises_nothing`, `a_wall_clock_jump_changes_nothing_about_the_sixty_seconds`, `paused_tracks_the_sixty_seconds_and_unpausing_shows_nothing`, `a_summed_popup_counts_your_turn_in_fixed_words`, `an_app_start_with_several_your_turn_sessions_shows_one_popup_after_sixty_seconds`, `no_question_or_markup_reaches_a_your_turn_alert`); `rust/crates/app/src/notify_tests.rs` `alerts::notify_tests` (AC 7-8: `a_popup_has_the_default_action_and_its_click_is_passed_on`, `a_service_without_actions_gets_a_popup_with_none`, `a_click_is_only_for_a_popup_this_process_sent`, `without_a_click_channel_no_action_is_offered`, `a_service_that_never_answers_is_bounded`, `a_service_that_drops_mid_wait_fails_soft`, `no_bus_with_a_click_channel_does_not_panic`, `a_hung_service_holds_neither_the_start_nor_the_stop`); `rust/crates/app/src/window.rs` (`window::tests::the_address_of_a_click_is_allowed`, AC 9 `navigation_allowed`). Names read from `git diff f5f839f..HEAD`; the run: `tray:: alerts:: live_feed::` `test result: ok. 90 passed; 0 failed`. |
| Verification command run | ☑ pass | Run as written (2026-11-17, commit `2c066cb`, README files uncommitted): `test result: ok. 90 passed; 0 failed; 0 ignored; 0 measured; 139 filtered out; finished in 3.03s` (`tray:: alerts:: live_feed::`); `cargo build --features window`: `Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.40s`; `cargo test --workspace | tail -5` ends `every_connector_and_doctor_step_equals_the_python_cli` / `test result: FAILED. 1 passed; 1 failed; ... finished in 8.72s` / `error: test failed, to rerun pass `-p easy-personal-rs --test connector_cli_parity``. That last target is the known environmental failure (`/usr/bin/easy-personal-launch` of the installed `.deb` is on `PATH`; fails on `main` too, as in T094; branch T097 fixes it), not from T096. The full run with `--no-fail-fast` is in the smoke row. Leak gate (AC 10), `cargo build` first, `env -u CLAUDE_CONFIG_DIR rust/parity/.venv/bin/python parity/leak_rust.py`: last line `leak check: 0 hits over 42 canaries, 843 surfaces, 2015853 bytes`, `exit=0`. |
| Negative cases hold | ☑ pass | Two mutations by the Stage-3 closer (the first agent's red runs are not recorded, so none is claimed), each restored with `git checkout`. (1) `alerts.rs:37` `YOUR_TURN_HOLD` 60 s to 30 s (`git diff --stat`: `1 file changed, 1 insertion(+), 1 deletion(-)`): 4 tests fail, `your_turn_alerts_once_after_sixty_seconds ... FAILED`, `a_state_that_ends_before_sixty_seconds_raises_nothing ... FAILED`, `a_wall_clock_jump_changes_nothing_about_the_sixty_seconds ... FAILED`, `an_app_start_with_several_your_turn_sessions_shows_one_popup_after_sixty_seconds ... FAILED`. (2) `tray.rs:129` `is_session_id` without `\|\| b == b'-'` (diff: `-    ... \|\| b == b'-')` / `+    ... is_ascii_hexdigit())`, 1 insertion, 1 deletion): `test result: FAILED. 34 passed; 2 failed` (`a_row_and_a_popup_give_the_same_address`, `the_address_names_the_session_and_keeps_the_token_in_the_fragment`). Both files restored; `git status --short` shows no code change. |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | `cd rust && cargo test --workspace --no-fail-fast` (2026-11-17, commit `2c066cb`): 93 targets, 1064 passed, 1 failed, 28 ignored. The one failure is `every_connector_and_doctor_step_equals_the_python_cli` (`connector_cli_parity`, `crates/app/tests/connector_cli_parity.rs:250` panic): the known environmental failure (installed `/usr/bin/easy-personal-launch`; fails on `main` too; T097 fixes it). `memgraph_parity` passed. Every other target `test result: ok.` `GET /api/live` and `live_feed.rs` unchanged (`git diff --name-only f5f839f..HEAD` lists no `live_feed.rs`, no `core/`). The two `notify` test doubles in `api_tests.rs` and `server.rs` got the new third argument (2 lines; the `Notifier` trait changed). Format and lint by the first agent, not re-run here (no code changed by this round): `cargo fmt --check`, both clippy commands, exit 0. |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (2026-11-17T02:43:20Z, branch `T096` at `f5f839f`, no implementation commit yet): a throwaway test (not committed) printed `tray::view` for a snapshot with one `waiting` session (`shop`), one `your_turn` session (`blog`) and one `failing` session (`api`): one count row for waiting, one for failing, and no row at all for the your-turn session or for any single session. The alert engine has no `your_turn` kind and `Notifier::notify(summary, body)` has no action.

```
$ cd rust && cargo test -p easy-personal-rs --bin easy-personal-rs -- tray::tests::before_t096_print_rows --nocapture
running 1 test
state: problem
Info("Easy Personal · problem")
Separator
Info("3 sessions active")
Action("waiting", "1 waiting for you · shop")
Action("failing", "1 failing · api")
Info("5h limit · no data · 7d · no data")
Info("Machine · no data")
Separator
Action("open", "Open dashboard")
Check("pause", "Pause alerts", false)
Separator
Info("127.0.0.1 · local only")
Action("quit", "Quit")
test tray::tests::before_t096_print_rows ... ok
```

**AFTER** (real run of the window build `tray` by the first agent, 2026-11-17 ~09:59 local, on a temp home with planted transcripts, files `tasks/evidence-T096/menu-rows.txt`, `dbus-notify-trace.txt`, two screenshots; the Stage-3 closer read them and re-ran nothing of this run). The snapshot differs from BEFORE (one more session, `plan to approve`), and the rows are the tray's own dbusmenu dump:
```
 37 item off '4 sessions active'
 38 item on  'failing · api · deadbeef'
 39 item on  'your turn · docs · 89abcdef'
 40 item on  'plan to approve · blog · 4567ef01'
 41 item on  'question · shop · 0123abcd'
```
(`on` is a clickable row; the two count rows `N waiting for you` and `N failing` are gone; the other rows are as before.) The popup, from `dbus-monitor` on the session bus: `Notify` with `string "default"`, `string "Open"` as the actions and the body `Your turn, project docs, session 89abcdef`, then `ActionInvoked` `uint32 11`, `string "default"` back to the sender that answered `Notify`. A dbusmenu `Event` on a row opened the window on the Live view with that session expanded (`click-your-turn-row-navigates-open-window.png`, `click-question-row.png` shows the planted question text on the Live view only). No human mouse click: the click was sent as a dbusmenu `Event` and the popup click as a `dbus-monitor`-seen `ActionInvoked` from the service, so the real mouse click is for the owner's eyes (AC 12).

**DELTA**: The tray menu now lists one clickable row per waiting, your-turn or failing session instead of two count rows, the alert engine raises a *your turn* popup after 60 s, and a click on a row or a popup opens the window on the Live view with that session selected.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
