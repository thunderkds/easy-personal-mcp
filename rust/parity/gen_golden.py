#!/usr/bin/env python3
"""Parity oracle (T003 / RS-003): freeze what the Python app answers, as golden JSON.

Ingests tests/fixtures/claude_code into a throwaway store, then asks every read-only `/api/*` route and
every MCP read tool of the Python app and writes the answers to `<out>/golden/*.json`, plus
`golden/manifest.json` and the generated store `<out>/store-v<N>.sqlite`.

Run with the venv's Python:  rust/parity/.venv/bin/python rust/parity/gen_golden.py [--out DIR]

It only reads: the app is imported from src/ and never changed, the fixtures are never written, and it
refuses to run when any env var points at the owner's real ~/.easy-personal, ~/.claude*, ~/.codex*, ...
POSIX only (`pwd`, `time.tzset`).
"""

import argparse
import asyncio
import hashlib
import json
import os
import platform
import pwd
import re
import shutil
import sqlite3
import sys
import tempfile
import time
import tomllib
from importlib import metadata
from pathlib import Path

PARITY = Path(__file__).resolve().parent
REPO = PARITY.parents[1]
DEFAULT_FIXTURES = REPO / "tests" / "fixtures" / "claude_code"
FIXTURES = DEFAULT_FIXTURES

# Fixed inputs of the run: everything that could vary per machine or per run is pinned here.
TOKEN, PORT = "parity-token", 5555
FROZEN_NOW = "2026-01-20T00:00:00+00:00"  # fixtures are dated 2026-01-15; the 7-day rule windows must see them
WINDOW = {"since": "2026-01-15T10:05:00Z", "until": "2026-01-15T10:15:00Z"}
MACHINE_ID = "00000000-0000-4000-8000-0000000000aa"
ENV_PATHS = (  # env vars that name a folder the app would read or write
    "EASY_PERSONAL_HOME",
    "EASY_PERSONAL_CLAUDE_PROJECTS",
    "EASY_PERSONAL_AGENT_ROOT",
    "EASY_PERSONAL_UI_REPORTS",
    "EASY_PERSONAL_VERIFIER_REPORTS",
    "EASY_MEMORY_HOME",
)
PROTECTED_PREFIXES = (
    ".claude",
    ".codex",
    ".easy-",
)  # folders of a real home, whether or not they exist yet
HOME_REFUSED = {
    "EASY_PERSONAL_AGENT_ROOT",
    "--out",
    "--fixtures",
}  # a real home itself is no place for these
DEMO_APP = "/work/demo-app"  # the `cwd` of the default fixtures; the kit goldens hold that this folder does not exist
GENERATOR = "rust/parity/gen_golden.py"  # what every manifest's `generator` starts with
MARKER = ".gen_golden.json"  # at the --out root, never inside an oracle folder
LOCKED = (
    "starlette",
    "anyio",
    "sse-starlette",
    "mcp",
)  # packages whose drift from uv.lock is warned about
SECRET_LIKE = re.compile(
    r"/home/|/Users/|\bsk-|ghp_"
)  # `\bsk-` so `task-`, `risk-` and `desk-` are not a key


# -- refusing the owner's real data (NFR-003); runs before easy_personal is imported ---------------------


def _real_homes() -> set[Path]:
    homes = {Path.home().resolve(), Path(pwd.getpwuid(os.getuid()).pw_dir).resolve()}
    return homes


def _protected() -> list[Path]:
    out = []
    for home in _real_homes():
        out += [home / ".easy-personal", home / ".easy-memory", home / ".easy-verifier-reports"]
        out += sorted(home.glob(".claude*")) + sorted(home.glob(".codex*"))
    return out


def refuse_real_paths(out_dir: Path) -> None:
    """Exit 1 when an env var, `--out` or `--fixtures` is, or is inside, the owner's real data; nothing is opened.

    Paths are resolved first (symlinks; a folder that does not exist yet). Inside a real home, any `.claude*`,
    `.codex*` or `.easy-*` folder is refused whether or not it exists. A real home itself is refused for the agent
    root, `--out` and `--fixtures`; `--fixtures` is also refused above a home (it would read the transcripts below).
    """
    homes = _real_homes()
    candidates = [(name, os.environ[name]) for name in ENV_PATHS if os.environ.get(name)]
    candidates.append(("--out", str(out_dir)))
    candidates.append(("--fixtures", str(FIXTURES)))
    for name, value in candidates:
        target = Path(value).expanduser().resolve()
        for home in homes:
            if name in HOME_REFUSED and target == home:
                sys.exit(f"refused: {name} is the real home folder (NFR-003); nothing was opened")
            if name == "--fixtures" and target in home.parents:
                sys.exit(
                    f"refused: {name} is above the real home folder (NFR-003); nothing was opened"
                )
            if home in target.parents:
                first = target.relative_to(home).parts[0]
                if first.startswith(PROTECTED_PREFIXES):
                    sys.exit(
                        f"refused: {name} points inside the real {first} (NFR-003); nothing was opened"
                    )


def refuse_fixture_folder() -> None:
    """The default fixtures say their `cwd` is /work/demo-app and the kit goldens hold that it is gone: it must not exist."""
    if Path(DEMO_APP).exists():
        sys.exit(
            f"refused: {DEMO_APP} exists on this machine; the goldens depend on it being absent (rename it for the run)"
        )


def warn_if_not_locked() -> None:
    """One stderr line when an installed package differs from uv.lock (`pip install -e .` ignores the lock). Never fails the run."""
    try:
        with open(REPO / "uv.lock", "rb") as handle:
            locked = {p["name"]: p["version"] for p in tomllib.load(handle)["package"]}
    except (OSError, ValueError, LookupError):
        return
    drift = []
    for name in LOCKED:
        try:
            installed = metadata.version(name)
        except metadata.PackageNotFoundError:
            installed = None
        if name in locked and installed != locked[name]:
            drift.append(f"{name} {installed} (uv.lock {locked[name]})")
    if drift:
        print(
            "warning: installed packages differ from uv.lock: "
            + ", ".join(drift)
            + "; the goldens may drift, "
            "install with `uv sync --frozen` (rust/parity/README.md)",
            file=sys.stderr,
        )


def _made_by_us(folder: Path) -> bool:
    """A folder this script wrote: its manifest.json names the generator."""
    try:
        return json.loads((folder / "manifest.json").read_text())["generator"].startswith(GENERATOR)
    except (OSError, ValueError, LookupError, TypeError, AttributeError):
        return False


def claim_output(out_dir: Path, unit: str, stores: list[Path]) -> None:
    """Delete only what this script wrote. `unit` is the folder this mode replaces, `stores` the store files it replaces beside it.

    Ours = listed in `<out>/.gen_golden.json` (written by an earlier run, outside every oracle folder so a `diff -r`
    against the committed folders stays empty) or a folder whose manifest names the generator (the committed layout).
    Anything else stops the run before one file is deleted. The unit is recorded before the first file is written.
    """
    try:
        made = set(json.loads((out_dir / MARKER).read_text())["made"])
    except (OSError, ValueError, LookupError, TypeError):
        made = set()
    folder = out_dir / unit
    ours = unit in made or (not folder.is_symlink() and _made_by_us(folder))
    if os.path.lexists(folder) and not ours:
        sys.exit(
            f"refused: {unit}/ in --out was not made by this generator (not in {MARKER}, no manifest of ours); nothing was deleted"
        )
    if stores and not ours:
        sys.exit(
            f"refused: {stores[0].relative_to(out_dir)} in --out was not made by this generator; nothing was deleted"
        )
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / MARKER).write_text(
        json.dumps({"generator": GENERATOR, "made": sorted(made | {unit})}, indent=2) + "\n"
    )


# -- environment of the run ------------------------------------------------------------------------------


