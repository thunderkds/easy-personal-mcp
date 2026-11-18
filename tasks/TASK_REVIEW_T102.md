# TASK_REVIEW — T102: Write sessions 0 — spike

> Sibling of `tasks/TASK_GUIDE_T102.md`. Filled by the Supervisor at Stage 4/5.

---

## Auto decisions

Mode: auto
1. Spike scope kept to new files only (`rust/spikes/write-sessions/`, `tasks/evidence-T102/`); no change to `rust/crates/` or any Claude settings file. RULE: `tasks/TASK_GUIDE_T102.md` § Files must NOT touch
2. SIGINT is replaced by the stdin `control_request` interrupt in FR-110: SIGINT ends the process. FACT: `tasks/evidence-T102/SPIKE.md` (AC 3, "SIGINT … the process exits (code 0)")
3. The permission prompt tool becomes a loopback HTTP route in the dashboard process with `MCP_TOOL_TIMEOUT`, with a stdio child + socket as the fallback. FACT: `tasks/evidence-T102/SPIKE.md` (AC 5, "Without `MCP_TOOL_TIMEOUT` claude cuts the request at 60 s")
4. Stage 4 for a spike = a scope and secret check of the branch (15 new files, no secret or transcript line) instead of the multi-reviewer code review: the scripts are throwaway and nothing in the app runs them. SAFER OPTION: the same review effort goes to T103 and T106, which carry these findings into code. ⚠ decided for you — please review
Checks: Verification Command `test -s tasks/evidence-T102/SPIKE.md …` — 8 AC rows, 15 scripts listed; real run: about 26 `claude --model haiku` probes by the agent (auto-checked, not /verify)
Memory: memory/decisions.md — the write sessions entry already names T102; spike findings live in `tasks/evidence-T102/SPIKE.md`
Not done: merge (admin: after the auto verify pass); auto verify: ☑ pass (session d9c66400-4138-4fc8-842c-0b20626f4c68, commit 369917147c08576a751076245732456886fc39fc, log .claude/verify-runs/T102-369917147c08.log)
Paused: done not moved — Gate 5: no test run in memory/event-trace/T102.jsonl

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | Spike: each AC is a probe script with its recorded output in `tasks/evidence-T102/SPIKE.md`. Re-run 2026-11-17 under T102: `test -s tasks/evidence-T102/SPIKE.md && grep -cE '^\| *[0-9]+ ' …` → `8`; `ls rust/spikes/write-sessions/` → 14 files (`cc.py`, `hook.py`, `p1_stream_json.py` … `p7_fork.py`, `perm_*_server.py`); `python3 -m py_compile` on each → no failure |
| Verification command run | ☑ pass | `SPIKE.md` non-empty, 8 AC rows, `rust/spikes/write-sessions/` lists 15 files |
| Negative cases hold | ☑ pass | Fail-open probes: hook with exit 0 and no output, and a hook timeout (8 s vs sleep 12), both leave the terminal dialog; an unknown result shape from the permission tool fails closed |
| verify | ☑ pass | auto-run by hook, session d9c66400-4138-4fc8-842c-0b20626f4c68, commit 369917147c08576a751076245732456886fc39fc, log .claude/verify-runs/T102-369917147c08.log — pass; done not moved: Gate 5: no test run in memory/event-trace/T102.jsonl |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Only new spike files; no app code touched |
| Full smoke suite still green (no regression) | ☑ N/A | No app code changed |
| **Companion score (easy-verifier)** | ☑ N/A | Spike scripts, not product code |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | No UI |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | No UI |
| **UI: Responsiveness at target viewports** | ☑ N/A | No UI |

---

## Demonstration

**BEFORE**: `PRD.md` FR-110 (as of `637cf2c`): "**Interrupt** sends SIGINT (as Esc in the CLI)"; FR-111: "The exact wiring (which MCP transport, how the tool reaches the running app) is fixed by the T102 spike."

**AFTER**: `tasks/evidence-T102/SPIKE.md`: a go/no-go per route on `claude` 2.1.326; interrupt = stdin `control_request`; permission tool = loopback HTTP in the dashboard process + `MCP_TOOL_TIMEOUT`; hook outputs confirmed, with fail-open.

**DELTA**: T103 and T106 build on routes proven on the installed `claude`, not on unverified docs.

**WITNESS**: the backend-developer agent ran the probes on 2026-11-17; the Supervisor checked the branch scope and the secret scan.
