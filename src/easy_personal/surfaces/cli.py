import argparse
import json
import os
import sys
from datetime import datetime
from importlib.metadata import version
from pathlib import Path

from easy_personal import core
from easy_personal.core import config, lock, metrics, schedule, store
from easy_personal.sources import claude_code
from easy_personal.surfaces import mcp as mcp_surface


def iso_datetime(value: str) -> str:
    datetime.fromisoformat(value)  # raises ValueError -> argparse usage error
    return value


def positive_int(value: str) -> int:
    number = int(value)
    if number < 1:
        raise ValueError("must be at least 1")
    return number


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="easy-personal")
    parser.add_argument("--version", action="version", version=version("easy-personal-mcp"))
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("ping", help="print the shared core status")
    mcp_p = sub.add_parser("mcp", help="serve the MCP server (stdio by default)")
    mcp_p.add_argument("--transport", choices=["stdio", "http"], default="stdio")
    mcp_p.add_argument("--host", default=mcp_surface.LOOPBACK)
    mcp_p.add_argument("--port", type=int, default=8765)
    ingest_p = sub.add_parser(
        "ingest", help="copy new Claude Code transcript metadata to the store"
    )
    ingest_p.add_argument(
        "--source", type=Path, help="transcript root (default ~/.claude/projects)"
    )
    ingest_p.add_argument(
        "--retention-days",
        type=positive_int,
        help="delete events older than N days (default: keep all)",
    )
    schedule_p = sub.add_parser("schedule", help="run ingest on a systemd user timer")
    schedule_sub = schedule_p.add_subparsers(dest="action", required=True)
    install_p = schedule_sub.add_parser("install", help="install and enable the daily timer")
    install_p.add_argument("--retention-days", type=positive_int, help="passed to scheduled ingest")
    install_p.add_argument("--dry-run", action="store_true", help="print the units, write nothing")
    remove_p = schedule_sub.add_parser("remove", help="disable and delete the timer")
    remove_p.add_argument(
        "--dry-run", action="store_true", help="print what would go, change nothing"
    )
    usage_p = sub.add_parser("usage", help="usage summary for a period (same as MCP usage_summary)")
    usage_p.add_argument("--since", type=iso_datetime, help="ISO date/datetime, inclusive")
    usage_p.add_argument("--until", type=iso_datetime, help="ISO date/datetime, exclusive")
    usage_p.add_argument(
        "--by", choices=metrics.DIMENSIONS, help="break tokens down by this dimension"
    )
    outliers_p = sub.add_parser("outliers", help="long or tool-heavy sessions (session_outliers)")
    outliers_p.add_argument("--since", type=iso_datetime, help="ISO date/datetime, inclusive")
    outliers_p.add_argument("--until", type=iso_datetime, help="ISO date/datetime, exclusive")
    return parser


def _print_json(value) -> None:
    print(json.dumps(value, indent=2, sort_keys=True))


def _ingest(args) -> int:
    timeout = float(os.environ.get("EASY_PERSONAL_LOCK_TIMEOUT", lock.DEFAULT_TIMEOUT))
    try:
        with (
            lock.ingest_lock(config.data_dir() / "ingest.lock", timeout),
            store.opened(config.store_path()) as conn,
        ):
            result = claude_code.ingest(conn, args.source or config.claude_projects_dir())
            if args.retention_days is not None:
                result["pruned"] = store.prune(conn, args.retention_days)
    except lock.LockBusy:
        print("another ingest is running; try again later", file=sys.stderr)
        return 75
    _print_json(result)
    return 0


def _schedule(args) -> int:
    if args.action == "remove":
        if args.dry_run:
            print(f"would disable {schedule.TIMER} and delete it and {schedule.SERVICE} "
                  f"from {schedule.unit_dir()}")  # fmt: skip
            return 0
        print(f"removed {schedule.TIMER} from {schedule.remove()}")
        return 0
    cmd = schedule.command()
    if args.dry_run:
        for name, text in schedule.render_units(cmd, args.retention_days).items():
            print(f"# {name}\n{text}")
        return 0
    if not schedule.systemd_available():
        print("no systemd user session found; nothing written. Use cron instead:")
        print(schedule.cron_snippet(cmd, args.retention_days))
        return 1
    print(f"installed {schedule.TIMER} in {schedule.install(cmd, args.retention_days)}")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.command == "ping":
        print(core.ping())
        return 0
    if args.command == "ingest":
        return _ingest(args)
    if args.command == "schedule":
        return _schedule(args)
    if args.command == "usage":
        with store.opened(config.store_path()) as conn:
            if args.by:
                _print_json(metrics.usage_breakdown(conn, args.by, args.since, args.until))
            else:
                _print_json(metrics.usage_summary(conn, args.since, args.until))
        return 0
    if args.command == "outliers":
        with store.opened(config.store_path()) as conn:
            _print_json(metrics.session_outliers(conn, args.since, args.until))
        return 0
    try:
        mcp_surface.validate_host(args.host)
    except ValueError as exc:
        parser.error(str(exc))
    mcp_surface.serve(args.transport, args.host, args.port)
    return 0


if __name__ == "__main__":
    sys.exit(main())
