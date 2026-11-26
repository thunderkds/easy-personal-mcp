# TASK_REVIEW — T136: Git diff 1 — read-only diff routes

> Sibling of `tasks/TASK_GUIDE_T136.md`. Everything here is **filled by the reviewer at Stage
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

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass / ☐ fail | At `50870c3`: `rust/crates/core/tests/repo_diff.rs` (30 tests, 1 of them run as a child process), `rust/crates/core/src/repo_diff.rs` `mod tests` (3 tests), `rust/crates/app/src/diff_routes.rs` `mod tests` (4 tests). Every criterion has a test. Gaps: no test holds that the patch call runs in the work tree root (R1); no test swaps a folder and a submodule (R2). |
| Verification command run | ☑ pass / ☐ fail | At `50870c3`, `cd rust && cargo test -p easy-personal-core --test repo_diff && cargo test -p easy-personal-rs diff_routes`: `test result: ok. 29 passed; 0 failed; 1 ignored` and `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 479 filtered out` |
| Negative cases hold | ☑ pass / ☐ fail | At `50870c3`, 22 hostile scratch layouts through the core functions under `strace`, plus the real binary's HTTP routes under `strace`: no program but `git` was executed, no planted marker, no `AF_INET` connect, no file content or inode changed. One timestamp change: `.git/sharedindex.*` of a split-index repo (R4). 18 single-edit mutants: 12 red, 6 survived (third review, Mutation run). |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass / ☐ fail | At `50870c3`: `d3215f5~1...50870c3`, close reading of `1cc683b..50870c3`, and how the T137 page (`9c59b3e`) takes `base` and `tree`. Skipped: web, Python app, ingest, store schema (unchanged in this diff). |
| Full smoke suite still green (no regression) | ☑ pass / ☐ fail | At `50870c3`, `cargo test -p easy-personal-core -p easy-personal-rs --no-fail-fast` under load average 13 to 29: exit 101, 100 suites, 1429 passed, 6 failed, 30 ignored. The 6 are timing-bound, in 3 files this diff does not change; each file run alone: `live_parity` 15 passed; `answer_bridge` 18 passed; `memgraph_parity` 26 passed, 1 failed at load 23, then 27 passed twice at load 1.4. `cargo clippy --workspace --all-targets -- -D warnings` exit 0; `cargo fmt --all -- --check` exit 0. Web and Python suites not run (no change there). |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☑ N/A | Not run in this review. |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☑ N/A | No UI file in the diff (page is T137). |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☑ N/A | No UI file in the diff. |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☑ N/A | No UI file in the diff. |

---

## Stage 4 review (fresh-context reviewer, commit `d3215f5`)

**Verdict: fail.** One P1 (F1). AC1 and AC8 are met only in part. No security vulnerability confirmed. No source or test file changed.

Probes ran the real `repo_diff` functions (release build of a scratch crate, git 2.53.0) on scratch repos under the session temp folder.

### Acceptance criteria

| AC | Result | Basis |
|----|--------|-------|
| 1 | partial | F1, F2 |
| 2 | met | tests; edge F10 |
| 3 | met for refusal; edge F8 | tests |
| 4 | met | tests; F3 on the cut |
| 5 | met in the tested cases; F5 on which folder a session gives | route tests |
| 6 | met | tests, mutants, probe below |
| 7 | met | tests, mutants |
| 8 | partial | F3, F4, F9 |

### P0
None.

### P1
| ID | File:line | Finding and failure scenario | Confirmed by | Conf. | Action |
|----|-----------|------------------------------|--------------|-------|--------|
| F1 | `rust/crates/core/src/repo_diff.rs:630-634` | A tracked binary file whose time changed but not its content is listed as `unstaged / modified / binary`. Case: a build rewrites the same image, or the repo folder is copied. `git status` says clean; the list says changed; opening it gives an empty patch. `!e.binary` keeps every binary entry, and git prints `- -` for a stat-dirty binary. The test of this rule touches only a text file. | Probe: commit `b.bin`, `touch -d 2001-01-01`; `git status --porcelain` empty, `files` answers `b.bin` modified, `file` answers `"patch":""`. | 100 | For an unstaged binary entry with no mode change, prove the content differs before keeping it (for example its patch has no `Binary files` line, as seen here, or compare blob ids). Add the touched-binary test. |

