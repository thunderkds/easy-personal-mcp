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

## Auto decisions

Mode: auto
1. The panel uses the existing Live / SessionPanel / appsession.css styles, no new design — RULE: owner answer 2026-11-22 ("Use existing styles"), which replaces the guide's ask-for-a-design step
2. Built in parallel with T133 against the route contract in `tasks/TASK_GUIDE_T133.md`, tested with a fake WebSocket; checked against T133's committed routes before merge (list fields, POST fields, frames all match, re-review at `63dd426`) — RULE: owner request 2026-11-22 ("start the parallel session if possible")
3. A failing `GET /api/term-sessions` (any error, not only 404) hides the Terminal choice and leaves Chat working (`web/src/NewSessionSheet.jsx:12`) — RULE: `tasks/TASK_GUIDE_T134.md:30` "Chat default, today's form unchanged"
4. The session list stays visible beside an open terminal (the three hiding rules and `data-term` removed, `b3a2aea`) — RULE: owner answer 2026-11-22 ("Keep the list visible"); cost measured by the re-review: the terminal is 44 columns at 1280 px (81 before), so Claude Code's wide screens wrap
5. Terminal text stays 13 px, below the 16 px baseline (B-e note) — RULE: owner answer 2026-11-22 ("Keep 13 px")
Checks: tests `cd web && npm ci && npm test -- --run && npm run build && cd .. && python3 scripts/check_static.py` — 1611 passed (1611), build ok, `check_static: ok`; reviews code-review — 0 P0/P1/P2, 3 P3 (below the fold at 375, polling a 404 route, keys before open dropped); security-review — no findings; reviewer spawns `Reviewer: T134` on `3122a17`, `63dd426`, `b3a2aea` — PASS each time; real run built page against a stand-in for the T133 routes in Chrome and WebKitGTK (auth frame sent, typed bytes arrived, Ended (code 130)) (auto-checked, not /verify)
Memory: memory/decisions.md — 2026-11-22 T134 terminal page: existing styles, list visible beside the terminal, 13 px (owner)
Not done: merge (admin: after the auto verify pass; the `[admin]` request must name T134); auto verify: running (T182) — the pass on `8fcf0b8` predates `b3a2aea`

## Review findings (independent reviewer, 2026-11-22, commit 3122a17)

Scope: `main...3122a17`, bounded to the blast radius: `web/src` files in the diff plus `LiveView` callers, `scripts/check_static.py`, the built bundle. T133 is not merged (its worktree is uncommitted work in progress), so nothing here ran against the real routes. `Skill` was not callable in this reviewer's context: `code-review`, `ux-evaluate` and `security-review` (no `skills/security-review/` in the repo) were applied by hand from `skills/code-review/SKILL.md` and `skills/ux-evaluate/SKILL.md`.

Verdict at 3122a17 (superseded by the re-review above): **PASS with findings. No P0, no P1. The merge of the page can go ahead once the owner accepts the two P2 items or has them fixed; AC 5 and the real-server half of AC 8 wait on T133.**

### Re-review (independent reviewer, 2026-11-22, commit 63dd426)

Scope: `3122a17..63dd426` (3 source files: `LiveView.jsx` comment, `NewSessionSheet.jsx` one line, `NewSessionSheet.test.jsx` one test) plus the full `main...63dd426` diff re-read for blast radius. `Skill` was not callable here: `code-review`, `ux-evaluate` and `security-review` were applied by hand from `skills/code-review/SKILL.md` and `skills/ux-evaluate/SKILL.md` (no `skills/security-review/` in the repo). `ux-evaluate`: the fix commit changes no markup or CSS, so the 3122a17 verdict table below stands unchanged (B-a still fails, finding 3).

Verdict: **PASS. No P0, no P1, no P2 open. The merge can go ahead** (the page side; AC 5 and the real-server half of AC 8 still need `/verify` against `serve` after T133 is merged).

