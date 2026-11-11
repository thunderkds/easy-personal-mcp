# TASK_REVIEW — T023: The window — app subcommand behind the window feature (Tauri v2)

> Sibling of `tasks/TASK_GUIDE_T023.md`. Everything here is **filled by the reviewer at Stage
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

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its proposal>; Advocate 2: <its proposal>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Memory: <each entry the memory pass wrote, as `file — entry` | nothing new>
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours)>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/src/window.rs` (unit tests: navigation table, blocked-target text, Tauri config, compiled capability set is empty), `rust/crates/app/src/display.rs` (preflight parsing and bounded-connect tests), `rust/crates/app/tests/window.rs` (4 real-window tests, `#[ignore]`d, need a display), `rust/crates/app/tests/app.rs` (no-feature exit 4, missing `--static-dir`, no-display exit 5, no-display `serve`/`mcp` incl. `tools/call usage_summary`/`store-info`), fixture `rust/crates/app/tests/fixtures/self-test/{index.html,app.js}` |
| Verification command run | ☑ pass | Supervisor, own runs 2026-11-11: default `cargo test --workspace` = 290 passed, 0 failed, 8 ignored under TZ unset, `Pacific/Kiritimati` and `Pacific/Pago_Pago`; `--features window` = 295 passed, 0 failed, 12 ignored; `cargo fmt --check` ok; clippy `-D warnings` 0 warnings (default and `--features window`); the 4 ignored real-window tests with `--ignored` = 4 passed (20.56 s); real `app --self-test --static-dir crates/app/tests/fixtures/self-test` printed `self-test: ok`, exit 0 (03:49:57Z) |
| Negative cases hold | ☑ pass | agent-reported, not re-run by the Supervisor: adding a remote capability JSON turned both capability tests red, green again once removed. Supervisor re-ran the real-window negatives: `external_links_and_new_windows_do_nothing` ok, `without_an_authenticated_request_the_self_test_exits_3_after_20_seconds` ok, `sigint_and_sigterm_close_the_window_exit_0_and_leave_no_process` ok. The navigation table covers `.evil.com`, `@evil.com`, `user@`, other port, `localhost`, `ws:`, `about:blank` |
| verify | ☑ N/A | `/verify` recipe not run: it needs `uv`, which is not installed on this machine (not installed unasked). Not claimed as verified. T023 changes no Python (`git status -- src web tests pyproject.toml uv.lock`: 0 entries). Real-run evidence of the Rust app is in the rows above and in the Demonstration |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | reviewed `rust/crates/app/src/{window,display,server}.rs`, `main.rs`, `Cargo.toml`, `build.rs`, `tauri.conf.json`, the two test files, the fixture, the README; not reviewed (not T023's): `rust/crates/core/**` except its read paths in the security pass, `rust/parity/**`, `rust/spikes/**`. Lane check: `lane: C3, 6 files, 93 lines, FITS`. code-review: 0 P0, 0 P1, 3 P2, 3 P3, 0 fixes needed, report `reports/code-review_main_20261111T104341.html`. security-review: nothing at confidence 8 or above, report `reports/security-review_main_20261111T104848.html` |
| Full smoke suite still green (no regression) | ☑ pass | whole workspace 290 passed, 0 failed (295 with the feature) under three time zones; Python tree untouched; `rust/parity/**` unchanged (agent: 310 files, sha256 `aa6fcaed…bff92`) |
| **Companion score (easy-verifier)** | ☑ N/A | `score` not run in this session |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no dashboard UI changed (the window loads the existing dashboard; only a 5-line test fixture page was added) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no UI |
| **UI: Responsiveness at target viewports** | ☑ N/A | no UI |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured by common-infrastructure on 2026-11-11, before any change of this run. Caveat (stated, not hidden):
the T023 work in progress was already committed in c26d7d4 (with T002-T022), so no capture from before *any* T023
implementation commit can exist any more; this is the state of HEAD e06f7bd before this run's changes. It is the guide's
Verification Command, with a scratch fixture folder because the tree had none (`rust/crates/app/tests/fixtures`: "No such
file or directory"), and `grep -ci window rust/README.md` printed `1` (no window paragraph; README says "four commands").

```
2026-11-11T03:17:38Z
HEAD e06f7bd, rust/ clean: 0 changed
... cargo build, cargo test --workspace, clippy -D warnings, fmt --check, cargo build --features window,
    cargo test --workspace --features window: all ok (584 passed, 0 failed, 20 ignored over both test runs)
     Running `target/debug/easy-personal-rs app --self-test --static-dir <scratchpad>/fixture-before`
MESA: error: ZINK: failed to choose pdev
libEGL warning: failed to open /dev/dri/renderD128: Permission denied
easy-personal-rs: window open on http://127.0.0.1:45165/
self-test: ok
easy-personal-rs: window closed
	Elapsed (wall clock) time (h:mm:ss or m:ss): 0:01.20
	Maximum resident set size (kbytes): 197420
	Exit status: 0
exit=0
2026-11-11T03:18:16Z
```

**AFTER**: the same command after this run's changes, run by the Supervisor 2026-11-11T03:49:57Z (the agent's own AFTER run was 03:39:21Z):

```
$ cargo run -q --features window -- app --self-test --static-dir crates/app/tests/fixtures/self-test
easy-personal-rs: window open on http://127.0.0.1:46351/
self-test: ok
easy-personal-rs: window closed
exit=0
```
(`libEGL`/`MESA` warnings from the WSLg graphics stack also reach stderr; they are not ours.) The fixture folder is now committed, so the command no longer needs a scratch folder, and `rust/README.md` has the window section.

**DELTA**: on Ubuntu with a display, `easy-personal-rs app --static-dir DIR` (built with `--features window`) opens the dashboard in a native window on the loopback server and ends cleanly on close or signal, while the default build, `serve` and `mcp` still need no display.

**WITNESS**: Supervisor session 2026-11-11 (own runs at 03:49:05Z-03:49:57Z, after the implementing agent's run); tool calls recorded in `memory/event-trace/T023.jsonl`.

## Open items (owner decisions, not blockers for review)

1. A `--features window` binary maps GTK and WebKitGTK at load time, so even `serve` and `mcp` need `libwebkit2gtk-4.1-0` and `libgtk-3-0t64` installed (no display needed). Suggestion: use the default binary for CLI and MCP.
2. The Tauri tree includes MPL-2.0 crates (`cssparser`, `cssparser-macros`, `selectors`, `dtoa-short`, `option-ext`): accept the licence before any release or packaging.
3. BEFORE was captured on HEAD `e06f7bd`, after the T023 code already existed in `c26d7d4`: accept explicitly or waive.
4. Not tested: a display that goes away mid-run; a second `app` launch opens a second window (documented as not done).
