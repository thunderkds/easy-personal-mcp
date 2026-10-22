# learnings.md — Cold Tier: Clarifications, Patterns & Gotchas

> **Rules**: Supervisor-only writes. Each entry dated (`YYYY-MM-DD`) and citing the file/task it came from (the diff-driven pass greps this file by changed file path).

## Requirement Clarifications

### 2026-10-21 — Ignore Claude Code's `cost-state.totalCostUSD`
**Learning**: Owner confirmed: cost is only the labelled "API-equivalent cost" from the dated price table (FR-005); the transcript's own `totalCostUSD` is never read.
**Files**: `src/easy_personal/sources/claude_code.py`, `tasks/TASK_GUIDE_HUN-34.md`

## Patterns

### 2026-10-21 — Dashboard static build is gitignored; build before `uv build`
**Learning**: `src/easy_personal/dashboard/static/` is not committed. Run `npm --prefix web ci && npm --prefix web run build` before `uv build`, else the wheel ships no page; `scripts/check_static.py` fails on missing or stale builds. On a fresh checkout `tests/test_web_toolchain.py` skips 2 tests ("web/ not built in this checkout") — expected, not a regression.
**Files**: `scripts/check_static.py`, `pyproject.toml`, `web/`, `tests/test_web_toolchain.py`

### 2026-10-21 — Runtime verify against fixtures, never the real transcripts
**Learning**: Copy `tests/fixtures/claude_code/*/` into a temp dir as project folders and point `EASY_PERSONAL_CLAUDE_PROJECTS` at it (with a temp `EASY_PERSONAL_HOME`); then `ingest` → `usage` / MCP `usage_summary`, and grep every output + the store dir for `INVENTED_` / `planted-secre`.
**Files**: `src/easy_personal/core/config.py`, `tests/fixtures/claude_code/`


### 2026-10-21 — FastMCP returns structured output only for typed dicts
**Learning**: An MCP tool annotated `-> dict` returns no structured output; use `-> dict[str, Any]`. The package version reaches `initialize` only via the private `app._mcp_server.version` (no public argument in mcp 1.x).
**Files**: `src/easy_personal/surfaces/mcp.py`

### 2026-10-21 — Merge gate blocks while ANY task is In Progress
**Learning**: `.claude/hooks/pre_bash_block_unsafe_merge.py` refuses every `git merge`/`push` while any KANBAN row is In Progress, and needs a `verify` row containing "pass" for Ready for Review rows. With no review status in Linear (decision: In Progress → Done), move a task to Done only after its Evidence is complete, then merge. Parallel tasks must all reach Done before any of them merges. The command regex `\bgit\s+merge\b` also matches `git merge-base` and `git merge-tree`, so those read-only commands are blocked too: use `git rev-list --count A..B` to check divergence. The hook judges the whole Bash command before any part runs, so a compound `board push done && … && git merge` is refused: push the board status in one call, merge in the next. 2026-10-22: HUN-39/44/49 were verified together, pushed Done, then merged one by one (HUN-44 needed an additive conflict fix in `cli.py` + `tests/leak/harness.py`).
**Files**: `.claude/hooks/pre_bash_block_unsafe_merge.py`, `PROJECT_KANBAN.md`

### 2026-10-21 — Skills the Supervisor cannot run itself
**Learning**: `/verify` was user-invoked only on 2026-10-21; on 2026-10-22 the Supervisor loaded it with `Skill({ skill: "verify" })` (it returns the recipe; the Supervisor runs it); built-in `security-review` diffs `origin/HEAD` (remote here is `github`) on the current checkout, so the Supervisor reviews worktree diffs inline. Sub-agents have no Skill tool: write tdd red-green steps into spawn prompts instead of naming the skill.
**Files**: `skills/verify/SKILL.md`, `skills/craft-spawn-prompt/SKILL.md`

### 2026-10-21 — easy-context registered for this repo (local scope)
**Learning**: `claude mcp add --transport http easy-context http://127.0.0.1:8767/mcp --header 'X-Easy-Context-Root: ${PWD}'` (local scope, `~/.claude-personal/.claude.json`); tools load only after a session restart. Indexed 2026-10-21 (177 files, 1646 slices); retrieval rule in `CLAUDE.md → Context Retrieval`; re-index (`index_path`, incremental) after each merge to main.
**Files**: `PROJECT_SPEC.md`

