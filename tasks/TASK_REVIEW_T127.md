# TASK_REVIEW — T127: Live like Claude 2d — allow always and plan cards

> Sibling of `tasks/TASK_GUIDE_T[NNN].md`. Everything here is **filled by the reviewer at Stage
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
With `/verify (yours)`, your pass goes in the `verify` row below: Result `☑ pass`, Notes `/verify by the user, session <id>, commit <sha>`.

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its proposal>; Advocate 2: <its proposal>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Memory: <each entry the memory pass wrote, as `file — entry` | nothing new>
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours), with [admin] only after the one retry also ended blocked>

-->

## Auto decisions

Mode: auto
1. Allow always sends back only the CLI's own suggestion, by index, unchanged; the app writes no settings file — RULE: docs/ddr/0008-live-subagent-logs-and-steering.md:15 "Allow always passes the CLI's own suggestion back unchanged"
2. Direction Path A of `BRAINSTORMING_LOG_LIVE2.md`, and terminal ExitPlanMode via PermissionRequest only (PreToolUse path cut, AC3 reworded) — DEBATED: Path A + cut PreToolUse, asked twice, all six Advocates agreed (first round: Advocate 1: cut, PermissionRequest entry has no matcher so plans already reach the card, PreToolUse has no updatedPermissions; Advocate 2: cut, a PreToolUse path would put two cards on one call and change every FR-113 install, but add a real-terminal smoke check and a version feature-check later; Advocate 3: cut, smallest change, no settings file touched) ⚠ decided for you — please review
3. Stage 4 P1 process finding (role guides not read by the implementer) closed by a Supervisor check of the diff against agents/backend.md:76-82 — RULE: `.claude/skills/code-review/SKILL.md` § Phase 4 — Apply Safe Fixes
4. Not built: version feature-check (buttons appear only when the CLI sent `permission_suggestions`); built: projectSettings labelled "this project (shared)", localSettings "this project (just you)" — DEBATED: split label, no version check (Advocate 1: split the label, skip the check, presence already gates it; Advocate 2: split the label, the shared file is a different place, skip the check; Advocate 3: leave both, or the cheap hedge "this project (shared)"); settled by PRD.md:231 "naming where it lands" ⚠ decided for you — please review
Checks: tests `cargo test --workspace --no-fail-fast` — 1344 passed / 0 failed (Supervisor, before and after the label fix; one pre-existing flaky session_procs test seen once, passes on rerun); `npm test` — 43 files, 1455/1455; reviews code-review — 0 P0, 1 P1 (process, closed), 1 P2 (debated, cut), 1 P3; security-review — no finding >= 8 (independent opus reviewer); real run leak_rust.py — printed `leak check: 0 hits over 42 canaries` (implementer)
Memory: memory/decisions.md — 2026-11-20 — T124-T128 Live like Claude 2 resumed and Stage 4 under [admin] (T127: allow always by index, PreToolUse plan path cut); memory/learnings.md — 2026-11-20 — leak_rust.py needs the main .venv python and env -u CLAUDE_CONFIG_DIR
Not done: merge (admin: after the auto verify pass); auto verify: running
Paused: auto-mode check failed — Risk High: decision 4 (`SAFER OPTION`) is a choice — it is flagged ⚠ — so it must be `DEBATED` with exactly 3 Advocates named [report 1d76a91e9400]
Paused: done not moved — report incomplete: auto verify slot

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/src/permission_suggestions.rs` (4), `rust/crates/app/src/app_session.rs` (4: suggestion unchanged, forged/misplaced, none, plan accept-edits), `rust/crates/app/src/app_session_routes.rs` (1: HTTP, forged index 400), `rust/crates/app/src/answer_bridge.rs` (4: hook output by index, none, terminal plan, cut plan); `web/src/PromptCard.test.jsx` (5), `web/src/LiveAppPanel.test.jsx` (answerBody), `web/src/LiveAnswer.test.jsx` |
| Verification command run | ☑ pass | 2026-11-19T22:47Z in the worktree. `cargo fmt --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` and `-p easy-personal-rs --features window` clean; `cargo test --workspace`: 1344 passed, 0 failed; `leak_rust.py` (parity venv python, `env -u CLAUDE_CONFIG_DIR`): `leak check: 0 hits over 42 canaries, 1306 surfaces, 2852094 bytes`; `npx vitest run`: `Test Files 43 passed (43) / Tests 1455 passed (1455)`; `npm run build` built; `check_static: ok` |
| Negative cases hold | ☑ pass | forged index (2, 7, -1, "0", 1.5, usize::MAX), suggestion with deny, suggestion with accept_edits, accept_edits on a non-plan, suggestion on a plan, a cut input or plan, no suggestions: all refused (400 / Incomplete) and the prompt stays pending (tests listed above). `git diff --stat`: 11 files changed, 913 insertions |
| verify | ☑ pass | auto-run by hook, session f2dbcf16-43d0-411a-b006-d0afdca451d0, commit 2d937dc70042dd8323bb274b58f980d50f3b18d8, log .claude/verify-runs/T127-2d937dc70042.log — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | prompt build and answer in `app_session.rs`, `app_session_routes.rs`, `answer_bridge.rs`; cards in `PromptCard.jsx`, `SessionPanel.jsx`, `api.js`; Stage 4 is the Supervisor's |
| Full smoke suite still green (no regression) | ☑ pass | the whole Rust workspace and the whole web suite, counts above |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier refuses repo paths since 2026-11-01 (memory/learnings.md, easy-verifier registered); advisory only |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | ux-evaluate steps by hand (a sub-agent has no Skill tool): throwaway Vite harness of the three cards, screenshots read at 375 and 1280 (harness deleted; scratch `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/b0e607b5-0126-47ad-b65d-c4fbbbdebda6/scratchpad/ux/cards-375.png`, `cards-1280.png`). U1 pass: buttons read "Allow and don't ask again for Bash(git status:*) — this project" and "… — you, everywhere". U2 pass: a terminal plan card shows the numbered plan text in the plan box with Approve / Approve and auto-accept edits (this session) / Keep planning / Answer in terminal, no JSON |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | the new buttons use the card's existing button style; no new CSS |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 and 1280: no horizontal overflow; long button labels wrap onto two lines at 375 and stay one line at 1280; wrapped buttons are about 40px tall at 375 (the card's existing button style, unchanged) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (Supervisor, before any implementation commit; `main` the branch point):
```
$ date -u; git log -1 --oneline; grep -rn 'permission_suggestions\|updatedPermissions' rust/crates/app/src web/src | wc -l
2026-11-19T11:18:19Z
af4aa48 Board + memory: T122, T123 Done (merged); font decision
0
```

**AFTER**:
```
$ date -u; grep -rn 'permission_suggestions\|updatedPermissions' rust/crates/app/src web/src | wc -l
Thu Nov 19 10:47:12 PM UTC 2026
23
```

**DELTA**: From a Live prompt card the owner can now allow a tool and have Claude Code remember it ("don't ask again", with where the rule lands on the button), and approve a plan with edits auto-accepted, for app and terminal sessions; a terminal plan reads as a plan.

**WITNESS**: resumed backend/frontend agent ran the Verification Command in this worktree on 2026-11-19 (output above); `memory/event-trace/T127.jsonl` was not present in the worktree when this was written, so the Supervisor confirms it from the trace.
