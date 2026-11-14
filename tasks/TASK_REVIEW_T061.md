# TASK_REVIEW — T061: parity run of Python against Rust

> Sibling of `tasks/TASK_GUIDE_T061.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/parity/test_parity_report.py` (21 tests after the review rounds: AC1 inventory read from Python and a row for every surface, a new surface fails it; AC4 each README item exists and holds its stated words, a rule explains only what it describes, a missing store only when Python made one; AC5 each `differs: new` has Reproduce / Python / Rust / Proposed task; AC6 report and scanner; N1-N3 do not hide a difference). Red first: before `docs/PARITY_T061.md` existed, 5 of 9 tests failed with `docs/PARITY_T061.md does not exist: run parity/run_parity.py first`. Green: `passed: 0 failing` (21 `ok` lines). Round 3 red first: 5 failures (usage-error findings, blind check, bare `saved_utc` mask, counts line, inventory temp folder). Runner: `rust/parity/run_parity.py` (also a negative control: Rust on a store with one more workspace must be a difference, or the run stops). |
| Verification command run | ☑ pass | Run from `rust/` of the worktree (the worktree has no venv: the parity venv of the main checkout, `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/rust/parity/.venv/bin/python`, was used): `cargo build` -> `Finished dev profile ... in 0.13s`; `env -u CLAUDE_CONFIG_DIR <venv python> parity/run_parity.py` -> `87 rows, 179 probes -> .../rust/docs/PARITY_T061.md` and `Counts: equal 25 + equal except JSON key order 29, differs README 27, differs new 6, not run 0.` (after review round 3); `<venv python> parity/test_parity_report.py` -> 21 `ok`, `passed: 0 failing`; `cargo test --workspace --no-fail-fast` -> exit 0, 882 passed, 0 failed, 29 ignored (sum of the 70 `test result:` lines; run before the last edit of `run_parity.py`, which changed only `check=False` arguments). |
| Negative cases hold | ☑ pass | The runner's control probe (Rust on a world with one more workspace) is seen as `new`; unit tests: an exit-code difference, an extra stdout, or a missing-store text with no store made by Python is `new` even when the rule is declared; a changed number stays a difference after N1-N3. |
| verify | ☑ pass | Supervisor, 2026-11-14, on `d905887`: skill run, feature confirmed working — pass. This task has no app code, so the runner is the real run: `run_parity.py` regenerated `docs/PARITY_T061.md` (87 rows, 180 probes) and the file was identical in a foreground run and in a background job (`diff`), `parity/test_parity_report.py` 21 `ok`, `passed: 0 failing`; `git diff --stat main...T061 -- rust/crates rust/Cargo.toml rust/Cargo.lock src tests web` empty. Three findings re-checked by hand against Python and Rust without the runner: `--version` (`0.4.0` against `easy-personal-rs 0.2.0`), `prompts/get` of an unknown prompt (code `0` against `-32602`), `ping` equal. Not covered: the real Secret Service, a real `claude`, `systemctl`, a browser; macOS and Windows |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `git diff main...T061`: 4 files (`rust/parity/run_parity.py`, `rust/parity/test_parity_report.py`, `rust/docs/PARITY_T061.md`, this file); no app code. One independent read-only reviewer (correctness and security) read the whole runner, the test file, the report and the cited README items. Findings: P0 0, P1 2 (usage and date error texts filed as documented although no README item describes them; about 30 route rows counted `equal` with a different key order), P2 7, P3 several; plus my own P1 (the report depended on the SIGINT disposition the runner inherited: foreground and background launches gave different reports). Fix rounds `bb7cfef` and `d905887` closed both P1s and P2-3, 4, 6, 7, the `saved_utc` race and the leftover temp dir; the rest is written in the report's "Known limits" (the negative control only proves CLI stdout; `r_missing_store`; unchecked `<VERSION>` column; dropped `mcp get` calls; SIGTERM skips `finally`; static fixture has no dotfile or symlink). Security part of the review: no real data, keychain, `claude`, `systemctl` or `crontab` reached (env built from scratch, fakes first on PATH), no `shell=True`, children killed by process group in `finally` |
| Full smoke suite still green (no regression) | ☑ pass | `cargo test --workspace --no-fail-fast`: exit 0, 882 passed, 0 failed, 29 ignored. No file of `src`, `tests`, `web`, `rust/crates`, `rust/Cargo.toml`, `rust/Cargo.lock` changed: `git status --short -- src tests web rust/crates rust/Cargo.toml rust/Cargo.lock` printed nothing. |
| **Companion score (easy-verifier)** | ☑ pass (advisory) | `score` scope=`changes` ref=`main` task=T061, change_kind `chore`: `overall` 41 (4 of 13 dimensions rated: blast-radius 46, requirement-fidelity 33, security 30, test-strategy 55); `change_risk` abstained (`all_metrics_abstained`). Low signal, the same numbers as T050 and T060: the `security` 30 comes from lockfile and manifest redaction hits in the repo, not from this diff. report: /home/hungnguyenhuu/.claude-hung2/projects/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp-rust/e64d19ef-48c3-4f22-9423-6b479756ddac/tool-results/mcp-easy-verifier-score-1794642666204.txt |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | QA tooling and a generated report, no UI component |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | QA tooling, no UI component |
| **UI: Responsiveness at target viewports** | ☑ N/A | QA tooling, no UI component |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-14T05:08:46Z, branch `T061` at `8d1792a`, before any implementation commit):