| # | Status at 63dd426 | Evidence |
|---|---|---|
| 1 (P2) | **fixed** | `NewSessionSheet.jsx:12` now `getTermSessions().catch(() => null)`. Mutation: I put the old call back, `npx vitest run src/NewSessionSheet.test.jsx` -> `FAIL ... a failing terminal list (500) leaves the Chat form working, with no Terminal choice`, `1 failed \| 14 passed`; restored with `git checkout`, tree clean. The only other caller (`LiveView.jsx` `useTerms`) already had its own try/catch. |
| 2 (P2) | **closed: contract matches** | T133 is now committed (`T133` head `d7388b4`). `term_session.rs:350 json()` returns `id, repo, branch, account {id,label}|null, started, state "running"|"exited", code, reason, pages, session_id`; `term_routes.rs list` returns `{sessions, max}` -> `getTermSessions` reads `list.sessions`, `isExited` reads `state === 'exited'`, `accountLabel` reads `.label`, `termOnly` reads `repo, session_id, code`. POST `{repo, workspace, account?, model, mode, worktree?, confirmed}` and `write_body` rejects any other field: `NewSession.jsx` sends exactly those (no prompt; test asserts the body). 201 `{id}`. Stop `POST /api/term-sessions/{id}/stop {confirmed}` -> 200 term json, 404 "no such session". WebSocket: first frame text `{t:'auth', token, cols, rows}` within 5 s (`AUTH_WAIT`), page sends it in `onopen`; binary in = typed bytes (page sends `Uint8Array`); text `{t:'resize', cols, rows}` (page sends it, debounced); out: binary screen, `{t:'exit', code}` (code may be null -> "Ended"), `{t:'pages', n}`, `{t:'notice', text}` (-> dropped line); `RecvError::Lagged` closes the socket -> page shows "Not attached" + Reconnect, a new attach replays. Token not in the URL (server never reads it). Max frame 1 MiB: key row sends 1-3 bytes. No mismatch. (Page-local tolerance for `running === false` and a bare list is dead code against T133: harmless.) |
| 3 (P3) | open | Terminal below the fold after Start (owner decides the design). |
| 4 (P3) | **fixed** | Comment now says "about 85 KB gzip", matching the build (85.43 kB). |
| 5 (P3) | open | `useTerms` keeps polling a server without the route (T133 serves it, so only the Python server or an old app hits it). |
| 6 (P3) | open | Dropped input before open is silent while "Connecting" shows; xterm screen-reader mode off. Not an AC. |

New findings in the fix commit: none. The change is one `.catch` and one comment; the test added is red without the fix (above).

Commands run (this re-review):

```
$ cd web && npm ci && npm test -- --run && npm run build && cd .. && python3 scripts/check_static.py
found 0 vulnerabilities
Tests  1611 passed (1611)          (48 files; the +1 is the new 500 test)
TermPanel-hSWxPZkd.js  337.14 kB | gzip: 85.43 kB   index-DEWu3EJ3.js  473.06 kB | gzip: 139.79 kB   built in 199ms
check_static: ok
$ git status --short   -> clean (the build output is not tracked)
```

Security review (Medium, manual), re-checked on the full diff and against T133: no finding above P3. The auth frame carries the token and the server compares it in constant time; the page never puts it in a URL, storage or console; Stop sends `confirmed: true`; no HTML is built from server bytes; no link or clipboard addon; CSP and `rust/**` untouched; two pinned MIT dependencies, 0 vulnerabilities; the built `TermPanel` chunk holds no `http(s)://` URL. Residual (owner-accepted, FR-137): every attached page can type.

### Re-review 2 (independent reviewer, 2026-11-22, commit b3a2aea)

