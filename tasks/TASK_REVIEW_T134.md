# TASK_REVIEW — T134: Terminal 2 — Terminal choice and the xterm panel (page)

> Sibling of `tasks/TASK_GUIDE_T134.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/TermPanel.test.jsx` (30 tests: AC 2, 3, 4, 5), `web/src/LiveTerm.test.jsx` (10: AC 6, start-to-panel), `web/src/NewSession.test.jsx` (+10: AC 1), `web/src/NewSessionSheet.test.jsx` (+4: Terminal offered only with the route, one cap), helper `web/src/test/fakeXterm.js` (fake Terminal / FitAddon / WebSocket / ResizeObserver), `web/src/test/appSessions.js` (`terms`, `termSession`). AC 7 = build + `check_static.py`. AC 8 and the real-server part of AC 5: waiting on T133 (see Demonstration). |
| Verification command run | ☑ pass | `cd web && npm ci && npm test -- --run && npm run build && cd .. && python3 scripts/check_static.py` at 2026-11-22T15:02:51Z: `found 0 vulnerabilities`; `Tests  1610 passed (1610)` (48 test files, 54 of them new: 30 + 10 + 10 + 4; no existing test was changed or removed); build `✓ built in 290ms`, lazy chunk `assets/TermPanel-Bj9yDdaC.js 337.14 kB │ gzip: 85.43 kB` + `TermPanel-BrP-ENHg.css 1.01 kB gzip`, entry `index-B7eL-4YZ.js 473.05 kB │ gzip: 139.79 kB`; `check_static: ok` (exit 0; no change to `ALLOWED_URL_PREFIXES` or `vite.config.js` was needed: Vite drops the comment URLs and `sourcemap: false`). |
| Negative cases hold | ☑ pass | Red first: `npx vitest run src/TermPanel.test.jsx` before `TermPanel.jsx` existed failed to import it (0 tests ran). Mutation: in `TermPanel.jsx` keep the input on after exit (`live = link === 'open'` and `send` without `!endedRef.current`): `FAIL src/TermPanel.test.jsx > the end and the pages (AC 4) > an exit frame says "Ended (code N)", turns the input off ...`, `Tests  1 failed \| 29 passed (30)`; restored: `Tests  30 passed (30)`. Negative tests: token not in the URL, nothing sent before open, nothing typed after exit, malformed text frame ignored, no storage/console, 404 server hides the choice, refused start/Stop shown. |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | Whole web suite 1610 / 1610 (all 48 files), Chat panel files (`LiveAppPanel`, `LiveAnswer`, `LivePanel`, `LiveSteer`, `NewSessionSheet`, `NewSession`) unchanged and green. Python smoke / Rust suites not run (no `rust/**` or `src/**` change). |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Real Chrome (headless, real built page) against a scratch stand-in for T133's routes (not in the repo): screenshots in `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/52d65067-54e0-48e5-831f-372e5cc75bee/scratchpad/ux` (1-form-chat, 2-form-terminal, 3-confirm, 4-panel, 5-typed, 5b-lost, 5c-reattached, 6-ended at 375 and 1280). The Chat form screenshot shows today's form plus the Chat/Terminal switch. See the ux-evaluate verdict table in the report. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Owner choice 2026-11-22 "Use existing styles": the panel reuses `session-head`, `session-facts`, `session-state`, `periods` (the Live/Past switch), `prompt-cut`, `danger`, `field-error`; new CSS in `appsession.css` uses tokens only (`--space-*`, `--color-*`, `--radius-sm`, `--font-mono`); xterm theme reads `--color-bg`, `--color-text`, `--color-accent`. |
| **UI: Responsiveness at target viewports** | ☑ pass | 375x800 and 1280x1000: `document.documentElement.scrollWidth` = 375 at 375 (no horizontal scroll); key buttons 69x44 at 375 (8 keys, two rows of four), Back 157x44, Stop 65x44, form buttons 44 high; at 1280 the open terminal takes the full Live column (81 columns). Terminal cells are 13 px by design (see report: baseline B-e). |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-22T09:08:56Z on `2f0faca`, no implementation commit yet; the guide names no Demonstration command, so this is the absence check the Supervisor named):

```
$ grep -rniE "TermPanel|@xterm|term-sessions" web/src web/package.json | wc -l
0
$ grep -nE "Terminal|Chat" web/src/NewSession.jsx web/src/NewSessionSheet.jsx | wc -l
0
$ grep -c xterm web/package-lock.json
0
```

The New session form has no Terminal / Chat choice, `web/` has no `TermPanel` and no `@xterm` dependency.

**AFTER** (page against a scratch stand-in for T133's routes; real Chrome and real WebKitGTK webview; the real `serve` is waiting on T133):

```
$ grep -rniE "TermPanel|@xterm|term-sessions" web/src web/package.json | wc -l   -> files now name them (TermPanel.jsx, TermPanel.test.jsx, LiveTerm.test.jsx, api.js, package.json ...)
$ grep -n '"@xterm' web/package.json
    "@xterm/addon-fit": "0.11.0",
    "@xterm/xterm": "6.0.0",
Chrome 1280x1000 and 375x800 (stand-in server log): AUTH {'t': 'auth', 'cols': 81, 'rows': 35} token-len 16   (cols 36 rows 23 at 375; the URL carries no token)
  KEYS b'ls -la' / b'\r' / b'\x1b[A' / b'\x1b' / b'\x03'  (typed text, Enter, Up, Esc, Ctrl-C from the key row), then the exit frame -> "Ended (code 130)", every key disabled
  a dropped socket -> "Not attached" + "Disconnected ... Reconnect"; Reconnect -> a second AUTH frame and the replay drawn once on a cleared screen
WebKitGTK 4.1 webview under Xvfb, same CSP header as the dashboard (default-src 'self'): xterm drawn true; screen text "  ✻ Welcome to Claude Code  cwd: /work/company"; status "Running"; AUTH cols 83 rows 35; key row Enter and Up reached the server; terminal focused after a key-row press true
```

**DELTA**: New session offers Terminal beside Chat, and a started terminal session opens in Live as an xterm panel (with a key row for phones, Stop, and re-attach) instead of only Chat cards.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
