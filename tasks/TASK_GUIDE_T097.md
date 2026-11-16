# TASK_GUIDE — T097: app icon on notifications, window and dock; one real-machine smoke script
**Date**: 2026-11-16
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/crates/app/src/alerts.rs` (`send`, `DBusNotifier`), `rust/crates/app/src/window.rs` (`WebviewWindowBuilder` at about line 261, the tray `icon()`), `rust/packaging/easy-personal.desktop`, `rust/packaging/easy-personal-tray.desktop`, `rust/packaging/build-deb.sh`, `rust/packaging/test_deb.sh`, `rust/crates/app/tests/connector_cli_parity.rs`, `rust/crates/app/src/agent_run.rs` (`command`, `Action`)
- **Never run anything against the owner's real setup** except what AC 7 names: no `sudo`, no `dpkg -i`/`apt install`, no write under `/usr`, `~/.local`, `~/.config`, `~/.easy-personal`, `~/.claude*`; no `systemctl --user`, no `claude mcp add/remove`. **Never run the real `claude`** (`agent-run now` without a fake `claude` first on `PATH`): it spends the owner's plan limits; the Supervisor runs the one real run. Never run `real_keychain_items_cross_check` or any `#[ignore]` test. Do not kill any process you did not start (another session runs a `tray` from `rust/target/release`). Use your own `CARGO_TARGET_DIR` only if a build in the main checkout's `rust/target` is going.

---

## Requirement

**Original request** (owner, 2026-11-16, session tag `[admin]`): "let make sure the claude invoke work, the notification work, the latest build is fine. let do automate"; then "set the icon for the app also, let automate all".

**Findings** (Supervisor, 2026-11-16, on `main` `7f3aaef`, Ubuntu 26.04 GNOME on Wayland, the 0.3.0 `.deb` installed):
- Build `cargo build --workspace --features window`: exit 0. `cargo test --workspace --no-fail-fast`: 1037 passed, 1 failed: `connector_cli_parity` `every_connector_and_doctor_step_equals_the_python_cli`, because the installed `/usr/bin/easy-personal-launch` is found in the `*_no_launcher` cases (`doctor_no_launcher step 0 ["doctor"]: exit 0, Python 1; stdout "... ok launcher: /usr/bin/easy-personal-launch"`). The timing test `two_hundred_thousand_links_are_counted_in_time` failed once at load 12 and passed alone (known load flake, not in scope).
- Notification: a throw-away HOME with one session open on `AskUserQuestion`, `serve --no-ingest --alerts`, and `dbus-monitor` on the session bus saw `Notify("easy-personal", 0, "", "Easy Personal", "Waiting for you, AskUserQuestion, project tmp, session notify-c", ...)`: it works, but the **app icon argument is empty** and there is no `desktop-entry` hint.
- Window (`app` under `GDK_BACKEND=x11`, `xprop`): `WM_CLASS = "easy-personal-rs", "Easy-personal-rs"`, and **no `_NET_WM_ICON`**. Neither `.desktop` file has `StartupWMClass`, and the file is `easy-personal.desktop`, so GNOME cannot tie the window to the installed icon `easy-personal` (dock and Alt-Tab show a generic icon).
- Real `claude` run: `agent-run now` with the default account failed in 3 s: that account's login is expired (`Failed to authenticate: OAuth session expired and could not be refreshed`); with `--account hung2` it ended `investigate: ok ... report 6` in 53 s. The runner works; nothing to change in it.

