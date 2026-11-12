# Parity oracle (T003)

Golden JSON captured from the Python app over `tests/fixtures/claude_code` (the default oracle, one day of data:
it proves the shape of most answers, not their logic) and, since T007, over a richer invented store (`--rich`, below).
The Rust build is later proven against both (RS-003). Python is only read, never changed.

## Build the venv (once, no global install, no sudo)

The generator imports the Python app, so the goldens are only as repeatable as the packages the venv holds. Install them
**from the lock**. From the repo root, with `uv`:

```bash
UV_PROJECT_ENVIRONMENT=rust/parity/.venv uv sync --frozen    # exactly the versions in uv.lock (app + dev group), into the parity venv
```

or, with `pip`, from a constraints file exported from the lock:

```bash
python3 -m venv rust/parity/.venv
uv export --frozen --no-hashes --no-emit-project -o "$TMPDIR/constraints.txt"
rust/parity/.venv/bin/pip install -c "$TMPDIR/constraints.txt" -e . pytest anyio ruff
rust/parity/.venv/bin/python -m pytest tests/surfaces/test_api.py -q   # the app's own tests must pass here
```

(Neither `uv` command was run on the machine that wrote this note: `uv` is not installed there. The venv in use was built with a bare `pip install -e .`, as the next paragraph warns against.)

Do not use a bare `pip install -e .`: it resolves the newest versions that `pyproject.toml` allows and ignores `uv.lock`, so the
venv drifts (on 2026-11-10: starlette 1.10.0 installed against 1.9.0 locked, anyio 4.16.0 against 4.15.1, sse-starlette 3.7.0
against 3.5.0). A new starlette can change a response body or a header, and then a regeneration on another machine writes
different goldens for no reason the Python app can explain. The generator prints one line on stderr when the installed
`starlette`, `anyio`, `sse-starlette` or `mcp` differs from `uv.lock` (it never fails the run, and the manifest is not changed:
`versions` already records what was installed).

The generator is POSIX only (`pwd`, `time.tzset`); it does not run on Windows.

`rust/parity/.venv/` is gitignored.

## Generate

```bash
rust/parity/.venv/bin/python rust/parity/gen_golden.py            # writes rust/parity/golden/ and store-v<N>.sqlite
rust/parity/.venv/bin/python rust/parity/gen_golden.py --out DIR  # same, elsewhere (the verification command diffs two runs)
rust/parity/.venv/bin/python rust/parity/gen_golden.py --tz         # opt-in: writes rust/parity/golden-tz/ only (T009)
rust/parity/.venv/bin/python rust/parity/gen_golden.py --rich       # opt-in: writes rust/parity/golden-rich/ and store-rich/ only (T007)
```

A run replaces only what an earlier run wrote. Each mode records the folder it replaces (`golden`, `golden-tz` or `golden-rich`;
the store files beside it belong to that folder) in `<out>/.gen_golden.json`, at the `--out` root and never inside an oracle folder, so
a `diff -r` against a committed folder stays empty. A folder with no such record is still replaced when its `manifest.json` names this
generator (the committed layout). Any other `golden*/` folder, and any `store-v*.sqlite*` file that nothing proves is ours, stops the
run before one file is deleted ("was not made by this generator"). A symlink is never replaced.

Output, byte-identical on every run of one machine:

| File | What |
|------|------|
| `golden/api__<route>__<case>.json` | body of one read-only `GET /api/*` answer (sorted keys, 2-space indent) |
| `golden/mcp__<tool>__<case>.json` | structured result of one MCP read tool, the tool list, the `investigate` prompt |
| `golden/manifest.json` | per entry: surface, route or tool, params, HTTP status; versions; store `user_version`; SHA-256 of the fixture tree; `volatile` (what is normalised); `patched`; `skipped` with a reason each |
| `store-v<N>.sqlite` | the ingested store (`N` = the last migration number), single file, for the Rust tests to open read-only |

## How it stays deterministic and safe

- `TZ=UTC` (the default run; `--tz` below runs other zones), a frozen clock (`frozen_now` in the manifest), a fake `HOME`, and every folder the app reads
  (`EASY_PERSONAL_HOME`, agent root, easy-memory, verifier reports, context URL) points at a throwaway
  or fixture path. The fixtures are only read.
- Refuses (exit 1, nothing opened) when `EASY_PERSONAL_HOME`, `EASY_PERSONAL_CLAUDE_PROJECTS`, `EASY_PERSONAL_AGENT_ROOT`,
  `EASY_PERSONAL_UI_REPORTS`, `EASY_PERSONAL_VERIFIER_REPORTS`, `EASY_MEMORY_HOME`, `--out` or `--fixtures` is, or is inside, a
  `.claude*`, `.codex*` or `.easy-*` folder of a real home (`$HOME` and the passwd home), **whether or not that folder exists
  yet**. Paths are resolved first, so a symlink and a parent that does not exist yet are caught. `EASY_PERSONAL_AGENT_ROOT`,
  `--out` and `--fixtures` equal to a real home are refused too, and `--fixtures` above a home (it would read the transcripts
  below).
