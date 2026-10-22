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
- [Local merge to main, push in batches](decisions.md#2026-10-21--stage-35-git-flow-local-merge-to-main-push-in-batches) — no per-task PR; board goes In Progress → Done (no review status in Linear).
- [v1 = Python core + React/Vite page, on demand](decisions.md#2026-10-21--v1--python-core--reactvite-page-on-demand) — Option B; page on 127.0.0.1 only while `easy-personal dashboard` runs; systemd timer ingests.
- [Read-only MCP client of easy-context](decisions.md#2026-10-21--read-only-mcp-client-of-easy-context) — only list_indexes/get_stats on 127.0.0.1:8767; never index_path; DDR-0002; built in HUN-41 (allow-list + host guard, `context_index` table).
- [Native runtime; writes via `claude mcp` user scope](decisions.md#2026-10-21--native-runtime-connector-writes-only-via-claude-mcp-user-scope) — keychain + Claude Code only; no Codex/Cursor; DDR-0001.
- [Board scoped to Linear project P-HUN-3](decisions.md#2026-10-21--board-scoped-to-linear-project-easy-personal-mcp-p-hun-3) — empty `project` in board.json pulls the whole team.

- [Connectors register `easy-personal-launch exec`](decisions.md#2026-10-22--connectors-register-easy-personal-launch-exec-name) — not `easy-personal exec` (~300 ms import vs 200 ms NFR-006); owner 2026-10-22.
- [Catalog servers pinned; Slack = slack-mcp-server@1.3.0](decisions.md#2026-10-22--catalog-servers-are-pinned-slack--slack-mcp-server130) — parse refuses unpinned npx/uvx/pipx-run; old Slack package deprecated.
- [HUN-46 servers: Gmail, SendGrid, Teams, Discord](decisions.md#2026-10-22--hun-46-catalog-servers-gmail-sendgrid-teams-discord) — pinned community servers; `claude_ai_link` required with yes.
### Learnings
- [Static build gitignored](learnings.md#2026-10-21--dashboard-static-build-is-gitignored-build-before-uv-build) — `npm --prefix web run build` before `uv build`; 2 web tests skip unbuilt.
- [Verify against fixtures](learnings.md#2026-10-21--runtime-verify-against-fixtures-never-the-real-transcripts) — temp `EASY_PERSONAL_CLAUDE_PROJECTS` + `EASY_PERSONAL_HOME`; grep outputs for `INVENTED_`.
- [FastMCP typed dict output](learnings.md#2026-10-21--fastmcp-returns-structured-output-only-for-typed-dicts) — `-> dict[str, Any]`; version via private `_mcp_server.version`.
- [Merge gate blocks while any task In Progress](learnings.md#2026-10-21--merge-gate-blocks-while-any-task-is-in-progress) — Done after evidence, then merge; parallel tasks finish together; regex also blocks `git merge-base`; push board status and merge in separate calls.
- [Skills the Supervisor cannot run](learnings.md#2026-10-21--skills-the-supervisor-cannot-run-itself) — /verify loadable via Skill since 2026-10-22; security-review inline; no Skill tool in sub-agents.
- [easy-verifier registered (local scope)](learnings.md#2026-10-21--easy-verifier-registered-for-this-repo-local-scope) — `~/.claude-personal` profile only; 127.0.0.1:8766; absolute `repo` arg; Stage 4 step 4b, advisory, no hook.
- [easy-verifier score output](learnings.md#2026-10-21--easy-verifier-score-output-read-with-jq-known-security-false-positives) — >budget, read via jq; security 18 = SQL/lockfile false positives; `write_report` for path.
- [easy-context registered (local scope)](learnings.md#2026-10-21--easy-context-registered-for-this-repo-local-scope) — indexed; search via MCP, not Read; re-index after merge.
- [Ignore cost-state.totalCostUSD](learnings.md#2026-10-21--ignore-claude-codes-cost-statetotalcostusd) — cost only from the labelled price table.
- [Board pull overwrites `## Blocked`](learnings.md#2026-10-21--board_syncpy-pull-overwrites-the--blocked-table) — keep external blockers in the guide + PROJECT_SPEC Tasks.
- [Git remote is `github`](learnings.md#2026-10-21--git-remote-is-github-not-origin) — `git push -u github <branch>`.
- [Linear escapes backticks in titles](learnings.md#2026-10-21--linear-escapes-backticks-in-issue-titles) — cosmetic in KANBAN cache.
- [Register new surfaces in the leak suite](learnings.md#2026-10-21--new-mcp-tool-or-cli-command-must-be-registered-in-the-leak-suite) — `MCP_CALLS`/`CLI_CALLS` in `tests/leak/harness.py`; flag variants by hand; also `API_CALLS`, `CLI_KEYCHAIN_COMMANDS`.
- [Owner checks: prices + 200k context](learnings.md#2026-10-21--open-owner-checks-from-hun-35-price-numbers-and-the-long-context-threshold) — `prices.yaml` numbers unverified; outlier threshold is implementer's choice.
- [Board push needs LINEAR_API_KEY](learnings.md#2026-10-21--board-push-needs-linear_api_key-in-the-sessions-shell) — else only the KANBAN cache moves; nothing queued.
- [No codebase-map yet](learnings.md#2026-10-21--memorycodebase-mapmd-does-not-exist) — run `/map-codebase` or drop it from C2 spawn reads.
- [Owner check: compaction inferred](learnings.md#2026-10-21--open-owner-check-from-hun-37-compaction-is-inferred) — context drop < half of peak = compacted; rules abstain with reason.
- [Scheduled ingest lock + systemd](learnings.md#2026-10-21--scheduled-ingest-lock-exit-75-systemd-only-via---dry-run-in-tests) — flock, exit 75; tests stub systemctl, leak suite dry-run only; timer has no shell env → settings go in `config.yaml`.
- [No `rm` in `bash -c`](learnings.md#2026-10-21--rm-inside-bash--c-is-blocked-by-the-harness-safety-check) — use a fresh temp dir per verify case.
- [Verify dashboard/kit_trace/launcher](learnings.md#2026-10-22--verify-recipes-for-the-dashboard-kit_trace-and-the-launcher) — `X-Dashboard-Token`; rewrite fixture `cwd`; throwaway `PYTHON_KEYRING_BACKEND`, never the real keychain.
- [UI evidence via headless Chrome](learnings.md#2026-10-22--ui-evidence-without-easy-ui-headless-chrome-on-a-combined-checkout) — no easy-ui MCP here; screenshots + net log at 375/768/1280; throwaway cherry-pick checkout for cross-branch UI; CDP via Node `WebSocket` for tab clicks.
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
