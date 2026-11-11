# TASK_REVIEW — T014: Usage, warnings and plan over the guarded HTTP server

> Sibling of `tasks/TASK_GUIDE_T014.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/tests/server.rs` (13 existing untouched + 13 new spawned-binary tests: usage goldens over HTTP, guard on the new routes, the three encoded `/api` forms, query decoding, 501, 503 and a folder that stays empty, HEAD/405, fixtures unchanged) and `rust/crates/app/src/api_tests.rs` (8 in-process tests with an injected clock: warnings and plan goldens, default and rich, `Query` parsing, error mapping, off-the-async-worker read) |
| Verification command run | ☑ pass | Supervisor ran `cargo test -p easy-personal-rs --test server`: 26 passed; the 8 `api` unit tests passed; the agent's whole-workspace run after its change was 157 passed, 0 failed, 4 ignored. clippy `-D warnings` and `cargo fmt --check` clean (agent). The Supervisor re-runs the whole suite once T015 (same crate) lands |
| Negative cases hold | ☑ pass | red run: 11 of 26 server tests failed with 404 while the routes were absent; the 13 old tests passed. Mutations (agent, restored byte for byte): first-value-wins and `+` kept as a plus fail 3 tests each; the filter check disabled fails its test; moving the store read onto the async worker fails `a_store_read_does_not_block_the_async_worker`; +1 `output_tokens` in a temp copy differs from the golden. `git status --short -- src web tests pyproject.toml uv.lock` empty (Supervisor re-checked) |
| verify | ☑ pass | Supervisor, real `serve` over a temp copy of `store-v11.sqlite` (`EASY_PERSONAL_HOME`): no token, wrong token, token in the query, bad Host, bad Origin -> 403 `{"error":"forbidden"}`; `/%61pi/usage/summary`, `//api/usage/summary`, `/api%2fusage%2fsummary` -> 404 with and without a token; `/api/usage/summary` -> 200 with the golden's numbers (cache_hit_ratio 0.7905); `by=bad` -> 400 with Python's text; `workspace=acme` -> 501; `since=` -> 400 `Invalid isoformat string: ''`; `/api/usage/sessions` -> 404; POST -> 405; the token appears in no body. Missing store: all four routes 503 with the `no store yet` text and the data folder stays empty — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `api.rs`, `api_tests.rs`, `server.rs` (router state and the merge inside the guard layer), `main.rs` (`mod api;`), `tests/server.rs`, app `Cargo.toml` dev-dependencies (`tower =0.5.3`, `http-body-util =0.1.5`, both already in the lock), `rust/README.md`; `mcp.rs`, `core/**`, `parity/**` untouched |
| Full smoke suite still green (no regression) | ☑ pass | the 13 existing server tests, 6 cli tests and every core suite pass; whole-suite re-run after T015 recorded in the board note |
| **Companion score (easy-verifier)** | ☑ N/A | not run: easy-verifier is not installed in this session |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no UI |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no UI |
| **UI: Responsiveness at target viewports** | ☑ N/A | no UI |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-10T11:23:00Z by the Supervisor, before the agent's first command:
```
$ EASY_PERSONAL_HOME=<temp copy of store-v11> ./target/debug/easy-personal-rs serve   # then GET with the printed token
GET /api/usage/summary -> 404
GET /api/warnings -> 404
GET /api/plan -> 404
GET /api/usage/breakdown?by=model -> 404
GET /api/version -> 200
```
(the four routes do not exist; only /api/version answers.)

**AFTER**: captured 2026-11-10 by the Supervisor (same setup as BEFORE):
```
GET /api/usage/summary -> 200   {"abstained":{...},"cache_creation_input_tokens":250,"cache_hit_ratio":0.7905,...}
GET /api/usage/breakdown?by=bad -> 400 {"error":"by must be one of day, model, project, tool"}
GET /api/warnings?workspace=acme -> 501 {"error":"the workspace and account filters are not available in this build yet"}
GET /api/usage/summary (no token) -> 403 {"error":"forbidden"}
```

**DELTA**: a client with the token can fetch usage summary, usage breakdown, warnings and the improvement plan from the Rust server, with the same JSON and status codes as the Python dashboard API for the cases the goldens cover.

**WITNESS**: Supervisor, 2026-11-10, ran the live probes and the server tests independently of the implementing agent.