- Refuses when `/work/demo-app` exists on the machine (default and `--tz` modes): the default fixtures name it as their `cwd`
  and the `kit_summary` golden holds that the folder is gone.
- Fails loudly when the app drifts under the generator: a module the frozen clock patches no longer imports `datetime` as
  `from datetime import datetime` (the run names the module), and `list_prompts()` / `list_resources()` differ from the known
  lists (a new prompt or resource is named), as a new GET route or tool already did.
- The store's machine id, fixture paths and inode numbers are replaced with fixed values (listed under
  `volatile`). Keychain, `claude` CLI and systemd answers are pinned (listed under `patched`).
- The run fails if any output contains `/home/`, `/Users/`, `sk-` at the start of a word (`task-`, `risk-` and `desk-` are not a
  key) or `ghp_`, or if an `/api` GET route of the app is neither a golden entry nor a skipped one.
- Mutation check: `--fixtures DIR` ingests a changed copy of the fixtures; the matching golden files change.

## Non-UTC zones (`--tz`, T009)

The default goldens are made under `TZ=UTC`, so they cannot show a time-zone bug: Python groups `by=day` and reads an offset-less
`since`/`until` in the machine's local zone. `--tz` writes a second oracle, and never touches `golden/` or `store-v<N>.sqlite`
(`diff -r` of a default run against the committed `golden/` stays empty):

| File | What |
|------|------|
| `golden-tz/store-v<N>.sqlite` | the fixtures plus 30 extra assistant messages (UTC times listed in `manifest.json` `extra_events`, `TZ_EVENTS` in the script: day edges, both New York DST days, a year edge and a leap day), ingested like the default store |
| `golden-tz/<zone>/api__usage_<summary\|breakdown>__<case>.json` | the answers of `/api/usage/summary` and `/api/usage/breakdown` under that zone (`TZ=<zone>` + `time.tzset()`), over the store above |
| `golden-tz/manifest.json` | per zone: the entries (route, query, status), the extra events, `tzdata_version`, versions |

Zones: `Asia/Bangkok`, `America/New_York`, and `Nowhere/Invalid` (not a zone: libc, so Python, reads UTC). The extra events are generated into a
temporary copy of the fixtures; `tests/fixtures/` is never written. The Rust tests (`rust/crates/core/tests/usage_tz_parity.rs`) read the
manifest, so a new entry in `TZ_ENTRIES` needs no Rust change. The goldens depend on the system tz database (`tzdata_version`): regenerate on a machine
with the same version, or compare the manifests first.

## Key order: which rule wins (T007)

The golden files are written with **sorted keys** (T003), so a file does not show the order in which the app emitted its
keys, and the Rust tests compare parsed JSON (key order ignored, list order kept). Where the emitted order matters, the
rich manifest carries it: every entry of `golden-rich/manifest.json` has `raw_sha256`, the SHA-256 of the raw body text
as the app emitted it (HTTP: the response body; MCP: the structured result as compact JSON in the order FastMCP returned
it), taken after the `<WORK>`/`<FIXTURES>`/`<REPO>` scrub. So: **sorted keys decide the golden files; `raw_sha256` decides the
order the app emitted.** A port that must also match the order serialises compactly (no spaces) and hashes its own body.
`golden/manifest.json` (the default oracle) is not changed and has no `raw_sha256`. An entry whose body holds text that
is specific to CPython (`api__usage_sessions__bad_since`, the `datetime.fromisoformat` message) has `raw_sha256: null` and a
`compare` note: only the status (400) and the `error` key are compared; the manifest repeats it under `compare_notes`.

## A richer oracle (`--rich`, T007)

The default store has 39 events on one day, so warnings and plan only abstain and outliers, kit, board, accounts, workspaces
and reports are empty. `--rich` writes a second oracle and never touches `golden/`, `golden-tz/` or `store-v<N>.sqlite`
(a default run is still byte-identical to them):

| File | What |
|------|------|
| `fixtures-rich/` | invented transcripts and kit-project files, committed (all text fictional); written once by `make_fixtures_rich.py`, which the generator does not run |
| `store-rich/store-v<N>.sqlite` | the store built from them the way the CLI does: `accounts.detect` + `confirm`, `claude_code.ingest_accounts` (two accounts), `kit_trace.ingest`, `context.ingest` (the `list_indexes` reply is supplied, no network), `ui` / `verifier` ingest, then `workspaces.add` (2), `accounts.use`, `reports.save` (2) |
| `golden-rich/api__*.json`, `golden-rich/mcp__*.json` | every route and MCP tool of the default mode over that store, plus the filter parameters |
| `golden-rich/manifest.json` | per entry: surface, route or tool, params, status, `raw_sha256`; `rule_coverage`, `cannot_fire`, `windows`, `volatile`, `patched`, `skipped`, versions, the frozen clock, TZ, the fixture tree hash |

