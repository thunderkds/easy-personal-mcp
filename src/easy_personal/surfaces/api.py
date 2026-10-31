"""Dashboard server: Starlette JSON API + built SPA, on 127.0.0.1 only, guarded per launch.

Every request must carry a `Host` equal to the dashboard's own address (blocks DNS rebinding);
a present `Origin` must match it too. `/api/*` also needs the per-launch token in the
`X-Dashboard-Token` header. Every response carries a CSP of `default-src 'self'`.
"""

import contextlib
import hmac
import json
import secrets
import socket
import threading
import webbrowser
from collections.abc import Callable
from importlib import resources
from pathlib import Path

import uvicorn
from starlette.applications import Starlette
from starlette.concurrency import run_in_threadpool
from starlette.responses import JSONResponse
from starlette.routing import Mount, Route
from starlette.staticfiles import StaticFiles

from easy_personal.core import (
    accounts,
    board,
    config,
    first_run,
    jobs,
    kit,
    memgraph,
    metrics,
    plan,
    rules,
    store,
    workspaces,
)
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


def _who(q):
    """The optional `workspace` name and `account` id or label (empty = no filter), as keyword arguments."""
    return {"workspace": q.get("workspace") or None, "account": q.get("account") or None}


def _scope(q):
    """`since`, `until` and the optional `workspace` name and `account` id or label (empty = no filter)."""
    return *_period(q), (q.get("workspace") or None), (q.get("account") or None)


def _memory_graph(conn, q):
    """No `project`: the projects that have memory notes. With `project`: that project's nodes and edges."""
    project = q.get("project")
    return memgraph.project_graph(conn, project) if project else memgraph.project_list(conn)


_WRITE_LOCK = threading.Lock()  # one connector write at a time (double submit, two tabs)


def _entry_json(entry, configured: bool) -> dict:
    return {
        "id": entry.id,
        "category": entry.category,
        "provider": entry.provider,
        "claude_ai_connector": entry.claude_ai_connector,
        "claude_ai_link": entry.claude_ai_link,
        "fields": [{"env": f.env, "label": f.label, "secret": f.secret} for f in entry.fields],
        "configured": configured,
    }


def _configured(name: str) -> bool:
    """Keychain item present AND a Claude Code entry exists. Never reads a value out."""
    from keyring.errors import KeyringError

    from easy_personal.connectors import writer
    from easy_personal.launch import LaunchError

    try:
        return writer._keychain_item(name) and writer._entry_exists(name, writer.account_env(None)) is not None
    except (LaunchError, KeyringError):  # no usable, locked or broken keychain
        return False


def connectors_list(request):
    from easy_personal.connectors import catalog

    entries = sorted((catalog.load(i) for i in catalog.ids()),
                     key=lambda e: (e.category, not e.claude_ai_connector, e.provider))  # fmt: skip
    with store.opened(config.store_path()) as conn:
        stored = conn.execute(
            "SELECT c.id, c.alias, c.provider, w.name FROM connectors c "
            "JOIN workspaces w ON w.id = c.workspace_id ORDER BY w.name, c.alias"
        ).fetchall()
    return JSONResponse({
        "connectors": [_entry_json(e, _configured(e.id)) for e in entries],
        "workspace_connectors": [dict(zip(("id", "alias", "provider", "workspace"), r, strict=True)) for r in stored],
    })  # fmt: skip


async def _json_body(request):
    """The JSON object body of a write request, or an error response (415 / 400)."""
    ctype = request.headers.get("content-type", "").split(";")[0].strip().lower()
    if ctype != "application/json":
        return None, JSONResponse({"error": "content-type must be application/json"}, 415)
    try:
        body = await request.json()
    except ValueError:
        return None, JSONResponse({"error": "body must be JSON"}, 400)
    if not isinstance(body, dict) or not isinstance(body.get("confirm", False), bool):
        return None, JSONResponse({"error": "body must be an object with boolean confirm"}, 400)
    return body, None


def _run_writer(call, extra: Callable[[int], dict] | None = None, own_status: bool = False) -> JSONResponse:
    """Run a writer call; `pending` is true when it stopped at the confirmation step.

    `extra(status)` adds keys to the answer; `own_status`: a refused workspace operation answers with its
    own status (404, 403, 409) instead of 400."""
    from easy_personal.connectors import catalog, writer
    from easy_personal.launch import LaunchError

    lines: list[str] = []
    asked: list[bool] = []

    def confirm() -> bool:
        asked.append(len(lines))  # the plan ends here; the writer's abort line comes after
        return False

    try:
        with _WRITE_LOCK:
            status = call(confirm, lines.append)
    except workspaces.WorkspaceError as exc:
        return JSONResponse({"error": str(exc)}, status_code=exc.status if own_status else 400)
    except (catalog.CatalogError, writer.ConnectorError) as exc:
        return JSONResponse({"error": str(exc)}, status_code=400)  # messages never echo input
    except LaunchError as exc:
        status = 1
        lines.append(str(exc))
    answer = {"status": status, "lines": lines[: asked[0]] if asked else lines, "pending": bool(asked)}
    return JSONResponse(answer | (extra(status) if extra else {}))


