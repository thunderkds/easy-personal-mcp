# TASK_REVIEW — T078: Keychain leftovers

> Sibling of `tasks/TASK_GUIDE_T078.md`. Everything here is **filled by the reviewer at Stage
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

**BEFORE** (captured 2026-11-15T02:51:44Z to 02:55:44Z, branch `t078-keychain-leftovers` at `532aeb8`, no implementation commit yet):

```
$ grep -n 'path = "keychain.rs"\|allow(dead_code)' rust/crates/app/src/launch.rs
12:#[path = "keychain.rs"]
13:#[allow(dead_code)] // the launcher reads items; it never stores or removes them
$ grep -n "mod keychain" rust/crates/app/src/main.rs rust/crates/app/src/launch.rs
rust/crates/app/src/main.rs:15:mod keychain;
rust/crates/app/src/launch.rs:15:mod keychain;
$ cargo build --bin easy-personal-launch --bin easy-personal-rs      # cold, fresh target dir
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 38s
$ ls -l target/debug/easy-personal-launch target/debug/easy-personal-rs
-rwxrwxr-x 2 ... 101078368 Nov 15 09:54 target/debug/easy-personal-launch
-rwxrwxr-x 2 ... 202314880 Nov 15 09:55 target/debug/easy-personal-rs
$ cargo tree -p easy-personal-rs -e normal --depth 1
easy-personal-rs v0.2.0 (.../rust/crates/app)
|- axum v0.8.9, clap v4.6.7, easy-personal-core v0.2.0, futures-core v0.3.34, getrandom v0.4.3,
|  keyring v3.6.3, rmcp v3.9.0, rusqlite v0.41.1, serde_json v1.0.154, tokio v1.54.1
$ strings -a target/debug/easy-personal-launch | grep -c <name>      # what the launcher binary holds
axum: 0   rmcp: 0   clap: 0   rusqlite: 8091   tokio: 46242   zbus: 58399   sqlite3_open: 3
$ wc -l crates/app/src/keychain.rs crates/app/src/launch.rs
  665 keychain.rs   77 launch.rs
```

Item 2 BEFORE: the measuring test (below) did not exist yet; see its first run under "Item 2".

**AFTER** (2026-11-15T03:00:16Z, commit `6241e1c`):

```
$ grep -rn 'path = "keychain.rs"\|allow(dead_code)' rust/crates/app/src/launch.rs rust/crates/app/src/keychain.rs   # no output
$ grep -n "keychain" rust/crates/app/src/main.rs | head -1
15:use easy_personal_keychain as keychain;        # the one line that changed in main.rs (was `mod keychain;`)
$ cargo build --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s)           (no warning)
$ cargo build --features window
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 53s
$ ls -l target/debug/easy-personal-launch target/debug/easy-personal-rs
101873856 easy-personal-launch      (was 101078368, +0.8 %)
203385088 easy-personal-rs          (was 202314880, +0.5 %)
$ strings -a target/debug/easy-personal-launch | grep -c <name>
axum: 0   rmcp: 0   clap: 0   rusqlite: 8106   tokio: 46242   zbus: 58399   sqlite3_open: 3     # same set as BEFORE
$ cargo tree -p easy-personal-rs -e normal --depth 1      # identical to BEFORE (same ten direct dependencies)
$ cargo test -p easy-personal-rs --lib
     Running unittests src/keychain.rs
test result: ok. 7 passed; 0 failed; 1 ignored     # the keychain unit tests run once (the ignored one is the real-keychain test: not run)
$ cargo test -p easy-personal-rs --bin easy-personal-rs      -> 126 passed   (the keychain tests are no longer in this binary)
$ cargo test -p easy-personal-rs --bin easy-personal-launch  -> 0 tests      (they used to run here a second time)
```

The change: `[lib] name = "easy_personal_keychain", path = "src/keychain.rs"` in `rust/crates/app/Cargo.toml` (the library is that one file, no `lib.rs`); `main.rs` and `launch.rs` each have `use easy_personal_keychain as keychain;`. Every `crate::keychain::...` in the server modules resolves through that one `use` line. The three item-level `#[cfg_attr(not(test), allow(dead_code))]` in `keychain.rs` are gone too (a library item that is `pub` is not dead). `keychain.rs` is still at `rust/crates/app/src/keychain.rs`: it did not move.

