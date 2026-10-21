"""Shared core: every surface (MCP, CLI, later the dashboard API) calls only this."""


def ping() -> str:
    return "easy-personal core ok"