async def connectors_add(request):
    from easy_personal.connectors import catalog, writer

    body, error = await _json_body(request)
    if error:
        return error
    values = body.get("values", {})
    if not isinstance(values, dict) or not all(isinstance(v, str) for v in values.values()):
        return JSONResponse({"error": "values must map field names to strings"}, 400)
    try:
        entry = catalog.load(str(body.get("provider", "")))
    except catalog.CatalogError:  # the message would echo the caller's input: say it generically
        return JSONResponse({"error": "unknown provider; see GET /api/connectors"}, 400)
    workspace, alias = body.get("workspace"), body.get("alias")
    if not isinstance(workspace, str) or not workspace:
        return JSONResponse({"error": "workspace is required (a workspace name)"}, 400)
    if alias is not None and not isinstance(alias, str):
        return JSONResponse({"error": "alias must be a string"}, 400)
    by_label = {f.label: values.get(f.env, "") for f in entry.fields}
    yes = body.get("confirm", False)

    def add(confirm, out):
        with store.opened(config.store_path()) as conn:
            return writer.add_to_workspace(
                conn, entry, workspace, alias, lambda label, _secret: by_label[label], confirm, out, yes
            )

    return await run_in_threadpool(_run_writer, add)


def connectors_legacy(request):
    from easy_personal.connectors import legacy
    from easy_personal.launch import LaunchError

    try:
        with store.opened(config.store_path()) as conn:
            return JSONResponse({"legacy": [i.as_dict() for i in legacy.detect(conn)]})
    except LaunchError as exc:  # no usable keychain: the message is the guidance, no input echoed
        return JSONResponse({"error": str(exc)}, 503)


async def connectors_legacy_adopt(request):
    from easy_personal.connectors import legacy

    body, error = await _json_body(request)
    if error:
        return error
    yes = body.get("confirm", False)

    def adopt(confirm, out):
        with store.opened(config.store_path()) as conn:
            return legacy.adopt(conn, confirm, out, yes)

    return await run_in_threadpool(_run_writer, adopt)


def _stored_connector(name: str) -> bool:
    with store.opened(config.store_path()) as conn:
        return conn.execute("SELECT 1 FROM connectors WHERE id = ?", (name,)).fetchone() is not None


async def connectors_remove(request):
    from easy_personal.connectors import catalog, sync, writer

    body, error = await _json_body(request)
    if error:
        return error
    name, yes = request.path_params["name"], body.get("confirm", False)
    if await run_in_threadpool(_stored_connector, name):

        def remove(confirm, out):
            with store.opened(config.store_path()) as conn:
                return sync.remove_connector(conn, name, confirm, out, yes)

        return await run_in_threadpool(_run_writer, remove)
    if name not in catalog.ids():  # never remove entries we did not create
        return JSONResponse({"error": "unknown connector"}, 404)
    return await run_in_threadpool(
        _run_writer, lambda confirm, out: writer.remove(name, confirm, out, yes)  # pre-workspace connector
    )


def _sync_account(raw) -> str | None:
    """`account` of a sync request: absent = every confirmed Claude account; else an integer id."""
    if raw is None or raw == "":
        return None
    if isinstance(raw, bool) or not (isinstance(raw, int) or (isinstance(raw, str) and raw.isdigit())):
        raise accounts.AccountError("account must be an account id")
    return str(raw)


def sync_plan(request):
    from easy_personal.connectors import sync

    try:
        account = _sync_account(request.query_params.get("account"))
        with store.opened(config.store_path()) as conn:
            accounts.detect(conn)
            return JSONResponse({"steps": [s.as_dict() for s in sync.plan_all(conn, account)]})
    except accounts.NotFound as exc:
        return JSONResponse({"error": str(exc)}, 404)
    except accounts.AccountError as exc:
        return JSONResponse({"error": str(exc)}, 400)


