# TASK_REVIEW — T062: Retire Python 5: cutover

> Sibling of `tasks/TASK_GUIDE_T062.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/tests/cutover_doc.rs` (4 tests): `test result: ok. 4 passed; 0 failed` (06:12Z); fails on a made-up subcommand (see Demonstration AFTER). Implementer's paste; reviewer to confirm |
| Verification command run | ☑ pass | workspace: 954 passed, 0 failed, 28 ignored; clippy `-D warnings` clean; `cargo fmt --check` ok (Demonstration AFTER, AC7). Implementer's paste; reviewer to confirm |
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

**BEFORE**: captured 2026-11-15T05:27:48Z, before any implementation commit (branch `t062-cutover`, HEAD `1dfd4bb`).

Terminal capture (the thing absent):

```
$ ls -l rust/docs/CUTOVER.md rust/crates/app/tests/cutover_doc.rs
ls: cannot access 'rust/docs/CUTOVER.md': No such file or directory
ls: cannot access 'rust/crates/app/tests/cutover_doc.rs': No such file or directory
$ cd rust && cargo test -p easy-personal-rs --test cutover_doc
error: no test target named `cutover_doc` in `easy-personal-rs` package
$ grep -c "Rust app" README.md
0
```

Verbatim prior `README.md` `## Install` section (README has no `## Rust app` section; the next heading is `## Quick start`):

~~~~markdown
## Install