What the store holds: 202 events over 2026-01-02..19 (18 days; the clock is frozen at 2026-01-20T00:00:00Z), 48 sessions, 3 models
(`claude-opus-5-5` and `claude-sonnet-5-5` are in `prices.yaml`, `claude-fixture-model-1` is not, so its cost abstains),
two agent accounts (`~/.claude` as the default account and `~/.claude-work`, ingested through `ingest_accounts`; both hold sessions in
the same project folder `alpha`), two workspaces (`acme` = alpha + a gone folder, `personal` = beta + a gone folder) with sessions in
neither (`gone-c`, and sessions with no project), two saved reports (one scoped to `acme`), and two kit projects with kernel traces (64 calls and one unreadable line),
a board (`PROJECT_KANBAN.md` + `.claude/board.json`), a memory graph (3 notes, one dangling link) and report folders
(`reports/`, `smoke-reports/`). A session straddles the window start; 37 timeline gaps; outliers hold a session over 200,000 context tokens
and one with 90 tool calls; `kit_findings` has 12 sessions on each side of easy-ui in one stratum.

Filter parameters covered: `workspace` and `account` (by label and by id, one workspace + one account, and unknown values that answer 400) on
usage summary / breakdown / outliers / sessions, warnings, plan, kit and board (HTTP), and on MCP `warnings`, `improvement_plan`, `kit_summary`;
`/api/kit/workspaces` with `account`; the report-folder parameters `repo_no` and `folder` (a folder, a miss, a bad `repo_no`, a file, a
missing file); `/api/memory-graph?project=`. Every GET route and MCP tool that is neither a golden entry nor skipped fails the run, as in the default mode.

### The seven rules (which entry makes each fire)

Windows: the rules read 2026-01-13..20 (and 2026-01-06..13 for `tokens_jump`). `account_default` = the default account, `account_work` = the
work account. Entries named below are `warnings__<name>` (`api__warnings__*.json`, `mcp__warnings__*.json`); the manifest's `rule_coverage`
lists every entry per rule and state (the generator fails the run if a rule is never measured and clear, or, but for `context_index_stale`, never fires).

| Rule | Fires in | Measured and clear in | Made by |
|------|----------|-----------------------|---------|
| `cache_hit_low` | `all`, `account_default`, `workspace_acme`, `workspace_personal` | `account_work`, `workspace_acme_account_work` | account A messages read 10 % of the prompt from cache, account B 90 % |
| `session_huge_no_compact` | `all`, `account_default`, `workspace_acme` | `account_work`, `workspace_personal`, `workspace_acme_account_work` | 8 open sessions of 155,000-205,000 context tokens (one over 170,000 with no compaction) against 7 small ones; a ninth session compacts (context below half its peak) and is left out of the median |
| `tokens_jump` | `all`, `account_default`, `workspace_acme`, `workspace_personal` | `account_work`, `workspace_acme_account_work` | account A: small sessions the week before, huge ones in the window; account B uses one template in both weeks (change 0.0) |
| `project_dominates` | `all`, `account_default` | `account_work` (abstains in the single-project scopes) | see the note below |
| `tool_errors_high` | `all`, `account_default`, `account_work`, `workspace_acme`, `workspace_acme_account_work` | `workspace_personal` | kernel traces: alpha 12 of 30 failed (0.4), beta 3 of 30 = **0.1, exactly the threshold** (the rule fires above it); 4 older failed calls and one unreadable line are outside the count |
| `records_unreadable` | `all`, `account_default` | `account_work`, both workspaces | 4 records of unknown types (version 2.1.330, account A only) |
| `context_index_stale` | never | `all`, `account_default`, `workspace_acme` (left out when the scope has no easy-context call: `account_work`, `workspace_personal`, `workspace_acme_account_work`) | see below |

**Rules that cannot be made to fire in isolation, and why**

- `context_index_stale` cannot fire. It fires when a project's newest commit is more than 60 s newer than its easy-context index, so a project
  needs a git repo at a path that exists on the machine that runs the test. The golden must not depend on that, and the store's project paths
  are gone folders on any test machine. The rule is measured (0 stale projects, from a `list_indexes` reply the generator supplies; the two
  stored indexes point at a live non-git folder and a gone folder, so no commit is found) and never fires. The firing path stays covered by the 36
  synthetic cases in `warnings_parity.rs`, not by this oracle.
