# TASK_REVIEW — HUN-245: Dashboard button that copies a Claude Code command

> Sibling of `tasks/TASK_GUIDE_HUN-245.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` tag,
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

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its option>; Advocate 2: <its option>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours)>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/CopyCommand.test.jsx` (33 tests: `shellLine` byte-exact cases and refusals; allow-list; component 0/1/2 repos, each entry copies its exact line; refused/missing clipboard fallback; no fetch / no localStorage / no console on Copy; two blocks independent). Mutation check: removing the `\'` escaping and the control-char refusal made 10 of them fail. |
| Verification command run | ☑ pass | `npm --prefix web test -- --run` → `Test Files  26 passed (26)`, `Tests  871 passed (871)`; `npm --prefix web run build` → `✓ built in 641ms`; `python3 scripts/check_static.py` → `check_static: ok`. |
| Negative cases hold | ☑ pass | Path with \n, \r, NUL, tab, ESC, U+001F, U+007F → `shellLine` returns null, no Copy button, fixed note; clipboard missing / no `writeText` / rejects / throws → read-only selected input + "Copy it with Ctrl+C."; 0 repos → no block. |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Real server (Rust `serve --no-ingest --static-dir`, temp store): `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/9a293e6d-1db7-42ec-9315-2625f8ddcdc7/scratchpad/hun245/before-1280x720-before.png` vs `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/9a293e6d-1db7-42ec-9315-2625f8ddcdc7/scratchpad/hun245/after-1280x720-after.png`, `after-1280x720-after-copied.png`; mobile `before-390x844-before.png` vs `after-390x844-after.png`, `after-390x844-after-copied.png`; fallback `extra-fallback.png`, `extra-fallback-390.png`; keyboard focus `extra-keyboard-focus.png` (all in `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/9a293e6d-1db7-42ec-9315-2625f8ddcdc7/scratchpad/hun245/`). Clipboard read back in headless Chrome for 3 commands x 2 repos at both sizes, e.g. `cd '.../it'\''s a repo with space' && claude "/wake"`; the two lines were run in `sh` with a fake `claude` on PATH and it started in the right folder (`claude ran in: .../it's a repo with space args: /wake`). |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Only existing tokens (`--color-control`, `--color-input`, `--color-divider`, `--color-muted`, `--color-secondary`, `--space-*`, `--fs-*`, `--radius-sm`, `--font-mono`) in a new `/* HUN-245 */` block at the end of `web/src/app.css`; Copy uses the existing `main button` style; focus ring is the global `:focus-visible` (computed `solid 2px rgb(60, 245, 168)`). No new colour. |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280x720 and 390x844: `scrollWidth - clientWidth` = 0 px at both, also with the fallback field open; the block wraps (Command, Repo, Copy stack at card width). |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-15T05:29Z, base commit `4c12530`, before any implementation commit; Rust `serve --no-ingest --static-dir` on a temp store holding workspaces `company` (2 repos, one path with a space and a `'`), `solo` (1 repo), `empty` (0 repos)):

```
$ grep -c 'Copy a Claude command' dist-before/assets/*.js      # build of the unmodified worktree
0
$ grep -c 'claude --continue' dist-before/assets/*.js
0
```

Screenshots of the Workspaces page today, no copy block on any card (overflow 0 px at both sizes):
- `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/9a293e6d-1db7-42ec-9315-2625f8ddcdc7/scratchpad/hun245/before-1280x720-before.png`
- `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/9a293e6d-1db7-42ec-9315-2625f8ddcdc7/scratchpad/hun245/before-390x844-before.png`

**AFTER** (2026-11-15, final build, same temp store and server, headless Chrome over the DevTools protocol; script `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/9a293e6d-1db7-42ec-9315-2625f8ddcdc7/scratchpad/hun245/tools/cdp.mjs`, `extra.mjs`):

```
$ grep -c 'Copy a Claude command' dist-after/assets/*.js
1
1280x720 horizontal overflow px: 0        390x844 horizontal overflow px: 0
Start a session            -> status "Copied", clipboard: cd -- '<tmp>/repos/plain' && claude "/wake"
Continue the last session  -> status "Copied", clipboard: cd -- '<tmp>/repos/plain' && claude --continue
Verify                     -> status "Copied", clipboard: cd -- '<tmp>/repos/plain' && claude "/verify"
(same three for the repo "it's a repo with space": cd -- '<tmp>/repos/it'\''s a repo with space' && claude ...)
focused after 2 Tabs: BUTTON Copy   (Command select -> Repo select -> Copy); Enter -> status "Copied"
new resource requests after Copy: 0 ; localStorage keys: 0
fallback (writeText rejected): readonly=true selected=true focused=true note "Copy it with Ctrl+C."
```

