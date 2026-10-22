"""A secret typed into `connector add` never reaches stdout, stderr, logs, `claude` argv or disk.

The keychain is an in-memory fake and `claude` is a fake script on PATH; the real ones are never used.
"""
import builtins
import getpass
import json
import sys

import keyring
import pytest

from connectors.fake_backend import MemoryKeyring
from easy_personal.connectors import writer
from easy_personal.surfaces import cli
from leak import harness
from leak.canaries import Canaries

FAKE_CLAUDE = """#!{python}
import json, os, sys
open(os.environ["FAKE_CLAUDE_LOG"], "a").write(json.dumps(sys.argv[1:]) + "\\n")
if sys.argv[2:3] == ["get"]:
    sys.exit(1)
sys.stderr.write("claude: could not write")
sys.exit(int(os.environ.get("FAKE_CLAUDE_RC", "0")))
"""


@pytest.fixture
def world(tmp_path, monkeypatch):
    bindir = tmp_path / "bin"
    bindir.mkdir()
    exe = bindir / "claude"
    exe.write_text(FAKE_CLAUDE.format(python=sys.executable))
    exe.chmod(0o755)
    home = tmp_path / "home"
    home.mkdir()
    monkeypatch.setenv("PATH", str(bindir))
    monkeypatch.setenv("FAKE_CLAUDE_LOG", str(tmp_path / "argv.log"))
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(home))
    monkeypatch.setattr(writer, "launcher_path", lambda: "/opt/fake/easy-personal-launch")
    old = keyring.get_keyring()
    keyring.set_keyring(MemoryKeyring())
    yield tmp_path, home
    keyring.set_keyring(old)


def _run(capsys, monkeypatch, canaries, rc_claude, argv, confirm="y"):
    monkeypatch.setenv("FAKE_CLAUDE_RC", str(rc_claude))
    monkeypatch.setattr(getpass, "getpass", lambda prompt="": canaries.values["api_key"])
    monkeypatch.setattr(
        builtins, "input", lambda prompt="": confirm if "[y/N]" in prompt else canaries.values["email"]
    )
    code = cli.main(argv)
    out = capsys.readouterr()
    return code, {"stdout": out.out, "stderr": out.err}


def test_secret_typed_into_connector_add_never_leaks(world, capsys, monkeypatch):
    tmp_path, home = world
    canaries = Canaries.new()
    blobs = {}
    with harness.LogCapture() as logs:
        for rc_claude, argv in (
            (0, ["connector", "add", "slack"]),  # success
            (1, ["connector", "add", "slack"]),  # claude fails -> rollback
            (0, ["connector", "add", "slack", "--yes"]),
            (0, ["doctor"]),
            (0, ["connector", "remove", "slack", "--yes"]),
        ):
            code, shown = _run(capsys, monkeypatch, canaries, rc_claude, argv)
            blobs[f"{argv} rc={rc_claude} -> {code}"] = shown
    # the argv the fake claude saw, and every file written under the home or the temp tree
    blobs["claude argv"] = (tmp_path / "argv.log").read_text()
    blobs["logs"] = logs.text()
    for path in sorted(p for p in tmp_path.rglob("*") if p.is_file() and p.name != "claude"):
        blobs[f"file {path.relative_to(tmp_path)}"] = path.read_bytes()
    for surface, blob in blobs.items():
        canaries.assert_clean(surface, blob)
    assert json.loads((tmp_path / "argv.log").read_text().splitlines()[1])[1] == "add-json"
    assert not list(home.glob("store.sqlite*"))  # connector commands never touch the usage store


def test_scan_would_catch_a_leaking_argv(world, capsys, monkeypatch):
    canaries = Canaries.new()
    leaked = json.dumps(["mcp", "add-json", "slack", canaries.values["api_key"]])
    assert canaries.find(leaked)
