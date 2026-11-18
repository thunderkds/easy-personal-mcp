# TASK_REVIEW — T115: Live like Claude 1 — the transcript route

> Sibling of `tasks/TASK_GUIDE_T115.md`. Everything here is **filled by the reviewer at Stage
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

-->

## Auto decisions

Mode: auto
1. Transcript read on demand by a route, not streamed on /api/live — FACT: BRAINSTORMING_LOG_LIVENATIVE.md Path A (Path B rejected: every page would get all text)
2. Full tool input and output shown, no redaction — FACT: PRD.md:344 "trace = **full transcript**, shown never stored; busy input = **queue until the turn end**"
3. Tool result split across pages: orphan entry with the same id, the page merges by id field-wise (no held offset) — DEBATED: (a) orphan + merge (Advocate 1: orphan entry, server stateless, every record sent once; Advocate 2: orphan entry, a held offset would freeze the 2 s feed during a long Bash run; Advocate 3: hold `next` at the first unresolved call, with a cap) ⚠ decided for you — please review
4. Answer gains `start` (offset of the first entry) for Load earlier — RULE: T118 contract gap; smallest addition to the FR-123 shape
Checks: tests `cargo test --workspace --no-fail-fast` — 1285 passed, 1 failed (`store_check` sigterm test, load-related; `--test store_check` alone 12 passed); leak gate `leak_rust.py` — 0 hits over 42 canaries, 1234 surfaces; reviews code-review — 0 P0/P1, 1 P2 + 3 P3 fixed in `ca9e153`, re-check of `ca9e153` no regression; security-review — no finding ≥ 8 (two same-user-write notes below the bar)
Memory: memory/decisions.md — "2026-11-18 — Live like Claude: transcript on demand, queue, Stop/Force stop, past sessions, one panel" (+ MEMORY.md index line), `5c38bf4` on main
Not done: merge (admin: after the auto verify pass); owner smoke test (open a running terminal session in Live and read its conversation); auto verify: runs next


<!-- template:
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

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/src/transcript.rs` tests (15: pairing across records, cut at 16,000 while building, 500 entries / 2 MB body both directions, after/before paging, mid-line offset, skipped runs, far orphans, `start`), `rust/crates/core/tests/transcript_golden.rs` + `transcript_golden/session.jsonl` (every record kind of claude 2.1.327), `rust/crates/app/src/transcript_route.rs` tests (6: live file re-read each call, past path from the store, 404 one line for gone/unknown/hostile/outside-roots files, guard); `rust/parity/leak_rust.py` T115 canaries |
| Verification command run | ☑ pass | at `ca9e153`: `cargo fmt --check` ok; clippy `-D warnings` workspace and `--features window` rc 0; `cargo test --workspace --no-fail-fast` 1285 passed / 1 failed (`store_check::sigterm_stops_like_ctrl_c_and_the_private_copy_is_removed`, load; alone `12 passed; 0 failed; 1 ignored`); `leak check: 0 hits over 42 canaries, 1234 surfaces, 2828649 bytes` |
| Negative cases hold | ☑ pass | mutation: an added `eprintln!` of the entry text made the leak gate print `HIT T115 transcript_user … [serve stderr]` (reverted); hostile files (symlink, FIFO, directory, 32 MB line, `/etc/passwd`, `..`, `.txt`, symlinked folder outside the roots) → 404; no token / wrong token / wrong Host / foreign Origin / query token → 403 |
| verify | ☑ pass | auto-run by hook, session 9cde4efc-3f82-4e1b-8602-083795893c8a, commit 3784378174060a0507d04f7548921e38280059f3, log .claude/verify-runs/T115-378437817406.log — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | code-review and security-review on `main...T115` (transcript.rs, transcript_route.rs, live.rs helpers, store.rs lookup, api.rs route, leak gate) plus callers; re-check of the fix commit `ca9e153` |
| Full smoke suite still green (no regression) | ☑ pass | Rust 1285 + the load-flaky `store_check` target green alone; leak 0 hits
| **Companion score (easy-verifier)** | ☑ N/A | not run: independent code and security reviews used instead | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | server route only; the page is T118 |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | server route only; the page is T118 |
| **UI: Responsiveness at target viewports** | ☑ N/A | server route only; the page is T118 |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (Supervisor, before any T115 implementation commit; `main` at `ddb947f`, the branch point):
```
$ date -u; git log -1 --oneline; grep -rn 'transcript?after\|/transcript' rust/crates/app/src/ | wc -l; ls rust/crates/core/src/transcript.rs
2026-11-18T10:15:50Z
ddb947f Plan: Live like Claude (question 45, FR-123..FR-128, DDR-0007, T115-T119)
0
ls: cannot access 'rust/crates/core/src/transcript.rs': No such file or directory
```
(No transcript route and no parser: the Live view shows only tool labels for terminal sessions.)
first implementation commit] OR [verbatim excerpt of the prior content, for non-executable changes]

**AFTER** (`51834d8`, 2026-11-18T11:41:59Z):
```
$ grep -rn '/transcript' rust/crates/app/src/api.rs | head -2; ls rust/crates/core/src/transcript.rs
rust/crates/app/src/api.rs:222:            "/api/live/sessions/{id}/transcript",
rust/crates/core/src/transcript.rs
```
The route answers `{entries, next, start, complete}` for a live or past session (golden test `every_record_kind_gives_the_golden_entries`; route tests in `transcript_route.rs`).

**DELTA**: the page can now read any session's whole conversation (messages, Claude's text, each tool call's input and result) on demand, without the app storing any of it.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