**Restated intent**:
> 1. **Notification icon**: every desktop notification names the app icon: app icon argument `easy-personal` (the icon theme name the `.deb` installs) and the hint `desktop-entry` = `easy-personal`. Nothing else in the call changes.
> 2. **Window icon**: the `app` window (and the window the tray opens) has the app icon set, from the PNG already in `rust/crates/app/icons/` (compiled in, no file read at run time).
> 3. **Dock match**: both `.desktop` files carry `StartupWMClass=` with the window's real class, so GNOME (X11 and Wayland) shows the installed icon for the running window. `test_deb.sh` checks the line.
> 4. **Test isolation**: `connector_cli_parity` passes on a machine where the `.deb` is installed (`/usr/bin/easy-personal-launch` exists). Fix the test's isolation, not the app's launcher lookup, unless the lookup itself is wrong (then say why).
> 5. **One real-machine smoke script** `rust/scripts/smoke_real.sh` that the owner runs with one command and that prints one `PASS`/`FAIL`/`SKIP` line per step and exits non-zero on any `FAIL`:
>    a. build: `cargo build --locked -p easy-personal-rs --features window`;
>    b. tests: `cargo test --workspace --no-fail-fast` (summary line: passed/failed counts; the names of failed tests);
>    c. notification: the throw-away-HOME check above (fake waiting session, `serve --no-ingest --alerts`, `dbus-monitor`), PASS only when the `Notify` call carries app icon `easy-personal`; SKIP with a reason when there is no session bus or no `dbus-monitor`;
>    d. window: open `app` under XWayland, read `WM_CLASS` and `_NET_WM_ICON` with `xprop`; PASS when the icon is set and the class equals the `StartupWMClass` of `rust/packaging/easy-personal.desktop`; SKIP with a reason when there is no display or no `xprop`;
>    e. real `claude` run, **opt-in only** (`--claude <account id or label>`; without the flag the step prints `SKIP (spends plan limits; pass --claude <account>)`): remembers the agent run setting, turns it on, runs `agent-run now --account <account>`, puts the setting back as it was (also on failure or Ctrl-C), PASS when the line says `ok`.
>    Every temp folder and every process the script starts is removed or stopped on exit (trap). It never touches the owner's store except step e (which is the point of step e).

