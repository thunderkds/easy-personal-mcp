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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/src/tray.rs` tests (state pick boundaries, menu rows, secret/markup filter, autostart on/off on temp XDG_CONFIG_HOME, symlink refusals, lock, icon folder never /tmp, same_shape); window build `tray`+`window` tests 24 passed (Supervisor run) |
| Verification command run | ☑ pass | `cargo test --workspace --no-fail-fast` 2026-11-15T09:33:59Z: 987 passed, 0 failed, 28 ignored (agent); clippy clean default and `--features window` |
| Negative cases hold | ☑ pass | second `tray` prints `a tray already runs for this user`, exit 0; no display exit 5; symlinked lock / autostart target refused; XDG_RUNTIME_DIR unset → icon in `<store>/tray-icon`, `/tmp/tray-icon` absent |
| verify | ☑ pass | Supervisor `/verify` 2026-11-15: first run ~08:55Z (routes 200/403, pause 200/400/403/415, mcp 13 tools, ingest twice exit 0, window `self-test: ok`, tray `--autostart off` on temp XDG ok) found leak gate `5 failed checks` + `tray` unregistered → fix round `ad5eeee`; re-run ~09:50Z at load 12.9: leak gate `0 hits over 42 canaries, 818 surfaces, 1800525 bytes`, 0 FAILED lines, exit 0; `app --self-test` → `self-test: ok`; tray+window tests 24 passed; merge-tree vs main clean — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | merge-base..T076 (tray.rs, window.rs, server.rs, alerts.rs, main.rs, Cargo.toml, icons README, leak_rust.py) plus callers. code-review: 0 P0/P1; P2 icon+menu rewritten each tick → fixed; P2 tray host checked only at start → fixed; P3 tooltip no-op on Linux → removed (cut list); P3 `signalled` kept. security-review: 0 P0/P1; P2 icon PNG fallback to /tmp (symlink clobber) → fixed via `temp_dir_path` + `tray::icon_dir`; P3 lock hardening → symlink refusal added. lane_check `C3, 1326 lines, FITS` |
| Full smoke suite still green (no regression) | ☑ pass | workspace 987 passed 0 failed (agent, fix round); leak gate clean (Supervisor) |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier MCP not connected this session (ECONNREFUSED); advisory only |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Owner verdict 2026-11-15 after the cutover tray ran on the owner's panel: "76 is good to mark done". Earlier note (owner step): real-panel screenshot of icon + open menu needs the AppIndicator extension, which the agent may not enable. Interim evidence: private-bus `GetLayout` = the 10 approved rows; served icon = T071 `dark/idle-32.png` (0 px diff) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | icons are T071's generated tray PNGs (Graphite Mint); menu is native text only (owner-approved: no colours) |
| **UI: Responsiveness at target viewports** | ☑ N/A | native tray menu, no viewport |

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

**WITNESS**: Supervisor (not the implementing agent) ran `/verify` twice on 2026-11-15 (~08:55Z, ~09:50Z after fix round 1); active_task was set to T076 before both. [template: who ran it and when — derived from `memory/event-trace/T076.jsonl`, never the
implementing agent alone]

---

## Menu proposal

Native Linux menu (AppIndicator via Tauri `tray-icon`), refreshed on each 5 s look (in place since the fix round; rebuilt only when rows come or go). Source: `docs/design/graphite-mint/TrayPopover.dc.html`, reduced to the rows named in the guide's Restated intent 2. Example values are the canvas's `attention` sample. "info" = disabled row (text only); "action" = clickable.

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

- The icon file the item served (first round: `IconName` = `/run/user/1000/tray-icon/tray-icon-easy-personal-0.png`, the library's default folder; since the fix round the tray names the folder itself, see below; 32x32) has `magick compare -metric AE` = `0` differing pixels against `icons/tray/dark/idle-32.png`. That is the T071 icon (dark theme: the portal is unreachable on the test bus).
- Clicking `Pause alerts` (dbusmenu `Event clicked`) made the next rebuilt menu show `'Pause alerts'`, `'toggle-state': <1>`.
- `Open dashboard` opened one window `easy-personal` (X11 backend, `xwininfo` count 0 -> 2 frame+client). A second click made no second window (count stays 2). After Wnck closed the window, the tray kept running, and `Open dashboard` opened it again. `Quit` ended the process with `exit 0`, and the icon temp file was removed. SIGTERM printed `stopping on a signal` and exited 0. With no display, `tray` exited 5 (output in AFTER).

**Leak gate**: no new `/api` route. The `tray` CLI command is registered in `rust/parity/leak_rust.py` `CLI_ELSEWHERE` (fix round; window-only like `app`). The pause toggle sets the same in-memory flag as `POST /api/alerts/pause`.

**Dependency rule**: no new crate. Feature `tauri/tray-icon` was added to the `window` feature. `tray-icon` 0.25.1, `libappindicator` 0.9.0 and `muda` 0.20.0 were already in `Cargo.lock`, which is unchanged. At run time it loads `libayatana-appindicator3.so.1`, which is installed. The T023 test `no_capability_file_exists_and_no_tauri_feature_adds_power` now expects this one feature (`tray-icon` left its forbidden list). `the_compiled_capability_set_is_empty` still passes.

**Cut list**:
- The icon follows the current values with no 85 % re-arm band (the popups keep theirs).
- The theme is read once at start.
- No tooltip: AppIndicator has none on Linux (`tray-icon` `set_tooltip` is a no-op there), so the tray sets none.
- No ingest at tray start (`app` ingests by default).
- The waiting/failing rows open the dashboard, not the Live view directly.
- One icon size (32 px; the panel scales it).
- `--autostart on` refuses paths holding `" ` $ \ %` or control characters instead of escaping them.
- Non-Linux trays (out of scope).

