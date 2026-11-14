# TASK_REVIEW — T050: Rust keychain, launcher, connector writes, sync apply, legacy

> Sibling of `tasks/TASK_GUIDE_T050.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/tests/connector_writes.rs` (10 tests, incl. fix-round `a_claude_that_cannot_run_removes_no_stored_connector`, `..._no_pre_workspace_item`, `two_writes_at_once_run_one_after_the_other`), `rust/crates/app/tests/launcher.rs` (6), `rust/crates/app/tests/connector_parity.rs` (2; 40 cases, 102 requests, 175 `claude` calls, 14 launcher runs against Python-made goldens), unit tests in `keychain.rs`, `claude_cli.rs`, `connector_writes.rs`, `api.rs`; leak `rust/parity/leak_rust.py`. AC9 (Python-keyring interop) is one `#[ignore]` test, NOT run: unverified |
| Verification command run | ☑ pass | 2026-11-14, Supervisor, on `1b142db`: `cargo test --workspace --offline --no-fail-fast`: 75 suites ok, 793 passed, 0 failed; `cargo clippy --workspace --offline --all-targets -- -D warnings` exit 0; `cargo fmt --check` exit 0; `env -u CLAUDE_CONFIG_DIR parity/.venv/bin/python parity/leak_rust.py`: `leak check: 0 hits over 42 canaries, 385 surfaces, 1395520 bytes`; `git diff --stat 93aca76..HEAD -- rust/parity/golden-connectors`: empty |
| Negative cases hold | ☑ pass | `connector_writes.rs`: locked keychain refuses with the fixed text and writes nothing; hostile connector name reaches neither keychain nor `claude`; hostile account folder is only an env value; 413 over 1 MiB and wrong methods 405; every new route guarded; store refusal rolls the keychain item back; a `claude` that cannot run changes no row and no keychain item. `launcher.rs`: refusals are Python's fixed texts, exit 1, never show a secret; no session bus refuses cleanly |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `git diff main...T050` (28 files, ~10.9k lines; `lane_check`: C3 FITS). Two independent read-only reviewers (security; correctness and tests) read all of `keychain.rs`, `launch.rs`, `claude_cli.rs`, `connector_writes.rs`, `sync_apply.rs`, `legacy.rs`, `writes.rs`, `api.rs`, core `sync.rs`/`workspaces.rs`/`pyjson.rs`, the tests and the oracle generator; golden `cases.json` spot-checked only. Findings before the fix round: P0 0, P1 0, P2 4, P3 8; fix round `5cf3d57..1b142db` closed P2 `entry_exists`, P2 concurrency test, P3 rollback report, P3 duplicate helper. Open and deferred, see the Stage 4 summary: 0600/atomic write of the debug fake keychain (owner call), D-Bus call timeout under the write lock (owner call), delete-before-row order, `kill()` of grandchildren, `claude` call inside a write transaction, `#[path]` double compile, `Cargo.lock` still lists unused `dbus-secret-service`/`dbus`/`libdbus-sys` |
| Full smoke suite still green (no regression) | ☑ pass | the 793-test workspace run above (no failures; the `ingest_coexist` timing flake did not occur in this run) |
| **Companion score (easy-verifier)** | ☑ pass (advisory) | `score` scope=`changes` ref=`main` task=T050, change_kind confirmed `feature`: `overall` 41 (4 of 13 dimensions rated: blast-radius 46, requirement-fidelity 33, security 30, test-strategy 55); `change_risk` abstained (`all_metrics_abstained`). Low signal: the `security` 30 comes from lockfile/manifest redaction hits, not from the diff. report: /home/hungnguyenhuu/.claude-hung2/projects/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp-rust/e64d19ef-48c3-4f22-9423-6b479756ddac/tool-results/mcp-easy-verifier-score-1794625164961.txt |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | pure backend task, no UI component |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | pure backend task |
| **UI: Responsiveness at target viewports** | ☑ N/A | pure backend task |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured by backend-developer, 2026-11-13T14:39Z..14:43Z, branch T050 at `983f7b1`, before any code file):

*Step 0, dependency record (guide "Dependency rule").*

Before the change (`cargo 1.99.1`, 2026-11-13T14:39:55Z):

