# TASK_REVIEW — T011: Generator hardening

> Sibling of `tasks/TASK_GUIDE_T011.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | the generator checks are the tests: `rust/parity/gen_golden.py` fails loudly on a missed clock patch (names the module), a new MCP prompt or resource (names it), an existing `/work/demo-app` (default and `--tz`), a foreign `golden*/` or store in `--out`, and a home-folder or not-yet-existing `.claude*` / `.easy-*` path. Agent mutations on a scratch copy of the repo: `kitperf` datetime subclass, a new prompt, a new resource — each exit 1 with the module or name; the unmutated copy exit 0 |
| Verification command run | ☑ pass | Supervisor: fresh runs of the default, `--tz` and `--rich` modes (under `PYTHONHASHSEED=3 LC_ALL=C TZ=America/New_York`) into a scratch `--out`: `diff -r` empty for `golden`, `golden-tz`, `golden-rich`; `cmp` identical for `store-v11.sqlite` and `store-rich/store-v11.sqlite`. Combined oracle hash (310 files) `aa6fcaed…bff92` equals the value recorded before the task. `ruff check rust/parity` passes; `ruff format --check gen_golden.py` clean. Agent also ran the whole `cargo test` (all ok) |
| Negative cases hold | ☑ pass | Supervisor, fake `HOME` in the scratchpad: a foreign `golden/` (with `keep.txt`) in `--out` -> `refused: golden/ in --out was not made by this generator … nothing was deleted`, file still there; `--out` = the fake home, `EASY_PERSONAL_HOME` inside the fake `.easy-personal`, `--out` under a not-yet-existing `.claude-new` -> each `refused: … (NFR-003); nothing was opened`; the fake home still holds only `.claude` and `.easy-personal` (empty) afterwards. Agent: 15 more refusals (all home variables, symlinks, `--fixtures` above the home) with a positive control. `git status --short -- src web tests pyproject.toml uv.lock` empty |
| verify | ☑ pass | the generator runs in all three modes and produces the committed output byte for byte; a second run into the same `--out` replaces only its own output (agent: exit 0, stale files removed); the `uv.lock` drift warning printed on a real run (starlette 1.10.0 vs 1.9.0, anyio 4.16.0 vs 4.15.1, sse-starlette 3.7.0 vs 3.5.0). Not tested: the two `uv` install commands in the README (`uv` is not installed here) |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `rust/parity/gen_golden.py` and `rust/parity/README.md` only (plus `.gitignore` by the Supervisor); the oracle folders, `rust/crates/**` and the Python tree untouched |
| Full smoke suite still green (no regression) | ☑ pass | agent's whole `cargo test` green; the Rust suites read the committed goldens, whose hash is unchanged |
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

**BEFORE**: captured 2026-11-10T10:57:57Z by the Supervisor, before the agent's first command:
```
$ cd rust/parity; find golden golden-tz golden-rich fixtures-rich store-rich store-v11.sqlite -type f | sort | xargs sha256sum | sha256sum
aa6fcaed763cd89336f3ac8307ca033333baed616f5077d2dfd2eafa338bff92
$ file count; sk- count in gen_golden.py; lines
310; 1; 988 gen_golden.py
```
(combined oracle hash before the task; it must be identical after.)

**AFTER**: captured 2026-11-10 by the Supervisor:
```
$ cd rust/parity; find golden golden-tz golden-rich fixtures-rich store-rich store-v11.sqlite -type f | sort | xargs sha256sum | sha256sum
aa6fcaed763cd89336f3ac8307ca033333baed616f5077d2dfd2eafa338bff92     (equal to the BEFORE value)
$ ruff check rust/parity -> All checks passed!
```

**DELTA**: the oracle generator now fails loudly where it used to pass silently (missed clock patch, new prompt or resource, a present `/work/demo-app`), deletes only what it wrote, refuses a home folder or a not-yet-existing `.claude*` / `.easy-*` as `--out` or `--fixtures`, and warns when installed packages drift from `uv.lock` — with every committed oracle byte unchanged.

**WITNESS**: Supervisor, 2026-11-10, re-ran the three generator modes, the hash and the refusal probes independently of the implementing agent.