async def sync_apply(request):
    """The plan's lines; with `confirm: true` it is applied (CLAUDE_CONFIG_DIR per account)."""
    from easy_personal.connectors import sync

    body, error = await _json_body(request)
    if error:
        return error
    try:
        account = _sync_account(body.get("account"))
    except accounts.AccountError as exc:
        return JSONResponse({"error": str(exc)}, 400)

    def work(confirm, out):
        with store.opened(config.store_path()) as conn:
            accounts.detect(conn)
            steps = sync.plan_all(conn, account)
            for step in steps:
                out(step.line())
            if not any(s.action in sync.APPLIED for s in steps):
                out("nothing to change")
                return 0
            if not body.get("confirm", False) and not confirm():
                return 0
            return sync.apply(conn, steps, out)

    try:
        return await run_in_threadpool(_run_writer, work)
    except accounts.NotFound as exc:
        return JSONResponse({"error": str(exc)}, 404)


def _account_write(call: Callable, status: int = 200):
    """A guarded account write: JSON object body, one fresh connection; bad input 400, unknown 404."""

    async def endpoint(request):
        body, error = await _json_body(request)
        if error:
            return error
        raw_id = request.path_params.get("id")
        if raw_id is not None and not raw_id.isdigit():
            return JSONResponse({"error": "no such account"}, 404)

        def work():
            with store.opened(config.store_path()) as conn:
                accounts.detect(conn)
                return call(conn, raw_id, body)

        try:
            return JSONResponse(await run_in_threadpool(work), status_code=status)
        except accounts.NotFound as exc:
            return JSONResponse({"error": str(exc)}, 404)
        except accounts.AccountError as exc:
            return JSONResponse({"error": str(exc)}, 400)

    return endpoint


async def _object_body(request, allow_empty: bool = False):
    """The JSON object body of a write request, or an error response (415 / 400)."""
    ctype = request.headers.get("content-type", "").split(";")[0].strip().lower()
    if ctype != "application/json":
        return None, JSONResponse({"error": "content-type must be application/json"}, 415)
    raw = await request.body()
    if allow_empty and not raw.strip():
        return {}, None
    try:
        body = json.loads(raw)
    except ValueError:
        return None, JSONResponse({"error": "body must be JSON"}, 400)
    if not isinstance(body, dict):
        return None, JSONResponse({"error": "body must be a JSON object"}, 400)
    return body, None


def _workspace_call(call, status: int = 200) -> JSONResponse:
    """Run `call(conn)` on a fresh store connection; a refused operation -> its status + reason."""
    try:
        with store.opened(config.store_path()) as conn:
            return JSONResponse(call(conn), status_code=status)
    except workspaces.WorkspaceError as exc:
        return JSONResponse({"error": str(exc)}, status_code=exc.status)


def _workspace_id(request) -> int | None:
    raw = request.path_params["id"]
    return int(raw) if raw.isdigit() else None


_UNKNOWN_WORKSPACE = JSONResponse({"error": "unknown workspace"}, 404)


async def workspaces_add(request):
    body, error = await _object_body(request)
    if error:
        return error
    if set(body) - set(workspaces.FIELDS):
        return JSONResponse({"error": "unknown field"}, 400)
    text = {k: v for k, v in body.items() if k not in ("name", "repos")}
    return await run_in_threadpool(
        _workspace_call,
        lambda c: {"workspace": workspaces.add(c, body.get("name"), repos=body.get("repos", []), **text)},
        201,
    )


def _confirmed(body: dict) -> bool:
    """`confirm` of a write body: absent = False; anything but a boolean is a 400 (TypeError)."""
    value = body.pop("confirm", False)
    if not isinstance(value, bool):
        raise TypeError("confirm must be true or false")
    return value


async def workspaces_edit(request):
    """Edit a workspace. A new name shows the alias plan; with `confirm: true` it is applied."""
    from easy_personal.connectors import sync

    body, error = await _object_body(request)
    if error:
        return error
    ws_id = _workspace_id(request)
    if ws_id is None:
        return _UNKNOWN_WORKSPACE
    try:
        yes = _confirmed(body)
    except TypeError as exc:
        return JSONResponse({"error": str(exc)}, 400)

    def work(confirm, out):
        with store.opened(config.store_path()) as conn:
            current = workspaces.get(conn, ws_id)
            if body.get("name") == current["name"]:
                body.pop("name")
            if "name" not in body:
                workspaces.edit(conn, ws_id, body)
                return 0
            return sync.rename_workspace(conn, ws_id, body, confirm, out, yes)

    def workspace_now(_status):
        with store.opened(config.store_path()) as conn:
            return {"workspace": workspaces.get(conn, ws_id)}

    return await run_in_threadpool(_run_writer, work, workspace_now, True)


