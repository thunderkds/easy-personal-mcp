# TASK_GUIDE — T132: Agents — create a new account, remove an account
**Date**: 2026-11-20
**Complexity Level**: C2
**Risk Level**: High (a new write route that creates a folder in the owner's home; account state feeds sync and agent run)
**Priority**: P2
**Assigned agents**: backend-developer (route + core), then frontend-developer (Agents page)
**Agent guides**: `agents/backend.md`, `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` (accounts FRs), `docs/ddr/0001*` (never read real `~/.claude*` content)
2. `BRAINSTORMING_LOG_BOARDRUN.md` (T132 section: Path A, edge cases)
3. `rust/crates/core/src/accounts.rs` (`prepare_add`, `add`, `ignore`, `confirm`), `rust/crates/core/src/ingest/accounts.rs` (`insert`, `detect`, `provider_of`), `rust/crates/app/src/writes.rs` (`account_add`: lock, guard, error shape), `rust/crates/app/src/api.rs:292`, `rust/parity/run_parity.py` (Rust-only routes list)
4. `web/src/AgentsView.jsx` + test
- Tests use a temp agent root only. Never touch, read or list the owner's real `~/.claude*` folders.

## Requirement
Owner 2026-11-20: "from the agents, let suppoert feature to remove tha account and adding the account". Owner decisions: **Add = create a new account** (the app makes the folder and shows how to sign in); **Remove = from the app only** (folder and past usage kept, never re-detected, default refused), done by **reusing ignore** with a collapsed "Removed (N)" section that can restore; follow the existing styles.

## Dependencies & Reachability
**Depends on**: none
**Entry point**: Agents tab → "New account" (top of list); a confirmed account's **Remove** button; "Removed (N)" section

## Acceptance Criteria
| # | Criterion | Evidence |
|---|---|---|
| 1 | `POST /api/accounts/create {name}`: name must match `^[a-z0-9][a-z0-9-]{0,29}$`, else 400 before any filesystem work (`..`, `/`, upper case, empty, non-string) | route tests |
| 2 | Creates `<agent root>/.claude-<name>` with `create_dir` (mode 0700) and `settings.json` = `{}` (create-new, mode 0600); then registers it through the same path as `accounts::add`; returns 201 + the account row; the row starts `detected` | route test on a temp root: perms, file, row |
| 3 | An existing path (dir, file or dangling symlink) at that name → 409, nothing written or changed | route tests |
| 4 | Any failure after the folder is made (settings write, store) removes the folder it made, and only that one | test with a forced failure |
| 5 | Same token/origin guard, body limits, write lock rule (`prepare_add`: filesystem work outside the lock) and leak gate as `POST /api/accounts`; the route is in the parity Rust-only list | route tests + `rust/parity` list + leak gate run |
| 6 | Page: "New account" form (name field with hint, Create) → on success shows the sign-in command `CLAUDE_CONFIG_DIR=<folder> claude` with the existing CopyCommand, then the new account's Name field (existing detected flow) | vitest |
| 7 | Page: a confirmed non-default account shows **Remove** (calls the existing `/ignore`); the default shows none; detected accounts keep **Ignore** | vitest |
| 8 | Page: ignored accounts are not in the main list; a collapsed "Removed (N)" toggle lists them with **Use this account** (existing confirm flow); N = 0 hides the toggle; both group-by modes | vitest |
| 9 | Removing the agent-run account turns the run off (existing rule, `agent_run.rs` setting) — a test proves it holds after `/ignore` | Rust test |
| 10 | Workspace tests, clippy, web suite, lint, build pass | commands below |

## Approach
Path A (no schema): core `accounts::create(agent_root, name) -> Done<Folder>` does the checks and filesystem work and hands its `Folder` to `add`; `writes.rs` gets `account_create` mirroring `account_add`; route in `api.rs`. Page: `NewAccount` block, `Remove` label, `Removed (N)` section filtering `status === 'ignored'`.

## Files to change (predicted)
| File | Change |
|------|--------|
| `rust/crates/core/src/accounts.rs` | `create` + tests |
| `rust/crates/app/src/writes.rs`, `rust/crates/app/src/api.rs` | route |
| `rust/crates/app/src/api_tests.rs` (or the accounts route tests) | route tests |
| `rust/parity/run_parity.py` | Rust-only route listed |
| `web/src/AgentsView.jsx`, `web/src/AgentsView.test.jsx` | New account, Remove, Removed section |
| `README.md` accounts part | one paragraph |

## Files must NOT touch
| File | Reason |
|------|--------|
| `rust/crates/core/src/schema_v11.sql` | no schema change (Path A) |
| `rust/crates/core/src/ingest/accounts.rs` detect rules, `rust/crates/core/src/sync.rs`, keychain | out of scope |
| owner's real `~/.claude*` | DDR-0001 |

## Test plan
Rust: route + core tests on a temp agent root; leak gate. Web: vitest for ACs 6–8; `ux-evaluate` 375/1280. `/verify`: real serve with a temp `HOME`, create → row listed → remove → hidden under Removed → ingest → still not re-detected as new.

## Verification Command
```bash
cd rust && cargo test --workspace && cargo clippy --workspace -- -D warnings && cd ../web && npm test -- --run && npm run lint && npm run build
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
