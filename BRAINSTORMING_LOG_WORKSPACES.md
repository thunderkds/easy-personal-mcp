# BRAINSTORMING_LOG_WORKSPACES.md
**Generated**: 2026-10-26
**Task / Context**: Workspaces + agent accounts. Inputs: `PRD.md` US-014..US-016, FR-028..FR-036, NFR-005 (revised), NFR-009; glossary terms *Workspace*, *Agent account* in `PROJECT_SPEC.md`; decision entry 2026-10-26 in `memory/decisions.md`. Requirement grilling gate: PASS (2026-10-26).
**Skill**: `Skill({ skill: "brainstorming" })` — tier **Deep** (schema migration on the main `events` table; writes into several Claude config folders; ~400× more ingest volume).
**Sibling of**: `BRAINSTORMING_LOG.md` (v1). That log's decisions still hold.

---

## The Problem Space

One person, one machine, several agent accounts (Claude config folders) and several life areas (personal, company, side…). Today every connector is registered once, in the default Claude config, under a bare provider name (`slack`), and usage is read only from `~/.claude/projects`. The feature must:
1. Group connectors into owner-created **workspaces** (`<workspace>-<provider>`), with a built-in `orphan` workspace for connectors whose workspace was deleted and for pre-workspace connectors.
2. Detect **agent accounts** (`~/.claude*`, `~/.codex*`), let the owner confirm/name them, and let each Claude account pick workspaces; keep each account's Claude config in step with its picks — only through `claude mcp add-json|remove --scope user` (DDR-0001).
3. Ingest usage from every confirmed Claude account, tagged by account, and filter usage by workspace (repo paths) and by account.
4. Do this on a store built to best practice (NFR-009) and fast at ~1M events (NFR-005, NFR-009).

### Verified facts (checked 2026-10-26)

| # | Fact | Source |
|---|---|---|
| W1 | Account folders present: `~/.claude` (projects 2.3M), `~/.claude-personal` (550M), `~/.claude-work` (294M), `~/.claude-wife` (112M), `~/.codex`. `~/.claude.json` and `~/.claude.json.backup` are **files** that match `~/.claude*`. | `ls -d ~/.claude* ~/.codex*`, `du -sh */projects` |
| W2 | A non-default account keeps its user-scope MCP config at `<folder>/.claude.json` (e.g. `~/.claude-personal/.claude.json`); the default account uses `~/.claude.json`, **outside** `~/.claude`. So `CLAUDE_CONFIG_DIR` must be **unset** for the default account — setting it to `~/.claude` would target a different file. | `ls -a ~/.claude-personal` |
| W3 | `~/.codex` holds `config.toml` (Codex MCP format differs) — listed only, never written. | `ls ~/.codex` |
| W4 | Store: `PRAGMA user_version` = 0, `foreign_keys` = 0; schema is `CREATE TABLE IF NOT EXISTS` + hand patch `_rebuild_old_verifier_scores` (the HUN-58 drift class). No `busy_timeout`. | `core/store.py:13-135`, `sqlite3 -readonly` |
| W5 | Usage query `_messages` groups **all** events by `message_id`, then filters time in the outer query → the `events_ts` index is not used; full scan every call. | `core/metrics.py:90-104` |
| W6 | `events` has 575 rows, 3 projects (one account). `source_files` key `(machine_id, source, path)` uses absolute paths, so several account folders never collide. | store, `core/store.py:18-25` |
| W7 | Project path = git common-dir parent (worktrees → main repo; subfolders → top level). Workspace repo paths must be compared after the same resolution. | `core/discovery.py:35-46` |
| W8 | Connector name rule `[a-z0-9][a-z0-9_-]{0,63}` — `<workspace>-<provider>` fits if workspace slug ≤ 64 − 1 − longest provider id. | `launch.py:16` |
| W9 | Writer refuses to overwrite an existing entry or keychain item, rolls back keychain if `claude mcp add-json` fails; keychain service `easy-personal/<name>`. | `connectors/writer.py:62-110` |
| W10 | Ingest is single-writer via `flock` (`core/lock.py`); dashboard and MCP read concurrently (WAL). | `core/lock.py` |

---

## Questions for the User

| # | Question | Recommended answer |
|---|---|---|
| Q1 | Can a workspace's name change after creation? | **Resolved 2026-10-26: yes.** Owner proposed id + alias: connector and workspace have fixed ids; the Claude name is an alias (default `<workspace>-<provider>`, owner-editable). Renames/moves swap Claude entries only; keychain never moves. Legacy connectors keep their name as id. |
| Q2 | ASCII mockup of the Workspaces / Agents pages before Stage 2? | **Resolved 2026-10-26: owner approved the sketch below ("ok").** |

