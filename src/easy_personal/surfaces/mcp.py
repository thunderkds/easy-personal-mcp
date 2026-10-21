from mcp.server.fastmcp import FastMCP

from easy_personal import core

LOOPBACK = "127.0.0.1"


def validate_host(host: str) -> str:
    if host != LOOPBACK:
        raise ValueError(f"refusing host {host!r}: HTTP must bind {LOOPBACK} only")
    return host


def create_app(host: str = LOOPBACK, port: int = 8765) -> FastMCP:
    app = FastMCP("easy-personal", host=validate_host(host), port=port)

    @app.tool()
    def ping() -> str:
        """Placeholder tool: returns the shared core's status string."""
        return core.ping()

    return app


def serve(transport: str = "stdio", host: str = LOOPBACK, port: int = 8765) -> None:
    app = create_app(host, port)
    app.run(transport="streamable-http" if transport == "http" else "stdio")