```
$ cargo tree -p easy-personal-rs -e normal --depth 1
easy-personal-rs v0.2.0 (.../T050/rust/crates/app)
├── axum v0.8.9
├── clap v4.6.7
├── easy-personal-core v0.2.0 (.../T050/rust/crates/core)
├── futures-core v0.3.34
├── getrandom v0.4.3
├── rmcp v3.9.0
├── rusqlite v0.41.1
├── serde_json v1.0.154
└── tokio v1.54.1
$ grep -c '^name = ' Cargo.lock
462
$ cargo tree -p easy-personal-rs -e normal --prefix none | sort -u | wc -l     # packages in the default build
104
```

The change (the one allowed line, `rust/crates/app/Cargo.toml`). `cargo info keyring@3.6.3` lists both feature names
(`sync-secret-service = [dep:dbus-secret-service]`, `crypto-rust = [dbus-secret-service?/crypto-rust, secret-service?/crypto-rust]`):

```
keyring = { version = "=3.6.3", default-features = false, features = ["sync-secret-service", "crypto-rust"] }
```

After the change (2026-11-13T14:41:54Z):

```
$ cargo build -p easy-personal-rs
   Compiling libdbus-sys v0.2.7
   Compiling dbus v0.9.12
   Compiling dbus-secret-service v4.1.0
   Compiling keyring v3.6.3
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 28.47s
$ grep -c '^name = ' Cargo.lock
536                      # 74 new lock entries (60..90 is fine), 0 removed
$ cargo tree ... | sort -u | wc -l
132                      # 28 new packages in the default build
$ git diff --stat rust/Cargo.lock
 rust/Cargo.lock | 803 +++++++++++++++++++++++++++++++++++++++++++++++++++++++-
 1 file changed, 795 insertions(+), 8 deletions(-)
```

New lock entries with licence (`cargo metadata --locked`), all in the allowed set (each `OR` has MIT or Apache-2.0):
aes 0.8.4 MIT OR Apache-2.0 · async-broadcast 0.7.2 MIT OR Apache-2.0 · async-channel 2.5.0 Apache-2.0 OR MIT · async-io 2.6.0 Apache-2.0 OR MIT ·
async-lock 3.5.0 Apache-2.0 OR MIT · async-process 2.5.0 Apache-2.0 OR MIT · async-recursion 1.2.0 MIT OR Apache-2.0 · async-signal 0.2.14 Apache-2.0 OR MIT ·
async-task 4.7.1 Apache-2.0 OR MIT · async-trait 0.1.92 MIT OR Apache-2.0 · block-padding 0.3.3 MIT OR Apache-2.0 · blocking 1.7.0 Apache-2.0 OR MIT ·
cbc 0.1.2 MIT OR Apache-2.0 · cfg_aliases 0.2.2 MIT · cipher 0.4.4 MIT OR Apache-2.0 · concurrent-queue 2.5.0 Apache-2.0 OR MIT ·
dbus-secret-service 4.1.0 MIT OR Apache-2.0 · endi 1.1.1 MIT · enumflags2 0.7.12 MIT OR Apache-2.0 · enumflags2_derive 0.7.12 MIT OR Apache-2.0 ·
event-listener 5.4.2 Apache-2.0 OR MIT · event-listener-strategy 0.5.4 Apache-2.0 OR MIT · futures-lite 2.6.1 Apache-2.0 OR MIT ·
getrandom 0.2.17 MIT OR Apache-2.0 · hermit-abi 0.5.3 MIT OR Apache-2.0 · hkdf 0.12.4 MIT OR Apache-2.0 · hmac 0.12.1 MIT OR Apache-2.0 ·
inout 0.1.4 MIT OR Apache-2.0 · keyring 3.6.3 MIT OR Apache-2.0 · linux-raw-sys 0.12.1 Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT ·
nix 0.29.0 MIT · num 0.4.3, num-bigint 0.4.8, num-complex 0.4.6, num-integer 0.1.47, num-iter 0.1.46, num-rational 0.4.2 MIT OR Apache-2.0 ·
ordered-stream 0.2.0 MIT OR Apache-2.0 · parking 2.2.1 Apache-2.0 OR MIT · piper 0.2.5 MIT OR Apache-2.0 · polling 3.11.0 Apache-2.0 OR MIT ·
ppv-lite86 0.2.21 MIT OR Apache-2.0 · rand 0.8.8, rand_chacha 0.3.1, rand_core 0.6.4 MIT OR Apache-2.0 ·
rustix 1.1.6 Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT · secret-service 4.0.0 MIT OR Apache-2.0 · sha1 0.10.7 MIT OR Apache-2.0 ·
static_assertions 1.1.0 MIT OR Apache-2.0 · subtle 2.6.1 BSD-3-Clause · tempfile 3.27.0 MIT OR Apache-2.0 · uds_windows 1.2.1 MIT ·
windows-sys 0.52.0 / 0.59.0, windows-targets 0.52.6, windows_{aarch64_gnullvm,aarch64_msvc,i686_gnu,i686_gnullvm,i686_msvc,x86_64_gnu,x86_64_gnullvm,x86_64_msvc} 0.52.6 MIT OR Apache-2.0 ·
xdg-home 1.3.0 MIT · zbus 4.4.0, zbus_macros 4.4.0, zbus_names 3.0.0 MIT · zerocopy 0.8.63, zerocopy-derive 0.8.63 BSD-2-Clause OR Apache-2.0 OR MIT ·
zeroize 1.9.1, zeroize_derive 1.5.0 Apache-2.0 OR MIT · zvariant 4.2.0, zvariant_derive 4.2.0, zvariant_utils 2.1.0 MIT.