---

## Fix round 1 (Stage 4 + /verify findings), backend-developer, 2026-11-15

| # | Finding | Fix | Evidence |
|---|---|---|---|
| 1 | Security P2: `tray-icon` writes the icon PNG with `File::create` (follows symlinks, predictable name) into `dirs::runtime_dir().unwrap_or_else(std::env::temp_dir).join("tray-icon")`. With `XDG_RUNTIME_DIR` unset that is `/tmp/tray-icon`, so another local user could plant a symlink. | `tray::icon_dir` uses the lock's folder rule (`private_dir`): `$XDG_RUNTIME_DIR/tray-icon` when absolute, else `<store folder>/tray-icon` (`~/.easy-personal/tray-icon` or `EASY_PERSONAL_HOME`). It never falls back to `/tmp`, and with no folder it is an error. `TrayIconBuilder::temp_dir_path(icon_dir)`. The lock also refuses a symlink (P3). | Tests `the_icon_folder_is_private_and_never_tmp` and `a_symlinked_lock_is_refused`. Real run with `env -u XDG_RUNTIME_DIR`: the item's `IconName` = `$SCRATCH/ephome2/tray-icon/tray-icon-easy-personal-0.png`, and `ls /tmp/tray-icon` gave `No such file or directory`. |
| 2a | Leak gate: `check cli commands all registered FAILED (22 commands; unregistered ['tray'])` | `tray` added to `CLI_ELSEWHERE` (window-only like `app`) | `check cli commands all registered   ok (22 commands)` |
| 2b | Leak gate: `store-check made no file beside the store ... FAILED` | Not caused by T076 code; it is load-dependent. A debug print showed the extra files were `store.sqlite-shm`, `store.sqlite-wal`, left by the gate's `serve` stop. The failing labels changed between runs: first healthy, text past the cut, blob source, wal without shm; then healthy, text past the cut, blob source, blob time. Load average at those runs was about 30 (my full test suites and others were running). The same T076 binary then passed twice under lower load (09:06 and 09:07Z, `0 hits`, no FAILED). The `serve` code path is unchanged apart from one extra `Arc` field in `Running`. | Final gate below |
| 3 | Code-review P2: `set_icon` and `set_menu` ran on every view change | `watch_tray` sets the icon only when the state changes. When `tray::same_shape` holds (same row kinds and ids), it calls `set_text` / `set_checked` on the kept item handles. It rebuilds the menu only when a row comes or goes. | Test `the_shape_decides_update_or_rebuild`. Real run: after `Pause alerts` was clicked, the dbusmenu item ids stayed 2..12 (`same item ids: updated in place, not rebuilt`) with `toggle-state <1>`. After 70 s, row 6 changed in place to `CPU 14 % · RAM 29 % · disk 21 %`, and the icon file stayed `-0.png` (not rewritten). |
| 4 | Code-review P2: a host that appears later never got the icon | The icon is always built. The "no tray area" line is only a warning. | Real run: the tray started on a private bus with no watcher and printed the warning. The stand-in watcher started 6 s later, and the log shows `register sender=:1.2 arg=/org/ayatana/NotificationItem/tray_icon_tray_app_easy_personal`. |
| P3 | Tooltip is a no-op on Linux | `View.tooltip` and the tooltip calls were removed; recorded in the cut list | — |
| P3 | `signalled` param | Kept: `watch_signals` uses it to tell a signal stop from Quit (same as `app`) | — |
| new | The first version of fix 3 kept a `TrayIcon` clone in the refresh thread, so at Quit the icon file stayed in the folder | The thread now looks the icon up each tick (`tray_by_id`) and holds it only for that tick | Real run: icon folder while running `tray-icon-easy-personal-0.png`; after Quit `exit 0` and the folder is empty (`[]`) |

**Tests**:
- Window build: `cargo test --features window -p easy-personal-rs --bin easy-personal-rs -- window:: tray::` gave `test result: ok. 22 passed; 0 failed`.
- 2026-11-15T09:33:59Z: `cargo test --workspace --no-fail-fast` gave `suite exit 0`, 91 binaries, **987 passed, 0 failed, 28 ignored**.
- `cargo clippy --all-targets` (default build and `--features window`): no warning.

**Leak gate, final** (2026-11-15T09:43:25Z, after `cargo build`, with no tray, watcher, bus or cargo of mine running; command `env -u CLAUDE_CONFIG_DIR .../parity/.venv/bin/python parity/leak_rust.py`):

```text
check store-check made no file beside the store (healthy)  ok (4 files)
check cli commands all registered   ok (22 commands)
leak check: 0 hits over 42 canaries, 818 surfaces, 1800313 bytes
exit=0
```
85 `check ... ok` lines, no `FAILED` line.