async def workspaces_delete(request):
    """Delete a workspace: its connectors move to orphan. Shows the plan; `confirm: true` applies it."""
    from easy_personal.connectors import sync

    body, error = await _object_body(request, allow_empty=True)
    if error:
        return error
    ws_id = _workspace_id(request)
    if ws_id is None:
        return _UNKNOWN_WORKSPACE
    try:
        yes = _confirmed(body)
    except TypeError as exc:
        return JSONResponse({"error": str(exc)}, 400)

    def work(confirm, out):
        with store.opened(config.store_path()) as conn:
            return sync.delete_workspace(conn, ws_id, confirm, out, yes)

    def deleted(status):
        return {"deleted": ws_id if status == 0 else None}

    return await run_in_threadpool(_run_writer, work, deleted, True)


async def connectors_move(request):
    from easy_personal.connectors import sync

    body, error = await _json_body(request)
    if error:
        return error
    workspace = body.get("workspace")
    if not isinstance(workspace, str) or not workspace:
        return JSONResponse({"error": "workspace is required (a workspace name)"}, 400)
    connector_id, yes = request.path_params["id"], body.get("confirm", False)

    def work(confirm, out):
        with store.opened(config.store_path()) as conn:
            return sync.move_connector(conn, connector_id, workspace, confirm, out, yes)

    return await run_in_threadpool(_run_writer, work, None, True)


async def connectors_alias(request):
    from easy_personal.connectors import sync

    body, error = await _json_body(request)
    if error:
        return error
    alias = body.get("alias")
    if alias is not None and not isinstance(alias, str):
        return JSONResponse({"error": "alias must be a string, or null for the default"}, 400)
    connector_id, yes = request.path_params["id"], body.get("confirm", False)

    def work(confirm, out):
        with store.opened(config.store_path()) as conn:
            return sync.set_alias(conn, connector_id, alias, confirm, out, yes)

    return await run_in_threadpool(_run_writer, work, None, True)


def create_app(token: str, port: int, static: Path | None = None) -> Starlette:
    routes = [
        Route("/api/usage/summary", _query(lambda c, q: metrics.usage_summary(c, *_scope(q)))),
        Route(
            "/api/usage/breakdown",
            _query(lambda c, q: metrics.usage_breakdown(c, q.get("by", ""), *_period(q), workspace=_scope(q)[2], account=_scope(q)[3])),
        ),
        Route("/api/usage/outliers", _query(lambda c, q: metrics.session_outliers(c, *_scope(q)))),
        Route("/api/usage/sessions", _query(lambda c, q: metrics.session_timeline(c, *_scope(q)))),
        Route("/api/workspaces", _query(lambda c, q: {"workspaces": workspaces.list_all(c)})),
        Route("/api/workspaces", workspaces_add, methods=["POST"]),
        Route("/api/workspaces/{id}", workspaces_edit, methods=["PATCH"]),
        Route("/api/workspaces/{id}", workspaces_delete, methods=["DELETE"]),
        Route("/api/warnings", _query(lambda c, q: rules.evaluate(c, **_who(q)))),
        Route("/api/plan", _query(lambda c, q: plan.improvement_plan(c, **_who(q)))),
        Route("/api/kit", _query(lambda c, q: kit.kit_summary(c, **_who(q)))),
        Route("/api/board", _query(lambda c, q: board.board(c, **_who(q)))),
        Route("/api/accounts", _query(lambda c, q: accounts.grouped(c, q.get("by", "account")))),
        Route("/api/accounts", _account_write(lambda c, _id, b: accounts.add(c, b.get("path")), 201), methods=["POST"]),
        Route("/api/accounts/{id}/confirm", _account_write(lambda c, i, b: accounts.confirm(c, i, b.get("label"))), methods=["POST"]),
        Route("/api/accounts/{id}/ignore", _account_write(lambda c, i, b: accounts.ignore(c, i)), methods=["POST"]),
        Route("/api/accounts/{id}/workspaces", _account_write(lambda c, i, b: accounts.use(c, i, b.get("workspaces"))), methods=["PUT"]),
        Route("/api/jobs", lambda request: JSONResponse(jobs.list_jobs())),  # sync: runs in a worker thread
        Route("/api/memory-graph", _query(_memory_graph)),
        Route("/api/first-run", _query(lambda c, q: {"steps": first_run.steps(c)})),
        Route("/api/sync", sync_plan),
        Route("/api/sync", sync_apply, methods=["POST"]),
        Route("/api/connectors", connectors_list),
        Route("/api/connectors", connectors_add, methods=["POST"]),
        Route("/api/connectors/legacy", connectors_legacy),
        Route("/api/connectors/legacy", connectors_legacy_adopt, methods=["POST"]),
        Route("/api/connectors/{name}", connectors_remove, methods=["DELETE"]),
        Route("/api/connectors/{id}/move", connectors_move, methods=["POST"]),
        Route("/api/connectors/{id}/alias", connectors_alias, methods=["PUT"]),
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
