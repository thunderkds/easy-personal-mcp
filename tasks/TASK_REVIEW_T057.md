# TASK_REVIEW — T057: Rust live feed part 2 (kit-trace feed, app wiring)

> Sibling of `tasks/TASK_GUIDE_T057.md`. Everything here is **filled by the reviewer at Stage
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

**BEFORE** (Step 0, implementing agent, branch `T057` at `c245d89`, no implementation commit):

**Step 0 finding: Python has no live kit-trace feed. The stop rule applies; no code was written.**

- Which file produces kit-trace data in Python: only `src/easy_personal/sources/kit_trace.py`, a
  scheduled-ingest source (`ingest(conn)`, called from `surfaces/cli.py:168` in `easy-personal ingest`).
  It reads `<project>/memory/event-trace/*.jsonl` and **stores** rows in `kit_trace_events`
  (`kit_task_id` from the file name, `ts_utc`, `tool_name`, `is_error`, `abstained`; never `summary`).
- Which route or page shows it: the stored rows feed `core/kit.py` (`kit_summary`, `GET /api/kit`,
  the Kit page). That is history from the store, not a live feed.
- The live tail (`core/live.py`), the stream (`surfaces/api.py` `live_stream`) and the Live tab
  (`web/src/LiveView.jsx`) never read a trace file: 0 matches for `kit_trace` / `event-trace` below.
  The only kit item in the live stream is the **kit stage** (FR-083: `stage`, `stage_skill` from
  `STAGE_MAP`, worked out from `Skill` calls in transcripts). The Rust T049 engine already has it
  (`rust/crates/core/src/live.rs:72` `stage_of`, `:821` snapshot keys) and T056 streams it.
- App wiring is already in code since T056: `window.rs:136` starts `LiveFeed::system()`, `:140`
  passes it to `server::start`, `running.stop()` joins it; the page opens
  `/api/live?token=...` (`LiveView.jsx:302`), which is T056's query-token carve-out.

```
2026-11-13T08:19:29Z
c245d89 T056 merged: board row, guides T056/T057, memory decision
$ grep -rln kit_trace src web/src
src/easy_personal/core/kit.py
src/easy_personal/core/migrations/m0001_baseline.py
src/easy_personal/core/store.py
src/easy_personal/sources/kit_trace.py
src/easy_personal/sources/verifier.py
src/easy_personal/surfaces/cli.py
$ grep -n -i "kit_trace\|event-trace" src/easy_personal/core/live.py src/easy_personal/surfaces/api.py web/src/LiveView.jsx
(no match)
$ grep -n "STAGE_MAP = \|\"stage\": s.stage" src/easy_personal/core/live.py
66:STAGE_MAP = {
387:                        "stage": s.stage,
$ grep -n "fn stage_of\|\"stage\": s.stage" rust/crates/core/src/live.rs
72:fn stage_of(skill: &str) -> Option<&'static str> {
821:                    "attention": attention, "stage": s.stage, "stage_skill": s.stage_skill,
$ grep -n "LiveFeed::system\|server::start" rust/crates/app/src/window.rs
136:        let feed = match LiveFeed::system() {
140:        let running = match server::start(Some(&root), Some(feed)) {
$ grep -n "new EventSource" web/src/LiveView.jsx
302:    const source = new EventSource(`/api/live?token=${encodeURIComponent(readToken())}`)
```

Supervisor narrowed T057 after Step 0 to AC 3 (self-test), AC 6 (regression) and AC 8 (docs). BEFORE of the self-test
(window build under WSLg, temp `HOME`, before any code change): it passes on one `/api/version` call and never opens `/api/live`.

```
2026-11-13T08:31:39Z
5493d24 T057: Step 0 recorded (Python has no live kit-trace feed; stop rule applies)
$ HOME=<temp> EASY_PERSONAL_HOME=<temp> target/debug/easy-personal-rs app --self-test --static-dir crates/app/tests/fixtures/self-test
easy-personal-rs: window open on http://127.0.0.1:34091/
self-test: ok
easy-personal-rs: window closed
exit=0
$ grep -n "api/live\|snapshot" crates/app/src/window.rs
(no match: the self-test never opens /api/live)
```

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T057.jsonl`, never the
implementing agent alone]
