# TASK_REVIEW — T048: Rust store-only write routes: workspaces (POST, PATCH, DELETE) and accounts (add, confirm, ignore, workspaces)

> Sibling of `tasks/TASK_GUIDE_T048.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/tests/writes.rs` (writes oracle 262 cases / 355 requests, 0 different), `rust/crates/core/tests/writes_parity.rs` (37 path cases vs Python), `rust/crates/app/tests/writes_coexist.rs` (writes while ingesting), unit tests in `rust/crates/app/src/writes.rs` |
| Verification command run | ☑ pass | Author, commit 66e94a6, TZ unset / Pacific/Kiritimati / Pacific/Pago_Pago: 602 passed, 1 failed, 26 ignored each; the one failure is the known `workspaces_parity::edge_store_python_values` (worktree under `.claude/`); `leak check: 0 hits over 42 canaries, 236 surfaces`; clippy (default and `--features window`) and fmt clean. Not re-run by the Supervisor on 66e94a6; see T054 for the final tree. |
| Negative cases hold | ☑ pass | Author: mutation of one refusal text gave `4 different` in the oracle, reverted. Stage 4 reviews: one required security P2 (exponential `resolve_path`), reproduced from source; fixed by T054 (see TASK_REVIEW_T054.md for its red runs). |
| verify | ☑ pass | Independent Stage 5 run by qa-expert on c847fc8 (T054 on 66e94a6), 2026-11-12, `env -u TZ`, fixtures and temp dirs only. (1) Guide command: build ok; `cargo test --workspace --no-fail-fast` 606 passed, 2 failed, 26 ignored (known `workspaces_parity::edge_store_python_values` plus `writes_coexist::writes_and_an_ingest_share_the_wal_store_without_a_lock_error`: `stdout is one JSON value: EOF` = the ingest child printed nothing); `leak check: 0 hits over 42 canaries, 236 surfaces`; `writes oracle: 262 cases, 355 requests, 0 different`; clippy default and `--features window` exit 0; fmt exit 0. The coexist test is intermittent under host load (load avg 4-9 from other agents): re-run alone 30+ times, about 7 failures, of two kinds: a write answered 503 `the store is busy (another process is writing); try again in a moment` (writes_coexist.rs:84/96/103) and the ingest child exiting with no stdout; the other runs passed (57-65 rounds, 240000 events). Not shown to be pre-existing (an old-commit build was attempted and aborted: /tmp full); T054 did not change locking, so cause unproven. (2) `serve` over a temp copy of store-rich, `env -i`: POST workspaces 201 id 4; PATCH /2 description 200 `pending:false`; DELETE /2 confirm 200 `deleted:2`; POST accounts `/nonexistent` 400 `not an agent folder: it must be an existing directory`; confirm {label:L} 200 status `missing` label L; ignore 200 status `ignored`; PUT workspaces 400 `only confirmed Claude accounts pick workspaces`; each 3-8 ms. Store after (read-only sqlite): workspaces 1,3,4 (acme gone), repos 4 -> 2, account_workspaces 1 -> 0, account 1 'L' ignored, account 2 unchanged: as expected. T054: depth-30 symlink chain as a repo path: 201 in 0.009 s; 1,045,001-byte PATCH with 95,000 distinct keys: 400 `unknown field` in 0.27 s, store unchanged; server stopped, serve.err empty. Evidence rows' claims match, except the all-green claim for the full suite. Notes: fail, because one step (the guide's test run) showed a failure beyond the known one; every other step (oracle, leak, lint, serve routes, hostile cases) passed. **Supervisor update, same day (result changed from fail to pass; the QA run above stays as recorded):** the `writes_coexist` failure is load-induced and predates T054. On a quiet host the T054 tree passed it 10 of 10; under identical synthetic load (12 busy loops) the base commit 66e94a6 failed 3 of 8 and c847fc8 failed 0 of 8. Full guide command re-run on c847fc8 on a quiet host, TZ unset: 607 passed, 1 failed (the known `workspaces_parity::edge_store_python_values`), 26 ignored. Runtime drive through the built `serve` binary passed in the QA run (7 write requests, depth-30 symlink chain 0.009 s, 95,000-key PATCH 400 in 0.27 s). verify — pass. Open item: `writes_coexist` can fail on a loaded host (the designed 503 busy text after the 5 s timeout); follow-up T055 should make it print the ingest child's stderr and tolerate load. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Three independent read-only reviewers (correctness opus, security opus, testing sonnet) on 66e94a6, scoped to `workspaces.rs`, `accounts.rs`, `writes.rs`, `api.rs`, the write tests and goldens, README and parity README. No P0, no P1; P2s fixed by T054. reports/code-review_T048_20261112T091000.html, reports/security-review_T048_20261112T091100.html (main checkout). Skipped: `sync.rs` and T050 files (not touched). |
| Full smoke suite still green (no regression) | ☑ pass | Same run as above: 602 passed, 1 failed (known, pre-existing on 90b7254). |
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

**BEFORE** (captured 2026-11-12T04:18:04Z by the implementing agent, base `90b7254`, binary `target/debug/easy-personal-rs serve` built from it, before any T048 file existed; `env -i`, temp copy of `rust/parity/store-rich/store-v11.sqlite`, temp `HOME`/`EASY_PERSONAL_HOME`/`EASY_PERSONAL_AGENT_ROOT`, token read from the printed URL; the guide names no Demonstration command, so this shows the guide's seven write routes with a guarded `curl`):

```
--- POST /api/workspaces {"name":"newone"}                 HTTP 405
--- PATCH /api/workspaces/2 {"description":"x"}            HTTP 404
--- DELETE /api/workspaces/2 {"confirm":true}              HTTP 404
--- POST /api/accounts {"path":"/nonexistent"}             HTTP 405
--- POST /api/accounts/1/confirm {"label":"L"}             HTTP 404
--- POST /api/accounts/1/ignore {}                         HTTP 404
--- PUT /api/accounts/1/workspaces {"workspaces":[]}       HTTP 404
```
(the answers had empty bodies: Python answers 201/200/400/404 with JSON for the same requests)

**AFTER** (implementing agent, 2026-11-12, same binary built from the final tree; evidence of the run, not the reviewer's verify):
`cargo test --workspace --no-fail-fast` under TZ unset, Pacific/Kiritimati, Pacific/Pago_Pago: each 602 passed, 1 failed, 26 ignored; the one failure is `workspaces_parity::edge_store_python_values`, which the guide says fails inside a worktree under `.claude/` (not touched by T048).
Oracle: `writes oracle: 262 cases, 355 requests, 0 different` (`crates/app/tests/writes.rs`; mutation of one refusal text -> `4 different`, reverted). Paths: 37 inputs equal Python (`crates/core/tests/writes_parity.rs`).
Concurrency: `writes while ingesting: 49 rounds of 2-3 requests, ingest stored 240000 new events` (`writes_coexist.rs`). `leak_rust.py`: `leak check: 0 hits over 42 canaries, 236 surfaces, 910728 bytes`.
`cargo clippy --workspace --all-targets -- -D warnings` (default and `--features window`) clean; `cargo fmt --check` ok; `git status -- src tests web rust/Cargo.toml rust/Cargo.lock` empty.

**DELTA**: the dashboard can create, edit and delete workspaces and add, confirm, ignore and set the workspaces of agent accounts against the Rust server with Python's statuses and texts, changing only the store; a change that needs the claude CLI is refused with a fixed 409 until T050.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