You need Python 3.11 or newer, [uv](https://docs.astral.sh/uv/) and Node.js. The dashboard page is built from `web/` and is not in git, so build it before you install.

```bash
git clone https://github.com/thunderkds/easy-personal-mcp.git
cd easy-personal-mcp
npm --prefix web ci && npm --prefix web run build
uv tool install --reinstall --no-cache .
```

Check the install:

```bash
easy-personal --version
easy-personal ping
```

The first command prints `0.4.0`. The install adds two commands: `easy-personal` and `easy-personal-launch`.
~~~~

**AFTER**: captured 2026-11-15 (05:34Z to 06:30Z), branch `t062-cutover`, all runs in temp folders under `/tmp/claude-1000/.../scratchpad/t062/` (`root` = install root, `localbin` = stand-in for `~/.local/bin`, `home`/`data` = temp HOME and `EASY_PERSONAL_HOME`, `fakebin/claude` = the T050 fake `claude`, `DBUS_SESSION_BUS_ADDRESS` dead).

New files: `rust/docs/CUTOVER.md` (steps 0-5, the week, rollback, differences), `rust/crates/app/tests/cutover_doc.rs`. `README.md` gains only `## Rust app (default from T062)` (4 added lines, right after `## Install`, links `rust/docs/CUTOVER.md`).

**AC5, test (written first; red on the absent document, then green)**

```
$ cargo test -p easy-personal-rs --test cutover_doc          (before CUTOVER.md existed, 05:3xZ)
test a_made_up_subcommand_is_caught ... ok
test real_commands_arguments_and_paths_pass ... ok
test every_command_in_the_cutover_document_exists ... FAILED   (rust/docs/CUTOVER.md must exist)
test the_document_shows_the_commands_of_every_step ... FAILED
$ cargo test -p easy-personal-rs --test cutover_doc          (06:12Z, with the document)
test the_document_shows_the_commands_of_every_step ... ok
test a_made_up_subcommand_is_caught ... ok
test real_commands_arguments_and_paths_pass ... ok
test every_command_in_the_cutover_document_exists ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
```

Made-up subcommand appended to the real document (then restored):

```
$ printf '\n```bash\neasy-personal-rs frobnicate --now\n```\n' >> rust/docs/CUTOVER.md && cargo test -p easy-personal-rs --test cutover_doc
test every_command_in_the_cutover_document_exists ... FAILED
    "`easy-personal-rs frobnicate`: no such subcommand",
test result: FAILED. 3 passed; 1 failed
```

**AC4, the install block run into temp folders** (`cargo install` with `CARGO_TARGET_DIR=<worktree>/rust/target` instead of the document's `--target-dir rust/target`, the same thing; `--root <scratchpad>/t062/root`)

```
$ npm --prefix web ci && npm --prefix web run build           (05:30:41Z)
found 0 vulnerabilities
../src/easy_personal/dashboard/static/assets/index-DyyWCd3E.js   347.36 kB | gzip: 104.04 kB
✓ built in 143ms
$ cargo install --locked --path rust/crates/app --features window --root <t062>/root
    Finished `release` profile [optimized] target(s) in 3m 46s
  Installing <t062>/root/bin/easy-personal-launch
  Installing <t062>/root/bin/easy-personal-rs
   Installed package `easy-personal-rs v0.2.0 (...)` (executables `easy-personal-launch`, `easy-personal-rs`)
$ mkdir -p <t062>/root/static && cp -r src/easy_personal/dashboard/static/. <t062>/root/static/ && ls <t062>/root/static
assets  index.html
$ ln -sf <t062>/root/bin/easy-personal-rs <t062>/localbin/easy-personal-rs     (an old regular file `easy-personal-rs` was there; `localbin/easy-personal-launch` is a stand-in for Python's launcher)
lrwxrwxrwx ... easy-personal-rs -> <t062>/root/bin/easy-personal-rs
$ <t062>/localbin/easy-personal-rs --version
easy-personal-rs 0.2.0
$ easy-personal-rs dashboard --static-dir <t062>/root/static               (05:34:50Z; run as a child, killed afterwards)
ingesting before start (waits if another ingest runs)...      (stderr)
dashboard: http://127.0.0.1:42075/#<token>
Ctrl-C to stop.
GET /                        -> 200 text/html; charset=utf-8   (<!doctype html> ... <title>easy-personal</title>)
GET /api/version, no token   -> 403
GET /api/version, token      -> {"version":"0.2.0"}
kill <that pid>              -> stopped
```

**AC3, Python's launcher untouched; the linked program finds the launcher beside its real file**

```
$ sha256sum <t062>/localbin/easy-personal-launch     before the install block: 8dfcb512...defe16b
$ sha256sum -c launch.sha                            after link, doctor, sync, dashboard: easy-personal-launch: OK
$ easy-personal-rs doctor        (run as <t062>/localbin/easy-personal-rs, via the link; PATH = localbin, fakebin, /usr/bin; dead D-Bus)
FAIL keychain: no usable OS keychain (Secret Service); ...        (expected: the bus is dead on purpose)
ok claude: on PATH
ok launcher: <t062>/root/bin/easy-personal-launch                  (beside the REAL file, not localbin's)
$ easy-personal-rs connector sync </dev/null
nothing to change.
$ easy-personal-rs schedule install --dry-run | grep ExecStart
ExecStart=<t062>/root/bin/easy-personal-rs ingest                 (the real file's path, not the link's)
```

**Finding (Step 4 text follows it, not the guide's wording)**: `connector sync` decides "points at the launcher" by the program's file name only (`crates/core/src/sync.rs` `points_at_launcher`, same as Python's `writer.points_at_launcher`). Debug binary, fake keychain, fake `claude`, temp world:

```
$ connector add slack --workspace acme --yes ; connector sync --yes
Claude default: add acme-slack -> exec c36b29787       (entry: Command: <worktree>/rust/target/debug/easy-personal-launch)
$ (entry edited to Command: /home/you/.local/bin/easy-personal-launch)  connector sync
nothing to change.                                     (an entry using Python's launcher is kept)
$ (entry edited to Command: /home/you/.local/bin/other-launcher)         connector sync --yes
... remove acme-slack (does not point at the launcher) / add acme-slack -> exec c36b29787 (re-add)
```

So after the cutover, existing connector entries keep running Python's `~/.local/bin/easy-personal-launch` (a rollback needs nothing), and only new connectors use the Rust launcher. T063 must re-point the entries before it removes Python.

**AC7, verification command (06:12Z to 06:35Z)**

```
$ cargo test --workspace --no-fail-fast      -> 92 "test result: ok" lines; total passed 954, failed 0, ignored 28; 0 "FAILED"; exit 0
$ cargo clippy --workspace --all-targets -- -D warnings    -> Finished `dev` profile, no warning
$ cargo fmt --check                                         -> FMT_OK
```

The 28 ignored tests were not run (they include `real_keychain_items_cross_check`; the owner runs that in Step 0.3).

**Stage 4 fix round (2026-11-15, 08:08Z to 08:30Z)**

P1-1, keychain gate (Step 0.5 in the document, was 0.3). `--bin` listed nothing since T078 made `keychain.rs` the `[lib]` target:

```
$ cargo test -p easy-personal-rs --bin easy-personal-rs real_keychain_items_cross_check -- --list --ignored
     Running unittests src/main.rs (...)
0 tests, 0 benchmarks
$ cargo test -p easy-personal-rs --lib real_keychain_items_cross_check -- --list --ignored
     Running unittests src/keychain.rs (target/debug/deps/easy_personal_keychain-...)
tests::real_keychain_items_cross_check_with_python_keyring: test
1 test, 0 benchmarks
```

The document now uses `--lib` and says `0 passed` means the test did not run. The doc comment at `rust/crates/app/src/keychain.rs:582` names the same `--lib` command (the only src edit; one comment line). New test `the_keychain_gate_command_runs_the_lib_test` in `cutover_doc.rs`: the one `cargo test` line with `real_keychain_items_cross_check` must contain `-p easy-personal-rs`, `--lib`, `-- --ignored` and no `--bin`; `keychain.rs` must still hold `fn real_keychain_items_cross_check` and `--lib real_keychain_items_cross_check` in its comment. With the document line changed back to `--bin easy-personal-rs` it fails: ``the keychain command lacks `--lib`: ...--bin easy-personal-rs real_keychain_items_cross_check``.

P1-2, WAL-safe backup and restore. Step 0 order is now 0.1 record, 0.2 stop the timer (`systemctl --user stop easy-personal-ingest.timer easy-personal-ingest.service`), `pgrep -af easy-personal` must print nothing, and close every Claude Code session (each runs its own Python MCP server) and the Python dashboard; 0.3 backup with the SQLite backup call (the coordinator's `python3 -c` command, used verbatim); 0.4 build and `store-check`; 0.5 keychain test. Step 3 says the install enables and starts the timer again, with a fallback `systemctl --user start`. The rollback's restore block: stop timer, `pgrep`, `rm -f ~/.easy-personal/store.sqlite-wal ~/.easy-personal/store.sqlite-shm`, `cp -p` the backup back, start the timer. Tested on a temp WAL store (HOME = `<t062>/walhome`; the `systemctl` and `pgrep` lines were NOT run, they touch the real user session):

```
(writer: WAL mode, wal_autocheckpoint=0, 5 rows committed, then kill -9, so they live only in the -wal file)
-rw-r--r-- 8192  store.sqlite    -rw-r--r-- 32768 store.sqlite-shm    -rw-r--r-- 20632 store.sqlite-wal
plain cp of store.sqlite -> rows in plain cp: 0
$ HOME=<walhome> python3 -c "import sqlite3,os;p=os.path.expanduser('~/.easy-personal/store.sqlite');s=sqlite3.connect(p);d=sqlite3.connect(p+'.before-rust');s.backup(d);d.close();s.close()"
exit 0   -> store.sqlite, store.sqlite.before-rust (8192 bytes)
rows in backup: 5 ('ok',)
(later writer adds rows, killed again, a stale -wal is left; store.sqlite-wal and -shm present)
$ HOME=<walhome> sh -c 'rm -f ~/.easy-personal/store.sqlite-wal ~/.easy-personal/store.sqlite-shm; cp -p ~/.easy-personal/store.sqlite.before-rust ~/.easy-personal/store.sqlite'
exit 0   -> store.sqlite, store.sqlite.before-rust (no -wal, no -shm)
rows after restore: 5 [0, 1, 2, 3, 4] ('ok',)
```

P2-1: Step 0.1 saves `grep ExecStart ~/.config/systemd/user/easy-personal-ingest.service` to `~/easy-personal-cutover-notes/timer.txt` and says to remember `--retention-days N`; Step 3 (both `schedule install` commands, and its expected `ExecStart=` line) and the rollback pass the same `--retention-days N`.

P2-2: Step 0.1 saves `claude mcp get easy-personal` per account to `~/easy-personal-cutover-notes/mcp-default.txt` (other accounts: another file name, with `CLAUDE_CONFIG_DIR`). The rollback re-adds with the saved `Command:` and `Args:` (and `-e KEY=value` per `Environment:` line), not the README default; an account with no saved registration only gets the `remove` line. Step 2 points back at the notes.

P3, flag typos: skipped. The check needs the tokens after each command, which means a different return type for `word_runs` and a second walk, about 25 lines (the limit was 20). Not built; subcommand paths only, as the guide says. I checked the flags in the document by hand while writing them.

Re-run (08:08:49Z, after the fixes; state file rewritten to T062 before each run):

```
$ cargo test -p easy-personal-rs --test cutover_doc
test the_keychain_gate_command_runs_the_lib_test ... ok
test the_document_shows_the_commands_of_every_step ... ok
test a_made_up_subcommand_is_caught ... ok
test real_commands_arguments_and_paths_pass ... ok
test every_command_in_the_cutover_document_exists ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
$ cargo test --workspace --no-fail-fast   -> exit 0; passed 955, failed 0, ignored 28; 0 "FAILED"
$ cargo clippy --workspace --all-targets -- -D warnings   -> exit 0, no warning
$ cargo fmt --check                                        -> exit 0
```

**DELTA**: The owner can follow one document, `rust/docs/CUTOVER.md`, to switch the MCP server, the timer, the connectors and the dashboard from `easy-personal` to `easy-personal-rs` and back, and a test fails when the document names a command that does not exist.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T062.jsonl`, never the
implementing agent alone]