**Out of scope**: the expired login of the default account (owner's machine, not the app); the `memgraph_parity` load flake; any change of the alert rules, the popup text, the tray icons or menu; a new icon design; macOS/Windows; CI; the T093 container build; the landing page `docs/`.

### Requirement Fidelity Gate
- [x] Restated intent confirmed: auto (Supervisor), 2026-11-16, session tag `[admin]`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: `PRD.md` FR-097 (desktop alerts), FR-099 (tray, autostart), `tasks/TASK_GUIDE_T087.md` (the package, its icons and desktop entries), `tasks/TASK_GUIDE_T090.md` AC 9 (the one real run)

---

## Dependencies & Reachability
**Depends on**: T074, T076, T087, T090 (all merged)
**Entry points**: `easy-personal-rs serve --alerts`, `easy-personal-rs app`, `easy-personal-rs tray`, the `.deb`'s desktop entries, `rust/scripts/smoke_real.sh`

## Dependency rule
No new crate, no new npm package. The smoke script uses only tools already on an Ubuntu desktop or already used by the repo: `bash`, `cargo`, `python3`, `dbus-monitor`, `xprop`. A missing optional tool makes its step SKIP, never FAIL.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | The `Notify` call sends app icon `easy-personal` and the hint `desktop-entry` = `easy-personal`: a unit test on the value the code builds (pull the call's arguments into a small pure function if needed) and the smoke step c output | intent 1 |
| 2 | The window has an icon: smoke step d prints `_NET_WM_ICON` set; before the change the same step prints it not set (RED pasted) | intent 2 |
| 3 | Both `.desktop` files hold `StartupWMClass=<class>` equal to the `WM_CLASS` that step d reads; `test_deb.sh` fails on a package whose `.desktop` lacks the line (negative case pasted) and passes on the new build | intent 3 |
| 4 | `cargo test -p easy-personal-rs --test connector_cli_parity` passes on this machine with `/usr/bin/easy-personal-launch` present (it fails today: RED pasted) | intent 4 |
| 5 | `rust/scripts/smoke_real.sh` without flags: steps a-d PASS (or SKIP with a reason), step e SKIP, exit 0; output pasted | intent 5 |
| 6 | Smoke script failure paths, each shown with a fake (a fake `claude` on `PATH` that exits 1, a `NOTIFY`-less bus or a removed `dbus-monitor` from `PATH`): step e with a failing fake `claude` prints `FAIL`, exits non-zero, and the agent run setting is back to what it was (`agent-run status` before and after pasted); a missing tool gives `SKIP`, not `FAIL` | intent 5 |
| 7 | Step e with a **fake** `claude` that prints nothing and exits 0 against a throw-away `EASY_PERSONAL_HOME`: the script restores the setting. The one real `--claude hung2` run is the Supervisor's, not yours | intent 5 |
| 8 | `cd rust && cargo test --workspace`, `cargo clippy --workspace --all-targets --features window -- -D warnings`, `cargo fmt --check` pass; `shellcheck` on every script touched or added, if installed: exit 0 | all |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test -p easy-personal-rs --test connector_cli_parity && cargo clippy --workspace --all-targets --features window -- -D warnings && cargo fmt --check && ./scripts/smoke_real.sh
```

### Evidence (filled by reviewer at Stage 4/5)
> Moved to `tasks/TASK_REVIEW_T097.md`.

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T097.md`. BEFORE (capture before your first implementation commit): the `dbus-monitor` lines with the empty icon argument, the `xprop` lines with no `_NET_WM_ICON`, `grep StartupWMClass rust/packaging/*.desktop` printing nothing, and the `connector_cli_parity` failure. The Findings above give the commands.

## UI / Design Acceptance Criteria
The visible change is the app icon in three places: the notification, the window title bar / Alt-Tab, and the dock. No page, layout or colour change.
- Visual: the icon shown is the existing `rust/crates/app/icons/` artwork (`128x128.png` / `icon.png`), no new artwork. Proof: smoke steps c and d output (icon name in the `Notify` call, `_NET_WM_ICON` set) — a screenshot is not required.
- Design-system: no CSS, token or font change — N/A with that reason.
- Responsiveness: no page change — N/A with that reason.

---

## Approach
Test first: the unit test for the `Notify` arguments, the `test_deb.sh` `StartupWMClass` check, and the smoke script steps c and d, run against today's code and seen to fail for the right reason. Then the fixes. Then the parity test isolation (read why the `*_no_launcher` cases find `/usr/bin/easy-personal-launch`: a fixed path list in the app's launcher lookup, or a `PATH` the test inherits; fix the test's environment). Then the smoke script's other steps.

Notes:
- Notify signature: `(app_name, replaces_id, app_icon, summary, body, actions, hints, expire_timeout)`; the hint value is a D-Bus string variant.
- Tauri 2 window icon: `WebviewWindowBuilder::icon(include_image!(...))` (the tray already uses `include_image!`), or the app's default window icon; pick the one that also covers the window the tray opens, and say which.
- Get the class from the real window (step d), do not guess it. If it differs between `app` and `tray`, say so and set the one GNOME needs for each desktop file.
- Smoke step c: copy `rust/parity/store-v11.sqlite` into the temp HOME and insert one `agent_accounts` row whose folder is the temp `.claude` (as `rust/crates/app/tests/live.rs` `Serve::start` does). Append one assistant record with an open `AskUserQuestion` `tool_use`; the alert engine ticks every 5 s; wait up to 30 s.

## Edge Case Checklist
- No session bus (`DBUS_SESSION_BUS_ADDRESS` unset, SSH session): step c SKIP, the app's Notify failure path unchanged (one log line)
- Icon theme without `easy-personal` (the app run from `cargo` with no `.deb` installed): the notification still shows; the name is only a hint
- Step d leaves no window behind when the app fails to open (kill on trap)
- Step e on Ctrl-C mid-run: the setting is put back; the run itself is stopped by `agent-run off` as T090 built it
- The script runs from any folder (resolves the repo from its own path)
- A `cargo test` that times out under load (the `memgraph_parity` flake) is reported by name, not hidden

## Files to Change (Predicted)
`rust/crates/app/src/alerts.rs`, `rust/crates/app/src/window.rs`, `rust/packaging/easy-personal.desktop`, `rust/packaging/easy-personal-tray.desktop`, `rust/packaging/test_deb.sh`, `rust/crates/app/tests/connector_cli_parity.rs` (and its helpers under `tests/` only if the isolation lives there), `rust/scripts/smoke_real.sh` (new), `rust/README.md` (one short part: how to run the smoke script), `tasks/TASK_REVIEW_T097.md`

## Files Must NOT Touch
`rust/Cargo.toml`, `rust/Cargo.lock`, `web/**`, `src/easy_personal/**`, `docs/**`, `PRD.md`, any `PROJECT_KANBAN.md`, `memory/**`, `.claude/**`, `rust/packaging/build-deb.sh` (the T093 branch changes it), `rust/crates/app/icons/**` (no new artwork), `rust/crates/app/src/agent_run.rs`

## Test Plan
AC 1: the new unit test. AC 2, 5: smoke script output, before and after. AC 3: `test_deb.sh` on a package built by `rust/packaging/build-deb.sh` (host build, offline) and on a re-packed copy without the line. AC 4: the parity test, before and after. AC 6, 7: fakes on `PATH` and a throw-away `EASY_PERSONAL_HOME`. AC 8: the commands' output.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T097.md`
- [ ] Commits on branch `T097` only; final report with `Status:`

## Questions Log
