"""Fixture store + route snapshot for the HUN-186 golden-equality test (speed work must not change a number).

`snapshot()` calls the core function behind each dashboard read with fixed times, and `normalized()`
turns it into stable JSON. `hun186_golden.json` was captured from the code BEFORE HUN-186 (main
de96ca4) with `python tests/surfaces/hun186_fixture.py <out.json>`; the test compares today's output to it.
"""

import json
import os
import sys
import time
from datetime import UTC, datetime
from pathlib import Path

from easy_personal.core import board, kit, memgraph, metrics, plan, rules, store

NOW = datetime(2026, 10, 20, 0, 0, tzinfo=UTC)
SINCE, UNTIL = "2026-10-13T00:00:00Z", "2026-10-20T00:00:00Z"
PULL = 1_760_000_000  # fixed mtime of the board file


def _event(conn, n, project, session, ts, *, message=None, model="model-a", tools=None, i=10, o=5, cc=0, cr=0,
           sidechain=0, entrypoint="cli", abstained=0, version="2.1.1"):  # fmt: skip
    conn.execute(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, record_type, version,"
        " ts_utc, session_id, project_path, is_sidechain, entrypoint, model, message_id, tool_names, input_tokens,"
        " output_tokens, cache_creation_input_tokens, cache_read_input_tokens)"
        " VALUES ('m','claude_code','/x',1,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        (n, abstained, "assistant", version, ts, session, project, sidechain, entrypoint, model, message, tools,
         None if message is None else i, None if message is None else o, None if message is None else cc,
         None if message is None else cr),
    )  # fmt: skip


def _trace(conn, n, project, task, ts, error, abstained=0):
    conn.execute(
        "INSERT INTO kit_trace_events (machine_id, source_path, inode, byte_offset, project_path, kit_task_id,"
        " abstained, ts_utc, tool_name, is_error) VALUES ('m', ?, 1, ?, ?, ?, ?, ?, 'Bash', ?)",
        (f"{project}/t.jsonl", n, project, task, abstained, ts, error),
    )


