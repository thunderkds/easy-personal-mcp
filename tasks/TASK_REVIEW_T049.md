# TASK_REVIEW — T049: Rust live tail engine (core)

> Sibling of `tasks/TASK_GUIDE_T049.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass / ☐ fail | `rust/crates/core/tests/live_parity.rs` (13 tests), oracle `rust/parity/gen_live.py` -> `rust/parity/golden-live/{items,labels,scripts,scripts-2,labels-2}.json` (28 items, 100 + 3 labels, 10 + 6 scripts, 72 + 39 steps, exact counts asserted) |
| Verification command run | ☑ pass / ☐ fail | Run by backend-developer 2026-11-12T15:22:41Z-15:30:13Z at HEAD `00ec78f`, TMPDIR `/var/tmp/t049-verify-20261112T152241/tmp` (removed after). `cargo build && cargo test --workspace --no-fail-fast` TZ unset: build exit 0, 65 suites, passed=674 failed=1 ignored=26, the 1 = `workspaces_parity::edge_store_python_values` (known: checkout under `.claude/`, learnings.md:583). TZ=Pacific/Kiritimati: passed=673 failed=2 ignored=26: the same one plus `a_crontab_of_ten_thousand_lines_with_hostile_lines_is_listed_fast` (timing; re-run alone under Kiritimati: `... ok`; not in this diff). `live_parity` TZ unset / Pacific/Kiritimati / Pacific/Pago_Pago: `ok. 13 passed; 0 failed` each. Leak gate: `check planted sources unchanged ok (sha256 of the tree)`, `leak check: 0 hits over 42 canaries, 248 surfaces, 937243 bytes`. `clippy --workspace --all-targets -- -D warnings` exit 0; same `--features window` exit 0; `cargo fmt --check` exit 0. `git diff 0ad72fb..HEAD --numstat -- rust/parity`: README 25/0, gen_live.py 465/0, items.json 360/0, labels-2.json 17/0, labels.json 536/0, scripts-2.json 1909/0, scripts.json 4050/0 (deletions in any golden folder: 0; scripts/items/labels.json unchanged vs `1ec8e02`). Oracle regenerated into a scratch copy: `diff -r` exit 0, 0 lines. Leftover temp folders: 19 entries in the suite's TMPDIR (other suites' files; live_parity writes under `CARGO_TARGET_TMPDIR` only, all removed); 1 `live-pairing-100k-*` in `rust/target/tmp` left by the deliberate red run of fix 4 (process ended on the 60 s timeout while the worker thread held it), removed by hand. |
| Negative cases hold | ☑ pass / ☐ fail | First round: a changed golden value (attention count, a Bash label) turned `scripts_equal_python_after_every_step` and `labels_equal_python` red. Fix round, red before each fix: accounts `"Claude default"` vs Python `"Work"` (fix 1); `history published again` / 3 events instead of 1 (fix 2); `attempt to subtract with overflow` (fix 3); 100k pairing timed out at 60 s (fix 4); `["projects", "store.sqlite", "store.sqlite-shm", "store.sqlite-wal"]` on a WAL store (fix 5). Mutants of `live.rs`: 14 bound/stage mutants (fix 6) and each of the 19 STAGE_MAP values set to "9" in a scratch copy: 33/33 red, 0 survived. |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass / ☐ fail | No regression from this diff: the only failures are the known worktree-path test (`edge_store_python_values`, fails at base too per learnings.md:583) and one timing test under load that passes alone; see the Verification row. |
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

**BEFORE**: captured by backend-developer in `/home/thunderkds/workspace/pets/easy-personal-mcp/.claude/worktrees/agent-a85418ffb80d79e9c` at base `0ad72fb`, before any T049 commit.

(a) Rust has no live module or route (every `live` hit is an unrelated word):

```
$ date -u; grep -rn "live" rust/crates/core/src rust/crates/app/src | head; ls rust/crates/core/src | grep -i live
2026-11-12T12:16:19Z
rust/crates/core/src/store.rs:175:/// Where the store lives: `$EASY_PERSONAL_HOME/store.sqlite`, else `~/.easy-personal/store.sqlite`
rust/crates/core/src/board.rs:466:    let alive: Vec<&String> = stored.iter().filter(|p| is_dir(p)).collect();
rust/crates/core/src/board.rs:468:        "projects": alive.iter().map(|p| read_project(p)).collect::<Vec<_>>(),
rust/crates/core/src/board.rs:469:        "gone": stored.len() - alive.len(),
rust/crates/core/src/accounts.rs:280:/// account's file lives beside its folder when `CLAUDE_CONFIG_DIR` is unset.
rust/crates/core/src/suggestions.rs:12://! connector rows hold only an id, an alias, a provider id and a workspace name (the secrets live in the keychain).
rust/crates/app/src/mcp.rs:13://! Everything that touches `rmcp` lives in this file, so a later major version of the SDK changes one module.
rust/crates/app/src/display.rs:266:        let live = dir.join("live");
rust/crates/app/src/display.rs:267:        let _listener = UnixListener::bind(&live).unwrap();
rust/crates/app/src/display.rs:268:        assert!(unix_answers(&live, LIMIT));
--- ls rust/crates/core/src | grep -i live:
(grep exit 1)
```

(b) Python's `LiveTail.poll_once` over a temp fixture tree (two lines: a `Skill` `code-review` call, then its failed result), a temp store, frozen clock 2026-01-20T00:01Z; script outside the repo (scratchpad `before.py`, not committed), temp folder removed after:

```
$ date -u; PYTHONPATH=src rust/parity/.venv/bin/python before.py .scratch
2026-11-12T12:16:26Z
{"id": 1, "session_id": "s1", "account": "Claude default", "project": "demo", "ts": "2026-01-20T00:00:00.000000Z", "kind": "tool_use", "tool": "Skill", "is_error": false, "is_sidechain": false, "label": "code-review", "agent": null, "tokens_in": null, "tokens_out": null, "cache_creation": null, "cache_read": null, "cost_usd": null}
{"id": 2, "session_id": "s1", "account": "Claude default", "project": "demo", "ts": "2026-01-20T00:00:00.000000Z", "kind": "usage", "tool": null, "is_error": false, "is_sidechain": false, "label": null, "agent": null, "tokens_in": 10, "tokens_out": 5, "cache_creation": 0, "cache_read": 0, "cost_usd": null}
{"id": 3, "session_id": "s1", "account": "Claude default", "project": "demo", "ts": "2026-01-20T00:00:01.000000Z", "kind": "tool_result", "tool": "Skill", "is_error": true, "is_sidechain": false, "label": null, "agent": null, "tokens_in": null, "tokens_out": null, "cache_creation": null, "cache_read": null, "cost_usd": null}
{"active_window_s": 300, "last_id": 3, "state_version": 1, "open_call_minutes": 10, "sessions": [{"session_id": "s1", "account": "Claude default", "project": "demo", "last_ts": "2026-01-20T00:00:01.000000Z", "tool_calls": 1, "tokens_in": 10, "tokens_out": 5, "cache_creation": 0, "cache_read": 0, "cost_usd": null, "attention": [], "stage": "4", "stage_skill": "code-review"}]}
```

**AFTER**: captured by backend-developer at HEAD `00ec78f`, 2026-11-12T15:22:41Z-15:30:13Z (the combined run, Evidence row "Verification command run"). (a) Rust now has the module: `rust/crates/core/src/live.rs` (`pub mod live;` in `lib.rs`); `api.rs` still serves no `/api/live` (T056). (b) The same Python `LiveTail` is now the oracle: `gen_live.py` runs it over 16 scripts / 111 steps and the Rust engine gives equal events and snapshots after every step:

```
== live_parity TZ=unset: test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.22s
== live_parity TZ=Pacific/Kiritimati: test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.20s
== live_parity TZ=Pacific/Pago_Pago: test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.22s
== oracle regenerated into a scratch copy, diff -r exit 0 (0 diff lines)
leak check: 0 hits over 42 canaries, 248 surfaces, 937243 bytes
```

The BEFORE script's two-line case (a `Skill` `code-review` call, then its failed result) is the first-round "stage" / "pairing" pattern; the Rust engine has no CLI of its own yet (T056 adds the route), so it is shown through the parity test, not a second script.

**DELTA**: The Rust core can now follow Claude transcripts the way Python's live tail does (same events, session totals, kit stage, open calls and attention states, publish-only and read-only), so T056 can serve `/api/live` from Rust.

**Fix round** (review findings after `1ec8e02`, one commit each):

| Finding | Commit |
|---|---|
| P1 NULL account label made every account "Claude default" | `2d0831f` |
| P1 one failed store open re-published all history / used up the warm start | `b571e90` |
| P2 i64 overflow in token sums, growth and `rolled_past` | `212e8c4` |
| P2 `Ordered::pop` O(n) (100k calls ~ 1e10 steps) | `c1ac3ce` |
| P2 read-only claim false for a WAL store (sidecars) | `fe5e3ea` |
| Test gaps: stage values, five bounds, exact counts, 64 MiB trim | `061b22f` |
| P3 hand-coded open flags | `f234f19` |
| Undocumented differences from Python; AC 6 wording | `0da0f61` |
| parity README counts and new golden files | `00ec78f` |
| Stage-map mutation: all 19 values red, no script change needed | (no commit) |
| F1 (P2) core did not compile on macOS (fallback `sys` lacked the flags). No darwin target installed (`rustup target list --installed`: `x86_64-unknown-linux-gnu`); cfg proxy in a scratch copy (fallback `sys` active on Linux): before `error[E0432]: unresolved imports ...sys::O_NOFOLLOW, ...sys::O_NONBLOCK`, after `Finished dev profile` | `40c6be8` |
| F2 (P2) skip path "store opens but the cursor/label query fails" untested; queries ran twice. New tests rename `source_files` away for a step; the `false` mutant (scratch copy): `14 passed; 1 failed` (`a_cursor_query_failing_on_the_first_step_keeps_the_warm_start`), also red against the refactored code's mutant; worktree `15 passed`. The good/broken/good test stays green under the mutant (a steady state does not use the cursor) and guards the skip itself | `b219fd8` |
| F3 (P3) README: history re-published when a root's files come back; zero-account first scan (both as Python, accepted unless the owner overrules) | `1a077e4` |
| F4 (P3) README item 9: Python's pre-`try` open of a file over 64 MiB does not bump `state_version`; Rust always bumps | `9e01842` |

Last re-run (backend-developer, 2026-11-12T15:54:02Z-15:58:05Z, HEAD `9e01842`, TMPDIR `/var/tmp/t049-verify-f-20261112T155402/tmp`, removed after):

```
== live_parity TZ=unset: test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.33s
== live_parity TZ=Pacific/Kiritimati: test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.26s
== live_parity TZ=Pacific/Pago_Pago: test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.75s
== cargo test --workspace --no-fail-fast (TZ unset) exit 101
65 suites, passed=676 failed=1 ignored=26
test edge_store_python_values ... FAILED      (known: worktree under .claude/, learnings.md:583)
leak check: 0 hits over 42 canaries, 248 surfaces, 937511 bytes
== clippy -D warnings exit 0
== clippy --features window -D warnings exit 0
== cargo fmt --check exit 0
== golden deletions vs 0ad72fb: 0; golden-live changes since 00ec78f: 0 lines
== oracle regenerated into a scratch copy, diff -r exit 0 (0 diff lines)
== leftovers: suite TMPDIR 9 entries (0 live-*); regen target 0; rust/target/tmp live-* 0; /var/tmp/t049-* other than this run 0
```

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
