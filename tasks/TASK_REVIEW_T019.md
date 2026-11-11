# TASK_REVIEW — T019: Wire workspace and account into HTTP and MCP

> Sibling of `tasks/TASK_GUIDE_T019.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/src/api_tests.rs` (35 unit tests with `mcp.rs` unit tests), `rust/crates/app/tests/server.rs` (30), `rust/crates/app/tests/mcp.rs` (20): the 31 HTTP and 10 MCP scoped goldens counted from the rich manifest; unknown scope 400 on four routes; `by` before scope; empty = no filter; stripped and case-insensitive account; last value wins; no caller text in any body or log; two mutation tests; `TZ=UTC` pinned in `Mcp::start` |
| Verification command run | ☑ pass | Supervisor, own runs: `cargo test -p easy-personal-rs --no-fail-fast` under TZ unset and `Pacific/Kiritimati`: 35 + 6 + 20 + 30 passed, 0 failed (both); whole workspace 213 passed, 0 failed, 8 ignored under all three zones; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --check` clean; Python tree empty; oracle hash `aa6fcaed…bff92` unchanged; sweep for `501` / `not available in this build` / `FILTER_NOT_BUILT` finds nothing in `crates/app`, README or the core tests |
| Negative cases hold | ☑ pass | agent's red run (routes still 501, tools still erroring): 6 app unit, 8 mcp unit, 4 server and 2 mcp spawn tests failed with e.g. `/api/warnings?workspace=nope: 501`. TZ debt: with the pin removed from `Mcp::start` under `TZ=Pacific/Kiritimati`, 3 mcp tests fail (17 passed, 3 failed); with it restored 20 pass. Mutation tests: all account-2 events moved to account 1 breaks the scoped warnings and plan comparison; one priced event moved over the binary breaks `account=Claude%20work` and `account=1` |
| verify | ☑ pass | Supervisor's own live probe, `serve` over a temp copy of the rich store (loopback, `TZ=UTC`, scratch HOME): `usage/summary?workspace=acme` 200; `?account=Claude+work` 200; `?workspace=nope` 400 `{"error":"unknown workspace"}`; `warnings?account=nope` 400 with the labels; `breakdown?by=bad&workspace=nope` 400 (`by` first); `workspace=&account=` 200 unscoped; `plan?workspace=personal` 200 `{"groups":[]}`; no token 403; the process was killed, none left, stdout/stderr held no `acme` or `nope`. The agent's MCP transcript and Python cross-check of the tool-error wording (byte for byte for warnings and improvement_plan) were not rerun — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `rust/crates/app/src/{api,api_tests,mcp}.rs`, `rust/crates/app/tests/{server,mcp}.rs`, comment text in `rust/crates/core/tests/{rich_parity,plan_parity}.rs`, `rust/README.md`; `rust/crates/core/src/**` untouched. code-review `reports/code-review_main_20261110T211200.html` (no P0/P1; 1 P2 decision, 2 P3, 1 info) and security-review `reports/security-review_main_20261110T211300.html` (nothing at confidence 8+) |
| Full smoke suite still green (no regression) | ☑ pass | whole workspace 213 passed, 0 failed under TZ unset, `Pacific/Kiritimati` and `Pacific/Pago_Pago` (the two T015 tests that failed under the other zones now pass). Seven pin tests of the 501 / tool error were replaced or deleted, each listed by the agent with a replacement. Open for the owner: Python's MCP `usage_summary` / `usage_breakdown` ignore `workspace` / `account`; ours honour them (the guide asked for it) |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier is not installed in this session |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no UI |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no UI |
| **UI: Responsiveness at target viewports** | ☑ N/A | no UI |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-10T13:29:12Z by the Supervisor, after the agents were spawned and before the T019 agent's first edit:
```
$ grep -n "FILTER_NOT_BUILT" rust/crates/app/src/api.rs | head -3
30:const FILTER_NOT_BUILT: &str =
109:        return respond(StatusCode::NOT_IMPLEMENTED, &error_body(FILTER_NOT_BUILT));
$ grep -n "not available in this build" rust/crates/app/src/mcp.rs | head -3
31: / 538: / 571: ... "the workspace and account filters are not available in this build yet"
```
(a non-empty workspace or account answers 501 over HTTP and a tool error over MCP.)

**AFTER**: captured 2026-11-10 by the Supervisor, live probe over loopback on a temp copy of the rich store:
```
GET /api/usage/summary?workspace=acme            -> 200 {"abstained":{...},"cache_creation_input_tokens":2953280,...}
GET /api/usage/summary?workspace=nope            -> 400 {"error":"unknown workspace"}
GET /api/warnings?account=nope                   -> 400 {"error":"unknown account; valid labels: Claude default, Claude work"}
GET /api/usage/breakdown?by=bad&workspace=nope   -> 400 {"error":"by must be one of day, model, project, tool"}
GET /api/usage/summary?workspace=acme  (no token) -> 403 {"error":"forbidden"}
```

**DELTA**: a workspace or an account in a request or tool call now narrows usage, warnings and the plan like Python, with Python's 400 text for an unknown one, instead of a 501 or a tool error; the whole workspace suite is green in any time zone.

**WITNESS**: Supervisor, 2026-11-10, reran the app suite, the workspace under three zones, the oracle hash and the Python-tree check independently of the implementing agent. Also ran a live probe.
