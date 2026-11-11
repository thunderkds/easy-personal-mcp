# TASK_REVIEW — T003: Parity oracle — golden JSON from the Python app

> Sibling of `tasks/TASK_GUIDE_T003.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | the generator is the oracle; it fails the run if any GET route or MCP tool is neither golden nor skipped; mutation check: one changed `output_tokens` changed exactly the dependent golden files |
| Verification command run | ☑ pass | Supervisor re-ran `gen_golden.py` twice to `/tmp/v-a` and `/tmp/v-b`: both exit 0, `diff -r` empty, fresh `golden/` equals committed `golden/`; leak grep for `/home/`, `/Users/`, `ghp_`, `AKIA`, PEM headers over golden and the store: no match; agent also ran `pytest tests/surfaces/test_api.py -q`: 19 passed |
| Negative cases hold | ☑ pass | `EASY_PERSONAL_HOME=$HOME/.easy-personal gen_golden.py` → `refused: ... nothing was opened`, exit 1, no output folder created (Supervisor re-ran); `git status --short -- src web tests pyproject.toml uv.lock` empty |
| verify | ☑ pass | generator run end to end by the Supervisor: 52 golden entries, 18 skipped with reasons, store-v11.sqlite `user_version` 11, `integrity_check` ok, 39 events — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | code-review of `rust/parity/gen_golden.py` still to run |
| Full smoke suite still green (no regression) | ☑ pass | `tests/surfaces/test_api.py` 19 passed. Unrelated: `tests/test_kit_local_edits.py::test_stage4_always_runs_the_easy_verifier_score` fails on kit files modified before this work (agent report) — not re-run by the Supervisor |
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

**BEFORE**: captured 2026-11-10T08:14:15Z by the Supervisor, before the agent's first command:
```
$ ls rust; ls rust/parity; ls rust/parity/.venv
PROJECT_KANBAN.md
PROJECT_SPEC.md
docs
spikes
ls: cannot access 'rust/parity': No such file or directory
ls: cannot access 'rust/parity/.venv': No such file or directory
```
(no `rust/parity/`, no golden files, no Python venv; system Python 3.14 has no `starlette`, `pytest` or `uv`.)

**AFTER**: captured 2026-11-10 by the Supervisor:
```
$ ls rust/parity/golden | wc -l
53            (52 golden entries + manifest.json; 18 skipped, each with a reason)
$ python3 -c "...user_version/integrity_check on store-v11.sqlite"
user_version 11 integrity ok   (events 39, 163840 bytes)
$ diff -r /tmp/v-a /tmp/v-b
(identical)
```

**DELTA**: the Rust build now has a frozen, deterministic record of what the Python app answers for every read-only route and MCP read tool, plus a version-11 fixture store, to be proven against.

**WITNESS**: Supervisor re-ran the generator twice, the leak grep, the store check and the negative check after the backend agent's report (2026-11-10).


## Independent QA read (qa-expert, 2026-11-10; Supervisor accepted)

- All 8 Acceptance Criteria pass with evidence: two runs plus 9 more under other hash seeds, locale, TZ and cwd are byte-identical; a mutation of `output_tokens` changes exactly the dependent goldens; the committed store replays forward and in reverse; the home guards refuse each protected variable and a symlink; `git status --short -- src web tests pyproject.toml uv.lock` is empty; `ruff check` passes.
- Safety: no path to the real home, keychain, `claude` CLI, systemd or the network was found.
- **Not trusted yet** (the oracle proves shape only): warnings, plan, outliers, kit, board, accounts, workspaces, reports, memory graph, sync, `first_run`, the connector "configured" state, `api__usage_sessions__bad_since` (CPython text), and the `tools_list` schema text. Trustworthy: `usage_summary`, `usage_breakdown`, `usage_sessions`, the investigate prompt, `ping`, the connector catalog, the store.
- Follow-ups recorded on the board: **T007** (the three P1s: richer store, scope parameters, key order) and **T011** (the P2/P3 generator fixes). Sorted keys won over "preserve key order": T007 either adds a body digest or states that order is not compared.
