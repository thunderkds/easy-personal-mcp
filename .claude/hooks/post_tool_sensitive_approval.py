#!/usr/bin/env python3
"""
PostToolUse recorder (T314) — fires after every Edit and Write tool call.

Approvals are Edit/Write only: a Bash event records nothing. It records an approval only
for what `pre_tool_sensitive_path.py` asked. When that hook
asks about an Edit or Write in board mode `online` with an active task, it writes
`.claude/state/pending/<tool_use_id>.json` = `{"task_id": T, "keys": [...]}`. A PostToolUse
event with the same `tool_use_id` means the call ran, so the user said yes: that yes is the
only trust source (NFR-005 stays — board text approves nothing). This recorder then moves
exactly those keys under T into `.claude/state/sensitive-approvals.json` and deletes the
pending entry. It records only when the event's `permission_mode` is `default` or `auto`,
the modes where a hook `ask` shows the prompt. No pending entry -> nothing is recorded.

The keys, state checks and file writes are the PreToolUse hook's own code (imported, not
copied). Never breaks the session: any error -> exit 0, no output.
"""
import json
import os
import sys

RECORDING_MODES = ("default", "auto")


def record(event):
    hooks_dir = os.path.dirname(os.path.abspath(__file__))
    sys.path.insert(0, hooks_dir)
    import pre_tool_sensitive_path as hook  # exits 0 itself outside a kit project

    if hook.IMPORT_ERROR is not None:
        return
    if event.get("tool_name") not in hook.APPROVABLE_TOOLS:
        return  # decision 4: approvals are Edit/Write only; Bash always asks
    root = hook.project_dir()
    pending = hook.pending_file(root, event.get("tool_use_id"))
    if pending is None:
        return
    entry = hook.read_json_in_state(root, pending)
    if entry is None:
        return
    if os.path.lexists(pending) and not os.path.isdir(pending):
        os.remove(pending)
    if event.get("permission_mode") not in RECORDING_MODES or not isinstance(entry, dict):
        return
    task_id, keys = entry.get("task_id"), entry.get("keys")
    if not isinstance(task_id, str) or not task_id or not isinstance(keys, list):
        return
    keys = [k for k in keys if isinstance(k, str) and k and not hook.is_state_key(k)]

    data = hook.read_approvals(root)
    approved = data.setdefault(task_id, [])
    added = [k for k in dict.fromkeys(keys) if k not in approved]
    if added:
        approved.extend(added)
        hook.write_json_in_state(root, hook.approvals_file(root), data)


def main():
    try:
        record(json.load(sys.stdin))
    except BaseException:
        pass
    sys.exit(0)


if __name__ == "__main__":
    main()
