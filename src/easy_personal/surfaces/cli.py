import argparse
import json
import sys
from datetime import datetime
from importlib.metadata import version
from pathlib import Path

from easy_personal import core
from easy_personal.core import config, metrics, store
from easy_personal.sources import claude_code
from easy_personal.surfaces import mcp as mcp_surface


def iso_datetime(value: str) -> str:
    datetime.fromisoformat(value)  # raises ValueError -> argparse usage error
    return value


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
    usage_p = sub.add_parser("usage", help="usage summary for a period (same as MCP usage_summary)")
    usage_p.add_argument("--since", type=iso_datetime, help="ISO date/datetime, inclusive")
    usage_p.add_argument("--until", type=iso_datetime, help="ISO date/datetime, exclusive")
    return parser


def _print_json(value) -> None:
    print(json.dumps(value, indent=2, sort_keys=True))


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.command == "ping":
        print(core.ping())
        return 0
    if args.command == "ingest":
        with store.opened(config.store_path()) as conn:
            _print_json(claude_code.ingest(conn, args.source or config.claude_projects_dir()))
        return 0
    if args.command == "usage":
        with store.opened(config.store_path()) as conn:
            _print_json(metrics.usage_summary(conn, args.since, args.until))
        return 0
    try:
        mcp_surface.validate_host(args.host)
    except ValueError as exc:
        parser.error(str(exc))
    mcp_surface.serve(args.transport, args.host, args.port)
    return 0


if __name__ == "__main__":
    sys.exit(main())
