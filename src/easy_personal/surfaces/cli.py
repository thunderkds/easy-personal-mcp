import argparse
import sys
from importlib.metadata import version

from easy_personal import core
from easy_personal.surfaces import mcp as mcp_surface


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="easy-personal")
    parser.add_argument("--version", action="version", version=version("easy-personal-mcp"))
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("ping", help="print the shared core status")
    mcp_p = sub.add_parser("mcp", help="serve the MCP server (stdio by default)")
    mcp_p.add_argument("--transport", choices=["stdio", "http"], default="stdio")
    mcp_p.add_argument("--host", default=mcp_surface.LOOPBACK)
    mcp_p.add_argument("--port", type=int, default=8765)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.command == "ping":
        print(core.ping())
        return 0
    try:
        mcp_surface.validate_host(args.host)
    except ValueError as exc:
        parser.error(str(exc))
    mcp_surface.serve(args.transport, args.host, args.port)
    return 0


if __name__ == "__main__":
    sys.exit(main())
