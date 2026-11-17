# TASK_REVIEW — T093: one .deb that installs on Ubuntu 22.04, 24.04 and 26.04

> Sibling of `tasks/TASK_GUIDE_T[NNN].md`. Everything here is **filled by the reviewer at Stage
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
1. Track extended, C2, Risk Low — RULE: `docs/claude-md/tracks.md` `## Track rule` (3+ files and a design choice; build tooling only, no runtime code, no user data)
2. Stage 0 and Stage 1 not run again; the pipeline starts at Stage 2 — FACT: rust/PROJECT_SPEC.md:36 "- **Approved by**: user 2026-11-10"
3. Read the bare `[admin]` after the error report as "fix this error", then the board loop — SAFER OPTION: the error was the only request in the session; T090 is held by another session (worktree `easy-personal-mcp-wt/T090`, commit 2026-11-16 14:16), T091 and T092 depend on it, T063 waits for the owner gate, so no other ticket is ready ⚠ decided for you — please review
4. Fix = build the package in an Ubuntu 22.04 container (opt-in), keep the host build as it is — SAFER OPTION: one `.deb` for 22.04, 24.04 and 26.04; the other paths were "upgrade the other machine" (not ours to do) and "build on the other machine" (needs Rust, Node and the dev libraries there) ⚠ decided for you — please review
5. Lowest Ubuntu version is 22.04; 20.04 is out of scope — FACT: rust/packaging/README.md:35 "`Depends` comes from `dpkg-shlibdeps` (GTK 3, WebKitGTK 4.1 and the C library)" (Ubuntu 20.04 has no WebKitGTK 4.1 package) ⚠ decided for you — please review
6. One new host tool, Docker, for the container build only (T087 said "no new tool") — SAFER OPTION: no other way gives a glibc 2.35 build with the Ubuntu 22.04 package names on this machine; the host build needs nothing new ⚠ decided for you — please review
7. Requirement Fidelity Gate signed `auto (Supervisor)` — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` row 13
8. Agent: common-infrastructure, on this session's CLI and model — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` rows 11 and 14
9. Container build is a flag of `build-deb.sh` (`--docker`), not a sibling script — SAFER OPTION: one script keeps staging and control steps in one place (guide: "Keep one build-deb.sh"); the container re-runs the same script with an internal `--in-container` flag ⚠ decided for you — please review
10. Web page built on the host, no Node in the image — FACT: guide Approach "nodejs from the Ubuntu 22.04 archive is too old for Vite. Simplest: build the web page on the host"
11. Both builds write the same file name; the one that ran last is in `rust/target/deb` (documented) — SAFER OPTION: the verification command, `test_deb.sh` and the unchanged host path all read that one path ⚠ decided for you — please review
12. Changelog date passed in as `EP_DEB_STAMP` from the host — FACT: in a worktree `.git` is a file that points outside the container mount, so `git log` would fail there and the date would be the build time
13. One extra install proof on `ubuntu:26.04` (one more image pulled) — SAFER OPTION: the README claims 26.04; the script default stays 22.04 and 24.04
Checks: tests `rust/packaging/build-deb.sh --docker && DEB=<deb> EP_DEB_EXPECT_BASE=jammy rust/packaging/test_deb.sh && rust/packaging/test_deb_install.sh` — exit 0 (agent); the Supervisor's own run of `EP_DEB_EXPECT_BASE=jammy rust/packaging/test_deb.sh` and `rust/packaging/test_deb_install.sh` — both `PASS`, exit 0; `shellcheck rust/packaging/*.sh` — exit 0; reviews code-review — 0 P0, 0 P1, 1 P2 fixed (29ef6f6); real run `rust/packaging/test_deb_install.sh` — printed `ok: ubuntu:22.04 printed 'easy-personal-rs 0.3.0'` and `ok: ubuntu:24.04 printed 'easy-personal-rs 0.3.0'` (auto-checked, not /verify)
14. Review fix: `README.md` said a 26.04 host build does not install on 24.04; it does (`libc6 (>= 2.39)` and the `t64` names exist on 24.04), so the line now names 22.04 only — FACT: `test_deb_install.sh` output "os: Ubuntu 24.04.5 LTS (glibc 2.39)"
Memory: nothing new
Not done: merge (yours: reply `T093 [admin]` or merge it yourself); auto verify: ☑ pass (session 8a21fb90-68a5-4fe7-8ffd-8401377da907, commit 614843ebdcc0ace4154ab7f37dc93aa2f11ef2b3, log .claude/verify-runs/T093-614843ebdcc0.log); readiness incomplete: root `PROJECT_SPEC.md` has no `## Readiness` section (yours)
Paused: auto-mode check failed — no test run in memory/event-trace/T093.jsonl — write /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d/$CLAUDE_CODE_SESSION_ID with the task ID (minimal-track step 1): `mkdir -p /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d && printf '%s\n%s\n' "T093" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d/$CLAUDE_CODE_SESSION_ID`, then rerun the task's tests so the trace records them under the task, then stop [report 3c4f7aab28a6]
Paused: done not moved — Gate 5: no test run in memory/event-trace/T093.jsonl; report incomplete: Checks:

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/packaging/test_deb.sh` (new check 1b `EP_DEB_EXPECT_BASE=jammy`, new negative check 6) and `rust/packaging/test_deb_install.sh` (new); output in the rows below and in Demonstration AFTER |
| Verification command run | ☑ pass | `rust/packaging/build-deb.sh --docker && DEB=$(ls rust/target/deb/easy-personal-rs_*_amd64.deb \| tail -1) EP_DEB_EXPECT_BASE=jammy rust/packaging/test_deb.sh && rust/packaging/test_deb_install.sh`, 2026-11-16T07:47:44Z to 07:51:48Z, exit 0. Tail: `ok: Depends: libc6 (>= 2.34), ... libglib2.0-0 (>= 2.65.1), libgtk-3-0 (>= 3.21.5) ...` / `PASS: .../easy-personal-rs_0.3.0_amd64.deb` / `os: Ubuntu 22.04.5 LTS (glibc 2.35)` ... `easy-personal-rs 0.3.0` / `os: Ubuntu 24.04.5 LTS (glibc 2.39)` ... `easy-personal-rs 0.3.0` / `PASS: ... installs on: ubuntu:22.04 ubuntu:24.04` |
| Negative cases hold | ☑ pass | RED before the build existed, on the host-built package (tests written first): `DEB=<host deb> EP_DEB_EXPECT_BASE=jammy test_deb.sh` printed `FAIL: Depends has libc6 (>= 2.39), newer than 2.35 (Ubuntu 22.04)`, `FAIL: Depends has a t64 package name`, `FAIL: Depends lacks libgtk-3-0`, `FAIL: Depends lacks libglib2.0-0`, then `the Ubuntu 22.04 Depends check failed`. `test_deb_install.sh` on the host-built package: `FAIL: ubuntu:22.04: the install failed` with `Depends: libc6 (>= 2.39) but 2.35-0ubuntu3.15 is to be installed`. Check 6 of `test_deb.sh` re-packs the container-built control file with `libc6 (>= 2.39)` and `t64` names, and the jammy check rejects it and names both (offline, every run). Docker down (`DOCKER_HOST=unix:///nonexistent.sock build-deb.sh --docker`): `build-deb: --docker needs the docker daemon, and it is not reachable`, exit 1. Bad flag: `usage: build-deb.sh [--docker]`, exit 2 |
| verify | ☑ pass | auto-run by hook, session 8a21fb90-68a5-4fe7-8ffd-8401377da907, commit 614843ebdcc0ace4154ab7f37dc93aa2f11ef2b3, log .claude/verify-runs/T093-614843ebdcc0.log — pass; done not moved: Gate 5: no test run in memory/event-trace/T093.jsonl |
| Review scope bounded ☑ pass | Reviewed `rust/packaging/*` and the install part of `README.md`; nothing under `rust/crates`, `web`, `src`, `docs` changed (`git diff --stat main`); no hub file |
| Full smoke suite still green ☑ pass | No Rust or Python code changed; `test_deb.sh` sections 1 to 6 pass on the container-built package; the host build still works (AC 5, AC 6) |
| **Companion score (easy-verifier)** | ☑ N/A | not run by the implementing agent (Supervisor Stage 4) |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | No page change (guide: UI / Design AC is N/A) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | No page change (guide: UI / Design AC is N/A) |
| **UI: Responsiveness at target viewports** | ☑ N/A | No page change (guide: UI / Design AC is N/A) |


