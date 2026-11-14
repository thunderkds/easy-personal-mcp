# TASK_REVIEW — T060: Rust connector and doctor commands

> Sibling of `tasks/TASK_GUIDE_T060.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | (filled by backend-developer, 2026-11-14, on `b4430d5`) AC1/AC3/AC4: `rust/crates/app/tests/connector_cli_parity.rs` (2 tests; oracle `rust/parity/golden-connector-cli/cases.json` from `rust/parity/gen_connector_cli.py`: 95 cases, 116 steps, 295 `claude` calls; `--check`: `golden-connector-cli is equal to a fresh run`). AC2/AC4 and the T050 "could not run" rule: `rust/crates/app/tests/connector_cli.rs` (7 tests: pty echo off via util-linux `script`, Ctrl-C at the secret prompt, no hang with no terminal and no stdin, secret never in `claude` argv/stdout/stderr/store, a `claude` that cannot run removes and forgets nothing, refused names reach neither keychain nor `claude`, the `account use` hint's command exists). AC3: `rust/crates/app/tests/doctor_cli.rs` (4 tests). AC5: `rust/parity/leak_rust.py` `connector_cli` (29 calls). Shared world: `rust/crates/app/tests/connector_world/mod.rs` (T050's `connector_parity.rs` moved onto it, still green). `test result: ok. 2 passed` (connector_cli_parity), `ok. 7 passed` (connector_cli), `ok. 4 passed` (doctor_cli) |
| Verification command run | ☑ pass | (backend-developer, 2026-11-14, `--offline`) `cargo test --workspace --no-fail-fast`: TZ unset `84 suites, 882 passed, 0 failed`; `TZ=Pacific/Kiritimati` `84 suites, 882 passed, 0 failed`; `TZ=Pacific/Pago_Pago` `84 suites, 882 passed, 0 failed` (no `ingest_coexist` flake). `env -u CLAUDE_CONFIG_DIR parity/.venv/bin/python parity/leak_rust.py`: `leak check: 0 hits over 42 canaries, 778 surfaces, 1755689 bytes` (`check cli add stored the typed secret in the keychain  ok`, `check cli claude failed, its stderr not shown ok`, `check cli account path only in CLAUDE_CONFIG_DIR ok`, `check cli remove deleted the keychain items ok`, `check cli connector/doctor exit codes as expected ok (29 calls, one process each)`). `cargo clippy --workspace --all-targets -- -D warnings`: exit 0; `--features window`: `Finished`, exit 0. `cargo fmt --check`: exit 0. `git diff --stat main...HEAD -- rust/Cargo.toml rust/Cargo.lock`: empty; `git status --short -- src tests web`: empty |
| Negative cases hold | ☑ pass | Oracle refusals equal Python's (unknown provider/workspace/connector/account, orphan, bad/long/empty alias, `--default` with an alias, held provider, no backend, no launcher, failing `claude`, skipped account, end of input, `aborted`). Rust-only: a `claude` that cannot run is never "no entry" (`connector remove slack` exit 1 `the \`claude\` CLI could not run ...; nothing was removed.`, sync/remove `skipped (\`claude\` could not run)`, store and keychain unchanged; `doctor` `FAIL ... could not run ...; not checked`); names `a/b`, `..`, `Slack`, 65 chars, empty: exit 2, no `claude` call, keychain unchanged; a locked keychain is a `doctor` FAIL line; a missing store is refused, nothing created |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `git diff main...T060` (20 files, ~10.6k lines; `lane_check`: C3 FITS; the T059 test fix `e6f1daf` is in the branch base). Two independent read-only reviewers (correctness and tests; security) read `connector_cmd.rs`, `doctor_cmd.rs`, the `cli_cmds.rs`, `connector_writes.rs`, `legacy.rs`, `keychain.rs`, core `sync.rs`/`accounts.rs`/`workspaces.rs` changes, the tests and the oracle generator; `golden-connector-cli/cases.json` only summarised. Findings: P0 0, P1 0, P2 2, P3 7; security-review: no findings. Open, not fixed: P2 stdin `\r\n` in a secret differs from real Python (README item 10 corrected to say so; the oracle reads stdin with universal newlines and hides it); P2 no echo-off when `/dev/tty` cannot be opened but stdin is a terminal (Python keeps echo off); P3 ignored `tcsetattr` result, no test of echo restore, `legacy.rs` still reads "could not run" as "no entry" (disclosed item 7, non-destructive), `doctor` FAIL noise per unconfigured id with an unusable `claude`, test hard-codes `/usr/bin/script`, a dead default `Keychain::name()` |
| Full smoke suite still green (no regression) | ☑ pass | Supervisor, 2026-11-14, on `157040b`: `cargo test --workspace --offline --no-fail-fast`: exit 0, 84 suites ok, 882 passed, 0 failed, 29 ignored (the `ingest_coexist` flake did not occur) |
| **Companion score (easy-verifier)** | ☑ pass (advisory) | `score` scope=`changes` ref=`main` task=T060, change_kind confirmed `feature`: `overall` 41 (4 of 13 dimensions rated: blast-radius 46, requirement-fidelity 33, security 30, test-strategy 55); `change_risk` abstained (`all_metrics_abstained`). Low signal: `security` 30 comes from lockfile and manifest redaction hits, not from the diff. report: /home/hungnguyenhuu/.claude-hung2/projects/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp-rust/e64d19ef-48c3-4f22-9423-6b479756ddac/tool-results/mcp-easy-verifier-score-1794629674479.txt |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | pure backend task (CLI commands), no UI component |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | pure backend task |
| **UI: Responsiveness at target viewports** | ☑ N/A | pure backend task |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured by backend-developer, 2026-11-14T03:37Z, branch T060 at `e6f1daf`, before any implementation file;
`env -i` with a temp `HOME`, `EASY_PERSONAL_HOME` (a copy of `parity/store-rich/store-v11.sqlite`), `EASY_PERSONAL_AGENT_ROOT`,
the debug fake keychain `EASY_PERSONAL_TEST_KEYCHAIN=<tmp>/kc.json`, the oracle's fake `claude` first on `PATH`, a bogus
`DBUS_SESSION_BUS_ADDRESS`; stdin `/dev/null`):