def isolate(work: Path) -> None:
    """Point every folder the app reads at throwaway or fixture paths, in UTC, with a fake HOME."""
    os.environ["TZ"] = "UTC"  # the `day` breakdown groups by local day
    time.tzset()
    for name in ("verifier-none", "agent-root", "em-none", "home", "nowhere"):
        (work / name).mkdir(parents=True, exist_ok=True)
    os.environ["XDG_CONFIG_HOME"] = str(
        work / "nowhere" / ".config"
    )  # where the ingest-timer check looks
    os.environ["HOME"] = str(work / "nowhere")  # Path.home() can no longer reach the real one
    os.environ["EASY_PERSONAL_HOME"] = str(work / "home")
    os.environ["EASY_PERSONAL_CLAUDE_PROJECTS"] = str(FIXTURES)
    os.environ["EASY_PERSONAL_AGENT_ROOT"] = str(work / "agent-root")
    os.environ["EASY_MEMORY_HOME"] = str(work / "em-none")
    os.environ["EASY_PERSONAL_VERIFIER_REPORTS"] = str(work / "verifier-none")
    os.environ["EASY_PERSONAL_CONTEXT_URL"] = (
        "http://127.0.0.1:9/mcp"  # dead port, as tests/conftest.py
    )
    os.environ.pop("EASY_PERSONAL_UI_REPORTS", None)
    sys.path.insert(0, str(REPO / "src"))


def guard_sqlite() -> None:
    """Like tests/conftest.py: opening anything under the real ~/.easy-personal fails loudly."""
    real = [p.resolve() for p in _protected()]
    real_connect = sqlite3.connect

    def guarded(database, *args, **kwargs):
        if isinstance(database, str | Path) and str(database) != ":memory:":
            target = Path(database).resolve()
            if any(target == r or r in target.parents for r in real):
                raise AssertionError(f"refused to open the real store: {target}")
        return real_connect(database, *args, **kwargs)

    sqlite3.connect = guarded


def freeze_clock() -> None:
    """The rules, kit and report windows are relative to `now` and have no parameter for it: pin it."""
    from datetime import UTC, datetime

    now = datetime.fromisoformat(FROZEN_NOW).astimezone(UTC)

    class Frozen(datetime):
        @classmethod
        def now(cls, tz=None):
            return now.astimezone(tz) if tz else now.replace(tzinfo=None)

    from easy_personal.core import accounts, easymem, kit, kitperf, reports, rules
    from easy_personal.sources import context

    for module in (accounts, easymem, kit, kitperf, reports, rules, context):
        if (
            getattr(module, "datetime", None) is not datetime
        ):  # e.g. `import datetime` or an alias: the patch would miss it
            raise RuntimeError(
                f"{module.__name__} no longer has `datetime` imported as `from datetime import datetime`: the frozen clock would miss it"
            )
        module.datetime = Frozen


def unplug_machine_state(api, suggestions) -> None:
    """Keychain, `claude` CLI and systemd answers are machine state: pin them."""
    from easy_personal.core import schedule

    api._configured = lambda name: False  # every connector "not configured"
    suggestions.configured_ids = lambda: []  # noqa: PIE807 - no configured connector; unlike `list`, a call with an argument fails
    schedule.systemd_available = lambda: (
        True
    )  # first-run hint: no crontab line with this machine's command path


# -- the store -------------------------------------------------------------------------------------------


def tree_sha256(root: Path) -> str:
    h = hashlib.sha256()
    for path in sorted(p for p in root.rglob("*") if p.is_file()):
        h.update(path.relative_to(root).as_posix().encode() + b"\0")
        h.update(hashlib.sha256(path.read_bytes()).digest())
    return h.hexdigest()


def normalise_store(conn: sqlite3.Connection) -> None:
    """Replace what differs per machine (machine id, absolute paths, inode numbers) with fixed values."""
    files = [r[0] for r in conn.execute("SELECT path FROM source_files ORDER BY path")]
    with conn:
        conn.execute("UPDATE meta SET value = ? WHERE key = 'machine_id'", (MACHINE_ID,))
        for number, old in enumerate(files, start=1):
            new = "claude_code/" + Path(old).relative_to(FIXTURES).as_posix()
            conn.execute(
                "UPDATE source_files SET path = ?, inode = ? WHERE path = ?", (new, number, old)
            )
            conn.execute(
                "UPDATE events SET source_path = ?, inode = ? WHERE source_path = ?",
                (new, number, old),
            )
        conn.execute("UPDATE events SET machine_id = ?", (MACHINE_ID,))
        conn.execute("UPDATE source_files SET machine_id = ?", (MACHINE_ID,))


def build_store(work: Path, out_dir: Path) -> tuple[int, Path]:
    """Ingest the fixtures, normalise, save a compact single-file copy; returns (user_version, saved path)."""
    from easy_personal.core import config, store
    from easy_personal.core.migrations import MIGRATIONS
    from easy_personal.sources import claude_code

    version = max(number for number, _ in MIGRATIONS)
    with store.opened(work / "build.sqlite") as conn:
        claude_code.ingest(conn, FIXTURES)
        normalise_store(conn)
        if conn.execute("PRAGMA user_version").fetchone()[0] != version:
            raise RuntimeError("store user_version differs from the migrations list")
        saved = out_dir / f"store-v{version}.sqlite"
        for old in out_dir.glob("store-v*.sqlite*"):
            old.unlink()
        conn.execute("VACUUM INTO ?", (str(saved),))
    with sqlite3.connect(saved) as copy:  # a single file: no -wal / -shm for a read-only opener
        copy.execute("PRAGMA journal_mode=DELETE")
    shutil.copy(saved, config.store_path())  # the API runs on a copy: opening it switches it to WAL
    return version, saved


# -- what is asked ---------------------------------------------------------------------------------------

API_ENTRIES = [  # (entry id, path, query params)
    ("usage_summary__all", "/api/usage/summary", {}),
    ("usage_summary__window", "/api/usage/summary", WINDOW),
    ("usage_breakdown__day", "/api/usage/breakdown", {"by": "day"}),
    ("usage_breakdown__model", "/api/usage/breakdown", {"by": "model"}),
    ("usage_breakdown__project", "/api/usage/breakdown", {"by": "project"}),
    ("usage_breakdown__tool", "/api/usage/breakdown", {"by": "tool"}),
    ("usage_breakdown__model_window", "/api/usage/breakdown", {"by": "model", **WINDOW}),
    ("usage_breakdown__bad_by", "/api/usage/breakdown", {"by": "bad"}),
    ("usage_outliers__all", "/api/usage/outliers", {}),
    ("usage_sessions__all", "/api/usage/sessions", {}),
    ("usage_sessions__window", "/api/usage/sessions", WINDOW),
    ("usage_sessions__bad_since", "/api/usage/sessions", {"since": "bad"}),
    ("warnings__all", "/api/warnings", {}),
    ("plan__all", "/api/plan", {}),
    ("kit__all", "/api/kit", {}),
    ("kit_workspaces__all", "/api/kit/workspaces", {}),
    ("kit_reports__all", "/api/kit/reports", {}),
    ("kit_report_files__no_folder", "/api/kit/report-files", {}),
    ("kit_report_file__no_path", "/api/kit/report-file", {}),
    ("board__all", "/api/board", {}),
    ("accounts__by_account", "/api/accounts", {}),
    ("accounts__by_provider", "/api/accounts", {"by": "provider"}),
    ("accounts__bad_by", "/api/accounts", {"by": "bad"}),
    ("workspaces__all", "/api/workspaces", {}),
    ("workspaces_unassigned__all", "/api/workspaces/unassigned", {}),
    ("reports__all", "/api/reports", {}),
    ("reports_one__missing", "/api/reports/1", {}),
    ("memory_graph__list", "/api/memory-graph", {}),
    ("first_run__all", "/api/first-run", {}),
    ("sync_plan__all", "/api/sync", {}),
    ("connectors__all", "/api/connectors", {}),
]
GUARD_ENTRIES = [  # the 403 answers: the same body for a missing token, a wrong host and a foreign origin
    ("guard__no_token", "/api/usage/summary", {"no_token": True}),
    ("guard__bad_host", "/api/usage/summary", {"headers": {"Host": "evil.example"}}),
    ("guard__bad_origin", "/api/usage/summary", {"headers": {"Origin": "http://evil.example"}}),
]
SKIP_ROUTES = {  # route template -> reason (only routes that could be GET; write methods are listed by the script)
    "/api/live": "SSE stream: never ends, and its frames carry live clock readings",
    "/api/dirs": "browses an absolute folder given by the caller: needs real machine paths",
    "/api/jobs": "reads the machine's systemd user timers and crontab (machine state)",
    "/api/connectors/legacy": "reads the real keychain and `claude mcp list` (machine state)",
    "/{path:path}": "static SPA files of the built page, not an /api route",
}
MCP_CALLS = [  # (entry id, tool, arguments)
    ("usage_summary__all", "usage_summary", {}),
    ("usage_summary__window", "usage_summary", WINDOW),
    ("usage_breakdown__day", "usage_breakdown", {"by": "day"}),
    ("usage_breakdown__model", "usage_breakdown", {"by": "model"}),
    ("usage_breakdown__project", "usage_breakdown", {"by": "project"}),
    ("usage_breakdown__tool", "usage_breakdown", {"by": "tool"}),
    ("session_outliers__all", "session_outliers", {}),
    ("warnings__all", "warnings", {}),
    ("improvement_plan__all", "improvement_plan", {}),
    ("kit_summary__all", "kit_summary", {}),
    ("kit_findings__all", "kit_findings", {}),
    ("connector_catalog__all", "connector_catalog", {}),
    ("connector_status__all", "connector_status", {}),
    ("connector_suggestions__all", "connector_suggestions", {}),
    ("ping__all", "ping", {}),
]
MCP_PROMPTS = {
    "investigate"
}  # every prompt the app lists; the run asks each one (a new one fails the run)
MCP_RESOURCES: set[str] = set()  # every resource the app lists: none (a new one fails the run)
MCP_SKIPPED = {"save_report": "writes a row to the store (the only MCP write tool)"}
BAD_SINCE_NOTE = "compare status and the `error` key only: the message text is CPython-specific (datetime.fromisoformat)"  # rich mode


