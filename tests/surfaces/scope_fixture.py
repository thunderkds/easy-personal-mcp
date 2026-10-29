"""Fixture store for the warnings/plan/kit scope tests (HUN-117): 2 accounts, 3 projects, 2 workspaces.

Window (now = 2026-10-20): account 1 reads from cache (/work/a, /work/b); account 2 does not
(/work/b, /work/c) and is the only one with tokens in the previous week. Kernel traces: /work/a is
clean, /work/c fails half its calls.
"""

from datetime import UTC, datetime

from easy_personal.core import store, workspaces
from tests_helpers import add_event

NOW = datetime(2026, 10, 20, 0, 0, tzinfo=UTC)
# (project, account id, day of October, cache_read tokens)
EVENTS = (
    [("/work/a", 1, 15, 900)] * 3
    + [("/work/b", 1, 16, 900)] * 2
    + [("/work/c", 2, 17, 0)] * 3
    + [("/work/b", 2, 18, 0)]
    + [("/work/c", 2, 8, 0)] * 2  # previous week, account 2
    + [("/work/a", 1, 9, 0)]  # previous week, account 1
)


def stamp(day: int, n: int = 0) -> str:
    return f"2026-10-{day:02d}T12:00:{n % 60:02d}.000000Z"


def seed(home) -> None:
    with store.opened(home / "store.sqlite") as conn:
        conn.execute(
            "INSERT INTO agent_accounts (provider, folder, label, status, config_dir_env) VALUES "
            "('claude', '/a/.claude', 'Claude default', 'confirmed', NULL),"
            "('claude', '/a/.claude-work', 'Claude work', 'confirmed', '/a/.claude-work')"
        )
        add_event(conn, 0, "/work/a", ts="2026-09-20T00:00:00.000000Z")  # history start
        conn.execute("UPDATE events SET agent_account_id = 1 WHERE byte_offset = 0")
        for n, (project, account, day, cache_read) in enumerate(EVENTS, 1):
            add_event(conn, n, project, ts=stamp(day, n))
            conn.execute(
                "UPDATE events SET agent_account_id = ?, cache_read_input_tokens = ? WHERE byte_offset = ?",
                (account, cache_read, n),
            )
        for n in range(10):
            for project, errors in (("/work/a", 0), ("/work/c", 5)):
                conn.execute(
                    "INSERT INTO kit_trace_events (machine_id, source_path, inode, byte_offset, project_path,"
                    " kit_task_id, abstained, ts_utc, tool_name, is_error) VALUES ('m', ?, 1, ?, ?, 'T001', 0, ?, 'Bash', ?)",
                    (f"{project}/t.jsonl", n, project, stamp(16, n), int(n < errors)),
                )
        workspaces.add(conn, "wa", repos=["/work/a"])
        workspaces.add(conn, "wc", repos=["/work/c", "/work/b"])
        conn.commit()