New in the default build (28): aes, block-buffer, block-padding, cbc, cipher, cpufeatures, crypto-common, dbus 0.9.12, dbus-secret-service, digest,
fastrand, generic-array, hkdf, hmac, inout, keyring, libdbus-sys 0.2.7, log, num (+5 num-*), sha2, subtle, zeroize, zeroize_derive. `dbus` and
`libdbus-sys` (Apache-2.0/MIT) were already in the lock (Tauri tree). The `zbus`/`secret-service` lines are lock-only (optional deps of
`keyring`, not compiled by these features).

Stop-rule check: 74 new (in 60..90); every licence allowed; the system library the build needs (`libdbus-1-dev`, pkg-config `dbus-1` 1.16.2)
is installed; no other crate needed. **No stop condition holds.** Note: the options file named `zbus` as the main part; with
`sync-secret-service` the compiled path is `dbus-secret-service` over the C `libdbus-1` (system library, already installed).

*Connector write routes today* (temp copy of `parity/store-rich/store-v11.sqlite`, `env -i` with temp `HOME`, `EASY_PERSONAL_HOME`,
`EASY_PERSONAL_AGENT_ROOT`, `PATH=/usr/bin:/bin`, `serve --no-ingest`, guarded requests with the token):

```
date: 2026-11-13T14:42:56Z  binary: .../T050/rust/target/debug/easy-personal-rs  commit: 983f7b1
POST /api/connectors                             -> 405
POST /api/connectors/slack-acme/alias            -> 404
POST /api/connectors/slack-acme/move             -> 404
DELETE /api/connectors/slack-acme                -> 404
POST /api/connectors/legacy                      -> 404
POST /api/sync                                   -> 405
GET /api/connectors                              -> 200  {"connectors":[{"category":"Email","claude_ai_connector":true,...
$ date -u; ls target/debug/easy-personal-launch; grep -n '\[\[bin\]\]' crates/app/Cargo.toml
2026-11-13T14:43:00Z
ls: cannot access 'target/debug/easy-personal-launch': No such file or directory
(no [[bin]] section; exit 1)
```

**Step 0 addendum** (Supervisor decision 2026-11-13: the pure-Rust backend; captured 2026-11-13T16:31Z..16:40Z on branch T050 at `7b546ea`).
The first measurement above used `sync-secret-service`, which is `dbus-secret-service` over the C `libdbus-1` (both binaries loaded
`libdbus-1.so.3` and `libsystemd.so.0`). The allowed change is now exactly:

```
keyring = { version = "=3.6.3", default-features = false, features = ["async-secret-service", "tokio", "crypto-rust"] }
```

