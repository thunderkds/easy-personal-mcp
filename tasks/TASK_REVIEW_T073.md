# TASK_REVIEW — T073: Watch 2: plan-limit watch per account (5 h / 7 d %) in the sampler, `GET /api/limits/history`

> Sibling of `tasks/TASK_GUIDE_T073.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/src/limits_watch.rs` (5 tests: AC1 fresh/stale/missing/broken -> values/null/null, over-100 kept, AC2 1,440 bound + oldest dropped, add/remove/ignore/codex, no write to the account folder); `rust/crates/app/src/api_tests.rs::limits_history_is_guarded_read_only_and_has_the_shape` (AC3: 403 without token, shape, 405 on POST/PUT/PATCH/DELETE, store file and account folder unchanged). All 6 `ok` in `cargo test --workspace` |
| Verification command run | ☑ pass | Fix round 3, on `841217c` (after `git merge main`, which brought T079). The plan-limit sampler now runs on `sampler::Sampler<Round>` (thread `easy-personal-limits`, kept in `Running`, joined by `stop()`, `machine::INTERVAL_S`/`CAPACITY`; the duplicate constants and the own loop are gone). `cargo test --workspace --no-fail-fast`: `passed 950 failed 0 ignored 28`, `exit=0`. New/changed tests: `limits_watch` 6 (incl. a gap round), `api_tests::limits_history_is_guarded_read_only_and_has_the_shape`, `api_tests::the_limits_sampler_ticks_on_its_own_thread_and_stop_joins_it_quickly` (10 ms interval, 3 rounds, `stop` < 1 s). `cargo clippy --workspace --all-targets`: clean; `cargo check --features window`: clean. Leak gate: `surface api GET /api/limits/history: 249 bytes`, `leak check: 0 hits over 42 canaries, 812 surfaces, 1791460 bytes`, `exit=0`, no `failed checks`. First sample at once was tried and fails: `store_health_route::the_route_writes_nothing_and_names_no_folder` (store `wal_no_shm`) sees the `-shm` mtime change (`before` vs `after` differ by 3 ms) even with T079; so the first round of `sampler::limits()` is a gap and the first real round comes after one interval (comment states the true reason). With the gap: `store_health_route` 10 passed |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☑ pass | Supervisor `/verify` 2026-11-15 on `0dd259e`, store copy of `rust/parity/store-v11.sqlite`: default build, `easy-personal-rs 0.2.0`; `serve`: `GET /api/limits/history` with token → `{"accounts":[],"interval_s":60}` (no accounts in the fixture store), no token 403, POST 405, `/api/machine` 1 sample beside it; stopped by PID; window step: `app --self-test` → `self-test: live snapshot ok`, `self-test: ok`, exit 0; agent: 950 passed 0 failed, leak 0 hits over 812 surfaces — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | independent code-review 2026-11-15 over `git diff main...T073` + callers of `accounts::limit_percents`, `server::start`, `Api` constructors: P0 0, P1 0, P2 1 (own sampler loop beside T072's `Sampler<T>`, not joined) → fixed `841217c`; P3 3: wrong reason in the first-sample comment → comment now gives the true reason (store_health `-shm` mtime), thread spawned before fallible steps → fixed by `Sampler`, no thread test → added. Verdict merge. |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☑ N/A | not run: low signal on this repo (memory 2026-11-14) |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (real `serve --no-ingest` of the unchanged `T073` branch at `244e983`, temp HOME, guarded request with the token):
```
2026-11-15T02:48:02Z
GET /api/limits/history:
HTTP/1.1 404 Not Found
content-security-policy: default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'
content-length: 0
date: Sun, 15 Nov 2026 02:48:02 GMT
```

**AFTER** (real `serve --no-ingest`, temp HOME, store copy with one Claude account "Demo" whose folder holds a fresh `easy-personal-limits.json` with 37.5 / 12.0):
```
2026-11-15T03:50:47Z
GET /api/limits/history (no token):
403
GET /api/limits/history (token), at start:
{"accounts":[],"interval_s":60}
2026-11-15T03:51:52Z
after 65 s:
HTTP/1.1 200 OK
content-type: application/json
{"accounts":[{"id":1,"label":"Demo","samples":[{"five_h_pct":37.5,"seven_d_pct":12.0,"t":1794714707}]}],"interval_s":60}
```

**DELTA**: the dashboard server now answers `GET /api/limits/history` with a per-account 24 h in-memory series of 5 h / 7 d plan-limit used % (null where the snapshot is missing, broken or stale), one sample per minute.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T073.jsonl`, never the
implementing agent alone]
