# TASK_REVIEW — T[NNN]: [Short Title]

> Sibling of `tasks/TASK_GUIDE_T[NNN].md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-15T15:54Z, before any implementation commit; `site/index.html` at commit 9a74ae4):

Test run (`uv run pytest tests/test_site_content.py tests/test_site_theme.py tests/test_site_deploy_config.py -q 2>&1 | tail -3`, 2026-11-15T15:54:36Z):
```
FAILED tests/test_site_content.py::test_stat_strip_numbers_come_from_real_sources
1 failed, 73 passed in 1.38s
```
(Pre-existing failure, not from T089: the page shows 9 dashboard views, `web/src/App.jsx` has 10.)

Hero install box (`#install-cmd`):
```
git clone https://github.com/thunderkds/easy-personal-mcp.git
cd easy-personal-mcp
npm --prefix web ci && npm --prefix web run build
uv tool install --reinstall --no-cache .
```
Quick start strip (`Quick start in four steps`): 1 Install `uv tool install --reinstall --no-cache .` ("After the build in the box above, install the tool."); 2 Ingest `easy-personal ingest`; 3 Register with Claude Code `claude mcp add --scope user easy-personal -- easy-personal mcp`; 4 Open the dashboard `easy-personal dashboard`.

`#install` ("Install and deploy"): intro "You need Python 3.11 or newer, `uv` and Node.js. The dashboard page is built from `web/` and is not in git, so build it before you install. It runs natively on Linux." Eight steps: Install (the four lines above, then `easy-personal --version` / `easy-personal ping`), First run, Register the MCP server, Open the dashboard, Schedule, Connectors, Upgrade, Roll back. No mention of the `.deb`, `easy-personal-rs`, the tray or Recent alerts. `README.md` `## Install` starts with "You need Python 3.11 or newer, [uv](...) and Node.js." and the same four lines.

**AFTER** (2026-11-15, after the change):

Hero install box (`#install-cmd`), label "Install on Ubuntu (Rust app)":
```
git clone https://github.com/thunderkds/easy-personal-mcp.git
cd easy-personal-mcp
rust/packaging/build-deb.sh
sudo apt install ./rust/target/deb/easy-personal-rs_*_amd64.deb
```
Quick start: 1 Install `sudo apt install ./rust/target/deb/easy-personal-rs_*_amd64.deb`; 2 Schedule `easy-personal-rs schedule install`; 3 Register `claude mcp add --scope user easy-personal -- /usr/bin/easy-personal-rs mcp`; 4 Open the dashboard `easy-personal-rs dashboard --static-dir /usr/share/easy-personal-rs/static`.
`#install`: six Rust steps (Build the package, Install the package, Schedule and register, Open the dashboard, Start the tray, Remove the package), then the heading "Python app (second option)" and the eight old Python steps unchanged. The tray step says: "The tray icon shows one state: idle, active, attention or problem. Its menu lists the sessions, the plan limits and the machine load. The "Recent alerts" block lists the newest 5 alerts, and the row "All alerts in dashboard…" opens the dashboard when there are more. The block stays while alerts are paused." and the "Ubuntu AppIndicators" tray-host note. `README.md` `## Install` has `### Ubuntu package (Rust app)` first, then `### Python app`. `rust/README.md` "Not done" no longer lists packaging or a tray.

Test run (`uv run pytest tests/test_site_content.py tests/test_site_theme.py tests/test_site_deploy_config.py tests/test_readme.py -q`): `1 failed, 97 passed`; the one failure is `test_stat_strip_numbers_come_from_real_sources` (page 9 views, App.jsx 10), the same failure as in BEFORE. Full suite: `1 failed, 1855 passed, 2 skipped, 7 deselected, 1 xfailed in 175.68s`.

Screenshots (headless google-chrome): `tasks/T089-screens/hero-desktop.png`, `hero-375.png`, `install-desktop.png`, `install-375.png`. At 375 px an iframe check found `scrollWidth == clientWidth` (360 = 375 minus the scrollbar) and no element past the right edge outside `pre`; long commands scroll inside their `pre`.

**DELTA**: A reader of the page and README now installs the Rust app from a `.deb` on Ubuntu (with the tray and its Recent alerts block explained), and the Python `uv` install is the labelled second option.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T089.jsonl`, never the
implementing agent alone]

---

## Stage 4 — code-review (Supervisor inline, 2026-11-15)

Scope: `site/index.html`, `README.md`, `rust/README.md`, `tests/test_site_content.py`. Commands checked against `rust/crates/app/src/main.rs` and `rust/packaging/*.desktop`.

- P1 (fixed, `014f7eb`): the window step used `easy-personal-rs dashboard`. In the Rust app `dashboard` is an alias of `serve` (`main.rs:71`): it serves on 127.0.0.1 and prints a URL; it opens no window. The `.desktop` entry runs `app --static-dir /usr/share/easy-personal-rs/static`. Page, README and `RS_DASHBOARD` now use `app`.
- Pre-existing failure fixed in scope: `test_stat_strip_numbers_come_from_real_sources` failed on main (page 9 views, `web/src/App.jsx` 10 since T077 Watch view). Stat → 10.
- P1 (process): startup reads partial (PROJECT_SPEC.md, rust/PROJECT_SPEC.md, agents/frontend.md line counts only). No content impact found.
- P3: `rust/packaging/README.md` says "The dashboard needs the page folder: `easy-personal-rs dashboard --static-dir …`" — correct for `serve`, but a reader may expect a window. Not touched (out of scope); candidate follow-up.
- Agent choice accepted: the AppIndicators `gnome-extensions enable …@ubuntu.com` command is linked, not copied (the `@` trips the no-email scan).

Tests after fixes: site + readme `98 passed`; full suite `1856 passed, 2 skipped, 7 deselected, 1 xfailed`.
