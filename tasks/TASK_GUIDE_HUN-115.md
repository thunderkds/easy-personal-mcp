**Date**: 2026-10-29
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: *Workspace*, *Agent account*, *API-equivalent cost*)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Verify on fixtures only: temp `EASY_PERSONAL_HOME` + `EASY_PERSONAL_CLAUDE_PROJECTS` + `EASY_PERSONAL_AGENT_ROOT`; never open the real `~/.easy-personal/` or `~/.claude*/`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner issue 2026-10-29 #4: "for the agent tab by account, should show each account like the current status line".

The owner's status line (`~/.claude/statusline-command.sh`, shared by all accounts) shows: model, folder, branch, account email, context %, session time, and plan limits "🔄 5h N% (↻ reset)" / "📅 7d N% (↻ reset)". Claude Code never saves the plan limits to disk; they exist only in the JSON the status line receives on stdin.

Owner decision 2026-10-29: **disk data + plan limits** — the status line saves a small snapshot file; the dashboard reads it.

**Restated intent**:

> `/api/accounts` returns, per account, a `status_line` object read from disk only: `email`, `plan` (`oauthAccount.billingType`) and `organization` (`oauthAccount.organizationName`) from `<folder>/.claude.json` (no other key is read); `last_model` and `last_active_utc` from the account's newest stored event; `tokens_today` and `tokens_7d` from stored events; and `limits` from `<folder>/easy-personal-limits.json` (written by the owner's status line): `five_hour` / `seven_day` each `{used_percentage, resets_at}` plus `written_at`. Missing or unreadable files give `null` fields, never an error. The README documents the one-line status-line snippet that writes the snapshot.

**Out of scope**: the card UI (follow-up task); editing the owner's status line script (the Supervisor does that separately, with consent); live polling.

**Requirement Refs**: owner issue #4; FR-036.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's issue and decision (Supervisor, 2026-10-29)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: none. **Entry point**: `GET /api/accounts?by=account|provider`, and `easy-personal account list` if it prints accounts (add the email column there only if the list already exists).

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | Each account in `/api/accounts` (both groupings) has `status_line` with keys `email, plan, organization, last_model, last_active_utc, tokens_today, tokens_7d, limits` | status line data |
| 2 | Only `oauthAccount.emailAddress`, `.billingType`, `.organizationName` are read from `.claude.json`; a planted secret anywhere else in that file (e.g. `mcpServers.*.env`, `userID`) never appears in any API, MCP, CLI output or log | leak safety |
| 3 | `tokens_today` = sum of all token columns for the account since local midnight; `tokens_7d` = last 7×24 h; `last_model` / `last_active_utc` from the newest event of that account; NULL-account events count for the default account (existing rule) | usage per account |
| 4 | `limits` parses `easy-personal-limits.json` `{five_hour:{used_percentage,resets_at}, seven_day:{…}, written_at}`; `resets_at` epoch seconds → ISO UTC; a reset time in the past gives `stale: true`; malformed / missing file → `limits: null` | plan limits |
| 5 | Codex accounts: `email` etc. `null` (no `.claude.json` read) | provider scope |
| 6 | README section "Plan limits on the Agents page" gives the exact snippet for a status-line script (jq one-liner writing `${CLAUDE_CONFIG_DIR:-$HOME/.claude}/easy-personal-limits.json` atomically via temp file + `mv`) | snapshot writer |
| 7 | New reads registered in the leak suite (`tests/leak/harness.py` `API_CALLS`) with a planted `.claude.json` secret | leak suite rule |
| 8 | `/api/accounts` stays under 100 ms on the 1M-event benchmark store (HUN-61) — one grouped query, not one per account | speed |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | fixture `.claude.json` with email + planted `INVENTED_SECRET` in `mcpServers` env | email returned; secret absent everywhere | pytest + leak suite |
| 2 | events today 100 tokens, 3 days ago 50 | today 100, 7d 150 | pytest |
| 3 | limits file 5h 42.5%, resets in 1h | `used_percentage` 42.5, ISO reset, `stale` false | pytest |
| 4 | limits file with past reset | `stale` true | pytest |
| 5 | broken JSON limits file | `limits` null, 200 | pytest |
| 6 | benchmark store | `/api/accounts` < 100 ms | pytest benchmark marker |

### Verification Command (exact, runnable)

```bash
uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-115.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-115.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

Pure backend task: section N/A.

### 1. Visual Regression

☐ N/A — no UI change (card UI is the follow-up task)

### 2. Design-System Compliance

☐ N/A — no UI change

### 3. Layout / Responsiveness

☐ N/A — no UI change

---

## Approach

**Pattern reference**: `src/easy_personal/core/accounts.py` `_with_picks()` / `list_accounts()`; `metrics._scope`.
**Vital slice**: one helper `status_line(conn, rows)` that adds the object to every row with one grouped SQL query + per-account small file reads.
**Cut list**: per-session context % (not stored); 5h/7d computed from tokens (the plan's real limit is unknown); a CLI command that writes the snapshot.

---

## Edge Case Checklist

- [ ] `.claude.json` missing / not JSON / no `oauthAccount` → nulls
- [ ] limits file older than both resets → `stale` true
- [ ] account folder missing → nulls, status `missing` as today
- [ ] `percent` given as string or >100 → keep number as given if numeric, else null

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/accounts.py` | `status_line` |
| `tests/` | unit + API tests |
| `tests/leak/harness.py` + fixtures | planted `.claude.json` |
| `README.md` | snippet section |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/statusline-command.sh` and any real `~/.claude*/` | owner's files; Supervisor edits with consent later |
| `web/` | follow-up task |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |

---

## Test Plan

pytest for AC 1–8; leak suite run.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows N/A (Hard-Stop Gate 6)
- [ ] `Skill({ skill: "verify" })` run on fixtures
- [ ] Supervisor notified: task ready for Stage 4 review