def seed(root: Path, home: Path) -> dict[str, str]:
    """Three live projects (one with a board and memory notes), two gone ones; returns name -> path."""
    paths = {name: str(root / name) for name in ("alive-a", "alive-b", "gone-x", "gone-y")}
    for name in ("alive-a", "alive-b"):
        (root / name).mkdir(parents=True)
    (root / "alive-a" / ".claude").mkdir()
    (root / "alive-a" / ".claude" / "board.json").write_text('{"board": "linear"}')
    kanban = root / "alive-a" / "PROJECT_KANBAN.md"
    kanban.write_text("## Board\n### Todo\n- [ ] **HUN-1** — First task | a\n### Done\n- [x] **HUN-2** — Second task\n")
    os.utime(kanban, (PULL, PULL))
    mem = root / "alive-a" / "memory"
    mem.mkdir()
    (mem / "MEMORY.md").write_text("---\nname: idx\ndescription: Index\n---\n[[notes]] [[ghost]]\n")
    (mem / "notes.md").write_text("---\nname: notes\ndescription: Notes\n---\n[[idx]]\n")

    with store.opened(home / "store.sqlite") as conn:
        n = 0

        def add(*args, **kwargs):
            nonlocal n
            n += 1
            _event(conn, n, *args, **kwargs)

        add(paths["alive-a"], "hist", "2026-10-01T09:00:00.000000Z")  # history start (no tokens)
        # alive-a: session a1 (cache-poor, a 40 min pause), a2 (huge context), a3 (agent work, SDK entrypoint)
        add(paths["alive-a"], "a1", "2026-10-14T10:00:00.000000Z", message="a1-1", tools="Bash,Read", cr=100)
        add(paths["alive-a"], "a1", "2026-10-14T10:00:01.000000Z", message="a1-1", tools="Edit", cr=100)  # 2nd row, same message
        add(paths["alive-a"], "a1", "2026-10-14T10:05:00.000000Z", message="a1-2", tools="Bash", i=20, o=7, cc=3)
        add(paths["alive-a"], "a1", "2026-10-14T10:45:00.000000Z", message="a1-3", model="model-b")
        add(paths["alive-a"], "a1", "2026-10-14T10:46:00.000000Z", abstained=1, version="9.9.9")
        add(paths["alive-a"], "a2", "2026-10-15T23:30:00.000000Z", message="a2-1", i=250_000, o=9)
        add(paths["alive-a"], "a2", "2026-10-16T00:10:00.000000Z", message="a2-2", cr=50_000)
        add(paths["alive-a"], "a3", "2026-10-16T12:00:00.000000Z", message="a3-1", sidechain=1, entrypoint="sdk-cli")
        add(paths["alive-a"], "a3", "2026-10-16T12:01:00.000000Z", message="a3-2", entrypoint=None)
        add(paths["alive-a"], "a4", "2026-10-17T12:00:00.000000Z", message="a4-1", tools=",".join(["Bash"] * 95))
        # alive-b and the gone projects: their usage still counts
        add(paths["alive-b"], "b1", "2026-10-15T08:00:00.000000Z", message="b1-1", cr=900)
        add(paths["alive-b"], "b1", "2026-10-15T08:01:00.000000Z", message="b1-2", cr=900)
        add(paths["gone-x"], "gx1", "2026-10-15T09:00:00.000000Z", message="gx1-1", i=500, o=500)
        add(paths["gone-x"], "gx1", "2026-10-15T09:02:00.000000Z", message="gx1-2", i=500, o=500)
        add(paths["gone-y"], "gy1", "2026-10-16T09:00:00.000000Z", message="gy1-1", model="model-b")
        # the week before the window (week-over-week)
        add(paths["alive-b"], "b0", "2026-10-08T08:00:00.000000Z", message="b0-1", i=100, o=100)
        add(paths["gone-x"], "gx0", "2026-10-09T08:00:00.000000Z", message="gx0-1", i=100, o=100)
        for k in range(12):
            _trace(conn, k, paths["alive-a"], f"T00{k % 2}", f"2026-10-15T10:{k:02d}:00.000000Z", int(k < 2))
        _trace(conn, 99, paths["alive-a"], "T009", "2026-10-15T11:00:00.000000Z", None, abstained=1)
        for k in range(5):
            _trace(conn, k, paths["gone-x"], "T001", f"2026-10-15T10:{k:02d}:00.000000Z", int(k < 3))
        conn.commit()
    return paths


def snapshot(conn) -> dict:
    out = {
        "summary": metrics.usage_summary(conn, SINCE, UNTIL),
        "summary_all": metrics.usage_summary(conn),
        "outliers": metrics.session_outliers(conn, SINCE, UNTIL),
        "sessions": metrics.session_timeline(conn, SINCE, UNTIL),
        "sessions_all": metrics.session_timeline(conn),
        "warnings": rules.evaluate(conn, NOW),
        "plan": plan.improvement_plan(conn, NOW),
        "kit": kit.kit_summary(conn),
        "board": board.board(conn),
        "memory_list": memgraph.project_list(conn),
    }
    for by in metrics.DIMENSIONS:
        out[f"breakdown_{by}"] = metrics.usage_breakdown(conn, by, SINCE, UNTIL)
    return out


def normalized(data: dict, root: Path) -> dict:
    """JSON-stable copy: the temp folder replaced by `<ROOT>`, keys sorted."""
    text = json.dumps(data, sort_keys=True, default=str).replace(str(root), "<ROOT>")
    return json.loads(text)


if __name__ == "__main__":  # capture the golden: python tests/surfaces/hun186_fixture.py out.json
    import tempfile

    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp)
        os.environ["TZ"] = "UTC"  # the `day` breakdown groups by local day
        time.tzset()
        os.environ["EASY_PERSONAL_CONTEXT_URL"] = "http://127.0.0.1:9/mcp"
        os.environ["EASY_MEMORY_HOME"] = str(base / "em-none")
        os.environ["EASY_PERSONAL_AGENT_ROOT"] = str(base / "agent-root")
        (base / "agent-root").mkdir()
        seed(base / "ws", base)
        with store.opened(base / "store.sqlite") as conn:
            Path(sys.argv[1]).write_text(
                json.dumps(normalized(snapshot(conn), base / "ws"), indent=1, sort_keys=True) + "\n"
            )