```
$ cargo build
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 24.75s
$ grep -c '^name = ' Cargo.lock        # main (983f7b1): 462
538                                    # 76 new lock entries against main (70..90), 0 removed
$ git diff --stat 7b546ea -- rust/Cargo.lock
 rust/Cargo.lock | 34 ++++++++++++++++++++++++++++++++++   # + async-executor 1.14.1, async-fs 2.2.0 against the first change
$ cargo tree -p easy-personal-rs -e normal --prefix none | sort -u | wc -l
175                                    # main: 104
$ cargo tree -p keyring -e normal --depth 1
keyring v3.6.3
├── log v0.4.35
├── secret-service v4.0.0
└── zbus v4.4.0
$ ldd target/debug/easy-personal-rs
linux-vdso.so.1 (0x00007f35e3833000)
	libgcc_s.so.1 => /usr/lib/x86_64-linux-gnu/libgcc_s.so.1 (0x00007f35e1dbb000)
	libm.so.6 => /usr/lib/x86_64-linux-gnu/libm.so.6 (0x00007f35e1c95000)
	libc.so.6 => /usr/lib/x86_64-linux-gnu/libc.so.6 (0x00007f35e1a00000)
	/lib64/ld-linux-x86-64.so.2 (0x00007f35e3835000)
$ ldd target/debug/easy-personal-launch
linux-vdso.so.1 (0x0000754458c93000)
	libgcc_s.so.1 => /usr/lib/x86_64-linux-gnu/libgcc_s.so.1 (0x0000754458c46000)
	libm.so.6 => /usr/lib/x86_64-linux-gnu/libm.so.6 (0x00007544582da000)
	libc.so.6 => /usr/lib/x86_64-linux-gnu/libc.so.6 (0x0000754458000000)
	/lib64/ld-linux-x86-64.so.2 (0x0000754458c95000)
```

New lock entries against main, with licence (`cargo metadata --locked`); every one has MIT or Apache-2.0 or BSD in its expression, no
other licence: aes 0.8.4 MIT OR Apache-2.0 · async-broadcast 0.7.2 MIT OR Apache-2.0 · async-channel 2.5.0 Apache-2.0 OR MIT · async-executor 1.14.1 Apache-2.0 OR MIT · async-fs 2.2.0 Apache-2.0 OR MIT · async-io 2.6.0 Apache-2.0 OR MIT · async-lock 3.5.0 Apache-2.0 OR MIT · async-process 2.5.0 Apache-2.0 OR MIT · async-recursion 1.2.0 MIT OR Apache-2.0 · async-signal 0.2.14 Apache-2.0 OR MIT · async-task 4.7.1 Apache-2.0 OR MIT · async-trait 0.1.92 MIT OR Apache-2.0 · block-padding 0.3.3 MIT OR Apache-2.0 · blocking 1.7.0 Apache-2.0 OR MIT · cbc 0.1.2 MIT OR Apache-2.0 · cfg_aliases 0.2.2 MIT · cipher 0.4.4 MIT OR Apache-2.0 · concurrent-queue 2.5.0 Apache-2.0 OR MIT · dbus-secret-service 4.1.0 MIT OR Apache-2.0 · endi 1.1.1 MIT · enumflags2 0.7.12 MIT OR Apache-2.0 · enumflags2_derive 0.7.12 MIT OR Apache-2.0 · event-listener 5.4.2 Apache-2.0 OR MIT · event-listener-strategy 0.5.4 Apache-2.0 OR MIT · futures-lite 2.6.1 Apache-2.0 OR MIT · getrandom 0.2.17 MIT OR Apache-2.0 · hermit-abi 0.5.3 MIT OR Apache-2.0 · hkdf 0.12.4 MIT OR Apache-2.0 · hmac 0.12.1 MIT OR Apache-2.0 · inout 0.1.4 MIT OR Apache-2.0 · keyring 3.6.3 MIT OR Apache-2.0 · linux-raw-sys 0.12.1 Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT · nix 0.29.0 MIT · num 0.4.3 MIT OR Apache-2.0 · num-bigint 0.4.8 MIT OR Apache-2.0 · num-complex 0.4.6 MIT OR Apache-2.0 · num-integer 0.1.47 MIT OR Apache-2.0 · num-iter 0.1.46 MIT OR Apache-2.0 · num-rational 0.4.2 MIT OR Apache-2.0 · ordered-stream 0.2.0 MIT OR Apache-2.0 · parking 2.2.1 Apache-2.0 OR MIT · piper 0.2.5 MIT OR Apache-2.0 · polling 3.11.0 Apache-2.0 OR MIT · ppv-lite86 0.2.21 MIT OR Apache-2.0 · rand 0.8.8 MIT OR Apache-2.0 · rand_chacha 0.3.1 MIT OR Apache-2.0 · rand_core 0.6.4 MIT OR Apache-2.0 · rustix 1.1.6 Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT · secret-service 4.0.0 MIT OR Apache-2.0 · sha1 0.10.7 MIT OR Apache-2.0 · static_assertions 1.1.0 MIT OR Apache-2.0 · subtle 2.6.1 BSD-3-Clause · tempfile 3.27.0 MIT OR Apache-2.0 · uds_windows 1.2.1 MIT · windows-sys 0.52.0 MIT OR Apache-2.0 · windows-sys 0.59.0 MIT OR Apache-2.0 · windows-targets 0.52.6 MIT OR Apache-2.0 · windows_aarch64_gnullvm 0.52.6 MIT OR Apache-2.0 · windows_aarch64_msvc 0.52.6 MIT OR Apache-2.0 · windows_i686_gnu 0.52.6 MIT OR Apache-2.0 · windows_i686_gnullvm 0.52.6 MIT OR Apache-2.0 · windows_i686_msvc 0.52.6 MIT OR Apache-2.0 · windows_x86_64_gnu 0.52.6 MIT OR Apache-2.0 · windows_x86_64_gnullvm 0.52.6 MIT OR Apache-2.0 · windows_x86_64_msvc 0.52.6 MIT OR Apache-2.0 · xdg-home 1.3.0 MIT · zbus 4.4.0 MIT · zbus_macros 4.4.0 MIT · zbus_names 3.0.0 MIT · zerocopy 0.8.63 BSD-2-Clause OR Apache-2.0 OR MIT · zerocopy-derive 0.8.63 BSD-2-Clause OR Apache-2.0 OR MIT · zeroize 1.9.1 Apache-2.0 OR MIT · zeroize_derive 1.5.0 Apache-2.0 OR MIT · zvariant 4.2.0 MIT · zvariant_derive 4.2.0 MIT · zvariant_utils 2.1.0 MIT.

