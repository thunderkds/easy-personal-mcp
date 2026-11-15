# TASK_REVIEW — T[NNN]: [Short Title]

> Sibling of `tasks/TASK_GUIDE_T[NNN].md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
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

**BEFORE** (2026-11-15T14:57:25Z, branch `T088` at the guide commit, no implementation commit; a throwaway test printing `tray::view` rows for one waiting session, since `view` has no alert input; test file restored after the run). Command: `cd rust && cargo test -p easy-personal-rs --bin easy-personal-rs zz_before -- --nocapture`

```
ROW Info("Easy Personal · attention")
ROW Separator
ROW Info("1 session active")
ROW Action("waiting", "1 waiting for you · easy-memory")
ROW Info("5h limit · no data · 7d · no data")
ROW Info("Machine · no data")
ROW Separator
ROW Action("open", "Open dashboard")
ROW Check("pause", "Pause alerts", false)
ROW Separator
ROW Info("127.0.0.1 · local only")
ROW Action("quit", "Quit")
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 178 filtered out
```
No alert rows exist: the menu cannot show the alert list (the engine raises "Waiting for you, ..." for this session, but only a popup and `GET /api/alerts` carry it).

**AFTER** (2026-11-15T15:19:20Z, same kind of throwaway print test, now with 6 alerts, local zone +07:00; test file restored after the run):

```
ROW Info("Easy Personal · attention")
ROW Separator
ROW Info("1 session active")
ROW Action("waiting", "1 waiting for you · easy-memory")
ROW Info("5h limit · no data · 7d · no data")
ROW Info("Machine · no data")
ROW Separator
ROW Info("Recent alerts")
ROW Info("21:02 · a0")
ROW Info("21:01 · a1")
ROW Info("21:00 · a2")
ROW Info("20:59 · a3")
ROW Info("20:58 · a4")
ROW Action("alerts", "All alerts in dashboard…")
ROW Separator
ROW Action("open", "Open dashboard")
ROW Check("pause", "Pause alerts", false)
ROW Separator
ROW Info("127.0.0.1 · local only")
ROW Action("quit", "Quit")
test result: ok. 1 passed; 0 failed
```

**DELTA**: A user can open the tray menu and read the newest 5 alerts (time and text), even after the popups faded or while "Pause alerts" is on, and reach the rest in the dashboard.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T088.jsonl`, never the
implementing agent alone]

---

## Stage 4 — code-review (Supervisor inline, 2026-11-15)

Scope: `rust/crates/app/src/{alerts.rs,tray.rs,window.rs}`, `rust/crates/app/Cargo.toml`; caller of `tray::view` = `Inputs::view` only (grep).
Lane check: `lane: C1, 6 files, 575 lines, EXCEEDS (lines 575 > 250)` — ~190 lines are tests and ~250 are guide/log/review docs; code is ~110 lines. C1 kept.
Reachability: entry `tray.rs::view` present (line 276). Conditional reviewers: security skipped (no new input; alert text already `clean`ed by the engine, NFR-002); over-engineering run (new fns + libc edge).

P0: None. P1 (code): None. P2: None.
P1 (process): startup reads — `PROJECT_SPEC.md` read head only; `rust/PROJECT_SPEC.md` not read; `agents/backend.md` reported as loaded system prompt (true for this agent type). No code impact found.
P3:
| File | Line | Finding | Confidence | Action |
|---|---|---|---|---|
| rust/crates/app/src/tray.rs | ~242 | Hand-rolled civil-date maths duplicates what `localtime_r` already returns (`tm_mon`, `tm_mday`, `tm_hour`, `tm_min`); kept because it makes the day logic testable with a fixed offset | 75 | optional: keep |
| rust/crates/app/src/tray.rs | ~228 | `libc::localtime_r` / `tm_gmtoff` is Unix-only; fine while the tray is Linux-only, needs a cfg when macOS/Windows come | 75 | note for the macOS/Windows port |
| rust/crates/app/src/tray.rs | test `five_rows_at_most…` | the n=50 case passes `recent(n.min(6))`, so it tests 6, not 50 (the 50-slice case is covered just below) | 75 | optional |

Safe fix applied: guide verification command named a non-existent package `easy-personal-app` → `easy-personal-rs`.
Supervisor rerun: `cargo test -p easy-personal-rs --features window --bin easy-personal-rs -- tray:: window:: alerts::` → `test result: ok. 50 passed; 0 failed; 0 ignored; 0 measured; 137 filtered out; finished in 0.68s`; `cargo clippy -p easy-personal-rs --features window --all-targets` clean; `cargo fmt --check` ok.

## AC 7 — owner real run (2026-11-15)

Owner ran `cargo run --features window --bin easy-personal-rs -- tray --static-dir /usr/share/easy-personal-rs/static` from this worktree (installed tray quit first). First try showed no block: correct, no alert had been raised (a permission prompt is not "waiting"; only an open `AskUserQuestion` / `ExitPlanMode` call is, `core/src/live.rs` `WAITING_TOOLS`). The Supervisor then held an `AskUserQuestion` open; owner opened the tray menu and answered "Block is shown".

## Stage 5 — /verify (Supervisor, 2026-11-15, at c0880db)

Rust recipe in this worktree, temp `EASY_PERSONAL_HOME`:
- `easy-personal-rs 0.3.0`; `store-info` user_version 11
- `serve`: `/api/version` with token `{"version":"0.3.0"}`, without token `403`; stopped by PID
- `mcp`: initialize + `usage_summary` answered
- `ingest` s3 twice: run 1 new_events 3, run 2 new_events 0, both exit 0
- leak gate (`env -u CLAUDE_CONFIG_DIR`, the session sets it): `leak check: 0 hits over 42 canaries, 828 surfaces, 1808320 bytes`, exit 0
- window step: `cargo build --features window`; `app --self-test` → `self-test: ok`, exit 0; plain `cargo build` last