Screenshots: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/9a293e6d-1db7-42ec-9315-2625f8ddcdc7/scratchpad/hun245/after-1280x720-after-copied.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/9a293e6d-1db7-42ec-9315-2625f8ddcdc7/scratchpad/hun245/after-390x844-after-copied.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/9a293e6d-1db7-42ec-9315-2625f8ddcdc7/scratchpad/hun245/extra-fallback.png`.

**Amendment 2026-11-15 (Supervisor decision): line format is now `cd -- '<repo>' && <command>`** so a repo path starting with `-` is never read as a `cd` option.
- RED first (2026-11-15T06:13:52Z, current code, new tests for `-x` and `--help`): `Expected: "cd -- '-x' && claude "/wake""  Received: "cd '-x' && claude "/wake""` (`Tests  1 failed | 32 skipped`). Then code and all expected strings changed: `Test Files  26 passed (26)`, `Tests  871 passed (871)`; build `✓ built in 641ms`; `check_static: ok`.
- Real browser: Copy on the `company` card puts `cd -- '<tmp>/repos/it'\\''s a repo with space' && claude "/wake"` (and `--continue`, `"/verify"`, and the `plain` repo) on the clipboard; overflow 0 px at 1280x720 and 390x844.
- Real `sh`, fake `claude` on PATH, folders `-x` and `--help` under `$S/repos/`, line produced by the real `shellLine` source text:
```
cd -- '-x' && claude "/wake"        -> claude ran in: $S/repos/-x args: /wake
cd '-x' && claude "/wake" (old)     -> sh: 1: cd: Illegal option -x
cd -- '--help' && claude "/wake"   -> claude ran in: $S/repos/--help args: /wake
cd '--help' && claude "/wake" (old) -> sh: 1: cd: Illegal option --
cd -- '<tmp>/repos/it'\\''s a repo with space' && claude "/wake" -> claude ran in: <tmp>/repos/it's a repo with space args: /wake
```
- Finding: the store refuses a repo path that is not absolute or `~`-prefixed (`easy-personal: repo path must be absolute or start with ~`; a `~` path is expanded to the home folder when stored), so a `-x` repo cannot reach the page today. The `--` is defence in depth; the `-x` case was therefore checked through `shellLine` and `sh`, not through a stored workspace.

**Amendment 2 (Stage 4), 2026-11-15: wider refused set, refusal announced, new note texts**
- Refused now (`HIDDEN` in `web/src/CopyCommand.jsx`): U+0000-U+001F, U+007F-U+009F, U+200B-U+200F, U+2028, U+2029, U+202A-U+202E, U+2066-U+2069, U+FEFF. Reason: bidi and zero-width characters make the shown path differ from the copied bytes.
- Notes: "This repo path has a hidden or control character; copy is off." and, for an empty path, "This repo has no path; copy is off." Both now sit in the existing `aria-live="polite"` status `<p>` (the separate note `<p>` is gone), so a change of the repo select announces them.
- RED first (2026-11-15T08:16:42Z, code unchanged): `Tests  16 failed | 32 passed (48)` — 12 range tests (`refuses a path with a hidden character: C1 start U+0080` ... `byte order mark U+FEFF`) plus the 4 note tests (`offers no Copy for a repo path with a control character, and says why`, `... bidi or zero-width character`, `says it has no path ... for an empty repo`, `announces the refusal in the polite live region ...`).
- After the change (2026-11-15T08:17:09Z):
```
npm --prefix web test -- --run   → Test Files  26 passed (26)   Tests  886 passed (886)
npm --prefix web run build       → ../src/easy_personal/dashboard/static/assets/index-CDVgWusl.js  349.36 kB │ gzip: 104.69 kB
                                   ✓ built in 637ms
python3 scripts/check_static.py  → check_static: ok
```
- Layout unchanged (the note moved into the existing status line), so no new browser check.

**DELTA**: A workspace card with repos now has a block where the user picks one of three fixed Claude commands (and a repo), and copies one shell-safe line to paste into their own terminal; the page runs nothing.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
