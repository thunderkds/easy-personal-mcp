# TASK_REVIEW — T030: Leak suite over the Rust ingest — planted secrets never reach the store, the surfaces or the logs

> Sibling of `tasks/TASK_GUIDE_T030.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/parity/leak_rust.py` (694 lines: driver, scanner, `--self-test`, coverage and anti-vacuity checks); the self-test proves each of seven encodings (raw, json, unicode-escape, url, hex, base64 and base64url at three alignments), a SQLite row and masking |
| Verification command run | ☑ pass | Supervisor 2026-11-11T07:57:42Z: `leak_rust.py --self-test` = `self-test: ok`; two full runs, each exit 0 with different canary sets (fingerprints `84e3a62b74bb`, `422de1302c05`): `leak check: 0 hits over 42 canaries, 116 surfaces, 786719 bytes`, all 12 `check` lines ok; no canary text in the saved reports; `rust/parity/.venv/bin/python -m pytest tests/leak -q` = `46 passed, 1 warning in 71.71s`; `git status` for `tests src web` = 0 entries |
| Negative cases hold | ☑ pass | agent-reported and log re-read by the Supervisor: in a patched COPY of the workspace (scratchpad, never the repo) that stores the raw `gitBranch` text, the gate exits 1 with `leak check: 10 hits over 42 canaries, 116 surfaces` naming `branch_text` on the store WAL, the store file and both dumps; real-home guard refuses (`EASY_PERSONAL_AGENT_ROOT` = the home folder → `refused …`, exit 1). The Supervisor re-read the red log (10 `HIT` lines) but did not rebuild the patched copy |
| verify | ☑ pass | the gate is itself a real run of the built binary: ingest, `store-info`, `serve` over real HTTP with the token, `mcp` over stdio (27 tool calls), and the store bytes were scanned; serve and mcp stopped and checked gone by PID. The `/verify` recipe itself was not edited; the line to add is `cd rust && cargo build && ./parity/.venv/bin/python parity/leak_rust.py` |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | read by the Supervisor: the risky-call grep of `leak_rust.py` (no shell, no network; two `rmtree` of its own mkdtemp roots) and its output; the 694-line script was not read line by line (proven by the self-test, the run, and the negative proof). Scope: `rust/parity/leak_rust.py` (new), `rust/parity/README.md` (+48 lines), this review file, and the stale wording in `BRAINSTORMING_LOG_RUSTP2.md`; no change in `tests/`, `src/`, `rust/crates/`. code-review `reports/code-review_main_20261111T145943.html`: 0 P0, 0 P1, 0 P2, 3 P3. security-review `reports/security-review_main_20261111T145944.html`: nothing at confidence 8 or above |
| Full smoke suite still green (no regression) | ☑ pass | Python leak suite 46 passed (Supervisor re-run); Rust code untouched by this task |
| **Companion score (easy-verifier)** | ☑ N/A | `score` not run in this session |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no UI |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no UI |
| **UI: Responsiveness at target viewports** | ☑ N/A | no UI |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured by qa-expert before any file was created):

```
$ date -u +%Y-%m-%dT%H:%M:%SZ; ls rust/parity/leak_rust.py; rust/parity/.venv/bin/python -m pytest tests/leak -q | tail -5; date -u ...
2026-11-11T07:42:12Z
ls: cannot access 'rust/parity/leak_rust.py': No such file or directory
  /home/thunderkds/workspace/pets/easy-personal-mcp/tests/leak/test_kit_report_files_leak.py:10: StarletteDeprecationWarning: Using `httpx` with `starlette.testclient` is deprecated; install `httpx2` instead.
    from starlette.testclient import TestClient

-- Docs: https://docs.pytest.org/en/stable/how-to/capture-warnings.html
46 passed, 1 warning in 64.82s (0:01:04)
2026-11-11T07:43:17Z
```

No Rust leak check exists; the Python leak suite (46 tests) is green.

**AFTER**: Supervisor 2026-11-11T07:57:42Z: `leak_rust.py --self-test` ok; two runs `leak check: 0 hits over 42 canaries, 116 surfaces, 786719 bytes` (exit 0, different canary sets); `pytest tests/leak -q` = `46 passed`; the agent's patched-copy run exited 1 with `10 hits` naming `branch_text`.

**DELTA**: one command now proves, for the Rust binary, that 42 planted secrets in every encoding never reach the store, its WAL, the API, the MCP tools, the logs or any output, and the check is shown to fail when a leak is introduced.

**WITNESS**: Supervisor re-ran the self-test, two full runs and the Python leak suite after the implementing agent; tool calls recorded in `memory/event-trace/T030.jsonl`.

## Notes for the owner

1. Leaks found in the Rust product: **none**.
2. Not planted/read yet because Rust has no such surface: the in-memory keyring and the `account confirm` flow (the `.claude-work` account); their canaries are still scanned everywhere. Revisit at P3.
3. The line to add to the `/verify` recipe (a later task): `cd rust && cargo build && ./parity/.venv/bin/python parity/leak_rust.py`.
