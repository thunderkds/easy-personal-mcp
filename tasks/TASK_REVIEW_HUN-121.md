# TASK_REVIEW — HUN-121: Restyle dashboard to reference image 2: indigo + monospace, grouped sidebar, right rail

> Sibling of `tasks/TASK_GUIDE_HUN-121.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Task breakdown and labels approved; run all tasks in auto mode — FACT: PROJECT_SPEC.md:175 "do all by parallel with tag [auto], merge to main and push"
2. Requirement Fidelity Gate signed auto (Supervisor) — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (row 13)
3. Spawn model from the CLI Model Table — RULE: `PROJECT_SPEC.md` § CLI Model Table
4. Owner's mid-run request (image 2 + "looks like the timeline") became new task HUN-121, run last because it restyles the same files — RULE: `PROJECT_SPEC.md` § Tasks (waves: tasks that edit the same files start from their predecessor's branch)
5. Merge to main and push after `/verify` passes (the owner's explicit ask overrides the auto-mode no-merge rule for this one step) — FACT: docs/claude-md/tracks.md:365 "the explicit ask wins for X"
6. HUN-121 scope: restyle + grouped sidebar + right rail from existing data + timeline as feed cards; social features cut — SAFER OPTION: smallest reading of "update the UI like this" that adds no API (Risk Low: Supervisor decides alone) ⚠ decided for you — please review
7. HUN-121 base = HUN-120 branch + cherry-picked HUN-116 and HUN-119 implementation commits (a merge is blocked while tasks are open; identical patches merge cleanly into main later) — SAFER OPTION: the only way to start from all three views without merging (Risk Low: Supervisor decides alone) ⚠ decided for you — please review
Checks: tests `npm --prefix web test && npm --prefix web run build && uv run pytest -q` — 229 Vitest passed, 768 pytest passed; reviews code-review — P3 1 (Risk Low: no security-review); real run stub fixture server, headless Chrome 375/1024/1440 for all 5 views in reports/HUN-121/ (auto-checked, not /verify)
Not done: merge (owner asked for it; runs after verify); auto verify: running (T182)
Paused: auto-mode check failed — extended track, but the trace has no `code-review` Skill call (Stage 4 runs in auto mode)

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | New: `web/src/Rail.test.jsx` (14 tests: AC5, AC6, SC2, SC3, SC4, long label, failures, scope refetch), `web/src/FeedCard.test.jsx` (3). Extended: `web/src/App.test.jsx` (AC3 groups/dots/scope-on-top, SC1 collapse Setup, AC5 rail in App, AC6 scope in rail, AC9 x2), `web/src/Timeline.test.jsx` (SC5 75%/25% bars, icon, summary, time, footer, tool calls only when present, count), `web/src/KitView.test.jsx`, `web/src/WarningsPlanView.test.jsx` (feed cards). Red first: 16 failed / 196 passed before the code; after: `Test Files  10 passed (10)  Tests  229 passed (229)` (2026-10-29) |
| Verification command run | ☑ pass | `npm --prefix web test && npm --prefix web run build && uv run pytest -q` (2026-10-29T09:00:09Z, clean env, no EASY_PERSONAL_* vars): vitest `Tests  229 passed (229)`; build `✓ built in 1.48s` (index-PhZFsGJT.css 16.94 kB, index-Dxwtpt0t.js 276.64 kB); pytest `768 passed, 4 deselected, 1 xfailed, 1 warning in 120.18s` |
| Negative cases hold | ☑ pass | Rail: every request 500 -> each of the 3 cards shows `no data`, nothing throws; one failing route leaves the other cards working; `cache_hit_ratio: null` -> ring `no data`, no 0%; no hits -> `Nothing to fix`; stale / empty / codex accounts left out of Plan limits; long label ellipsis + `title`. Timeline: tool calls omitted when the data has none; 60 sessions -> 50 cards, shares still of all 60. AC9: reload after confirm/ignore updates the sidebar list and the ignored note. `grep -nE '#[0-9a-fA-F]{3,8}\b|rgba?\(|hsl' web/src/app.css web/src/*.jsx` (non-test) -> no output |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | code-review (Supervisor, 2026-10-29): tokens/app.css restyle, App grouped sidebar + reloadOptions, Rail, FeedCard, Timeline feed, Warnings/Kit on FeedCard, AgentsView onAccountsChanged + tests, base a487cdc. lane: C2, 20 files, 1748 lines, EXCEEDS (full restyle; label note). Findings after gate: P0 0, P1 0, P2 0, P3 1 (rail fetches while hidden below 80rem). Screenshot after-usage-tall-1440 checked by Supervisor against reference image 2 — pass |
| Full smoke suite still green (no regression) | ☑ pass | All 195 prior Vitest tests still pass (only selectors changed: nav buttons filtered by `aria-expanded`, `Kit by project` heading matched by `/^Kit by project/` because it now reads `Kit by project (N)`); full run above: 229 vitest + 768 pytest passed; no new npm dependency (`git diff web/package.json` empty) |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Screenshots `reports/HUN-121/after-{usage,warnings,kit,workspaces,agents}-{375,1024,1440}.png` (+ `after-usage-tall-1440.png`), taken with headless Chrome against the stub fixture dashboard; before images `before-*-1280.png`. Verdict vs reference image 2 (my own reading of the screenshots): reads like the reference. Indigo page and cards, one monospace stack, left nav with collapsible INSIGHTS / SETUP groups and colored dots, Scope block on top, center feed of rounded cards with thin dividers (session cards: round initial icon, accent project name, one-line summary, right-aligned time after a divider, chart, gradient token-share bar, footer counts), blue-to-purple gradient bars and ring, right rail with Cache reuse ring, Next actions (3), Plan limits. Differences: no avatar, social features, search or menus (out of scope / cut list) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Colors: grep above finds no hex/rgb/hsl outside `tokens.css`; JSX `stroke`/`fill` are `currentColor` or `url(#gradient)` whose stops use token classes. Font: computed `font-family` of every element under `<body>` includes `monospace` at 1440 px (probe over usage, kit, agents, workspaces: non-monospace list empty); stack is `ui-monospace, 'SF Mono', 'Cascadia Mono', 'DejaVu Sans Mono', Menlo, Consolas, 'Liberation Mono', monospace`, no web font. Contrast (WCAG, computed): text #efecff on card #251f52 13.0, on card-raised 11.0; muted #b9b3e3 on card 7.6, on card-raised 6.4; accent text #a9c0ff on card 8.4; error 6.7; up/down/gap chips 4.8 to 9.3; white on gradient start #3b6be0 4.81, end #9140dc 5.29; dark initial on the 5 dot colors 5.0 to 9.7 |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 / 1024 / 1440 px screenshots of all 5 views. Probe (`scrollWidth` vs `clientWidth`) shows no horizontal overflow at 375 (Chrome headless floors the window at ~485-500 px, the 375 px PNGs are 375 wide and show no clipping), 1024 and 1440. 375: sidebar behind the Menu button, rail `display: none`. 1024: sidebar + content, rail `display: none`. 1440: sidebar + content + rail (`display: flex`). Wide tables scroll inside their `.table-wrap` |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-10-29T08:52:24Z, before any edit; base commit a487cdc; fixture dashboard = Node stub server serving the built `web/` bundle with the Vitest fixtures as `/api/*`, headless Chrome 1280 px; files in `reports/HUN-121/before-<view>-1280.png`, gitignored):

```
$ npm --prefix web test      (2026-10-29 15:51 local)
 Test Files  8 passed (8)
      Tests  195 passed (195)
$ ls reports/HUN-121
before-agents-1280.png  before-kit-1280.png  before-usage-1280.png  before-warnings-1280.png  before-workspaces-1280.png
```

Observed in the screenshots: near-black navy page (#080b22), system-ui sans-serif font, a flat sidebar with 5 ungrouped text buttons, two-column shell (no right rail), plain bordered sections, Usage "Session timeline" as a bare list (project, session id, "start to end", chart, gap list), Warnings and Kit as plain cards with no icon or right-aligned time.

**AFTER** (2026-10-29T09:00Z, same fixture dashboard and Chrome; `reports/HUN-121/after-<view>-{375,1024,1440}.png`):

```
$ npm --prefix web test      (Rail + FeedCard + extended App/Timeline/Kit/Warnings tests)
 Test Files  10 passed (10)
      Tests  229 passed (229)
$ npm --prefix web run build && uv run pytest -q
 ✓ built in 1.48s
 768 passed, 4 deselected, 1 xfailed, 1 warning in 120.18s (0:02:00)
probe at 1440 px: {"sw":1425,"cw":1425,"railDisplay":"flex","nonMono":""}   (375 px / 1024 px: railDisplay "none")
```

The screenshots show the indigo monospace shell, grouped sidebar with dots, feed cards and the right rail.

**DELTA**: the dashboard now has the reference look (indigo, monospace, grouped sidebar, gradient bars, feed cards, right rail with cache ring, top actions and plan limits at 80 rem and wider), and the sidebar account list and ignored-accounts note update after Agents changes without a page reload.

**WITNESS**: frontend-developer agent, 2026-10-29, ran the commands above in the HUN-121 worktree; the Supervisor confirms from `memory/event-trace/HUN-121.jsonl`.