- `project_dominates` needs two projects in the window. A project is a git repo that exists on disk, else every gone folder, every folder under the
  temp folder and every no-folder session is one `Scratch` group. The store therefore has no real project to name. The rule fires between the two groups
  that exist on every machine: `Scratch` (gone folders, and the kit folders, which sit under the temp folder in the generator and are gone in the saved
  store) and `(unknown)` (headless sessions with no `cwd`). Python and Rust must agree on that grouping, but this oracle does not prove the
  folder-name path of a real repository; `/api/workspaces/unassigned` is empty for the same reason (every project is Scratch).

### What the store and the goldens do not depend on

- Paths: the generator copies the kit projects and the two agent roots into its throw-away work folder, replaces `@KIT@` in the transcripts with that
  folder, ingests, and saves a copy of the store in which the work folder is replaced by `/work/rich` in every text column. That folder does not exist
  on a test machine, so usage groups it as `Scratch`, the same as in the generator (a folder under the temp folder). The goldens that read the live folders (kit,
  board, memory graph, report folders, accounts, workspaces) hold `<WORK>/...`; a Rust port of those routes has to rebuild the folders from `fixtures-rich/`
  and substitute the same token. The generator checks that the saved store answers usage and warnings exactly like the live one (`check_saved_store`).
- Time: the clock is frozen, TZ is UTC, every copied file gets mtime 2026-01-18T00:00:00Z (report-folder and board answers show file times).
- Machine state: keychain, `claude` CLI and systemd are patched as in the default mode, and the two `writer` calls that would reach them raise.
- Leaks: the run fails on `/home/`, `/Users/`, `sk-` (at the start of a word) or `ghp_` in any golden, the manifest or any fixture file, and checks that the saved store holds no work folder path.

### Rust tests

`rust/crates/core/tests/rich_parity.rs` reads the manifest and compares, as parsed JSON, `usage_summary` and `usage_breakdown` (all `by`, with and
without a window, under `TZ=UTC` in a child process) and `rules::evaluate` against the unscoped goldens (API and MCP). It also recounts the
rule coverage from the files, checks the `raw_sha256` and `bad_since` rules above, and runs a mutation check (one changed value in a temp copy of the
store fails the usage and the warnings comparison). It only counts the entries with a `workspace` or `account` parameter, the report-folder entries and the
entries of the other routes; those are compared by the test of their own slice: `scope_parity.rs` (the 41 scoped usage, warnings and plan entries), `plan_parity.rs`,
`sessions_parity.rs`, `kit_parity.rs` and `kitperf_parity.rs`, `report_files_parity.rs`, `reports_parity.rs`, `accounts_parity.rs`, `workspaces_parity.rs`,
`connectors_parity.rs`, `folders_parity.rs`, `jobs_parity.rs` and `memgraph_parity.rs` (the board and the memory graph, with `golden-memgraph/`). On this branch every GET route
of the Rust server and every MCP tool it serves has a golden or an oracle; the Python routes it does not serve at all are `/api/live` and the write routes
(the accounts, connector and workspace writes), which are other tasks.

## Ingest oracle (T026)

What the real Python `easy-personal ingest` writes, frozen for the P2 Rust ingest (T027-T033): exit code, stdout, stderr and
the store tables after every run, plus the schema of a fresh v11 store. Nothing in P2 is proven without it.

```bash
python3 rust/parity/make_fixtures_ingest.py                     # only to change the inputs: rewrites rust/parity/ingest/fixtures/
rust/parity/.venv/bin/python rust/parity/gen_ingest.py          # (re)writes rust/parity/ingest/golden/
rust/parity/.venv/bin/python rust/parity/gen_ingest.py --check  # regenerates in a temp folder; exit 0 equal, 1 different (names the files)
```