# -- running it ------------------------------------------------------------------------------------------


class Writer:
    """Collects golden files, scrubs per-run text, and keeps the manifest lists."""

    def __init__(self, golden: Path, work: Path) -> None:
        self.golden, self.entries, self.volatile, self.bodies = golden, [], [], {}
        self.scrub = [(str(work), "<WORK>"), (str(FIXTURES), "<FIXTURES>"), (str(REPO), "<REPO>")]

    def add(self, entry: dict, body, raw: str | None = None, note: str | None = None) -> None:
        """`raw` (rich mode): the body text as the app emitted it; its SHA-256 goes into the entry after the same scrub."""
        text = json.dumps(body, sort_keys=True, indent=2, ensure_ascii=False) + "\n"
        for needle, token in self.scrub:
            if needle in text:
                text = text.replace(needle, token)
                self.volatile.append(
                    {"file": entry["file"], "token": token, "reason": "absolute path of this run"}
                )
            if raw is not None:
                raw = raw.replace(needle, token)
            if needle in json.dumps(entry):  # a path in the request (the memory-graph `project`)
                entry = json.loads(json.dumps(entry).replace(needle, token))
        if SECRET_LIKE.search(text):
            raise RuntimeError(
                f"{entry['file']}: absolute home path or secret-like text in the output"
            )
        if raw is not None:
            entry = {
                **entry,
                "raw_sha256": None if note else hashlib.sha256(raw.encode("utf-8")).hexdigest(),
            }
        if note:
            entry = {**entry, "compare": note}
        (self.golden / entry["file"]).write_bytes(text.encode("utf-8"))
        self.entries.append(entry)
        self.bodies[entry["file"]] = body


def api_status_and_body(client, path, params, extra, headers_ok, raw_out: list | None = None):
    headers = {} if extra.get("no_token") else dict(headers_ok)
    headers.update(extra.get("headers", {}))
    res = client.get(path, params=params, headers=headers)
    if raw_out is not None:
        raw_out.append(res.text)
        if res.headers.get(
            "content-disposition"
        ):  # a served report file (rich mode): its bytes are not the app's JSON
            kept = (
                "content-type",
                "content-disposition",
                "x-content-type-options",
                "cache-control",
            )
            body = {
                "served_file": {"headers": {k: res.headers.get(k) for k in kept}, "text": res.text}
            }
            return res.status_code, body, res.headers.get("content-security-policy")
    try:
        body = res.json()
    except ValueError:
        raise RuntimeError(f"{path}: not a JSON body (status {res.status_code})") from None
    # the file is a re-dump of the body; prove the re-dump loses nothing (floats, ints, unicode)
    if json.dumps(body, separators=(",", ":"), ensure_ascii=False) != res.text:
        raise RuntimeError(f"{path}: re-serialising the body changed its text")
    return res.status_code, body, res.headers.get("content-security-policy")


def run_api(
    writer: Writer, work: Path, entries=None, guards=None, rich: bool = False
) -> tuple[list[dict], str | None]:
    from starlette.testclient import TestClient

    from easy_personal.surfaces import api

    client = TestClient(
        api.create_app(TOKEN, PORT, work / "no-static"), base_url=f"http://127.0.0.1:{PORT}"
    )
    auth = {api.TOKEN_HEADER: TOKEN}
    skipped, csp = [], None
    asked = set()
    for entry_id, path, params in (API_ENTRIES if entries is None else entries) + (
        GUARD_ENTRIES if guards is None else guards
    ):
        extra = params if entry_id.startswith("guard__") else {}
        query = {} if extra else params
        raw = []
        status, body, csp_seen = api_status_and_body(
            client, path, query, extra, auth, raw if rich else None
        )
        csp = csp or csp_seen
        asked.add(re.sub(r"/\d+$", "/{id}", path))
        writer.add(
            {
                "file": f"api__{entry_id}.json",
                "surface": "http",
                "route": re.sub(r"/\d+$", "/{id}", path),
                "method": "GET",
                "params": query,
                "request_headers": sorted(extra.get("headers", {}))
                + (["(no token)"] if extra.get("no_token") else []),
                "status": status,
            },
            body,
            raw[0] if raw else None,
            BAD_SINCE_NOTE if rich and "bad_since" in entry_id else None,
        )
    # every route of the app is a golden entry, a skipped read route, or a write route
    for route in client.app.routes:
        template = route.path
        methods = sorted(route.methods or [])
        if "GET" in methods:
            if template in asked:
                continue
            reason = SKIP_ROUTES.get(template)
            if reason is None:
                raise RuntimeError(f"GET {template} is neither a golden entry nor skipped: add it")
            skipped.append(
                {"surface": "http", "route": template, "method": "GET", "reason": reason}
            )
        elif not methods:  # a Mount
            skipped.append(
                {
                    "surface": "http",
                    "route": template,
                    "method": "GET",
                    "reason": SKIP_ROUTES["/{path:path}"],
                }
            )
        else:
            skipped.append(
                {
                    "surface": "http",
                    "route": template,
                    "method": methods[0],
                    "reason": "write route",
                }
            )
    # a skipped route that no longer exists is a stale list, not a silent pass
    known = {r.path for r in client.app.routes}
    for template in SKIP_ROUTES:
        if template not in known and template != "/{path:path}":
            raise RuntimeError(f"SKIP_ROUTES names {template}, which the app no longer serves")
    return skipped, csp