Why a library and no new crate: the launcher already depended on `easy-personal-core` for `pyjson`, so it links the same crates as before; the library target is one file and adds no dependency. Cost, not a blocker: debug binaries +0.8 % and +0.5 % (the library's `pub` items are no longer removed as dead code); no release build was measured (a cold release build of this tree is minutes, and the choice does not depend on it).

**DELTA**: `keychain.rs` compiles once, a store or write to the keychain cannot be reached by a second copy of the module, and a workspace or connector change no longer holds the store's write lock while `claude` runs.

**WITNESS**: [to be derived by the reviewer from `memory/event-trace/T078.jsonl`; the implementing agent ran the commands above on 2026-11-15, 02:51Z to 03:10Z]


---

## Implementation notes (backend-developer, 2026-11-15)

### Item 2: `claude` inside a write transaction (AC3, AC4)

**Where `claude` ran inside an open transaction**: only `workspaces::change` (`rust/crates/core/src/workspaces.rs`), which `patch` (rename of a workspace that holds connectors), `delete` (workspace), `move_connector` and `set_alias` all use. It opened `BEGIN IMMEDIATE`, ran the mutation, then `sync::plan` ran `claude mcp get <alias>` for every alias of the touched accounts (up to 60 s each, `claude_cli.rs` `TIMEOUT`), and only then rolled back (plan) or committed (`yes`). Everywhere else no `claude` call is inside a transaction: `add_to_workspace` makes no `claude` call; `remove_connector` plans and applies with no transaction and opens one only for the row delete plus the keychain delete (T066); `sync_apply` is `plan_all` then `apply` on the bare connection; `legacy`, `remove_pre_workspace` use no transaction.

**Measurement** (test first, `rust/crates/core/tests/claude_outside_transaction.rs`): a fake `claude` that, at the moment it is called, opens a second connection to the store, reads, and runs `BEGIN IMMEDIATE; ROLLBACK` with a busy timeout of 0. BEFORE the change (2026-11-15T02:56:17Z):

```
test an_alias_change_does_not_hold_the_write_lock_while_claude_runs ... FAILED
test a_move_plan_does_not_hold_the_write_lock_while_claude_runs ... FAILED
the store's write lock is held during `claude mcp get acme-slack` (a writer on another connection gets SQLITE_BUSY)
test result: FAILED. 0 passed; 2 failed
```

The reader probe passed in both (the store is WAL: a reader is never blocked). A second **writer** (the ingest, a second `easy-personal`, the Python app) was blocked, and gets `database is locked` after its 5 s busy timeout while `claude` ran. So the answer to "is a reader blocked": no; "is a writer blocked": yes.

**Fix** (`workspaces.rs`: `change`, `plan_change`, `Record`, `Replay`, `copy_repos`; about 90 lines): `mutate` and the plan now run twice. Run 1 only records which `claude` calls the plan makes (canned "no entry" answers; the transaction is dropped, rolled back). Those calls then run for real with no transaction open (an account where `claude` could not run is not asked again, as the plan stops there too). Run 2 replays the answers, and is the one that commits (`yes`) or is rolled back (plan). Apply (`sync::apply`) is unchanged: after the commit, no transaction, same `claude`. A call run 2 makes that run 1 did not (the store moved between the runs) answers "could not run": that account is skipped, never guessed. `WRITE_LOCK` is untouched and still held across the whole call. T066's guarantee holds: the keychain is not touched in `change` (`keychain: untouched`); a row change is committed only on `yes`, before apply, exactly as before; the T066 rollback tests (`a_store_refusal_at_the_row_delete_keeps_the_keychain_item`, `a_keychain_refusal_keeps_the_row`) pass.

AFTER: the same tests pass (rename, delete, move, alias, sync apply, and the `remove` unit test in `connector_writes.rs`; one more test checks a confirmed rename commits once, with its repo list and renamed alias).

```
$ cargo test -p easy-personal-core --test claude_outside_transaction
test a_sync_apply_does_not_hold_the_write_lock_while_claude_runs ... ok
test an_alias_change_does_not_hold_the_write_lock_while_claude_runs ... ok
test a_confirmed_rename_commits_once_with_its_repos ... ok
test a_move_plan_does_not_hold_the_write_lock_while_claude_runs ... ok
test a_workspace_rename_and_delete_do_not_hold_the_write_lock_while_claude_runs ... ok
test result: ok. 5 passed; 0 failed
$ cargo test -p easy-personal-rs --bin easy-personal-rs connector_writes
test connector_writes::tests::a_remove_runs_claude_with_the_write_lock_free ... ok   (and the 6 earlier ones, T066's included)
```

Side effect to know: the plan costs a second run of the mutation (a few SQL statements) and the write lock is taken twice, briefly, instead of once for the whole `claude` time. A plan whose `claude` calls all fail "could not run" still asks each account once.

### Item 3: D-Bus call timeout under the write lock

Skipped, no code. Decision for the owner (`[admin]` mode, recorded in the guide): a timeout changes behaviour (a slow but working keychain unlock prompt would fail the write); the Secret Service call already fails fast when no session bus exists (T050 negative test "no session bus refuses cleanly"); the recommendation of record since T050 (`memory/decisions.md`, `tasks/MINIMAL_LOG.md`) is skip. One revert re-opens it.

### Item 4: unused `dbus` entries in `Cargo.lock`

```
$ cargo tree -i dbus-secret-service --workspace --all-features --target all
warning: nothing to print.                      # no target and no feature reaches it
$ cargo tree -i dbus --workspace --all-features --target all
dbus v0.9.12
└── tao v0.37.2
    └── tauri-runtime-wry v2.12.3
        └── tauri v2.12.5
            └── easy-personal-rs v0.2.0 (.../rust/crates/app)
$ cargo tree -i libdbus-sys --workspace --all-features --target all
libdbus-sys v0.2.7
└── dbus v0.9.12
    └── tao v0.37.2 ... tauri v2.12.5 ... easy-personal-rs      # same path
```

`dbus` and `libdbus-sys` are used (the `window` feature: tauri, `tauri-runtime-wry`, `tao`). `dbus-secret-service 4.1.0` is built for no target and no feature: `keyring` 3.6.3 declares it as an optional dependency (feature `sync-secret-service`, which this project does not enable). `cargo` still lists it in `Cargo.lock` under `keyring`'s `dependencies`, because the lock records the optional dependency for every platform (`keyring` names it for `linux`, `freebsd`, `openbsd`) and the weak feature lines `crypto-rust = ["dbus-secret-service?/crypto-rust"]`. Cargo does not drop it:

```
$ cargo update --workspace --dry-run          -> Locking 0 packages (nothing removed)
$ cargo update -p dbus-secret-service --precise 4.1.0 --dry-run   -> no change
$ cargo metadata --locked                     -> ok (the lock needs no rewrite)
$ git status --short rust/Cargo.lock          -> (empty; every `cargo build/test/clippy` of this task left it as it was)
```

Rule from the guide: remove a lock entry only when no target and no feature reaches it AND `cargo` itself drops it. The first holds for `dbus-secret-service`, the second does not. Nothing was changed; `Cargo.lock` is byte-identical to `main`. Removing the entry would need a `keyring` change (not in scope: no new crate, no change of features) or a hand edit (forbidden).

### Cut list

- Release-build binary size and build time of the library change (debug numbers only; not a blocker).
- No wall-clock "slow claude" test: the probe at the moment of the call proves the lock state with no timing and no flake.
- A `claude` ask-once cache across `patch`/`move`/`alias` calls (each call still asks for its own aliases).
- Item 3 (D-Bus timeout) and the `dbus-secret-service` lock entry: recorded, no change (above).

### Verification (this task, worktree `t078-keychain-leftovers`)

`cargo test --workspace --no-fail-fast`: 87 test binaries `ok`, 0 failed (the `ingest_coexist` binary ran 14, ignored 10; the real-keychain test is `ignored` and was not run). `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --check` clean; `cargo build --features window` ok (1m 53s); `parity/leak_rust.py`: `leak check: 0 hits over 42 canaries, 778 surfaces, 1761981 bytes`. After the last clippy-driven edit (`copy_repos`, one line), the affected test binaries were re-run: core `claude_outside_transaction`, `workspaces_parity`, `connectors_parity`; app lib, `launcher`, `connector_writes`, `workspaces`, `connector_cli`, bin unit tests: all ok.
