**Date**: 2026-10-21
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P2
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read `agents/backend.md`
5. Note the **Complexity Level** above and apply the matching process (brainstorm / decompose / verify depth) from the Complexity matrix in your role guide (`agents/backend.md`)
6. **C2/C3 or multi-file tasks only**: read `memory/codebase-map.md` for directory layout, entry points, and blast-radius hotspots — skip if the task is C0/C1 and touches a single known file

---

## Requirement (Pillar 1 — Adapt the requirement)

Email and Messaging categories with the providers the owner named; when an official [claude.ai](<http://claude.ai>) connector exists it is offered first (PRD US-008, FR-016, FR-020).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Add catalog entries for Gmail, SendGrid, Teams, Discord (Slack exists), each with correct fields/secret flags and the claude.ai-connector flag + link where one exists.

**Out of scope** (what this task explicitly does NOT do):

* Calling provider APIs
* Creating [claude.ai](<http://claude.ai>) connectors

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-016
* FR-020

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-21)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (`grill-with-docs` terminology run 2026-10-21)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-45 — Connector tracer

**Entry point**: `easy-personal connector list` (CLI) and `connector_catalog` data

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | All entries pass the catalog schema | FR-016 |
| 2 | Entries with an official [claude.ai](<http://claude.ai>) connector carry its link and are ordered first | FR-020 |
| 3 | `connector list` groups by category | US-008 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Catalog load | 5 providers (slack, gmail, sendgrid, teams, discord), 2 categories (Email, Messaging), schema-valid, all commands pinned | automated |
| 2 | `easy-personal connector list` | grouped by category; gmail before sendgrid, slack/teams before discord; links shown; no secret values | automated CLI test |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/connectors/test_catalog.py -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-46.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-46.md` (review sections are never taken from the board).

## Approach

**Pattern reference**: The Slack entry from the connector tracer task

**Vital slice**: schema-valid entries
**Cut list**: No Teams/Discord launch verification beyond schema (documented as unverified if the provider server is unclear)

Servers are fixed by the owner (approved 2026-10-22, see `memory/decisions.md`); do not substitute. Record the npm URL in each entry's comment; mark setup steps you could not run as unverified.

| Entry | category | claude_ai_connector | command | field lines |
| -- | -- | -- | -- | -- |
| `gmail.yaml` | Email | yes (official Gmail connector) | `npx -y @gongrzhe/server-gmail-autoauth-mcp@1.1.11` | `GMAIL_OAUTH_PATH \| OAuth client keys file (gcp-oauth.keys.json) \| plain` — comment: one-time browser `auth` step per its README, unverified |
| `sendgrid.yaml` | Email | no | `npx -y sendgrid-mcp@1.4.0` | `SENDGRID_API_KEY \| API key (SG....) \| secret` |
| `teams.yaml` | Messaging | yes (Microsoft 365 connector) | `npx -y @softeria/ms-365-mcp-server@0.170.2 --org-mode` | `MS365_MCP_TENANT_ID \| Tenant ID \| plain` — comment: device-code login, server keeps its own token cache, unverified |
| `discord.yaml` | Messaging | no | `npx -y @pasympa/discord-mcp@2.2.0` | `DISCORD_TOKEN \| Bot token \| secret` |

Schema change (AC2): add a single key `claude_ai_link:` to `connectors/catalog.py` and `Entry`. Required (https URL) when `claude_ai_connector: yes`, refused when `no`. Value for every `yes` entry, including the existing `slack.yaml`: `https://claude.ai/settings/connectors` (owner check: a per-connector deep link may be preferred later).

`easy-personal connector list` does not exist yet: add it to `surfaces/cli.py` (prints entries grouped by category; within a category, entries with a claude.ai connector first, then by provider; shows the link). Print only id, provider, category, connector flag + link and field labels — never values. Cover it in the leak suite: `tests/leak/harness.py` lists `connector` under `CLI_KEYCHAIN_COMMANDS` (excluded); `connector list` reads no keychain, so add a `CLI_CALLS` case for it (or a flag variant) so its output is scanned.

---

## Edge Case Checklist

- [ ] Provider has no MCP server → entry marked unavailable, not guessed
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/catalog/*.yaml` | Entries |
| `src/easy_personal/connectors/catalog.py` | `claude_ai_link` key + validation |
| `src/easy_personal/surfaces/cli.py` | `connector list` |
| `tests/connectors/test_catalog.py`, `tests/surfaces/` CLI test, `tests/leak/harness.py` | Tests + leak registration |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Schema + ordering tests.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Security review: `Skill({ skill: "security-review" })` run (if Medium/High risk)
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md`'s Evidence table (Hard-Stop Gate 5)
- [ ] `Skill({ skill: "verify" })` run — feature confirmed working in running app
- [ ] `memory/MEMORY.md` updated (if new patterns or feedback learned)
- [ ] Supervisor notified: task ready for Stage 4 review