# TASK_GUIDE — T129: Bug — an app session started from Live shows nothing and cannot be worked with
**Date**: 2026-11-20
**Complexity Level**: C2
**Risk Level**: High (app session runner and the transcript route: the owner's own `claude`, the NFR-002 fifth exception)
**Priority**: P1
**Assigned agent**: backend-developer (→ frontend-developer if the page is part of the cause)
**Agent guide**: `agents/backend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` FR-109..FR-116, FR-123, FR-128, FR-131, NFR-002
2. `docs/ddr/0006-write-sessions-cli-parity.md`, `docs/ddr/0007-live-transcript-show-never-store.md`, `docs/ddr/0008-live-subagent-logs-and-steering.md`
3. `rust/crates/app/src/app_session.rs` (`argv`, `child_env`, start, stdout handling), `rust/crates/app/src/transcript_route.rs` (`main_file`, `past`), `web/src/SessionPanel.jsx`, `web/src/SubAgents.jsx`, `web/src/Transcript.jsx`, `web/src/LiveView.jsx`
- Tests write only to temp config dirs and a temp `EASY_PERSONAL_HOME`; never the real `~/.claude*` or `~/.easy-personal`. Never open a `*.key` file or a Claude Code messaging socket. Kill only processes your test started. A spike with the real `claude` runs in a scratchpad temp folder, as the T102/T126 spikes did (`tasks/evidence-T126/SPIKE.md` § How it ran).

## Bug Fix Task Guide — T129

### Mental Model (confirmed by user)
- Observed: owner started an app session "idea-ladder-4a" from Live (account `hung`, model opus, mode dontAsk, effort medium) on the 2026-11-20 `.deb` (main `e2e163e`, 0.3.0). The panel shows the stream's numbers (1,883 tokens, $0.40, 5 tool calls, "Waiting for your message", Last record 1 min ago) but the conversation area is an empty box: no text, no "Loading", no "cannot be shown", no "Nothing yet". The owner says it "just starts and can not do any things related to this session", and the session is in no Claude history (terminal `/resume` list, `~/.claude*`). Screenshot: `/home/hungnguyenhuu/Pictures/Screenshots/Screenshot From 2026-11-20 09-47-40.png`.
- Expected: starting a session from Live shows **Working** while Claude runs, the conversation fills in live in the panel, a message sent from the panel is answered, and afterwards the session is in the Claude history of the account it ran under (resumable).
- Likely divergence point (from code reading, not proven): (a) the app starts `claude -p --input-format stream-json --output-format stream-json --verbose … --permission-prompt-tool … --mcp-config …` with `CLAUDE_CONFIG_DIR` = the account's folder (`app_session.rs` `argv`, `child_env`); the transcript is written (if at all) under that account's `projects/`, which the transcript route may not resolve for an app session (`transcript_route.rs` `main_file`: live tail origin, else the store's past path), and the page renders an empty log instead of a "cannot be shown" line; or (b) Claude Code does not persist this run at all (a `-p` stream-json run, the cwd, or the session id differ from what the app shows), so there is nothing to read or resume.
- Recent context: T124 (agent= and the listing on the transcript route), T125 (`Conversation` wraps `Transcript` in the panel), T126 (`--effort`, steering), T127 (prompt answers), T128 (New session sheet in Live) all merged 2026-11-20. T118 made the transcript route the panel's only conversation source for a session with an id.

### Mental model update (diagnosis, owner-approved 2026-11-20)
- Diagnosis (`ca1175f`) disproved (a) and (b): Claude Code saves the app session under the account's `projects/` and `--resume` works; the route resolves it and answers 200. Proven cause: the route's entries are keyed `type` (`rust/crates/core/src/transcript.rs`), the page reads `kind` (`web/src/Transcript.jsx`), so every entry draws nothing — in every conversation view (terminal, past, sub-agent logs), since T118.
- Owner decision 2026-11-20 ("go with both"): fix on the route side (entries carry `kind`; the agents listing keeps `type` = agent type); add a contract test that renders the page over the Rust golden answer; AC 5 one-line message when entries exist but none draws. AC 4: the session is saved and resumable; not appearing in the terminal `/resume` list is Claude Code's own picker, no code change.

### Intake
- Trigger: Live → New session → folder `idea-ladder`, account `hung`, model opus, mode dontAsk, effort medium → Start (owner's first prompt) → open the new session's panel.
- Severity: P1 (blocking: an app session cannot be followed or used).
- Affected area: `rust/crates/app/src/app_session.rs`, `rust/crates/app/src/transcript_route.rs`, `web/src/SessionPanel.jsx` / `SubAgents.jsx` / `Transcript.jsx` (conversation view for an app session).

### Complexity & Risk
- Complexity: C2 (floor C1 for a bug touching >1 file)
- Risk: High

### Diagnosis Gates (Pillar 1 — must pass before any fix)
- [x] Phase 1 feedback loop built and running (a real `claude` app-session start in a scratchpad temp repo through the app's own `Sessions::start`, the route over the file it wrote, and a vitest over the route's real answer shape)
- [x] Bug reproduces deterministically on the loop (2026-11-20T03:14-03:15Z; see `tasks/TASK_REVIEW_T129.md` Repro loop)
- [ ] 3–5 ranked falsifiable hypotheses listed (consistent with confirmed mental model) — NOT consistent: the proven cause is a third one (below); stopped for the Supervisor
- [x] Correct hypothesis identified via Phase 4 instrumentation (H3, see Attempts Log)

### Attempts Log (filled live during diagnosis — required if >1 hypothesis tested)
| # | Hypothesis | Predicted signal | Actual result | Verdict |
|---|---|---|---|---|
| 1 | (b) Claude Code does not persist an app session (`-p` stream-json run) | no `<id>.jsonl` under the account's `projects/<cwd>/`; `--resume <id>` fails | Real run through `Sessions::start` (haiku, dontAsk, effort medium, scratchpad repo, account `CLAUDE_CONFIG_DIR` kept): the app view got `session_id` 8715b26e-…; `projects/<munged cwd>/8715b26e-….jsonl` exists, 31 records, `entrypoint: sdk-cli`; `claude -p --resume 8715b26e-…` answered "Your previous message was: "Reply with the single word OK."" | Disproven |
| 2 | (a) the transcript route does not resolve an app session's file, so the page draws an empty log instead of "cannot be shown" | route 404 or no origin from the live tail | The live tail read the file (`session_origin` = the `.jsonl` and the repo folder); `GET /api/live/sessions/<id>/transcript` answered 200 with entries (user "Reply with the single word OK.", assistant "OK", skipped runs). A 404 would have drawn "The conversation cannot be shown", so the route resolves | Disproven |
| 3 | The route's entries name their kind in `type`; the page reads `kind` | every entry from the real route has `type` and no `kind`; the page draws an `<ol role=log>` with 0 children (no "Loading", no "cannot be shown", no "Nothing yet" because `entries.length > 0`) | Rust debug test: every entry `keys=[cut, offset, text, type]`, `kind=null`. Vitest over core's golden answer (14 entries): `log <ol> children: 0, innerHTML: ""`, `getByText('<typed text>')` fails. `Transcript.jsx` `Entry` returns `null` for an unknown kind | Proven |

**Stuck checkpoint** (if 2 consecutive hypotheses disproven):
- [ ] 3 options presented (next hypothesis / widen scope / abandon+escalate)
- [ ] Chosen option: ___
- [ ] User's explicit go-ahead: ___

### Fix Gates (Pillar 2)
- [x] Regression test written before the fix (or no-seam documented)
- [x] Fix applied; regression test passes
- [x] Phase 1 loop no longer reproduces the bug
- [x] Fix matches the "correct behaviour" definition from the mental model

### Cleanup Checklist (Pillar 3)
- [x] All [DEBUG-...] instrumentation removed (grep verified)
- [x] Throwaway prototypes deleted
- [x] Correct hypothesis stated in commit message
- [x] Post-mortem: a web test that reads the Rust golden answer (added) instead of a hand-made fixture

## Dependencies & Reachability
**Depends on**: none (T124–T128 merged)
**Entry point**: Live → New session → the new session's panel (`web/src/SessionPanel.jsx`)

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | A session started from Live shows its conversation in the panel while it runs and after (same entry view as a terminal session), never a blank box | FR-123, FR-128 |
| 2 | Its state reads Working during a turn and Waiting for your message after | FR-128 |
| 3 | A message sent from the panel reaches the session and its answer appears | FR-110 |
| 4 | The session is saved by Claude Code under the account it ran under and can be resumed (`--resume` from the app, or the CLI's own history for that account) | FR-112, DDR-0006 |
| 5 | When the conversation truly cannot be read, the panel says so in one line (never an empty box) | FR-123 |
| 6 | Regression test(s) lock the cause; Rust lint/test/leak and web `npm test`/build/check_static pass | all |

## Files must NOT touch
| File | Reason |
|------|--------|
| `~/.claude.json`, any real `~/.claude*` file, any `*.key`, Claude Code's messaging socket | DDR-0001, DDR-0007 |
| store schema (`rust/crates/core/src/schema_v11.sql`) | no schema change |
| `src/easy_personal/**` | Python app |

## Verification Command
```bash
cd rust && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace --no-fail-fast && cd ../web && npm test && npm run build && cd .. && python3 scripts/check_static.py
```

## Completion checklist
- [ ] Diagnosis gates checked
- [ ] Fix gates checked
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed (owner: start a session from Live, see Working and the conversation, send a message, find it in the account's history)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete

## Evidence (filled by reviewer at Stage 4/5)

> **Moved.** Filled by the reviewer at Stage 4/5 in `tasks/TASK_REVIEW_T129.md`.

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_T129.md`.
