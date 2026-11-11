# TASK_REVIEW — T016: Window spike (Tauri v2 under WSLg)

> Sibling of `tasks/TASK_GUIDE_T016.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | a spike has no unit tests; the run is the test: `rust/spikes/t016-window/` (`src/main.rs`, `src/page/index.html`, `src/page/app.js`) and the findings `rust/docs/spike-T016.md` |
| Verification command run | ☑ pass | Supervisor ran `cargo run` in `rust/spikes/t016-window` under `timeout 90`: exit 0 in 0.71 s wall, one stdout JSON line. Report from inside the webview: `fetch_with_token` 200 `{"version":"spike"}`, `fetch_without_token` 403, `fetch_wrong_token` 403, `hash_present` true (64 chars), `origin` `http://127.0.0.1:<port>`, `ready_state_at_post` `complete`; CSP enforced (inline script blocked, other-origin fetch blocked, inline style applied); window events `window created`, `gdk display type: GdkWaylandDisplay`, `page load Started` / `Finished`; `DISPLAY=:0`, `WAYLAND_DISPLAY=wayland-0` |
| Negative cases hold | ☑ pass | agent's failure-mode runs: no display and no preflight -> tao panics, exit 101 in 0.11 s; with a `gtk::init()` preflight -> clear message, exit 5; a dead `DISPLAY=:99` with `GDK_BACKEND=x11` HANGS (killed by `timeout` at 40 s); the page never reporting -> exit 3 at 20.13 s with a message; no workaround variable is needed (`GDK_BACKEND=x11`, `LIBGL_ALWAYS_SOFTWARE=1`, `WEBKIT_DISABLE_COMPOSITING_MODE=1` also pass). `git status --short -- src web tests pyproject.toml uv.lock` empty (Supervisor re-checked). Supervisor read of `main.rs`: the only bind is `TcpListener::bind(127.0.0.1:0)`; no other host appears; the config has no updater, tray or network plugin |
| verify | ☑ pass | the Supervisor's own run (above) shows a Tauri v2 window opening under WSLg on Wayland and the webview loading `http://127.0.0.1:<port>/#<token>`, reading the fragment and calling the guarded endpoint with the header — pass. Nobody looked at the window by eye (the agent and the Supervisor rely on the in-webview report plus the focus and resize events). `ps` after the run showed no `WebKitWebProcess` or spike process |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | only `rust/spikes/t016-window/**` and `rust/docs/spike-T016.md` are new; `rust/Cargo.toml`, `Cargo.lock`, `rust/parity/**` hashes equal before and after (agent's `sha256sum -c`); `rust/crates/**` untouched by this task |
| Full smoke suite still green (no regression) | ☑ pass | the spike is outside the workspace (its own `[workspace]` table and lock) and cannot affect `cargo test`; the whole suite is re-run when T017 lands |
| **Companion score (easy-verifier)** | ☑ N/A | not run: easy-verifier is not installed in this session |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | a throwaway page with no design; no screenshot was taken |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no product UI |
| **UI: Responsiveness at target viewports** | ☑ N/A | no product UI |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-10T11:58:10Z by the Supervisor, before the agent's first command:
```
$ cd rust; sha256 of crates/, Cargo.toml, Cargo.lock (product files); ls spikes docs
bc3a5114fe8658d85b9693ea96bbf71afed46ac95a4f64709df9fe0f9a2f2acd
t001-crates SPEC.md main-flow.md spike-T001.md spike-T009.md
(no spikes/t016-window, no docs/spike-T016.md)
```
(no window code or finding exists; the product hash must be identical after the task.)

**AFTER**: captured 2026-11-10 by the Supervisor:
```
$ cd rust/spikes/t016-window && timeout 90 cargo run -q        # rc=0, 0.71 s wall
[spike +194ms] window created: label 'main'
[spike +194ms] gdk display type: GdkWaylandDisplay
[spike +320ms] page load Started url=http://127.0.0.1:45729/
{"fetch_with_token":{"status":200,...},"fetch_without_token":{"status":403,...},"hash_present":true,...}
```

**DELTA**: it is now proven on this machine that a Tauri v2 window opens under WSLg and its webview can load a loopback page, read the token from the URL fragment and call the guarded API under the dashboard CSP, so P1b can build the real window on a verified base.

**WITNESS**: Supervisor, 2026-11-10, ran the spike independently of the implementing agent.

**Stage 4 report (added at close-out 2026-11-11):** code-review `reports/code-review_main_20261111T112730.html` (Supervisor, inline): 0 P0, 0 P1, 0 P2, 2 P3 (deliberate in a throwaway).