**AC rows (all pass):**
- AC 1: `build-deb.sh --docker` exit 0, wrote `rust/target/deb/easy-personal-rs_0.3.0_amd64.deb`. First run 8 min 3 s (image build plus cold cargo cache, cargo 4 m 48 s); a rerun 25 s (image and `rust/target/jammy` reused).
- AC 2: `dpkg-deb -f <deb> Depends` = `libc6 (>= 2.34), libcairo2 (>= 1.10.0), libdbus-1-3 (>= 1.9.14), libgcc-s1 (>= 4.2), libgdk-pixbuf-2.0-0 (>= 2.36.9), libglib2.0-0 (>= 2.65.1), libgtk-3-0 (>= 3.21.5), libjavascriptcoregtk-4.1-0, libsoup-3.0-0 (>= 3.0.3), libwebkit2gtk-4.1-0 (>= 2.41.90), libayatana-appindicator3-1`. No `t64`; libc6 2.34 is not above 2.35.
- AC 3: `test_deb.sh` PASS on the container-built package with `EP_DEB_EXPECT_BASE=jammy` (sections 1 to 6). The host-built package fails check 1b (RED above). Check 6 proves the rejection offline.
- AC 4: `test_deb_install.sh` printed `ok: ubuntu:22.04 printed 'easy-personal-rs 0.3.0'` and `ok: ubuntu:24.04 printed 'easy-personal-rs 0.3.0'` (`apt-get install -y ./easy-personal-rs_0.3.0_amd64.deb`; `os: Ubuntu 22.04.5 LTS (glibc 2.35)`, `os: Ubuntu 24.04.5 LTS (glibc 2.39)`). Extra: `EP_INSTALL_IMAGES=ubuntu:26.04` printed `os: Ubuntu 26.04.1 LTS (glibc 2.43)` and `ok: ubuntu:26.04 printed 'easy-personal-rs 0.3.0'`.
- AC 5: `git diff main -- rust/packaging/build-deb.sh`: the default path runs the same commands in the same order. The only edits are the flag parsing, the web step inside `if [ "$mode" != container ]`, `target=` set before the cargo line, and `stamp=${EP_DEB_STAMP:-...}`. Proof by run: `rust/packaging/build-deb.sh` with no argument, started 2026-11-16T07:41Z, exit 0, release build 5 m 49 s, wrote the same path; its `Depends` began `libc6 (>= 2.39), ... libglib2.0-0t64 ..., libgtk-3-0t64 ...` (host style, as before).
- AC 6: after the container builds and the host `build-deb.sh` (host cargo build in `rust/target/release`), `find rust web src -user root` printed nothing. The container build then reran fine (cargo `Finished ... in 1.43s`), so the two target folders do not disturb each other.
- AC 7: read `rust/packaging/README.md` (Build table, host-build limit, `--docker` steps, install proof, full command) and `README.md` (Install part).
- AC 8: `shellcheck rust/packaging/*.sh` exit 0. No Rust file changed.

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-16T07:19:10Z and 07:19:26Z, before any implementation commit; host-built package `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/rust/target/deb/easy-personal-rs_0.3.0_amd64.deb`, read only):

