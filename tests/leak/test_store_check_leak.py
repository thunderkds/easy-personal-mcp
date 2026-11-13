"""`store-check` and `GET /api/store/health` never show a row, a source path, a file name or an absolute path
(HUN-244, NFR-002, FR-091): canaries in store rows, in source paths, in file names beside the store and in a corrupted page.
"""

import contextlib
import io
import sqlite3

from starlette.testclient import TestClient

from easy_personal.core import config, store
from easy_personal.surfaces import api, cli
from leak import harness
from leak.canaries import Canaries
from tests_helpers import add_event

TOKEN, PORT = "leak-244", 5544


def _plant(canaries, home):
    values = canaries.values
    with store.opened(config.store_path()) as conn:
        for n in range(2500):  # enough pages to corrupt one of them below
            add_event(conn, n, f"/work/{values['file_path']}")
        conn.execute("UPDATE events SET source_path = ?, git_branch = ?, model = ?", (values["file_path"], values["branch_text"], values["model_text"]))
        conn.execute("INSERT INTO source_files VALUES ('m', 'claude_code', ?, 1, 0)", (values["file_path"],))
        conn.execute("INSERT INTO meta VALUES ('planted', ?)", (values["api_key"],))
        conn.commit()
    with sqlite3.connect(config.store_path()) as conn:
        conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    # file names beside the store: a decoy that is no backup, and a name that merely starts like one
    (home / f"{values['agent_name'].replace(' ', '_')}.txt").write_bytes(b"x")
    (home / "store.sqlite.bak-v2").write_bytes(b"x")
    (home / f"store.sqlite.bak-v3-{values['api_key']}").write_bytes(b"x")


def _corrupt(home):
    target = config.store_path()
    size = sqlite3.connect(target).execute("PRAGMA page_size").fetchone()[0]
    pages = target.stat().st_size // size
    with open(target, "r+b") as handle:
        handle.seek(size * (pages // 2))
        handle.write(b"\xff" * size)


def _scan(canaries, monkeypatch, home):
    """CLI (both forms, in process: stdout, stderr, logs) and the route; return every blob."""
    blobs = {}
    client = TestClient(api.create_app(TOKEN, PORT), base_url=f"http://127.0.0.1:{PORT}")
    with harness.LogCapture() as logs:
        for argv in (["store-check"], ["store-check", "--json"]):
            out, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                rc = cli.main(argv)
            blobs[" ".join(argv)] = (rc, out.getvalue(), err.getvalue())
        res = client.get("/api/store/health", headers={api.TOKEN_HEADER: TOKEN})
        blobs["route"] = (res.status_code, dict(res.headers), res.text)
        blobs["route no token"] = (client.get("/api/store/health").text,)
        blobs["log"] = (logs.text(),)
    return blobs


def test_nothing_planted_leaves_through_either_surface(tmp_path, monkeypatch):
    canaries = Canaries.new()
    home = tmp_path.resolve() / "home-244"
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(home))
    _plant(canaries, home)
    for state in ("healthy", "corrupted"):
        if state == "corrupted":
            _corrupt(home)
        blobs = _scan(canaries, monkeypatch, home)
        assert blobs["store-check"][0] == (0 if state == "healthy" else 1)
        for label, blob in blobs.items():
            canaries.assert_clean(f"{state} {label}", blob)
            text = repr(blob)
            assert str(tmp_path) not in text and "home-244" not in text and "/work/" not in text
        listed = blobs["route"][2]
        assert "store.sqlite.bak-v2" in listed and "bak-v3-" not in listed  # a real backup is named; the lookalike is not
