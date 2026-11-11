# TASK_REVIEW — T021: Serve the dashboard static files (--static-dir)

> Sibling of `tasks/TASK_GUIDE_T021.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/tests/server.rs` (16 new tests, 52 in all: fixture with an outside marker file; 44 traversal forms on the wire with and without the token; symlinks in and out; dotfiles; a FIFO with a timeout; long and repeated paths; the no-token rule; guard; HEAD/405; the `api/usage/summary` shadow file; fixture bytes unchanged) and 2 unit tests in `rust/crates/app/src/static_files.rs` |
| Verification command run | ☑ pass | Supervisor, own runs with T022's work in: whole `cargo test --workspace --no-fail-fast` = 266 passed, 0 failed, 8 ignored under TZ unset, `Pacific/Kiritimati` and `Pacific/Pago_Pago`; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --check` clean; oracle hash over 310 files `aa6fcaed…bff92` unchanged; `git status --short -- src web tests pyproject.toml uv.lock` empty |
| Negative cases hold | ☑ pass | agent's red run (tests in, no implementation): `37 passed; 15 failed` (clap `unexpected argument '--static-dir'` or 404 on `/`). Mutation of the layers: removing the two `starts_with(root)` checks alone left the suite green (the `strip_prefix` check also holds); adding a dummy `strip_prefix` fallback turned three tests red (`/link-out answered 200: OUTSIDE-SECRET-MARKER-7731`); removing the textual dot rules too turned `/../outside/secret.txt` red. File restored and diffed after each step |
| verify | ☑ pass | Supervisor's own adversarial probe of a live `serve --static-dir` over a throwaway site next to an outside marker file: `/` 200 `text/html; charset=utf-8`; `/assets/app.js` 200 `text/javascript; charset=utf-8`; `..`, `%2e%2e`, `..%2f`, `%252e%252e`, `//etc/passwd`, a symlink to the outside file, a symlink to the outside folder, `.env`, `/assets` and an index-less `/assets/` all 404 with marker=0; `/api/usage/summary` with no token 403 and `/%61pi/usage/summary` and `//api/usage/summary` 404 even with a shadow file `api/usage/summary` in the site; bad Host and foreign Origin 403; `/api/version` with the token 200; the CSP on `/`; stdout 2 lines, stderr 0 bytes, fixture unchanged, no process left — pass. Not rerun: the agent's 88-request table and the FIFO test (they run in the suite) |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `rust/crates/app/src/{static_files,server,main}.rs`, `rust/crates/app/tests/server.rs`, `rust/README.md`; `api.rs`, `mcp.rs`, the core and `rust/parity/**` untouched by this task. code-review `reports/code-review_main_20261110T231000.html` (no P0/P1; 1 P2, 3 P3, 1 info) and security-review `reports/security-review_main_20261110T231100.html` (High risk; nothing at confidence 8+) |
| Full smoke suite still green (no regression) | ☑ pass | whole workspace 266 passed, 0 failed under three zones. No pre-existing test body changed: two helpers in `tests/server.rs` were extended (`Server::launch` delegates to `launch_args`; `Response` gained `bytes`); the 36 earlier tests and the no-flag pin pass unchanged. The `router(..)` wrapper became `#[cfg(test)]` so T022's in-process tests compile |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier is not installed in this session |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no UI (the dashboard is not built or loaded by this task) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no UI |
| **UI: Responsiveness at target viewports** | ☑ N/A | no UI |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-10T14:45:31Z by the Supervisor, before the agents' first edits:
```
$ grep -n "static-dir\|static_dir" rust/crates/app/src/*.rs | head -2
(no output: `serve` has no static option; `GET /` is 404)
```

**AFTER**: captured 2026-11-10 by the Supervisor, live probe over a throwaway site next to an outside marker file:
```
/ -> 200 text/html; charset=utf-8     /assets/app.js -> 200 text/javascript; charset=utf-8
/../outside/secret.txt 404   /%2e%2e/... 404   /..%2f... 404   /%252e%252e/... 404   //etc/passwd 404
/leak (symlink out) 404   /leakdir/secret.txt 404   /.env 404   /%61pi/usage/summary 404
/api/usage/summary (no token) 403   bad Host 403   foreign Origin 403   marker never in a body
```

**DELTA**: `serve --static-dir <DIR>` now serves the built dashboard from the API's own origin behind the same guard, so the P1b window can load `http://127.0.0.1:<port>/#<token>`; nothing outside the folder can be read.

**WITNESS**: Supervisor, 2026-11-10, reran the whole workspace under three zones, the oracle hash and the Python-tree check, and probed a live server with traversal, symlink, dotfile and guard cases independently of the implementing agent.

## Real dashboard check (close-out 2026-11-11)

The dashboard was built for the first time (`npm --prefix web ci` and `run build`, 496 KB, gitignored output). Supervisor runs against a temp copy of the rich synthetic store: `serve --static-dir <built bundle>`: `/` 200 `text/html`, the JS bundle 200 `text/javascript`, a CSP header on the response. `app --self-test --static-dir <built bundle>` (feature build, WSLg): `self-test: ok`, exit 0, no `easy-personal-rs` or WebKit process left. The dashboard calls 12 `/api` routes the Rust server does not serve yet (accounts, connectors, dirs, kit/*, live, memory-graph, reports, sync): those views are the P2/P3 ports listed in `rust/PROJECT_SPEC.md`.
