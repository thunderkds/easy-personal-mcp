**Date**: 2026-11-06
**Complexity Level**: C1
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (Critical Constraints: metadata only, no secrets in the store)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-022, FR-076; then `src/easy_personal/sources/claude_code.py` (`_bash_word`, `_signals`), `src/easy_personal/core/reports.py` (`_SECRET`), `src/easy_personal/core/live.py` (`_safe`, uses the same check), `tests/connectors/test_mcp_tools.py`, the leak suite (`tests/leak/`)

---

## Bug Fix Task Guide

### Mental Model (confirmed: auto (Supervisor), 2026-11-06 — from code reading and a /verify repro)

* Observed: ingest (scheduled ingest and the dashboard live tail, both through `claude_code.parse_record` → `store.save_file`) stores the first word of every Bash command as a `tool_signals` row `kind='bash'`. `/verify HUN-233` (2026-11-06) stored `sk-ant-api03-VERIFYSECRETKEY` from the command `sk-ant-api03-VERIFYSECRETKEY run`.
* Expected: a Bash first word that looks like a secret (the FR-076 check, `reports._SECRET`: key prefixes `sk-`, `ghp_`/`github_pat_`, `xox…`, `AKIA`/`ASIA`, `-----BEGIN`, or a 32+ letter-and-digit run) is not stored — no `tool_signals` row for it. Ordinary words (`git`, `uv`, `pytest`, `python3`) are stored as today. Domain signals are unchanged.
* Likely divergence point: `claude_code._bash_word` (src/easy_personal/sources/claude_code.py:64) — its docstring says "The 64-char cap plus the `_WORD` regex are the only guard against a token-like first word"; `_WORD` (line 58) accepts `sk-ant-api03-…`.
* Recent context: HUN-47 added `tool_signals`; HUN-233 (merged 2026-11-06) added the same `_SECRET` check to live labels (`live._safe`). No code reads `kind='bash'` rows (`connectors/suggestions.py:77` reads only `domain`).

### Intake

* Trigger: a transcript line with a `Bash` tool_use whose `command` starts with a secret-looking word (e.g. `sk-ant-api03-ABCDEF run`), then `easy-personal ingest` (or the dashboard live tail) on it.
* Severity: P1 (privacy rule broken: PROJECT_SPEC.md:118 bans secrets in the store; no outage)
* Affected area: `src/easy_personal/sources/claude_code.py` (`_bash_word`); tests

### Scope decision (auto Debate, 2026-11-06)

* Ingest check only. **No migration** to delete already-stored rows: the owner's real store (v11) has 39,300 bash rows and 0 secret-looking (read-only count); a delete is not undoable once the `.bak-v11` copy is pruned (store.py:61-62). See `tasks/TASK_REVIEW_<ID>.md` `## Auto decisions`.

### Complexity & Risk

* Complexity: C1 (one source function + tests)
* Risk: Medium (secret handling)

### Diagnosis Gates (Pillar 1 — must pass before any fix)

- [X] Phase 1 feedback loop built and running (a test that parses a planted `Bash` line and reads `tool_signals`)
- [X] Bug reproduces deterministically on the loop
- [X] 3–5 ranked falsifiable hypotheses listed (consistent with confirmed mental model)
- [X] Correct hypothesis identified via Phase 4 instrumentation

### Attempts Log (filled live during diagnosis — required if >1 hypothesis tested)

| \# | Hypothesis | Predicted signal | Actual result | Verdict |
| -- | -- | -- | -- | -- |
| 1 | `_bash_word`'s `_WORD` regex accepts a key-shaped first word and nothing else checks it | The loop stores `sk-ant-api03-abcdef`, `AKIAABCDEFGHIJKL`, `xoxb-1234567890`, `ghp_…`, a 40-char run; fails before the fix | 5 of 6 planted words stored (the `/tmp/sk-ant…` one included, its path dropped); 2 tests failed | Confirmed. Fixed here |
| 2 | A second writer of `tool_signals` skips `_bash_word` (live tail, store) | `grep` finds another producer of `("bash", …)` rows | Only `claude_code._signals` produces signals (`store.py:167` only reads `event["signals"]`); the live tail uses the same `parse_record` | Rejected |
| 3 | `store.save_file` adds the word after parse | A stored word differs from the parsed one | Stored value equals the parsed word (loop output) | Rejected |
| 4 | The pattern is too narrow to match the planted words | `SECRET.search` is None for a planted word | Each planted word matches (`sk-` prefix, `AKIA`, `xox`, `ghp_`, 32+ run); after the fix 0 rows | Rejected |
| 5 | Importing `reports` into `claude_code` is safe | `import easy_personal.sources.claude_code` works | `ImportError` (cycle `reports` → `rules` → `kit` → `sources.ui` → `claude_code`) | Rejected: pattern moved to `core/secretlike.py` |

