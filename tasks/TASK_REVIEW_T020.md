# TASK_REVIEW — T020: Sessions and outliers over HTTP and MCP

> Sibling of `tasks/TASK_GUIDE_T020.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/tests/server.rs` (36), `rust/crates/app/tests/mcp.rs` (28), `rust/crates/app/src/api_tests.rs` (36 unit): 20 HTTP goldens (default 4, rich 16) and 3 MCP goldens counted from the manifests; the six tools equal the golden `tools/list`; bad period before unknown scope; guard, HEAD/405 and store-state tests extended to six routes; seven more encoded-path forms; two mutation tests |
| Verification command run | ☑ pass | Supervisor, own runs: whole `cargo test --workspace --no-fail-fast` = 228 passed, 0 failed, 8 ignored under TZ unset, `Pacific/Kiritimati` and `Pacific/Pago_Pago`; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --check` clean; oracle hash over 310 files `aa6fcaed…bff92` unchanged; `git status --short -- src web tests pyproject.toml uv.lock` empty; sweep for 'not ported' / 'answer 404' / '404 for now' empty in README and `crates/app` |
| Negative cases hold | ☑ pass | agent's red run before the code: `tests/mcp.rs` 9 failed (`-32602 Unknown tool: session_outliers`, five tools listed), `tests/server.rs` 13 failed (every new route 404). Mutation: `UPDATE events SET ts_utc = '2026-01-15T10:01:30.000000Z' WHERE id = 1` makes the sessions comparison name `/api/usage/sessions?`; `tool_names || ',Edit'` on the 90-call session makes it 91 calls and differ from the golden |
| verify | ☑ pass | Supervisor's own live probe over loopback on a temp copy of the rich store (`TZ=UTC`, scratch HOME): `usage/outliers` 200; `usage/sessions?workspace=acme` 200 with `detail_sessions` 50; `since=bad` 400 `Invalid isoformat string: 'bad'`; `outliers?until=bad&workspace=nope` 400 (period first); `sessions?workspace=nope` 400 `unknown workspace`; `account=Claude+work` 200; no token 403; POST 405; stdout 2 startup lines, stderr 0 bytes, no process left. MCP run by hand: `tools/list` = ping, usage_summary, usage_breakdown, session_outliers, warnings, improvement_plan; `session_outliers {since:bad}` -> `isError` true `Error executing tool session_outliers: Invalid isoformat string: 'bad'`; no arguments -> answers — pass. The agent's Python wording check (pydantic text for a wrong type; extra args dropped) was not rerun |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `rust/crates/app/src/{api,api_tests,mcp}.rs`, `rust/crates/app/tests/{server,mcp}.rs`, `rust/README.md`; `rust/crates/core/**` untouched. code-review `reports/code-review_main_20261110T221000.html` (no P0/P1; 3 P3; 1 info) and security-review `reports/security-review_main_20261110T221100.html` (nothing at confidence 8+) |
| Full smoke suite still green (no regression) | ☑ pass | whole workspace 228 passed, 0 failed under three zones (213 before, +15). One pin test (the old 404) deleted and replaced; `FORBIDDEN_ROUTES` widened to six routes; the tools/list test renamed for six tools; each change listed by the agent |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier is not installed in this session |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no UI |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no UI |
| **UI: Responsiveness at target viewports** | ☑ N/A | no UI |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-10T14:33:21Z by the Supervisor, after the guide was written and before the agent's first edit:
```
$ grep -c "api/usage/sessions" rust/crates/app/src/api.rs
0
(the two routes answer 404; tools/list has five tools)
```

**AFTER**: captured 2026-11-10 by the Supervisor:
```
GET /api/usage/sessions?workspace=acme  -> 200 {"detail_sessions":50,"gap_minutes":30,...}
GET /api/usage/sessions?since=bad       -> 400 {"error":"Invalid isoformat string: 'bad'"}
GET /api/usage/outliers?until=bad&workspace=nope -> 400 (period first)
mcp tools/list -> ['ping','usage_summary','usage_breakdown','session_outliers','warnings','improvement_plan']
```

**DELTA**: the Rust server now serves the dashboard's session timeline and outlier list, and the Rust MCP server a sixth tool, `session_outliers`, with the same JSON and error text as Python for 23 goldens.

**WITNESS**: Supervisor, 2026-11-10, reran the workspace under three zones, the oracle hash, the Python-tree check, a live HTTP probe and an MCP run independently of the implementing agent.