### 2026-10-21 — easy-verifier registered for this repo (local scope)
**Learning**: `claude mcp add --transport http easy-verifier http://127.0.0.1:8766/mcp` (local scope, `~/.claude-personal/.claude.json`); tools load only after a session restart. Local scope is per Claude profile: a session started with `CLAUDE_CONFIG_DIR=~/.claude-wife` does not see it ; register it in each profile, or move both servers to a project `.mcp.json`. Until then, call the running server directly over HTTP JSON-RPC (initialize → `mcp-session-id` header → notifications/initialized → tools/call `score`, then `write_report` with `findings: []`); HUN-35/HUN-36 were scored that way. Pass the repo's absolute path as the tool's `repo` argument; reports land in `~/.easy-verifier-reports/<repo name>/`. Stage 4 step 4b runs `score` (`scope="changes"`, `ref="main...<branch>"`) on every task, advisory only (user decision 2026-10-21); no hook runs it.
**Files**: `PROJECT_SPEC.md`

### 2026-10-21 — easy-verifier `score` output: read with jq; known security false positives
**Learning**: First run (HUN-34 merge, `ref="bfd9c86...3f4aceb"`) returned 780,505 chars despite `budget_bytes=120000`; the harness saves it to a file, so extract `.overall`, `.ratings[]`, `.warnings` with jq. Overall 75 (7 of 13 dimensions rated). Security 18 is false positives: `sink_hits_observed` flags `core/metrics.py` f-string SQL (only a fixed column name is interpolated, values are `?` params); `redaction_hits_observed` 123 comes from a pack that reads `uv.lock` and `web/package-lock.json` plus planted test secrets. `score` writes no report; call `write_report` (findings `[]`) for the path.
**Files**: `src/easy_personal/core/metrics.py`, `docs/claude-md/pipeline-stages.md`

### 2026-10-21 — `board_sync.py pull` overwrites the `## Blocked` table
**Learning**: The Blocked section of `PROJECT_KANBAN.md` is a fixed footer rendered on every pull; manual rows vanish. Record external blockers in the task's guide (`Depends on` external note) and in `PROJECT_SPEC.md` § Tasks instead (e.g. XR-1 → easy-verifier T120 in `tasks/TASK_GUIDE_HUN-42.md`).
**Files**: `PROJECT_KANBAN.md`, `scripts/board_sync.py`, `tasks/TASK_GUIDE_HUN-42.md`

### 2026-10-21 — Git remote is `github`, not `origin`
**Learning**: Push with `git push -u github <branch>`; `origin` does not exist in this repo.
**Files**: `.git/config`

