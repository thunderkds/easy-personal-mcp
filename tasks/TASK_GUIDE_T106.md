# TASK_GUIDE — T106: Write sessions 4 — answer bridge: `hooks install|remove|status`, `hook ask`, the answer socket and routes
**Date**: 2026-11-17
**Complexity Level**: C3
**Risk Level**: High (writes Claude Code user settings; a fault touches every session on the machine)
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `docs/ddr/0006-write-sessions-cli-parity.md`, `docs/ddr/0001-*` (why `~/.claude.json` is never edited)
2. `PRD.md` FR-113, FR-114, FR-115, FR-116, FR-106 (cleaning and caps), question 41
3. `tasks/evidence-T102/SPIKE.md` AC 6 (exact hook output that works)
4. `rust/crates/core/src/live.rs` (`call_questions`, the waiting state), `rust/crates/app/src/live_feed.rs`, `rust/crates/core/src/accounts.rs` (config dir per account), `rust/crates/app/src/connector_writes.rs` (how user-scope writes are done today)
- Tests write only to temp config dirs; never the real `~/.claude*`.

## Requirement
Owner 2026-11-17: "you can input the text to the ask question in the app and send to claude session are running"; hook install as an opt-in button (question 40).

## Dependencies & Reachability
**Depends on**: T102 (go on route 6). Independent of T103-T105.
**Entry point**: `easy-personal-rs hooks …`, `easy-personal-rs hook ask`, `/api/hooks`, `/api/answer`

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | `hooks install --account <id>`: adds exactly the two entries (PreToolUse `AskUserQuestion`, PermissionRequest), absolute binary path, timeout 130; every other key and hook kept (golden test on a settings file with other hooks, `permissions`, unknown keys); temp + rename; backup in `~/.easy-personal/backups/`; invalid JSON → refused, file byte-identical | FR-113 |
| 2 | Running install twice adds nothing; `remove` drops only equal entries (a hand-edited entry is kept, `status` = *changed by hand*); `status` = *binary moved* when the path is gone | FR-113 |
| 3 | `GET /api/hooks` + `PUT /api/hooks` `{account, on, confirmed:true}` behind the guard; the GET shows the exact entry text for the confirm step | FR-113, FR-116 |
| 4 | `hook ask` with no socket: exit 0, empty stdout, < 50 ms (timed test, 20 runs); a socket owned by another uid or a stale socket → same | FR-114 |
| 5 | With the app: the question (FR-106 cleaning; header, labels, descriptions capped) appears as `pending_answer` on `GET /api/live`; `POST /api/answer` → `hook ask` prints the T102-proven JSON; a multi-select joins labels with `, `; Other text cleaned, ≤ 2,000 chars | FR-114 |
| 6 | Release, 120 s timeout, app exit mid-wait, malformed stdin → exit 0, empty stdout | FR-114 |
| 7 | Two waiting sessions → answers never cross (keyed by session + tool use id + random id); a second answer → 409 | FR-114 |
| 8 | Socket: `~/.easy-personal/answer.sock`, 0600, folder 0700, peer uid check; removed on clean exit, replaced on start when stale | FR-114 |
| 9 | Leak gate: option labels, Other text and the permission summary reach `/api/live` only; never the store, logs, `/api/alerts`, tray, popups, MCP, CLI (`hook ask` stdout is the documented hook JSON only) | FR-115 |
| 10 | `cargo fmt --check`, clippy `-D warnings` (with and without `window`), `cargo test --workspace` pass | all |

## Files must NOT touch
| File | Reason |
|------|--------|
| `~/.claude.json` | DDR-0001 |
| project `.claude/settings.json` of any repo | kernel-owned |
| `LiveTail::snapshot()` content read by tray/alerts | DDR-0005 split stays |
| `src/easy_personal/**` | Python app |

## Verification Command
```bash
cd rust && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p easy-personal-rs --features window --all-targets -- -D warnings && cargo test --workspace 2>&1 | tail -5 && python3 parity/leak_rust.py
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed (real terminal session answered from the app, owner gate)
- [ ] `docs/` updated (README: turning the bridge on and off, what it writes)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
