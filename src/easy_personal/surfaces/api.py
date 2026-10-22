"""Dashboard server: Starlette JSON API + built SPA, on 127.0.0.1 only, guarded per launch.

Every request must carry a `Host` equal to the dashboard's own address (blocks DNS rebinding);
a present `Origin` must match it too. `/api/*` also needs the per-launch token in the
`X-Dashboard-Token` header. Every response carries a CSP of `default-src 'self'`.
"""

import contextlib
import hmac
import secrets
import socket
import webbrowser
from collections.abc import Callable
from importlib import resources
from pathlib import Path

import uvicorn
from starlette.applications import Starlette
from starlette.responses import JSONResponse
from starlette.routing import Mount, Route
from starlette.staticfiles import StaticFiles

from easy_personal.core import config, metrics, plan, rules, store
from easy_personal.surfaces.mcp import LOOPBACK

TOKEN_HEADER = "x-dashboard-token"
CSP = "default-src 'self'; frame-ancestors 'none'"


def static_dir() -> Path:
    return Path(str(resources.files("easy_personal") / "dashboard" / "static"))


class Guard:
    """Pure ASGI middleware: Host/Origin/token checks, then the CSP header on every response."""

    def __init__(self, app, token: str, port: int) -> None:
        self.app, self.token = app, token
        self.host = f"{LOOPBACK}:{port}"
        self.origin = f"http://{self.host}"

    def _allowed(self, scope) -> bool:
        headers = {k.decode("latin-1").lower(): v.decode("latin-1") for k, v in scope["headers"]}
        if headers.get("host") != self.host:
            return False
        if "origin" in headers and headers["origin"] != self.origin:
            return False
        if scope["path"].startswith("/api/"):
            sent = headers.get(TOKEN_HEADER, "")
            return hmac.compare_digest(sent.encode(), self.token.encode())
        return True

    async def __call__(self, scope, receive, send):
        if scope["type"] == "lifespan":
            return await self.app(scope, receive, send)
        if scope["type"] != "http":  # no websocket routes: refuse (403 before accept)
            await receive()
            return await send({"type": "websocket.close", "code": 1008})

        async def send_csp(message):
            if message["type"] == "http.response.start":
                message["headers"] = [
                    *(h for h in message["headers"] if h[0].lower() != b"content-security-policy"),
                    (b"content-security-policy", CSP.encode()),
                ]
            await send(message)

        if not self._allowed(scope):
            response = JSONResponse({"error": "forbidden"}, status_code=403)
            return await response(scope, receive, send_csp)
        await self.app(scope, receive, send_csp)


def _query(handler: Callable):
    """Run `handler(conn, query_params)` on a fresh store connection; bad input -> 400."""

    def endpoint(request):
        try:
            with store.opened(config.store_path()) as conn:
                return JSONResponse(handler(conn, request.query_params))
        except ValueError as exc:
            return JSONResponse({"error": str(exc)}, status_code=400)

    return endpoint


def _period(q):
    return q.get("since"), q.get("until")


def create_app(token: str, port: int, static: Path | None = None) -> Starlette:
    routes = [
        Route("/api/usage/summary", _query(lambda c, q: metrics.usage_summary(c, *_period(q)))),
        Route(
            "/api/usage/breakdown",
            _query(lambda c, q: metrics.usage_breakdown(c, q.get("by", ""), *_period(q))),
        ),
        Route("/api/usage/outliers", _query(lambda c, q: metrics.session_outliers(c, *_period(q)))),
        Route("/api/warnings", _query(lambda c, q: rules.evaluate(c))),
        Route("/api/plan", _query(lambda c, q: plan.improvement_plan(c))),
    ]
    static = static or static_dir()
    if static.is_dir():
        routes.append(Mount("/", StaticFiles(directory=static, html=True)))
    app = Starlette(routes=routes)
    app.add_middleware(Guard, token=token, port=port)
    return app


def bind_free_port() -> socket.socket:
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.bind((LOOPBACK, 0))
    return sock


def serve(open_browser: Callable[[str], object] = webbrowser.open) -> None:
    """Bind a free loopback port, print the URL, open the browser, run until Ctrl-C."""
    sock = bind_free_port()
    port = sock.getsockname()[1]
    token = secrets.token_urlsafe(32)
    url = f"http://{LOOPBACK}:{port}/#{token}"
    server = uvicorn.Server(
        uvicorn.Config(create_app(token, port), log_config=None, access_log=False)
    )
    print(f"dashboard: {url}\nCtrl-C to stop.", flush=True)
    with contextlib.suppress(Exception):  # headless: the URL is already printed
        open_browser(url)
    try:
        server.run(sockets=[sock])
    except KeyboardInterrupt:
        pass