Scope: `63dd426..b3a2aea` is two files (`web/src/LiveView.jsx` drops the `data-term` attribute, `web/src/appsession.css` drops three rules: the grid override, the `.live-sessions { display: none }` and the forced Back button). Blast radius: every reader of `data-term` or of those selectors (`grep -rn "data-term\|dataset.term" web/src src README.md docs`: none left, no test read them), the Chat panel (it shares `.live-split`, `.session-back`), the narrow layout. `Skill` was not callable in this reviewer's context: `code-review` and `ux-evaluate` were applied by hand from `skills/code-review/SKILL.md` and `skills/ux-evaluate/SKILL.md`. Real Chrome 1280x1000, 375x800 and (extra) 1600x1000 against the built page and a scratch stand-in for the T133 routes (not in the repo), two terminal sessions listed, the second opened, key row and Stop measured; screenshots in the reviewer scratchpad `.../scratchpad/rr2/g-open-1280.png`, `h-keys-1280.png`, `g-open-375.png`, `h-keys-375.png`, `i-ended-*.png`.

Verdict: **PASS. No P0, no P1. One P2 (owner-decided trade-off, see 1) and two P3. The merge can go ahead**; AC 5 and the real-server half of AC 8 still need `/verify` against `serve`.

| # | Sev | Finding |
|---|-----|---------|
| 1 | P2 | **The terminal is now narrow beside the list.** The split is the `@container live (min-width: 40rem)` rule: list 2fr, panel 3fr. Measured (fit, the cols the page sent in its auth frame): 1280 window, Live column 673 px, box 362 px, **44 columns**; 1600 window, box 466 px, **57 columns**; before this commit the open terminal had the whole column (81 columns at 1280, same stand-in). The removed CSS comment named the reason ("Claude Code's screen is drawn for 80 columns"). It is the owner's choice (commit message, 2026-11-22) and 13 px stays, so it is not a defect to fix here; the cost is that Claude Code's welcome and trust screens wrap below about 60 columns and a real window needs roughly 2000 px of screen to reach 80. Options for the owner if it bites: let the panel take 2fr/3fr the other way for a terminal only, or hide the right-hand cards column while a terminal is open. No test or AC names a column count. |
| 2 | P3 | **Key row at 1280 is 5 + 3 with a wrapped label.** The grid is `auto-fit, minmax(4rem, 1fr)`: in the narrower box (362 px) the keys are 66x44 and `Ctrl-C` breaks to two lines ("Ctrl-" / "C"), the arrows fall to a second row. Readable and tappable (44 px high), not wrong; before, at 1280 it was one row of eight at 74x44. A `min-width` on the Ctrl-C key or `white-space: nowrap` with a 5rem minimum would keep it one word. |
| 3 | P3 | **Stop is below the fold at 1280x1000.** The panel is 938 px high from y=403 (below the "All sessions" card), so with the page unscrolled the box ends at 1168 and Stop is at y=1288 (key row at 1180). Same as the first review's finding 3 (open P3, owner decides the design); the new split does not make it worse. At 375 the same applies (box top y=813). |

Not defects, checked: (a) **at 375 the list is not visible beside an open terminal, and that is by design**: below 40 rem of Live column the list is `display: none` while any panel is open and "Back to sessions" (157x44) returns to it, exactly like the Chat panel and FR-128 ("list, then panel full width with Back"); measured `listDisplay none, back block, scrollWidth 375`. The request to see the list at 375 is met only through Back. (b) At 1280: `data-open="true"` with no `data-term`, columns `269px 404px`, list `display: flex` showing both sessions, the open row marked by the accent rail, `Back to sessions` hidden (`display: none`), no horizontal scroll (`scrollWidth` 1265 under a 15 px scrollbar, viewport 1280). (c) Terminal text 13 px (`getComputedStyle(.xterm-rows).fontSize` = `13px`); xterm draws, replay text present; Ctrl-C then "Ended (code 130)" with every key disabled and the screen kept (`i-ended-*.png`). (d) Chat panel untouched: `SessionPanel.jsx` not in the diff, `LiveView.test.jsx:1204` (`.session-back { display: none; }` wide rule) still passes. (e) No dead code left: the three removed rules were the only users of `data-term`; no comment elsewhere says the terminal takes the whole column (grep). (f) The Back button of the terminal panel on a wide screen is hidden like Chat's; the panel closes by activating its row again, the same as Chat.

