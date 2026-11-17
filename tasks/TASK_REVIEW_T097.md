# TASK_REVIEW — T097: app icon on notifications, window and dock; one real-machine smoke script

> Sibling of `tasks/TASK_GUIDE_T097.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` or `[admin]` tag; with `[admin]`, the `Not done:` line names `merge (admin: after the auto verify pass)`,
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
`Memory:` records the diff-driven memory pass you run before you stop: each entry written, or `nothing new`.
After an auto verify pass the hook, never you, adds `Closed: row moved to Done <date> — …` and moves the row,
or adds `Paused: done not moved — <reason>`. It moves the row only when this block holds `Mode: auto`, a
numbered decision, `Checks:`, `Memory:`, the `Not done:` auto verify slot with its result, and `Not done:`
naming the merge — no `<...>` left (`docs/claude-md/tracks.md` `### Auto close-out`).
With `/verify (yours)`, your pass goes in the `verify` row below: Result `☑ pass`, Notes `/verify by the user, session <id>, commit <sha>`.

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its proposal>; Advocate 2: <its proposal>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Memory: <each entry the memory pass wrote, as `file — entry` | nothing new>
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours), with [admin] only after the one retry also ended blocked>

-->

## Auto decisions

Mode: auto
1. Scope: notification icon, window icon, `StartupWMClass`, the `connector_cli_parity` isolation and one smoke script, as one task — RULE: `docs/claude-md/tracks.md` `### Session tags` (the loop runs each request Stage 0 to 5); the owner asked for the icon and for "automate all" in one session
2. Track extended, not minimal: the change is visible on the desktop (notification, window, dock icon) — RULE: `AGENTS.md` § Tracks ("a UI component")
3. `StartupWMClass=easy-personal-rs` (the program name), not a renamed app id — FACT: `rust/crates/app/src/window.rs` window class read from the real window: `WM_CLASS(STRING) = "easy-personal-rs", "Easy-personal-rs"` and Wayland `set_app_id("easy-personal-rs")` (agent report, pasted under AC 3)
4. Window icon from `icons/128x128.png`, not `icons/icon.png` — FACT: smoke step d with `icon.png` (512 px) printed `_NET_WM_ICON not found`, with `128x128.png` `Icon (128 x 128)` (agent report, AC 2)
5. `connector_cli_parity` fixed in the test (own `PATH`), not in the launcher lookup — FACT: `rust/crates/app/src/claude_cli.rs` looks beside the binary, then `PATH`; the test inherited `/usr/bin:/bin`
6. Code review: 0 P0, 0 P1 code, 0 P2 code; P1 process: the agent did not read the root `PROJECT_SPEC.md` (its report says so; `agents/backend.md` reached it as its system prompt); P2: lane check `EXCEEDS (lines 568 > 500)`, 315 of them the new script, accepted; P2: `rust/packaging/test_deb.sh` is also changed on the unmerged branch `T093` (its section 5), so whichever merges second needs a small merge of that file — RULE: `skills/code-review/SKILL.md` Phase 0 / Phase 0.5
6a. Code review run again through the `code-review` skill (2026-11-17, Supervisor, `main...T097`; the 2026-11-16 review was inline): 0 P0, 0 P1, 1 P2, 0 P3. P2 (confidence 100): `rust/scripts/smoke_real.sh` `--only <list>` does not check the step names, so a typo (`--only notifcation`) runs no step and still prints `smoke_real: no FAIL`, exit 0; not fixed here (follow-up). Lane check: `lane: C2, 11 files, 582 lines, EXCEEDS (lines 582 > 500)`, 315 of them the new script. Entry points found (`serve --alerts`, `app`, `tray`, both desktop entries, the script). Known: `rust/packaging/test_deb.sh` is also changed on branch `T093`, so the second merge needs a small conflict fix — RULE: `docs/claude-md/pipeline-stages.md` Stage 4 (code-review always)
7. Security review not run — RULE: `AGENTS.md` § Permanent Rules (mandatory for Medium/High risk only; T097 is Low)
8. One more real `claude` run through the script (`--only claude --claude hung2`), at the owner's request "make sure the claude invoke work ... let do automate" — FACT: it printed `PASS claude: investigate: ok, started 2026-11-16T12:17:17Z, ended 2026-11-16T12:17:59Z, report 7 (setting back to off)`
Checks: tests `rust/scripts/smoke_real.sh` (agent, 2026-11-16T12:02Z) — `PASS build`, `PASS tests: 1039 passed, 0 failed`, `PASS notification`, `PASS window`, `SKIP claude`, exit 0; Supervisor rerun `rust/scripts/smoke_real.sh --only build,notification,window` — `PASS notification: Notify call carried app icon easy-personal and desktop-entry easy-personal`, `PASS window: _NET_WM_ICON set; WM_CLASS(STRING) = "easy-personal-rs", "Easy-personal-rs" equals StartupWMClass easy-personal-rs`, exit 0; reviews code-review — 0 P0/P1 code findings; real run `rust/scripts/smoke_real.sh --only claude --claude hung2` — printed `PASS claude: investigate: ok ... report 7 (setting back to off)` (auto-checked, not /verify)
Memory: `memory/learnings.md` — T090 entry: the `connector_cli_parity` note marked fixed by T097; `memory/MEMORY.md` — its index line
Not done: merge (admin: after the auto verify pass); auto verify: running (T182); push of `main` (yours: the remote is `github`, the `[admin]` gate pushes to `origin` only); the default account's expired login (`~/.claude`, yours: `claude` then `/login`)

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/src/alerts.rs` `the_notify_call_names_the_app_icon_and_the_desktop_entry` (RED then green: `left: "" right: "easy-personal"`); `rust/crates/app/tests/connector_cli_parity.rs` (RED before, green after, see Demonstration BEFORE); `rust/packaging/test_deb.sh` checks 2 and 6 (StartupWMClass; RED on the 0.3.0 package: `FAIL: easy-personal.desktop has no StartupWMClass=easy-personal-rs line`); `rust/scripts/smoke_real.sh` steps c and d (RED against the pre-change binary: `FAIL notification: Notify call carried icon= hint=`, `FAIL window: _NET_WM_ICON not set`) |
| Verification command run | ☑ pass | `cargo test -p easy-personal-rs --test connector_cli_parity`: 2 passed. `cargo fmt --check`: exit 0. `cargo clippy --workspace --all-targets --features window -- -D warnings`: `Finished`, no warning. `shellcheck` on smoke_real.sh and test_deb.sh: exit 0. `./scripts/smoke_real.sh`: output under AFTER |
| Negative cases hold | ☑ pass | RED runs before each fix (BEFORE and AFTER blocks); `test_deb.sh` rejects a package without the line (section 6); smoke c and d FAIL on the pre-change binary; the claude step FAILs on a failing fake and the setting is restored |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | reviewed: the 8 changed files (alerts.rs `send`/`notify_args`, window.rs `open_window`, 2 `.desktop`, test_deb.sh, connector_cli_parity.rs, smoke_real.sh, README part). Skipped: `build-deb.sh`, `agent_run.rs`, icons (must-not-touch) |
| Full smoke suite still green (no regression) | ☑ pass | `cargo test --workspace --no-fail-fast` inside smoke_real.sh: `PASS tests: 1039 passed, 0 failed` (2026-11-16T12:02Z to 12:08Z) |
| **Companion score (easy-verifier)** | ☑ N/A | not run: Risk Low, no companion score asked for this task |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | the artwork is the existing `icons/128x128.png`; proof is smoke c and d: Notify app icon `easy-personal`, `_NET_WM_ICON(CARDINAL) = Icon (128 x 128)` (no screenshot required by the guide) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no CSS, token or font change |
| **UI: Responsiveness at target viewports** | ☑ N/A | no page change |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-16 on branch T097 at d822063, no implementation commit yet, debug build `cargo build --locked -p easy-personal-rs --features window`; probes: notify_check.sh / window_probe.sh from the Supervisor's scratchpad):

```
2026-11-16T11:10:43Z
== notify
--- Notify call seen on the session bus:
   string "easy-personal"
   uint32 0
   string ""
   string "Easy Personal"
   string "Waiting for you, AskUserQuestion, project tmp, session notify-c"
   array [
   ]
