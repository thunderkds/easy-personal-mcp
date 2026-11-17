#!/usr/bin/env python3
"""SessionStart hook (T317, matcher `resume`) — a resume ends a session tag.

`[admin]` and `[ticket]` hold for a whole session (`lib/auto_mode.py` `session_tag`). A resume,
continue or fork carries the old transcript into a new run, so a tag typed there must not hold
again. On a `SessionStart` event whose `source` is `resume`, this hook writes the current UTC time
(ISO 8601) to `auto_mode.resume_mark_path(<session_id>)`:
`<$CLAUDE_TRACE_ANCHOR_DIR or ~/.claude/harness-anchors>/<repo slug>/resume-at-<session_id>`, beside
the auto verify run records, which `.claude/settings.json` denies agents write access to (T317 R2).
`session_tag` then ignores every tag typed before that time; the user types the tag again to turn it
back on.

It writes nothing for any other source (`startup`, `clear`, `compact`: a tag survives `/compact`),
for a session ID that is not a plain name, or for input it cannot read. It never blocks and never
decides a permission: it always exits 0 with no output.
"""
import json
import os
import sys
from datetime import datetime, timezone

HOOKS_DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HOOKS_DIR, "lib"))


def main():
    try:
        from project_root import exit_outside_kit_project
        exit_outside_kit_project()
        import auto_mode
        event = json.load(sys.stdin)
        if not isinstance(event, dict) or event.get("source") != "resume":
            return
        path = auto_mode.resume_mark_path(event.get("session_id"))
        if path is None:
            return
        os.makedirs(os.path.dirname(path), exist_ok=True)
        now = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
        tmp = f"{path}.{os.getpid()}.tmp"
        with open(tmp, "w", encoding="utf-8") as f:
            f.write(now + "\n")
        os.replace(tmp, path)  # replaces a symlink at `path`, never writes through it
    except SystemExit:
        raise
    except Exception as exc:
        print(f"[hook:resume-mark] no resume mark written ({type(exc).__name__})", file=sys.stderr)


if __name__ == "__main__":
    main()
    sys.exit(0)
