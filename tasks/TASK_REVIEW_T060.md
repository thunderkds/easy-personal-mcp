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
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
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

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
