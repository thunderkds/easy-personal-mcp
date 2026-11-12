# TASK_REVIEW — T043: Rust `GET /api/board` and `GET /api/memory-graph` (read-only, bounded file reads)

> Sibling of `tasks/TASK_GUIDE_T043.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/memgraph_parity.rs`, `rust/crates/app/tests/` board and memory-graph route tests, goldens `rust/parity/golden-memgraph/` (12/12 plus a hostile-tree oracle) |
| Verification command run | ☑ pass | Supervisor ran the verification command on the staged T043 tree (identical to a59def6) before unstaging it: everything green. Not re-run on the final merged tree yet. T053's results (below) cover the fixed tree. |
| Negative cases hold | ☑ pass | Stage 4 reviewers reproduced the defects in the T043 sources (backslash drop in `drop_escapes`, `.git` and `commondir` reads that follow symlinks and are unbounded); T053 holds the red evidence for their fixes (see TASK_REVIEW_T053.md). |
| verify | ☑ pass | Independent qa-expert run, 2026-11-12, HEAD b2b7af0, worktree under `.claude/`, `TZ` unset: `cargo build` ok; `cargo test --workspace --no-fail-fast` 632 passed / 2 failed / 26 ignored: the known `workspaces_parity::edge_store_python_values` (worktree under `.claude/`) and `ingest_coexist::serve_mcp_and_store_info_answer_while_a_rust_ingest_runs` (`StorageFull`, `/tmp` at 95% from parallel agents; passes alone: `1 passed`); `memgraph_parity` 27/27, `--lib` 36/36, `board_memgraph` 3/3; `leak_rust.py` `leak check: 0 hits over 42 canaries, 212 surfaces, 913260 bytes`; clippy default and `--features window` clean, `cargo fmt --check` clean. `serve` over a temp copy of `store-rich`: board and memory-graph routes 200/400 as in the AFTER block, 403/403/403/405 for no token / bad Host / bad Origin / POST; FIFO `.git` and a 20 MB `commondir` answered in 0.045 s and 0.025 s. See Demonstration AFTER. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Three independent read-only reviewers (correctness opus, security opus, testing sonnet) on a59def6: P1s and P2s found, all fixed by T053 and re-verified independently with no P0, P1 or P2 (reports/code-review_T043_20261112T090000.html, reports/security-review_T043_20261112T090100.html in the main checkout). Skipped: Python source (oracle only). |
| Full smoke suite still green (no regression) | ☑ pass | Independent QA run on b2b7af0, TZ unset: 632 passed, 2 failed, 26 ignored. One failure is the known `workspaces_parity::edge_store_python_values`; the other, `ingest_coexist::serve_mcp_and_store_info_answer_while_a_rust_ingest_runs`, failed with `StorageFull` because /tmp was at 95%, and passed when re-run alone (1 passed). The author's TZ runs were 633 passed, 1 failed. T053's own tests: memgraph_parity 27/27, lib 36/36, board_memgraph 3/3. |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier not run for this task. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | No UI file changed (Rust routes, docs and tests only). |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | No UI file changed (Rust routes, docs and tests only). |
| **UI: Responsiveness at target viewports** | ☑ N/A | No UI file changed (Rust routes, docs and tests only). |

---

## Notes

- `git` is spawned (T053 AC 8): `memgraph.rs` `resolve_project` -> `metrics::repo_of` runs `git` for a Claude transcript's `cwd` that is a folder, exactly where Python does (`discovery.resolve_project`), with the mitigations Rust already has (`core.fsmonitor` off, `GIT_OPTIONAL_LOCKS=0`, stdin null, a timeout, a kill). Decided in T053: kept, as Python parity (also in `rust/README.md` "Decided for you"). `GIT_NO_LAZY_FETCH=1` is T052's AC 5a and is not set on this base.

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-11T16:02:58Z, before any file of T043 was created; `rust/target/debug/easy-personal-rs serve` over a temp copy of `rust/parity/store-rich`, `HOME`/`EASY_PERSONAL_HOME`/`EASY_MEMORY_HOME` in a temp folder, `TZ=UTC`, GET with the per-run token):
```
GET /api/kit/reports -> 200 b'{"workspaces":[{"folders":[],"name":"acme"},{"folders":[],"name":"personal"}]}'   (control: a served route)
GET /api/board -> 404 b''
GET /api/board?workspace=acme -> 404 b''
GET /api/memory-graph -> 404 b''
GET /api/memory-graph?project=nope -> 404 b''
```

**AFTER** (captured 2026-11-12T08:13:42Z by the independent Stage 5 verifier on HEAD b2b7af0 (T053 on T043), `rust/target/debug/easy-personal-rs serve` over a temp copy of `rust/parity/store-rich`, `env -i` with `HOME`/`EASY_PERSONAL_HOME`/`EASY_MEMORY_HOME`/`EASY_PERSONAL_AGENT_ROOT` in a temp folder, `TZ=UTC`, GET with the per-run token; same five requests as BEFORE, plus the 400 text). The fixture's stored folders (`/work/rich/...`) do not exist on this machine, so the lists are empty and `gone` counts them (the goldens' `gone: 3` with live folders is proved by the parity test, not this run):
```
2026-11-12T08:13:42Z GET /api/kit/reports -> 200 (0.003003s) {"workspaces":[{"folders":[],"name":"acme"},{"folders":[],"name":"personal"}]} [78 bytes]   (control: a served route)
2026-11-12T08:13:42Z GET /api/board -> 200 (0.023281s) {"gone":5,"projects":[]} [24 bytes]
2026-11-12T08:13:42Z GET /api/board?workspace=acme -> 200 (0.029676s) {"gone":2,"projects":[]} [24 bytes]
2026-11-12T08:13:42Z GET /api/board?workspace=nope -> 400 (0.003212s) {"error":"unknown workspace"} [29 bytes]
2026-11-12T08:13:42Z GET /api/memory-graph -> 200 (0.025658s) {"gone":0,"projects":[]} [24 bytes]
2026-11-12T08:13:42Z GET /api/memory-graph?project=nope -> 400 (0.024058s) {"error":"unknown project"} [27 bytes]
```
Guards and a live project (second run, 2026-11-12T08:13:32Z, one stored project path of the temp store copy pointed at a temp folder `$T/hostile` with `memory/MEMORY.md` and a FIFO as its `.git`): no token -> 403, bad `Host` -> 403, bad `Origin` -> 403, POST -> 405; `GET /api/board` -> 200 `{"gone":4,"projects":[{"abstained":0,"board":null,"last_pull":null,"project":"hostile","status":"no data","tasks":[]}]}`; `GET /api/memory-graph?project=$T/hostile` -> 200 in 0.045 s with the note node `hostile-note` and one dangling link `nope` (`"counts":{"dangling":1,"links":1,"notes":1}`). Before T043 every one of these routes answered 404 with an empty body.

**DELTA**: the dashboard can now read the kanban board of every project and the memory-note graph from the Rust server (the same answers as Python, read-only, with every file read bounded and non-following), where before both routes answered 404.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