```
$ dpkg-deb -f easy-personal-rs_0.3.0_amd64.deb Depends
libc6 (>= 2.39), libcairo2 (>= 1.10.0), libdbus-1-3 (>= 1.10), libgcc-s1 (>= 4.2), libgdk-pixbuf-2.0-0 (>= 2.36.9), libglib2.0-0t64 (>= 2.66.0), libgtk-3-0t64 (>= 3.21.5), libjavascriptcoregtk-4.1-0, libsoup-3.0-0 (>= 3.0.3), libwebkit2gtk-4.1-0 (>= 2.41.90), libayatana-appindicator3-1

$ docker run --rm -v <deb folder>:/deb:ro ubuntu:22.04 sh -c 'apt-get update; apt-get install -y /deb/easy-personal-rs_0.3.0_amd64.deb'
PRETTY_NAME="Ubuntu 22.04.5 LTS"
The following packages have unmet dependencies:
 easy-personal-rs : Depends: libc6 (>= 2.39) but 2.35-0ubuntu3.15 is to be installed
                    Depends: libglib2.0-0t64 (>= 2.66.0) but it is not installable
                    Depends: libgtk-3-0t64 (>= 3.21.5) but it is not installable
E: Unable to correct problems, you have held broken packages.
```

**AFTER** (2026-11-16, container-built package, same files):