### 2026-10-21 — Linear escapes backticks in issue titles
**Learning**: Titles with inline code come back as `` \` `` in the KANBAN cache; cosmetic only, the guide bodies are intact.
**Files**: `PROJECT_KANBAN.md`

## Gotchas

### 2026-10-21 — New MCP tool or CLI command must be registered in the leak suite
**Learning**: `tests/leak/test_leak_suite.py` meta-tests fail for any MCP tool or CLI command missing from `MCP_CALLS` / `CLI_CALLS` in `tests/leak/harness.py`. Since HUN-39/44/49 the harness also has `API_CALLS` (dashboard routes, covered in-process) and `CLI_KEYCHAIN_COMMANDS` (`exec`/`headers`, covered in `tests/connectors` with a fake backend); `CLI_SERVER_COMMANDS` = `mcp`, `dashboard`. A new *flag* on an existing command (e.g. `usage --by`) is not caught: add its argv variants by hand. Tasks that add surfaces register them in the same branch (HUN-35 did, after merging HUN-36). Dashboard/API tasks add response bodies to `harness.Collected` (`tests/leak/README.md`).
**Files**: `tests/leak/harness.py`, `tests/leak/README.md`, `src/easy_personal/surfaces/mcp.py`, `src/easy_personal/surfaces/cli.py`

### 2026-10-21 — Open owner checks from HUN-35: price numbers and the long-context threshold
**Learning**: `src/easy_personal/data/prices.yaml` (as_of 2026-10-21) holds the implementer's approximations, not a checked price list. `OUTLIER_CONTEXT_TOKENS = 200_000` (one message's input + cache-creation + cache-read) was the implementer's choice; the guide gave no number. Both await owner confirmation. The price file is a one-line-per-model YAML subset read without a YAML library (no `pyyaml` dependency).
**Files**: `src/easy_personal/data/prices.yaml`, `src/easy_personal/core/prices.py`, `src/easy_personal/core/metrics.py`

### 2026-10-21 — Board push needs `LINEAR_API_KEY` in the session's shell
**Learning**: `board_sync.py push status <ID> done` fails with `LINEAR_API_KEY is not set` and queues nothing (`status` → 0 unsynced) when the session's environment lacks the key. Then only the `PROJECT_KANBAN.md` cache is updated; the Linear issues must be moved by hand or by a re-push from a shell that has the key.
**Files**: `scripts/board_sync.py`, `PROJECT_KANBAN.md`

### 2026-10-21 — `memory/codebase-map.md` does not exist
**Learning**: Guides' Mandatory Startup step 6 names it for C2/C3 tasks, but `map-codebase` was never run; agents report it missing. Run `/map-codebase` once, or drop the read from spawn prompts until then.
**Files**: `memory/codebase-map.md`, `templates/TASK_GUIDE_template.md`

### 2026-10-21 — Open owner check from HUN-37: compaction is inferred
**Learning**: The store has no `/compact` marker. `session_huge_no_compact` treats a session as compacted when a later message's context (input + cache tokens) drops below half of its earlier peak (`metrics.session_contexts`). Owner to confirm; a real marker needs a parser + schema change (separate task, `migration-safety`). Rules abstain (with a reason) below `min_history_days` or on an empty window; comparisons are strict, so a value at the threshold does not fire.
**Files**: `src/easy_personal/core/rules.py`, `src/easy_personal/core/metrics.py`, `src/easy_personal/data/rules.yaml`

### 2026-10-21 — Scheduled ingest: lock, exit 75, systemd only via `--dry-run` in tests
**Learning**: `ingest` takes an flock on `<data dir>/ingest.lock` (crash-safe); busy past `EASY_PERSONAL_LOCK_TIMEOUT` (default 300 s) → stderr message, exit 75. `schedule install|remove` call real `systemctl --user`; tests stub it and the leak suite registers only `--dry-run` variants. `ExecStart` takes `easy-personal` from PATH at install time. Pruned rows can return when a replaced transcript (new inode) is re-read, until the next prune. The systemd unit has no `Environment=`: the timer never sees the owner's shell env, so any setting a scheduled ingest needs must also live in `config.yaml` (read with `rules._read_flat`: `section:` + indented `name: value`; env overrides). HUN-43 nearly shipped env-only `EASY_PERSONAL_UI_REPORTS`, which would have cleared `ui_sessions` on every timer run; it now reads `ui: reports_dir:`.
**Files**: `src/easy_personal/core/lock.py`, `src/easy_personal/core/schedule.py`, `src/easy_personal/core/store.py`, `src/easy_personal/surfaces/cli.py`

### 2026-10-21 — `rm` inside `bash -c` is blocked by the harness safety check
**Learning**: A Bash call that runs `rm` inside a `bash -c '…'` script is refused (the check cannot inspect the script). For verify runs, give each case its own temp dir instead of deleting files, and do not wrap the recipe in `bash -c` when it removes anything.
**Files**: `.claude/skills/verify/SKILL.md`

### 2026-10-22 — Verify recipes for the dashboard, `kit_trace` and the launcher
**Learning**: Dashboard API token goes in the `X-Dashboard-Token` header (not `Authorization`); the URL is printed as `http://127.0.0.1:<port>/#<token>`; set `BROWSER=true` so no browser opens. `kit_trace` finds projects only from `claude_code` events' `cwd` that still exist on disk: rewrite a fixture transcript's `cwd` (`/work/demo-app`) to a temp repo holding `memory/event-trace/*.jsonl`. Launcher verify must never touch the owner's keychain: set `PYTHON_KEYRING_BACKEND` to a throwaway file backend on `PYTHONPATH` in the scratchpad, and `keyring.backends.fail.Keyring` for the refusal case.
**Files**: `src/easy_personal/surfaces/api.py`, `src/easy_personal/sources/kit_trace.py`, `src/easy_personal/launch.py`, `.claude/skills/verify/SKILL.md`

