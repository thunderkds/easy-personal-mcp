**Date**: 2026-10-21
**Complexity Level**: C2
**Risk Level**: High
**Priority**: P2
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read `agents/frontend.md`
5. Note the **Complexity Level** above and apply the matching process (brainstorm / decompose / verify depth) from the Complexity matrix in your role guide (`agents/frontend.md`)
6. **C2/C3 or multi-file tasks only**: read `memory/codebase-map.md` for directory layout, entry points, and blast-radius hotspots — skip if the task is C0/C1 and touches a single known file

---

## Requirement (Pillar 1 — Adapt the requirement)

Pick a category and provider and get the right config form on the dashboard; the [claude.ai](<http://claude.ai>) connector link is offered first; writes are guarded and secrets never come back (PRD US-008, FR-020, FR-023, NFR-008; DDR-0001).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Connectors view: catalog by category, provider form generated from the catalog, confirmation showing what will change, submit to a guarded write endpoint that reuses the CLI's connector add/remove path; status shows configured yes/no.

**Out of scope** (what this task explicitly does NOT do):

* New write targets beyond DDR-0001

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-020
* FR-023
* NFR-008
* DDR-0001

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-21)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (`grill-with-docs` terminology run 2026-10-21)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-49 — Dashboard server; HUN-45 — Connector tracer

**Entry point**: Dashboard route `/connectors` and `POST /api/connectors`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Form fields come from the catalog entry; secret fields are password inputs | FR-023 |
| 2 | [claude.ai](<http://claude.ai>) connector link shown first when flagged | FR-020 |
| 3 | Write endpoint requires token + same Host/Origin; 403 otherwise | NFR-008 |
| 4 | Responses and re-rendered forms never contain secret values (leak suite on the API) | NFR-002 |
| 5 | Write goes through the same code path as `connector add` (one core) | FR-008, DDR-0001 |
| 6 | Confirmation step shows the `claude mcp` change before writing | DDR-0001 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Cross-origin POST | 403 | automated |
| 2 | Valid submit with fake keyring + fake claude | connector configured, no secret echoed | automated + easy-ui session |

### Verification Command (exact, runnable)

```bash
npm --prefix web test && uv run pytest tests/surfaces/test_api_connectors.py tests/leak -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-52.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-52.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

> UI evidence is captured with the easy-ui container during development (PRD Open Question 9): QA drives the running dashboard with easy-ui and attaches the session JSON + screenshots.

### 1. Visual Regression

| Screen / Component | Verification method | Expected result |
| -- | -- | -- |
| Connectors view (catalog, provider form, confirmation, status) | easy-ui screenshot + LLM vision review against this guide's AC | matches AC; no broken layout |

### 2. Design-System Compliance

| Criterion | Verification method | Expected result |
| -- | -- | -- |
| Colors match design tokens | CSS audit of `web/src` tokens file | all colors come from the tokens file |
| Typography matches spec | computed style via easy-ui | one font family, sizes from tokens |
| Spacing / layout matches spec | easy-ui screenshot + LLM vision | consistent spacing scale |

### 3. Layout / Responsiveness

| Viewport | Verification method | Expected result |
| -- | -- | -- |
| Mobile (320–480px) | easy-ui screenshot | single column, no horizontal scroll |
| Tablet (768px) | easy-ui screenshot | single or two columns, readable charts |
| Desktop (1024px+) | easy-ui screenshot | multi-panel layout |

---

## Approach

**Pattern reference**: Connector tracer (CLI path) and dashboard server guard

**Vital slice**: one provider form end to end
**Cut list**: No editing of existing connectors (remove + add)

Backend endpoint thin wrapper over connectors core; frontend form generator from catalog schema.

Supervisor design (2026-10-22, after HUN-45/46 merged):

* `GET /api/connectors`: catalog entries (id, category, provider, claude_ai_connector, claude_ai_link, fields as {env, label, secret}) + `configured` yes/no per entry (keychain item present AND `claude mcp get <id>` succeeds; reuse writer helpers, never return values). Order like `easy-personal connector list` (category, connector-first, provider).
* `POST /api/connectors` body `{"provider": id, "values": {ENV: value}, "confirm": bool}` → call `writer.add(catalog.load(id), prompt=<lookup in values>, confirm=lambda: False, out=<collect lines>)` when `confirm` is false: the collected lines ARE the confirmation plan (writer already masks secrets as `***` and writes nothing on abort). With `confirm: true` call it with `assume_yes=True`. Return `{"status": rc, "lines": [...]}`. Same for `DELETE /api/connectors/<id>` via `writer.remove`. One core: no new write logic in `api.py` (AC5).
* Both write routes use the existing token + Host/Origin guard; additionally require `Content-Type: application/json` (blocks simple cross-site form posts). Values are never logged, never echoed, never stored outside the keychain.
* Catalog now has 5 entries (slack, gmail, sendgrid, teams, discord) and a `claude_ai_link` key (HUN-46).
* Register new routes in `tests/leak/harness.py` `API_CALLS` (write routes against the fake keychain + fake `claude`, see `tests/connectors/conftest.py`; set `FAKE_CLAUDE_GET_ONLY` when faking a registered connector).

---

## Edge Case Checklist

- [ ] Keychain missing → form shows the refusal guidance
- [ ] `claude` missing → snippet displayed
- [ ] Double submit
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/**` | Connectors view |
| `src/easy_personal/surfaces/api.py` | Write endpoint |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

API guard tests, leak suite, Vitest, easy-ui session.

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