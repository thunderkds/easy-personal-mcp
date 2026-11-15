# TASK_REVIEW — T076: Watch 5: `tray` command

> Sibling of `tasks/TASK_GUIDE_T076.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` tag,
`docs/claude-md/tracks.md` `## Auto mode`). To use it, copy the block below out of this comment, to this
place, and replace every <...>. Keep `Mode: auto` as its first line, exactly as written: T182's hook reads
it. Write it to this file before the run ends; the chat message is a copy, not the report. The same
block is the Supervisor's final chat message. It is not the `delivery-report` page and not
Evidence. A taste decision (what the user wants) also ends with the flag. Write `DEBATED` only when ≥2
Advocate `Agent` calls ran for that question, naming each Advocate's option. A decision with no Advocates is
labelled `RULE`, `FACT` or `SAFER OPTION`, never `DEBATED`. A decision flagged `⚠ decided for you` is a choice, and is never labelled `FACT`. A
safety action a rule requires (for example the backup before a delete) is `RULE` without the flag.
At Risk Medium or High, a `RULE:` counts only when it names a written rule (`path:line`,
`path § <heading>`, or `docs/claude-md/tracks.md` `### <section>`), and a `FACT:` needs a Citation
(`path:line "quoted text"`); without one it is a choice and must be `DEBATED`. Good: `2. backed up the CSV
— RULE: `docs/claude-md/tracks.md` `### Stops that stay``. Bad: `1. kept recurring.csv — RULE: Surgical Changes`.

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its option>; Advocate 2: <its option>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours)>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
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

**BEFORE**: default build of branch `T076` at 892aa99 (no implementation commit yet), temp `XDG_CONFIG_HOME`; captured by backend-developer:

```text
2026-11-15T08:08:06Z
892aa99 Board: T063 note: re-point connector entries at the Rust launcher before removing Python (from T062)
$ easy-personal-rs tray
error: unrecognized subcommand 'tray'

Usage: easy-personal-rs <COMMAND>

For more information, try '--help'.
exit 2
$ easy-personal-rs tray --autostart on
error: unrecognized subcommand 'tray'

Usage: easy-personal-rs <COMMAND>

For more information, try '--help'.
exit 2
$ ls $XDG_CONFIG_HOME/autostart
ls: cannot access '/tmp/tmp.qUaeGVdD0E/autostart': No such file or directory
```

**AFTER**: same binary, same temp `XDG_CONFIG_HOME` pattern, branch `T076` (default build answers exit 4 for `tray`; the window build below). Captured by backend-developer:

```text
2026-11-15T08:33:09Z
$ tray --autostart on --static-dir web
autostart on: wrote $SCRATCH/tmp.bRiWLzX5sm/autostart/easy-personal-tray.desktop
exit 0
$ find $XDG_CONFIG_HOME
$XDG_CONFIG_HOME
$XDG_CONFIG_HOME/autostart
$XDG_CONFIG_HOME/autostart/easy-personal-tray.desktop
$ cat the entry
[Desktop Entry]
Type=Application
Name=Easy Personal tray
Comment=Tray icon and desktop alerts of easy-personal
Exec="/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/T076/rust/target/debug/easy-personal-rs" tray --static-dir "$SCRATCH/web"
Terminal=false
NoDisplay=true
X-GNOME-Autostart-enabled=true
$ tray --autostart off
autostart off: removed $SCRATCH/tmp.bRiWLzX5sm/autostart/easy-personal-tray.desktop
exit 0
$XDG_CONFIG_HOME
$XDG_CONFIG_HOME/autostart
$ tray --autostart off (again)
autostart off: $SCRATCH/tmp.bRiWLzX5sm/autostart/easy-personal-tray.desktop was not there
exit 0
$ tray --autostart on (no --static-dir)
easy-personal-rs: tray --autostart on needs --static-dir <DIR>
exit 2
$ tray with no display
easy-personal-rs: no display: neither WAYLAND_DISPLAY nor DISPLAY is set (serve, mcp and store-info need no display)
exit 5
```