--
--- serve stderr:
notify-check: PASS
== window
WM_CLASS(STRING) = "easy-personal-rs", "Easy-personal-rs"
window icon: not set
== desktop
(grep exit 1)
```

Parity test (`cargo test -p easy-personal-rs --test connector_cli_parity`, `/usr/bin/easy-personal-launch` installed):

```
2026-11-16T11:10:59Z
---- every_connector_and_doctor_step_equals_the_python_cli stdout ----
connector cli oracle: 95 cases, 116 steps, 5 different

thread 'every_connector_and_doctor_step_equals_the_python_cli' (1767769) panicked at crates/app/tests/connector_cli_parity.rs:250:5:
5 problems:
sync_no_launcher step 0 ["connector", "sync", "--yes"]: exit 0, Python 1; stdout "Claude default: add acme-discord -> exec discord-acme\nClaude default: add acme-gmail -> exec gmail-acme\nClaude default: add acme-sendgrid -> exec sendgrid-acme\nClaude default: add acme-slack -> exec slack-acme\nClau
=====
sync_no_launcher: claude writes [Object {"argv": Array [String("mcp"), String("add-json"), String("acme-discord"), String("{\"type\": \"stdio\", \"command\": \"/usr/bin/easy-personal-launch\", \"args\": [\"exec\", \"discord-acme\"]}"), String("--scope"), String("user")], "ccd": String("<unset>")}, O
  want []
=====
sync_no_launcher: claude state {"<WORK>/agent-root/.claude-work":{"acme-discord":"Command: <LAUNCHER>\nArgs: exec discord-acme\n","acme-gmail":"Command: /usr/bin/easy-personal-launch\nArgs: exec gmail-acme\n","acme-sendgrid":"Command: /usr/bin/some-server\nArgs: --serve\n","acme-slack":"Command: <LA
=====
sync_no_launcher: table account_registrations [Array [Number(1), String("acme-discord"), String("discord-acme")], Array [Number(1), String("acme-gmail"), String("gmail-acme")], Array [Number(1), String("acme-sendgrid"), String("sendgrid-acme")], Array [Number(1), String("acme-slack"), String("slack-
  want [Array [Number(2), String("acme-gmail"), String("gmail-acme")], Array [Number(2), String("acme-slack"), String("slack-acme")], Array [Number(2), String("acme-teams"), String("teams-acme")], Array [Number(2), String("old-one"), String("slack-personal")], Array [Number(2), String("old-two"), St
=====
doctor_no_launcher step 0 ["doctor"]: exit 0, Python 1; stdout "ok keychain: <BACKEND>\nok claude: on PATH\nok launcher: /usr/bin/easy-personal-launch\n" stderr ""
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    every_connector_and_doctor_step_equals_the_python_cli

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.47s

error: test failed, to rerun pass `-p easy-personal-rs --test connector_cli_parity`
```

**AFTER** (same probes; plus the full smoke script, 2026-11-16T12:02:38Z to 12:08:43Z, branch T097 at e22826a, `EP_STATIC_DIR` pointed at the main checkout's built page because the worktree has none):

```
2026-11-16T12:02:38Z
PASS build: cargo build --locked -p easy-personal-rs --features window
PASS tests: 1039 passed, 0 failed
PASS notification: Notify call carried app icon easy-personal and desktop-entry easy-personal
PASS window: _NET_WM_ICON set; WM_CLASS(STRING) = "easy-personal-rs", "Easy-personal-rs" equals StartupWMClass easy-personal-rs
SKIP claude (spends plan limits; pass --claude <account>)
smoke_real: no FAIL
exit=0
2026-11-16T12:08:43Z
```

Package built by `packaging/build-deb.sh` (host, offline npm) and checked by `packaging/test_deb.sh` (tail):

```
built: /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/T097/rust/target/deb/easy-personal-rs_0.3.0_amd64.deb
== test_deb
== package: /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/T097/rust/target/deb/easy-personal-rs_0.3.0_amd64.deb
== 1. file list, modes, owner, control
ok: file list, modes, owner, control
== 2. both .desktop files
ok: desktop-file-validate passed on both files
== 3. extracted program
ok: --version printed 'easy-personal-rs 0.3.0'
ok: doctor: ok launcher: /tmp/t087-test-oYyuBT/extract/usr/bin/easy-personal-launch
== 4. dashboard serves the page from the package
ok: GET http://127.0.0.1:42823/ gave 200
ok: server 1984708 stopped
== 5. the check fails when a listed file is missing
ok: rejected, and it names the file:
    FAIL: file list differs from the listed files:
    3d2
    < usr/share/applications/easy-personal-tray.desktop
    FAIL: easy-personal-tray.desktop has no StartupWMClass=easy-personal-rs line
== 6. the check fails when a .desktop file has no StartupWMClass line
ok: rejected, and it names StartupWMClass:
    FAIL: easy-personal.desktop has no StartupWMClass=easy-personal-rs line
PASS: /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/T097/rust/target/deb/easy-personal-rs_0.3.0_amd64.deb
exit=0
```

**DELTA**: a desktop notification, the app window and the dock (once the new `.deb` is installed) now show the Easy Personal icon, and the owner can run one command, `rust/scripts/smoke_real.sh`, to check build, tests, notification, window (and with `--claude <account>` one real agent run) on the real machine.

**WITNESS**: backend-developer agent session, T097 worktree, 2026-11-16 (output pasted above); the Supervisor's own run of `rust/scripts/smoke_real.sh` and the one real `--claude hung2` run are still to come (not run by this agent).

### Per-AC evidence

AC 1: the unit test above (RED `left: ""`, `right: "easy-personal"`, then `alerts:: 21 passed`); smoke step c: `PASS notification: Notify call carried app icon easy-personal and desktop-entry easy-personal`.

AC 2: RED against the pre-change binary (main checkout `rust/target/debug/easy-personal-rs`, run through `EP_SMOKE_BIN`): `FAIL window: _NET_WM_ICON not set; WM_CLASS(STRING) = "easy-personal-rs", "Easy-personal-rs"`. GREEN: `PASS window: _NET_WM_ICON set; WM_CLASS(STRING) = "easy-personal-rs", "Easy-personal-rs" equals StartupWMClass easy-personal-rs`. Finding: `icons/icon.png` (512 px) is NOT accepted (`_NET_WM_ICON:  not found.`: the X property is too large), so the window uses `icons/128x128.png` (`xprop` then prints `Icon (128 x 128)`).

AC 3: the Wayland app id was read with `WAYLAND_DEBUG=client`: `xdg_toplevel#36.set_app_id("easy-personal-rs")`; X11 `WM_CLASS = "easy-personal-rs", "Easy-personal-rs"`. One value, `StartupWMClass=easy-personal-rs`, in both `.desktop` files (`app` and `tray` open the same window through `open_window`, so they share the class; the tray's click path was not run here). `test_deb.sh`: RED on the 0.3.0 package of the main checkout (`FAIL: easy-personal.desktop has no StartupWMClass=easy-personal-rs line`, `FAIL: easy-personal-tray.desktop ...`, `the package check failed`); GREEN on the new build; section 6 is the negative case (in AFTER).

AC 4: RED in BEFORE (5 problems, `doctor_no_launcher ... ok launcher: /usr/bin/easy-personal-launch`); GREEN: `test every_connector_and_doctor_step_equals_the_python_cli ... ok`, `test result: ok. 2 passed`. The lookup in `claude_cli.rs` is right (beside the program, else `PATH`); the test's `PATH` ended in `/usr/bin:/bin`, so the fix is in the test: a no-launcher case gets a `PATH` of the fake `claude`'s folder plus links to `sh tr sed mkdir rm` only.

AC 5: the AFTER block above (exit=0; build, tests, notification, window PASS; claude SKIP). `cargo test` rebuilds `target/debug/easy-personal-rs` without the window feature, so the script rebuilds quietly after the tests when notification or window follow in the same run.

AC 6 and 7 (fake `claude` first on `PATH`, throw-away `EASY_PERSONAL_HOME`; `agent-run status` before and after):
```
--- failing fake claude, setting was off: setting before: off
FAIL claude: exit 1: investigate: failed, started 2026-11-16T12:08:52Z, ended 2026-11-16T12:08:52Z, report none (setting back to off)
smoke_real: 1 FAIL
    exit=1  setting after: off
--- failing fake claude, setting was on: setting before: on
FAIL claude: exit 1: investigate: failed, started 2026-11-16T12:08:52Z, ended 2026-11-16T12:08:52Z, report none (setting back to on)
smoke_real: 1 FAIL
    exit=1  setting after: on
--- silent fake claude (exit 0), setting was off: setting before: off
PASS claude: investigate: ok, started 2026-11-16T12:08:52Z, ended 2026-11-16T12:08:52Z, report none (setting back to off)
smoke_real: no FAIL
    exit=0  setting after: off
--- silent fake claude (exit 0), setting was on: setting before: on
PASS claude: investigate: ok, started 2026-11-16T12:08:53Z, ended 2026-11-16T12:08:53Z, report none (setting back to on)
smoke_real: no FAIL
    exit=0  setting after: on
--- no --claude flag
SKIP claude (spends plan limits; pass --claude <account>)
smoke_real: no FAIL
    exit=0
--- unknown account
FAIL claude: exit 1: easy-personal-rs: unknown account; valid labels: smoke (setting back to on)
smoke_real: 1 FAIL
    exit=1 setting after: on

```
Signal mid-run (a fake `claude` that sleeps; SIGTERM to the process group, because a background job of a non-interactive shell ignores SIGINT; Ctrl-C from a terminal takes the same trap):
```
--- signal (TERM, to the whole group) mid-run (a fake claude that sleeps), setting was off
    setting while running: on
    script exit=130  setting after: off
interrupted
```
Missing tools:
```
SKIP notification (dbus-monitor is not installed)
smoke_real: no FAIL
    (dbus-monitor removed from PATH) exit=0
SKIP notification (no session bus)
smoke_real: no FAIL
    exit=0
SKIP window (no display (DISPLAY is not set))
smoke_real: no FAIL
    exit=0
```
