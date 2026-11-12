# TASK_REVIEW — T054: Rust T048 review fixes

> Sibling of `tasks/TASK_GUIDE_T054.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/tests/writes.rs` (exact `cases == 262`, `requests == 355`, HTTP hostile test), `rust/crates/core/tests/writes_parity.rs` (`cases == 44`, chain depth 30, non-UTF-8), unit tests in `rust/crates/app/src/writes.rs` (`prepare_runs_before_the_write_lock_is_taken`, 95,000-key `key_order`) |
| Verification command run | ☑ pass | Author, commit c847fc8, three TZ runs: 607 passed, 1 failed, 26 ignored each; the one failure is the known `workspaces_parity::edge_store_python_values`. Independent reviewer, one run TZ unset: 607 passed, 1 failed, 26 ignored; `leak check: 0 hits over 42 canaries, 236 surfaces`; clippy -D warnings (default and `--features window`) clean; fmt exit 0. `writes oracle: 262 cases, 355 requests, 0 different`. |
| Negative cases hold | ☑ pass | Author red runs on the T048 sources: chain test `no answer in 10 s`, non-UTF-8 test FAILED, HTTP test `no status line in ""` after 30.8 s. Mutations: `Vec::contains` back gave the 95,000-key test failing (42.3 s); `prepare()` moved under the lock gave `prepare_runs_before_the_write_lock_is_taken` failing; last case dropped from `cases.json` gave `cases left: 261 right: 262`. The independent reviewer did not re-run these; it reasoned from the diff and the BEFORE capture and ran a 24,000-query differential fuzz of `resolve_path` against `Path.resolve()`: 0 different. |
| verify | ☑ pass | Independent Stage 5 run by qa-expert on c847fc8 (T054 on 66e94a6), 2026-11-12, `env -u TZ`, fixtures and temp dirs only. (1) Guide command: build ok; `cargo test --workspace --no-fail-fast` 606 passed, 2 failed, 26 ignored (known `workspaces_parity::edge_store_python_values` plus `writes_coexist::writes_and_an_ingest_share_the_wal_store_without_a_lock_error`: `stdout is one JSON value: EOF` = the ingest child printed nothing); `leak check: 0 hits over 42 canaries, 236 surfaces`; `writes oracle: 262 cases, 355 requests, 0 different`; clippy default and `--features window` exit 0; fmt exit 0. The coexist test is intermittent under host load (load avg 4-9 from other agents): re-run alone 30+ times, about 7 failures, of two kinds: a write answered 503 `the store is busy (another process is writing); try again in a moment` (writes_coexist.rs:84/96/103) and the ingest child exiting with no stdout; the other runs passed (57-65 rounds, 240000 events). Not shown to be pre-existing (an old-commit build was attempted and aborted: /tmp full); T054 did not change locking, so cause unproven. (2) `serve` over a temp copy of store-rich, `env -i`: POST workspaces 201 id 4; PATCH /2 description 200 `pending:false`; DELETE /2 confirm 200 `deleted:2`; POST accounts `/nonexistent` 400 `not an agent folder: it must be an existing directory`; confirm {label:L} 200 status `missing` label L; ignore 200 status `ignored`; PUT workspaces 400 `only confirmed Claude accounts pick workspaces`; each 3-8 ms. Store after (read-only sqlite): workspaces 1,3,4 (acme gone), repos 4 -> 2, account_workspaces 1 -> 0, account 1 'L' ignored, account 2 unchanged: as expected. T054: depth-30 symlink chain as a repo path: 201 in 0.009 s; 1,045,001-byte PATCH with 95,000 distinct keys: 400 `unknown field` in 0.27 s, store unchanged; server stopped, serve.err empty. Evidence rows' claims match, except the all-green claim for the full suite. Notes: fail, because one step (the guide's test run) showed a failure beyond the known one; every other step (oracle, leak, lint, serve routes, hostile cases) passed. **Supervisor update, same day (result changed from fail to pass; the QA run above stays as recorded):** the `writes_coexist` failure is load-induced and predates T054. On a quiet host the T054 tree passed it 10 of 10; under identical synthetic load (12 busy loops) the base commit 66e94a6 failed 3 of 8 and c847fc8 failed 0 of 8. Full guide command re-run on c847fc8 on a quiet host, TZ unset: 607 passed, 1 failed (the known `workspaces_parity::edge_store_python_values`), 26 ignored. Runtime drive through the built `serve` binary passed in the QA run (7 write requests, depth-30 symlink chain 0.009 s, 95,000-key PATCH 400 in 0.27 s). verify — pass. Open item: `writes_coexist` can fail on a loaded host (the designed 503 busy text after the 5 s timeout); follow-up T055 should make it print the ingest child's stderr and tolerate load. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Independent read-only reviewer on `git diff 66e94a6 c847fc8` (12 files): no P0, P1 or P2; three P3s, all docs (README item 10 placement, status table omits the 500 and NoHome 503, parity README wording). Skipped: T052 and T053 files (untouched). |
| Full smoke suite still green (no regression) | ☑ pass | Same runs: 607 passed, 1 failed (known). |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier not run for this task. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | No UI file changed (Rust routes, docs and tests only). |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | No UI file changed (Rust routes, docs and tests only). |
| **UI: Responsiveness at target viewports** | ☑ N/A | No UI file changed (Rust routes, docs and tests only). |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured on the unchanged T048 code (HEAD `66e94a6`, `target/debug/easy-personal-rs`, debug build) with a
throwaway script outside the repo that starts the built `serve` binary over a temp copy of `store-rich` and times one request
(no tracked file had been changed). Terminal capture, `date -u` as the host clock printed it:

```
$ date -u +%FT%TZ                          # 2026-11-12T05:14:11Z
$ timeout 60 python3 hostile.py chain 18   # POST /api/workspaces, repos: ["<tmp>/chain/L0"], L_i -> "L_{i+1}/L_{i+1}", L_18 -> "."
depth 18 (201, '{"workspace":{...,"name":"hostile","repos"', 4.04104117299994)        # 4.04 s wall, WRITE_LOCK held
$ timeout 60 python3 hostile.py patch      # PATCH /api/workspaces/2, 1 MiB body, 95,000 distinct keys "k00000":1,...
bytes 1045001 keys 95000 (400, '{"error":"unknown field"}', 36.87778788000014)        # 36.9 s wall on one async worker
$ date -u +%FT%TZ                          # 2026-11-12T05:14:52Z
$ date -u +%FT%TZ                          # 2026-11-12T06:57:30Z
$ timeout 20 python3 hostile.py chain 30
exit=124 (124 = timeout killed it)         # depth 30 never answers (2^30 expansions)
```

**AFTER**: the same script against the rebuilt `serve` binary (debug build), same host, `date -u` as printed:

```
$ date -u +%FT%TZ                          # 2026-11-12T07:21:19Z
$ timeout 60 python3 hostile.py chain 18
depth 18 (201, '{"workspace":{...,"name":"hostile","repos"', 0.005046042000685702)    # was 4.04 s
$ timeout 60 python3 hostile.py chain 30
depth 30 (201, '{"workspace":{...,"name":"hostile","repos"', 0.005360771001505782)    # was: never answered (timeout, exit 124)
$ timeout 60 python3 hostile.py patch
bytes 1045001 keys 95000 (400, '{"error":"unknown field"}', 0.2557988700009446)         # was 36.9 s
$ date -u +%FT%TZ                          # 2026-11-12T07:21:20Z
```

New tests, red on the T048 code and green now (red runs: the three source files restored from `66e94a6` for the integration tests, a
mutation for the two unit tests; every red was run under `timeout`):
`crates/core/tests/writes_parity.rs` (`a_chain_of_links_that_doubles_at_every_hop_is_resolved_in_linear_time`: "no answer in 10 s",
`a_repo_that_resolves_to_a_path_that_is_not_utf8_is_refused`: FAILED), `crates/app/tests/writes.rs`
(`a_hostile_link_chain_and_a_body_of_95000_keys_are_answered_in_under_a_second`: "no status line" after 30.8 s), `crates/app/src/writes.rs`
(`prepare_runs_before_the_write_lock_is_taken`: "the write lock is held while the request prepares" with `prepare()` moved under the lock;
`ninety_five_thousand_keys_are_scanned_in_linear_time`: 42.3 s with `Vec::contains` put back). The exact counts: dropping the last case of
`cases.json` fails `assertion left == right failed: cases  left: 261  right: 262` (reverted with `git checkout`).

**DELTA**: a repo path made of a hostile symlink chain or a PATCH body of 95,000 keys is now answered in milliseconds instead of holding the write lock for seconds to forever, and a non-UTF-8 repo path is a clean 400.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T054.jsonl`, never the
implementing agent alone]
