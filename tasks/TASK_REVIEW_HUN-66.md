# TASK_REVIEW — HUN-66: Move, alias, rename, delete to orphan

> Sibling of `tasks/TASK_GUIDE_HUN-66.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | pass (implementer) | `tests/connectors/test_workspace_ops.py` (20: AC 1-7 incl. keychain call log empty, dry-run rollback, conflicts, missing folder, mid-plan failure), `tests/connectors/test_workspace_ops_cli.py` (3), `tests/surfaces/test_api_workspace_ops.py` (4); `tests/leak/` registered. Combined: `27 passed` |
| Verification command run | pass (implementer) | ```Mon Oct 26 10:28:18 AM UTC 2026
469 passed, 3 deselected, 1 warning in 35.48s
694 passed, 4 deselected, 1 warning in 51.54s
All checks passed!
27 passed, 1 warning in 5.16s``` |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | pass (implementer) | `uv run pytest -q`: 694 passed, 4 deselected (perf) |
| easy-verifier score (advisory) | ☐ recorded / ☐ N/A | [overall result, abstaining dimensions, report path — or `N/A — <reason>` if the server is down; never blocks Done] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-26 10:16 UTC on branch HUN-66 before any implementation commit (temp HOME and agent root, fake `claude`, file-backed fake keyring; real home untouched):

```
$ date -u
Mon Oct 26 10:16:16 AM UTC 2026
$ easy-personal workspace add company
Using CPython 3.13.2
Creating virtual environment at: .venv
   Building easy-personal-mcp @ file:///home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-66
      Built easy-personal-mcp @ file:///home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-66
Installed 43 packages in 18ms
{
  "color": null,
  "description": null,
$ echo tok | easy-personal connector add slack --workspace company --yes
  store: connector c8a6d0d51, alias company-slack, workspace company
  Claude Code: no change now; `easy-personal connector sync` registers it
connector c8a6d0d51 (company-slack) added to workspace 'company'.
run `easy-personal connector sync` to register it in the accounts that picked this workspace.
$ easy-personal connector list (tail)
configured (id alias provider workspace)
  c8a6d0d51 company-slack slack company
$ easy-personal workspace add side
{
  "color": null,
$ easy-personal connector move <id> side
usage: easy-personal connector [-h] {list,add,remove,sync} ...
easy-personal connector: error: argument action: invalid choice: 'move' (choose from list, add, remove, sync)
exit=2
$ easy-personal connector alias <id> x
usage: easy-personal connector [-h] {list,add,remove,sync} ...
easy-personal connector: error: argument action: invalid choice: 'alias' (choose from list, add, remove, sync)
exit=2
$ easy-personal workspace delete company --yes
easy-personal: the workspace still has connectors; remove them first
exit=2
$ workspace rename (edit --rename) then connector list
{
  "color": null,
  "description": null,
  c8a6d0d51 company-slack slack firm
```

Note the last case: today a workspace rename leaves the connector alias `company-slack` while its workspace is now `firm`.

**AFTER**: 2026-10-26 10:28 UTC, same fakes, fresh temp HOME (implementer run; reviewer to confirm):

```
$ date -u
Mon Oct 26 10:28:03 AM UTC 2026
$ easy-personal connector move c72d03a3b side   (answer n)
will change (connector c72d03a3b): move to workspace 'side', alias company-slack -> side-slack
  Claude default: remove company-slack (exec c72d03a3b)
  Claude work: remove company-slack (exec c72d03a3b)
  Claude work: add side-slack -> exec c72d03a3b
  keychain: untouched
Apply these changes? [y/N] aborted; nothing was changed.
exit=1
$ easy-personal connector move c72d03a3b side --yes
will change (connector c72d03a3b): move to workspace 'side', alias company-slack -> side-slack
  Claude default: remove company-slack (exec c72d03a3b)
  Claude work: remove company-slack (exec c72d03a3b)
  Claude work: add side-slack -> exec c72d03a3b
  keychain: untouched
done: Claude default: remove company-slack (exec c72d03a3b)
done: Claude work: remove company-slack (exec c72d03a3b)
done: Claude work: add side-slack -> exec c72d03a3b
restart Claude Code in the changed accounts to use the change.
exit=0
$ easy-personal connector alias c72d03a3b team-chat --yes
will change (connector c72d03a3b): alias side-slack -> team-chat
  Claude work: remove side-slack (exec c72d03a3b)
  Claude work: add team-chat -> exec c72d03a3b
  keychain: untouched
$ easy-personal connector alias c72d03a3b --default --yes
will change (connector c72d03a3b): alias team-chat -> side-slack
  Claude work: remove team-chat (exec c72d03a3b)
  Claude work: add side-slack -> exec c72d03a3b
  keychain: untouched
$ easy-personal workspace edit side --rename firm --yes
will change (workspace 3, now 'firm'):
  connector c72d03a3b: alias side-slack -> firm-slack
  Claude work: remove side-slack (exec c72d03a3b)
  Claude work: add firm-slack -> exec c72d03a3b
  keychain: untouched
done: Claude work: remove side-slack (exec c72d03a3b)
done: Claude work: add firm-slack -> exec c72d03a3b
restart Claude Code in the changed accounts to use the change.
{
}
$ easy-personal workspace delete firm --yes
will change (delete workspace 'firm'):
  connectors to orphan: c72d03a3b
  account picks removed: 1; repo rows deleted: 0
  Claude work: remove firm-slack (exec c72d03a3b)
  keychain: untouched
done: Claude work: remove firm-slack (exec c72d03a3b)
restart Claude Code in the changed accounts to use the change.
exit=0
$ easy-personal connector list | tail -1
configured (id alias provider workspace)
  c72d03a3b firm-slack slack orphan
$ claude state per account
{"<unset>": {}, "/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/0a221257-03c2-404a-adc3-6706867aa89a/scratchpad/hun66b/agent-root/.claude-work": {}}$ keyring unchanged?
identical

```

**DELTA**: The owner can move a connector, change its alias, rename a workspace or delete a workspace (connectors go to orphan) and see the Claude-entry plan first, with the keychain untouched.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