### P2
| ID | File:line | Finding and failure scenario | Confirmed by | Conf. | Action |
|----|-----------|------------------------------|--------------|-------|--------|
| F2 | `repo_diff.rs:672-680` | AC1: an untracked binary file is `binary: false` and has no counts in the list; it is marked only when opened. | Code; test at `tests/repo_diff.rs:176-183` asserts the mark only on the `file` answer for `raw.bin`. | 100 | Mark it in the list (bounded probe of the first 8000 bytes, same safe open), or have the owner change the criterion. |
| F3 | `repo_diff.rs:377`, `:292`, `:410` | AC8 "caps cut and say so": branches/tags are cut at 500 and worktrees at 100 with no flag. Refs sort heads, remotes, tags, so many branches hide every tag. | Probe: 505 branches + tag `v1` gave 500 entries, kinds `['branch']`, keys `base, branches, commits`. Worktree cap by code only. | 100 | Add a `truncated` flag to the overview answer. |
| F4 | `repo_diff.rs:627-634` with `:651-656` | When the `--raw --numstat` output passes 4 MiB, unstaged entries with no numstat field read as "empty" and are dropped. | Probe: 60,000 modified files gave `rows 0, total 0, truncated true`. 34,000 gave `total 17746`. | 100 | Drop an entry as empty only when the part was not cut. |
| F5 | `rust/crates/core/src/transcript.rs:624-645` | FR-139 "newest recorded folder": when the last 1 MiB holds no complete main-thread record with `cwd` (a final record over 1 MiB), the first 1 MiB is used, so the session opens the old folder or worktree with no notice. | Probe: records `/work/old` ... `/work/newer`, last line 1.1 MiB, gave `Some("/work/old")`; without the huge line `Some("/work/newer")`. | 100 | Read further back from the end (bounded) before the start fallback, or answer "folder not known". |
| F6 | `repo_diff.rs:148-156` | No test holds `GIT_NO_LAZY_FETCH` or the `GIT_*` removal; both can be deleted with every test green. Without the first, a partial clone fetches and runs the configured upload-pack program. | Mutants survived. Probe: app answered `git could not read this repo`, nothing ran; plain `git diff` on the same clone ran the planted `uploadpack`. | 100 | Add a partial-clone test and an inherited-`GIT_DIR` test. |