### 2026-10-22 — UI evidence without easy-ui: headless Chrome on a combined checkout
**Learning**: The easy-ui MCP is not registered in Supervisor sessions. Hard-Stop Gate 6 evidence was taken with `google-chrome --headless=new --window-size=<w>,2600 --virtual-time-budget=8000 --screenshot=… --log-net-log=…` against `easy-personal dashboard` (URL with `#<token>`), at 375/768/1280 px, then read with LLM vision. The net log also lists Chrome's own background hosts (google.com, update.googleapis.com); count only the page's `127.0.0.1` requests for NFR-001. A UI task whose API is in a sibling branch is checked on a throwaway `git worktree add --detach` + `git cherry-pick -n <code commit>` (merge is blocked by the gate while tasks are In Progress); cherry-pick can mis-detect `TASK_REVIEW_<A>.md` → `TASK_REVIEW_<B>.md` as a rename, so restore that file. Fixture dates (2026-01-15) fall outside "Last 7 days": shift them with `sed` in a temp copy. Screenshots live in local `reports/<task>/` (gitignored). For pages that need clicks (HUN-51 tabs), drive Chrome over CDP with Node 26's built-in `WebSocket` (no package): launch `google-chrome --headless=new --remote-debugging-port=…`, `Page.navigate`, `Runtime.evaluate` to click, `Emulation.setDeviceMetricsOverride` per width, `Page.captureScreenshot`; measure `scrollWidth == clientWidth` for no horizontal scroll; `Network.requestWillBeSent` lists only the page's own requests (no Chrome background noise). The dashboard views are in-page tabs (Usage / Warnings & plan / Kit): no server page paths, no hash routing (fragment = token).
**Files**: `web/src/app.css`, `src/easy_personal/surfaces/api.py`, `tests/fixtures/claude_code/`

### 2026-10-22 — CSS grid tracks need `minmax(0, 1fr)` for scrolling tables
**Learning**: In `web/src/app.css`, grid items default to `min-width: auto`, so a `white-space: nowrap` table widens its `.panels` track past the viewport and `.table-wrap { overflow-x: auto }` never scrolls (HUN-50 P1, seen only in screenshots; jsdom tests cannot catch it). Use `minmax(0, 1fr)` tracks and `section { min-width: 0 }`.
**Files**: `web/src/app.css`

### 2026-10-22 — Open owner check from HUN-53: timeline gap threshold
**Learning**: `metrics.TIMELINE_GAP_MINUTES = 30` (a pause ≥ 30 min between consecutive events of a session is a gap) was the Supervisor's choice in the HUN-53 guide; no PRD number exists. Owner to confirm. `/api/usage/sessions` is dashboard-only (no MCP tool/CLI command, cut list). A message is placed at its earliest row time; one crossing `since` is judged by that time.
**Files**: `src/easy_personal/core/metrics.py`, `src/easy_personal/surfaces/api.py`

### 2026-10-22 — `PROJECT_SPEC.md` Tasks Status column is not maintained
**Learning**: Every row reads `Todo`, including Done tasks; the board (`PROJECT_KANBAN.md` cache of Linear) is the status source of truth. Do not update single rows (it makes the table inconsistent); fix the whole column or drop it, with owner approval.
**Files**: `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`

### 2026-10-22 — `git` kit part: query time, `run_git`, and an open owner check on `--remotes`
**Learning**: HUN-40 reads git at query time (no table, no migration): `sources/git_local.git_part` runs `git log --branches --tags --remotes --format=%ct` through `core/discovery.run_git` (hardened: `-c core.fsmonitor=false`, `GIT_OPTIONAL_LOCKS=0`, timeout; raises `TimeoutExpired` → part `abstained`). `--all` was avoided because it counts `refs/stash`. Owner check (P2): `--remotes` also counts fetched commits by others as "local git activity" (FR-014); dropping it is a one-word change. `commits_by_day` is all-time (not period-bound). HUN-41 can get a project's latest commit with `run_git(project, "log", "-1", "--format=%ct")`.
**Files**: `src/easy_personal/sources/git_local.py`, `src/easy_personal/core/discovery.py`, `src/easy_personal/core/kit.py`