The generator needs the parity venv (above) and `git`. Each scenario runs the CLI entry (`easy_personal.surfaces.cli:main`, what the
`easy-personal` script runs) in a child process with a fixed environment (`manifest.json` `child.env`: `TZ=UTC`, `LC_ALL=C.UTF-8`,
`PYTHONHASHSEED=0`, git config isolated, easy-context pointed at a dead port) and fresh temp folders. It uses the real-home guard of
`gen_golden.py`, refuses `/work/demo-app` (the committed fixtures' `cwd`), a temp folder on `/mnt` or inside a git repo, and fails on a home path,
an `@`, a token-shaped string or a planted canary (`FAKE`, `INVENTED`) in any golden, and on a home path, an `@` or a real-looking key in any fixture.
Keep `TMPDIR` short (the work root is padded to 240 characters) and on the Linux filesystem.

### Inputs

| Path | What |
|------|------|
| `tests/fixtures/claude_code/` | the committed fixtures (read only; copied, last lines still truncated) |
| `ingest/fixtures/synthetic/` | the invented S6 set, one folder per theme: line framing, timestamps, cwd, usage, labels, split messages, tools and tool results, every record type, unversioned records, entrypoint back-fill, a sidechain file, project resolution |
| `ingest/fixtures/crash-*/` | one crashing input each (deep nesting, an int beyond int64, a folder named `x.jsonl`, a list as `type`) with a file before and after it |
| `ingest/fixtures/s3`, `s5`, `s7`, `outside/` | inputs of single scenarios; `s7/agents/<name>` is copied to `<AGENT_ROOT>/.<name>` |
| `ingest/fixtures/cases.json` | every line of every input file: file, line, byte offset, case. A Rust test maps a golden row back to its case by `(file, byte_offset)` |
| `ingest/fixtures/repos.json` | the work-root layout: folders, git commands (fixed author, date and config), symlinks, the cwd cases, `work_path_length` |

`%WORK%` in a transcript is the work root. The generator makes a work root whose path is exactly `work_path_length` (240) characters,
so every byte offset is the same on every machine; a Rust test pads its own temp folder the same way and runs the `repos.json` steps in order.

### Goldens (`ingest/golden/`)

| File | What |
|------|------|
| `S<NN>_<name>.json` | one scenario: `setup` (the steps, tokenised), then `runs`; each run has `argv`, `exit_code`, `stdout_json`, `stdout_equals_json_dumps`, `stderr_text_tokenised`, `exception` (class and last line of a traceback), `home_files`, `home_mode`, `tables_after` |
| `schema-v11.sql` | every `sqlite_master` row of a fresh store, sorted by name, as an executable script, then the seed row and `PRAGMA user_version = 11` (the generator proves it re-creates the same `sqlite_master`) |
| `schema-v11.json` | the same rows, the seed rows and the pragmas (`user_version` 11, `journal_mode` wal, `foreign_keys` 1, `busy_timeout` 5000) for an exact comparison |
| `manifest.json` | versions (Python, SQLite, git), fixture tree hashes, the child's env, the tokens, `volatile`, the scenario list with exit codes, sha256 of every file |

`tables_after` holds `events`, `tool_signals`, `source_files` and `agent_accounts` row by row (one row per line), `row_counts` of every table,
`user_version` and `machine_id_check`. Each table says which columns it `dropped` (row ids, `machine_id`) and which it `replaced`: paths become
`<HOME>`, `<AGENT_ROOT>`, `<WORK>`, `<CASE>`; `inode` becomes `<INODE_OF_FILE>` / `<INODE_OTHER>` / `<INODE_FILE_GONE>` (compared with the file at
that path after the run); `events.agent_account_id` becomes the account's folder. `volatile` lists the one text that changes between runs
(the C stack figure in CPython's `RecursionError` message, S06a).

### Scenarios

| Id | What | Exit codes |
|----|------|-----------|
| S01 | cold ingest of the committed fixtures (default account detected by the CLI) | 0 |
| S02 | the same ingest twice (second: `new_events` 0) | 0, 0 |
| S03 | one line appended to a file (incremental; earlier rows take the session's entrypoint) | 0, 0 |
| S04 | offsets reset to 0 with `entrypoint` / `tool_error_count` cleared: the upsert back-fills them, no new row | 0, 0 |
| S05 | a last line truncated at the first run, completed at the second | 0, 0 |
| S06 | cold ingest of the synthetic set (`repos.json` layout, a symlinked folder that is not followed) | 0 |
| S06a-d | deep nesting, int beyond int64, a folder named `x.jsonl`, a list as `type`: Python crashes (exit 1), the crashing file rolls back, later files are not read, every next run crashes on the same file | 1, 1 |
| S07 | `.claude` (detected) and `.claude-work` (`account confirm`), `.claude-other` detected only (not read), a `.codex` folder, a `.claude.json` file | 0, 0 |
| S08a / S08b / S08c | `--retention-days 36500` (nothing pruned), `1` (every timestamped row pruned, NULL-timestamp rows kept), `0` (argparse refuses) | 0 / 0 / 2 |
| S09 | `ingest.lock` held by the generator, `EASY_PERSONAL_LOCK_TIMEOUT=0.5` | 75 |
| S10a / S10b | `--source PATH` with a default account (tagged with it, its own folder not read) and with none (untagged) | 0 / 0 |
| S11 | a store with `user_version` 12: `RuntimeError` traceback | 0, 1 |
| S12 | a v10 store made with `store.migrate(..., MIGRATIONS[:10])`: backed up to `store.sqlite.bak-v10`, migrated, ingested | 0 |
| S13 | a file replaced by an identical copy (new inode): read again, duplicate rows | 0, 0 |
| S14 | a file deleted between runs: its `source_files` row is retired, its events stay | 0, 0 |

## Ingest leak check (T030)

The privacy gate of the Rust ingest (NFR-002, RS-003): the Python leak suite's planted canaries go through the **Rust** binary, and no
Rust surface may show any of them. `leak_rust.py` imports `tests/leak/canaries.py` and `tests/fakes/jobs_fakes.py` read-only (by
`sys.path`; `tests/` is never written) and plants the same trees as `tests/leak/conftest.py::leak_run` (transcripts, kit traces,
verifier and easy-ui reports, memory and easy-memory notes, `.claude.json`, the limits file, fake `systemctl` / `crontab`) into a fresh
temp folder. Rust P2a ingests only the transcripts; the other trees are planted and scanned anyway (a leak through a source Rust does not
read must still be impossible).

```bash
cd rust && cargo build                                   # the script runs rust/target/debug/easy-personal-rs
./parity/.venv/bin/python parity/leak_rust.py            # exit 0 clean; 1 on a hit or a failed check
./parity/.venv/bin/python parity/leak_rust.py --self-test   # the scanner's own check only
./parity/.venv/bin/python parity/leak_rust.py --keep        # keep the temp folder after a clean run (a failed run always keeps it)
```

It runs every child under `timeout` with an env built from scratch (`HOME` is an empty temp folder; `EASY_PERSONAL_HOME`,
`EASY_PERSONAL_AGENT_ROOT`, `EASY_PERSONAL_CLAUDE_PROJECTS` and the other source variables point into the temp folder), and refuses to
start when a variable or `TMPDIR` points at a real home or its `.claude*` / `.codex*` / `.easy-*` folders, or the temp folder is on `/mnt`.

**What is scanned** (each one a *surface*, scanned with `Canaries.find`: raw, JSON, `\u`, URL, base64 at 3 alignments, hex):
stdout and stderr of four `ingest` runs (an empty source that creates the store; the planted tree; the same again, which must add 0; and
`--retention-days 3650`); `store-info` and `store-info --json`; every `/api/*` route of `serve` over real HTTP with the token (query
variants and bad input), guard refusals (no or wrong token, wrong Host or Origin, a POST, an unknown route) and the static route
(`--static-dir`, with a dotfile and a traversal), plus the server's stdout and stderr; `mcp` over stdio (initialize, `tools/list`, every
tool with error paths, an unknown tool and method, a parse error, an invalid request) and its stderr; the raw bytes of `store.sqlite`,
`-wal` and `-shm` while a reader still holds the WAL open (the writer's frames are not merged yet) and after close, and at the end;
`sqlite3 .dump`; the data folder listing. `RUST_LOG=trace` is set, so any log line would be scanned too.

**Anti-vacuity** (each fails the run when false): the self-test (a canary in each encoding is found, also inside a SQLite file; a clean
blob has no hit; a hit is masked); ingest #1 `new_events > 0` and `events` rows exist; the failed `tool_result` is counted
(`SUM(tool_error_count) = 1`) while its text is nowhere; the **control canary** (the fixture's `sessionId`, stored on purpose) is found
in the open WAL and in the store file, so the scan does see inside them; the summary route and the `usage_summary` tool return data;
`serve` and `mcp` stop and leave no process (checked by PID); ingest #2 adds 0; the data folder holds only `store.sqlite*` and
`ingest.lock`; the planted tree is unchanged (sha256); every canary was planted somewhere and scanned on every surface; a route in
`api.rs` / `server.rs` or a tool in `tools/list` that the script does not call fails the run (register it in `API_CALLS` / `MCP_CALLS`).

**Reading the report**: `self-test` lines; `canaries: 42 ... set fingerprint <hash>` (it differs per run: the canaries are random);
one `canary <name> planted in <files>; scanned on N/N surfaces` line per canary; one `surface <label>: <B> bytes` line per surface;
the `check` lines; and the last line, `leak check: 0 hits over 42 canaries, <M> surfaces, <B> bytes`. A hit prints
`HIT canary <name> as <encoding> in [<surface>]: abcd[<name>]efgh` (at most 4 characters each side, the canary itself replaced by its
name; the whole report is scanned before it is printed) and the script exits 1.

**A hit is a product bug** in the Rust code that wrote that surface: report it with the canary name and surface, never weaken the scan.

**For the Supervisor** (not edited here): to add this to the `/verify` recipe, add after the Rust step of `skills/verify/SKILL.md`:
`cd rust && cargo build && ./parity/.venv/bin/python parity/leak_rust.py` (expects exit 0 and the `leak check: 0 hits` line).

## Kit sources oracle (T033)

`gen_ingest_kit.py` runs the real Python `easy-personal ingest` over planted `kit_trace`, `ui` and `verifier` folders and freezes the result in
`rust/parity/ingest/golden-kit/` (ten scenarios `K01`..`K09`, `K04c` is a Python crash: a folder named `x.jsonl` in an event-trace folder). A scenario is a list of
steps (write, append, rewrite in place, replace by a new inode, pad to a size, symlink, FIFO, remove, run the ingest); the steps are stored in the golden, so
there is no separate fixtures folder and the Rust replay (`crates/core/tests/ingest_kit_replay.rs`) plays the same steps. After every `ingest` step the golden
holds the exit code, the `kit_trace` / `ui` / `verifier` objects of the printed JSON (no `context`) and the tables `kit_trace_events`, `ui_sessions`,
`verifier_scores` and the `kit_trace` rows of `source_files` (ids, machine id and inodes left out, as T026). It reuses the guards and the `Case` of `gen_ingest.py`
(real-home refusal, `TMPDIR` must be on the Linux filesystem and not inside a git repo).

```bash
TMPDIR=/tmp rust/parity/.venv/bin/python rust/parity/gen_ingest_kit.py           # (re)write rust/parity/ingest/golden-kit/
TMPDIR=/tmp rust/parity/.venv/bin/python rust/parity/gen_ingest_kit.py --check   # regenerate in a temp folder, compare: exit 0 = 0 different
cd rust && cargo test -p easy-personal-core --test ingest_kit_replay             # the replay, no Python needed
cd rust && cargo test -p easy-personal-core --test ingest_kit_python -- --ignored   # real Python after Rust and Rust after Python add 0 rows
```

## Ingest speed and memory (T032)

NFR-005 for the Rust ingest, measured beside the Python ingest over the same generated data. `bench_ingest.py` (parity venv) makes
invented transcripts (seeded: the same bytes every run; about 3 KB per line; user prompts, assistant records split per content block
with one usage, tool calls and results, a few of 200-600 KB, attachments, system and metadata lines; no real text, path or secret):
set **A** 200 MiB in 72 files, set **B** one 12 MiB file, and **C** = A again, unchanged, into the store the cold run of A made. It runs
the **release** binary and the Python CLI (fresh data folder per cold run, an empty agent root, so the source is read untagged).

```bash
cd rust && cargo build --release && ./parity/.venv/bin/python parity/bench_ingest.py   # about 30 s; exit 0 = every Rust gate passed, tables equal
./parity/.venv/bin/python parity/bench_ingest.py --impl rust --runs 1                  # Rust only, one run
cargo test --release -p easy-personal-rs --test ingest_perf -- --ignored --nocapture  # the cargo-side gates (memory, re-run)
```

How it measures: wall time with `time.perf_counter`; peak RSS of the program as GNU `/usr/bin/time -f %M` reports it (the child's
`VmHWM` at exit; the script's own `wait4` would carry the script's RSS into the child, Linux keeps the high-water mark across `execve`);
the idle baseline is the same program running `--version`; median of 3 cold runs; MB = 2^20 bytes as in `tests/sources/test_ingest_perf.py`.
Before each cold run the dataset files are dropped from the page cache with `posix_fadvise(DONTNEED)` (best effort, no root: the program,
its libraries and, on WSL, the Windows host's disk cache stay warm). A speed number counts only when `events`, `tool_signals` and
`source_files` of every cold run equal the first Python run's (row id, machine_id and inode left out). It writes only under
`rust/target/bench/` (git-ignored; `datasets/` is reused, one new `runs/<time>/` per invocation of about 450 MB, nothing is deleted),
refuses with less than 1 GiB free, and prints the filesystem it used (a RAM-backed `tmpfs` would inflate the numbers).

Result, 2026-11-11 (AMD Ryzen 7 5700X, 16 threads, 16 GB, WSL2 kernel 6.18.58.1, ext4 on `/dev/sdd`, git 2.53.0, Python 3.14.4 with
SQLite 3.46.1, Rust with rusqlite's bundled SQLite (libsqlite3-sys 0.39.1)), after the T032 fix below:

| run | impl | seconds (median) | MB/s | peak RSS MiB | above baseline MiB | new_events | tables |
|---|---|---|---|---|---|---|---|
| baseline (`--version`) | rust | 0.00 | - | 4.2 | - | - | - |
| baseline (`--version`) | python | 0.38 | - | 60.9 | - | - | - |
| B 12 MiB, 1 file | rust | 0.08 | 147.6 | 10.0 | 5.8 | 4210 | equal |
| B 12 MiB, 1 file | python | 0.54 | 22.1 | 65.7 | 4.8 | 4210 | equal |
| A 200 MiB, 72 files | rust | 1.13 | 176.9 | 11.1 | 6.9 | 67078 | equal |
| A 200 MiB, 72 files | python | 2.66 | 75.0 | 66.4 | 5.5 | 67078 | equal |
| C unchanged re-run | rust | 0.01 | - | 7.5 | 3.3 | 0 | - |
| C unchanged re-run | python | 0.45 | - | 65.9 | 5.0 | 0 | - |

Gates (Rust): A 176.9 MB/s (>= 8), C 0.01 s (< 2), B +5.8 MiB (< 8): all pass; Python passes all three too. The cargo test
`ingest_perf` adds the worst case for memory, a 12 MB file of 27,053 small events (the shape of `tests/ops/test_ingest_memory.py`):
release +4.8 to +5.1 MiB, debug +7.4 to +7.5 MiB (a thin margin), and an unchanged re-run of 200 files in 0.012 s (release).

**The T032 fix**: before it, `folders.rs` collected all the `Line`s of a file before its one transaction, so memory grew with the events
of a file (about 0.47 KiB per event: 11.7 / 13.7 / 18.6 / 27.9 MiB peak for 5k / 10k / 20k / 40k small lines) and the 12 MB small-event
file took **+17.5 MiB** (gate failed); set B passed by 0.1 MiB (+7.9). Now `Store::save_file_streamed` reads the lines one by one inside
the same single transaction per file (a read error part-way rolls the file back and skips it, as before); peak is flat (9.5 / 9.3 / 9.3 /
9.4 MiB for the same four files). Still buffered per file: the `tool_signals` rows (inserted after the events, as Python does) and the
distinct sessions; `resolve_project` (git, once per distinct `cwd` per run) now runs inside the write transaction.

## Schedule oracle (T034)

What the real Python `easy-personal schedule install|remove` does, frozen for the Rust `schedule` command (RS-003):

```bash
rust/parity/.venv/bin/python rust/parity/gen_schedule.py          # (re)writes rust/parity/schedule/golden/
rust/parity/.venv/bin/python rust/parity/gen_schedule.py --check  # regenerates in a temp folder; exit 0 equal, 1 different (names the files)
cargo test -p easy-personal-rs --test schedule_cli                # the Rust binary replays every golden run
```

Every scenario runs the CLI entry in a child process inside a throwaway case folder. `HOME`, `XDG_CONFIG_HOME` and `EASY_PERSONAL_HOME`
point into it and `PATH` is the case's `bin/` alone: a stub `easy-personal`, and (when the scenario says so) `schedule/fake-systemctl.sh`
as `systemctl`. The real `systemctl` cannot be reached and nothing is written outside the case. The fake logs each call's arguments, prints
a marker on both streams (so a test can prove the tool's output is never echoed), and fails on request (`FAKE_SYSTEMCTL_FAIL=<verb>`,
`FAKE_SYSTEMCTL_NO_SESSION=1`). The generator refuses to run when an env var or `TMPDIR` points inside a real `.claude*`, `.codex*` or
`.easy-*` folder.

| File | What |
|------|------|
| `schedule/fake-systemctl.sh` | the fake; shared by the generator and the Rust test |
| `schedule/golden/<id>.json` | one scenario: per run `before` (steps done since the previous run: `systemctl` on/off the PATH, `put`, `mkdir`, `rm`, `stub`), argv, env, exit code, stdout, the kind of stderr, every file and folder left under the case, the `systemctl` calls |
| `schedule/golden/manifest.json` | scenarios and exit codes, sha256 of every golden, of the Python unit templates and of the fake |

| Scenario | What | Exit codes |
|---|---|---|
| D01 | the three dry runs (install, install `--retention-days 30`, remove) and an install dry run with no `systemctl`: nothing written, no call | 0 0 0 0 |
| I01 | install, install again with a retention, remove, remove with nothing installed | 0 0 0 0 |
| R01 | remove with only the timer file (`disable` and `daemon-reload` run), only the service file, neither | 0 0 0 |
| N01 / N02 | no `systemctl`; `systemctl` present but `--user show-environment` fails: the message and the cron line, nothing written | 1 1 0 / 1 1 |
| F01 / F02 | a `systemctl` call fails (enable, daemon-reload, disable, the last daemon-reload; no `systemctl` for a remove). **Python: traceback.** The files and the calls are frozen; the traceback text is not | 1 |
| W01 | the unit folder cannot be made (`systemd` is a file), a unit path is a folder. **Python: traceback** | 1 |
| X01 | the unit folder: `XDG_CONFIG_HOME` unset / empty / relative / `./rel/`, then a real install with it unset (units under `HOME/.config`) | 0 |
| U01 | bad arguments (retention 0, -5, abc, 1.5, no value, unknown flag, a retention on `remove`, no action, an unknown action) | 2 |
| Q01 | the command's quoting: the stub sits in a folder named plain, with a space, `'`, `"`, non-ASCII, `$`, `\`, `%`, `*`, a tab, `;`, and the characters that stay bare; the last run has no systemd and prints the cron line | 0 x 12, then 1 |

**The command is normalised.** Python's `ExecStart` runs `easy-personal` (the stub's path, `<CMD>` in every golden but Q01); Rust's runs
itself (`easy-personal-rs ingest`) on purpose. In Q01 the Rust test puts its own binary at the same path (a hard link, so `current_exe` is that
path) and compares the quoted line as it is. Folder names with `:` are left out: `PATH` cannot hold them. A newline in the path, and `%`
or `\` in it (which systemd reads as specifiers and escapes even inside quotes), are not handled by Python either; Rust copies `shlex.quote`.
