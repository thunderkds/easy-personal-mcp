# TASK_REVIEW — HUN-132: Landing page in the Easy family layout

> Sibling of `tasks/TASK_GUIDE_HUN-132.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/test_site_theme.py` (rewritten: Graphite Mint + 5 glow tokens, no raw color outside `:root`, self-hosted fonts all used, one inline script only, AA contrast, 64 px / 248 px sizes, 768 px rule; negatives for hex, rgba, remote font, 2nd script, changed token) and `tests/test_site_content.py` (new: header brand + version pill = pyproject, family nav `aria-current="page"` + the 3 family URLs = `web/src/App.jsx` FAMILY, aside groups + own-section links + one current item, hero eyebrow/h1/primary button, stat strip numbers = server registration / catalog / `App.jsx` views with drift negatives, install box lines in README + Copy button + `navigator.clipboard.writeText(` script; tools check now reads `tr[data-tool]`; `easy-memory.vercel.app` allowlisted). Mutation check: stat 10 -> 11, pill v0.1.0, removed `aria-current`, bad aside href, raw `#fff` each fail a named test. `uv run pytest -q tests/test_site_content.py tests/test_site_theme.py tests/test_site_deploy_config.py` -> `55 passed in 0.48s` (2026-10-29T15:51:50Z) |
| Verification command run | ☑ pass | `uv run pytest -q tests/test_site_content.py tests/test_site_theme.py tests/test_site_deploy_config.py && uv run pytest -q` -> `55 passed in 0.48s` then `829 passed, 5 deselected, 1 xfailed, 1 warning in 108.10s (0:01:48)` (2026-10-29T15:51:50Z to 15:53:39Z). `uv run ruff check` on the two test files: `All checks passed!` |
| Negative cases hold | ☑ pass | `test_drifted_stat_numbers_fail`, `test_wrong_version_fails`, `test_hex_outside_root_fails`, `test_rgba_outside_root_fails`, `test_remote_font_fails`, `test_second_inline_script_fails`, `test_changed_token_fails` plus the HUN-57 negatives all pass. No-clipboard case (Playwright, `navigator.clipboard` undefined): `copy button hidden = true`, command text stays selectable (`user-select: auto`). |
| verify | ☐ pass / ☐ fail / ☐ N/A | Left for the Supervisor (`Skill verify`, Stage 5). Implementer ran the browser checks above. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | Left for Stage 4 `code-review`. Blast radius: `site/**` and the two site tests only; `src/` and `web/` not touched (the dashboard build used for screenshots wrote only the gitignored `src/easy_personal/dashboard/static/`). |
| Full smoke suite still green (no regression) | ☑ pass | `829 passed, 5 deselected, 1 xfailed, 1 warning in 108.10s (0:01:48)` |
| **Companion score (easy-verifier)** | ☑ N/A | Not run by the implementer; the Supervisor runs the companion tools at Stage 4. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Host Playwright (easy-ui-mcp `node_modules`; easy-ui is fixed 1280x720) on `python3 -m http.server` of `site/`. 1280 px viewport: `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/HUN-132/reports/HUN-132/site-1280-viewport.png`; sections: `site-1280-flow.png`, `site-1280-dashboard.png`, `site-1280-connectors.png`. Verdict by reading the images against `Memory-Mint.dc.html` / `Graphite-Mint.dc.html`: 64 px header (brand mark with line glow, mono version pill, family nav with accent dot + dot glow on Easy Personal, bordered GitHub button), 248 px aside with `// overview` / `// getting started` / `// reference` and the accent-tinted current item with inset ring, mono `>` eyebrow with neon text, 44 px / 600 h1, accent primary button with glow, install box (`#0E100F`, control border, glow shadow) with Copy, 3-cell bordered stat strip, numbered-step flow lanes, tool table, connector cards. Measured: header 64 px, aside 248 px, main padding 48/56 px, h1 44 px / 600, button bg `rgb(60, 245, 168)` text `rgb(14, 16, 15)` with box-shadow. Copy test: click -> `Copied` -> `Copy` after 2.1 s; clipboard = the 4 install lines. Dashboard re-shot (neon build, fixture data `tests/fixtures/claude_code`, dates shifted, empty HOME, fake `claude`): `site/dashboard-usage.png`, `-warnings.png`, `-kit.png`, `-workspaces.png` (each <= 107 KB; read: only `demo-app`, `claude-fixture-model-1`, `/work/demo-app` fixture paths, no email or token). Old `dashboard-connectors.png` removed: the new dashboard has no Connectors tab (connectors live in a workspace). |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Theme test: all 16 color tokens + 5 glow tokens pinned, no raw color outside `:root`, text tokens >= 4.5:1 on `#121413` (faint `#6e7772` is 4.0:1 so it is not used). Playwright computed style at 1280: body font `"IBM Plex Sans", system-ui…`, eyebrow `"IBM Plex Mono", ui-monospace…`, `document.fonts` = `IBM Plex Sans 400/500/600 loaded`, `IBM Plex Mono 400/500 loaded` (all served from `site/fonts/`). Network log: `requests 10 origins ["http://127.0.0.1:8137"] errors []` at 375, 768 and 1280 px (no off-origin request; log in `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/HUN-132/reports/HUN-132/site-check.log`). |
| **UI: Responsiveness at target viewports** | ☑ pass | `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/HUN-132/reports/HUN-132/site-check.log`. 375 px: `doc":[375,375]` (no horizontal scroll; first run showed `[535,375]` from the install box `pre` in a column flex, fixed with `align-items: stretch`), aside stacked above main as a wrapping link row, header nav wraps (172 px tall), tool table scrolls in its box (`tableWrap [456,341,"auto"]`), 6 `pre` blocks scroll in their own boxes, `cardsOverflow 0`. 768 px: `[768,768]`, `cardsOverflow 0`. 1280 px: `[1280,1280]`. Screenshots: `site-375.png`, `site-375-top.png`, `site-375-tools.png`, `site-375-flow.png`, `site-768.png`, `site-768-top.png`, `site-1280.png`. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-10-29T15:40:03Z on branch HUN-132 at 7107301, before any implementation commit):

