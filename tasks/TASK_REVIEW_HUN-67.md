# TASK_REVIEW — HUN-67: Adopt pre-workspace connectors into orphan

> Sibling of `tasks/TASK_GUIDE_HUN-67.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | pass | `tests/connectors/test_legacy.py` (14, incl. hand-made entry cases), `tests/surfaces/test_api_legacy.py` (3); leak: `tests/leak/test_connector_leak.py`, `tests/leak/harness.py` |
| Verification command run | pass | `uv run pytest tests/connectors tests/surfaces tests/leak -q` -> `269 passed, 1 warning in 66.33s`; `uv run pytest -q` -> `682 passed, 2 skipped, 4 deselected, 1 warning in 87.86s`; `uv run ruff check .` -> `All checks passed!` |
| Negative cases hold | pass | declined, remove failure (row kept, pending), no keychain backend, other account untouched, hand-made same-name entry never removed, non-catalog keychain item ignored |
| verify | ☑ pass | Supervisor 2026-10-26T10:26:34Z: isolated install, fake claude default account holding `slack` (ours: launcher `exec slack`), `gmail` (hand-made npx) and `discord` (hand-made); file keyring holding easy-personal/slack + easy-personal/gmail → doctor 'note legacy connectors not adopted: gmail, slack'; `connector adopt-legacy --yes` → plan lists gmail 'not an easy-personal entry; left as it is', slack 'remove slack'; only claude write = `mcp remove slack --scope user` with CLAUDE_CONFIG_DIR <unset>; Claude state keeps gmail + discord; keyring unchanged; store: gmail and slack in orphan; second run 'nothing to adopt' — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed `connectors/legacy.py` (detect/adopt/doctor), cli `connector adopt-legacy`, api GET/POST `/api/connectors/legacy`, leak harness + connector leak test. code-review: P0 0 / P1 1 FIXED (a hand-made same-name MCP server in the default account would have been removed — now only entries pointing at easy-personal-launch `exec <name>` are removed; tests for both hand-made cases) / P2 0 / P3 1 FIXED (internal task ID in user text). security-review (inline): keychain read-only (presence check), only the default account env, catalog ids only, argv lists, no secret in output (keychain canary leak test passes); no findings. migration-safety: N/A (no schema change). |
| Full smoke suite still green (no regression) | pass | 682 passed (above) |
| easy-verifier score (advisory) | ☑ N/A | N/A — easy-verifier MCP is registered only in the `~/.claude-personal` profile; this Supervisor session runs another profile, tool not loaded |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Pure backend task; no UI file changed |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Pure backend task; no UI file changed |
| **UI: Responsiveness at target viewports** | ☑ N/A | Pure backend task; no UI file changed |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (temp HOME/agent root, fake `claude` with `slack` in the default account, file keyring with `easy-personal/slack`; captured before any implementation commit):

```
Mon Oct 26 10:16:20 AM UTC 2026
$ easy-personal connector adopt-legacy
usage: easy-personal connector [-h] {list,add,remove,sync} ...
easy-personal connector: error: argument action: invalid choice: 'adopt-legacy' (choose from list, add, remove, sync)
$ easy-personal connector list (tail)
  teams Teams | claude.ai connector: yes (https://claude.ai/settings/connectors)
    fields: Tenant ID
  discord Discord | claude.ai connector: no
    fields: Bot token
(rc above); keychain has easy-personal/slack:
['easy-personal/slack']
claude default-account entries:
['slack']
connectors table:
0
```

**AFTER** (same temp setup, after the change):

```
Mon Oct 26 10:21:31 AM UTC 2026
$ easy-personal doctor | grep legacy
note legacy connectors not adopted: slack; run `easy-personal connector adopt-legacy`
$ easy-personal connector adopt-legacy --yes
pre-workspace connectors found:
  slack: keychain yes, Claude Code default account yes
will change:
  store: adopt slack (id slack, alias slack) into orphan
  Claude Code (default account): remove slack
  keychain: untouched
done: slack adopted into orphan.
done: removed slack from Claude Code (default account).
assign them to a workspace with a move (HUN-66) when you are ready.
$ again
nothing to adopt: no pre-workspace connector found.
$ connector list (tail)
configured (id alias provider workspace)
  slack slack slack orphan
keychain:
['easy-personal/slack']
claude default entries:
[]
```

**DELTA**: The owner can run `easy-personal connector adopt-legacy` to put pre-workspace connectors such as `slack` into orphan and remove them from the default Claude account, with the keychain untouched.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
