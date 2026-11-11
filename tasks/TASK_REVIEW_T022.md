# TASK_REVIEW — T022: Workspaces list and unassigned (core and routes)

> Sibling of `tasks/TASK_GUIDE_T022.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/workspaces_parity.rs` (9 tests: the four goldens counted from the manifests, 2 per oracle; three mutation tests; read-only check; error-text check; the Python edge-case store), `rust/crates/app/tests/workspaces.rs` (8 spawn tests: the four goldens over HTTP, guard, raw-path, methods, store states, query ignored) and 3 unit tests (`name_of` in `workspaces.rs`, 2 in `api_tests.rs`) |
| Verification command run | ☑ pass | Supervisor, own runs: whole `cargo test --workspace --no-fail-fast` = 266 passed, 0 failed, 8 ignored under TZ unset, `Pacific/Kiritimati` and `Pacific/Pago_Pago`; clippy `-D warnings` and `cargo fmt --check` clean; oracle hash `aa6fcaed…bff92` unchanged; Python tree empty |
| Negative cases hold | ☑ pass | agent's red runs: stubbed core (`Ok(Null)`) `3 passed, 6 failed` (`api__workspaces__all.json: differs got null`); routes absent: `1 passed, 7 failed` (404; `POST` check failed). Mutations on a temp copy of the rich store: delete the `/work/rich/kit/beta` repo row; change that repo_path to `.../betx`; change `personal`'s email: each fails the comparison naming `api__workspaces__all.json`; the unmutated store passes with 2 compared |
| verify | ☑ pass | Supervisor's own live probe, `serve` over a temp copy of the rich store: `/api/workspaces` 200 with the orphan and the invented workspaces; `/api/workspaces/unassigned` 200 `{"repos":[]}`; `?workspace=nope` ignored (as Python); `POST /api/workspaces` 405; no token 403; store bytes identical afterwards (`cmp`), stderr 0 bytes. The agent's Python edge-case check (gone worktree, owner by branch, scratch, unpriced model, tie in last use, NULL session id, shared repo, non-ASCII paths; Python's values embedded in the test) was not rerun — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `rust/crates/core/src/{workspaces,lib,scope,metrics}.rs` (the last two: `pub(crate)` only), `rust/crates/core/tests/workspaces_parity.rs`, `rust/crates/app/src/{api,api_tests}.rs`, `rust/crates/app/tests/workspaces.rs`, `rust/README.md`; `server.rs`, `main.rs`, `mcp.rs`, `tests/server.rs`, `rust/parity/**` untouched. code-review `reports/code-review_main_20261110T231200.html` (no P0/P1; 1 P2 stale README heading, 3 P3, 1 info) and security-review `reports/security-review_main_20261110T231300.html` (nothing at confidence 8+) |
| Full smoke suite still green (no regression) | ☑ pass | whole workspace 266 passed, 0 failed under three zones; no pre-existing test changed or pinned these paths as 404 |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier is not installed in this session |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no UI (the dashboard is not built or loaded by this task) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no UI |
| **UI: Responsiveness at target viewports** | ☑ N/A | no UI |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-10T14:45:31Z by the Supervisor, before the agents' first edits:
```
$ grep -c "workspaces" rust/crates/app/src/api.rs ; ls rust/crates/core/src
0
clock.rs lib.rs metrics.rs plan.rs prices.rs rules.rs scope.rs sessions.rs store.rs   (no workspaces.rs)
```

**AFTER**: captured 2026-11-10 by the Supervisor, live probe on a temp copy of the rich store:
```
GET /api/workspaces            -> 200 {"workspaces":[{"is_orphan":true,"name":"orphan",...},{"color":"#3366cc","description":"invented workspace one",...}]}
GET /api/workspaces/unassigned -> 200 {"repos":[]}
POST /api/workspaces           -> 405        no token -> 403 {"error":"forbidden"}
```

**DELTA**: the Rust server now serves the workspace list and the unassigned projects the dashboard needs for its workspace picker, with the same JSON as Python for four goldens; writes stay absent (405).

**WITNESS**: Supervisor, 2026-11-10, reran the whole workspace under three zones, the oracle hash and the Python-tree check, and probed the two routes live, independently of the implementing agent.
