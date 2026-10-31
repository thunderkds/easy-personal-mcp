"""Latency reference reader (HUN-165).

Returns {tool_use_id: (tool_name, transcript_timestamp)} for a session id, reading only the
tool_use id, tool name and record timestamp from the transcript (main file plus subagents/).
Each record is dropped as soon as the three fields are taken; no text is kept or printed.
The transcript is opened read-only.

Usage: python reference.py SESSION_ID [--config-dir DIR] [--json]
"""
import argparse
import json
import os
import sys
from datetime import datetime
from pathlib import Path

DEFAULT_CONFIG_DIR = Path(os.environ.get("CLAUDE_CONFIG_DIR", "/home/hungnguyenhuu/.claude-hung2"))


def transcript_files(session_id: str, config_dir=DEFAULT_CONFIG_DIR) -> list[Path]:
    """Main transcript(s) plus <session>/subagents/*.jsonl for a session id."""
    projects = Path(config_dir) / "projects"
    found = sorted(projects.glob(f"*/{session_id}.jsonl"))
    found += sorted(projects.glob(f"*/{session_id}/subagents/*.jsonl"))
    return found


def read_file(path) -> dict[str, tuple[str, str]]:
    """Stream one transcript file line by line; keep (name, timestamp) per tool_use id."""
    out: dict[str, tuple[str, str]] = {}
    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            try:
                rec = json.loads(line)
            except ValueError:
                continue  # partial or foreign line
            if not isinstance(rec, dict):
                continue
            ts = rec.get("timestamp")
            msg = rec.get("message")
            content = msg.get("content") if isinstance(msg, dict) else None
            if isinstance(content, list) and isinstance(ts, str):
                for block in content:
                    if isinstance(block, dict) and block.get("type") == "tool_use":
                        tid, name = block.get("id"), block.get("name")
                        if isinstance(tid, str) and isinstance(name, str):
                            out.setdefault(tid, (name, ts))
            del rec, msg, content  # drop the record
    return out


def read_reference(session_id: str, config_dir=DEFAULT_CONFIG_DIR) -> dict[str, tuple[str, str]]:
    """{tool_use_id: (tool_name, transcript timestamp)} for a session (incl. subagents)."""
    out: dict[str, tuple[str, str]] = {}
    for path in transcript_files(session_id, config_dir):
        for tid, val in read_file(path).items():
            out.setdefault(tid, val)
    return out


def to_epoch_ms(ts: str) -> float:
    """ISO-8601 transcript timestamp ('...Z' allowed) -> epoch milliseconds."""
    return datetime.fromisoformat(ts).timestamp() * 1000.0


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("session_id")
    ap.add_argument("--config-dir", default=str(DEFAULT_CONFIG_DIR))
    ap.add_argument("--json", action="store_true", help="print one JSON object")
    args = ap.parse_args()
    ref = read_reference(args.session_id, args.config_dir)
    if not ref:
        print(f"no tool_use records for session {args.session_id}", file=sys.stderr)
        sys.exit(1)
    if args.json:
        print(json.dumps({k: list(v) for k, v in ref.items()}, indent=1))
    else:
        for tid, (name, ts) in sorted(ref.items(), key=lambda kv: kv[1][1]):
            print(f"{ts}\t{name}\t{tid}")


if __name__ == "__main__":
    main()