---

## Alternative Paths

### Option A — Desired state + reconcile (recommended)

The store holds the **desired state**: workspaces, connectors (name, provider, workspace), account → workspace picks. One function `plan(account)` compares desired connectors for that account with what `claude mcp get/list` reports in that account's config, and returns the add/remove list; `apply(plan)` runs it through the existing writer. Every user action (pick a workspace, move a connector, delete a workspace, migrate old connectors) = one DB transaction + `plan`/`apply` for the affected accounts. The confirmation screen shows the plan.

- **Pros**: one write path for all operations; idempotent — a crash half-way is fixed by running it again; `doctor` becomes "show the plan for each account"; the plan doubles as the confirmation text.
- **Cons**: must parse `claude mcp list` output per account (one subprocess each); must only ever touch entries it owns (names that are connectors in the store), never other MCP servers.

### Option B — Imperative operations

Each operation calls the CLI directly in its own sequence (move = add new in each account, remove old, move keychain), each with its own rollback, like `writer.add` today.

- **Pros**: closest to today's code; no listing/parsing of account configs.
- **Cons**: 5 operations × N accounts × partial-failure paths; a crash leaves drift that nothing detects or fixes; each operation needs its own rollback tests.

### Option C — Files as the source of truth

Workspaces as `~/.easy-personal/workspaces/<name>.yaml`; picks inside each file; no new tables.

- **Pros**: hand-editable; no migration for workspace data.
- **Cons**: no constraints (one provider per workspace, unique names) — only code checks; the usage filter and account tagging need the DB anyway (NFR-009, FR-035/036), so two stores; concurrent dashboard + CLI edits race.

---

## Adversarial Review — why each might fail

| Option | Failure mode | Mitigation |
|---|---|---|
| A | Reconcile removes an MCP entry the owner added by hand with a matching name. | Only names recorded in the store's `connectors` table (plus the legacy catalog ids during FR-034) are ever managed; anything else is reported, not touched. |
| A | `claude mcp list` output format changes. | Use `claude mcp get <name>` per managed name (already parsed by `_safe_lines`); exit code decides presence. |
| B | Move fails after adding the new name in account 1 of 3 → duplicates, no detection. | (none built in — the reason to prefer A) |
| C | Two writers (dashboard + CLI) overwrite each other's YAML. | File lock — rebuilding what SQLite gives for free. |
| All | Ingest of ~960 MB holds the writer lock for minutes; the systemd timer run overlaps. | Commit per file batch; `flock` already serialises; lock wait 300 s → log and skip, next run continues from offsets. |
| All | Default account written with `CLAUDE_CONFIG_DIR=~/.claude` → wrong file (W2). | Account row stores `config_dir_env` = NULL for the default account; test asserts env for both kinds. |

---

## 50% Rule Check

- **Recommended path, ~50% less code**: A replaces five bespoke write sequences with one `plan`/`apply`; move, delete-workspace, pick/unpick and the legacy migration become DB updates + reconcile. With id + alias (Q1), there is no keychain move at all: keychain and launcher are keyed by the fixed id; renames/moves only swap Claude entries.
- **Usage at scale without a new rollup table**: push the time filter inside `_messages` (filter `events` by `ts_utc` before `GROUP BY message_id`) and add one composite index; a pre-aggregated `messages` table is **cut** unless the 1M-event benchmark fails.
- **Cut list (deliberately not built)**: several connectors of one provider per workspace (ids make it possible later); per-account usage charts beyond a filter; new providers; MCP tools for workspaces; Codex writes; `messages` rollup table (only if benchmark fails).

---

## Recommended Path

**Option A — desired state in SQLite + per-account reconcile through the existing writer**, with:
- **Store (NFR-009)**: migration runner on `PRAGMA user_version` — ordered `migrations/NNNN_*.sql|py`, each in one transaction, v0 = today's schema (the existing hand patch becomes migration 0001); `foreign_keys=ON`, `busy_timeout=5000` in `connect`. New tables: `workspaces(id, name UNIQUE, color, description, email, is_orphan)` with `orphan` seeded; `workspace_repos(workspace_id FK, repo_path, PK both)`; `agent_accounts(id, provider, folder UNIQUE, label, confirmed, config_dir_env)`; `account_workspaces(account_id FK, workspace_id FK, PK both)` + trigger/CHECK refusing orphan; `connectors(id TEXT PK, alias, alias_custom, provider, workspace_id FK)` + partial unique indexes `(workspace_id, provider)` and `(alias)` `WHERE workspace_id <> orphan`. `events.agent_account_id INTEGER REFERENCES agent_accounts(id)` added by `ALTER TABLE`, backfilled to the default account.
- **Performance**: time filter before grouping; indexes `events(ts_utc, message_id)` and `events(agent_account_id, ts_utc)`; workspace filter resolves repo paths (W7) to the **distinct** stored `project_path` values first, then `project_path IN (…)` — never `LIKE` on the big table; `EXPLAIN QUERY PLAN` assertions in tests; a seeded 1M-event benchmark test for the < 500 ms target; ingest commits per file and streams (NFR-005 < 2 min cold for ~960 MB).
- **Secrets**: never in the store (NFR-002) — only names; keychain stays the only secret holder.

