# TASK_REVIEW — HUN-72: Fix workspace review defects

> Sibling of `tasks/TASK_GUIDE_HUN-72.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☒ pass | `tests/connectors/test_workspace_defects.py` (D1 x4, D2 x2, D3 x2, D5 x2), `tests/surfaces/test_api_workspace_ops.py::test_pending_plan_has_no_abort_line` (D4), un-xfailed in `tests/e2e/test_workspaces_flow.py`: `test_doctor_is_clean_after_the_flow` (D2), `test_connector_add_prompt_at_stdin_eof_is_a_clean_error` (D5), `test_cli_remove_of_an_unknown_name_leaves_a_hand_made_server` (D1) |
| Verification command run | ☒ pass | `334 passed, 1 xfailed` (connectors/surfaces/e2e/leak, only xfail = tool_signals); `745 passed, 2 skipped, 4 deselected, 1 xfailed` (full); `All checks passed!` (ruff) |
| Negative cases hold | ☒ pass | foreign `slack` entry and hand-made `notes` untouched (no claude write); stored connector without keychain item still `FAIL`; EOF at prompt writes nothing; every default-account call logs `config_dir == <unset>` with `CLAUDE_CONFIG_DIR=/some/other` set |
| verify | ☑ pass | Supervisor 2026-10-26 on an isolated install (demo.sh: temp HOME/AGENT_ROOT, fake `claude`, file keyring) with this session's own `CLAUDE_CONFIG_DIR` set: D1 `connector remove notes --yes` -> `easy-personal: unknown connector; nothing was removed` rc=2, hand-made `notes` kept; `connector remove sendgrid --yes` with a foreign entry -> `...did not create; nothing was removed.` rc=1, entry kept; D5 `connector add sendgrid --workspace company < /dev/null` -> `easy-personal: input ended at the prompt; nothing was written` rc=2, no traceback; D2 after adopt-legacy + move + sync -> `doctor` rc=0; D3 fake log: 0 default-account calls with an inherited CLAUDE_CONFIG_DIR; D4 `PUT /api/connectors/<id>/alias` confirm:false -> pending:true, last line `  keychain: untouched` (no abort line) — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed full src diff (writer.remove/doctor, cli `_ask`/remove fallback/doctor, api `_run_writer`/`_configured`, suggestions `_configured`) and the 4 adjusted tests (each now supplies a launcher entry, matching the new ownership rule — not weakened). Security (High risk, inline): removal now requires our launcher entry in the default account; no secret in new messages; keychain untouched on refusal. Decisions accepted: API unknown name stays 404; foreign entry blocks whole removal; exit 2 for unknown name. Added: e2e mtime guard skips ~/.codex* dirs (audit hook still covers) — pass |
| Full smoke suite still green (no regression) | ☒ pass | `745 passed, 2 skipped, 4 deselected, 1 xfailed`. Note: `test_nothing_outside_the_temp_machine_is_touched[cli]` failed once because the owner's real Codex app-server (VS Code extension, pid running) wrote `~/.codex` during the run; it passed on the full rerun. Environmental, not caused by this change. Fixed in a follow-up commit: `real_mtimes` in `tests/e2e/e2e_support.py` now also skips `~/.codex*` directories (audit hook still covers them); rerun: `334 passed, 1 xfailed` / `747 passed, 4 deselected, 1 xfailed` / ruff clean. |
| easy-verifier score (advisory) | ☐ N/A | N/A — easy-verifier MCP server not loaded in this profile |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | N/A — backend only; API plan lines change only by dropping the abort line |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | N/A — backend only |
| **UI: Responsiveness at target viewports** | ☑ N/A | N/A — backend only |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: 2026-10-26T15:19:19Z, before any fix, `uv run pytest <new tests> tests/e2e -q --runxfail`:
```
FAILED test_remove_of_a_name_that_is_not_a_catalog_id_changes_nothing
FAILED test_remove_of_a_catalog_id_with_a_foreign_entry_leaves_it_and_the_keychain
FAILED test_remove_of_a_keychain_only_legacy_id_makes_no_claude_write
FAILED test_default_account_calls_unset_an_inherited_config_dir
FAILED test_configured_checks_unset_an_inherited_config_dir
FAILED test_doctor_skips_ids_that_are_stored_connectors
FAILED test_connector_add_prompt_at_eof_is_one_line[slack] / [gmail]
FAILED test_pending_plan_has_no_abort_line
FAILED e2e test_doctor_is_clean_after_the_flow
FAILED e2e test_connector_add_prompt_at_stdin_eof_is_a_clean_error
FAILED e2e test_cli_remove_of_an_unknown_name_leaves_a_hand_made_server
FAILED e2e test_copied_transcript_counts_tool_signals_once   (out of scope, stays xfail)
13 failed, 28 passed
```
(The tests and the fix are in one commit; this capture was taken before the fix existed.)

**AFTER**: same tests: all pass except the out-of-scope tool_signals xfail (`334 passed, 1 xfailed`; full suite `745 passed, 2 skipped, 4 deselected, 1 xfailed`; ruff clean).

**DELTA**: `connector remove`, `doctor` and the dashboard act only on the default account with an ours-only rule, and no longer delete a hand-made server, fail a healthy adoption or print a traceback.

**WITNESS**: Supervisor, 2026-10-26 ~22:50 +07 — drove D1–D5 on an isolated install of branch HUN-72 (outputs in the verify row).