```
date: 2026-11-14T03:37:34Z  commit: e6f1daf  binary: target/debug/easy-personal-rs
$ easy-personal-rs connector list
error: unrecognized subcommand 'connector'

Usage: easy-personal-rs <COMMAND>

For more information, try '--help'.
(exit 2)
$ easy-personal-rs connector add slack --workspace acme
error: unrecognized subcommand 'connector'

Usage: easy-personal-rs <COMMAND>

For more information, try '--help'.
(exit 2)
$ easy-personal-rs connector sync --yes
error: unrecognized subcommand 'connector'

Usage: easy-personal-rs <COMMAND>

For more information, try '--help'.
(exit 2)
$ easy-personal-rs doctor
error: unrecognized subcommand 'doctor'

Usage: easy-personal-rs <COMMAND>

For more information, try '--help'.
(exit 2)
ls: cannot access '<tmp>/claude.log': No such file or directory     # no `claude` call was made
ls: cannot access '<tmp>/kc.json': No such file or directory        # no keychain item was written
```

**AFTER**:

```
# captured 2026-11-14T04:23:50Z on 157040b, temp copy of rust/parity/store-rich, temp HOME and agent root, PATH=/usr/bin:/bin (no claude), bogus session bus, file-fake keychain (first 3 lines of each)
$ easy-personal-rs connector list
Email
  gmail Gmail | claude.ai connector: yes (https://claude.ai/settings/connectors)
    fields: OAuth client keys file (gcp-oauth.keys.json)
exit=0
$ easy-personal-rs connector add slack --workspace acme
Warning: Password input may be echoed.
Bot token (xoxb-...): easy-personal-rs: input ended at the prompt; nothing was written
exit=2
$ easy-personal-rs connector sync --yes
Claude default: skipped (folder missing)
Claude work: skipped (folder missing)
nothing to change.
exit=0
$ easy-personal-rs doctor
ok keychain: FileKeychain
FAIL claude: not on PATH
ok launcher: /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/T060/rust/target/debug/easy-personal-launch
exit=1
```

**DELTA**: the Rust binary now runs `connector list|add|remove|sync|move|alias|adopt-legacy` and `doctor` with Python's stdout, stderr and exit codes (95 oracle cases, 116 steps, 295 `claude` calls), so the CLI no longer needs Python for connectors.

**WITNESS**: Supervisor session, 2026-11-14: re-ran the AFTER commands (04:23Z), the workspace tests (882 passed, 0 failed), clippy x2 (default and `--features window`), fmt and the leak check (`leak check: 0 hits over 42 canaries, 778 surfaces`) on `157040b`, and checked that `Cargo.toml`, `Cargo.lock`, `src`, `tests` and `web` have no diff. Not witnessed: the real Secret Service, a real `claude`, a real terminal prompt (one test uses util-linux `script` as a pseudo-terminal). `memory/event-trace/T060.jsonl` was not available.