```
$ date -u +%FT%TZ && ls rust/parity/run_parity.py rust/docs/PARITY_T061.md rust/parity/test_parity_report.py; echo "exit=$?"
2026-11-14T05:08:46Z
ls: cannot access 'rust/parity/run_parity.py': No such file or directory
ls: cannot access 'rust/docs/PARITY_T061.md': No such file or directory
ls: cannot access 'rust/parity/test_parity_report.py': No such file or directory
exit=2
```

Size of the job, read from Python (argparse tree, Starlette route table, FastMCP tool and prompt managers; `create_app(..., static=<a folder>)` so the static mount is listed; HEAD not counted):

```
cli leaf subcommands: 32        (plus the top-level --version option)
routes (method,path): 40        (39 API method+path pairs and the "/" static mount)
mcp tools: 12 prompts: 1
```

That is 85 surfaces (86 with `--version`) to run on both apps.

**AFTER**: 2026-11-14, `env -u CLAUDE_CONFIG_DIR rust/parity/.venv/bin/python parity/run_parity.py` from `rust/` on `d905887`; the head of the generated `rust/docs/PARITY_T061.md`:

```
Counts: equal 25 + equal except JSON key order 29, differs README 26, differs new 7, not run 0.
87 rows (86 surfaces read from Python, 1 protocol row), 181 probes, each run on Python and then on Rust over the same world.
New differences: cli --version; cli mcp (http, Ctrl-C exit); cli usage (--by, --since error texts); mcp prompt investigate (unknown prompt code); mcp protocol initialize (capabilities, unknown method code, stderr log); route GET /api/live (charset); route GET /api/workspaces (404 against 405 when the page is mounted)
```

**DELTA**: one command now lists every Python CLI subcommand, route and MCP tool, runs it on Python and on Rust over the same store copy, and says which differences are already documented in the README and which are new (7 findings, all Low risk), so T063 can decide to remove Python on evidence.

**WITNESS**: Supervisor session, 2026-11-14: re-ran the runner in both launch styles and diffed the report, re-ran the 21 tests, and checked three findings by hand against Python and Rust directly. `memory/event-trace/T061.jsonl` was not available.

**Stage 5 addendum (Supervisor, 2026-11-14), after merge `cc3df7d` and fix round 4 `3c255d3` (merge in `main`)**: running `run_parity.py --check` from the main checkout, which has the built dashboard bundle (`src/easy_personal/dashboard/static`, gitignored), failed (`route: GET /api/workspaces` was `differs: new` with no FINDINGS entry): the runner depended on the bundle. Fixed in `3c255d3` (Python always gets an explicit static folder; a new probe `guard-with-page` covers the real deployment; 3 tests, red first with a throwaway bundle). After the merge, from `main` with the real bundle: `--check` printed `the report equals a fresh run`; a fresh `--out` report is identical in a foreground run and in a background job; `parity/test_parity_report.py` 24 `ok`, `passed: 0 failing`. Lesson: run the runner's `--check` from the main checkout before merging, not only from the task worktree.