Stop-rule check: 76 new (in 70..90), every licence permissive, no system library, no other crate or feature. **No stop condition holds.**
`zbus` (feature `tokio`) blocks on a private runtime of its own; every real-backend call now runs on a plain thread
(`keychain::off_runtime`, unit test `the_real_backend_wrapper_runs_off_any_tokio_runtime`).

**AFTER**: 2026-11-14, `cargo test -p easy-personal-rs --offline --test connector_writes --test launcher --test connector_parity` on `1b142db` (the routes that answered 405/404 before are now served and guarded):

```
test every_connector_case_equals_what_python_answered_wrote_and_ran ... ok
test the_launcher_reads_pythons_items_and_answers_as_launch_main ... ok
test result: ok. 2 passed; 0 failed
test every_new_route_is_guarded_and_a_refused_request_changes_nothing ... ok
test a_body_over_one_mib_is_413_and_the_wrong_methods_are_405 ... ok
test two_writes_at_once_run_one_after_the_other ... ok
test a_claude_that_cannot_run_removes_no_stored_connector ... ok
test result: ok. 10 passed; 0 failed
test exec_puts_the_secret_in_the_providers_env_never_its_argv_and_forwards_io_and_exit_code ... ok
test exec_adds_less_than_200_ms ... ok (fake keychain only)
test result: ok. 6 passed; 0 failed
```

**DELTA**: the Rust app can now add, remove, move and alias connectors (secrets in the system keychain), run `connector sync` and legacy adopt through the `claude` CLI, and the `easy-personal-launch` binary starts a provider with its secret in the environment; 40 oracle cases equal what Python answered, wrote and ran.

**WITNESS**: Supervisor session, 2026-11-14: re-ran the workspace tests, clippy, fmt and the leak check on `1b142db` itself, and checked the fix round (4 commits, clean tree, goldens and `writes.rs` unchanged). Not witnessed: the real Secret Service (every test uses the file fake or a missing bus), so keychain interop with Python (AC9) and the launcher's 200 ms limit against the real keychain are unverified. `memory/event-trace/T050.jsonl` was not available.