```
$ uv run pytest -q tests/test_site_content.py tests/test_site_theme.py tests/test_site_deploy_config.py
.........................................                                [100%]
41 passed in 1.46s
```

Current page facts at that commit (from `site/index.html`): old Kit cyan/purple palette (`--bg: #0a0a12`, `--cyan: #00d4ff`, `--green: #00ff88`), system font stack only (`font: 1rem/1.6 system-ui ...`, no `@font-face`), no `site/fonts/` directory (`ls site/fonts` -> `No such file or directory`), single centered `.wrap` column with no header bar, no left section nav, no stat strip and no `<script>` (`grep -c "fonts/\|Plex" site/index.html` -> `0`). The theme test pins the old tokens (`"--bg": "#0a0a12"`, `"--cyan": "#00d4ff"`) and asserts `"system-ui" in style and "@font-face" not in style`.

**AFTER** (2026-10-29T15:51:50Z, same command):

```
$ uv run pytest -q tests/test_site_content.py tests/test_site_theme.py tests/test_site_deploy_config.py
.......................................................                  [100%]
55 passed in 0.48s
```

Page at 1280 px (`/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-wt/HUN-132/reports/HUN-132/site-1280-viewport.png`): 64 px header with brand, `usage dashboard · v0.2.0` pill and family nav (Easy Personal active with dot glow), 248 px aside with the three `// …` groups, neon eyebrow, 44 px h1, README pitch, accent primary button, install box with a working Copy button (`Copied` for about 2 s, clipboard holds the 4 install lines), stat strip `10 MCP tools / 5 connector providers / 5 dashboard views`. `site/fonts/` holds 5 Plex woff2 + `OFL.txt`; every request goes to the site origin.

**DELTA**: A visitor now gets the Easy family layout (header, section nav, hero, install box with Copy, stat strip) in the neon mint look, with the existing content and fresh dashboard screenshots, and every stat number is checked against its source by a test.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/HUN-132.jsonl`, never the
implementing agent alone]
