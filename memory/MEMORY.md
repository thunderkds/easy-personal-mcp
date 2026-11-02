# MEMORY.md — Hot-Tier Memory Index

> **Rules**: Supervisor-only writes. Max 42,000 characters — a ratchet: `/compact-memory` may lower
> it, never raise it to fit growth. One-line summaries + links to cold files.
> Each spawn prompt carries a slice of these lines (craft-spawn-prompt's memory_slice.py) and this
> path to read in full only when the slice falls short; the whole file is not pasted.
> Updated by the Supervisor — prompted by the PostToolUse hook on `git push` / `git merge` (diff-driven pass), or via the `/compact-memory` skill.

---

## Memory Architecture

- [decisions.md](decisions.md) — code + infra architectural decisions (the "why")
- [glossary.md](glossary.md) — canonical biz domain terms and core domain models
- [learnings.md](learnings.md) — specs/requirement clarifications, patterns, gotchas

---

## Index

<!-- Format: - [Title](cold-file.md#section) — one-line summary.
     Target ≤150 chars/entry. Advisory: reported by the size test, never enforced.
     The enforced gate is the 42,000-character whole-file budget above. -->

### Decisions
- [easy-memory layer = Option A](decisions.md#2026-11-02--easy-memory-layer-in-the-memory-view--one-graph-response-option-a) — `~/.easy-memory` read as files (no MCP), metadata only; top-30 cited-file nodes + stale mark + 30-day panel in `/api/memory-graph`; HUN-184 → HUN-185.
- [v0.4.0 live attention = Option C](decisions.md#2026-11-02--v040-live-attention-states--snapshot-pushed-state-option-c) — failing / waiting for you / needs attention worked out in the live tail; snapshot re-sent only on a state change; no new dep, nothing stored.
- [v0.3.0 live view = transcript tail only](decisions.md#2026-10-31--v030-live-view--transcript-tail-only-option-a) — Option A: tail in `easy-personal dashboard` → store → `GET /api/live` SSE → Live view; no hooks/OTel, no Claude config writes; HUN-175..179.
- [Live trace spike in-repo, metadata only](decisions.md#2026-10-31--live-trace-spike-runs-inside-easy-personal-mcp-metadata-only) — HUN-165..170 in `experiments/live-trace/`; plan items are hypotheses; E5 docs-only (no Admin key); prototype waits for owner review.
- [Work view, read-only (Javis OS ideas)](decisions.md#2026-10-31--work-view-inspired-by-javis-os-read-only) — board from PROJECT_KANBAN.md, agent = sidechain or `sdk*`, timers+cron first word, memory graph names+links live; merged HUN-160..164 2026-10-31.
- [Local merge to main, push in batches](decisions.md#2026-10-21--stage-35-git-flow-local-merge-to-main-push-in-batches) — no per-task PR; board goes In Progress → Done (no review status in Linear).
- [v1 = Python core + React/Vite page, on demand](decisions.md#2026-10-21--v1--python-core--reactvite-page-on-demand) — Option B; page on 127.0.0.1 only while `easy-personal dashboard` runs; systemd timer ingests.
- [Read-only MCP client of easy-context](decisions.md#2026-10-21--read-only-mcp-client-of-easy-context) — only list_indexes/get_stats on 127.0.0.1:8767; never index_path; DDR-0002; built in HUN-41 (allow-list + host guard, `context_index` table).
- [Native runtime; writes via `claude mcp` user scope](decisions.md#2026-10-21--native-runtime-connector-writes-only-via-claude-mcp-user-scope) — keychain + Claude Code only; no Codex/Cursor; DDR-0001.
- [Board scoped to Linear project P-HUN-3](decisions.md#2026-10-21--board-scoped-to-linear-project-easy-personal-mcp-p-hun-3) — empty `project` in board.json pulls the whole team.

- [Connectors register `easy-personal-launch exec`](decisions.md#2026-10-22--connectors-register-easy-personal-launch-exec-name) — not `easy-personal exec` (~300 ms import vs 200 ms NFR-006); owner 2026-10-22.
- [Catalog servers pinned; Slack = slack-mcp-server@1.3.0](decisions.md#2026-10-22--catalog-servers-are-pinned-slack--slack-mcp-server130) — parse refuses unpinned npx/uvx/pipx-run; old Slack package deprecated.
- [HUN-46 servers: Gmail, SendGrid, Teams, Discord](decisions.md#2026-10-22--hun-46-catalog-servers-gmail-sendgrid-teams-discord) — pinned community servers; `claude_ai_link` required with yes.
- [Workspaces; agent account picks workspace](decisions.md#2026-10-26--workspaces-group-connectors-the-agent-account-picks-its-workspace) — workspace = email/repos/connectors, no agent settings; Codex listed only (DDR-0001 stands); not yet planned.
### Learnings
- [Live attention HUN-181](learnings.md#2026-11-02--live-attention-hun-181-contracts-resume-re-snapshot-and-three-process-gotchas) — `attention` item shape for the page; resume now sends 1 fresh snapshot; Linear has no review state → push `done`, pull, merge; built-in `security-review` needs `origin/HEAD`; dashboard URL prints before listen.
- [Live badge HUN-182 + easy-memory HUN-184](learnings.md#2026-11-02--live-badge-hun-182--easy-memory-reader-hun-184-two-p1s-and-a-stale-build-test-failure) — cited paths merge into kit nodes only (Claude paths are account-relative); store key/meta facts, owner timing 204/166 ms; pill keys need index; rebuild web before post-merge suite.
- [Dashboard UX HUN-186..188](learnings.md#2026-11-02--dashboard-ux-pass-hun-186188-gone-projects-speed-numbers-and-four-process-gotchas) — 133 of 160 projects are gone folders (hidden from Board/Kit/Memory, still counted in usage); kit 81 s → 0.65 s, sessions 7.6 MB → 0.4 MB; `pull` reverts guide edits; Linear lags ~1 min after `push done` (gate pull too); gate blocks `merge-tree` dry-runs.
- [easy-memory ring HUN-185 + owner project HUN-189](learnings.md#2026-11-02--easy-memory-ring-hun-185--owner-project-for-gone-worktrees-hun-189) — ring 4 widens canvas to 800, panels under graph; gone worktrees counted under owner repo at read time (parent repo → branch ref/merge subject incl. `Merge <b>: <text>` → hidden-clone tie-break), keys 157 → 123; dashboard headless via `BROWSER=true`.
- [Usage top 10 HUN-190](learnings.md#2026-11-02--usage-lists-top-10-hun-190--a-flaky-full-suite-test) — `useTopN` in Widgets.jsx (10 rows + "Show N more", Kit button style); day table never cut; one flaky full-suite test passed alone — use `pytest -rf` to keep its name.
- [v0.3.0 release: migrations since last tag](learnings.md#2026-11-01--v030-release-prep-check-migrations-since-the-last-tag-not-since-the-planning-brief) — 0.3.0 = store 6→7 (m0007, HUN-161), `.bak-v6`; RUNBOOK v0.3.0 entry (rollback = `1550317`, never tag v0.2.0); always `git log <prev tag>..main -- core/migrations/`.
- [Live view built ahead of backend](learnings.md#2026-11-01--live-view-hun-178-built-before-its-backend-restart-and-history-semantics) — snapshot `last_id` drop = restart → clear feed; Supervisor merges main into a task branch before it goes In Progress; `scratchpad/hun178` UI verify scripts.
- [`/api/live` + verify gotcha](learnings.md#2026-11-01--apilive-hun-177-and-a-verify-gotcha-hand-made-assistant-lines-abstain) — query token only on `/api/live`, access log off; resume rules; hand-made assistant lines need all 4 usage ints or they abstain (rows still stored).
- [Live tail buffer + delta usage events](learnings.md#2026-11-01--live-tail-hun-176-buffer-api-and-usage-events-are-deltas) — `live.current()`; `events_since`/`rolled_past`/`snapshot()`; later growth of a message = delta `usage` event; totals since dashboard start only.
- [Live trace spike results](learnings.md#2026-10-31--live-trace-spike-hun-165169-what-each-claude-code-source-really-does-21316-haiku) — transcripts p95 1.35 s + 8 new record types abstained; http hooks ignore `async` (slow collector blocks); OTel exact tokens/cost but carries user.email; stream-json use `result_index` 0.
- [Work view parallel build](learnings.md#2026-10-31--work-view-hun-160164-parallel-worktree-hazards-new-route-and-new-view-checklists) — `git stash` is shared across worktrees (forbid it); shared scratchpad/active_task; new route → leak `API_CALLS`; new view → bump `site/index.html` views stat; real entrypoints cli/sdk-cli/claude-desktop; CDP `shoot.mjs` evidence; never `pkill -f "easy-personal dashboard"`.
- [easy-ui + Graphite Mint](learnings.md#2026-10-29--easy-ui-mcp-is-reachable-graphite-mint-look-landed-hun-126) — easy-ui at localhost:8765 via curl JSON-RPC, fixed 1280×720 viewport, batch checks (90-call limit); Plex self-hosted; family links allowlisted in check_static; `[auto]` false alarm on later tasks.
- [Nine-branch integration](learnings.md#2026-10-29--nine-branch-dashboard-integration-hun-113121-auto-verify-stacked-merges) — headless verify rows read blocked (no verdict line); Done before merge; cherry-picked stacks auto-merge duplicates; regen goldens; `uv tool install --reinstall`; `test_launch` timing flakes under load.
- [Static build gitignored](learnings.md#2026-10-21--dashboard-static-build-is-gitignored-build-before-uv-build) — `npm --prefix web run build` before `uv build`; 2 web tests skip unbuilt.
- [Verify against fixtures](learnings.md#2026-10-21--runtime-verify-against-fixtures-never-the-real-transcripts) — temp `EASY_PERSONAL_CLAUDE_PROJECTS` + `EASY_PERSONAL_HOME`; grep outputs for `INVENTED_`.
- [FastMCP typed dict output](learnings.md#2026-10-21--fastmcp-returns-structured-output-only-for-typed-dicts) — `-> dict[str, Any]`; version via private `_mcp_server.version`.
- [Merge gate blocks while any task In Progress](learnings.md#2026-10-21--merge-gate-blocks-while-any-task-is-in-progress) — Done after evidence, then merge; parallel tasks finish together; regex also blocks `git merge-base`; push board status and merge in separate calls.
- [Skills the Supervisor cannot run](learnings.md#2026-10-21--skills-the-supervisor-cannot-run-itself) — /verify loadable via Skill since 2026-10-22; security-review inline; no Skill tool in sub-agents.
- [easy-verifier registered (local scope)](learnings.md#2026-10-21--easy-verifier-registered-for-this-repo-local-scope) — `~/.claude-personal` profile only; 127.0.0.1:8766; absolute `repo` arg; Stage 4 step 4b, advisory, no hook; 2026-11-01 refused every repo path → N/A.
- [easy-verifier score output](learnings.md#2026-10-21--easy-verifier-score-output-read-with-jq-known-security-false-positives) — >budget, read via jq; security 18 = SQL/lockfile false positives; `write_report` for path.
- [easy-context registered (local scope)](learnings.md#2026-10-21--easy-context-registered-for-this-repo-local-scope) — indexed; search via MCP, not Read; re-index after merge.
- [Ignore cost-state.totalCostUSD](learnings.md#2026-10-21--ignore-claude-codes-cost-statetotalcostusd) — cost only from the labelled price table.
- [Register hook catches other repos' guides](learnings.md#2026-10-21--the-register-hook-catches-guides-written-in-other-repos) — re-pull the board before committing after a cross-repo guide write.
- [Board pull overwrites `## Blocked`](learnings.md#2026-10-21--board_syncpy-pull-overwrites-the--blocked-table) — keep external blockers in the guide + PROJECT_SPEC Tasks.
- [Git remote is `github`](learnings.md#2026-10-21--git-remote-is-github-not-origin) — `git push -u github <branch>`.
- [Linear escapes backticks in titles](learnings.md#2026-10-21--linear-escapes-backticks-in-issue-titles) — cosmetic in KANBAN cache.
- [Register new surfaces in the leak suite](learnings.md#2026-10-21--new-mcp-tool-or-cli-command-must-be-registered-in-the-leak-suite) — `MCP_CALLS`/`CLI_CALLS` in `tests/leak/harness.py`; flag variants by hand; also `API_CALLS`, `CLI_KEYCHAIN_COMMANDS`.
- [Owner checks: prices + 200k context](learnings.md#2026-10-21--open-owner-checks-from-hun-35-price-numbers-and-the-long-context-threshold) — `prices.yaml` numbers unverified; outlier threshold is implementer's choice.
- [Board push needs LINEAR_API_KEY](learnings.md#2026-10-21--board-push-needs-linear_api_key-in-the-sessions-shell) — else only the KANBAN cache moves; nothing queued.
- [Store migrations + real-store guard (HUN-61)](learnings.md#2026-10-26--store-migrations-on-user_version-tests-must-never-open-the-real-store-hun-61) — `core/migrations/mNNNN_*`, `.bak-v<N>` rollback; numbers m0003–m0006 pre-assigned; tests isolated from `~/.easy-personal`.
- [Accounts + workspaces landed; migration merge order](learnings.md#2026-10-26--agent-accounts--workspaces-landed-hun-62-hun-64-merge-order-of-numbered-migrations) — detect via `EASY_PERSONAL_AGENT_ROOT`; default account unignorable; `metrics._scope`; merge mNNNN branches in number order.
- [Multi-account ingest + connector sync (HUN-63/65)](learnings.md#2026-10-26--multi-account-ingest--connector-sync-landed-hun-63-hun-65-parallel-branch-gotchas) — NULL account = default; `sync.plan/apply` owns only registered aliases; fakes for verify; merge gate scans whole command.
- [Move/rename/delete, legacy, Agents page, filters (HUN-66/67/68/70)](learnings.md#2026-10-26--moverenamedelete-legacy-adoption-agents-page-usage-filters-landed-hun-66676870) — `sync._change` dry-run/commit; legacy removes only launcher entries; `font: inherit`; demo.sh + shoot.mjs UI evidence.
- [No codebase-map yet](learnings.md#2026-10-21--memorycodebase-mapmd-does-not-exist) — run `/map-codebase` or drop it from C2 spawn reads.
- [Owner check: compaction inferred](learnings.md#2026-10-21--open-owner-check-from-hun-37-compaction-is-inferred) — context drop < half of peak = compacted; rules abstain with reason.
- [Scheduled ingest lock + systemd](learnings.md#2026-10-21--scheduled-ingest-lock-exit-75-systemd-only-via---dry-run-in-tests) — flock, exit 75; tests stub systemctl, leak suite dry-run only; timer has no shell env → settings go in `config.yaml`.
- [No `rm` in `bash -c`](learnings.md#2026-10-21--rm-inside-bash--c-is-blocked-by-the-harness-safety-check) — use a fresh temp dir per verify case.
- [Verify dashboard/kit_trace/launcher](learnings.md#2026-10-22--verify-recipes-for-the-dashboard-kit_trace-and-the-launcher) — `X-Dashboard-Token`; rewrite fixture `cwd`; throwaway `PYTHON_KEYRING_BACKEND`, never the real keychain.
- [UI evidence via headless Chrome](learnings.md#2026-10-22--ui-evidence-without-easy-ui-headless-chrome-on-a-combined-checkout) — (2026-10-22; easy-ui now reachable, see below) screenshots + net log at 375/768/1280; throwaway cherry-pick checkout for cross-branch UI; CDP via Node `WebSocket` for tab clicks.
- [Grid tracks `minmax(0, 1fr)`](learnings.md#2026-10-22--css-grid-tracks-need-minmax0-1fr-for-scrolling-tables) — nowrap tables overflow `.panels` otherwise; jsdom can't see it.
- [Owner check: 30-min timeline gap](learnings.md#2026-10-22--open-owner-check-from-hun-53-timeline-gap-threshold) — `TIMELINE_GAP_MINUTES` is the Supervisor's choice; sessions API dashboard-only.
- [PROJECT_SPEC Status column stale](learnings.md#2026-10-22--project_specmd-tasks-status-column-is-not-maintained) — all `Todo`; board is the truth; don't patch single rows.
- [git kit part + `--remotes` owner check](learnings.md#2026-10-22--git-kit-part-query-time-run_git-and-an-open-owner-check-on---remotes) — query time via hardened `run_git`; `--remotes` may count others' commits; all-time counts.
- [easy-context spike facts](learnings.md#2026-10-22--easy-context-facts-from-the-hun-41-spike) — root header always needed; 300 s auto-refresh makes stale rare; exact-root matching; tests use a dead port.
- [Always run easy-verifier score](learnings.md#2026-10-22--run-the-easy-verifier-score-on-every-task-even-when-it-repeats) — advisory but listed; never skip to save time.
- [kit_summary shape](learnings.md#2026-10-22--kit_summary-shape-after-hun-39404143) — parts kernel/git/context/verifier per project + top-level `ui`.
- [Partial role-guide reads recur](learnings.md#2026-10-22--implementers-keep-reading-their-role-guide-partially) — HUN-49/41/51/42/56; conformance check; next: Read tool without limit or paste key rules.
- [Connector verify recipe](learnings.md#2026-10-22--connector-verify-fresh-install-fake-claude-fake-keychain) — `--reinstall --no-cache`; fake `claude` + throwaway keyring; never the owner's config.
- [Scope fake `claude mcp get` to one name](learnings.md#2026-10-22--fake-claude-mcp-get-must-be-scoped-to-one-connector-name) — `FAKE_CLAUDE_GET_ONLY`; open owner check: generic claude.ai link, Gmail/Teams login unverified.
- [Connector surfaces gotchas](learnings.md#2026-10-22--connector-surfaces-claudeai-servers-private-writer-helpers-ui-verify-with-fakes) — `claude_ai_*` MCP servers = already available; duplicated `_configured` (P2); rebuild web before pytest after merge; UI verify with fake claude + keyring.
- [Check sibling boards before 'blocked'](learnings.md#2026-10-22--check-the-sibling-repos-board-before-calling-a-task-externally-blocked) — T120 was Done unseen; Done ≠ live until the verifier container is rebuilt.
- [README drift test](learnings.md#2026-10-22--readme-is-drift-tested-against-the-parser-the-mcp-server-and-runbook) — new subcommand/flag/MCP tool must update README; site quotes README verbatim.
- [Landing site facts](learnings.md#2026-10-22--landing-site-facts-hun-57) — Vercel from main (owner connects); site tests read README/catalog/tools; "Read tool, no limit" fixed partial reads.
- [Shipped table shape needs a rebuild step](learnings.md#2026-10-22--a-shipped-tables-shape-never-changes-in-place-without-a-rebuild-step-hun-58) — `CREATE TABLE IF NOT EXISTS` never migrates; HUN-58 drop-on-connect for the verifier cache; sub-agents lack `Skill`; `security-review` needs `origin/HEAD`.
- [Workspaces page + e2e QA (HUN-69/71)](learnings.md#2026-10-26--workspaces-page-and-e2e-qa-landed-hun-69-hun-71-env-less-claude-calls-session-restart-gotchas) — env-less `claude` calls hit the wrong account; API plans end with an abort line; scratchpad lost on restart; `LINEAR_API_KEY` from launching terminal.
- [Remove follows ownership rule (HUN-72)](learnings.md#2026-10-26--pre-workspace-connector-remove-follows-the-ownership-rule-hun-72) — non-catalog name refused; foreign entry refused; doctor skips adopted ids; e2e guard skips ~/.codex* mtime; open P3 tool_signals double count.
- [Release prep needs its own task (HUN-73)](learnings.md#2026-10-26--release-prep-needs-its-own-task-hun-73) — version pinned by test_readme; v0.2.0 tags ac64aae, rollback to 30e21a3; real store v2/575 events.
- [Dark sidebar shell + Usage widgets (HUN-83)](learnings.md#2026-10-29--dark-sidebar-shell--usage-widgets-landed-hun-83-verify-and-report-gotchas) — dark-only tokens; filters portal to top bar; previous-period breakdown call; shift fixtures to 2 weeks; check agent-listed evidence paths exist.
- [Neon mint pass + family landing (HUN-131/132)](learnings.md#2026-10-29--neon-mint-pass-merged-hun-131-landing-in-family-layout-hun-132-merge-and-verify-gotchas) — accent #3cf5a8 + 4 glow tokens (off under prefers-contrast); diff canvas versions; untracked board guide blocks merge; verify rm -rf blocked; use 7-day priced fixture for site shots; rebuild web before full suite after merge; push gate blocks while a task is In Progress.