### 2026-10-22 — easy-context facts from the HUN-41 spike
**Learning**: The service needs a root on every call, even `list_indexes` (else "no project root: send the X-Easy-Context-Root header or pass the `root` argument"); easy-personal sends an existing known project's root (≤3 tries on a tool error). `list_indexes` returns `{project_id, root, indexes:[{project_id, root, root_exists, files, slices, last_index_at, indexing, status, ...}]}`, `last_index_at` ISO with ms and offset. In service mode easy-context re-indexes every index every `EASY_CONTEXT_REFRESH_SECONDS` (default 300): all `last_index_at` jump together on that tick — not caused by easy-personal — so `context_index_stale` (index older than latest local commit + 60 s) rarely fires while that timer runs. Index matching is by exact root: a worktree-keyed index is not mapped to its main project. `latest_commit` uses `--remotes` (same owner check as the git part). Tests: `tests/conftest.py` points `EASY_PERSONAL_CONTEXT_URL` at a dead port so no test hits the real service.
**Files**: `src/easy_personal/sources/context.py`, `src/easy_personal/core/store.py`, `src/easy_personal/data/rules.yaml`, `tests/conftest.py`, `tests/fakes/mcp_server.py`

### 2026-10-22 — Run the easy-verifier score on every task, even when it repeats
**Learning**: CLAUDE.md Stage 4 says the advisory easy-verifier `score` runs always. On HUN-41 the Supervisor first marked it N/A "to save time", then ran it: skipping a listed step is not the Supervisor's call even when the result is advisory and repetitive. Read the >budget output with `jq` (overall, per-dimension, failed inputs).
**Files**: `docs/claude-md/pipeline-stages.md`

### 2026-10-22 — `kit_summary` shape after HUN-39/40/41/43
**Learning**: `{"projects": [{project, missing_on_disk, parts: {kernel, git, context}}], "ui": {...}}`. Per-project parts: `kernel` (kit traces), `git` (query-time commits per local day), `context` (easy-context `last_index_at` stored at ingest, `stale` judged at query time). `ui` is overall (easy-ui sessions: sessions/passed/failed/other/actions/first_utc/last_utc), a top-level key, never inside `parts`. Every part: nothing to read → `{"status": "no data"}`. Verifier part (HUN-42) is still missing (blocked by XR-1). HUN-51's kit panel renders this shape.
**Files**: `src/easy_personal/core/kit.py`, `src/easy_personal/sources/ui.py`, `src/easy_personal/sources/context.py`, `src/easy_personal/sources/git_local.py`

### 2026-10-22 — Implementers keep reading their role guide partially
**Learning**: HUN-49 (backend), HUN-41 (backend: "head only") and HUN-51 (frontend: `head -5`) reported partial reads of `agents/<role>.md`, even when the spawn prompt said a partial read counts as not read (HUN-51). Each was closed by a Supervisor conformance check (P1 process finding). Expect it; keep the startup-reads check in code-review, and consider pasting the role guide's key rules into the spawn prompt instead of relying on the read. Update 2026-10-22: it recurred in HUN-42 (lines 1–40) and HUN-56 (lines 56–80 unread, "tool output cut it off") even with "read the WHOLE file" in the prompt — agents read via shell `head`/`sed` or truncated output. Next spawn: tell them to use the Read tool with no offset/limit, or paste the role guide's Pillar/Complexity rules.
**Files**: `agents/backend.md`, `agents/frontend.md`, `skills/craft-spawn-prompt/SKILL.md`

### 2026-10-22 — Connector verify: fresh install, fake `claude`, fake keychain
**Learning**: `uv tool install .` from the same worktree path can reuse a cached build of the same version (0.1.0) and serve old package data (HUN-45 served the old `slack.yaml`): use `uv tool install --reinstall --no-cache .` for a re-verify. Connector verify never touches the owner's config or keychain: a fake `claude` script first on PATH (logs argv, simulates `mcp get`/`add-json`/`remove`) and a throwaway file keyring via `PYTHON_KEYRING_BACKEND`; that fake must raise `keyring.errors.PasswordDeleteError` on deleting a missing item, like real backends. Catalog files use the restricted line format (`key: value`, repeated `field: ENV | label | secret|plain`), no YAML dependency. `tests/leak/` can import `connectors.fake_backend` (tests/ is on pythonpath).
**Files**: `src/easy_personal/connectors/writer.py`, `src/easy_personal/connectors/catalog.py`, `tests/connectors/fake_backend.py`, `tests/leak/test_connector_leak.py`