**DELTA**: the user can run `easy-personal-rs tray --static-dir <DIR>` to get one tray process (live tail, samplers, alerts on) with a state icon and a native menu, and `tray --autostart on|off` to start it at login; before, `tray` was an unknown subcommand (exit 2).

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T076.jsonl`, never the
implementing agent alone]

---

## Menu proposal

Native Linux menu (AppIndicator via Tauri `tray-icon`), rebuilt on each 5 s alert-engine look. Source: `docs/design/graphite-mint/TrayPopover.dc.html`, reduced to the rows named in the guide's Restated intent 2. Example values are the canvas's `attention` sample. "info" = disabled row (text only); "action" = clickable.

| # | Row (exact label) | Kind | Shown when | Example / other forms |
|---|---|---|---|---|
| 1 | `Easy Personal · <state>` | info | always | `Easy Personal · attention` (state = `idle` / `active` / `attention` / `problem`) |
| — | separator | | | |
| 2 | live line | info | always | `2 sessions active`, `1 session active`, `No sessions running` |
| 3 | `<n> waiting for you · <project>` | action: opens the dashboard (the canvas's "Open" button) | at least 1 session waiting | `1 waiting for you · easy-memory`; several projects: `3 waiting for you · easy-memory +2` |
| 4 | `<n> failing · <project>` | action: opens the dashboard | at least 1 session failing | `1 failing · easy-personal-mcp` (NOT in the canvas; explains a `problem` state. Owner choice: keep or drop) |
| 5 | `5h limit · <p> % · 7d · <p> %` | info | always; one row per Claude account when there are several, prefixed `<label>: ` | `5h limit · 78 % · 7d · 41 %`; missing/stale window: `5h limit · no data · 7d · 41 %` |
| 6 | machine line | info | always | `CPU 12 % · RAM 64 % · disk 71 % · battery 80 %`; no battery: the battery part is left out (never 0 %); no sample: `Machine · no data` |
| — | separator | | | |
| 7 | `Open dashboard` | action: opens the existing dashboard window (`app` window), focuses it if already open | always | |
| 8 | `Pause alerts` | check item (action): toggles the same in-memory flag as `POST /api/alerts/pause` | always | ticked while paused |
| — | separator | | | |
| 9 | `127.0.0.1 · local only` | info | always | canvas footer |
| 10 | `Quit` | action: stops the server, the feed, the samplers and the alerts, removes the icon, exit 0 | always | |

Differences from the canvas (proposed cuts; owner to confirm):
- No ingest row and no "Run ingest now": not in the Restated intent 2 (the tray does not run ingest).
- No "resets in 2h 10m": the plan-limit sampler (`LimitPercents`) carries no reset time; adding it is extra reading.
- "Pause alerts", not "Pause alerts for 1 hour": T074's pause is a plain flag with no timer (Restated intent 2 says toggle).
- No colours, dots or progress bars: a native AppIndicator menu shows text only. The state colour is on the icon.
- State rule follows the Restated intent 1 (attention = waiting or a limit >= 90 %; problem = a failing or machine-pressure alert active), not the canvas's 70 % / ingest-failed wording.
- Tooltip `Easy Personal · <live line>` is set, but AppIndicator does not show tooltips on GNOME.


### Owner answers (2026-11-15, relayed by the Supervisor via AskUserQuestion)

1. Menu rows: "Approve as listed". All 10 rows are approved; row 4 (`<n> failing · <project>`) is KEPT. The listed differences from the canvas are accepted (no ingest row and no "Run ingest now", no reset time, "Pause alerts" with no timer, text only).
2. State rule: "Approved plan". Restated intent 1 wins over the canvas: attention = a session waiting or a plan limit >= 90 %; problem = a failing session or a machine-pressure alert.

---

## Implementer evidence (backend-developer, 2026-11-15)

Tests were written in the same step as `tray.rs` (no separate red run was captured; the first run passed).

**AC 1 (state pick), AC 2 (autostart), AC 3 (single instance, no tray area)**: `rust/crates/app/src/tray.rs` `mod tests` (13 tests). Window build, `cargo test --features window -p easy-personal-rs --bin easy-personal-rs -- window:: tray::`:

```text
test tray::tests::an_entry_refuses_a_path_it_cannot_carry ... ok
test tray::tests::autostart_makes_the_folder_and_refuses_a_symlink ... ok
test tray::tests::autostart_path_follows_the_xdg_rules ... ok
test tray::tests::no_secret_or_markup_reaches_a_row ... ok
test tray::tests::autostart_writes_and_removes_only_its_file ... ok
test tray::tests::rows_say_no_data_and_never_zero ... ok
test tray::tests::several_projects_and_accounts_and_failing_rows ... ok
test tray::tests::the_lock_lets_one_tray_run ... ok
test tray::tests::the_menu_has_the_approved_rows_in_order ... ok
test tray::tests::the_state_is_the_most_urgent_input ... ok
test tray::tests::the_theme_is_light_only_when_the_desktop_says_light ... ok
test tray::tests::the_lock_falls_back_beside_the_store ... ok
test window::tests::a_blocked_url_is_logged_by_host_only ... ok
test window::tests::the_compiled_capability_set_is_empty ... ok
test window::tests::everything_else_is_blocked ... ok
test window::tests::no_capability_file_exists_and_no_tauri_feature_adds_power ... ok
test window::tests::only_the_servers_own_origin_is_allowed ... ok
test window::tests::the_tauri_config_gives_the_page_no_power ... ok
test tray::tests::no_bus_means_no_tray_area_and_the_dark_theme ... ok
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 153 filtered out; finished in 0.01s
```

**AC 4, verification command** (`cd rust && cargo build --features window 2>&1 | tail -2 && cargo test --workspace 2>&1 | tail -5`, 2026-11-15T08:41:18Z): the build finished (`Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 29.40s`). The test run stopped at `memgraph_parity::two_hundred_thousand_links_are_counted_in_time ... FAILED`. That is a timing test in `crates/core` that this task does not touch. It passed alone at 2026-11-15T08:47:44Z (`test result: ok. 1 passed`). Then `cargo test --workspace --no-fail-fast`: `suite exit 0`, 91 test binaries, **984 passed, 0 failed, 28 ignored**. `cargo clippy --all-targets` (default) and `cargo clippy --all-targets --features window -p easy-personal-rs`: no warning. `cargo fmt --all` applied.

**AC 4, real run on this desktop (Ubuntu GNOME 50.1, Wayland)**:
- Real session bus: `ubuntu-appindicators@ubuntu.com` is installed but `Enabled: No`, `State: INACTIVE`, and `NameHasOwner org.kde.StatusNotifierWatcher` = `(false,)`. The tray printed `easy-personal-rs: no tray area (no StatusNotifier host): the tray icon is not shown; alerts keep working` once, kept running, and stopped on SIGTERM. Restated intent 3 / the "GNOME without AppIndicator" edge case was observed live. A second `tray` printed `easy-personal-rs: a tray already runs for this user` and exited 0.
- The panel screenshot of the icon and the open menu was **not taken**. This desktop has no tray area, and enabling the extension changes desktop config, which the guide forbids. Owner step: `gnome-extensions enable ubuntu-appindicators@ubuntu.com`, then `rust/target/debug/easy-personal-rs tray --static-dir <built web/dist>` (window build), click the icon, and take the screenshot.
- Instead, on a private test bus (`dbus-daemon --session`) with a stand-in StatusNotifierWatcher, the tray registered `/org/ayatana/NotificationItem/tray_icon_tray_app_easy_personal`. `EASY_PERSONAL_HOME` pointed at an empty temp folder, so the state was idle. The exported menu (`com.canonical.dbusmenu.GetLayout`) read:

```text
2 'label': <'Easy Personal · idle'>, 'enabled': <false>
3 'enabled': <true>, 'type': <'separator'>
4 'label': <'No sessions running'>, 'enabled': <false>
5 'label': <'5h limit · no data · 7d · no data'>, 'enabled': <false>
6 'label': <'CPU no data · RAM 28 % · disk 20 %'>, 'enabled': <false>
7 'enabled': <true>, 'type': <'separator'>
8 'label': <'Open dashboard'>, 'enabled': <true>
9 'label': <'Pause alerts'>, 'enabled': <true>, 'toggle-type': <'checkmark'>, 'toggle-state': <0>
10 'enabled': <true>, 'type': <'separator'>
11 'label': <'127.0.0.1 · local only'>, 'enabled': <false>
12 'label': <'Quit'>, 'enabled': <true>
```

- The icon file the item served (`IconName` = `/run/user/1000/tray-icon/tray-icon-easy-personal-0.png`, 32x32) has `magick compare -metric AE` = `0` differing pixels against `icons/tray/dark/idle-32.png`. That is the T071 icon (dark theme: the portal is unreachable on the test bus).
- Clicking `Pause alerts` (dbusmenu `Event clicked`) made the next rebuilt menu show `'Pause alerts'`, `'toggle-state': <1>`.
- `Open dashboard` opened one window `easy-personal` (X11 backend, `xwininfo` count 0 -> 2 frame+client). A second click made no second window (count stays 2). After Wnck closed the window, the tray kept running, and `Open dashboard` opened it again. `Quit` ended the process with `exit 0`, and the icon temp file was removed. SIGTERM printed `stopping on a signal` and exited 0. With no display, `tray` exited 5 (output in AFTER).

**Leak gate**: no new `/api` route, so nothing was registered in `rust/parity/leak_rust.py`. The pause toggle sets the same in-memory flag as `POST /api/alerts/pause`.

**Dependency rule**: no new crate. Feature `tauri/tray-icon` was added to the `window` feature. `tray-icon` 0.25.1, `libappindicator` 0.9.0 and `muda` 0.20.0 were already in `Cargo.lock`, which is unchanged. At run time it loads `libayatana-appindicator3.so.1`, which is installed. The T023 test `no_capability_file_exists_and_no_tauri_feature_adds_power` now expects this one feature (`tray-icon` left its forbidden list). `the_compiled_capability_set_is_empty` still passes.

**Cut list**:
- The icon follows the current values with no 85 % re-arm band (the popups keep theirs).
- The theme is read once at start.
- No ingest at tray start (`app` ingests by default).
- The waiting/failing rows open the dashboard, not the Live view directly.
- One icon size (32 px; the panel scales it).
- `--autostart on` refuses paths holding `" ` $ \ %` or control characters instead of escaping them.
- Non-Linux trays (out of scope).