def run_mcp(writer: Writer, calls=None, prompts=None, rich: bool = False) -> list[dict]:
    from easy_personal.surfaces import mcp as mcp_surface

    app = mcp_surface.create_app()
    listed = asyncio.run(app.list_tools())
    names = {t.name for t in listed}
    tools = sorted(
        (t.model_dump(mode="json", exclude_none=True) for t in listed), key=lambda t: t["name"]
    )
    writer.add(
        {
            "file": "mcp__tools_list.json",
            "surface": "mcp",
            "tool": "tools/list",
            "params": {},
            "status": "ok",
        },
        tools,
        json.dumps(tools, separators=(",", ":"), ensure_ascii=False) if rich else None,
    )
    for kind, listed, known in (
        ("prompt", {p.name for p in asyncio.run(app.list_prompts())}, MCP_PROMPTS),
        ("resource", {str(r.uri) for r in asyncio.run(app.list_resources())}, MCP_RESOURCES),
    ):
        if listed != known:
            raise RuntimeError(
                f"MCP {kind}s differ from the known list: new {sorted(listed - known)}, gone {sorted(known - listed)}: update the generator"
            )
    calls = MCP_CALLS if calls is None else calls
    asked = {tool for _, tool, _ in calls}
    for entry_id, tool, args in calls:
        _, structured = asyncio.run(app.call_tool(tool, args))
        writer.add(
            {
                "file": f"mcp__{entry_id}.json",
                "surface": "mcp",
                "tool": tool,
                "params": args,
                "status": "ok",
            },
            structured,
            json.dumps(structured, separators=(",", ":"), ensure_ascii=False) if rich else None,
        )
    for name, args in (
        (("investigate", {}), ("investigate", {"workspace": "demo"}))
        if prompts is None
        else prompts
    ):
        result = asyncio.run(app.get_prompt(name, args))
        text = "\n".join(m.content.text for m in result.messages)
        suffix = "__scoped" if args else "__all"
        writer.add(
            {
                "file": f"mcp__prompt_{name}{suffix}.json",
                "surface": "mcp",
                "prompt": name,
                "params": args,
                "status": "ok",
            },
            {"text": text},
            json.dumps({"text": text}, separators=(",", ":"), ensure_ascii=False) if rich else None,
        )
    skipped = []
    for tool in sorted(names - asked):
        reason = MCP_SKIPPED.get(tool)
        if reason is None:
            raise RuntimeError(f"MCP tool {tool} is neither a golden entry nor skipped: add it")
        skipped.append({"surface": "mcp", "tool": tool, "reason": reason})
    return skipped


def versions() -> dict:
    out = {"python": platform.python_version(), "platform": platform.system()}
    for name in (
        "easy-personal-mcp",
        "mcp",
        "starlette",
        "uvicorn",
        "keyring",
        "anyio",
        "httpx",
        "sse-starlette",
    ):
        try:
            out[name] = metadata.version(name)
        except metadata.PackageNotFoundError:
            out[name] = None
    return out


# -- opt-in: goldens for a non-UTC machine zone (T009; `--tz`) --------------------------------------------
#
# The default goldens are made under TZ=UTC and cannot show a zone bug. With `--tz` the generator ingests a copy of
# the fixtures plus one extra transcript file (the committed fixtures and the default store stay untouched), then asks
# the usage routes under each zone below and writes `<out>/golden-tz/<zone>/*.json`, `<out>/golden-tz/store-v<N>.sqlite`
# and `<out>/golden-tz/manifest.json`. Nothing under `<out>/golden/` is read or written in this mode.

TZ_ZONES = (
    "Asia/Bangkok",
    "America/New_York",
    "Nowhere/Invalid",
)  # the last is no zone: libc falls back to UTC
TZ_SESSION = "00000000-0000-4000-8000-000000000005"
TZ_EVENTS = (  # UTC time of one assistant message each; the label says why it is there (the local result is per zone)
    "2026-01-15T16:59:59.999Z",  # Bangkok: last instant of Jan 15
    "2026-01-15T17:00:00.000Z",  # Bangkok: first instant of Jan 16
    "2026-01-15T20:00:00.000Z",  # Bangkok: 03:00 on the next day
    "2026-01-16T04:59:59.000Z",  # New York: 23:59:59 on Jan 15
    "2026-01-16T05:00:00.000Z",  # New York: midnight
    "2026-03-08T04:59:59.000Z",  # New York, spring-forward day (23 hours): last second of Mar 7
    "2026-03-08T05:00:00.000Z",  # first instant of Mar 8
    "2026-03-08T06:59:59.000Z",  # 01:59:59 EST, the last second before the change
    "2026-03-08T07:00:00.000Z",  # 03:00 EDT, the first second after it
    "2026-03-08T07:15:00.000Z",  # before the skipped 02:30 read as 07:30Z
    "2026-03-08T07:45:00.000Z",  # after it
    "2026-03-09T03:59:59.000Z",  # 23:59:59 EDT, last second of the 23-hour day
    "2026-03-09T04:00:00.000Z",  # Mar 9 midnight
    "2026-11-01T03:59:59.000Z",  # New York, fall-back day (25 hours): last second of Oct 31
    "2026-11-01T04:00:00.000Z",  # Nov 1 midnight
    "2026-11-01T05:30:00.000Z",  # 01:30 EDT, first occurrence
    "2026-11-01T06:00:00.000Z",  # 01:00 EST, right after the change
    "2026-11-01T06:30:00.000Z",  # 01:30 EST, second occurrence
    "2026-11-02T04:59:59.000Z",  # 23:59:59 EST, last second of the 25-hour day
    "2026-11-02T05:00:00.000Z",  # Nov 2 midnight
    "2026-12-31T16:59:59.000Z",  # year boundary, Bangkok: Dec 31
    "2026-12-31T17:00:00.000Z",  # Bangkok: Jan 1
    "2027-01-01T04:59:59.000Z",  # New York: Dec 31
    "2027-01-01T05:00:00.000Z",  # New York: Jan 1
    "2028-02-28T16:59:59.000Z",  # leap year, Bangkok: Feb 28
    "2028-02-28T17:00:00.000Z",  # Bangkok: Feb 29
    "2028-02-29T04:59:59.000Z",  # New York: Feb 28
    "2028-02-29T05:00:00.000Z",  # New York: Feb 29
    "2028-02-29T16:59:59.000Z",  # Bangkok: still Feb 29
    "2028-02-29T17:00:00.000Z",  # Bangkok: Mar 1
)
TZ_MODELS = (
    "claude-fixture-model-1",
    "claude-fixture-model-2",
)  # neither is in the price table: usd stays null
TZ_ENTRIES = (  # (case, route, query): offset-less bounds on purpose, plus two that carry an offset
    (
        "usage_summary__window_date",
        "/api/usage/summary",
        {"since": "2026-01-15", "until": "2026-01-16"},
    ),
    (
        "usage_summary__window_datetime",
        "/api/usage/summary",
        {"since": "2026-01-15T23:30", "until": "2026-01-16T03:30:00.5"},
    ),
    (
        "usage_summary__dst_spring_day",
        "/api/usage/summary",
        {"since": "2026-03-08", "until": "2026-03-09"},
    ),
    (
        "usage_summary__dst_fall_day",
        "/api/usage/summary",
        {"since": "2026-11-01", "until": "2026-11-02"},
    ),
    (
        "usage_summary__dst_skipped_hour",
        "/api/usage/summary",
        {"since": "2026-03-08T02:30", "until": "2026-03-08T04:30"},
    ),
    (
        "usage_summary__dst_repeated_hour",
        "/api/usage/summary",
        {"since": "2026-11-01T01:30", "until": "2026-11-01T01:45"},
    ),
    (
        "usage_summary__offset_z",
        "/api/usage/summary",
        {"since": "2026-03-08T00:00:00Z", "until": "2026-03-09T00:00:00Z"},
    ),
    (
        "usage_summary__offset_plus7",
        "/api/usage/summary",
        {"since": "2026-01-16T00:00:00+07:00", "until": "2026-01-17T00:00:00+07:00"},
    ),
    ("usage_breakdown__day_all", "/api/usage/breakdown", {"by": "day"}),
    (
        "usage_breakdown__day_window",
        "/api/usage/breakdown",
        {"by": "day", "since": "2026-03-01", "until": "2026-04-01"},
    ),
    (
        "usage_breakdown__day_leap",
        "/api/usage/breakdown",
        {"by": "day", "since": "2028-02-28", "until": "2028-03-02"},
    ),
    (
        "usage_breakdown__day_year",
        "/api/usage/breakdown",
        {"by": "day", "since": "2026-12-31", "until": "2027-01-02"},
    ),
    (
        "usage_breakdown__day_offset_z",
        "/api/usage/breakdown",
        {"by": "day", "since": "2026-11-01T00:00:00Z", "until": "2026-11-03T00:00:00Z"},
    ),
    (
        "usage_breakdown__model_window",
        "/api/usage/breakdown",
        {"by": "model", "since": "2026-03-08", "until": "2026-03-09"},
    ),
    (
        "usage_breakdown__model_fall_window",
        "/api/usage/breakdown",
        {"by": "model", "since": "2026-11-01T01:30", "until": "2026-11-02"},
    ),
)