---

## Surgical Scope

| Should touch | Must not touch |
|---|---|
| `core/store.py` (+ new `core/migrations/`), `core/metrics.py` (`_messages`, filters), `core/config.py` (account folders), `sources/claude_code.py` (per-account roots, tag events) | `launch.py` beyond what naming needs (NFR-006: stdlib + `keyring` only, < 200 ms) |
| `connectors/writer.py` (env per account, plan/apply), new `connectors/workspaces.py` / `accounts.py` | Any `~/.claude*/projects` file (NFR-003); `~/.codex/*` (listed only) |
| `surfaces/cli.py`, `surfaces/api.py` (guarded writes, NFR-008), `tests/leak/harness.py` (new surfaces) | MCP write tools (FR-021 stays read-only) |
| `web/src/` new Workspaces + Agents views, usage filters | Project-scope Claude files (FR-019) |

---

## Edge Case Checklist for TASK_GUIDE

- [ ] `~/.claude.json`, `~/.claude.json.backup` and other non-directories are never detected as accounts; a folder counts only if it has `projects/` or `settings.json` (Claude) / `config.toml` (Codex).
- [ ] Default account → `claude` run **without** `CLAUDE_CONFIG_DIR`; others → with it (W2).
- [ ] An account folder that disappears → account shown *missing*; its events kept; no writes attempted.
- [ ] Alias ≤ 64 chars and matching `launch._NAME` (W8); default alias length checked on workspace create/rename; `orphan` reserved.
- [ ] Move to a workspace that already has the provider → refused before any write; orphan exempt.
- [ ] Deleting a workspace with picks → connectors to orphan **and** removed from every account that picked it, in one plan.
- [ ] Reconcile never touches MCP entries whose names are not managed connectors.
- [ ] Alias change / move / workspace rename: add new alias → remove old alias, same `exec <id>`; keychain untouched; a crash between steps is repaired by the next reconcile.
- [ ] Tool-usage counters key on the Claude tool name (`mcp__<alias>__…`): an alias change splits history — documented, not merged.
- [ ] Legacy connector id = old name (`slack`); new ids random, matching `launch._NAME`.
- [ ] Same `message_id` found in two accounts (copied transcript) counts once.
- [ ] Workspace repo path given as a subfolder or worktree resolves to the stored project path (W7); trailing slash / `~` normalised.
- [ ] Migration from a real v0 store (copy of the owner's schema, fixture data) keeps every row; downgrade path documented (`migration-safety`).
- [ ] Ingest of a large account is resumable: kill mid-run, re-run continues from `source_files` offsets.
- [ ] No secret value, token or email body in any new table, API response or log (leak suite `MCP_CALLS`/`CLI_CALLS`/`API_CALLS` registered).

---

## Next Actions

1. Owner answers Q2 (mockup) and selects the path.
2. Stage 2: `to-issues` → tracer-bullet slices, likely: store migrations + perf (C2/Med, `migration-safety`) → accounts detect + multi-account ingest → workspaces + connectors model + reconcile (C3/High) → CLI → dashboard Workspaces/Agents + usage filters (UI Evidence).
3. Stage 2 grill (terminology mode) on *connector* vs *workspace connector*, *account* vs *agent account*.

---

## User Selection

**Selected Path**: Option A — desired state + per-account reconcile, with connector id + alias (Q1).
**Owner notes**: 2026-10-26 — no objection to Option A; "ok, go ahead with planning" after the sketch. Owner asked for DB best practice and performance throughout; raise for review when needed.

### Approved sketch (2026-10-26)

```
Workspaces page: cards per workspace (color, email, repo count, connectors by alias, "Used by" accounts, [Open]);
orphan card (not active, [Assign…]); [+ Add workspace].
Workspace form: name, color, email, description, repos (+/✕), connectors (alias field, [Move to…], [Delete]),
[Delete] workspace; Save shows the plan "Will change: <account> → add/remove <alias> … [Confirm]".
Agents page: toggle By account / By provider; row per account: label, folder, workspaces multi-select, confirmed ✓;
Codex "listed only"; new folder found → [Confirm & name] [Ignore].
Usage page: Workspace ▾ and Account ▾ filters.
```
