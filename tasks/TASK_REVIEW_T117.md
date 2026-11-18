# TASK_REVIEW — T117: Live like Claude 3 — past sessions and own names

> Sibling of `tasks/TASK_GUIDE_T117.md`. Everything here is **filled by the reviewer at Stage
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
With `/verify (yours)`, your pass goes in the `verify` row below: Result `☑ pass`, Notes `/verify by the user, session <id>, commit <sha>`.

## Auto decisions

Mode: auto
1. Past sessions listed from the store's `events` metadata, no schema change — FACT: rust/crates/core/src/schema_v11.sql:79 "session_id                  TEXT,"
2. Own names in one owner-only file `session_names.json` — RULE: FR-119 pattern (owner-only file, temp + rename)
3. Past sessions with no `from`: default window of the last 30 days (ts index), stated in the answer; any explicit `from` lifts it — DEBATED: (c) 30-day default (Advocate 1: 30-day default, the only option within no-schema-change and fast at real scale; Advocate 2: 30-day default with a visible "Show older") ⚠ decided for you — please review
4. Kind filter `terminal|headless` only; the page marks app sessions from its live app-session list — FACT: reviewer, real store 2026-11-18: entrypoints are only `cli`, `claude-desktop`, `sdk-cli` and empty, so app and headless runs cannot be told apart in the store
Checks: tests `cargo test --workspace --no-fail-fast` — 1283 passed, 2 failed (`store_check` signal tests, load-related; `--test store_check` alone 12 passed); leak gate — 0 hits over 42 canaries, 1266 surfaces; reviews code-review — 0 P0/P1, 4 P2 + 3 P3 fixed (`afa11e1`, `2909860`); security-review — no finding ≥ 8; real store timing: default 30-day window ~80 ms (release), whole store 1.18 s
Memory: memory/decisions.md — "2026-11-18 — Live like Claude: transcript on demand, queue, Stop/Force stop, past sessions, one panel" (+ MEMORY.md index line), `5c38bf4` on main
Not done: owner smoke test (name a session and see it in Live); auto verify: runs next

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its proposal>; Advocate 2: <its proposal>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Memory: <each entry the memory pass wrote, as `file — entry` | nothing new>
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours), with [admin] only after the one retry also ended blocked>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/past_sessions.rs` (8 + 1 ignored timing: cursor paging with ties over 120 sessions, filters, bad values 400, 30-day default window and `window` field, EXPLAIN asserts `SEARCH events USING INDEX events_ts_message`, kind terminal/headless), `rust/crates/app/src/session_names_tests.rs` (13: guard/confirmed, 1..80 chars, null removes, 0600 temp+rename, 2,000 cap, unusable file never written over, bad entry shown but never written over, unique temp names under 8 threads, stamp cache), live_feed name test; `leak_rust.py` `session_name` canary |
| Verification command run | ☑ pass | at `2909860`: fmt + both clippy runs clean (agent); `cargo test --workspace --no-fail-fast` 1283 passed / 2 failed (`store_check` signal tests under load; alone `12 passed; 0 failed; 1 ignored`); `leak check: 0 hits over 42 canaries, 1266 surfaces, 2624311 bytes` |
| Negative cases hold | ☑ pass | bad `kind`/`from`/`to`/`before`/`project`/`branch`/account → 400 with fixed text; `kind=app` → 400; no token / wrong Host / foreign Origin → 403; missing/false `confirmed`, unknown field → 400; a PUT over a corrupt, 0666 or bad-entry file → 500 `UNUSABLE` with the file bytes unchanged |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | code-review and security-review on `main...T117` (sessions.rs, session_names.rs, live_feed.rs, api.rs, tests, leak gate) + callers; re-check of `afa11e1`; real store read-only for timing |
| Full smoke suite still green (no regression) | ☑ pass | Rust 1283 + `store_check` green alone; leak 0 hits
| **Companion score (easy-verifier)** | ☑ N/A | not run: independent code and security reviews used instead | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | server routes only; the page is T119 |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | server routes only; the page is T119 |
| **UI: Responsiveness at target viewports** | ☑ N/A | server routes only; the page is T119 |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (Supervisor, before any T117 implementation commit; `main` at `ddb947f`, the branch point):
```
$ date -u; git log -1 --oneline; grep -rn 'sessions/past\|session_names' rust/crates/app/src/ | wc -l; ls rust/crates/app/src/session_names.rs
2026-11-18T10:15:50Z
ddb947f Plan: Live like Claude (question 45, FR-123..FR-128, DDR-0007, T115-T119)
0
ls: cannot access 'rust/crates/app/src/session_names.rs': No such file or directory
```
(No past-sessions route and no names: sessions older than the live hour cannot be opened from the app.)
first implementation commit] OR [verbatim excerpt of the prior content, for non-executable changes]

**AFTER** (`2909860`, 2026-11-18T11:53:55Z):
```
$ grep -rn 'sessions/past\|session_names' rust/crates/app/src/api.rs rust/crates/app/src/main.rs | head -3; ls rust/crates/app/src/session_names.rs
rust/crates/app/src/main.rs:30:mod session_names;
rust/crates/app/src/api.rs:37:use crate::{connector_writes, hooks_cmd, legacy, session_names, sync_apply, writes};
rust/crates/app/src/api.rs:222:        .merge(session_names::routes()) // T117: PUT /api/sessions/{id}/name
rust/crates/app/src/session_names.rs
```

**DELTA**: the page can now list sessions older than the live hour (last 30 days by default, any range on request) and give any session the owner's own name.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