### Fix Gates (Pillar 2)

- [X] Regression test written before the fix
- [X] Fix applied; regression test passes
- [X] Phase 1 loop no longer reproduces the bug
- [X] Fix matches the "Expected" definition above

### Cleanup Checklist (Pillar 3)

- [X] All \[DEBUG-...\] instrumentation removed (grep verified)
- [X] Throwaway prototypes deleted
- [X] Correct hypothesis stated in commit message
- [X] Post-mortem: what would have prevented this?

---

## Dependencies & Reachability

**Depends on**: None

**Entry point**: `claude_code._signals` → `_bash_word`, called from `claude_code.parse_record` on every ingest and live-tail line

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Ingesting `Bash` commands `sk-ant-api03-abcdef run`, `ghp_abcdefghijklmnop`, `AKIAABCDEFGHIJKL x`, `xoxb-1234567890 y` and a 40-character letter-and-digit first word stores no `tool_signals` row for them | PROJECT_SPEC.md:118, FR-076 check |
| 2 | `git status`, `uv run pytest`, `/usr/bin/python3 x.py` (→ `python3`), `FOO=1 pytest` (existing env-assign behaviour) store as today; `WebFetch` domain signals unchanged | FR-022 |
| 3 | One secret check: `_bash_word` uses the same pattern as `reports._SECRET` / `live._safe` (no copied regex). No import cycle (`python -c "import easy_personal.sources.claude_code"` and the full suite pass) | Surgical Changes |
| 4 | Leak suite: a planted secret-looking Bash first word is not in the store dump | PROJECT_SPEC.md:118 |
| 5 | Full `uv run pytest -q` passes | — |

---

## UI / Design Acceptance Criteria

N/A — pure backend bugfix.

---

## Evaluation & Acceptance

> Filled by the reviewer in `tasks/TASK_REVIEW_<this task ID>.md`. UI Evidence rows: ☐ N/A (pure backend).

### Verification Command (exact, runnable)

```bash
uv run pytest tests/sources tests/connectors/test_mcp_tools.py tests/leak -q && uv run pytest -q
```

## Approach

**Vital slice**: in `_bash_word`, return None when the word matches the shared secret pattern; update its docstring. If importing `core.reports` into `sources/claude_code.py` makes an import cycle, move the compiled pattern into one small shared place that `reports`, `live` and `claude_code` all import — never copy it.
**Cut list**: no migration, no change to `tool_signals` schema, no change to domain signals or suggestions.

---

## Edge Case Checklist

- [X] A word that only *contains* a short `sk-` later (e.g. `task-runner`) is still stored (the pattern has `\b` anchors; prove with a test)
- [X] `sk-learn`-like words are dropped (accepted loss; no reader uses bash rows)
- [X] `bash -c "sk-…"`: `_bash_word` takes the literal first word (`bash`), unchanged

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/sources/claude_code.py` | secret check in `_bash_word` |
| `tests/sources/test_claude_code.py` (or the existing parser test file) | AC 1–2 |
| `tests/leak/` | AC 4 |
| `src/easy_personal/core/reports.py`, `src/easy_personal/core/live.py` | only if the pattern must move to avoid an import cycle |
| `src/easy_personal/core/secretlike.py` (new) | the shared pattern (import cycle forced the move); `reports._SECRET` is now an alias of it, `live.py` unchanged |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/migrations/`, `src/easy_personal/core/store.py` | No migration (scope decision) |
| `src/easy_personal/connectors/suggestions.py` | Reader unchanged |
| `web/`, `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Out of scope / kit files |

## Questions Log

* 2026-11-06 DEBATE Should the fix also ship migration m0012 deleting already-stored secret-looking `tool_signals` rows? → (A) ingest check only, no migration (Advocate 1: (A) — write-time leak, no reader of `kind='bash'`, a DELETE migration is not undoable once `.bak-v11` is pruned; Advocate 2: (B) — rows stored before the fix stay on other installs, `.bak-v11` is a restore path; its own confidence low) — rule 2 safer option (can be undone, keeps all data, smaller change); owner store v11: 39,300 bash rows, 0 secret-looking ⚠ decided for you — please review