def tz_fixture_tree(work: Path) -> Path:
    """A copy of the fixtures plus `tz-extra/session.jsonl` (the TZ_EVENTS messages). The originals are only read."""
    tree = work / "tz-fixtures"
    shutil.copytree(DEFAULT_FIXTURES, tree)
    lines = []
    for number, stamp in enumerate(TZ_EVENTS):
        lines.append(
            json.dumps(
                {
                    "parentUuid": None,
                    "isSidechain": False,
                    "userType": "external",
                    "entrypoint": "cli",
                    "cwd": "/work/demo-app",
                    "sessionId": TZ_SESSION,
                    "version": "2.1.293",
                    "gitBranch": "main",
                    "uuid": f"55555555-0000-4000-8000-{number:012d}",
                    "timestamp": stamp,
                    "type": "assistant",
                    "requestId": f"req_tz_{number}",
                    "message": {
                        "id": f"msg_tz_{number}",
                        "type": "message",
                        "role": "assistant",
                        "model": TZ_MODELS[number % 2],
                        "content": [{"type": "text", "text": "INVENTED_TZ_REPLY"}],
                        "stop_reason": None,
                        "stop_sequence": None,
                        # powers of two: every set of messages has its own sum, so one message on the wrong day shows
                        "usage": {
                            "input_tokens": 1,
                            "output_tokens": 1 << number,
                            "cache_read_input_tokens": 0,
                            "cache_creation_input_tokens": 0,
                        },
                    },
                },
                separators=(",", ":"),
            )
        )
    extra = tree / "tz-extra"
    extra.mkdir()
    (extra / "session.jsonl").write_text("\n".join(lines) + "\n")
    return tree


def tzdata_version() -> str | None:
    """The tz database version of the system, e.g. `2026a` (the Rust core reads the same files)."""
    try:
        with open("/usr/share/zoneinfo/tzdata.zi") as handle:
            first = handle.readline().split()
    except OSError:
        return None
    return first[2] if first[:2] == ["#", "version"] and len(first) > 2 else None


def write_tz_goldens(out_dir: Path, work: Path) -> None:
    global FIXTURES
    FIXTURES = tz_fixture_tree(work)
    tz_dir = out_dir / "golden-tz"
    if tz_dir.exists():
        shutil.rmtree(tz_dir)
    tz_dir.mkdir(parents=True)
    version, saved = build_store(work, tz_dir)
    from starlette.testclient import TestClient

    from easy_personal.surfaces import api

    client = TestClient(
        api.create_app(TOKEN, PORT, work / "no-static"), base_url=f"http://127.0.0.1:{PORT}"
    )
    auth = {api.TOKEN_HEADER: TOKEN}
    zones = {}
    for zone in TZ_ZONES:
        os.environ["TZ"] = zone
        time.tzset()  # the app reads local time through libc; this is what the OS zone or `TZ` does for it
        zone_dir = tz_dir.joinpath(*zone.split("/"))
        zone_dir.mkdir(parents=True)
        writer = Writer(zone_dir, work)
        for case, path, query in TZ_ENTRIES:
            status, body, _ = api_status_and_body(client, path, query, {}, auth)
            writer.add(
                {
                    "file": f"api__{case}.json",
                    "surface": "http",
                    "route": path,
                    "method": "GET",
                    "params": query,
                    "status": status,
                },
                body,
            )
        zones[zone] = {
            "dir": "/".join(zone.split("/")),
            "entries": sorted(writer.entries, key=lambda e: e["file"]),
        }
    os.environ["TZ"] = "UTC"
    time.tzset()
    manifest = {
        "generator": "rust/parity/gen_golden.py --tz",
        "note": "usage_summary / usage_breakdown under a non-UTC machine zone. One folder per zone; `Nowhere/Invalid` is not a zone, so libc reads UTC. Object keys are sorted, lists keep the app's order.",
        "versions": versions(),
        "tzdata_version": tzdata_version(),
        "store": {"file": saved.name, "user_version": version, "bytes": saved.stat().st_size},
        "fixture_dir": "tests/fixtures/claude_code + generated tz-extra/session.jsonl",
        "fixture_sha256": tree_sha256(FIXTURES),
        "extra_events": list(TZ_EVENTS),
        "token": TOKEN,
        "zones": zones,
    }
    text = json.dumps(manifest, sort_keys=True, indent=2, ensure_ascii=False) + "\n"
    if SECRET_LIKE.search(text):
        raise RuntimeError("golden-tz manifest.json: absolute home path or secret-like text")
    (tz_dir / "manifest.json").write_bytes(text.encode("utf-8"))
    print(
        f"{sum(len(z['entries']) for z in zones.values())} tz golden entries in {len(zones)} zones; store v{version}, {saved.stat().st_size} bytes"
    )


# -- opt-in: a richer invented store and its goldens (T007; `--rich`) -----------------------------------------
#
# The default goldens come from one day of data: warnings and plan only abstain, and outliers, kit, board, accounts,
# workspaces and reports are empty. `--rich` ingests the committed transcripts under `fixtures-rich/` (made by
# `make_fixtures_rich.py`) the way the CLI does (accounts, `ingest_accounts`, kernel traces, easy-context), adds two
# workspaces, an account pick and two saved reports through Python's own write functions, then asks every route and
# MCP tool again, and the filter parameters too. It writes `<out>/golden-rich/*.json`, `<out>/golden-rich/manifest.json`
# and `<out>/store-rich/store-v<N>.sqlite`. Nothing under `<out>/golden/`, `<out>/golden-tz/` or the default store is
# read or written in this mode.

RICH_FIXTURES = PARITY / "fixtures-rich"
RICH_PREFIX = "/work/rich"  # what the work folder is called inside the saved store (the folders are gone on any machine)
RICH_WINDOW = {
    "since": "2026-01-13T00:00:00Z",
    "until": "2026-01-20T00:00:00Z",
}  # the rules' 7-day window
RICH_WINDOW_MID = {
    "since": "2026-01-12T18:00:00Z",
    "until": "2026-01-15T06:00:00Z",
}  # cuts a session at its start
RICH_MTIME = (
    1768694400  # 2026-01-18T00:00:00Z: file times are part of the report-folder and board answers
)
RICH_SCOPES = (  # (id suffix, query) for the filter parameters: workspace name, account label or id
    ("workspace_acme", {"workspace": "acme"}),
    ("workspace_personal", {"workspace": "personal"}),
    ("account_work", {"account": "Claude work"}),
    ("account_default", {"account": "1"}),
    ("workspace_acme_account_work", {"workspace": "acme", "account": "Claude work"}),
)
NEVER_FIRES = {  # rule id -> why the rich store cannot make it fire (README says the same)
    "context_index_stale": "it fires only when a project's latest commit is newer than its easy-context index, which needs a "
    "git repo at a path that exists on the machine running the test; the golden must not depend on that, so the rule is "
    "measured (0 stale projects, from an easy-context reply the generator supplies) and never fires",
}
EASY_CONTEXT_REPLY = {  # what `list_indexes` answers in this run (no network): roots the store knows, one it does not, one gone
    "indexes": [
        {"root": "@KIT@/alpha", "last_index_at": "2026-01-10T00:00:00Z", "root_exists": True},
        {"root": "/work/rich/gone-a", "last_index_at": "2026-01-05T00:00:00Z", "root_exists": True},
        {
            "root": "/work/rich/gone-b",
            "last_index_at": "2026-01-05T00:00:00Z",
            "root_exists": False,
        },
        {
            "root": "/work/rich/not-in-the-store",
            "last_index_at": "2026-01-05T00:00:00Z",
            "root_exists": True,
        },
    ]
}


def rich_work_tree(work: Path) -> None:
    """Copy `fixtures-rich` into the work folder: the two agent roots, and the kit projects (`@KIT@` becomes their folder)."""
    kit = work / "kit"
    shutil.copytree(FIXTURES / "kit", kit)
    for source, target in (("claude", ".claude"), ("claude-work", ".claude-work")):
        shutil.copytree(FIXTURES / source, work / "agent-root" / target, dirs_exist_ok=True)
    for path in sorted((work / "agent-root").rglob("*.jsonl")):
        path.write_text(path.read_text().replace("@KIT@", str(kit)))
    for path in [
        work / "kit",
        work / "agent-root",
        *(work / "kit").rglob("*"),
        *(work / "agent-root").rglob("*"),
    ]:
        os.utime(path, (RICH_MTIME, RICH_MTIME))