```
$ rust/packaging/build-deb.sh --docker    # exit 0
$ dpkg-deb -f rust/target/deb/easy-personal-rs_0.3.0_amd64.deb Depends
libc6 (>= 2.34), libcairo2 (>= 1.10.0), libdbus-1-3 (>= 1.9.14), libgcc-s1 (>= 4.2), libgdk-pixbuf-2.0-0 (>= 2.36.9), libglib2.0-0 (>= 2.65.1), libgtk-3-0 (>= 3.21.5), libjavascriptcoregtk-4.1-0, libsoup-3.0-0 (>= 3.0.3), libwebkit2gtk-4.1-0 (>= 2.41.90), libayatana-appindicator3-1
$ rust/packaging/test_deb_install.sh
== ubuntu:22.04: apt-get install -y ./easy-personal-rs_0.3.0_amd64.deb
    os: Ubuntu 22.04.5 LTS (glibc 2.35)
    easy-personal-rs 0.3.0
ok: ubuntu:22.04 printed 'easy-personal-rs 0.3.0'
== ubuntu:24.04: apt-get install -y ./easy-personal-rs_0.3.0_amd64.deb
    os: Ubuntu 24.04.5 LTS (glibc 2.39)
    easy-personal-rs 0.3.0
ok: ubuntu:24.04 printed 'easy-personal-rs 0.3.0'
PASS: ... installs on: ubuntu:22.04 ubuntu:24.04
$ EP_INSTALL_IMAGES=ubuntu:26.04 rust/packaging/test_deb_install.sh
    os: Ubuntu 26.04.1 LTS (glibc 2.43)
ok: ubuntu:26.04 printed 'easy-personal-rs 0.3.0'
```

**DELTA**: A user on Ubuntu 22.04 can now install the package with `apt install ./easy-personal-rs_<version>_amd64.deb` (the same file also installs on 24.04 and 26.04). The host-built package stopped with `Depends: libc6 (>= 2.39) but 2.35-... is to be installed`.

**WITNESS**: run by the common-infrastructure agent in this worktree, 2026-11-16 07:19Z to 07:52Z. The trace file `memory/event-trace/T093.jsonl` is the independent record (Supervisor to confirm).

## Stage 4 (Supervisor, 2026-11-16)
- code-review (inline, scope: `rust/packaging/*`, `README.md` install part; `lane: C2, 6 files, 238 lines, FITS`): 0 P0, 0 P1. P2 fixed: `README.md` wrongly said a 26.04 host build does not install on 24.04. P3, not fixed: the image takes Rust `stable` at image build time through `rustup` (`curl | sh`), not a pinned version; the `Dockerfile` installs a few dev packages the build may not need. Entry point `rust/packaging/build-deb.sh` exists. Startup reads: all three Permanent-Rule reads named (`rust/PROJECT_SPEC.md` and the T087 guide read in part).
- Supervisor's own run in the worktree (auto-checked, not /verify): `EP_DEB_EXPECT_BASE=jammy rust/packaging/test_deb.sh` exit 0 (`PASS`, steps 1, 1b, 2-6 `ok:`); `rust/packaging/test_deb_install.sh` exit 0 (`ok: ubuntu:22.04 printed 'easy-personal-rs 0.3.0'`, `ok: ubuntu:24.04 printed 'easy-personal-rs 0.3.0'`); `shellcheck rust/packaging/*.sh` exit 0; `find rust web src -user root` printed nothing.
- security-review: not required (Risk Low). No secret, no maintainer script, the package content is unchanged; install checks run only in `docker run --rm` containers with the package folder mounted read-only.
