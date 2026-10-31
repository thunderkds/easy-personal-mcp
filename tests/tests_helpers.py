import json
import os
from pathlib import Path

SRC = str(Path(__file__).resolve().parents[1] / "src")


def child_env(tmp_path: Path) -> dict[str, str]:
    env = dict(os.environ)
    env.update(PYTHONPATH=SRC, EASY_PERSONAL_HOME=str(tmp_path / "home"))
    return env


def add_event(conn, n: int, project: str | None, ts: str | None = None) -> None:
    """One assistant message with tokens at 2026-01-01 + n seconds (callers commit)."""
    from time import gmtime, strftime

    stamp = ts or strftime("%Y-%m-%dT%H:%M:%S", gmtime(1_767_225_600 + n)) + ".000000Z"
    conn.execute(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc,"
        " session_id, project_path, model, message_id, input_tokens, output_tokens,"
        " cache_creation_input_tokens, cache_read_input_tokens)"
        " VALUES ('m','claude_code','/x',1,?,0,?,?,?,'model-0',?,10,5,0,0)",
        (n, stamp, f"s-{project}", project, f"msg{n}"),
    )


TOKENS = 15  # every assistant message below: input 10 + output 5


def rec(n, session, kind="assistant", entry="cli", sidechain=False, message=None, day=15):
    """One transcript record at minute `n` of 2026-01-`day`; `entry=None` leaves out `entrypoint`."""
    record = {
        "type": kind,
        "version": "2.1.302",
        "cwd": "/work/demo-app",
        "sessionId": session,
        "isSidechain": sidechain,
        "timestamp": f"2026-01-{day:02d}T10:{n:02d}:00.000Z",
    }
    if entry is not None:
        record["entrypoint"] = entry
    if kind == "assistant":
        record["message"] = {
            "id": message or f"msg-{session}-{n}",
            "model": "claude-fixture-model-1",
            "content": [],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0,
            },
        }
    return record


def write(root, name, records, mode="w"):
    root.mkdir(parents=True, exist_ok=True)
    with open(root / name, mode) as handle:
        handle.write("".join(json.dumps(r) + "\n" for r in records))
    return root
