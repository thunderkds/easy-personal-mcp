# TASK_REVIEW — T087: Ubuntu .deb package of the Rust app

> Sibling of `tasks/TASK_GUIDE_T[NNN].md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/packaging/test_deb.sh` (shell test, chosen over a Rust test: the package is files plus `dpkg-deb` output; a Rust test would call the same tools and add a cargo target). Covers AC 2-6. Last run: `PASS: .../rust/target/deb/easy-personal-rs_0.2.0_amd64.deb`, exit 0, steps 1-5 each `ok:` (see Demonstration AFTER). Written first and seen RED: `./test_deb.sh: ... build-deb.sh: No such file or directory`; the dotfile check also seen RED on the first package (`FAIL: a dotfile is in usr/share/easy-personal-rs/static`), then GREEN after the fix |
| Verification command run | ☑ pass | build: `build exit 0`, `built: .../rust/target/deb/easy-personal-rs_0.2.0_amd64.deb` (12 m 56 s for the first release build); test: `test exit 0`; `cargo clippy --workspace --all-targets -- -D warnings` exit 0; `cargo fmt --check` exit 0; `shellcheck rust/packaging/*.sh` exit 0 (2026-11-15T11:12Z). `cargo test --workspace --no-fail-fast` with a temp HOME: 991 passed, 0 failed, 28 ignored on a first run except one timing test (`live.rs` `ten_streams_get_the_events_while_one_client_never_reads`, `timed out; got []`, load average 6 from other sessions' builds); alone, `cargo test -p easy-personal-rs --test live`: `8 passed; 0 failed`. No Rust file changed. Ignored tests (keychain) not run |
| Negative cases hold | ☑ pass | Step 5 of the test: package unpacked with `dpkg-deb -R`, `usr/share/applications/easy-personal-tray.desktop` deleted, packed again; the check printed `FAIL: file list differs from the listed files:` / `< usr/share/applications/easy-personal-tray.desktop` and rejected it. Also a package with a dotfile in the page folder is rejected |
| verify | ☐ N/A | Not run by the implementer (Supervisor stage). Real run done by the test: unpacked `usr/bin/easy-personal-rs --version` printed `easy-personal-rs 0.2.0`; `dashboard --no-ingest --static-dir <extract>/usr/share/easy-personal-rs/static` gave `GET / 200`; no server left running — pass. [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ N/A | Implementer did not run `code-review`; scope for the reviewer: new files in `rust/packaging/` only (no Rust, web or Python file touched; `git diff main --stat` shows only `rust/packaging/*` and this file) |
| Full smoke suite still green (no regression) | ☑ pass | See Verification command row: `cargo test --workspace` 991 passed; the one load-timing failure passed alone (8 passed) |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | No page change (guide: UI / Design Acceptance Criteria N/A); the `.desktop` entries reuse the existing icons |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | No page change (guide: UI / Design Acceptance Criteria N/A); the `.desktop` entries reuse the existing icons |
| **UI: Responsiveness at target viewports** | ☑ N/A | No page change (guide: UI / Design Acceptance Criteria N/A); the `.desktop` entries reuse the existing icons |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-15T09:57:14Z in the T087 worktree, branch `t087-deb`, before any implementation commit:

```
$ ls rust/packaging
ls: cannot access 'rust/packaging': No such file or directory
$ grep -n '"bundle"' rust/crates/app/tauri.conf.json
12:  "bundle": { "active": false }
$ ls rust/target/deb
ls: cannot access 'rust/target/deb': No such file or directory
```

No packaging script, no `.deb` output folder, Tauri bundling off.

**AFTER**: 2026-11-15, same worktree, after the commits. `rust/packaging/build-deb.sh` (exit 0) then `rust/packaging/test_deb.sh` (exit 0):

```
$ rust/packaging/test_deb.sh
== package: .../rust/target/deb/easy-personal-rs_0.2.0_amd64.deb
== 1. file list, modes, owner, control
ok: file list, modes, owner, control
== 2. both .desktop files
ok: desktop-file-validate passed on both files
== 3. extracted program
ok: --version printed 'easy-personal-rs 0.2.0'
ok: doctor: ok launcher: <temp>/extract/usr/bin/easy-personal-launch
== 4. dashboard serves the page from the package
ok: GET http://127.0.0.1:38085/ gave 200
ok: server 65070 stopped
== 5. the check fails when a listed file is missing
ok: rejected, and it names the file:
    FAIL: file list differs from the listed files:
    3d2
    < usr/share/applications/easy-personal-tray.desktop
PASS: .../rust/target/deb/easy-personal-rs_0.2.0_amd64.deb
```

`dpkg-deb -c` (page assets elided): `usr/bin/easy-personal-launch` and `usr/bin/easy-personal-rs` `-rwxr-xr-x root/root`; both `.desktop` files, `copyright`, `changelog.gz`, three `easy-personal.png` icons (32x32, 128x128, 256x256) `-rw-r--r-- root/root`; `usr/share/easy-personal-rs/static/index.html` plus `assets/`. Nothing outside `/usr`.

`dpkg-deb -I`: `Package: easy-personal-rs`, `Version: 0.2.0`, `Architecture: amd64`, `Installed-Size: 28168`, `Section: utils`, `Priority: optional`,
`Depends: libc6 (>= 2.39), libcairo2 (>= 1.10.0), libdbus-1-3 (>= 1.10), libgcc-s1 (>= 4.2), libgdk-pixbuf-2.0-0 (>= 2.36.9), libglib2.0-0t64 (>= 2.66.0), libgtk-3-0t64 (>= 3.21.5), libjavascriptcoregtk-4.1-0, libsoup-3.0-0 (>= 3.0.3), libwebkit2gtk-4.1-0 (>= 2.41.90), libayatana-appindicator3-1`. Control archive: `./`, `./control` only (no maintainer scripts). Package size 7.6 MB.

**DELTA**: The owner can run one script to get an installable Ubuntu `.deb` of the Rust app (programs, dashboard page, menu and tray entries, icons) and one test that proves its contents, without installing anything.

**WITNESS**: run by the common-infrastructure agent for T087 on 2026-11-15 (build at 17:12-17:13 local, test runs after; timestamps in the agent's own session); `memory/event-trace/T087.jsonl` not read by the agent, so the Supervisor should derive the witness from it.

## Stage 4/5 (Supervisor, 2026-11-15)
- code-review: 0 P0, 0 P1. P2 fixed in a4a9d94: README now names the tray host (Ubuntu AppIndicators extension); found live, the owner's GNOME has no `org.kde.StatusNotifierWatcher`, the tray prints `no tray area (no StatusNotifier host)`. Accepted agent decisions: absolute `Exec=/usr/bin/easy-personal-rs` (menu never runs the ~/.local cutover copy); Maintainer `thunderkds <thunderkds@users.noreply.github.com>` (no personal mail). Entry point `rust/packaging/build-deb.sh` exists.
- security-review: files only, no maintainer scripts (control archive `./control` only), all root/root, binaries 0755, no setuid, no link, nothing outside /usr; `rm -rf` limited to `rust/target/deb/stage`; secret scan of added lines: 0 hits (only the word "secrets" in Description).
- /verify (worktree, a4a9d94): `build-deb.sh` build exit=0; `test_deb.sh` PASS exit=0 (`--version` 0.2.0, doctor launcher at extract path, `GET / 200`, server stopped, negative case rejected naming easy-personal-tray.desktop); `cargo clippy --workspace --all-targets -- -D warnings` exit=0; `cargo fmt --check` exit=0. Workspace tests: agent run 991 passed; `live.rs ten_streams_...` failed once under load, passed alone (8/8); no Rust file changed.
- easy-verifier score: not run (server ECONNREFUSED this session).