def rich_leak_check() -> None:
    """The invented fixtures hold no home path and no secret-like text; the run fails on a match (as the goldens do)."""
    for path in sorted(FIXTURES.rglob("*")):
        if path.is_file() and SECRET_LIKE.search(path.read_text()):
            raise RuntimeError(
                f"{path.relative_to(FIXTURES)}: absolute home path or secret-like text in a fixture"
            )


def guard_machine_state() -> None:
    """On top of `unplug_machine_state`: the `claude` CLI and the keychain are never reached; a call fails the run."""
    from easy_personal.connectors import writer

    def refuse(*args, **kwargs):
        raise AssertionError("the parity run must not reach the claude CLI or the keychain")

    writer._claude = refuse
    writer._keychain_item = refuse


def normalise_rich_store(conn: sqlite3.Connection, work: Path) -> None:
    """Replace what differs per machine: the work folder in every text column, machine ids, inode numbers."""
    tables = [
        r[0]
        for r in conn.execute("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
    ]
    with conn:
        for table in tables:
            for _, column, ctype, *_ in conn.execute(f"PRAGMA table_info({table})").fetchall():
                if ctype.upper() == "TEXT":
                    conn.execute(
                        f"UPDATE {table} SET {column} = replace({column}, ?, ?) WHERE instr({column}, ?) > 0",
                        (str(work), RICH_PREFIX, str(work)),
                    )
        conn.execute("UPDATE meta SET value = ? WHERE key = 'machine_id'", (MACHINE_ID,))
        for table in ("events", "source_files", "kit_trace_events", "tool_signals"):
            conn.execute(f"UPDATE {table} SET machine_id = ?", (MACHINE_ID,))
        files = [r[0] for r in conn.execute("SELECT path FROM source_files ORDER BY path")]
        for number, path in enumerate(files, start=1):
            conn.execute("UPDATE source_files SET inode = ? WHERE path = ?", (number, path))
            conn.execute("UPDATE events SET inode = ? WHERE source_path = ?", (number, path))
            conn.execute(
                "UPDATE kit_trace_events SET inode = ? WHERE source_path = ?", (number, path)
            )
            conn.execute("UPDATE tool_signals SET inode = ? WHERE source_path = ?", (number, path))


def build_rich_store(work: Path, out_dir: Path) -> tuple[int, Path]:
    """Ingest like `easy-personal ingest`, then write workspaces, an account pick and two reports with Python's own functions."""
    from datetime import datetime

    from easy_personal.core import accounts, config, reports, store, workspaces
    from easy_personal.core.migrations import MIGRATIONS
    from easy_personal.sources import claude_code, context, kit_trace, ui, verifier

    version = max(number for number, _ in MIGRATIONS)
    kit = work / "kit"
    reply = json.loads(json.dumps(EASY_CONTEXT_REPLY).replace("@KIT@", str(kit)))
    context.call_tool = lambda tool, arguments=None, *, url=None, root=None: (
        reply
    )  # no network: the service answer is supplied
    os.environ.pop(
        "EASY_PERSONAL_CLAUDE_PROJECTS", None
    )  # each account reads its own `<folder>/projects`
    now = datetime.fromisoformat(FROZEN_NOW)
    with store.opened(config.store_path()) as conn:
        accounts.detect(conn)
        work_id = conn.execute(
            "SELECT id FROM agent_accounts WHERE folder = ?",
            (str(work / "agent-root" / ".claude-work"),),
        ).fetchone()[0]
        accounts.confirm(conn, str(work_id), "Claude work")
        claude_code.ingest_accounts(conn)
        kit_trace.ingest(conn)
        context.ingest(conn)
        ui.ingest(conn)
        verifier.ingest(conn)
        workspaces.add(
            conn,
            "acme",
            repos=[str(kit / "alpha"), "/work/rich/gone-a"],
            color="#3366cc",
            description="invented workspace one",
        )
        workspaces.add(
            conn,
            "personal",
            repos=[str(kit / "beta"), "/work/rich/gone-b"],
            email="invented@example.invalid",
        )
        accounts.use(conn, str(work_id), ["acme"])
        reports.save(
            conn,
            "Invented summary: cache reuse is low and sessions grow large.\nSecond line of the summary.",
            [
                {"text": "Run /compact earlier in long sessions.", "number": 205000},
                {"text": "Keep the prompt prefix stable.", "tool": "Read"},
            ],
            None,
            None,
            now,
        )
        reports.save(
            conn,
            "Invented scoped summary for the acme workspace.",
            [{"text": "Check the failing Bash calls."}],
            "acme",
            None,
            now,
        )
        user_version = conn.execute("PRAGMA user_version").fetchone()[0]
        if user_version != version:
            raise RuntimeError("store user_version differs from the migrations list")
        conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        conn.execute(
            "VACUUM INTO ?", (str(work / "snapshot.sqlite"),)
        )  # before any API call can touch the live store
    snapshot = work / "snapshot.sqlite"
    with store.opened(snapshot) as conn:
        normalise_rich_store(conn, work)
        saved = out_dir / f"store-v{version}.sqlite"
        out_dir.mkdir(parents=True, exist_ok=True)
        for old in out_dir.glob("store-v*.sqlite*"):
            old.unlink()
        conn.execute("VACUUM INTO ?", (str(saved),))
    with sqlite3.connect(saved) as copy:
        copy.execute("PRAGMA journal_mode=DELETE")
    if str(work).encode() in saved.read_bytes():
        raise RuntimeError("the saved store still holds the work folder")
    if SECRET_LIKE.search(
        " ".join(
            row[0]
            for row in sqlite3.connect(saved).execute(
                "SELECT project_path FROM events WHERE project_path IS NOT NULL"
            )
        )
    ):
        raise RuntimeError("the saved store holds a home path")
    return version, saved


def rich_api_entries(work: Path) -> list[tuple[str, str, dict]]:
    """The default entries over the rich store, then every filter parameter and the report-folder parameters."""
    entries = []
    for entry_id, path, params in API_ENTRIES:
        if entry_id == "reports_one__missing":
            continue  # the rich store holds reports: see reports_one__found / __scoped / __missing below
        entries.append(
            (
                entry_id,
                path,
                {**params, **RICH_WINDOW} if params.get("since") == WINDOW["since"] else params,
            )
        )
    entries += [
        ("usage_summary__window_mid", "/api/usage/summary", RICH_WINDOW_MID),
        (
            "usage_breakdown__model_window_mid",
            "/api/usage/breakdown",
            {"by": "model", **RICH_WINDOW_MID},
        ),
        (
            "usage_breakdown__project_window",
            "/api/usage/breakdown",
            {"by": "project", **RICH_WINDOW},
        ),
        ("usage_breakdown__tool_window", "/api/usage/breakdown", {"by": "tool", **RICH_WINDOW}),
        ("usage_breakdown__day_window", "/api/usage/breakdown", {"by": "day", **RICH_WINDOW}),
        ("usage_outliers__window", "/api/usage/outliers", RICH_WINDOW),
        ("usage_sessions__window_mid", "/api/usage/sessions", RICH_WINDOW_MID),
        ("kit_workspaces__window", "/api/kit/workspaces", RICH_WINDOW),
        ("reports_one__found", "/api/reports/1", {}),
        ("reports_one__scoped", "/api/reports/2", {}),
        ("reports_one__missing", "/api/reports/99", {}),
        ("memory_graph__project", "/api/memory-graph", {"project": str(work / "kit" / "alpha")}),
        ("memory_graph__unknown_project", "/api/memory-graph", {"project": "nope"}),
        (
            "kit_report_files__ok",
            "/api/kit/report-files",
            {"workspace": "acme", "repo_no": "0", "folder": "reports"},
        ),
        (
            "kit_report_files__smoke",
            "/api/kit/report-files",
            {"workspace": "acme", "repo_no": "0", "folder": "smoke-reports"},
        ),
        (
            "kit_report_files__missing_folder",
            "/api/kit/report-files",
            {"workspace": "acme", "repo_no": "0", "folder": "docs"},
        ),
        (
            "kit_report_files__bad_repo_no",
            "/api/kit/report-files",
            {"workspace": "acme", "repo_no": "7", "folder": "reports"},
        ),
        (
            "kit_report_file__html",
            "/api/kit/report-file",
            {"workspace": "acme", "repo_no": "0", "folder": "reports", "path": "index.html"},
        ),
        (
            "kit_report_file__json",
            "/api/kit/report-file",
            {"workspace": "acme", "repo_no": "0", "folder": "reports", "path": "session-001.json"},
        ),
        (
            "kit_report_file__missing",
            "/api/kit/report-file",
            {"workspace": "acme", "repo_no": "0", "folder": "reports", "path": "nope.html"},
        ),
    ]
    for suffix, scope in RICH_SCOPES:
        entries += [
            (f"usage_summary__{suffix}", "/api/usage/summary", scope),
            (f"usage_breakdown__model_{suffix}", "/api/usage/breakdown", {"by": "model", **scope}),
            (
                f"usage_breakdown__project_{suffix}",
                "/api/usage/breakdown",
                {"by": "project", **scope},
            ),
            (f"usage_outliers__{suffix}", "/api/usage/outliers", scope),
            (f"usage_sessions__{suffix}", "/api/usage/sessions", scope),
            (f"warnings__{suffix}", "/api/warnings", scope),
            (f"plan__{suffix}", "/api/plan", scope),
            (f"kit__{suffix}", "/api/kit", scope),
            (f"board__{suffix}", "/api/board", scope),
        ]
    entries += [
        (
            "usage_summary__window_workspace_acme",
            "/api/usage/summary",
            {**RICH_WINDOW, "workspace": "acme"},
        ),
        (
            "usage_breakdown__day_workspace_personal",
            "/api/usage/breakdown",
            {"by": "day", "workspace": "personal"},
        ),
        ("usage_summary__workspace_unknown", "/api/usage/summary", {"workspace": "nope"}),
        ("usage_summary__account_unknown", "/api/usage/summary", {"account": "nope"}),
        ("warnings__workspace_unknown", "/api/warnings", {"workspace": "nope"}),
        ("plan__account_unknown", "/api/plan", {"account": "nope"}),
        ("kit__workspace_unknown", "/api/kit", {"workspace": "nope"}),
        ("board__account_unknown", "/api/board", {"account": "nope"}),
        ("kit_workspaces__account_work", "/api/kit/workspaces", {"account": "Claude work"}),
    ]
    return entries


def rich_mcp_calls() -> list[tuple[str, str, dict]]:
    calls = []
    for entry_id, tool, args in MCP_CALLS:
        calls.append((entry_id, tool, RICH_WINDOW if args == WINDOW else args))
    calls += [
        ("usage_summary__window_mid", "usage_summary", RICH_WINDOW_MID),
        ("usage_breakdown__model_window", "usage_breakdown", {"by": "model", **RICH_WINDOW}),
        ("session_outliers__window", "session_outliers", RICH_WINDOW),
        ("kit_findings__limit_10", "kit_findings", {"limit": 10}),
        ("kit_findings__since_week", "kit_findings", {"since": "2026-01-06T00:00:00Z"}),
    ]
    for suffix, scope in RICH_SCOPES:
        calls += [
            (f"{tool_id}__{suffix}", tool, scope)
            for tool_id, tool in (
                ("warnings", "warnings"),
                ("improvement_plan", "improvement_plan"),
                ("kit_summary", "kit_summary"),
            )
        ]
    return calls


def rule_coverage(conn: sqlite3.Connection, entries: list[dict]) -> dict:
    """Per rule: the warnings entries where it fired, where it was measured and clear, where it abstained, where it was left out.

    Uses `rules.evaluate(checked=True)` over the same store and scope, and checks its hits against the golden body.
    """
    from datetime import UTC, datetime

    from easy_personal.core import rules

    now = datetime.fromisoformat(FROZEN_NOW).astimezone(UTC)
    ids = [r.id for r in rules.load_rules()]
    cov = {i: {"fired": [], "clear": [], "abstained": [], "left_out": []} for i in ids}
    for entry in entries:
        if entry.get("route") != "/api/warnings" or entry["status"] != 200:
            continue
        found = rules.evaluate(
            conn,
            now,
            workspace=entry["params"].get("workspace"),
            account=entry["params"].get("account"),
            checked=True,
        )
        fired = {h["rule_id"] for h in found["hits"]}
        clear = {c["rule_id"] for c in found["checked"] if not c["fired"]}
        abstained = {a["rule_id"] for a in found["abstained"]}
        name = entry["file"].removeprefix("api__").removesuffix(".json")
        for rule_id in ids:
            state = (
                "fired"
                if rule_id in fired
                else "clear"
                if rule_id in clear
                else "abstained"
                if rule_id in abstained
                else "left_out"
            )
            cov[rule_id][state].append(name)
    return cov


def write_rich_goldens(out_dir: Path, work: Path) -> None:
    rich_leak_check()
    rich_work_tree(work)
    rich_dir = out_dir / "golden-rich"
    if rich_dir.exists():
        shutil.rmtree(rich_dir)
    rich_dir.mkdir(parents=True)
    store_dir = out_dir / "store-rich"
    freeze_clock()
    from easy_personal.connectors import suggestions
    from easy_personal.core import config, store
    from easy_personal.surfaces import api

    unplug_machine_state(api, suggestions)
    guard_machine_state()
    version, saved = build_rich_store(work, store_dir)
    writer = Writer(rich_dir, work)
    skipped_http, csp = run_api(writer, work, rich_api_entries(work), None, rich=True)
    skipped_mcp = run_mcp(
        writer,
        rich_mcp_calls(),
        (("investigate", {}), ("investigate", {"workspace": "acme"})),
        rich=True,
    )
    entries = sorted(writer.entries, key=lambda e: e["file"])
    # the pair the Rust tests use must agree: the same answers over the saved (normalised) store as over the live one
    check_saved_store(saved, writer)
    with store.opened(config.store_path()) as conn:
        coverage = rule_coverage(conn, entries)
    for rule_id, found in coverage.items():
        if not found["fired"] and rule_id not in NEVER_FIRES:
            raise RuntimeError(f"rule {rule_id} fires in no warnings entry")
        if not found["clear"]:
            raise RuntimeError(f"rule {rule_id} is never measured and clear in a warnings entry")
    manifest = {
        "generator": "rust/parity/gen_golden.py --rich",
        "note": "golden files hold the response body only; status is here. Object keys are sorted in the golden files (so a file does not show the order the app emitted); `raw_sha256` is the SHA-256 of the raw body text as the app emitted it (HTTP: the response body; MCP: the structured result as compact JSON, key order as FastMCP returned it), taken after the `<WORK>`/`<FIXTURES>`/`<REPO>` scrub. The order rule: compare the golden files as parsed JSON, and compare `raw_sha256` when the order of the keys matters. Lists keep the app's order.",
        "key_order": "golden files: sorted keys, compared as parsed JSON; raw_sha256: the order the app emitted",
        "versions": versions(),
        "store": {
            "file": f"../store-rich/{saved.name}",
            "user_version": version,
            "bytes": saved.stat().st_size,
        },
        "fixture_dir": "rust/parity/fixtures-rich",
        "fixture_sha256": tree_sha256(FIXTURES),
        "timezone": "UTC",
        "frozen_now": FROZEN_NOW,
        "token": TOKEN,
        "csp": csp,
        "windows": {"rules": RICH_WINDOW, "mid": RICH_WINDOW_MID},
        "compare_notes": {"api__usage_sessions__bad_since.json": BAD_SINCE_NOTE},
        "rule_coverage": coverage,
        "cannot_fire": NEVER_FIRES,
        "volatile": sorted(writer.volatile, key=lambda v: (v["file"], v["token"]))
        + [
            {
                "field": "store.events.machine_id / source_files.machine_id / kit_trace_events.machine_id / meta.machine_id",
                "token": MACHINE_ID,
                "reason": "random uuid per store",
            },
            {
                "field": "store.* text columns holding a path (source_path, project_path, folder, repo_path, ...)",
                "token": RICH_PREFIX + "/<relative path>",
                "reason": "the generator's work folder is replaced by a fixed prefix; those folders do not exist on a test machine, so usage groups them as Scratch",
            },
            {
                "field": "store.*.inode",
                "token": "1..N in sorted path order",
                "reason": "inode of this checkout",
            },
            {
                "field": "now (rules/kit/kitperf/accounts/reports windows)",
                "token": FROZEN_NOW,
                "reason": "clock patched in the generator: the routes have no parameter for it",
            },
            {
                "field": "mtime of every copied fixture file",
                "token": "2026-01-18T00:00:00Z",
                "reason": "report-folder and board answers show file times",
            },
        ],
        "patched": [
            {
                "what": "api._configured, suggestions.configured_ids",
                "to": "every connector 'not configured'",
                "reason": "keychain and `claude` CLI are machine state",
            },
            {
                "what": "writer._claude, writer._keychain_item",
                "to": "raise (the run fails if one is reached)",
                "reason": "the claude CLI and the keychain must never run",
            },
            {
                "what": "schedule.systemd_available",
                "to": "True",
                "reason": "first-run hint would otherwise differ per machine; XDG_CONFIG_HOME points at an empty folder",
            },
            {
                "what": "context.call_tool",
                "to": "a fixed list_indexes reply (EASY_CONTEXT_REPLY)",
                "reason": "no network; the service is machine state",
            },
            {
                "what": "datetime.now in rules, kit, kitperf, reports, accounts, easymem, context",
                "to": FROZEN_NOW,
                "reason": "see frozen_now",
            },
        ],
        "entries": entries,
        "skipped": sorted(
            skipped_http + skipped_mcp,
            key=lambda s: (s["surface"], s.get("route", s.get("tool", "")), s.get("method", "")),
        ),
    }
    text = json.dumps(manifest, sort_keys=True, indent=2, ensure_ascii=False) + "\n"
    if SECRET_LIKE.search(text):
        raise RuntimeError("golden-rich manifest.json: absolute home path or secret-like text")
    (rich_dir / "manifest.json").write_bytes(text.encode("utf-8"))
    fired = sum(1 for found in coverage.values() if found["fired"])
    print(
        f"{len(entries)} rich golden entries, {len(manifest['skipped'])} skipped; {fired} of {len(coverage)} rules fire somewhere; store v{version}, {saved.stat().st_size} bytes"
    )


def check_saved_store(saved: Path, writer: Writer) -> None:
    """The Rust tests read the saved store, the goldens were made over the live one: prove they answer the same."""
    from datetime import UTC, datetime

    from easy_personal.core import metrics, rules

    now = datetime.fromisoformat(FROZEN_NOW).astimezone(UTC)
    with sqlite3.connect(saved) as conn:
        pairs = [
            ("api__usage_summary__all.json", metrics.usage_summary(conn)),
            ("api__usage_summary__window.json", metrics.usage_summary(conn, **RICH_WINDOW)),
            ("api__usage_breakdown__project.json", metrics.usage_breakdown(conn, "project")),
            ("api__usage_breakdown__model.json", metrics.usage_breakdown(conn, "model")),
            ("api__warnings__all.json", rules.evaluate(conn, now)),
        ]
    for name, body in pairs:
        if json.loads(json.dumps(body)) != json.loads(json.dumps(writer.bodies[name])):
            raise RuntimeError(f"{name}: the saved store answers differently from the live store")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument(
        "--out", type=Path, default=PARITY, help="folder for golden/ and store-v<N>.sqlite"
    )
    parser.add_argument(
        "--tz",
        action="store_true",
        help="opt-in: write golden-tz/ (non-UTC zones, a few extra events) under --out instead of golden/",
    )
    parser.add_argument(
        "--rich",
        action="store_true",
        help="opt-in: write golden-rich/ and store-rich/ (a richer invented store, filter parameters, raw body hashes) under --out instead of golden/",
    )
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=None,
        help="transcript tree to ingest (default: the repo fixtures, or fixtures-rich with --rich; for the mutation check)",
    )
    args = parser.parse_args()
    if args.rich and args.tz:
        parser.error("--rich and --tz are separate modes")
    out_dir = args.out.resolve()
    global FIXTURES
    FIXTURES = (args.fixtures or (RICH_FIXTURES if args.rich else DEFAULT_FIXTURES)).resolve()
    refuse_real_paths(out_dir)
    if not args.rich:  # the rich fixtures name no folder of the machine
        refuse_fixture_folder()
    warn_if_not_locked()
    if args.tz:
        claim_output(out_dir, "golden-tz", [])
    elif args.rich:
        claim_output(
            out_dir, "golden-rich", sorted((out_dir / "store-rich").glob("store-v*.sqlite*"))
        )
    else:
        claim_output(out_dir, "golden", sorted(out_dir.glob("store-v*.sqlite*")))

    with tempfile.TemporaryDirectory(prefix="parity-") as tmp:
        work = Path(tmp).resolve()
        isolate(work)
        guard_sqlite()
        if args.tz:
            write_tz_goldens(out_dir, work)
            return
        if args.rich:
            write_rich_goldens(out_dir, work)
            return
        golden = out_dir / "golden"
        if golden.exists():
            shutil.rmtree(golden)  # regenerated whole; no stale file survives a renamed entry
        golden.mkdir()

        version, saved = build_store(work, out_dir)
        freeze_clock()
        from easy_personal.connectors import suggestions
        from easy_personal.surfaces import api

        unplug_machine_state(api, suggestions)

        writer = Writer(golden, work)
        skipped_http, csp = run_api(writer, work)
        skipped_mcp = run_mcp(writer)
        entries = sorted(writer.entries, key=lambda e: e["file"])
        manifest = {
            "generator": "rust/parity/gen_golden.py",
            "note": "golden files hold the response body only; status is here. Object keys are sorted, lists keep the app's order.",
            "versions": versions(),
            "store": {"file": saved.name, "user_version": version, "bytes": saved.stat().st_size},
            "fixture_sha256": tree_sha256(FIXTURES),
            "fixture_dir": "tests/fixtures/claude_code"
            if FIXTURES == DEFAULT_FIXTURES
            else "(custom)",
            "timezone": "UTC",
            "frozen_now": FROZEN_NOW,
            "token": TOKEN,
            "csp": csp,
            "volatile": sorted(writer.volatile, key=lambda v: (v["file"], v["token"]))
            + [
                {
                    "field": "store.events.machine_id / source_files.machine_id / meta.machine_id",
                    "token": MACHINE_ID,
                    "reason": "random uuid per store",
                },
                {
                    "field": "store.events.source_path / source_files.path",
                    "token": "claude_code/<relative path>",
                    "reason": "absolute fixture path",
                },
                {
                    "field": "store.events.inode / source_files.inode",
                    "token": "1..N in sorted path order",
                    "reason": "inode of this checkout",
                },
                {
                    "field": "now (rules/kit/kitperf/accounts windows)",
                    "token": FROZEN_NOW,
                    "reason": "clock patched in the generator: the routes have no parameter for it",
                },
            ],
            "patched": [
                {
                    "what": "api._configured, suggestions.configured_ids",
                    "to": "every connector 'not configured'",
                    "reason": "keychain and `claude` CLI are machine state",
                },
                {
                    "what": "schedule.systemd_available",
                    "to": "True",
                    "reason": "first-run hint would otherwise differ per machine; XDG_CONFIG_HOME points at an empty folder so no ingest timer exists",
                },
                {
                    "what": "datetime.now in rules, kit, kitperf, reports, accounts, easymem, context",
                    "to": FROZEN_NOW,
                    "reason": "see frozen_now",
                },
            ],
            "entries": entries,
            "skipped": sorted(
                skipped_http + skipped_mcp,
                key=lambda s: (
                    s["surface"],
                    s.get("route", s.get("tool", "")),
                    s.get("method", ""),
                ),
            ),
        }
        text = json.dumps(manifest, sort_keys=True, indent=2, ensure_ascii=False) + "\n"
        if SECRET_LIKE.search(text):
            raise RuntimeError("manifest.json: absolute home path or secret-like text")
        (golden / "manifest.json").write_bytes(text.encode("utf-8"))
        print(
            f"{len(entries)} golden entries, {len(manifest['skipped'])} skipped; store v{version}, {saved.stat().st_size} bytes"
        )


if __name__ == "__main__":
    main()
