"""First-run checklist (FR-042): four read-only done / not-done checks over the store and the unit dir."""

import sqlite3

from easy_personal.core import accounts, schedule, workspaces

INGEST_HINT = "easy-personal ingest"
TIMER_HINT = "easy-personal schedule install"


def _timer_hint() -> str:
    """The install command; on a machine with no systemd user session, the crontab line instead."""
    if schedule.systemd_available():
        return TIMER_HINT
    cron = schedule.cron_snippet(schedule.command(), None).splitlines()[-1]
    return f"no systemd user session: run crontab -e and add: {cron}"


def steps(conn: sqlite3.Connection) -> list[dict]:
    """`[{id, done, hint}]` in checklist order. `hint` is the existing command or the view to open."""
    ingested = conn.execute("SELECT 1 FROM events LIMIT 1").fetchone() is not None
    timer = (schedule.unit_dir() / schedule.TIMER).exists()
    confirmed = any(a["status"] == "confirmed" for a in accounts.list_accounts(conn))
    workspace = any(not w["is_orphan"] for w in workspaces.list_all(conn))
    return [
        {"id": "first_ingest", "done": ingested, "hint": INGEST_HINT},
        {"id": "ingest_timer", "done": timer, "hint": TIMER_HINT if timer else _timer_hint()},
        {
            "id": "accounts_confirmed",
            "done": confirmed,
            "hint": "Open Agents and confirm an account",
        },
        {"id": "first_workspace", "done": workspace, "hint": "Open Workspaces and create one"},
    ]
