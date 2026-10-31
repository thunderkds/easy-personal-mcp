# TASK_REVIEW — HUN-168: Live trace E3: Claude Code OpenTelemetry to a local OTLP receiver

> Sibling of `tasks/TASK_GUIDE_HUN-168.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `experiments/live-trace/tests/test_e3.py` (15 tests: logs/metrics/traces parsing, allow-list, planted-secret, bad requests, loopback bind, latency, token/cost vs transcript, attribute inventory, runner env, fake-claude end-to-end under flock, grep over `e3/`). Output: `15 passed, 12 deselected in 5.15s` (pass) |
| Verification command run | ☑ pass | `uv run pytest experiments/live-trace/tests -q -k e3` at 2026-10-31T16:08:36Z -> `15 passed, 12 deselected in 5.20s` (pass) |
| Negative cases hold | ☑ pass | invalid JSON -> 400, unknown path -> 404, nothing stored; non-loopback host -> ValueError; sentence in an allow-listed key dropped; prompt-text body gives event_name None; planted secret/sentence/email absent from `e3/` files (pass) |
| verify | ☑ pass | Supervisor 2026-10-31: verify recipe from the HUN-168 worktree — `easy-personal --version` → `0.2.0`, `ping` → `easy-personal core ok`, MCP stdio `tools/call ping` → `easy-personal core ok`; diff touches only `experiments/live-trace/` + review files (no `src/`), so product behaviour is unchanged; spike suite `27 passed in 5.12s` — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor code-review 2026-10-31 over the branch diff vs HUN-165 (only experiments/live-trace/ + this review file). Content scan of stored runs for emails, scratch-file text, planted key: 0 hits. Inline security pass (Medium risk): subprocess list args, no shell=True; no credential reads; no ~/.claude* writes; receiver refuses any host but 127.0.0.1; value allow-list (STR_KEYS/NUM_KEYS) + SAFE_STR pattern, so user.email/account ids are never stored (0 email hits). Findings: P0 0, P1 0; P3: RUF100 unused noqa; 1.4 MB of run JSON committed — acceptable for spike evidence — pass |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest experiments/live-trace/tests -q` -> `27 passed in 5.20s` (12 harness + 15 e3) (pass) |
| **Companion score (easy-verifier)** | ☑ N/A | spike code under `experiments/`, not product code; companion not run |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no UI: headless spike scripts |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no UI: headless spike scripts |
| **UI: Responsiveness at target viewports** | ☑ N/A | no UI: headless spike scripts |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-31T15:48:51Z by qa-expert, before any implementation commit:

```
$ ls experiments/live-trace/e3 2>&1
ls: cannot access 'experiments/live-trace/e3': No such file or directory
$ uv run pytest experiments/live-trace/tests -q -k e3 2>&1 | tail -3
Installed 43 packages in 20ms

12 deselected in 0.04s
```

**AFTER**: captured 2026-10-31T16:08:36Z by qa-expert:

```
$ ls experiments/live-trace/e3
RESULT.md  aggregate.py  analyze.py  receiver.py  run_e3.py  runs
$ uv run pytest experiments/live-trace/tests -q -k e3 2>&1 | tail -3
...............                                                          [100%]
15 passed, 12 deselected in 5.20s
```

**DELTA**: The spike can now receive Claude Code OpenTelemetry on 127.0.0.1:4318 and measure its freshness (median 1007 ms at the lowered interval), token and cost accuracy (0.000% diff) and attributes.

**WITNESS**: qa-expert agent, 2026-10-31 15:48-16:08 UTC; 9 headless Haiku sessions recorded in `experiments/live-trace/e3/runs/*.json` (Supervisor to confirm from `memory/event-trace/HUN-168.jsonl`).
