**Date**: 2026-10-22
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read `agents/backend.md`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-10-22: "did you update the readme … to deploy the guide also". The README still says "Status: planning" and has no install, usage or deploy steps.

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Rewrite `README.md` so a developer can install, run, register and deploy easy-personal v0.1.0 from it alone: pitch line, what it reads (metadata only), install, first ingest, MCP registration, dashboard, daily schedule, connectors (catalog, add/remove/list/doctor, keychain), config (`~/.easy-personal/config.yaml` keys), upgrade, rollback summary linking `RUNBOOK.md`, and the Easy Kit links — with a test that fails when a documented command does not exist in the CLI.

**Out of scope** (what this task explicitly does NOT do):

* The landing site under `site/` (separate task; it reads this README)
* Any change to `src/` behaviour
* Editing `RUNBOOK.md` beyond fixing a command that the README test proves wrong

**Requirement Refs**: none in `PRD.md` by design (documentation); ACs trace to the owner request above and `RUNBOOK.md`.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner answer "Yes, as described", 2026-10-22)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (Source, Kit part, Launcher, Surface)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] Requirement Refs: none by design

---

## Dependencies & Reachability

**Depends on**: none (all v0.1.0 features merged; `RUNBOOK.md` committed)

**Entry point**: `README.md`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | README has sections, in order: pitch (first paragraph, one-line pitch the site can quote verbatim), What it reads, Install, Quick start (ingest, MCP registration, dashboard), Schedule, Connectors, Configuration, Upgrade & rollback (links `RUNBOOK.md`), Development, Easy Kit; "Status: planning" is gone and the status names v0.1.0 | owner request |
| 2 | Every `easy-personal <subcommand>` and `easy-personal-launch <subcommand>` shown in a README code block exists in the real argparse parser (`surfaces/cli.py` `build_parser`, launcher) — `tests/test_readme.py` derives the subcommand set from the parser, not a hard-coded list | "deploy guide" must be runnable |
| 3 | Every MCP tool name mentioned in the README is registered by `surfaces/mcp.py` and every registered tool is listed (test derives the set from the server) | drift |
| 4 | Install / upgrade commands match `RUNBOOK.md` (`uv tool install --reinstall --no-cache .`, `npm --prefix web ci && npm --prefix web run build` before install) | deploy guide |
| 5 | README contains no absolute host path (`/home/`, `/Users/`, `/tmp/`), email address or token-shaped string | safety |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | README + parser + MCP server | commands and tools agree | `tests/test_readme.py` |
| 2 | Isolated install following the README step by step | every README command runs (with temp `EASY_PERSONAL_HOME`, fixtures) | Supervisor verify |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/test_readme.py -q && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-56.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-56.md` (review sections are never taken from the board).

## Approach

**Pattern reference**: `../easy-verifier-mcp/README.md` (Running it / CLI / MCP / Docker sections) and `../easy-ui-mcp/README.md` (Quick Start)

**Vital slice**: Install + Quick start + Connectors + Upgrade & rollback, all commands test-backed
**Cut list**: no per-rule reference table (rules live in `data/rules.yaml`), no screenshots (the site has them), no architecture deep-dive (link `PROJECT_SPEC.md`)

Facts to use (verify each against the code, do not copy blindly): CLI subcommands in `src/easy_personal/surfaces/cli.py`; config keys `ui: reports_dir:`, `verifier: reports_dir:` in `~/.easy-personal/config.yaml` (env overrides `EASY_PERSONAL_UI_REPORTS`, `EASY_PERSONAL_VERIFIER_REPORTS`, `EASY_PERSONAL_CONTEXT_URL`, `EASY_PERSONAL_HOME`, `EASY_PERSONAL_CLAUDE_PROJECTS`); MCP registration of the server itself via `claude mcp add` (check the exact form the project expects — stdio `easy-personal mcp`); connectors register `easy-personal-launch exec <id>` through `claude mcp add-json --scope user` (DDR-0001); dashboard binds 127.0.0.1 with a per-launch token in the URL fragment. The dashboard static build is gitignored: building `web/` before `uv tool install` is mandatory from source.

---

## Edge Case Checklist

- [ ] A command that needs a running `claude` CLI (connector add/remove, doctor) says so
- [ ] Cost is described as "API-equivalent cost", never as a bill
- [ ] Planted secret/text fixtures never appear in the README

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `README.md` | Rewrite |
| `tests/test_readme.py` | New drift test |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/**` | Docs task |
| `~/.claude.json`, real keychain, `~/.claude/projects/` | Never in verify |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

`tests/test_readme.py` (parser-derived subcommands, server-derived tools, safety patterns).

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md`
- [ ] Supervisor notified: task ready for Stage 4 review