### P3
| ID | File:line | Finding | Conf. |
|----|-----------|---------|-------|
| F7 | `repo_diff.rs:586`, `:618` | Linear search per entry: 34,000 changed files took 4.3 to 5.1 s per call against 0.9 s for git, and every one-file open repeats it. Not under the 10 s git limit. Use a map by path. | 100 |
| F8 | `repo_diff.rs:804` | A literal pathspec still matches a folder: opening deleted file `a` (now folder `a/`) also returns the patch of `a/b` (probe). | 100 |
| F9 | `repo_diff.rs:251-253`, `:701`, `:410` | `shown` keeps raw newline and tab of a file name (probe: `"shown":"n\nl\tx"`); overview `base` is raw with no shown form (probe: U+202E passed through). | 100 |
| F10 | `repo_diff.rs:329-332`, `:506-509` | A tag named `main` wins over the branch: the default base is checked as `refs/heads/main` but then resolved by short name (probe: base was the tag's commit). | 100 |
| F11 | `repo_diff.rs:669-680`, `:736-743` | A nested untracked repo or worktree is listed as `inner/`, `openable: true`; opening it answers "that file is not in this compare; refresh the list" (probe). | 100 |
| F12 | `repo_diff.rs:708` | `filters_off` is true for committed compares too, where filters do not apply. | 100 |
| F13 | `rust/README.md` | The README describes other route families but not `/api/diff/*`. | 75 |
| F14 | `diff_routes.rs` tests | No planted-secret check that diff text reaches no store row, log, MCP or CLI answer (brainstorm edge list). Code reading found no write or log path. | 75 |

### Security review

Actor and boundary: content and config of a repo, against the app that runs git in it; and a web page, against the loopback routes.

| Lead | Verdict | Evidence |
|------|---------|----------|
| Repo config starts a program | rejected | Probe on both work trees, five compares, `overview`, `trees`, every listed file: filters with odd names (space, quote, backslash, dots, non-ASCII, `;$(x)`, empty name tested with git alone), filter from `include.path` and `config.worktree`, global filter via `HOME`, textconv, diff driver and external diff, pager, fsmonitor, ssh, askpass, editor, gpg, hooks, submodule fsmonitor/filter/diff, a merge conflict. No program ran. Plain `git diff` there ran textconv, fsmonitor and a filter. |
| Diff routes write to the repo | rejected | Same probe: size, mtime, inode and SHA-256 of `.git` files and SHA-256 of work files equal before and after. |
| Network through lazy fetch | rejected on git 2.53.0 | F6 probe (`file://` remote). Older git not tested. |
| Filter that cannot be overridden runs in a committed compare | rejected | Git-level probe with the app's diff flags and a required filter `a=b`: no program ran; the repo test checks the same through the app. |
| Ref or path injection, traversal, symlink, FIFO | rejected | `--end-of-options` plus id-only; listed path only; `:(literal)`; per-part `O_NOFOLLOW` open; tests and mutants. |
| Unauthenticated or cross-origin read | rejected | Guard test: no token, query token, wrong Host, wrong Origin all 403; POST/PUT/DELETE 405. |
| Absolute path or repo text in errors, storage or logs | rejected | `DiffError` fixed lines, git stderr discarded, no write or log call in the new code; `no-store`. |

Hardening notes (not findings): the config change between reading filter names and running the diff is accepted in DDR-0010. A stored workspace folder that is not a repo but lies inside one (a home dotfiles repo) shows the enclosing repo, untracked files included, to the owner (`diff_routes.rs:78`, code only).

### Process checks
- Entry point `/api/diff/` found in `diff_routes.rs`.
- `lane: C2, 14 files, 2251 lines, EXCEEDS (lines 2251 > 500); unpredicted: BRAINSTORMING_LOG_GITDIFF.md, PRD.md, PROJECT_SPEC.md, docs/ddr/0010-git-diff-view-show-never-store.md, rust/crates/app/src/api.rs, rust/crates/app/src/main.rs, rust/crates/app/src/transcript_route.rs, rust/crates/core/src/lib.rs, rust/crates/core/src/transcript.rs, rust/crates/core/src/workspaces.rs, tasks/TASK_GUIDE_T136.md`
- The skill's second-agent re-check of security leads was not done (single reviewer); no security lead is rated confirmed.
- No fix applied (the caller asked for review only).

### Mutation run
Each row: diff size of the mutant against `d3215f5` (`git diff --no-index --numstat`), then the test result.

| Mutant | Diff | Result |
|--------|------|--------|
| no filter overrides | `repo_diff.rs` +1 -3 | red |
| fsmonitor on | +0 -1 | red |
| hooks on | +0 -1 | survived (no index write, so no hook fires) |
| showSignature config on | +0 -1 | survived (`--no-show-signature` covers) |
| both signature guards off | +0 -2 | red |
| index refresh on | +0 -1 | red |
| optional locks on | +0 -1 | survived (index bytes still unchanged) |
| lazy fetch on | +0 -1 | survived (F6) |
| `GIT_*` inherited | +1 -1 | survived (F6) |
| external diff on | +1 -1 | red |
| textconv on | +1 -1 | red |
| submodules entered | +1 -1 | red |
| no `--end-of-options` in `resolve` | +0 -1 | survived (`^{commit}` suffix and id-only still refuse) |
| pathspec not literal | +1 -1 | red |
| symlink check off | +1 -1 | red |
| symlink check and safe open off | +2 -2 | red |
| listed-path check off | +2 -2 | red |
| stuck filter ignored | +0 -1 | red |
| sidechain folder taken | `transcript.rs` +0 -3 | red (route test) |

---

## Supervisor response (2026-11-26, after the review of `d3215f5`)

Tests first, then the fixes; the commit that holds this response needs its own reviewer run. `cargo test --workspace`:
1431 passed, 0 failed, 30 ignored; clippy `-D warnings` and `fmt --check` pass. Eight single-edit mutants (lazy fetch on,
`GIT_*` inherited, binary content check off, filters on in `hash-object`, folder excludes off, short default base, cut
flag ignored, untracked probe off) each fail a test.

| # | Result | What changed |
|---|--------|--------------|
| F1 | fixed | An unstaged binary file of unchanged mode is listed only if `hash-object` (filters off, nothing written) gives another id than the index holds. The planted-program test now covers that call |
| F2 | fixed | The list marks untracked binary files and counts the lines of text ones (1 MiB a file, 16 MiB a list; past that no count). PRD FR-139 states the caps |
| F3 | fixed | The overview answers `truncated` for cut branches, tags or work trees. The page shows it in T137 |
| F4 | fixed | A cut list keeps files whose counts did not come and drops an unfinished last name; the list cap is 16 MiB |
| F5 | fixed | The folder is read from the last 1, 4, then 16 MiB; never from the start of a longer file |
| F6 | fixed | Tests for a partial clone (nothing fetched) and for inherited `GIT_DIR`, `GIT_WORK_TREE`, `GIT_EXTERNAL_DIFF` |
| F7 | fixed | Paths are found by a map |
| F8 | fixed | Other listed files under a folder of the opened name are excluded from its patch |
| F9 | fixed | Shown names escape line breaks and tabs; the overview adds `base_shown` |
| F10 | fixed | The default base is resolved by its full ref |
| F11 | fixed | A folder with its own repository opens with a note |
| F12 | fixed | `filters_off` is true only where the work tree is read |
| F13 | fixed | `rust/README.md` section |
| F14 | fixed | The route test checks the planted change is not in the store files |

Hardening note on a stored folder inside an enclosing repository: not changed (not a finding; the owner sees their own
repository). Lane note: the diff is past C2's line guide; the level stays as written, and the task already runs at Risk
High.

---

## Stage 4 review, second run (2026-11-26, fresh-context reviewer, commit `1cc683b`)

**Verdict: fail.** One P0 (N1), new in this commit: the added `git hash-object` call can start a program named by a repo's config. AC6 and FR-140 are not met. The earlier findings are closed except for the remainders noted below. The Evidence rows above now describe `1cc683b`; the rows for `d3215f5` are summed up in the first review section. No source or test file changed.

Probes ran the real `repo_diff` functions (release build from `git archive 1cc683b` and `git archive d3215f5`, git 2.53.0) on scratch repos under the session temp folder.

### Acceptance criteria

| AC | Result | Basis |
|----|--------|-------|
| 1 | met | tests, mutants, probes (touched binary, 60,000 files) |
| 2 | met | tests |
| 3 | met for refusal; edges N3, N4 | tests, probes |
| 4 | met; ref name edge N5 | tests |
| 5 | met in the tested cases | route tests |
| 6 | **not met** | N1 |
| 7 | met | tests |
| 8 | met (what is answered is cut and says so); the read budget does not hold, N2 | tests, probe |

### P0
| ID | File:line | Finding and failure scenario | Confirmed by | Conf. | Action |
|----|-----------|------------------------------|--------------|-------|--------|
| N1 | `rust/crates/core/src/repo_diff.rs:715`, with `:498-499` | `hash-object` runs in `plan.top`, but the filter overrides are read in `dir` and the diff runs in `dir`. When `core.worktree` makes `toplevel(dir)` a folder that is its own repo, `hash-object` reads that other repo's config and attributes, and its clean filter is not overridden: it runs. Layout: repo `a` has `core.worktree=b`; repo `b` has `core.worktree=c` and tracks `pic.bin`; repo `c` defines `filter.evil.clean` and marks `pic.bin filter=evil`. The route takes `toplevel(a)` = `b`, then the plan takes `toplevel(b)` = `c`. Opening the uncommitted compare (list or file) runs the program as the owner. Plain `git status` in `a` runs nothing. The three `.git/config` files cannot come from `git clone`; a copied or unpacked folder can hold them. `d3215f5` runs nothing on the same layout. | Probe through `toplevel`, `worktrees`, `files` in the order `picked()` calls them (HTTP route not driven): marker written, parent command `git --no-pager -C .../chain/c ... hash-object -- pic.bin`; marker absent with the `d3215f5` build and with plain `git status` in `a`. | 100 | Add `--no-filters` to the `hash-object` arguments (checked with git on the same layout: no program ran). Also read the filters and make every call in one folder, or refuse when `toplevel(top)` is not `top`. Add this layout as a test that fails first. |

### P1
None.

### P2
| ID | File:line | Finding and failure scenario | Confirmed by | Conf. | Action |
|----|-----------|------------------------------|--------------|-------|--------|
| N2 | `repo_diff.rs:863-880` | The 16 MiB budget is not a bound. A binary untracked file of at most 1 MiB is read whole and returns before `*budget -= data.len()`. With up to 2,000 listed files one list reads about 2 GiB. FR-139 and `COUNT_BUDGET` say 16 MiB. No test holds the budget. | Probe: 600 untracked 1 MiB binary files, one list, `rchar: 629197271`. Mutant "budget never charged" survived. | 100 | Read the first 8,000 bytes, return if binary, then read the rest; test the budget. |

### P3
| ID | File:line | Finding | Confirmed by | Conf. |
|----|-----------|---------|--------------|-------|
| N3 | `repo_diff.rs:957-967` | One exclude pathspec per listed file under the opened name. A staged file replaced by a folder: 15,000 files took 3.4 s; 30,000 and 45,000 answered `TooSlow`, where `d3215f5` answered a cut patch (5.2 s, 7.8 s). When no own path is under the folder, one `:(exclude,literal)<name>/` does the same (checked with git). | probe | 100 |
| N4 | `repo_diff.rs:963` | A file name that is not UTF-8 under that folder refuses the opened file with "that file is not in this compare; refresh the list"; `d3215f5` answered its patch. The fix for N3 covers it. | probe | 100 |
| N5 | `repo_diff.rs:425-430` | Remainder of F10: with no `base` the full ref is used, but `/api/diff/repo` answers the short `base` (`main`), and a client that sends it back, or a branch name from `branches`, gets the tag of that name. The T137 page builds full refs, but sends the short name when the base is not among the listed refs (past the 500 cut). The README does not tell a client to send full refs. | probe (`main..HEAD` gave the tag's commit); T137 `ChangesView.jsx:181-187` read | 100 |
| N6 | `diff_routes.rs:353-357` | Remainder of F14: the new check scans only the two store files (and passes if one cannot be read); no log, MCP or CLI answer is checked. Code reading found no write or log path. | code | 100 |
| N7 | `repo_diff.rs:710-728` | The 10 s limit is per `hash-object` call of 500 paths, with no limit for the whole request, and every one-file open hashes all stat-changed binaries again. | code only | 75 |

### Earlier findings at `1cc683b`

| ID | State | Checked by |
|----|-------|------------|
| F1 | closed (the fix brings N1) | probe, test, mutants |
| F2 | closed (the fix brings N2) | tests, mutant |
| F3 | closed | test, mutants |
| F4 | closed | unit test, mutants; 60,000 files gave `total=60000` |
| F5 | closed | test, mutant |
| F6 | closed: both guards now have a test that fails without them | mutants |
| F7 | closed: 60,000 changed files listed in 1.5 s | probe |
| F8 | closed for the case found (side effects N3, N4) | test, mutant |
| F9 | closed | test, mutant |
| F10 | closed for the default; remainder N5 | test, probe |
| F11 | closed | test, mutant |
| F12 | closed | test, mutant |
| F13 | closed; its line "A repo's config cannot start a program" is untrue until N1 is fixed | read |
| F14 | partly closed (N6) | read |

### Security review

| Lead | Verdict | Evidence |
|------|---------|----------|
| Repo content or config starts a program (N1) | confirmed, Medium (code runs as the owner; needs a crafted folder and the owner opening it) | probe above |
| Extra `hash-object` call writes or fetches | rejected | no `-w`; `GIT_NO_LAZY_FETCH` set by `run`; code read. A `process` filter in the same repo did not run on a touched binary (probe) |
| Exclude pathspecs inject pathspec magic or options | rejected | after `--`, `:(exclude,literal)` prefix |
| Cut-list parse panics or lists half a name | rejected | unit test, two mutants |
| Untracked reads follow a link or block on a FIFO | rejected | same safe open as before, regular files only; existing test |
| Backwards transcript read answers an older folder | rejected | each window is the file's end; test and mutant |

### Process checks
- Entry point `/api/diff/` found in `diff_routes.rs`.
- `lane: C2, 15 files, 2689 lines, EXCEEDS (files 15 > 14; lines 2689 > 500); unpredicted: BRAINSTORMING_LOG_GITDIFF.md, PRD.md, PROJECT_SPEC.md, docs/ddr/0010-git-diff-view-show-never-store.md, rust/README.md, rust/crates/app/src/api.rs, rust/crates/app/src/main.rs, rust/crates/app/src/transcript_route.rs, rust/crates/core/src/lib.rs, rust/crates/core/src/transcript.rs, rust/crates/core/src/workspaces.rs, tasks/TASK_GUIDE_T136.md`
- `PRD.md` FR-139 gained caps in this commit (untracked counts, the 16 MiB transcript reach) after the first review.
- No second agent re-checked N1; it rests on the probe. No fix applied (review only).
- Not checked: N1 through the HTTP route; a layout delivered by `git clone` (embedded bare repos); N1 through a picked `tree`; older git; non-Linux; web and Python suites; `/verify`; easy-verifier.

---


## Supervisor response (2026-11-26, after the review of `1cc683b`)

All fixed in the next commit, tests first.

| # | State | Change |
|---|-------|--------|
| N1 | fixed | The plan reads the filters in the root of the work tree and runs every call there (`list`, `hash-object`, patch, untracked reads). `toplevel` answers only a root that is its own root, so a chain such as the reviewer's `a` is refused; `hash-object` also gets `--no-filters`. Probe with the new chain test's layout: the planted filter runs on `1cc683b` and not after the fix. Of the three measures, reading and running in one folder blocks it alone; `--no-filters` alone does not (the diff itself would run the filter) |
| N2 | fixed | An untracked file is probed first (8,000 bytes); only text that fits is read whole and charged. Unit test counts the bytes read; a list test pins 16 of 18 one-MiB files counted |
| N3, N4 | fixed | One `:(exclude,literal)<name>/` for the folder. Name by name only when the opened file's own other name is inside that folder |
| N5 | fixed | `repo` answers `base` as the full ref and `base_shown` short; the README says to send full refs. The page (T137) already takes a full `base` as it is |
| N6 | fixed | The route test scans every file under its temp folder except the two work trees, and requires the store to be among them |
| N7 | fixed | One deadline for all `hash-object` calls of a list; opening a file hashes that file only |

---

## Stage 4 review, third run (2026-11-26, fresh-context reviewer, commit `50870c356b50057f5b5a30fd73c1a91880187713`)

**Verdict: pass** for commit `50870c356b50057f5b5a30fd73c1a91880187713`. No P0, no P1. N1 (the P0) is closed: no layout tried made the app run a program, change a file's content or reach the network. Two P2 and five P3 findings are open. R4 needs the Supervisor's decision: it is a literal exception to FR-140's "no write" wording. No source or test file changed.

The Evidence rows above now describe `50870c3`. The rows for `1cc683b` are kept word for word at the end of this section. The `verify` row is untouched.

Probes: the real `repo_diff` functions built from `git archive 50870c3` (and `1cc683b` for comparison), and the worktree's debug binary (`serve --no-ingest`, scratch `HOME`) for the HTTP routes; git 2.53.0.

### Acceptance criteria

| AC | Result | Basis |
|----|--------|-------|
| 1 | met; edge R3 | tests, probes |
| 2 | met | tests, probes |
| 3 | met except one case, R2; refusal met | tests, probes |
| 4 | met; edge R5 | tests, HTTP probe |
| 5 | met | route tests, HTTP probe (workspace, session, tree, folder parameter refused) |
| 6 | met; timestamp edge R4 | tests, mutants, hostile probes |
| 7 | met | tests, HTTP probe (option-like refs, unlisted and traversal paths) |
| 8 | met | tests, mutants |

### P0 and P1
None.

### P2
| ID | File:line | Finding and failure scenario | Confirmed by | Conf. | Action |
|----|-----------|------------------------------|--------------|-------|--------|
| R1 | `rust/crates/core/src/repo_diff.rs:1011` (also `:807`, `:1003`) | No test holds that the patch call runs in the root where the filters were read, which is what closes N1. The code is correct. With `run(&plan.top, ...)` changed to `run(dir, ...)` every test stays green, and on a repo `b` with its own clean filter and `core.worktree=c` that mutant runs the filter. The same change to the two untracked reads also survives (wrong folder read, no program). Rated P2, not P1: the behaviour holds today and AC6 has tests. | mutants; probe (marker absent on the commit, written by the mutant) | 100 | Add that layout (filter in `b`'s config, `.gitattributes` in `c`) to the chain test and open a patch through `b`; add an untracked file in `c`. |
| R2 | `repo_diff.rs:983` | New in this commit (N3 fix). When a folder is swapped with a submodule of the same name, the submodule's entry opens as an empty patch with no note, though the list shows +1 or -1. `:(exclude,literal)a/` also matches the gitlink `a`. Seen for staged (both directions) and for a commit compare (folder to submodule). `1cc683b` showed the `Subproject commit` line for both staged cases. | probe (staged on both commits); plain git with the same two pathspecs prints 0 bytes | 100 | For a submodule entry (mode 160000) leave the others out name by name; add the swap as a test. |

### P3
| ID | File:line | Finding | Confirmed by | Conf. |
|----|-----------|---------|--------------|-------|
| R3 | `repo_diff.rs:727` | New in this commit (`--no-filters`). With `core.autocrlf=true`, files marked `-diff` (alone, with `text eol=crlf`, with `ident`) whose time alone changed are each listed `unstaged / modified / binary` with an empty patch; `git status` is clean and `1cc683b` listed nothing (F1 again, for these files). With `--no-filters` removed the same probe lists nothing; no test needs that flag (mutant survived), and the filter overrides already apply to the call. Add the case as a test. | probe on both commits and on the edit | 100 |
| R4 | `repo_diff.rs:137-160` | In a repo with `core.splitIndex`, git updates the timestamp of `.git/sharedindex.<id>` on every read (`utimensat`, to keep it from expiring). Content, size and `.git/index` are unchanged, so AC6 holds; FR-140 "no diff route writes to a repo (its index included)" is not literally true here. The same git calls are in `d3215f5` (not re-run there). Accept and say so in DDR-0010 or FR-140, or decide to change it. | probe through the core; `strace` of `git diff` | 100 |
| R5 | `repo_diff.rs:275-326` | git names the main work tree of a `--separate-git-dir` repo and of a submodule by its git directory. The answer's `tree` is then null; for the separate-git-dir repo, picking that listed tree answers "this folder is not a git work tree". Not new in this commit (code read for earlier commits). | probe | 100 |
| R6 | `repo_diff.rs:333` | N5 side effect: a base ref of more than 200 characters is refused as an unknown ref when the page sends the answered full `base` back (with no `base` it was already refused). Contrived names only. | probe (211 characters) | 100 |
| R7 | `rust/crates/app/src/diff_routes.rs:353-372` | Remainder of N6: the test scans files only; the app's stdout and stderr, MCP and CLI answers are still checked by no test. This review's HTTP probe found the planted text in no server output and no file under the scratch `HOME`. | code, probe | 100 |

### Earlier findings at `50870c3`

| ID | State | Checked by |
|----|-------|------------|
| N1 | closed. The chain, a chain with a filter in every repo, a cycle, a chain through a subfolder and a symlink, a relative move to a plain folder, `.git` files, crafted `.git/worktrees` entries and an embedded bare repo ran nothing. Mutants "filters read in the asked folder", "no second root check" and "unchecked root in `work_tree`" are red. Test gap R1 | probes, HTTP probe, mutants |
| N2 | closed: both budget mutants and "whole file read before the binary check" are red | tests, mutants |
| N3 | closed for the case found: a file swapped for 30,000 files answered its own patch in 2.6 s. Side effect R2. A rename into a folder of the old name still excludes name by name (code read) | probe, mutants |
| N4 | closed | test, mutants |
| N5 | closed: full `base` answered, short `base_shown`; the T137 page takes a `refs/` base as it is (`ChangesView.jsx:193`). Edge R6 | tests, mutant, read |
| N6 | closed for files; remainder R7 | read, probe |
| N7 | closed: one deadline across calls, and opening a file hashes only that file. A call that starts just before the deadline can still run its own 10 s (code read); "no shared hash deadline" survives the tests | mutants, read |
| F1 to F14 | no regression seen in the tests; F1 returns only as R3 | tests |

### Security review

| Lead | Verdict | Evidence |
|------|---------|----------|
| Repo content or config starts a program | rejected | 22 layouts: odd filter names, `process` filters, plain and conditional includes (`gitdir`, `gitdir/i`, `onbranch`, `hasconfig`, relative), per-work-tree config, external diff, driver command, textconv, fsmonitor, hooks and config hooks, pager, editor, ssh, askpass, gpg, trace2, submodule config, global config, stuck filter names, 30,000 filters (every compare then fails closed), root commit. Under `strace`, only `git` was executed |
| A diff route writes to a repo | rejected for content (timestamp edge R4) | size, time, inode and SHA-256 snapshots; no write open but `/dev/null`; no rename, unlink or mkdir |
| A diff route uses the network | rejected on git 2.53.0 | blobless and treeless partial clones with a planted `uploadpack` and an `ext::` promisor remote: nothing ran, no `AF_INET` connect |
| Ref or path injection, traversal, guard bypass | rejected | HTTP probe: option-like refs 400, unlisted / absolute / `..` / `.git/config` / magic paths 404, no token / query token / bad Host / bad Origin 403, POST 405 |
| Diff text kept by the app | rejected for what was probed | no planted text in server output or any file under the scratch `HOME`; answers `no-store` |
| Crafted config exhausts memory (S1) | confirmed, Low; outside FR-140's three rules; not new | `diff.orderFile=/dev/zero`: through the core, git reached 1.3 GB in 0.4 s under a 1.5 GB `ulimit -v` and the answer was `Failed`. Without a cap it would go on until the 10 s limit (not run). `-c diff.orderFile=/dev/null` stops it (checked with git). Plain `git diff` does the same. Severity P3 |
| A crafted `.git/worktrees` entry shows another of the owner's repos | rejected as a boundary crossing | listed and readable by `tree` id, with that repo's filters off; only the owner sees it |

### Process checks
- Entry point `/api/diff/` found in `diff_routes.rs`.
- `lane: C2, 15 files, 2894 lines, EXCEEDS (files 15 > 14; lines 2894 > 500); unpredicted: BRAINSTORMING_LOG_GITDIFF.md, PRD.md, PROJECT_SPEC.md, docs/ddr/0010-git-diff-view-show-never-store.md, rust/README.md, rust/crates/app/src/api.rs, rust/crates/app/src/main.rs, rust/crates/app/src/transcript_route.rs, rust/crates/core/src/lib.rs, rust/crates/core/src/transcript.rs, rust/crates/core/src/workspaces.rs, tasks/TASK_GUIDE_T136.md`
- No forbidden file in the diff (store schema, ingest, `src/easy_personal/`, `.claude/`, `skills/`).
- No second agent re-checked the security leads (single reviewer). No fix applied (review only).
- Not checked: git other than 2.53.0 (lazy-fetch switch on older versions, config-based hooks on newer ones); non-Linux and case-insensitive file systems; a config change between reading filter names and the diff (accepted in DDR-0010); layouts delivered by `git clone`; R2, R3, R5 and R6 through HTTP (core only); web and Python suites; `/verify`; easy-verifier.

### Mutation run
Single edits of `repo_diff.rs` on a scratch copy, then `--lib repo_diff` and `--test repo_diff`. A first batch hit a full temp-folder quota after its eighth mutant; the other ten were run again.

| Mutant | Result |
|--------|--------|
| filters read in the asked folder | red |
| no second root check in `toplevel` | red |
| `work_tree` uses the unchecked root | red |
| opening a file asks about no binary | red |
| hash deadline already past | red |
| budget never charged | red |
| whole file read before the binary check | red |
| budget ignored | red |
| excludes always name by name | red |
| excludes always the whole folder | red |
| own names not left out | red |
| `base` answered short | red |
| `hash-object` without `--no-filters` | survived (the other two measures hold) |
| untracked count read in the asked folder | survived (R1) |
| untracked patch read in the asked folder | survived (R1) |
| patch run in the asked folder | survived (R1) |
| opening a file hashes every binary | survived (speed only) |
| no shared hash deadline | survived (speed only) |

### Evidence rows at `1cc683b` (replaced above, kept as written)

> | **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☑ fail | At `1cc683b`: `rust/crates/core/tests/repo_diff.rs` (28 tests, 1 of them run as a child process), `rust/crates/core/src/repo_diff.rs` `mod tests` (1 test), `rust/crates/app/src/diff_routes.rs` `mod tests` (4 tests). Gaps: AC6 (no test with a work tree root that is another repo, N1); the 16 MiB read budget has no test (N2). |
> | Verification command run | ☑ pass / ☐ fail | At `1cc683b`, `cd rust && cargo test -p easy-personal-core --test repo_diff && cargo test -p easy-personal-rs diff_routes`: `test result: ok. 27 passed; 0 failed; 1 ignored` and `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 479 filtered out` |
> | Negative cases hold | ☐ pass / ☑ fail | At `1cc683b` a program named by repo config ran during the uncommitted compare (N1; marker file written, command line `git ... -C <c> ... hash-object -- pic.bin`). 16 single-edit mutants on a scratch copy: 15 red, 1 survived ("count budget never charged"). |
> | Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass / ☐ fail | At `1cc683b`: `main...1cc683b` and `d3215f5..1cc683b`, the callees `clean_block`, `report_files::sys`, `transcript::last_folder`, and what the T137 page sends as a ref. Skipped: web, Python app, ingest, store schema (unchanged in this diff). |
> | Full smoke suite still green (no regression) | ☑ pass / ☐ fail | At `1cc683b`, `cargo test --workspace --no-fail-fast` exit 0: 100 suites, 1431 passed, 0 failed, 30 ignored. `cargo clippy --workspace --all-targets -- -D warnings` exit 0; `cargo fmt --all -- --check` exit 0. Web and Python suites not run (no change there). |

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (no diff route exists):

```
$ date -u +%FT%TZ; git rev-parse --short HEAD; grep -rn '/api/diff' rust/crates web/src; echo "grep exit=$?"
2026-11-26T05:35:57Z
2a491e5
grep exit=1
```

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
