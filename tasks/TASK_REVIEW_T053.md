# TASK_REVIEW — T053: Rust T043 review fixes

> Sibling of `tasks/TASK_GUIDE_T053.md`. Everything here is **filled by the reviewer at Stage
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

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its proposal>; Advocate 2: <its proposal>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Memory: <each entry the memory pass wrote, as `file — entry` | nothing new>
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours)>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/memgraph_parity.rs` and `rust/crates/app/tests/` (cases from the BEFORE block now permanent), golden `rust/parity/golden-memgraph/cases-t053.json`, fixtures `rust/parity/fixtures-memgraph/{em-store,projem,seed-t053.sql}`, per-project leak scan with a mutation proof in `rust/parity/leak_rust.py` |
| Verification command run | ☑ pass | Author, commit b2b7af0, three TZ runs: 632 passed / 2 failed, 633 / 1, 633 / 1. The failures are the known `workspaces_parity::edge_store_python_values` (worktree under `.claude/`) and one unrelated timing flake. `leak check: 0 hits over 42 canaries`; clippy and fmt clean; goldens had no deletions. Independent re-verification re-ran the generator: both golden files byte-identical to the committed ones. |
| Negative cases hold | ☑ pass | BEFORE block (cases 1-6) is the red evidence against a59def6: `\\*` gave `* hi`, a FIFO `.git` hung, a 100 MB `commondir` was read whole, dangling symlinks were mis-scored, duplicate ids put link counts on the wrong node; the leak scan has a mutation proof. |
| verify | ☑ pass | Independent qa-expert run, 2026-11-12, HEAD b2b7af0, `TZ` unset, worktree under `.claude/`: `cargo test --workspace --no-fail-fast` 632 passed / 2 failed / 26 ignored. Failures: the known `workspaces_parity::edge_store_python_values`, and `ingest_coexist::serve_mcp_and_store_info_answer_while_a_rust_ingest_runs` with `StorageFull` while writing its fixture (`/tmp` was 95% full from parallel agents; re-run alone: `1 passed`). T053 tests: `memgraph_parity` 27/27, `--lib` 36/36, `board_memgraph` 3/3. `leak_rust.py`: `leak check: 0 hits over 42 canaries, 212 surfaces, 913260 bytes`. Clippy (default and `--features window`) and `cargo fmt --check` clean. Hostile drive over a real `serve` (temp copy of `store-rich`, `env -i`): a project whose `.git` is a FIFO answered `GET /api/memory-graph` and `?project=` in 0.045 s (200); a worktree-style `.git` with a 20 MB `commondir` answered in 0.025 s (200), server peak RSS 18 MB; no token / bad Host / bad Origin -> 403, POST -> 405. Not re-run here: the TZ=Pacific/Kiritimati and Pago_Pago suites and the leak mutation proof (author and earlier reviewer evidence stand). |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Independent re-verifier on b2b7af0 (read-only): no P0, P1 or P2. Accepted P3s are listed in the review notes: `metrics.rs` `realpath` has a 40-link cap and no seen map, a NUL in a `.git` line, 4096 bytes vs characters, HOME-empty wording, `is_dir` left in `identity`, hard-link dedupe by inode. |
| Full smoke suite still green (no regression) | ☑ pass | Same author runs: 633 passed, 1 failed (known). |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier not run for this task. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | No UI file changed (Rust routes, docs and tests only). |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | No UI file changed (Rust routes, docs and tests only). |
| **UI: Responsiveness at target viewports** | ☑ N/A | No UI file changed (Rust routes, docs and tests only). |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-12T04:24:58Z (cases 1-3, 5, 6) and 2026-11-12T04:25:17Z (case 4) on commit `a59def6`, before any
implementation change, with a throwaway test file `rust/crates/core/tests/t053_before.rs` (deleted before the first commit), run
as `cargo test -p easy-personal-core --test t053_before -- --exact <case> --nocapture`. Python's values are what the Python
source returns by reading `board.py` / `easymem.py` / `memgraph.py`; the Python-generated golden in the same task proves them later.

```
BEFORE1 titles: ["* hi", "\\y"]            (python: ["\\* hi", "\\\\y"])        # AC 1: text `- T1 \\* hi` and `- T2 \\\\y`
BEFORE2 HUNG: find() did not return within 5 s on a FIFO .git                    # AC 2: `<project>/.git` is a FIFO
BEFORE3 bytes read by find(): 104857739   (100 MB sparse commondir; Python readline(4096) reads at most 4096)
BEFORE4 find() returned after reading 2147483785 bytes of /dev/zero (stopped only by `ulimit -v 3 GB`; Python reads at most 4096)
BEFORE5a cited states [("dang","unchecked"),("out","unchecked"),("real.txt","stale")], outside=0
        (python: dang -> gone, out left out of the graph, outside=1)             # AC 4: dangling symlinks
BEFORE5b meta path a/link/.. finds project b: false                              (python: true)
BEFORE6 nodes (name, id, links_out): [("idx-a","claude:1:memory/MEMORY.md",1), ("c1",..,0), ("idx-b","claude:1:memory/MEMORY.md",0)]
        (python: links_out=1 lands on the LAST node with the shared id, idx-b)   # AC 3: duplicate node ids
```

**AFTER** (2026-11-12): the same cases are now permanent tests in `rust/crates/core/tests/memgraph_parity.rs` (and unit tests in `board.rs`, `easymem.rs`, `memgraph.rs`).
Run against the OLD `a59def6` sources at 2026-11-12T04:34:53Z (`ulimit -v 4 GB`): 11 of 27 fail (`kanban_texts_parse_as_python_does` on `escaped_backslash_pairs`, the FIFO, the 100 MB `commondir`, the
`/dev/zero` link, dangling/outside cited links, `lnk/..`, duplicate ids, 200,000 links in 15 s, the four-zone panel, the canary run: easy-memory store with a BOM `meta.json` not found). Against the new
sources: `memgraph_parity` 27/27, `--lib` 36/36, `board_memgraph` 3/3; `leak_rust.py`: `0 hits over 42 canaries, 212 surfaces` and, with one canary echoed on purpose, `3 hits ... [api GET /api/memory-graph project=repo]`.

**DELTA**: a hostile `.git`/`commondir` can no longer hang or flood the memory-graph route, `\\*` in a kanban title reads as Python reads it, and the easy-memory layer is checked against Python over a store with notes.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T053.jsonl`, never the
implementing agent alone]