ux-evaluate (by hand, screenshots read with the image reader; the guide has no UX Acceptance Criteria rows, so baseline checks only):

| Check | 375 | 1280 | Reason |
|---|---|---|---|
| B-a primary action in view | fail (known, first review finding 3) | fail (known) | At 375 the terminal box starts y=813 and the key row y=1225; at 1280 the box is visible (y=568..1168) but the key row (1180) and Stop (1288) are below the 1000 px fold. Chat sits in the same place. P3, owner decides the design |
| B-b hierarchy | pass | pass | Title, facts, state, screen, keys, Stop in one reading order; list left, panel right |
| B-c spacing | pass | pass with note | Even gaps; at 1280 the key row is 5 + 3 and "Ctrl-C" wraps (finding 2) |
| B-d targets 44x44 | pass | pass | Keys 69x44, Back 157x44, Stop 65x44 at 375 (measured); at 1280 desktop, Stop is 65x32 (the 44 px rule is the 375 rule) |
| B-e readable text | pass with note | pass with note | Terminal cells 13 px by owner decision (below the 16 px body rule; the guide and the owner accept it); labels and facts are body size, contrast clear |
| B-f states | pass | pass | Running and Ended (code 130) with keys off seen; connecting, lost, dropped-input covered by the first review's shots and unchanged here |
| B-g feedback | pass | pass | "Ended (code 130)" and disabled keys appear at once after Ctrl-C |
| B-h phone layout | pass | n/a | One column with Back, no horizontal scroll (`scrollWidth` 375), 36 columns, nothing clipped |
| B-i one visual language | pass | pass | Existing tokens, one accent, same radius and button style as the Chat panel |

Commands run (this re-review), in the T134 worktree:

```
$ cd web && npm ci && npm test -- --run && npm run build && cd .. && python3 scripts/check_static.py
found 0 vulnerabilities
Tests  1611 passed (1611)
TermPanel-14BkAdyq.js  337.14 kB | gzip: 85.43 kB   index-3mMlrwzZ.js  473.03 kB | gzip: 139.78 kB   built in 1.21s
check_static: ok
$ git status --short   -> clean before this file was edited (the build output is not tracked)
```

### Findings (first review, commit 3122a17; status in the re-review table above)

| # | Sev | Where | Finding |
|---|-----|-------|---------|
| 1 | P2 | `web/src/NewSessionSheet.jsx:12` + `web/src/api.js` `getTermSessions` | The terminal list is read inside the same `Promise.all` as the Chat data. `getTermSessions` turns only a 404 into `null`; a 500 or a network error on `/api/term-sessions` rejects the whole load, so the New session sheet fails and **Chat cannot be started either**. AC 1 says today's Chat form is unchanged; this couples it to the new route. Fix: catch to `null` for this one call (the Terminal choice then stays hidden). No test covers a failing term list. |
| 2 | P2 | `web/src/LiveView.jsx` (`useTerms`, `termOnly`), `web/src/api.js` (`isExited`, `accountLabel`) | The page guesses the list shape: `state: 'exited'` or `running === false`, `account` as `{id,label}` or a string, a bare list or `{sessions}`. T133's guide only says "`running` or `exited` + code". The T133 worktree (uncommitted) returns `state`, `code`, `pages`, `session_id`, `started`, `account {id,label}` inside `{sessions}`, which matches, and POST returns `201 {id}`; the field list T133 accepts (`repo, workspace, account, model, mode, worktree`) matches what `NewSession.jsx` sends. Re-check once T133 is committed: a renamed field would show every ended session as "Running". |
| 3 | P3 | UX, `web/src/appsession.css` `.term-box`, `LiveView.jsx` placement | After Start the panel opens below the "All sessions" summary card: at 375x800 the terminal box top is y=813 (below the fold; only the title, facts and "Running" show), at 1280x1000 the box starts y=580 and the key row and Stop are below the fold. The terminal is the page's one job here, yet the user must scroll to reach it. Chat panels sit in the same place, so it is not new for them, but a 60vh terminal makes it matter. Suggest scrolling the box into view on open (the owner decides the design). B-a: fail at 375, partial at 1280. |
| 4 | P3 | `web/src/LiveView.jsx` comment above `TermPanel` | Says "about 124 KB gzip"; the build prints `TermPanel-*.js` 337.14 kB, gzip 85.43 kB (+ css 1.01 kB gzip). Wrong number in a comment only. |
| 5 | P3 | `web/src/LiveView.jsx` `useTerms` | A server with no terminal route answers 404 to the first read, `useTerms` returns `null` and still polls every `APPS_IDLE_MS` for ever. Harmless noise; stop after a 404 if it ever matters. |
| 6 | P3 | `web/src/TermPanel.jsx` | Typed bytes before the socket opens are dropped silently (tested as intended). The "Connecting" line shows, so acceptable. The xterm screen-reader mode is off: the terminal is an unnamed-content `role=group`. Not an AC; note for later. |

