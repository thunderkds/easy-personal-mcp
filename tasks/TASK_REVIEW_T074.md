# TASK_REVIEW — T074: Watch 3: alerts with hysteresis + desktop notifications over D-Bus, `GET /api/alerts`, pause

> Sibling of `tasks/TASK_GUIDE_T074.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | 21 new tests, all pass: `rust/crates/app/src/alerts.rs` (16: AC1 hysteresis disk/ram/limit/battery + per-account, AC2 session once/leave/return, AC3 leak, AC4 engine pause, AC5 no D-Bus warns once, popup gap + summary, clock jump, live-tail path), `rust/crates/app/src/api_tests.rs` (3: routes, guard, 415/400, pause through the route keeps the list), `rust/crates/app/src/server.rs` (2: engine wired + stop joins, no engine without a notifier) |
| Verification command run | ☑ pass | `cd rust && cargo test --workspace`: 91 test binaries, passed 971, failed 0, ignored 28 (ignored = existing). `cargo clippy --workspace --all-targets`: no warning. `cargo fmt --check`: clean. `cargo check --features window`: finished. |
| Negative cases hold | ☑ pass | secret-looking tool / project / session id and planted message text never reach popup or list; pause with bad body 400, wrong type 415, no token 403, store never opened by the routes (counter 0) |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | same `cargo test --workspace` run, 0 failed; leak gate: `env -u CLAUDE_CONFIG_DIR <main>/rust/parity/.venv/bin/python parity/leak_rust.py` -> `leak check: 0 hits over 42 canaries, 818 surfaces, 1793501 bytes`, exit 0, no failed check line (`/api/alerts`, `/api/alerts/pause` and 4 pause write calls registered) |
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

**BEFORE** (main 5b9adb4 + branch T074, no implementation yet; HOME is an empty fixture folder):

```
2026-11-15T05:24:47Z
$ easy-personal-rs serve --alerts
error: unexpected argument '--alerts' found

Usage: easy-personal-rs serve [OPTIONS]

For more information, try '--help'.
exit=2
$ easy-personal-rs serve --no-ingest ; GET /api/alerts and POST /api/alerts/pause with the right token
GET /api/alerts -> 404
POST /api/alerts/pause -> 404
```

**AFTER** (2026-11-15, fixture store and transcript in a scratch folder, no real home; `DISPLAY=:0`, session D-Bus; `serve --no-ingest --alerts`; a transcript line with an open `AskUserQuestion` call was written after start):

```
GET /api/alerts -> {"alerts":[{"account":null,"kind":"session_waiting","session":"0123abcd","t":1794723133,"text":"Waiting for you, AskUserQuestion, project shop-front, session 0123abcd"}],"paused":false}
serve stderr: (empty)
dbus-monitor --session "interface='org.freedesktop.Notifications'" (the Notify call from our process, sender :1.524):
method call time=1794723133.524542 sender=:1.524 -> destination=org.freedesktop.Notifications serial=2 path=/org/freedesktop/Notifications; interface=org.freedesktop.Notifications; member=Notify
   string "easy-personal"
   uint32 0
   string ""
   string "Easy Personal"
   string "Waiting for you, AskUserQuestion, project shop-front, session 0123abcd"
   array [
   ]
   array [
   ]
   int32 10000
```

(The shell then forwarded the same Notify to the desktop's notification server, so the popup was shown. The planted text "hunter2-planted" and "planted question text" in the transcript appear in neither the popup nor the list.) Leak gate and tests: see Evidence.

**DELTA**: With `serve --alerts` (or `app --alerts`) the owner gets one desktop popup when a session waits for them or fails, a plan limit, disk, RAM or battery crosses its line, and can read the last 50 alerts at `GET /api/alerts` and pause the popups with `POST /api/alerts/pause`.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T074.jsonl`, never the
implementing agent alone]