### 2026-10-22 — Fake `claude mcp get` must be scoped to one connector name
**Learning**: `writer.doctor` checks every catalog entry that `claude mcp get <name>` reports as registered. A test fake that answers "exists" for any name breaks as soon as the catalog grows (HUN-46: 4 new providers → `test_doctor_all_ok` failed with "no keychain item"). The fake in `tests/connectors/conftest.py` now honours `FAKE_CLAUDE_GET_ONLY=<name>`; set it whenever a test fakes a registered connector. Open owner checks from HUN-46: `claude_ai_link` is the general `https://claude.ai/settings/connectors` page, not a per-connector deep link; the Gmail one-time `auth` step and the Teams device-code login are unverified.
**Files**: `tests/connectors/conftest.py`, `tests/connectors/test_writer.py`, `src/easy_personal/catalog/*.yaml`

### 2026-10-22 — Connector surfaces: claude.ai servers, private writer helpers, UI verify with fakes
**Learning**: (1) Official claude.ai connectors appear in transcripts as MCP server `claude_ai_<Provider>` (e.g. `mcp__claude_ai_Gmail__search_threads`); `connectors/suggestions.py` treats such a provider as already available — a naive substring match suggested Gmail to an owner who already uses it (HUN-47 P1). (2) `configured` (keychain item AND `claude mcp get` ok) is implemented twice — `suggestions._configured` (catches Exception) and `surfaces/api._configured` (catches LaunchError, KeyringError) — both through private `writer._keychain_item/_entry_exists`; open P2 follow-up: one public helper in `writer`. (3) Dashboard write flow verify: install isolated, put a fake `claude` (sh script logging argv; `mcp get` exit 1) first on PATH and an in-memory keyring module via `PYTHONPATH` + `PYTHON_KEYRING_BACKEND=fakekr.Keyring`, then drive the form over CDP (`Input.insertText` into the password field); script `scratchpad/ui52/shoot.mjs` pattern. (4) After merging any `web/` change, run `npm --prefix web run build` in the main checkout before `uv run pytest`, else `tests/test_web_toolchain.py` fails with `static build stale` (two tests). (5) A fresh worktree has no `web/node_modules`: `npm --prefix web ci` first. (6) Starlette `TestClient.delete` takes no body: use `client.request("DELETE", url, content=...)`. (7) `DELETE /api/connectors/<id>` is limited to catalog ids (404 otherwise) while CLI `connector remove` accepts any valid name — intentional.
**Files**: `src/easy_personal/connectors/suggestions.py`, `src/easy_personal/surfaces/api.py`, `src/easy_personal/connectors/writer.py`, `tests/test_web_toolchain.py`, `web/src/ConnectorsView.jsx`

### 2026-10-22 — Check the sibling repo's board before calling a task externally blocked
**Learning**: HUN-42 was reported "blocked by XR-1" for a whole day after easy-verifier-mcp T120 was already Done (2026-10-21) — the owner caught it. `board_sync.py pull` never sees sibling boards. Before listing an external blocker, grep the sibling's `PROJECT_KANBAN.md` (e.g. `grep -n T120 ../easy-verifier-mcp/PROJECT_KANBAN.md`). Second gap: a Done sibling task is not live until its runtime is rebuilt — the `easy-verifier` container (image `temvar/easy-verifier-mcp:latest`, built 2026-10-19) predates T120, so no JSON sidecars exist yet; rebuild with `docker compose -f ../easy-verifier-mcp/compose.yaml up -d --build` (owner approval) and then `write_report` produces them. Verifier JSON lands in `~/.easy-verifier-reports/<repo dir name>/` (worktree dirs `<repo>-HUN-xx` are ignored by the source). Config: env `EASY_PERSONAL_VERIFIER_REPORTS`, else `config.yaml` `verifier:` → `reports_dir:`.
**Files**: `src/easy_personal/sources/verifier.py`, `src/easy_personal/core/config.py`, `PROJECT_SPEC.md`

### 2026-10-22 — README is drift-tested against the parser, the MCP server and RUNBOOK
**Learning**: `tests/test_readme.py` (HUN-56) walks every `easy-personal …` line in a README code block through `build_parser()`, checks the launcher subcommands, compares the "MCP tools" list with `create_app().list_tools()`, and requires install/upgrade commands to match `RUNBOOK.md`; it also fails when a top-level subcommand is undocumented (`exec`/`headers` excluded — launcher-only). Any new CLI subcommand, flag shown in docs, or MCP tool must update the README in the same task. The landing site (HUN-57) quotes the README pitch and commands verbatim, so README edits can break site tests too.
**Files**: `README.md`, `tests/test_readme.py`, `RUNBOOK.md`, `src/easy_personal/surfaces/cli.py`, `src/easy_personal/surfaces/mcp.py`