Not defects, checked: token only in the first frame and in the `Stop` header (no `?token`, tested); no storage or console output of screen bytes (tested); no HTML from server bytes (xterm draws them); no link or clipboard addon loaded; CSP header unchanged and `@xterm` brings no remote URL (`grep -o "https\?://"` on the built `TermPanel-*.js`: none; `check_static: ok`); two pinned MIT deps, no install scripts, `npm ci` reports 0 vulnerabilities; Esc is not bound to close the terminal panel (the Chat panel's Esc handler sits on `SessionPanel` and the row `li`, neither wraps `TermPanel`); the 8-key bytes match the guide.

### Security review (Medium risk, manual)

No finding above P3. A page that sends typed bytes is bounded by the same-origin guard and token that T133 enforces; the page adds no new input path beyond the xterm keys and the fixed key row. xterm answers terminal queries (device attributes, cursor position) back as stdin: that is a normal terminal property, not a page flaw. OSC 8 links in xterm 6 default to its own confirm before `window.open`; the build adds no link addon. Residual (owner-accepted, FR-137): every attached page can type.

### AC table

| # | Verdict | Evidence |
|---|---------|----------|
| 1 | pass | `NewSession.test.jsx` (Chat default, Terminal hides prompt/effort/worktree, note text, POST body exactly repo/workspace/account/model/mode/confirmed, bypass second step, 409 shown) and `NewSessionSheet.test.jsx`; see finding 1 for the Chat-coupling gap |
| 2 | pass | `TermPanel.test.jsx` "the socket (AC 2)" (auth first, no token in URL, wss, fit, binary typed bytes, debounced resize); real Chrome run against a stand-in: `AUTH {'t':'auth','cols':36,'rows':23}` at 375, `cols 81 rows 35` at 1280 |
| 3 | pass | `it.each` of the eight keys; my run: keys 69x44 px at 375 (two rows of four), 74x44 at 1280, no horizontal scroll (`scrollWidth` 375). Terminal is below the fold on open (finding 3) |
| 4 | pass | Tests for exit, pages, notice, Stop with `confirmed`, refused Stop; my run: Ctrl-C then `Ended (code 130)`, all keys disabled, screen text kept |
| 5 | waits on T133 | Page side tested (new panel opens its own socket, authenticates, draws the replay; Reconnect clears the screen first). The replay itself and `/verify` need the real routes |
| 6 | pass | `LiveTerm.test.jsx` (listed, ended row, dedupe by Claude session id, opens `TermPanel` not `SessionPanel`, Chat still opens Chat) |
| 7 | pass | `git diff` of `web/package.json` and lock: only `@xterm/xterm 6.0.0` and `@xterm/addon-fit 0.11.0`, exact pins; `check_static: ok`; no `.map` in the build; no `https?://` in the new chunk; `server.rs:39` CSP untouched |
| 8 | pass for the page, waits on T133 for the real server | Chrome (above) and WebKitGTK 4.1 under Xvfb against a stand-in: xterm drawn, screen text "Welcome to Claude Code cwd: /work/company", status Running, focus on the terminal after a key-row press. Not run against `serve` |
| 9 | pass | Run below |

### Commands run (this reviewer)

```
$ cd web && npm ci && npm test -- --run && npm run build && cd .. && python3 scripts/check_static.py
found 0 vulnerabilities
Test Files  48 passed (48)
     Tests  1610 passed (1610)
TermPanel-Bj9yDdaC.js  337.14 kB | gzip: 85.43 kB   index-B7eL-4YZ.js  473.05 kB | gzip: 139.79 kB
check_static: ok
$ git status --short   (after the build)   -> clean (the build output is not tracked)
```

ux-evaluate (by hand; screenshots in the reviewer scratchpad `.../scratchpad/qa/*-375.png`, `*-1280.png`, read with the image reader): B-a fail (finding 3), B-b pass, B-c pass, B-d pass at 375 (keys 69x44, Stop 64x44), B-e pass, B-f pass (connecting, lost with Reconnect, ended, dropped-input notice all say what to do), B-g pass, B-h pass (no horizontal scroll, 36 columns at 375), B-i pass (existing tokens, one accent). The ended state shows the accent border from `:focus-within` while input is off: cosmetic.

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/TermPanel.test.jsx` (30 tests: AC 2, 3, 4, 5), `web/src/LiveTerm.test.jsx` (10: AC 6, start-to-panel), `web/src/NewSession.test.jsx` (+10: AC 1), `web/src/NewSessionSheet.test.jsx` (+4: Terminal offered only with the route, one cap), helper `web/src/test/fakeXterm.js` (fake Terminal / FitAddon / WebSocket / ResizeObserver), `web/src/test/appSessions.js` (`terms`, `termSession`). AC 7 = build + `check_static.py`. AC 8 and the real-server part of AC 5: waiting on T133 (see Demonstration). |
| Verification command run | ☑ pass | `cd web && npm ci && npm test -- --run && npm run build && cd .. && python3 scripts/check_static.py` at 2026-11-22T15:02:51Z: `found 0 vulnerabilities`; `Tests  1610 passed (1610)` (48 test files, 54 of them new: 30 + 10 + 10 + 4; no existing test was changed or removed); build `✓ built in 290ms`, lazy chunk `assets/TermPanel-Bj9yDdaC.js 337.14 kB │ gzip: 85.43 kB` + `TermPanel-BrP-ENHg.css 1.01 kB gzip`, entry `index-B7eL-4YZ.js 473.05 kB │ gzip: 139.79 kB`; `check_static: ok` (exit 0; no change to `ALLOWED_URL_PREFIXES` or `vite.config.js` was needed: Vite drops the comment URLs and `sourcemap: false`). |
| Negative cases hold | ☑ pass | Red first: `npx vitest run src/TermPanel.test.jsx` before `TermPanel.jsx` existed failed to import it (0 tests ran). Mutation: in `TermPanel.jsx` keep the input on after exit (`live = link === 'open'` and `send` without `!endedRef.current`): `FAIL src/TermPanel.test.jsx > the end and the pages (AC 4) > an exit frame says "Ended (code N)", turns the input off ...`, `Tests  1 failed \| 29 passed (30)`; restored: `Tests  30 passed (30)`. Negative tests: token not in the URL, nothing sent before open, nothing typed after exit, malformed text frame ignored, no storage/console, 404 server hides the choice, refused start/Stop shown. |
| verify | ☑ pass | auto-run by hook, session 88e2119e-1931-40e9-90a7-64a47cf941a2, commit 8fcf0b8b984296f56486c30b0be6e4340641b2a8, log .claude/verify-runs/T134-8fcf0b8b9842.log — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed the 15 changed files and the `LiveView` callers; skipped `rust/**` (T133) and Chat panel files except to confirm they are untouched. See "Review findings" |
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
